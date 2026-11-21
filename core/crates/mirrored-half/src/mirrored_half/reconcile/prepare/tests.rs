use super::*;
use crate::mirrored_half::{EditDispatch, reconcile::tests::fixtures::fixture_document};
use crate::model::ElectricalBoardConfiguration;
use serde_json::json;

const DISPATCH: EditDispatch = EditDispatch {
    apply_with_ledger: |document, _, operation, _| apply(document, operation),
    apply_unscripted: apply,
};

// Use the real keymap and matrix kernels. The wiring arm supplies the authored mode write
// at the injected engine boundary, so the oracle compares the two mirror preparation paths.
fn apply(document: &mut ProjectDoc, operation: &EditOperation) -> Result<Vec<String>, String> {
    match operation {
        EditOperation::EditKeymap { change } => crate::keymap::apply_edit(document, change),
        EditOperation::SetWiringMode { board_id, mode } => {
            let row = document
                .hardware
                .as_mut()
                .unwrap()
                .boards
                .iter_mut()
                .find(|row| row.board_id == *board_id)
                .unwrap();
            if row.mode == *mode {
                return Ok(vec![]);
            }
            row.mode = *mode;
            Ok(vec![board_id.clone()])
        }
        EditOperation::SetKeyBinding {
            board_id,
            key_id,
            binding,
        } => {
            assert_eq!(binding, "&trans");
            document
                .hardware
                .as_mut()
                .unwrap()
                .boards
                .iter_mut()
                .find(|row| row.board_id == *board_id)
                .unwrap()
                .key_bindings
                .insert(key_id.clone(), binding.clone());
            document.keymap.as_mut().unwrap().layers[0]
                .bindings
                .insert(key_id.clone(), crate::keymap::KeyBinding::Transparent);
            Ok(vec![board_id.clone(), key_id.clone()])
        }
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        } => crate::matrix::set_matrix(document, matrix),
        _ => panic!("unexpected dispatch: {operation:?}"),
    }
}

fn document() -> ProjectDoc {
    let mut document = fixture_document().unwrap();
    let hardware = document.hardware.as_mut().unwrap();
    hardware.boards = ["source-board", "mirror-board"]
        .map(|board_id| ElectricalBoardConfiguration {
            board_id: board_id.into(),
            ..Default::default()
        })
        .into();
    hardware.pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(
            serde_json::from_value(json!({
                "kind": "electrical-board", "sourceId": "source-board", "mirrorId": "mirror-board"
            }))
            .unwrap(),
        );
    prepare_open(&document, &[]).unwrap().0
}

fn compare(accepted: &ProjectDoc, operation: serde_json::Value) -> (ProjectDoc, Vec<String>) {
    let operation: EditOperation = serde_json::from_value(operation).unwrap();
    let identities = crate::mechanical::edit::MechanicalIdentityLedger::seeded(accepted);
    let full = prepare_transaction(accepted, &[], &operation, &identities, DISPATCH, false)
        .unwrap_or_else(|error| panic!("full {operation:?}: {error}"));
    let directed =
        prepare_transaction_with_ledger(accepted, &[], &operation, &identities, DISPATCH)
            .unwrap_or_else(|error| panic!("directed {operation:?}: {error}"));
    assert_eq!(
        serde_json::to_value(&directed.0).unwrap(),
        serde_json::to_value(&full.0).unwrap(),
        "{operation:?}"
    );
    assert_eq!(directed.1, full.1, "{operation:?}");
    directed
}

#[test]
fn directed_settings_sequence_matches_full_preparation() {
    let mut accepted = document();
    for operation in [
        json!({"kind":"set-wiring-mode","boardId":"mirror-board","mode":"direct"}),
        json!({"kind":"set-wiring-mode","boardId":"source-board","mode":"direct"}),
        json!({"kind":"edit-keymap","change":{"kind":"binding","layerId":"base",
            "keyId":"matrix/mirror-matrix/r0c0","binding":{"kind":"key-press","keycode":"Q"}}}),
        json!({"kind":"set-key-binding","boardId":"mirror-board",
            "keyId":"matrix/mirror-matrix/r0c1","binding":"&trans"}),
        json!({"kind":"set-half-exception","pairId":"pair-source-mirror",
            "itemKind":"electrical-board","mirrorId":"mirror-board","state":"local"}),
        json!({"kind":"set-wiring-mode","boardId":"mirror-board","mode":"matrix"}),
        json!({"kind":"set-half-exception","pairId":"pair-source-mirror",
            "itemKind":"electrical-board","mirrorId":"mirror-board","state":"linked"}),
        json!({"kind":"set-wiring-mode","boardId":"source-board","mode":"matrix"}),
        json!({"kind":"set-half-exception","pairId":"pair-source-mirror",
            "itemKind":"part","mirrorId":"matrix/mirror-matrix/r0c1","state":"omitted"}),
        json!({"kind":"set-wiring-mode","boardId":"mirror-board","mode":"direct"}),
    ] {
        accepted = compare(&accepted, operation).0;
    }
}

#[test]
fn layout_pose_refresh_uses_full_preparation() {
    let mut accepted = document();
    let source_id = "matrix/source-matrix/r0c0";
    let part = accepted
        .parts
        .iter_mut()
        .find(|part| part.id == source_id)
        .unwrap();
    part.pose.at.x += 10.0;
    let (draft, changed) = compare(
        &accepted,
        json!({"kind":"set-wiring-mode","boardId":"source-board","mode":"direct"}),
    );
    assert_ne!(draft.parts, accepted.parts);
    assert!(changed.iter().any(|id| id == source_id));
}

#[test]
fn matrixless_pair_retains_wiring_refresh_on_ordinary_edits() {
    let mut accepted = document();
    accepted.matrices.clear();
    accepted.parts.clear();
    for board in &mut accepted.boards {
        board.part_ids.clear();
    }
    accepted.keymap = Some(Default::default());
    let hardware = accepted.hardware.as_mut().unwrap();
    hardware.pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|link| link.kind == MirroredItemKind::ElectricalBoard);
    // A replacement can preserve a matrixless pair's stale electrical row.
    hardware.boards[1].mode = crate::electrical::ElectricalMode::Direct;
    let (draft, _) = compare(
        &accepted,
        json!({"kind":"edit-keymap","change":{"kind":"rename-layer","id":"base","name":"Primary"}}),
    );
    let boards = &draft.hardware.as_ref().unwrap().boards;
    assert_eq!(boards[0].board_id, "source-board");
    assert_eq!(boards[1].board_id, "mirror-board");
    assert_eq!(boards[1].mode, crate::electrical::ElectricalMode::Matrix);
}
