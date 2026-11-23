use super::*;
use crate::support::fixture;

#[test]
fn union_difference_make_one_hole() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("left", -5.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline
        .push(fixture::rect("right", 5.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline.push(fixture::rect(
        "hole",
        0.0,
        0.0,
        5.0,
        5.0,
        Operation::Subtract,
    ));
    let (scene, _) = scene(engine.handle(fixture::open(doc)));
    assert_eq!(scene.contours.len(), 2);
    assert_eq!(scene.contours.iter().filter(|c| c.hole).count(), 1);
    assert!(scene.readiness.outline);
    assert!(!scene.readiness.pcb);
}

#[test]
fn duplicate_pad_nets_and_invalid_case_block_readiness() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.definitions.push(PartDefinition {
        pads: vec![Pad {
            id: "p".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Rect,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        }],
        ..fixture::part_definition("part", "Part", PartKind::Custom)
    });
    doc.parts.push(fixture::part("u1", "part", "U1"));
    for id in ["a", "b"] {
        doc.nets.push(Net {
            id: id.into(),
            name: id.into(),
            pins: vec![Pin {
                part_id: "u1".into(),
                pad_id: "p".into(),
            }],
        });
    }
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec!["u1".into()],
        net_ids: vec!["a".into(), "b".into()],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "case".into(),
        name: "Case".into(),
        board_id: "board".into(),
        kind: CaseKind::Tray,
        thickness: -1.0,
        clearance: 0.0,
        material_id: None,
        z: None,
        wall_height: Some(5.0),
        wall_thickness: Some(2.0),
        mounts: None,
        pcb_source: None,
        gasket: None,
    });
    let (bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "bad".into(),
        document: doc.clone(),
    }));
    assert!(!bad.readiness.pcb);
    assert!(
        bad.findings
            .iter()
            .any(|f| f.id == "pad:u1:p:multiple-nets")
    );
    doc.nets.pop();
    doc.boards[0].net_ids.pop();
    let (case_bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "case-bad".into(),
        document: doc,
    }));
    assert!(case_bad.readiness.pcb);
    assert!(!case_bad.readiness.case_ready);
    assert!(
        case_bad
            .findings
            .iter()
            .any(|f| f.id == "case:case:dimensions")
    );
}

#[test]
fn board_contours_follow_each_boards_authored_order() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline.push(fixture::rect(
        "left",
        -20.0,
        0.0,
        10.0,
        10.0,
        Operation::Add,
    ));
    doc.outline.push(fixture::rect(
        "cut",
        -20.0,
        0.0,
        2.0,
        2.0,
        Operation::Subtract,
    ));
    doc.outline.push(fixture::rect(
        "right",
        20.0,
        0.0,
        10.0,
        10.0,
        Operation::Add,
    ));
    doc.boards.push(Board {
        outline_ids: vec!["left".into(), "cut".into()],
        ..fixture::board("left-board", "Left")
    });
    doc.boards.push(Board {
        outline_ids: vec!["right".into()],
        ..fixture::board("right-board", "Right")
    });
    let (scene, _) = scene(engine.handle(fixture::open(doc)));
    assert_eq!(scene.board_contours.len(), 2);
    assert_eq!(scene.board_contours[0].board_id, "left-board");
    assert_eq!(scene.board_contours[0].contours.len(), 2);
    assert!(
        scene.board_contours[0]
            .contours
            .iter()
            .any(|contour| contour.hole)
    );
    assert_eq!(scene.board_contours[1].board_id, "right-board");
    assert_eq!(scene.board_contours[1].contours.len(), 1);
    assert!(
        scene.board_contours[1].contours[0]
            .points
            .iter()
            .all(|point| point.x > 0.0)
    );
    assert!(scene.readiness.pcb);
}

#[test]
fn add_part_accepts_definition_and_selected_layout_atomically() {
    let (mut engine, document) = linked_pair();
    let mut definition = document.definitions[0].clone();
    definition.id = "catalogue/custom-switch".into();
    definition.name = "Custom switch".into();
    let mut part = document.parts[0].clone();
    part.id = "board/custom-switch".into();
    part.definition_id = definition.id.clone();
    part.reference = "SW99".into();
    let operation = EditOperation::AddPart {
        part: part.clone(),
        board_id: Some("board".into()),
        definition: Some(definition.clone()),
        layout_id: Some("left-layout".into()),
    };
    let (_, added) = scene(engine.handle(edit(document.revision, EditPhase::Commit, operation)));
    assert_eq!(
        added
            .definitions
            .iter()
            .filter(|item| item.id == definition.id)
            .count(),
        1
    );
    assert!(added.boards[0].part_ids.contains(&part.id));
    assert!(
        added
            .layouts
            .iter()
            .find(|layout| layout.id == "left-layout")
            .unwrap()
            .part_ids
            .contains(&part.id)
    );
    assert!(added.outline.iter().any(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. } if id == "edge" && part_ids.contains(&part.id))));
}

