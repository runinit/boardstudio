//! Diagnostic companion to layout-part-editing.md; run using that document’s temporary crate.

use boardstudio_application::{Completion, Effect, Event, OperationId, SaveResult, Session};
use boardstudio_core::{CoreEngine, model::*};

fn core(
    effects: &[Effect],
) -> (
    boardstudio_application::RequestId,
    boardstudio_application::ExecutorEpoch,
    CoreRequest,
) {
    effects
        .iter()
        .find_map(|e| {
            if let Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } = e
            {
                Some((*request_id, *executor_epoch, (**request).clone()))
            } else {
                None
            }
        })
        .expect("core effect")
}
fn save(effects: &[Effect]) -> boardstudio_application::SaveAttemptId {
    effects
        .iter()
        .find_map(|e| {
            if let Effect::Persist {
                save_attempt_id, ..
            } = e
            {
                Some(*save_attempt_id)
            } else {
                None
            }
        })
        .expect("save effect")
}
fn settle(session: &mut Session, engine: &mut CoreEngine, effects: Vec<Effect>) -> Vec<Effect> {
    let (request_id, executor_epoch, request) = core(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let save_attempt_id = save(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    })
}
fn edit(op: u64, x: f64, y: f64) -> Event {
    // This reproduces the pre-ADR-0005 event; Event::Edit has since been removed.
    Event::Edit {
        operation_id: OperationId(op),
        command: EditCommand {
            base_revision: 0,
            transaction_id: format!("inspector-{op}"),
            phase: EditPhase::Commit,
            target_ids: vec!["part".into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: "part".into(),
                    at: Vec2 { x, y },
                }],
            },
        },
    }
}
fn main() {
    let mut doc = ProjectDoc::empty("doc", "doc");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: "def".into(),
        name: "Def".into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    });
    doc.parts.push(Part {
        keycap: None,
        outline: None,
        id: "part".into(),
        definition_id: "def".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 {
                x: 66.675,
                y: -47.625,
            },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: doc,
    });
    settle(&mut session, &mut engine, effects);
    // Synthetic equivalent of dispatcher payloads: both computed against same original accepted snapshot.
    let first = session.submit(edit(2, 60.0, -47.625));
    let queued_effects = session.submit(edit(3, 66.675, -40.0));
    assert!(
        queued_effects.is_empty(),
        "second edit should wait for the first"
    );
    let after_first = settle(&mut session, &mut engine, first);
    settle(&mut session, &mut engine, after_first);
    let accepted = &session.read_model().accepted.as_ref().unwrap().document;
    let part = accepted.parts.iter().find(|p| p.id == "part").unwrap();
    println!(
        "after queued inspector-shaped edits revision={} position=({}, {})",
        accepted.revision, part.pose.at.x, part.pose.at.y
    );
    // Undo the second operation, then the first, to expose per-command history.
    let effects = session.submit(Event::Undo {
        operation_id: OperationId(4),
    });
    settle(&mut session, &mut engine, effects);
    let d = &session.read_model().accepted.as_ref().unwrap().document;
    let p = d.parts.iter().find(|p| p.id == "part").unwrap();
    println!(
        "after first undo revision={} position=({}, {})",
        d.revision, p.pose.at.x, p.pose.at.y
    );
    let effects = session.submit(Event::Undo {
        operation_id: OperationId(5),
    });
    settle(&mut session, &mut engine, effects);
    let d = &session.read_model().accepted.as_ref().unwrap().document;
    let p = d.parts.iter().find(|p| p.id == "part").unwrap();
    println!(
        "after second undo revision={} position=({}, {})",
        d.revision, p.pose.at.x, p.pose.at.y
    );
}
