use super::*;
use crate::support::fixture;

#[test]
fn convert_errors_on_a_splayed_default_origin_and_changes_nothing() {
    let (mut engine, _, doc) = fixture(1, 3);
    let mut matrix = doc.matrices[0].clone();
    {
        let values = vec![0.0, 15.0, 0.0];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.splay = 0.0;
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    }
    {
        let values = vec![Vec2::default(), Vec2 { x: 0.0, y: 3.0 }, Vec2::default()];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    let (_, before) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
    let reason = "Converting requires zero Splay, Column rotation and carried Bend";
    assert_eq!(
        convert_offset_available(&before.matrices[0], 1),
        Err(reason.to_string())
    );
    let reply = engine.handle(fixture::commit(
        "convert",
        "convert",
        before.revision,
        convert("main", 1),
    ));
    match reply {
        CoreReply::Error { message, .. } => assert_eq!(message, reason),
        other => panic!("expected an error: {other:?}"),
    }
    assert_eq!(snapshot(&mut engine), before);
}

/// Save and reopen: the placement parameters survive a JSON round trip through a fresh
/// engine with nothing normalized, including an old-toolbar `column_offsets[c].y`.
#[test]
fn placement_survives_save_and_reopen_unnormalized() {
    let mut doc = ProjectDoc::empty("roundtrip", "Roundtrip");
    doc.definitions = fixture::companion_definitions();
    let (mut engine, _, doc) = open_engine(doc);
    let mut configured = companion_matrix();
    configured.rows = 2;
    configured.columns = 3;
    {
        let values = vec![0.0, 4.5, -2.0];
        configured.column_layout.resize_with(
            values.len().max(configured.column_layout.len()),
            Default::default,
        );
        for record in &mut configured.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in configured.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    {
        let values = vec![
            Vec2::default(),
            Vec2 { x: 0.0, y: -9.525 },
            Vec2 { x: 1.5, y: 0.0 },
        ];
        configured.column_layout.resize_with(
            values.len().max(configured.column_layout.len()),
            Default::default,
        );
        for record in &mut configured.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in configured.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    {
        let values = vec![Vec2::default(), Vec2 { x: 2.0, y: 1.0 }];
        let count = values.len().max(configured.row_layout.len());
        configured.row_layout.resize_with(count, Default::default);
        for record in &mut configured.row_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in configured.row_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    configured.cells[0].offset = Some(Vec2 { x: 0.75, y: -0.5 });
    configured.cells[0].rotation = Some(5.0);
    let (_, laid_out) = commit_edit(&mut engine, &doc, fixture::set_matrix(configured));

    let saved = serde_json::to_string(&laid_out).unwrap();
    let reloaded: ProjectDoc = serde_json::from_str(&saved).unwrap();
    let (mut reopened_engine, _, reopened) = open_engine(reloaded);
    let again = snapshot(&mut reopened_engine);

    assert_eq!(laid_out.matrices, reopened.matrices, "matrix values");
    assert_eq!(laid_out.matrices, again.matrices, "after the open settles");
    assert_same_poses(&laid_out, &reopened, "reopened poses");
    assert_same_poses(&laid_out, &again, "settled poses");
    assert_eq!(
        reopened.matrices[0].column_layout[1].offset,
        Vec2 { x: 0.0, y: -9.525 },
        "the old-toolbar offset is kept, not converted"
    );
}

#[test]
fn curved_key_world_move_changes_only_its_independent_offset() {
    let (mut engine, _, doc) = fixture(3, 2);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![ColumnLayout {
        rotation: 90.0,
        ..Default::default()
    }];
    matrix.row_layout = vec![RowLayout {
        bend: 90.0,
        ..Default::default()
    }];
    let (_, before) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
    let id = member("main", 1, 0);
    let start = at(&before, &id);
    let (changed, after) = commit_edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: id.clone(),
                at: Vec2 {
                    x: start.x + 4.0,
                    y: start.y - 3.0,
                },
            }],
        },
    );
    assert!(close(
        at(&after, &id),
        Vec2 {
            x: start.x + 4.0,
            y: start.y - 3.0
        }
    ));
    for part in &before.parts {
        if part.id != id {
            assert_eq!(after.parts.iter().find(|p| p.id == part.id).unwrap(), part);
        }
    }
    let cell = after.matrices[0]
        .cells
        .iter()
        .find(|c| c.row == 1 && c.column == 0)
        .unwrap();
    assert!(close(cell.offset.unwrap(), Vec2 { x: -4.0, y: 3.0 }));
    assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
    assert_eq!(
        after.matrices[0].column_layout,
        before.matrices[0].column_layout
    );
    assert_eq!(changed.changed_ids, vec!["main".to_owned(), id]);
}

