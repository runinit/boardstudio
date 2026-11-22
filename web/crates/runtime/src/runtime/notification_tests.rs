use super::firmware_export_test_support as support;
use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn notification_queue_keeps_repeated_events_and_drains_once_without_notifying() {
    let (runtime, _, _) = support::fixture().await;
    runtime.take_notifications();
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    runtime.subscribe(Rc::new(move || observed.set(observed.get() + 1)));
    runtime.apply_report(RuntimeReport::status("Same result").transient());
    runtime.apply_report(RuntimeReport::status("Same result").transient());
    runtime.apply_report(RuntimeReport::alert("Action failed").transient());
    runtime.report("Saved locally.");
    assert!(
        !runtime.status_is_transient(),
        "classification never matches message text"
    );
    assert_eq!(changes.get(), 4);
    let pending = runtime.take_notifications();
    assert_eq!(
        pending
            .iter()
            .map(|n| (n.message.as_str(), n.is_error))
            .collect::<Vec<_>>(),
        vec![
            ("Same result", false),
            ("Same result", false),
            ("Action failed", true)
        ]
    );
    assert!(runtime.take_notifications().is_empty());
    assert_eq!(changes.get(), 4, "draining cannot schedule another render");
}

#[wasm_bindgen_test]
async fn export_delivery_errors_are_consumed_only_by_their_operation_and_cancel_cleans_them() {
    let (runtime, _, _) = support::fixture().await;
    runtime.take_notifications();
    let effects = support::start_export(&runtime);
    let operation = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { operation_id, .. } => Some(*operation_id),
            _ => None,
        })
        .unwrap();
    let unrelated = OperationId(1_000_001);
    runtime
        .export_delivery_errors
        .borrow_mut()
        .insert(operation, "browser refused download".into());
    runtime
        .run(Effect::Settled {
            operation_id: unrelated,
            outcome: TerminalOutcome::ExecutorFailed("unrelated executor".into()),
            landing: None,
        })
        .await;
    assert!(runtime.take_notifications().is_empty());
    assert!(
        runtime
            .export_delivery_errors
            .borrow()
            .contains_key(&operation)
    );
    runtime
        .run(Effect::Settled {
            operation_id: operation,
            outcome: TerminalOutcome::Completed,
            landing: None,
        })
        .await;
    let notifications = runtime.take_notifications();
    assert_eq!(
        notifications,
        vec![RuntimeNotification {
            message: "browser refused download".into(),
            is_error: true
        }]
    );
    assert!(runtime.export_delivery_errors.borrow().is_empty());
    runtime
        .export_delivery_errors
        .borrow_mut()
        .insert(unrelated, "cancelled delivery".into());
    runtime
        .run(Effect::CancelExport {
            operation_id: unrelated,
        })
        .await;
    assert!(runtime.export_delivery_errors.borrow().is_empty());
    runtime
        .run(Effect::Settled {
            operation_id: unrelated,
            outcome: TerminalOutcome::ExecutorFailed("late executor result".into()),
            landing: None,
        })
        .await;
    assert!(
        runtime.take_notifications().is_empty(),
        "a removed lease cannot turn a late executor failure into a Toast"
    );
    let stale_effects = support::start_export(&runtime);
    let stale_operation = stale_effects
        .iter()
        .find_map(|effect| match effect {
            Effect::RunExport { operation_id, .. } => Some(*operation_id),
            _ => None,
        })
        .unwrap();
    runtime
        .export_delivery_errors
        .borrow_mut()
        .insert(stale_operation, "obsolete delivery error".into());
    runtime.latest_firmware_export.set(Some(unrelated));
    runtime
        .run(Effect::Settled {
            operation_id: stale_operation,
            outcome: TerminalOutcome::Completed,
            landing: None,
        })
        .await;
    assert!(
        runtime.export_delivery_errors.borrow().is_empty(),
        "stale latest-owner gate still consumes its exact error"
    );
    assert!(
        runtime.take_notifications().is_empty(),
        "obsolete delivery errors cannot replay"
    );
}

#[wasm_bindgen_test]
async fn outline_delivery_notifies_once_and_completed_is_silent() {
    let (runtime, accepted, scope) = support::fixture().await;
    runtime.take_notifications();
    let operation = runtime.operation();
    runtime.begin_export_lease(
        operation,
        ExportKind::BoardOutline { dxf: false },
        &accepted,
        &scope,
    );
    runtime.artifacts.borrow_mut().insert(
        "outline-result".into(),
        Artifact {
            operation_id: operation,
            bytes: b"<svg/>".to_vec(),
            filename: "board.svg".into(),
            media_type: Some("image/svg+xml".into()),
            scope,
            token: accepted.token,
            board_outline: true,
        },
    );
    runtime
        .run(Effect::DeliverExport {
            operation_id: operation,
            artifact_id: "outline-result".into(),
            token: accepted.token,
        })
        .await;
    assert_eq!(runtime.status(), "Saved board.svg.");
    assert_eq!(
        runtime.take_notifications(),
        vec![RuntimeNotification {
            message: "Saved board.svg.".into(),
            is_error: false
        }]
    );
    runtime
        .run(Effect::Settled {
            operation_id: operation,
            outcome: TerminalOutcome::Completed,
            landing: None,
        })
        .await;
    assert!(
        runtime.take_notifications().is_empty(),
        "outline Completed cannot emit a second result"
    );
    let deliveries = support::take_deliveries(&runtime);
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].filename, "board.svg");
    assert_eq!(deliveries[0].bytes, b"<svg/>");
    assert!(runtime.export_delivery_errors.borrow().is_empty());
}
