//! Document engine, edit dispatch and stateless Core operations.

pub mod archive;
pub use boardstudio_core_artifact::artifact;
pub use boardstudio_core_mechanical::{electrical, electrical_peripherals};
pub use boardstudio_model::electrical_jumpers;
pub use boardstudio_model::electrical_profiles;
pub mod firmware;
mod keycaps;
use boardstudio_core_mechanical::keymap;
pub use boardstudio_core_mechanical::mechanical;
use boardstudio_core_mechanical::modules;
use boardstudio_core_mirrored_half as mirrored_half;
pub use boardstudio_model::document_format;
pub use boardstudio_model::model;
use std::sync::Arc;
mod outline_validation;
mod validate;

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub use validate::validate as validate_document_for_test;

use boardstudio_core_kernel::{
    case, constraints, geometry, hardware, inputs, matrix, outline_controls, outline_versions,
};
pub use boardstudio_core_kernel::{generators, hash, outline_edit};

use geometry::{OutlineCache, outlines};
pub use matrix::kernel as layout_kernel;
use matrix::layout;
pub use matrix::layout::evaluated_part_pose;
pub use matrix::layout::half_offset;
pub use matrix::layout::is_matrix_key;
pub use matrix::layout::layout_half_offset;
pub use matrix::layout::part_correspondence;
pub use matrix::layout::proposed_creation_gap;
use model::*;
use std::collections::{BTreeMap, BTreeSet};
#[path = "apply.rs"]
mod apply;
#[path = "dispatch.rs"]
mod dispatch;
#[cfg(test)]
#[path = "edit_tests.rs"]
mod edit_tests;
#[path = "preview.rs"]
mod preview;
mod scene_reuse;

pub use self::apply::{apply_for_test, apply_unscripted_for_test, apply_with_ledger_for_test};
pub struct CoreEngine {
    document: ProjectDoc,
    resolved: Arc<mirrored_half::ResolvedDoc>,
    undo: Vec<ProjectDoc>,
    redo: Vec<ProjectDoc>,
    contours: Vec<Contour>,
    findings: Vec<Finding>,
    outline_cache: OutlineCache,
    /// Accepted projection when safe to reuse. Reply metadata is set per request.
    reusable_scene: Option<SceneDelta>,
    stagger_preview: Option<scene_reuse::StaggerPreview>,
    #[cfg(test)]
    stagger_outline_projections: std::cell::Cell<usize>,
    #[cfg(test)]
    full_scene_projections: std::cell::Cell<usize>,
    #[cfg(test)]
    independent_board_projections: std::cell::Cell<usize>,
    /// Session high-water marks; not part of any Undo snapshot or saved document.
    identity_ledger: mechanical::edit::MechanicalIdentityLedger,
}

impl CoreEngine {
    // Worker transport keeps the Rust and TypeScript protocols identical.
    pub fn request(&mut self, json: &str) -> String {
        let preflight = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|error| error.to_string())
            .and_then(|value| {
                let candidate = if value["kind"] == "open" {
                    Some(&value["document"])
                } else if value["kind"] == "edit"
                    && value["command"]["operation"]["kind"] == "replace-document"
                {
                    Some(&value["command"]["operation"]["document"])
                } else {
                    None
                };
                candidate.map_or(Ok(()), document_format::require_current_value)
            });
        if let Err(message) = preflight {
            return boardstudio_model::wire::encode_core_reply(&CoreReply::Error {
                id: String::new(),
                message,
                revision: self.document.revision,
            });
        }
        let reply = match boardstudio_model::wire::decode_core_request(json) {
            Ok(request) => self.handle(request),
            Err(error) => CoreReply::Error {
                id: String::new(),
                message: error.to_string(),
                revision: self.document.revision,
            },
        };
        boardstudio_model::wire::encode_core_reply(&reply)
    }
}

/// Returns Core's reason when a linked item cannot change companion placement.
/// Accepts either the source or mirror part id so Inspectors can normalize either selection.
pub fn companion_placement_disabled_reason(
    document: &ProjectDoc,
    pair_id: &str,
    item_id: &str,
) -> Option<&'static str> {
    mirrored_half::companion_placement_disabled_reason(document, pair_id, item_id)
}

pub use boardstudio_model::case_readiness::{authored_case_geometry_ready, case_preparation_ready};

impl Default for CoreEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreEngine {
    fn refresh_resolved(&mut self) {
        self.resolved = Arc::new(mirrored_half::project_document(&self.document));
    }

    pub fn new() -> Self {
        let document = ProjectDoc::empty("untitled", "Untitled");
        Self {
            resolved: Arc::new(mirrored_half::project_document(&document)),
            document,
            undo: vec![],
            redo: vec![],
            contours: vec![],
            findings: vec![],
            outline_cache: OutlineCache::default(),
            reusable_scene: None,
            stagger_preview: None,
            #[cfg(test)]
            stagger_outline_projections: std::cell::Cell::default(),
            #[cfg(test)]
            full_scene_projections: std::cell::Cell::default(),
            #[cfg(test)]
            independent_board_projections: std::cell::Cell::default(),
            identity_ledger: mechanical::edit::MechanicalIdentityLedger::default(),
        }
    }
}

pub use boardstudio_core_mirrored_half::{ResolvedDoc, project_document};
