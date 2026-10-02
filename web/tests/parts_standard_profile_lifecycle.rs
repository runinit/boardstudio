//! The detached Parts Core request outlives the editor component. The production
//! owner token must retire before any editor Signal is accessed on late success
//! or failure.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]

extern crate self as wasm_bindgen_futures;

use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[path = "../src/presentation/parts/standard_profile_lifetime.rs"]
mod production_lifetime;

type DetachedTask = Pin<Box<dyn Future<Output = ()>>>;
thread_local! {
    static DETACHED_TASKS: RefCell<Vec<DetachedTask>> = RefCell::default();
}

pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    DETACHED_TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(future)));
}

fn poll_detached() {
    DETACHED_TASKS.with_borrow_mut(|tasks| {
        tasks.retain_mut(|task| {
            task.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        })
    });
}

#[derive(Default)]
struct Gate {
    result: RefCell<Option<Result<(), &'static str>>>,
    waiter: RefCell<Option<Waker>>,
}

impl Gate {
    async fn request(self: Rc<Self>) -> Result<(), &'static str> {
        std::future::poll_fn(|context| {
            if let Some(result) = self.result.borrow_mut().take() {
                Poll::Ready(result)
            } else {
                *self.waiter.borrow_mut() = Some(context.waker().clone());
                Poll::Pending
            }
        })
        .await
    }

    fn resolve(&self, result: Result<(), &'static str>) {
        *self.result.borrow_mut() = Some(result);
        if let Some(waiter) = self.waiter.borrow_mut().take() {
            waiter.wake();
        }
    }
}

#[derive(Default)]
struct Probe {
    gate: Rc<Gate>,
    signal_accesses: Cell<usize>,
    publications: Cell<usize>,
}

fn mounted_editor_owner() -> Element {
    let probe = use_context::<Rc<Probe>>();
    let lifetime = use_hook(production_lifetime::PartsStandardProfileLifetime::new);
    let mut editor_signal = use_signal(|| 0usize);
    use_drop({
        let lifetime = lifetime.clone();
        move || lifetime.retire()
    });
    use_hook({
        let lifetime = lifetime.clone();
        let probe = probe.clone();
        move || {
            spawn_local(async move {
                let result = probe.gate.clone().request().await;
                let _ = lifetime.run_if_mounted(|| {
                    // This mirrors the first UI continuation's local Signal reads
                    // and success/error publication after CoreWorker::request.
                    probe.signal_accesses.set(probe.signal_accesses.get() + 1);
                    editor_signal.with_mut(|value| *value += 1);
                    if result.is_ok() || result.is_err() {
                        probe.publications.set(probe.publications.get() + 1);
                    }
                });
            });
        }
    });
    rsx! { div { "Mechanical fit editor" } }
}

fn mounted(probe: Rc<Probe>) -> VirtualDom {
    let mut dom = VirtualDom::new(mounted_editor_owner);
    dom.provide_root_context(probe);
    dom.rebuild_to_vec();
    poll_detached();
    dom
}

fn assert_unmounted_completion(result: Result<(), &'static str>) {
    let probe = Rc::new(Probe::default());
    let dom = mounted(probe.clone());
    DETACHED_TASKS.with_borrow(|tasks| assert_eq!(tasks.len(), 1, "Core request remains pending"));

    drop(dom);
    probe.gate.resolve(result);
    poll_detached();

    assert_eq!(probe.signal_accesses.get(), 0);
    assert_eq!(probe.publications.get(), 0);
    DETACHED_TASKS.with_borrow(|tasks| assert!(tasks.is_empty()));
}

#[test]
fn pending_standard_fit_success_after_editor_unmount_does_not_access_signals() {
    assert_unmounted_completion(Ok(()));
}

#[test]
fn pending_standard_fit_failure_after_editor_unmount_does_not_publish_error() {
    assert_unmounted_completion(Err("fixture failure"));
}

#[test]
fn mounted_standard_fit_success_and_failure_publish_once() {
    for result in [Ok(()), Err("fixture failure")] {
        let probe = Rc::new(Probe::default());
        let dom = mounted(probe.clone());
        probe.gate.resolve(result);
        poll_detached();
        assert_eq!(probe.signal_accesses.get(), 1);
        assert_eq!(probe.publications.get(), 1);
        drop(dom);
    }
}
