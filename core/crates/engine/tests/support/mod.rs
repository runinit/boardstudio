use boardstudio_core_mirrored_half::mirrored_half::{
    EditDispatch, prepare_open_with_scripts, prepare_transaction_with_ledger,
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
    boardstudio_core_engine::apply_for_test(
        doc,
        board_contours,
        op,
        boardstudio_script::apply_scripts,
    )
}

pub fn prepare_open(
    accepted: &ProjectDoc,
    board_contours: &[BoardContours],
) -> Result<(ProjectDoc, Vec<Finding>), String> {
    prepare_open_with_scripts(accepted, board_contours, boardstudio_script::apply_scripts)
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
        boardstudio_script::apply_scripts,
        test_dispatch(),
    )
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
