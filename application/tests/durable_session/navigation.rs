use super::*;

#[test]
fn selection_navigation_and_camera_updates_are_session_only() {
    let mut session = Session::new(CoreEngine::validate_open);
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

    let effects = session.submit(Event::SelectParts {
        operation_id: OperationId(51),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            outcome: TerminalOutcome::Completed,
            ..
        }
    )));
    assert!(
        settled_landing(&effects, OperationId(51)).1.is_none(),
        "selection changes no document"
    );
    session.submit(Event::SelectParts {
        operation_id: OperationId(60),
        part_ids: vec!["key-2".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Add,
    });
    assert_eq!(session.read_model().selected_part_ids, vec!["key", "key-2"]);
    session.submit(Event::SelectParts {
        operation_id: OperationId(61),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Toggle,
    });
    assert_eq!(session.read_model().selected_part_ids, vec!["key-2"]);
    session.submit(Event::SelectParts {
        operation_id: OperationId(52),
        part_ids: vec!["key-2".into()],
        range_part_ids: vec!["key".into(), "key-2".into()],
        mode: boardstudio_application::SelectionMode::Range,
    });
    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(53),
        board_id: "main".into(),
        instance_id: None,
    });
    assert!(
        settled_landing(&effects, OperationId(53)).1.is_none(),
        "navigation changes no document"
    );
    let effects = session.submit(Event::SetCamera {
        operation_id: OperationId(54),
        center: Vec2 { x: 10.0, y: 12.0 },
        zoom: 2.0,
    });
    assert!(
        settled_landing(&effects, OperationId(54)).1.is_none(),
        "camera changes no document"
    );
    let effects = session.submit(Event::CancelGeneration {
        operation_id: OperationId(55),
    });
    assert!(
        settled_landing(&effects, OperationId(55)).1.is_none(),
        "cancelling a job changes no document"
    );

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
fn preview_queued_behind_a_commit_is_discarded_after_its_revision_changes() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);

    let effects = session.submit(move_edit(20, 0, 3.0));
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    let commit_reply = engine.handle(commit_request);
    session.submit(Event::PreviewEdit {
        operation_id: OperationId(21),
        transaction_id: "stale-position-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 8.0, y: 0.0 },
            }],
        },
    });

    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(commit_reply),
    });
    let (save_attempt_id, committed) = save_effect(&effects);
    assert_eq!(committed.parts[0].pose.at.x, 3.0);

    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(21),
            outcome: TerminalOutcome::Superseded,
            ..
        }
    )));
    assert!(session.read_model().display_preview.is_none());
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
        3.0
    );
}

#[test]
fn matrix_range_selects_rectangle_of_live_primary_members_and_keeps_anchor() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let (document, matrix_scope_ids) = matrix_range_fixture();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(501),
        document,
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.as_ref().unwrap();
    let scope = boardstudio_application::Scope {
        session_epoch: accepted.session_epoch,
        document_id: accepted.document.id.clone(),
        board_id: "main".into(),
        instance_id: None,
    };

    let anchor = "matrix/main/r0c1".to_owned();
    let target = "matrix/main/r2c1".to_owned();
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(502),
        scope: scope.clone(),
        matrix_id: "main".into(),
        target_part_id: anchor.clone(),
        // The projection is an extent whose first member differs from the clicked key.
        part_ids: matrix_scope_ids.clone(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(503),
        scope: scope.clone(),
        matrix_id: "main".into(),
        target_part_id: target.clone(),
        part_ids: vec![target.clone()],
        mode: boardstudio_application::SelectionMode::Range,
    });

    assert_eq!(
        session.read_model().selected_part_ids,
        vec![anchor.clone(), target]
    );
    assert_eq!(
        session.read_model().selection_anchor_id.as_deref(),
        Some(anchor.as_str())
    );
    assert_eq!(
        session.read_model().selection_mode,
        boardstudio_application::SelectionMode::Range
    );

    let repeated_target = "matrix/main/r2c0".to_owned();
    session.submit(Event::SelectMatrixCell {
        operation_id: OperationId(504),
        scope,
        matrix_id: "main".into(),
        target_part_id: repeated_target,
        part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Range,
    });
    assert_eq!(
        session.read_model().selected_part_ids,
        vec![
            "matrix/main/r0c0",
            "matrix/main/r0c1",
            "matrix/main/r1c0",
            "matrix/main/r2c0",
            "matrix/main/r2c1"
        ]
    );
    assert_eq!(
        session.read_model().selection_anchor_id.as_deref(),
        Some(anchor.as_str())
    );
}

