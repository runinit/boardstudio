use boardstudio_application::{
    Completion, Effect, Event, ExecutorEpoch, OperationId, RequestId, SaveAttemptId, SaveResult,
    Session, TerminalOutcome,
};
use boardstudio_core::{CoreEngine, model::*};

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
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document
}

fn core_effect(effects: &[Effect]) -> (RequestId, ExecutorEpoch, CoreRequest) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => Some((*request_id, *executor_epoch, (**request).clone())),
            _ => None,
        })
        .expect("one core request")
}

fn save_effect(effects: &[Effect]) -> (SaveAttemptId, ProjectDoc) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
                ..
            } => Some((*save_attempt_id, (**document).clone())),
            _ => None,
        })
        .expect("one persistence request")
}

#[test]
fn retry_persists_retained_commit_without_replaying_engine_edit() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();

    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (open_request_id, open_epoch, open) = core_effect(&effects);
    let open_reply = engine.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: open_request_id,
        executor_epoch: open_epoch,
        reply: Box::new(open_reply),
    });
    let (open_save_id, opened) = save_effect(&effects);
    assert_eq!(opened.revision, 0);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: open_save_id,
        result: SaveResult::Committed,
    });
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
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(1),
            outcome: TerminalOutcome::Completed
        }
    )));

    let event = Event::Edit {
        operation_id: OperationId(2),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "move-1".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: "key".into(),
                    at: Vec2 { x: 3.0, y: 0.0 },
                }],
            },
        },
    };
    let effects = session.submit(event);
    let (edit_request_id, edit_epoch, edit) = core_effect(&effects);
    let committed_reply = engine.handle(edit);
    let effects = session.complete(Completion::Core {
        request_id: edit_request_id,
        executor_epoch: edit_epoch,
        reply: Box::new(committed_reply),
    });
    let (failed_save_id, pending) = save_effect(&effects);
    assert_eq!(pending.revision, 1);
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
    let effects = session.complete(Completion::Persist {
        save_attempt_id: failed_save_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(2),
            outcome: TerminalOutcome::PersistenceFailed(_)
        }
    )));
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

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(3),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_save_id, retried) = save_effect(&effects);
    assert_ne!(retry_save_id, failed_save_id);
    assert_eq!(retried, pending);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: retry_save_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        1
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(3),
            outcome: TerminalOutcome::Completed
        }
    )));

    let snapshot_request = CoreRequest::Snapshot {
        id: "inspect".into(),
    };
    assert!(
        matches!(engine.handle(snapshot_request), CoreReply::Scene { scene, .. } if scene.revision == 1)
    );
}

fn move_edit(operation_id: u64, base_revision: u64, x: f64) -> Event {
    Event::Edit {
        operation_id: OperationId(operation_id),
        command: EditCommand {
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
        },
    }
}

fn open_ready(session: &mut Session, engine: &mut CoreEngine) {
    let effects = session.submit(Event::Open {
        operation_id: OperationId(10),
        document: fixture(),
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
}

#[test]
fn queued_discrete_edits_use_each_preceding_durable_revision() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);

    let first_effects = session.submit(move_edit(11, 0, 2.0));
    let (first_id, first_epoch, first_request) = core_effect(&first_effects);
    let second_effects = session.submit(move_edit(12, 0, 6.0));
    assert!(
        !second_effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );

    let first_reply = engine.handle(first_request);
    let effects = session.complete(Completion::Core {
        request_id: first_id,
        executor_epoch: first_epoch,
        reply: Box::new(first_reply),
    });
    let (first_save, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: first_save,
        result: SaveResult::Committed,
    });
    let (second_id, second_epoch, second_request) = core_effect(&effects);
    assert!(
        matches!(&second_request, CoreRequest::Edit { command, .. } if command.base_revision == 1)
    );

    let second_reply = engine.handle(second_request);
    let effects = session.complete(Completion::Core {
        request_id: second_id,
        executor_epoch: second_epoch,
        reply: Box::new(second_reply),
    });
    let (second_save, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id: second_save,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .revision,
        2
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(12),
            outcome: TerminalOutcome::Completed
        }
    )));
}

