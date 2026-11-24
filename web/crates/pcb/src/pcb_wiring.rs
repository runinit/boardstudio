//! PCB wiring controls and accepted board-plan edits.
//!
//! The inspector is one declarative component: every control commits a
//! plain board/part edit synchronously through `Runtime::commit`, and each
//! resolver revalidates against the accepted document it receives.
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlan};

/// Shared outer section shell for the wiring panels.
pub(crate) const WIRING_SECTION_CLS: &str = "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground";
/// Shared bordered card wrapper for the wiring detail groups.
pub(crate) const WIRING_CARD_CLS: &str = "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs";
use boardstudio_model::model::{EditOperation, PartDefinition, PartKind, ProjectDoc};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::editor::EditorContext;
use boardstudio_web_ui_shared::inspector_fields::{InspectorSection, SelectField, TOUCH_CLS};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use dioxus::prelude::*;
use std::rc::Rc;

pub use boardstudio_web_ui_model::wiring::{PcbWiringResolution, PcbWiringSource};

mod connections;
mod mode;
mod part_connections;
mod part_input_projection;
mod part_input_settings;
mod pins;
use crate::firmware_position_projection::{self, FirmwarePositionAdmission, admits_edit};
pub use crate::firmware_position_projection::{
    FirmwarePlanIdentity as WiringPlanIdentity, FirmwarePositionFeedbackTarget,
    FirmwarePositionIdentity, FirmwarePositionProjection, PlanLifecycle,
};
use crate::runtime::Runtime;
pub(crate) use mode::ready_snapshot;
use part_connections::PartNetIntent;
pub use part_input_settings::{PartInputEditRequest, PartInputIdentity, PartInputIntent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartNetEditAction {
    AssignPads {
        pad_ids: Vec<String>,
        net_id: Option<String>,
    },
    CreateNet {
        name: String,
    },
}

/// One part-net edit bound to the rendered snapshot/scope/selection identity;
/// the handler verifies currency before and after async generator classification,
/// and the resolver revalidates the definition against the accepted document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartNetEditRequest {
    pub ui_scope: Scope,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub generator_source: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub scope_generation: u64,
    pub action: PartNetEditAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionEditRequest {
    pub identity: FirmwarePositionIdentity,
    pub key_id: String,
    pub binding: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FirmwarePositionFeedbackState {
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePositionFeedback {
    pub target: FirmwarePositionFeedbackTarget,
    pub state: FirmwarePositionFeedbackState,
}

pub fn firmware_position_projection(
    source: &PcbWiringSource,
    generation: u64,
    resolution: &PcbWiringResolution,
) -> FirmwarePositionProjection {
    let lifecycle = match resolution {
        PcbWiringResolution::Idle => PlanLifecycle::Idle,
        PcbWiringResolution::Pending { identity } => PlanLifecycle::Pending(identity),
        PcbWiringResolution::Current { identity, plan } => PlanLifecycle::Current(identity, plan),
        PcbWiringResolution::Failed { identity, message } => {
            PlanLifecycle::Failed(identity, message)
        }
    };
    firmware_position_projection::project(
        &source.document,
        &source.identity,
        &source.ui_scope,
        generation,
        lifecycle,
    )
}

/// A synchronous edit may land only while the PCB workspace and its physical
/// instance selection are still the ones the inspector rendered for.
fn guard_current(editor: EditorContext, runtime: &Runtime) -> bool {
    (editor.workspace)() == Workspace::Pcb && editor.instance_selection.is_current(&runtime.model())
}

/// The accepted snapshot when it still matches the rendered wiring source.
fn current_snapshot(
    editor: EditorContext,
    runtime: &Runtime,
    source: &PcbWiringSource,
) -> Option<AcceptedSnapshot> {
    let (snapshot, live) = mode::pcb_source(runtime, (editor.scope_generation)())?;
    (live.identity == source.identity
        && live.ui_scope == source.ui_scope
        && live.scope_generation == source.scope_generation)
        .then_some(snapshot)
}

#[derive(Props, Clone, PartialEq)]
pub struct PcbWiringInspectorProps {
    #[props(default)]
    pub board_overview: bool,
    pub source: PcbWiringSource,
    pub resolution: PcbWiringResolution,
    pub on_choose_controller: EventHandler<()>,
    pub on_edit_board_wiring: EventHandler<()>,
}

#[component]
pub fn PcbWiringInspector(props: PcbWiringInspectorProps) -> Element {
    let editor = use_context::<EditorContext>();
    let wiring = use_context::<SharedPcbWiring>();
    let board_failure = use_signal(|| None::<String>);
    let mut firmware_feedback = use_signal(|| None::<FirmwarePositionFeedback>);
    let remap_feedback = use_signal(|| None::<(String, Result<(), String>)>);

    let on_firmware_edit = EventHandler::new(move |request: FirmwarePositionEditRequest| {
        let runtime = editor.runtime();
        if !guard_current(editor, &runtime) {
            return;
        }
        let generation = (editor.scope_generation)();
        let Some((snapshot, live)) = mode::pcb_source(&runtime, generation) else {
            return;
        };
        let resolution = (wiring.resolution_signal)();
        let PcbWiringResolution::Current { identity, plan } = &resolution else {
            return;
        };
        let current = firmware_position_projection::project(
            &snapshot.document,
            &live.identity,
            &live.ui_scope,
            live.scope_generation,
            PlanLifecycle::Current(identity, plan),
        );
        if !admits_edit(
            &request.identity,
            &request.key_id,
            FirmwarePositionAdmission {
                current_generation: generation,
                runtime_scope: Some(&live.ui_scope),
                accepted: &snapshot,
                current_plan: Some(identity),
                current_projection: &current,
            },
        ) {
            return;
        }
        let target = FirmwarePositionFeedbackTarget {
            ui_scope: request.identity.ui_scope.clone(),
            scope_generation: request.identity.scope_generation,
            key_id: request.key_id.clone(),
        };
        let result = runtime.commit("firmware-position", firmware_resolver(request));
        firmware_feedback.set(result.err().map(|message| FirmwarePositionFeedback {
            target,
            state: FirmwarePositionFeedbackState::Failed(message),
        }));
    });

    let source = &props.source;
    let document = &source.document;
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == source.identity.scope.board_id)
    else {
        return rsx! {
            section { class: "{WIRING_SECTION_CLS}", role: "status",
                h2 { "Electrical wiring" }
                p { "The selected board wiring is unavailable." }
            }
        };
    };
    let active_part = (!props.board_overview)
        .then_some(source.active_part_id.as_deref())
        .flatten()
        .and_then(|part_id| {
            document
                .parts
                .iter()
                .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
        });
    let active_kind = active_part.and_then(|part| definition(document, &part.definition_id));
    let board_section = |props: &PcbWiringInspectorProps| {
        board_wiring(
            props,
            editor,
            wiring.on_resolve,
            board_failure,
            remap_feedback,
            firmware_panel(props, editor, firmware_feedback, on_firmware_edit),
        )
    };
    match (active_part, active_kind) {
        // A selected part the board no longer holds renders nothing.
        (Some(_), None) => rsx! {},
        (Some(part), Some(definition)) => match &definition.kind {
            PartKind::Controller => board_section(&props),
            PartKind::Switch => {
                let _ = (part, definition);
                part_input_settings::switch_section(&props.source, props.on_edit_board_wiring)
            }
            _ => rsx! {
                part_input_settings::PartSection {
                    source: props.source.clone(),
                    editable: mode::pcb_source(&editor.runtime(), (editor.scope_generation)()).is_some(),
                }
            },
        },
        _ => board_section(&props),
    }
}

fn firmware_panel(
    props: &PcbWiringInspectorProps,
    editor: EditorContext,
    firmware_feedback: Signal<Option<FirmwarePositionFeedback>>,
    on_firmware_edit: EventHandler<FirmwarePositionEditRequest>,
) -> Element {
    let projection = firmware_position_projection(
        &props.source,
        props.source.scope_generation,
        &props.resolution,
    );
    let feedback = firmware_feedback().filter(|feedback| {
        feedback.target.is_visible(
            &props.source.ui_scope,
            props.source.scope_generation,
            &projection,
        )
    });
    let editable = guard_current(editor, &editor.runtime())
        && projection.identity.is_some()
        && !projection.keys.is_empty();
    rsx! {
        crate::firmware_positions::FirmwareKeymapPanel {
            projection, feedback, editable, on_change: on_firmware_edit,
        }
    }
}

fn definition<'a>(document: &'a ProjectDoc, definition_id: &str) -> Option<&'a PartDefinition> {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
}

