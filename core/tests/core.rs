use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;

fn rect(id: &str, x: f64, y: f64, width: f64, height: f64, operation: Operation) -> OutlineFeature {
    OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: id.into(),
        center: Vec2 { x, y },
        size: Vec2 {
            x: width,
            y: height,
        },
        radius: 0.0,
        operation,
    }
}
fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
fn edit(base: u64, phase: EditPhase, op: EditOperation) -> CoreRequest {
    CoreRequest::Edit {
        id: "request".into(),
        command: EditCommand {
            base_revision: base,
            transaction_id: "drag-1".into(),
            phase,
            target_ids: vec![],
            operation: op,
        },
    }
}

#[test]
fn preview_does_not_commit_and_stale_edit_fails() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 5.0 },
        }],
    };
    let preview = preview(engine.handle(edit(0, EditPhase::Preview, op.clone())));
    assert_eq!(preview.revision, 0);
    assert_eq!(preview.transforms[0].pose.at.x, 10.0);
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
    let failed = EditOperation::MoveParts {
        positions: vec![
            Position {
                id: "k1".into(),
                at: Vec2 { x: 20.0, y: 0.0 },
            },
            Position {
                id: "missing".into(),
                at: Vec2::default(),
            },
        ],
    };
    assert!(matches!(
        engine.handle(edit(0, EditPhase::Preview, failed)),
        CoreReply::Error { .. }
    ));
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    assert_eq!(snapshot.parts[0].pose.at.x, 0.0);
    let (committed, _) = scene(engine.handle(edit(0, EditPhase::Commit, op.clone())));
    assert_eq!(committed.revision, 1);
    assert!(matches!(
        engine.handle(edit(0, EditPhase::Commit, op)),
        CoreReply::Error { revision: 1, .. }
    ));
}

#[test]
fn undo_redo_increments_revision() {
    let mut engine = CoreEngine::new();
    // Ticket 37: raw SetOutline is deleted; the typed case write is the live
    // op for this undo/redo accounting.
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let add = || EditOperation::SetCase {
        body: CaseBody {
            features: None,
            openings: None,
            id: "case".into(),
            name: "Case".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 2.0,
            clearance: 0.0,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
        },
    };
    scene(engine.handle(edit(0, EditPhase::Commit, add())));
    let (undo, doc) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undo.revision, 2);
    assert!(doc.case_bodies.is_empty());
    let (redo, doc) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redo.revision, 3);
    assert_eq!(doc.case_bodies.len(), 1);
}

#[test]
fn union_difference_make_one_hole() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("left", -5.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline
        .push(rect("right", 5.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline
        .push(rect("hole", 0.0, 0.0, 5.0, 5.0, Operation::Subtract));
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert_eq!(scene.contours.len(), 2);
    assert_eq!(scene.contours.iter().filter(|c| c.hole).count(), 1);
    assert!(scene.readiness.outline);
    assert!(!scene.readiness.pcb);
}

#[test]
fn wire_roundtrip_matches_contract() {
    let request = r#"{"id":"snap","kind":"snapshot"}"#;
    let mut engine = CoreEngine::new();
    let reply: CoreReply = serde_json::from_str(&engine.request(request)).unwrap();
    let (_, doc) = scene(reply);
    assert_eq!(doc.format, "boardstudio/v2");
    assert_eq!(
        serde_json::to_value(&doc).unwrap()["caseBodies"],
        serde_json::json!([])
    );
}

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
    let json = serde_json::to_string(&CoreRequest::Open {
        id: "open".into(),
        document: doc,
    })
    .unwrap();
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
fn mechanical_holes_do_not_need_pad_numbers() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads = vec!["center", "left", "right"]
        .into_iter()
        .map(|id| Pad {
            id: id.into(),
            number: String::new(),
            at: Vec2::default(),
            size: Vec2 { x: 2.0, y: 2.0 },
            shape: PadShape::Circle,
            drill: Some(2.0),
            plated: Some(false),
            side: None,
            rotation: None,
            net_id: None,
        })
        .collect();
    doc.parts.push(Part {
        anchor: None,
        id: "switch".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let mut engine = CoreEngine::new();
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert!(
        !scene
            .findings
            .iter()
            .any(|finding| finding.id.contains(":number")
                || finding.id.contains(":duplicate-number"))
    );
}

#[test]
fn part_envelope_tracks_committed_move() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "boundary".into(),
        part_ids: vec!["k1".into()],
        settings: Default::default(),
        margin: 1.0,
        operation: Operation::Add,
    });
    let (before, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "k1".into(),
            at: Vec2 { x: 10.0, y: 0.0 },
        }],
    };
    let (after, _) = scene(engine.handle(edit(0, EditPhase::Commit, op)));
    let before_min = before.contours[0]
        .points
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    let after_min = after.contours[0]
        .points
        .iter()
        .map(|p| p.x)
        .fold(f64::INFINITY, f64::min);
    assert_eq!(after_min - before_min, 10.0);
}

#[test]
fn scripts_emit_stable_ids_and_keep_visual_pose() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.scripts.push(Script {
        id: "layout".into(), name: "Layout".into(), enabled: true,
        source: "part(\"main\", \"k1\", \"switch\", \"SW1\", 0.0, 0.0, 0.0); rect(\"main\", \"board\", 0.0, 0.0, 20.0, 10.0, 0.0, \"add\");".into(),
    });
    let (opened, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert_eq!(doc.parts[0].id, "script/layout/main/k1");
    assert_eq!(doc.outline[0].id(), "script/layout/main/board");
    assert!(opened.readiness.outline);
    let op = EditOperation::MoveParts {
        positions: vec![Position {
            id: "script/layout/main/k1".into(),
            at: Vec2 { x: 7.0, y: 3.0 },
        }],
    };
    let (_, edited) = scene(engine.handle(edit(0, EditPhase::Commit, op)));
    let (reopened, persisted) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: edited,
    }));
    assert_eq!(persisted.parts.len(), 1);
    assert_eq!(reopened.transforms[0].pose.at.x, 7.0);
}

#[test]
fn bad_script_is_atomic_and_limited() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.scripts.push(Script {
        id: "bad".into(),
        name: "Bad".into(),
        enabled: true,
        source: "while true {}".into(),
    });
    let reply = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    assert!(matches!(reply, CoreReply::Error { revision: 0, .. }));
    let (_, unchanged) = scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
    assert_eq!(unchanged.id, "untitled");
}

#[test]
fn added_part_joins_board_and_envelope_then_removes_cleanly() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
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
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["env".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let part = Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    };
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
fn add_part_targets_board_and_removal_cleans_both_boards() {
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
        id: "right".into(),
        name: "Right".into(),
        outline_ids: vec!["right-edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let part = Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    };
    let op = EditOperation::AddPart {
        part: part.clone(),
        board_id: Some("right".into()),
        definition: None,
        layout_id: None,
    };
    let preview = preview(engine.handle(edit(0, EditPhase::Preview, op.clone())));
    assert!(preview.board_readiness[1].pcb);
    assert!(!preview.board_readiness[0].pcb);
    assert!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts
        .is_empty()
    );
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
    let (_, added) = scene(engine.handle(edit(0, EditPhase::Commit, op)));
    assert!(added.boards[0].part_ids.is_empty());
    assert_eq!(added.boards[1].part_ids, vec!["k1"]);
    let OutlineFeature::PartEnvelope { part_ids: left, .. } = &added.outline[0] else {
        panic!("expected envelope")
    };
    let OutlineFeature::PartEnvelope {
        part_ids: right, ..
    } = &added.outline[1]
    else {
        panic!("expected envelope")
    };
    assert!(left.is_empty());
    assert_eq!(right, &vec!["k1"]);
    let mut invalid = EditOperation::AddPart {
        part,
        board_id: Some("missing".into()),
        definition: None,
        layout_id: None,
    };
    assert!(matches!(
        engine.handle(edit(1, EditPhase::Commit, invalid.clone())),
        CoreReply::Error { .. }
    ));
    if let EditOperation::AddPart { part, .. } = &mut invalid {
        part.id = "k2".into();
    }
    assert!(matches!(
        engine.handle(edit(1, EditPhase::Commit, invalid)),
        CoreReply::Error { .. }
    ));
    let (_, removed) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["k1".into()],
        },
    )));
    assert!(removed.boards.iter().all(|board| board.part_ids.is_empty()));
    assert!(removed.outline.iter().all(|feature| matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.is_empty())));
}

#[test]
fn pcb_readiness_checks_pads_outlines_and_thickness() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
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
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
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
    let (bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "bad".into(),
        document: doc.clone(),
    }));
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
    let (good, _) = scene(engine.handle(CoreRequest::Open {
        id: "good".into(),
        document: doc,
    }));
    assert!(good.readiness.pcb);
}

