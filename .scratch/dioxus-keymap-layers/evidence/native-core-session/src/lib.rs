#[cfg(test)]
mod tests {
use boardstudio_application::{
    Completion, Effect, Event, OperationId, SaveResult, Session, TerminalOutcome,
};
use boardstudio_core::{CoreEngine, model::*};
use std::collections::VecDeque;

fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("keymap-layer-session", "Keymap layer session");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn settle_with_real_core(
    session: &mut Session,
    engine: &mut CoreEngine,
    initial: Vec<Effect>,
) -> Vec<Effect> {
    let mut queue = VecDeque::from(initial);
    let mut observed = Vec::new();
    while let Some(effect) = queue.pop_front() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => {
                let reply = engine.handle(*request);
                queue.extend(session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(reply),
                }));
            }
            Effect::Persist { save_attempt_id, .. } => {
                queue.extend(session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                }));
            }
            other => observed.push(other),
        }
    }
    observed
}

fn settle(session: &mut Session, engine: &mut CoreEngine, event: Event) -> Vec<Effect> {
    let initial = session.submit(event);
    settle_with_real_core(session, engine, initial)
}

fn open_ready(session: &mut Session, engine: &mut CoreEngine) -> Vec<Effect> {
    settle(
        session,
        engine,
        Event::Open {
            operation_id: OperationId(1),
            document: fixture(),
        },
    )
}

fn edit_layer(operation_id: u64, revision: u64, change: KeymapChange) -> Event {
    Event::Edit {
        operation_id: OperationId(operation_id),
        command: EditCommand {
            base_revision: revision,
            transaction_id: format!("keymap-layer-{operation_id}"),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::EditKeymap { change },
        },
    }
}

fn outcome(effects: &[Effect], operation_id: u64) -> TerminalOutcome {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Settled {
                operation_id: actual,
                outcome,
            } if *actual == OperationId(operation_id) => Some(outcome.clone()),
            _ => None,
        })
        .expect("the real Session emits a terminal outcome")
}

fn accepted(session: &Session) -> &ProjectDoc {
    session
        .read_model()
        .accepted
        .as_ref()
        .expect("real Core open/edit acceptance")
        .document
        .as_ref()
}

fn revision(session: &Session) -> u64 {
    accepted(session).revision
}

#[test]
fn adding_first_saved_layer_materializes_virtual_base_and_completes_durably() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let opened = open_ready(&mut session, &mut engine);
    assert_eq!(outcome(&opened, 1), TerminalOutcome::Completed);
    assert!(accepted(&session).keymap.is_none());

    let effects = settle(
        &mut session,
        &mut engine,
        edit_layer(
            2,
            0,
            KeymapChange::AddLayer {
                id: "layer-function".into(),
                name: "Function".into(),
            },
        ),
    );

    assert_eq!(outcome(&effects, 2), TerminalOutcome::Completed);
    assert_eq!(revision(&session), 1);
    assert_eq!(
        session.read_model().durability,
        boardstudio_application::Durability::Saved { revision: 1 }
    );
    let layers = &accepted(&session).keymap.as_ref().unwrap().layers;
    assert_eq!(
        layers.iter().map(|layer| (layer.id.as_str(), layer.name.as_str())).collect::<Vec<_>>(),
        [("base", "Base"), ("layer-function", "Function")]
    );
}

#[test]
fn rename_preserves_layer_identity_and_undo_restores_the_accepted_name() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let added = settle(
        &mut session,
        &mut engine,
        edit_layer(
            2,
            0,
            KeymapChange::AddLayer {
                id: "stable-function".into(),
                name: "Function".into(),
            },
        ),
    );
    assert_eq!(outcome(&added, 2), TerminalOutcome::Completed);

    let renamed = settle(
        &mut session,
        &mut engine,
        edit_layer(
            3,
            1,
            KeymapChange::RenameLayer {
                id: "stable-function".into(),
                name: "Navigation".into(),
            },
        ),
    );
    assert_eq!(outcome(&renamed, 3), TerminalOutcome::Completed);
    let layer = accepted(&session)
        .keymap
        .as_ref()
        .unwrap()
        .layers
        .iter()
        .find(|layer| layer.id == "stable-function")
        .unwrap();
    assert_eq!(layer.name, "Navigation");
    assert_eq!(revision(&session), 2);

    let undone = settle(
        &mut session,
        &mut engine,
        Event::Undo {
            operation_id: OperationId(4),
        },
    );
    assert_eq!(outcome(&undone, 4), TerminalOutcome::Completed);
    let layers = &accepted(&session).keymap.as_ref().unwrap().layers;
    assert_eq!(layers[1].id, "stable-function");
    assert_eq!(layers[1].name, "Function");
    assert_eq!(revision(&session), 3);
    assert_eq!(
        session.read_model().durability,
        boardstudio_application::Durability::Saved { revision: 3 }
    );
}

#[test]
fn removing_nonbase_layer_then_undo_restores_its_stable_id_and_name() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let added = settle(
        &mut session,
        &mut engine,
        edit_layer(
            2,
            0,
            KeymapChange::AddLayer {
                id: "stable-function".into(),
                name: "Function".into(),
            },
        ),
    );
    assert_eq!(outcome(&added, 2), TerminalOutcome::Completed);

    let removed = settle(
        &mut session,
        &mut engine,
        edit_layer(
            3,
            1,
            KeymapChange::RemoveLayer {
                id: "stable-function".into(),
            },
        ),
    );
    assert_eq!(outcome(&removed, 3), TerminalOutcome::Completed);
    assert_eq!(
        accepted(&session)
            .keymap
            .as_ref()
            .unwrap()
            .layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<Vec<_>>(),
        ["base"]
    );

    let undone = settle(
        &mut session,
        &mut engine,
        Event::Undo {
            operation_id: OperationId(4),
        },
    );
    assert_eq!(outcome(&undone, 4), TerminalOutcome::Completed);
    let layers = &accepted(&session).keymap.as_ref().unwrap().layers;
    assert_eq!(layers[1].id, "stable-function");
    assert_eq!(layers[1].name, "Function");
}

#[test]
fn removing_base_is_rejected_without_changing_the_accepted_snapshot() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let before = accepted(&session).clone();

    let effects = settle(
        &mut session,
        &mut engine,
        edit_layer(
            2,
            0,
            KeymapChange::RemoveLayer { id: "base".into() },
        ),
    );

    assert_eq!(
        outcome(&effects, 2),
        TerminalOutcome::Rejected("The base layer cannot be removed".into())
    );
    assert_eq!(accepted(&session), &before);
    assert_eq!(revision(&session), 0);
}

}
