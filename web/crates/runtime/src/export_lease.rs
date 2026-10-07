//! Captured ownership for asynchronous exports.
//!
//! A lease is valid only while Session still owns the export and the accepted project,
//! scope, Core executor epoch, and worker identity remain the ones captured at start.

use boardstudio_application::{
    AcceptedSnapshot, ExecutorEpoch, OperationId, Scope, Session, SnapshotToken,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    ProjectCopy,
    Step,
    KeycapsStep,
    Firmware,
    Footprints,
    PcbHandoff { draft: bool },
    Mechanical { filename: String },
    BoardOutline,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExportLease {
    kind: ExportKind,
    snapshot: AcceptedSnapshot,
    scope: Scope,
    executor_epoch: ExecutorEpoch,
    worker_identity: usize,
}

impl ExportLease {
    pub(crate) fn new(
        kind: ExportKind,
        snapshot: AcceptedSnapshot,
        scope: Scope,
        executor_epoch: ExecutorEpoch,
        worker_identity: usize,
    ) -> Self {
        Self {
            kind,
            snapshot,
            scope,
            executor_epoch,
            worker_identity,
        }
    }

    pub(crate) fn kind(&self) -> &ExportKind {
        &self.kind
    }

    pub(crate) fn snapshot(&self) -> &AcceptedSnapshot {
        &self.snapshot
    }

    pub(crate) fn scope(&self) -> &Scope {
        &self.scope
    }

    pub(crate) fn token(&self) -> SnapshotToken {
        self.snapshot.token
    }

    pub(crate) fn executor_epoch(&self) -> ExecutorEpoch {
        self.executor_epoch
    }

    /// Advance ownership after an export's own accepted commit, retaining its strict origin.
    pub(crate) fn advance(&mut self, snapshot: AcceptedSnapshot) -> Result<(), String> {
        if snapshot.session_epoch != self.snapshot.session_epoch
            || snapshot.document.id != self.snapshot.document.id
            || snapshot.document.revision <= self.snapshot.document.revision
            || snapshot.scene.revision != snapshot.document.revision
        {
            return Err("Export commit did not advance the captured project revision.".into());
        }
        self.snapshot = snapshot;
        Ok(())
    }

    pub(crate) fn require_current(
        &self,
        operation_id: OperationId,
        session: &Session,
        executor_epoch: ExecutorEpoch,
        worker_identity: usize,
    ) -> Result<(), String> {
        let accepted_matches = session
            .read_model()
            .accepted
            .as_ref()
            .is_some_and(|current| self.accepted_snapshot_matches(current));
        self.require_authority(
            session.export_is_current(operation_id, self.snapshot.token, &self.scope)
                && session.scope().as_ref() == Some(&self.scope),
            accepted_matches,
            executor_epoch,
            worker_identity,
        )
    }

    fn accepted_snapshot_matches(&self, current: &AcceptedSnapshot) -> bool {
        current.session_epoch == self.snapshot.session_epoch
            && current.document.id == self.snapshot.document.id
            && current.token == self.snapshot.token
            && current.document.revision == self.snapshot.document.revision
            && current.scene.revision == self.snapshot.scene.revision
    }

    fn require_authority(
        &self,
        session_owns_export: bool,
        accepted_snapshot_matches: bool,
        executor_epoch: ExecutorEpoch,
        worker_identity: usize,
    ) -> Result<(), String> {
        if session_owns_export
            && accepted_snapshot_matches
            && executor_epoch == self.executor_epoch
            && worker_identity == self.worker_identity
        {
            Ok(())
        } else {
            Err("Export lease is no longer current.".into())
        }
    }
}

#[derive(Default)]
pub(crate) struct ExportLeases {
    active: BTreeMap<OperationId, ExportLease>,
}

impl ExportLeases {
    pub(crate) fn begin(&mut self, operation_id: OperationId, lease: ExportLease) {
        self.active.insert(operation_id, lease);
    }

    pub(crate) fn get(&self, operation_id: OperationId) -> Option<&ExportLease> {
        self.active.get(&operation_id)
    }

    pub(crate) fn get_mut(&mut self, operation_id: OperationId) -> Option<&mut ExportLease> {
        self.active.get_mut(&operation_id)
    }

    pub(crate) fn finish(&mut self, operation_id: OperationId) -> Option<ExportLease> {
        self.active.remove(&operation_id)
    }

    pub(crate) fn cancel(&mut self, operation_id: OperationId) -> Option<ExportLease> {
        self.active.remove(&operation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;

    fn captured(kind: ExportKind) -> ExportLease {
        let document = ProjectDoc::empty("project-a", "Project A");
        let scene = boardstudio_core::model::SceneDelta {
            module_scenes: Vec::new(),
            revision: document.revision,
            transaction_id: String::new(),
            changed_ids: Vec::new(),
            transforms: Vec::new(),
            matrix_scenes: Vec::new(),
            contours: Vec::new(),
            board_contours: Vec::new(),
            board_readiness: Vec::new(),
            board_outline_scenes: Vec::new(),
            finding_markers: Vec::new(),
            findings: Vec::new(),
            readiness: boardstudio_core::model::Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        };
        let snapshot = AcceptedSnapshot {
            token: SnapshotToken(17),
            session_epoch: boardstudio_application::SessionEpoch(5),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        let scope = Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: "board-a".into(),
            instance_id: None,
        };
        ExportLease::new(kind, snapshot, scope, ExecutorEpoch(11), 123)
    }

    #[test]
    fn worker_replacement_invalidates_every_export_kind() {
        for kind in [
            ExportKind::ProjectCopy,
            ExportKind::Step,
            ExportKind::KeycapsStep,
            ExportKind::Firmware,
            ExportKind::Footprints,
            ExportKind::PcbHandoff { draft: false },
            ExportKind::Mechanical {
                filename: "board-mechanical.zip".into(),
            },
            ExportKind::BoardOutline,
        ] {
            let lease = captured(kind);
            assert!(lease.require_authority(true, true, ExecutorEpoch(11), 123).is_ok());
            assert!(lease.require_authority(true, true, ExecutorEpoch(12), 123).is_err());
            assert!(lease.require_authority(true, true, ExecutorEpoch(11), 456).is_err());
            assert!(lease.require_authority(false, true, ExecutorEpoch(11), 123).is_err());
            assert!(lease.require_authority(true, false, ExecutorEpoch(11), 123).is_err());
        }
    }

    #[test]
    fn registry_finishes_and_cancels_leases_by_operation() {
        let operation = OperationId(9);
        let mut leases = ExportLeases::default();
        leases.begin(operation, captured(ExportKind::Step));
        assert!(leases.get(operation).is_some());
        assert!(leases.finish(operation).is_some());
        assert!(leases.get(operation).is_none());
        leases.begin(operation, captured(ExportKind::Firmware));
        assert!(leases.cancel(operation).is_some());
        assert!(leases.get(operation).is_none());
    }

    #[test]
    fn advancing_a_lease_requires_a_new_consistent_revision() {
        let mut lease = captured(ExportKind::PcbHandoff { draft: true });
        let mut next = lease.snapshot.clone();
        let mut document = (*next.document).clone();
        document.revision += 1;
        next.document = std::sync::Arc::new(document);
        let mut scene = (*next.scene).clone();
        scene.revision = next.document.revision;
        next.scene = std::sync::Arc::new(scene);
        next.token = SnapshotToken(18);
        assert!(lease.advance(next.clone()).is_ok());
        assert_eq!(lease.token(), SnapshotToken(18));
        assert!(lease.advance(next).is_err());
    }
}
