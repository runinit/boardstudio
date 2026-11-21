//! Signal-bound field and one-shot helpers over the Runtime `PendingEdits`
//! collection. The helpers own the settlement coordination panels repeated per field —
//! submitted-draft memory, draft restoration, inline failures, one-shot disabling — so a
//! panel binds its Signals once and reads drained results for its own follow-ups.
//!
//! A panel uses the helpers like this:
//!
//! ```ignore
//! let helpers = use_hook(|| Rc::new(PendingEditSignals::<MatrixField>::new()));
//! let rows = use_signal(String::new);
//! let rows_failure = use_signal(|| None::<String>);
//! let preset_disabled = use_signal(|| false);
//! helpers.bind_field(MatrixField::Rows, rows, rows_failure);
//! helpers.bind_one_shot(MatrixField::Preset, preset_disabled);
//!
//! // When the user commits a draft, submit it as intent; the draft stays visible:
//! helpers.begin_field(&runtime, MatrixField::Rows, "matrix-rows", Some("matrix".into()),
//!     resolver, &rows.peek().clone());
//! // One-shot controls disable while their edit is pending:
//! helpers.begin_one_shot(&runtime, MatrixField::Preset, "matrix-preset",
//!     Some("matrix".into()), resolver);
//!
//! // Settle on each render pass (the panel owns the loop, as today):
//! for result in helpers.settle(owner_is_live(), |key| accepted_text(key)) {
//!     // Landed: run caller follow-ups (exact selection, refresh). Failed: place the
//!     // message wherever else the panel needs it. Retired: nothing to do.
//! }
//! ```
//!
//! `PendingEditSignals<K, D = String>` also binds typed drafts with `D: Clone + PartialEq`.
//! A caller binds `Signal<D>` with the same `bind_field`, submits the exact value through
//! `begin_value(..., &draft)`, and projects an accepted `D` through `settle`. Submitted
//! values are compared directly; no text conversion or serialization is involved. The
//! default String form keeps `begin_field(..., &str)` and its existing field policy.
//! Typed fields and one-shot actions can share one collection and the same binding epochs.
//!
//! A component that owns bound Signals releases them when it unmounts, so a settlement
//! never writes a dropped Signal and an outcome submitted before the unmount never reaches
//! a later component that binds the same key:
//!
//! ```ignore
//! use_hook(|| helpers.bind_field(Key::Name, draft, failure));
//! use_drop({
//!     let helpers = helpers.clone();
//!     // A composite control that also binds the key as a one-shot releases both.
//!     move || { helpers.unbind_field(&Key::Name); helpers.unbind_one_shot(&Key::Name); }
//! });
//! ```
//!
//! Unbinding is idempotent and does not touch the Session: the queued edit still runs and
//! its result is still returned by `settle` for caller follow-ups. Rebinding the same
//! Signals on every render is harmless; binding other Signals for a key detaches the edits
//! submitted for the old ones in the same way.
//!
//! Invariants (decision [01](../../../../docs/plans/module-deepening/issues/01-decide-pending-edit-settlement.md),
//! [ADR-0005 amendment](../../../../docs/adr/0005-resolve-queued-edits-at-execution.md)):
//! the latest ticket per key drives it; a settlement may only write a draft that still
//! is the value its edit was submitted with, so an older outcome never overwrites a newer
//! draft; failures surface inline at the field by default while the results keep them
//! available for other placement; retirement is silent; there is no Saved state and no
//! automatic retry. Domain projection and value conversion stay with the caller through
//! the `accepted` projection.

use boardstudio_application::EditResolver;
use boardstudio_web_runtime::edit_ticket::EditTicketPort;
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
use dioxus::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Whether a settlement may write a field's draft: only while the live draft still is
/// the value the edit was submitted with. An unknown submitted draft never qualifies.
fn draft_may_restore<D: PartialEq + ?Sized>(submitted: Option<&D>, current: &D) -> bool {
    submitted.is_some_and(|submitted| submitted == current)
}

