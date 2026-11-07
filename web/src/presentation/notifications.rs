//! One shell consumer; workspace changes never remount the provider or drain a result twice.
use super::{Runtime, RuntimeReportBanner};
use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog, UpdateOpener};
use boardstudio_web_ui_shared::ui::alert_dialog::{
    AlertDialog, AlertDialogActions, AlertDialogCancel, AlertDialogDescription, AlertDialogTitle,
};
use boardstudio_web_ui_shared::ui::toast::{
    ToastActionSlot, ToastOptions, ToastProvider, use_toast,
};
use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// The one title of the persistent update notice. The shared Toast action slot
/// and the notice's own toast both read it, so they cannot drift apart.
const UPDATE_NOTICE_TITLE: &str = "Update Available";

#[component]
pub(super) fn NotificationProvider(children: Element) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let action_runtime = runtime.clone();
    let dismiss_runtime = runtime.clone();
    // Ticket 019: the update notice's working Update Now rides the shared Toast
    // action slot; dismissal hands the notice to the header bar. Transient
    // result toasts never match the slot title and keep their stock shape.
    let slot = ToastActionSlot {
        toast_title: UPDATE_NOTICE_TITLE.to_string(),
        action_label: "Update Now".to_string(),
        on_action: Callback::new(move |_| action_runtime.begin_update(UpdateOpener::Toast)),
        on_dismiss: Callback::new(move |_| dismiss_runtime.dismiss_update_notice()),
    };
    rsx! { ToastProvider { max_toasts: usize::MAX, class: "workbench-notifications", action_slot: slot, {children} } }
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
    rsx! {
        UpdateAvailableNotice {}
        UpdateWarningDialog {}
        RuntimeReportBanner {}
    }
}

/// One persistent, dismissible "Update Available" notice for a real waiting
/// build. This is the narrow, documented exception to the transient-result
/// Toast pattern: it stays past the normal five-second expiry until dismissed
/// or adopted, never steals focus, and derives from the live registration on
/// every page load, so dismissal stores nothing and the notice returns on the
/// next load. Its working "Update Now" action rides the shared Toast action
/// slot (see [`NotificationProvider`]); dismissal hands the notice to the
/// persistent header bar. Reduced motion needs no handling here: the shared
/// Toast renders no animation, so the notice is fully usable without motion.
#[component]
fn UpdateAvailableNotice() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let toasts = use_toast();
    let mut shown = use_signal(|| false);
    use_effect(move || {
        let _ = version();
        if runtime.update_waiting() {
            if !shown() {
                shown.set(true);
                toasts.info(
                    UPDATE_NOTICE_TITLE.to_string(),
                    ToastOptions::new().permanent(true),
                );
            }
        } else if shown() {
            // A cleared wait ends the notice cycle, so a later wait announces
            // itself again within the same page lifetime.
            shown.set(false);
        }
    });
    rsx! {}
}

/// The words for one deferral cause. Each names what the user must resolve;
/// the dialog adds the shared retry line below.
fn blocker_message(blocker: &UpdateBlocker) -> &'static str {
    match blocker {
        UpdateBlocker::PendingWork => {
            "Some edits are still being applied. Wait for them to land, then choose Update Now again."
        }
        UpdateBlocker::SaveInFlight => {
            "A save is still in progress. Wait for it to finish, then choose Update Now again."
        }
        UpdateBlocker::SaveFailed => {
            "The last save did not complete. Resolve the save problem first \u{2014} updating now could lose work."
        }
        UpdateBlocker::StorageNotReady => {
            "Browser storage is not ready, so saves cannot be verified yet. Wait a moment, then choose Update Now again."
        }
        UpdateBlocker::RecoveryRequired => {
            "This session needs recovery before it can update. Recover or reopen a saved keyboard first."
        }
        UpdateBlocker::InvalidDraft => {
            "A field holds an invalid value that is not saved. Fix or clear it, then choose Update Now again."
        }
    }
}

/// Focus the Update Now control that opened the warning, or the surviving one
/// when the opener is gone (the Toast may have been dismissed meanwhile).
fn focus_update_control(opener: UpdateOpener) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let header = || {
        document
            .query_selector(".header-update [aria-label='Update Now']")
            .ok()
            .flatten()
            .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
    };
    let toast = || {
        document
            .query_selector("[data-slot=toast-provider] [aria-label='Update Now']")
            .ok()
            .flatten()
            .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
    };
    let (first, second) = match opener {
        UpdateOpener::Toast => (toast(), header()),
        UpdateOpener::Header => (header(), toast()),
    };
    if let Some(target) = first.or(second) {
        let _ = target.focus();
    }
}

