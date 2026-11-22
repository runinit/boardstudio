//! Captured ownership for asynchronous exports.
//!
//! A lease is valid only while Session still owns the export and the accepted project,
//! scope remain the ones captured at start.

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExportKind {
    ProjectCopy { embed_used_models: bool },
    Step,
    KeycapsStl,
    Firmware,
    Footprints,
    PcbHandoff { draft: bool },
    Mechanical { filename: String },
    BoardOutline { dxf: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExportLease {
    kind: ExportKind,
    snapshot: AcceptedSnapshot,
    scope: Scope,
}

impl ExportLease {
    pub(crate) fn new(kind: ExportKind, snapshot: AcceptedSnapshot, scope: Scope) -> Self {
        Self {
            kind,
            snapshot,
            scope,
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

    pub(crate) fn source_is_current(
        &self,
        current_scope: Option<&Scope>,
        current_snapshot: Option<&AcceptedSnapshot>,
    ) -> bool {
        current_scope == Some(&self.scope)
            && current_snapshot.is_some_and(|current| self.accepted_snapshot_matches(current))
    }

    /// Advance ownership after an export's own accepted commit, retaining its strict origin.
    pub(crate) fn advance(&mut self, snapshot: AcceptedSnapshot) -> Result<(), String> {
        if snapshot.session_epoch != self.snapshot.session_epoch
            || snapshot.document.id != self.snapshot.document.id
            || snapshot.document.revision < self.snapshot.document.revision
            || (snapshot.document.revision == self.snapshot.document.revision
                && snapshot.token == self.snapshot.token)
            || snapshot.scene.revision != snapshot.document.revision
        {
            return Err("Export commit did not advance the captured project revision.".into());
        }
        self.snapshot = snapshot;
        Ok(())
    }

    pub(crate) fn require_current(
        &self,
        session_owns_export: bool,
        current_snapshot: Option<&AcceptedSnapshot>,
    ) -> Result<(), String> {
        if session_owns_export && self.source_is_current(Some(&self.scope), current_snapshot) {
            Ok(())
        } else {
            Err("Export lease is no longer current.".into())
        }
    }

    fn accepted_snapshot_matches(&self, current: &AcceptedSnapshot) -> bool {
        current.session_epoch == self.snapshot.session_epoch
            && current.document.id == self.snapshot.document.id
            && current.token == self.snapshot.token
            && current.document.revision == self.snapshot.document.revision
            && current.scene.revision == self.snapshot.scene.revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::ProjectDoc;

    fn captured(kind: ExportKind) -> ExportLease {
        let document = ProjectDoc::empty("project-a", "Project A");
        let scene = boardstudio_model::model::SceneDelta {
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
            readiness: boardstudio_model::model::Readiness {
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
        ExportLease::new(kind, snapshot, scope)
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
