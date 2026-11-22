use super::*;

#[test]
fn uncertain_worker_outcome_is_never_replayed_and_requires_explicit_reopen() {
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
            outcome: TerminalOutcome::ExecutorFailed(_),

            ..
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
            outcome: TerminalOutcome::Completed,

            ..
        }
    )));
}

#[test]
fn failed_replacement_retries_the_exact_candidate_without_core_or_early_publication() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    settle_core_and_save(&mut session, &mut engine, effects);
    let before = session.read_model().accepted.clone().unwrap();
    let scope = session.scope();
    let candidate = ProjectDoc::empty("fresh-replacement", "Fresh");
    let effects = session.submit(Event::Open {
        operation_id: OperationId(71),
        document: candidate.clone(),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, pending) = save_effect(&effects);
    assert_eq!(pending, candidate);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    assert_eq!(session.read_model().accepted.as_ref(), Some(&before));
    assert_eq!(session.scope(), scope);
    assert_eq!(session.read_model().lifecycle, Lifecycle::RecoveryRequired);
    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_id, retried) = save_effect(&effects);
    assert_eq!(retried, candidate);
    session.complete(Completion::Persist {
        save_attempt_id: retry_id,
        result: SaveResult::Committed,
    });
    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(accepted.document.as_ref(), &candidate);
    assert_ne!(accepted.session_epoch, before.session_epoch);
    let effects = session.submit(Event::Undo {
        operation_id: OperationId(73),
    });
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(73)).0, TerminalOutcome::Rejected(reason) if reason == "History is empty")
    );
    assert_eq!(session.read_model().accepted.as_ref(), Some(&accepted));
}

#[test]
fn invalid_recovery_keeps_the_exact_failed_result_for_storage_only_retry() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, pending) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("quota".into()),
    });
    let before = session.read_model().clone();
    let mut invalid = fixture();
    invalid.format_version = 6;
    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: invalid,
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(71)).0, TerminalOutcome::Rejected(reason) if reason.contains("Unsupported formatVersion: expected 8"))
    );
    assert_eq!(session.read_model(), &before);
    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Core { .. }))
    );
    let (retry_id, retried) = save_effect(&effects);
    assert_eq!(retried, pending);
    session.complete(Completion::Persist {
        save_attempt_id: retry_id,
        result: SaveResult::Committed,
    });
    assert_eq!(
        session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref(),
        &pending
    );
}

#[test]
fn recovery_reopen_discards_failed_save_and_ignores_stale_executor_restart() {
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

// Characterization for the IDB-quota UI escape hatch: after a quota-aborted save
// latches RecoveryRequired, opening a fresh document discards the retained oversized
// candidate without recommitting it, and RetrySave stays an explicit-only action.
// The Runtime create-new/delete recovery flows rely on this Session contract.
#[test]
fn quota_escape_recover_discards_oversized_candidate_and_retry_stays_explicit() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    open_ready(&mut session, &mut engine);
    let effects = session.submit(move_edit(70, 0, 4.0));
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let reply = engine.handle(request);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(reply),
    });
    let (save_attempt_id, oversized) = save_effect(&effects);
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Aborted("QuotaExceededError: origin storage is full".into()),
    });
    assert_eq!(session.read_model().lifecycle, Lifecycle::RecoveryRequired);

    let fresh = ProjectDoc::empty("quota-escape-fresh", "Untitled keyboard");
    let effects = session.submit(Event::RecoverWithDocument {
        operation_id: OperationId(71),
        document: fresh.clone(),
    });
    assert!(
        !effects.iter().any(
            |effect| matches!(effect, Effect::Persist { document, .. } if **document == oversized)
        ),
        "recovery must not recommit the quota-exceeding candidate"
    );
    let (request_id, executor_epoch, request) = core_effect(&effects);
    let effects = session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    });
    let (save_attempt_id, persisted) = save_effect(&effects);
    assert_eq!(
        persisted, fresh,
        "the only persistence after escape carries the fresh document"
    );
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
            .as_ref(),
        &fresh
    );
    assert!(
        matches!(
            settled_landing(&effects, OperationId(71)).0,
            TerminalOutcome::Completed
        ),
        "the escape hatch settles the fresh open as completed"
    );

    let effects = session.submit(Event::RetrySave {
        operation_id: OperationId(72),
    });
    assert!(
        matches!(settled_landing(&effects, OperationId(72)).0, TerminalOutcome::Rejected(reason) if reason == "there is no retained save to retry"),
        "after the escape hatch there is no retained save left to retry"
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Persist { .. })),
        "no automatic save retries the discarded document"
    );
}
