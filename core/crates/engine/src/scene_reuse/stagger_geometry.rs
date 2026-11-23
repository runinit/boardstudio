use super::*;

#[test]
fn first_column_stagger_preserves_rotated_precision_and_gap_wire() {
    let mut saw_gap = false;
    for columns in [None, Some(7), Some(12)] {
        let mut document = stagger_document(columns);
        let matrix = &mut document.matrices[0];
        matrix.rotation = Some(17.375);
        matrix.origin = Vec2 {
            x: -74.0005,
            y: 35.1255,
        };
        matrix.column_layout = vec![
            ColumnLayout::default(),
            ColumnLayout {
                splay: -8.123_456,
                stagger: 3.012_345,
                ..Default::default()
            },
        ];
        let OutlineFeature::PartEnvelope {
            settings, margin, ..
        } = &mut document.outline[0]
        else {
            panic!("expected a generated perimeter");
        };
        *margin = 3.9995;
        settings.repair = Some(OutlineRepairSettings {
            maximum_gap_span: 45.0,
            enabled: false,
            ..Default::default()
        });
        let (mut engine, mut cold) = open_preview_fixture(document);
        let mut changed = engine.document.matrices[0].part_ids.clone();
        changed.push("matrix".into());
        changed.sort();
        for value in [1.000_499_9, 1.000_500_1, -6.872_500_1, 29.124_499_9] {
            let Ok(EditOutcome::Preview(scene)) =
                compare(&mut engine, &mut cold, preview_op(0, stagger(0, value)))
            else {
                panic!("expected a stagger preview");
            };
            assert_eq!(scene.changed_ids, changed);
            assert!(engine.stagger_preview.is_some());
            assert!(!scene.contours.is_empty());
            saw_gap |= scene
                .board_outline_scenes
                .iter()
                .any(|outline| !outline.gaps.is_empty());
        }
        assert_preview_preserves_snapshot(&mut engine, &mut cold);
    }
    assert!(
        saw_gap,
        "the differential must exercise generated gap metadata"
    );
}

#[test]
fn stagger_reuses_only_matching_settled_preview_geometry_and_validation() {
    for document in [fixture(), stagger_document(Some(12))] {
        let (mut engine, mut cold) = open_preview_fixture(document);
        let accepted = engine.document.clone();
        let matrix = &accepted.matrices[0];
        let column = matrix.columns - 1;
        let preview = |revision, column, value| preview_op(revision, stagger(column, value));
        let mut runs = engine.stagger_outline_projections.get();
        for (start, value, projected) in [
            (column, 1.25, true),
            (column, 1.25, false),
            (column, 2.75, true),
            (0, 2.0, true),
            (column / 2, -3.375, true),
            (column, 4.0, true),
        ] {
            let Ok(EditOutcome::Preview(scene)) =
                compare(&mut engine, &mut cold, preview(0, start, value))
            else {
                panic!("expected a stagger preview");
            };
            assert!(!scene.contours.is_empty());
            let mut changed = vec!["matrix".into()];
            for row in 0..matrix.rows {
                for col in 0..matrix.columns {
                    let id = format!("matrix/matrix/r{row}c{col}");
                    let before = accepted.parts.iter().find(|part| part.id == id).unwrap();
                    let transform = scene.transforms.iter().find(|pose| pose.id == id).unwrap();
                    let mut expected = before.pose;
                    if col >= start {
                        changed.push(id.clone());
                        expected.at.y += value;
                    }
                    assert_eq!(transform.pose, expected, "{id}");
                    if matrix.columns == 2 {
                        let finding_id = format!("board:main:outline:body:{id}");
                        assert!(scene.findings.iter().any(|finding| {
                            finding.id == finding_id
                                && finding.target_ids == ["main", id.as_str()]
                                && finding.severity == Severity::Error
                        }));
                        let marker = scene
                            .finding_markers
                            .iter()
                            .find(|marker| marker.finding_id == finding_id)
                            .unwrap();
                        assert_eq!(marker.board_id, "main");
                        let (sin, cos) = expected.rotation.to_radians().sin_cos();
                        assert_eq!(
                            marker.contours[0].points[0],
                            Vec2 {
                                x: expected.at.x - 9.0 * cos + 9.0 * sin,
                                y: expected.at.y - 9.0 * sin - 9.0 * cos,
                            }
                        );
                    }
                }
            }
            changed.sort();
            assert_eq!(scene.changed_ids, changed);
            runs += usize::from(projected);
            assert_eq!(engine.stagger_outline_projections.get(), runs);
        }
        assert_eq!(engine.document, accepted);
        assert!(engine.undo.is_empty() && engine.redo.is_empty());

        // Reopening at the same revision with the same poses must retire the
        // Preview even though only a geometry source has changed.
        let mut reopened = engine.document.clone();
        let OutlineFeature::PartEnvelope { margin, .. } = &mut reopened.outline[0] else {
            panic!("expected a generated perimeter");
        };
        *margin += 1.0;
        let _ = compare(&mut engine, &mut cold, open_req(reopened));
        assert!(engine.stagger_preview.is_none());
        assert!(matches!(
            compare(&mut engine, &mut cold, preview(0, column, 4.0)),
            Ok(EditOutcome::Preview(_))
        ));
        runs += 1;
        assert_eq!(engine.stagger_outline_projections.get(), runs);
        // A read between Preview and Commit may refresh the receipt but does
        // not change the current projection's inputs.
        let _ = compare(&mut engine, &mut cold, ReuseOp::Snapshot);
        assert!(matches!(
            compare(&mut engine, &mut cold, edit(0, stagger(column, 4.0))),
            Ok(EditOutcome::Committed(CommittedScene { scene, .. })) if scene.revision == 1
        ));
        assert_eq!(engine.stagger_outline_projections.get(), runs);
        assert!(engine.stagger_preview.is_none());
        assert_eq!(engine.undo.len(), 1);

        assert!(matches!(
            compare(&mut engine, &mut cold, preview(1, column, 3.0)),
            Ok(EditOutcome::Preview(_))
        ));
        assert!(engine.stagger_preview.is_some());
        assert!(compare(&mut engine, &mut cold, preview(0, column, 8.0)).is_err());
        let _ = compare(&mut engine, &mut cold, ReuseOp::Undo);
        assert!(engine.stagger_preview.is_none());
        assert!(matches!(
            compare(&mut engine, &mut cold, preview(2, column, 3.0)),
            Ok(EditOutcome::Preview(_))
        ));
        assert!(engine.stagger_preview.is_some());
        let _ = compare(&mut engine, &mut cold, ReuseOp::Redo);
        assert!(engine.stagger_preview.is_none());
    }
}
