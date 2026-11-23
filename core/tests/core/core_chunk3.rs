use super::*;
use crate::support::fixture;

#[test]
fn pcb_readiness_checks_pads_outlines_and_thickness() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.definitions.push(PartDefinition {
        pads: vec![Pad {
            id: "pad1".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        }],
        ..fixture::part_definition("switch", "Switch", PartKind::Switch)
    });
    doc.parts.push(fixture::part("k1", "switch", "SW1"));
    doc.nets.push(Net {
        id: "row".into(),
        name: "ROW".into(),
        pins: vec![Pin {
            part_id: "k1".into(),
            pad_id: "missing".into(),
        }],
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["wrong".into()],
        part_ids: vec!["k1".into()],
        net_ids: vec!["row".into()],
        thickness: 0.0,
        traces: vec![],
        vias: vec![],
    });
    let (bad, _) = scene(engine.open(doc.clone()));
    assert!(!bad.readiness.pcb);
    assert!(bad.findings.iter().any(|f| f.id == "board:board:thickness"));
    assert!(
        bad.findings
            .iter()
            .any(|f| f.id == "board:board:outline:wrong")
    );
    assert!(
        bad.findings
            .iter()
            .any(|f| f.id == "net:row:pad:k1:missing")
    );
    doc.boards[0].outline_ids = vec!["edge".into()];
    doc.boards[0].thickness = 1.6;
    doc.nets[0].pins[0].pad_id = "pad1".into();
    let (good, _) = scene(engine.open(doc));
    assert!(good.readiness.pcb);
}

#[test]
fn empty_board_subset_blocks_pcb() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("outer", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline.push(fixture::rect(
        "cut",
        0.0,
        0.0,
        2.0,
        2.0,
        Operation::Subtract,
    ));
    doc.boards.push(Board {
        outline_ids: vec!["cut".into()],
        ..fixture::board("board", "Board")
    });
    let (scene, _) = scene(engine.open(doc));
    assert!(!scene.contours.is_empty());
    assert!(scene.board_contours[0].contours.is_empty());
    assert!(!scene.readiness.pcb);
    assert!(
        scene
            .findings
            .iter()
            .any(|finding| finding.id == "board:board:empty-outline")
    );
}

#[test]
fn authored_case_geometry_ignores_missing_material_metadata() {
    let mut doc = ProjectDoc::empty("authored-case", "Authored Case");
    doc.outline
        .push(fixture::rect("edge", 0.0, 0.0, 30.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        outline_ids: vec!["edge".into()],
        ..fixture::board("left", "Left PCB")
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "plate".into(),
        name: "Left plate".into(),
        board_id: "left".into(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id: Some("pla".into()),
        z: Some(0.0),
        wall_height: Some(14.0),
        wall_thickness: Some(2.0),
        mounts: Some(vec![]),
        pcb_source: None,
        gasket: None,
    });
    let mut engine = CoreEngine::new();
    let (scene, document) = scene(engine.open(doc));
    assert!(!scene.board_readiness[0].case_ready);
    assert!(
        scene
            .findings
            .iter()
            .any(|finding| finding.id == "case:plate:material")
    );
    assert!(boardstudio_core::authored_case_geometry_ready(
        &document, &scene, "left"
    ));

    let reply = CoreEngine::prepare_case(&CaseAssemblyIR {
        revision: document.revision,
        bodies: vec![CaseIR {
            revision: document.revision,
            body: document.case_bodies[0].clone(),
            contours: scene.board_contours[0].contours.clone(),
        }],
    });
    assert!(matches!(reply, Ok(_)));
}

#[test]
fn add_part_accepts_legacy_payload_without_optional_placement_fields() {
    let operation: EditOperation = serde_json::from_value(serde_json::json!({
        "kind": "add-part",
        "part": {
            "id": "board/legacy",
            "definitionId": "switch",
            "reference": "SW88",
            "pose": {"at": {"x": 10.0, "y": 20.0}, "rotation": 0.0},
            "side": "front"
        },
        "boardId": "board"
    }))
    .expect("legacy AddPart JSON remains readable");
    let EditOperation::AddPart {
        definition,
        layout_id,
        ..
    } = &operation
    else {
        panic!("expected AddPart")
    };
    assert!(definition.is_none());
    assert!(layout_id.is_none());
    let encoded = serde_json::to_value(operation).expect("AddPart JSON serializes");
    assert!(encoded.get("definition").is_none());
    assert!(encoded.get("layoutId").is_none());
}

#[test]
fn new_matrix_rejects_out_of_bounds_edits() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let mut value = matrix(1, 1);
    {
        let values = vec![Vec2::default(); 2];
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
    assert!(
        matches!(engine.commit_edit(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value.clone(), definitions: None })), Err(message) if message == "layout list exceeds axis count")
    );
    value.row_layout.clear();
    value.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        assemblies_local: None,
        row: 1,
        column: 0,
        enabled: true,

        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![],
    }];
    assert!(
        matches!(engine.commit_edit(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value, definitions: None })), Err(message) if message == "invalid or duplicate cell coordinate")
    );
}