#[test]
fn add_part_reuses_identical_definition_without_duplicate_registration() {
    let (mut engine, document) = linked_pair();
    let definition = document.definitions[0].clone();
    let mut part = document.parts[0].clone();
    part.id = "board/reused-definition".into();
    part.reference = "REUSE1".into();
    let (_, added) = scene(engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part: part.clone(),
            board_id: Some("board".into()),
            definition: Some(definition.clone()),
            layout_id: Some("left-layout".into()),
        },
    )));
    assert_eq!(
        added
            .definitions
            .iter()
            .filter(|candidate| candidate.id == definition.id)
            .count(),
        1,
        "an identical registered definition is reused"
    );
    assert!(added.boards[0].part_ids.contains(&part.id));
    assert!(
        added
            .layouts
            .iter()
            .find(|layout| layout.id == "left-layout")
            .unwrap()
            .part_ids
            .contains(&part.id)
    );
}

#[test]
fn add_part_refuses_conflicting_definitions_and_invalid_layouts() {
    for (layout_id, conflicting_definition) in [("left-layout", true), ("missing-layout", false)] {
        let (mut engine, document) = linked_pair();
        let definition = conflicting_definition.then(|| {
            let mut definition = document.definitions[0].clone();
            definition.name = "Conflicting replacement".into();
            definition
        });
        let mut part = document.parts[0].clone();
        part.id = format!("board/refusal-{layout_id}");
        part.definition_id = "switch".into();
        part.reference = "REFUSE1".into();
        let reply = engine.handle(edit(
            document.revision,
            EditPhase::Commit,
            EditOperation::AddPart {
                part,
                board_id: Some("board".into()),
                definition,
                layout_id: Some(layout_id.into()),
            },
        ));
        assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
        let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot-after-add-part-refusal".into(),
        }));
        assert_eq!(
            after_refusal, document,
            "definition or layout refusal preserves the complete document"
        );
    }
}

#[test]
fn matrix_shrink_discards_out_of_bounds_edits() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let mut value = matrix(6, 7);
    {
        let values = vec![Vec2 { x: 0.0, y: 0.0 }; 6];
        value
            .row_layout
            .resize_with(values.len().max(value.row_layout.len()), Default::default);
        for record in &mut value.row_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in value.row_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    {
        let values = vec![Vec2 { x: 0.0, y: 0.0 }; 7];
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    value.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        assemblies_local: None,
        row: 5,
        column: 0,
        enabled: true,

        definition_id: None,
        variant: None,
        offset: Some(Vec2 { x: 2.0, y: 0.0 }),
        rotation: None,
        assemblies: vec![],
    }];
    let (_, original) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(value.clone()),
    )));
    value.rows = 5;
    value.columns = 6;
    let (_, shrunk) = scene(engine.handle(edit(
        original.revision,
        EditPhase::Commit,
        fixture::set_matrix(value),
    )));
    assert_eq!(shrunk.matrices[0].row_layout.len(), 5);
    assert_eq!(shrunk.matrices[0].column_layout.len(), 6);
    assert!(shrunk.matrices[0].cells.is_empty());
    assert!(
        !shrunk
            .parts
            .iter()
            .any(|part| part.id == "matrix/main/r5c0")
    );
}

#[test]
fn failed_matrix_preview_restores_document() {
    let mut doc = matrix_doc();
    doc.parts
        .push(fixture::part("matrix/main/r0c1", "switch", "SW99"));
    let mut engine = CoreEngine::new();
    let (_, original) = scene(engine.handle(fixture::open(doc)));
    let reply = engine.handle(edit(
        0,
        EditPhase::Preview,
        fixture::set_matrix(matrix(1, 2)),
    ));
    assert!(matches!(reply, CoreReply::Error { .. }));
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(snapshot, original);
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
}

