use crate::{physical_setup::SetupIntent, runtime::Runtime};
use boardstudio_application::{AcceptedSnapshot, Event, Lifecycle, SnapshotToken, TerminalOutcome};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, HardwareTopology, HardwareTransport, ProjectDoc,
};
use dioxus::prelude::*;
use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};
use wasm_bindgen_futures::spawn_local;

pub(in crate::presentation) type ProposalFuture =
    Pin<Box<dyn Future<Output = Result<ProjectDoc, String>>>>;
pub(in crate::presentation) type ProposalPreparer =
    Rc<dyn Fn(ProjectDoc, SetupIntent) -> ProposalFuture>;
pub(in crate::presentation) type CurrentOwner = Rc<dyn Fn(&OwnerIdentity, bool) -> bool>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum OwnerContext {
    ProjectGuide,
    CaseInspector,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct OwnerIdentity {
    pub context: OwnerContext,
    pub session_epoch: boardstudio_application::SessionEpoch,
    pub document_id: String,
    pub board_id: String,
    pub instance_id: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum PhysicalSetupIntent {
    ProjectTopology(bool),
    ProjectTransport(HardwareTransport),
    ProjectReversibleLayout(bool),
    CaseTransport(HardwareTransport),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PhysicalSetupProjection {
    pub topology: HardwareTopology,
    pub transport: HardwareTransport,
    pub reversible: bool,
    pub feedback: Option<String>,
    pub busy: bool,
}

#[derive(Clone)]
pub(in crate::presentation) struct PhysicalSetupMount {
    pub projection: PhysicalSetupProjection,
    pub on_intent: EventHandler<PhysicalSetupIntent>,
}

pub(in crate::presentation) fn use_controller(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    generation: Signal<u64>,
    project_setup_active: bool,
    instance_selection: super::super::InstanceSelection,
    is_current_owner: CurrentOwner,
    prepare: ProposalPreparer,
) -> PhysicalSetupMount {
    let _version = version();
    let _generation = generation();
    let mut feedback = use_signal(|| None::<String>);
    let busy = use_signal(|| false);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let projection = project_setup(&runtime, feedback(), busy());
    let on_intent = use_callback({
        let runtime = runtime.clone();
        let prepare = prepare.clone();
        let alive = alive.clone();
        let is_current_owner = is_current_owner.clone();
        move |intent: PhysicalSetupIntent| {
            if busy() {
                return;
            }
            let context = match intent {
                PhysicalSetupIntent::CaseTransport(_) => OwnerContext::CaseInspector,
                _ => OwnerContext::ProjectGuide,
            };
            if context == OwnerContext::ProjectGuide && !project_setup_active {
                return;
            }
            let Some((accepted, board_id, selected_instance_id, mut identity)) =
                current_source(&runtime, generation(), context)
            else {
                feedback.set(Some(
                    "Physical setup is unavailable for the current selection.".into(),
                ));
                return;
            };
            identity.context = context;
            if !is_current_owner(&identity, true) {
                feedback.set(Some(
                    "Select a current project or Case assembly before changing physical setup."
                        .into(),
                ));
                return;
            }
            let operation_id = runtime.operation();
            let setup_intent = match intent {
                PhysicalSetupIntent::ProjectTopology(split) => SetupIntent::Topology {
                    board_id: board_id.clone(),
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
            };
            let reconcile_primary = matches!(setup_intent, SetupIntent::Topology { .. });
            let alive = alive.clone();
            let runtime = runtime.clone();
            let prepare = prepare.clone();
            let identity = identity.clone();
            let reconcile_primary = reconcile_primary;
            let is_current_owner = is_current_owner.clone();
            let mut feedback = feedback;
            let mut busy = busy;
            busy.set(true);
            feedback.set(Some("Preparing physical setup…".into()));
            spawn_local(async move {
                let prepared = prepare((*accepted.document).clone(), setup_intent).await;
                let proposed = match prepared {
                    Ok(proposed) => proposed,
                    Err(message) => {
                        if alive.get() {
                            busy.set(false);
                            feedback
                                .set(Some(format!("Physical setup was not applied: {message}")));
                        }
                        return;
                    }
                };
                if proposed == *accepted.document {
                    if alive.get() {
                        busy.set(false);
                        feedback.set(Some("That physical setup is already active.".into()));
                    }
                    return;
                }
                if !alive.get() || !is_current_owner(&identity, true) {
                    if alive.get() {
                        busy.set(false);
                        feedback.set(Some(
                            "The accepted project or selection changed; retry physical setup."
                                .into(),
                        ));
                    }
                    return;
                }

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
                if alive.get() {
                    feedback.set(Some("Physical setup submitted; waiting for save…".into()));
                }
                loop {
                    if let Some(outcome) = outcome.borrow_mut().take() {
                        finish_operation(
                            &runtime,
                            &identity,
                            &proposed,
                            reconcile_primary,
                            outcome,
                            &is_current_owner,
                            instance_selection,
                            &mut feedback,
                            &mut busy,
                            alive.get(),
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
        on_intent,
    }
}

pub(in crate::presentation) fn project_setup_controls(mount: PhysicalSetupMount) -> Element {
    let projection = mount.projection;
    let on_intent = mount.on_intent;
    let split = projection.topology == HardwareTopology::Split;
    rsx! {
        section { class: "m1-project-physical-setup", "aria-label": "Physical assembly setup",
            div { class: "m1-project-setup-choice",
                span { "Keyboard configuration" }
                div { role: "group", "aria-label": "Keyboard configuration",
                    button {
                        r#type: "button", "aria-pressed": !split, disabled: projection.busy,
                        onclick: move |_| on_intent.call(PhysicalSetupIntent::ProjectTopology(false)),
                        "One keyboard"
                    }
                    button {
                        r#type: "button", "aria-pressed": split, disabled: projection.busy,
                        onclick: move |_| on_intent.call(PhysicalSetupIntent::ProjectTopology(true)),
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
                            onclick: move |_| on_intent.call(PhysicalSetupIntent::ProjectTransport(HardwareTransport::Wireless)),
                            "Wireless"
                        }
                        button {
                            r#type: "button", "aria-pressed": projection.transport == HardwareTransport::Wired,
                            disabled: projection.busy,
                            onclick: move |_| on_intent.call(PhysicalSetupIntent::ProjectTransport(HardwareTransport::Wired)),
                            "Wired"
                        }
                    }
                }
                p { class: "m1-project-setup-note", if projection.reversible { "Use the same PCB on both halves, turned over on the right." } else { "Both halves use front-facing assemblies." } }
            }
            label { class: "m1-project-reversible-choice",
                input {
                    r#type: "checkbox", checked: projection.reversible, disabled: projection.busy,
                    onchange: move |event: FormEvent| on_intent.call(PhysicalSetupIntent::ProjectReversibleLayout(event.checked()))
                }
                "Reversible layout"
            }
            if let Some(feedback) = projection.feedback {
                p { role: "status", "{feedback}" }
            }
        }
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
        context: OwnerContext::CaseInspector,
        session_epoch: accepted.session_epoch,
        document_id: accepted.document.id.clone(),
        board_id: board_id.clone(),
        instance_id: instance_id.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
        generation,
    };
    Some((accepted, board_id, instance_id, identity))
}

fn project_setup(
    runtime: &Runtime,
    feedback: Option<String>,
    busy: bool,
) -> PhysicalSetupProjection {
    let document = runtime.model().accepted.map(|accepted| accepted.document);
    let hardware = document
        .as_ref()
        .and_then(|document| document.hardware.as_ref());
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
        feedback,
        busy,
    }
}

fn finish_operation(
    runtime: &Rc<Runtime>,
    identity: &OwnerIdentity,
    proposal: &ProjectDoc,
    reconcile_primary: bool,
    outcome: TerminalOutcome,
    is_current_owner: &CurrentOwner,
    instance_selection: super::super::InstanceSelection,
    feedback: &mut Signal<Option<String>>,
    busy: &mut Signal<bool>,
    alive: bool,
) {
    if !alive {
        return;
    }
    match outcome {
        TerminalOutcome::Completed => {
            let model = runtime.model();
            let accepted = model.accepted.as_ref().filter(|accepted| {
                accepted.session_epoch == identity.session_epoch
                    && accepted.document.id == identity.document_id
            });
            let current_owner = is_current_owner(identity, false);
            if !crate::physical_setup::can_reconcile_primary(
                &TerminalOutcome::Completed,
                current_owner,
                accepted.map(|accepted| accepted.document.as_ref()),
                proposal,
            ) {
                busy.set(false);
                feedback.set(Some(if current_owner {
                    "The setup operation completed, but its proposal is no longer the accepted project.".into()
                } else {
                    "Physical setup was saved; selection changed before it could be reconciled.".into()
                }));
                return;
            }
            let accepted = accepted.expect("accepted proposal was checked above");
            if reconcile_primary {
                if let Some(explicit_id) = proposal.hardware.as_ref().and_then(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .find(|instance| instance.board_id == identity.board_id)
                        .map(|instance| instance.id.clone())
                }) {
                    instance_selection.reconcile(
                        accepted.session_epoch,
                        accepted.document.id.clone(),
                        explicit_id.clone(),
                    );
                    runtime.submit(Event::Navigate {
                        operation_id: runtime.operation(),
                        board_id: identity.board_id.clone(),
                        instance_id: Some(explicit_id),
                    });
                }
            }
            busy.set(false);
            feedback.set(Some("Physical setup saved.".into()));
        }
        TerminalOutcome::Rejected(message)
        | TerminalOutcome::ExecutorFailed(message)
        | TerminalOutcome::PersistenceFailed(message)
        | TerminalOutcome::BlockedByRecovery(message) => {
            busy.set(false);
            feedback.set(Some(format!("Physical setup failed: {message}")));
        }
        TerminalOutcome::Cancelled | TerminalOutcome::Closed | TerminalOutcome::Superseded => {
            busy.set(false);
            feedback.set(Some(
                "Physical setup was cancelled before it could be saved.".into(),
            ));
        }
    }
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
