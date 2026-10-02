//! Editor-lifetime owner for automatic, board-scoped electrical plan resolution.
//! The selected part is intentionally absent from this controller's request identity.
use super::{
    FirmwarePositionEditRequest, FirmwarePositionFeedback, FirmwarePositionFeedbackState,
    PcbWiringResolution, WiringPlanIdentity, firmware_position_projection,
};
use crate::firmware_position_projection::{
    EditSettlement, EditSettlementSource, FirmwarePositionAdmission,
    FirmwarePositionFeedbackTarget, admits_edit, settle_edit,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, Event, Lifecycle, Scope};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct PendingFirmwarePositionEdit {
    request: FirmwarePositionEditRequest,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

pub(in crate::presentation) struct FirmwarePositionActions {
    pub feedback: Option<FirmwarePositionFeedback>,
    pub on_edit: EventHandler<FirmwarePositionEditRequest>,
}

/// Editor-lifetime root owner for legacy SetKeyBinding admission and exact outcome settlement.
pub(in crate::presentation) fn use_firmware_position_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    resolution: Signal<PcbWiringResolution>,
) -> FirmwarePositionActions {
    let pending = use_signal(|| None::<PendingFirmwarePositionEdit>);
    let feedback = use_signal(|| None::<FirmwarePositionFeedback>);
    let generation = scope_generation();
    let observed_version = version();

    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                return;
            };
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let target_still_current = runtime.scope().as_ref().is_some_and(|scope| {
                same_feedback_target(scope, &waiting.request.identity.ui_scope)
            }) && snapshot.session_epoch
                == waiting.request.identity.ui_scope.session_epoch
                && snapshot.document.id == waiting.request.identity.ui_scope.document_id
                && model.active_board_id == waiting.request.identity.ui_scope.board_id
                && model.active_instance_id == waiting.request.identity.ui_scope.instance_id
                && snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == waiting.request.identity.ui_scope.board_id)
                    .is_some_and(|board| {
                        let part_id = waiting
                            .request
                            .key_id
                            .strip_suffix("/push")
                            .unwrap_or(&waiting.request.key_id);
                        board.part_ids.iter().any(|id| id == part_id)
                            && snapshot
                                .document
                                .parts
                                .iter()
                                .any(|part| part.id == part_id)
                    });
            let saved = model.lifecycle == Lifecycle::Ready
                && model.durability
                    == (Durability::Saved {
                        revision: snapshot.document.revision,
                    });
            let durability_failure = match &model.durability {
                Durability::Failed { reason, .. } => Some(reason.as_str()),
                _ => None,
            };
            let accepted_value = legacy_binding(
                snapshot,
                &waiting.request.identity.ui_scope.board_id,
                &waiting.request.key_id,
            );
            match settle_edit(
                &outcome,
                EditSettlementSource {
                    target_is_current: target_still_current,
                    accepted_is_saved: saved,
                    accepted_revision: snapshot.document.revision,
                    base_revision: waiting.base_revision,
                    durability_failure,
                    accepted_value,
                    requested_value: &waiting.request.binding,
                },
            ) {
                EditSettlement::Wait => {}
                EditSettlement::Suppress => {
                    pending.set(None);
                    feedback.set(None);
                }
                EditSettlement::Saved => {
                    pending.set(None);
                    feedback.set(Some(FirmwarePositionFeedback {
                        target: feedback_target(&waiting.request),
                        state: FirmwarePositionFeedbackState::Saved,
                    }));
                }
                EditSettlement::Failed(message) => {
                    pending.set(None);
                    feedback.set(Some(FirmwarePositionFeedback {
                        target: feedback_target(&waiting.request),
                        state: FirmwarePositionFeedbackState::Failed(message),
                    }));
                }
            }
        }
    }));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let instance_is_current = instance_is_current.clone();
        move |request: FirmwarePositionEditRequest| {
            if workspace() != "PCB"
                || !instance_is_current()
                || scope_generation() != generation
                || runtime.scope().as_ref() != Some(&request.identity.ui_scope)
            {
                return;
            }
            let model = runtime.model();
            if model.lifecycle != Lifecycle::Ready
                || model.display_preview.is_some()
                || model.gesture.is_some()
                || !matches!(model.durability, Durability::Saved { .. })
                || model.active_board_id != request.identity.ui_scope.board_id
                || model.active_instance_id != request.identity.ui_scope.instance_id
            {
                return;
            }
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.session_epoch != request.identity.ui_scope.session_epoch
                || snapshot.document.id != request.identity.ui_scope.document_id
                || snapshot.token != request.identity.plan.token
                || snapshot.document.revision != request.identity.plan.revision
                || runtime.electrical_preview_executor_epoch()
                    != request.identity.plan.executor_epoch
                || !snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == request.identity.ui_scope.board_id)
            {
                return;
            }
            let live_scope = runtime
                .scope()
                .unwrap_or_else(|| request.identity.ui_scope.clone());
            let plan_identity = WiringPlanIdentity {
                scope: Scope {
                    instance_id: None,
                    ..live_scope.clone()
                },
                token: snapshot.token,
                revision: snapshot.document.revision,
                executor_epoch: runtime.electrical_preview_executor_epoch(),
            };
            if plan_identity != request.identity.plan {
                return;
            }
            let PcbWiringResolution::Current { identity, plan } = &*resolution.read() else {
                return;
            };
            if identity != &request.identity.plan {
                return;
            }
            let current = firmware_position_projection::project(
                &snapshot.document,
                &plan_identity,
                &request.identity.ui_scope,
                generation,
                super::PlanLifecycle::Current(identity, plan),
            );
            if !admits_edit(
                &request.identity,
                &request.key_id,
                FirmwarePositionAdmission {
                    workspace: workspace(),
                    current_generation: scope_generation(),
                    instance_is_current: instance_is_current(),
                    runtime_scope: runtime.scope().as_ref(),
                    accepted: snapshot,
                    executor_epoch: runtime.electrical_preview_executor_epoch(),
                    current_plan: Some(identity),
                    current_projection: &current,
                },
            ) {
                return;
            }
            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let base_revision = snapshot.document.revision;
            pending.set(Some(PendingFirmwarePositionEdit {
                request: request.clone(),
                base_revision,
                outcome,
            }));
            feedback.set(Some(FirmwarePositionFeedback {
                target: feedback_target(&request),
                state: FirmwarePositionFeedbackState::Pending,
            }));
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision,
                    transaction_id: format!(
                        "firmware-position-{}-{}",
                        request.identity.scope_generation, operation_id.0
                    ),
                    phase: EditPhase::Commit,
                    target_ids: vec![
                        request.identity.ui_scope.board_id.clone(),
                        request.key_id.clone(),
                    ],
                    operation: EditOperation::SetKeyBinding {
                        board_id: request.identity.ui_scope.board_id,
                        key_id: request.key_id,
                        binding: request.binding,
                    },
                },
            });
        }
    });

    FirmwarePositionActions {
        feedback: feedback(),
        on_edit,
    }
}

