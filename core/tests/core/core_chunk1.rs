use super::*;
use crate::support::fixture;

#[test]
fn open_preserves_generator_and_copper() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads.push(Pad {
        id: "p1".into(),
        number: "1".into(),
        at: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Circle,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    });
    doc.definitions[0].pads[0].plated = Some(false);
    doc.definitions[0].pads[0].side = Some(Side::Back);
    doc.definitions[0].pads[0].rotation = Some(90.0);
    doc.definitions[0].generator = Some(PartGenerator {
        source: "builtin:mx-switch".into(),
        version: "1".into(),
        parameters: Default::default(),
    });
    doc.boards[0].traces.push(CopperTrace {
        id: "trace".into(),
        start: Vec2 { x: 0.0, y: 0.0 },
        end: Vec2 { x: 1.0, y: 0.0 },
        width: 0.25,
        layer: Side::Front,
        net_id: None,
    });
    doc.boards[0].vias.push(CopperVia {
        id: "via".into(),
        at: Vec2 { x: 1.0, y: 0.0 },
        size: 0.8,
        drill: 0.4,
        net_id: None,
    });
    let mut engine = CoreEngine::new();
    let json = serde_json::to_string(&fixture::open(doc)).unwrap();
    let (_, reopened) = scene(serde_json::from_str(&engine.request(&json)).unwrap());
    assert_eq!(
        reopened.definitions[0].generator.as_ref().unwrap().source,
        "builtin:mx-switch"
    );
    assert_eq!(reopened.boards[0].traces[0].id, "trace");
    assert_eq!(reopened.boards[0].vias[0].id, "via");
    assert_eq!(reopened.definitions[0].pads[0].plated, Some(false));
    assert_eq!(reopened.definitions[0].pads[0].rotation, Some(90.0));
}

#[test]
fn added_part_joins_board_and_envelope_then_removes_cleanly() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        ..fixture::part_definition("switch", "Switch", PartKind::Switch)
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "env".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 1.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        outline_ids: vec!["env".into()],
        ..fixture::board("board", "Board")
    });
    engine.handle(fixture::open(doc));
    let part = fixture::part("k1", "switch", "SW1");
    let (added, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::AddPart {
            part,
            board_id: None,
            definition: None,
            layout_id: None,
        },
    )));
    assert!(added.readiness.outline);
    assert_eq!(doc.boards[0].part_ids, vec!["k1"]);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &doc.outline[0] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids, &vec!["k1"]);
    let (_, removed) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["k1".into()],
        },
    )));
    assert!(removed.boards[0].part_ids.is_empty());
    let OutlineFeature::PartEnvelope { part_ids, .. } = &removed.outline[0] else {
        panic!("expected envelope")
    };
    assert!(part_ids.is_empty());
}

#[test]
fn invalid_definition_geometry_blocks_own_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    for (id, x) in [("left", -20.0), ("right", 20.0)] {
        doc.outline
            .push(fixture::rect(id, x, 0.0, 10.0, 10.0, Operation::Add));
        doc.boards.push(Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![id.into()],
            part_ids: vec![format!("{id}-part")],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(Part {
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            ..fixture::part(format!("{id}-part"), format!("{id}-def"), id)
        });
        doc.definitions.push(PartDefinition {
            courtyard: vec![Vec2 { x: 0.0, y: 0.0 }],
            pads: vec![Pad {
                id: "p1".into(),
                number: "1".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: PadShape::Roundrect,
                drill: Some(0.5),
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            }],
            ..fixture::part_definition(format!("{id}-def"), id, PartKind::Custom)
        });
    }
    let bad = &mut doc.definitions[1];
    bad.courtyard[0].x = f64::NAN;
    bad.pads[0].size.x = -1.0;
    bad.pads[0].drill = Some(-0.5);
    bad.pads.push(Pad {
        id: "p1".into(),
        number: "1".into(),
        at: Vec2 {
            x: f64::INFINITY,
            y: 0.0,
        },
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Oval,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    });
    let (scene, _) = scene(engine.handle(fixture::open(doc)));
    assert!(scene.board_readiness[0].pcb);
    assert!(!scene.board_readiness[1].pcb);
    for suffix in [
        "courtyard",
        "pad:p1:size",
        "pad:p1:drill",
        "pad:p1:position",
        "pad:p1:duplicate-id",
    ] {
        assert!(
            scene.findings.iter().any(|finding| finding.id
                == format!("definition:right-def:{suffix}")
                && finding.target_ids.contains(&"right-part".to_string())),
            "missing {suffix}"
        );
    }
}