#[test]
fn named_row_shift_carries_later_keys_without_changing_offsets() {
    let (mut engine, _, before) = fixture(3, 2);
    let reply = engine.request(&serde_json::json!({
        "kind": "edit", "id": "shift", "command": {
            "baseRevision": before.revision, "transactionId": "shift", "phase": "commit", "targetIds": [],
            "operation": { "kind": "set-matrix-placement", "matrixId": "main",
                "change": { "kind": "row-shift", "row": 1, "value": { "x": 4.0, "y": -3.0 } } }
        }
    }).to_string());
    let (changed, after) = scene(serde_json::from_str(&reply).unwrap());
    for row in 0..3 {
        for column in 0..2 {
            let id = member("main", row, column);
            assert!(close(
                delta(at(&before, &id), at(&after, &id)),
                if row == 0 {
                    Vec2::default()
                } else {
                    Vec2 { x: 4.0, y: -3.0 }
                }
            ));
        }
    }
    assert_eq!(
        after.matrices[0].row_layout[1].shift,
        Vec2 { x: 4.0, y: -3.0 }
    );
    assert!(after.matrices[0].cells.iter().all(|c| c.offset.is_none()));
    assert_eq!(changed.changed_ids.len(), 5);
}

#[test]
fn unchanged_splay_and_base_origin_do_not_write_or_create_history() {
    let (mut engine, _, before) = fixture(2, 2);
    for change in [
        MatrixSplayChange::Angle {
            angle: 0.0,
            affect: MatrixSplayAffect::Column,
        },
        MatrixSplayChange::Angle {
            angle: 0.0,
            affect: MatrixSplayAffect::Following,
        },
        MatrixSplayChange::Origin { world: None },
    ] {
        let (changed, after) = commit_edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixSplay {
                matrix_id: "main".into(),
                column: 1,
                change,
            },
        );
        assert_eq!(after, before);
        assert!(changed.changed_ids.is_empty());
    }
    for operation in [
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 0),
                at: at(&before, &member("main", 0, 0)),
            }],
        },
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnTranslation {
                column: 1,
                delta: Vec2::default(),
            },
        },
    ] {
        let (changed, after) = commit_edit(&mut engine, &before, operation);
        assert_eq!(after, before);
        assert!(changed.changed_ids.is_empty());
    }
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(
        undone.matrices.is_empty(),
        "no-op edits must leave the setup as the only Undo entry"
    );
    let (_, _, mut authored) = fixture(1, 1);
    authored.parts[0].properties =
        Some(serde_json::from_value(serde_json::json!({"layoutOverride":1})).unwrap());
    let (mut engine, _, before) = open_engine(authored);
    let id = member("main", 0, 0);
    let (_, after) = commit_edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: id.clone(),
                at: at(&before, &id),
            }],
        },
    );
    assert_eq!(after, before, "a no-op must retain the authored key record");
}

