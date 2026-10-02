//! PCB wiring preview plus contextual selected-part connection projection.
//!
//! Root owns the accepted-source controller and Runtime request. This leaf receives only a
//! cheap accepted-document handle, board scope, active selection, and guarded edit actions.
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{Net, PartDefinition, PartKind, ProjectDoc},
};
use dioxus::prelude::*;
use std::{rc::Rc, sync::Arc};

mod controller;
mod part_connections;
use crate::firmware_position_projection;
pub(in crate::presentation) use crate::firmware_position_projection::{
    FirmwarePlanIdentity as WiringPlanIdentity, FirmwarePositionFeedbackTarget,
    FirmwarePositionIdentity, FirmwarePositionProjection, PlanLifecycle,
};
pub(in crate::presentation) use controller::{
    use_firmware_position_edits, use_pcb_part_net_edits, use_pcb_wiring_controller,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PartNetEditIdentity {
    pub board_id: String,
    pub ui_scope: Scope,
    pub part_id: String,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum PartNetEditAction {
    AssignPads {
        pad_ids: Vec<String>,
        net_id: Option<String>,
    },
    CreateNet {
        name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PartNetEditRequest {
    pub identity: PartNetEditIdentity,
    pub action: PartNetEditAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum PartNetFeedbackState {
    Saved,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct PartNetFeedback {
    pub identity: PartNetEditIdentity,
    pub message: String,
    pub state: PartNetFeedbackState,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct PartNetActions {
    pub identity: Option<PartNetEditIdentity>,
    pub editable: bool,
    pub feedback: Option<PartNetFeedback>,
    pub on_edit: EventHandler<PartNetEditRequest>,
}
/// A cheap view of the accepted source. Arc identity avoids deep document comparisons in the
/// Dioxus props diff; the leaf never receives writable Session access.
#[derive(Clone)]
pub(in crate::presentation) struct PcbWiringSource {
    pub identity: WiringPlanIdentity,
    pub ui_scope: Scope,
    document: Arc<ProjectDoc>,
    active_part_id: Option<String>,
}

impl PcbWiringSource {
    pub(in crate::presentation) fn new(
        accepted: &AcceptedSnapshot,
        scope: &Scope,
        ui_scope: &Scope,
        active_part_id: Option<&str>,
        executor_epoch: u64,
    ) -> Option<Self> {
        if scope.instance_id.is_some()
            || (Scope {
                instance_id: None,
                ..ui_scope.clone()
            }) != *scope
            || scope.session_epoch != accepted.session_epoch
            || scope.document_id != accepted.document.id
            || accepted.scene.revision != accepted.document.revision
            || !accepted
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        {
            return None;
        }
        Some(Self {
            identity: WiringPlanIdentity {
                scope: scope.clone(),
                token: accepted.token,
                revision: accepted.document.revision,
                executor_epoch,
            },
            ui_scope: ui_scope.clone(),
            document: accepted.document.clone(),
            active_part_id: active_part_id.map(str::to_owned),
        })
    }
}

impl PartialEq for PcbWiringSource {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
            && Arc::ptr_eq(&self.document, &other.document)
            && self.active_part_id == other.active_part_id
            && self.ui_scope == other.ui_scope
    }
}

#[derive(Clone, Debug)]
pub(in crate::presentation) enum PcbWiringResolution {
    Idle,
    Pending {
        identity: WiringPlanIdentity,
    },
    Current {
        identity: WiringPlanIdentity,
        plan: Rc<ElectricalPlan>,
    },
    Failed {
        identity: WiringPlanIdentity,
        message: String,
    },
}

impl PartialEq for PcbWiringResolution {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Idle, Self::Idle) => true,
            (Self::Pending { identity: a }, Self::Pending { identity: b }) => a == b,
            (
                Self::Current {
                    identity: a_identity,
                    plan: a_plan,
                },
                Self::Current {
                    identity: b_identity,
                    plan: b_plan,
                },
            ) => a_identity == b_identity && Rc::ptr_eq(a_plan, b_plan),
            (
                Self::Failed {
                    identity: a_identity,
                    message: a_message,
                },
                Self::Failed {
                    identity: b_identity,
                    message: b_message,
                },
            ) => a_identity == b_identity && a_message == b_message,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ControllerChoice {
    id: String,
    label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalRow {
    name: String,
    net_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum SelectionView {
    Board,
    Switch {
        title: String,
        breadcrumb: String,
        terminals: Vec<TerminalRow>,
    },
    GenericPart {
        title: String,
        breadcrumb: String,
        thickness: f64,
        placed_parts: usize,
        assigned_pins: usize,
        connections: Vec<ConnectionRow>,
        nets: Vec<NetChoice>,
    },
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ConnectionRow {
    label: String,
    pad_ids: Vec<String>,
    selected_net_id: Option<String>,
    standalone_pad: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NetChoice {
    id: String,
    name: String,
    pin_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct WiringDisplayProjection {
    board_name: String,
    mode: ElectricalMode,
    configured_controller_id: Option<String>,
    controller_choices: Vec<ControllerChoice>,
    selection: SelectionView,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct PcbWiringInspectorProps {
    pub source: PcbWiringSource,
    pub resolution: PcbWiringResolution,
    pub firmware_positions: FirmwarePositionProjection,
    pub firmware_feedback: Option<FirmwarePositionFeedback>,
    pub firmware_controls: Element,
    pub part_net_actions: PartNetActions,
    pub on_firmware_edit: EventHandler<FirmwarePositionEditRequest>,
    pub on_resolve: EventHandler<()>,
    pub on_edit_board_wiring: EventHandler<()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct FirmwarePositionEditRequest {
    pub identity: FirmwarePositionIdentity,
    pub key_id: String,
    pub binding: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum FirmwarePositionFeedbackState {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct FirmwarePositionFeedback {
    pub target: FirmwarePositionFeedbackTarget,
    pub state: FirmwarePositionFeedbackState,
}

pub(in crate::presentation) fn firmware_position_projection(
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

#[component]
pub(in crate::presentation) fn PcbWiringInspector(props: PcbWiringInspectorProps) -> Element {
    let new_net_name = use_signal(String::new);
    let generator_source = props.source.active_part_id.as_ref().and_then(|part_id| {
        let part = props
            .source
            .document
            .parts
            .iter()
            .find(|part| part.id == *part_id)?;
        let definition = definition(&props.source.document, &part.definition_id)?;
        definition
            .generator
            .as_ref()
            .map(|generator| generator.source.clone())
    });
    let ergogen_source = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        match source {
            Some(source) => crate::presentation::parts::is_ergogen_source(source).await,
            None => Ok(false),
        }
    }));
    let standalone_pads_allowed = matches!(ergogen_source.read().as_ref(), Some(Ok(false)));
    let display = use_memo(use_reactive((&props.source,), move |(source,)| {
        project_display(&source)
    }));
    let Some(display) = display() else {
        return rsx! {
            section { class: "m1-pcb-wiring", role: "status",
                h2 { "Electrical wiring" }
                p { "The selected board wiring is unavailable." }
            }
        };
    };
    match &display.selection {
        SelectionView::Board => board_wiring(&props, &display),
        SelectionView::Switch {
            title,
            breadcrumb,
            terminals,
        } => switch_wiring(title, breadcrumb, terminals, props.on_edit_board_wiring),
        SelectionView::GenericPart {
            title,
            breadcrumb,
            thickness,
            placed_parts,
            assigned_pins,
            connections,
            nets,
        } => generic_part_wiring(GenericPartWiringProps {
            props: &props,
            source: &props.source,
            title,
            breadcrumb,
            thickness: *thickness,
            placed_parts: *placed_parts,
            assigned_pins: *assigned_pins,
            connections,
            nets,
            standalone_pads_allowed,
            new_net_name,
        }),
        SelectionView::Unsupported => rsx! {},
    }
}

fn project_display(source: &PcbWiringSource) -> Option<WiringDisplayProjection> {
    let document = &source.document;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == source.identity.scope.board_id)?;
    let configuration = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|configuration| configuration.board_id == board.id)
    });
    let controller_choices = board
        .part_ids
        .iter()
        .filter_map(|part_id| {
            let part = document.parts.iter().find(|part| part.id == *part_id)?;
            let definition = definition(document, &part.definition_id)?;
            let controller_candidate = matches!(&definition.kind, PartKind::Controller)
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.contains("/mcu_"));
            controller_candidate.then(|| ControllerChoice {
                id: part.id.clone(),
                label: format!("{} · {}", part.reference, definition.name),
            })
        })
        .collect();

    let selection = match source.active_part_id.as_deref() {
        None => SelectionView::Board,
        Some(part_id) => {
            let Some(part) = document
                .parts
                .iter()
                .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
            else {
                return Some(WiringDisplayProjection {
                    board_name: board.name.clone(),
                    mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
                    configured_controller_id: configuration
                        .and_then(|item| item.controller_part_id.clone()),
                    controller_choices,
                    selection: SelectionView::Unsupported,
                });
            };
            let Some(definition) = definition(document, &part.definition_id) else {
                return Some(WiringDisplayProjection {
                    board_name: board.name.clone(),
                    mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
                    configured_controller_id: configuration
                        .and_then(|item| item.controller_part_id.clone()),
                    controller_choices,
                    selection: SelectionView::Unsupported,
                });
            };
            match &definition.kind {
                PartKind::Controller => SelectionView::Board,
                PartKind::Switch => {
                    let terminals = definition
                        .terminals
                        .iter()
                        .map(|(name, pad_ids)| TerminalRow {
                            name: name.clone(),
                            net_name: first_terminal_net(
                                document,
                                &board.part_ids,
                                &board.net_ids,
                                part_id,
                                pad_ids,
                            )
                            .map(|net| net.name.clone()),
                        })
                        .collect();
                    SelectionView::Switch {
                        title: format!("{} · {}", part.reference, definition.name),
                        breadcrumb: format!("{} / PCB", board.name),
                        terminals,
                    }
                }
                _ => {
                    let nets = document
                        .nets
                        .iter()
                        .filter(|net| {
                            board.net_ids.contains(&net.id)
                                || net
                                    .pins
                                    .iter()
                                    .any(|pin| board.part_ids.contains(&pin.part_id))
                        })
                        .collect::<Vec<_>>();
                    let connections = definition
                        .terminals
                        .iter()
                        .map(|(name, pad_ids)| ConnectionRow {
                            label: format!("{name} terminal"),
                            pad_ids: pad_ids.clone(),
                            selected_net_id: unique_assigned_net(&nets, part_id, pad_ids),
                            standalone_pad: false,
                        })
                        .chain(
                            definition
                                .pads
                                .iter()
                                .filter(|pad| {
                                    pad.plated != Some(false)
                                        && !pad.number.is_empty()
                                        && !definition
                                            .terminals
                                            .values()
                                            .any(|ids| ids.contains(&pad.id))
                                })
                                .map(|pad| ConnectionRow {
                                    label: pad.number.clone(),
                                    pad_ids: vec![pad.id.clone()],
                                    selected_net_id: unique_assigned_net(
                                        &nets,
                                        part_id,
                                        std::slice::from_ref(&pad.id),
                                    ),
                                    standalone_pad: true,
                                }),
                        )
                        .collect();
                    let assigned_pins = nets
                        .iter()
                        .map(|net| {
                            net.pins
                                .iter()
                                .filter(|pin| board.part_ids.contains(&pin.part_id))
                                .count()
                        })
                        .sum();
                    SelectionView::GenericPart {
                        title: format!("{} · {}", part.reference, definition.name),
                        breadcrumb: format!("{} / PCB", board.name),
                        thickness: board.thickness,
                        placed_parts: board.part_ids.len(),
                        assigned_pins,
                        connections,
                        nets: nets
                            .into_iter()
                            .map(|net| NetChoice {
                                id: net.id.clone(),
                                name: net.name.clone(),
                                pin_count: net
                                    .pins
                                    .iter()
                                    .filter(|pin| board.part_ids.contains(&pin.part_id))
                                    .count(),
                            })
                            .collect(),
                    }
                }
            }
        }
    };

    Some(WiringDisplayProjection {
        board_name: board.name.clone(),
        mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
        configured_controller_id: configuration.and_then(|item| item.controller_part_id.clone()),
        controller_choices,
        selection,
    })
}

fn unique_assigned_net(nets: &[&Net], part_id: &str, pad_ids: &[String]) -> Option<String> {
    let assigned = nets
        .iter()
        .filter(|net| {
            net.pins
                .iter()
                .any(|pin| pin.part_id == part_id && pad_ids.contains(&pin.pad_id))
        })
        .map(|net| net.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    (assigned.len() == 1)
        .then(|| assigned.into_iter().next())
        .flatten()
}

struct GenericPartWiringProps<'a> {
    props: &'a PcbWiringInspectorProps,
    source: &'a PcbWiringSource,
    title: &'a str,
    breadcrumb: &'a str,
    thickness: f64,
    placed_parts: usize,
    assigned_pins: usize,
    connections: &'a [ConnectionRow],
    nets: &'a [NetChoice],
    standalone_pads_allowed: bool,
    new_net_name: Signal<String>,
}

fn generic_part_wiring(input: GenericPartWiringProps<'_>) -> Element {
    let GenericPartWiringProps {
        props,
        source,
        title,
        breadcrumb,
        thickness,
        placed_parts,
        assigned_pins,
        connections,
        nets,
        standalone_pads_allowed,
        mut new_net_name,
    } = input;
    let actions = props.part_net_actions.clone();
    let identity = actions.identity.clone().filter(|identity| {
        source.active_part_id.as_deref() == Some(identity.part_id.as_str())
            && source.ui_scope == identity.ui_scope
            && source.identity.scope.board_id == identity.board_id
            && source.identity.token == identity.token
            && source.identity.revision == identity.revision
    });
    let add_identity = identity.clone();
    let add_handler = actions.on_edit;
    let mut add_name = new_net_name;
    let add_net = move |event: FormEvent| {
        event.prevent_default();
        let name = add_name().trim().to_owned();
        if name.is_empty() {
            return;
        }
        if let Some(identity) = add_identity.clone() {
            add_handler.call(PartNetEditRequest {
                identity,
                action: PartNetEditAction::CreateNet { name },
            });
            add_name.set(String::new());
        }
    };
    rsx! {
        section { class: "m1-pcb-wiring m1-pcb-part-connections",
            p { class: "m1-pcb-wiring-breadcrumb", "{breadcrumb}" }
            h2 { "{title}" }
            details { class: "m1-pcb-wiring-section",
                summary { "Board details" }
                ul {
                    li { span { "Board thickness" } strong { "{thickness:.2} mm" } }
                    li { span { "Placed parts" } strong { "{placed_parts}" } }
                    li { span { "Net assignments" } strong { "{assigned_pins}" } }
                }
            }
            div { class: "m1-pcb-wiring-section",
                h3 { "Connections" }
                if connections.is_empty() {
                    p { class: "m1-pcb-wiring-empty", "This part has no assignable terminals or pads." }
                } else {
                    for row in connections.iter().filter(|row| {
                        !row.standalone_pad || standalone_pads_allowed
                    }) {
                        {
                            let on_edit = actions.on_edit;
                            let identity = actions.identity.clone();
                            let pad_ids = row.pad_ids.clone();
                            let value = row.selected_net_id.clone().unwrap_or_default();
                            rsx! {
                                label { class: "m1-pcb-wiring-assignment", key: "{row.label}",
                                    span { "{row.label}" }
                                    select {
                                        "aria-label": "Net for {row.label}",
                                        value: "{value}",
                                        disabled: !actions.editable || identity.is_none(),
                                        onchange: move |event| {
                                            let Some(identity) = identity.clone() else { return; };
                                            on_edit.call(PartNetEditRequest {
                                                identity,
                                                action: PartNetEditAction::AssignPads {
                                                    pad_ids: pad_ids.clone(),
                                                    net_id: (!event.value().is_empty()).then(|| event.value()),
                                                },
                                            });
                                        },
                                        option { value: "", "Unmapped" }
                                        for net in nets {
                                            option { value: "{net.id}", "{net.name}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            form { class: "m1-pcb-wiring-assignment m1-pcb-new-net", onsubmit: add_net,
                input {
                    "aria-label": "New net name",
                    placeholder: "New net name",
                    value: "{new_net_name}",
                    disabled: !actions.editable || identity.is_none(),
                    oninput: move |event| new_net_name.set(event.value()),
                }
                button { r#type: "submit", disabled: !actions.editable || identity.is_none() || new_net_name().trim().is_empty(), "Add net" }
            }
            details { class: "m1-pcb-wiring-section",
                summary { "Electrical nets ({nets.len()})" }
                ul {
                    for net in nets {
                        li { key: "{net.id}", span { "{net.name}" } small { "{net.pin_count} pins" } }
                    }
                    if nets.is_empty() { li { "No nets are assigned to this board." } }
                }
            }
            if let Some(feedback) = actions.feedback {
                p { role: if matches!(feedback.state, PartNetFeedbackState::Failed) { "alert" } else { "status" }, "{feedback.message}" }
            }
        }
    }
}

fn definition<'a>(document: &'a ProjectDoc, definition_id: &str) -> Option<&'a PartDefinition> {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
}

fn first_terminal_net<'a>(
    document: &'a ProjectDoc,
    board_part_ids: &[String],
    board_net_ids: &[String],
    part_id: &str,
    pad_ids: &[String],
) -> Option<&'a Net> {
    document.nets.iter().find(|net| {
        (board_net_ids.contains(&net.id)
            || net
                .pins
                .iter()
                .any(|pin| board_part_ids.contains(&pin.part_id)))
            && net.pins.iter().any(|pin| {
                pin.part_id == part_id && pad_ids.iter().any(|pad_id| pad_id == &pin.pad_id)
            })
    })
}

fn switch_wiring(
    title: &str,
    breadcrumb: &str,
    terminals: &[TerminalRow],
    on_edit_board_wiring: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "m1-pcb-wiring m1-pcb-switch-wiring",
            p { class: "m1-pcb-wiring-breadcrumb", "{breadcrumb}" }
            h2 { "{title}" }
            p { "Switch wiring is inherited from its key assembly." }
            div { class: "m1-pcb-wiring-section",
                h3 { "Named terminals" }
                if terminals.is_empty() {
                    p { class: "m1-pcb-wiring-empty", "This switch has no named terminals." }
                } else {
                    ul {
                        for terminal in terminals {
                            li { key: "{terminal.name}",
                                strong { "{terminal.name}" }
                                span { " → " }
                                span { "{terminal.net_name.as_deref().unwrap_or(\"Unmapped\")}" }
                            }
                        }
                    }
                }
            }
            button { type: "button", onclick: move |_| on_edit_board_wiring.call(()), "Edit board wiring" }
        }
    }
}

fn board_wiring(props: &PcbWiringInspectorProps, display: &WiringDisplayProjection) -> Element {
    let identity = &props.source.identity;
    let matching_plan = match &props.resolution {
        PcbWiringResolution::Current {
            identity: plan_identity,
            plan,
        } if plan_identity == identity
            && plan.revision == identity.revision
            && plan.board_id.as_deref() == Some(identity.scope.board_id.as_str())
            && plan.instance_id.is_none() =>
        {
            Some(plan.as_ref())
        }
        _ => None,
    };
    let pending = matches!(
        &props.resolution,
        PcbWiringResolution::Pending { identity: plan_identity } if plan_identity == identity
    );
    let error = match &props.resolution {
        PcbWiringResolution::Failed {
            identity: plan_identity,
            message,
        } if plan_identity == identity => Some(message.as_str()),
        _ => None,
    };
    let chosen_controller_id = display
        .configured_controller_id
        .as_deref()
        .or_else(|| matching_plan.and_then(|plan| plan.controller_part_id.as_deref()))
        .filter(|id| {
            display
                .controller_choices
                .iter()
                .any(|choice| choice.id == *id)
        });
    let controller_name = chosen_controller_id.map_or("No controller selected", |id| {
        display
            .controller_choices
            .iter()
            .find(|choice| choice.id == id)
            .map_or(id, |choice| choice.label.as_str())
    });
    let mode = matching_plan.map_or(display.mode, |plan| plan.mode);
    let used_pins = matching_plan.map(used_pins).unwrap_or_default();
    let free_pins = matching_plan
        .map(|plan| plan.free_pins.as_slice())
        .unwrap_or(&[]);
    let plan_ready = matching_plan.is_some_and(|plan| {
        !plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    });
    let on_resolve = props.on_resolve;
    rsx! {
        section { class: "m1-pcb-wiring",
            p { class: "m1-pcb-wiring-breadcrumb", "{display.board_name} / PCB" }
            h2 { "Electrical wiring" }
            div { class: "m1-pcb-wiring-controller",
                strong { "Controller" }
                span { "{controller_name}" }
            }
            p { class: "m1-pcb-wiring-mode", "Mode: {mode_label(mode)}" }
            if pending {
                p { role: "status", "Resolving wiring…" }
            } else if let Some(error) = error {
                p { role: "alert", "{error}" }
            } else if matching_plan.is_none() {
                p { role: "status", "Waiting for a current wiring plan." }
            }
            button { type: "button", disabled: pending || display.controller_choices.is_empty(), onclick: move |_| on_resolve.call(()),
                if pending { "Resolving…" } else { "Resolve automatically" }
            }
            {props.firmware_controls.clone()}
            if let Some(plan) = matching_plan {
                div { class: "m1-pcb-wiring-pin-summary",
                    div { strong { "Used pins" }
                        p { if used_pins.is_empty() { "None assigned" } else { "{used_pins.join(\" · \")}" } }
                    }
                    div { strong { "Free pins" }
                        p { if free_pins.is_empty() { "None available" } else { "{free_pins.join(\" · \")}" } }
                    }
                }
                p { class: "m1-pcb-wiring-readiness", role: "status",
                    if plan_ready { "Plan is ready for review." } else { "Plan needs review." }
                }
                div { class: "m1-pcb-wiring-section",
                    h3 { "Assignments" }
                    if mode == ElectricalMode::Matrix {
                        for (index, pin) in plan.row_pins.iter().enumerate() {
                            div { class: "m1-pcb-wiring-assignment", key: "row-{index}",
                                strong { "Row {index + 1}" }
                                span { "{pin}" }
                            }
                        }
                        for (index, pin) in plan.column_pins.iter().enumerate() {
                            div { class: "m1-pcb-wiring-assignment", key: "column-{index}",
                                strong { "Column {index + 1}" }
                                span { "{pin}" }
                            }
                        }
                    } else {
                        for assignment in &plan.assignments {
                            div { class: "m1-pcb-wiring-assignment", key: "{assignment.key_id}",
                                strong { "{part_label(&props.source.document, &assignment.key_id)}" }
                                span { "{assignment.column_pin}" }
                                small { "{assignment.direct_gpio.as_deref().unwrap_or(\"Unresolved\")}" }
                            }
                        }
                    }
                    for (terminal_id, pin) in &plan.peripheral_terminals {
                        div { class: "m1-pcb-wiring-assignment", key: "{terminal_id}",
                            strong { "{terminal_label(terminal_id)}" }
                            span { "{pin}" }
                            if let Some(detail) = plan.peripheral_pins.get(terminal_id) {
                                small { "{detail}" }
                            }
                        }
                    }
                    if plan.row_pins.is_empty()
                        && plan.column_pins.is_empty()
                        && plan.assignments.is_empty()
                        && plan.peripheral_terminals.is_empty() {
                        p { class: "m1-pcb-wiring-empty", "No assignments are available." }
                    }
                }
                if !plan.diagnostics.is_empty() {
                    div { class: "m1-pcb-wiring-section", role: "alert",
                        h3 { "Findings" }
                        ul {
                            for (index, diagnostic) in plan.diagnostics.iter().enumerate() {
                                li { key: "{index}", "{diagnostic.severity}: {diagnostic.message}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mode_label(mode: ElectricalMode) -> &'static str {
    match mode {
        ElectricalMode::Matrix => "Matrix",
        ElectricalMode::Direct => "Direct",
    }
}

fn terminal_label(id: &str) -> String {
    id.strip_prefix("peripheral/")
        .map(|name| {
            name.rsplit('/')
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_else(|| id.to_owned())
}

fn part_label(document: &ProjectDoc, part_id: &str) -> String {
    document
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .map_or_else(|| part_id.to_owned(), |part| part.reference.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};

    #[test]
    fn executor_restart_invalidates_same_accepted_wiring_identity() {
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let before_restart = WiringPlanIdentity {
            scope: scope.clone(),
            token: SnapshotToken(11),
            revision: 13,
            executor_epoch: 17,
        };
        let after_restart = WiringPlanIdentity {
            executor_epoch: 18,
            ..before_restart.clone()
        };

        assert_ne!(before_restart, after_restart);
        assert_eq!(before_restart.scope, after_restart.scope);
        assert_eq!(before_restart.token, after_restart.token);
        assert_eq!(before_restart.revision, after_restart.revision);
    }
}
