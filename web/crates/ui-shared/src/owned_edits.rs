//! The helper observing one controller's edits for one owner lifetime.
//!
//! A controller's requests all belong to the Scope and Scope generation they were
//! admitted under. A changed owner retires the observations silently while their
//! Session edits still run. Mounted fields keep their bindings across owner changes.
use crate::pending_edit_helpers::PendingEditSignals;
use boardstudio_application::Scope;

pub struct OwnedEdits<K: PartialEq + 'static> {
    pub helper: PendingEditSignals<K>,
    owner: Option<(Option<Scope>, u64)>,
}

impl<K: PartialEq + 'static> Default for OwnedEdits<K> {
    fn default() -> Self {
        Self {
            helper: PendingEditSignals::new(),
            owner: None,
        }
    }
}

impl<K: PartialEq + 'static> OwnedEdits<K> {
    pub fn owner_changed(&self, scope: Option<&Scope>, generation: u64) -> bool {
        self.owner.as_ref() != Some(&(scope.cloned(), generation))
    }

    /// Adopt the current owner, retiring every observation of the previous one.
    pub fn follow_owner(&mut self, scope: Option<&Scope>, generation: u64) {
        if self.owner_changed(scope, generation) {
            if self.owner.is_some() {
                self.helper.retire_owner();
            }
            self.owner = Some((scope.cloned(), generation));
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "owned_edits_tests.rs"]
mod tests;
