//! Mount the production physical-setup owner with deterministic detached async ports.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
extern crate self as gloo_timers;
extern crate self as wasm_bindgen_futures;

use boardstudio_application::{
    AcceptedSnapshot, Event, Lifecycle, OperationId, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[path = "../crates/runtime/src/operation_outcomes.rs"]
mod operation_outcomes;
#[path = "../src/physical_setup.rs"]
mod physical_setup;
#[path = "../src/presentation/pcb_physical_setup.rs"]
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
#[derive(Clone, Copy, Default)]
struct InstanceSelection;
impl InstanceSelection {
    fn reconcile(self, _: SessionEpoch, _: String, _: String) {}
}
mod runtime {
    use super::*;
    #[derive(Clone)]
    pub struct Model {
        pub lifecycle: Lifecycle,
        pub accepted: Option<AcceptedSnapshot>,
        pub active_board_id: String,
        pub active_instance_id: Option<String>,
    }
    pub struct Runtime {
        pub model: RefCell<Model>,
        pub events: RefCell<Vec<Event>>,
        pub outcomes: operation_outcomes::OperationOutcomes,
        next: Cell<u64>,
    }
    impl Runtime {
        pub fn new() -> Rc<Self> {
            Rc::new(Self {
                model: RefCell::new(model("A", 1, 1)),
                events: RefCell::default(),
                outcomes: Default::default(),
                next: Cell::new(10),
            })
        }
        pub fn model(&self) -> Model {
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
    pub fn model(id: &str, token: u64, revision: u64) -> Model {
        let mut document = ProjectDoc::empty(id, id);
        document.revision = revision;
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":1.6, "traces":[], "vias":[]
        })).unwrap());
        let scene: SceneDelta=serde_json::from_value(serde_json::json!({
            "revision":revision,"transactionId":"fixture","changedIds":[],"transforms":[],"matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],
            "readiness":{"layout":true,"outline":true,"pcb":true,"case":false}
        })).unwrap();
        Model {
            lifecycle: Lifecycle::Ready,
            active_board_id: "board".into(),
            active_instance_id: None,
            accepted: Some(AcceptedSnapshot {
                session_epoch: SessionEpoch(1),
                token: SnapshotToken(token),
                document: std::sync::Arc::new(document),
                scene: std::sync::Arc::new(scene),
            }),
        }
    }
}
