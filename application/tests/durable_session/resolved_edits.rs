use super::*;

#[test]
fn queued_position_edits_resolve_against_the_accepted_document() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: positioned_fixture(66.675, -47.625),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let seen = Rc::new(RefCell::new(Vec::new()));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector-x".into(),
        resolver: position_resolver("inspector-x", 'x', 60.0, seen.clone()),
    });
    assert!(
        has_core_request(&effects),
        "an intent at the head of an idle queue resolves immediately"
    );
    let (x_request_id, x_epoch, x_request) = core_effect(&effects);
    assert!(
        matches!(&x_request, CoreRequest::Edit { command, .. } if command.transaction_id == "m1-inspector-x-2"),
        "the transaction id is derived from the label and operation id"
    );

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector-y".into(),
        resolver: position_resolver("inspector-y", 'y', -40.0, seen.clone()),
    });
    assert!(
        !has_core_request(&effects),
        "the queued edit waits for the running one"
    );

    let reply = engine.handle(x_request);
    let effects = session.complete(Completion::Core {
        request_id: x_request_id,
        executor_epoch: x_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (y_request_id, y_epoch, y_request) = core_effect(&effects);
    assert!(
        matches!(&y_request, CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, .. }, .. } if positions[0].at == Vec2 { x: 60.0, y: -40.0 }),
        "the y edit is built from the document the x edit produced"
    );
    let reply = engine.handle(y_request);
    let effects = session.complete(Completion::Core {
        request_id: y_request_id,
        executor_epoch: y_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });

    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 { x: 60.0, y: -40.0 },
        "rapid x then y entry keeps both coordinates"
    );
    assert_eq!(
        seen.borrow().as_slice(),
        &[(66.675, -47.625), (60.0, -47.625)],
        "each resolver observes the commit accepted before it"
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(4),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 {
            x: 60.0,
            y: -47.625
        }
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(5),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        resolved_position(&accepted),
        &Vec2 {
            x: 66.675,
            y: -47.625
        },
        "the second undo restores the original position"
    );
}

#[test]
fn a_vanished_target_retires_with_the_resolvers_reason() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |_accepted| {
            Resolution::Retire("the selected part was deleted".into())
        }),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert_eq!(
        outcome,
        TerminalOutcome::Rejected("the selected part was deleted".into())
    );
    assert!(landing.is_none(), "a retired edit never landed");
    assert!(
        !has_core_request(&effects),
        "a retired edit never reaches Core"
    );
}

#[test]
fn an_unchanged_resolution_lands_at_the_current_snapshot_and_keeps_draining() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |_accepted| Resolution::Unchanged),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert_eq!(outcome, TerminalOutcome::Completed);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        landing,
        Some(Landing {
            revision: accepted.document.revision,
            token: accepted.token,
        }),
        "an unchanged edit lands at the current accepted snapshot"
    );
    assert!(
        !has_core_request(&effects),
        "an unchanged edit never reaches Core"
    );

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |accepted: &AcceptedSnapshot| {
            let mut document = (*accepted.document).clone();
            document.name = "Renamed".into();
            Resolution::Submit(EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "supplied-transaction".into(),
                phase: EditPhase::Commit,
                target_ids: vec![document.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            })
        }),
    });
    assert!(
        has_core_request(&effects),
        "the queue keeps draining after an unchanged resolution"
    );
    let (_request_id, _epoch, request) = core_effect(&effects);
    assert!(
        matches!(&request, CoreRequest::Edit { command, .. } if command.transaction_id == "supplied-transaction"),
        "a supplied transaction id is kept"
    );
    let (_, settled) = settle_core_and_save(&mut session, &mut engine, effects);
    let (outcome, _) = settled_landing(&settled, OperationId(3));
    assert_eq!(outcome, TerminalOutcome::Completed);
    assert_eq!(
        session.read_model().accepted.clone().unwrap().document.name,
        "Renamed"
    );
}

#[test]
fn a_resolved_command_core_rejects_settles_rejected_with_cores_message() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(2),
        label: "inspector".into(),
        resolver: EditResolver::new("inspector", |accepted: &AcceptedSnapshot| {
            Resolution::Submit(EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: String::new(),
                phase: EditPhase::Commit,
                target_ids: vec!["gone".into()],
                operation: EditOperation::MoveParts {
                    positions: vec![Position {
                        id: "gone".into(),
                        at: Vec2 { x: 1.0, y: 1.0 },
                    }],
                },
            })
        }),
    });
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(2));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none(), "a rejected resolution never landed");
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. })),
        "a rejected resolution never saves"
    );
}

