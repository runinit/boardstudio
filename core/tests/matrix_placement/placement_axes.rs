use super::*;
use crate::support::fixture;

#[test]
fn stagger_axis_matches_location() {
    axis_case("plain", |_| {});
    axis_case("rotated", |matrix| matrix.rotation = Some(30.0));
    axis_case("mirror_x", |matrix| matrix.mirror = Some(Mirror::X));
    axis_case("mirror_y", |matrix| matrix.mirror = Some(Mirror::Y));
    axis_case("default splay origins", |matrix| {
        let values = vec![10.0, -15.0, 5.0];
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
    });
    // A custom origin on column 1 is "on the column itself" for column 1, "on a
    // later column" for column 0 and "on an earlier column" for column 2.
    axis_case("custom origin on column 1", |matrix| {
        {
            let values = vec![10.0, -15.0, 5.0];
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
            let values = (vec![None, Some(Vec2 { x: 25.0, y: -12.0 }), None])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
    });
}

#[test]
fn column_stagger_moves_this_and_following_columns() {
    let (mut engine, _, doc) = fixture(1, 3);
    let mut matrix = doc.matrices[0].clone();
    {
        let values = vec![0.0, 2.0, -1.0];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    let (_, doc) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
    let before: Vec<_> = (0..3)
        .map(|column| at(&doc, &member("main", 0, column)))
        .collect();
    let (_, after) = commit_edit(&mut engine, &doc, stagger(1, 5.0));
    assert_eq!(
        after.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 5.0, -1.0]
    );
    assert_eq!(at(&after, &member("main", 0, 0)), before[0]);
    assert!(close(
        delta(before[1], at(&after, &member("main", 0, 1))),
        Vec2 { x: 0.0, y: 3.0 }
    ));
    assert!(close(
        delta(before[2], at(&after, &member("main", 0, 2))),
        Vec2 { x: 0.0, y: 3.0 }
    ));
}

#[test]
fn column_offset_moves_only_its_column() {
    let (mut engine, _, doc) = fixture(2, 3);
    let before: Vec<_> = (0..2u32)
        .flat_map(|row| (0..3u32).map(move |column| (row, column)))
        .map(|(row, column)| at(&doc, &member("main", row, column)))
        .collect();
    let (_, after) = commit_edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 1,
                value: Vec2 { x: 4.0, y: -2.0 },
            },
        },
    );
    let mut index = 0;
    for row in 0..2u32 {
        for column in 0..3u32 {
            let moved = delta(before[index], at(&after, &member("main", row, column)));
            index += 1;
            if column == 1 {
                assert!(close(moved, Vec2 { x: 4.0, y: -2.0 }), "{row}/{column}");
            } else {
                assert!(close(moved, Vec2::default()), "{row}/{column}");
            }
        }
    }
    assert_eq!(
        after.matrices[0].column_layout[1].offset,
        Vec2 { x: 4.0, y: -2.0 }
    );
}

#[test]
fn row_offset_moves_only_its_row() {
    let (mut engine, _, doc) = fixture(2, 3);
    let before: Vec<_> = (0..2u32)
        .flat_map(|row| (0..3u32).map(move |column| (row, column)))
        .map(|(row, column)| at(&doc, &member("main", row, column)))
        .collect();
    let (_, after) = commit_edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::RowOffset {
                row: 1,
                value: Vec2 { x: 3.0, y: 5.0 },
            },
        },
    );
    let mut index = 0;
    for row in 0..2u32 {
        for column in 0..3u32 {
            let moved = delta(before[index], at(&after, &member("main", row, column)));
            index += 1;
            if row == 1 {
                assert!(close(moved, Vec2 { x: 3.0, y: 5.0 }), "{row}/{column}");
            } else {
                assert!(close(moved, Vec2::default()), "{row}/{column}");
            }
        }
    }
    assert_eq!(
        after.matrices[0].row_layout[1].offset,
        Vec2 { x: 3.0, y: 5.0 }
    );
}

#[test]
fn companions_follow_placement_changes() {
    let (mut engine, before) = companion_fixture();
    let key_before = at(&before, &member("main", 0, 0));
    let diode_before = at(&before, &companion("main", 0, 0, "diode"));
    let led_before = at(&before, &companion("main", 0, 0, "led"));
    let (_, after) = commit_edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 0,
                value: Vec2 { x: 4.0, y: 7.0 },
            },
        },
    );
    let key_delta = delta(key_before, at(&after, &member("main", 0, 0)));
    assert!(close(key_delta, Vec2 { x: 4.0, y: 7.0 }));
    assert!(close(
        delta(diode_before, at(&after, &companion("main", 0, 0, "diode"))),
        key_delta
    ));
    assert!(close(
        delta(led_before, at(&after, &companion("main", 0, 0, "led"))),
        key_delta
    ));
    // An explicit constraint takes priority over the attachment.
    assert!(close(
        at(&after, &companion("main", 0, 0, "sensor")),
        Vec2 { x: 1.0, y: -4.0 }
    ));
    assert_eq!(after.constraints, before.constraints);
}

