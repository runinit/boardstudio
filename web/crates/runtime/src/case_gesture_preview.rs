//! Transient Case CAD preview ownership for direct-manipulation gestures.
//!
//! Accepted Case CAD scenes remain owned by Runtime's generation path. This
//! state can publish only a disposable, same-revision scene for the active
//! gesture and retires it as soon as that gesture moves on or ends.

use boardstudio_application::{Scope, SnapshotToken};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseGesturePreviewOwner {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Default)]
pub struct CaseGesturePreviewState {
    generation: u64,
    active: Option<CaseGesturePreviewOwner>,
    pending: Option<CaseGesturePreviewOwner>,
    published: Option<(CaseGesturePreviewOwner, Rc<crate::runtime::CadScene>)>,
    displayed: Option<(Scope, SnapshotToken, u64, Rc<crate::runtime::CadScene>)>,
    error: Option<(CaseGesturePreviewOwner, String)>,
}

impl CaseGesturePreviewState {
    pub fn begin(
        &mut self,
        scope: Scope,
        snapshot_token: SnapshotToken,
        revision: u64,
    ) -> Result<CaseGesturePreviewOwner, String> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| "Case gesture preview generation exhausted".to_owned())?;
        self.generation = generation;
        let owner = CaseGesturePreviewOwner {
            scope,
            snapshot_token,
            revision,
            generation,
        };
        if self
            .displayed
            .as_ref()
            .is_some_and(|(scope, token, revision, _)| {
                scope != &owner.scope
                    || *token != owner.snapshot_token
                    || *revision != owner.revision
            })
        {
            self.displayed = None;
        }
        self.active = Some(owner.clone());
        self.pending = Some(owner.clone());
        self.published = None;
        self.error = None;
        Ok(owner)
    }

    pub fn is_current(&self, owner: &CaseGesturePreviewOwner) -> bool {
        self.active.as_ref() == Some(owner) && self.generation == owner.generation
    }

    pub fn active_owner(&self) -> Option<CaseGesturePreviewOwner> {
        self.active.clone()
    }

    pub fn pending(&self, owner: &CaseGesturePreviewOwner) -> bool {
        self.is_current(owner) && self.pending.as_ref() == Some(owner)
    }

    pub fn publish(
        &mut self,
        owner: &CaseGesturePreviewOwner,
        scene: Rc<crate::runtime::CadScene>,
    ) -> bool {
        if !self.pending(owner) {
            return false;
        }
        self.pending = None;
        self.error = None;
        self.displayed = Some((
            owner.scope.clone(),
            owner.snapshot_token,
            owner.revision,
            scene.clone(),
        ));
        self.published = Some((owner.clone(), scene));
        true
    }

    pub fn fail(&mut self, owner: &CaseGesturePreviewOwner, error: String) {
        if self.pending(owner) {
            self.pending = None;
            self.error = Some((owner.clone(), error));
        }
    }

    pub fn scene(
        &self,
        scope: &Scope,
        snapshot_token: SnapshotToken,
        revision: u64,
    ) -> Option<Rc<crate::runtime::CadScene>> {
        self.displayed
            .as_ref()
            .filter(|(owner_scope, owner_token, owner_revision, _)| {
                self.active.is_some()
                    && owner_scope == scope
                    && *owner_token == snapshot_token
                    && *owner_revision == revision
            })
            .map(|(_, _, _, scene)| scene.clone())
    }

    pub fn message(&self, owner: &CaseGesturePreviewOwner) -> Option<String> {
        if !self.is_current(owner) {
            return None;
        }
        if self.pending(owner) {
            return Some("Updating Case preview…".into());
        }
        self.error
            .as_ref()
            .filter(|(current, _)| current == owner)
            .map(|(_, error)| format!("Case preview update failed: {error}"))
    }

    /// Retire only the specified gesture. A late cleanup from an old viewer
    /// cannot cancel a newer gesture on the same accepted source.
    pub fn cancel(&mut self, expected: &CaseGesturePreviewOwner) -> bool {
        if !self.is_current(expected) {
            return false;
        }
        self.active = None;
        self.pending = None;
        self.published = None;
        self.displayed = None;
        self.error = None;
        self.generation = self.generation.saturating_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CadScene;
    use boardstudio_application::AcceptedSnapshot;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{PreparedCaseAssemblyIR, ProjectDoc, Readiness, SceneDelta};
    use boardstudio_web_host::cad_jobs::CadResult;
    use std::sync::Arc;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: Some("left".into()),
        }
    }

    fn scene(scope: &Scope, token: SnapshotToken, revision: u64) -> Rc<CadScene> {
        let mut document = ProjectDoc::empty(&scope.document_id, "gesture preview");
        document.revision = revision;
        let snapshot = AcceptedSnapshot {
            token,
            session_epoch: scope.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision,
                transaction_id: String::new(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: true,
                    outline: true,
                    pcb: true,
                    case_ready: false,
                },
            }),
        };
        Rc::new(CadScene {
            scope: scope.clone(),
            token,
            snapshot,
            result: CadResult::default(),
            prepared: PreparedCaseAssemblyIR {
                revision,
                bodies: vec![],
            },
            physical_fingerprint: None,
            mechanical: None,
            exact: false,
            contours: vec![],
        })
    }

    #[wasm_bindgen_test]
    fn a_newer_move_retires_the_prior_preview_owner_and_cleanup_is_scoped() {
        let mut state = CaseGesturePreviewState::default();
        let first = state.begin(scope(), SnapshotToken(7), 12).unwrap();
        let first_scene = scene(&first.scope, first.snapshot_token, first.revision);
        assert!(state.publish(&first, first_scene.clone()));
        let second = state.begin(scope(), SnapshotToken(7), 12).unwrap();

        assert_ne!(first.generation, second.generation);
        assert!(!state.pending(&first));
        assert!(state.pending(&second));
        assert!(
            state
                .scene(&second.scope, second.snapshot_token, second.revision)
                .is_some_and(|shown| Rc::ptr_eq(&shown, &first_scene))
        );
        assert!(!state.publish(&first, scene(&first.scope, first.snapshot_token, 12)));
        assert!(!state.cancel(&first));
        assert!(state.pending(&second));
        assert!(state.cancel(&second));
        assert!(!state.is_current(&second));
        assert!(
            state
                .scene(&second.scope, second.snapshot_token, second.revision)
                .is_none()
        );
    }
}