#[test]
fn invalid_definition_geometry_blocks_own_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    for (id, x) in [("left", -20.0), ("right", 20.0)] {
        doc.outline
            .push(rect(id, x, 0.0, 10.0, 10.0, Operation::Add));
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
            anchor: None,
            id: format!("{id}-part"),
            definition_id: format!("{id}-def"),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
        doc.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            mechanical_profile: None,
            id: format!("{id}-def"),
            name: id.into(),
            kind: PartKind::Custom,
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
            models: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
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
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
fn empty_definition_and_pad_ids_are_reported() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "".into(),
        name: "Custom".into(),
        kind: PartKind::Custom,
        courtyard: vec![],
        pads: vec![Pad {
            id: "".into(),
            number: "".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Oval,
            drill: Some(f64::INFINITY),
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        }],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "p".into(),
        definition_id: "".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    for suffix in ["id", "pad:0:id", "pad:0:drill"] {
        assert!(
            scene
                .findings
                .iter()
                .any(|finding| finding.id == format!("definition::{suffix}")),
            "missing {suffix}"
        );
    }
}

#[test]
fn duplicate_pad_nets_and_invalid_case_block_readiness() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "part".into(),
        name: "Part".into(),
        kind: PartKind::Custom,
        courtyard: vec![],
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
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.parts.push(Part {
        anchor: None,
        id: "u1".into(),
        definition_id: "part".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
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
    doc.outline
        .push(rect("left", -20.0, 0.0, 10.0, 10.0, Operation::Add));
    doc.outline
        .push(rect("cut", -20.0, 0.0, 2.0, 2.0, Operation::Subtract));
    doc.outline
        .push(rect("right", 20.0, 0.0, 10.0, 10.0, Operation::Add));
    doc.boards.push(Board {
        id: "left-board".into(),
        name: "Left".into(),
        outline_ids: vec!["left".into(), "cut".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.boards.push(Board {
        id: "right-board".into(),
        name: "Right".into(),
        outline_ids: vec!["right".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
fn empty_board_subset_blocks_pcb() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("outer", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.outline
        .push(rect("cut", 0.0, 0.0, 2.0, 2.0, Operation::Subtract));
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["cut".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
fn case_readiness_uses_its_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "case-board".into(),
        name: "Case Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.boards.push(Board {
        id: "empty".into(),
        name: "Empty".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
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
    let (valid_scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
fn authored_case_geometry_ignores_missing_material_metadata() {
    let mut doc = ProjectDoc::empty("authored-case", "Authored Case");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 30.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "left".into(),
        name: "Left PCB".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
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
    let (scene, document) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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

    let reply = engine.handle(CoreRequest::PrepareCase {
        id: "prepare".into(),
        ir: CaseAssemblyIR {
            revision: document.revision,
            bodies: vec![CaseIR {
                revision: document.revision,
                body: document.case_bodies[0].clone(),
                contours: scene.board_contours[0].contours.clone(),
            }],
        },
    });
    assert!(matches!(reply, CoreReply::CasePrepared { .. }));
}

#[test]
fn case_mounts_walls_and_gasket_are_validated() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
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

fn matrix_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

fn mirrored_pair_request(base: u64, phase: &str) -> String {
    serde_json::json!({
        "kind": "edit", "id": "pair",
        "command": {"baseRevision": base, "transactionId": "pair", "phase": phase, "targetIds": [],
            "operation": {"kind": "create-mirrored-pair",
                "matrix": {"id": "left-matrix", "rows": 2, "columns": 3, "pitch": {"x": 19, "y": 19},
                    "origin": {"x": -20, "y": 5}, "mirror": "x", "definitionId": "switch", "boardId": "board", "partIds": []},
                "left": {"id": "left-layout", "name": "Left half", "boardId": "board", "matrixIds": ["left-matrix"], "partIds": []},
                "right": {"id": "right-layout", "name": "Right half", "boardId": "board", "matrixIds": ["right-matrix"], "partIds": [],
                    "mirrorLink": {"sourceId": "left-layout", "axisX": 10}}
            }
        }
    }).to_string()
}

#[test]
fn mirrored_pair_is_atomic_and_survives_history_and_reopen() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, document) = scene(serde_json::from_str(&response).unwrap());
    assert_eq!(document.matrices.len(), 2);
    assert_eq!(document.parts.len(), 12);
    assert_eq!(document.boards[0].part_ids.len(), 12);
    let serialized = serde_json::to_value(&document).unwrap();
    assert_eq!(
        serialized["layouts"][1]["mirrorLink"]["sourceId"],
        "left-layout"
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.matrices.is_empty());
    assert!(undone.parts.is_empty());
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.parts, document.parts);
    let (_, reopened) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: serde_json::from_value(serialized).unwrap(),
    }));
    assert_eq!(reopened.parts, document.parts);
}

#[test]
fn public_add_part_from_source_board_creates_one_mirror_correspondence() {
    let (mut engine, document) = public_native_pair();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let response = engine.request(&serde_json::json!({
        "kind": "edit",
        "id": "source-add",
        "command": {
            "baseRevision": document.revision,
            "transactionId": "source-add",
            "phase": "commit",
            "targetIds": [],
            "operation": {
                "kind": "add-part",
                "boardId": halves.source_board_id,
                "part": {
                    "id": "source/J1", "definitionId": "generator:ceoloide/switch_mx", "reference": "J1",
                    "pose": {"at": {"x": 8.0, "y": 12.0}, "rotation": 0.0}, "side": "front"
                }
            }
        }
    }).to_string());
    let (_, added) = scene(serde_json::from_str(&response).unwrap());
    let halves = added.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert_eq!(
        added
            .parts
            .iter()
            .filter(|part| part.id == "source/J1")
            .count(),
        1
    );
    assert_eq!(
        halves
            .correspondences
            .iter()
            .filter(|item| { item.kind == MirroredItemKind::Part && item.source_id == "source/J1" })
            .count(),
        1,
        "source-side AddPart must create one durable mirror correspondence"
    );
    let association = halves
        .correspondences
        .iter()
        .find(|item| item.kind == MirroredItemKind::Part && item.source_id == "source/J1")
        .unwrap();
    assert!(
        added
            .boards
            .iter()
            .find(|board| board.id == halves.mirror_board_id)
            .unwrap()
            .part_ids
            .contains(&association.mirror_id)
    );
    for (outline_id, part_id) in [
        ("left/synthetic-envelope", &association.source_id),
        ("mirror/synthetic-envelope", &association.mirror_id),
    ] {
        let OutlineFeature::PartEnvelope { part_ids, .. } = added
            .outline
            .iter()
            .find(|feature| feature.id() == outline_id)
            .unwrap_or_else(|| {
                panic!(
                    "missing {outline_id}; present {:?}",
                    added
                        .outline
                        .iter()
                        .map(OutlineFeature::id)
                        .collect::<Vec<_>>()
                )
            })
        else {
            panic!("expected {outline_id}")
        };
        assert_eq!(
            part_ids.iter().filter(|id| *id == part_id).count(),
            1,
            "{outline_id} membership"
        );
    }
}

#[test]
fn matrixless_linked_add_part_projects_from_either_half_and_preserves_omission() {
    for (board_id, part_id, reference) in [
        ("board-main", "board-main/source-add", "SRC1"),
        ("board-mirror", "board-mirror/reverse-add", "REV1"),
    ] {
        let (mut engine, before) = matrixless_native_pair_with_omitted_part();
        assert!(before.matrices.is_empty());
        assert!(before.layouts.is_empty());
        let mut definition = before.definitions[0].clone();
        definition.id = "catalog:matrixless-addition".into();
        definition.name = "Matrixless addition".into();
        let part = Part {
            anchor: None,
            id: part_id.into(),
            definition_id: definition.id.clone(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 5.0, y: -2.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        };
        let (_, added) = scene(engine.handle(edit(
            before.revision,
            EditPhase::Commit,
            EditOperation::AddPart {
                part: part.clone(),
                board_id: Some(board_id.into()),
                definition: Some(definition.clone()),
                layout_id: None,
            },
        )));
        let halves = added.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap();
        let new_associations: Vec<_> = halves
            .correspondences
            .iter()
            .filter(|association| {
                association.kind == MirroredItemKind::Part
                    && association.source_id != "part-previous"
            })
            .collect();
        assert_eq!(new_associations.len(), 1, "one new placement links once");
        let association = new_associations[0];
        let source_part = added
            .parts
            .iter()
            .find(|candidate| candidate.id == association.source_id)
            .unwrap();
        let mirror_part = added
            .parts
            .iter()
            .find(|candidate| candidate.id == association.mirror_id)
            .unwrap();
        let (authored, counterpart) = if board_id == "board-main" {
            (source_part, mirror_part)
        } else {
            (mirror_part, source_part)
        };
        assert_eq!(
            authored.pose, part.pose,
            "the requested side keeps its pose"
        );
        let expected_counterpart_at = if board_id == "board-main" {
            Vec2 { x: 202.0, y: -5.0 }
        } else {
            Vec2 { x: 2.0, y: 195.0 }
        };
        assert!(
            (counterpart.pose.at.x - expected_counterpart_at.x).abs() < 1e-9
                && (counterpart.pose.at.y - expected_counterpart_at.y).abs() < 1e-9,
            "rotated map gives counterpart position {:?}, expected {:?}",
            counterpart.pose.at,
            expected_counterpart_at
        );
        assert_eq!(
            counterpart.pose.rotation, 270.0,
            "the rotated reflection maps heading exactly"
        );
        assert!(
            added.boards.iter().any(|board| {
                board.id == halves.source_board_id
                    && board.part_ids.contains(&association.source_id)
            }),
            "source Part has a board owner"
        );
        assert!(
            added.boards.iter().any(|board| {
                board.id == halves.mirror_board_id
                    && board.part_ids.contains(&association.mirror_id)
            }),
            "mirror Part has a board owner"
        );
        assert!(
            halves.exceptions.iter().any(|exception| {
                exception.kind == MirroredItemKind::Part
                    && exception.mirror_id == "part-previous-mirror"
                    && exception.state == MirroredExceptionState::Omitted
            }),
            "the pre-existing Omitted association remains unchanged"
        );
        assert_eq!(
            added
                .definitions
                .iter()
                .filter(|item| item.id == definition.id)
                .count(),
            1
        );
        assert_eq!(added.matrices, before.matrices);
        assert_eq!(added.layouts, before.layouts);
    }
}

fn public_native_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let fixture = super::support::synthetic_half::document();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(fixture).unwrap(),
    });
    let pair_response = engine.request(&serde_json::json!({
        "kind": "edit", "id": "pair", "command": {
            "baseRevision": 0, "transactionId": "pair", "phase": "commit", "targetIds": [],
            "operation": {"kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "left",
                "mode": {"kind": "new-board", "boardId": "mirror", "instanceId": "mirror"},
                "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
                "idRule": "board-prefix", "mountingHole": null}
        }
    }).to_string());
    let (_, document) = scene(serde_json::from_str(&pair_response).unwrap());
    (engine, document)
}

fn matrixless_native_pair_with_omitted_part() -> (CoreEngine, ProjectDoc) {
    let mut document = ProjectDoc::empty("matrixless", "Matrixless native pair");
    let mut previous_definition = matrix_doc().definitions.remove(0);
    previous_definition.id = "catalog:previous-part".into();
    previous_definition.name = "Previous part".into();
    previous_definition.kind = PartKind::Passive;
    document.definitions.push(previous_definition.clone());
    document.parts.push(Part {
        anchor: None,
        id: "part-previous".into(),
        definition_id: previous_definition.id,
        reference: "J0".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards = vec![
        Board {
            id: "board-main".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec!["part-previous".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
        Board {
            id: "board-mirror".into(),
            name: "Mirror".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
    ];
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split",
            "transport": "wired",
            "instances": [
                {"id":"instance-main", "name":"Main", "boardId":"board-main",
                 "half":"left", "role":"central", "flipped":false,
                 "controllerPartId":null, "mechanical":null},
                {"id":"instance-mirror", "name":"Mirror", "boardId":"board-mirror",
                 "half":"right", "role":"central", "flipped":false,
                 "controllerPartId":null, "mechanical":null}
            ],
            "boards": [],
            "pairs": [{
                "id":"pair-main-mirror", "members":["instance-main", "instance-mirror"],
                "constructionSource":"instance-main",
                "mapping":{"at":{"x":200.0,"y":0.0},"rotation":90.0},
                "construction":{
                    "method":"printed", "mount":"rigid", "integratedPlateFrame":false,
                    "plateThickness":1.5, "plateFoamThickness":0.0, "bottomFoamThickness":0.0,
                    "bottomThickness":2.0, "plateToPcb":3.5, "wallThickness":2.0, "clearance":0.2
                },
                "features":[],
                "halves":{
                    "sourceBoardId":"board-main", "mirrorBoardId":"board-mirror",
                    "correspondences":[{"kind":"part","sourceId":"part-previous","mirrorId":"part-previous-mirror"}],
                    "exceptions":[{"kind":"part","mirrorId":"part-previous-mirror","state":"omitted"}]
                }
            }]
        }))
        .unwrap(),
    );
    let mut engine = CoreEngine::new();
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-matrixless-pair".into(),
        document,
    }));
    (engine, opened)
}

#[test]
fn mirror_only_part_is_target_owned_across_preview_history_and_open() {
    let (mut engine, mut accepted) = public_native_pair();
    accepted.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "mirror-only-envelope".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    let target_board_id = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == target_board_id)
        .unwrap()
        .outline_ids
        .push("mirror-only-envelope".into());
    engine.handle(CoreRequest::Open {
        id: "reopen-fixture".into(),
        document: accepted.clone(),
    });
    let (_, accepted) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-fixture".into(),
    }));
    let halves = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();
    let source_parts: Vec<_> = accepted
        .parts
        .iter()
        .filter(|part| {
            accepted.boards.iter().any(|board| {
                board.id == halves.source_board_id && board.part_ids.contains(&part.id)
            })
        })
        .cloned()
        .collect();
    let pair_before = serde_json::to_value(&accepted.hardware.as_ref().unwrap().pairs[0]).unwrap();
    let mut part = accepted.parts[0].clone();
    part.id = "mirror/only".into();
    part.reference = "MO1".into();
    part.pose.at = Vec2 { x: 230.0, y: 18.0 };
    let operation = EditOperation::AddMirrorOnlyPart {
        part: part.clone(),
        board_id: halves.mirror_board_id.clone(),
        definition: None,
        layout_id: None,
    };

    assert!(matches!(
        engine.handle(edit(
            accepted.revision,
            EditPhase::Preview,
            operation.clone()
        )),
        CoreReply::Preview { .. }
    ));
    let (_, after_preview) = scene(engine.handle(CoreRequest::Snapshot {
        id: "after-preview".into(),
    }));
    assert_eq!(
        after_preview.parts, accepted.parts,
        "Preview does not publish the target Part"
    );

    let (_, added) = scene(engine.handle(edit(accepted.revision, EditPhase::Commit, operation)));
    assert_eq!(
        added
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    assert_eq!(
        added
            .boards
            .iter()
            .find(|board| board.id == halves.mirror_board_id)
            .unwrap()
            .part_ids
            .iter()
            .filter(|id| **id == part.id)
            .count(),
        1
    );
    let resulting_source_parts: Vec<_> = added
        .parts
        .iter()
        .filter(|candidate| {
            added.boards.iter().any(|board| {
                board.id == halves.source_board_id && board.part_ids.contains(&candidate.id)
            })
        })
        .cloned()
        .collect();
    assert_eq!(
        resulting_source_parts, source_parts,
        "mirror-only addition leaves source parts untouched"
    );
    let pair_after = serde_json::to_value(&added.hardware.as_ref().unwrap().pairs[0]).unwrap();
    assert_eq!(
        pair_after, pair_before,
        "mirror-only addition creates no pair provenance"
    );
    let OutlineFeature::PartEnvelope { part_ids, .. } = added
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-only-envelope")
        .unwrap()
    else {
        panic!("expected target envelope")
    };
    assert_eq!(part_ids.iter().filter(|id| **id == part.id).count(), 1);
    let OutlineFeature::PartEnvelope { part_ids, .. } = added
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror/synthetic-envelope")
        .unwrap()
    else {
        panic!("expected native linked target envelope")
    };
    assert_eq!(
        part_ids.iter().filter(|id| **id == part.id).count(),
        1,
        "the target-owned Part is retained in the linked target envelope"
    );

    let (_, undone) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-mirror-only".into(),
    }));
    assert!(!undone.parts.iter().any(|candidate| candidate.id == part.id));
    let (_, redone) = scene(engine.handle(CoreRequest::Redo {
        id: "redo-mirror-only".into(),
    }));
    assert_eq!(
        redone
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    let serialized = serde_json::to_value(&redone).unwrap();
    let (_, reopened) = scene(engine.handle(CoreRequest::Open {
        id: "reopen-mirror-only".into(),
        document: serde_json::from_value(serialized).unwrap(),
    }));
    assert_eq!(
        reopened
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    let OutlineFeature::PartEnvelope { part_ids, .. } = reopened
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror/synthetic-envelope")
        .unwrap()
    else {
        panic!("expected native linked target envelope after Open")
    };
    assert_eq!(
        part_ids.iter().filter(|id| **id == part.id).count(),
        1,
        "Open retains the independent target envelope membership"
    );
}

#[test]
fn mirror_only_part_refuses_a_source_board_target() {
    let (mut engine, document) = public_native_pair();
    let halves = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();
    let mut prior = document.parts[0].clone();
    prior.id = "mirror/prior-redo".into();
    prior.reference = "PRIOR1".into();
    scene(engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: prior.clone(),
            board_id: halves.mirror_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )));
    scene(engine.handle(CoreRequest::Undo {
        id: "undo-before-refusal".into(),
    }));
    let (_, before_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "before-refusal".into(),
    }));

    let mut part = document.parts[0].clone();
    part.id = "wrong-half/only".into();
    part.reference = "WRONG1".into();
    let error = match engine.handle(edit(
        before_refusal.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part,
            board_id: halves.source_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )) {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected a mirror target refusal, got {other:?}"),
    };
    assert!(
        error.contains("Mirror-only placement requires one exact native mirror board"),
        "{error}"
    );
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "after-refusal".into(),
    }));
    assert_eq!(
        after_refusal, before_refusal,
        "refusal preserves the full document"
    );
    let (_, redone) = scene(engine.handle(CoreRequest::Redo {
        id: "redo-after-refusal".into(),
    }));
    assert!(
        redone
            .parts
            .iter()
            .any(|candidate| candidate.id == prior.id),
        "refusal preserves the existing Redo entry"
    );
}