#[test]
fn matrix_edit_preserves_moved_member_and_prunes_removed() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(2, 2)),
    ));
    let moved_id = "matrix/main/r0c0";
    let (_, moved) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: moved_id.into(),
                at: Vec2 { x: 5.0, y: 7.0 },
            }],
        },
    )));
    assert_eq!(
        moved.matrices[0].cells[0].offset,
        Some(Vec2 { x: 5.0, y: 7.0 })
    );
    assert!(
        !moved.parts[0]
            .properties
            .as_ref()
            .is_some_and(|properties| properties.contains_key("layoutOverride"))
    );
    let mut changed = moved.matrices[0].clone();
    changed.rows = 1;
    changed.origin = Vec2 { x: 100.0, y: 50.0 };
    let (_, doc) = scene(engine.handle(edit(2, EditPhase::Commit, fixture::set_matrix(changed))));
    assert_eq!(doc.parts.len(), 2);
    assert_eq!(
        doc.parts
            .iter()
            .find(|part| part.id == moved_id)
            .unwrap()
            .pose
            .at,
        Vec2 { x: 105.0, y: 57.0 }
    );
    assert_eq!(
        doc.parts
            .iter()
            .find(|part| part.id == "matrix/main/r0c1")
            .unwrap()
            .pose
            .at,
        Vec2 { x: 119.0, y: 50.0 }
    );
    assert_eq!(doc.boards[0].part_ids.len(), 2);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &doc.outline[0] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 2);
    assert!(doc.parts.iter().all(|part| part.id != "matrix/main/r1c0"));
}

#[test]
fn constraint_preview_commit_and_undo() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    for id in ["source", "target"] {
        doc.parts.push(fixture::part(id, "switch", id));
    }
    doc.constraints.push(Constraint::Offset {
        id: "offset".into(),
        source_part_id: "source".into(),
        target_part_id: "target".into(),
        offset: Vec2 { x: 10.0, y: 0.0 },
        rotation: 0.0,
    });
    let (_, opened) = scene(engine.handle(fixture::open(doc)));
    assert_eq!(opened.parts[1].pose.at.x, 10.0);
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "source".into(),
            at: Vec2 { x: 5.0, y: 0.0 },
        }],
    };
    let preview = preview(engine.handle(edit(0, EditPhase::Preview, op.clone())));
    assert!(preview.changed_ids.contains(&"target".into()));
    assert_eq!(
        preview
            .transforms
            .iter()
            .find(|item| item.id == "target")
            .unwrap()
            .pose
            .at
            .x,
        15.0
    );
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts[1]
            .pose
            .at
            .x,
        10.0
    );
    let (_, committed) = scene(engine.handle(edit(0, EditPhase::Commit, op.clone())));
    assert_eq!(committed.parts[1].pose.at.x, 15.0);
    assert!(matches!(
        engine.handle(edit(0, EditPhase::Commit, op)),
        CoreReply::Error { revision: 1, .. }
    ));
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.parts[1].pose.at.x, 10.0);
    let (_, moved) = scene(engine.handle(edit(
        2,
        EditPhase::Commit,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "target".into(),
                at: Vec2 { x: 8.0, y: 0.0 },
            }],
        },
    )));
    assert_eq!(moved.parts[1].pose.at.x, 8.0);
    let (_, restored) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-offset".into(),
    }));
    assert_eq!(restored.parts[1].pose.at.x, 10.0);
}

#[test]
fn matrix_cumulative_stagger_splay_survive_growth_and_serialization() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let mut value = matrix(2, 3);
    value.pitch = Vec2 { x: 20.0, y: 20.0 };
    {
        let values = vec![0.0, 5.0];
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    {
        let values = vec![0.0, 90.0];
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.splay = 0.0;
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    }
    let (_, doc) = scene(engine.handle(edit(0, EditPhase::Commit, fixture::set_matrix(value))));
    let key = doc
        .parts
        .iter()
        .find(|p| p.id == "matrix/main/r1c1")
        .unwrap();
    assert!(key.pose.at.x.abs() < 1e-9);
    assert!((key.pose.at.y - 5.0).abs() < 1e-9);
    assert_eq!(key.pose.rotation, 90.0);
    let reopened: ProjectDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        reopened.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay)
            .collect::<Vec<_>>(),
        vec![0.0, 90.0]
    );
    let mut value = reopened.matrices[0].clone();
    value.columns = 4;
    value.mirror = Some(Mirror::Y);
    let (_, grown) = scene(engine.handle(edit(1, EditPhase::Commit, fixture::set_matrix(value))));
    let key = grown
        .parts
        .iter()
        .find(|p| p.id == "matrix/main/r0c3")
        .unwrap();
    assert!((key.pose.at.x - 20.0).abs() < 1e-9);
    assert!((key.pose.at.y + 45.0).abs() < 1e-9);
    assert_eq!(key.pose.rotation, -90.0);
    let mut value = grown.matrices[0].clone();
    value.columns = 1;
    let (_, shrunk) = scene(engine.handle(edit(2, EditPhase::Commit, fixture::set_matrix(value))));
    assert_eq!(
        shrunk.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0]
    );
    assert_eq!(
        shrunk.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay)
            .collect::<Vec<_>>(),
        vec![0.0]
    );
}

