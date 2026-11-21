//! The submitted values and views a panel still presents while Session owns execution.

#[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
use crate::runtime::Runtime;
use boardstudio_application::EditResolver;
use boardstudio_web_runtime::operation_outcomes::{EditOutcome, Settlement};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub struct FieldView<D = String> {
    pub draft: Signal<D>,
    pub failure: Signal<Option<String>>,
}

impl<D: 'static> PartialEq for FieldView<D> {
    fn eq(&self, other: &Self) -> bool {
        self.draft == other.draft && self.failure == other.failure
    }
}

impl<D> Copy for FieldView<D> {}

impl<D> Clone for FieldView<D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<D> FieldView<D> {
    pub fn new(draft: Signal<D>, failure: Signal<Option<String>>) -> Self {
        Self { draft, failure }
    }

    pub fn submitted(self, value: D) -> EditDisplay<D> {
        EditDisplay {
            draft: Some((value, Some(self))),
            disabled: None,
        }
    }

    /// Apply the terminal display policy to the submitting view. Conditional
    /// views may already have dropped their generational signal handles.
    pub fn restore<Q>(mut self, submitted: &Q, accepted: impl FnOnce() -> D, message: Option<&str>)
    where
        D: std::borrow::Borrow<Q> + 'static,
        Q: PartialEq + ?Sized,
    {
        let restore = self
            .draft
            .try_peek()
            .is_ok_and(|draft| std::borrow::Borrow::borrow(&*draft) == submitted);
        if restore {
            let projected = accepted();
            if projected.borrow() != submitted {
                self.draft.set(projected);
            }
        }
        if self
            .failure
            .try_peek()
            .is_ok_and(|failure| failure.as_deref() != message)
        {
            self.failure.set(message.map(str::to_owned));
        }
    }
}

pub struct EditDisplay<D> {
    draft: Option<(D, Option<FieldView<D>>)>,
    disabled: Option<Signal<bool>>,
}

impl<D> Default for EditDisplay<D> {
    fn default() -> Self {
        Self {
            draft: None,
            disabled: None,
        }
    }
}

