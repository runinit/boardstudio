//! Case geometry and CAD preparation from the page's accepted snapshot.

use boardstudio_application::{AcceptedSnapshot, Scope};
#[cfg(test)]
use boardstudio_core::CoreEngine;
use boardstudio_model::model::*;
use boardstudio_web_worker_host::cad_jobs::CadJobError;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_worker_host::cad_jobs::PreparedCadSnapshot;
#[cfg(any(target_arch = "wasm32", test))]
use boardstudio_web_worker_host::cad_jobs::{
    CadSnapshotIdentity, revision_supported_by_cad_boundary,
};

/// Return consumer-owned geometry for the selected physical board instance.
/// The accepted snapshot remains immutable; callers use this scene alongside
/// the corresponding `PreparedCadSnapshot` for preview rendering.
pub fn captured_case_scene(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<SceneDelta, CadJobError> {
    captured_case_inputs(snapshot, scope).map(|(_, scene)| scene)
}

/// Return the consumer-owned document projection for a selected physical
/// board instance, including effective shared mechanical defaults and flip
/// transforms. The accepted snapshot is never mutated.
pub fn captured_case_document(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<ProjectDoc, CadJobError> {
    captured_case_inputs(snapshot, scope).map(|(document, _)| document)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapturedPcbSource {
    pub reference: BoardReference,
    pub asset: Asset,
    pub placement: Pose2,
    pub flipped: bool,
}

/// Capture raw PCB ownership before the Case display projection disables mirrored references.
pub fn captured_pcb_source(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<Option<CapturedPcbSource>, CadJobError> {
    let document = snapshot.document.as_ref();
    if snapshot.session_epoch != scope.session_epoch
        || document.id != scope.document_id
        || snapshot.scene.revision != document.revision
    {
        return Err(CadJobError::Stale("PCB source scope changed".into()));
    }
    if document
        .boards
        .iter()
        .filter(|board| board.id == scope.board_id)
        .count()
        != 1
    {
        return Err(CadJobError::Blocked(
            "PCB source board is missing or ambiguous".into(),
        ));
    }
    let flipped = if let Some(id) = &scope.instance_id {
        let instances = document
            .hardware
            .as_ref()
            .into_iter()
            .flat_map(|hardware| &hardware.instances)
            .filter(|instance| instance.id == *id && instance.board_id == scope.board_id)
            .collect::<Vec<_>>();
        if instances.len() != 1 {
            return Err(CadJobError::Blocked(
                "PCB source physical instance is missing or ambiguous".into(),
            ));
        }
        instances[0].flipped
    } else {
        false
    };
    let references = document
        .board_references
        .iter()
        .filter(|reference| reference.board_id == scope.board_id && reference.enabled)
        .collect::<Vec<_>>();
    if references.is_empty() {
        return Ok(None);
    }
    if references.len() != 1 {
        return Err(CadJobError::Blocked(
            "PCB source reference is ambiguous".into(),
        ));
    }
    let reference = references[0];
    if !reference.elevation.is_finite()
        || !reference.pose.at.x.is_finite()
        || !reference.pose.at.y.is_finite()
        || !reference.pose.rotation.is_finite()
    {
        return Err(CadJobError::Blocked(
            "PCB source placement must be finite".into(),
        ));
    }
    let assets = document
        .assets
        .iter()
        .filter(|asset| asset.id == reference.asset_id)
        .collect::<Vec<_>>();
    if assets.len() != 1 || assets[0].sha256.is_empty() {
        return Err(CadJobError::Blocked(
            "PCB source asset is missing or ambiguous".into(),
        ));
    }
    let mut placement = reference.pose;
    if flipped {
        // World-X reflection after placement equals reflected local X at (-tx, ty, -angle).
        placement.at.x = -placement.at.x;
        placement.rotation = -placement.rotation;
    }
    Ok(Some(CapturedPcbSource {
        reference: reference.clone(),
        asset: assets[0].clone(),
        placement,
        flipped,
    }))
}

fn captured_case_inputs(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<(ProjectDoc, SceneDelta), CadJobError> {
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || snapshot.scene.revision != snapshot.document.revision
    {
        return Err(CadJobError::Stale(
            "case scene scope or accepted revision changed".into(),
        ));
    }
    effective_case_inputs(&snapshot.document, &snapshot.scene, scope)
}

/// Prepare CAD from the selected accepted snapshot. STEP includes the nominal PCB body.
#[cfg(target_arch = "wasm32")]
pub fn prepare_captured_case(
    mut core: impl FnMut(CoreRequest) -> CoreReply,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    include_pcb_reference: bool,
) -> Result<PreparedCadSnapshot, CadJobError> {
    let (identity, initial) = preparation_request(snapshot, scope, request_prefix)?;
    let reply = core(initial.clone());
    let mut pcb_reference = None;
    let mechanical_assembly;
    let prepare = match reply {
        CoreReply::CasePrepared { id: _, ir } => {
            return validate_prepared(identity, ir, None);
        }
        CoreReply::MechanicalResolved { assembly, .. } => {
            if assembly.revision != identity.revision || assembly.case.revision != identity.revision
            {
                return Err(CadJobError::Stale(
                    "core resolved another case revision".into(),
                ));
            }
            if assembly.generation_blocked {
                return Err(CadJobError::Blocked(
                    "mechanical findings block case generation".into(),
                ));
            }
            let errors = assembly
                .diagnostics
                .iter()
                .filter(|finding| finding.severity == Severity::Error)
                .map(|finding| finding.message.as_str())
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                return Err(CadJobError::Blocked(errors.join("\n")));
            }
            if include_pcb_reference {
                let (document, contours) = match &initial {
                    CoreRequest::ResolveMechanical {
                        document, contours, ..
                    } => (document, contours),
                    _ => unreachable!("mechanical reply requires a mechanical request"),
                };
                let configuration = document.mechanical.as_ref().ok_or_else(|| {
                    CadJobError::Blocked("mechanical export has no selected configuration".into())
                })?;
                pcb_reference = Some(assembly.pcb_reference.clone().unwrap_or_else(|| CaseIR {
                    revision: identity.revision,
                    contours: contours.clone(),
                    body: CaseBody {
                        id: "pcb-reference".into(),
                        name: "PCB reference — unpopulated".into(),
                        board_id: configuration.board_id.clone(),
                        kind: CaseKind::Plate,
                        thickness: configuration.pcb_thickness,
                        clearance: 0.0,
                        material_id: None,
                        z: Some(-configuration.pcb_thickness),
                        wall_height: None,
                        wall_thickness: None,
                        mounts: None,
                        pcb_source: None,
                        gasket: None,
                        openings: None,
                        features: None,
                    },
                }));
            }
            mechanical_assembly = Some(assembly.clone());
            CoreRequest::PrepareCase {
                id: format!("{request_prefix}-prepare"),
                ir: assembly.case,
            }
        }
        CoreReply::Error { message, .. } => return Err(CadJobError::Blocked(message)),
        _ => {
            return Err(CadJobError::Failed(
                "unexpected core case preparation reply".into(),
            ));
        }
    };
    let reply = core(prepare);
    let mut prepared = match reply {
        CoreReply::CasePrepared { ir, .. } => Ok(ir),
        CoreReply::Error { message, .. } => Err(CadJobError::Blocked(message)),
        _ => Err(CadJobError::Failed(
            "unexpected core case preparation reply".into(),
        )),
    }?;
    if let Some(reference) = pcb_reference {
        let request_id = format!("{request_prefix}-pcb-reference");
        let prepare_reference = CoreRequest::PrepareCase {
            id: request_id.clone(),
            ir: CaseAssemblyIR {
                revision: identity.revision,
                bodies: vec![reference],
            },
        };
        let reply = core(prepare_reference);
        match reply {
            CoreReply::CasePrepared { ir, .. } if ir.revision == identity.revision => {
                prepared.bodies.extend(ir.bodies);
            }
            CoreReply::CasePrepared { .. } => {
                return Err(CadJobError::Stale(
                    "PCB reference has another revision".into(),
                ));
            }
            CoreReply::Error { message, .. } => return Err(CadJobError::Blocked(message)),
            _ => {
                return Err(CadJobError::Failed(
                    "unexpected PCB reference preparation reply".into(),
                ));
            }
        }
    }
    validate_prepared(identity, prepared, mechanical_assembly)
}

#[cfg(target_arch = "wasm32")]
fn validate_prepared(
    identity: CadSnapshotIdentity,
    prepared: PreparedCaseAssemblyIR,
    mechanical_assembly: Option<MechanicalAssembly>,
) -> Result<PreparedCadSnapshot, CadJobError> {
    if prepared.revision != identity.revision
        || prepared
            .bodies
            .iter()
            .any(|body| body.revision != identity.revision)
    {
        return Err(CadJobError::Stale(
            "prepared case does not match its captured revision".into(),
        ));
    }
    if prepared.bodies.is_empty() {
        return Err(CadJobError::Blocked(
            "the selected board has no case bodies".into(),
        ));
    }
    Ok(PreparedCadSnapshot {
        identity,
        prepared,
        mechanical_assembly,
    })
}

fn effective_case_inputs(
    document: &ProjectDoc,
    scene: &SceneDelta,
    scope: &Scope,
) -> Result<(ProjectDoc, SceneDelta), CadJobError> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
        .ok_or_else(|| {
            CadJobError::Blocked("selected board is not in the captured document".into())
        })?;
    let selected_instance = match &scope.instance_id {
        None => None,
        Some(instance_id) => {
            let instance = document
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.id == *instance_id)
                })
                .ok_or_else(|| {
                    CadJobError::Blocked("selected physical board instance is unavailable".into())
                })?;
            if instance.board_id != scope.board_id {
                return Err(CadJobError::Stale(
                    "selected instance belongs to another board".into(),
                ));
            }
            Some(instance)
        }
    };

    let flipped = selected_instance.is_some_and(|instance| instance.flipped);

    // Core owns the effective-document projection: the fully defaulted configuration,
    // the physical-instance identity, flipped-instance part/reference mirroring and the
    // target board's mirrored contours. The host keeps only the scene transform and
    // target-contour mirroring that are not part of the document projection.
    let target = mechanical_target(scope);
    let (effective, _target_contours) =
        boardstudio_core_mechanical::mechanical::inputs::effective_document(document, &target)
            .map_err(CadJobError::Blocked)?;

    let mut effective_scene = scene.clone();
    if flipped {
        // The scene transform and contour mirroring are not part of the document
        // projection, so the host keeps them and mirrors the accepted scene's own
        // target-board contours in place. Core's `effective_document` returns the
        // same contours for its own resolver and clearance projection.
        let selected_parts = board
            .part_ids
            .iter()
            .collect::<std::collections::HashSet<_>>();
        for transform in &mut effective_scene.transforms {
            if selected_parts.contains(&transform.id) {
                transform.pose.at.x = -transform.pose.at.x;
                transform.pose.rotation = -transform.pose.rotation;
            }
        }
        for board_contours in &mut effective_scene.board_contours {
            if board_contours.board_id == scope.board_id {
                for contour in &mut board_contours.contours {
                    for point in &mut contour.points {
                        point.x = -point.x;
                    }
                    contour.points.reverse();
                }
            }
        }
    }
    Ok((effective, effective_scene))
}

