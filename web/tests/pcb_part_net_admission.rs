//! Mounted regressions for the exact lifetime/workspace guard used after async PCB classification.
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

#[path = "../src/presentation/pcb_wiring/part_net_admission.rs"]
mod part_net_admission;

type Detached = Pin<Box<dyn Future<Output = ()>>>;
thread_local! { static TASKS: RefCell<Vec<Detached>> = RefCell::default(); }
pub fn spawn_local(task: impl Future<Output = ()> + 'static) {
    TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(task)));
}
fn poll_detached() {
    TASKS.with_borrow_mut(|tasks| {
        tasks.retain_mut(|task| {
            task.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        })
    });
}

#[derive(Clone)]
struct Probe {
    release: Rc<Cell<bool>>,
    result: Rc<Cell<Option<bool>>>,
    start: Rc<RefCell<Option<EventHandler<()>>>>,
    switch_to_layout: Rc<RefCell<Option<EventHandler<()>>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let alive = part_net_admission::use_part_net_owner_lifetime();
    let mut workspace = use_signal(|| "PCB");
    let generation = use_signal(|| 1_u64);
    let classification_workspace = workspace;
    let classification_generation = generation;
    let start = use_callback({
        let release = probe.release.clone();
        let result = probe.result.clone();
        let alive = alive.clone();
        move |_| {
            let release = release.clone();
            let result = result.clone();
            let alive = alive.clone();
            spawn_local(async move {
                std::future::poll_fn(|cx| {
                    if release.get() {
                        Poll::Ready(())
                    } else {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                })
                .await;
                if part_net_admission::part_net_owner_context_is_current(
                    &alive,
                    classification_workspace,
                    classification_generation,
                    1,
                ) {
                    result.set(Some(true));
                }
            });
        }
    });
    let switch_to_layout = use_callback(move |_| workspace.set("Layout"));
    *probe.start.borrow_mut() = Some(start);
    *probe.switch_to_layout.borrow_mut() = Some(switch_to_layout);
    rsx! { span { "PCB async owner probe" } }
}

fn flush(dom: &mut VirtualDom) {
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..4 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = std::future::Future::poll(work.as_mut(), &mut Context::from_waker(Waker::noop()));
    }
}

fn mounted() -> (Probe, VirtualDom) {
    TASKS.with_borrow_mut(Vec::clear);
    let probe = Probe {
        release: Rc::new(Cell::new(false)),
        result: Rc::new(Cell::new(None)),
        start: Rc::new(RefCell::new(None)),
        switch_to_layout: Rc::new(RefCell::new(None)),
    };
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush(&mut dom);
    (probe, dom)
}

#[test]
fn pending_classification_finishing_after_owner_unmount_does_not_read_dropped_signals() {
    let (probe, dom) = mounted();
    probe.start.borrow().as_ref().unwrap().call(());
    poll_detached();
    drop(dom);
    probe.release.set(true);
    poll_detached();
    assert_eq!(probe.result.get(), None, "a dropped owner must not be read");
}

#[test]
fn pending_classification_rechecks_live_workspace_before_admission() {
    let (probe, mut dom) = mounted();
    probe.start.borrow().as_ref().unwrap().call(());
    poll_detached();
    probe.switch_to_layout.borrow().as_ref().unwrap().call(());
    flush(&mut dom);
    probe.release.set(true);
    poll_detached();
    assert_eq!(probe.result.get(), None);
}