#[test]
fn cancelled_pointer_makes_late_preview_inert_and_releases_capture() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(59),
        transaction_id: "notice-before-gesture".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 2.0, y: 0.0 },
            }],
        },
    });
    let (notice_id, notice_epoch, notice_request) = core_effect(&effects);
    let mut notice_reply = engine.handle(notice_request);
    add_preview_electrical_notice(&mut notice_reply, "gesture-current");
    session.complete(Completion::Core {
        request_id: notice_id,
        executor_epoch: notice_epoch,
        reply: Box::new(notice_reply),
    });
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-current")
    );

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
        pixels_per_world: None,
        alt: true,
    });
    session.submit(Event::GestureSample {
        pointer_id: 7,
        positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 4.0, y: 0.0 },
        }],
        pixels_per_world: None,
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
            outcome: TerminalOutcome::Cancelled,

            ..
        }
    )));
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-current")
    );
    let accepted = session.read_model().accepted.as_ref().unwrap().clone();
    let mut stale_reply = engine.handle(preview_request);
    add_preview_electrical_notice(&mut stale_reply, "gesture-late");
    let duplicate_stale_reply = stale_reply.clone();

    let effects = session.submit(Event::PreviewEdit {
        operation_id: OperationId(61),
        transaction_id: "after-cancel-preview".into(),
        target_ids: vec!["key".into()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: "key".into(),
                at: Vec2 { x: 9.0, y: 0.0 },
            }],
        },
    });
    assert!(
        !has_core_request(&effects),
        "the newer preview waits for the canceled request"
    );

    let effects = session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(stale_reply),
    });
    assert!(
        session
            .read_model()
            .display_preview
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-current")
    );
    let (newer_id, newer_epoch, newer_request) = core_effect(&effects);
    let mut newer_reply = engine.handle(newer_request);
    add_preview_electrical_notice(&mut newer_reply, "after-cancel-newer");
    session.complete(Completion::Core {
        request_id: newer_id,
        executor_epoch: newer_epoch,
        reply: Box::new(newer_reply),
    });

    let newer_preview = session.read_model().display_preview.as_ref().unwrap();
    assert_eq!(newer_preview.revision, accepted.document.revision);
    assert!(
        newer_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:after-cancel-newer")
    );
    assert!(
        !newer_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-late")
    );

    // Core is single-flight; replay after dispatch to exercise staleness against the newer display.
    session.complete(Completion::Core {
        request_id: preview_id,
        executor_epoch: preview_epoch,
        reply: Box::new(duplicate_stale_reply),
    });
    assert!(session.read_model().gesture.is_none());
    assert_eq!(session.read_model().accepted.as_ref(), Some(&accepted));
    let display_preview = session.read_model().display_preview.as_ref().unwrap();
    assert!(
        display_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:after-cancel-newer")
    );
    assert!(
        !display_preview
            .findings
            .iter()
            .any(|finding| finding.id == "preview:electrical:gesture-late")
    );
}