#[test]
fn matrix_targets_board_and_preview_is_scoped() {
    let mut engine = CoreEngine::new();
    let mut doc = matrix_doc();
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "right-edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        outline_ids: vec!["right-edge".into()],
        ..fixture::board("right", "Right")
    });
    engine.open(doc).unwrap();
    let mut target = matrix(1, 2);
    target.board_id = Some("right".into());
    let preview = preview(engine.apply_edit(edit(
        0,
        EditPhase::Preview,
        fixture::set_matrix(target.clone()),
    )));
    assert_eq!(preview.board_contours[1].contours.len(), 1);
    assert!(preview.board_contours[0].contours.is_empty());
    assert!(preview.board_readiness[1].pcb);
    assert!(!preview.board_readiness[0].pcb);
    assert!(scene(engine.snapshot()).1.parts.is_empty());
    let (_, placed) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(target.clone()),
    )));
    assert_eq!(placed.boards[1].part_ids.len(), 2);
    assert!(placed.boards[0].part_ids.is_empty());
    let OutlineFeature::PartEnvelope { part_ids, .. } = &placed.outline[1] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 2);
    let mut edit_matrix = target;
    edit_matrix.board_id = None;
    let (_, updated) =
        scene(engine.commit_edit(edit(1, EditPhase::Commit, fixture::set_matrix(edit_matrix))));
    assert_eq!(updated.matrices[0].board_id.as_deref(), Some("right"));
    assert_eq!(updated.boards[1].part_ids.len(), 2);
    let mut invalid = matrix(1, 1);
    invalid.board_id = Some("missing".into());
    assert!(matches!(
        engine.commit_edit(edit(
            2,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: invalid,
                definitions: None,
            }
        )),
        Err(_)
    ));
}

#[test]
fn matrix_generates_stable_parts_and_one_undo_step() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let preview = preview(engine.apply_edit(edit(
        0,
        EditPhase::Preview,
        fixture::set_matrix(matrix(2, 2)),
    )));
    assert_eq!(preview.transforms.len(), 4);
    assert!(scene(engine.snapshot()).1.parts.is_empty());
    let (committed_scene, doc) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(2, 2)),
    )));
    assert_eq!(committed_scene.revision, 1);
    assert_eq!(
        doc.matrices[0].part_ids,
        vec![
            "matrix/main/r0c0",
            "matrix/main/r0c1",
            "matrix/main/r1c0",
            "matrix/main/r1c1"
        ]
    );
    assert_eq!(doc.boards[0].part_ids.len(), 4);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &doc.outline[0] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 4);
    assert!(committed_scene.readiness.outline);
    let (undone, doc) = scene(engine.undo());
    assert_eq!(undone.revision, 2);
    assert!(doc.parts.is_empty());
}

#[test]
fn replacing_matrix_switch_remaps_existing_terminal_nets_and_undoes() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads = ["one", "two"].iter().enumerate().map(|(i, id)| serde_json::from_value(serde_json::json!({
        "id": id, "number": (i + 1).to_string(), "at": {"x": i, "y": 0}, "size": {"x": 1, "y": 1}, "shape": "circle"
    })).unwrap()).collect();
    let mut engine = CoreEngine::new();
    engine.open(doc).unwrap();
    let (_, mut placed) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 1)),
    )));
    let id = placed.parts[0].id.clone();
    placed.nets.push(Net {
        id: "external".into(),
        name: "ROW0".into(),
        pins: vec![Pin {
            part_id: id.clone(),
            pad_id: "one".into(),
        }],
    });
    placed.boards[0].net_ids.push("external".into());
    placed.nets.push(Net {
        id: "external-column".into(),
        name: "COL0".into(),
        pins: vec![Pin {
            part_id: id.clone(),
            pad_id: "two".into(),
        }],
    });
    placed.boards[0].net_ids.push("external-column".into());
    engine.open(placed.clone()).unwrap();
    let mut replacement = placed.definitions[0].clone();
    replacement.id = "new-switch".into();
    replacement.pads[0].id = "new-row".into();
    replacement.pads[1].id = "new-column".into();
    replacement
        .terminals
        .insert("from".into(), vec!["new-row".into()]);
    replacement
        .terminals
        .insert("to".into(), vec!["new-column".into()]);
    replacement.matrix_terminals = Some(MatrixTerminals {
        row: "from".into(),
        column: "to".into(),
    });
    let mut incoming = placed.matrices[0].clone();
    incoming.definition_id = replacement.id.clone();
    let mut conflicting = replacement.clone();
    conflicting.matrix_terminals.as_mut().unwrap().column = "from".into();
    assert!(matches!(
        engine.commit_edit(edit(
            placed.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: incoming.clone(),
                definitions: Some(vec![conflicting])
            }
        )),
        Err(_)
    ));
    let (_, changed) = scene(engine.commit_edit(edit(
        placed.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: incoming,
            definitions: Some(vec![replacement]),
        },
    )));
    assert_eq!(
        changed
            .nets
            .iter()
            .find(|n| n.id == "external")
            .unwrap()
            .pins,
        vec![Pin {
            part_id: id,
            pad_id: "new-row".into()
        }]
    );
    let (_, undone) = scene(engine.undo());
    assert_eq!(undone.nets, placed.nets);
    assert_eq!(undone.parts, placed.parts);
}

