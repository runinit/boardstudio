//! Ticket 019: the refresh safety gate needs one global signal for committed work
//! still moving through the engine (queued intents, an executing Core request, or an
//! unanswered persist). Durability alone cannot see a submitted edit that has not yet
//! produced its save, so Session exposes it directly.
use boardstudio_application::{
    Completion, EditResolver, Effect, Event, OperationId, Resolution, SaveAttemptId, SaveResult,
    Session,
};
use boardstudio_core::model::*;
use boardstudio_core::{CoreEngine, model::ProjectDoc};

fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Project");
    document.definitions.push(PartDefinition {
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![],
        pads: vec![],
        models: None,
        hardware_profile: None,
        input_profile: None,
        keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    document.parts.push(Part {
        id: "key".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document
}

fn move_edit(operation_id: u64, base_revision: u64, x: f64) -> Event {
    let command = EditCommand {
        base_revision,
        transaction_id: format!("move-{operation_id}"),
        phase: EditPhase::Commit,
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x, y: 0.0 },
            }],
        },
    };
    Event::ResolveEdit {
        operation_id: OperationId(operation_id),
        label: "pending-work probe".into(),
        resolver: EditResolver::new("pending-work probe", move |_| {
            Resolution::Submit(command.clone())
        }),
    }
}

fn core_request_id(effects: &[Effect]) -> boardstudio_application::RequestId {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { request_id, .. } => Some(*request_id),
            _ => None,
        })
        .expect("one core request")
}

fn save_attempt(effects: &[Effect]) -> SaveAttemptId {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id, ..
            } => Some(*save_attempt_id),
            _ => None,
        })
        .expect("one persistence request")
}

fn open_ready(session: &mut Session, engine: &mut CoreEngine) {
    let effects = session.submit(Event::Open {
        operation_id: OperationId(10),
        document: fixture(),
    });
    let request_id = core_request_id(&effects);
    let reply = engine.handle(
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Core { request, .. } => Some((**request).clone()),
                _ => None,
            })
            .expect("open core request"),
    );
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: session.core_executor_epoch(),
        reply: Box::new(reply),
    });
    let save_attempt_id = save_attempt(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
}

#[test]
fn pending_work_covers_queued_active_and_unanswered_save() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    assert!(
        !session.has_pending_work(),
        "a fresh session holds no committed work"
    );

    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    assert!(
        session.has_pending_work(),
        "an open with an outstanding Core request is pending work"
    );
    let request_id = core_request_id(&effects);
    let reply = engine.handle(
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Core { request, .. } => Some((**request).clone()),
                _ => None,
            })
            .expect("open core request"),
    );
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: session.core_executor_epoch(),
        reply: Box::new(reply),
    });
    assert!(
        !session.has_pending_work(),
        "the save leg is owned by Durability, not engine pending work"
    );
    assert!(
        matches!(
            session.read_model().durability,
            boardstudio_application::Durability::Saving { .. }
        ),
        "the unanswered persist shows as Saving for the gate's other input"
    );
    let save_attempt_id = save_attempt(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert!(
        !session.has_pending_work(),
        "a settled open leaves no pending work"
    );

    // A submitted edit is pending through execution and its save.
    let effects = session.submit(move_edit(2, 0, 5.0));
    assert!(
        session.has_pending_work(),
        "a submitted edit is pending before its Core reply"
    );
    // A second edit queues behind the running one and stays pending too.
    session.submit(move_edit(3, 0, 7.0));
    assert!(
        session.has_pending_work(),
        "a queued edit behind a running one is pending work"
    );
    let request_id = core_request_id(&effects);
    let reply = engine.handle(
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Core { request, .. } => Some((**request).clone()),
                _ => None,
            })
            .expect("edit core request"),
    );
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: session.core_executor_epoch(),
        reply: Box::new(reply),
    });
    assert!(
        session.has_pending_work(),
        "the queued edit still waits while the first edit's save is unanswered"
    );
    assert!(
        matches!(
            session.read_model().durability,
            boardstudio_application::Durability::Saving { .. }
        ),
        "the save leg shows as Saving for the gate's other input"
    );
    let save_attempt_id = save_attempt(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert!(
        session.has_pending_work()
            || matches!(
                session.read_model().durability,
                boardstudio_application::Durability::Saving { .. }
            ),
        "the queued edit keeps moving after the first save lands"
    );
    // Draining the rest must return to idle: no stuck pending state.
    let mut effects = effects;
    for _ in 0..8 {
        let idle = !session.has_pending_work()
            && !matches!(
                session.read_model().durability,
                boardstudio_application::Durability::Saving { .. }
            )
            && effects.is_empty();
        if idle {
            break;
        }
        if let Some(request_id) = effects.iter().find_map(|effect| match effect {
            Effect::Core { request_id, .. } => Some(*request_id),
            _ => None,
        }) {
            let request = effects
                .iter()
                .find_map(|effect| match effect {
                    Effect::Core { request, .. } => Some((**request).clone()),
                    _ => None,
                })
                .expect("queued edit core request");
            let reply = engine.handle(request);
            effects = session.complete(Completion::Core {
                request_id,
                executor_epoch: session.core_executor_epoch(),
                reply: Box::new(reply),
            });
        } else if let Some(save_attempt_id) = effects.iter().find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id, ..
            } => Some(*save_attempt_id),
            _ => None,
        }) {
            effects = session.complete(Completion::Persist {
                save_attempt_id,
                result: SaveResult::Committed,
            });
        } else {
            break;
        }
    }
    assert!(
        !session.has_pending_work(),
        "draining every queued edit and save returns to idle"
    );

    // The helper path agrees: open_ready from a fresh session is idle.
    let mut second = Session::new();
    open_ready(&mut second, &mut engine);
    assert!(!second.has_pending_work());
}
