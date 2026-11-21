use super::*;

#[test]
fn owned_gesture_commit_finishes_before_project_open() {
    for preview_first in [true, false] {
        let mut session = Session::new(CoreEngine::validate_open);
        let mut engine = CoreEngine::new();
        open_ready(&mut session, &mut engine);
        session.submit(Event::GestureBegin {
            operation_id: OperationId(21),
            pointer_id: 7,
            target_ids: vec!["key".into()],
            transaction_id: "owned-before-open".into(),
            start: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
            }],
            pitch: Vec2 { x: 19.05, y: 19.05 },
            snap_fraction: 0.0,
            geometry_snap: false,
            gap: None,
            pixels_per_world: None,
            alt: false,
        });
        let preview = if preview_first {
            session.submit(Event::GestureSample {
                pointer_id: 7,
                positions: vec![Position {
                    id: "key".into(),
                    at: Vec2 { x: 4.0, y: 0.0 },
                }],
                pixels_per_world: None,
                alt: false,
            });
            let generation = session.read_model().gesture.as_ref().unwrap().generation;
            Some(session.submit(Event::GestureFrame { generation }))
        } else {
            None
        };
        let commit = session.submit(Event::GestureEnd {
            pointer_id: 7,
            final_positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 4.0, y: 0.0 },
            }],
            pixels_per_world: None,
            alt: false,
        });
        let (request_id, executor_epoch, request) =
            core_effect(preview.as_ref().unwrap_or(&commit));
        let next = ProjectDoc::empty("next-project", "Next");
        let open = session.submit(Event::Open {
            operation_id: OperationId(22),
            document: next.clone(),
        });
        assert!(
            !open.iter().any(|effect| matches!(
                effect,
                Effect::Settled {
                    operation_id: OperationId(21),
                    outcome: TerminalOutcome::Cancelled,
                    ..
                }
            )),
            "project switching must not cancel an owned commit"
        );
        let mut effects = session.complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(engine.handle(request)),
        });
        if preview_first {
            let (request_id, executor_epoch, request) = core_effect(&effects);
            effects = session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(engine.handle(request)),
            });
        }
        let (save_attempt_id, saved) = save_effect(&effects);
        assert_eq!(saved.id, "project");
        assert_eq!(saved.parts[0].pose.at.x, 4.0);
        let effects = session.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        assert_eq!(
            settled_landing(&effects, OperationId(21)).0,
            TerminalOutcome::Completed
        );
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .as_ref(),
            &saved
        );
        let (replacement, _) = settle_core_and_save(&mut session, &mut engine, effects);
        assert_eq!(replacement, next);
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .as_ref(),
            &next
        );
    }
}

#[test]
fn invalid_open_preserves_the_current_gesture_and_accepted_session() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    session.submit(Event::GestureBegin {
        operation_id: OperationId(21),
        pointer_id: 7,
        target_ids: vec!["key".into()],
        transaction_id: "keep-draft".into(),
        start: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        pitch: Vec2 { x: 19.05, y: 19.05 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: false,
    });
    let before = session.read_model().clone();
    let mut invalid = fixture();
    invalid.format_version = 6;
    let mut effects = session.submit(Event::Open {
        operation_id: OperationId(22),
        document: invalid,
    });
    if effects
        .iter()
        .any(|effect| matches!(effect, Effect::Core { .. }))
    {
        let (request_id, epoch, request) = core_effect(&effects);
        effects.extend(session.complete(Completion::Core {
            request_id,
            executor_epoch: epoch,
            reply: Box::new(engine.handle(request)),
        }));
    }
    assert!(effects.iter().any(|effect| matches!(effect,
        Effect::Settled { operation_id: OperationId(22), outcome: TerminalOutcome::Rejected(reason), .. }
            if reason.contains("No conversion"))));
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::Persist { .. } | Effect::ReleasePointer { .. }
    )));
    assert_eq!(session.read_model().accepted, before.accepted);
    assert_eq!(session.read_model().gesture, before.gesture);
    assert_eq!(
        session.scope(),
        Some(boardstudio_application::Scope {
            session_epoch: before.accepted.unwrap().session_epoch,
            document_id: "project".into(),
            board_id: String::new(),
            instance_id: None
        })
    );
}

