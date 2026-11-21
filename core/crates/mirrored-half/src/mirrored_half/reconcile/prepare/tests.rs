use super::projection::matrix_by_id;
use super::*;
use crate::mirrored_half::{EditDispatch, reconcile::tests::fixtures::fixture_document};
use crate::model::ElectricalBoardConfiguration;
use serde_json::json;

const DISPATCH: EditDispatch = EditDispatch {
    apply_with_ledger: |document, _, operation, _| apply(document, operation),
    apply_unscripted: apply,
};

// Use the real keymap and matrix kernels where possible. Authored wiring and independent
// move writes supplied at the injected engine boundary use fixture-specific inputs.
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
        EditOperation::SetMatrixPlacement { matrix_id, change } => {
            use crate::matrix::{layout, placement, set_matrix};

            layout::refuse_omitted_storage_edit(document, matrix_id, &[])?;
            let current = matrix_by_id(document, matrix_id)?;
            let matrix = placement::update(current, change, document)?;
            if matrix == *current {
                return Ok(Vec::new());
            }
            placement::carrying_authored_companions(document, placement::Reported::Moved, |doc| {
                let mut changed = set_matrix(doc, &matrix)?;
                changed.extend(layout::sync(doc, matrix_id)?);
                Ok(changed)
            })
        }
        EditOperation::MoveParts { positions } => {
            let (mut changed, handled) = crate::matrix::layout::move_keys(document, positions)?;
            for position in positions
                .iter()
                .filter(|position| !handled.contains(&position.id))
            {
                // This fixture uses unanchored independent Parts outside any layout.
                assert!(document.constraints.is_empty());
                assert!(
                    document
                        .layouts
                        .iter()
                        .all(|layout| !layout.part_ids.contains(&position.id))
                );
                assert_eq!(
                    crate::matrix::layout::clamp_part_to_half(document, &position.id, position.at),
                    position.at
                );
                let part = document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == position.id)
                    .unwrap();
                assert!(part.anchor.is_none());
                part.pose.at = position.at;
                changed.push(position.id.clone());
            }
            Ok(changed)
        }
        EditOperation::SetPartRotation { part_id, rotation } => {
            crate::matrix::layout::set_detached_part_rotation(document, part_id, *rotation)
        }
        EditOperation::SetLayout { layout } => {
            let current = document
                .layouts
                .iter_mut()
                .find(|row| row.id == layout.id)
                .unwrap();
            assert_eq!(current.matrix_ids, layout.matrix_ids);
            assert_eq!(current.board_id, layout.board_id);
            *current = layout.clone();
            crate::matrix::layout::resolve(document)?;
            Ok(vec![layout.id.clone()])
        }
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
        serde_json::to_vec(&directed.0).unwrap(),
        serde_json::to_vec(&full.0).unwrap(),
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

fn with_second_pair(mut accepted: ProjectDoc) -> ProjectDoc {
    let mut other = serde_json::to_string(&accepted).unwrap();
    for identity in [
        "source-board",
        "mirror-board",
        "source-matrix",
        "mirror-matrix",
        "source-layout",
        "mirror-layout",
        "pair-source-mirror",
        "\"left\"",
        "\"right\"",
    ] {
        let suffix = identity.trim_matches('"');
        let replacement = if identity.starts_with('"') {
            format!("\"other-{suffix}\"")
        } else {
            format!("other-{suffix}")
        };
        other = other.replace(identity, &replacement);
    }
    let other: ProjectDoc = serde_json::from_str(&other).unwrap();
    accepted.boards.extend(other.boards);
    accepted.parts.extend(other.parts);
    accepted.matrices.extend(other.matrices);
    accepted.layouts.extend(other.layouts);
    let hardware = accepted.hardware.as_mut().unwrap();
    let other_hardware = other.hardware.unwrap();
    hardware.instances.extend(other_hardware.instances);
    hardware.boards.extend(other_hardware.boards);
    hardware.pairs.extend(other_hardware.pairs);
    accepted
}

