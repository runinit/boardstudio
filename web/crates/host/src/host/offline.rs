use crate::offline::AdoptError;
use futures_channel::oneshot;
use js_sys::Array;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{RegistrationOptions, ServiceWorker, ServiceWorkerRegistration};

/// Registers the Rust-owned service worker for a root or slash-terminated deployment prefix.
/// The release build stages its generated worker at `{scope}service-worker.js`.
pub async fn register_offline(scope: &str) -> Result<ServiceWorkerRegistration, JsValue> {
    validate_scope(scope)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let container = window.navigator().service_worker();
    let script_url = format!("{scope}service-worker.js");
    let options = RegistrationOptions::new();
    options.set_scope(scope);
    options.set_type("module");
    let registration =
        JsFuture::from(container.register_with_options(&script_url, &options)).await?;
    registration.dyn_into::<ServiceWorkerRegistration>()
}

fn validate_scope(scope: &str) -> Result<(), JsValue> {
    if !scope.starts_with('/')
        || !scope.ends_with('/')
        || scope
            .chars()
            .any(|character| matches!(character, '?' | '#' | '\\'))
        || scope.split('/').any(|segment| segment == "..")
    {
        return Err(JsValue::from_str(
            "service worker scope must be a normalized same-origin slash-terminated path",
        ));
    }
    Ok(())
}

/// Point-in-time release lifecycle signals for one service worker registration.
///
/// The mounted seam (ticket 018) drives this trait with a scripted fake while
/// production uses the real [`ServiceWorkerRegistration`] implementation; the
/// packaged A-then-B scenario proves the real registration end to end.
/// Application state owns the waiting decision itself (see
/// [`crate::offline::is_update_waiting`]); this trait only surfaces signals.
/// Dropping the watch guard removes every installed listener.
pub trait UpdateSignals: 'static {
    /// Read the current waiting/installing/controlled signals.
    fn snapshot(&self) -> crate::offline::UpdateSnapshot;
    /// Call `on_change` whenever the registration or its installing worker
    /// changes state, so the caller can re-snapshot and re-apply the rule.
    /// Keeps no registry itself: the caller owns the returned guard and drops
    /// the previous one, so repeated observations never stack listeners.
    fn watch(&self, on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch>;
    /// Ask the waiting worker to take over and wait until this page's
    /// controller changes. The worker, not the page, decides whether adoption
    /// is safe for other open editors: it re-checks its own window clients at
    /// the moment it would skip waiting, so no editor can open between a
    /// page-side probe and activation. Resolves `Err(OtherTabs)` when the worker
    /// declines, and `Err(Failed)` when it never takes over. Never reloads:
    /// the caller rechecks refresh safety once the controller has changed and
    /// then calls [`UpdateSignals::reload`], so work that began during the
    /// takeover is never discarded.
    fn adopt_waiting(&self) -> Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>>;
    /// Reload this page onto the adopted build.
    fn reload(&self);
}

/// Removes the registration listeners installed by [`UpdateSignals::watch`].
/// Dropping the guard stops observation as well.
pub trait UpdateWatch {
    fn stop(&mut self);
}

/// Read the live signals of a real registration. A waiting worker counts only
/// together with a controller (see [`crate::offline::is_update_waiting`]): a
/// waiting worker on an uncontrolled page is the initial install, not an update.
fn snapshot_registration(
    registration: &ServiceWorkerRegistration,
) -> crate::offline::UpdateSnapshot {
    let controlled = web_sys::window()
        .map(|window| window.navigator().service_worker().controller().is_some())
        .unwrap_or(false);
    crate::offline::UpdateSnapshot {
        waiting: registration.waiting().is_some(),
        installing: registration.installing().is_some(),
        controlled,
    }
}

/// Shared slot for the currently tracked installing worker and its
/// `statechange` listener. The `updatefound` closure moves a clone so it can
/// re-point the slot without borrowing the guard that owns the closure.
type InstallingSlot =
    Rc<std::cell::RefCell<Option<(ServiceWorker, Closure<dyn FnMut(web_sys::Event)>)>>>;

struct RegistrationWatch {
    registration: ServiceWorkerRegistration,
    updatefound: Option<Closure<dyn FnMut(web_sys::Event)>>,
    installing: InstallingSlot,
}

/// Point the shared installing slot at the registration's current installing
/// worker, removing the previous worker's listener first. A `None` installing
/// worker (installed, redundant or idle) simply leaves the slot empty; the
/// caller re-snapshots and applies the waiting rule instead.
fn track_installing(
    registration: &ServiceWorkerRegistration,
    slot: &InstallingSlot,
    on_change: &Rc<dyn Fn()>,
) {
    if let Some((worker, statechange)) = slot.borrow_mut().take() {
        let _ = worker.remove_event_listener_with_callback(
            "statechange",
            statechange.as_ref().unchecked_ref(),
        );
    }
    if let Some(worker) = registration.installing() {
        let notify = on_change.clone();
        let statechange =
            Closure::wrap(Box::new(move |_: web_sys::Event| notify()) as Box<dyn FnMut(_)>);
        let _ = worker
            .add_event_listener_with_callback("statechange", statechange.as_ref().unchecked_ref());
        *slot.borrow_mut() = Some((worker, statechange));
    }
}

