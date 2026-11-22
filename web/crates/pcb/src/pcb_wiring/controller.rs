//! Editor-lifetime owner for automatic, board-scoped electrical plan resolution.
//! The selected part is intentionally absent from this controller's request identity.
use super::part_net_admission::{part_net_owner_context_is_current, use_part_net_owner_lifetime};
use super::part_net_identity::request_matches_current_part;
use super::{
    FirmwarePositionEditRequest, FirmwarePositionFeedback, FirmwarePositionFeedbackState,
    PcbWiringResolution, WiringPlanIdentity, firmware_position_projection,
};
use crate::firmware_position_projection::{
    FirmwarePositionAdmission, FirmwarePositionFeedbackTarget, admits_edit,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};
use wasm_bindgen_futures::spawn_local;

use super::{
    PartNetActions, PartNetEditAction, PartNetEditIdentity, PartNetEditRequest, PartNetFeedback,
    part_connections::{self, PartNetIntent},
};

/// The stable presentation target of one firmware key's edits, without plan tokens.
fn firmware_key_target(
    identity: &crate::firmware_position_projection::FirmwarePositionIdentity,
    key_id: &str,
) -> FirmwarePositionFeedbackTarget {
    FirmwarePositionFeedbackTarget {
        ui_scope: identity.ui_scope.clone(),
        scope_generation: identity.scope_generation,
        key_id: key_id.to_owned(),
    }
}

pub struct FirmwarePositionActions {
    pub feedback: Option<FirmwarePositionFeedback>,
    pub editable: bool,
    pub on_edit: EventHandler<FirmwarePositionEditRequest>,
}

/// Editor-lifetime root owner for legacy SetKeyBinding admission and exact outcome settlement.
pub fn use_firmware_position_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    resolution: Signal<PcbWiringResolution>,
) -> FirmwarePositionActions {
    let _ = version();
    let feedback = use_signal(|| None::<FirmwarePositionFeedback>);
    let generation = scope_generation();
    let on_edit = use_callback({
        let runtime = runtime.clone();
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
            if !matches!(
                model.lifecycle,
                Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
            ) || model.display_preview.is_some()
                || model.gesture.is_some()
                || !matches!(
                    model.durability,
                    Durability::Saved { .. } | Durability::Saving { .. }
                )
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
            let target = firmware_key_target(&request.identity, &request.key_id);
            let result = runtime.commit("firmware-position", firmware_position_resolver(request));
            feedback.set(result.err().map(|message| FirmwarePositionFeedback {
                target,
                state: FirmwarePositionFeedbackState::Failed(message),
            }));
        }
    });

    let editable = firmware_position_editable(
        &runtime,
        workspace(),
        scope_generation(),
        generation,
        instance_is_current(),
        &resolution.read(),
    );
    FirmwarePositionActions {
        feedback: feedback(),
        editable,
        on_edit,
    }
}

fn firmware_position_resolver(request: FirmwarePositionEditRequest) -> EditResolver {
    EditResolver::new("firmware-position", move |accepted: &AcceptedSnapshot| {
        let scope = &request.identity.ui_scope;
        let boardstudio_model::model::CoreRequest::ResolveElectrical {
            request: electrical_request,
            ..
        } = boardstudio_web_runtime::electrical_preview_request(
            "firmware-position",
            &accepted.document,
            &scope.board_id,
        )
        else {
            unreachable!()
        };
        let plan = boardstudio_core_mechanical::electrical::resolve(electrical_request);
        let identity = WiringPlanIdentity {
            token: accepted.token,
            revision: accepted.document.revision,
            ..request.identity.plan.clone()
        };
        let projection = firmware_position_projection::project(
            &accepted.document,
            &identity,
            scope,
            request.identity.scope_generation,
            super::PlanLifecycle::Current(&identity, &plan),
        );
        if !projection.keys.iter().any(|key| key.id == request.key_id) {
            return Resolution::Retire(
                "The firmware key was deleted or is no longer eligible on this board.".into(),
            );
        }
        if projection
            .bindings
            .get(&request.key_id)
            .map(String::as_str)
            .unwrap_or("&none")
            == request.binding
        {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![scope.board_id.clone(), request.key_id.clone()],
            EditOperation::SetKeyBinding {
                board_id: scope.board_id.clone(),
                key_id: request.key_id.clone(),
                binding: request.binding.clone(),
            },
        )
    })
}