#[test]
fn group_position_edit_moves_selected_parts_together_and_undo_restores_both() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    let mut second = document.parts[0].clone();
    second.id = "key-2".into();
    second.reference = "SW2".into();
    second.pose.at.x = 10.0;
    document.parts.push(second);
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document,
    });
    settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(
        OperationId(2),
        EditCommand {
            base_revision: 0,
            transaction_id: "group-position-blur".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into(), "key-2".into()],
            operation: EditOperation::MoveParts {
                positions: vec![
                    Position {
                        id: "key".into(),
                        at: Vec2 { x: 3.0, y: 0.0 },
                    },
                    Position {
                        id: "key-2".into(),
                        at: Vec2 { x: 13.0, y: 0.0 },
                    },
                ],
            },
        },
    ));
    let (moved, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(moved.parts[0].pose.at.x, 3.0);
    assert_eq!(moved.parts[1].pose.at.x, 13.0);

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    let (restored, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(restored.parts[0].pose.at.x, 0.0);
    assert_eq!(restored.parts[1].pose.at.x, 10.0);
}

#[test]
fn undo_save_completing_after_a_superseded_drag_save_keeps_the_restored_document() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (opened, _) = settle_core_and_save(&mut session, &mut engine, effects);
    assert_eq!(opened.revision, 0);

    // Commit a drag-like change and hold its save open: Persist A is the only
    // outstanding completion.
    let effects = session.submit(move_edit(2, 0, 3.0));
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (attempt_a, dragged) = save_effect(&effects);
    assert_eq!(dragged.revision, 1);
    assert_eq!(dragged.parts[0].pose.at.x, 3.0);

    // Undo while save A is still in flight: the intent queues behind the save
    // instead of dispatching a second core request.
    let queued = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    assert!(
        !queued
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. })),
        "Undo must wait for the outstanding save, not race it"
    );

    // The host persists the dragged revision; only then does Undo run and
    // overwrite the single pending-save slot with its own Persist B.
    let effects = session.complete(Completion::Persist {
        save_attempt_id: attempt_a,
        result: SaveResult::Committed,
    });
    assert_eq!(
        settled_landing(&effects, OperationId(2)).0,
        TerminalOutcome::Completed
    );
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (attempt_b, restored) = save_effect(&effects);
    assert_ne!(attempt_b, attempt_a);
    assert_eq!(restored.parts[0].pose.at.x, 0.0);
    assert_ne!(restored, dragged);

    // The Undo save lands, then a late duplicate of save A arrives (a retried
    // host callback after the slot moved on). It must be dropped: the accepted
    // document stays the restored one and durability stays Saved.
    let effects = session.complete(Completion::Persist {
        save_attempt_id: attempt_b,
        result: SaveResult::Committed,
    });
    assert_eq!(
        settled_landing(&effects, OperationId(3)).0,
        TerminalOutcome::Completed
    );
    let late = session.complete(Completion::Persist {
        save_attempt_id: attempt_a,
        result: SaveResult::Committed,
    });
    assert!(
        late.is_empty(),
        "a late completion for a superseded save must be dropped, got: {late:?}"
    );
    let accepted = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(accepted.document.as_ref(), &restored);
    assert!(
        matches!(
            session.read_model().durability,
            Durability::Saved { revision } if revision == restored.revision
        ),
        "durability must track the restored document"
    );
}

#[test]
fn preview_edit_displays_without_changing_the_accepted_document_or_history() {
    let mut session = Session::new(CoreEngine::validate_open);
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

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(11),
        transaction_id: "preview-position".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 8.0, y: 0.0 },
            }],
        },
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    assert!(matches!(
        &request,
        CoreRequest::Edit { command, .. } if command.phase == EditPhase::Preview
    ));
    let mut reply = engine.handle(request);
    add_preview_electrical_notice(&mut reply, "first");
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });

    let after_preview = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(after_preview.document.as_ref(), &accepted);
    assert_eq!(after_preview.document.revision, 0);
    let accepted_token = after_preview.token;
    let display_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(display_preview.findings.iter().any(|finding| {
        finding.id == "preview:electrical:first"
            && finding.severity == Severity::Info
            && finding.scope == Scope::Pcb
    }));

    let board_id = session.read_model().active_board_id.clone();
    let clear = |operation_id, transaction_id: &str| Event::ClearPreview {
        operation_id: OperationId(operation_id),
        token: accepted_token,
        revision: 0,
        board_id: board_id.clone(),
        transaction_id: transaction_id.into(),
    };

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(13),
        transaction_id: "replacement-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 12.0, y: 0.0 },
            }],
        },
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let mut reply = engine.handle(request);
    add_preview_electrical_notice(&mut reply, "replacement");
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let replacement_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(
        replacement_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:replacement")
    );
    assert!(
        !replacement_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:first")
    );

    session.submit(clear(14, "preview-position"));
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:replacement")
    );
    session.submit(clear(15, "replacement-preview"));
    assert!(session.read_model().display_preview.is_none());

    let accepted_after_clears = session.read_model().accepted.as_ref().unwrap();
    assert_eq!(accepted_after_clears.document.as_ref(), &accepted);
    assert_eq!(accepted_after_clears.document.revision, 0);
    assert_eq!(accepted_after_clears.token, accepted_token);

    let undo = engine.handle(CoreRequest::Undo {
        id: "preview-history-check".into(),
    });
    assert!(matches!(
        undo,
        CoreReply::Error { message, .. } if message == "History is empty"
    ));
}

