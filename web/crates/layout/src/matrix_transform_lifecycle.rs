//! Private transform selection-retention policy, shared by the mounted controller
//! and native regressions. Draft merging remains field-local in the inspector.
#[cfg(test)]
use boardstudio_application::SnapshotToken;

#[derive(Default)]
pub struct SelectionMembership {
    pub eligible: Vec<String>,
    pub live: Vec<String>,
}

#[derive(Default)]
pub struct SelectionMembershipCache {
    source: Option<SelectionMembershipSource>,
    membership: std::rc::Rc<SelectionMembership>,
}

struct SelectionMembershipSource {
    accepted: boardstudio_application::AcceptedSnapshot,
    board_id: String,
    instance_id: Option<String>,
}

impl SelectionMembershipSource {
    fn matches(&self, model: &boardstudio_application::ReadModel) -> bool {
        model.accepted.as_ref().is_some_and(|accepted| {
            self.board_id == model.active_board_id
                && self.instance_id == model.active_instance_id
                && self.accepted.token == accepted.token
                && self.accepted.session_epoch == accepted.session_epoch
                && std::sync::Arc::ptr_eq(&self.accepted.document, &accepted.document)
                && std::sync::Arc::ptr_eq(&self.accepted.scene, &accepted.scene)
        })
    }
}

impl SelectionMembershipCache {
    pub fn project(
        &mut self,
        model: &boardstudio_application::ReadModel,
        compute: impl FnOnce(&boardstudio_application::ReadModel) -> SelectionMembership,
    ) -> std::rc::Rc<SelectionMembership> {
        if (self.source.is_none() && model.accepted.is_none())
            || self
                .source
                .as_ref()
                .is_some_and(|source| source.matches(model))
        {
            return self.membership.clone();
        }
        // Membership depends on accepted document/scene and scope, never a drag's
        // display preview, camera, selected IDs or pending operation status. Hold
        // the immutable sources strongly so pointer identity cannot be recycled.
        self.source = model
            .accepted
            .as_ref()
            .map(|accepted| SelectionMembershipSource {
                accepted: accepted.clone(),
                board_id: model.active_board_id.clone(),
                instance_id: model.active_instance_id.clone(),
            });
        self.membership = std::rc::Rc::new(if self.source.is_some() {
            compute(model)
        } else {
            SelectionMembership::default()
        });
        self.membership.clone()
    }
}

/// Remembers selected generated keys across accepted cell-disable transitions.
/// The owner supplied by the mounted Layout is its full ScopedTreeContext, so
/// another project/session/board/cell cannot inherit the remembered IDs.
pub struct SelectionRetention<K> {
    last: Option<(K, Vec<String>)>,
    suspended: Option<(K, Vec<String>)>,
}

impl<K> Default for SelectionRetention<K> {
    fn default() -> Self {
        Self {
            last: None,
            suspended: None,
        }
    }
}

impl<K: Clone + PartialEq> SelectionRetention<K> {
    pub fn reconcile(
        &mut self,
        owner: Option<&K>,
        selected_ids: &[String],
        eligible_ids: &[String],
        _live_ids: &[String],
    ) -> Vec<String> {
        let eligible = |id: &str| eligible_ids.iter().any(|candidate| candidate == id);
        if owner.is_none_or(|owner| {
            self.last
                .as_ref()
                .is_some_and(|(last_owner, _)| last_owner != owner)
                || self
                    .suspended
                    .as_ref()
                    .is_some_and(|(held_owner, _)| held_owner != owner)
        }) {
            self.last = None;
            self.suspended = None;
        }

        let reconciled = selected_ids
            .iter()
            .filter(|id| eligible(id))
            .cloned()
            .collect::<Vec<_>>();

        let Some(owner) = owner else {
            return reconciled;
        };
        if !reconciled.is_empty() {
            self.last = Some((owner.clone(), reconciled.clone()));
            self.suspended = None;
            return reconciled;
        }

        if selected_ids.is_empty()
            && self.suspended.is_none()
            && let Some((last_owner, last_ids)) = &self.last
            && last_owner == owner
        {
            if last_ids.iter().any(|id| !eligible(id)) {
                self.suspended = Some((owner.clone(), last_ids.clone()));
            } else {
                // An explicit deselection while the target is still eligible must not
                // become a future undo-driven reselection.
                self.last = None;
            }
        }

        if let Some((held_owner, held_ids)) = &self.suspended
            && held_owner == owner
        {
            let restored = held_ids
                .iter()
                .filter(|id| eligible(id))
                .cloned()
                .collect::<Vec<_>>();
            if !restored.is_empty() {
                self.last = Some((owner.clone(), restored.clone()));
                self.suspended = None;
                return restored;
            }
        }
        reconciled
    }
}