#[test]
fn mirror_only_part_refuses_missing_and_unpaired_targets_atomically() {
    let (mut engine, mut document) = public_native_pair();
    let mut unpaired = document.boards[0].clone();
    unpaired.id = "unpaired-board".into();
    unpaired.name = "Unpaired board".into();
    unpaired.outline_ids.clear();
    unpaired.part_ids.clear();
    unpaired.net_ids.clear();
    unpaired.traces.clear();
    unpaired.vias.clear();
    document.boards.push(unpaired);
    scene(engine.handle(CoreRequest::Open {
        id: "open-unpaired-fixture".into(),
        document,
    }));
    let (_, baseline) = scene(engine.handle(CoreRequest::Snapshot {
        id: "mirror-only-target-baseline".into(),
    }));

    for (index, board_id) in ["missing-board", "unpaired-board"].into_iter().enumerate() {
        let mut part = baseline.parts[0].clone();
        part.id = format!("invalid-target/{index}");
        part.reference = format!("BAD{index}");
        let error = match engine.handle(edit(
            baseline.revision,
            EditPhase::Commit,
            EditOperation::AddMirrorOnlyPart {
                part,
                board_id: board_id.into(),
                definition: None,
                layout_id: None,
            },
        )) {
            CoreReply::Error { message, .. } => message,
            other => panic!("expected invalid target refusal for {board_id}, got {other:?}"),
        };
        assert!(
            error.contains("Mirror-only placement requires one exact native mirror board"),
            "{board_id}: {error}"
        );
        let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
            id: format!("snapshot-after-{board_id}"),
        }));
        assert_eq!(
            after_refusal, baseline,
            "refusing {board_id} preserves the complete document and revision"
        );
    }
}