fn feedback_target(request: &FirmwarePositionEditRequest) -> FirmwarePositionFeedbackTarget {
    FirmwarePositionFeedbackTarget {
        ui_scope: request.identity.ui_scope.clone(),
        scope_generation: request.identity.scope_generation,
        key_id: request.key_id.clone(),
    }
}

fn same_feedback_target(current: &Scope, expected: &Scope) -> bool {
    current.session_epoch == expected.session_epoch
        && current.document_id == expected.document_id
        && current.board_id == expected.board_id
        && current.instance_id == expected.instance_id
}

fn legacy_binding<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
    key_id: &str,
) -> Option<&'a str> {
    snapshot
        .document
        .hardware
        .as_ref()?
        .boards
        .iter()
        .find(|board| board.board_id == board_id)?
        .key_bindings
        .get(key_id)
        .map(String::as_str)
}

#[derive(Clone)]
pub(in crate::presentation) struct PcbWiringMount {
    pub resolution: PcbWiringResolution,
    pub resolution_signal: Signal<PcbWiringResolution>,
    pub on_resolve: EventHandler<()>,
}

/// Keep the board-plan query alive at Editor lifetime, regardless of the selected component
/// or which workspace is currently visible. Call this hook unconditionally in the page parent.
pub(in crate::presentation) fn use_pcb_wiring_controller(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
) -> PcbWiringMount {
    let resolution = use_signal(|| PcbWiringResolution::Idle);
    let latest_request = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    // Runtime is deliberately not a Dioxus signal. The page's existing version signal wakes
    // this projection after Runtime notifications; identical board identity is deduplicated.
    let _runtime_version = version();
    let identity = current_input(&runtime).map(|(identity, _, _)| identity);
    use_effect(use_reactive((&identity, &_runtime_version), {
        let runtime = runtime.clone();
        let mut resolution = resolution;
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |(identity, _version)| {
            if identity.is_some() {
                start_resolution(
                    runtime.clone(),
                    resolution,
                    latest_request.clone(),
                    alive.clone(),
                    false,
                );
            } else {
                invalidate_pending(latest_request.as_ref());
                resolution.set(PcbWiringResolution::Idle);
            }
        }
    }));

    let on_resolve = use_callback({
        let runtime = runtime.clone();
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |()| {
            start_resolution(
                runtime.clone(),
                resolution,
                latest_request.clone(),
                alive.clone(),
                true,
            );
        }
    });
    PcbWiringMount {
        resolution: resolution(),
        resolution_signal: resolution,
        on_resolve,
    }
}

