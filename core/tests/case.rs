use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;

fn outline(id: &str) -> OutlineFeature {
    OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: id.into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 20.0, y: 10.0 },
        radius: 0.0,
        operation: Operation::Add,
    }
}

fn commit_case(base_revision: u64, id: &str) -> CoreRequest {
    // Ticket 37: raw SetOutline is deleted; the history setup below uses the live
    // typed case write instead.
    CoreRequest::Edit {
        id: format!("edit-{id}"),
        command: EditCommand {
            base_revision,
            transaction_id: format!("tx-{id}"),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetCase {
                body: CaseBody {
                    features: None,
                    openings: None,
                    id: id.into(),
                    name: id.into(),
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
            },
        },
    }
}

fn case_ir(assembly_revision: u64, body_revision: u64) -> CaseAssemblyIR {
    CaseAssemblyIR {
        revision: assembly_revision,
        bodies: vec![CaseIR {
            revision: body_revision,
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
            contours: vec![Contour {
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 20.0, y: 0.0 },
                    Vec2 { x: 20.0, y: 10.0 },
                    Vec2 { x: 0.0, y: 10.0 },
                ],
                hole: false,
            }],
        }],
    }
}

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene snapshot, got {other:?}"),
    }
}

#[test]
fn case_preparation_uses_request_revision_and_preserves_document_and_history() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("project", "Project");
    doc.outline.push(outline("initial"));
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: doc
        }),
        CoreReply::Scene { .. }
    ));
    assert!(matches!(
        engine.handle(commit_case(0, "first")),
        CoreReply::Scene { .. }
    ));
    assert!(matches!(
        engine.handle(commit_case(1, "second")),
        CoreReply::Scene { .. }
    ));
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Scene { .. }
    ));

    let before = snapshot(&mut engine);
    assert_eq!(before.revision, 3);
    let request_revision = 77;
    let reply = engine.handle(CoreRequest::PrepareCase {
        id: "prepare".into(),
        ir: case_ir(request_revision, request_revision),
    });
    match reply {
        CoreReply::CasePrepared { ir, .. } => {
            assert_eq!(ir.revision, request_revision);
            assert_eq!(ir.bodies[0].revision, request_revision);
        }
        other => panic!("expected prepared case, got {other:?}"),
    }
    assert_eq!(snapshot(&mut engine), before);

    let stale_assembly_revision = 88;
    let reply = engine.handle(CoreRequest::PrepareCase {
        id: "prepare-stale".into(),
        ir: case_ir(stale_assembly_revision, request_revision),
    });
    match reply {
        CoreReply::Error {
            revision, message, ..
        } => {
            assert_eq!(revision, stale_assembly_revision);
            assert!(message.contains("stale revision"));
        }
        other => panic!("expected stale revision error, got {other:?}"),
    }
    assert_eq!(snapshot(&mut engine), before);

    match engine.handle(CoreRequest::Undo {
        id: "undo-after-prepare".into(),
    }) {
        CoreReply::Scene { scene, .. } => assert_eq!(scene.revision, before.revision + 1),
        other => panic!("preparation changed undo history: {other:?}"),
    }
    match engine.handle(CoreRequest::Redo {
        id: "redo-after-prepare".into(),
    }) {
        CoreReply::Scene { scene, .. } => assert_eq!(scene.revision, before.revision + 2),
        other => panic!("preparation changed redo history: {other:?}"),
    }
}

#[test]
fn prepared_case_schema_roundtrips_through_the_existing_model_path() {
    let prepared = PreparedCaseIR {
        revision: 8,
        body: CaseBody {
            features: Some(vec![CaseFeature::ConicalSeat {
                id: "seat".into(),
                at: Vec2 { x: 3.0, y: 4.0 },
                z: 2.0,
                height: 5.0,
                diameter: 7.0,
                end_diameter: 4.0,
            }]),
            openings: Some(vec![CaseOpening {
                points: vec![Vec2 { x: 1.0, y: 2.0 }, Vec2 { x: 3.0, y: 4.0 }],
                z: 1.0,
                height: 2.0,
            }]),
            id: "tray".into(),
            name: "Tray".into(),
            board_id: "board".into(),
            kind: CaseKind::Tray,
            thickness: 2.0,
            clearance: 0.25,
            material_id: Some("aluminum".into()),
            z: Some(-1.0),
            wall_height: Some(12.0),
            wall_thickness: Some(1.5),
            mounts: Some(vec![Mount {
                id: "mount".into(),
                at: Vec2 { x: 5.0, y: 6.0 },
                kind: MountKind::Boss,
                hole_diameter: 2.0,
                boss_diameter: Some(5.0),
                height: Some(3.0),
            }]),
            pcb_source: None,
            gasket: Some(Gasket {
                inset: 1.0,
                width: 2.0,
                depth: 0.5,
            }),
        },
        regions: vec![PreparedCaseRegion {
            outer: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 10.0, y: 0.0 }],
            holes: vec![],
            cavities: vec![],
            gaskets: vec![PreparedGasketRegion {
                outer: vec![Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: 9.0, y: 1.0 }],
                holes: vec![],
            }],
            mounts: vec![],
        }],
    };

    let value = serde_json::to_value(&prepared).unwrap();
    assert!(value["body"].get("pcb_source").is_none());
    assert_eq!(value["body"]["boardId"], "board");
    assert_eq!(value["body"]["kind"], "tray");
    assert_eq!(value["body"]["features"][0]["kind"], "conical-seat");
    assert_eq!(value["body"]["features"][0]["endDiameter"], 4.0);
    assert_eq!(value["body"]["mounts"][0]["kind"], "boss");
    let decoded: PreparedCaseIR = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, prepared);
}

