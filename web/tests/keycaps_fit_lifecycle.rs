//! Mount the production hook with a deterministic request port. Browser worker transport and
//! scene projection are outside this test; Dioxus signal tracking and effect scheduling are real.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]

extern crate self as wasm_bindgen_futures;

use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::KeycapResolution;
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, rc::Rc, task::Context};

// The production WASM executor schedules the same future on the browser microtask queue.
// The mounted native test schedules it on the VirtualDom so it can drain deterministically.
pub fn spawn_local(future: impl Future<Output = ()> + 'static) {
    spawn(future);
}

mod runtime {
    use super::*;
    use boardstudio_core::model::KeycapResolution;

    #[derive(Default)]
    pub struct Runtime {
        pub requests: Cell<usize>,
    }

    impl Runtime {
        pub async fn resolve_keycaps_preview(
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
#[allow(dead_code)] // The imported module also contains inspector UI outside this hook test.
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
    // Observe the actual hook output just as Editor does, keeping its signal subscription.
    rsx! { div { "{fit.state.is_some()}" } }
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
fn same_source_retry_and_failure_keep_retained_result_stale() {
    let mut accepted = keycaps_fit::KeycapsFitState::begin(source(1), None);
    accepted.finish(Ok(KeycapResolution {
        revision: 1,
        specs: vec![],
        findings: vec![],
    }));
    assert!(accepted.is_current());

    let mut retrying = keycaps_fit::KeycapsFitState::begin(source(1), Some(&accepted));
    assert!(
        !retrying.is_current(),
        "same-source retry is pending, so prior result is stale"
    );
    retrying.finish(Err("worker unavailable".into()));
    assert!(
        !retrying.is_current(),
        "failed retry cannot make retained result current"
    );
}

#[test]
fn findings_are_deduplicated_severity_sorted_and_grouped_like_react() {
    use boardstudio_core::model::{Board, Finding, ProjectDoc, Scope as FindingScope, Severity};

    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board-1".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let warning = Finding {
        id: "board:board-1:feature:clearance:near-key".into(),
        severity: Severity::Warning,
        scope: FindingScope::Layout,
        message: "Keycaps are close".into(),
        target_ids: vec!["board-1".into()],
    };
    let duplicate = Finding {
        id: "other-board:board-1:feature:clearance:near-key".into(),
        ..warning.clone()
    };
    let error = Finding {
        id: "worker:error".into(),
        severity: Severity::Error,
        scope: FindingScope::Layout,
        message: "Worker failed".into(),
        target_ids: vec![],
    };
    let groups = keycaps_fit::grouped_findings(&[warning, duplicate, error], &document);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].label, "Layout review");
    assert_eq!(groups[0].findings.len(), 1);
    assert_eq!(groups[0].findings[0].severity, Severity::Error);
    assert_eq!(groups[1].label, "Left PCB");
    assert_eq!(
        groups[1].findings.len(),
        1,
        "same feature/message/geometry collapses"
    );
}
