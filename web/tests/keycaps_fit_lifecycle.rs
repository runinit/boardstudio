//! Mount the production hook with a deterministic request port. Browser worker transport and
//! scene projection are outside this test; Dioxus signal tracking and effect scheduling are real.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]

use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::ProjectDoc;
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, rc::Rc, task::Context};

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
    }

    impl Runtime {
        pub fn resolve_keycaps_preview(
            &self,
            _scope: Scope,
            _token: SnapshotToken,
            revision: u64,
        ) -> Result<KeycapResolution, String> {
            self.requests.set(self.requests.get() + 1);
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
        dom.render_immediate_to_vec();
        let mut pending = std::pin::pin!(dom.wait_for_work());
        if pending
            .as_mut()
            .poll(&mut Context::from_waker(std::task::Waker::noop()))
            .is_pending()
        {
            settled = true;
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
