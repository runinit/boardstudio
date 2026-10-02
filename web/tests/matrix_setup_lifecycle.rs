//! Mount the production Matrix Setup owner with deterministic detached async ports.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
extern crate self as gloo_timers;
extern crate self as wasm_bindgen_futures;

use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, ReadModel, Scope, SessionEpoch,
    SnapshotToken,
};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[path = "../src/matrix_setup_operation.rs"]
mod matrix_setup_operation;
#[path = "../src/operation_outcomes.rs"]
mod operation_outcomes;
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
mod runtime {
    use super::*;
    pub struct Runtime {
        pub model: RefCell<ReadModel>,
        pub events: RefCell<Vec<Event>>,
        pub outcomes: operation_outcomes::OperationOutcomes,
        next: Cell<u64>,
    }
    impl Runtime {
        pub fn new() -> Rc<Self> {
            Rc::new(Self {
                model: RefCell::new(model("A", 1, 10)),
                events: RefCell::default(),
                outcomes: Default::default(),
                next: Cell::new(10),
            })
        }
        pub fn model(&self) -> ReadModel {
            self.model.borrow().clone()
        }
        pub fn scope(&self) -> Option<Scope> {
            let model = self.model();
            let accepted = model.accepted?;
            Some(Scope {
                session_epoch: accepted.session_epoch,
                document_id: accepted.document.id.clone(),
                board_id: model.active_board_id,
                instance_id: model.active_instance_id,
            })
        }
        pub fn operation(&self) -> OperationId {
            let next = self.next.get();
            self.next.set(next + 1);
            OperationId(next)
        }
        pub fn observe_operation(&self, id: OperationId) -> operation_outcomes::OutcomeSlot {
            self.outcomes.observe(id)
        }
        pub fn submit(&self, event: Event) {
            self.events.borrow_mut().push(event);
        }
    }
    pub fn model(id: &str, token: u64, revision: u64) -> ReadModel {
        let mut document = ProjectDoc::empty(id, id);
        document.revision = revision;
        document.boards.push(serde_json::from_value(serde_json::json!({ "id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":1.6, "traces":[], "vias":[] })).unwrap());
        let scene: SceneDelta=serde_json::from_value(serde_json::json!({ "revision":revision,"transactionId":"fixture","changedIds":[],"transforms":[],"matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[], "readiness":{"layout":true,"outline":true,"pcb":true,"case":false} })).unwrap();
        ReadModel {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved { revision },
            active_board_id: "board".into(),
            accepted: Some(AcceptedSnapshot {
                session_epoch: SessionEpoch(1),
                token: SnapshotToken(token),
                document: std::sync::Arc::new(document),
                scene: std::sync::Arc::new(scene),
            }),
            ..Default::default()
        }
    }
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
