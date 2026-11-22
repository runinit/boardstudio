use super::*;
use crate::runtime::firmware_export_test_support as test_support;
use futures_channel::oneshot;
use std::task::Poll;
use wasm_bindgen_test::wasm_bindgen_test;

fn run_export_effect(effects: Vec<Effect>) -> (OperationId, Effect) {
    let exports = effects
        .into_iter()
        .filter(|effect| matches!(effect, Effect::RunExport { .. }))
        .collect::<Vec<_>>();
    assert_eq!(exports.len(), 1, "one click starts one export operation");
    let effect = exports.into_iter().next().unwrap();
    let Effect::RunExport { operation_id, .. } = &effect else {
        unreachable!()
    };
    (*operation_id, effect)
}

async fn wait_for_actual_stage(
    run: &mut Pin<Box<dyn Future<Output = Vec<Effect>> + '_>>,
    entered: oneshot::Receiver<()>,
) {
    let mut entered = Box::pin(entered);
    std::future::poll_fn(|context| {
        if let Poll::Ready(effects) = run.as_mut().poll(context) {
            panic!("production RunExport completed before the gated stage: {effects:?}");
        }
        match entered.as_mut().poll(context) {
            Poll::Ready(Ok(())) => Poll::Ready(()),
            Poll::Ready(Err(_)) => panic!("controlled stage notification was dropped"),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

#[wasm_bindgen_test]
async fn production_run_export_retries_generation_without_editing_session() {
    for (stage, expected) in [(
        test_support::Stage::Generation,
        "Firmware generation failed: injected firmware-generation failure",
    )] {
        let (runtime, accepted, scope) = test_support::fixture().await;
        test_support::fail(&runtime, stage);
        let before = runtime.session.borrow().read_model().clone();
        let (operation_id, run_effect) = run_export_effect(test_support::start_export(&runtime));
        assert!(
            matches!(&run_effect, Effect::RunExport { scope: emitted_scope, .. } if emitted_scope == &scope)
        );

        test_support::run_effects(&runtime, vec![run_effect]).await;

        assert_eq!(runtime.session.borrow().read_model(), &before);
        assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
        assert_eq!(runtime.scope().as_ref(), Some(&scope));
        assert_eq!(runtime.status(), expected);
        assert!(runtime.status_is_alert());
        assert!(runtime.artifacts.borrow().is_empty());
        assert!(test_support::take_deliveries(&runtime).is_empty());

        let retry_effects = test_support::start_export(&runtime);
        assert!(
            !runtime.status_is_alert(),
            "retry clears the prior firmware alert"
        );
        let (retry_operation, retry_effect) = run_export_effect(retry_effects);
        assert_ne!(retry_operation, operation_id);
        test_support::run_effects(&runtime, vec![retry_effect]).await;

        let deliveries = test_support::take_deliveries(&runtime);
        assert_eq!(
            deliveries.len(),
            1,
            "retry through the same Runtime action delivers once"
        );
        assert_eq!(deliveries[0].filename, "ZMK export test-zmk.zip");
        assert_eq!(deliveries[0].media_type.as_deref(), Some("application/zip"));
        assert_eq!(runtime.session.borrow().read_model(), &before);
        assert!(!runtime.status_is_alert());
    }
}

#[wasm_bindgen_test]
async fn production_completion_after_owner_switch_does_not_deliver_or_report() {
    let (runtime, accepted, _scope) = test_support::fixture().await;
    let (entered, release) = test_support::gate(&runtime, test_support::Stage::Generation);
    let (_, effect) = run_export_effect(test_support::start_export(&runtime));
    let mut run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> = Box::pin(runtime.run(effect));
    wait_for_actual_stage(&mut run, entered).await;

    let mut next_document = (*accepted.document).clone();
    next_document.id = "next-project".into();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: next_document,
    });
    project_name_test_support::run_pending(&runtime).await;
    let next_accepted = runtime.model().accepted.unwrap();
    let next_scope = runtime.scope().unwrap();
    runtime.take_notifications();
    release.send(()).expect("release old project generation");
    test_support::run_effects(&runtime, run.await).await;

    assert_eq!(runtime.model().accepted.as_ref(), Some(&next_accepted));
    assert_eq!(runtime.scope().as_ref(), Some(&next_scope));
    assert!(runtime.artifacts.borrow().is_empty());
    assert!(test_support::take_deliveries(&runtime).is_empty());
    assert!(
        runtime.take_notifications().is_empty(),
        "obsolete completion cannot Toast in the new project"
    );
    assert!(
        !runtime.status_is_alert(),
        "an old project failure must not be announced in the new project"
    );
}

#[wasm_bindgen_test]
async fn production_older_export_can_deliver_without_replacing_newer_report() {
    let (runtime, _accepted, _scope) = test_support::fixture().await;
    let (entered, release) = test_support::gate(&runtime, test_support::Stage::Generation);
    let (_, old_effect) = run_export_effect(test_support::start_export(&runtime));
    let mut old_run: Pin<Box<dyn Future<Output = Vec<Effect>> + '_>> =
        Box::pin(runtime.run(old_effect));
    wait_for_actual_stage(&mut old_run, entered).await;

    let (new_operation, new_effect) = run_export_effect(test_support::start_export(&runtime));
    assert!(new_operation.0 > 0);
    test_support::run_effects(&runtime, vec![new_effect]).await;
    let new_deliveries = test_support::take_deliveries(&runtime);
    assert_eq!(new_deliveries.len(), 1, "newest attempt delivers once");
    assert_eq!(runtime.status(), "Saved locally.");
    runtime.take_notifications();

    release.send(()).expect("release older generation");
    test_support::run_effects(&runtime, old_run.await).await;
    let older_deliveries = test_support::take_deliveries(&runtime);
    assert_eq!(
        older_deliveries.len(),
        1,
        "Session keeps a still-current concurrent export independently deliverable"
    );
    assert_eq!(older_deliveries[0].filename, "ZMK export test-zmk.zip");
    assert_eq!(runtime.status(), "Saved locally.");
    assert!(!runtime.status_is_alert());
    assert!(
        runtime.take_notifications().is_empty(),
        "older output cannot replay a terminal notification"
    );
}

#[wasm_bindgen_test]
fn firmware_failure_announcement_has_explicit_alert_severity_and_stale_owner_is_silent() {
    for reason in [
        "Firmware generation failed: worker rejected the plan.",
        "Firmware packaging failed: ZIP worker returned no bytes.",
    ] {
        let report =
            export_terminal_report(true, TerminalOutcome::ExecutorFailed(reason.into()), None)
                .expect("current firmware failure is announced");
        assert_eq!(report.message, reason);
        assert_eq!(report.severity, RuntimeReportSeverity::Alert);
    }

    assert!(
        export_terminal_report(
            false,
            TerminalOutcome::ExecutorFailed("late failure from old project".into()),
            None,
        )
        .is_none()
    );

    let mut report = RuntimeReport::status(
        "Firmware generation failed: same text is routine status without typed severity.",
    );
    assert_eq!(report.severity, RuntimeReportSeverity::Status);
    report = export_terminal_report(
        true,
        TerminalOutcome::Completed,
        Some("Browser refused the firmware download.".into()),
    )
    .expect("delivery failure is announced");
    assert_eq!(report.severity, RuntimeReportSeverity::Alert);
    report.clear_alert();
    assert_eq!(report.severity, RuntimeReportSeverity::Status);
    assert!(report.message.is_empty());
}