pub fn use_pcb_part_net_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
) -> PartNetActions {
    let _ = version();
    let feedback = use_signal(|| None::<PartNetFeedback>);
    let preparing_create = use_signal(|| false);
    let alive = use_part_net_owner_lifetime();
    let generation = scope_generation();
    let on_edit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |request: PartNetEditRequest| {
            if workspace() != "PCB" || scope_generation() != generation || !instance_is_current() {
                return;
            }
            let Some(current) =
                current_part_net_identity(&runtime, generation, instance_is_current())
            else {
                return;
            };
            if !request_matches_current_part(&request.identity, &current) {
                return;
            }
            let create = matches!(&request.action, PartNetEditAction::CreateNet { .. });
            if create && *preparing_create.peek() {
                return;
            }
            let accepted = runtime.model().accepted.unwrap();
            let part = accepted
                .document
                .parts
                .iter()
                .find(|part| part.id == request.identity.part_id)
                .unwrap();
            let definition = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .unwrap();
            let definition_id = definition.id.clone();
            let source = definition
                .generator
                .as_ref()
                .map(|generator| generator.source.clone());
            let mut feedback = feedback;
            let target = super::PartNetTarget::of(&request.identity);
            let seed = runtime.operation().0;
            if source.is_none() {
                let resolver = part_net_resolver(request, definition_id, None, false, seed);
                feedback.set(
                    runtime
                        .commit("pcb-part-net", resolver)
                        .err()
                        .map(|failure| PartNetFeedback { target, failure }),
                );
                return;
            }

            let mut preparing_create = preparing_create;
            if create {
                preparing_create.set(true);
            }
            let runtime = runtime.clone();
            let alive = alive.clone();
            let instance_is_current = instance_is_current.clone();
            spawn_local(async move {
                let classified =
                    crate::presentation::parts::is_generator_source(source.clone().unwrap()).await;
                if !alive.get() {
                    return;
                }
                if create {
                    preparing_create.set(false);
                }
                if !part_net_owner_context_is_current(
                    &alive,
                    workspace,
                    scope_generation,
                    request.identity.generation,
                ) || current_part_net_identity(&runtime, generation, instance_is_current())
                    .is_none_or(|current| {
                        !request_matches_current_part(&request.identity, &current)
                    })
                {
                    return;
                }
                let result = classified.and_then(|is_generator_source| {
                    let resolver = part_net_resolver(
                        request,
                        definition_id,
                        source,
                        is_generator_source,
                        seed,
                    );
                    runtime.commit("pcb-part-net", resolver)
                });
                feedback.set(
                    result
                        .err()
                        .map(|failure| PartNetFeedback { target, failure }),
                );
            });
        }
    });
    let identity = current_part_net_identity(&runtime, generation, instance_is_current());
    let editable = identity.is_some() && workspace() == "PCB";
    let create_pending = preparing_create();
    let visible_feedback = feedback().filter(|feedback| {
        identity
            .as_ref()
            .is_some_and(|identity| feedback.target.matches(identity))
    });
    PartNetActions {
        identity,
        editable,
        feedback: visible_feedback,
        create_pending,
        on_edit,
    }
}

