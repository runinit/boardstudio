//! Page-private source capture and identity for the native Case board preview.
//!
//! This is a render-only accepted projection. It deliberately does not depend on
//! a mechanical `CadScene`. Authored and imported boards share the physical Case
//! owner while retaining their existing, distinct Core preview producers.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    ArtifactRequest, Asset, BoardReference, ExportTarget, PcbPreview, PrepareExportRequest,
    ProjectDoc,
};
use boardstudio_web::cad_jobs::{captured_case_document, captured_case_scene};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CasePreviewOwnerIdentity {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) accepted_revision: u64,
    pub(crate) accepted_scene_identity: usize,
    pub(crate) viewer_instance: u64,
    pub(crate) projection_generation: u64,
    pub(crate) batch_generation: u64,
    pub(crate) core_executor_epoch: u64,
    pub(crate) core_worker_identity: usize,
    pub(crate) request_token: String,
}

impl CasePreviewOwnerIdentity {
    pub(crate) fn capture(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        projection_generation: u64,
        batch_generation: u64,
        core_executor_epoch: u64,
        core_worker_identity: usize,
        request_token: String,
    ) -> Self {
        Self {
            scope: scope.clone(),
            snapshot_token: snapshot.token,
            accepted_revision: snapshot.document.revision,
            accepted_scene_identity: std::sync::Arc::as_ptr(&snapshot.scene) as usize,
            viewer_instance: projection_generation,
            projection_generation,
            batch_generation,
            core_executor_epoch,
            core_worker_identity,
            request_token,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum CasePreviewRequest {
    Authored(Box<PrepareExportRequest>),
    Imported {
        reference: BoardReference,
        asset: Asset,
    },
}

/// The producer-owned lease is independent of mechanical `CadScene` lifetime.
/// Model deliveries retain only a weak handle and are invalid when the accepted
/// Case source is superseded or its preview owner is replaced.
#[derive(Debug)]
pub(crate) struct CasePreviewOwnerLease {
    active: Cell<bool>,
    identity: CasePreviewOwnerIdentity,
}

impl CasePreviewOwnerLease {
    fn new(identity: CasePreviewOwnerIdentity) -> Rc<Self> {
        Rc::new(Self {
            active: Cell::new(true),
            identity,
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) fn matches(&self, identity: &CasePreviewOwnerIdentity) -> bool {
        self.is_active() && self.identity == *identity
    }

    pub(crate) fn identity_matches(
        &self,
        scope: &Scope,
        token: SnapshotToken,
        viewer_instance: u64,
        projection_generation: u64,
    ) -> bool {
        self.is_active()
            && self.identity.scope == *scope
            && self.identity.snapshot_token == token
            && self.identity.viewer_instance == viewer_instance
            && self.identity.projection_generation == projection_generation
    }

    pub(crate) fn invalidate(&self) {
        self.active.set(false);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NativePreviewCapture {
    pub(crate) owner: CasePreviewOwnerIdentity,
    pub(crate) lease: Rc<CasePreviewOwnerLease>,
    pub(crate) document: ProjectDoc,
    pub(crate) contours: Vec<boardstudio_core::model::Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) request: CasePreviewRequest,
}

impl NativePreviewCapture {
    pub(crate) fn imported_artifact_request(
        &self,
        id: String,
        source: String,
    ) -> Result<ArtifactRequest, String> {
        if !matches!(self.request, CasePreviewRequest::Imported { .. }) {
            return Err("Authored Case preview cannot consume imported-board bytes".into());
        }
        if id.is_empty() || source.is_empty() {
            return Err(
                "Imported Case preview requires a request ID and verified source bytes".into(),
            );
        }
        Ok(ArtifactRequest::PreviewBoard {
            id,
            source,
            revision: self.owner.accepted_revision,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NativePreviewSnapshot {
    pub(crate) owner: CasePreviewOwnerIdentity,
    pub(crate) lease: Rc<CasePreviewOwnerLease>,
    pub(crate) accepted_document: ProjectDoc,
    pub(crate) contours: Vec<boardstudio_core::model::Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) preview: PcbPreview,
    pub(crate) board_reference: Option<BoardReference>,
}

impl PartialEq for NativePreviewSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && Rc::ptr_eq(&self.lease, &other.lease)
    }
}

impl Eq for NativePreviewSnapshot {}

/// Resolve a renderer-picked preview reference to the unique Part on the
/// preview's accepted board. Decoded mesh ownership stays keyed by model ID;
/// the renderer emits its model reference for picks.
pub(crate) fn part_for_native_preview_reference(
    document: &ProjectDoc,
    board_id: &str,
    preview: &PcbPreview,
    model_reference: &str,
) -> Option<String> {
    if preview.revision != document.revision {
        return None;
    }
    if !preview
        .models
        .iter()
        .any(|model| model.reference == model_reference)
    {
        return None;
    }
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let mut parts = document.parts.iter().filter(|part| {
        part.reference == model_reference && board.part_ids.iter().any(|id| id == &part.id)
    });
    let part = parts.next()?;
    parts.next().is_none().then(|| part.id.clone())
}

/// Validate the producer-owned source/renderer identity before resolving a
/// native-preview pick to an accepted board Part. `model_reference` is the
/// renderer pick ID; renderer-sequence freshness is checked by its scoped
/// signal owner and is a distinct identity domain from the producer lease.
pub(crate) fn native_preview_pick_part_id(
    preview: &NativePreviewSnapshot,
    scope: &Scope,
    snapshot_token: SnapshotToken,
    revision: u64,
    model_reference: &str,
) -> Option<String> {
    let owner = &preview.owner;
    if !preview.lease.matches(owner)
        || owner.scope != *scope
        || owner.snapshot_token != snapshot_token
        || owner.accepted_revision != revision
        || preview.accepted_document.id != scope.document_id
        || preview.accepted_document.revision != revision
    {
        return None;
    }
    part_for_native_preview_reference(
        &preview.accepted_document,
        &owner.scope.board_id,
        &preview.preview,
        model_reference,
    )
}

#[cfg(test)]
mod native_preview_pick_tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{Board, Part, PcbModel, Pose2, Side, Vec2, Vec3};

    fn source() -> (ProjectDoc, PcbPreview) {
        let mut document = ProjectDoc::empty("doc", "Sofle");
        document.revision = 12;
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "left/U1".into(),
            definition_id: "controller".into(),
            reference: "left-U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards.push(Board {
            id: "left".into(),
            name: "Left PCB".into(),
            outline_ids: vec![],
            part_ids: vec!["left/U1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let preview = PcbPreview {
            revision: 12,
            thickness: 1.6,
            contours: vec![],
            surfaces: vec![],
            holes: vec![],
            models: vec![PcbModel {
                id: "left-U1:0".into(),
                reference: "left-U1".into(),
                path: "models/mcu.step".into(),
                pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                side: Side::Front,
                offset: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            }],
            diagnostics: vec![],
        };
        (document, preview)
    }

    #[test]
    fn picked_renderer_reference_resolves_to_board_part_and_not_asset_or_mesh_id() {
        let (document, preview) = source();
        assert_eq!(
            part_for_native_preview_reference(&document, "left", &preview, "left-U1"),
            Some("left/U1".into())
        );
        assert!(
            part_for_native_preview_reference(&document, "left", &preview, "left-U1:0").is_none(),
            "renderer picks carry references, not model IDs or asset IDs"
        );
        let mut repeated_model_reference = preview.clone();
        repeated_model_reference.models.push(PcbModel {
            id: "left-U1:1".into(),
            ..repeated_model_reference.models[0].clone()
        });
        assert_eq!(
            part_for_native_preview_reference(
                &document,
                "left",
                &repeated_model_reference,
                "left-U1"
            ),
            Some("left/U1".into()),
            "a component may have multiple rendered models for one reference"
        );
        let mut duplicate_reference = document.clone();
        duplicate_reference.parts.push(Part {
            id: "left/other".into(),
            ..duplicate_reference.parts[0].clone()
        });
        duplicate_reference.boards[0]
            .part_ids
            .push("left/other".into());
        assert!(
            part_for_native_preview_reference(&duplicate_reference, "left", &preview, "left-U1")
                .is_none(),
            "ambiguous board references must not select an arbitrary Part"
        );
    }

    #[test]
    fn native_pick_requires_the_current_preview_owner_source_identity() {
        let (document, board_preview) = source();
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: document.id.clone(),
            board_id: "left".into(),
            instance_id: Some("left-half".into()),
        };
        let owner = CasePreviewOwnerIdentity {
            scope: scope.clone(),
            snapshot_token: SnapshotToken(43),
            accepted_revision: 12,
            accepted_scene_identity: 1,
            viewer_instance: 4,
            projection_generation: 5,
            batch_generation: 6,
            core_executor_epoch: 7,
            core_worker_identity: 8,
            request_token: "request-9".into(),
        };
        let lease = CasePreviewOwnerLease::new(owner.clone());
        let preview = NativePreviewSnapshot {
            owner,
            lease,
            accepted_document: document,
            contours: vec![],
            path_assets: BTreeMap::new(),
            preview: board_preview,
            board_reference: None,
        };

        assert_eq!(
            native_preview_pick_part_id(&preview, &scope, SnapshotToken(43), 12, "left-U1"),
            Some("left/U1".into())
        );
        let mut stale_scope = scope.clone();
        stale_scope.instance_id = Some("right-half".into());
        assert!(
            native_preview_pick_part_id(&preview, &stale_scope, SnapshotToken(43), 12, "left-U1")
                .is_none()
        );
        assert!(
            native_preview_pick_part_id(&preview, &scope, SnapshotToken(44), 12, "left-U1")
                .is_none()
        );
        assert!(
            native_preview_pick_part_id(&preview, &scope, SnapshotToken(43), 13, "left-U1")
                .is_none()
        );
        assert!(
            native_preview_pick_part_id(&preview, &scope, SnapshotToken(43), 12, "another-model")
                .is_none()
        );
    }
}

/// One owner for pending, accepted and failed native preview state.
#[derive(Default)]
pub(crate) struct NativePreviewState {
    pub(crate) published: Option<Rc<NativePreviewSnapshot>>,
    pub(crate) pending: Option<(CasePreviewOwnerIdentity, Rc<CasePreviewOwnerLease>)>,
    pub(crate) error: Option<(CasePreviewOwnerIdentity, String)>,
    pub(crate) generation: u64,
}

impl NativePreviewState {
    pub(crate) fn begin(
        &mut self,
        owner: CasePreviewOwnerIdentity,
        lease: Rc<CasePreviewOwnerLease>,
    ) {
        self.retire_leases();
        self.pending = Some((owner, lease));
    }

    pub(crate) fn owns(&self, owner: &CasePreviewOwnerIdentity) -> bool {
        self.published
            .as_ref()
            .is_some_and(|preview| preview.lease.matches(owner))
            || self
                .pending
                .as_ref()
                .is_some_and(|(pending, lease)| pending == owner && lease.matches(owner))
    }

    pub(crate) fn is_stale(&self, is_current: impl Fn(&CasePreviewOwnerIdentity) -> bool) -> bool {
        self.published
            .as_ref()
            .is_some_and(|preview| !is_current(&preview.owner))
            || self
                .pending
                .as_ref()
                .is_some_and(|(owner, _)| !is_current(owner))
            || self
                .error
                .as_ref()
                .is_some_and(|(owner, _)| !is_current(owner))
    }

    pub(crate) fn cancel(&mut self) {
        self.retire_leases();
        self.generation = self.generation.saturating_add(1);
    }

    fn retire_leases(&mut self) {
        if let Some(preview) = self.published.take() {
            preview.lease.invalidate();
        }
        if let Some((_, lease)) = self.pending.take() {
            lease.invalidate();
        }
        self.error.take();
    }

    pub(crate) fn publish(&mut self, preview: NativePreviewSnapshot) -> Result<(), String> {
        if !self.pending.as_ref().is_some_and(|(owner, lease)| {
            owner == &preview.owner && lease.matches(owner) && Rc::ptr_eq(lease, &preview.lease)
        }) || self.generation != preview.owner.projection_generation
        {
            preview.lease.invalidate();
            return Err("The Case preview owner was cancelled before publication".into());
        }
        self.published = Some(Rc::new(preview));
        // The accepted snapshot now owns the same lease as the retired pending slot.
        self.pending.take();
        self.error.take();
        Ok(())
    }
}

/// Capture the physical document and contours used by the native preview from
/// the accepted source projection before mechanical Case preparation/CAD.
pub(crate) fn capture_native_preview(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    projection_generation: u64,
    batch_generation: u64,
    core_executor_epoch: u64,
    core_worker_identity: usize,
    request_token: String,
) -> Result<NativePreviewCapture, String> {
    if request_token.is_empty() {
        return Err("Case preview request token must not be empty".into());
    }
    let document = captured_case_document(snapshot, scope)
        .map_err(|error| format!("Could not capture accepted physical Case document: {error:?}"))?;
    if document.revision != snapshot.document.revision {
        return Err("Physical Case projection changed the accepted revision".into());
    }
    if document.physical_instance_id.as_deref() != scope.instance_id.as_deref() {
        return Err("Physical Case projection selected another instance".into());
    }
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
        .ok_or_else(|| "Selected board is unavailable in the accepted Case document".to_owned())?;
    let scene = captured_case_scene(snapshot, scope)
        .map_err(|error| format!("Could not capture accepted physical Case contours: {error:?}"))?;
    let contours = scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default();
    let path_assets = preview_model_paths(&document);
    let owner = CasePreviewOwnerIdentity::capture(
        snapshot,
        scope,
        projection_generation,
        batch_generation,
        core_executor_epoch,
        core_worker_identity,
        request_token.clone(),
    );
    let mut references = document
        .board_references
        .iter()
        .filter(|reference| reference.board_id == scope.board_id && reference.enabled);
    let request = if let Some(reference) = references.next() {
        if references.next().is_some() {
            return Err("Selected board has multiple enabled imported-board sources".into());
        }
        let mut assets = document
            .assets
            .iter()
            .filter(|asset| asset.id == reference.asset_id);
        let asset = assets
            .next()
            .filter(|asset| !asset.sha256.is_empty())
            .cloned()
            .ok_or_else(|| {
                "Imported board asset is unavailable in the accepted Case document".to_owned()
            })?;
        if assets.next().is_some() {
            return Err(
                "Imported board asset identity is ambiguous in the accepted Case document".into(),
            );
        }
        CasePreviewRequest::Imported {
            reference: reference.clone(),
            asset,
        }
    } else {
        CasePreviewRequest::Authored(Box::new(PrepareExportRequest {
            snapshot_token: request_token,
            expected_revision: document.revision,
            document: document.clone(),
            target: ExportTarget::Board {
                board_id: board.id.clone(),
            },
            contours: contours.clone(),
            model_paths: path_assets.clone(),
        }))
    };

    let lease = CasePreviewOwnerLease::new(owner.clone());
    Ok(NativePreviewCapture {
        owner,
        lease,
        document,
        contours,
        path_assets,
        request,
    })
}

pub(crate) fn accept_native_preview(
    capture: NativePreviewCapture,
    preview: PcbPreview,
) -> Result<NativePreviewSnapshot, String> {
    if preview.revision != capture.owner.accepted_revision {
        return Err("Core returned a Case board preview for another revision".into());
    }
    if capture.document.revision != capture.owner.accepted_revision {
        return Err("Case preview request identity does not match its accepted source".into());
    }
    let board_reference = match &capture.request {
        CasePreviewRequest::Authored(request) => {
            if request.expected_revision != capture.owner.accepted_revision
                || request.snapshot_token != capture.owner.request_token
            {
                return Err(
                    "Case preview request identity does not match its accepted source".into(),
                );
            }
            None
        }
        CasePreviewRequest::Imported { reference, .. } => Some(reference.clone()),
    };
    let contours = if board_reference.is_some() {
        preview.contours.clone()
    } else {
        capture.contours
    };
    Ok(NativePreviewSnapshot {
        owner: capture.owner,
        lease: capture.lease,
        accepted_document: capture.document,
        contours,
        path_assets: capture.path_assets,
        preview,
        board_reference,
    })
}

pub(crate) fn same_core_executor(
    captured_epoch: u64,
    current_epoch: u64,
    captured_worker_identity: usize,
    current_worker_identity: usize,
) -> bool {
    captured_epoch == current_epoch && captured_worker_identity == current_worker_identity
}

fn model_extension(filename: &str) -> Option<&'static str> {
    match filename.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "step" => Some("step"),
        "stp" => Some("stp"),
        "stl" => Some("stl"),
        "wrl" => Some("wrl"),
        _ => None,
    }
}

/// Build the existing export-relative path table for preview model resolution.
/// Layout and Case share this metadata rule; their accepted source projections
/// and freshness owners remain separate.
pub(crate) fn preview_model_paths(document: &ProjectDoc) -> BTreeMap<String, String> {
    let mut path_assets = document
        .assets
        .iter()
        .filter_map(|asset| {
            let extension = model_extension(&asset.name)?;
            (!asset.id.is_empty() && !asset.sha256.is_empty()).then(|| {
                (
                    asset.id.clone(),
                    format!("models/{}.{extension}", asset.sha256),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let document_asset_ids = document
        .assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<BTreeSet<_>>();
    for (asset_id, path) in crate::bundled_models::preview_model_paths() {
        if !document_asset_ids.contains(asset_id) {
            path_assets.insert(asset_id.to_owned(), path.to_owned());
        }
    }
    path_assets
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{
        Asset, Board, BoardContours, BoardReference, Contour, HardwareConfiguration,
        PhysicalBoardInstance, Pose2, Readiness, SceneDelta, Vec2,
    };
    use std::sync::Arc;

    fn snapshot(flipped: bool, imported: bool) -> (AcceptedSnapshot, Scope) {
        let mut document = ProjectDoc::empty("doc", "Layered Sofle");
        document.revision = 12;
        document.boards.push(Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.assets.push(Asset {
            id: "model-1".into(),
            name: "controller.STEP".into(),
            media_type: "model/step".into(),
            sha256: "ab".repeat(32),
            license: None,
            source: None,
        });
        document.hardware = Some(HardwareConfiguration {
            instances: vec![PhysicalBoardInstance {
                id: "left-half".into(),
                name: "Left half".into(),
                board_id: "left".into(),
                half: "left".into(),
                role: "controller".into(),
                flipped,
                controller_part_id: None,
                mechanical: None,
                construction_linked: true,
            }],
            ..HardwareConfiguration::default()
        });
        if imported {
            document.board_references.push(BoardReference {
                id: "left-reference".into(),
                board_id: "left".into(),
                asset_id: "board-source".into(),
                enabled: true,
                pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                elevation: 0.0,
                model_assets: BTreeMap::new(),
            });
        }
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: "left".into(),
            instance_id: Some("left-half".into()),
        };
        let scene = SceneDelta {
            module_scenes: vec![],
            revision: 12,
            transaction_id: String::new(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "left".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: 1.0, y: 0.0 },
                        Vec2 { x: 2.0, y: 0.0 },
                        Vec2 { x: 2.0, y: 1.0 },
                    ],
                    hole: false,
                }],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        };
        (
            AcceptedSnapshot {
                token: SnapshotToken(43),
                session_epoch: scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(scene),
            },
            scope,
        )
    }

    #[test]
    fn native_preview_captures_accepted_physical_contours_without_case_readiness_or_cad() {
        let (snapshot, scope) = snapshot(false, false);
        let captured = capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-7".into())
            .expect("native preview source is available before Case CAD");
        let CasePreviewRequest::Authored(request) = &captured.request else {
            panic!("an authored physical board must use the existing prepared-preview pipeline");
        };
        assert!(
            captured
                .imported_artifact_request("wrong-source".into(), "bytes".into())
                .is_err()
        );
        assert_eq!(request.expected_revision, 12);
        assert_eq!(request.snapshot_token, "preview-7");
        assert_eq!(
            request.target,
            ExportTarget::Board {
                board_id: "left".into()
            }
        );
        assert_eq!(request.contours, captured.contours);
        assert_eq!(captured.contours[0].points[0].x, 1.0);
        assert_eq!(request.document.case_bodies.len(), 0);
        assert_eq!(
            captured.path_assets["model-1"],
            format!("models/{}.step", "ab".repeat(32))
        );
        assert_eq!(
            captured.path_assets["bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"],
            "assets/ergogen-models/model-dd931656985824ce.stp"
        );
        assert_eq!(
            request.model_paths["bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"],
            "assets/ergogen-models/model-dd931656985824ce.stp"
        );
        assert_eq!(captured.owner.snapshot_token, SnapshotToken(43));
    }

    #[test]
    fn native_preview_does_not_substitute_packaged_paths_for_document_owned_ids() {
        let (mut snapshot, scope) = snapshot(false, false);
        Arc::make_mut(&mut snapshot.document).assets.push(Asset {
            id: "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp".into(),
            name: "unrecognized-model.bin".into(),
            media_type: "application/octet-stream".into(),
            sha256: String::new(),
            license: None,
            source: None,
        });

        let captured = capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-10".into())
            .expect("native preview source remains available");

        assert!(
            !captured
                .path_assets
                .contains_key("bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"),
            "a document-owned but unusable asset must not silently resolve to packaged bytes"
        );
    }

    #[test]
    fn native_preview_preserves_document_path_precedence_for_packaged_model_ids() {
        let (mut snapshot, scope) = snapshot(false, false);
        Arc::make_mut(&mut snapshot.document).assets.push(Asset {
            id: "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp".into(),
            name: "local-switch.STEP".into(),
            media_type: "model/step".into(),
            sha256: "cd".repeat(32),
            license: None,
            source: None,
        });

        let captured = capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-11".into())
            .expect("native preview source remains available");

        assert_eq!(
            captured.path_assets["bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"],
            format!("models/{}.step", "cd".repeat(32))
        );
    }

    #[test]
    fn native_preview_uses_the_existing_flipped_physical_projection() {
        let (snapshot, scope) = snapshot(true, true);
        let captured = capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-8".into())
            .expect("flipped physical source remains previewable");
        assert_eq!(
            captured.document.physical_instance_id.as_deref(),
            Some("left-half")
        );
        assert_eq!(captured.contours[0].points[0].x, -2.0);
        assert!(
            !captured
                .document
                .board_references
                .iter()
                .any(|item| item.enabled)
        );
    }

    // Reduced from the paired r22 routed-folder qualification: an enabled
    // routed source survives the selected nonflipped physical projection.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn physical_case_keeps_a_preview_owner_for_an_enabled_routed_board() {
        let (mut accepted, scope) = snapshot(false, false);
        let document = Arc::make_mut(&mut accepted.document);
        document.revision = 22;
        document.hardware.as_mut().unwrap().instances[0].construction_linked = false;
        document.board_references = serde_json::from_str(
            r#"[{"id":"c976e201-ecaf-4ec2-803e-aaa85c250a4d","boardId":"left","assetId":"27a7ecfd-570a-4e96-84c0-1db32be8e683","enabled":true,"pose":{"at":{"x":4.2,"y":-3.1},"rotation":15.0},"elevation":2.5,"modelAssets":{"${KIPRJMOD}/models/31dd9ab0a892e02f43c13f07c3d78f83dbe64390178e64b2f1f6df4849935e5b.stp":"157020b7-1608-4b58-bf00-0aefcb6dd903","${KIPRJMOD}/models/42d7eb2588578a42bfc08d91a8f27e28db37b9c76ff62395bb9bb67a934b23b3.step":"c3a7f7fc-dfa4-4f02-96d4-18ae794e6bf8","${KIPRJMOD}/models/54889afbae4bb3f7deca2a39e9031752845b94e647c1ebbcac7ba1a5261e7e0e.step":"22492d28-81a7-4a1f-8549-b9f2438174da","${KIPRJMOD}/models/77fcad08b9bceca821b32733443e7bc9157230fb6340aa390797f56d5694182f.step":"fd84413b-f8f0-4af5-97ee-95899ad9d6fa","${KIPRJMOD}/models/abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722.step":"2be1144d-0e37-4fab-93d7-9f02568cd688","${KIPRJMOD}/models/ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4.stp":"e42a7d7d-49f5-4e33-aaad-67ef79d3939d"}}]"#,
        )
        .unwrap();
        document.assets = serde_json::from_str(
            r#"[{"id":"27a7ecfd-570a-4e96-84c0-1db32be8e683","name":"Left_PCB.kicad_pcb","mediaType":"application/x-kicad_pcb","sha256":"a0b71c10f2f77742d113e0688889df5384e2f4c6244f5521da3eda87d60ac487"},{"id":"157020b7-1608-4b58-bf00-0aefcb6dd903","name":"31dd9ab0a892e02f43c13f07c3d78f83dbe64390178e64b2f1f6df4849935e5b.stp","mediaType":"model/step","sha256":"31dd9ab0a892e02f43c13f07c3d78f83dbe64390178e64b2f1f6df4849935e5b"},{"id":"c3a7f7fc-dfa4-4f02-96d4-18ae794e6bf8","name":"42d7eb2588578a42bfc08d91a8f27e28db37b9c76ff62395bb9bb67a934b23b3.step","mediaType":"model/step","sha256":"42d7eb2588578a42bfc08d91a8f27e28db37b9c76ff62395bb9bb67a934b23b3"},{"id":"22492d28-81a7-4a1f-8549-b9f2438174da","name":"54889afbae4bb3f7deca2a39e9031752845b94e647c1ebbcac7ba1a5261e7e0e.step","mediaType":"model/step","sha256":"54889afbae4bb3f7deca2a39e9031752845b94e647c1ebbcac7ba1a5261e7e0e"},{"id":"fd84413b-f8f0-4af5-97ee-95899ad9d6fa","name":"77fcad08b9bceca821b32733443e7bc9157230fb6340aa390797f56d5694182f.step","mediaType":"model/step","sha256":"77fcad08b9bceca821b32733443e7bc9157230fb6340aa390797f56d5694182f"},{"id":"2be1144d-0e37-4fab-93d7-9f02568cd688","name":"abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722.step","mediaType":"model/step","sha256":"abe50072b241c6ce69f8e79d787644ab3ff54d1ea09d74ee8025fb2bf4228722"},{"id":"e42a7d7d-49f5-4e33-aaad-67ef79d3939d","name":"ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4.stp","mediaType":"model/step","sha256":"ddf0fb3a776faa6105303efb98e39645e044c43b3ac03cbfe75d8eda28e297f4"}]"#,
        )
        .unwrap();
        Arc::make_mut(&mut accepted.scene).revision = 22;
        let physical_document = captured_case_document(&accepted, &scope).unwrap();
        let physical_scene = captured_case_scene(&accepted, &scope).unwrap();
        assert_eq!(physical_document.physical_instance_id, scope.instance_id);
        assert!(physical_document.mechanical.is_none());
        assert!(physical_document.case_bodies.is_empty());
        let reference = physical_document.board_references[0].clone();
        assert!(reference.enabled);
        assert_eq!(reference.board_id, scope.board_id);

        // Establish the actual existing imported producer choice for these same
        // accepted physical inputs before exercising the missing Case owner.
        let physical = AcceptedSnapshot {
            document: Arc::new(physical_document),
            scene: Arc::new(physical_scene),
            ..accepted.clone()
        };
        let imported = crate::layout_viewer_source::LayoutSourceCapture::capture(
            &physical,
            &scope,
            4,
            "routed-case-22".into(),
            preview_model_paths(&physical.document),
        )
        .expect("the saved physical reference has an existing imported source owner");
        assert!(matches!(
            &imported.request,
            crate::layout_viewer_source::LayoutPreviewRequest::Imported { reference: selected, asset }
                if selected == &reference && asset.id == reference.asset_id
        ));
        assert!(matches!(
            imported.artifact_request("routed-case-22".into(), Some("verified KiCad bytes".into())),
            Ok(boardstudio_core::model::ArtifactRequest::PreviewBoard { revision: 22, .. })
        ));

        let capture =
            capture_native_preview(&accepted, &scope, 4, 9, 3, 7, "routed-case-22".into()).expect(
                "Case must retain an imported preview owner instead of leaving the viewport empty",
            );
        assert!(matches!(&capture.request,
            CasePreviewRequest::Imported { reference: selected, asset }
                if selected == &reference && asset.id == reference.asset_id
        ));
        let request = capture
            .imported_artifact_request(
                "routed-case-core-22".into(),
                "captured source marker".into(),
            )
            .unwrap();
        assert!(matches!(request,
            ArtifactRequest::PreviewBoard { id, source, revision: 22 }
                if id == "routed-case-core-22" && source == "captured source marker"
        ));
        assert!(
            capture
                .imported_artifact_request("id".into(), String::new())
                .is_err()
        );
        assert_eq!(capture.owner.scope, scope);
        assert_eq!(capture.owner.snapshot_token, accepted.token);
        assert_eq!(capture.owner.accepted_revision, 22);
        assert_eq!(capture.document.board_references, vec![reference.clone()]);
        assert_eq!(capture.document.physical_instance_id, scope.instance_id);
        let imported_contours = vec![Contour {
            points: vec![
                Vec2 { x: 101.0, y: 102.0 },
                Vec2 { x: 111.0, y: 102.0 },
                Vec2 { x: 111.0, y: 112.0 },
            ],
            hole: false,
        }];
        let published = accept_native_preview(
            capture,
            PcbPreview {
                revision: 22,
                thickness: 1.6,
                contours: imported_contours.clone(),
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            },
        )
        .unwrap();
        assert_eq!(published.board_reference.as_ref(), Some(&reference));
        assert_eq!(
            published.contours, imported_contours,
            "routed contours, not authored substitute"
        );
        let (path, asset_id) = reference.model_assets.iter().next().unwrap();
        assert!(matches!(
            crate::model_delivery::select_model_asset(path, published.board_reference.as_ref(),
                &BTreeMap::new(), &published.accepted_document.assets, |_| None),
            crate::model_delivery::AssetSelection::Archived(asset) if &asset.id == asset_id
        ));
    }

    #[test]
    fn imported_case_preview_rejects_missing_accepted_asset() {
        let (snapshot, scope) = snapshot(false, true);
        assert!(
            capture_native_preview(&snapshot, &scope, 1, 1, 3, 7, "preview".into())
                .expect_err("imported board requires its accepted source asset")
                .contains("Imported board asset is unavailable")
        );
    }

    #[test]
    fn preview_result_must_match_the_captured_accepted_revision() {
        let (snapshot, scope) = snapshot(false, false);
        let captured =
            capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-7".into()).unwrap();
        let preview = PcbPreview {
            revision: 11,
            thickness: 1.6,
            contours: vec![],
            surfaces: vec![],
            holes: vec![],
            models: vec![],
            diagnostics: vec![],
        };
        assert!(
            accept_native_preview(captured, preview)
                .expect_err("stale preview result")
                .contains("another revision")
        );
    }

    #[test]
    fn pending_scope_away_and_back_cannot_publish_the_old_completion() {
        let (snapshot, scope) = snapshot(false, false);
        let capture =
            capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "pending".into()).unwrap();
        let mut state = NativePreviewState {
            generation: 4,
            ..Default::default()
        };
        state.begin(capture.owner.clone(), capture.lease.clone());
        let mut away = scope.clone();
        away.instance_id = Some("another-instance".into());
        if state.is_stale(|owner| owner.scope == away) {
            state.cancel();
        }
        // Returning to the same source cannot revive the old request's lease.
        assert_eq!(capture.owner.scope, scope);
        assert!(
            !state.owns(&capture.owner),
            "scope loss must retire a pending-only owner"
        );
        assert!(!capture.lease.is_active());
        assert!(state.generation > capture.owner.projection_generation);
    }

    #[test]
    fn cancellation_rejects_late_completion_and_preserves_replacement_owner() {
        let (snapshot, scope) = snapshot(false, false);
        let old = capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "old".into()).unwrap();
        let mut state = NativePreviewState {
            generation: 4,
            ..Default::default()
        };
        state.begin(old.owner.clone(), old.lease.clone());
        state.cancel();
        let replacement =
            capture_native_preview(&snapshot, &scope, state.generation, 10, 3, 7, "new".into())
                .unwrap();
        state.begin(replacement.owner.clone(), replacement.lease.clone());
        let old_result = accept_native_preview(
            old,
            PcbPreview {
                revision: 12,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            },
        )
        .unwrap();
        assert!(state.publish(old_result).is_err());
        assert!(state.published.is_none());
        assert!(state.owns(&replacement.owner));
        assert!(replacement.lease.is_active());
    }

    #[test]
    fn successful_publication_keeps_the_preview_visible_after_pending_clears() {
        let (snapshot, scope) = snapshot(false, false);
        let capture =
            capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-9".into()).unwrap();
        let owner = capture.owner.clone();
        let weak = Rc::downgrade(&capture.lease);
        let mut state = NativePreviewState {
            generation: 4,
            ..Default::default()
        };
        state.begin(owner.clone(), capture.lease.clone());
        let preview = accept_native_preview(
            capture,
            PcbPreview {
                revision: owner.accepted_revision,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            },
        )
        .unwrap();
        state.publish(preview).unwrap();

        assert!(state.pending.is_none());
        let visible = state
            .published
            .as_ref()
            .filter(|preview| preview.lease.matches(&owner));
        assert!(
            visible.is_some(),
            "success must remain visible after pending ownership transfers"
        );
        assert!(weak.upgrade().unwrap().is_active());
        state.published.take();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn preview_owner_lease_tracks_liveness_and_exact_source_identity() {
        let (snapshot, scope) = snapshot(false, false);
        let captured =
            capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-9".into()).unwrap();
        let weak = Rc::downgrade(&captured.lease);
        assert!(captured.lease.matches(&captured.owner));
        assert!(
            captured
                .lease
                .identity_matches(&scope, SnapshotToken(43), 4, 4)
        );

        let mut replaced = captured.owner.clone();
        replaced.snapshot_token = SnapshotToken(44);
        assert!(!captured.lease.matches(&replaced));
        assert!(
            !captured
                .lease
                .identity_matches(&scope, SnapshotToken(44), 4, 4)
        );

        captured.lease.invalidate();
        assert!(!captured.lease.is_active());
        assert!(!captured.lease.matches(&captured.owner));
        drop(captured);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn preview_source_identity_rejects_same_revision_snapshot_aba() {
        let (snapshot, scope) = snapshot(false, false);
        let captured =
            capture_native_preview(&snapshot, &scope, 4, 9, 3, 7, "preview-9".into()).unwrap();
        let mut replaced = captured.owner.clone();
        replaced.accepted_scene_identity = replaced.accepted_scene_identity.wrapping_add(1);
        assert_eq!(captured.owner.accepted_revision, replaced.accepted_revision);
        assert_eq!(captured.owner.snapshot_token, replaced.snapshot_token);
        assert!(!captured.lease.matches(&replaced));
    }

    #[test]
    fn executor_replacement_and_epoch_reuse_reject_aba() {
        assert!(same_core_executor(4, 4, 0x1000, 0x1000));
        assert!(!same_core_executor(4, 5, 0x1000, 0x1000));
        assert!(!same_core_executor(4, 4, 0x1000, 0x2000));
        assert!(!same_core_executor(4, 5, 0x1000, 0x2000));
    }
}
