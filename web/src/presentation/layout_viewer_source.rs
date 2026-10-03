//! Private canonical Layout preview source capture and pick mapping.
//!
//! This module deliberately does not own async jobs, asset storage, model decoding,
//! or a renderer. Runtime and the shared viewer supply those owners and recheck this
//! captured identity after each asynchronous boundary.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    ArtifactRequest, Asset, BoardReference, Contour, ExportTarget, PcbPreview,
    PrepareExportRequest, ProjectDoc,
};
use std::{cell::Cell, rc::Rc};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutSourceIdentity {
    /// Keep the complete active scope, including a retained physical instance.
    /// The instance participates in freshness only; it never projects geometry.
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) accepted_revision: u64,
    pub(crate) accepted_document_identity: usize,
    pub(crate) accepted_scene_identity: usize,
    pub(crate) source_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LayoutPreviewRequest {
    Authored(Box<PrepareExportRequest>),
    Imported {
        reference: BoardReference,
        asset: Asset,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutSourceCapture {
    pub(crate) owner: LayoutSourceIdentity,
    pub(crate) lease: Rc<LayoutSourceLease>,
    pub(crate) document: Arc<ProjectDoc>,
    pub(crate) contours: Vec<Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) request: LayoutPreviewRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutPreviewSnapshot {
    pub(crate) owner: LayoutSourceIdentity,
    pub(crate) lease: Rc<LayoutSourceLease>,
    pub(crate) document: Arc<ProjectDoc>,
    pub(crate) contours: Vec<Contour>,
    pub(crate) path_assets: BTreeMap<String, String>,
    pub(crate) board_reference: Option<BoardReference>,
    pub(crate) preview: PcbPreview,
}

#[derive(Debug)]
pub(crate) struct LayoutSourceLease {
    active: Cell<bool>,
    identity: LayoutSourceIdentity,
}

impl LayoutSourceLease {
    fn new(identity: LayoutSourceIdentity) -> Rc<Self> {
        Rc::new(Self {
            active: Cell::new(true),
            identity,
        })
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) fn matches(&self, identity: &LayoutSourceIdentity) -> bool {
        self.is_active() && self.identity == *identity
    }

    pub(crate) fn invalidate(&self) {
        self.active.set(false);
    }
}

impl PartialEq for LayoutSourceLease {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[derive(Default)]
pub(crate) struct LayoutPreviewState {
    pub(crate) published: Option<Rc<LayoutPreviewSnapshot>>,
    pub(crate) pending: Option<(LayoutSourceIdentity, Rc<LayoutSourceLease>)>,
    pub(crate) error: Option<(LayoutSourceIdentity, String)>,
    source_generation: u64,
}

impl LayoutPreviewState {
    pub(crate) fn next_generation(&mut self) -> Result<u64, String> {
        self.retire();
        let generation = self
            .source_generation
            .checked_add(1)
            .ok_or_else(|| "Layout source generation exhausted".to_owned())?;
        self.source_generation = generation;
        Ok(generation)
    }

    pub(crate) fn begin(&mut self, capture: &LayoutSourceCapture) {
        self.retire();
        self.pending = Some((capture.owner.clone(), capture.lease.clone()));
    }

    pub(crate) fn owns(&self, owner: &LayoutSourceIdentity) -> bool {
        self.published
            .as_ref()
            .is_some_and(|source| source.lease.matches(owner))
            || self
                .pending
                .as_ref()
                .is_some_and(|(pending, lease)| pending == owner && lease.matches(owner))
    }

    pub(crate) fn publish(&mut self, source: LayoutPreviewSnapshot) -> Result<(), String> {
        if !self.owns(&source.owner)
            || !self.pending.as_ref().is_some_and(|(owner, lease)| {
                owner == &source.owner && Rc::ptr_eq(lease, &source.lease)
            })
        {
            source.lease.invalidate();
            return Err("Layout source owner changed before preview publication".into());
        }
        self.pending.take();
        self.error.take();
        self.published = Some(Rc::new(source));
        Ok(())
    }

    pub(crate) fn retire_generation(&mut self, generation: u64) -> bool {
        if generation == self.source_generation {
            self.retire();
            self.source_generation = self.source_generation.saturating_add(1);
            true
        } else {
            false
        }
    }

    pub(crate) fn retire_unless_request_matches(
        &mut self,
        expected: Option<(&Scope, SnapshotToken, u64)>,
    ) -> bool {
        let current = self
            .published
            .as_ref()
            .map(|source| &source.owner)
            .or_else(|| self.pending.as_ref().map(|(owner, _)| owner))
            .or_else(|| self.error.as_ref().map(|(owner, _)| owner));
        let Some(current) = current else {
            return false;
        };
        if expected.is_some_and(|(scope, token, revision)| {
            current.scope == *scope
                && current.snapshot_token == token
                && current.accepted_revision == revision
        }) {
            return false;
        }
        self.retire_generation(current.source_generation)
    }

    pub(crate) fn fail(&mut self, owner: LayoutSourceIdentity, error: String) {
        let is_current_pending = self
            .pending
            .as_ref()
            .is_some_and(|(pending, lease)| pending == &owner && lease.matches(&owner));
        if is_current_pending {
            if let Some((_, lease)) = self.pending.take() {
                lease.invalidate();
            }
            self.error = Some((owner, error));
        }
    }

    pub(crate) fn fail_before_begin(&mut self, owner: LayoutSourceIdentity, error: String) {
        self.retire();
        self.error = Some((owner, error));
    }

    fn retire(&mut self) {
        if let Some(source) = self.published.take() {
            source.lease.invalidate();
        }
        if let Some((_, lease)) = self.pending.take() {
            lease.invalidate();
        }
        self.error.take();
    }
}

impl LayoutSourceIdentity {
    pub(crate) fn from_accepted(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
    ) -> Self {
        Self {
            scope: scope.clone(),
            snapshot_token: snapshot.token,
            accepted_revision: snapshot.document.revision,
            accepted_document_identity: Arc::as_ptr(&snapshot.document) as usize,
            accepted_scene_identity: Arc::as_ptr(&snapshot.scene) as usize,
            source_generation,
        }
    }

    pub(crate) fn matches_current(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
    ) -> bool {
        self.scope == *scope
            && self.source_generation == source_generation
            && snapshot.token == self.snapshot_token
            && snapshot.session_epoch == self.scope.session_epoch
            && snapshot.session_epoch == scope.session_epoch
            && snapshot.document.id == self.scope.document_id
            && snapshot.document.id == scope.document_id
            && snapshot.document.revision == self.accepted_revision
            && Arc::as_ptr(&snapshot.document) as usize == self.accepted_document_identity
            && snapshot.scene.revision == self.accepted_revision
            && Arc::as_ptr(&snapshot.scene) as usize == self.accepted_scene_identity
            && self.accepted_revision == snapshot.document.revision
    }
}

impl LayoutSourceCapture {
    /// Build the canonical selected-board request from the accepted source.
    /// `model_paths` comes from the existing bundled/document model path owner.
    pub(crate) fn capture(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
        request_token: String,
        model_paths: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        if request_token.is_empty() {
            return Err("Layout preview request token must not be empty".into());
        }
        if source_generation == 0 {
            return Err("Layout source generation must be nonzero".into());
        }
        if snapshot.session_epoch != scope.session_epoch
            || snapshot.document.id != scope.document_id
            || snapshot.scene.revision != snapshot.document.revision
        {
            return Err("Layout preview scope does not match its accepted source".into());
        }
        let mut boards = snapshot
            .document
            .boards
            .iter()
            .filter(|board| board.id == scope.board_id);
        let board = boards.next().ok_or_else(|| {
            "Selected board is unavailable in the accepted Layout document".to_owned()
        })?;
        if boards.next().is_some() {
            return Err(
                "Selected board identity is ambiguous in the accepted Layout document".into(),
            );
        }
        let mut board_contours = snapshot
            .scene
            .board_contours
            .iter()
            .filter(|entry| entry.board_id == scope.board_id);
        let contours = board_contours
            .next()
            .ok_or_else(|| {
                "Selected board contours are unavailable in the accepted Layout scene".to_owned()
            })?
            .contours
            .clone();
        if board_contours.next().is_some() {
            return Err(
                "Selected board contours are ambiguous in the accepted Layout scene".into(),
            );
        }

        let mut references = snapshot
            .document
            .board_references
            .iter()
            .filter(|reference| reference.board_id == scope.board_id && reference.enabled);
        let request = if let Some(reference) = references.next() {
            if references.next().is_some() {
                return Err("Selected board has multiple enabled imported-board sources".into());
            }
            let mut assets = snapshot
                .document
                .assets
                .iter()
                .filter(|asset| asset.id == reference.asset_id);
            let asset = assets
                .next()
                .filter(|asset| !asset.sha256.is_empty())
                .cloned()
                .ok_or_else(|| {
                    "Imported board asset is unavailable in the accepted document".to_owned()
                })?;
            if assets.next().is_some() {
                return Err(
                    "Imported board asset identity is ambiguous in the accepted document".into(),
                );
            }
            LayoutPreviewRequest::Imported {
                reference: reference.clone(),
                asset,
            }
        } else {
            LayoutPreviewRequest::Authored(Box::new(PrepareExportRequest {
                snapshot_token: request_token,
                expected_revision: snapshot.document.revision,
                document: snapshot.document.as_ref().clone(),
                target: ExportTarget::Board {
                    board_id: board.id.clone(),
                },
                contours: contours.clone(),
                model_paths: model_paths.clone(),
            }))
        };

        let owner = LayoutSourceIdentity::from_accepted(snapshot, scope, source_generation);
        Ok(Self {
            lease: LayoutSourceLease::new(owner.clone()),
            owner,
            document: snapshot.document.clone(),
            contours,
            path_assets: model_paths,
            request,
        })
    }

    /// Convert the captured producer choice to the existing Core artifact request.
    /// Imported bytes are resolved by Runtime from this exact accepted asset SHA.
    pub(crate) fn artifact_request(
        &self,
        id: String,
        imported_source: Option<String>,
    ) -> Result<ArtifactRequest, String> {
        if id.is_empty() {
            return Err("Layout preview artifact request ID must not be empty".into());
        }
        match (&self.request, imported_source) {
            (LayoutPreviewRequest::Authored(request), None) => {
                Ok(ArtifactRequest::PreparePreview {
                    id,
                    request: request.as_ref().clone(),
                })
            }
            (LayoutPreviewRequest::Imported { .. }, Some(source)) if !source.is_empty() => {
                Ok(ArtifactRequest::PreviewBoard {
                    id,
                    source,
                    revision: self.owner.accepted_revision,
                })
            }
            (LayoutPreviewRequest::Imported { .. }, _) => {
                Err("Imported Layout preview requires its verified accepted asset bytes".into())
            }
            (LayoutPreviewRequest::Authored(_), Some(_)) => {
                Err("Authored Layout preview cannot consume imported-board bytes".into())
            }
        }
    }

    /// Accept only a preview for this accepted revision. Core reply/request IDs are
    /// checked by Runtime before this method is called.
    pub(crate) fn accept_preview(
        &self,
        preview: PcbPreview,
    ) -> Result<LayoutPreviewSnapshot, String> {
        if preview.revision != self.owner.accepted_revision
            || self.document.revision != self.owner.accepted_revision
        {
            return Err("Core returned a Layout board preview for another revision".into());
        }
        Ok(LayoutPreviewSnapshot {
            owner: self.owner.clone(),
            lease: self.lease.clone(),
            document: self.document.clone(),
            contours: self.contours.clone(),
            path_assets: self.path_assets.clone(),
            board_reference: match &self.request {
                LayoutPreviewRequest::Imported { reference, .. } => Some(reference.clone()),
                LayoutPreviewRequest::Authored(_) => None,
            },
            preview,
        })
    }
}

impl LayoutPreviewSnapshot {
    pub(crate) fn same_live_source(&self, expected: &Self) -> bool {
        self.owner == expected.owner
            && Rc::ptr_eq(&self.lease, &expected.lease)
            && self.lease.matches(&self.owner)
    }

    /// A renderer reference selects only a unique Part on this captured board and
    /// only while the full accepted scope and source generation still match.
    pub(crate) fn part_for_current_pick(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
        model_reference: &str,
    ) -> Option<String> {
        if !self
            .owner
            .matches_current(snapshot, scope, source_generation)
            || self.document.id != scope.document_id
            || self.document.revision != self.owner.accepted_revision
            || Arc::as_ptr(&self.document) as usize != self.owner.accepted_document_identity
            || !Arc::ptr_eq(&self.document, &snapshot.document)
            || self.preview.revision != self.owner.accepted_revision
        {
            return None;
        }
        crate::case_preview::part_for_native_preview_reference(
            &self.document,
            &scope.board_id,
            &self.preview,
            model_reference,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{
        Board, BoardContours, Contour, Part, PcbModel, Pose2, Readiness, SceneDelta, Side, Vec2,
        Vec3,
    };

    fn accepted(imported: bool) -> (AcceptedSnapshot, Scope) {
        let mut document = ProjectDoc::empty("doc", "Split board");
        document.revision = 7;
        document.boards.push(Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec!["part-1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "part-1".into(),
            definition_id: "mcu".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 2.0, y: 3.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        if imported {
            document.assets.push(Asset {
                id: "board-source".into(),
                name: "board.kicad_pcb".into(),
                media_type: "application/vnd.kicad.pcb".into(),
                sha256: "abc123".into(),
                license: None,
                source: None,
            });
            document.board_references.push(BoardReference {
                id: "left-reference".into(),
                board_id: "left".into(),
                asset_id: "board-source".into(),
                enabled: true,
                pose: Pose2 {
                    at: Vec2 {
                        x: 500.0,
                        y: -300.0,
                    },
                    rotation: 180.0,
                },
                elevation: 9.0,
                model_assets: BTreeMap::new(),
            });
        }
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "doc".into(),
            board_id: "left".into(),
            instance_id: Some("flipped-left".into()),
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(11),
            session_epoch: scope.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: 7,
                transaction_id: "accepted".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![BoardContours {
                    board_id: "left".into(),
                    contours: vec![Contour {
                        points: vec![
                            Vec2 { x: 1.0, y: 2.0 },
                            Vec2 { x: 4.0, y: 2.0 },
                            Vec2 { x: 4.0, y: 5.0 },
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
            }),
        };
        (snapshot, scope)
    }

    fn source_capture(snapshot: &AcceptedSnapshot, scope: &Scope) -> LayoutSourceCapture {
        LayoutSourceCapture::capture(
            snapshot,
            scope,
            3,
            "layout-preview-11".into(),
            BTreeMap::new(),
        )
        .unwrap()
    }

    fn preview() -> PcbPreview {
        PcbPreview {
            revision: 7,
            thickness: 1.6,
            contours: vec![],
            surfaces: vec![],
            holes: vec![],
            models: vec![PcbModel {
                id: "U1:0".into(),
                reference: "U1".into(),
                path: "models/mcu.step".into(),
                pose: Pose2 {
                    at: Vec2 { x: 2.0, y: 3.0 },
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
        }
    }

    #[test]
    fn authored_capture_uses_canonical_document_and_board_contours_independent_of_instance() {
        let (snapshot, scope) = accepted(false);
        let capture = source_capture(&snapshot, &scope);
        let mut another_instance = scope.clone();
        another_instance.instance_id = Some("other-physical-half".into());
        let other = source_capture(&snapshot, &another_instance);

        assert_eq!(capture.document.as_ref(), snapshot.document.as_ref());
        assert_eq!(capture.contours, snapshot.scene.board_contours[0].contours);
        assert_eq!(capture.contours, other.contours);
        assert_ne!(capture.owner, other.owner);
        let LayoutPreviewRequest::Authored(request) = capture.request else {
            panic!("authored board should use PreparePreview");
        };
        assert_eq!(request.document.parts[0].pose.at.x, 2.0);
        assert_eq!(request.contours, snapshot.scene.board_contours[0].contours);
        assert_eq!(
            request.target,
            ExportTarget::Board {
                board_id: "left".into()
            }
        );
    }

    #[test]
    fn imported_capture_selects_matching_accepted_asset_and_existing_preview_operation() {
        let (snapshot, scope) = accepted(true);
        let capture = source_capture(&snapshot, &scope);
        let LayoutPreviewRequest::Imported { reference, asset } = &capture.request else {
            panic!("enabled BoardReference should use PreviewBoard");
        };
        assert_eq!(reference.pose.at.x, 500.0);
        assert_eq!(asset.sha256, "abc123");
        assert!(matches!(
            capture.artifact_request("layout-imported".into(), Some("verified source".into())),
            Ok(ArtifactRequest::PreviewBoard { id, source, revision: 7 })
                if id == "layout-imported" && source == "verified source"
        ));
        assert!(
            capture
                .artifact_request("layout-imported".into(), None)
                .is_err()
        );
    }

    #[test]
    fn imported_layout_component_model_flows_through_existing_asset_and_mesh_delivery_route() {
        #[cfg(not(target_arch = "wasm32"))]
        use crate::model_delivery::{
            AssetSelection, MeshArrays, ModelAssetSource, ModelBatchIdentity, ModelDeliveryAdapter,
            ModelDeliveryPorts, ModelOwnerIdentity, ResolvedModelAsset, VerifiedModelBytes,
            native_model_path_assets, resolve_preview_assets,
        };
        #[cfg(target_arch = "wasm32")]
        use crate::presentation::model_delivery::{
            AssetSelection, MeshArrays, ModelAssetSource, ModelBatchIdentity, ModelDeliveryAdapter,
            ModelDeliveryPorts, ModelOwnerIdentity, ResolvedModelAsset, VerifiedModelBytes,
            native_model_path_assets, resolve_preview_assets,
        };
        use sha2::{Digest, Sha256};
        use std::{future::Future, task::Waker};

        let (snapshot, scope) = accepted(true);
        let model_bytes = b"accepted-layout-component-model".to_vec();
        let model_sha = Sha256::digest(&model_bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut document = snapshot.document.as_ref().clone();
        document.assets.push(Asset {
            id: "component-model".into(),
            name: "mcu.step".into(),
            media_type: "model/step".into(),
            sha256: model_sha.clone(),
            license: None,
            source: None,
        });
        document.board_references[0]
            .model_assets
            .insert("models/mcu.step".into(), "component-model".into());
        let snapshot = AcceptedSnapshot {
            document: Arc::new(document),
            ..snapshot
        };
        let capture = source_capture(&snapshot, &scope);
        let preview = capture.accept_preview(preview()).unwrap();

        let native = native_model_path_assets(&preview.path_assets);
        let selections = resolve_preview_assets(
            &preview.preview.models,
            preview.board_reference.as_ref(),
            &native,
            &preview.document,
            |_| None,
            |_| None,
        )
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        assert_eq!(
            selections.get("U1:0"),
            Some(&AssetSelection::Archived(ResolvedModelAsset {
                id: "component-model".into(),
                sha256: model_sha.clone(),
                filename: "mcu.step".into(),
                source: ModelAssetSource::Document,
            }))
        );

        let decode_count = Rc::new(Cell::new(0));
        let decode_count_for_port = decode_count.clone();
        let bytes_for_port = model_bytes.clone();
        let ports = ModelDeliveryPorts {
            load_verified_bytes: Rc::new(move |asset| {
                let bytes = bytes_for_port.clone();
                Box::pin(async move { Ok(Some(VerifiedModelBytes::verify(bytes, &asset.sha256)?)) })
            }),
            decode_stl: Rc::new(|_| Box::pin(async { Ok(MeshArrays::default()) })),
            decode_wrl: Rc::new(|_| Box::pin(async { Ok(MeshArrays::default()) })),
            read_step: Rc::new(move |_, _| {
                decode_count_for_port.set(decode_count_for_port.get() + 1);
                Box::pin(async {
                    Ok(MeshArrays {
                        positions: vec![0.0; 9],
                        normals: vec![1.0; 9],
                        colors: None,
                    })
                })
            }),
        };
        let owner = ModelOwnerIdentity::new_layout(
            preview.owner.scope.clone(),
            preview.owner.snapshot_token,
            preview.owner.source_generation,
            &preview.lease,
        );
        let batch = ModelBatchIdentity::new(owner, preview.owner.accepted_revision, 1);
        let lease = preview.lease.clone();
        let expected_owner = preview.owner.clone();
        let rows = block_on(ModelDeliveryAdapter::default().deliver_models(
            preview.preview.revision,
            &preview.preview.models,
            &selections,
            &ports,
            &batch,
            Rc::new(move || lease.matches(&expected_owner)),
        ))
        .expect("the accepted Layout model delivery should finish while current");

        assert_eq!(decode_count.get(), 1);
        assert_eq!(rows.delivered.len(), 1);
        assert_eq!(rows.delivered[0].id, "U1:0");
        assert!(rows.failures.is_empty());

        fn block_on<F: Future>(future: F) -> F::Output {
            let mut future = Box::pin(future);
            let waker = Waker::noop();
            let mut context = std::task::Context::from_waker(waker);
            loop {
                match future.as_mut().poll(&mut context) {
                    std::task::Poll::Ready(output) => return output,
                    std::task::Poll::Pending => std::thread::yield_now(),
                }
            }
        }
    }

    #[test]
    fn authored_request_uses_existing_prepare_preview_and_rejects_wrong_preview_revision() {
        let (snapshot, scope) = accepted(false);
        let capture = source_capture(&snapshot, &scope);
        assert!(matches!(
            capture.artifact_request("layout-authored".into(), None),
            Ok(ArtifactRequest::PreparePreview { id, request })
                if id == "layout-authored" && request.expected_revision == 7
        ));
        let mut stale = preview();
        stale.revision = 8;
        assert!(capture.accept_preview(stale).is_err());
        assert!(capture.accept_preview(preview()).is_ok());
    }

    #[test]
    fn picks_require_full_current_scope_source_generation_and_unique_board_reference() {
        let (snapshot, scope) = accepted(false);
        let accepted_preview = source_capture(&snapshot, &scope)
            .accept_preview(preview())
            .unwrap();
        assert_eq!(
            accepted_preview.part_for_current_pick(&snapshot, &scope, 3, "U1"),
            Some("part-1".into())
        );
        assert!(
            accepted_preview
                .part_for_current_pick(&snapshot, &scope, 4, "U1")
                .is_none()
        );
        let stale_scene = AcceptedSnapshot {
            scene: Arc::new(snapshot.scene.as_ref().clone()),
            ..snapshot.clone()
        };
        assert!(
            accepted_preview
                .part_for_current_pick(&stale_scene, &scope, 3, "U1")
                .is_none()
        );
        let mut other_instance = scope.clone();
        other_instance.instance_id = Some("right-half".into());
        assert!(
            accepted_preview
                .part_for_current_pick(&snapshot, &other_instance, 3, "U1")
                .is_none()
        );
        assert!(
            accepted_preview
                .part_for_current_pick(&snapshot, &scope, 3, "U1:0")
                .is_none()
        );

        let mut duplicate = snapshot.document.as_ref().clone();
        duplicate.parts.push(Part {
            id: "part-2".into(),
            ..duplicate.parts[0].clone()
        });
        duplicate.boards[0].part_ids.push("part-2".into());
        let duplicate_snapshot = AcceptedSnapshot {
            document: Arc::new(duplicate),
            ..snapshot.clone()
        };
        assert!(
            accepted_preview
                .part_for_current_pick(&duplicate_snapshot, &scope, 3, "U1")
                .is_none()
        );
    }

    #[test]
    fn source_state_rejects_late_worker_completion_after_owner_replacement() {
        let (snapshot, scope) = accepted(false);
        let mut state = LayoutPreviewState::default();
        let first_generation = state.next_generation().unwrap();
        let first = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            first_generation,
            "layout-preview-first".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&first);
        assert!(state.owns(&first.owner));

        let second_generation = state.next_generation().unwrap();
        let second = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            second_generation,
            "layout-preview-second".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&second);
        assert!(!first.lease.is_active());
        assert!(state.owns(&second.owner));

        let late_result = first.accept_preview(preview()).unwrap();
        assert!(state.publish(late_result).is_err());
        assert!(state.published.is_none());

        state
            .publish(second.accept_preview(preview()).unwrap())
            .unwrap();
        assert!(state.published.as_ref().is_some_and(|current| {
            current.owner == second.owner && current.lease.matches(&second.owner)
        }));
        assert!(state.retire_generation(second_generation));
        assert!(state.published.is_none());
        assert!(!second.lease.is_active());
    }

    #[test]
    fn suspended_model_failure_settlement_cannot_target_replacement_preview_owner() {
        let (snapshot, scope) = accepted(false);
        let mut state = LayoutPreviewState::default();

        let first_generation = state.next_generation().unwrap();
        let first = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            first_generation,
            "layout-preview-suspended-model-error-a".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&first);
        let captured_delivery = first.accept_preview(preview()).unwrap();
        state.publish(captured_delivery.clone()).unwrap();

        // Model delivery for A is now suspended. Retire A and publish a same-scope,
        // same-revision replacement, which exercises the generation/lease ABA case.
        let second_generation = state.next_generation().unwrap();
        let second = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            second_generation,
            "layout-preview-suspended-model-error-b".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&second);
        let replacement = second.accept_preview(preview()).unwrap();
        state.publish(replacement.clone()).unwrap();

        let current = state.published.as_ref().unwrap();
        assert!(current.same_live_source(&replacement));
        assert!(!current.same_live_source(&captured_delivery));
        assert!(!captured_delivery.lease.is_active());
    }

    #[test]
    fn source_request_reconciliation_retires_pending_owner_when_scope_disappears_or_changes() {
        let (snapshot, scope) = accepted(false);
        let mut state = LayoutPreviewState::default();
        let generation = state.next_generation().unwrap();
        let capture = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            generation,
            "layout-preview-request-reconcile".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&capture);

        assert!(!state.retire_unless_request_matches(Some((
            &scope,
            snapshot.token,
            snapshot.document.revision,
        ))));
        assert!(capture.lease.is_active());

        assert!(state.retire_unless_request_matches(None));
        assert!(!capture.lease.is_active());
        assert!(state.pending.is_none());

        let next_generation = state.next_generation().unwrap();
        let next = LayoutSourceCapture::capture(
            &snapshot,
            &scope,
            next_generation,
            "layout-preview-request-reconcile-next".into(),
            BTreeMap::new(),
        )
        .unwrap();
        state.begin(&next);
        assert!(state.retire_unless_request_matches(Some((
            &scope,
            snapshot.token,
            snapshot.document.revision + 1,
        ))));
        assert!(!next.lease.is_active());
    }

    #[test]
    fn capture_rejects_stale_scope_scene_board_and_ambiguous_import_source() {
        let (snapshot, scope) = accepted(false);
        let mut wrong_scope = scope.clone();
        wrong_scope.document_id = "another-document".into();
        assert!(
            LayoutSourceCapture::capture(
                &snapshot,
                &wrong_scope,
                3,
                "request".into(),
                BTreeMap::new()
            )
            .is_err()
        );
        let mut wrong_board = scope.clone();
        wrong_board.board_id = "missing".into();
        assert!(
            LayoutSourceCapture::capture(
                &snapshot,
                &wrong_board,
                3,
                "request".into(),
                BTreeMap::new()
            )
            .is_err()
        );
        let mut duplicate_board = snapshot.document.as_ref().clone();
        duplicate_board
            .boards
            .push(duplicate_board.boards[0].clone());
        let duplicate_board_snapshot = AcceptedSnapshot {
            document: Arc::new(duplicate_board),
            ..snapshot.clone()
        };
        assert!(
            LayoutSourceCapture::capture(
                &duplicate_board_snapshot,
                &scope,
                3,
                "request".into(),
                BTreeMap::new()
            )
            .is_err()
        );
        let mut duplicate_import = snapshot.document.as_ref().clone();
        duplicate_import.board_references.push(BoardReference {
            id: "another-reference".into(),
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
        let imported_snapshot = AcceptedSnapshot {
            document: Arc::new(duplicate_import),
            ..snapshot.clone()
        };
        assert!(
            LayoutSourceCapture::capture(
                &imported_snapshot,
                &scope,
                3,
                "request".into(),
                BTreeMap::new()
            )
            .is_err()
        );
    }
}
