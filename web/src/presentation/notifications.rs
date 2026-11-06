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
    #[derive(Clone)]
    struct FakeUpdateSignals {
        snapshot: Rc<RefCell<UpdateSnapshot>>,
        watchers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        stopped: Rc<Cell<bool>>,
    }

    impl FakeUpdateSignals {
        fn new(snapshot: UpdateSnapshot) -> Self {
            Self {
                snapshot: Rc::new(RefCell::new(snapshot)),
                watchers: Rc::new(RefCell::new(Vec::new())),
                stopped: Rc::new(Cell::new(false)),
            }
        }

        fn set(&self, snapshot: UpdateSnapshot) {
            *self.snapshot.borrow_mut() = snapshot;
            for watcher in self.watchers.borrow().iter() {
                watcher();
            }
        }
    }

    struct FakeUpdateWatch {
        stopped: Rc<Cell<bool>>,
    }

    impl UpdateWatch for FakeUpdateWatch {
        fn stop(&mut self) {
            self.stopped.set(true);
        }
    }

    impl Drop for FakeUpdateWatch {
        fn drop(&mut self) {
            self.stopped.set(true);
        }
    }

    impl UpdateSignals for FakeUpdateSignals {
        fn snapshot(&self) -> UpdateSnapshot {
            *self.snapshot.borrow()
        }

        fn watch(&self, on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
            self.watchers.borrow_mut().push(on_change);
            Box::new(FakeUpdateWatch {
                stopped: self.stopped.clone(),
            })
        }
    }

    fn update_root(runtime: &Rc<Runtime>) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(fixture);
        dom.provide_root_context(runtime.clone());
        let visibility = Rc::new(Cell::new(None::<Signal<bool>>));
        dom.provide_root_context(visibility);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
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
        let root = update_root(&runtime);
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
        let root = update_root(&runtime);
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
            let root = update_root(&runtime);
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
    async fn update_observation_replaces_listeners_without_stacking() {
        let (runtime, _, _) = support::fixture().await;
        let idle = UpdateSnapshot {
            waiting: false,
            installing: false,
            controlled: true,
        };
        let first = FakeUpdateSignals::new(idle);
        let second = FakeUpdateSignals::new(idle);
        runtime.observe_update_waiting(Rc::new(first.clone()));
        assert!(!first.stopped.get());
        runtime.observe_update_waiting(Rc::new(second.clone()));
        assert!(
            first.stopped.get(),
            "replacing the observer must stop the previous listeners"
        );
        assert!(!second.stopped.get());
    }
}