/// The Signals one field binds: its live draft and its inline failure message.
pub struct FieldView<D = String> {
    pub draft: Signal<D>,
    pub failure: Signal<Option<String>>,
}

// Signal handles are Copy even when their values are not.
impl<D> Copy for FieldView<D> {}

impl<D> Clone for FieldView<D> {
    fn clone(&self) -> Self {
        *self
    }
}

/// The Signal one one-shot control binds: unavailable while its edit is pending.
#[derive(Clone, Copy)]
pub struct OneShotView {
    pub disabled: Signal<bool>,
}

struct Binding<K, D> {
    key: K,
    view: FieldView<D>,
    /// Identifies this component's binding; a rebind with other Signals gets a new one.
    epoch: u64,
}

struct OneShotBinding<K> {
    key: K,
    view: OneShotView,
    epoch: u64,
}

/// What the latest edit for a key was submitted with, and which bindings were live then.
/// A settlement writes a Signal only while the same binding is still bound, so an old
/// outcome never reaches a component that unmounted or a remount that took its key.
struct Submission<K, D> {
    key: K,
    /// The submitted draft value, for field edits.
    text: Option<D>,
    field_epoch: Option<u64>,
    one_shot_epoch: Option<u64>,
}

/// The collection, the bound views and the submitted-draft memory, shared by every
/// clone of [`PendingEditSignals`].
struct Shared<K, D> {
    edits: RefCell<PendingEdits<K>>,
    fields: RefCell<Vec<Binding<K, D>>>,
    one_shots: RefCell<Vec<OneShotBinding<K>>>,
    submitted: RefCell<Vec<Submission<K, D>>>,
    next_epoch: Cell<u64>,
}

/// Signal-bound coordination over one [`PendingEdits`] collection. Clone to share
/// between a panel's hooks and handlers; every clone drives the same collection and
/// bindings.
#[derive(Clone)]
pub struct PendingEditSignals<K, D = String> {
    shared: Rc<Shared<K, D>>,
}

impl<K, D> Default for PendingEditSignals<K, D>
where
    K: PartialEq + 'static,
    D: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, D> PendingEditSignals<K, D>
