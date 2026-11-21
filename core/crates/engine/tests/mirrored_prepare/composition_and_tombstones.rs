use super::*;

#[test]
fn set_case_keeps_the_other_boards_custom_body_independent() {
    let mut accepted = pair_document();
    let accepted_correspondences = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .clone();
    accepted.case_bodies = [("left-case", "left", 2.0), ("right-case", "right", 3.0)]
        .into_iter()
        .map(|(id, board_id, thickness)| {
            serde_json::from_value::<CaseBody>(json!({
                "id": id,
                "name": id,
                "boardId": board_id,
                "kind": "tray",
                "thickness": thickness,
                "clearance": 0.2
            }))
            .unwrap()
        })
        .collect();
    let right_before = accepted
        .case_bodies
        .iter()
        .find(|body| body.id == "right-case")
        .unwrap()
        .clone();
    let mut left = accepted
        .case_bodies
        .iter()
        .find(|body| body.id == "left-case")
        .unwrap()
        .clone();
    left.thickness = 4.0;

    let (prepared, _) = prepare_transaction(&accepted, &[], &EditOperation::SetCase { body: left })
        .expect("the per-board case edit is preserved through pair reconciliation");

    assert_eq!(
        prepared
            .case_bodies
            .iter()
            .find(|body| body.id == "left-case")
            .unwrap()
            .thickness,
        4.0
    );
    assert_eq!(
        prepared
            .case_bodies
            .iter()
            .find(|body| body.id == "right-case")
            .unwrap(),
        &right_before
    );
    assert_eq!(
        prepared.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences,
        accepted_correspondences
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
}

#[test]
fn no_op_mirror_move_reports_no_changed_id() {
    let accepted = pair_document();
    let current = part(&accepted, "right/U1").pose.at;
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: "right/U1".into(),
            at: current,
        }],
    };

    let (prepared, changed) = prepare_transaction(&accepted, &[], &operation)
        .expect("an unchanged move is a valid inert preparation");

    assert_eq!(prepared, accepted);
    assert!(changed.is_empty());
}

#[test]
fn open_preserves_selected_tombstones_across_later_source_and_mirror_edits() {
    fn cell<'a>(
        document: &'a ProjectDoc,
        matrix_id: &str,
        coordinate: (u32, u32),
    ) -> &'a boardstudio_model::model::MatrixCell {
        document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .unwrap()
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == coordinate)
            .unwrap()
    }

    fn assert_tombstone(document: &ProjectDoc) {
        for (matrix_id, board_id) in [("left-keys", "left"), ("right-keys", "right")] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            let cell_id = format!("matrix/{matrix_id}/r0c0");
            assert!(!cell(document, matrix_id, (0, 0)).enabled);
            assert!(cell(document, matrix_id, (0, 0)).deleted);
            assert!(!matrix.part_ids.iter().any(|id| id == &cell_id));
            assert!(
                !document.parts.iter().any(|part| {
                    part.id == cell_id || part.id.starts_with(&format!("{cell_id}/"))
                })
            );
            assert!(
                !document
                    .boards
                    .iter()
                    .find(|board| board.id == board_id)
                    .unwrap()
                    .part_ids
                    .iter()
                    .any(|id| id == &cell_id || id.starts_with(&format!("{cell_id}/")))
            );
        }
    }

    let accepted = pair_document();
    let remove = EditOperation::RemoveSelectedMatrixCells {
        cells: std::collections::BTreeMap::from([("left-keys".into(), vec![(0, 0)])]),
    };
    let (removed, _) = prepare_transaction(&accepted, &[], &remove)
        .expect("selected-cell removal prepares paired tombstones");
    let source_cells = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .cells
        .clone();
    let mirror_cells = removed
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .cells
        .clone();
    assert_tombstone(&removed);

    let (opened, findings) = crate::support::prepare_open(&removed, &[])
        .expect("Open accepts a paired matrix with a selected tombstone");
    assert!(findings.is_empty());
    assert_tombstone(&opened);
    assert_eq!(
        cell(&opened, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "left-keys")
            .unwrap()
            .cells,
        source_cells
    );
    assert_eq!(
        opened
            .matrices
            .iter()
            .find(|matrix| matrix.id == "right-keys")
            .unwrap()
            .cells,
        mirror_cells
    );

    let source_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "matrix/left-keys/r0c1".into(),
            at: Vec2 { x: 43.0, y: 17.0 },
        }],
    };
    let (source_edited, _) = prepare_transaction(&opened, &[], &source_move)
        .expect("a source-side edit reconciles around the tombstone");
    assert_tombstone(&source_edited);
    let (opened_source_edit, findings) = crate::support::prepare_open(&source_edited, &[])
        .expect("Open preserves the tombstone after a source-side edit");
    assert!(findings.is_empty());
    assert_tombstone(&opened_source_edit);
    assert_eq!(
        cell(&opened_source_edit, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened_source_edit, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );

    let mirror_move = EditOperation::MoveParts {
        positions: vec![Position {
            id: "matrix/right-keys/r0c1".into(),
            at: Vec2 { x: 71.0, y: 83.0 },
        }],
    };
    let (mirror_edited, _) = prepare_transaction(&opened_source_edit, &[], &mirror_move)
        .expect("a mirror-side edit preserves the tombstone");
    assert_tombstone(&mirror_edited);
    let (opened_mirror_edit, findings) = crate::support::prepare_open(&mirror_edited, &[])
        .expect("Open preserves the tombstone after a mirror-side edit");
    assert!(findings.is_empty());
    assert_tombstone(&opened_mirror_edit);
    assert_eq!(
        cell(&opened_mirror_edit, "left-keys", (0, 0)),
        cell(&removed, "left-keys", (0, 0))
    );
    assert_eq!(
        cell(&opened_mirror_edit, "right-keys", (0, 0)),
        cell(&removed, "right-keys", (0, 0))
    );
    let before_repeat = serde_json::to_vec(&opened_mirror_edit).unwrap();
    let (repeated, findings) = crate::support::prepare_open(&opened_mirror_edit, &[])
        .expect("repeated Open remains valid");
    assert!(findings.is_empty());
    assert_eq!(repeated, opened_mirror_edit);
    assert_eq!(serde_json::to_vec(&repeated).unwrap(), before_repeat);
}