fn current_input(runtime: &Runtime) -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)> {
    let model = runtime.model();
    if matches!(
        model.lifecycle,
        Lifecycle::Empty | Lifecycle::Opening | Lifecycle::Closing | Lifecycle::Closed
    ) {
        return None;
    }
    let accepted = model.accepted?;
    let mut scope = runtime.scope()?;
    // The electrical preview is a board plan with React's instanceId=null. Normalize away a
    // physical-instance selection so Case/Layout instance changes don't churn the board query.
    scope.instance_id = None;
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || model.active_board_id != scope.board_id
        || accepted.scene.revision != accepted.document.revision
        || !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    {
        return None;
    }
    let identity = WiringPlanIdentity {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    Some((identity, accepted, scope))
}

fn start_resolution(
    runtime: Rc<Runtime>,
    mut resolution: Signal<PcbWiringResolution>,
    latest_request: Rc<Cell<u64>>,
    alive: Rc<Cell<bool>>,
    force: bool,
) {
    if !alive.get() {
        return;
    }
    let Some((identity, accepted, scope)) = current_input(&runtime) else {
        invalidate_pending(&latest_request);
        resolution.set(PcbWiringResolution::Idle);
        return;
    };
    match &*resolution.read() {
        PcbWiringResolution::Pending { identity: pending } if pending == &identity => return,
        PcbWiringResolution::Current {
            identity: current, ..
        }
        | PcbWiringResolution::Failed {
            identity: current, ..
        } if !force && current == &identity => return,
        _ => {}
    }
    let request_generation = next_request(latest_request.as_ref());
    resolution.set(PcbWiringResolution::Pending {
        identity: identity.clone(),
    });
    spawn_local(async move {
        let mut resolution = resolution;
        let result = runtime.resolve_electrical_preview(accepted, scope).await;
        if !alive.get() || latest_request.get() != request_generation {
            return;
        }
        let Some((current, _, _)) = current_input(&runtime) else {
            resolution.set(PcbWiringResolution::Idle);
            return;
        };
        if current != identity {
            return;
        }
        resolution.set(match result {
            Ok(plan) => PcbWiringResolution::Current {
                identity,
                plan: Rc::new(plan),
            },
            Err(message) => PcbWiringResolution::Failed { identity, message },
        });
    });
}

fn next_request(sequence: &Cell<u64>) -> u64 {
    let request = sequence
        .get()
        .checked_add(1)
        .expect("PCB wiring request sequence exhausted");
    sequence.set(request);
    request
}

fn invalidate_pending(sequence: &Cell<u64>) {
    let _ = next_request(sequence);
}
