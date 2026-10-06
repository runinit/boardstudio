use crate::{physical_setup::SetupIntent, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Event, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, HardwareTopology, HardwareTransport, ProjectDoc,
};
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};
use wasm_bindgen_futures::spawn_local;

pub type ProposalFuture =
    Pin<Box<dyn Future<Output = Result<ProjectDoc, String>>>>;
pub type ProposalPreparer =
    Rc<dyn Fn(ProjectDoc, SetupIntent) -> ProposalFuture>;
pub type CurrentOwner = Rc<dyn Fn(&OwnerIdentity, bool) -> bool>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerContext {
    ProjectGuide,
    CaseInspector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerIdentity {
    pub context: OwnerContext,
    pub session_epoch: boardstudio_application::SessionEpoch,
    pub document_id: String,
    pub board_id: String,
    pub instance_id: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub scope_transition: Option<super::super::selection::ScopeTransition>,
}

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
#[derive(Clone)]
struct OperationUi {
    feedback: Signal<Option<OperationFeedback>>,
    busy: Signal<Option<OperationId>>,
    alive: Rc<Cell<bool>>,
}
impl OperationUi {
    fn publish(
        &mut self,
        operation_id: OperationId,
        owner: &OwnerIdentity,
        message: String,
        terminal: bool,
    ) {
        if !self.alive.get() || *self.busy.peek() != Some(operation_id) {
            return;
        }
        self.feedback.set(Some(OperationFeedback {
            operation_id,
            owner: owner.clone(),
            message,
        }));
        if terminal {
            self.busy.set(None);
        }
    }
}
struct SubmittedSetup {
    operation_id: OperationId,
    owner: OwnerIdentity,
    proposal: ProjectDoc,
    navigate_to: Option<(String, String)>,
    case_reassignment: bool,
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
    let feedback = use_signal(|| None::<OperationFeedback>);
    let busy = use_signal(|| None::<OperationId>);
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
                    && activity.matches(&value.owner, true)
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
        let mut ui = OperationUi {
            feedback,
            busy,
            alive,
        };
        move |request: PhysicalSetupRequest| {
            if ui.busy.peek().is_some() {
                return;
            }
            let identity = request.owner;
            let Some((accepted, board_id, selected_instance_id, current)) =
                current_source(&runtime, *generation.peek(), identity.context)
            else {
                return;
            };
            if identity != current || !activity.matches(&identity, true) {
                return;
            }
            let operation_id = runtime.operation();
            let (proposal_future, navigate_to, case_reassignment) = match request.intent {
                PhysicalSetupIntent::CasePcbDesign(board_id) => {
                    let proposal = propose_case_instance_edit(
                        &accepted.document,
                        &identity,
                        CaseInstanceEdit::PcbDesign(board_id.clone()),
                    );
                    let case_reassignment = board_id != identity.board_id;
                    (
                        Box::pin(async move { proposal }) as ProposalFuture,
                        Some((board_id, identity.instance_id.clone().unwrap_or_default())),
                        case_reassignment,
                    )
                }
                PhysicalSetupIntent::CaseFlip(flipped) => {
                    let proposal = propose_case_instance_edit(
                        &accepted.document,
                        &identity,
                        CaseInstanceEdit::Flip(flipped),
                    );
                    (
                        Box::pin(async move { proposal }) as ProposalFuture,
                        None,
                        false,
                    )
                }
                intent => {
                    let setup_intent = match intent {
                        PhysicalSetupIntent::ProjectTopology(split) => SetupIntent::Topology {
                            board_id,
                            selected_instance_id,
                            split,
                            new_primary_id: fresh_id(
                                &accepted.document,
                                "physical-primary",
                                operation_id.0,
                            ),
                            new_secondary_id: fresh_id(
                                &accepted.document,
                                "physical-secondary",
                                operation_id.0,
                            ),
                        },
                        PhysicalSetupIntent::ProjectTransport(transport)
                        | PhysicalSetupIntent::CaseTransport(transport) => {
                            SetupIntent::Transport(transport)
                        }
                        PhysicalSetupIntent::ProjectReversibleLayout(enabled) => {
                            SetupIntent::ReversibleLayout(enabled)
                        }
                        PhysicalSetupIntent::CasePcbDesign(_)
                        | PhysicalSetupIntent::CaseFlip(_) => unreachable!(),
                    };
                    let navigate_to = matches!(setup_intent, SetupIntent::Topology { .. })
                        .then(|| (identity.board_id.clone(), String::new()));
                    let future = prepare((*accepted.document).clone(), setup_intent.clone());
                    (future, navigate_to, false)
                }
            };
            let runtime = runtime.clone();
            let activity = activity.clone();
            ui.busy.set(Some(operation_id));
            ui.publish(
                operation_id,
                &identity,
                "Preparing physical setup…".into(),
                false,
            );
            let mut ui = ui.clone();
            spawn_local(async move {
                let proposed = match proposal_future.await {
                    Ok(proposed) => proposed,
                    Err(message) => {
                        ui.publish(
                            operation_id,
                            &identity,
                            format!("Physical setup was not applied: {message}"),
                            true,
                        );
                        return;
                    }
                };
                if proposed == *accepted.document {
                    ui.publish(
                        operation_id,
                        &identity,
                        "That physical setup is already active.".into(),
                        true,
                    );
                    return;
                }
                if !ui.alive.get() || !activity.matches(&identity, true) {
                    ui.publish(
                        operation_id,
                        &identity,
                        "The accepted project or selection changed; retry physical setup.".into(),
                        true,
                    );
                    return;
                }
                // Register before submission and retain this exact slot through hidden/unmounted UI.
                let outcome = runtime.observe_operation(operation_id);
                runtime.submit(Event::Edit {
                    operation_id,
                    command: EditCommand {
                        base_revision: accepted.document.revision,
                        transaction_id: format!("physical-setup-{}", operation_id.0),
                        phase: EditPhase::Commit,
                        target_ids: Vec::new(),
                        operation: EditOperation::ReplaceDocument {
                            document: Box::new(proposed.clone()),
                        },
                    },
                });
                ui.publish(
                    operation_id,
                    &identity,
                    "Physical setup submitted; waiting for save…".into(),
                    false,
                );
                let navigate_to = navigate_to.and_then(|(board_id, instance_id)| {
                    if instance_id.is_empty() {
                        proposed
                            .hardware
                            .as_ref()?
                            .instances
                            .iter()
                            .find(|instance| instance.board_id == board_id)
                            .map(|instance| (board_id, instance.id.clone()))
                    } else {
                        Some((board_id, instance_id))
                    }
                });
                let submitted = SubmittedSetup {
                    operation_id,
                    owner: identity,
                    proposal: proposed,
                    navigate_to,
                    case_reassignment,
                };
                loop {
                    if let Some(outcome) = outcome.borrow_mut().take() {
                        finish_operation(
                            &runtime,
                            &submitted,
                            outcome,
                            &activity,
                            instance_selection,
                            generation,
                            &mut ui,
                        );
                        break;
                    }
                    gloo_timers::future::TimeoutFuture::new(16).await;
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
    let topology_single = mount.clone();
    let topology_split = mount.clone();
    let wireless = mount.clone();
    let wired = mount.clone();
    let split = projection.topology == HardwareTopology::Split;
    rsx! {
        section { class: "m1-project-physical-setup", "aria-label": "Physical assembly",
            div { class: "m1-project-setup-choice",
                span { "Keyboard configuration" }
                div { role: "group", "aria-label": "Keyboard configuration",
                    button {
                        r#type: "button", "aria-pressed": !split, disabled: projection.busy,
                        onclick: move |_| topology_single.submit(PhysicalSetupIntent::ProjectTopology(false)),
                        "One keyboard"
                    }
                    button {
                        r#type: "button", "aria-pressed": split, disabled: projection.busy,
                        onclick: move |_| topology_split.submit(PhysicalSetupIntent::ProjectTopology(true)),
                        "Split keyboard"
                    }
                }
            }
            if split {
                div { class: "m1-project-setup-choice",
                    span { "Half connection" }
                    div { role: "group", "aria-label": "Half connection",
                        button {
                            r#type: "button", "aria-pressed": projection.transport == HardwareTransport::Wireless,
                            disabled: projection.busy,
                            onclick: move |_| wireless.submit(PhysicalSetupIntent::ProjectTransport(HardwareTransport::Wireless)),
                            "Wireless"
                        }
                        button {
                            r#type: "button", "aria-pressed": projection.transport == HardwareTransport::Wired,
                            disabled: projection.busy,
                            onclick: move |_| wired.submit(PhysicalSetupIntent::ProjectTransport(HardwareTransport::Wired)),
                            "Wired"
                        }
                    }
                }
                p { class: "m1-project-setup-note", if projection.reversible { "Use the same PCB on both halves, turned over on the right." } else { "Both halves use front-facing assemblies." } }
            }
            label { class: "m1-project-reversible-choice",
                input {
                    r#type: "checkbox", checked: projection.reversible, disabled: projection.busy,
                    onchange: move |event: FormEvent| mount.submit(PhysicalSetupIntent::ProjectReversibleLayout(event.checked()))
                }
                "Reversible layout"
            }
            if let Some(feedback) = projection.project_feedback {
                p { role: "status", "{feedback}" }
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

fn expected_case_reassignment_owner(
    owner: &OwnerIdentity,
    target_board_id: &str,
    accepted_token: SnapshotToken,
    current_generation: u64,
) -> Option<OwnerIdentity> {
    if owner.context != OwnerContext::CaseInspector || target_board_id == owner.board_id {
        return None;
    }
    let instance_id = owner.instance_id.clone()?;
    let next_generation = owner.generation.checked_add(1)?;
    if current_generation != next_generation {
        return None;
    }
    let previous_scope = Scope {
        session_epoch: owner.session_epoch,
        document_id: owner.document_id.clone(),
        board_id: owner.board_id.clone(),
        instance_id: Some(instance_id),
    };
    let next_scope = Scope {
        instance_id: None,
        ..previous_scope.clone()
    };
    let transition = super::super::selection::ScopeTransition {
        previous_scope: Some(previous_scope),
        previous_token: Some(owner.token),
        next_scope: Some(next_scope),
        next_token: Some(accepted_token),
        previous_generation: owner.generation,
        next_generation,
    };
    let mut current_owner = owner.clone();
    current_owner.instance_id = None;
    current_owner.generation = next_generation;
    current_owner.scope_transition = Some(transition);
    Some(current_owner)
}

#[cfg(test)]
mod case_reassignment_scope_transition_tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

    fn owner() -> OwnerIdentity {
        OwnerIdentity {
            context: OwnerContext::CaseInspector,
            session_epoch: SessionEpoch(3),
            document_id: "project".into(),
            board_id: "board-a".into(),
            instance_id: Some("right-half".into()),
            token: SnapshotToken(41),
            revision: 8,
            generation: 12,
            scope_transition: None,
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn only_the_accepted_reassignment_scope_transition_admits_followup_navigation() {
        let owner = owner();
        let accepted_token = SnapshotToken(42);
        let normalized_owner =
            expected_case_reassignment_owner(&owner, "board-b", accepted_token, 13)
                .expect("one accepted scope normalization advances generation once");
        let expected = normalized_owner
            .scope_transition
            .as_ref()
            .expect("reassignment identity carries its exact transition");

        assert!(expected.matches_current(Some(expected), Some(accepted_token), 13));

        let mut user_cleared = expected.clone();
        user_cleared.next_token = Some(owner.token);
        assert!(!expected.matches_current(Some(&user_cleared), Some(accepted_token), 13));

        let mut user_navigated = expected.clone();
        user_navigated.next_scope = Some(Scope {
            instance_id: Some("left-half".into()),
            ..expected
                .next_scope
                .clone()
                .expect("same-board scope exists")
        });
        assert!(!expected.matches_current(Some(&user_navigated), Some(accepted_token), 13));
        assert!(!expected.matches_current(Some(expected), Some(accepted_token), 12));
    }
}

fn current_source(
    runtime: &Runtime,
    generation: u64,
    context: OwnerContext,
) -> Option<(AcceptedSnapshot, String, Option<String>, OwnerIdentity)> {
    let model = runtime.model();
    if !matches!(model.lifecycle, Lifecycle::Ready) {
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

fn finish_operation(
    runtime: &Rc<Runtime>,
    submitted: &SubmittedSetup,
    outcome: TerminalOutcome,
    activity: &OwnerActivity,
    instance_selection: super::super::InstanceSelection,
    generation: Signal<u64>,
    ui: &mut OperationUi,
) {
    if !ui.alive.get() {
        return;
    }
    let identity = &submitted.owner;
    let proposal = &submitted.proposal;
    let model = runtime.model();
    let accepted = model.accepted.as_ref().filter(|accepted| {
        accepted.session_epoch == identity.session_epoch
            && accepted.document.id == identity.document_id
    });
    let accepted_proposal = crate::physical_setup::can_reconcile_primary(
        &outcome,
        true,
        accepted.map(|accepted| accepted.document.as_ref()),
        proposal,
    );
    let accepted_token = accepted.map(|snapshot| snapshot.token);
    let reassignment_owner = if accepted_proposal && submitted.case_reassignment {
        submitted
            .navigate_to
            .as_ref()
            .and_then(|(target_board_id, _)| {
                expected_case_reassignment_owner(
                    identity,
                    target_board_id,
                    accepted_token?,
                    *generation.peek(),
                )
            })
    } else {
        None
    };
    let current_owner = activity.matches(identity, false)
        || reassignment_owner
            .as_ref()
            .is_some_and(|owner| activity.matches(owner, false));
    let navigation_target = (accepted_proposal && current_owner)
        .then(|| submitted.navigate_to.clone())
        .flatten();
    // A hidden owner still owns its exact accepted result. Visibility gates navigation,
    // not attribution; projection will hide feedback until this owner is visible again.
    let mut feedback_owner = identity.clone();
    if accepted_proposal {
        let accepted = accepted.expect("accepted proposal checked above");
        feedback_owner.token = accepted.token;
        feedback_owner.revision = accepted.document.revision;
        if let Some((board_id, instance_id)) = &navigation_target {
            feedback_owner.board_id = board_id.clone();
            feedback_owner.instance_id = Some(instance_id.clone());
            feedback_owner.scope_transition = None;
            let target_scope = Scope {
                session_epoch: accepted.session_epoch,
                document_id: accepted.document.id.clone(),
                board_id: board_id.clone(),
                instance_id: Some(instance_id.clone()),
            };
            let navigation_changes_scope = runtime.scope().as_ref() != Some(&target_scope);
            let current_generation = *generation.peek();
            feedback_owner.generation = current_generation
                .checked_add(if navigation_changes_scope { 1 } else { 0 })
                .unwrap_or(current_generation);
        }
    }
    let message = match outcome {
        TerminalOutcome::Completed if accepted_proposal => {
            let accepted = accepted.expect("accepted proposal checked above");
            if let Some((board_id, explicit_id)) = &navigation_target {
                    instance_selection.reconcile(accepted.session_epoch, accepted.document.id.clone(), explicit_id.clone());
                    runtime.submit(Event::Navigate { operation_id: runtime.operation(), board_id: board_id.clone(), instance_id: Some(explicit_id.clone()) });
            }
            "Physical setup saved.".into()
        }
        TerminalOutcome::Completed => "The setup operation completed, but its proposal is no longer the current accepted project.".into(),
        TerminalOutcome::Rejected(message) | TerminalOutcome::ExecutorFailed(message)
        | TerminalOutcome::PersistenceFailed(message) | TerminalOutcome::BlockedByRecovery(message) => format!("Physical setup failed: {message}"),
        TerminalOutcome::Cancelled | TerminalOutcome::Closed | TerminalOutcome::Superseded => "Physical setup was cancelled before it could be saved.".into(),
    };
    ui.publish(submitted.operation_id, &feedback_owner, message, true);
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