#[test]
fn grouped_matrix_cell_actions_are_atomic_across_unlinked_matrices() {
    let mut engine = CoreEngine::new();
    engine.open(matrix_doc()).unwrap();
    let (_, doc) = scene(engine.commit_edit(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 2)),
    )));
    let mut other = matrix(1, 2);
    other.id = "other".into();
    other.origin.x = 60.0;
    let (_, before) = scene(engine.commit_edit(edit(
        doc.revision,
        EditPhase::Commit,
        fixture::set_matrix(other),
    )));
    let operation = serde_json::from_value(serde_json::json!({"kind":"set-matrix-cells-enabled", "cells":{"main":[[0,0]],"other":[[0,1]]}, "enabled":false})).expect("Core must support grouped matrix enable changes atomically");
    let (geometry, disabled) =
        scene(engine.commit_edit(edit(before.revision, EditPhase::Commit, operation)));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 2)
    );
    assert!(
        disabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 1)
    );
    let operation = serde_json::from_value(serde_json::json!({"kind":"set-matrix-cells-enabled", "cells":{"main":[[0,0]],"other":[[0,1]]}, "enabled":true})).unwrap();
    let (_, enabled) =
        scene(engine.commit_edit(edit(disabled.revision, EditPhase::Commit, operation)));
    assert!(
        enabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 2)
    );
    let operation = serde_json::from_value(serde_json::json!({"kind":"remove-selected-matrix-cells", "cells":{"main":[[0,0]],"other":[[0,1]]}})).expect("Core must delete a grouped selection atomically");
    let (geometry, deleted) =
        scene(engine.commit_edit(edit(enabled.revision, EditPhase::Commit, operation)));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
    let (_, undone) = scene(engine.undo());
    assert_eq!(undone.matrices, enabled.matrices);
    assert_eq!(undone.parts, enabled.parts);
    let (_, redone) = scene(engine.redo());
    assert_eq!(redone.matrices, deleted.matrices);
    let document = serde_json::from_value(serde_json::to_value(&deleted).unwrap()).unwrap();
    let (geometry, _) = scene(engine.open(document));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
}

#[test]
fn undo_retains_only_the_latest_hundred_commits() {
    let mut engine = CoreEngine::new();
    let mut seed = ProjectDoc::empty("bounded-history", "0");
    seed.parts
        .push(fixture::part("part-0", "catalog:unused", "P0"));
    let (_, mut document) = scene(engine.open(seed));
    for index in 1..=101 {
        (_, document) = scene(engine.commit_edit(edit(
            document.revision,
            EditPhase::Commit,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: "part-0".into(),
                    at: Vec2 {
                        x: index as f64,
                        y: 0.0,
                    },
                }],
            },
        )));
    }
    for _ in 0..100 {
        (_, document) = scene(engine.undo());
    }
    assert_eq!(document.parts[0].pose.at.x, 1.0);
    assert!(matches!(engine.undo(), Err(_)));
    for _ in 0..100 {
        (_, document) = scene(engine.redo());
    }
    assert_eq!(document.parts[0].pose.at.x, 101.0);
    (_, document) = scene(engine.undo());
    scene(engine.commit_edit(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "part-0".into(),
                at: Vec2 { x: 0.5, y: 0.0 },
            }],
        },
    )));
    assert!(matches!(engine.redo(), Err(_)));
    scene(engine.open(ProjectDoc::empty("fresh", "Fresh")));
    assert!(matches!(engine.undo(), Err(_)));
}
