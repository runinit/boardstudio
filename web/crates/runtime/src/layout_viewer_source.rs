//! Private canonical Layout preview source capture and pick mapping.
//!
//! This module deliberately does not own async jobs, asset storage, model decoding,
//! or a renderer. Runtime and the shared viewer supply those owners and recheck this
//! captured identity after each asynchronous boundary.

use crate::model_delivery::{
    PreviewLease, PreviewSlot, PreviewSnapshot, capture_board_preview_request,
};
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::{
    ArtifactRequest, BoardReference, Contour, KeycapSpec, PcbPreview, ProjectDoc, ResolvedModule,
};
#[cfg(test)]
use boardstudio_model::model::{Asset, ExportTarget};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutSourceIdentity {
    /// Keep the complete active scope, including a retained physical instance.
    /// The instance participates in freshness only; it never projects geometry.
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,

    pub source_generation: u64,
}

pub use crate::model_delivery::BoardPreviewRequest as LayoutPreviewRequest;
pub type LayoutSourceLease = PreviewLease<LayoutSourceIdentity>;

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSourceCapture {
    pub owner: LayoutSourceIdentity,
    pub lease: Rc<LayoutSourceLease>,
    pub document: Arc<ProjectDoc>,
    /// Current resolved modules from the same accepted SceneDelta as this capture.
    pub module_scenes: Vec<ResolvedModule>,
    pub contours: Vec<Contour>,
    pub path_assets: BTreeMap<String, String>,
    pub request: LayoutPreviewRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutPreviewSnapshot {
    pub owner: LayoutSourceIdentity,
    pub lease: Rc<LayoutSourceLease>,
    pub document: Arc<ProjectDoc>,
    /// Board-scoped resolved modules captured with the accepted Layout source.
    pub module_scenes: Vec<ResolvedModule>,
    pub contours: Vec<Contour>,
    pub path_assets: BTreeMap<String, String>,
    pub board_reference: Option<BoardReference>,
    pub preview: PcbPreview,
}

/// Keycap metadata from the exact mesh generation supplied to the current viewer.
pub struct LayoutKeycapPickSource<'a> {
    pub scope: &'a Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub rendered_generation: u64,
    pub specs: &'a [KeycapSpec],
    pub body_ids: &'a [String],
}

impl PreviewSnapshot<LayoutSourceIdentity> for LayoutPreviewSnapshot {
    fn source(
        &self,
    ) -> (
        &LayoutSourceIdentity,
        &Rc<PreviewLease<LayoutSourceIdentity>>,
    ) {
        (&self.owner, &self.lease)
    }
}

pub type LayoutPreviewState = PreviewSlot<LayoutSourceIdentity, LayoutPreviewSnapshot>;

impl LayoutSourceIdentity {
    pub fn from_accepted(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
    ) -> Self {
        Self {
            scope: scope.clone(),
            snapshot_token: snapshot.token,

            source_generation,
        }
    }

    pub fn matches_current(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
    ) -> bool {
        self.scope == *scope
            && self.source_generation == source_generation
            && snapshot.token == self.snapshot_token
            && snapshot.session_epoch == scope.session_epoch
            && snapshot.document.id == scope.document_id
            && snapshot.scene.revision == snapshot.document.revision
    }
}