#[test]
fn recovery_and_closing_refuse_resolvers_without_calling_them() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let called = Rc::new(Cell::new(false));
    let resolver = {
        let called = called.clone();
        EditResolver::new("inspector", move |_accepted| {
            called.set(true);
            Resolution::Unchanged
        })
    };

    let effects = session.submit(resolved_edit(
        OperationId(2),
        rename_edit_command(0, "Renamed"),
    ));
    let (request_id, epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let _effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert!(matches!(
        session.read_model().lifecycle,
        Lifecycle::RecoveryRequired
    ));

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver,
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(3));
    assert!(matches!(outcome, TerminalOutcome::BlockedByRecovery(_)));
    assert!(landing.is_none());
    assert!(!called.get(), "recovery refuses the resolver");

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(4),
    });
    let (retry_save_id, _) = save_effect(&effects);
    session.submit(Event::Close {
        operation_id: OperationId(5),
    });
    assert!(matches!(session.read_model().lifecycle, Lifecycle::Closing));

    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(6),
        label: "inspector".into(),
        resolver: {
            let called = called.clone();
            EditResolver::new("inspector", move |_accepted| {
                called.set(true);
                Resolution::Unchanged
            })
        },
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(6));
    assert!(matches!(
        &outcome,
        TerminalOutcome::BlockedByRecovery(reason) if reason == "session is closing"
    ));
    assert!(landing.is_none());
    assert!(!called.get(), "closing refuses the resolver");

    let effects = session.complete(Completion::Persist {
        save_attempt_id: retry_save_id,
        result: SaveResult::Committed,
    });
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(5),
            outcome: TerminalOutcome::Completed,
            ..
        }
    )));
}

#[test]
fn a_reopen_rejects_queued_resolvers_without_calling_them() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let mut second = fixture();
    second.name = "Second".into();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(2),
        document: second,
    });
    let (open_request_id, open_epoch, open_request) = core_effect(&effects);

    let called = Rc::new(Cell::new(false));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector".into(),
        resolver: {
            let called = called.clone();
            EditResolver::new("inspector", move |_accepted| {
                called.set(true);
                Resolution::Unchanged
            })
        },
    });
    assert!(
        !has_core_request(&effects),
        "the intent queues behind the running open"
    );

    let reply = engine.handle(open_request);
    let effects = session.complete(Completion::Core {
        request_id: open_request_id,
        executor_epoch: open_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (outcome, landing) = settled_landing(&effects, OperationId(3));
    assert!(matches!(
        &outcome,
        TerminalOutcome::Rejected(reason) if reason == "document session changed before command began"
    ));
    assert!(landing.is_none());
    assert!(
        !called.get(),
        "an epoch change drops the intent before its resolver runs"
    );
}

#[test]
fn a_gesture_commit_ahead_is_not_disturbed_and_the_intent_behind_resolves_on_its_result() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: fixture(),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    session.submit(Event::GestureBegin {
        operation_id: OperationId(2),
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
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        pixels_per_world: None,
        alt: true,
    });
    let effects = session.submit(Event::GestureEnd {
        pointer_id: 77,
        final_positions: vec![Position {
            id: "key".into(),
            at: Vec2 { x: 9.0, y: 0.0 },
        }],
        pixels_per_world: None,
        alt: true,
    });
    let (commit_id, commit_epoch, commit_request) = core_effect(&effects);
    assert!(
        matches!(
            &commit_request,
            CoreRequest::Edit { command: EditCommand { phase: EditPhase::Commit, operation: EditOperation::MoveParts { positions }, .. }, .. }
                if positions[0].at == Vec2 { x: 9.0, y: 0.0 }
        ),
        "the gesture commit runs first, untouched"
    );

    let seen = Rc::new(RefCell::new(Vec::new()));
    let effects = session.submit(Event::ResolveEdit {
        operation_id: OperationId(3),
        label: "inspector-y".into(),
        resolver: position_resolver("inspector-y", 'y', 5.0, seen),
    });
    assert!(
        !has_core_request(&effects),
        "the resolved edit queues behind the running gesture commit"
    );

    let reply = engine.handle(commit_request);
    let effects = session.complete(Completion::Core {
        request_id: commit_id,
        executor_epoch: commit_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    let effects = session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let (edit_id, edit_epoch, edit_request) = core_effect(&effects);
    assert!(
        matches!(
            &edit_request,
            CoreRequest::Edit { command: EditCommand { operation: EditOperation::MoveParts { positions }, .. }, .. }
                if positions[0].at == Vec2 { x: 9.0, y: 5.0 }
        ),
        "the intent behind the gesture resolves against the gesture's result"
    );

    let reply = engine.handle(edit_request);
    let effects = session.complete(Completion::Core {
        request_id: edit_id,
        executor_epoch: edit_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, _) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    });
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(resolved_position(&accepted), &Vec2 { x: 9.0, y: 5.0 });
}
