// Mirrored-half prepared-transaction integration tests, split into the
// submodules under `mirrored_prepare/` so no file exceeds ~1,600 LOC.
// Shared fixtures/helpers live in `support/`; each submodule inherits this
// crate's imports via `use super::*`.
#[path = "support/matrix.rs"]
pub mod matrix_fixtures;
pub mod support;

use crate::matrix_fixtures::pair_document;
use crate::support::{
    linked_endpoint, part_by_id as part, part_by_id_mut as part_mut, prepare_transaction,
};
use boardstudio_core_kernel::migrate::CURRENT_VERSION;
use boardstudio_model::model::{
    CaseBody, Constraint, ConstraintEndpoint, ConstraintRelation, CoreReply, CoreRequest,
    EditOperation, Position, ProjectDoc, Vec2,
};
use serde_json::json;

// The shared fixture also serves the integration target, which calls with_construction_features.
#[allow(dead_code)]
#[path = "../../../tests/support/synthetic_half.rs"]
mod synthetic_half_fixture;

#[path = "mirrored_prepare/composition_and_tombstones.rs"]
mod composition_and_tombstones;
#[path = "mirrored_prepare/open_and_preview.rs"]
mod open_and_preview;
#[path = "mirrored_prepare/open_repairs_and_mirror_moves.rs"]
mod open_repairs_and_mirror_moves;
