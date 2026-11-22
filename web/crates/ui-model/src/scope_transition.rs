use boardstudio_application::{Scope, SnapshotToken};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeTransition {
    pub previous_scope: Option<Scope>,
    pub previous_token: Option<SnapshotToken>,
    pub next_scope: Option<Scope>,
    pub next_token: Option<SnapshotToken>,
    pub previous_generation: u64,
    pub next_generation: u64,
}

impl ScopeTransition {
    pub fn matches_current(
        &self,
        observed: Option<&Self>,
        accepted_token: Option<SnapshotToken>,
        generation: u64,
    ) -> bool {
        observed == Some(self)
            && accepted_token == self.next_token
            && generation == self.next_generation
    }
}
