//! Immutable selected-board firmware-position values projected from an accepted electrical plan.
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::{electrical::ElectricalPlan, model::ProjectDoc};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePlanIdentity {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub executor_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionIdentity {
    pub plan: FirmwarePlanIdentity,
    pub ui_scope: Scope,
    pub scope_generation: u64,
}

/// Stable presentation target for an accepted edit. Plan tokens and revisions are deliberately
/// excluded so a successful operation remains visible while Runtime refreshes its plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionFeedbackTarget {
    pub ui_scope: Scope,
    pub scope_generation: u64,
    pub key_id: String,
}

impl FirmwarePositionFeedbackTarget {
    pub fn is_visible(
        &self,
        current_scope: &Scope,
        current_generation: u64,
        current_projection: &FirmwarePositionProjection,
    ) -> bool {
        self.ui_scope == *current_scope
            && self.scope_generation == current_generation
            && current_projection
                .keys
                .iter()
                .any(|key| key.id == self.key_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionKey {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FirmwarePositionState {
    Idle,
    Pending,
    Current,
    Failed(String),
}

/// Immutable board values and plan lifecycle supplied to the future F6-owned control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionProjection {
    pub identity: Option<FirmwarePositionIdentity>,
    pub state: FirmwarePositionState,
    pub keys: Rc<[FirmwarePositionKey]>,
    pub bindings: Rc<BTreeMap<String, String>>,
}

pub struct FirmwarePositionAdmission<'a> {
    pub workspace: &'a str,
    pub current_generation: u64,
    pub instance_is_current: bool,
    pub runtime_scope: Option<&'a Scope>,
    pub accepted: &'a AcceptedSnapshot,
    pub executor_epoch: u64,
    pub current_plan: Option<&'a FirmwarePlanIdentity>,
    pub current_projection: &'a FirmwarePositionProjection,
}

pub fn admits_edit(
    identity: &FirmwarePositionIdentity,
    key_id: &str,
    admission: FirmwarePositionAdmission<'_>,
) -> bool {
    let normalized_scope = Scope {
        instance_id: None,
        ..identity.ui_scope.clone()
    };
    admission.workspace == "PCB"
        && admission.instance_is_current
        && admission.current_generation == identity.scope_generation
        && admission.runtime_scope == Some(&identity.ui_scope)
        && admission.accepted.session_epoch == identity.ui_scope.session_epoch
        && admission.accepted.document.id == identity.ui_scope.document_id
        && admission.accepted.document.revision == identity.plan.revision
        && admission.accepted.scene.revision == admission.accepted.document.revision
        && identity.plan.scope == normalized_scope
        && identity.plan.token == admission.accepted.token
        && identity.plan.executor_epoch == admission.executor_epoch
        && admission.current_plan == Some(&identity.plan)
        && admission
            .accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == identity.ui_scope.board_id)
        && admission.current_projection.identity.as_ref() == Some(identity)
        && admission
            .current_projection
            .keys
            .iter()
            .any(|key| key.id == key_id)
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlanLifecycle<'a> {
    Idle,
    Pending(&'a FirmwarePlanIdentity),
    Current(&'a FirmwarePlanIdentity, &'a ElectricalPlan),
    Failed(&'a FirmwarePlanIdentity, &'a str),
}

pub fn project(
    document: &ProjectDoc,
    identity: &FirmwarePlanIdentity,
    ui_scope: &Scope,
    scope_generation: u64,
    lifecycle: PlanLifecycle<'_>,
) -> FirmwarePositionProjection {
    let bindings = board_bindings(document, &identity.scope.board_id);
    let (state, keys, current_identity) = match lifecycle {
        PlanLifecycle::Idle => (FirmwarePositionState::Idle, Vec::new(), None),
        PlanLifecycle::Pending(pending) if pending == identity => {
            (FirmwarePositionState::Pending, Vec::new(), None)
        }
        PlanLifecycle::Failed(failed, message) if failed == identity => (
            FirmwarePositionState::Failed(message.to_owned()),
            Vec::new(),
            None,
        ),
        PlanLifecycle::Current(current, plan)
            if current == identity
                && plan.board_id.as_deref() == Some(identity.scope.board_id.as_str())
                && plan.revision == identity.revision
                && plan.instance_id.is_none() =>
        {
            let keys = project_keys(document, plan);
            (
                FirmwarePositionState::Current,
                keys,
                Some(FirmwarePositionIdentity {
                    plan: identity.clone(),
                    ui_scope: ui_scope.clone(),
                    scope_generation,
                }),
            )
        }
        _ => (FirmwarePositionState::Idle, Vec::new(), None),
    };
    FirmwarePositionProjection {
        identity: current_identity,
        state,
        keys: keys.into(),
        bindings: Rc::new(bindings),
    }
}

fn board_bindings(document: &ProjectDoc, board_id: &str) -> BTreeMap<String, String> {
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        })
        .map(|configuration| configuration.key_bindings.clone())
        .unwrap_or_default()
}

