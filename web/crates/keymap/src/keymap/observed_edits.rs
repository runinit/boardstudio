//! The latest observed edit per logical key, paired with the request that produced it.
//!
//! Keymap controllers keep their request and owner metadata beside the shared
//! `PendingEdits` collection. Owner liveness differs per request, so `settle` reports
//! every terminal result with its request and the controller decides whether that
//! request's owner is still current.
use boardstudio_application::{EditResolver, OperationId};
use boardstudio_web_runtime::edit_ticket::EditTicketPort;
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};

pub(super) struct Observation<K, M> {
    pub key: K,
    pub meta: M,
    pub operation: OperationId,
}

pub(super) struct ObservedEdits<K, M> {
    edits: PendingEdits<K>,
    observations: Vec<Observation<K, M>>,
}

impl<K, M> Default for ObservedEdits<K, M> {
    fn default() -> Self {
        Self {
            edits: PendingEdits::default(),
            observations: Vec::new(),
        }
    }
}

impl<K: PartialEq + Clone, M> ObservedEdits<K, M> {
    /// Observe `resolver` under `key`; the newest observation replaces the old one.
    pub fn begin(
        &mut self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: &str,
        meta: M,
        resolver: EditResolver,
    ) -> OperationId {
        let operation = self
            .edits
            .begin(port, key.clone(), label, Some(feature.into()), resolver);
        self.observations.retain(|existing| existing.key != key);
        self.observations.push(Observation {
            key,
            meta,
            operation,
        });
        operation
    }

    pub fn is_pending(&self, key: &K) -> bool {
        self.edits.is_pending(key)
    }

    /// Observations whose edit is still pending.
    pub fn pending(&self) -> impl Iterator<Item = &Observation<K, M>> {
        self.observations
            .iter()
            .filter(|observation| self.edits.is_pending(&observation.key))
    }

    pub fn has_terminal(&self) -> bool {
        self.observations
            .iter()
            .any(|observation| !self.edits.is_pending(&observation.key))
    }

    /// Drain terminal results, each with the observation that produced it.
    pub fn settle(&mut self) -> Vec<(Observation<K, M>, PendingEditResult<K>)> {
        let results = self.edits.settle(true);
        let mut settled = Vec::with_capacity(results.len());
        for result in results {
            let key = match &result {
                PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key } => key,
            };
            if let Some(index) = self
                .observations
                .iter()
                .position(|observation| observation.key == *key)
            {
                settled.push((self.observations.remove(index), result));
            }
        }
        settled
    }
}