#[test]
fn omitted_source_in_ownerless_matrix_leaves_reference_cleanup_to_lifecycle() {
    let mut accepted = document();
    let source = "matrix/source-matrix/r0c0";
    let mirror = "matrix/mirror-matrix/r0c0";
    accepted
        .matrices
        .retain(|matrix| matrix.id == "source-matrix");
    accepted.matrices[0].board_id = None;
    assert!(accepted.matrices[0].part_ids.iter().any(|id| id == source));
    accepted.parts.retain(|part| part.id != mirror);
    let mut free = accepted.parts[0].clone();
    free.id = "unrelated".into();
    accepted.boards[0].part_ids.push(free.id.clone());
    accepted.parts.push(free);
    for board in &mut accepted.boards {
        board.part_ids.retain(|id| id != mirror);
    }
    accepted.nets.push(
        serde_json::from_value(json!({
            "id":"unlinked", "name":"Unlinked", "pins":[{"partId":mirror,"padId":"1"}]
        }))
        .unwrap(),
    );
    let pair = &mut accepted.hardware.as_mut().unwrap().pairs[0];
    let halves = pair.halves.as_mut().unwrap();
    halves
        .correspondences
        .retain(|link| link.kind != MirroredItemKind::Matrix);
    assert!(
        halves
            .correspondences
            .iter()
            .any(|link| link.source_id == source && link.mirror_id == mirror)
    );
    halves.exceptions.push(
        serde_json::from_value(json!({
            "kind":"part", "mirrorId":mirror, "state":"omitted"
        }))
        .unwrap(),
    );
    let mut draft = accepted.clone();
    let pair = &accepted.hardware.as_ref().unwrap().pairs[0];
    let operation = EditOperation::SetPartRotation {
        part_id: "unrelated".into(),
        rotation: 15.0,
    };
    assert!(
        !projection::project_matrixless_placement(&accepted, &mut draft, pair, &operation).unwrap()
    );
    assert_eq!(draft, accepted);
}

#[test]
fn matrixless_placement_repairs_drift_in_both_mixed_pair_orders() {
    let mut accepted = document();
    for board in &mut accepted.boards {
        let mut part = accepted.parts[0].clone();
        part.id = format!("{}/free", board.id);
        part.reference = part.id.clone();
        board.part_ids.push(part.id.clone());
        accepted.parts.push(part);
    }
    let pair = &mut accepted.hardware.as_mut().unwrap().pairs[0];
    pair.mapping = Pose2 {
        at: crate::model::Vec2 { x: 80.0, y: -25.0 },
        rotation: 0.0,
    };
    pair.halves.as_mut().unwrap().correspondences.push(
        serde_json::from_value(json!({
            "kind":"part", "sourceId":"source-board/free", "mirrorId":"mirror-board/free"
        }))
        .unwrap(),
    );
    let mut accepted = with_second_pair(accepted);
    accepted
        .matrices
        .retain(|matrix| matrix.id.starts_with("other-"));
    accepted.parts.retain(|part| {
        !part.id.starts_with("matrix/source-matrix/")
            && !part.id.starts_with("matrix/mirror-matrix/")
    });
    for board in &mut accepted.boards[..2] {
        board.part_ids.retain(|id| id.ends_with("/free"));
    }
    accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|link| !link.source_id.contains("source-matrix"));
    let accepted = prepare_open(&accepted, &[]).unwrap().0;
    let matrixless_mirror = "mirror-board/free";
    let matrix_backed_mirror = "other-mirror-board/free";
    let part_index = |id| {
        accepted
            .parts
            .iter()
            .position(|part| part.id == id)
            .unwrap()
    };
    let matrixless_index = part_index(matrixless_mirror);
    let matrix_backed_index = part_index(matrix_backed_mirror);

    for reverse_order in [false, true] {
        let mut drifted = accepted.clone();
        if reverse_order {
            drifted.hardware.as_mut().unwrap().pairs.reverse();
        }
        drifted.parts[matrix_backed_index].pose.at.x += 37.0;
        let (mut moved, changed) = compare(
            &drifted,
            json!({"kind":"move-parts","positions":[{"id":"source-board/free","at":{"x":15.0,"y":-8.0}}]}),
        );
        assert_eq!(
            moved.parts[matrix_backed_index],
            accepted.parts[matrix_backed_index]
        );
        assert_eq!(
            moved.parts[matrixless_index].pose.at,
            crate::model::Vec2 { x: 65.0, y: -33.0 }
        );
        assert_eq!(
            changed,
            [matrixless_mirror, matrix_backed_mirror, "source-board/free"]
        );

        let expected_mirror = moved.parts[matrixless_index].clone();
        moved.parts[matrixless_index].pose.rotation = 73.0;
        let rotation = json!({
            "kind":"set-part-rotation", "partId":"other-source-board/free", "rotation":35.0
        });
        let mut projected = moved.clone();
        let pair = moved
            .hardware
            .as_ref()
            .unwrap()
            .pairs
            .iter()
            .find(|pair| pair.id == "pair-source-mirror")
            .unwrap();
        assert!(
            projection::project_matrixless_placement(
                &moved,
                &mut projected,
                pair,
                &serde_json::from_value(rotation.clone()).unwrap(),
            )
            .unwrap()
        );
        assert_eq!(projected.parts[matrixless_index], expected_mirror);
        let (rotated, changed) = compare(&moved, rotation);
        assert_eq!(rotated.parts[matrixless_index], expected_mirror);
        assert_eq!(rotated.parts[matrix_backed_index].pose.rotation, 145.0);
        assert_eq!(
            changed,
            [
                matrixless_mirror,
                matrix_backed_mirror,
                "other-source-board/free"
            ]
        );
    }
}

