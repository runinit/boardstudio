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

fn commit_outline(base_revision: u64, id: &str) -> CoreRequest {
    CoreRequest::Edit {
        id: format!("edit-{id}"),
        command: EditCommand {
            base_revision,
            transaction_id: format!("tx-{id}"),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetOutline {
                feature: outline(id),
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
        CoreReply::Scene { document, .. } => document,
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
        engine.handle(commit_outline(0, "first")),
        CoreReply::Scene { .. }
    ));
    assert!(matches!(
        engine.handle(commit_outline(1, "second")),
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
