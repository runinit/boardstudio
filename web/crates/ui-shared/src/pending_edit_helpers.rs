//! Compatibility bindings for Layout Objects controllers. New forms attach their
//! view directly to each submission through [`crate::draft_edits::DraftEdits`].

use crate::draft_edits::{DraftEdits, EditDisplay, FieldView};
use boardstudio_application::EditResolver;
use boardstudio_web_runtime::edit_ticket::EditTicketPort;
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

type Bindings<K, V> = Rc<RefCell<Vec<(K, V)>>>;

#[derive(Clone)]
pub struct PendingEditSignals<K, D = String> {
    edits: DraftEdits<K, D>,
    fields: Bindings<K, FieldView<D>>,
    one_shots: Bindings<K, Signal<bool>>,
}

impl<K, D> Default for PendingEditSignals<K, D> {
    fn default() -> Self {
        Self {
            edits: DraftEdits::default(),
            fields: Rc::default(),
            one_shots: Rc::default(),
        }
    }
}

impl<K: PartialEq, D: Clone + PartialEq + 'static> PendingEditSignals<K, D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_field(&self, key: K, draft: Signal<D>, failure: Signal<Option<String>>) {
        bind(
            &mut self.fields.borrow_mut(),
            key,
            FieldView::new(draft, failure),
            |key| {
                self.edits.detach_field(key);
            },
        );
    }

    pub fn bind_one_shot(&self, key: K, disabled: Signal<bool>) {
        bind(&mut self.one_shots.borrow_mut(), key, disabled, |key| {
            self.edits.detach_disabled(key);
        });
    }

    pub fn unbind_field(&self, key: &K) {
        self.fields.borrow_mut().retain(|(bound, _)| bound != key);
        self.edits.detach_field(key);
    }

    pub fn begin_value(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        current_draft: &D,
    ) {
        let display = find(&self.fields.borrow(), &key)
            .map(|view| view.submitted(current_draft.clone()))
            .unwrap_or_default();
        self.submit(port, key, label, feature, resolver, display);
    }

    pub fn begin_one_shot(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) {
        self.submit(port, key, label, feature, resolver, EditDisplay::default());
    }

    fn submit(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        mut display: EditDisplay<D>,
    ) {
        if let Some(disabled) = find(&self.one_shots.borrow(), &key) {
            display = display.disabling(disabled);
        }
        self.edits
            .submit(port, key, label, feature, resolver, display);
    }

    pub fn is_pending(&self, key: &K) -> bool {
        self.edits.is_pending(key)
    }

    pub fn settle(
        &self,
        owner_is_live: bool,
        accepted: impl Fn(&K) -> D,
    ) -> Vec<PendingEditResult<K>> {
        self.edits.settle(owner_is_live, accepted)
    }
}

impl<K: PartialEq> PendingEditSignals<K> {
    pub fn begin_field(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        current_draft: &str,
    ) {
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

fn find<K: PartialEq, V: Copy>(bindings: &[(K, V)], key: &K) -> Option<V> {
    bindings
        .iter()
        .find(|(bound, _)| bound == key)
        .map(|(_, value)| *value)
}

fn bind<K: PartialEq, V: Copy + PartialEq>(
    bindings: &mut Vec<(K, V)>,
    key: K,
    value: V,
    detach: impl FnOnce(&K),
) {
    if find(bindings, &key) == Some(value) {
        return;
    }
    bindings.retain(|(bound, _)| bound != &key);
    detach(&key);
    bindings.push((key, value));
}
