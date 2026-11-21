use super::*;

#[test]
fn swing_origin_changes_only_the_origin_and_swing_reset_restores_poses() {
    let (mut engine, _, doc) = fixture(3, 3);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            rotation: 90.0,
            ..Default::default()
        },
    ];
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let request = |revision, world: serde_json::Value| {
        serde_json::json!({
        "kind":"edit","id":"swing","command":{"baseRevision":revision,"transactionId":format!("swing-{revision}"),"phase":"commit","targetIds":[],
        "operation":{"kind":"set-matrix-splay","matrixId":"main","column":1,"change":{"kind":"origin-swing","world":world}}}
    }).to_string()
    };
    let reply = engine.request(&request(
        before.revision,
        serde_json::json!({"x":22.0,"y":2.0}),
    ));
    let (swing_scene, after) = scene(serde_json::from_str(&reply).unwrap());
    let mut expected = before.matrices[0].clone();
    expected.column_layout[1].splay_origin = Vec2 { x: 3.0, y: 2.0 };
    assert_eq!(after.matrices[0], expected);
    assert!(close(
        piece(&swing_scene, "main").columns[1].splay_origin,
        Vec2 { x: 22.0, y: 2.0 }
    ));
    for row in 0..3 {
        assert!(close(
            delta(
                at(&before, &member("main", row, 1)),
                at(&after, &member("main", row, 1))
            ),
            Vec2 { x: 5.0, y: -1.0 }
        ));
        assert_eq!(
            at(&before, &member("main", row, 2)),
            at(&after, &member("main", row, 2))
        );
    }
    let reply = engine.request(&request(after.revision, serde_json::Value::Null));
    let (_, reset) = scene(serde_json::from_str(&reply).unwrap());
    assert_eq!(reset.matrices, before.matrices);
    assert_eq!(reset.parts, before.parts);
}

#[test]
fn relative_origin_field_holds_keys_by_writing_only_the_nearest_steps() {
    let (mut engine, _, doc) = fixture(3, 3);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            splay: 90.0,
            ..Default::default()
        },
    ];
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let reply=engine.request(&serde_json::json!({"kind":"edit","id":"relative","command":{
        "baseRevision":before.revision,"transactionId":"relative","phase":"commit","targetIds":[],
        "operation":{"kind":"set-matrix-splay","matrixId":"main","column":1,"change":{"kind":"origin-relative","value":{"x":2.0,"y":3.0}}}
    }}).to_string());
    let (_, after) = scene(serde_json::from_str(&reply).unwrap());
    assert_same_poses(&before, &after, "relative field holds");
    assert_eq!(
        after.matrices[0].column_layout[1].splay_origin,
        Vec2 { x: 2.0, y: 3.0 }
    );
    assert_eq!(after.matrices[0].column_layout[1].spread, Some(14.0));
    assert!(close(
        Vec2 {
            x: after.matrices[0].column_layout[1].stagger,
            y: 0.0
        },
        Vec2 { x: -1.0, y: 0.0 }
    ));
    assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
    assert_eq!(after.matrices[0].cells, before.matrices[0].cells);
}

