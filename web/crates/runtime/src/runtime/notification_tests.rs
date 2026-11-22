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