#[test]
fn adoption_inventory_preserves_authorable_case_features_through_public_preparation() {
    for source in [
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/cone-clipped-height.json"),
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/cone-wall.json"),
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/sealed-round-seat.json"),
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/sealed-opening.json"),
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/opening-ring-island.json"),
        include_str!(
            "../../cad/monstertruck/tests/fixtures/adoption/overlapping-openings-lineage.json"
        ),
    ] {
        let expected: PreparedCaseIR = serde_json::from_str(source).unwrap();
        let reply = CoreEngine::new().handle(CoreRequest::PrepareCase {
            id: "adoption".into(),
            ir: CaseAssemblyIR {
                revision: expected.revision,
                bodies: vec![CaseIR {
                    revision: expected.revision,
                    body: expected.body.clone(),
                    contours: vec![Contour {
                        points: expected.regions[0].outer.clone(),
                        hole: false,
                    }],
                }],
            },
        });
        match reply {
            CoreReply::CasePrepared { ir, .. } => assert_eq!(ir.bodies, vec![expected]),
            other => panic!("valid authorable adoption fixture refused: {other:?}"),
        }
    }
}

#[test]
fn adoption_inventory_keeps_all_source_drills_slots_and_outline_segments() {
    for (source, literal) in [
        (
            include_str!("../../cad/monstertruck/tests/fixtures/adoption/pcb-edge-drill.kicad_pcb"),
            include_str!("../../cad/monstertruck/tests/fixtures/adoption/pcb-edge-drill.json"),
        ),
        (
            include_str!(
                "../../cad/monstertruck/tests/fixtures/adoption/pcb-overlapping-drills.kicad_pcb"
            ),
            include_str!(
                "../../cad/monstertruck/tests/fixtures/adoption/pcb-overlapping-drills.json"
            ),
        ),
        (
            include_str!(
                "../../cad/monstertruck/tests/fixtures/adoption/pcb-drill-in-slot.kicad_pcb"
            ),
            include_str!("../../cad/monstertruck/tests/fixtures/adoption/pcb-drill-in-slot.json"),
        ),
    ] {
        let reply: serde_json::Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &serde_json::json!({"id":"adoption-source","kind":"read-board-source",
                "source":source,"placement":{"at":{"x":0.,"y":0.},"rotation":0.},
                "flipped":false})
            .to_string(),
        ))
        .unwrap();
        assert_eq!(reply["kind"], "read-board-source", "{reply}");
        let board = &reply["result"];
        let literal: PreparedCaseIR = serde_json::from_str(literal).unwrap();
        let actual_source: CasePcbSource =
            serde_json::from_value(board["pcbSource"].clone()).unwrap();
        let actual_drills: Vec<Mount> =
            serde_json::from_value(board["roundDrills"].clone()).unwrap();
        assert_eq!(Some(actual_source), literal.body.pcb_source);
        assert_eq!(Some(actual_drills), literal.body.mounts);
        let drills = board["roundDrills"].as_array().unwrap();
        let slots = board["pcbSource"]["slots"].as_array().unwrap();
        assert_eq!(drills.len(), if source.contains("pad-1") { 2 } else { 1 });
        assert_eq!(slots.len(), usize::from(source.contains("tstamp slot")));
        assert!(
            drills
                .iter()
                .all(|drill| drill["id"].as_str().unwrap().starts_with("footprint/pad-"))
        );
        let primitives = board["pcbSource"]["contours"][0]["primitives"]
            .as_array()
            .unwrap();
        assert_eq!(primitives.len(), 4);
        for (index, primitive) in primitives.iter().enumerate() {
            assert_eq!(primitive["sourceId"], "edge");
            assert_eq!(primitive["segmentIndex"], index);
        }
        let prepared: serde_json::Value = serde_json::from_str(
            &CoreEngine::new().request(
                &serde_json::json!({"id":"adoption-pcb","kind":"prepare-case","ir":{
                "revision":1,"bodies":[{"revision":1,"contours":board["contours"],
                    "body":{"id":"pcb","name":"PCB","boardId":"board","kind":"plate",
                        "thickness":1.6,"clearance":0.,"pcb_source":board["pcbSource"],
                        "mounts":board["roundDrills"]}}]}})
                .to_string(),
            ),
        )
        .unwrap();
        assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
        assert_eq!(
            prepared["ir"]["bodies"][0]["body"]["pcb_source"],
            board["pcbSource"]
        );
        assert_eq!(
            prepared["ir"]["bodies"][0]["body"]["mounts"],
            board["roundDrills"]
        );
    }
}
