//! Native coverage for the persist-completion notification contract.
//!
//! The browser `Runtime` (`runtime.rs`) is `wasm32`-only, so a
//! `#[wasm_bindgen_test]` here never executes in CI (`scripts/check.py`
//! launches only `host`, `worker-host` and packaged-page tests). This test
//! drives the real `boardstudio_application::Session` through Open, rename and
//! persist completions and feeds the resulting durability transitions through
//! the extracted [`super::persist_completion_decision`] plus
//! [`super::publish`]/[`super::take_notifications`], preserving the original
//! browser-test assertions: the general subscriber fires only on the refresh
//! path, the durability subscriber fires on save-only completions, the status
//! reads `"Saved locally."` after a commit and `"disk full"` after a failure,
//! and exactly one transient notification survives a commit (zero on failure).

use super::{
    PersistCompletionPath, RuntimeNotification, RuntimeReport, persist_completion_decision,
    publish, take_notifications,
};
use boardstudio_application::{
    Completion, EditResolver, Effect, Event, OperationId, SaveResult, Session,
};
use boardstudio_model::model::ProjectDoc;
use std::cell::RefCell;

struct Harness {
    session: Session,
    next_operation: u64,
    general: usize,
    durability: usize,
    status: RefCell<RuntimeReport>,
    notifications: RefCell<Vec<RuntimeNotification>>,
}

impl Harness {
    fn new() -> Self {
        Self {
            session: Session::new(),
            next_operation: 1,
            general: 0,
            durability: 0,
            status: RefCell::new(RuntimeReport::default()),
            notifications: RefCell::new(Vec::new()),
        }
    }

    fn operation(&mut self) -> OperationId {
        let operation = OperationId(self.next_operation);
        self.next_operation += 1;
        operation
    }

    /// Submit `event`, complete its persist with `result`, and dispatch the
    /// resulting notification exactly as `Runtime::complete` does: record the
    /// decision report, ping the durability subscriber on save-only movement,
    /// stay silent when nothing observable changed, and otherwise count a
    /// general refresh.
    fn persist(&mut self, event: Event, result: SaveResult) {
        let effects = self.session.submit(event);
        let Effect::Persist {
            save_attempt_id, ..
        } = effects.last().unwrap()
        else {
            panic!("{effects:?}")
        };
        let save_attempt_id = *save_attempt_id;
        let previous = self.session.read_model().clone();
        let previous_scope = self.session.scope();
        let effects = self.session.complete(Completion::Persist {
            save_attempt_id,
            result,
        });
        let next = self.session.read_model().clone();
        let (report, path) = persist_completion_decision(
            &previous.durability,
            &next.durability,
            next.accepted.as_ref().map(|snapshot| snapshot.token)
                != previous.accepted.as_ref().map(|snapshot| snapshot.token),
            self.session.scope() != previous_scope,
            next.lifecycle == previous.lifecycle,
            &effects,
        );
        if let Some(report) = report {
            publish(&self.status, &self.notifications, report, || {});
        }
        match path {
            PersistCompletionPath::DurabilityChanged => {
                self.durability += 1;
                return;
            }
            PersistCompletionPath::Silent => return,
            PersistCompletionPath::Refresh => {}
        }
        self.general += 1;
    }

    fn status(&self) -> String {
        self.status.borrow().message.clone()
    }

    fn take_notifications(&self) -> Vec<RuntimeNotification> {
        take_notifications(&self.notifications)
    }
}

#[test]
fn a_save_completion_notifies_only_the_durability_subscriber() {
    let mut harness = Harness::new();
    let open = Event::Open {
        operation_id: harness.operation(),
        document: ProjectDoc::empty("project", "Original"),
    };
    harness.persist(open, SaveResult::Committed);
    assert_eq!([harness.general, harness.durability], [1, 0]);
    harness.take_notifications();

    for (name, error) in [("Edited", None), ("Unsaved", Some("disk full"))] {
        let edit = Event::ResolveEdit {
            operation_id: harness.operation(),
            label: "rename".into(),
            resolver: EditResolver::rename_project("rename", name.into()),
        };
        let result = error.map_or(SaveResult::Committed, |error| {
            SaveResult::Aborted(error.into())
        });
        harness.persist(edit, result);
        assert_eq!(harness.status(), error.unwrap_or("Saved locally."));
    }
    assert_eq!([harness.general, harness.durability], [1, 2]);
    assert_eq!(harness.take_notifications().len(), 1);
}
