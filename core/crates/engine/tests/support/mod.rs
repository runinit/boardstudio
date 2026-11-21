use boardstudio_core_mirrored_half::mirrored_half::{
    EditDispatch, prepare_transaction_with_ledger,
};
use boardstudio_model::model::{BoardContours, EditOperation, Finding, ProjectDoc};

pub fn test_dispatch() -> EditDispatch {
    EditDispatch {
        apply_with_ledger: boardstudio_core_engine::apply_with_ledger_for_test,
        apply_unscripted: boardstudio_core_engine::apply_unscripted_for_test,
    }
}

pub fn apply_for_test(
    doc: &mut ProjectDoc,
    board_contours: &[boardstudio_model::model::BoardContours],
    op: &boardstudio_model::model::EditOperation,
) -> Result<Vec<String>, String> {
    boardstudio_core_engine::apply_for_test(doc, board_contours, op)
}

pub fn prepare_open(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
) -> Result<(ProjectDoc, Vec<Finding>), String> {
    boardstudio_core_mirrored_half::mirrored_half::prepare_open(accepted, board_contours)
}

/// Builds a complete document draft for one edit without changing the accepted document.
pub fn prepare_transaction(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
    operation: &EditOperation,
) -> Result<(ProjectDoc, Vec<String>), String> {
    prepare_transaction_with_ledger(
        accepted,
        board_contours,
        operation,
        &boardstudio_core_mechanical::mechanical::edit::MechanicalIdentityLedger::default(),
        test_dispatch(),
    )
}

/// A linked-mirror constraint endpoint for `pair_id` naming both halves' parts.
/// Shared by the lifecycle and prepare suites, which previously defined
/// the same constructor (the prepare copy hard-coded `pair_id`).
pub fn linked_endpoint(
    pair_id: &str,
    source_part_id: &str,
    mirror_part_id: &str,
) -> boardstudio_model::model::ConstraintEndpoint {
    boardstudio_model::model::ConstraintEndpoint::LinkedMirror {
        pair_id: pair_id.into(),
        source_part_id: source_part_id.into(),
        mirror_part_id: mirror_part_id.into(),
    }
}

/// Finds a part by id, panicking with the id absent (test-fixture lookup).
pub fn part_by_id<'a>(document: &'a ProjectDoc, id: &str) -> &'a boardstudio_model::model::Part {
    document
        .parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
}

/// Mutable counterpart of [`part_by_id`].
pub fn part_by_id_mut<'a>(
    document: &'a mut ProjectDoc,
    id: &str,
) -> &'a mut boardstudio_model::model::Part {
    document
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
}

pub fn canonicalize(
    document: &ProjectDoc,
    operation: &EditOperation,
) -> Result<
    boardstudio_core_mirrored_half::mirrored_half::inverse::Canonical,
    boardstudio_core_mirrored_half::mirrored_half::inverse::Refusal,
> {
    boardstudio_core_mirrored_half::mirrored_half::inverse::canonicalize_with_dispatch(
        document,
        operation,
        test_dispatch(),
    )
}
