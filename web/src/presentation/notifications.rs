//! One shell consumer; workspace changes never remount the provider or drain a result twice.
use super::Runtime;
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

#[component]
pub(crate) fn RuntimeReportBanner() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let alert = runtime.status_is_alert();
    let status = runtime.status();
    rsx! {
        if !runtime.status_is_transient() && (alert || status != "Saved locally.") {
            p {
                role: if alert { "alert" } else { "status" },
                "aria-live": if alert { "assertive" } else { "polite" },
                class: "m-0 flex-none truncate border-t border-border bg-card px-3 py-[5px] text-xs text-muted-foreground",
                "{status}"
            }
        }
    }
}