impl RegistrationWatch {
    fn observe(registration: &ServiceWorkerRegistration, on_change: Rc<dyn Fn()>) -> Self {
        let installing = Rc::new(std::cell::RefCell::new(None));
        track_installing(registration, &installing, &on_change);
        let watched = registration.clone();
        let tracked = installing.clone();
        let notify = on_change.clone();
        let updatefound = Closure::wrap(Box::new(move |_: web_sys::Event| {
            track_installing(&watched, &tracked, &notify);
            notify();
        }) as Box<dyn FnMut(_)>);
        let _ = registration
            .add_event_listener_with_callback("updatefound", updatefound.as_ref().unchecked_ref());
        Self {
            registration: registration.clone(),
            updatefound: Some(updatefound),
            installing,
        }
    }

    fn stop_all(&mut self) {
        if let Some(updatefound) = self.updatefound.take() {
            let _ = self.registration.remove_event_listener_with_callback(
                "updatefound",
                updatefound.as_ref().unchecked_ref(),
            );
        }
        if let Some((worker, statechange)) = self.installing.borrow_mut().take() {
            let _ = worker.remove_event_listener_with_callback(
                "statechange",
                statechange.as_ref().unchecked_ref(),
            );
        }
    }
}

impl Drop for RegistrationWatch {
    fn drop(&mut self) {
        self.stop_all();
    }
}

impl UpdateWatch for RegistrationWatch {
    fn stop(&mut self) {
        self.stop_all();
    }
}

impl UpdateSignals for ServiceWorkerRegistration {
    fn snapshot(&self) -> crate::offline::UpdateSnapshot {
        snapshot_registration(self)
    }

    fn watch(&self, on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
        Box::new(RegistrationWatch::observe(self, on_change))
    }

    fn adopt_waiting(&self) -> Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>> {
        let registration = self.clone();
        Box::pin(adopt_waiting_registration(registration))
    }

    fn reload(&self) {
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
        }
    }
}

/// Ask the waiting worker to take over, then wait for this page's controller to
/// change. The reply port carries the worker's own decision, made against its
/// live window clients when it would skip waiting (see `service_worker.rs`), so
/// the page never decides safety for other editors. A legacy or unresponsive
/// worker answers nothing; the bounded waits report that instead of reloading.
async fn adopt_waiting_registration(
    registration: ServiceWorkerRegistration,
) -> Result<(), AdoptError> {
    let failed = |message: &str| AdoptError::Failed(message.to_string());
    let Some(waiting) = registration.waiting() else {
        return Err(failed("no waiting build is ready to adopt"));
    };
    let Some(window) = web_sys::window() else {
        return Err(failed("the page is gone; the update cannot finish"));
    };
    let container = window.navigator().service_worker();
    // Attach the takeover listener before asking, so a fast takeover is never missed.
    let (took_sender, took_receiver) = oneshot::channel();
    let took_sender = Rc::new(RefCell::new(Some(took_sender)));
    let changed = took_sender.clone();
    let on_change = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(sender) = changed.borrow_mut().take() {
            let _ = sender.send(true);
        }
    });
    container
        .add_event_listener_with_callback("controllerchange", on_change.as_ref().unchecked_ref())
        .map_err(|_| failed("the takeover listener could not be attached"))?;
    let channel = web_sys::MessageChannel::new()
        .map_err(|_| failed("the waiting build could not be reached"))?;
    let (reply_sender, reply_receiver) = oneshot::channel();
    let reply_sender = Rc::new(RefCell::new(Some(reply_sender)));
    let replied = reply_sender.clone();
    let on_reply =
        Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            if let Some(sender) = replied.borrow_mut().take() {
                let _ = sender.send(event.data().as_string());
            }
        });
    let port = channel.port1();
    port.set_onmessage(Some(on_reply.as_ref().unchecked_ref()));
    let transfers = Array::new();
    transfers.push(&channel.port2());
    let posted = waiting.post_message_with_transferable(
        &JsValue::from_str(crate::offline::ADOPT_WAITING_MESSAGE),
        &transfers,
    );
    let outcome = if posted.is_err() {
        Err(failed("the waiting build could not be reached"))
    } else {
        let no_reply = reply_sender.clone();
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(5_000).await;
            if let Some(sender) = no_reply.borrow_mut().take() {
                let _ = sender.send(None);
            }
        });
        match reply_receiver.await.ok().flatten().as_deref() {
            Some(crate::offline::ADOPT_DECLINED) => Err(AdoptError::OtherTabs),
            Some(crate::offline::ADOPT_ACCEPTED) => {
                let no_takeover = took_sender.clone();
                spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                    if let Some(sender) = no_takeover.borrow_mut().take() {
                        let _ = sender.send(false);
                    }
                });
                if took_receiver.await.unwrap_or(false) {
                    Ok(())
                } else {
                    Err(failed("the waiting build did not take over"))
                }
            }
            _ => Err(failed("the waiting build did not answer")),
        }
    };
    let _ = container.remove_event_listener_with_callback(
        "controllerchange",
        on_change.as_ref().unchecked_ref(),
    );
    port.set_onmessage(None);
    port.close();
    channel.port2().close();
    drop(on_reply);
    drop(on_change);
    outcome
}