#[test]
fn part_drag_uses_gap_geometry_snap_at_the_two_millimeter_tolerance() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut moving = document.parts[0].clone();
    moving.id = "moving".into();
    moving.reference = "SW2".into();
    moving.pose.at = Vec2 { x: 19.6, y: 0.0 };
    document.parts.push(moving);
    let effects = session.submit(Event::Open {
        operation_id: OperationId(20),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    session.submit(Event::GestureBegin {
        operation_id: OperationId(21),
        pointer_id: 7,
        target_ids: vec!["moving".into()],
        transaction_id: "drag-gap".into(),
        start: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: true,
        gap: Some(1.0),
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        alt: false,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    assert!(
        matches!(core_effect(&effects).2, CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, phase: EditPhase::Preview, .. }, .. } if positions[0].at.x == 19.0)
    );
}

#[test]
fn geometry_snap_ignores_parts_owned_by_another_board() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut moving = document.parts[0].clone();
    moving.id = "moving".into();
    moving.reference = "SW2".into();
    moving.pose.at = Vec2 { x: 19.6, y: 0.0 };
    document.parts.push(moving);
    document.boards = vec![
        Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec!["moving".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
        Board {
            id: "right".into(),
            name: "Right".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        },
    ];
    let effects = session.submit(Event::Open {
        operation_id: OperationId(22),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });

    session.submit(Event::Navigate {
        operation_id: OperationId(24),
        board_id: "left".into(),
        instance_id: None,
    });

    session.submit(Event::GestureBegin {
        operation_id: OperationId(23),
        pointer_id: 8,
        target_ids: vec!["moving".into()],
        transaction_id: "cross-board-snap".into(),
        start: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: true,
        gap: Some(1.0),
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 8,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.4, y: 0.0 },
        }],
        alt: false,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    assert!(matches!(
        core_effect(&effects).2,
        CoreRequest::Edit {
            command: EditCommand {
                operation: EditOperation::MoveParts { positions },
                phase: EditPhase::Preview,
                ..
            },
            ..
        } if positions[0].at.x == 19.4
    ));
}

#[test]
fn drag_coalesces_previews_and_commits_the_pointerup_sample_once() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(30),
        pointer_id: 77,
        target_ids: vec!["key".into()],
        transaction_id: "drag-once".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
        }],
        alt: true,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    let (preview_id, preview_epoch, preview_request) = core_effect(&effects);
    assert!(matches!(
        preview_request,
        CoreRequest::Edit {
            command: EditCommand {
                phase: EditPhase::Preview,
                ..
            },
            ..
        }
    ));

    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 3.0, y: 0.0 },
        }],
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 5.0, y: 0.0 },
        }],
        alt: true,
    });
    session.submit(Event::GestureEnd {
        pointer_id: 77,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        alt: true,
    });
    let preview_reply = engine.handle(preview_request);
    let effects = session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(preview_reply),
    });
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    assert!(
        matches!(&commit_request, CoreRequest::Edit { command: EditCommand { phase: EditPhase::Commit, operation: EditOperation::MoveParts { positions }, .. }, .. } if positions[0].at.x == 9.0)
    );
    let commit_reply = engine.handle(commit_request);
    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(commit_reply),
    });
    let (save_attempt_id, saved) = save_effect(&effects);
    assert_eq!(saved.revision, 1);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .parts[0]
            .pose
            .at
            .x,
        9.0
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(30),
            outcome: TerminalOutcome::Completed
        }
    )));
    assert!(
        matches!(engine.handle(CoreRequest::Undo { id: "undo-check".into() }), CoreReply::Scene { scene, document, .. } if scene.revision == 2 && document.parts[0].pose.at.x == 0.0)
    );
}