#[test]
fn mirror_only_part_refuses_reusing_an_existing_part_identity_atomically() {
    let (mut engine, document) = public_native_pair();
    let mirror_board_id = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    let reused = document.parts[0].clone();
    let reply = engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: reused,
            board_id: mirror_board_id,
            definition: None,
            layout_id: None,
        },
    ));
    assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-after-identity-reuse".into(),
    }));
    assert_eq!(after_refusal, document);
}

#[test]
fn mirror_only_part_refuses_ids_owned_by_other_document_entities_atomically() {
    let (mut engine, accepted) = public_native_pair();
    let mirror_board_id = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    let reused_ids = [
        ("board", mirror_board_id.clone()),
        ("matrix", accepted.matrices[0].id.clone()),
        ("layout", accepted.layouts[0].id.clone()),
        ("outline", accepted.outline[0].id().to_owned()),
        ("net", accepted.nets[0].id.clone()),
    ];
    let mut prior = accepted.parts[0].clone();
    prior.id = "mirror/prior-redo".into();
    prior.reference = "PRIOR".into();
    scene(engine.handle(edit(
        accepted.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: prior,
            board_id: mirror_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )));
    let (_, before_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "before-identity-refusals".into(),
    }));

    for (kind, reused_id) in reused_ids {
        assert!(
            accepted.parts.iter().all(|part| part.id != reused_id),
            "the {kind} fixture identity must not already belong to a Part"
        );
        let mut part = accepted.parts[0].clone();
        part.id = reused_id;
        part.reference = format!("COLL-{kind}");
        let reply = engine.handle(edit(
            before_refusal.revision,
            EditPhase::Commit,
            EditOperation::AddMirrorOnlyPart {
                part,
                board_id: mirror_board_id.clone(),
                definition: None,
                layout_id: None,
            },
        ));
        assert!(
            matches!(reply, CoreReply::Error { .. }),
            "the {kind} identity was reused as a Part: {reply:?}"
        );
        let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
            id: format!("after-{kind}-identity-refusal"),
        }));
        assert_eq!(
            after_refusal, before_refusal,
            "refusal preserves the full document and revision for {kind} identity reuse"
        );
    }

    let (_, undone) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-after-identity-refusals".into(),
    }));
    let mut expected = accepted.clone();
    expected.revision = undone.revision;
    assert_eq!(
        undone, expected,
        "every refusal preserves the seeded commit's Undo entry"
    );
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
fn add_part_refuses_definition_id_mismatch_without_mutation() {
    let (mut engine, document) = linked_pair();
    let mut definition = document.definitions[0].clone();
    definition.id = "catalogue/other-switch".into();
    let mut part = document.parts[0].clone();
    part.id = "board/mismatched-definition".into();
    part.reference = "MISMATCH1".into();
    let reply = engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part,
            board_id: Some("board".into()),
            definition: Some(definition),
            layout_id: Some("left-layout".into()),
        },
    ));
    assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-after-definition-id-mismatch".into(),
    }));
    assert_eq!(after_refusal, document);
}

fn linked_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, doc) = scene(serde_json::from_str(&response).unwrap());
    (engine, doc)
}

fn assert_reflected_keys(doc: &ProjectDoc) {
    for left in doc.parts.iter().filter(|part| {
        part.id.starts_with("matrix/left-matrix/") && part.id.matches('/').count() == 2
    }) {
        let right = doc
            .parts
            .iter()
            .find(|part| part.id == left.id.replace("left-matrix", "right-matrix"))
            .unwrap();
        assert!(
            (left.pose.at.x + right.pose.at.x - 20.0).abs() < 1e-8,
            "{left:?} {right:?}"
        );
        assert!((left.pose.at.y - right.pose.at.y).abs() < 1e-8);
        assert!((left.pose.rotation + right.pose.rotation).abs() < 1e-8);
    }
}

#[test]
fn mirrored_pair_links_geometry_bidirectionally_and_keeps_hardware_local() {
    let (mut engine, mut doc) = linked_pair();
    let mut right = doc.matrices[1].clone();
    right.cells.push(MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        assemblies_local: Some(true),
        row: 0,
        column: 1,
        enabled: true,
        variant: Some("right-only".into()),

        definition_id: None,
        offset: None,
        rotation: None,
        assemblies: vec![MatrixAssembly {
            id: "encoder".into(),
            definition_id: "switch".into(),
            offset: Vec2 { x: 3.0, y: -11.0 },
            rotation: Some(22.0),
            side: None,
        }],
    });
    doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )))
    .1;
    for side in [0, 1] {
        let mut matrix = doc.matrices[side].clone();
        matrix.rows = 3;
        matrix.pitch = Vec2 { x: 20.0, y: 18.0 };
        {
            let values = vec![2.0, -3.0, 7.0];
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
        {
            let values = vec![5.0, 12.0, -8.0];
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
            let values = (vec![
                Some(Vec2 { x: -9.0, y: 17.0 }),
                None,
                Some(Vec2 { x: 25.0, y: -12.0 }),
            ])
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
        matrix.rotation = Some(13.0);
        matrix
            .cells
            .retain(|cell| cell.row != 1 || cell.column != 2);
        matrix.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 1,
            column: 2,
            enabled: false,

            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
        });
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == 0 && cell.column == 1)
            .unwrap();
        cell.offset = Some(Vec2 { x: 2.0, y: 4.0 });
        cell.rotation = Some(7.0);
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        assert_reflected_keys(&doc);
        assert_eq!(doc.matrices[0].rows, 3);
        assert_eq!(doc.matrices[1].rows, 3);
        assert!(
            !doc.parts
                .iter()
                .any(|part| part.id == "matrix/left-matrix/r1c2"
                    || part.id == "matrix/right-matrix/r1c2")
        );
        assert!(
            !doc.parts
                .iter()
                .any(|part| part.id == "matrix/left-matrix/r0c1/encoder")
        );
        let companion = doc.matrices[1]
            .cells
            .iter()
            .find(|cell| cell.row == 0 && cell.column == 1)
            .unwrap();
        assert_eq!(companion.variant.as_deref(), Some("right-only"));
        assert_eq!(companion.assemblies[0].offset.x, 3.0);
    }
}