fn project_keys(document: &ProjectDoc, plan: &ElectricalPlan) -> Vec<FirmwarePositionKey> {
    let mut keys = plan
        .assignments
        .iter()
        .map(|assignment| FirmwarePositionKey {
            id: assignment.key_id.clone(),
            label: part_reference(document, &assignment.key_id),
        })
        .collect::<Vec<_>>();
    keys.extend(plan.peripherals.iter().filter_map(|peripheral| {
        let supports_push = matches!(peripheral.kind.as_str(), "encoder" | "press")
            && peripheral
                .gpio_terminals
                .iter()
                .any(|(terminal, function)| {
                    terminal == "S1"
                        || function.ends_with("/encoder-push")
                        || function.ends_with("/input-push")
                });
        let id = peripheral.press_key_id.as_ref().filter(|_| supports_push)?;
        Some(FirmwarePositionKey {
            id: id.clone(),
            label: format!("{} push", part_reference(document, &peripheral.part_id)),
        })
    }));
    keys
}

fn part_reference(document: &ProjectDoc, part_id: &str) -> String {
    document
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .map_or_else(|| part_id.to_owned(), |part| part.reference.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken, TerminalOutcome};
    use boardstudio_core::electrical::{ElectricalMode, ElectricalPlanRequest};

    fn document() -> ProjectDoc {
        serde_json::from_str(include_str!(
            "../../../../core/tests/fixtures/reviung41-outline-original.json"
        ))
        .expect("checked-in Reviung fixture is a valid saved project")
    }

    fn identity(document: &ProjectDoc) -> FirmwarePlanIdentity {
        FirmwarePlanIdentity {
            scope: Scope {
                session_epoch: SessionEpoch(7),
                document_id: document.id.clone(),
                board_id: "main".into(),
                instance_id: None,
            },
            token: SnapshotToken(11),
            revision: document.revision,
            executor_epoch: 2,
        }
    }

    #[test]
    fn settled_feedback_survives_plan_refresh_but_not_target_or_key_changes() {
        let document = document();
        let identity = identity(&document);
        let plan = resolve(&document);
        let ui_scope = Scope {
            instance_id: Some("main".into()),
            ..identity.scope.clone()
        };
        let projection = project(
            &document,
            &identity,
            &ui_scope,
            5,
            PlanLifecycle::Current(&identity, &plan),
        );
        let target = FirmwarePositionFeedbackTarget {
            ui_scope: ui_scope.clone(),
            scope_generation: 5,
            key_id: projection.keys[0].id.clone(),
        };

        // Plan token/revision are intentionally outside this visibility target.
        let mut refreshed = identity.clone();
        refreshed.token = SnapshotToken(12);
        refreshed.revision += 1;
        let mut refreshed_plan = resolve(&document);
        refreshed_plan.revision = refreshed.revision;
        let refreshed_projection = project(
            &document,
            &refreshed,
            &ui_scope,
            5,
            PlanLifecycle::Current(&refreshed, &refreshed_plan),
        );
        assert!(target.is_visible(&ui_scope, 5, &refreshed_projection));

        let other_board = Scope {
            board_id: "other".into(),
            ..ui_scope.clone()
        };
        assert!(!target.is_visible(&other_board, 5, &refreshed_projection));
        let other_instance = Scope {
            instance_id: Some("other-instance".into()),
            ..ui_scope.clone()
        };
        assert!(!target.is_visible(&other_instance, 5, &refreshed_projection));
        assert!(!target.is_visible(&ui_scope, 6, &refreshed_projection));

        let no_key_projection = FirmwarePositionProjection {
            keys: Rc::from([]),
            ..refreshed_projection
        };
        assert!(!target.is_visible(&ui_scope, 5, &no_key_projection));
    }

    fn resolve(document: &ProjectDoc) -> ElectricalPlan {
        boardstudio_core::electrical::resolve(ElectricalPlanRequest {
            document: document.clone(),
            instance_id: None,
            mode: ElectricalMode::Matrix,
            locks: BTreeMap::new(),
            controller_profile: None,
            board_id: Some("main".into()),
            controller_part_id: Some("main/U1".into()),
        })
    }

    fn core_effect(
        effects: &[boardstudio_application::Effect],
    ) -> (
        boardstudio_application::RequestId,
        boardstudio_application::ExecutorEpoch,
        boardstudio_core::model::CoreRequest,
    ) {
        effects
            .iter()
            .find_map(|effect| match effect {
                boardstudio_application::Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => Some((*request_id, *executor_epoch, (**request).clone())),
                _ => None,
            })
            .expect("session emits a Core request")
    }

    fn save_effect(
        effects: &[boardstudio_application::Effect],
    ) -> (boardstudio_application::SaveAttemptId, ProjectDoc) {
        effects
            .iter()
            .find_map(|effect| match effect {
                boardstudio_application::Effect::Persist {
                    save_attempt_id,
                    document,
                    ..
                } => Some((*save_attempt_id, (**document).clone())),
                _ => None,
            })
            .expect("session emits a persistence request")
    }

    fn open_ready_session() -> (
        boardstudio_application::Session,
        boardstudio_core::CoreEngine,
    ) {
        use boardstudio_application::{
            Completion, Effect, Event, OperationId, SaveResult, Session,
        };
        let mut session = Session::new();
        let mut engine = boardstudio_core::CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document: document(),
        });
        let (request_id, executor_epoch, request) = core_effect(&effects);
        let reply = engine.handle(request);
        let effects = session.complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(reply),
        });
        let (save_attempt_id, _) = save_effect(&effects);
        let effects = session.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Settled {
                operation_id: OperationId(1),
                outcome: TerminalOutcome::Completed,

                ..
            }
        )));
        (session, engine)
    }

    #[test]
    fn admission_and_exact_session_core_outcome_follow_the_accepted_board() {
        use boardstudio_application::{Completion, Effect, Event, OperationId, SaveResult};
        use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};

        let (mut session, mut engine) = open_ready_session();
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let runtime_scope = session.scope().unwrap();
        let board_scope = Scope {
            instance_id: None,
            ..runtime_scope.clone()
        };
        let plan_identity = FirmwarePlanIdentity {
            scope: board_scope,
            token: snapshot.token,
            revision: snapshot.document.revision,
            executor_epoch: session.core_executor_epoch().0,
        };
        let plan = resolve(&snapshot.document);
        let projection = project(
            &snapshot.document,
            &plan_identity,
            &runtime_scope,
            3,
            PlanLifecycle::Current(&plan_identity, &plan),
        );
        let key_id = projection
            .keys
            .first()
            .expect("fixture plan has a key")
            .id
            .clone();
        let identity = projection.identity.as_ref().unwrap();
        assert!(admits_edit(
            identity,
            &key_id,
            FirmwarePositionAdmission {
                workspace: "PCB",
                current_generation: 3,
                instance_is_current: true,
                runtime_scope: Some(&runtime_scope),
                accepted: &snapshot,
                executor_epoch: session.core_executor_epoch().0,
                current_plan: Some(&plan_identity),
                current_projection: &projection,
            },
        ));
        assert!(!admits_edit(
            identity,
            &key_id,
            FirmwarePositionAdmission {
                workspace: "PCB",
                current_generation: 3,
                instance_is_current: true,
                runtime_scope: Some(&Scope {
                    board_id: "stale-board".into(),
                    ..runtime_scope.clone()
                }),
                accepted: &snapshot,
                executor_epoch: session.core_executor_epoch().0,
                current_plan: Some(&plan_identity),
                current_projection: &projection,
            },
        ));
        assert!(!admits_edit(
            identity,
            &key_id,
            FirmwarePositionAdmission {
                workspace: "PCB",
                current_generation: 3,
                instance_is_current: true,
                runtime_scope: Some(&runtime_scope),
                accepted: &snapshot,
                executor_epoch: session.core_executor_epoch().0 + 1,
                current_plan: Some(&plan_identity),
                current_projection: &projection,
            },
        ));

        let outcomes = crate::operation_outcomes::OperationOutcomes::default();
        let operation_id = OperationId(2);
        let outcome = outcomes.observe(operation_id);
        let effects = session.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "firmware-position-test".into(),
                phase: EditPhase::Commit,
                target_ids: vec![runtime_scope.board_id.clone(), key_id.clone()],
                operation: EditOperation::SetKeyBinding {
                    board_id: runtime_scope.board_id.clone(),
                    key_id: key_id.clone(),
                    binding: "&kp Q".into(),
                },
            },
        });
        let (request_id, executor_epoch, request) = core_effect(&effects);
        let reply = engine.handle(request);
        let effects = session.complete(Completion::Core {
            request_id,
            executor_epoch,
            reply: Box::new(reply),
        });
        let (save_attempt_id, pending_document) = save_effect(&effects);
        assert!(
            outcome.borrow().is_none(),
            "Core acceptance alone is not a saved outcome"
        );
        assert_eq!(pending_document.revision, snapshot.document.revision + 1);

        let effects = session.complete(Completion::Persist {
            save_attempt_id,
            result: SaveResult::Committed,
        });
        let terminal = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Settled {
                    operation_id: id,
                    outcome,
                    ..
                } if *id == operation_id => Some(outcome.clone()),
                _ => None,
            })
            .expect("the exact submitted operation settles");
        assert!(outcomes.settle(operation_id, terminal));
        assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
        let accepted = session.read_model().accepted.as_ref().unwrap();
        assert_eq!(accepted.document.revision, snapshot.document.revision + 1);
        assert_eq!(
            accepted
                .document
                .hardware
                .as_ref()
                .unwrap()
                .boards
                .iter()
                .find(|board| board.board_id == identity.ui_scope.board_id)
                .unwrap()
                .key_bindings
                .get(&key_id)
                .map(String::as_str),
            Some("&kp Q"),
        );
    }

    #[test]
    fn current_real_plan_preserves_matrix_order_labels_and_selected_board_bindings() {
        let mut document = document();
        document
            .hardware
            .as_mut()
            .unwrap()
            .boards
            .iter_mut()
            .find(|board| board.board_id == "main")
            .unwrap()
            .key_bindings
            .insert("matrix/main-right-keys/r0c0".into(), "&kp Q".into());
        let identity = identity(&document);
        let plan = resolve(&document);
        assert!(!plan.assignments.is_empty());
        let ui_scope = Scope {
            instance_id: Some("main".into()),
            ..identity.scope.clone()
        };
        let projection = project(
            &document,
            &identity,
            &ui_scope,
            5,
            PlanLifecycle::Current(&identity, &plan),
        );
        assert_eq!(projection.state, FirmwarePositionState::Current);
        assert_eq!(projection.keys.len(), plan.assignments.len());
        assert_eq!(projection.keys[0].id, plan.assignments[0].key_id);
        assert_eq!(projection.keys[0].label, "main-right-keys-SW1");
        assert_eq!(projection.bindings["matrix/main-right-keys/r0c0"], "&kp Q");
        for peripheral in &plan.peripherals {
            let supported_role = matches!(peripheral.kind.as_str(), "encoder" | "press")
                && peripheral
                    .gpio_terminals
                    .iter()
                    .any(|(terminal, function)| {
                        terminal == "S1"
                            || function.ends_with("/encoder-push")
                            || function.ends_with("/input-push")
                    });
            match (supported_role, peripheral.press_key_id.as_ref()) {
                (true, Some(id)) => assert!(projection.keys.iter().any(|key| key.id == *id)),
                _ => assert!(
                    projection
                        .keys
                        .iter()
                        .all(|key| key.id != format!("{}/push", peripheral.part_id))
                ),
            }
        }
        assert_eq!(
            projection
                .identity
                .as_ref()
                .unwrap()
                .ui_scope
                .instance_id
                .as_deref(),
            Some("main")
        );
        assert_eq!(projection.identity.as_ref().unwrap().scope_generation, 5);
    }

    #[test]
    fn lifecycle_and_mismatched_plan_never_look_like_current_empty_inputs() {
        let document = document();
        let identity = identity(&document);
        let empty = ElectricalPlan {
            assignments: Vec::new(),
            board_id: Some("main".into()),
            revision: document.revision,
            instance_id: None,
            ..resolve(&document)
        };
        assert_eq!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Current(&identity, &empty)
            )
            .state,
            FirmwarePositionState::Current
        );
        assert!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Current(&identity, &empty)
            )
            .keys
            .is_empty()
        );
        assert_eq!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Pending(&identity)
            )
            .state,
            FirmwarePositionState::Pending
        );
        assert_eq!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Failed(&identity, "resolver failed")
            )
            .state,
            FirmwarePositionState::Failed("resolver failed".into())
        );
        assert_eq!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Idle
            )
            .state,
            FirmwarePositionState::Idle
        );
        let stale = FirmwarePlanIdentity {
            revision: identity.revision + 1,
            ..identity.clone()
        };
        assert_eq!(
            project(
                &document,
                &identity,
                &identity.scope,
                0,
                PlanLifecycle::Current(&stale, &empty)
            )
            .state,
            FirmwarePositionState::Idle
        );
    }

    #[test]
    fn executor_restart_invalidates_same_accepted_firmware_identity() {
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let before_restart = FirmwarePlanIdentity {
            scope: scope.clone(),
            token: SnapshotToken(11),
            revision: 13,
            executor_epoch: 17,
        };
        let after_restart = FirmwarePlanIdentity {
            executor_epoch: 18,
            ..before_restart.clone()
        };

        assert_ne!(before_restart, after_restart);
        assert_eq!(before_restart.scope, after_restart.scope);
        assert_eq!(before_restart.token, after_restart.token);
        assert_eq!(before_restart.revision, after_restart.revision);
    }
}
