use super::package_then_protect;
use boardstudio_application::{
    AcceptedSnapshot, Completion, Effect, Event, ExportCommitRequest, OperationId, SaveResult,
    Scope, Session, SnapshotToken,
};
use boardstudio_core::CoreEngine;
use boardstudio_core_mechanical::electrical::{
    ElectricalMode, ElectricalPlan, ElectricalPlanRequest,
};
use boardstudio_model::model::{Board, CoreReply, CoreRequest, ProjectDoc};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::Future,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type SharedSession = Rc<RefCell<Session>>;
type SharedEngine = Rc<RefCell<CoreEngine>>;

#[test]
fn failed_package_retry_and_stale_owner_use_session_protection_order() {
    block_on(async {
        let (session, engine, accepted, scope) = session_fixture();
        let first_export = OperationId(901);
        let first_token = start_export(&session, first_export, scope.clone());
        let protection_calls = Cell::new(0);

        let failed = package_then_protect(
            async { Err("injected archive packaging failure".to_owned()) },
            || require_current(&session, first_export, first_token, &scope),
            || async {
                protection_calls.set(protection_calls.get() + 1);
                commit_protection(
                    &session,
                    &engine,
                    first_export,
                    first_token,
                    &scope,
                    resolve_plan(&engine, &accepted, &scope),
                )
                .await
            },
            |token| require_current(&session, first_export, *token, &scope),
        )
        .await;
        assert_eq!(failed, Err("injected archive packaging failure".to_owned()));
        assert_eq!(protection_calls.get(), 0);
        assert!(!has_protection(&session, &scope));

        let failure_effects = session.borrow_mut().complete(Completion::ExportFailed {
            operation_id: first_export,
            reason: "archive packaging failed".into(),
        });
        settle_session(&session, &engine, failure_effects);
        let retry_export = OperationId(902);
        let retry_token = start_export(&session, retry_export, scope.clone());
        let retry_accepted = session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .expect("accepted project for retry")
            .clone();
        let retry_plan = resolve_plan(&engine, &retry_accepted, &scope);
        let retried = package_then_protect(
            async { Ok(vec![0x50, 0x4b]) },
            || require_current(&session, retry_export, retry_token, &scope),
            || async {
                protection_calls.set(protection_calls.get() + 1);
                commit_protection(
                    &session,
                    &engine,
                    retry_export,
                    retry_token,
                    &scope,
                    retry_plan,
                )
                .await
            },
            |token| require_current(&session, retry_export, *token, &scope),
        )
        .await
        .expect("the same export action can retry after package failure");
        assert_eq!(retried.0, vec![0x50, 0x4b]);
        assert_eq!(protection_calls.get(), 1);
        assert!(has_protection(&session, &scope));

        let (stale_session, stale_engine, _stale_accepted, stale_scope) = session_fixture();
        let stale_export = OperationId(903);
        let stale_token = start_export(&stale_session, stale_export, stale_scope.clone());
        let stale_protection_calls = Cell::new(0);
        let stale_mutation_pending = Cell::new(false);
        let stale_accepted = stale_session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .expect("accepted project for stale export")
            .clone();
        let stale_plan = resolve_plan(&stale_engine, &stale_accepted, &stale_scope);
        let stale_result = package_then_protect(
            async {
                stale_mutation_pending.set(true);
                Ok(vec![0x50, 0x4b])
            },
            || {
                if stale_mutation_pending.replace(false) {
                    reopen_current_document(&stale_session, &stale_engine);
                }
                require_current(&stale_session, stale_export, stale_token, &stale_scope)
            },
            || async {
                stale_protection_calls.set(stale_protection_calls.get() + 1);
                commit_protection(
                    &stale_session,
                    &stale_engine,
                    stale_export,
                    stale_token,
                    &stale_scope,
                    stale_plan,
                )
                .await
            },
            |token| require_current(&stale_session, stale_export, *token, &stale_scope),
        )
        .await;
        assert_eq!(
            stale_result,
            Err("KiCad export was superseded before wiring protection.".to_owned())
        );
        assert_eq!(stale_protection_calls.get(), 0);
        assert!(!has_protection(&stale_session, &stale_scope));
    });
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Same one-board accepted project the browser fixtures' `opened_session` yields.
fn opened_session() -> (Session, AcceptedSnapshot, Scope) {
    let mut document = ProjectDoc::empty("zmk-export-test", "ZMK export test");
    document.boards.push(Board {
        id: "main-board".into(),
        name: "Main board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    let mut session = Session::new(CoreEngine::validate_open);
    let mut core = CoreEngine::new();
    let mut effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document,
    });
    while let Some(effect) = effects.pop() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => effects.extend(session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(core.handle(*request)),
            })),
            Effect::Persist {
                save_attempt_id, ..
            } => effects.extend(session.complete(Completion::Persist {
                save_attempt_id,
                result: SaveResult::Committed,
            })),
            _ => {}
        }
    }
    let accepted = session.read_model().accepted.clone().unwrap();
    let scope = session.scope().unwrap();
    (session, accepted, scope)
}