#[test]
fn mirrored_pair_previews_restore_both_halves_and_nudges_stay_parametric() {
    let (mut engine, doc) = linked_pair();
    let mut next = doc.matrices[1].clone();
    next.columns = 4;
    preview(engine.handle(edit(
        doc.revision,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: next,
            definitions: None,
        },
    )));
    let snapshot = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .1;
    assert_eq!(snapshot, doc);
    let id = "matrix/right-matrix/r0c0";
    let original = doc.parts.iter().find(|part| part.id == id).unwrap();
    let operation = EditOperation::MoveParts {
        positions: vec![Position {
            id: id.into(),
            at: Vec2 {
                x: original.pose.at.x + 4.0,
                y: original.pose.at.y + 3.0,
            },
        }],
    };
    preview(engine.handle(edit(doc.revision, EditPhase::Preview, operation.clone())));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
    let doc = scene(engine.handle(edit(doc.revision, EditPhase::Commit, operation))).1;
    assert_reflected_keys(&doc);
    assert!(doc.matrices[1].cells[0].offset.is_some());
    let mut grow = doc.matrices[0].clone();
    grow.columns += 1;
    let grown = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: grow,
            definitions: None,
        },
    )))
    .1;
    assert_eq!(
        grown.parts.iter().find(|part| part.id == id).unwrap().pose,
        doc.parts.iter().find(|part| part.id == id).unwrap().pose
    );
    assert_reflected_keys(&grown);
}

/// Board-space pose of a part from the scene transforms.
fn board_pose(scene: &SceneDelta, id: &str) -> Pose2 {
    scene
        .transforms
        .iter()
        .find(|transform| transform.id == id)
        .unwrap_or_else(|| panic!("missing transform {id}"))
        .pose
}

#[test]
fn mirrored_pair_links_extra_components_and_unlink_preserves_poses() {
    let (mut engine, doc) = linked_pair();
    let mut local = doc.parts[0].clone();
    local.id = "left-encoder".into();
    local.reference = "ENC1".into();
    let mut doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::AddPart {
            part: local.clone(),
            board_id: Some("board".into()),
            definition: None,
            layout_id: None,
        },
    )))
    .1;
    let mut left = doc.layouts[0].clone();
    if !left.part_ids.contains(&local.id) {
        left.part_ids.push(local.id.clone());
    }
    let (joined_scene, joined) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetLayout { layout: left },
    )));
    doc = joined;
    // Ticket 12: joining the half converts the drop to half-frame storage;
    // the evaluated board pose is unchanged.
    assert_eq!(board_pose(&joined_scene, &local.id), local.pose);
    let mut right = doc.matrices[1].clone();
    {
        let values = vec![7.0];
        let count = values.len().max(right.column_layout.len());
        right.column_layout.resize_with(count, Default::default);
        for record in &mut right.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in right.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    let (mirrored_scene, mirrored) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )));
    doc = mirrored;
    assert_eq!(doc.layouts[1].part_ids.len(), 1);
    // Ticket 12: the linked partner mirrors in the half frame (stored halves
    // negate about the origin) and evaluates to the board reflection.
    let stored = doc
        .parts
        .iter()
        .find(|part| part.id == local.id)
        .unwrap()
        .pose;
    let counterpart = doc
        .parts
        .iter()
        .find(|part| part.id == doc.layouts[1].part_ids[0])
        .unwrap();
    assert!((counterpart.pose.at.x + stored.at.x).abs() < 1e-8);
    assert!((counterpart.pose.at.y - stored.at.y).abs() < 1e-8);
    let placed = board_pose(&mirrored_scene, &local.id);
    let mirrored_board = board_pose(&mirrored_scene, &counterpart.id);
    assert!((mirrored_board.at.x + placed.at.x - 20.0).abs() < 1e-8);
    assert!((mirrored_board.at.y - placed.at.y).abs() < 1e-8);
    let mut independent = doc.layouts[1].clone();
    independent.mirror_link = None;
    let (unlinked_scene, unlinked) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetLayout {
            layout: independent,
        },
    )));
    // Unlink preserves every evaluated pose; storage returns board-space.
    assert_eq!(
        unlinked_scene
            .transforms
            .iter()
            .map(|transform| (&transform.id, transform.pose))
            .collect::<Vec<_>>(),
        mirrored_scene
            .transforms
            .iter()
            .map(|transform| (&transform.id, transform.pose))
            .collect::<Vec<_>>()
    );
    let mut change = unlinked.matrices[0].clone();
    change.rows += 1;
    let changed = scene(engine.handle(edit(
        unlinked.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: change,
            definitions: None,
        },
    )))
    .1;
    assert_eq!(changed.matrices[1], unlinked.matrices[1]);
}

#[test]
fn mirrored_pair_deletion_keeps_surviving_half_and_cleans_membership() {
    // Ticket 21 supersedes the old linked survival: a plain linked
    // counterpart follows the source deletion, while a Local counterpart
    // survives complete and its layout turns independent. Name kept for the
    // native baseline; the linked follow-deletion itself is proven in
    // `matrix_anchors::linked_source_removal_deletes_linked_partner_and_keeps_unrelated_pair`.
    let (mut engine, doc) = linked_pair();
    let removed = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["matrix/left-matrix/r0c0".into()],
        },
    )))
    .1;
    assert_eq!(removed.parts.len(), 10);
    assert_reflected_keys(&removed);
    // The surviving half goes Local before the source is deleted.
    let mut right_layout = removed.layouts[1].clone();
    right_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            mirror_id: "right-matrix".into(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
    let removed = scene(engine.handle(edit(
        removed.revision,
        EditPhase::Commit,
        EditOperation::SetLayout {
            layout: right_layout,
        },
    )))
    .1;
    let surviving = removed.matrices[1].clone();
    let removed = scene(engine.handle(edit(
        removed.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrix {
            id: "left-matrix".into(),
        },
    )))
    .1;
    assert_eq!(removed.matrices, vec![surviving]);
    assert_eq!(removed.layouts.len(), 1);
    assert!(removed.layouts[0].mirror_link.is_none());
    assert_eq!(removed.parts.len(), 5);
}

#[test]
fn mirrored_pair_invalid_links_and_failed_previews_do_not_change_document() {
    let (mut engine, doc) = linked_pair();
    let mut invalid = doc.matrices[0].clone();
    invalid.mirror = Some(Mirror::Y);
    assert!(matches!(
        engine.handle(edit(
            doc.revision,
            EditPhase::Preview,
            EditOperation::SetMatrix {
                matrix: invalid,
                definitions: None
            }
        )),
        CoreReply::Error { .. }
    ));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
    let mut cycle = doc.layouts[0].clone();
    cycle.mirror_link = Some(LayoutMirrorLink {
        correspondences: vec![],
        exceptions: vec![],
        source_id: "right-layout".into(),
        axis_x: 10.0,
        split_gap: None,
        spacing: None,
    });
    assert!(matches!(
        engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetLayout { layout: cycle }
        )),
        CoreReply::Error { .. }
    ));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
}

fn matrix(rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: 0.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

#[test]
fn matrix_shrink_discards_out_of_bounds_edits() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
        EditOperation::SetMatrix {
            matrix: value.clone(),
            definitions: None,
        },
    )));
    value.rows = 5;
    value.columns = 6;
    let (_, shrunk) = scene(engine.handle(edit(
        original.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
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
fn new_matrix_rejects_out_of_bounds_edits() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
        matches!(engine.handle(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value.clone(), definitions: None })), CoreReply::Error { message, .. } if message == "layout list exceeds axis count")
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
        matches!(engine.handle(edit(0, EditPhase::Commit, EditOperation::SetMatrix { matrix: value, definitions: None })), CoreReply::Error { message, .. } if message == "invalid or duplicate cell coordinate")
    );
}

