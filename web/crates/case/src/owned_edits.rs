//! The helper observing one Case controller's edits for one owner lifetime.
//!
//! A controller's requests all belong to the Scope and Scope generation they were
//! admitted under, so a changed owner replaces the helper: the old observations retire
//! silently while their Session edits still run. Request metadata travels in the keys,
//! which compare by logical field or action; this holder only remembers which keys are
//! still observed so presentation can project their pending values.
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;

pub(crate) struct OwnedEdits<K: PartialEq + Clone + 'static> {
    pub helper: PendingEditSignals<K>,
    owner: Option<(Option<Scope>, u64)>,
    requests: Vec<K>,
}

impl<K: PartialEq + Clone + 'static> Default for OwnedEdits<K> {
    fn default() -> Self {
        Self {
            helper: PendingEditSignals::new(),
            owner: None,
            requests: Vec::new(),
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
            }
            self.owner = Some((scope.cloned(), generation));
        }
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
