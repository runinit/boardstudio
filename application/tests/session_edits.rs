use boardstudio_application::{
    Completion, Durability, EditResolver, Effect, Event, GenerationStatus, OperationId, Resolution,
    SaveResult, Scope, Session, TerminalOutcome,
};
use boardstudio_core::{CoreEngine, model::ProjectDoc};
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

#[test]
fn rename_refusal_save_retry_and_history() {
    let mut session = Session::new(CoreEngine::validate_open);
    let doc = ProjectDoc::empty("project", "Original");
    let committed = SaveResult::Committed;
    let run = |session: &mut Session, event, save| drive(session, event, save);

    assert_eq!(
        run(
            &mut session,
            Event::Open {
                operation_id: OperationId(1),
                document: doc,
            },
            committed.clone(),
        ),
        [TerminalOutcome::Completed]
    );

    let refusal = run(
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
        committed.clone(),
    );
    assert!(matches!(refusal.as_slice(), [TerminalOutcome::Rejected(_)]));
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        0
    );

    let failure = run(
        &mut session,
        Event::ResolveEdit {
            operation_id: OperationId(3),
            label: "rename".into(),
            resolver: EditResolver::rename_project("rename", "Edited".into()),
        },
        SaveResult::Aborted("storage full".into()),
    );
    assert_eq!(failure, [TerminalOutcome::Completed]);
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .name,
        "Edited"
    );
    assert!(matches!(
        session.read_model().durability,
        Durability::Failed { revision: 1, .. }
    ));

    run(
        &mut session,
        Event::RetrySave {
            operation_id: OperationId(4),
        },
        committed.clone(),
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
            run(&mut session, event, committed.clone()),
            [TerminalOutcome::Completed]
        );
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .name,
            expected
        );
    }
}

#[test]
fn commits_publish_while_saving_and_only_save_the_latest_waiting_revision() {
    let mut session = Session::new(CoreEngine::validate_open);
    drive(
        &mut session,
        Event::Open {
            operation_id: OperationId(1),
            document: ProjectDoc::empty("project", "Original"),
        },
        SaveResult::Committed,
    );

    let rename = |session: &mut Session, id, name: &str| {
        session.submit(Event::ResolveEdit {
            operation_id: OperationId(id),
            label: "rename".into(),
            resolver: EditResolver::rename_project("rename", name.into()),
        })
    };

    let first = rename(&mut session, 2, "First");
    let first_save = first
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
                ..
            } => {
                assert_eq!(document.name, "First");
                Some(*save_attempt_id)
            }
            _ => None,
        })
        .expect("first write starts immediately");

    for (id, name) in [(3, "Second"), (4, "Latest")] {
        let effects = rename(&mut session, id, name);
        assert!(
            effects
                .iter()
                .all(|effect| !matches!(effect, Effect::Persist { .. }))
        );
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .name,
            name
        );
    }

    let latest = session.complete(Completion::Persist {
        save_attempt_id: first_save,
        result: SaveResult::Committed,
    });
    let [
        Effect::Persist {
            save_attempt_id,
            document,
            ..
        },
    ] = latest.as_slice()
    else {
        panic!("the latest revision should be the only follow-up write: {latest:?}");
    };
    assert_eq!(document.name, "Latest");
    assert_eq!(document.revision, 3);
    assert!(
        session
            .complete(Completion::Persist {
                save_attempt_id: *save_attempt_id,
                result: SaveResult::Committed,
            })
            .is_empty()
    );
    assert_eq!(
        session.read_model().durability,
        Durability::Saved { revision: 3 }
    );
}

#[test]
fn open_after_save_failure_stale_revision_refusal_and_generation_cancel() {
    let mut session = Session::new(CoreEngine::validate_open);
    assert_eq!(
        drive(
            &mut session,
            Event::Open {
                operation_id: OperationId(1),
                document: ProjectDoc::empty("project", "First"),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Completed]
    );
    assert_eq!(
        drive(
            &mut session,
            Event::ResolveEdit {
                operation_id: OperationId(2),
                label: "rename".into(),
                resolver: EditResolver::rename_project("rename", "Unsaved".into()),
            },
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

    assert_eq!(
        drive(
            &mut session,
            Event::Open {
                operation_id: OperationId(3),
                document: ProjectDoc::empty("other", "Replacement"),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Completed]
    );
    assert_eq!(
        session.read_model().accepted.as_ref().unwrap().document.id,
        "other"
    );
    assert_eq!(
        drive(
            &mut session,
            Event::ResolveEdit {
                operation_id: OperationId(4),
                label: "rename".into(),
                resolver: EditResolver::rename_project("rename", "Rev1".into()),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Completed]
    );

    let board_id = session.read_model().active_board_id.clone();
    assert_eq!(
        drive(
            &mut session,
            Event::ReviewElectricalRemap {
                operation_id: OperationId(5),
                base_revision: 0,
                board_id,
                expected_fingerprint: "fp".into(),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Rejected(
            "captured edit revision is stale".into()
        )]
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

    let model = session.read_model();
    let accepted = model.accepted.as_ref().unwrap();
    let scope = Scope {
        session_epoch: accepted.session_epoch,
        document_id: accepted.document.id.clone(),
        board_id: model.active_board_id.clone(),
        instance_id: None,
    };
    let gen_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(7),
        scope,
    });
    let job_id = gen_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .expect("generation starts");
    let cancel_effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(8),
        label: "rename".into(),
        resolver: EditResolver::rename_project("rename", "CancelledGen".into()),
    });
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
    let mut session = Session::new(CoreEngine::validate_open);
    assert_eq!(
        drive(
            &mut session,
            Event::Open {
                operation_id: OperationId(1),
                document: ProjectDoc::empty("project-1", "First"),
            },
            SaveResult::Committed,
        ),
        [TerminalOutcome::Completed]
    );

    let open_effects = session.submit(Event::Open {
        operation_id: OperationId(11),
        document: ProjectDoc::empty("project-2", "Replacement"),
    });
    let open_save = open_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id, ..
            } => Some(*save_attempt_id),
            _ => None,
        })
        .expect("replacement open starts persisting");

    assert_eq!(
        drive(
            &mut session,
            Event::ResolveEdit {
                operation_id: OperationId(2),
                label: "rename".into(),
                resolver: EditResolver::rename_project("rename", "Unsaved".into()),
            },
            SaveResult::Aborted("storage full".into()),
        ),
        [TerminalOutcome::Completed]
    );
    let retry_effects = session.submit(Event::RetrySave {
        operation_id: OperationId(10),
    });
    assert!(
        retry_effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. })),
        "retry save should emit Persist: {retry_effects:?}"
    );

    let commit_effects = session.complete(Completion::Persist {
        save_attempt_id: open_save,
        result: SaveResult::Committed,
    });
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
