//! Case geometry and CAD preparation from the page's accepted snapshot.

use boardstudio_application::{AcceptedSnapshot, Scope};
#[cfg(test)]
use boardstudio_core::CoreEngine;
use boardstudio_model::model::*;
use boardstudio_web_worker_host::cad_jobs::CadJobError;
#[cfg(any(target_arch = "wasm32", test))]
use boardstudio_web_worker_host::cad_jobs::{
    CadSnapshotIdentity, PreparedCadSnapshot, revision_supported_by_cad_boundary,
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

/// Resolve mechanical policy and prepare case regions using the public core protocol.
/// The CAD package uses JavaScript numbers for revision fields, so unsupported
/// wide identities fail before crossing that boundary.
#[cfg(target_arch = "wasm32")]
pub async fn prepare_captured_case(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<PreparedCadSnapshot, CadJobError> {
    prepare_captured_case_for_step(core, executor_epoch, request_prefix, snapshot, scope, false)
        .await
}

/// Prepare a mechanical export assembly with the nominal PCB reference body,
/// matching the existing mechanical assembly export contract.
#[cfg(target_arch = "wasm32")]
pub async fn prepare_captured_step_assembly(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Result<PreparedCadSnapshot, CadJobError> {
    prepare_captured_case_for_step(core, executor_epoch, request_prefix, snapshot, scope, true)
        .await
}

#[cfg(target_arch = "wasm32")]
async fn prepare_captured_case_for_step(
    core: &dyn crate::host::CoreExecutor,
    executor_epoch: &str,
    request_prefix: &str,
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    include_pcb_reference: bool,
) -> Result<PreparedCadSnapshot, CadJobError> {
    let (identity, initial) = preparation_request(snapshot, scope, request_prefix)?;
    let reply = core
        .request(
            &format!("{request_prefix}-resolve"),
            executor_epoch,
            &initial,
        )
        .await
        .map_err(|error| CadJobError::Failed(error.to_string()))?;
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
    let reply = core
        .request(
            &format!("{request_prefix}-prepare"),
            executor_epoch,
            &prepare,
        )
        .await
        .map_err(|error| CadJobError::Failed(error.to_string()))?;
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
        let reply = core
            .request(&request_id, executor_epoch, &prepare_reference)
            .await
            .map_err(|error| CadJobError::Failed(error.to_string()))?;
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

#[cfg(any(target_arch = "wasm32", test))]
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
    let target = match &scope.instance_id {
        Some(instance_id) => boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
            instance_id: instance_id.clone(),
        },
        None => boardstudio_core_mechanical::mechanical::MechanicalTarget::Board {
            board_id: scope.board_id.clone(),
        },
    };
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
    let target = match &scope.instance_id {
        Some(instance_id) => boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
            instance_id: instance_id.clone(),
        },
        None => boardstudio_core_mechanical::mechanical::MechanicalTarget::Board {
            board_id: scope.board_id.clone(),
        },
    };
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
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != document.id
        || scene.revision != document.revision
    {
        return Err(CadJobError::Stale(
            "case preparation snapshot or scope changed".into(),
        ));
    }
    if !revision_supported_by_cad_boundary(document.revision) {
        return Err(CadJobError::Blocked(
            "case revision exceeds the exact integer range of the existing CAD JavaScript boundary"
                .into(),
        ));
    }
    if !document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id)
    {
        return Err(CadJobError::Blocked(
            "selected board is not in the captured document".into(),
        ));
    }
    let (effective_document, effective_scene) = effective_case_inputs(document, scene, scope)?;
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
    let request = if configured {
        CoreRequest::ResolveMechanical {
            id: format!("{request_prefix}-resolve"),
            document: effective_document.clone(),
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
            document_id: effective_document.id.clone(),
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
    use boardstudio_model::model::{
        Board, BoardContours, BoardReadiness, CaseBody, CaseKind, SceneDelta,
    };

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
    fn flipped_instance_reflects_only_its_board_parts_and_scene_transform() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 4;
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline-a".into(),
            center: Vec2 { x: 10.0, y: 0.0 },
            size: Vec2 { x: 20.0, y: 10.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        document.boards.push(Board {
            id: "board-a".into(),
            name: "Board A".into(),
            outline_ids: vec!["outline-a".into()],
            part_ids: vec!["part-a".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.boards.push(Board {
            id: "board-b".into(),
            name: "Board B".into(),
            outline_ids: vec![],
            part_ids: vec!["part-b".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let part = |id: &str, x: f64| Part {
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 2.0 },
                rotation: 27.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        };
        document.parts = vec![part("part-a", 3.0), part("part-b", 8.0)];
        document.hardware = Some(HardwareConfiguration {
            instances: vec![PhysicalBoardInstance {
                id: "instance-a".into(),
                name: "A, flipped".into(),
                board_id: "board-a".into(),
                half: "left".into(),
                role: "controller".into(),
                flipped: true,
                controller_part_id: None,
                mechanical: None,
                construction_ownership:
                    boardstudio_model::model::MechanicalConstructionOwnership::Legacy,
                mechanical_identity_reservation: None,
            }],
            ..HardwareConfiguration::default()
        });
        let scene = SceneDelta {
            revision: 4,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![
                Transform {
                    id: "part-a".into(),
                    pose: Pose2 {
                        at: Vec2 { x: 3.0, y: 2.0 },
                        rotation: 27.0,
                    },
                },
                Transform {
                    id: "part-b".into(),
                    pose: Pose2 {
                        at: Vec2 { x: 8.0, y: 2.0 },
                        rotation: 27.0,
                    },
                },
            ],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board-a".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: 0.0, y: 5.0 },
                        Vec2 { x: 0.0, y: -5.0 },
                        Vec2 { x: 20.0, y: -5.0 },
                        Vec2 { x: 20.0, y: 5.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: true,
            },
        };
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board-a".into(),
            instance_id: Some("instance-a".into()),
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(9),
            session_epoch: scope.session_epoch,
            document: std::sync::Arc::new(document.clone()),
            scene: std::sync::Arc::new(scene.clone()),
        };
        let captured_document = captured_case_document(&snapshot, &scope).unwrap();
        let captured_scene = captured_case_scene(&snapshot, &scope).unwrap();
        let (effective, scene) = effective_case_inputs(&document, &scene, &scope).unwrap();
        assert_eq!(effective.parts[0].pose.at.x, -3.0);
        assert_eq!(effective.parts[0].pose.rotation, -27.0);
        assert_eq!(effective.parts[0].side, Side::Back);
        assert_eq!(effective.parts[1], document.parts[1]);
        assert_eq!(scene.transforms[0].pose.at.x, -3.0);
        assert_eq!(scene.transforms[1].pose.at.x, 8.0);
        // The target board's scene contours come from Core's mirrored projection.
        let board_contours = scene
            .board_contours
            .iter()
            .find(|entry| entry.board_id == "board-a")
            .unwrap();
        assert!(!board_contours.contours.is_empty());
        assert!(
            board_contours
                .contours
                .iter()
                .flat_map(|contour| &contour.points)
                .all(|point| point.x <= 0.0),
            "the mirrored contours negate X"
        );
        assert_eq!(
            captured_document.physical_instance_id.as_deref(),
            Some("instance-a")
        );
        assert_eq!(captured_document, effective);
        assert_eq!(captured_scene, scene);
    }

    #[test]
    fn captured_document_and_scene_prepare_through_the_public_core_provider() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 7;
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
        document.case_bodies.push(CaseBody {
            features: None,
            openings: None,
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: Some("pla".into()),
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        });
        let contour = Contour {
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 30.0, y: 0.0 },
                Vec2 { x: 30.0, y: 20.0 },
                Vec2 { x: 0.0, y: 20.0 },
            ],
            hole: false,
        };
        let scene = SceneDelta {
            revision: 7,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board".into(),
                contours: vec![contour],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "board".into(),
                outline: true,
                pcb: false,
                case_ready: false,
            }],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![Finding {
                id: "case:plate:material".into(),
                severity: Severity::Error,
                scope: boardstudio_model::model::Scope::Case,
                message: "Case material is missing".into(),
                target_ids: vec!["plate".into(), "pla".into()],
            }],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: false,
                case_ready: false,
            },
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(3),
            session_epoch: SessionEpoch(2),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let (identity, request) =
            preparation_request(&snapshot, &scope, "fixture").expect("captured request");
        let mut engine = CoreEngine::new();
        let reply = engine.handle(request);
        let CoreReply::CasePrepared { ir, .. } = reply else {
            panic!("core preparation failed: {reply:?}")
        };
        let prepared =
            validate_prepared(identity, ir, None).expect("revision guarded prepared case");
        assert_eq!(prepared.prepared.bodies[0].body.id, "plate");
        assert_eq!(prepared.prepared.bodies[0].regions.len(), 1);
        assert_eq!(prepared.prepared.bodies[0].regions[0].outer.len(), 4);
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

    /// Two halves sharing one PCB where the reflected linked mount clashes with the
    /// second half's own hardware. Admission must be decided per selected target.
    #[cfg(feature = "page")]
    #[test]
    fn case_admission_is_scoped_to_the_selected_half_of_a_shared_pcb() {
        use boardstudio_model::model::{
            HardwareConfiguration, HardwareTopology, HardwareTransport, MechanicalConfiguration,
            MechanicalConstructionOwnership, MechanicalFeature, MechanicalFeatureMember,
            MechanicalFeatureState, MechanicalFeatureValue, MechanicalMirrorPair, Mount, MountKind,
            PhysicalBoardInstance, Pose2,
        };
        fn configuration() -> MechanicalConfiguration {
            serde_json::from_value(serde_json::json!({
                "boardId": "board",
                "method": "printed",
                "mount": "rigid",
                "plateThickness": 1.5,
                "plateFoamThickness": 0.5,
                "pcbThickness": 1.6,
                "bottomFoamThickness": 0.4,
                "batteryHeight": 4.0,
                "bottomThickness": 3.0,
                "plateToPcb": 3.5,
                "wallThickness": 2.0,
                "clearance": 0.3,
                "bottomStyle": "shell",
                "middleFrame": false,
                "openingAllowance": 0.25,
                "mounts": [],
                "closureMounts": [],
                "profiles": []
            }))
            .unwrap()
        }
        fn mount(id: &str, x: f64, y: f64) -> Mount {
            Mount {
                id: id.into(),
                at: Vec2 { x, y },
                kind: MountKind::Hole,
                hole_diameter: 2.2,
                boss_diameter: None,
                height: None,
            }
        }
        fn instance(id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
            let mut mechanical = configuration();
            mechanical.mounts = mounts;
            PhysicalBoardInstance {
                id: id.into(),
                name: id.into(),
                board_id: "board".into(),
                half: id.into(),
                role: "controller".into(),
                flipped: false,
                controller_part_id: None,
                mechanical: Some(mechanical),
                construction_ownership: MechanicalConstructionOwnership::Local,
                mechanical_identity_reservation: None,
            }
        }

        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.revision = 7;
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
        document.case_bodies.push(CaseBody {
            features: None,
            openings: None,
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: Some("pla".into()),
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        });
        let left = instance("left", vec![mount("mount-1", 2.0, 3.0)]);
        let right = instance(
            "right",
            vec![
                mount("mount-1", 7.0, 3.0),
                mount("mount-encoder", -2.0, 3.0),
            ],
        );
        let construction = boardstudio_core_mechanical::mechanical::features::shared_construction(
            left.mechanical.as_ref().unwrap(),
        );
        document.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![left, right],
            boards: vec![],
            shared_construction: None,
            pairs: vec![MechanicalMirrorPair {
                id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction,
                construction_source: None,
                halves: None,
                features: vec![MechanicalFeature {
                    id: "mount-1".into(),
                    kind: MechanicalFeatureKind::Mount,
                    author: "left".into(),
                    value: MechanicalFeatureValue::Mount {
                        mount: mount("mount-1", 2.0, 3.0),
                    },
                    members: vec![
                        MechanicalFeatureMember {
                            instance_id: "left".into(),
                            feature_id: "mount-1".into(),
                            state: MechanicalFeatureState::Linked,
                        },
                        MechanicalFeatureMember {
                            instance_id: "right".into(),
                            feature_id: "mount-1".into(),
                            state: MechanicalFeatureState::Linked,
                        },
                    ],
                }],
            }],
        });
        let scene = SceneDelta {
            revision: 7,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: -40.0, y: -30.0 },
                        Vec2 { x: 40.0, y: -30.0 },
                        Vec2 { x: 40.0, y: 30.0 },
                        Vec2 { x: -40.0, y: 30.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "board".into(),
                outline: true,
                pcb: true,
                case_ready: false,
            }],
            board_outline_scenes: vec![],
            module_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(9),
            session_epoch: SessionEpoch(1),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = |instance_id: Option<&str>| Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: instance_id.map(str::to_string),
        };

        // The conflict belongs to the right half, so the left half is admitted.
        assert!(
            preparation_request(&snapshot, &scope(Some("left")), "fixture").is_ok(),
            "a sibling half's conflict must not block this half"
        );
        // The half that owns the conflict is blocked.
        assert!(matches!(
            preparation_request(&snapshot, &scope(Some("right")), "fixture"),
            Err(CadJobError::Blocked(_))
        ));
    }
}