#[test]
fn deleting_matrix_cells_removes_geometry_and_assembly_but_disabled_cells_stay() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let mut value = matrix(1, 3);
    value.cells = serde_json::from_value(serde_json::json!([
        {"row":0,"column":0,"enabled":true,"assemblies":[{"id":"led","definitionId":"switch","offset":{"x":0,"y":4}}]},
        {"row":0,"column":1,"enabled":false}
    ])).unwrap();
    scene(engine.handle(edit(0, EditPhase::Commit, fixture::set_matrix(value))));
    let operation = serde_json::from_value(
        serde_json::json!({"kind":"remove-matrix-cells","matrixId":"main","cells":[[0,0]]}),
    )
    .expect("Core must accept a key deletion operation independently of disable");
    let (geometry, doc) = scene(engine.handle(edit(1, EditPhase::Commit, operation)));
    assert!(
        !doc.parts
            .iter()
            .any(|part| part.id.starts_with("matrix/main/r0c0"))
    );
    assert!(
        !geometry.matrix_scenes[0]
            .cells
            .iter()
            .any(|cell| cell.row == 0 && cell.column == 0)
    );
    assert!(
        geometry.matrix_scenes[0]
            .cells
            .iter()
            .any(|cell| cell.column == 1 && !cell.enabled)
    );
    assert!(
        geometry.matrix_scenes[0]
            .cells
            .iter()
            .any(|cell| cell.column == 2 && cell.enabled)
    );
}

#[test]
fn deleting_matrix_row_remaps_nets_constraints_keycaps_and_direct_wiring() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(3, 1)),
    )));
    let mut probe = doc.parts[0].clone();
    probe.id = "probe".into();
    probe.reference = "PROBE".into();
    doc.parts.push(probe);
    doc.boards[0].part_ids.push("probe".into());
    doc.constraints.push(Constraint::Offset {
        id: "offset".into(),
        source_part_id: "matrix/main/r2c0".into(),
        target_part_id: "probe".into(),
        offset: Vec2 { x: 4.0, y: 2.0 },
        rotation: 0.0,
    });
    doc.nets.push(Net {
        id: "net".into(),
        name: "signal".into(),
        pins: vec![
            Pin {
                part_id: "matrix/main/r1c0".into(),
                pad_id: "1".into(),
            },
            Pin {
                part_id: "matrix/main/r2c0".into(),
                pad_id: "1".into(),
            },
        ],
    });
    doc.keycaps = Some(KeycapConfiguration::default());
    doc.keycaps.as_mut().unwrap().keys.insert(
        "matrix/main/r2c0".into(),
        KeycapKeySettings {
            legend: Some("A".into()),
            ..Default::default()
        },
    );
    doc.hardware = Some(HardwareConfiguration {
        boards: vec![ElectricalBoardConfiguration {
            board_id: "board".into(),
            assignments: [
                ("direct/matrix/main/r2c0".into(), "GPIO1".into()),
                ("direct/matrix/main/r1c0".into(), "GPIO2".into()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }],
        ..Default::default()
    });
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc.clone(),
    });
    let (_, compacted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixRow {
            matrix_id: "main".into(),
            row: 1,
        },
    )));
    assert_eq!(
        compacted.nets[0].pins,
        vec![Pin {
            part_id: "matrix/main/r1c0".into(),
            pad_id: "1".into()
        }]
    );
    assert_eq!(compacted.constraints[0].source(), "matrix/main/r1c0");
    assert_eq!(
        compacted.keycaps.unwrap().keys["matrix/main/r1c0"]
            .legend
            .as_deref(),
        Some("A")
    );
    assert_eq!(
        compacted.hardware.unwrap().boards[0].assignments,
        [("direct/matrix/main/r1c0".into(), "GPIO1".into())]
            .into_iter()
            .collect()
    );
}

#[test]
fn legacy_matrix_cells_default_to_not_deleted_and_false_is_omitted() {
    let cell: MatrixCell =
        serde_json::from_value(serde_json::json!({"row":0,"column":0,"enabled":false})).unwrap();
    assert!(!cell.deleted);
    assert!(serde_json::to_value(cell).unwrap().get("deleted").is_none());
}

