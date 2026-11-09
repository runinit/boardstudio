//! Mount the production Matrix Setup owner with deterministic detached async ports.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
extern crate self as gloo_timers;
extern crate self as wasm_bindgen_futures;

use boardstudio_application::{Event, OperationId};
use boardstudio_core::model::{Board, ProjectDoc};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[path = "../crates/catalogue/src/matrix_setup_operation.rs"]
mod matrix_setup_operation;
#[path = "support/matrix_setup_presentation.rs"]
mod presentation;

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
pub mod future {
    use super::*;
    pub struct TimeoutFuture(bool);
    impl TimeoutFuture {
        pub fn new(_: u32) -> Self {
            Self(false)
        }
    }
    impl Future for TimeoutFuture {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }
}

thread_local! { static TEMPLATES: RefCell<Option<Result<Vec<boardstudio_core::model::PartDefinition>, String>>> = const { RefCell::new(None) }; }
use boardstudio_web_runtime::runtime;

fn document(id: &str, revision: u64, with_instance: bool) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, id);
    document.revision = revision;
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    if with_instance {
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({"instances":[{
                "id":"primary", "name":"Primary", "boardId":"board", "half":"single", "role":"standalone", "flipped":false, "constructionLinked":false
            }]}))
            .unwrap(),
        );
    }
    document
}

fn open_document(runtime: &runtime::Runtime, id: &str, revision: u64, with_instance: bool) {
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: document(id, revision, with_instance),
    });
}

use boardstudio_core::model::{PartDefinition, PartGenerator, PartKind};
use std::collections::BTreeMap;
fn definition(id: &str, source: &str, kind: PartKind) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: BTreeMap::new(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: Some(PartGenerator {
            source: source.into(),
            version: "fixture".into(),
            parameters: BTreeMap::new(),
        }),
    }
}

fn catalogue() -> Vec<PartDefinition> {
    vec![
        definition(
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/diode_tht_sod123",
            "ceoloide/diode_tht_sod123",
            PartKind::Passive,
        ),
        definition(
            "ergogen:ceoloide/led_sk6812mini-e",
            "ceoloide/led_sk6812mini-e",
            PartKind::Passive,
        ),
    ]
}