where
    K: PartialEq + 'static,
    D: Clone + PartialEq + 'static,
{
    pub fn new() -> Self {
        Self {
            shared: Rc::new(Shared {
                edits: RefCell::new(PendingEdits::default()),
                fields: RefCell::new(Vec::new()),
                one_shots: RefCell::new(Vec::new()),
                submitted: RefCell::new(Vec::new()),
                next_epoch: Cell::new(1),
            }),
        }
    }

    fn new_epoch(&self) -> u64 {
        let epoch = self.shared.next_epoch.get();
        self.shared.next_epoch.set(epoch + 1);
        epoch
    }

    /// Bind one field's draft and inline failure Signals. Rebinding a key with the
    /// same Signals changes nothing, so a panel may bind on every render; rebinding with
    /// other Signals replaces the view and detaches edits submitted for the old one. Only
    /// bound fields receive settlement writes.
    ///
    /// A component that owns the Signals must call [`Self::unbind_field`] when it
    /// unmounts: a settlement never writes a Signal that was dropped without unbinding.
    pub fn bind_field(&self, key: K, draft: Signal<D>, failure: Signal<Option<String>>) {
        let view = FieldView { draft, failure };
        let unchanged = self
            .shared
            .fields
            .borrow()
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.view.draft == draft && binding.view.failure == failure);
        if unchanged == Some(true) {
            return;
        }
        let epoch = self.new_epoch();
        let mut fields = self.shared.fields.borrow_mut();
        match fields.iter_mut().find(|binding| binding.key == key) {
            Some(binding) => {
                binding.view = view;
                binding.epoch = epoch;
            }
            None => fields.push(Binding { key, view, epoch }),
        }
    }

    /// Bind one one-shot control's disabled Signal. Rebinding a key with the same Signal
    /// changes nothing; another Signal replaces the view and detaches older edits. The
    /// owning component calls [`Self::unbind_one_shot`] when it unmounts.
    pub fn bind_one_shot(&self, key: K, disabled: Signal<bool>) {
        let view = OneShotView { disabled };
        let unchanged = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.view.disabled == disabled);
        if unchanged == Some(true) {
            return;
        }
        let epoch = self.new_epoch();
        let mut one_shots = self.shared.one_shots.borrow_mut();
        match one_shots.iter_mut().find(|binding| binding.key == key) {
            Some(binding) => {
                binding.view = view;
                binding.epoch = epoch;
            }
            None => one_shots.push(OneShotBinding { key, view, epoch }),
        }
    }

    /// Release a field binding when its component leaves. Idempotent. The key's edit keeps
    /// running in the Session and its result is still returned by [`Self::settle`], but no
    /// settlement writes the dropped Signals, and an outcome submitted before the unbind
    /// never reaches a later binding of the same key.
    ///
    /// A composite component that binds the same key as a field and as a one-shot control
    /// releases both: call `unbind_field` and `unbind_one_shot` with the key.
    pub fn unbind_field(&self, key: &K) {
        self.shared
            .fields
            .borrow_mut()
            .retain(|binding| binding.key != *key);
    }

    /// Release a one-shot binding when its control leaves; the same guarantees as
    /// [`Self::unbind_field`]. Idempotent.
    pub fn unbind_one_shot(&self, key: &K) {
        self.shared
            .one_shots
            .borrow_mut()
            .retain(|binding| binding.key != *key);
    }

    fn field_epoch(&self, key: &K) -> Option<u64> {
        self.shared
            .fields
            .borrow()
            .iter()
            .find(|binding| binding.key == *key)
            .map(|binding| binding.epoch)
    }

    fn one_shot_epoch(&self, key: &K) -> Option<u64> {
        self.shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| binding.key == *key)
            .map(|binding| binding.epoch)
    }

    /// Submit a field edit as intent under `key`, remembering the draft it was
    /// submitted with so a later settlement cannot clobber a newer draft.
    pub fn begin_value(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        current_draft: &D,
    ) where
        K: Clone,
    {
        // A composite field action may also bind the same logical key as a one-shot
        // control. Keep its disabling lifetime inside the collection as well.
        let disabled = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.view.disabled);
        if let Some(mut disabled) = disabled {
            disabled.set(true);
        }
        self.shared
            .edits
            .borrow_mut()
            .begin(port, key.clone(), label, feature, resolver);
        let entry = Submission {
            field_epoch: self.field_epoch(&key),
            one_shot_epoch: self.one_shot_epoch(&key),
            text: Some(current_draft.clone()),
            key: key.clone(),
        };
        let mut submitted = self.shared.submitted.borrow_mut();
        match submitted.iter_mut().find(|existing| existing.key == key) {
            Some(existing) => *existing = entry,
            None => submitted.push(entry),
        }
    }

    /// Submit a one-shot edit as intent under `key` and disable its bound control until
    /// the edit settles or retires.
    pub fn begin_one_shot(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) where
        K: Clone,
    {
        let disabled = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.view.disabled);
        if let Some(mut disabled) = disabled {
            disabled.set(true);
        }
        self.shared
            .edits
            .borrow_mut()
            .begin(port, key.clone(), label, feature, resolver);
        let one_shot_epoch = self.one_shot_epoch(&key);
        let mut submitted = self.shared.submitted.borrow_mut();
        match submitted.iter_mut().find(|existing| existing.key == key) {
            Some(existing) => {
                existing.text = None;
                existing.field_epoch = None;
                existing.one_shot_epoch = one_shot_epoch;
            }
            None => submitted.push(Submission {
                key,
                text: None,
                field_epoch: None,
                one_shot_epoch,
            }),
        }
    }

    /// Whether the key's latest edit is still pending in its captured document scope.
    pub fn is_pending(&self, key: &K) -> bool {
        self.shared.edits.borrow().is_pending(key)
    }

    /// Drain terminal results once and apply the field policy:
    ///
    /// - *Landed* and *Retired* project the accepted value (via `accepted`) into the
    ///   field's draft while that draft still is the submitted value; retirement is
    ///   silent. *Failed* additionally reports the message inline. A newer draft is
    ///   never overwritten; its field still receives the failure message.
    /// - Any terminal re-enables a bound one-shot control and releases its submitted value.
    ///
    /// Every drained result is returned in drain order for caller follow-ups — Landed
    /// revisions, failure placement beyond the field, and domain projection the
    /// `accepted` closure does not cover.
    pub fn settle(
        &self,
        owner_is_live: bool,
        accepted: impl Fn(&K) -> D,
    ) -> Vec<PendingEditResult<K>> {
        let results = self.shared.edits.borrow_mut().settle(owner_is_live);

        for result in &results {
            match result {
                PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key } => {
                    self.apply_to_field(key, result, &accepted);
                    self.release_one_shot(key);
                    // Both consumers need the captured epochs before releasing this record.
                    self.shared
                        .submitted
                        .borrow_mut()
                        .retain(|submission| submission.key != *key);
                }
            }
        }
        results
    }

    /// The submission record of the key's latest edit, if one is held.
    fn submission_of(&self, key: &K) -> Option<(Option<D>, Option<u64>, Option<u64>)> {
        self.shared
            .submitted
            .borrow()
            .iter()
            .find(|existing| existing.key == *key)
            .map(|existing| {
                (
                    existing.text.clone(),
                    existing.field_epoch,
                    existing.one_shot_epoch,
                )
            })
    }

    fn apply_to_field(&self, key: &K, result: &PendingEditResult<K>, accepted: &impl Fn(&K) -> D) {
        let Some((submitted, field_epoch, _)) = self.submission_of(key) else {
            return;
        };
        let (mut view, epoch) = match self
            .shared
            .fields
            .borrow()
            .iter()
            .find(|binding| &binding.key == key)
        {
            Some(binding) => (binding.view, binding.epoch),
            None => return,
        };
        if field_epoch != Some(epoch) {
            // The edit was submitted for a binding that has since been replaced.
            return;
        }
        let message = match result {
            PendingEditResult::Failed { message, .. } => Some(message.as_str()),
            _ => None,
        };
        if let Some(message) = message {
            view.failure.set(Some(message.to_owned()));
        } else if view.failure.read().is_some() {
            view.failure.set(None);
        }
        let current = view.draft.peek().clone();
        let projected = accepted(key);
        if draft_may_restore(submitted.as_ref(), &current) && current != projected {
            view.draft.set(projected);
        }
    }

    fn release_one_shot(&self, key: &K) {
        let Some((_, _, one_shot_epoch)) = self.submission_of(key) else {
            return;
        };
        let bound = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| &binding.key == key)
            .map(|binding| (binding.view.disabled, binding.epoch));
        if let Some((mut disabled, epoch)) = bound
            && one_shot_epoch == Some(epoch)
        {
            disabled.set(false);
        }
    }
}

impl<K: PartialEq + 'static> PendingEditSignals<K> {
    /// Submit a String field's exact text, preserving the existing borrowed-str interface.
    pub fn begin_field(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        current_draft: &str,
    ) where
        K: Clone,
    {
        self.begin_value(
            port,
            key,
            label,
            feature,
            resolver,
            &current_draft.to_owned(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_requires_the_submitted_draft() {
        assert!(draft_may_restore(Some("1.5"), "1.5"));
        assert!(!draft_may_restore(Some("1.5"), "2.0"), "a newer draft wins");
        assert!(
            !draft_may_restore(None, "1.5"),
            "an unknown draft is never replaced"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "pending_edit_helpers/mounted_tests.rs"]
mod mounted_tests;
