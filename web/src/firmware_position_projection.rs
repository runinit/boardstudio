//! Immutable selected-board firmware-position values projected from an accepted electrical plan.
use boardstudio_application::{Scope, SnapshotToken, TerminalOutcome};
use boardstudio_core::{electrical::ElectricalPlan, model::ProjectDoc};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwarePlanIdentity {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub executor_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwarePositionIdentity {
    pub plan: FirmwarePlanIdentity,
    pub ui_scope: Scope,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwarePositionKey {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FirmwarePositionState {
    Idle,
    Pending,
    Current,
    Failed(String),
}

/// Immutable board values and plan lifecycle supplied to the future F6-owned control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FirmwarePositionProjection {
    pub identity: Option<FirmwarePositionIdentity>,
    pub state: FirmwarePositionState,
    pub keys: Rc<[FirmwarePositionKey]>,
    pub bindings: Rc<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditSettlement {
    Wait,
    Saved,
    Failed(String),
    Suppress,
}

pub(crate) struct EditSettlementSource<'a> {
    pub target_is_current: bool,
    pub accepted_is_saved: bool,
    pub accepted_revision: u64,
    pub base_revision: u64,
    pub durability_failure: Option<&'a str>,
    pub accepted_value: Option<&'a str>,
    pub requested_value: &'a str,
}

/// Admission identity is intentionally absent here: after submission, plan/source refresh cannot
/// orphan the exact operation. Only the stable target gates feedback visibility.
pub(crate) fn settle_edit(
    outcome: &TerminalOutcome,
    source: EditSettlementSource<'_>,
) -> EditSettlement {
    if !source.target_is_current {
        return EditSettlement::Suppress;
    }
    match outcome {
        TerminalOutcome::Completed => {
            if let Some(reason) = source.durability_failure {
                return EditSettlement::Failed(format!(
                    "The position edit completed but could not be saved: {reason}. Review the accepted value and retry."
                ));
            }
            if !source.accepted_is_saved || source.accepted_revision <= source.base_revision {
                return EditSettlement::Wait;
            }
            if source.accepted_value == Some(source.requested_value) {
                EditSettlement::Saved
            } else {
                EditSettlement::Failed(
                    "The accepted board value does not match this edit. Review it and retry.".into(),
                )
            }
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message)
        | TerminalOutcome::ExecutorFailed(message) => EditSettlement::Failed(message.clone()),
        TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
            EditSettlement::Failed(
                "The position edit did not complete in the active session. Review the accepted value and retry.".into(),
            )
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PlanLifecycle<'a> {
    Idle,
    Pending(&'a FirmwarePlanIdentity),
    Current(&'a FirmwarePlanIdentity, &'a ElectricalPlan),
    Failed(&'a FirmwarePlanIdentity, &'a str),
}

pub(crate) fn project(
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
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::electrical::{ElectricalMode, ElectricalPlanRequest};

    fn document() -> ProjectDoc {
        serde_json::from_str(include_str!(
            "../../core/tests/fixtures/reviung41-outline-original.json"
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
    fn exact_outcome_waits_across_revision_and_save_then_confirms_latest_value() {
        let completed = TerminalOutcome::Completed;
        // The control can be hidden here; this Editor-owned settlement uses only its stable target.
        assert_eq!(
            settle_edit(
                &completed,
                EditSettlementSource {
                    target_is_current: true,
                    accepted_is_saved: false,
                    accepted_revision: 8,
                    base_revision: 7,
                    durability_failure: None,
                    accepted_value: Some("&none"),
                    requested_value: "&kp Q",
                }
            ),
            EditSettlement::Wait
        );
        assert_eq!(
            settle_edit(
                &completed,
                EditSettlementSource {
                    target_is_current: true,
                    accepted_is_saved: true,
                    accepted_revision: 8,
                    base_revision: 7,
                    durability_failure: None,
                    accepted_value: Some("&kp Q"),
                    requested_value: "&kp Q",
                }
            ),
            EditSettlement::Saved
        );
    }

    #[test]
    fn persistence_failure_keeps_accepted_value_and_moved_target_suppresses_feedback() {
        let failed = TerminalOutcome::PersistenceFailed("disk full".into());
        assert_eq!(
            settle_edit(
                &failed,
                EditSettlementSource {
                    target_is_current: true,
                    accepted_is_saved: false,
                    accepted_revision: 7,
                    base_revision: 7,
                    durability_failure: None,
                    accepted_value: Some("&none"),
                    requested_value: "&kp Q",
                }
            ),
            EditSettlement::Failed("disk full".into())
        );
        assert_eq!(
            settle_edit(
                &TerminalOutcome::Completed,
                EditSettlementSource {
                    target_is_current: false,
                    accepted_is_saved: true,
                    accepted_revision: 8,
                    base_revision: 7,
                    durability_failure: None,
                    accepted_value: Some("&kp Q"),
                    requested_value: "&kp Q",
                }
            ),
            EditSettlement::Suppress
        );
        assert_eq!(
            settle_edit(&TerminalOutcome::Completed, EditSettlementSource {
                target_is_current: true, accepted_is_saved: false, accepted_revision: 8,
                base_revision: 7, durability_failure: Some("disk full"),
                accepted_value: Some("&none"), requested_value: "&kp Q",
            }),
            EditSettlement::Failed(
                "The position edit completed but could not be saved: disk full. Review the accepted value and retry.".into()
            )
        );
    }
}
