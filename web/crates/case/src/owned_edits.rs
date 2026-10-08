//! The helper observing one Case controller's edits for one owner lifetime.
//!
//! A controller's requests all belong to the Scope and Scope generation they were
//! admitted under, so a changed owner replaces the helper: the old observations retire
//! silently while their Session edits still run. Request metadata travels in the keys,
//! which compare by logical field or action; this holder only remembers which keys are
//! still observed so presentation can project their pending values.
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::cell::RefCell;

pub(crate) struct OwnedEdits<K: PartialEq + Clone + 'static> {
    pub helper: PendingEditSignals<K>,
    owner: Option<(Option<Scope>, u64)>,
    requests: Vec<K>,
    /// The bound field Signals of mounted components, kept so a replacement helper is
    /// bound the same way and the first edit after an owner change still restores its
    /// draft and places its failure.
    fields: RefCell<Vec<(K, Signal<String>, Signal<Option<String>>)>>,
}

impl<K: PartialEq + Clone + 'static> Default for OwnedEdits<K> {
    fn default() -> Self {
        Self {
            helper: PendingEditSignals::new(),
            owner: None,
            requests: Vec::new(),
            fields: RefCell::new(Vec::new()),
        }
    }
}

impl<K: PartialEq + Clone + 'static> OwnedEdits<K> {
    pub fn owner_changed(&self, scope: Option<&Scope>, generation: u64) -> bool {
        self.owner.as_ref() != Some(&(scope.cloned(), generation))
    }

    /// Adopt the current owner, retiring every observation of the previous one.
    pub fn follow_owner(&mut self, scope: Option<&Scope>, generation: u64) {
        if self.owner_changed(scope, generation) {
            if self.owner.is_some() {
                self.helper = PendingEditSignals::new();
                self.requests.clear();
                for (key, draft, failure) in self.fields.borrow().iter() {
                    self.helper.bind_field(key.clone(), *draft, *failure);
                }
            }
            self.owner = Some((scope.cloned(), generation));
        }
    }

    /// Bind a field's Signals to the current helper and remember them across owner changes.
    pub fn bind_field(&self, key: K, draft: Signal<String>, failure: Signal<Option<String>>) {
        self.helper.bind_field(key.clone(), draft, failure);
        let mut fields = self.fields.borrow_mut();
        match fields.iter_mut().find(|(existing, _, _)| *existing == key) {
            Some(entry) => {
                entry.1 = draft;
                entry.2 = failure;
            }
            None => fields.push((key, draft, failure)),
        }
    }

    /// Release a field's binding from the current helper and forget it. Idempotent.
    pub fn unbind_field(&self, key: &K) {
        self.helper.unbind_field(key);
        self.fields
            .borrow_mut()
            .retain(|(existing, _, _)| existing != key);
    }

    pub fn remember(&mut self, key: K) {
        self.requests.retain(|existing| *existing != key);
        self.requests.push(key);
    }

    pub fn is_pending(&self, key: &K) -> bool {
        self.helper.is_pending(key)
    }

    /// Keys whose edit is still pending, oldest first.
    pub fn pending(&self) -> impl Iterator<Item = &K> {
        self.requests
            .iter()
            .filter(|key| self.helper.is_pending(key))
    }

    pub fn has_terminal(&self) -> bool {
        self.requests.iter().any(|key| !self.helper.is_pending(key))
    }

    pub fn prune(&mut self) {
        let helper = self.helper.clone();
        self.requests.retain(|key| helper.is_pending(key));
    }
}