#[test]
fn matrix_cells_preserve_survivors_and_companions() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(2, 2);
    {
        let values = vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 3.0, y: 0.0 }];
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
    value.cells = vec![
        MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 0,
            column: 1,
            enabled: false,

            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
        },
        MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            assemblies_local: None,
            row: 1,
            column: 0,
            enabled: true,

            definition_id: None,
            variant: Some("hotswap".into()),
            offset: Some(Vec2 { x: 2.0, y: 1.0 }),
            rotation: Some(10.0),
            assemblies: vec![MatrixAssembly {
                id: "led".into(),
                definition_id: "switch".into(),
                offset: Vec2 { x: 0.0, y: 4.0 },
                rotation: None,
                side: None,
            }],
        },
    ];
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value.clone(),
            definitions: None,
        },
    )));
    assert_eq!(doc.matrices[0].part_ids.len(), 4);
    assert!(!doc.parts.iter().any(|part| part.id == "matrix/main/r0c1"));
    let host = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0")
        .unwrap();
    assert_eq!(host.pose.at, Vec2 { x: 5.0, y: 20.0 });
    assert_eq!(host.pose.rotation, 10.0);
    assert_eq!(host.properties.as_ref().unwrap()["variant"], "hotswap");
    let companion = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0/led")
        .unwrap();
    assert!((companion.pose.at.x - 4.305407289332279).abs() < 1e-9);
    assert!((companion.pose.at.y - 23.939231012048833).abs() < 1e-9);
    let reference = host.reference.clone();
    value.columns = 3;
    let (_, grown) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value.clone(),
            definitions: None,
        },
    )));
    assert_eq!(
        grown
            .parts
            .iter()
            .find(|part| part.id == host.id)
            .unwrap()
            .reference,
        reference
    );
    assert!(grown.parts.iter().any(|part| part.id == "matrix/main/r0c2"));
}

#[test]
fn matrix_duplicate_allocates_new_member_ids() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, original) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    let mut duplicate = original.matrices[0].clone();
    duplicate.id = "copy".into();
    duplicate.origin.x = 30.0;
    let (_, doc) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: duplicate,
            definitions: None,
        },
    )));
    assert_eq!(doc.matrices[0].part_ids, vec!["matrix/main/r0c0"]);
    assert_eq!(doc.matrices[1].part_ids, vec!["matrix/copy/r0c0"]);
    assert_ne!(doc.parts[0].reference, doc.parts[1].reference);
}

#[test]
fn failed_matrix_preview_restores_document() {
    let mut doc = matrix_doc();
    doc.parts.push(Part {
        anchor: None,
        id: "matrix/main/r0c1".into(),
        definition_id: "switch".into(),
        reference: "SW99".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let mut engine = CoreEngine::new();
    let (_, original) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let reply = engine.handle(edit(
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: matrix(1, 2),
            definitions: None,
        },
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
fn matrix_registers_definitions_atomically() {
    let mut doc = matrix_doc();
    let definitions = std::mem::take(&mut doc.definitions);
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let operation = EditOperation::SetMatrix {
        matrix: matrix(1, 1),
        definitions: Some(definitions),
    };
    let projected = preview(engine.handle(edit(0, EditPhase::Preview, operation.clone())));
    assert_eq!(projected.transforms.len(), 1);
    let (_, snapshot) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert!(snapshot.definitions.is_empty());
    assert!(snapshot.parts.is_empty());
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
    let (_, placed) = scene(engine.handle(edit(0, EditPhase::Commit, operation)));
    assert_eq!(placed.definitions.len(), 1);
    assert_eq!(placed.parts.len(), 1);
    let mut conflicting = placed.definitions[0].clone();
    conflicting.name = "Changed".into();
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: matrix(1, 1),
                definitions: Some(vec![conflicting]),
            }
        )),
        CoreReply::Error { .. }
    ));
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(undone.definitions.is_empty());
    assert!(undone.parts.is_empty());
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
        id: "right".into(),
        name: "Right".into(),
        outline_ids: vec!["right-edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let mut target = matrix(1, 2);
    target.board_id = Some("right".into());
    let preview = preview(engine.handle(edit(
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: target.clone(),
            definitions: None,
        },
    )));
    assert_eq!(preview.board_contours[1].contours.len(), 1);
    assert!(preview.board_contours[0].contours.is_empty());
    assert!(preview.board_readiness[1].pcb);
    assert!(!preview.board_readiness[0].pcb);
    assert!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts
        .is_empty()
    );
    let (_, placed) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: target.clone(),
            definitions: None,
        },
    )));
    assert_eq!(placed.boards[1].part_ids.len(), 2);
    assert!(placed.boards[0].part_ids.is_empty());
    let OutlineFeature::PartEnvelope { part_ids, .. } = &placed.outline[1] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids.len(), 2);
    let mut edit_matrix = target;
    edit_matrix.board_id = None;
    let (_, updated) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: edit_matrix,
            definitions: None,
        },
    )));
    assert_eq!(updated.matrices[0].board_id.as_deref(), Some("right"));
    assert_eq!(updated.boards[1].part_ids.len(), 2);
    let mut invalid = matrix(1, 1);
    invalid.board_id = Some("missing".into());
    assert!(matches!(
        engine.handle(edit(
            2,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: invalid,
                definitions: None,
            }
        )),
        CoreReply::Error { .. }
    ));
}

#[test]
fn matrix_generates_stable_parts_and_one_undo_step() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let preview = preview(engine.handle(edit(
        0,
        EditPhase::Preview,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
    )));
    assert_eq!(preview.transforms.len(), 4);
    assert!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1
        .parts
        .is_empty()
    );
    let (committed_scene, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
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
    let (undone, doc) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.revision, 2);
    assert!(doc.parts.is_empty());
}

#[test]
fn matrix_edit_preserves_moved_member_and_prunes_removed() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
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
    let (_, doc) = scene(engine.handle(edit(
        2,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: changed,
            definitions: None,
        },
    )));
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
fn matrix_mirror_rotation_and_bad_dimensions() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(1, 2);
    value.mirror = Some(Mirror::X);
    value.rotation = Some(90.0);
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
    let second = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r0c1")
        .unwrap();
    assert!((second.pose.at.x).abs() < 1e-9);
    assert!((second.pose.at.y + 19.0).abs() < 1e-9);
    assert_eq!(doc.matrices[0].edge_gap, Some(Vec2 { x: 1.0, y: 1.0 }));
    let mut bad_gap = matrix(1, 1);
    bad_gap.edge_gap = Some(Vec2 { x: 19.0, y: 1.0 });
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: bad_gap,
                definitions: None,
            }
        )),
        CoreReply::Error { .. }
    ));
    let bad = matrix(0, 3);
    assert!(matches!(
        engine.handle(edit(
            1,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: bad,
                definitions: None,
            }
        )),
        CoreReply::Error { revision: 1, .. }
    ));
}

#[test]
fn matrix_shrink_prunes_net_pins_and_stale_members() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 2),
            definitions: None,
        },
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
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
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
fn constraint_preview_commit_and_undo() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    for id in ["source", "target"] {
        doc.parts.push(Part {
            anchor: None,
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
    }
    doc.constraints.push(Constraint::Offset {
        id: "offset".into(),
        source_part_id: "source".into(),
        target_part_id: "target".into(),
        offset: Vec2 { x: 10.0, y: 0.0 },
        rotation: 0.0,
    });
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
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
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
    let (_, grown) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
    let (_, shrunk) = scene(engine.handle(edit(
        2,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
fn deleting_matrix_removes_its_container_and_parts_in_one_edit() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, before) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(2, 2),
            definitions: None,
        },
    )));
    let operation: EditOperation =
        serde_json::from_value(serde_json::json!({"kind":"remove-matrix", "id":"main"})).unwrap();
    let (_, removed) = scene(engine.handle(edit(before.revision, EditPhase::Commit, operation)));
    assert!(removed.matrices.is_empty());
    assert!(
        !removed
            .parts
            .iter()
            .any(|part| before.matrices[0].part_ids.contains(&part.id))
    );
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(restored.matrices, before.matrices);
    assert_eq!(restored.parts, before.parts);
    let (_, deleted_again) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert!(deleted_again.matrices.is_empty());
    let (_, restored) = scene(engine.handle(CoreRequest::Undo {
        id: "restore".into(),
    }));
    let (_, empty) = scene(engine.handle(edit(
        restored.revision,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: restored.matrices[0].part_ids.clone(),
        },
    )));
    assert!(empty.matrices[0].part_ids.is_empty());
    let (_, removed_empty) = scene(engine.handle(edit(
        empty.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrix { id: "main".into() },
    )));
    assert!(removed_empty.matrices.is_empty());
    let (_, restored_empty) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-empty".into(),
    }));
    assert_eq!(restored_empty.matrices, empty.matrices);
}