fn mechanical_target(scope: &Scope) -> boardstudio_core_mechanical::mechanical::MechanicalTarget {
    use boardstudio_core_mechanical::mechanical::MechanicalTarget;
    match &scope.instance_id {
        Some(instance_id) => MechanicalTarget::Instance {
            instance_id: instance_id.clone(),
        },
        None => MechanicalTarget::Board {
            board_id: scope.board_id.clone(),
        },
    }
}

/// Whether the selected target's own accepted findings leave its Case output clear.
///
/// Uses the approved target-scoped effective read, so a finding naming a sibling half of
/// the same board never blocks this target, and a finding naming this target does.
#[cfg(any(target_arch = "wasm32", test))]
fn target_case_findings_are_clear(
    document: &ProjectDoc,
    scene: &SceneDelta,
    scope: &Scope,
) -> bool {
    let target = mechanical_target(scope);
    match boardstudio_core_mechanical::mechanical::effective_inputs(
        document,
        &scene.board_contours,
        &target,
    ) {
        // Core owns the rule; the adapter only selects the target.
        Ok(inputs) => inputs.case_findings_are_clear(),
        // An unresolvable target is decided by the shared board-level admission.
        Err(_) => true,
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn preparation_request(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    request_prefix: &str,
) -> Result<(CadSnapshotIdentity, CoreRequest), CadJobError> {
    let document = &snapshot.document;
    let scene = &snapshot.scene;
    if !revision_supported_by_cad_boundary(document.revision) {
        return Err(CadJobError::Blocked(
            "case revision exceeds the exact integer range of the existing CAD JavaScript boundary"
                .into(),
        ));
    }
    let (effective_document, effective_scene) = captured_case_inputs(snapshot, scope)?;
    let configured = effective_document
        .mechanical
        .as_ref()
        .is_some_and(|case| case.board_id == scope.board_id);
    // Admission is decided per selected target: the target-scoped effective read is the
    // authority for this half's Case problems. A conflict that names a sibling half of
    // the same PCB is not this target's problem, while this target's own conflict blocks
    // its case generation even though the board-level admission cannot see the half.
    if !boardstudio_model::case_readiness::case_preparation_ready(
        &effective_document,
        &effective_scene,
        &scope.board_id,
        configured,
    ) || !target_case_findings_are_clear(document, scene, scope)
    {
        return Err(CadJobError::Blocked(
            "selected board is not ready for case generation".into(),
        ));
    }
    let contours = effective_scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default();
    let document_id = effective_document.id.clone();
    let request = if configured {
        CoreRequest::ResolveMechanical {
            id: format!("{request_prefix}-resolve"),
            document: effective_document,
            contours,
        }
    } else {
        let bodies = effective_document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .cloned()
            .map(|body| CaseIR {
                revision: effective_document.revision,
                body,
                contours: contours.clone(),
            })
            .collect();
        CoreRequest::PrepareCase {
            id: format!("{request_prefix}-resolve"),
            ir: CaseAssemblyIR {
                revision: effective_document.revision,
                bodies,
            },
        }
    };
    Ok((
        CadSnapshotIdentity {
            token: snapshot.token.0,
            session_epoch: snapshot.session_epoch.0,
            document_id,
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            revision: document.revision,
        },
        request,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_model::model::{Board, SceneDelta};

    #[test]
    fn pcb_source_capture_keeps_original_reference_and_composes_physical_reflection_once() {
        let mut document = ProjectDoc::empty("source-doc", "PCB source");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id":"board", "name":"Board", "outlineIds":[],"partIds":[],"netIds":[],
                "thickness":1.6,"traces":[],"vias":[]
            }))
            .unwrap(),
        );
        document.assets.push(
            serde_json::from_value(serde_json::json!({
                "id":"raw", "name":"routed.kicad_pcb", "mediaType":"application/vnd.kicad.pcb",
                "sha256":"0".repeat(64)
            }))
            .unwrap(),
        );
        document.board_references.push(BoardReference {
            id: "reference".into(),
            board_id: "board".into(),
            asset_id: "raw".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 100., y: 200. },
                rotation: 90.,
            },
            elevation: 3.,
            model_assets: Default::default(),
        });
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({
                "topology":"unibody","transport":"none","boards":[],"instances":[{
                    "id":"right","name":"Right","boardId":"board","half":"right","role":"central",
                    "flipped":true,"controllerPartId":null,"mechanical":null
                }]
            }))
            .unwrap(),
        );
        let CoreReply::Scene {
            document, scene, ..
        } = CoreEngine::new().handle(CoreRequest::Open {
            id: "open".into(),
            document,
        })
        else {
            panic!("source fixture must open");
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(1),
            session_epoch: SessionEpoch(1),
            document: std::sync::Arc::new(*document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "source-doc".into(),
            board_id: "board".into(),
            instance_id: Some("right".into()),
        };
        let before = snapshot.document.clone();
        let captured = captured_pcb_source(&snapshot, &scope).unwrap().unwrap();
        assert!(captured.flipped);
        assert_eq!(
            captured.placement,
            Pose2 {
                at: Vec2 { x: -100., y: 200. },
                rotation: -90.
            }
        );
        assert_eq!(captured.reference.pose.at.x, 100.);
        assert_eq!(captured.reference.elevation, 3.);
        assert_eq!(captured.asset.id, "raw");
        assert_eq!(
            snapshot.document, before,
            "capture must not disable or mutate the source"
        );

        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.document)
            .board_references
            .push(BoardReference {
                id: "second-reference".into(),
                ..captured.reference.clone()
            });
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Blocked(_))
        ));
        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.document)
            .assets
            .clear();
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Blocked(_))
        ));
        let mut invalid = snapshot.clone();
        std::sync::Arc::make_mut(&mut invalid.scene).revision += 1;
        assert!(matches!(
            captured_pcb_source(&invalid, &scope),
            Err(CadJobError::Stale(_))
        ));
        let mut authored = snapshot;
        std::sync::Arc::make_mut(&mut authored.document)
            .board_references
            .clear();
        assert!(captured_pcb_source(&authored, &scope).unwrap().is_none());
    }

    #[test]
    fn scope_or_scene_revision_mismatch_is_rejected_before_core_work() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(1),
            session_epoch: SessionEpoch(2),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(SceneDelta {
                revision: 1,
                transaction_id: String::new(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                module_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "other".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        assert!(matches!(
            preparation_request(&snapshot, &scope, "fixture"),
            Err(CadJobError::Stale(_))
        ));
    }
}