impl LayoutSourceCapture {
    /// Build the canonical selected-board request from the accepted source.
    /// `model_paths` comes from the existing bundled/document model path owner.
    pub fn capture(
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
        boards.next().ok_or_else(|| {
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

        let request = capture_board_preview_request(
            &snapshot.document,
            &scope.board_id,
            &contours,
            &model_paths,
            request_token,
            "accepted document",
        )?;

        let owner = LayoutSourceIdentity::from_accepted(snapshot, scope, source_generation);
        let module_scenes = snapshot
            .scene
            .module_scenes
            .iter()
            .filter(|resolved| {
                snapshot.document.modules.iter().any(|module| {
                    module.id == resolved.id && module.host_board_id == scope.board_id
                })
            })
            .cloned()
            .collect();
        Ok(Self {
            lease: LayoutSourceLease::new(owner.clone()),
            owner,
            document: snapshot.document.clone(),
            module_scenes,
            contours,
            path_assets: model_paths,
            request,
        })
    }

    /// Convert the captured producer choice to the existing Core artifact request.
    /// Imported bytes are resolved by Runtime from this exact accepted asset SHA.
    pub fn artifact_request(
        &self,
        id: String,
        imported_source: Option<String>,
    ) -> Result<ArtifactRequest, String> {
        if id.is_empty() {
            return Err("Layout preview artifact request ID must not be empty".into());
        }
        match (&self.request, imported_source) {
            (LayoutPreviewRequest::Authored(request), None) => Ok(ArtifactRequest::PreviewPcb {
                id,
                request: request.as_ref().clone(),
            }),
            (LayoutPreviewRequest::Imported { .. }, Some(source)) if !source.is_empty() => {
                Ok(ArtifactRequest::PreviewBoard {
                    id,
                    source,
                    revision: self.document.revision,
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
    pub fn accept_preview(&self, preview: PcbPreview) -> Result<LayoutPreviewSnapshot, String> {
        if preview.revision != self.document.revision {
            return Err("Core returned a Layout board preview for another revision".into());
        }
        Ok(LayoutPreviewSnapshot {
            owner: self.owner.clone(),
            lease: self.lease.clone(),
            document: self.document.clone(),
            module_scenes: self.module_scenes.clone(),
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
    pub fn same_live_source(&self, expected: &Self) -> bool {
        self.owner == expected.owner
            && Rc::ptr_eq(&self.lease, &expected.lease)
            && self.lease.matches(&self.owner)
    }

    /// Resolve a private Layout renderer ID to the mounted placement represented
    /// by this accepted scene, rejecting stale or cross-board pick events.
    pub fn module_for_current_pick(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
        renderer_id: &str,
    ) -> Option<String> {
        let module_id = self.module_scenes.iter().find_map(|module| {
            if let Some(body) = renderer_id.strip_prefix(&format!("module:{}:", module.id)) {
                return ["pcb:", "volume:", "standoff:"]
                    .iter()
                    .any(|kind| body.starts_with(kind))
                    .then_some(module.id.as_str());
            }
            let model_index = renderer_id
                .strip_prefix("module-model/")?
                .strip_prefix(&format!("{}/", module.id))?
                .parse::<usize>()
                .ok()?;
            module.models.get(model_index).map(|_| module.id.as_str())
        })?;
        if !self
            .owner
            .matches_current(snapshot, scope, source_generation)
            || self.document.id != scope.document_id
            || !Arc::ptr_eq(&self.document, &snapshot.document)
            || self.preview.revision != self.document.revision
            || !self.lease.matches(&self.owner)
            || !self
                .module_scenes
                .iter()
                .any(|module| module.id == module_id)
            || !self
                .document
                .modules
                .iter()
                .any(|module| module.id == module_id && module.host_board_id == scope.board_id)
        {
            return None;
        }
        Some(module_id.to_owned())
    }

    /// A renderer reference selects only a unique Part on this captured board and
    /// only while the full accepted scope and source generation still match.
    pub fn part_for_current_pick(
        &self,
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        source_generation: u64,
        model_reference: &str,
        keycaps: Option<&LayoutKeycapPickSource<'_>>,
    ) -> Option<String> {
        if !self
            .owner
            .matches_current(snapshot, scope, source_generation)
            || self.document.id != scope.document_id
            || !Arc::ptr_eq(&self.document, &snapshot.document)
            || self.preview.revision != self.document.revision
            || !self.lease.matches(&self.owner)
        {
            return None;
        }
        if let Some(id) = model_reference
            .strip_prefix("keycap:")
            .or_else(|| model_reference.strip_prefix("keycap-legend:"))
        {
            let keycaps = keycaps?;
            if keycaps.scope != scope
                || keycaps.snapshot_token != snapshot.token
                || keycaps.revision != snapshot.document.revision
                || keycaps.generation == 0
                || keycaps.generation != keycaps.rendered_generation
                || keycaps
                    .body_ids
                    .iter()
                    .filter(|id| id.as_str() == model_reference)
                    .count()
                    != 1
            {
                return None;
            }
            let mut matching_specs = keycaps.specs.iter().filter(|spec| spec.id == id);
            let spec = matching_specs.next()?;
            if matching_specs.next().is_some() {
                return None;
            }
            let board = self
                .document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)?;
            let mut matching_parts = self
                .document
                .parts
                .iter()
                .filter(|part| part.id == spec.id && board.part_ids.contains(&part.id));
            let part = matching_parts.next()?;
            return matching_parts.next().is_none().then(|| part.id.clone());
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
mod tests;