#[cfg(test)]
mod selection_retention_tests {
    use super::*;
    use boardstudio_application::{
        Completion, Event, OperationId, SaveResult, Scope, SelectionMode, Session,
    };
    use boardstudio_core::{CoreEngine, model::*};
    use std::collections::BTreeMap;

    fn advance(
        session: &mut Session,
        core: &mut CoreEngine,
        initial: Vec<boardstudio_application::Effect>,
    ) {
        let mut pending = initial;
        while let Some(effect) = pending.pop() {
            match effect {
                boardstudio_application::Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => {
                    let reply = core.handle(*request);
                    pending.extend(session.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }));
                }
                boardstudio_application::Effect::Persist {
                    save_attempt_id, ..
                } => {
                    pending.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
    }

    fn live_ids(model: &boardstudio_application::ReadModel, key_id: &str) -> Vec<String> {
        let Some(snapshot) = &model.accepted else {
            return vec![];
        };
        let Some(board) = snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == model.active_board_id)
        else {
            return vec![];
        };
        board
            .part_ids
            .iter()
            .filter(|id| {
                id.as_str() == key_id && snapshot.document.parts.iter().any(|part| &part.id == *id)
            })
            .cloned()
            .collect()
    }

    fn eligible_ids(
        model: &boardstudio_application::ReadModel,
        matrix_id: &str,
        key_id: &str,
    ) -> Vec<String> {
        let Some(snapshot) = &model.accepted else {
            return vec![];
        };
        let live = live_ids(model, key_id);
        snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .into_iter()
            .flat_map(|scene| &scene.cells)
            .filter(|cell| cell.enabled && cell.member_id.as_deref() == Some(key_id))
            .filter_map(|cell| cell.member_id.clone())
            .filter(|id| live.contains(id))
            .collect()
    }