#[test]
fn case_readiness_uses_its_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        outline_ids: vec!["edge".into()],
        ..fixture::board("case-board", "Case Board")
    });
    doc.boards.push(fixture::board("empty", "Empty"));
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "case".into(),
        name: "Case".into(),
        board_id: "case-board".into(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id: None,
        z: None,
        wall_height: None,
        wall_thickness: None,
        mounts: None,
        pcb_source: None,
        gasket: None,
    });
    let (valid_scene, doc) = scene(engine.handle(fixture::open(doc)));
    assert!(!valid_scene.readiness.pcb);
    assert!(valid_scene.board_readiness[0].pcb);
    assert!(!valid_scene.board_readiness[1].pcb);
    assert!(valid_scene.board_readiness[0].case_ready);
    assert!(!valid_scene.board_readiness[1].case_ready);
    assert!(valid_scene.readiness.case_ready);
    assert!(boardstudio_core::authored_case_geometry_ready(
        &doc,
        &valid_scene,
        "case-board"
    ));
    let mut pcb_unready_scene = valid_scene.clone();
    pcb_unready_scene.board_readiness[0].pcb = false;
    pcb_unready_scene.board_readiness[0].case_ready = false;
    assert!(boardstudio_core::authored_case_geometry_ready(
        &doc,
        &pcb_unready_scene,
        "case-board"
    ));
    let mut stale_scene = valid_scene.clone();
    stale_scene.revision += 1;
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &doc,
        &stale_scene,
        "case-board"
    ));
    let mut outline_unready_scene = valid_scene.clone();
    outline_unready_scene.board_readiness[0].outline = false;
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &doc,
        &outline_unready_scene,
        "case-board"
    ));
    let mut invalid = doc;
    invalid.case_bodies[0].thickness = -1.0;
    let (invalid_scene, invalid) = scene(engine.handle(CoreRequest::Open {
        id: "invalid".into(),
        document: invalid,
    }));
    assert!(!invalid_scene.board_readiness[0].case_ready);
    assert!(invalid_scene.board_readiness[0].pcb);
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &invalid,
        &invalid_scene,
        "case-board"
    ));
}

#[test]
fn case_mounts_walls_and_gasket_are_validated() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(fixture::rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        outline_ids: vec!["edge".into()],
        ..fixture::board("board", "Board")
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "tray".into(),
        name: "Tray".into(),
        board_id: "board".into(),
        kind: CaseKind::Tray,
        thickness: 3.0,
        clearance: 0.5,
        material_id: None,
        z: Some(0.0),
        wall_height: None,
        wall_thickness: None,
        mounts: Some(vec![Mount {
            id: "mount".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            kind: MountKind::Boss,
            hole_diameter: 3.0,
            boss_diameter: Some(2.0),
            height: None,
        }]),
        pcb_source: None,
        gasket: Some(Gasket {
            inset: 1.0,
            width: 2.0,
            depth: 3.0,
        }),
    });
    let (bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "bad".into(),
        document: doc.clone(),
    }));
    assert!(bad.readiness.pcb);
    assert!(!bad.readiness.case_ready);
    for id in [
        "case:tray:walls",
        "case:tray:mount:mount:boss",
        "case:tray:gasket",
    ] {
        assert!(bad.findings.iter().any(|finding| finding.id == id));
    }
    doc.case_bodies[0].wall_height = Some(8.0);
    doc.case_bodies[0].wall_thickness = Some(2.0);
    doc.case_bodies[0].mounts.as_mut().unwrap()[0].boss_diameter = Some(5.0);
    doc.case_bodies[0].mounts.as_mut().unwrap()[0].height = Some(4.0);
    doc.case_bodies[0].gasket.as_mut().unwrap().depth = 1.0;
    let (good, document) = scene(engine.handle(CoreRequest::Open {
        id: "good".into(),
        document: doc,
    }));
    assert!(good.readiness.case_ready);
    let wire = serde_json::to_value(&document).unwrap();
    assert_eq!(wire["caseBodies"][0]["wallHeight"], 8.0);
    assert_eq!(wire["caseBodies"][0]["mounts"][0]["bossDiameter"], 5.0);
}

#[test]
fn matrix_duplicate_allocates_new_member_ids() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let (_, original) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 1)),
    )));
    let mut duplicate = original.matrices[0].clone();
    duplicate.id = "copy".into();
    duplicate.origin.x = 30.0;
    let (_, doc) = scene(engine.handle(edit(1, EditPhase::Commit, fixture::set_matrix(duplicate))));
    assert_eq!(doc.matrices[0].part_ids, vec!["matrix/main/r0c0"]);
    assert_eq!(doc.matrices[1].part_ids, vec!["matrix/copy/r0c0"]);
    assert_ne!(doc.parts[0].reference, doc.parts[1].reference);
}

#[test]
fn matrix_shrink_prunes_net_pins_and_stale_members() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 2)),
    )));
    doc.nets.push(Net {
        id: "row".into(),
        name: "ROW".into(),
        pins: vec![Pin {
            part_id: "matrix/main/r0c1".into(),
            pad_id: "p1".into(),
        }],
    });
    doc.constraints.push(Constraint::Offset {
        id: "linked".into(),
        source_part_id: "matrix/main/r0c0".into(),
        target_part_id: "matrix/main/r0c1".into(),
        offset: Vec2 { x: 19.0, y: 0.0 },
        rotation: 0.0,
    });
    let (_, doc) = scene(engine.handle(CoreRequest::Open {
        id: "with-net".into(),
        document: doc,
    }));
    assert_eq!(doc.nets[0].pins.len(), 1);
    let (_, shrunk) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        fixture::set_matrix(matrix(1, 1)),
    )));
    assert!(shrunk.nets[0].pins.is_empty());
    assert!(shrunk.constraints.is_empty());
    assert_eq!(shrunk.matrices[0].part_ids, vec!["matrix/main/r0c0"]);
    assert!(
        shrunk
            .parts
            .iter()
            .all(|part| part.id != "matrix/main/r0c1")
    );
}

