//! Page-private source capture and identity for the native Case board preview.
//!
//! This is a render-only accepted projection. It deliberately does not depend on
//! a mechanical `CadScene`, and it leaves imported `BoardReference` previews to
//! the existing Issue07 producer path.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    ArtifactRequest, ExportTarget, PcbPreview, PrepareExportRequest, ProjectDoc,
};
use boardstudio_web::cad_jobs::{captured_case_document, captured_case_scene};
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

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
    pub(crate) request: PrepareExportRequest,
}

#[derive(Clone, Debug)]
pub(crate) struct NativePreviewSnapshot {
    pub(crate) owner: CasePreviewOwnerIdentity,
    pub(crate) lease: Rc<CasePreviewOwnerLease>,
    pub(crate) accepted_document: ProjectDoc,
    pub(crate) contours: Vec<boardstudio_core::model::Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) preview: PcbPreview,
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
    if document
        .board_references
        .iter()
        .any(|reference| reference.board_id == scope.board_id && reference.enabled)
    {
        return Err("Selected board uses the imported-board preview owner".into());
    }

    let scene = captured_case_scene(snapshot, scope)
        .map_err(|error| format!("Could not capture accepted physical Case contours: {error:?}"))?;
    let contours = scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default();
    let path_assets = document
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
    let owner = CasePreviewOwnerIdentity {
        scope: scope.clone(),
        snapshot_token: snapshot.token,
        accepted_revision: snapshot.document.revision,
        accepted_scene_identity: std::sync::Arc::as_ptr(&snapshot.scene) as usize,
        viewer_instance: projection_generation,
        projection_generation,
        batch_generation,
        core_executor_epoch,
        core_worker_identity,
        request_token: request_token.clone(),
    };
    let request = PrepareExportRequest {
        snapshot_token: request_token,
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: board.id.clone(),
        },
        contours: contours.clone(),
        model_paths: path_assets.clone(),
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
    if capture.document.revision != capture.owner.accepted_revision
        || capture.request.expected_revision != capture.owner.accepted_revision
        || capture.request.snapshot_token != capture.owner.request_token
    {
        return Err("Case preview request identity does not match its accepted source".into());
    }
    Ok(NativePreviewSnapshot {
        owner: capture.owner,
        lease: capture.lease,
        accepted_document: capture.document,
        contours: capture.contours,
        path_assets: capture.path_assets,
        preview,
    })
}

pub(crate) fn prepare_artifact(id: String, capture: &NativePreviewCapture) -> ArtifactRequest {
    ArtifactRequest::PreparePreview {
        id,
        request: capture.request.clone(),
    }
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
        assert_eq!(captured.request.expected_revision, 12);
        assert_eq!(captured.request.snapshot_token, "preview-7");
        assert_eq!(
            captured.request.target,
            ExportTarget::Board {
                board_id: "left".into()
            }
        );
        assert_eq!(captured.request.contours, captured.contours);
        assert_eq!(captured.contours[0].points[0].x, 1.0);
        assert_eq!(captured.request.document.case_bodies.len(), 0);
        assert_eq!(
            captured.path_assets["model-1"],
            format!("models/{}.step", "ab".repeat(32))
        );
        assert_eq!(captured.owner.snapshot_token, SnapshotToken(43));
    }

    #[test]
    fn native_preview_uses_the_existing_flipped_physical_projection() {
        let (snapshot, scope) = snapshot(true, false);
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

    #[test]
    fn native_preview_refuses_to_take_imported_board_producer_ownership() {
        let (snapshot, scope) = snapshot(false, true);
        assert!(
            capture_native_preview(&snapshot, &scope, 1, 1, 3, 7, "preview".into())
                .expect_err("imported board belongs to Issue07")
                .contains("imported-board preview owner")
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