    fn matrix_fixture() -> ProjectDoc {
        let mut document = ProjectDoc::empty("selection", "Selection");
        document.boards.push(Board {
            id: "main".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "switch".into(),
            name: "Switch".into(),
            kind: PartKind::Switch,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document
    }

    #[test]
    fn accepted_membership_reuses_preview_notifications_and_retires_on_source_change() {
        use boardstudio_application::{GestureView, SessionEpoch};
        use std::{cell::Cell, rc::Rc, sync::Arc};

        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: matrix_fixture(),
        });
        advance(&mut session, &mut core, effects);
        let mut model = session.read_model().clone();
        assert!(
            model.accepted.is_some(),
            "fixture must reach real Core acceptance"
        );
        let calls = Cell::new(0);
        let compute = |model: &boardstudio_application::ReadModel| {
            calls.set(calls.get() + 1);
            SelectionMembership {
                eligible: eligible_ids(model, "matrix-main", "key"),
                live: live_ids(model, "key"),
            }
        };
        let mut cache = SelectionMembershipCache::default();
        let first = cache.project(&model, compute);
        for generation in 1..=100 {
            model.gesture = Some(GestureView {
                pointer_id: 1,
                generation,
                target_ids: vec!["key".into()],
                changed: true,
                snap: true,
                alt: false,
            });
            model.display_preview =
                Some(Arc::new((*model.accepted.as_ref().unwrap().scene).clone()));
            model.camera.zoom += 0.01;
            model.selected_part_ids = vec![format!("selection-{generation}")];
            model.selection_anchor_id = model.selected_part_ids.first().cloned();
            let next = cache.project(&model, compute);
            assert_eq!(
                calls.get(),
                1,
                "gesture/display/selection notifications must reuse accepted membership"
            );
            assert!(Rc::ptr_eq(&first, &next));
        }
        let mut expect_refresh = |model: &boardstudio_application::ReadModel| {
            let before = calls.get();
            let refreshed = cache.project(model, compute);
            assert_eq!(
                calls.get(),
                before + 1,
                "changed membership owner must recompute"
            );
            assert!(!Rc::ptr_eq(&first, &refreshed));
        };
        model.active_board_id = "other-board".into();
        expect_refresh(&model);
        model.active_instance_id = Some("physical-instance".into());
        expect_refresh(&model);
        model.accepted.as_mut().unwrap().token = SnapshotToken(99);
        expect_refresh(&model);
        model.accepted.as_mut().unwrap().session_epoch = SessionEpoch(99);
        expect_refresh(&model);
        let accepted = model.accepted.as_mut().unwrap();
        accepted.document = Arc::new((*accepted.document).clone());
        expect_refresh(&model);
        let accepted = model.accepted.as_mut().unwrap();
        accepted.scene = Arc::new((*accepted.scene).clone());
        expect_refresh(&model);
        let accepted = model.accepted.as_mut().unwrap();
        Arc::make_mut(&mut accepted.document).revision += 1;
        expect_refresh(&model);
        model.accepted = None;
        let empty = cache.project(&model, |_| {
            panic!("closed source has no membership to project")
        });
        assert!(empty.eligible.is_empty() && empty.live.is_empty());
        let reopened = session.read_model().clone();
        let before = calls.get();
        cache.project(&reopened, compute);
        assert_eq!(
            calls.get(),
            before + 1,
            "reopen cannot inherit retired membership"
        );
    }

