//! Mount the production hook with a deterministic request port. Browser worker transport and
//! scene projection are outside this test; Dioxus signal tracking and effect scheduling are real.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]

extern crate self as wasm_bindgen_futures;

use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::ProjectDoc;
use dioxus::prelude::*;
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

thread_local! {
    // Model the production browser executor: request tasks are detached from the component
    // lifetime and may complete after the mounted VirtualDom has been dropped.
    static DETACHED: std::cell::RefCell<Vec<Pin<Box<dyn Future<Output = ()>>>>> =
        std::cell::RefCell::new(Vec::new());
}

pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    DETACHED.with(|tasks| tasks.borrow_mut().push(Box::pin(future)));
}

fn poll_detached() {
    DETACHED.with(|tasks| {
        let pending = std::mem::take(&mut *tasks.borrow_mut());
        let waker = std::task::Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut keep = Vec::new();
        for mut task in pending {
            if task.as_mut().poll(&mut context).is_pending() {
                keep.push(task);
            }
        }
        tasks.borrow_mut().extend(keep);
    });
}

fn detached_count() -> usize {
    DETACHED.with(|tasks| tasks.borrow().len())
}

mod runtime {
    use super::*;
    use boardstudio_core::model::{KeycapResolution, KeycapSpec};

    #[derive(Clone, Debug, PartialEq)]
    pub(crate) struct KeycapsPreviewInput {
        pub(crate) scope: Scope,
        pub(crate) token: SnapshotToken,
        pub(crate) revision: u64,
        pub(crate) specs: Vec<KeycapSpec>,
    }

    #[derive(Default)]
    pub struct Runtime {
        pub requests: Cell<usize>,
        pub hold: Cell<bool>,
    }

    impl Runtime {
        pub async fn resolve_keycaps_preview(
            &self,
            _scope: Scope,
            _token: SnapshotToken,
            revision: u64,
        ) -> Result<KeycapResolution, String> {
            self.requests.set(self.requests.get() + 1);
            std::future::poll_fn(|context| {
                if self.hold.get() {
                    context.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            })
            .await;
            Ok(KeycapResolution {
                revision,
                specs: Vec::new(),
                findings: Vec::new(),
            })
        }
    }
}

#[path = "../src/presentation/keycaps_fit.rs"]
mod keycaps_fit;

#[derive(Clone)]
struct Probe {
    runtime: Rc<runtime::Runtime>,
    source: Option<keycaps_fit::KeycapsFitSource>,
    renders: Rc<Cell<usize>>,
}

fn mounted_fit() -> Element {
    let probe = use_context::<Probe>();
    probe.renders.set(probe.renders.get() + 1);
    let fit = keycaps_fit::use_keycaps_fit(probe.runtime, probe.source);
    let document = Rc::new(ProjectDoc::empty("document", "Test document"));
    rsx! {
        keycaps_fit::KeycapsFitInspector {
            document,
            state: fit.state,
            mechanical_layer_ids: Rc::from([]),
            on_retry: fit.on_retry,
            on_navigate: |_| {},
        }
    }
}

fn assert_settles(source: Option<keycaps_fit::KeycapsFitSource>, expected_requests: usize) {
    let probe = Probe {
        runtime: Rc::default(),
        source,
        renders: Rc::default(),
    };
    let mut dom = VirtualDom::new(mounted_fit);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    let mut settled = false;
    for _ in 0..16 {
        dom.render_immediate_to_vec();
        poll_detached();
        dom.render_immediate_to_vec();
        let mut pending = std::pin::pin!(dom.wait_for_work());
        if pending
            .as_mut()
            .poll(&mut Context::from_waker(std::task::Waker::noop()))
            .is_pending()
        {
            settled = detached_count() == 0;
            break;
        }
    }
    assert!(
        settled,
        "Keycaps fit did not settle for an unchanged source: {} renders, {} requests",
        probe.renders.get(),
        probe.runtime.requests.get()
    );
    assert_eq!(probe.runtime.requests.get(), expected_requests);
}

#[test]
fn absent_source_settles_without_requests() {
    assert_settles(None, 0);
}

#[test]
fn accepted_source_settles_after_one_resolution() {
    assert_settles(
        Some(keycaps_fit::KeycapsFitSource {
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "document".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            token: SnapshotToken(1),
            revision: 1,
            case_preview_current: false,
        }),
        1,
    );
}

fn source(revision: u64) -> keycaps_fit::KeycapsFitSource {
    keycaps_fit::KeycapsFitSource {
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        token: SnapshotToken(revision),
        revision,
        case_preview_current: false,
    }
}

#[test]
fn pending_browser_request_does_not_touch_signals_after_inspector_unmounts() {
    let probe = Probe {
        runtime: Rc::new(runtime::Runtime::default()),
        source: Some(source(1)),
        renders: Rc::default(),
    };
    probe.runtime.hold.set(true);
    let mut dom = VirtualDom::new(mounted_fit);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    dom.render_immediate_to_vec();
    poll_detached();
    assert_eq!(probe.runtime.requests.get(), 1);
    assert_eq!(
        detached_count(),
        1,
        "resolution must still be pending before unmount"
    );

    drop(dom);
    probe.runtime.hold.set(false);
    poll_detached();
    assert_eq!(detached_count(), 0);
}

#[test]
fn completed_browser_request_settles_against_mounted_inspector() {
    assert_settles(Some(source(2)), 1);
}