#[test]
fn placement_repairs_stale_link_in_another_pair() {
    use crate::mirrored_half::reconcile::tests::fixtures::fixture_matrix;
    use crate::model::MirroredCorrespondence;

    let mut accepted = document();
    for (board_id, matrix_id) in [
        ("source-board", "source-matrix-extra"),
        ("mirror-board", "mirror-matrix-extra"),
    ] {
        crate::matrix::set_matrix(&mut accepted, &fixture_matrix(matrix_id, board_id)).unwrap();
        let mut part = accepted.parts[0].clone();
        part.id = format!("{board_id}/free");
        part.reference = format!("{board_id}-free");
        accepted
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(part.id.clone());
        accepted.parts.push(part);
    }
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    for (kind, source_id, mirror_id) in [
        (
            MirroredItemKind::Part,
            "source-board/free",
            "mirror-board/free",
        ),
        (
            MirroredItemKind::Matrix,
            "source-matrix-extra",
            "mirror-matrix-extra",
        ),
        (
            MirroredItemKind::Part,
            "matrix/source-matrix-extra/r0c0",
            "matrix/mirror-matrix-extra/r0c0",
        ),
        (
            MirroredItemKind::Part,
            "matrix/source-matrix-extra/r0c1",
            "matrix/mirror-matrix-extra/r0c1",
        ),
    ] {
        halves.correspondences.push(MirroredCorrespondence {
            kind,
            source_id: source_id.into(),
            mirror_id: mirror_id.into(),
            companion_mode: None,
            companion_tweak: None,
        });
    }
    accepted = prepare_open(&with_second_pair(accepted), &[]).unwrap().0;

    let unrelated_mirror = "other-mirror-board/free";
    let mirror_index = accepted
        .parts
        .iter()
        .position(|part| part.id == unrelated_mirror)
        .unwrap();
    let expected = accepted.parts[mirror_index].clone();
    accepted.parts[mirror_index].pose.at.x += 37.0;

    let opened = prepare_open(&accepted, &[]).unwrap();
    assert_eq!(opened.0.parts[mirror_index], expected);
    assert_eq!(opened.1.len(), 1);
    assert_eq!(
        opened.1[0].id,
        "mirrored-half:open-repaired:other-pair-source-mirror"
    );

    let (placed, changed) = compare(
        &accepted,
        json!({"kind":"set-matrix-placement","matrixId":"mirror-matrix",
            "change":{"kind":"column-stagger","column":1,"value":4.0}}),
    );
    assert_eq!(placed.parts[mirror_index], expected);
    assert_eq!(
        matrix_by_id(&placed, "source-matrix")
            .unwrap()
            .column_layout[1]
            .stagger,
        4.0
    );
    assert_eq!(
        changed,
        [
            "matrix/mirror-matrix/r0c1",
            "matrix/source-matrix/r0c1",
            "mirror-matrix",
            unrelated_mirror,
            "source-matrix",
        ]
    );

    let (moved, changed) = compare(
        &accepted,
        json!({"kind":"move-parts","positions":[{"id":"source-board/free","at":{"x":15.0,"y":-8.0}}]}),
    );
    assert_eq!(moved.parts[mirror_index], expected);
    for id in ["source-board/free", "mirror-board/free", unrelated_mirror] {
        assert!(changed.iter().any(|changed_id| changed_id == id));
    }

    let (rotated, changed) = compare(
        &moved,
        json!({"kind":"set-part-rotation","partId":"source-board/free","rotation":35.0}),
    );
    assert!(changed.iter().any(|id| id == "source-board/free"));
    assert!(changed.iter().any(|id| id == "mirror-board/free"));
    assert_eq!(
        rotated
            .parts
            .iter()
            .find(|part| part.id == "source-board/free")
            .unwrap()
            .pose
            .rotation,
        35.0
    );
    assert_eq!(rotated.parts[mirror_index], expected);

    let mut stale = accepted.clone();
    stale.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .retain(|link| link.source_id != "matrix/source-matrix-extra/r0c1");
    let operation = serde_json::from_value(json!({
        "kind":"set-matrix-placement","matrixId":"mirror-matrix",
        "change":{"kind":"column-stagger","column":1,"value":4.0}
    }))
    .unwrap();
    let before = serde_json::to_vec(&stale).unwrap();
    let identities = crate::mechanical::edit::MechanicalIdentityLedger::seeded(&stale);
    let error = prepare_transaction_with_ledger(&stale, &[], &operation, &identities, DISPATCH)
        .unwrap_err();
    assert_eq!(
        error,
        "Matrix part \"matrix/source-matrix-extra/r0c1\" has no correspondence"
    );
    assert_eq!(serde_json::to_vec(&stale).unwrap(), before);
}

