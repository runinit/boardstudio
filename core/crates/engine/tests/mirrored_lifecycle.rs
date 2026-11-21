// Mirrored-half lifecycle integration tests, split into the submodules
// under `mirrored_lifecycle/` so no file exceeds ~1,600 LOC. Shared fixture
// builders live in `support/`; each submodule inherits this crate's imports
// via `use super::*` and cross-imports helpers from sibling submodules.
#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "mirrored_reconcile_a/fixture_rows.rs"]
pub mod fixture_rows;
pub mod support;
use fixture_rows::{FixturePair, FixtureRows};

use crate::base_fixtures::fixture_document;
use boardstudio_model::model::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn add_link(document: &mut ProjectDoc, kind: MirroredItemKind, source_id: &str, mirror_id: &str) {
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            kind,
            source_id: source_id.into(),
            mirror_id: mirror_id.into(),
            companion_mode: None,
            companion_tweak: None,
        });
}

fn exception(
    document: &ProjectDoc,
    kind: MirroredItemKind,
    mirror_id: &str,
    state: HalfExceptionState,
) -> Result<ProjectDoc, String> {
    support::prepare_transaction(
        document,
        &[],
        &EditOperation::SetHalfException {
            pair_id: document.hardware.as_ref().unwrap().pairs[0].id.clone(),
            kind,
            mirror_id: mirror_id.into(),
            state,
        },
    )
    .map(|(doc, _)| doc)
}

#[path = "mirrored_lifecycle/constraint_lifecycle.rs"]
mod constraint_lifecycle;
#[path = "mirrored_lifecycle/independent_exceptions.rs"]
mod independent_exceptions;
#[path = "mirrored_lifecycle/matrix_and_circuit_lifecycle.rs"]
mod matrix_and_circuit_lifecycle;
#[path = "mirrored_lifecycle/multi_matrix_and_replacement.rs"]
mod multi_matrix_and_replacement;
#[path = "mirrored_lifecycle/owner_rejoin_electrical.rs"]
mod owner_rejoin_electrical;