#[test]
fn uncertain_worker_outcome_is_never_replayed_and_requires_explicit_reopen() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();

    let effects = session.submit(move_edit(40, 0, 4.0));
    let (request_id, epoch, request) = core_effect(&effects);
    let _uncertain_reply = engine.handle(request);
    let effects = session.complete(Completion::CoreFailed {
        request_id,
        executor_epoch: epoch,
        reason: "worker terminated after mutation".into(),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(40),
            outcome: TerminalOutcome::ExecutorFailed(_)
        }
    )));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::RestartCoreExecutor { .. }))
    );
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
    let blocked = session.submit(move_edit(41, 0, 5.0));
    assert!(matches!(
        blocked.as_slice(),
        [Effect::Settled {
            outcome: TerminalOutcome::BlockedByRecovery(_),
            ..
        }]
    ));
    assert!(
        !blocked
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );

    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(42),
        document: accepted,
    });
    let (open_id, new_epoch, open) = core_effect(&effects);
    assert!(matches!(open, CoreRequest::Open { .. }));
    let mut restarted = CoreEngine::new();
    let reply = restarted.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: open_id,
        executor_epoch: new_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, recovered) = save_effect(&effects);
    assert_eq!(recovered.revision, 0);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
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
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(42),
            outcome: TerminalOutcome::Completed
        }
    )));
}

#[test]
fn recovery_reopen_discards_failed_save_and_ignores_stale_executor_restart() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });

    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: accepted,
    });
    assert!(
        matches!(effects.first(), Some(Effect::Core { request, .. }) if matches!(**request, CoreRequest::Open { .. }))
    );
    let active_epoch = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { executor_epoch, .. } => Some(*executor_epoch),
            _ => None,
        })
        .unwrap();

    session.complete(Completion::ExecutorRestarted {
        executor_epoch: ExecutorEpoch(active_epoch.0 - 1),
    });
    session.submit(Event::Open {
        operation_id: OperationId(72),
        document: fixture(),
    });
    let open = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { request, .. } => Some((**request).clone()),
            _ => None,
        })
        .unwrap();
    let mut restarted = CoreEngine::new();
    let reply = restarted.handle(open);
    let effects = session.complete(Completion::Core {
        request_id: core_effect(&effects).0,
        executor_epoch: active_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let next_epoch = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core { executor_epoch, .. } => Some(*executor_epoch),
            _ => None,
        })
        .unwrap();
    assert_eq!(next_epoch, active_epoch);
}

#[test]
fn selection_navigation_and_camera_updates_are_session_only() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut second = document.parts[0].clone();
    second.id = "key-2".into();
    second.reference = "SW2".into();
    second.pose.at = Vec2 { x: 19.0, y: 0.0 };
    document.parts.push(second);
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into(), "key-2".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let effects = session.submit(Event::Open {
        operation_id: OperationId(50),
        document,
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let token = session.read_model().accepted.as_ref().unwrap().token;

    assert!(
        session
            .submit(Event::SelectParts {
                operation_id: OperationId(51),
                part_ids: vec!["key".into()],
                range_part_ids: vec![],
                mode: boardstudio_application::SelectionMode::Replace
            })
            .iter()
            .any(|effect| matches!(
                effect,
                Effect::Settled {
                    outcome: TerminalOutcome::Completed,
                    ..
                }
            ))
    );
    session.submit(Event::SelectParts {
        operation_id: OperationId(52),
        part_ids: vec!["key-2".into()],
        range_part_ids: vec!["key".into(), "key-2".into()],
        mode: boardstudio_application::SelectionMode::Range,
    });
    session.submit(Event::Navigate {
        operation_id: OperationId(53),
        board_id: "main".into(),
        instance_id: None,
    });
    session.submit(Event::SetCamera {
        operation_id: OperationId(54),
        center: Vec2 { x: 10.0, y: 12.0 },
        zoom: 2.0,
    });

    assert_eq!(session.read_model().selected_part_ids, vec!["key", "key-2"]);
    assert_eq!(session.read_model().active_board_id, "main");
    assert_eq!(
        session.read_model().camera,
        boardstudio_application::CameraState {
            center: Vec2 { x: 10.0, y: 12.0 },
            zoom: 2.0
        }
    );
    assert_eq!(session.read_model().accepted.as_ref().unwrap().token, token);
}

#[test]
fn cancelled_pointer_makes_late_preview_inert_and_releases_capture() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(60),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "cancel-me".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 4.0, y: 0.0 },
        }],
        alt: true,
    });
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let effects = session.submit(Event::GestureFrame { generation });
    let (preview_id, preview_epoch, preview_request) = core_effect(&effects);
    let effects = session.submit(Event::GestureCancel { pointer_id: 7 });
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ReleasePointer { pointer_id: 7 }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(60),
            outcome: TerminalOutcome::Cancelled
        }
    )));
    let stale_reply = engine.handle(preview_request);
    session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(stale_reply),
    });
    assert!(session.read_model().display_preview.is_none());
    assert!(session.read_model().gesture.is_none());
}