#[test]
fn deleting_matrix_key_detaches_inactive_outline_snapshots_in_world_coordinates() {
    for removal in 0..3 {
        for side in [Side::Front, Side::Back] {
            let mut engine = CoreEngine::new();
            engine.handle(fixture::open(matrix_doc()));
            let mut value = matrix(if removal == 2 { 1 } else { 2 }, 1);
            value.origin = Vec2 { x: 100.0, y: 20.0 };
            value.rotation = Some(90.0);
            let (_, mut doc) =
                scene(engine.handle(edit(0, EditPhase::Commit, fixture::set_matrix(value))));
            doc.parts
                .iter_mut()
                .find(|part| part.id == "matrix/main/r0c0")
                .unwrap()
                .side = side.clone();
            let features = vec![
                OutlineFeature::Rect {
                    id: "saved-rect".into(),
                    anchor_part_id: Some("matrix/main/r0c0".into()),
                    center: Vec2 { x: 2.0, y: 3.0 },
                    size: Vec2 { x: 4.0, y: 5.0 },
                    radius: 1.0,
                    rotation: Some(15.0),
                    operation: Operation::Add,
                },
                OutlineFeature::Polygon {
                    id: "saved-polygon".into(),
                    anchor_part_id: Some("matrix/main/r0c0".into()),
                    points: vec![
                        Vec2 { x: 1.0, y: 2.0 },
                        Vec2 { x: 4.0, y: 2.0 },
                        Vec2 { x: 4.0, y: 5.0 },
                    ],
                    operation: Operation::Add,
                },
            ];
            let snapshot = OutlineSnapshot {
                features,
                settings: Default::default(),
                expected_regions: 1,
                bridges: vec![],
                protected_gaps: vec![],
            };
            doc.board_outlines.push(BoardOutline {
                board_id: "board".into(),
                active_version_id: None,
                versions: vec![OutlineVersion {
                    id: "saved".into(),
                    name: "Saved".into(),
                    source: OutlineProvenance {
                        revision: doc.revision,
                        version_id: None,
                    },
                    geometry: snapshot.clone(),
                    ownership: OutlineOwnership::Fixed,
                }],
                generated_last_valid: Some(snapshot),
                retired_version_ids: vec![],
            });
            engine.handle(CoreRequest::Open {
                id: "reopen".into(),
                document: doc.clone(),
            });
            let operation = if removal != 0 {
                EditOperation::RemoveMatrixRow {
                    matrix_id: "main".into(),
                    row: 0,
                }
            } else {
                EditOperation::RemoveMatrixCells {
                    matrix_id: "main".into(),
                    cells: vec![(0, 0)],
                }
            };
            let (_, deleted) =
                scene(engine.handle(edit(doc.revision, EditPhase::Commit, operation)));
            for snapshot in [
                &deleted.board_outlines[0].versions[0].geometry,
                deleted.board_outlines[0]
                    .generated_last_valid
                    .as_ref()
                    .unwrap(),
            ] {
                let OutlineFeature::Rect {
                    anchor_part_id,
                    center,
                    rotation,
                    size,
                    radius,
                    ..
                } = &snapshot.features[0]
                else {
                    panic!("rectangle expected")
                };
                assert_eq!(anchor_part_id, &None);
                assert_eq!(
                    *center,
                    Vec2 {
                        x: 97.0,
                        y: if side == Side::Front { 22.0 } else { 18.0 }
                    },
                    "deleted snapshot anchor must retain rectangle's world center"
                );
                assert_eq!(
                    *rotation,
                    Some(if side == Side::Front { 105.0 } else { 75.0 })
                );
                assert_eq!(*size, Vec2 { x: 4.0, y: 5.0 });
                assert_eq!(*radius, 1.0);
                let OutlineFeature::Polygon {
                    anchor_part_id,
                    points,
                    ..
                } = &snapshot.features[1]
                else {
                    panic!("polygon expected")
                };
                assert_eq!(anchor_part_id, &None);
                assert_eq!(
                    *points,
                    if side == Side::Front {
                        vec![
                            Vec2 { x: 98.0, y: 21.0 },
                            Vec2 { x: 98.0, y: 24.0 },
                            Vec2 { x: 95.0, y: 24.0 },
                        ]
                    } else {
                        vec![
                            Vec2 { x: 98.0, y: 19.0 },
                            Vec2 { x: 98.0, y: 16.0 },
                            Vec2 { x: 95.0, y: 16.0 },
                        ]
                    }
                );
            }
        }
    }
}