impl<D> EditDisplay<D> {
    pub fn disabling(mut self, disabled: Signal<bool>) -> Self {
        self.disabled = Some(disabled);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PendingEditResult<K> {
    Landed { key: K, revision: u64 },
    Failed { key: K, message: String },
    Retired { key: K },
}

type Submission<K, D> = (K, EditOutcome, EditDisplay<D>);

pub struct DraftEdits<K, D = String>(Rc<RefCell<Vec<Submission<K, D>>>>);

impl<K, D> Clone for DraftEdits<K, D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<K, D> Default for DraftEdits<K, D> {
    fn default() -> Self {
        Self(Rc::default())
    }
}

impl<K: PartialEq, D> DraftEdits<K, D> {
    #[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
    pub fn submit(
        &self,
        runtime: &Rc<Runtime>,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        display: EditDisplay<D>,
    ) {
        self.track(key, runtime.resolve_edit(label, feature, resolver), display);
    }

    pub fn track(&self, key: K, outcome: EditOutcome, display: EditDisplay<D>) {
        if let Some(mut disabled) = display.disabled {
            disabled.set(true);
        }
        let mut edits = self.0.borrow_mut();
        if let Some(entry) = edits.iter_mut().find(|entry| entry.0 == key) {
            *entry = (key, outcome, display);
        } else {
            edits.push((key, outcome, display));
        }
    }

    #[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
    pub fn begin(
        &self,
        runtime: &Rc<Runtime>,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) {
        self.submit(
            runtime,
            key,
            label,
            feature,
            resolver,
            EditDisplay::default(),
        );
    }

    pub fn is_pending(&self, key: &K) -> bool {
        self.0
            .borrow()
            .iter()
            .any(|(field, outcome, _)| field == key && outcome.is_pending())
    }

    pub fn pending_submission(&self, key: &K) -> Option<D>
    where
        D: Clone,
    {
        self.0
            .borrow()
            .iter()
            .find_map(|(field, outcome, display)| {
                if field == key && outcome.is_pending() {
                    display.draft.as_ref().map(|(draft, _)| draft.clone())
                } else {
                    None
                }
            })
    }

    #[cfg(any(target_arch = "wasm32", test, feature = "test-support"))]
    pub fn submit_value(
        &self,
        runtime: &Rc<Runtime>,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        value: D,
    ) {
        self.submit(
            runtime,
            key,
            label,
            feature,
            resolver,
            EditDisplay {
                draft: Some((value, None)),
                disabled: None,
            },
        );
    }

    pub fn pending_values(&self, matches: impl Fn(&K) -> bool) -> Vec<D>
    where
        D: Clone,
    {
        self.pending_projection(|key, display| {
            display
                .draft
                .as_ref()
                .filter(|_| matches(key))
                .map(|(value, _)| value.clone())
        })
    }

    pub fn has_terminal(&self) -> bool {
        self.0
            .borrow()
            .iter()
            .any(|(_, outcome, _)| !outcome.is_pending())
    }

    pub fn observations(&self) -> Vec<(K, bool)>
    where
        K: Clone,
    {
        self.0
            .borrow()
            .iter()
            .map(|(key, outcome, _)| (key.clone(), outcome.is_pending()))
            .collect()
    }

    pub fn pending_keys(&self) -> Vec<K>
    where
        K: Clone,
    {
        self.pending_projection(|key, _| Some(key.clone()))
    }

    fn pending_projection<T>(&self, project: impl Fn(&K, &EditDisplay<D>) -> Option<T>) -> Vec<T> {
        let edits = self.0.borrow();
        let mut pending = edits
            .iter()
            .filter(|(_, outcome, _)| outcome.is_pending())
            .collect::<Vec<_>>();
        pending.sort_unstable_by_key(|(_, outcome, _)| outcome.operation());
        pending
            .into_iter()
            .filter_map(|(key, _, display)| project(key, display))
            .collect()
    }

    /// Keep the result available for follow-up after the view changes owner.
    pub fn detach_field(&self, key: &K) {
        if let Some((_, _, display)) = self.0.borrow_mut().iter_mut().find(|entry| &entry.0 == key)
        {
            display.draft = None;
        }
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    fn take_terminal(
        &self,
        owner_is_live: impl Fn(&EditDisplay<D>) -> bool,
    ) -> Vec<(PendingEditResult<K>, EditDisplay<D>)> {
        let mut states = Vec::new();
        let terminal = self
            .0
            .borrow_mut()
            .extract_if(.., |(_, outcome, display)| {
                let settlement = outcome.settlement(owner_is_live(display));
                if settlement == Settlement::Pending {
                    return false;
                }
                states.push(settlement);
                true
            })
            .collect::<Vec<_>>();
        terminal
            .into_iter()
            .zip(states)
            .map(|((key, _, display), settlement)| {
                let result = match settlement {
                    Settlement::Landed { revision } => PendingEditResult::Landed { key, revision },
                    Settlement::Failed { message } => PendingEditResult::Failed { key, message },
                    Settlement::Retired => PendingEditResult::Retired { key },
                    Settlement::Pending => unreachable!(),
                };
                (result, display)
            })
            .collect()
    }

    pub fn discard_drafts(&self, owner_is_live: bool) -> Vec<PendingEditResult<K>> {
        self.take_terminal(|_| owner_is_live)
            .into_iter()
            .map(|(result, _)| result)
            .collect()
    }

    pub fn settle_values_where(
        &self,
        owner_is_live: impl Fn(&D) -> bool,
    ) -> Vec<(PendingEditResult<K>, D)> {
        self.take_terminal(|display| {
            display
                .draft
                .as_ref()
                .is_some_and(|(value, _)| owner_is_live(value))
        })
        .into_iter()
        .filter_map(|(result, display)| Some((result, display.draft?.0)))
        .collect()
    }

    /// Apply terminal display changes and return each result once for caller follow-ups.
    pub fn settle(
        &self,
        owner_is_live: bool,
        accepted: impl Fn(&K) -> D,
    ) -> Vec<PendingEditResult<K>>
    where
        D: PartialEq + 'static,
    {
        self.take_terminal(|_| owner_is_live)
            .into_iter()
            .map(|(result, display)| {
                if let Some(mut disabled) = display.disabled
                    && let Ok(mut value) = disabled.try_write()
                {
                    *value = false;
                }
                if let Some((submitted, Some(view))) = display.draft {
                    let (message, key) = match &result {
                        PendingEditResult::Failed { message, key } => (Some(message.as_str()), key),
                        PendingEditResult::Landed { key, .. }
                        | PendingEditResult::Retired { key } => (None, key),
                    };
                    view.restore(&submitted, || accepted(key), message);
                }
                result
            })
            .collect()
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests;