    #[test]
    fn core_disable_then_undo_restores_scoped_key_selection() {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: matrix_fixture(),
        });
        advance(&mut session, &mut core, effects);
        let matrix = Matrix {
            id: "matrix-main".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 0.0, y: 0.0 },
            definition_id: "switch".into(),
            part_ids: vec![],
            board_id: Some("main".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![],
        };
        let effects = session.submit(Event::Edit {
            operation_id: OperationId(2),
            command: EditCommand {
                base_revision: 0,
                transaction_id: "create-matrix".into(),
                phase: EditPhase::Commit,
                target_ids: vec![matrix.id.clone()],
                operation: EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            },
        });
        advance(&mut session, &mut core, effects);
        let key_id = "matrix/matrix-main/r0c0";
        let accepted = session.read_model().accepted.as_ref().unwrap();
        let scope = Scope {
            session_epoch: accepted.session_epoch,
            document_id: accepted.document.id.clone(),
            board_id: "main".into(),
            instance_id: None,
        };
        session.submit(Event::SelectMatrixCell {
            operation_id: OperationId(3),
            scope: scope.clone(),
            matrix_id: "matrix-main".into(),
            target_part_id: key_id.into(),
            part_ids: vec![key_id.into()],
            mode: SelectionMode::Replace,
        });
        let owner = (scope, "key:matrix-main:r0c0".to_owned());
        let mut retention = SelectionRetention::default();
        let before = session.read_model();
        let selected = before.selected_part_ids.clone();
        assert_eq!(selected, vec![key_id]);
        assert_eq!(eligible_ids(before, "matrix-main", key_id), selected);
        assert_eq!(
            retention.reconcile(
                Some(&owner),
                &selected,
                &eligible_ids(before, "matrix-main", key_id),
                &live_ids(before, key_id)
            ),
            selected
        );

        let mut disabled = before.accepted.as_ref().unwrap().document.matrices[0].clone();
        disabled.cells.push(MatrixCell {
            row: 0,
            column: 0,
            enabled: false,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        });
        let revision = before.accepted.as_ref().unwrap().document.revision;
        let effects = session.submit(Event::Edit {
            operation_id: OperationId(4),
            command: EditCommand {
                base_revision: revision,
                transaction_id: "disable-key".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["matrix-main".into()],
                operation: EditOperation::SetMatrix {
                    matrix: disabled,
                    definitions: None,
                },
            },
        });
        advance(&mut session, &mut core, effects);
        let after_disable = session.read_model();
        assert!(
            live_ids(after_disable, key_id).is_empty(),
            "Core removes the disabled generated part from the board"
        );
        assert!(eligible_ids(after_disable, "matrix-main", key_id).is_empty());
        assert!(
            after_disable.selected_part_ids.is_empty(),
            "Session prunes the selected part before presentation reconciliation"
        );
        assert!(
            retention
                .reconcile(
                    Some(&owner),
                    &after_disable.selected_part_ids,
                    &eligible_ids(after_disable, "matrix-main", key_id),
                    &live_ids(after_disable, key_id)
                )
                .is_empty()
        );

        let effects = session.submit(Event::Undo {
            operation_id: OperationId(5),
        });
        advance(&mut session, &mut core, effects);
        let after_undo = session.read_model();
        assert_eq!(live_ids(after_undo, key_id), selected);
        assert_eq!(eligible_ids(after_undo, "matrix-main", key_id), selected);
        let restored = retention.reconcile(
            Some(&owner),
            &after_undo.selected_part_ids,
            &eligible_ids(after_undo, "matrix-main", key_id),
            &live_ids(after_undo, key_id),
        );
        assert_eq!(restored, selected);
    }

    #[test]
    fn deleting_a_disabled_key_discards_its_suspended_selection() {
        let mut retention = SelectionRetention::default();
        let owner = "matrix/r0c0";
        let selected = vec!["matrix/r0c0".to_owned()];
        assert_eq!(
            retention.reconcile(Some(&owner), &selected, &selected, &selected),
            selected
        );
        assert!(retention.reconcile(None, &[], &[], &[]).is_empty());
        assert!(
            retention
                .reconcile(Some(&owner), &[], &selected, &selected)
                .is_empty()
        );
    }

    #[test]
    fn explicit_clear_and_reopened_scope_do_not_restore_old_key_ids() {
        let mut retention = SelectionRetention::default();
        let original_scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "same-project".into(),
            board_id: "main".into(),
            instance_id: None,
        };
        let owner = (original_scope.clone(), "key:matrix:r0c0".to_owned());
        let selected = vec!["matrix/matrix/r0c0".to_owned()];
        assert_eq!(
            retention.reconcile(Some(&owner), &selected, &selected, &selected),
            selected
        );

        // An explicit deselection while the Key is still eligible clears its remembered state.
        assert!(
            retention
                .reconcile(Some(&owner), &[], &selected, &selected)
                .is_empty()
        );
        assert!(retention.reconcile(Some(&owner), &[], &[], &[]).is_empty());

        // Recreate a suspended selection, then prove session and cell identity changes clear it.
        for changed_owner in [
            (
                Scope {
                    session_epoch: boardstudio_application::SessionEpoch(2),
                    ..original_scope.clone()
                },
                "key:matrix:r0c0".to_owned(),
            ),
            (original_scope.clone(), "key:matrix:r0c1".to_owned()),
        ] {
            assert_eq!(
                retention.reconcile(Some(&owner), &selected, &selected, &selected),
                selected
            );
            assert!(retention.reconcile(Some(&owner), &[], &[], &[]).is_empty());
            assert!(
                retention
                    .reconcile(Some(&changed_owner), &[], &[], &[])
                    .is_empty()
            );
            assert!(
                retention
                    .reconcile(Some(&changed_owner), &[], &selected, &selected)
                    .is_empty()
            );
        }
    }
}