fn part_net_resolver(
    request: PartNetEditRequest,
    definition_id: String,
    source: Option<String>,
    is_generator_source: bool,
    seed: u64,
) -> EditResolver {
    EditResolver::new("pcb-part-net", move |accepted: &AcceptedSnapshot| {
        let part = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == request.identity.part_id);
        let definition = part.and_then(|part| {
            accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        });
        if definition.is_none_or(|definition| {
            definition.id != definition_id
                || definition
                    .generator
                    .as_ref()
                    .map(|generator| &generator.source)
                    != source.as_ref()
        }) {
            return Resolution::Retire("The part was deleted or its definition changed.".into());
        }
        let intent = match &request.action {
            PartNetEditAction::AssignPads { pad_ids, net_id } => PartNetIntent::AssignPads {
                pad_ids: pad_ids.clone(),
                net_id: net_id.clone(),
            },
            PartNetEditAction::CreateNet { name } => PartNetIntent::CreateNet {
                net_id: fresh_net_id(&accepted.document, seed),
                name: name.clone(),
            },
        };
        let proposal = match part_connections::propose(
            &accepted.document,
            &request.identity.board_id,
            &request.identity.part_id,
            is_generator_source,
            intent,
        ) {
            Ok(proposal) => proposal,
            Err(reason) => return Resolution::Retire(reason),
        };
        if proposal == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![
                request.identity.board_id.clone(),
                request.identity.part_id.clone(),
            ],
            EditOperation::ReplaceDocument {
                document: Box::new(proposal),
            },
        )
    })
}

fn current_part_net_identity(
    runtime: &Runtime,
    generation: u64,
    instance_is_current: bool,
) -> Option<PartNetEditIdentity> {
    if !instance_is_current {
        return None;
    }
    let model = runtime.model();
    let scope = runtime.scope()?;
    let accepted = model.accepted.as_ref()?;
    let [part_id] = model.selected_part_ids.as_slice() else {
        return None;
    };
    let board = accepted
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || accepted.session_epoch != scope.session_epoch
        || accepted.document.id != scope.document_id
        || accepted.scene.revision != accepted.document.revision
        || !board.part_ids.contains(part_id)
    {
        return None;
    }
    let part = accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == *part_id)?;
    let definition = accepted
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    if matches!(
        &definition.kind,
        boardstudio_model::model::PartKind::Switch | boardstudio_model::model::PartKind::Controller
    ) {
        return None;
    }
    Some(PartNetEditIdentity {
        board_id: scope.board_id.clone(),
        ui_scope: scope,
        part_id: part_id.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        generation,
    })
}

fn fresh_net_id(document: &ProjectDoc, operation: u64) -> String {
    let mut suffix = 0u64;
    loop {
        let id = format!("pcb-net-{operation}-{suffix}");
        if !document.nets.iter().any(|net| net.id == id) {
            return id;
        }
        suffix = suffix.checked_add(1).expect("net identity space exhausted");
    }
}

fn firmware_position_editable(
    runtime: &Runtime,
    workspace: &str,
    current_generation: u64,
    captured_generation: u64,
    instance_is_current: bool,
    resolution: &PcbWiringResolution,
) -> bool {
    if workspace != "PCB" || !instance_is_current || current_generation != captured_generation {
        return false;
    }
    let model = runtime.model();
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
    {
        return false;
    }
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    let Some(ui_scope) = runtime.scope() else {
        return false;
    };
    if model.active_board_id != ui_scope.board_id
        || model.active_instance_id != ui_scope.instance_id
        || snapshot.session_epoch != ui_scope.session_epoch
        || snapshot.document.id != ui_scope.document_id
        || snapshot.scene.revision != snapshot.document.revision
    {
        return false;
    }
    let mut board_scope = ui_scope.clone();
    board_scope.instance_id = None;
    let plan_identity = WiringPlanIdentity {
        scope: board_scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        executor_epoch: runtime.electrical_preview_executor_epoch(),
    };
    let PcbWiringResolution::Current { identity, plan } = resolution else {
        return false;
    };
    let projection = firmware_position_projection::project(
        &snapshot.document,
        &plan_identity,
        &ui_scope,
        current_generation,
        super::PlanLifecycle::Current(identity, plan),
    );
    identity == &plan_identity && projection.identity.is_some() && !projection.keys.is_empty()
}

#[derive(Clone)]
pub struct PcbWiringMount {
    pub resolution: PcbWiringResolution,
    pub resolution_signal: Signal<PcbWiringResolution>,
    pub on_resolve: EventHandler<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WiringResolutionNotice {
    Pending,
    Failed(String),
    Waiting,
}

impl WiringResolutionNotice {
    pub fn role(&self) -> &'static str {
        match self {
            Self::Pending | Self::Waiting => "status",
            Self::Failed(_) => "alert",
        }
    }
}