#[test]
fn matrix_only_pair_expands_and_trims_linked_membership() {
    let ordered_ids = [
        "matrix/mirror-matrix/r0c2",
        "matrix/source-matrix/r0c2",
        "mirror-board",
        "mirror-matrix",
        "pair-source-mirror",
        "source-board",
        "source-matrix",
    ];
    let accepted = document();
    let mut expanded = matrix_by_id(&accepted, "source-matrix").unwrap().clone();
    expanded.columns = 3;
    let mut new_cell = expanded.cells[1].clone();
    new_cell.column = 2;
    expanded.cells.push(new_cell);
    let (draft, changed) = compare(&accepted, json!({"kind":"set-matrix","matrix":expanded}));
    assert_eq!(changed, ordered_ids);
    let halves = draft.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.iter().any(|link| {
        link.kind == MirroredItemKind::Part
            && link.source_id == "matrix/source-matrix/r0c2"
            && link.mirror_id == "matrix/mirror-matrix/r0c2"
    }));
    assert_eq!(matrix_by_id(&draft, "mirror-matrix").unwrap().columns, 3);

    let mut trimmed = matrix_by_id(&draft, "source-matrix").unwrap().clone();
    trimmed.cells[2].enabled = false;
    let (draft, changed) = compare(&draft, json!({"kind":"set-matrix","matrix":trimmed}));
    assert_eq!(changed, ordered_ids);
    assert!(
        draft
            .parts
            .iter()
            .all(|part| !ordered_ids[..2].contains(&part.id.as_str()))
    );
    let halves = draft.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.iter().all(|link| {
        !ordered_ids[..2].contains(&link.source_id.as_str())
            && !ordered_ids[..2].contains(&link.mirror_id.as_str())
    }));
}