#[test]
fn matrix_accepts_stable_ergogen_catalogue_ids() {
    let definition: PartDefinition = serde_json::from_value(serde_json::json!({
        "id": "generator:ceoloide/switch_mx",
        "name": "MX switch",
        "kind": "switch",
        "courtyard": [{"x": -5.0, "y": -5.0}, {"x": 5.0, "y": -5.0}, {"x": 5.0, "y": 5.0}, {"x": -5.0, "y": 5.0}],
        "pads": [{"id": "pad-0", "number": "1", "at": {"x": -2.0, "y": 0.0}, "size": {"x": 1.0, "y": 1.0}, "shape": "rect"}, {"id": "pad-1", "number": "2", "at": {"x": 2.0, "y": 0.0}, "size": {"x": 1.0, "y": 1.0}, "shape": "rect"}],
        "matrixTerminals": {"row": "from", "column": "to"},
        "generator": {"source": "ceoloide/switch_mx", "version": "bundled-1", "parameters": {}}
    })).unwrap();
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let mut incoming = matrix(1, 1);
    incoming.definition_id = definition.id.clone();
    let reply = engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: incoming,
            definitions: Some(vec![definition]),
        },
    ));
    let (_, generated) = scene(reply);
    assert_eq!(
        generated.matrices[0].definition_id,
        "generator:ceoloide/switch_mx"
    );
}

#[test]
fn matrix_custom_splay_origin_survives_export_and_resize() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let mut value = matrix(2, 3);
    value.pitch = Vec2 { x: 20.0, y: 20.0 };
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
    {
        let values = (vec![None, Some(Vec2 { x: -20.0, y: -20.0 })])
            .into_iter()
            .map(|value| value.unwrap_or_default())
            .collect::<Vec<_>>();
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.splay_origin = Vec2::default();
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.splay_origin = value;
        }
    }
    let (_, doc) = scene(engine.handle(edit(0, EditPhase::Commit, fixture::set_matrix(value))));
    let key = doc
        .parts
        .iter()
        .find(|p| p.id == "matrix/main/r0c1")
        .unwrap();
    assert!((key.pose.at.x + 20.0).abs() < 1e-9);
    assert!(key.pose.at.y.abs() < 1e-9);
    assert_eq!(key.pose.rotation, 90.0);
    let reopened: ProjectDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        reopened.matrices[0].column_layout[1].splay_origin,
        Vec2 { x: -20.0, y: -20.0 }
    );
    let mut value = reopened.matrices[0].clone();
    value.columns = 1;
    let (_, shrunk) = scene(engine.handle(edit(1, EditPhase::Commit, fixture::set_matrix(value))));
    assert_eq!(
        shrunk.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>(),
        vec![Vec2::default()]
    );
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(
        restored.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>(),
        reopened.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>()
    );
}

#[test]
fn deleting_matrix_row_preserves_surviving_part_and_binding_identity() {
    let mut engine = CoreEngine::new();
    engine.handle(fixture::open(matrix_doc()));
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        fixture::set_matrix(matrix(3, 1)),
    )));
    let survivor = doc
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/main/r2c0")
        .unwrap();
    survivor.keycap = Some(Vec2 { x: 1.0, y: 1.5 });
    let reference = survivor.reference.clone();
    doc.keymap = Some(KeymapConfiguration::default());
    doc.keymap.as_mut().unwrap().layers[0].bindings.insert(
        "matrix/main/r2c0".into(),
        KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    );
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc,
    });
    let operation = serde_json::from_value(
        serde_json::json!({"kind":"remove-matrix-row","matrixId":"main","row":1}),
    )
    .expect("Core must remove a row rather than only shrink the last row");
    let (_, doc) = scene(engine.handle(edit(1, EditPhase::Commit, operation)));
    assert_eq!(doc.matrices[0].rows, 2);
    let survivor = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0")
        .unwrap();
    assert_eq!(survivor.reference, reference);
    assert_eq!(survivor.keycap, Some(Vec2 { x: 1.0, y: 1.5 }));
    assert_eq!(
        doc.keymap.as_ref().unwrap().layers[0].bindings["matrix/main/r1c0"],
        KeyBinding::KeyPress {
            keycode: "A".into()
        }
    );
    assert!(
        !doc.keymap.as_ref().unwrap().layers[0]
            .bindings
            .contains_key("matrix/main/r2c0")
    );
}
