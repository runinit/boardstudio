//! One shell consumer; workspace changes never remount the provider or drain a result twice.
use super::{Runtime, RuntimeReportBanner};
use boardstudio_web_ui_shared::ui::toast::{ToastOptions, ToastProvider, use_toast};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub(super) fn NotificationProvider(children: Element) -> Element {
    rsx! { ToastProvider { max_toasts: usize::MAX, class: "workbench-notifications", {children} } }
}

#[component]
pub(super) fn RuntimeNotifications() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let toasts = use_toast();
    use_effect(move || {
        let _ = version();
        for notification in runtime.take_notifications() {
            if notification.is_error {
                toasts.error(notification.message, ToastOptions::new());
            } else {
                toasts.success(notification.message, ToastOptions::new());
            }
        }
    });
    rsx! { RuntimeReportBanner {} }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_runtime::runtime::firmware_export_test_support as support;
    use boardstudio_web_ui_shared::test_role::{all_by_role_name, by_role_name};
    use boardstudio_web_ui_shared::test_wait::{wait_for, wait_until};
    use std::cell::Cell;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn fixture() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0u64);
        let visible = use_signal(|| true);
        let visibility = use_context::<Rc<Cell<Option<Signal<bool>>>>>();
        use_hook(move || visibility.set(Some(visible)));
        use_context_provider(|| version);
        use_hook(move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                let next = *version.peek() + 1;
                version.set(next);
            }));
        });
        rsx! {
            if visible() { NotificationProvider { RuntimeNotifications {} } }
            else { p { role: "status", "Notifications unmounted" } }
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_bridge_keeps_batched_duplicate_results_once_and_persistent_reports_inline() {
        let (runtime, _, _) = support::fixture().await;
        runtime.take_notifications();
        for _ in 0..6 {
            support::fail(&runtime, support::Stage::Generation);
            let effects = support::start_export(&runtime);
            support::run_effects(&runtime, effects).await;
        }
        runtime.report("Persistent setup notice");
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(fixture);
        dom.provide_root_context(runtime.clone());
        let visibility = Rc::new(Cell::new(None::<Signal<bool>>));
        dom.provide_root_context(visibility.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        let failure = "Firmware generation failed: injected firmware-generation failure";
        wait_until("all twelve queued results mount together", || {
            all_by_role_name(&root, "status", failure).len() == 6
                && all_by_role_name(&root, "status", "Export started.").len() == 6
        })
        .await;
        for toast in all_by_role_name(&root, "status", failure) {
            assert_eq!(toast.get_attribute("data-type").as_deref(), Some("error"));
        }
        let notice = by_role_name(&root, "status", "Persistent setup notice").unwrap();
        assert!(
            !notice.has_attribute("data-type"),
            "the persistent report is outside Toasts"
        );
        assert!(runtime.take_notifications().is_empty());
        runtime.report("Updated persistent notice");
        wait_until("runtime version rerenders the inline report", || {
            by_role_name(&root, "status", "Updated persistent notice").is_some()
        })
        .await;
        assert_eq!(all_by_role_name(&root, "status", failure).len(), 6);
        assert_eq!(
            all_by_role_name(&root, "status", "Export started.").len(),
            6
        );
        assert!(
            runtime.take_notifications().is_empty(),
            "rerenders never replay drained results"
        );
        wait_for(
            "default five-second expiry removes all twelve results",
            8_000.0,
            || {
                all_by_role_name(&root, "status", failure).is_empty()
                    && all_by_role_name(&root, "status", "Export started.").is_empty()
            },
        )
        .await
        .unwrap();
        assert!(by_role_name(&root, "region", "0 notifications").is_some());
        assert!(by_role_name(&root, "status", "Updated persistent notice").is_some());
        support::fail(&runtime, support::Stage::Generation);
        let effects = support::start_export(&runtime);
        support::run_effects(&runtime, effects).await;
        wait_until("a new export has two live expiring Toasts", || {
            by_role_name(&root, "region", "2 notifications").is_some()
        })
        .await;
        let active = by_role_name(&root, "status", failure).unwrap();
        let mut visible = visibility.get().unwrap();
        visible.set(false);
        wait_until("the provider and active Toasts unmount normally", || {
            by_role_name(&root, "status", "Notifications unmounted").is_some()
        })
        .await;
        assert!(!active.is_connected());
        assert!(by_role_name(&root, "region", "2 notifications").is_none());
        visible.set(true);
        wait_until("a fresh provider has no replayed results", || {
            by_role_name(&root, "region", "0 notifications").is_some()
        })
        .await;
        assert!(all_by_role_name(&root, "status", failure).is_empty());
        assert!(runtime.take_notifications().is_empty());
        root.remove();
    }
}
