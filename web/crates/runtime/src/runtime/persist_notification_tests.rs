//! Native persist-completion notification contract (see [`super::persist_report`]).
//!
//! `runtime.rs` is `wasm32`-only so no browser-only bindgen test here runs in CI.
//! This drives the real [`Session`] through Open/rename/persist completions and
//! dispatches through [`super::persist_report`] + [`super::publish`] exactly as
//! `Runtime::complete` does, keeping the original assertions: general fires only
//! on refresh, durability on save-only movement, `"Saved locally."` on commit,
//! `"disk full"` on failure, one transient notification on commit, none after.

use super::{RuntimeNotification, RuntimeReport, persist_report, publish, take_notifications};
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
    fn operation(&mut self) -> OperationId {
        let operation = OperationId(self.next_operation);
        self.next_operation += 1;
        operation
    }

    fn persist(&mut self, event: Event, result: SaveResult) {
        let effects = self.session.submit(event);
        let Effect::Persist {
            save_attempt_id, ..
        } = effects.last().unwrap()
        else {
            panic!("{effects:?}")
        };
        let previous = self.session.read_model().clone();
        let previous_scope = self.session.scope();
        let effects = self.session.complete(Completion::Persist {
            save_attempt_id: *save_attempt_id,
            result,
        });
        let next = self.session.read_model().clone();
        let changed = next.accepted.as_ref().map(|snapshot| snapshot.token)
            != previous.accepted.as_ref().map(|snapshot| snapshot.token);
        let report = persist_report(&previous.durability, &next.durability, changed, &effects);
        let has_report = report.is_some();
        if let Some(report) = report {
            publish(&self.status, &self.notifications, report, || {});
        }
        if next.lifecycle == previous.lifecycle
            && !changed
            && self.session.scope() == previous_scope
        {
            if next.durability != previous.durability {
                self.durability += 1;
                return;
            } else if !has_report {
                return;
            }
        }
        self.general += 1;
    }
}

#[test]
fn a_save_completion_notifies_only_the_durability_subscriber() {
    let mut harness = Harness {
        session: Session::new(),
        next_operation: 1,
        general: 0,
        durability: 0,
        status: RefCell::new(RuntimeReport::default()),
        notifications: RefCell::new(Vec::new()),
    };
    let operation_id = harness.operation();
    harness.persist(
        Event::Open {
            operation_id,
            document: ProjectDoc::empty("project", "Original"),
        },
        SaveResult::Committed,
    );
    assert_eq!([harness.general, harness.durability], [1, 0]);
    take_notifications(&harness.notifications);
    for (name, error) in [("Edited", None), ("Unsaved", Some("disk full"))] {
        let operation_id = harness.operation();
        harness.persist(
            Event::ResolveEdit {
                operation_id,
                label: "rename".into(),
                resolver: EditResolver::rename_project("rename", name.into()),
            },
            error.map_or(SaveResult::Committed, |error| {
                SaveResult::Aborted(error.into())
            }),
        );
        assert_eq!(
            harness.status.borrow().message,
            error.unwrap_or("Saved locally.")
        );
    }
    assert_eq!([harness.general, harness.durability], [1, 2]);
    assert_eq!(take_notifications(&harness.notifications).len(), 1);
}
