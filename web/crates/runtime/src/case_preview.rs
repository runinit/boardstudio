//! Page-private source capture and identity for the native Case board preview.
//!
//! This is a render-only accepted projection. It deliberately does not depend on
//! a mechanical `CadScene`. Authored and imported boards share the physical Case
//! owner while retaining their existing, distinct Core preview producers.

use crate::model_delivery::{
    PreviewLease, PreviewSlot, PreviewSnapshot, capture_board_preview_request,
};
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::{ArtifactRequest, BoardReference, PcbPreview, ProjectDoc};
use boardstudio_web_host::cad_jobs::{captured_case_document, captured_case_scene};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CasePreviewOwnerIdentity {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,

    pub projection_generation: u64,

    pub request_token: String,
}

impl CasePreviewOwnerIdentity {
    pub fn capture(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        projection_generation: u64,

        request_token: String,
    ) -> Self {
        Self {
            scope: scope.clone(),
            snapshot_token: snapshot.token,

            projection_generation,

            request_token,
        }
    }
}

pub use crate::model_delivery::BoardPreviewRequest as CasePreviewRequest;
pub type CasePreviewOwnerLease = PreviewLease<CasePreviewOwnerIdentity>;

#[derive(Clone, Debug)]
pub struct NativePreviewCapture {
    pub owner: CasePreviewOwnerIdentity,
    pub lease: Rc<CasePreviewOwnerLease>,
    pub document: ProjectDoc,
    pub contours: Vec<boardstudio_model::model::Contour>,
    pub path_assets: BTreeMap<String, String>,
    pub request: CasePreviewRequest,
}

impl NativePreviewCapture {
    pub fn imported_artifact_request(
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
            revision: self.document.revision,
        })
    }
}

#[derive(Clone, Debug)]
pub struct NativePreviewSnapshot {
    pub owner: CasePreviewOwnerIdentity,
    pub lease: Rc<CasePreviewOwnerLease>,
    pub accepted_document: ProjectDoc,
    pub contours: Vec<boardstudio_model::model::Contour>,
    pub path_assets: BTreeMap<String, String>,
    pub preview: PcbPreview,
    pub board_reference: Option<BoardReference>,
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
pub fn part_for_native_preview_reference(
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
pub fn native_preview_pick_part_id(
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
    use boardstudio_model::model::{Board, Part, PcbModel, Pose2, Side, Vec2, Vec3};

    pub(super) fn source() -> (ProjectDoc, PcbPreview) {
        let mut document = ProjectDoc::empty("doc", "Sofle");
        document.revision = 12;
        document.parts.push(Part {
            anchor: None,
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
                pose: document.parts[0].pose,
                side: Side::Front,
                offset: Vec3::default(),
                rotation: Vec3::default(),
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
            anchor: None,
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
    fn native_preview_picks_and_publication_require_the_current_source_owner() {
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

            projection_generation: 5,

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
        for (scope, token, revision, reference) in [
            (&stale_scope, 43, 12, "left-U1"),
            (&scope, 44, 12, "left-U1"),
            (&scope, 43, 13, "left-U1"),
            (&scope, 43, 12, "another-model"),
        ] {
            assert!(
                native_preview_pick_part_id(
                    &preview,
                    scope,
                    SnapshotToken(token),
                    revision,
                    reference
                )
                .is_none()
            );
        }

        let state = NativePreviewState::default();
        state.set_generation(preview.owner.projection_generation);
        state.begin(preview.lease.clone());
        state.begin(CasePreviewOwnerLease::new(preview.owner.clone()));
        assert!(state.publish(preview.clone(), 5, "cancelled").is_err());
        assert!(!preview.lease.is_active());
        state.fail(&preview.lease, "late error".into());
        assert!(state.published().is_none() && state.current_error(|_| true).is_none());
        assert!(state.owns(&preview.owner));
    }
}

impl PreviewSnapshot<CasePreviewOwnerIdentity> for NativePreviewSnapshot {
    fn source(
        &self,
    ) -> (
        &CasePreviewOwnerIdentity,
        &Rc<PreviewLease<CasePreviewOwnerIdentity>>,
    ) {
        (&self.owner, &self.lease)
    }
}

pub type NativePreviewState = PreviewSlot<CasePreviewOwnerIdentity, NativePreviewSnapshot>;

/// Capture the physical document and contours used by the native preview from
/// the accepted source projection before mechanical Case preparation/CAD.
pub fn capture_native_preview(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    projection_generation: u64,

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
    if !document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id)
    {
        return Err("Selected board is unavailable in the accepted Case document".into());
    }
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
        request_token.clone(),
    );
    let request = capture_board_preview_request(
        &document,
        &scope.board_id,
        &contours,
        &path_assets,
        request_token,
        "accepted Case document",
    )?;

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

pub fn accept_native_preview(
    capture: NativePreviewCapture,
    preview: PcbPreview,
) -> Result<NativePreviewSnapshot, String> {
    if preview.revision != capture.document.revision {
        return Err("Core returned a Case board preview for another revision".into());
    }
    let board_reference = match &capture.request {
        CasePreviewRequest::Authored(request) => {
            if request.expected_revision != capture.document.revision
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
pub fn preview_model_paths(document: &ProjectDoc) -> BTreeMap<String, String> {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod capture_error_tests {
    use super::*;
    use boardstudio_application::Event;

    #[test]
    fn missing_board_source_only_reports_to_the_captured_owner() {
        let runtime = crate::runtime::Runtime::new();
        let (mut document, _) = super::native_preview_pick_tests::source();
        document.parts.clear();
        document.boards[0].part_ids.clear();
        document.board_references.push(BoardReference {
            id: "reference".into(),
            board_id: document.boards[0].id.clone(),
            asset_id: "missing-asset".into(),
            enabled: true,
            pose: boardstudio_model::model::Pose2 {
                at: Default::default(),
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: Default::default(),
        });
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let snapshot = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let error = capture_native_preview(&snapshot, &scope, 1, "capture".into())
            .err()
            .unwrap();
        assert!(
            error.contains("Imported board asset is unavailable"),
            "{error}"
        );
        let owner = CasePreviewOwnerIdentity::capture(&snapshot, &scope, 1, "capture".into());
        let state = NativePreviewState::default();
        state.fail_before_begin(owner, error.clone());
        assert_eq!(
            state.current_error(|owner| owner.scope == scope),
            Some(error)
        );
        assert!(state.pending().is_none() && state.published().is_none());
        assert_eq!(
            state.current_error(|owner| owner.scope.board_id == "another-board"),
            None
        );
    }
}
