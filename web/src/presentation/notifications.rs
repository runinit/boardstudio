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
    rsx! {
        UpdateAvailableNotice {}
        RuntimeReportBanner {}
    }
}

/// One persistent, dismissible "Update Available" notice for a real waiting
/// build. This is the narrow, documented exception to the transient-result
/// Toast pattern: it stays past the normal five-second expiry until dismissed
/// or adopted, never steals focus, and derives from the live registration on
/// every page load, so dismissal stores nothing and the notice returns on the
/// next load. It carries no action and no header notice; the working
/// "Update Now" action and the persistent header notice arrive with 019.
/// Reduced motion needs no handling here: the shared Toast renders no
/// animation, so the notice is fully usable without motion.
#[component]
fn UpdateAvailableNotice() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let toasts = use_toast();
    let mut shown = use_signal(|| false);
    use_effect(move || {
        let _ = version();
        if runtime.update_waiting() && !shown() {
            shown.set(true);
            toasts.info(
                "Update Available".to_string(),
                ToastOptions::new().permanent(true),
            );
        }
    });
    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_host::host::{UpdateSignals, UpdateWatch};
    use boardstudio_web_host::offline::UpdateSnapshot;
    use boardstudio_web_runtime::runtime::firmware_export_test_support as support;
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
    }

    impl FakeUpdateSignals {
        fn new(snapshot: UpdateSnapshot) -> Self {
            Self {
                snapshot: Rc::new(RefCell::new(snapshot)),
                watchers: Rc::new(RefCell::new(Vec::new())),
                stopped: Rc::new(Cell::new(false)),
                dropped: Rc::new(Cell::new(false)),
            }
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
        use_hook(move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                let next = *version.peek() + 1;
                version.set(next);
            }));
        });
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
        by_role_name(&root, "button", "Open PCB workspace")
            .unwrap()
            .click();
        wait_until("the production control marks PCB current", || {
            by_role_name(&root, "button", "Open PCB workspace").is_some_and(|button| {
                button.get_attribute("aria-current").as_deref() == Some("page")
            })
        })
        .await;
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
}
