//! Exercise the production mount hook with detached futures, as in the browser.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
extern crate self as wasm_bindgen_futures;

use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Detached = Pin<Box<dyn Future<Output = ()>>>;
thread_local! { static TASKS: RefCell<Vec<Detached>> = RefCell::default(); }

pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(future)));
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

mod runtime {
    use super::*;

    #[derive(Default)]
    pub struct Runtime {
        pub starts: Cell<usize>,
        pub accepted: Cell<usize>,
        pub cancellations: Cell<usize>,
        pub released: Cell<bool>,
        pub away: Cell<bool>,
    }

    impl Runtime {
        pub fn cancel_native_case_preview(&self) {
            self.cancellations.set(self.cancellations.get() + 1);
        }

        pub async fn prepare_native_case_preview(
            &self,
            _: Scope,
            _: SnapshotToken,
            _: u64,
        ) -> Result<(), String> {
            self.starts.set(self.starts.get() + 1);
            let owner = self.cancellations.get();
            std::future::poll_fn(|_| {
                if self.released.get() {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            if owner == self.cancellations.get() {
                self.accepted.set(self.accepted.get() + 1);
            }
            Ok(())
        }
    }
}

#[path = "../src/case_preview_lifecycle.rs"]
mod case_preview_lifecycle;

fn mounted_preview() -> Element {
    let runtime = use_context::<Rc<runtime::Runtime>>();
    let source = (!runtime.away.get()).then_some((
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        SnapshotToken(43),
        12,
    ));
    case_preview_lifecycle::use_native_case_preview(runtime, source);
    rsx! { div { "Case preview" } }
}

fn mount(runtime: Rc<runtime::Runtime>) -> VirtualDom {
    let mut dom = VirtualDom::new(mounted_preview);
    dom.provide_root_context(runtime);
    dom.rebuild_to_vec();
    flush(&mut dom);
    TASKS.with_borrow(|tasks| {
        assert_eq!(tasks.len(), 1, "mount must schedule one detached request")
    });
    dom
}

fn flush(dom: &mut VirtualDom) {
    for _ in 0..4 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
    }
}

#[test]
fn source_away_and_back_discards_a_detached_start_from_the_previous_lifetime() {
    let runtime = Rc::new(runtime::Runtime::default());
    let mut dom = mount(runtime.clone());
    runtime.away.set(true);
    dom.mark_dirty(ScopeId::ROOT);
    flush(&mut dom);
    runtime.away.set(false);
    dom.mark_dirty(ScopeId::ROOT);
    flush(&mut dom);
    runtime.released.set(true);
    poll_detached();
    assert_eq!(
        runtime.starts.get(),
        1,
        "only the new source lifetime may begin"
    );
    assert_eq!(runtime.accepted.get(), 1);
    TASKS.with_borrow(|tasks| assert!(tasks.is_empty()));
}

#[test]
fn unmount_before_detached_start_prevents_acquiring_a_preview_owner() {
    let runtime = Rc::new(runtime::Runtime::default());
    let dom = mount(runtime.clone());
    drop(dom);
    runtime.released.set(true);
    poll_detached();
    assert_eq!(
        runtime.starts.get(),
        0,
        "unmounted queued task must not acquire an owner"
    );
    assert_eq!(runtime.accepted.get(), 0);
}

#[test]
fn unmount_retires_pending_owner_before_late_completion() {
    let runtime = Rc::new(runtime::Runtime::default());
    let dom = mount(runtime.clone());
    poll_detached();
    assert_eq!(runtime.starts.get(), 1);
    assert_eq!(runtime.accepted.get(), 0);
    drop(dom);
    runtime.released.set(true);
    poll_detached();
    assert_eq!(
        runtime.accepted.get(),
        0,
        "late completion must observe unmount cancellation"
    );
    TASKS.with_borrow(|tasks| assert!(tasks.is_empty()));
}