#[test]
fn matrix_custom_splay_origin_survives_export_and_resize() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
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
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
    let (_, shrunk) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
fn replacing_matrix_switch_remaps_existing_terminal_nets_and_undoes() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads = ["one", "two"].iter().enumerate().map(|(i, id)| serde_json::from_value(serde_json::json!({
        "id": id, "number": (i + 1).to_string(), "at": {"x": i, "y": 0}, "size": {"x": 1, "y": 1}, "shape": "circle"
    })).unwrap()).collect();
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let (_, mut placed) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
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
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: placed.clone(),
    });
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
        engine.handle(edit(
            placed.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix: incoming.clone(),
                definitions: Some(vec![conflicting])
            }
        )),
        CoreReply::Error { .. }
    ));
    let (_, changed) = scene(engine.handle(edit(
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
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.nets, placed.nets);
    assert_eq!(undone.parts, placed.parts);
}

#[test]
fn mirrored_resize_keeps_each_halves_local_preset_companions() {
    for changed_side in [0, 1] {
        let (mut engine, mut doc) = linked_pair();
        for side in [0, 1] {
            let mut matrix = doc.matrices[side].clone();
            matrix.cells = (0..matrix.rows)
                .flat_map(|row| {
                    (0..matrix.columns).map(move |column| MatrixCell {
                        shift: None,
                        bend: None,
                        deleted: false,
                        row,
                        column,
                        enabled: true,

                        definition_id: Some("switch".into()),
                        variant: Some("preset/mx-rgb/south".into()),
                        offset: None,
                        rotation: None,
                        assemblies_local: Some(true),
                        assemblies: vec![MatrixAssembly {
                            id: format!("local-{side}"),
                            definition_id: "switch".into(),
                            offset: Vec2 { x: 2.0, y: 3.0 },
                            rotation: None,
                            side: None,
                        }],
                    })
                })
                .collect();
            doc = scene(engine.handle(edit(
                doc.revision,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            )))
            .1;
        }
        let mut matrix = doc.matrices[changed_side].clone();
        matrix.rows = 3;
        for column in 0..matrix.columns {
            let mut cell = matrix.cells[0].clone();
            cell.row = 2;
            cell.column = column;
            matrix.cells.push(cell);
        }
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        for side in [0, 1] {
            let cells = &doc.matrices[side].cells;
            assert_eq!(cells.len(), 9);
            for cell in cells {
                assert_eq!(cell.assemblies.len(), 1, "side {side}, row {}", cell.row);
                assert_eq!(cell.assemblies[0].id, format!("local-{side}"));
            }
        }
    }
}

#[test]
fn deleting_matrix_cells_removes_geometry_and_assembly_but_disabled_cells_stay() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(1, 3);
    value.cells = serde_json::from_value(serde_json::json!([
        {"row":0,"column":0,"enabled":true,"assemblies":[{"id":"led","definitionId":"switch","offset":{"x":0,"y":4}}]},
        {"row":0,"column":1,"enabled":false}
    ])).unwrap();
    scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
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
fn deleting_matrix_row_preserves_surviving_part_and_binding_identity() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(3, 1),
            definitions: None,
        },
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

#[test]
fn linked_matrix_resize_retires_out_of_bounds_correspondences_and_reopens() {
    let (mut engine, mut doc) = linked_pair();
    for columns in [2, 4] {
        let mut matrix = doc.matrices[0].clone();
        matrix.columns = columns;
        doc = scene(engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        )))
        .1;
        let link = doc.layouts[1].mirror_link.as_ref().unwrap();
        for row in 0..doc.matrices[0].rows {
            for column in 0..columns {
                assert!(
                    link.correspondences.iter().any(|record| {
                        record.source_id == format!("matrix/left-matrix/r{row}c{column}")
                            && record.mirror_id == format!("matrix/right-matrix/r{row}c{column}")
                    }),
                    "missing r{row}c{column} after resizing to {columns} columns"
                );
            }
        }
        if columns == 2 {
            assert!(
                link.correspondences.iter().all(|record| {
                    !record.source_id.ends_with("r0c2") && !record.mirror_id.ends_with("r0c2")
                }),
                "the retired r0c2 pair must stay gone: {link:?}"
            );
        }
        let (_, reopened) = scene(CoreEngine::new().handle(CoreRequest::Open {
            id: "reopen-resized".into(),
            document: doc.clone(),
        }));
        assert_eq!(reopened.layouts, doc.layouts);
    }
}

#[test]
fn deleting_matrix_column_compacts_both_halves_and_preserves_local_assemblies() {
    let (mut engine, mut doc) = linked_pair();
    let mut right = doc.matrices[1].clone();
    right.cells = serde_json::from_value(serde_json::json!([
        {"row":0,"column":2,"enabled":true,"variant":"right-only","assembliesLocal":true,
        "assemblies":[{"id":"led","definitionId":"switch","offset":{"x":3,"y":4}}]}
    ]))
    .unwrap();
    doc = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: right,
            definitions: None,
        },
    )))
    .1;
    let reference = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-matrix/r0c2/led")
        .unwrap()
        .reference
        .clone();
    let before = doc.clone();
    let (_, compacted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "right-matrix".into(),
            column: 1,
        },
    )));
    assert!(compacted.matrices.iter().all(|matrix| matrix.columns == 2));
    let companion = compacted
        .parts
        .iter()
        .find(|part| part.id == "matrix/right-matrix/r0c1/led")
        .unwrap();
    assert_eq!(companion.reference, reference);
    assert!(
        !compacted
            .parts
            .iter()
            .any(|part| part.id == "matrix/left-matrix/r0c1/led")
    );
    assert!(
        compacted.matrices[1]
            .cells
            .iter()
            .any(|cell| cell.column == 1 && cell.variant.as_deref() == Some("right-only"))
    );
    let link = compacted.layouts[1].mirror_link.as_ref().unwrap();
    let mut pairs: Vec<_> = link
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect();
    pairs.sort();
    let mut expected: Vec<_> = (0..compacted.matrices[0].rows)
        .flat_map(|row| {
            (0..compacted.matrices[0].columns).map(move |column| {
                (
                    format!("matrix/left-matrix/r{row}c{column}"),
                    format!("matrix/right-matrix/r{row}c{column}"),
                )
            })
        })
        .collect();
    expected.sort();
    assert_eq!(pairs, expected, "only the surviving pairs may remain");
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.parts, before.parts);
    assert_eq!(undone.matrices, before.matrices);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.parts, compacted.parts);
    assert_eq!(redone.matrices, compacted.matrices);
}

#[test]
fn deleting_matrix_cells_propagates_tombstones_and_survives_reopen() {
    let (mut engine, doc) = linked_pair();
    let (geometry, deleted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixCells {
            matrix_id: "right-matrix".into(),
            cells: vec![(0, 1), (1, 2)],
        },
    )));
    for matrix in &deleted.matrices {
        assert!(
            matrix
                .cells
                .iter()
                .any(|cell| cell.row == 0 && cell.column == 1 && cell.deleted)
        );
        assert!(
            matrix
                .cells
                .iter()
                .any(|cell| cell.row == 1 && cell.column == 2 && cell.deleted)
        );
        assert!(
            !matrix
                .part_ids
                .iter()
                .any(|id| id.ends_with("r0c1") || id.ends_with("r1c2"))
        );
    }
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 4)
    );
    let document = serde_json::from_value(serde_json::to_value(&deleted).unwrap()).unwrap();
    let (geometry, reopened) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document,
    }));
    assert_eq!(reopened.parts, deleted.parts);
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 4)
    );
}

#[test]
fn unrelated_matrix_edit_never_resurrects_a_tombstoned_member_correspondence() {
    let (mut engine, doc) = linked_pair();
    let (_, deleted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixCells {
            matrix_id: "left-matrix".into(),
            cells: vec![(0, 1)],
        },
    )));
    let retired = |link: &LayoutMirrorLink| {
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        })
    };
    let link = deleted.layouts[1].mirror_link.as_ref().unwrap();
    assert!(!retired(link), "delete must retire the pair: {link:?}");
    let mut left = deleted.matrices[0].clone();
    left.edge_gap = Some(Vec2 { x: 6.0, y: 6.0 });
    let (_, edited) = scene(engine.handle(edit(
        deleted.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: left,
            definitions: None,
        },
    )));
    let link = edited.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !retired(link),
        "an unrelated edit must not resurrect the tombstoned pair: {link:?}"
    );
}

