use super::*;

fn exception(document: &ProjectDoc, kind: &str, id: &str, state: &str) -> EditOperation {
    serde_json::from_value(json!({"kind":"set-half-exception",
        "pairId": document.hardware.as_ref().unwrap().pairs[0].id,
        "itemKind":kind, "mirrorId":id, "state":state}))
    .unwrap()
}

#[test]
fn cutover_replay_compensation_survives_shifted_local_key() {
    let document = pair_document();
    let key = "matrix/right-keys/r1c1";
    let (before, _) =
        prepare_transaction(&document, &[], &exception(&document, "part", key, "local")).unwrap();
    let (after, _) = prepare_transaction(
        &before,
        &[],
        &EditOperation::RemoveMatrixColumn {
            matrix_id: "left-keys".into(),
            column: 0,
        },
    )
    .unwrap();
    let mut regenerated = after.clone();
    boardstudio_core_kernel::matrix::set_matrix(
        &mut regenerated,
        after.matrices.by_id("right-keys"),
    )
    .unwrap();
    for suffix in ["", "/diode"] {
        let old = format!("{key}{suffix}");
        let new = format!("matrix/right-keys/r1c0{suffix}");
        for doc in [&after, &regenerated] {
            assert_eq!(part(doc, &new).pose, part(&before, &old).pose, "{new}");
        }
    }
}

#[test]
fn cutover_lifecycle_rejects_linked_slot_outside_matrix_membership() {
    let mut document = pair_document();
    for side in ["left", "right"] {
        let mut orphan = part(&document, &format!("{side}/U1")).clone();
        orphan.id = format!("matrix/{side}-keys/r9c9");
        orphan.reference = format!("{side}_orphan");
        document
            .boards
            .by_id_mut(side)
            .part_ids
            .push(orphan.id.clone());
        document.parts.push(orphan);
    }
    document.first_halves_mut().correspondences.push(
        serde_json::from_value(json!({
            "kind":"part", "sourceId":"matrix/left-keys/r9c9", "mirrorId":"matrix/right-keys/r9c9"
        }))
        .unwrap(),
    );
    let ordinary = EditOperation::MoveParts { positions: vec![] };
    assert!(
        prepare_transaction(&document, &[], &ordinary).is_err(),
        "ordinary edits must also reject the orphaned slot"
    );
    let op = exception(&document, "part", "matrix/right-keys/r9c9", "local");
    let error = prepare_transaction(&document, &[], &op).unwrap_err();
    assert!(
        error.contains("matrix") || error.contains("Matrix"),
        "{error}"
    );
}

#[test]
fn cutover_rejoin_adopts_owned_overlap() {
    check_growth(true);
}

#[test]
fn cutover_rejoin_refuses_unrelated_collision() {
    check_growth(false);
}

fn check_growth(owned: bool) {
    let start = pair_document();
    let (mut doc, _) = prepare_transaction(
        &start,
        &[],
        &exception(&start, "matrix", "right-keys", "local"),
    )
    .unwrap();
    for (id, columns) in [("left-keys", 3), ("right-keys", 4)] {
        if !owned && id == "right-keys" {
            let mut foreign = part(&doc, "matrix/right-keys/r0c0").clone();
            foreign.id = "matrix/right-keys/r0c2".into();
            foreign.reference = "FOREIGN".into();
            doc.boards
                .by_id_mut("right")
                .part_ids
                .push(foreign.id.clone());
            doc.parts.push(foreign);
            break;
        }
        let mut matrix = doc.matrices.by_id(id).clone();
        let other = if id == "left-keys" {
            "right-keys"
        } else {
            "left-keys"
        };
        let unchanged = doc.matrices.by_id(other).clone();
        for column in matrix.columns..columns {
            for row in 0..matrix.rows {
                let mut cell = matrix
                    .cells
                    .iter()
                    .find(|c| c.column == 0 && c.row == row)
                    .unwrap()
                    .clone();
                cell.column = column;
                matrix.cells.push(cell);
            }
        }
        matrix.columns = columns;
        let op = serde_json::from_value(json!({"kind":"set-matrix","matrix":matrix})).unwrap();
        doc = prepare_transaction(&doc, &[], &op).unwrap().0;
        assert_eq!(doc.matrices.by_id(other), &unchanged);
    }
    part_mut(&mut doc, "matrix/left-keys/r0c2").properties = Some(
        std::collections::BTreeMap::from([("new-source".into(), json!(true))]),
    );
    let result = prepare_transaction(
        &doc,
        &[],
        &exception(&doc, "matrix", "right-keys", "linked"),
    );
    if !owned {
        assert!(result.is_err(), "an unrelated Part cannot be adopted");
        return;
    }
    let (after, _) = result.unwrap();
    for row in 0..2 {
        for suffix in ["", "/diode"] {
            let old = format!("matrix/left-keys/r{row}c2{suffix}");
            let new = format!("matrix/right-keys/r{row}c2{suffix}");
            assert_eq!(
                after
                    .first_halves()
                    .correspondences
                    .iter()
                    .filter(|link| link.source_id == old && link.mirror_id == new)
                    .count(),
                1
            );
            assert_eq!(part(&after, &old).properties, part(&after, &new).properties);
        }
        assert!(
            after
                .parts
                .iter()
                .all(|p| !p.id.starts_with(&format!("matrix/right-keys/r{row}c3")))
        );
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&after).unwrap();
    let (repeat, findings) = support::prepare_open(&after, &[]).unwrap();
    assert_eq!(repeat, after);
    assert!(findings.is_empty());
    let (repeat, changed) = prepare_transaction(
        &after,
        &[],
        &exception(&after, "matrix", "right-keys", "linked"),
    )
    .unwrap();
    assert_eq!(repeat, after);
    assert!(changed.is_empty());
}
