//! The helper observing one controller's edits for one owner lifetime.
//!
//! A controller's requests all belong to the Scope and Scope generation they were
//! admitted under. A changed owner retires the observations silently while their
//! Session edits still run. The controller can attach the same mounted field to a new submission.
use crate::draft_edits::DraftEdits;
use boardstudio_application::Scope;

pub struct OwnedEdits<K: PartialEq + 'static> {
    pub edits: DraftEdits<K>,
    owner: Option<(Option<Scope>, u64)>,
}

impl<K: PartialEq + 'static> Default for OwnedEdits<K> {
    fn default() -> Self {
        Self {
            edits: DraftEdits::default(),
            owner: None,
        }
    }
}

impl<K: PartialEq + 'static> OwnedEdits<K> {
    pub fn owner_changed(&self, scope: Option<&Scope>, generation: u64) -> bool {
        !self
            .owner
            .as_ref()
            .is_some_and(|(owner_scope, owner_generation)| {
                owner_scope.as_ref() == scope && *owner_generation == generation
            })
    }

    /// Adopt the current owner, retiring every observation of the previous one.
    pub fn follow_owner(&mut self, scope: Option<&Scope>, generation: u64) {
        if !self.owner_changed(scope, generation) {
            return;
        }
        if self.owner.replace((scope.cloned(), generation)).is_some() {
            self.edits.clear();
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "owned_edits_tests.rs"]
mod tests;