#[test]
fn unrelated_matrix_edit_keeps_a_local_exception_unassociated() {
    let (mut engine, mut doc) = linked_pair();
    let link = doc.layouts[1].mirror_link.as_mut().unwrap();
    link.correspondences.retain(|record| {
        !(record.kind == MirroredItemKind::Part && record.mirror_id == "matrix/right-matrix/r0c1")
    });
    link.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: "matrix/right-matrix/r0c1".into(),
        state: MirroredExceptionState::Local,
        removed_copper: None,
    });
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-unassociated".into(),
        document: doc,
    }));
    let mut left = opened.matrices[0].clone();
    left.edge_gap = Some(Vec2 { x: 6.0, y: 6.0 });
    let (_, edited) = scene(engine.handle(edit(
        opened.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: left,
            definitions: None,
        },
    )));
    let link = edited.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        }),
        "pairing must not infer a source for a local exception: {link:?}"
    );
}

#[test]
fn deleting_last_matrix_row_removes_matrix_and_key_metadata_and_is_undoable() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    doc.keymap = Some(KeymapConfiguration::default());
    doc.keymap.as_mut().unwrap().layers[0]
        .bindings
        .insert("matrix/main/r0c0".into(), KeyBinding::None);
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc.clone(),
    });
    let (_, removed) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixRow {
            matrix_id: "main".into(),
            row: 0,
        },
    )));
    assert!(removed.matrices.is_empty());
    assert!(removed.parts.is_empty());
    assert!(removed.keymap.unwrap().layers[0].bindings.is_empty());
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(restored.matrices, doc.matrices);
    assert_eq!(restored.keymap, doc.keymap);
}

#[test]
fn deleting_matrix_row_remaps_nets_constraints_keycaps_and_direct_wiring() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(3, 1),
            definitions: None,
        },
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
            engine.handle(CoreRequest::Open {
                id: "open".into(),
                document: matrix_doc(),
            });
            let mut value = matrix(if removal == 2 { 1 } else { 2 }, 1);
            value.origin = Vec2 { x: 100.0, y: 20.0 };
            value.rotation = Some(90.0);
            let (_, mut doc) = scene(engine.handle(edit(
                0,
                EditPhase::Commit,
                EditOperation::SetMatrix {
                    matrix: value,
                    definitions: None,
                },
            )));
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

#[test]
fn grouped_matrix_cell_actions_are_atomic_across_unlinked_matrices() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 2),
            definitions: None,
        },
    )));
    let mut other = matrix(1, 2);
    other.id = "other".into();
    other.origin.x = 60.0;
    let (_, before) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: other,
            definitions: None,
        },
    )));
    let operation = serde_json::from_value(serde_json::json!({"kind":"set-matrix-cells-enabled", "cells":{"main":[[0,0]],"other":[[0,1]]}, "enabled":false})).expect("Core must support grouped matrix enable changes atomically");
    let (geometry, disabled) =
        scene(engine.handle(edit(before.revision, EditPhase::Commit, operation)));
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
    let (_, enabled) = scene(engine.handle(edit(disabled.revision, EditPhase::Commit, operation)));
    assert!(
        enabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 2)
    );
    let operation = serde_json::from_value(serde_json::json!({"kind":"remove-selected-matrix-cells", "cells":{"main":[[0,0]],"other":[[0,1]]}})).expect("Core must delete a grouped selection atomically");
    let (geometry, deleted) =
        scene(engine.handle(edit(enabled.revision, EditPhase::Commit, operation)));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, enabled.matrices);
    assert_eq!(undone.parts, enabled.parts);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.matrices, deleted.matrices);
    let document = serde_json::from_value(serde_json::to_value(&deleted).unwrap()).unwrap();
    let (geometry, _) = scene(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document,
    }));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 1)
    );
}

#[test]
fn grouped_matrix_cell_actions_deduplicate_linked_halves_and_reject_deleted_targets() {
    let (mut engine, before) = linked_pair();
    let cells = [
        ("left-matrix".into(), vec![(0, 0), (0, 1)]),
        ("right-matrix".into(), vec![(0, 1), (1, 2)]),
    ]
    .into_iter()
    .collect();
    let (_, disabled) = scene(engine.handle(edit(
        before.revision,
        EditPhase::Commit,
        EditOperation::SetMatrixCellsEnabled {
            cells,
            enabled: false,
        },
    )));
    assert!(
        disabled
            .matrices
            .iter()
            .all(|matrix| matrix.part_ids.len() == 3)
    );
    let cells = [
        ("left-matrix".into(), vec![(0, 1)]),
        ("right-matrix".into(), vec![(0, 1)]),
    ]
    .into_iter()
    .collect();
    let (geometry, deleted) = scene(engine.handle(edit(
        disabled.revision,
        EditPhase::Commit,
        EditOperation::RemoveSelectedMatrixCells { cells },
    )));
    assert!(
        geometry
            .matrix_scenes
            .iter()
            .all(|matrix| matrix.cells.len() == 5)
    );
    assert!(
        deleted.matrices.iter().all(|matrix| matrix
            .cells
            .iter()
            .filter(|cell| cell.deleted)
            .count()
            == 1)
    );
    let reply = engine.handle(edit(
        deleted.revision,
        EditPhase::Commit,
        EditOperation::SetMatrixCellsEnabled {
            cells: [
                ("left-matrix".into(), vec![(0, 1)]),
                ("right-matrix".into(), vec![(1, 0)]),
            ]
            .into_iter()
            .collect(),
            enabled: true,
        },
    ));
    assert!(
        matches!(reply, CoreReply::Error { message, .. } if message == "Matrix cell was deleted")
    );
    let reply = engine.handle(edit(
        deleted.revision,
        EditPhase::Commit,
        EditOperation::RemoveSelectedMatrixCells {
            cells: [
                ("left-matrix".into(), vec![(1, 0)]),
                ("missing".into(), vec![(0, 0)]),
            ]
            .into_iter()
            .collect(),
        },
    ));
    assert!(
        matches!(reply, CoreReply::Error { message, .. } if message == "Matrix does not exist")
    );
    let (_, unchanged) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(unchanged, deleted);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, disabled.matrices);
}

#[test]
fn undo_retains_only_the_latest_hundred_commits() {
    let mut engine = CoreEngine::new();
    let mut seed = ProjectDoc::empty("bounded-history", "0");
    seed.parts.push(Part {
        anchor: None,
        id: "part-0".into(),
        definition_id: "catalog:unused".into(),
        reference: "P0".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    let (_, mut document) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: seed,
    }));
    for index in 1..=101 {
        (_, document) = scene(engine.handle(edit(
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
        (_, document) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    }
    assert_eq!(document.parts[0].pose.at.x, 1.0);
    assert!(matches!(
        engine.handle(CoreRequest::Undo {
            id: "too-old".into()
        }),
        CoreReply::Error { .. }
    ));
    for _ in 0..100 {
        (_, document) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    }
    assert_eq!(document.parts[0].pose.at.x, 101.0);
    (_, document) = scene(engine.handle(CoreRequest::Undo {
        id: "branch".into(),
    }));
    scene(engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "part-0".into(),
                at: Vec2 { x: 0.5, y: 0.0 },
            }],
        },
    )));
    assert!(matches!(
        engine.handle(CoreRequest::Redo {
            id: "no-redo".into()
        }),
        CoreReply::Error { .. }
    ));
    scene(engine.handle(CoreRequest::Open {
        id: "fresh".into(),
        document: ProjectDoc::empty("fresh", "Fresh"),
    }));
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "empty".into() }),
        CoreReply::Error { .. }
    ));
}

#[test]
fn adding_a_link_never_recreates_a_pair_excused_by_a_local_exception() {
    let (mut engine, mut doc) = linked_pair();
    let mut link = doc.layouts[1].mirror_link.take().unwrap();
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-unlinked".into(),
        document: doc.clone(),
    }));
    link.correspondences.clear();
    link.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: "matrix/right-matrix/r0c1".into(),
        state: MirroredExceptionState::Local,
        removed_copper: None,
    });
    let mut layout = opened.layouts[1].clone();
    layout.mirror_link = Some(link);
    let (_, linked) = scene(engine.handle(edit(
        opened.revision,
        EditPhase::Commit,
        EditOperation::SetLayout { layout },
    )));
    let link = linked.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        }),
        "adding a link must not recreate an excused pair: {link:?}"
    );
}
