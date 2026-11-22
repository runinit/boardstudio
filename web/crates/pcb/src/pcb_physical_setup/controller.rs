use super::reassignment_owner::expected_case_reassignment_owner;
pub use super::reassignment_owner::{OwnerContext, OwnerIdentity};
use crate::{physical_setup::SetupIntent, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, OperationId, Resolution,
};
use boardstudio_model::model::{EditOperation, HardwareTopology, HardwareTransport, ProjectDoc};
use boardstudio_web_catalogue::physical_setup::PreparedSetup;
use boardstudio_web_ui_shared::inspector_fields::ToggleField;
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};
use wasm_bindgen_futures::spawn_local;

pub type ProposalFuture = Pin<Box<dyn Future<Output = Result<PreparedSetup, String>>>>;
pub type ProposalPreparer = Rc<dyn Fn(ProjectDoc, SetupIntent) -> ProposalFuture>;
pub type CurrentOwner = Rc<dyn Fn(&OwnerIdentity, bool) -> bool>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhysicalSetupIntent {
    ProjectTopology(bool),
    ProjectTransport(HardwareTransport),
    ProjectReversibleLayout(bool),
    CaseTransport(HardwareTransport),
    CasePcbDesign(String),
    CaseFlip(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseInstanceProjection {
    pub name: String,
    pub role: String,
    pub board_id: String,
    pub board_label: String,
    pub flipped: bool,
    pub has_board_reference: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseBoardProjection {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalSetupProjection {
    pub topology: HardwareTopology,
    pub transport: HardwareTransport,
    pub reversible: bool,
    pub case_instance: Option<CaseInstanceProjection>,
    pub case_boards: Vec<CaseBoardProjection>,
    pub feedback: Option<String>,
    pub project_feedback: Option<String>,
    pub busy: bool,
}

#[derive(Clone)]
pub struct PhysicalSetupMount {
    pub projection: PhysicalSetupProjection,
    pub busy: Signal<Option<OperationId>>,
    project_owner: Option<OwnerIdentity>,
    case_owner: Option<OwnerIdentity>,
    on_intent: EventHandler<PhysicalSetupRequest>,
}

#[derive(Clone)]
struct PhysicalSetupRequest {
    owner: OwnerIdentity,
    intent: PhysicalSetupIntent,
}

impl PhysicalSetupMount {
    pub fn submit(&self, intent: PhysicalSetupIntent) {
        let owner = match intent {
            PhysicalSetupIntent::CaseTransport(_)
            | PhysicalSetupIntent::CasePcbDesign(_)
            | PhysicalSetupIntent::CaseFlip(_) => &self.case_owner,
            _ => &self.project_owner,
        };
        if let Some(owner) = owner {
            self.on_intent.call(PhysicalSetupRequest {
                owner: owner.clone(),
                intent,
            });
        }
    }
}

#[derive(Clone)]
struct OwnerActivity {
    project_active: Rc<dyn Fn() -> bool>,
    is_current: CurrentOwner,
}
impl OwnerActivity {
    fn matches(&self, owner: &OwnerIdentity, strict: bool) -> bool {
        (owner.context != OwnerContext::ProjectGuide || (self.project_active)())
            && (self.is_current)(owner, strict)
    }
}

#[derive(Clone)]
struct OperationFeedback {
    operation_id: OperationId,
    owner: OwnerIdentity,
    message: String,
}
pub fn use_controller(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    generation: Signal<u64>,
    project_setup_active: Rc<dyn Fn() -> bool>,
    instance_selection: super::super::InstanceSelection,
    is_current_owner: CurrentOwner,
    prepare: ProposalPreparer,
) -> PhysicalSetupMount {
    let _version = version();
    let current_generation = generation();
    let mut feedback = use_signal(|| None::<OperationFeedback>);
    let mut busy = use_signal(|| None::<OperationId>);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let activity = OwnerActivity {
        project_active: project_setup_active,
        is_current: is_current_owner,
    };
    let rendered_owner = |context| {
        current_source(&runtime, current_generation, context)
            .map(|(_, _, _, owner)| owner)
            .filter(|owner| activity.matches(owner, true))
    };
    let project_owner = rendered_owner(OwnerContext::ProjectGuide);
    let case_owner = rendered_owner(OwnerContext::CaseInspector);
    let pending = busy();
    let visible_feedback = |context| {
        feedback()
            .filter(|value| {
                value.owner.context == context
                    && activity.matches(&value.owner, false)
                    && pending.is_none_or(|id| id == value.operation_id)
            })
            .map(|value| value.message)
    };
    let projection = project_setup(
        &runtime,
        case_owner.as_ref(),
        visible_feedback(OwnerContext::CaseInspector),
        visible_feedback(OwnerContext::ProjectGuide),
        pending.is_some(),
    );
    let on_intent = use_callback({
        let runtime = runtime.clone();
        move |request: PhysicalSetupRequest| {
            if busy.peek().is_some() {
                return;
            }
            let identity = request.owner;
            let Some((accepted, _board_id, _selected_instance_id, current)) =
                current_source(&runtime, *generation.peek(), identity.context)
            else {
                return;
            };
            if identity != current || !activity.matches(&identity, true) {
                return;
            }
            let seed = runtime.operation().0;
            let intent = request.intent;
            let setup_intent = setup_intent(&accepted.document, &identity, &intent, seed);
            let preparation =
                setup_intent.map(|intent| prepare((*accepted.document).clone(), intent));
            let runtime = runtime.clone();
            let activity = activity.clone();
            let alive = alive.clone();
            busy.set(Some(OperationId(seed)));
            feedback.set(None);
            spawn_local(async move {
                let prepared = match preparation {
                    Some(future) => future.await,
                    None => Ok(PreparedSetup {
                        mounting_hole: None,
                    }),
                };
                if !alive.get() {
                    return;
                }
                busy.set(None);
                if !activity.matches(&identity, false) {
                    return;
                }
                match runtime.commit(
                    "physical-setup",
                    setup_resolver(identity.clone(), intent.clone(), seed, prepared),
                ) {
                    Ok(accepted) => navigate_after_setup(
                        &runtime,
                        &accepted,
                        &identity,
                        &intent,
                        instance_selection,
                        &activity,
                        *generation.peek(),
                    ),
                    Err(message) if activity.matches(&identity, false) => {
                        feedback.set(Some(OperationFeedback {
                            operation_id: OperationId(seed),
                            owner: identity,
                            message,
                        }));
                    }
                    Err(_) => {}
                }
            });
        }
    });
    PhysicalSetupMount {
        projection,
        busy,
        project_owner,
        case_owner,
        on_intent,
    }
}

pub fn project_setup_controls(mount: PhysicalSetupMount) -> Element {
    let projection = mount.projection.clone();
    let topology_mount = mount.clone();
    let transport_mount = mount.clone();
    let reversible_mount = mount.clone();
    let split = projection.topology == HardwareTopology::Split;
    let topology_pressed = usize::from(split);
    let transport_pressed = usize::from(projection.transport == HardwareTransport::Wired);
    let topology_options = ["One keyboard", "Split keyboard"];
    let transport_options = ["Wireless", "Wired"];
    rsx! {
        section { class: "grid gap-3.5", "aria-label": "Physical assembly",
            {setup_options_group("Keyboard configuration", topology_options, topology_pressed, projection.busy, EventHandler::new(move |index: usize| {
                topology_mount.submit(PhysicalSetupIntent::ProjectTopology(index == 1));
            }))}
            if split {
                {setup_options_group("Half connection", transport_options, transport_pressed, projection.busy, EventHandler::new(move |index: usize| {
                    transport_mount.submit(PhysicalSetupIntent::ProjectTransport(if index == 0 { HardwareTransport::Wireless } else { HardwareTransport::Wired }));
                }))}
                p { class: "m-0 text-muted-foreground text-sm leading-normal", if projection.reversible { "Use the same PCB on both halves, turned over on the right." } else { "Both halves use front-facing assemblies." } }
            }
            ToggleField { label: "Reversible layout", checked: projection.reversible,
                disabled_reason: projection.busy.then(|| "Physical setup is updating".to_owned()),
                on_commit: move |checked| reversible_mount.submit(PhysicalSetupIntent::ProjectReversibleLayout(checked))
            }
            if let Some(feedback) = projection.project_feedback {
                p { role: "status", "{feedback}" }
            }
        }
    }
}

fn setup_options_group(
    label: &'static str,
    options: [&'static str; 2],
    pressed: usize,
    disabled: bool,
    on_select: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "grid gap-2",
            span { "{label}" }
            ToggleGroup { class: "grid! grid-cols-2 gap-2!", aria_label: label, horizontal: true, disabled,
                pressed: Some([pressed].into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if next.contains(&0) || (next.is_empty() && pressed == 0) { on_select.call(0); }
                    else if next.contains(&1) || (next.is_empty() && pressed == 1) { on_select.call(1); }
                },
                for (index, option) in options.into_iter().enumerate() {
                    ToggleItem { key: "{index}", index, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{option}" }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CaseInstanceEdit {
    PcbDesign(String),
    Flip(bool),
}

fn propose_case_instance_edit(
    accepted: &ProjectDoc,
    owner: &OwnerIdentity,
    edit: CaseInstanceEdit,
) -> Result<ProjectDoc, String> {
    if owner.context != OwnerContext::CaseInspector {
        return Err("Case assembly controls are unavailable in this workspace".into());
    }
    let instance_id = owner
        .instance_id
        .as_deref()
        .ok_or_else(|| "Select a physical assembly before editing its PCB setup".to_string())?;
    let mut proposed = accepted.clone();
    let hardware = proposed
        .hardware
        .as_mut()
        .ok_or_else(|| "Physical assembly setup is unavailable".to_string())?;
    let instance = hardware
        .instances
        .iter_mut()
        .find(|instance| instance.id == instance_id && instance.board_id == owner.board_id)
        .ok_or_else(|| "The selected physical assembly changed; select it again".to_string())?;
    match edit {
        CaseInstanceEdit::PcbDesign(board_id) => {
            if !proposed.boards.iter().any(|board| board.id == board_id) {
                return Err("Choose an existing PCB design".into());
            }
            instance.board_id = board_id.clone();
            if let Some(mechanical) = instance.mechanical.as_mut() {
                mechanical.board_id = board_id;
            }
        }
        CaseInstanceEdit::Flip(flipped) => instance.flipped = flipped,
    }
    Ok(proposed)
}

fn current_source(
    runtime: &Runtime,
    generation: u64,
    context: OwnerContext,
) -> Option<(AcceptedSnapshot, String, Option<String>, OwnerIdentity)> {
    let model = runtime.model();
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || !matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
    {
        return None;
    }
    let accepted = model.accepted?.clone();
    let (board_id, instance_id) = match context {
        OwnerContext::ProjectGuide => (
            model.active_board_id.clone(),
            model.active_instance_id.clone(),
        ),
        OwnerContext::CaseInspector => {
            let scope = runtime.scope()?;
            if scope.session_epoch != accepted.session_epoch
                || scope.document_id != accepted.document.id
                || scope.board_id != model.active_board_id
            {
                return None;
            }
            (scope.board_id, scope.instance_id)
        }
    };
    if accepted.scene.revision != accepted.document.revision
        || !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == board_id)
    {
        return None;
    }
    let identity = OwnerIdentity {
        context,
        session_epoch: accepted.session_epoch,
        document_id: accepted.document.id.clone(),
        board_id: board_id.clone(),
        instance_id: instance_id.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        generation,
        scope_transition: None,
    };
    Some((accepted, board_id, instance_id, identity))
}

fn project_setup(
    runtime: &Runtime,
    case_owner: Option<&OwnerIdentity>,
    feedback: Option<String>,
    project_feedback: Option<String>,
    busy: bool,
) -> PhysicalSetupProjection {
    let document = runtime.model().accepted.map(|accepted| accepted.document);
    let hardware = document
        .as_ref()
        .and_then(|document| document.hardware.as_ref());
    let case_instance = case_owner.and_then(|owner| {
        let id = owner.instance_id.as_deref()?;
        let document = document.as_ref()?;
        let hardware = hardware?;
        let instance = hardware
            .instances
            .iter()
            .find(|instance| instance.id == id && instance.board_id == owner.board_id)?;
        Some(CaseInstanceProjection {
            name: instance.name.clone(),
            role: instance.role.clone(),
            board_id: instance.board_id.clone(),
            board_label: if hardware
                .instances
                .iter()
                .any(|other| other.id != instance.id && other.board_id == instance.board_id)
            {
                "Shared PCB".into()
            } else {
                document
                    .boards
                    .iter()
                    .find(|board| board.id == instance.board_id)
                    .map(|board| board.name.clone())
                    .unwrap_or_else(|| instance.board_id.clone())
            },
            flipped: instance.flipped,
            has_board_reference: document
                .board_references
                .iter()
                .any(|reference| reference.board_id == instance.board_id),
        })
    });
    PhysicalSetupProjection {
        topology: hardware
            .map(|hardware| hardware.topology)
            .unwrap_or_default(),
        transport: hardware
            .map(|hardware| hardware.transport)
            .unwrap_or_default(),
        reversible: document
            .as_ref()
            .and_then(|document| document.parameters.get("reversibleLayout"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        case_instance,
        case_boards: document
            .as_ref()
            .map(|document| {
                document
                    .boards
                    .iter()
                    .map(|board| CaseBoardProjection {
                        id: board.id.clone(),
                        name: board.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        feedback,
        project_feedback,
        busy,
    }
}

fn setup_intent(
    document: &ProjectDoc,
    owner: &OwnerIdentity,
    intent: &PhysicalSetupIntent,
    seed: u64,
) -> Option<SetupIntent> {
    match intent {
        PhysicalSetupIntent::ProjectTopology(split) => Some(SetupIntent::Topology {
            board_id: owner.board_id.clone(),
            selected_instance_id: owner.instance_id.clone(),
            split: *split,
            new_primary_id: fresh_id(document, "physical-primary", seed),
            new_secondary_id: fresh_id(document, "physical-secondary", seed),
        }),
        PhysicalSetupIntent::ProjectTransport(transport)
        | PhysicalSetupIntent::CaseTransport(transport) => Some(SetupIntent::Transport(*transport)),
        PhysicalSetupIntent::ProjectReversibleLayout(enabled) => {
            Some(SetupIntent::ReversibleLayout(*enabled))
        }
        _ => None,
    }
}

fn setup_resolver(
    owner: OwnerIdentity,
    intent: PhysicalSetupIntent,
    seed: u64,
    prepared: Result<PreparedSetup, String>,
) -> EditResolver {
    EditResolver::new("physical-setup", move |accepted: &AcceptedSnapshot| {
        let prepared = match &prepared {
            Ok(prepared) => prepared,
            Err(reason) => return Resolution::Retire(reason.clone()),
        };
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == owner.board_id)
        {
            return Resolution::Retire("The PCB design was deleted.".into());
        }
        let proposal = match &intent {
            PhysicalSetupIntent::CasePcbDesign(board_id) => propose_case_instance_edit(
                &accepted.document,
                &owner,
                CaseInstanceEdit::PcbDesign(board_id.clone()),
            ),
            PhysicalSetupIntent::CaseFlip(flipped) => propose_case_instance_edit(
                &accepted.document,
                &owner,
                CaseInstanceEdit::Flip(*flipped),
            ),
            intent @ (PhysicalSetupIntent::ProjectTopology(_)
            | PhysicalSetupIntent::ProjectTransport(_)
            | PhysicalSetupIntent::ProjectReversibleLayout(_)
            | PhysicalSetupIntent::CaseTransport(_)) => {
                let setup = setup_intent(&accepted.document, &owner, intent, seed)
                    .expect("every project setup intent maps to a catalogue intent");
                match boardstudio_web_catalogue::catalogue::prepare_physical_setup_proposal_with_pair(
                    &accepted.document,
                    setup,
                ) {
                    Ok(proposal) => match proposal.into_mirrored_operation(prepared.mounting_hole.as_ref()) {
                        Ok(Some((target_ids, operation))) => return Resolution::submit(target_ids, operation),
                        Ok(None) => Ok(proposal.document),
                        Err(reason) => Err(reason),
                    },
                    Err(reason) => Err(reason),
                }
            }
        };
        let proposal = match proposal {
            Ok(proposal) => proposal,
            Err(reason) => return Resolution::Retire(reason),
        };
        if proposal == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![owner.board_id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(proposal),
            },
        )
    })
}

fn navigate_after_setup(
    runtime: &Rc<Runtime>,
    accepted: &AcceptedSnapshot,
    identity: &OwnerIdentity,
    intent: &PhysicalSetupIntent,
    instance_selection: super::super::InstanceSelection,
    activity: &OwnerActivity,
    generation: u64,
) {
    let destination = match intent {
        PhysicalSetupIntent::CasePcbDesign(board_id) => identity
            .instance_id
            .as_ref()
            .map(|id| (board_id.clone(), id.clone())),
        PhysicalSetupIntent::ProjectTopology(_) => accepted
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| instance.board_id == identity.board_id)
            })
            .map(|instance| (instance.board_id.clone(), instance.id.clone())),
        _ => None,
    };
    let Some((board_id, instance_id)) = destination else {
        return;
    };
    let reassigned =
        expected_case_reassignment_owner(identity, &board_id, accepted.token, generation)
            .is_some_and(|owner| activity.matches(&owner, false));
    if !activity.matches(identity, false) && !reassigned {
        return;
    }
    instance_selection.reconcile(
        accepted.session_epoch,
        accepted.document.id.clone(),
        instance_id.clone(),
    );
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id,
        instance_id: Some(instance_id),
    });
}

fn fresh_id(document: &ProjectDoc, prefix: &str, operation: u64) -> String {
    let existing = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .map(|instance| instance.id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut candidate = format!("{prefix}-{operation}");
    let mut suffix = 1_u32;
    while existing.contains(candidate.as_str()) {
        candidate = format!("{prefix}-{operation}-{suffix}");
        suffix = suffix
            .checked_add(1)
            .expect("physical setup ID suffix exhausted");
    }
    candidate
}