#[test]
fn same_id_revision_reopen_invalidates_generation_and_export_tokens() {
    let mut session = Session::new(CoreEngine::validate_open);
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

    assert!(session.export_is_current(OperationId(71), old_token, &scope));

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
    assert!(!session.export_is_current(OperationId(71), old_token, &scope));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(71),
            outcome: TerminalOutcome::Cancelled,

            ..
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
fn instance_navigation_updates_scope_and_cancels_in_flight_case_work() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let mut document = fixture();
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.hardware = Some(HardwareConfiguration {
        instances: vec!["left", "right"]
            .into_iter()
            .map(|half| PhysicalBoardInstance {
                id: format!("instance-{half}"),
                name: format!("{half} instance"),
                board_id: "main".into(),
                half: half.into(),
                role: "primary".into(),
                flipped: half == "right",
                controller_part_id: None,
                mechanical: None,
                construction_ownership:
                    boardstudio_core::model::MechanicalConstructionOwnership::Local,
                mechanical_identity_reservation: None,
            })
            .collect(),
        ..HardwareConfiguration::default()
    });

    let effects = session.submit(Event::Open {
        operation_id: OperationId(10),
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

    let canonical_scope = session.scope().unwrap();
    assert_eq!(canonical_scope.board_id, "main");
    assert_eq!(canonical_scope.instance_id, None);
    let generation_effects = session.submit(Event::StartGeneration {
        operation_id: OperationId(11),
        scope: canonical_scope.clone(),
    });
    let job_id = generation_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunGeneration { job_id, .. } => Some(*job_id),
            _ => None,
        })
        .expect("generation should start in canonical scope");
    let export_effects = session.submit(Event::StartExport {
        operation_id: OperationId(12),
        scope: canonical_scope.clone(),
    });
    assert!(export_effects.iter().any(|effect| matches!(
        effect,
        Effect::RunExport {
            operation_id: OperationId(12),
            ..
        }
    )));

    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(13),
        board_id: "main".into(),
        instance_id: Some("instance-left".into()),
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelJob { job_id: cancelled } if *cancelled == job_id
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(12)
        }
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(13),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    let physical_scope = session.scope().unwrap();
    assert_eq!(physical_scope.board_id, "main");
    assert_eq!(physical_scope.instance_id.as_deref(), Some("instance-left"));
    assert_ne!(physical_scope, canonical_scope);

    let effects = session.submit(Event::Navigate {
        operation_id: OperationId(14),
        board_id: "main".into(),
        instance_id: None,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(14),
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
    assert_eq!(session.scope().unwrap(), canonical_scope);

    for (operation_id, board_id, instance_id) in [
        (15, "missing-board", None),
        (16, "main", Some("missing-instance")),
    ] {
        let effects = session.submit(Event::Navigate {
            operation_id: OperationId(operation_id),
            board_id: board_id.into(),
            instance_id: instance_id.map(str::to_owned),
        });
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Settled {
                operation_id: id,
                outcome: TerminalOutcome::Rejected(_),

            ..} if *id == OperationId(operation_id)
        )));
        assert_eq!(session.scope().unwrap(), canonical_scope);
    }
}

#[test]
fn failed_export_is_removed_and_not_cancelled_again_after_reopen() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let scope = session.scope().unwrap();
    let effects = session.submit(Event::StartExport {
        operation_id: OperationId(83),
        scope: scope.clone(),
    });
    let token = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .expect("export dispatch captures the accepted snapshot");

    session.complete(Completion::ExportFailed {
        operation_id: OperationId(83),
        reason: "archive writer failed".into(),
    });

    assert!(!session.export_is_current(OperationId(83), token, &scope));
    let durable = session
        .read_model()
        .accepted
        .as_ref()
        .unwrap()
        .document
        .as_ref()
        .clone();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(84),
        document: durable,
    });
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::CancelExport {
            operation_id: OperationId(83)
        }
    )));
}

#[test]
fn generation_block_is_typed_and_stale_block_completion_is_ignored() {
    let mut session = Session::new(CoreEngine::validate_open);
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
            outcome: TerminalOutcome::Rejected(reason),

        ..} if reason == "case has blocking findings"
    )));
}