fn session_fixture() -> (SharedSession, SharedEngine, AcceptedSnapshot, Scope) {
    let (session, accepted, scope) = opened_session();
    let mut engine = CoreEngine::new();
    let _ = engine.handle(CoreRequest::Open {
        id: "seed-session-core".into(),
        document: (*accepted.document).clone(),
    });
    (
        Rc::new(RefCell::new(session)),
        Rc::new(RefCell::new(engine)),
        accepted,
        scope,
    )
}

fn start_export(session: &SharedSession, operation_id: OperationId, scope: Scope) -> SnapshotToken {
    session
        .borrow_mut()
        .submit(Event::StartExport {
            operation_id,
            scope,
        })
        .into_iter()
        .find_map(|effect| match effect {
            Effect::RunExport { snapshot, .. } => Some(snapshot.token),
            _ => None,
        })
        .expect("Session starts the accepted export owner")
}

fn resolve_plan(
    engine: &SharedEngine,
    accepted: &AcceptedSnapshot,
    scope: &Scope,
) -> ElectricalPlan {
    match engine.borrow_mut().handle(CoreRequest::ResolveElectrical {
        id: "pcb-handoff-test-resolve".into(),
        request: ElectricalPlanRequest {
            document: (*accepted.document).clone(),
            instance_id: None,
            mode: ElectricalMode::Matrix,
            locks: Default::default(),
            controller_profile: None,
            board_id: Some(scope.board_id.clone()),
            controller_part_id: None,
        },
    }) {
        CoreReply::ElectricalResolved { plan, .. } => plan,
        other => panic!("expected a resolved electrical plan, got {other:?}"),
    }
}

async fn commit_protection(
    session: &SharedSession,
    engine: &SharedEngine,
    export_operation_id: OperationId,
    token: SnapshotToken,
    scope: &Scope,
    plan: ElectricalPlan,
) -> Result<SnapshotToken, String> {
    let effects = session.borrow_mut().submit(Event::ExportCommit {
        operation_id: OperationId(export_operation_id.0 + 1_000),
        export_operation_id,
        token,
        scope: scope.clone(),
        commit: ExportCommitRequest::ProtectElectricalHandoff { plan },
    });
    settle_session(session, engine, effects);
    let token = session
        .borrow()
        .read_model()
        .accepted
        .as_ref()
        .map(|accepted| accepted.token)
        .ok_or_else(|| "Session lost its accepted snapshot".to_owned())?;
    if !has_protection(session, scope) {
        return Err("Session did not accept PCB handoff protection".into());
    }
    Ok(token)
}

fn settle_session(session: &SharedSession, engine: &SharedEngine, effects: Vec<Effect>) {
    let mut pending = VecDeque::from(effects);
    while let Some(effect) = pending.pop_front() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => {
                let reply = engine.borrow_mut().handle(*request);
                pending.extend(session.borrow_mut().complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(reply),
                }));
            }
            Effect::Persist {
                save_attempt_id, ..
            } => {
                pending.extend(session.borrow_mut().complete(Completion::Persist {
                    save_attempt_id,
                    result: boardstudio_application::SaveResult::Committed,
                }));
            }
            _ => {}
        }
    }
}

fn require_current(
    session: &SharedSession,
    operation_id: OperationId,
    token: SnapshotToken,
    scope: &Scope,
) -> Result<(), String> {
    if session
        .borrow()
        .export_is_current(operation_id, token, scope)
    {
        Ok(())
    } else {
        Err("KiCad export was superseded before wiring protection.".into())
    }
}

fn reopen_current_document(session: &SharedSession, engine: &SharedEngine) {
    let document: ProjectDoc = session
        .borrow()
        .read_model()
        .accepted
        .as_ref()
        .expect("accepted project before reopen")
        .document
        .as_ref()
        .clone();
    let effects = session.borrow_mut().submit(Event::Open {
        operation_id: OperationId(999),
        document,
    });
    settle_session(session, engine, effects);
}

fn has_protection(session: &SharedSession, scope: &Scope) -> bool {
    session
        .borrow()
        .read_model()
        .accepted
        .as_ref()
        .and_then(|accepted| accepted.document.hardware.as_ref())
        .is_some_and(|hardware| {
            hardware
                .boards
                .iter()
                .any(|board| board.board_id == scope.board_id && board.protected_handoff.is_some())
        })
}
