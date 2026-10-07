//! Keyed collection for observing committed panel edits through their Session outcomes.
//!
//! A controller keeps this collection with its local panel state and settles it while
//! rendering:
//!
//! ```ignore
//! let mut edits = PendingEdits::default();
//! edits.begin(&runtime, field_key, "matrix-field", Some("matrix".into()), resolver);
//! if edits.is_pending(&field_key) {
//!     // Keep the draft value visible, or disable a one-shot action.
//! }
//! for result in edits.settle(owner_is_live) {
//!     // Landed: read the accepted value; Failed: show the accepted value and message;
//!     // Retired: discard the draft silently.
//! }
//! ```

use crate::edit_ticket::{EditTicket, EditTicketPort, Settlement};
use boardstudio_application::{EditResolver, OperationId};

/// A terminal result for the latest observed edit under one caller-defined key.
#[derive(Clone, Debug, PartialEq)]
pub enum PendingEditResult<K> {
    /// The edit landed in the accepted document at this revision.
    Landed { key: K, revision: u64 },
    /// The edit failed; the accepted value should be shown with this message.
    Failed { key: K, message: String },
    /// The owner or captured document scope ended; discard the draft silently.
    Retired { key: K },
}

/// Owns the latest edit observation for each panel field or one-shot action.
///
/// Replacing a key drops only its observation. The earlier edit remains owned by the
/// Session and continues to execute. Keys are compared with `PartialEq`, so callers can
/// use their existing field enum without adding ordering or hashing traits.
pub struct PendingEdits<K> {
    tickets: Vec<(K, EditTicket)>,
}

impl<K> Default for PendingEdits<K> {
    fn default() -> Self {
        Self {
            tickets: Vec::new(),
        }
    }
}

impl<K: PartialEq> PendingEdits<K> {
    /// Begin an edit and store its observation under `key`. The newest observation for
    /// that key replaces the old one; unrelated keys remain independently pending.
    /// Returns the Session operation identity for callers that need to correlate logs.
    pub fn begin(
        &mut self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) -> OperationId {
        let ticket = EditTicket::begin(port, label, feature, resolver);
        let operation = ticket.operation();
        if let Some(index) = self
            .tickets
            .iter()
            .position(|(existing, _)| *existing == key)
        {
            self.tickets[index] = (key, ticket);
        } else {
            self.tickets.push((key, ticket));
        }
        operation
    }

    /// Whether the latest observation for this key is still pending in its captured
    /// document scope. One-shot controls use this to block repeat submission.
    pub fn is_pending(&self, key: &K) -> bool {
        self.tickets
            .iter()
            .find(|(existing, _)| existing == key)
            .is_some_and(|(_, ticket)| ticket.is_pending())
    }

    /// Drain terminal observations once, preserving key insertion order. Pending tickets
    /// stay stored. The owner predicate describes only panel lifetime; each ticket checks
    /// its captured Scope and permanently retires after a mismatch.
    pub fn settle(&mut self, owner_is_live: bool) -> Vec<PendingEditResult<K>> {
        let mut pending = Vec::with_capacity(self.tickets.len());
        let mut results = Vec::new();
        for (key, ticket) in self.tickets.drain(..) {
            match ticket.settlement(owner_is_live) {
                Settlement::Pending => pending.push((key, ticket)),
                Settlement::Landed { revision } => {
                    results.push(PendingEditResult::Landed { key, revision });
                }
                Settlement::Failed { message } => {
                    results.push(PendingEditResult::Failed { key, message });
                }
                Settlement::Retired => results.push(PendingEditResult::Retired { key }),
            }
        }
        self.tickets = pending;
        results
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::{EditResolver, Event, Resolution};
    use boardstudio_core::model::{
        Board, EditOperation, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
    };
    use std::rc::Rc;

    #[derive(Debug, PartialEq)]
    enum Field {
        Name,
        Position,
    }

    fn document(id: &str, name: &str) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, name);
        document.definitions.push(PartDefinition {
            mechanical_profile: None,
            id: "key".into(),
            name: "Key".into(),
            kind: PartKind::Switch,
            courtyard: vec![],
            pads: vec![],
            models: None,
            hardware_profile: None,
            input_profile: None,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
        });
        document.parts.push(Part {
            id: "key".into(),
            definition_id: "key".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    fn opened_runtime() -> Rc<crate::runtime::Runtime> {
        let runtime = crate::runtime::Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: document("pending-edits", "Original"),
        });
        runtime
    }