#[test]
fn rigid_column_align_and_origin_hold_use_steps_under_bend_splay_and_mirrors() {
    for mirror in [None, Some(Mirror::X), Some(Mirror::Y)] {
        for column in 0..3 {
            let (mut engine, _, doc) = fixture(3, 3);
            let mut matrix = doc.matrices[0].clone();
            matrix.mirror = mirror;
            matrix.rotation = Some(31.0);
            matrix.origin = Vec2 { x: 7.0, y: -11.0 };
            matrix.row_layout = vec![RowLayout {
                bend: 23.0,
                shift: Vec2 { x: 2.0, y: 3.0 },
                ..Default::default()
            }];
            matrix.column_layout = (0..3)
                .map(|c| ColumnLayout {
                    splay: 17.0,
                    rotation: 13.0,
                    stagger: f64::from(c),
                    splay_origin: Vec2 { x: 3.0, y: -5.0 },
                    ..Default::default()
                })
                .collect();
            let (_, before) = edit(
                &mut engine,
                &doc,
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            );
            let (_, after) = edit(
                &mut engine,
                &before,
                EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change: MatrixPlacementChange::ColumnTranslation {
                        column,
                        delta: Vec2 { x: 4.0, y: -3.0 },
                    },
                },
            );
            for row in 0..3 {
                for col in 0..3 {
                    let id = member("main", row, col);
                    assert!(
                        close(
                            delta(at(&before, &id), at(&after, &id)),
                            if col == column {
                                Vec2 { x: 4.0, y: -3.0 }
                            } else {
                                Vec2::default()
                            }
                        ),
                        "{mirror:?} c{column}: {id}"
                    );
                }
            }
            assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
            assert_eq!(after.matrices[0].cells, before.matrices[0].cells);
            for c in 0..3 {
                assert_eq!(
                    after.matrices[0].column_layout[c].offset,
                    before.matrices[0].column_layout[c].offset
                );
            }
            let (held_scene, held) = edit(
                &mut engine,
                &after,
                EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column,
                    change: MatrixSplayChange::Origin {
                        world: Some(Vec2 { x: 43.0, y: -27.0 }),
                    },
                },
            );
            assert!(close(
                piece(&held_scene, "main").columns[column as usize].splay_origin,
                Vec2 { x: 43.0, y: -27.0 }
            ));
            assert_same_poses(&after, &held, "mirrored curved hold");
            for c in 0..3 {
                if c != column as usize && c != column as usize + 1 {
                    assert_eq!(
                        held.matrices[0].column_layout[c],
                        after.matrices[0].column_layout[c]
                    );
                }
            }
            let (_, reset) = edit(
                &mut engine,
                &held,
                EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column,
                    change: MatrixSplayChange::Origin { world: None },
                },
            );
            assert_same_poses(&held, &reset, "reset holds");
        }
    }
}

#[test]
fn exact_align_refuses_to_move_a_local_key_without_changing_state() {
    let (_, mut doc) = linked_pair();
    doc.layouts
        .iter_mut()
        .find_map(|l| l.mirror_link.as_mut())
        .unwrap()
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("right-matrix", 0, 1),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (mut engine, _, before) = open(doc);
    let reply = engine.handle(CoreRequest::Edit {
        id: "align".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "align".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetMatrixPlacement {
                matrix_id: "right-matrix".into(),
                change: MatrixPlacementChange::ColumnTranslation {
                    column: 1,
                    delta: Vec2 { x: 4.0, y: -3.0 },
                },
            },
        },
    });
    assert!(
        matches!(&reply,CoreReply::Error {message,..} if message.contains("Local")),
        "{reply:?}"
    );
    assert_eq!(snapshot(&mut engine), before);
    assert!(
        matches!(engine.handle(CoreRequest::Undo {id:"undo".into()}),CoreReply::Error {message,..} if message.contains("History is empty"))
    );
}

#[test]
fn named_placement_boundary_refuses_nonfinite_and_out_of_bounds_values_atomically() {
    let (mut engine, _, before) = fixture(3, 3);
    for change in [
        MatrixPlacementChange::ColumnSpread {
            column: 0,
            value: Some(3.0),
        },
        MatrixPlacementChange::ColumnSpread {
            column: 3,
            value: Some(3.0),
        },
        MatrixPlacementChange::ColumnSpread {
            column: 1,
            value: Some(f64::NAN),
        },
        MatrixPlacementChange::RowPadding {
            row: 0,
            value: Some(3.0),
        },
        MatrixPlacementChange::RowPadding {
            row: 1,
            value: Some(f64::INFINITY),
        },
        MatrixPlacementChange::RowShift {
            row: 3,
            value: Vec2::default(),
        },
        MatrixPlacementChange::RowShift {
            row: 1,
            value: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
        },
        MatrixPlacementChange::RowBend {
            row: 1,
            value: f64::NEG_INFINITY,
        },
        MatrixPlacementChange::KeyShift {
            row: 0,
            column: 3,
            value: Vec2::default(),
        },
        MatrixPlacementChange::KeyShift {
            row: 0,
            column: 0,
            value: Vec2 {
                x: 0.0,
                y: f64::INFINITY,
            },
        },
        MatrixPlacementChange::KeyBend {
            row: 3,
            column: 0,
            value: 0.0,
        },
        MatrixPlacementChange::KeyBend {
            row: 0,
            column: 0,
            value: f64::NAN,
        },
    ] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "invalid".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "invalid".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change,
                },
            },
        });
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(snapshot(&mut engine), before);
    }
    for change in [
        MatrixSplayChange::OriginRelative {
            value: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
        },
        MatrixSplayChange::OriginSwing {
            world: Some(Vec2 {
                x: 0.0,
                y: f64::INFINITY,
            }),
        },
        MatrixSplayChange::Origin {
            world: Some(Vec2 {
                x: f64::NEG_INFINITY,
                y: 0.0,
            }),
        },
    ] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "invalid-origin".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "invalid-origin".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column: 1,
                    change,
                },
            },
        });
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(snapshot(&mut engine), before);
    }
}