/// Ticket 019's cancel-only deferral warning. It names every detected reason
/// adoption must wait; closing it preserves the editor and all values, and the
/// update keeps waiting. Cancellation returns keyboard focus to the Update Now
/// control that opened the warning.
#[component]
fn UpdateWarningDialog() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let dialog = runtime.update_dialog();
    let was_open = use_hook(|| Rc::new(Cell::new(false)));
    let effect_runtime = runtime.clone();
    use_effect(move || {
        // The version read is the effect's reactive subscription: without a
        // signal read the effect runs once after mount and never observes the
        // open->close transition, so cancellation would leave focus on body.
        // The effect closure is captured at first render, so it reads the live
        // dialog and opener from the runtime instead of render-time values.
        let _ = version();
        let open = effect_runtime.update_dialog().is_some();
        let was = was_open.replace(open);
        if was && !open {
            focus_update_control(effect_runtime.update_opener());
        }
    });
    let Some(dialog) = dialog else {
        return rsx! {};
    };
    let cancel_runtime = runtime.clone();
    let close_runtime = runtime.clone();
    let (title, body) = match dialog {
        UpdateDialog::Deferred { blockers } => {
            let mut lines: Vec<&str> = blockers.iter().map(blocker_message).collect();
            lines.push("The update keeps waiting until you resolve this and choose Update Now again.");
            ("Update deferred", lines.join(" "))
        }
        UpdateDialog::OtherTabs => (
            "Close other tabs to update",
            "Please close any other open tabs, then choose Update Now again. Other tabs are never closed or reloaded for you."
                .to_string(),
        ),
    };
    rsx! {
        AlertDialog { default_open: true, on_open_change: move |open: bool| if !open { close_runtime.cancel_update_dialog(); },
            AlertDialogTitle { "{title}" }
            AlertDialogDescription { "{body}" }
            AlertDialogActions {
                AlertDialogCancel { on_click: move |_| cancel_runtime.cancel_update_dialog(), "Cancel" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::Durability;
    use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, ProjectDoc};
    use boardstudio_web_host::host::{UpdateSignals, UpdateWatch};
    use boardstudio_web_host::offline::{AdoptError, UpdateSnapshot};
    use boardstudio_web_runtime::runtime::firmware_export_test_support as support;
    use boardstudio_web_runtime::runtime::project_name_test_support as pnsupport;
    use boardstudio_web_ui_shared::test_role::{all_by_role_name, by_role_name};
    use boardstudio_web_ui_shared::test_wait::{wait_for, wait_until};
    use gloo_timers::future::TimeoutFuture;
    use std::cell::{Cell, RefCell};
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

    /// Scripted registration signals for the mounted update seam. Production
    /// uses the real [`ServiceWorkerRegistration`](web_sys::ServiceWorkerRegistration)
    /// implementation; the packaged A-then-B scenario proves that end.
    /// Stopped watchers are detached from the list, mirroring how the
    /// production guard removes its event listeners: a replaced listener can
    /// no longer emit.
    type Watchers = Rc<RefCell<Vec<Option<Rc<dyn Fn()>>>>>;

    #[derive(Clone)]
    struct FakeUpdateSignals {
        snapshot: Rc<RefCell<UpdateSnapshot>>,
        watchers: Watchers,
        stopped: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
        adoptions: Rc<Cell<u32>>,
        reloads: Rc<Cell<u32>>,
        /// While set, the scripted takeover stays in flight, like the real
        /// worker handshake does for up to ten seconds.
        hold_takeover: Rc<Cell<bool>>,
        adopt_result: Rc<RefCell<Result<(), AdoptError>>>,
    }

    impl FakeUpdateSignals {
        fn new(snapshot: UpdateSnapshot) -> Self {
            Self {
                snapshot: Rc::new(RefCell::new(snapshot)),
                watchers: Rc::new(RefCell::new(Vec::new())),
                stopped: Rc::new(Cell::new(false)),
                dropped: Rc::new(Cell::new(false)),
                adoptions: Rc::new(Cell::new(0)),
                reloads: Rc::new(Cell::new(0)),
                hold_takeover: Rc::new(Cell::new(false)),
                adopt_result: Rc::new(RefCell::new(Ok(()))),
            }
        }

        /// The worker declines because another editor is open.
        fn with_other_tabs(snapshot: UpdateSnapshot) -> Self {
            let signals = Self::new(snapshot);
            *signals.adopt_result.borrow_mut() = Err(AdoptError::OtherTabs);
            signals
        }

        fn fail_adoption(&self, reason: &str) {
            *self.adopt_result.borrow_mut() = Err(AdoptError::Failed(reason.to_string()));
        }

        fn set(&self, snapshot: UpdateSnapshot) {
            *self.snapshot.borrow_mut() = snapshot;
            for watcher in self.watchers.borrow().iter().flatten() {
                watcher();
            }
        }

        fn listener_count(&self) -> usize {
            self.watchers.borrow().iter().flatten().count()
        }
    }

    struct FakeUpdateWatch {
        signals: Watchers,
        index: usize,
        stopped: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
    }

    impl UpdateWatch for FakeUpdateWatch {
        fn stop(&mut self) {
            // Detach this listener so it can no longer emit, then record the
            // explicit stop separately from the eventual drop.
            if let Some(slot) = self.signals.borrow_mut().get_mut(self.index) {
                slot.take();
            }
            self.stopped.set(true);
        }
    }

    impl Drop for FakeUpdateWatch {
        fn drop(&mut self) {
            // Detach like the production guard's listener removal, then
            // record the drop: after owner drop no listener may remain.
            if let Some(slot) = self.signals.borrow_mut().get_mut(self.index) {
                slot.take();
            }
            self.dropped.set(true);
        }
    }

    impl UpdateSignals for FakeUpdateSignals {
        fn snapshot(&self) -> UpdateSnapshot {
            *self.snapshot.borrow()
        }

        fn watch(&self, on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
            let mut watchers = self.watchers.borrow_mut();
            watchers.push(Some(on_change));
            Box::new(FakeUpdateWatch {
                signals: self.watchers.clone(),
                index: watchers.len() - 1,
                stopped: self.stopped.clone(),
                dropped: self.dropped.clone(),
            })
        }

        /// Scripted takeover: records the call, optionally stays in flight,
        /// then answers `adopt_result`. Never touches the network or the
        /// worker; the packaged scenario proves the real worker handshake.
        fn adopt_waiting(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>> {
            let adoptions = self.adoptions.clone();
            let hold = self.hold_takeover.clone();
            let result = self.adopt_result.clone();
            Box::pin(async move {
                adoptions.set(adoptions.get() + 1);
                while hold.get() {
                    TimeoutFuture::new(10).await;
                }
                result.borrow().clone()
            })
        }

        /// Records the reload instead of leaving the test page.
        fn reload(&self) {
            self.reloads.set(self.reloads.get() + 1);
        }
    }

    fn update_root(
        runtime: &Rc<Runtime>,
        make: fn() -> Element,
    ) -> (web_sys::Element, Rc<Cell<Option<Signal<bool>>>>) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(make);
        dom.provide_root_context(runtime.clone());
        let visibility = Rc::new(Cell::new(None::<Signal<bool>>));
        dom.provide_root_context(visibility.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        (root, visibility)
    }

    /// The real production shell: `Header` (which owns the workspace
    /// navigation in production) beside the real notification bridge. Only
    /// the state atoms the shell itself owns (version, workspace, preference
    /// signals) are provided; every rendered control is production code.
    /// Mounting the full `WorkbenchApp` is not possible here: it creates its
    /// own `Runtime` and production wires observation inside `Runtime::new`
    /// behind cfg gates, so scripted registration signals could never reach
    /// that runtime. `Header` reads no contexts; its `ThemePicker` slot is
    /// fed by the same preference signals production provides.
    fn production_shell_fixture() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0u64);
        let workspace = use_signal(|| "Layout");
        let settings_open = use_signal(|| false);
        let palette_open = use_signal(|| false);
        use_context_provider(|| version);
        let preference_warning = use_signal(|| false);
        use_context_provider(|| super::super::PreferenceStorageWarning(preference_warning));
        let appearance = use_signal(|| "light");
        use_context_provider(|| super::super::AppearanceState(appearance));
        let theme = use_signal(|| "original");
        use_context_provider(|| super::super::ThemeState(theme));
        let subscriber = runtime.clone();
        use_hook(move || {
            subscriber.subscribe(Rc::new(move || {
                let mut version = version;
                let next = *version.peek() + 1;
                version.set(next);
            }));
        });
        // Like the production shell, re-render when the runtime changes so the
        // header notice follows dismissal.
        let _ = version();
        rsx! {
            NotificationProvider { RuntimeNotifications {} }
            super::super::header::Header {
                project_menu: rsx! {},
                workspace,
                show_workspaces: true,
                has_project: true,
                save_state: "saved",
                settings_open,
                palette_open,
                on_search: EventHandler::new(move |_| {}),
                on_undo: EventHandler::new(move |_| {}),
                on_redo: EventHandler::new(move |_| {}),
                on_export: EventHandler::new(move |_| {}),
                panel_controls: rsx! {},
                update_available: runtime.update_waiting() && runtime.update_notice_dismissed(),
                on_update: {
                    let runtime = runtime.clone();
                    EventHandler::new(move |_| runtime.begin_update(UpdateOpener::Header))
                },
            }
        }
    }

    fn location_href() -> String {
        web_sys::window()
            .unwrap()
            .location()
            .href()
            .unwrap_or_default()
    }

    #[wasm_bindgen_test]
    async fn mounted_waiting_update_is_one_persistent_dismissible_notice() {
        let (runtime, _, _) = support::fixture().await;
        runtime.take_notifications();
        let signals = FakeUpdateSignals::new(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        });
        let (root, _) = update_root(&runtime, fixture);
        let document = web_sys::window().unwrap().document().unwrap();
        let focused_before = document.active_element();
        let href_before = location_href();
        runtime.observe_update_waiting(Rc::new(signals));
        wait_until("the waiting update notice mounts", || {
            by_role_name(&root, "status", "Update Available").is_some()
        })
        .await;
        let notice = by_role_name(&root, "status", "Update Available").unwrap();
        assert_eq!(
            notice.get_attribute("aria-atomic").as_deref(),
            Some("true"),
            "the notice is one atomic announcement"
        );
        // Reduced-motion behavior is proven in the packaged browser run by
        // emulating `prefers-reduced-motion` and observing the rendered
        // notice (computed style carries no animation/transition and the
        // control stays fully operable); no motion-dependent code path exists
        // to unit-test here.
        assert_eq!(
            document.active_element(),
            focused_before,
            "the notice must not steal focus"
        );
        let close = by_role_name(&root, "button", "close").unwrap();
        assert_eq!(
            close.tab_index(),
            0,
            "dismissal stays keyboard-reachable through the shared Toast control"
        );
        runtime.observe_update_waiting(Rc::new(FakeUpdateSignals::new(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        })));
        runtime.report("A transient result still expires on its own");
        wait_until(
            "the transient result mounts beside the update notice",
            || {
                by_role_name(
                    &root,
                    "status",
                    "A transient result still expires on its own",
                )
                .is_some()
            },
        )
        .await;
        assert_eq!(
            all_by_role_name(&root, "status", "Update Available").len(),
            1,
            "repeated observations and rerenders never duplicate the notice"
        );
        TimeoutFuture::new(6_000).await;
        let leftovers = all_by_role_name(
            &root,
            "status",
            "A transient result still expires on its own",
        );
        assert!(
            !leftovers.is_empty()
                && leftovers
                    .iter()
                    .all(|toast| !toast.has_attribute("data-type")),
            "the transient result Toast expires while the inline persistent report stays"
        );
        assert!(
            by_role_name(&root, "status", "Update Available").is_some(),
            "the update notice outlives the normal five-second Toast expiry"
        );
        close.focus().unwrap();
        assert!(
            close.matches(":focus").unwrap(),
            "the shared Toast close control takes keyboard focus"
        );
        close.click();
        wait_until("dismissal removes the update notice", || {
            by_role_name(&root, "status", "Update Available").is_none()
        })
        .await;
        assert_eq!(
            location_href(),
            href_before,
            "dismissal must not reload the editor"
        );
        assert!(root.is_connected(), "dismissal keeps the editor mounted");
        assert!(
            runtime.update_waiting(),
            "dismissal stores nothing; the waiting state still derives the notice on the next load"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_installation_finishing_during_editing_raises_the_notice() {
        let (runtime, _, _) = support::fixture().await;
        runtime.take_notifications();
        let signals = FakeUpdateSignals::new(UpdateSnapshot {
            waiting: false,
            installing: true,
            controlled: true,
        });
        let (root, _) = update_root(&runtime, fixture);
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        TimeoutFuture::new(1_000).await;
        assert!(
            by_role_name(&root, "status", "Update Available").is_none(),
            "an installing worker is not a waiting update"
        );
        signals.set(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        });
        wait_until("finished installation raises the notice", || {
            by_role_name(&root, "status", "Update Available").is_some()
        })
        .await;
        assert_eq!(
            all_by_role_name(&root, "status", "Update Available").len(),
            1
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_initial_install_failure_and_idle_never_notify() {
        for (name, first, second) in [
            (
                "the initial install is not an update",
                UpdateSnapshot {
                    waiting: true,
                    installing: false,
                    controlled: false,
                },
                None,
            ),
            (
                "a failed installation is not an update",
                UpdateSnapshot {
                    waiting: false,
                    installing: true,
                    controlled: true,
                },
                Some(UpdateSnapshot {
                    waiting: false,
                    installing: false,
                    controlled: true,
                }),
            ),
            (
                "no update means no notice",
                UpdateSnapshot {
                    waiting: false,
                    installing: false,
                    controlled: true,
                },
                None,
            ),
        ] {
            let (runtime, _, _) = support::fixture().await;
            runtime.take_notifications();
            let signals = FakeUpdateSignals::new(first);
            let (root, _) = update_root(&runtime, fixture);
            runtime.observe_update_waiting(Rc::new(signals.clone()));
            if let Some(next) = second {
                signals.set(next);
            }
            TimeoutFuture::new(500).await;
            assert!(
                by_role_name(&root, "status", "Update Available").is_none(),
                "{name}"
            );
            root.remove();
        }
    }

    /// Switch to PCB through the production control. Below 1250px the header
    /// folds its commands into the "Workbench menu" and closes it after
    /// navigation (headless Chrome is 780px wide), so open it as needed.
    async fn switch_to_pcb(root: &web_sys::Element) {
        let pcb = || by_role_name(root, "button", "Open PCB workspace");
        let menu = || by_role_name(root, "button", "Workbench menu");
        if pcb().is_none() {
            menu().expect("a compact header offers its menu").click();
            wait_until("the workbench menu lists PCB", || pcb().is_some()).await;
        }
        pcb().unwrap().click();
        if let Some(menu) = menu() {
            wait_until("the menu closes after navigation", || pcb().is_none()).await;
            menu.click();
            wait_until("the reopened menu lists PCB", || pcb().is_some()).await;
        }
        wait_until("the production control marks PCB current", || {
            pcb().is_some_and(|button| {
                button.get_attribute("aria-current").as_deref() == Some("page")
            })
        })
        .await;
    }

    #[wasm_bindgen_test]
    async fn mounted_production_workspace_switch_keeps_one_notice() {
        let (runtime, _, _) = support::fixture().await;
        runtime.take_notifications();
        let signals = FakeUpdateSignals::new(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        });
        let (root, _) = update_root(&runtime, production_shell_fixture);
        runtime.observe_update_waiting(Rc::new(signals));
        wait_until("the waiting update notice mounts", || {
            by_role_name(&root, "status", "Update Available").is_some()
        })
        .await;
        switch_to_pcb(&root).await;
        TimeoutFuture::new(500).await;
        assert_eq!(
            all_by_role_name(&root, "status", "Update Available").len(),
            1,
            "switching workspaces through the production control does not duplicate the notice"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_bridge_unmount_remount_shows_a_single_notice() {
        let (runtime, _, _) = support::fixture().await;
        runtime.take_notifications();
        let signals = FakeUpdateSignals::new(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        });
        let (root, visibility) = update_root(&runtime, fixture);
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        wait_until("the waiting update notice mounts", || {
            by_role_name(&root, "status", "Update Available").is_some()
        })
        .await;
        let mut visible = visibility.get().expect("the fixture shares its visibility");
        visible.set(false);
        wait_until(
            "the production bridge unmounts with the notice live",
            || by_role_name(&root, "status", "Notifications unmounted").is_some(),
        )
        .await;
        assert!(
            by_role_name(&root, "status", "Update Available").is_none(),
            "unmounting the bridge removes the live notice"
        );
        signals.set(UpdateSnapshot {
            waiting: false,
            installing: false,
            controlled: true,
        });
        signals.set(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        });
        visible.set(true);
        wait_until("the remounted bridge shows the waiting notice", || {
            by_role_name(&root, "status", "Update Available").is_some()
        })
        .await;
        TimeoutFuture::new(500).await;
        assert_eq!(
            all_by_role_name(&root, "status", "Update Available").len(),
            1,
            "remounting the production bridge shows exactly one notice"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn update_observation_replaces_listeners_without_stacking() {
        let (runtime, _, _) = support::fixture().await;
        let idle = UpdateSnapshot {
            waiting: false,
            installing: false,
            controlled: true,
        };
        let waiting = UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        };
        let first = FakeUpdateSignals::new(idle);
        let second = FakeUpdateSignals::new(idle);
        runtime.observe_update_waiting(Rc::new(first.clone()));
        assert!(!first.stopped.get());
        assert_eq!(
            first.listener_count(),
            1,
            "observation installs exactly one listener"
        );
        first.set(waiting);
        assert!(
            runtime.update_waiting(),
            "the first listener drives the waiting state while attached"
        );
        runtime.observe_update_waiting(Rc::new(second.clone()));
        assert!(
            !runtime.update_waiting(),
            "the replacement observation re-applies its own snapshot"
        );
        assert!(
            first.stopped.get(),
            "replacing the observer must explicitly stop the previous listeners"
        );
        assert!(first.dropped.get(), "the replaced guard is released");
        assert_eq!(
            first.listener_count(),
            0,
            "the replaced signals keep no listeners behind"
        );
        first.set(waiting);
        assert!(
            !runtime.update_waiting(),
            "the stopped listener no longer emits: replacing never stacks observations"
        );
        second.set(waiting);
        assert!(
            runtime.update_waiting(),
            "the live listener still drives the waiting state"
        );
        assert!(!second.stopped.get());
        assert!(!second.dropped.get());
        let released = second.dropped.clone();
        drop(runtime);
        assert!(
            released.get(),
            "dropping the owning runtime releases its observer"
        );
        assert_eq!(
            second.listener_count(),
            0,
            "no listeners survive the owner drop, so nothing can emit afterwards"
        );
    }

    // ---- Ticket 019: protected Refresh through the production bridge. ----

    fn waiting_update() -> UpdateSnapshot {
        UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        }
    }

    fn idle_update() -> UpdateSnapshot {
        UpdateSnapshot {
            waiting: false,
            installing: false,
            controlled: true,
        }
    }

    fn update_doc() -> ProjectDoc {
        ProjectDoc::empty("update-test", "Update test")
    }

    async fn opened_update_runtime() -> Rc<Runtime> {
        let runtime = pnsupport::new_runtime();
        pnsupport::open_document(&runtime, update_doc()).await;
        runtime.take_notifications();
        runtime
    }

    fn rename_command(document: &ProjectDoc, name: &str) -> EditCommand {
        let mut renamed = document.clone();
        renamed.name = name.into();
        EditCommand {
            base_revision: document.revision,
            transaction_id: format!("update-probe-{name}"),
            phase: EditPhase::Commit,
            target_ids: vec![document.id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(renamed),
            },
        }
    }

    fn accepted_document(runtime: &Rc<Runtime>) -> ProjectDoc {
        (*runtime
            .model()
            .accepted
            .expect("an accepted document is open")
            .document)
            .clone()
    }

    fn dialog_text(root: &web_sys::Element) -> Option<String> {
        root.query_selector("[role=alertdialog]")
            .ok()
            .flatten()
            .and_then(|dialog| dialog.text_content())
    }

    async fn shown_update_button(root: &web_sys::Element) -> web_sys::HtmlElement {
        wait_until("the Update Now control mounts", || {
            by_role_name(root, "button", "Update Now").is_some()
        })
        .await;
        by_role_name(root, "button", "Update Now").unwrap()
    }

    async fn shown_cancel_button(root: &web_sys::Element) -> web_sys::HtmlElement {
        wait_until("the deferral warning mounts", || {
            by_role_name(root, "button", "Cancel").is_some()
        })
        .await;
        by_role_name(root, "button", "Cancel").unwrap()
    }

    #[wasm_bindgen_test]
    async fn mounted_update_now_warns_and_cancels_while_an_edit_is_pending() {
        let runtime = opened_update_runtime().await;
        let document = accepted_document(&runtime);
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        let update = shown_update_button(&root).await;
        assert_eq!(update.tab_index(), 0);
        // Hold the next Core reply and submit a rename: the edit is pending.
        let (entered, release) = pnsupport::gate_next_core_reply(&runtime);
        pnsupport::submit_fixed_command(&runtime, rename_command(&document, "Renamed"));
        pnsupport::drive_pending(&runtime);
        entered.await.unwrap();
        let href_before = location_href();
        update.click();
        let cancel = shown_cancel_button(&root).await;
        let text = dialog_text(&root).unwrap();
        assert!(
            text.contains("still being applied"),
            "the warning names the pending edit: {text:?}"
        );
        assert_eq!(signals.adoptions.get(), 0, "no adoption while blocked");
        assert_eq!(cancel.tab_index(), 0, "cancellation is a keyboard tab stop");
        cancel.click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        assert_eq!(location_href(), href_before, "cancellation never reloads");
        assert!(root.is_connected(), "cancellation keeps the editor mounted");
        assert_eq!(
            accepted_document(&runtime).name,
            "Update test",
            "cancellation preserves the unlanded work instead of discarding it"
        );
        assert!(
            by_role_name(&root, "button", "Update Now")
                .unwrap()
                .matches(":focus")
                .unwrap(),
            "cancellation returns focus to Update Now"
        );
        // The preserved edit still lands and saves afterwards.
        release.send(()).unwrap();
        pnsupport::run_pending(&runtime).await;
        assert_eq!(accepted_document(&runtime).name, "Renamed");
        assert_eq!(
            runtime.model().durability,
            Durability::Saved { revision: 1 }
        );
        // Settled work adopts exactly once despite repeated activation.
        let update = by_role_name(&root, "button", "Update Now").unwrap();
        update.click();
        update.click();
        wait_until("the settled update adopts", || signals.adoptions.get() == 1).await;
        TimeoutFuture::new(500).await;
        assert_eq!(signals.adoptions.get(), 1, "no duplicate adoption");
        assert_eq!(signals.reloads.get(), 1, "one reload after one adoption");
        root.remove();
    }

    /// An invalid draft that its panel refuses to commit never reaches the
    /// engine, so no engine owner sees it. The gate reads the field's own
    /// `aria-invalid` state and defers, naming the cause. Removing the field
    /// (the user fixed or cleared it) lets the update proceed.
    #[wasm_bindgen_test]
    async fn mounted_update_now_defers_while_an_invalid_draft_is_visible() {
        let runtime = opened_update_runtime().await;
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let field = document.create_element("input").unwrap();
        field.set_attribute("aria-invalid", "true").unwrap();
        field.set_attribute("aria-label", "Rows draft").unwrap();
        document.body().unwrap().append_child(&field).unwrap();
        by_role_name(&root, "button", "Update Now").unwrap().click();
        shown_cancel_button(&root).await;
        let text = dialog_text(&root).unwrap();
        assert!(
            text.contains("invalid value"),
            "the warning names the invalid draft: {text:?}"
        );
        assert_eq!(signals.adoptions.get(), 0, "an invalid draft never adopts");
        by_role_name(&root, "button", "Cancel").unwrap().click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        assert!(
            field.is_connected() && field.get_attribute("aria-invalid").is_some(),
            "cancellation preserves the draft field"
        );
        field.remove();
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the resolved update adopts", || {
            signals.adoptions.get() == 1
        })
        .await;
        root.remove();
    }

    fn space_keydown_is_consumed(target: &web_sys::HtmlElement) -> bool {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(" ");
        init.set_code("Space");
        init.set_bubbles(true);
        init.set_cancelable(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
        target.dispatch_event(&event).unwrap();
        event.default_prevented()
    }

    /// The Layout workspace's window Space-to-pan listener cancels native Space
    /// activation of every non-text control it sees. The update controls (header
    /// action and the warning's Cancel) must keep Space; found by the packaged
    /// keyboard run, where Space on Cancel closed nothing.
    #[wasm_bindgen_test]
    async fn mounted_update_controls_keep_space_under_the_layout_pan_listener() {
        let space_down = Rc::new(Cell::new(false));
        let listener =
            boardstudio_web_ui_model::canvas_interaction::LayoutSpacePanWindowListener::install(
                Rc::new(|| true),
                space_down.clone(),
            )
            .expect("the production pan listener installs");
        let runtime = opened_update_runtime().await;
        let (root, _) = update_root(&runtime, production_shell_fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        by_role_name(&root, "button", "close").unwrap().click();
        wait_until("dismissal hands the notice to the header", || {
            root.query_selector(".header-update").unwrap().is_some()
        })
        .await;
        let header_update = shown_update_button(&root).await;
        assert!(
            !space_keydown_is_consumed(&header_update),
            "Space on the header Update Now stays available for activation"
        );
        let document = web_sys::window().unwrap().document().unwrap();
        let field = document.create_element("input").unwrap();
        field.set_attribute("aria-invalid", "true").unwrap();
        document.body().unwrap().append_child(&field).unwrap();
        header_update.click();
        let cancel = shown_cancel_button(&root).await;
        assert!(
            !space_keydown_is_consumed(&cancel),
            "Space on Cancel stays available for activation"
        );
        assert!(!space_down.get(), "update controls never start a pan");
        cancel.click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        field.remove();
        drop(listener);
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_update_now_defers_while_a_save_is_in_flight() {
        let runtime = opened_update_runtime().await;
        let document = accepted_document(&runtime);
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        let (persist_entered, persist_release) = pnsupport::gate_next_persist(&runtime);
        pnsupport::submit_fixed_command(&runtime, rename_command(&document, "Renamed"));
        pnsupport::drive_pending(&runtime);
        persist_entered.await.unwrap();
        assert!(matches!(
            runtime.model().durability,
            Durability::Saving { .. }
        ));
        by_role_name(&root, "button", "Update Now").unwrap().click();
        shown_cancel_button(&root).await;
        let text = dialog_text(&root).unwrap();
        assert!(
            text.contains("save is still in progress"),
            "the warning names the in-flight save: {text:?}"
        );
        assert_eq!(signals.adoptions.get(), 0, "an in-flight save never adopts");
        by_role_name(&root, "button", "Cancel").unwrap().click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        persist_release.send(()).unwrap();
        pnsupport::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().durability,
            Durability::Saved { revision: 1 }
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_update_now_defers_when_the_save_failed() {
        let runtime = opened_update_runtime().await;
        let document = accepted_document(&runtime);
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        pnsupport::fail_next_persist(&runtime, "quota exceeded");
        pnsupport::submit_fixed_command(&runtime, rename_command(&document, "Renamed"));
        pnsupport::run_pending(&runtime).await;
        assert!(matches!(
            runtime.model().durability,
            Durability::Failed { .. }
        ));
        let href_before = location_href();
        by_role_name(&root, "button", "Update Now").unwrap().click();
        shown_cancel_button(&root).await;
        let text = dialog_text(&root).unwrap();
        assert!(
            text.contains("did not complete"),
            "the warning names the failed save: {text:?}"
        );
        assert_eq!(signals.adoptions.get(), 0, "a failed save never adopts");
        assert!(
            text.contains("Update Now"),
            "the warning explains how to retry: {text:?}"
        );
        by_role_name(&root, "button", "Cancel").unwrap().click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        assert_eq!(location_href(), href_before, "cancellation never reloads");
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_update_now_defers_with_other_tabs_then_adopts_when_sole() {
        let runtime = opened_update_runtime().await;
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::with_other_tabs(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        by_role_name(&root, "button", "Update Now").unwrap().click();
        shown_cancel_button(&root).await;
        let text = dialog_text(&root).unwrap();
        assert!(
            text.contains("Please close any other open tabs"),
            "the warning tells the user what to do: {text:?}"
        );
        assert_eq!(signals.adoptions.get(), 1, "the worker was asked once");
        assert_eq!(
            signals.reloads.get(),
            0,
            "a declined adoption never reloads"
        );
        by_role_name(&root, "button", "Cancel").unwrap().click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        assert!(
            by_role_name(&root, "button", "Update Now")
                .unwrap()
                .matches(":focus")
                .unwrap(),
            "cancellation returns focus to Update Now"
        );
        *signals.adopt_result.borrow_mut() = Ok(());
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the sole-client update adopts and reloads", || {
            signals.adoptions.get() == 2 && signals.reloads.get() == 1
        })
        .await;
        root.remove();
    }

    /// Edits can begin while the worker takeover is in flight. The reload must
    /// recheck refresh safety and wait for the work to settle, never discard it.
    #[wasm_bindgen_test]
    async fn mounted_work_begun_during_the_takeover_delays_the_reload_until_it_settles() {
        let runtime = opened_update_runtime().await;
        let document = accepted_document(&runtime);
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        signals.hold_takeover.set(true);
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the takeover is in flight", || signals.adoptions.get() == 1).await;
        let (entered, release) = pnsupport::gate_next_core_reply(&runtime);
        pnsupport::submit_fixed_command(&runtime, rename_command(&document, "Renamed"));
        pnsupport::drive_pending(&runtime);
        entered.await.unwrap();
        signals.hold_takeover.set(false);
        wait_until("the page explains why it has not reloaded", || {
            runtime.status().contains("installed")
        })
        .await;
        TimeoutFuture::new(600).await;
        assert_eq!(
            signals.reloads.get(),
            0,
            "work begun during the takeover blocks the reload"
        );
        release.send(()).unwrap();
        pnsupport::run_pending(&runtime).await;
        wait_until("the settled page reloads onto the adopted build", || {
            signals.reloads.get() == 1
        })
        .await;
        assert_eq!(accepted_document(&runtime).name, "Renamed");
        assert_eq!(
            runtime.model().durability,
            Durability::Saved { revision: 1 },
            "the work landed and saved before the reload"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_failed_adoption_reports_and_allows_a_retry() {
        let runtime = opened_update_runtime().await;
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        signals.fail_adoption("the waiting build did not take over");
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the adoption failure is reported", || {
            runtime.status().contains("could not be adopted")
        })
        .await;
        assert_eq!(signals.adoptions.get(), 1);
        assert!(by_role_name(&root, "button", "Cancel").is_none());
        *signals.adopt_result.borrow_mut() = Ok(());
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the retry adopts", || signals.adoptions.get() == 2).await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_stale_waiting_clears_without_dialog_or_adoption() {
        let runtime = opened_update_runtime().await;
        let (root, _) = update_root(&runtime, fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        signals.set(idle_update());
        wait_until("the waiting state clears", || !runtime.update_waiting()).await;
        by_role_name(&root, "button", "Update Now").unwrap().click();
        TimeoutFuture::new(500).await;
        assert!(
            by_role_name(&root, "button", "Cancel").is_none(),
            "a stale notification never warns"
        );
        assert_eq!(signals.adoptions.get(), 0);
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_dismissal_moves_the_notice_to_the_header() {
        let runtime = opened_update_runtime().await;
        let document = accepted_document(&runtime);
        let (root, _) = update_root(&runtime, production_shell_fixture);
        let signals = FakeUpdateSignals::new(waiting_update());
        runtime.observe_update_waiting(Rc::new(signals.clone()));
        shown_update_button(&root).await;
        assert!(
            root.query_selector(".header-update").unwrap().is_none(),
            "the header stays quiet while the Toast shows the action"
        );
        by_role_name(&root, "button", "close").unwrap().click();
        wait_until("dismissal removes the Toast", || {
            root.query_selector("[data-slot=toast]").unwrap().is_none()
        })
        .await;
        let header_update = shown_update_button(&root).await;
        assert!(
            header_update
                .closest(".workbench-header")
                .unwrap()
                .is_some(),
            "the surviving action lives in the header bar"
        );
        switch_to_pcb(&root).await;
        assert_eq!(
            all_by_role_name(&root, "button", "Update Now").len(),
            1,
            "the workspace switch keeps exactly one header action"
        );
        // The header action warns like the Toast one and returns focus to itself.
        let (entered, release) = pnsupport::gate_next_core_reply(&runtime);
        pnsupport::submit_fixed_command(&runtime, rename_command(&document, "Renamed"));
        pnsupport::drive_pending(&runtime);
        entered.await.unwrap();
        by_role_name(&root, "button", "Update Now").unwrap().click();
        let cancel = shown_cancel_button(&root).await;
        assert_eq!(cancel.tab_index(), 0);
        cancel.click();
        wait_until("cancellation closes the warning", || {
            by_role_name(&root, "button", "Cancel").is_none()
        })
        .await;
        let header_update = by_role_name(&root, "button", "Update Now").unwrap();
        assert!(header_update.matches(":focus").unwrap());
        assert!(
            header_update
                .closest(".workbench-header")
                .unwrap()
                .is_some(),
            "cancellation returns focus to the header action"
        );
        release.send(()).unwrap();
        pnsupport::run_pending(&runtime).await;
        by_role_name(&root, "button", "Update Now").unwrap().click();
        wait_until("the header action adopts", || signals.adoptions.get() == 1).await;
        root.remove();
    }
}