#[test]
fn linked_placement_sequence_matches_full_preparation() {
    let mut accepted = document();
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(
        serde_json::from_value(json!({
            "kind":"layout","sourceId":"source-layout","mirrorId":"mirror-layout"
        }))
        .unwrap(),
    );
    accepted.layouts = serde_json::from_value(json!([
        {"id":"source-layout","name":"Source","boardId":"source-board","matrixIds":["source-matrix"],"partIds":[]},
        {"id":"mirror-layout","name":"Mirror","boardId":"mirror-board","matrixIds":["mirror-matrix"],"partIds":[]}
    ])).unwrap();

    accepted = with_second_pair(accepted);
    accepted = prepare_open(&accepted, &[]).unwrap().0;

    let mut layout = accepted.layouts[1].clone();
    layout.name = "Mirror placement".into();
    let layout_edit = json!({"kind":"set-layout","layout":layout});
    let (renamed, changed) = compare(&accepted, layout_edit.clone());
    assert_eq!(changed, ["mirror-layout"]);
    assert_eq!(renamed.layouts[1].name, layout.name);
    assert!(stable_placement(
        &accepted,
        &renamed,
        &serde_json::from_value(layout_edit.clone()).unwrap()
    ));
    let (same, unchanged) = compare(&renamed, layout_edit);
    assert_eq!(same, renamed);
    assert!(unchanged.is_empty());

    let matrix = same
        .matrices
        .iter()
        .find(|row| row.id == "mirror-matrix")
        .unwrap();
    let mut placed = matrix.clone();
    placed.origin.y += 7.0;
    let placement = json!({"kind":"set-matrix","matrix":placed});
    let (placed, changed) = compare(&same, placement.clone());
    assert!(stable_placement(
        &same,
        &placed,
        &serde_json::from_value(placement).unwrap()
    ));
    assert!(changed.iter().any(|id| id == "source-matrix"));
    assert!(changed.iter().any(|id| id == "mirror-matrix"));
    assert!(!changed.iter().any(|id| id.contains("other-")));

    let key = "matrix/mirror-matrix/r0c1";
    let other_key = "matrix/other-mirror-matrix/r0c0";
    let local = |state| {
        json!({
            "kind":"set-half-exception","pairId":"pair-source-mirror",
            "itemKind":"part","mirrorId":key,"state":state
        })
    };
    let (local_draft, _) = compare(&placed, local("local"));
    let (moved, changed) = compare(
        &local_draft,
        json!({
            "kind":"move-parts","positions":[
                {"id": key,"at":{"x":5.0,"y":-18.0}},
                {"id": other_key,"at":{"x":60.0,"y":-27.0}}
            ]
        }),
    );
    assert!(changed.iter().any(|id| id == key));
    assert!(
        changed
            .iter()
            .any(|id| id == "matrix/other-source-matrix/r0c0")
    );
    let (rejoined, _) = compare(&moved, local("linked"));
    let (omitted, _) = compare(&rejoined, local("omitted"));
    assert!(omitted.parts.iter().all(|row| row.id != key));
    let (omitted, changed) = compare(
        &omitted,
        json!({
            "kind":"move-parts","positions":[
                {"id":"matrix/mirror-matrix/r0c0","at":{"x":6.0,"y":-19.0}}
            ]
        }),
    );
    assert!(omitted.parts.iter().all(|row| row.id != key));
    assert!(!changed.iter().any(|id| id == key));
    let denied: EditOperation = serde_json::from_value(json!({
        "kind": "move-parts", "positions": [{"id":key,"at":{"x":5.0,"y":-18.0}}]
    }))
    .unwrap();
    let identities = crate::mechanical::edit::MechanicalIdentityLedger::seeded(&omitted);
    let before_refusal = serde_json::to_vec(&omitted).unwrap();
    let refusal =
        prepare_transaction_with_ledger(&omitted, &[], &denied, &identities, DISPATCH).unwrap_err();
    assert!(
        refusal.contains("is omitted on its mirror half"),
        "{refusal}"
    );
    assert_eq!(serde_json::to_vec(&omitted).unwrap(), before_refusal);

    let (restored, _) = compare(&omitted, local("linked"));
    assert!(restored.parts.iter().any(|row| row.id == key));
    assert_eq!(
        restored.hardware.as_ref().unwrap().pairs,
        same.hardware.as_ref().unwrap().pairs,
        "lifecycle changes kept the original pair identities"
    );
}