#[test]
fn named_spread_padding_and_bends_preserve_independent_values_and_identity() {
    let (mut engine, _, before) = fixture(3, 3);
    let (_, spread) = wire_placement(
        &mut engine,
        &before,
        serde_json::json!({"kind":"column-spread","column":1,"value":23.0}),
    );
    assert_eq!(at(&spread, &member("main", 0, 1)), Vec2 { x: 23.0, y: 0.0 });
    assert_eq!(at(&spread, &member("main", 0, 2)), Vec2 { x: 42.0, y: 0.0 });
    let (_, padded) = wire_placement(
        &mut engine,
        &spread,
        serde_json::json!({"kind":"row-padding","row":1,"value":7.0}),
    );
    assert_eq!(at(&padded, &member("main", 2, 0)), Vec2 { x: 0.0, y: 26.0 });
    let (_, bent) = wire_placement(
        &mut engine,
        &padded,
        serde_json::json!({"kind":"row-bend","row":1,"value":90.0}),
    );
    assert!(close(
        at(&bent, &member("main", 2, 0)),
        Vec2 { x: -19.0, y: 7.0 }
    ));
    let (_, shifted) = wire_placement(
        &mut engine,
        &bent,
        serde_json::json!({"kind":"key-shift","row":1,"column":0,"value":{"x":3.0,"y":2.0}}),
    );
    assert!(close(
        at(&shifted, &member("main", 2, 0)),
        Vec2 { x: -16.0, y: 9.0 }
    ));
    assert_eq!(
        at(&shifted, &member("main", 1, 1)),
        Vec2 { x: 23.0, y: 7.0 }
    );
    let (_, turned) = wire_placement(
        &mut engine,
        &shifted,
        serde_json::json!({"kind":"key-bend","row":1,"column":0,"value":90.0}),
    );
    assert!(close(
        at(&turned, &member("main", 2, 0)),
        Vec2 { x: 3.0, y: -10.0 }
    ));
    assert_eq!(turned.matrices[0].pitch, before.matrices[0].pitch);
    assert_eq!(turned.matrices[0].part_ids, before.matrices[0].part_ids);
    for part in &turned.parts {
        let original = before.parts.iter().find(|p| p.id == part.id).unwrap();
        assert_eq!(part.reference, original.reference);
        assert_eq!(part.definition_id, original.definition_id);
    }
    let (unchanged, same) = wire_placement(
        &mut engine,
        &turned,
        serde_json::json!({"kind":"key-bend","row":1,"column":0,"value":90.0}),
    );
    assert_eq!(same, turned);
    assert!(unchanged.changed_ids.is_empty());
    let (_, defaulted) = wire_placement(
        &mut engine,
        &same,
        serde_json::json!({"kind":"column-spread","column":1,"value":null}),
    );
    assert_eq!(defaulted.matrices[0].column_layout[1].spread, None);
    assert_eq!(
        at(&defaulted, &member("main", 0, 1)),
        Vec2 { x: 19.0, y: 0.0 }
    );
}

#[test]
fn hold_origin_solves_the_final_point_with_partial_follow_and_refuses_atomically() {
    let (mut engine, _, doc) = open_engine(matrix_doc());
    let (_, doc) = commit_edit(
        &mut engine,
        &doc,
        fixture::set_matrix(fixture::matrix("anchor", 1, 1)),
    );
    let mut target = fixture::matrix("main", 3, 3);
    target.origin = Vec2 { x: 9.0, y: 3.0 };
    target.column_layout = vec![ColumnLayout {
        rotation: 90.0,
        ..Default::default()
    }];
    target.row_layout = vec![RowLayout {
        bend: 35.0,
        ..Default::default()
    }];
    let (_, doc) = commit_edit(&mut engine, &doc, fixture::set_matrix(target));
    let mut doc = doc;
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["anchor".into(), "main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let target = doc.matrices.iter_mut().find(|m| m.id == "main").unwrap();
    target.anchor = Some(MatrixAnchor {
        matrix_id: "anchor".into(),
        row: 0,
        column: 0,
        follow: Follow {
            x: false,
            y: true,
            rotation: true,
        },
        aggregate: None,
    });
    let (mut engine, before_scene, before) = open_engine(doc);
    assert_eq!(
        fixture::matrix_scene(&before_scene, "main").columns[0].splay_origin,
        Vec2 { x: 0.0, y: 3.0 }
    );
    let (after_scene, after) = commit_edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixSplay {
            matrix_id: "main".into(),
            column: 0,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 { x: 2.0, y: 5.0 }),
            },
        },
    );
    assert!(close(
        fixture::matrix_scene(&after_scene, "main").columns[0].splay_origin,
        Vec2 { x: 2.0, y: 5.0 }
    ));
    assert_same_poses(&before, &after, "representable partial-Follow hold");
    let changed = after.matrices.iter().find(|m| m.id == "main").unwrap();
    assert!(close(
        changed.column_layout[0].splay_origin,
        Vec2 { x: 2.0, y: -2.0 }
    ));
    assert!(changed.column_layout[0].spread.is_none());
    assert!(close(changed.origin, Vec2 { x: 9.0, y: 7.0 }));
    let refused = engine.handle(fixture::commit(
        "refuse",
        "refuse",
        after.revision,
        EditOperation::SetMatrixSplay {
            matrix_id: "main".into(),
            column: 0,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 { x: 2.0, y: 6.0 }),
            },
        },
    ));
    assert!(matches!(refused, CoreReply::Error { message, .. } if message.contains("Follow")));
    assert_eq!(snapshot(&mut engine), after);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, before.matrices);
    assert_eq!(undone.parts, before.parts);
}