#[test]
fn same_id_revision_reopen_invalidates_generation_and_export_tokens() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let generation_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(70),
        scope: scope.clone(),
    });
    let job_id = generation_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .unwrap();
    let export_effects = session.submit(Event::StartExport {
        operation_id: OperationId(71),
        scope: scope.clone(),
    });
    let old_token = export_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .unwrap();

    let durable = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(72),
        document: durable.clone(),
    });
    assert!(effects.iter().any(
        |effect| matches!(effect, Effect::CancelJob { job_id: cancelled } if *cancelled == job_id)
    ));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(71)
        }
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(71),
            outcome: TerminalOutcome::Cancelled
        }
    )));
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let current = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(current.document.id, durable.id);
    assert_eq!(current.document.revision, durable.revision);
    assert_ne!(current.token, old_token);
    assert_ne!(current.session_epoch, scope.session_epoch);

    let stale = session.complete(Completion::GenerationFinished {
        job_id,
        scope: scope.clone(),
        exact: true,
    });
    assert!(stale.is_empty());
    let stale_export = session.complete(Completion::ExportFinished {
        operation_id: OperationId(71),
        token: old_token,
        scope,
        artifact_id: "wrong-scope.step".into(),
    });
    assert!(
        !stale_export
            .iter()
            .any(|effect| matches!(effect, Effect::DeliverExport { .. }))
    );
}

#[test]
fn cancelled_export_is_no_longer_current_before_its_async_cancel_effect_runs() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let effects = session.submit(Event::StartExport {
        operation_id: OperationId(81),
        scope: scope.clone(),
    });
    let token = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .expect("export dispatch captures the accepted snapshot");

    assert!(session.export_is_current(OperationId(81), token, &scope));
    let durable = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(82),
        document: durable,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(81)
        }
    )));
    assert!(!session.export_is_current(OperationId(81), token, &scope));
}

#[test]
fn generation_block_is_typed_and_stale_block_completion_is_ignored() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(80),
        scope: scope.clone(),
    });
    let job_id = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .unwrap();
    let mut stale_scope = scope.clone();
    stale_scope.board_id = "other-board".into();
    session.complete(Completion::GenerationBlocked {
        job_id,
        scope: stale_scope,
        reason: "stale blocked outcome".into(),
    });
    assert!(matches!(
        session.read_model().generation,
        boardstudio_application::GenerationStatus::Preparing { job_id: current } if current == job_id
    ));
    let effects = session.complete(Completion::GenerationBlocked {
        job_id,
        scope,
        reason: "case has blocking findings".into(),
    });
    assert!(matches!(
        session.read_model().generation,
        boardstudio_application::GenerationStatus::Blocked { job_id: current, .. } if current == job_id
    ));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(80),
            outcome: TerminalOutcome::Rejected(reason)
        } if reason == "case has blocking findings"
    )));
}