fn board_wiring(
    props: &PcbWiringInspectorProps,
    editor: EditorContext,
    on_resolve: Callback<()>,
    board_failure: Signal<Option<String>>,
    mut remap_feedback: Signal<Option<(String, Result<(), String>)>>,
    firmware_controls: Element,
) -> Element {
    let source = props.source.clone();
    let document = &props.source.document;
    let board_id = props.source.identity.scope.board_id.clone();
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return rsx! {};
    };
    let configuration = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|configuration| configuration.board_id == board.id)
    });
    let controller_choices: Vec<(String, String)> = board
        .part_ids
        .iter()
        .filter_map(|part_id| {
            let part = document.parts.iter().find(|part| part.id == *part_id)?;
            let definition = definition(document, &part.definition_id)?;
            let candidate = matches!(&definition.kind, PartKind::Controller)
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.contains("/mcu_"));
            candidate.then(|| {
                (
                    part.id.clone(),
                    format!("{} · {}", part.reference, definition.name),
                )
            })
        })
        .collect();
    let matching_plan = mode::current_plan_for(&props.resolution, &props.source.identity);
    let plan = matching_plan.as_deref();
    let notice = wiring_resolution_notice(&props.resolution, &props.source.identity);
    let pending = matches!(notice, WiringResolutionNotice::Pending);
    let mode = mode::board_mode(document, &board_id);
    let chosen_controller_id = configuration
        .and_then(|item| item.controller_part_id.as_deref())
        .or_else(|| plan.and_then(|plan| plan.controller_part_id.as_deref()))
        .filter(|id| controller_choices.iter().any(|(choice, _)| choice == *id));
    let controller_name = chosen_controller_id.map_or("No controller selected", |id| {
        controller_choices
            .iter()
            .find(|(choice, _)| choice == id)
            .map_or(id, |(_, label)| label.as_str())
    });
    let used_pins = plan.map(used_pins).unwrap_or_default();
    let free_pins = plan.map(|plan| plan.free_pins.as_slice()).unwrap_or(&[]);
    let pin_rows = plan
        .map(|plan| pins::assignments(&props.source, plan))
        .unwrap_or_default();
    let assignment_groups = pins::group_assignments(&pin_rows);
    let existing_connections =
        plan.and_then(|plan| connections::existing_connection_review(&props.source.document, plan));
    let existing_names = existing_connections
        .as_ref()
        .map(|review| review.names().join(", "))
        .unwrap_or_default();
    let handoff = configuration.and_then(|item| item.protected_handoff.clone());
    let apply_enabled = mode::apply_plan(&props.source, &props.resolution, false).is_some();
    let commit = {
        let source = source.clone();
        move |label: &'static str, resolver: EditResolver| {
            let runtime = editor.runtime();
            if guard_current(editor, &runtime)
                && current_snapshot(editor, &runtime, &source).is_some()
            {
                let mut board_failure = board_failure;
                board_failure.set(runtime.commit(label, resolver).err());
            }
        }
    };
    let on_pin = EventHandler::new({
        let commit = commit.clone();
        let board_id = board_id.clone();
        let document = props.source.document.clone();
        let plan = matching_plan.clone();
        move |(assignment_id, pin): (String, Option<String>)| {
            let Some(plan) = &plan else { return };
            if pins::pin_edit_is_available(
                &document,
                plan,
                &board_id,
                &assignment_id,
                pin.as_deref(),
            ) {
                commit(
                    "pcb-wiring-pins",
                    pins::pins_resolver(board_id.clone(), assignment_id, pin),
                );
            }
        }
    });
    let on_choose_controller = props.on_choose_controller;
    let board_failure_message = board_failure();
    let remap = remap_feedback().filter(|(id, _)| *id == board_id);
    rsx! {
        section { class: "{WIRING_SECTION_CLS} gap-4", aria_label: "Electrical wiring",
            p { class: "!m-0 !text-[11px]", "{board.name} / PCB" }
            header { class: "flex items-start justify-between gap-3 [&_h2]:!m-0 [&_p]:!mt-[5px] [&_p]:!mb-0",
                div { h2 { "Routing & nets" } p { "{controller_name}" } }
                span { class: "border border-border rounded-[3px] px-1.5 py-[3px] text-muted-foreground font-mono text-[11px] uppercase", "{mode::mode_text(mode)}" }
            }
            div { class: "border-t border-border py-4 [&_ul]:list-none [&_ul]:m-0 [&_ul]:p-0 [&_li]:flex [&_li]:items-baseline [&_li]:justify-between [&_li]:gap-2 [&_li]:py-1.5 [&_li]:border-b [&_li]:border-border [&_li_span]:text-muted-foreground [&_li_span]:text-xs", InspectorSection { default_open: true, title: "Nets",
                ul {
                    for net in document.nets.iter().filter(|net| part_connections::net_on_board(net, board)) {
                        li { key: "{net.id}", strong { "{net.name}" } span { "{net.pins.len()} pins" } }
                    }
                }
            } }
            SelectField::<ElectricalMode> {
                label: "Wiring mode", value: mode, disabled: matching_plan.is_none(),
                on_commit: {
                    let commit = commit.clone();
                    let board_id = board_id.clone();
                    move |mode| commit("pcb-wiring-mode", mode::mode_resolver(board_id.clone(), mode))
                },
                options: vec![(ElectricalMode::Matrix, "Matrix".into()), (ElectricalMode::Direct, "Direct GPIO".into())],
            }
            if controller_choices.is_empty() {
                p { "Place a controller from Parts to assign this board’s wiring." }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |_| on_choose_controller.call(()), "Add controller" }
            }
            if let Some(message) = &board_failure_message {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if handoff.is_some() || remap.is_some() {
                section { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0", aria_label: "Protected handoff",
                    strong { "Protected handoff" }
                    if let Some(handoff) = &handoff {
                        p { "Pins protected by PCB handoff at revision {handoff.revision}" }
                        InspectorSection { default_open: false, title: "Review PCB remap",
                            p { "Changing a protected assignment creates a new hardware revision. The old PCB and firmware must be regenerated before export." }
                            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS,
                                onclick: {
                                    let source = source.clone();
                                    let fingerprint = handoff.fingerprint.clone();
                                    let board_id = board_id.clone();
                                    move |_| {
                                        let runtime = editor.runtime();
                                        let Some(snapshot) = current_snapshot(editor, &runtime, &source)
                                            .filter(|_| guard_current(editor, &runtime)) else { return };
                                        let operation_id = runtime.operation();
                                        let outcome = runtime.observe_operation(operation_id);
                                        runtime.submit(Event::ReviewElectricalRemap {
                                            operation_id,
                                            base_revision: snapshot.document.revision,
                                            board_id: board_id.clone(),
                                            expected_fingerprint: fingerprint.clone(),
                                        });
                                        let result = match outcome.borrow_mut().take() {
                                            Some(TerminalOutcome::Completed) => Ok(()),
                                            Some(TerminalOutcome::Rejected(message) | TerminalOutcome::ExecutorFailed(message) | TerminalOutcome::PersistenceFailed(message)) => Err(message),
                                            _ => Err("The new PCB revision could not be started.".into()),
                                        };
                                        remap_feedback.set(Some((board_id.clone(), result)));
                                    }
                                },
                                "Start a new PCB revision"
                            }
                        }
                    }
                    if let Some((_, result)) = &remap {
                        match result {
                            Ok(()) => rsx! { p { role: "status", "New PCB revision started. Regenerate PCB and firmware before export." } },
                            Err(message) => rsx! { Alert { variant: AlertVariant::Destructive, "{message}" } },
                        }
                    }
                }
            }
            match &notice {
                WiringResolutionNotice::Pending => rsx! { p { role: "status", Spinner {} "Resolving wiring…" } },
                WiringResolutionNotice::Failed(error) => rsx! { Alert { variant: AlertVariant::Destructive, "{error}" } },
                WiringResolutionNotice::Waiting if plan.is_none() => rsx! { p { role: "status", "Waiting for a current wiring plan." } },
                WiringResolutionNotice::Waiting => rsx! {},
            }
            if plan.is_some() && chosen_controller_id.is_some() {
                div { class: "grid gap-2 p-3 border border-border rounded-lg bg-secondary [&>div]:grid [&>div]:gap-[3px] [&>div]:min-w-0 [&_strong]:text-xs [&_strong]:text-muted-foreground [&_span]:text-[11px] [&_span]:font-mono [&_span]:text-muted-foreground [&_span]:[overflow-wrap:anywhere]",
                    div { strong { "Used pins" } span { if used_pins.is_empty() { "None assigned" } else { "{used_pins.join(\" · \")}" } } }
                    div { strong { "Free pins" } span { if free_pins.is_empty() { "None available" } else { "{free_pins.join(\" · \")}" } } }
                }
            }
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: pending || controller_choices.is_empty(),
                    onclick: move |_| on_resolve.call(()),
                    if pending { "Resolving…" } else { "Resolve automatically" }
                }
                Button { class: TOUCH_CLS, disabled: !apply_enabled,
                    onclick: {
                        let commit = commit.clone();
                        let board_id = board_id.clone();
                        let plan = matching_plan.clone();
                        move |_| if let Some(plan) = &plan {
                            commit("pcb-wiring-apply", mode::apply_resolver(board_id.clone(), (**plan).clone(), None));
                        }
                    }, "Apply wiring" }
            }
            if let Some(plan) = plan.filter(|plan| !plan.diagnostics.is_empty()) {
                Alert { variant: AlertVariant::Destructive, class: "[&_p]:!m-0 [&_p]:!text-xs",
                    strong { "Review before handoff" }
                    for (index, diagnostic) in plan.diagnostics.iter().enumerate() {
                        p { key: "{index}", "{diagnostic.severity}: {diagnostic.message}" }
                    }
                }
            }
            if let Some(review) = &existing_connections {
                div { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0",
                    strong { "Review existing connections" }
                    p { "{review.pin_count} pin connections already belong to {existing_names}. Switching them to automatic wiring removes these assignments so the board plan can replace them. Other connections stay in place. You can undo this change." }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: matching_plan.is_none(),
                        onclick: {
                            let commit = commit.clone();
                            let board_id = board_id.clone();
                            let plan = matching_plan.clone();
                            let review = review.clone();
                            move |_| if let Some(plan) = &plan {
                                commit("pcb-wiring-apply", mode::apply_resolver(board_id.clone(), (**plan).clone(), Some(review.clone())));
                            }
                        },
                        "Use automatic wiring for these connections"
                    }
                }
            }
            {firmware_controls}
            if let Some(plan) = plan {
                div { class: "m-0 p-0 border-t-0", InspectorSection { default_open: true, title: format!("Assignments ({})", pin_rows.len()),
                    if plan.mode == ElectricalMode::Matrix {
                        {pins::assignment_group("Row pins", &assignment_groups.rows, plan, on_pin)}
                        {pins::assignment_group("Column pins", &assignment_groups.columns, plan, on_pin)}
                        {pins::assignment_group("Peripheral pins", &assignment_groups.other, plan, on_pin)}
                    } else {
                        for row in assignment_groups.other.iter() {
                            {pins::assignment_row(row, plan, on_pin)}
                        }
                    }
                    if pin_rows.is_empty() {
                        p { class: "text-muted-foreground", "Resolve the board to see controller, matrix, and peripheral assignments." }
                    }
                } }
            }
        }
    }
}