#[test]
fn invalid_changes_error() {
    let (mut engine, _, doc) = fixture(1, 3);
    let before = snapshot(&mut engine);

    for operation in [
        stagger(3, 1.0),
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 0,
                value: Vec2 {
                    x: f64::NAN,
                    y: 0.0,
                },
            },
        },
        EditOperation::SetMatrixPlacement {
            matrix_id: "missing".into(),
            change: MatrixPlacementChange::RowOffset {
                row: 0,
                value: Vec2::default(),
            },
        },
    ] {
        let reply = engine.handle(fixture::commit("bad", "bad", doc.revision, operation));
        assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
        assert_eq!(snapshot(&mut engine), before);
    }

    // Setting the current value is a valid no-op.
    let (reply, after) = commit_edit(&mut engine, &doc, stagger(0, 0.0));
    assert!(reply.changed_ids.is_empty());
    assert_eq!(after.matrices, doc.matrices);
}

#[test]
fn preview_and_undo() {
    let (mut engine, _, doc) = fixture(1, 3);
    let operation = stagger(0, 2.0);
    let preview = match engine.handle(fixture::edit(
        "preview",
        "preview",
        doc.revision,
        EditPhase::Preview,
        operation.clone(),
    )) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    };
    assert_eq!(snapshot(&mut engine).matrices, doc.matrices);
    let (committed, after) = commit_edit(&mut engine, &doc, operation);
    assert_eq!(after.matrices[0].column_layout[0].stagger, 2.0);
    assert_eq!(preview.matrix_scenes, committed.matrix_scenes);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, doc.matrices);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.matrices, after.matrices);
}

#[test]
fn changed_ids_are_precise() {
    let mut doc = ProjectDoc::empty("placement", "Placement");
    doc.definitions = fixture::companion_definitions();
    doc.parts.push(Part {
        pose: Pose2 {
            at: Vec2 { x: 100.0, y: 100.0 },
            rotation: 0.0,
        },
        ..fixture::part("spare", "switch", "SPARE")
    });
    let (mut engine, _, doc) = open_engine(doc);
    let mut matrix = fixture::matrix("main", 1, 3);
    matrix.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 1,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![MatrixAssembly {
            id: "diode".into(),
            definition_id: "diode".into(),
            offset: Vec2 { x: 2.0, y: 5.0 },
            rotation: None,
            side: None,
        }],
        assemblies_local: None,
    }];
    let (_, doc) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
    let (reply, _) = commit_edit(&mut engine, &doc, stagger(1, 2.0));
    for id in [
        "main".to_string(),
        member("main", 0, 1),
        member("main", 0, 2),
        companion("main", 0, 1, "diode"),
    ] {
        assert!(
            reply.changed_ids.contains(&id),
            "missing {id}: {:?}",
            reply.changed_ids
        );
    }
    assert!(!reply.changed_ids.contains(&member("main", 0, 0)));
    assert!(!reply.changed_ids.iter().any(|id| id == "spare"));
}

#[test]
fn convert_preserves_poses_on_an_unsplayed_column() {
    let matrix = convert_case("middle", 1, |matrix| {
        {
            let values = vec![Vec2::default(), Vec2 { x: 1.5, y: -9.525 }, Vec2::default()];
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
        {
            let values = vec![0.0, 2.0, 1.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.stagger = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.stagger = value;
            }
        }
    });
    assert_eq!(
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 2.0 - 9.525, 1.0 + 9.525]
    );
    assert_eq!(matrix.column_layout[1].offset.x, 1.5);
}

#[test]
fn convert_preserves_poses_on_the_last_column_and_a_custom_origin() {
    let matrix = convert_case("last", 2, |matrix| {
        let values = vec![Vec2::default(), Vec2::default(), Vec2 { x: 0.0, y: 4.0 }];
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
    });
    assert_eq!(
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 0.0, 4.0]
    );
    convert_case("custom origin with preceding splay", 1, |matrix| {
        matrix.rotation = Some(20.0);
        matrix.mirror = Some(Mirror::X);
        {
            let values = vec![10.0, 0.0, 5.0];
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
            let values = (vec![None, Some(Vec2 { x: 25.0, y: -12.0 }), None])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        {
            let values = vec![Vec2::default(), Vec2 { x: 0.5, y: 3.0 }, Vec2::default()];
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
    });
    // Converting column 1: a splayed default-origin neighbour (column 2) must not move.
    convert_case("splay on a later column only", 1, |matrix| {
        {
            let values = vec![0.0, 0.0, 12.0];
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
            let values = vec![Vec2::default(), Vec2 { x: 0.0, y: -2.0 }, Vec2::default()];
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
    });
}
