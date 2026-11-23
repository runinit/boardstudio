//! Mounted lifetime of the native Case preview producer.
#[cfg(target_arch = "wasm32")]
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
pub fn use_native_case_preview(runtime: Rc<Runtime>, source: Option<(Scope, SnapshotToken, u64)>) {
    let request_lifetime = use_hook(|| Rc::new(RefCell::new(None::<Rc<Cell<bool>>>)));
    let dropping = request_lifetime.clone();
    let cancelling = runtime.clone();
    use_drop(move || {
        if let Some(active) = dropping.borrow_mut().take() {
            active.set(false);
        }
        cancelling.cancel_native_case_preview();
    });
    use_effect(use_reactive((&source,), move |(source,)| {
        if let Some(active) = request_lifetime.borrow_mut().take() {
            active.set(false);
        }
        runtime.cancel_native_case_preview();
        if let Some((scope, token, revision)) = source {
            let active = Rc::new(Cell::new(true));
            *request_lifetime.borrow_mut() = Some(active.clone());
            let runtime = runtime.clone();
            spawn_local(async move {
                // Detached browser futures may first poll after the owner unmounted
                // or its source was replaced. Such work must not acquire a new lease.
                if !active.get() {
                    return;
                }
                let _ = runtime
                    .prepare_native_case_preview(scope, token, revision)
                    .await;
            });
        }
    }));
}

#[cfg(all(not(target_arch = "wasm32"), test))]
mod native_tests {
    //! Native `VirtualDom` lifecycle invariants for the preview hook, driven through the shared
    //! detached queue instead of the browser microtask queue.
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_web_test_runtime::{pending_detached, poll_detached, spawn_local};
    use std::{
        future::Future,
        task::{Context, Poll},
    };

    /// Minimal port the preview hook needs: cancel a pending request and start a detached one.
    /// The browser Runtime implements this against its worker jobs; native tests use a fake.
    pub trait CasePreviewPort {
        fn cancel_native_case_preview(&self);
        fn prepare_native_case_preview(
            &self,
            scope: Scope,
            token: SnapshotToken,
            revision: u64,
        ) -> impl Future<Output = Result<(), String>>;
    }

    pub fn use_native_case_preview<R: CasePreviewPort + 'static>(
        runtime: Rc<R>,
        source: Option<(Scope, SnapshotToken, u64)>,
    ) {
        let request_lifetime = use_hook(|| Rc::new(RefCell::new(None::<Rc<Cell<bool>>>)));
        let dropping = request_lifetime.clone();
        let cancelling = runtime.clone();
        use_drop(move || {
            if let Some(active) = dropping.borrow_mut().take() {
                active.set(false);
            }
            cancelling.cancel_native_case_preview();
        });
        use_effect(use_reactive((&source,), move |(source,)| {
            if let Some(active) = request_lifetime.borrow_mut().take() {
                active.set(false);
            }
            runtime.cancel_native_case_preview();
            if let Some((scope, token, revision)) = source {
                let active = Rc::new(Cell::new(true));
                *request_lifetime.borrow_mut() = Some(active.clone());
                let runtime = runtime.clone();
                spawn_local(async move {
                    // Detached browser futures may first poll after the owner unmounted
                    // or its source was replaced. Such work must not acquire a new lease.
                    if !active.get() {
                        return;
                    }
                    let _ = runtime
                        .prepare_native_case_preview(scope, token, revision)
                        .await;
                });
            }
        }));
    }

    #[derive(Default)]
    pub struct Runtime {
        pub starts: Cell<usize>,
        pub accepted: Cell<usize>,
        pub cancellations: Cell<usize>,
        pub released: Cell<bool>,
        pub away: Cell<bool>,
    }

    impl CasePreviewPort for Runtime {
        fn cancel_native_case_preview(&self) {
            self.cancellations.set(self.cancellations.get() + 1);
        }

        async fn prepare_native_case_preview(
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

    fn mounted_preview() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
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
        use_native_case_preview(runtime, source);
        rsx! { div { "Case preview" } }
    }

    fn mount(runtime: Rc<Runtime>) -> VirtualDom {
        let mut dom = VirtualDom::new(mounted_preview);
        dom.provide_root_context(runtime);
        dom.rebuild_to_vec();
        flush(&mut dom);
        assert_eq!(
            pending_detached(),
            1,
            "mount must schedule one detached request"
        );
        dom
    }

    fn flush(dom: &mut VirtualDom) {
        use std::task::Waker;
        for _ in 0..4 {
            dom.render_immediate_to_vec();
            let mut work = std::pin::pin!(dom.wait_for_work());
            let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        }
    }

    #[test]
    fn source_away_and_back_discards_a_detached_start_from_the_previous_lifetime() {
        let runtime = Rc::new(Runtime::default());
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
        assert_eq!(pending_detached(), 0);
    }

    #[test]
    fn unmount_before_detached_start_prevents_acquiring_a_preview_owner() {
        let runtime = Rc::new(Runtime::default());
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
        let runtime = Rc::new(Runtime::default());
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
        assert_eq!(pending_detached(), 0);
    }
}