fn used_pins(plan: &ElectricalPlan) -> Vec<String> {
    let scan_pins = match plan.mode {
        ElectricalMode::Matrix => plan
            .row_pins
            .iter()
            .chain(plan.column_pins.iter())
            .cloned()
            .collect::<Vec<_>>(),
        ElectricalMode::Direct => plan
            .assignments
            .iter()
            .map(|assignment| assignment.column_pin.clone())
            .collect(),
    };
    scan_pins
        .into_iter()
        .chain(plan.peripheral_terminals.values().cloned())
        .collect()
}

fn part_net_resolver(
    request: PartNetEditRequest,
    is_generator_source: bool,
    seed: u64,
) -> EditResolver {
    mode::replace_document_resolver("pcb-part-net", move |accepted, replacement| {
        let part = replacement
            .parts
            .iter()
            .find(|part| part.id == request.part_id);
        let definition = part.and_then(|part| {
            replacement
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        });
        if definition.is_none_or(|definition| {
            definition.id != request.definition_id
                || definition
                    .generator
                    .as_ref()
                    .map(|generator| &generator.source)
                    != request.generator_source.as_ref()
        }) {
            return Err("The part was deleted or its definition changed.".into());
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
        *replacement = part_connections::propose(
            &accepted.document,
            &request.board_id,
            &request.part_id,
            is_generator_source,
            intent,
        )?;
        Ok(vec![request.board_id.clone(), request.part_id.clone()])
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

fn firmware_resolver(request: FirmwarePositionEditRequest) -> EditResolver {
    EditResolver::new("firmware-position", move |accepted: &AcceptedSnapshot| {
        let scope = &request.identity.ui_scope;
        let plan = mode::resolve_preview_plan(&accepted.document, &scope.board_id);
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
            PlanLifecycle::Current(&identity, &plan),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WiringResolutionNotice {
    Pending,
    Failed(String),
    Waiting,
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
        _ => WiringResolutionNotice::Waiting,
    }
}

/// One wiring solve shared by the PCB, Keymap and Export workspaces and the
/// Setup Guide. The signal lives at the editor-session lifetime so opening
/// the guide never starts a second solve for the same accepted plan.
#[derive(Clone, Copy)]
pub struct SharedPcbWiring {
    pub resolution_signal: Signal<PcbWiringResolution>,
    pub on_resolve: Callback<()>,
}

impl SharedPcbWiring {
    /// The resolution current for the runtime's accepted plan, or
    /// [`PcbWiringResolution::Idle`]. A disabled read never touches the
    /// signal, so callers that only sometimes need wiring (the Setup Guide
    /// inside the editor session) stay unsubscribed while disabled. The
    /// shared hook itself never reads the signal in its render body; only
    /// consumers calling this method subscribe.
    pub fn current_resolution(&self, runtime: &Runtime, enabled: bool) -> PcbWiringResolution {
        if !enabled {
            return PcbWiringResolution::Idle;
        }
        let identity = current_input(runtime).map(|(identity, _, _)| identity);
        let visible = (self.resolution_signal)();
        match &visible {
            PcbWiringResolution::Current {
                identity: resolved, ..
            }
            | PcbWiringResolution::Failed {
                identity: resolved, ..
            } if Some(resolved) == identity.as_ref() => visible,
            _ => PcbWiringResolution::Idle,
        }
    }
}

pub fn use_pcb_wiring_shared(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    enabled: bool,
) -> SharedPcbWiring {
    let _ = version();
    let identity = enabled
        .then(|| current_input(&runtime))
        .flatten()
        .map(|(identity, _, _)| identity);
    let mut resolution = use_signal(|| PcbWiringResolution::Idle);
    let resolve = use_callback(move |()| {
        if enabled {
            resolution.set(resolve_current_plan(&runtime));
        }
    });
    use_effect(use_reactive((&identity,), move |_| resolve.call(())));
    SharedPcbWiring {
        resolution_signal: resolution,
        on_resolve: resolve,
    }
}

fn resolve_current_plan(runtime: &Runtime) -> PcbWiringResolution {
    let Some((identity, accepted, scope)) = current_input(runtime) else {
        return PcbWiringResolution::Idle;
    };
    match runtime.resolve_electrical_preview(accepted, scope) {
        Ok(plan) => PcbWiringResolution::Current {
            identity,
            plan: Rc::new(plan),
        },
        Err(message) => PcbWiringResolution::Failed { identity, message },
    }
}

fn current_input(runtime: &Runtime) -> Option<(WiringPlanIdentity, AcceptedSnapshot, Scope)> {
    let model = runtime.model();
    if matches!(
        model.lifecycle,
        Lifecycle::Empty | Lifecycle::Closing | Lifecycle::Closed
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
    };
    Some((identity, accepted, scope))
}
