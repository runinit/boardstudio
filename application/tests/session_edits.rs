use boardstudio_application::{
    Completion, Durability, EditResolver, Effect, Event, GenerationStatus, OperationId, Resolution,
    SaveAttemptId, SaveResult, Session, TerminalOutcome,
};
use boardstudio_core::model::ProjectDoc;
use boardstudio_model::model::EditOperation;
use std::collections::VecDeque;

fn drive(session: &mut Session, event: Event, save: SaveResult) -> Vec<TerminalOutcome> {
    let mut effects = VecDeque::from(session.submit(event));
    let mut outcomes = Vec::new();
    while let Some(effect) = effects.pop_front() {
        match effect {
            Effect::Persist {
                save_attempt_id, ..
            } => effects.extend(session.complete(Completion::Persist {
                save_attempt_id,
                result: save.clone(),
            })),
            Effect::Settled { outcome, .. } => outcomes.push(outcome),
            _ => {}
        }
    }
    outcomes
}

fn open(session: &mut Session, id: u64, document: ProjectDoc) {
    assert_eq!(
        drive(
            session,
            Event::Open {
                operation_id: OperationId(id),
                document
            },
            SaveResult::Committed
        ),
        [TerminalOutcome::Completed]
    );
}

fn document(session: &Session) -> &ProjectDoc {
    &session.read_model().accepted.as_ref().unwrap().document
}

fn rename(id: u64, name: &str) -> Event {
    Event::ResolveEdit {
        operation_id: OperationId(id),
        label: "rename".into(),
        resolver: EditResolver::rename_project("rename", name.into()),
    }
}

fn scheduled_save(effects: &[Effect]) -> (SaveAttemptId, &ProjectDoc) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
            } => Some((*save_attempt_id, document.as_ref())),
            _ => None,
        })
        .expect("a write should be scheduled")
}

fn committed(save_attempt_id: SaveAttemptId) -> Completion {
    Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    }
}

#[test]
fn rename_refusal_save_retry_and_history() {
    let mut session = Session::new();
    open(&mut session, 1, ProjectDoc::empty("project", "Original"));

    let refusal = drive(
        &mut session,
        Event::ResolveEdit {
            operation_id: OperationId(2),
            label: "invalid rename".into(),
            resolver: EditResolver::new("invalid rename", |_| {
                Resolution::submit(
                    vec!["absent-project".into()],
                    EditOperation::RenameProject {
                        name: String::new(),
                    },
                )
            }),
        },
        SaveResult::Committed,
    );
    assert!(matches!(refusal.as_slice(), [TerminalOutcome::Rejected(_)]));
    assert_eq!(document(&session).revision, 0);

    let failure = drive(
        &mut session,
        rename(3, "Edited"),
        SaveResult::Aborted("storage full".into()),
    );
    assert_eq!(failure, [TerminalOutcome::Completed]);
    assert_eq!(document(&session).name, "Edited");
    assert!(matches!(
        session.read_model().durability,
        Durability::Failed { revision: 1, .. }
    ));

    drive(
        &mut session,
        Event::RetrySave {
            operation_id: OperationId(4),
        },
        SaveResult::Committed,
    );
    assert!(matches!(
        session.read_model().durability,
        Durability::Saved { revision: 1 }
    ));
    for (event, expected) in [
        (
            Event::Undo {
                operation_id: OperationId(5),
            },
            "Original",
        ),
        (
            Event::Redo {
                operation_id: OperationId(6),
            },
            "Edited",
        ),
    ] {
        assert_eq!(
            drive(&mut session, event, SaveResult::Committed),
            [TerminalOutcome::Completed]
        );
        assert_eq!(document(&session).name, expected);
    }
}

#[test]
fn commits_publish_while_saving_and_only_save_the_latest_waiting_revision() {
    let mut session = Session::new();
    open(&mut session, 1, ProjectDoc::empty("project", "Original"));

    let first = session.submit(rename(2, "First"));
    let (first_save, written) = scheduled_save(&first);
    assert_eq!(written.name, "First");

    for (id, name) in [(3, "Second"), (4, "Latest")] {
        let effects = session.submit(rename(id, name));
        assert!(
            effects
                .iter()
                .all(|effect| !matches!(effect, Effect::Persist { .. }))
        );
        assert_eq!(document(&session).name, name);
    }

    let latest = session.complete(committed(first_save));
    assert_eq!(
        latest.len(),
        1,
        "only the newest revision should be written"
    );
    let (latest_save, written) = scheduled_save(&latest);
    assert_eq!(written.name, "Latest");
    assert_eq!(written.revision, 3);
    assert!(session.complete(committed(latest_save)).is_empty());
    assert_eq!(
        session.read_model().durability,
        Durability::Saved { revision: 3 }
    );
}

#[test]
fn open_after_save_failure_stale_resolver_refusal_and_generation_cancel() {
    let mut session = Session::new();
    open(&mut session, 1, ProjectDoc::empty("project", "First"));
    assert_eq!(
        drive(
            &mut session,
            rename(2, "Unsaved"),
            SaveResult::Aborted("conflict".into()),
        ),
        [TerminalOutcome::Completed]
    );
    assert_eq!(
        session.read_model().durability,
        Durability::Failed {
            revision: 1,
            reason: "conflict".into(),
        }
    );

    open(&mut session, 3, ProjectDoc::empty("other", "Replacement"));
    assert_eq!(document(&session).id, "other");
    assert_eq!(
        drive(&mut session, rename(4, "Rev1"), SaveResult::Committed),
        [TerminalOutcome::Completed]
    );

    assert_eq!(
        drive(
            &mut session,
            Event::ResolveEdit {
                operation_id: OperationId(6),
                label: "stale resolver".into(),
                resolver: EditResolver::new("stale resolver", |_| {
                    Resolution::Retire("stale revision".into())
                }),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Rejected("stale revision".into())]
    );

    let gen_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(7),
        scope: session.scope().unwrap(),
    });
    let job_id = gen_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .expect("generation starts");
    let cancel_effects = session.submit(rename(8, "CancelledGen"));
    assert!(cancel_effects.contains(&Effect::CancelJob { job_id }));
    assert!(cancel_effects.contains(&Effect::Settled {
        operation_id: OperationId(7),
        outcome: TerminalOutcome::Cancelled,
        landing: None,
    }));
    assert_eq!(
        session.read_model().generation,
        GenerationStatus::Cancelled { job_id }
    );
}

#[test]
fn open_commit_supersedes_pending_retry_save_operation() {
    let mut session = Session::new();
    open(&mut session, 1, ProjectDoc::empty("project-1", "First"));

    let open_effects = session.submit(Event::Open {
        operation_id: OperationId(11),
        document: ProjectDoc::empty("project-2", "Replacement"),
    });
    let (open_save, _) = scheduled_save(&open_effects);

    assert_eq!(
        drive(
            &mut session,
            rename(2, "Unsaved"),
            SaveResult::Aborted("storage full".into()),
        ),
        [TerminalOutcome::Completed]
    );
    let retry_effects = session.submit(Event::RetrySave {
        operation_id: OperationId(10),
    });
    scheduled_save(&retry_effects);
    let commit_effects = session.complete(committed(open_save));
    assert!(commit_effects.contains(&Effect::Settled {
        operation_id: OperationId(10),
        outcome: TerminalOutcome::Superseded,
        landing: None,
    }));
    assert!(commit_effects.contains(&Effect::Settled {
        operation_id: OperationId(11),
        outcome: TerminalOutcome::Completed,
        landing: None,
    }));
}
