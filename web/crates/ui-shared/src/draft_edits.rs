//! Display state carried by an edit ticket. A submission captures its exact draft and
//! the view that submitted it. Session still owns execution, even if a view detaches.

use boardstudio_application::EditResolver;
use boardstudio_web_runtime::{
    edit_ticket::EditTicketPort,
    pending_edits::{PendingEditResult, PendingEdits},
};
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
            draft: Some((value, self)),
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
    draft: Option<(D, FieldView<D>)>,
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

pub struct DraftEdits<K, D = String>(Rc<RefCell<PendingEdits<K, EditDisplay<D>>>>);

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

impl<K: PartialEq, D: Clone + PartialEq + 'static> DraftEdits<K, D> {
    pub fn submit(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        display: EditDisplay<D>,
    ) {
        if let Some(mut disabled) = display.disabled {
            disabled.set(true);
        }
        self.0
            .borrow_mut()
            .begin_with(port, key, label, feature, resolver, display);
    }

    pub fn begin(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) {
        self.submit(port, key, label, feature, resolver, EditDisplay::default());
    }

    pub fn is_pending(&self, key: &K) -> bool {
        self.0.borrow().is_pending(key)
    }

    pub fn pending_submission(&self, key: &K) -> Option<D> {
        self.0
            .borrow()
            .pending_payload(key)?
            .draft
            .as_ref()
            .map(|(draft, _)| draft.clone())
    }

    pub fn has_terminal(&self) -> bool {
        self.0.borrow().has_terminal()
    }

    pub fn observations(&self) -> std::cell::Ref<'_, PendingEdits<K, EditDisplay<D>>> {
        self.0.borrow()
    }

    /// The view has changed ownership while its signals remain alive. Preserve
    /// the terminal result for controller follow-ups, and release the field view.
    pub fn detach_field(&self, key: &K) {
        if let Some(display) = self.0.borrow_mut().payload_mut(key) {
            display.draft = None;
        }
    }

    pub(crate) fn detach_disabled(&self, key: &K) {
        if let Some(display) = self.0.borrow_mut().payload_mut(key) {
            display.disabled = None;
        }
    }

    /// Forget observations when their presentation owner changes. Session still
    /// runs the edits. The caller resets any UI it reuses for the next owner.
    pub fn clear(&self) {
        *self.0.borrow_mut() = PendingEdits::default();
    }

    /// Apply terminal display changes and return each result once for caller follow-ups.
    pub fn settle(
        &self,
        owner_is_live: bool,
        accepted: impl Fn(&K) -> D,
    ) -> Vec<PendingEditResult<K>> {
        let terminal = self.0.borrow_mut().settle_with(owner_is_live);
        terminal
            .into_iter()
            .map(|(result, display)| {
                if let Some(mut disabled) = display.disabled
                    && let Ok(mut value) = disabled.try_write()
                {
                    *value = false;
                }
                if let Some((submitted, view)) = display.draft {
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