    fn rename_resolver(name: &'static str) -> EditResolver {
        EditResolver::new("pending-edits-test", move |accepted| {
            let mut document = (*accepted.document).clone();
            document.name = name.into();
            Resolution::submit(
                vec![document.id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        })
    }

    fn unchanged_resolver() -> EditResolver {
        EditResolver::new("pending-edits-test", |_accepted| Resolution::Unchanged)
    }

    #[test]
    fn latest_ticket_replaces_only_its_observation_and_both_session_edits_run() {
        let runtime = opened_runtime();
        let mut edits = PendingEdits::default();
        runtime.hold_next_core();
        edits.begin(
            &runtime,
            Field::Name,
            "rename first",
            Some("project".into()),
            rename_resolver("First"),
        );
        assert!(runtime.core_entered());
        assert!(edits.is_pending(&Field::Name));

        edits.begin(
            &runtime,
            Field::Name,
            "rename latest",
            Some("project".into()),
            rename_resolver("Latest"),
        );
        assert!(edits.is_pending(&Field::Name));
        runtime.release_core();

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.name, "Latest");
        assert_eq!(accepted.document.revision, 2);
        let resolve_count = runtime
            .events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::ResolveEdit { .. }))
            .count();
        assert_eq!(
            resolve_count, 2,
            "replacing observation must not cancel the first edit"
        );
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Landed {
                key: Field::Name,
                revision: 2,
            }]
        );
        assert!(!edits.is_pending(&Field::Name));
        assert!(edits.settle(true).is_empty());
    }

    #[test]
    fn independent_keys_do_not_block_each_other_and_one_shot_pending_ends_at_terminal() {
        let runtime = opened_runtime();
        let mut edits = PendingEdits::default();
        runtime.hold_next_core();
        edits.begin(
            &runtime,
            Field::Name,
            "rename",
            Some("project".into()),
            rename_resolver("Renamed"),
        );
        edits.begin(
            &runtime,
            Field::Position,
            "move",
            Some("layout".into()),
            unchanged_resolver(),
        );
        assert!(edits.is_pending(&Field::Name));
        assert!(edits.is_pending(&Field::Position));

        runtime.release_core();
        assert!(!edits.is_pending(&Field::Name));
        assert!(!edits.is_pending(&Field::Position));
        assert_eq!(
            edits.settle(true),
            vec![
                PendingEditResult::Landed {
                    key: Field::Name,
                    revision: 1,
                },
                PendingEditResult::Landed {
                    key: Field::Position,
                    revision: 1,
                },
            ]
        );
    }

    #[test]
    fn core_and_save_failures_keep_ticket_wording() {
        let runtime = opened_runtime();
        let mut edits = PendingEdits::default();
        runtime.fail_next_core("engine died");
        edits.begin(
            &runtime,
            Field::Name,
            "rename",
            Some("layout".into()),
            rename_resolver("Core failure"),
        );
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Failed {
                key: Field::Name,
                message: "The layout change could not be applied: engine died".into(),
            }]
        );

        let save_runtime = opened_runtime();
        save_runtime.fail_next_save("quota exceeded");
        edits.begin(
            &save_runtime,
            Field::Name,
            "rename after recovery",
            Some("layout".into()),
            rename_resolver("Save failure"),
        );
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Failed {
                key: Field::Name,
                message: "The layout completed, but the accepted document did not save: quota exceeded. Retry after recovery.".into(),
            }]
        );
    }

    #[test]
    fn unchanged_and_landed_edits_report_the_session_revision() {
        let runtime = opened_runtime();
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime,
            Field::Name,
            "rename",
            Some("layout".into()),
            rename_resolver("Changed"),
        );
        edits.begin(
            &runtime,
            Field::Position,
            "confirm",
            Some("layout".into()),
            unchanged_resolver(),
        );
        assert_eq!(
            edits.settle(true),
            vec![
                PendingEditResult::Landed {
                    key: Field::Name,
                    revision: 1,
                },
                PendingEditResult::Landed {
                    key: Field::Position,
                    revision: 1,
                },
            ]
        );
    }

    #[test]
    fn owner_departure_and_scope_change_retire_without_failure_messages() {
        let runtime = opened_runtime();
        let mut edits = PendingEdits::default();
        runtime.hold_next_core();
        edits.begin(
            &runtime,
            Field::Name,
            "rename",
            Some("layout".into()),
            rename_resolver("Retired"),
        );
        assert_eq!(
            edits.settle(false),
            vec![PendingEditResult::Retired { key: Field::Name }]
        );
        runtime.release_core();

        runtime.hold_next_core();
        edits.begin(
            &runtime,
            Field::Name,
            "rename another scope",
            Some("layout".into()),
            rename_resolver("Retired by navigation"),
        );
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: document("second-scope", "Second scope"),
        });
        assert!(edits.is_pending(&Field::Name));
        runtime.release_core();
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Retired { key: Field::Name }]
        );

        runtime.submit(Event::Close {
            operation_id: runtime.operation(),
        });
        edits.begin(
            &runtime,
            Field::Position,
            "closed-session edit",
            Some("layout".into()),
            unchanged_resolver(),
        );
        assert_eq!(
            edits.settle(true),
            vec![PendingEditResult::Retired {
                key: Field::Position
            }]
        );
    }
}