pub fn wiring_resolution_notice(
    resolution: &PcbWiringResolution,
    identity: &WiringPlanIdentity,
) -> WiringResolutionNotice {
    match resolution {
        PcbWiringResolution::Pending { identity: owner } if owner == identity => {
            WiringResolutionNotice::Pending
        }
        PcbWiringResolution::Failed {
            identity: owner,
            message,
        } if owner == identity => WiringResolutionNotice::Failed(message.clone()),
        PcbWiringResolution::Current {
            identity: owner, ..
        } if owner == identity => WiringResolutionNotice::Waiting,
        _ => WiringResolutionNotice::Waiting,
    }
}

/// Keep the board-plan query alive at Editor lifetime, regardless of the selected component
/// or which workspace is currently visible. Call this hook unconditionally in the page parent.
pub fn use_pcb_wiring_controller(runtime: Rc<Runtime>, version: Signal<u64>) -> PcbWiringMount {
    let current: Rc<dyn Fn() -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)>> = {
        let runtime = runtime.clone();
        Rc::new(move || current_input(&runtime))
    };
    let resolve: WiringResolver = Rc::new(move |accepted, scope| {
        let runtime = runtime.clone();
        Box::pin(async move { runtime.resolve_electrical_preview(accepted, scope) })
    });
    use_pcb_wiring_resolution_owner(version, current, resolve)
}

type WiringInput = (WiringPlanIdentity, AcceptedSnapshot, Scope);
type WiringResolver = Rc<
    dyn Fn(
        AcceptedSnapshot,
        Scope,
    ) -> Pin<
        Box<
            dyn Future<
                Output = Result<boardstudio_core_mechanical::electrical::ElectricalPlan, String>,
            >,
        >,
    >,
>;

/// The Runtime-facing hook delegates through this private seam so the mounted owner can be
/// qualified with controlled replies without changing production admission or settlement.
fn use_pcb_wiring_resolution_owner(
    version: Signal<u64>,
    current_input: Rc<dyn Fn() -> Option<WiringInput>>,
    resolver: WiringResolver,
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
    let identity = current_input().map(|(identity, _, _)| identity);
    use_effect(use_reactive((&identity, &_runtime_version), {
        let current_input = current_input.clone();
        let resolver = resolver.clone();
        let mut resolution = resolution;
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |(identity, _version)| {
            if identity.is_some() {
                start_resolution(
                    current_input.clone(),
                    resolver.clone(),
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
        let current_input = current_input.clone();
        let resolver = resolver.clone();
        let latest_request = latest_request.clone();
        let alive = alive.clone();
        move |()| {
            start_resolution(
                current_input.clone(),
                resolver.clone(),
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
    current_input: Rc<dyn Fn() -> Option<WiringInput>>,
    resolver: WiringResolver,
    mut resolution: Signal<PcbWiringResolution>,
    latest_request: Rc<Cell<u64>>,
    alive: Rc<Cell<bool>>,
    force: bool,
) {
    if !alive.get() {
        return;
    }
    let Some((identity, accepted, scope)) = current_input() else {
        invalidate_pending(&latest_request);
        resolution.set(PcbWiringResolution::Idle);
        return;
    };
    let already_requested = match &*resolution.read() {
        PcbWiringResolution::Pending { identity: current } => *current == identity,
        PcbWiringResolution::Current {
            identity: current, ..
        }
        | PcbWiringResolution::Failed {
            identity: current, ..
        } => !force && *current == identity,
        PcbWiringResolution::Idle => false,
    };
    if already_requested {
        return;
    }
    let request_generation = next_request(latest_request.as_ref());
    resolution.set(PcbWiringResolution::Pending {
        identity: identity.clone(),
    });
    spawn_local(async move {
        let mut resolution = resolution;
        let result = resolver(accepted, scope).await;
        if !alive.get() || latest_request.get() != request_generation {
            return;
        }
        let Some((current, _, _)) = current_input() else {
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

#[path = "controller_tests.rs"]
#[cfg(all(test, target_arch = "wasm32"))]
mod controller_tests;