#[test]
fn part_drag_uses_gap_geometry_snap_at_the_two_millimeter_tolerance() {
    let mut session = Session::new(CoreEngine::validate_open);
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
        pixels_per_world: None,
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.6, y: 0.0 },
        }],
        pixels_per_world: None,
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
    let mut session = Session::new(CoreEngine::validate_open);
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
        pixels_per_world: None,
        alt: false,
    });
    session.submit(Event::GestureSample {
        pointer_id: 8,
        positions: vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 19.4, y: 0.0 },
        }],
        pixels_per_world: None,
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
fn drag_preview_acknowledges_latest_and_coalesces_inflight_samples() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let position = |x| {
        vec![Position {
            id: "key".into(),
            at: Vec2 { x, y: 0.0 },
        }]
    };
    session.submit(Event::GestureBegin {
        operation_id: OperationId(30),
        pointer_id: 77,
        target_ids: vec!["key".into()],
        transaction_id: "preview-ack".into(),
        start: position(0.0),
        pitch: Vec2 { x: 19.0, y: 19.0 },
        snap_fraction: 0.0,
        geometry_snap: false,
        gap: None,
        pixels_per_world: None,
        alt: true,
    });
    let sample = |x| Event::GestureSample {
        pointer_id: 77,
        positions: position(x),
        pixels_per_world: None,
        alt: true,
    };
    session.submit(sample(1.0));
    let generation = session.read_model().gesture.as_ref().unwrap().generation;
    let (id, epoch, request) = core_effect(&session.submit(Event::GestureFrame { generation }));
    let effects = session.complete(Completion::Core {
        request_id: id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    assert!(
        !effects.iter().any(|e| matches!(e, Effect::Core { .. })),
        "unchanged acknowledged sample must not resubmit"
    );
    session.submit(sample(2.0));
    let (id, epoch, request) = core_effect(&session.submit(Event::GestureFrame { generation }));
    session.submit(sample(3.0));
    session.submit(sample(4.0));
    let effects = session.complete(Completion::Core {
        request_id: id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (id, epoch, request) = core_effect(&effects);
    assert!(
        matches!(&request, CoreRequest::Edit { command: EditCommand { phase: EditPhase::Preview, operation: EditOperation::MoveParts { positions }, .. }, .. } if positions == &position(4.0))
    );
    let effects = session.complete(Completion::Core {
        request_id: id,
        executor_epoch: epoch,
        reply: Box::new(engine.handle(request)),
    });
    assert!(
        !effects.iter().any(|e| matches!(e, Effect::Core { .. })),
        "latest follow-up must run only once"
    );
    session.submit(sample(0.0));
    let (_, _, request) = core_effect(&session.submit(Event::GestureFrame { generation }));
    assert!(
        matches!(&request, CoreRequest::Edit { command: EditCommand { phase: EditPhase::Preview, operation: EditOperation::MoveParts { positions }, .. }, .. } if positions == &position(0.0))
    );
}

#[test]
fn drag_coalesces_previews_and_commits_the_pointerup_sample_once() {
    let mut session = Session::new(CoreEngine::validate_open);
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
        pixels_per_world: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
        }],
        pixels_per_world: None,
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
        pixels_per_world: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 77,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 5.0, y: 0.0 },
        }],
        pixels_per_world: None,
        alt: true,
    });
    session.submit(Event::GestureEnd {
        pointer_id: 77,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        pixels_per_world: None,
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
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    assert!(
        matches!(engine.handle(CoreRequest::Undo { id: "undo-check".into() }), CoreReply::Scene { scene, document, .. } if scene.revision == 2 && document.parts[0].pose.at.x == 0.0)
    );
}