#[test]
fn carried_bend_carries_authored_companions_and_keeps_constraint_precedence() {
    let (mut engine, before) = companion_fixture();
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::KeyBend {
                row: 0,
                column: 0,
                value: 90.0,
            },
        },
    );
    assert!(close(
        at(&after, &companion("main", 0, 0, "led")),
        Vec2 { x: -8.0, y: -10.0 }
    ));
    assert!(close(
        at(&after, &companion("main", 0, 0, "diode")),
        Vec2 { x: -5.0, y: 2.0 }
    ));
    assert_eq!(
        at(&after, &companion("main", 0, 0, "sensor")),
        at(&before, &companion("main", 0, 0, "sensor"))
    );
    assert_eq!(after.constraints, before.constraints);
    assert_eq!(
        after.matrices[0].cells[0].assemblies,
        before.matrices[0].cells[0].assemblies
    );
}

#[test]
fn exact_column_align_refuses_external_constraint_without_retuning_it() {
    let (_, _, mut document) = fixture(2, 2);
    document.constraints.push(Constraint::Offset {
        id: "held".into(),
        source_part_id: member("main", 0, 0),
        target_part_id: member("main", 1, 1),
        offset: Vec2 { x: 19.0, y: 19.0 },
        rotation: 0.0,
    });
    let (mut engine, _, before) = open(document);
    for phase in [EditPhase::Preview, EditPhase::Commit] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "align".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "align".into(),
                phase,
                target_ids: vec![],
                operation: EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change: MatrixPlacementChange::ColumnTranslation {
                        column: 1,
                        delta: Vec2 { x: 4.0, y: -3.0 },
                    },
                },
            },
        });
        assert!(matches!(reply,CoreReply::Error {message,..} if message.contains("constraint")));
        assert_eq!(snapshot(&mut engine), before);
    }
    assert!(
        matches!(engine.handle(CoreRequest::Undo {id:"refusal-history".into()}),CoreReply::Error {message,..} if message.contains("History is empty"))
    );
}

#[test]
fn swing_origin_at_column_zero_uses_mirrored_incoming_axes() {
    for (mirror, expected) in [
        (Mirror::X, Vec2 { x: -3.0, y: 2.0 }),
        (Mirror::Y, Vec2 { x: 3.0, y: -2.0 }),
    ] {
        let (mut engine, _, doc) = fixture(2, 2);
        let mut matrix = doc.matrices[0].clone();
        matrix.mirror = Some(mirror);
        matrix.origin = Vec2 { x: 7.0, y: -11.0 };
        matrix.column_layout = vec![ColumnLayout {
            rotation: 90.0,
            ..Default::default()
        }];
        let (_, before) = edit(
            &mut engine,
            &doc,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        );
        let (_, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixSplay {
                matrix_id: "main".into(),
                column: 0,
                change: MatrixSplayChange::OriginSwing {
                    world: Some(Vec2 { x: 10.0, y: -9.0 }),
                },
            },
        );
        assert_eq!(after.matrices[0].column_layout[0].splay_origin, expected);
        assert!(close(
            at(&after, &member("main", 0, 0)),
            Vec2 { x: 8.0, y: -6.0 }
        ));
        assert_eq!(after.matrices[0].origin, before.matrices[0].origin);
        assert_eq!(after.matrices[0].column_layout.len(), 1);
    }
}

#[test]
fn matrix_translation_preserves_internal_constraint_without_retuning() {
    internal_constraint_translation(None);
}

#[test]
fn column_translation_preserves_internal_constraint_without_retuning() {
    internal_constraint_translation(Some(1));
}
