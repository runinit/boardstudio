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

mod apply;
mod connections;
mod controller;
mod mode;
mod part_connections;
mod part_input_settings;
mod part_net_admission;
mod pins;
mod remap;
use crate::firmware_position_projection;
pub(in crate::presentation) use crate::firmware_position_projection::{
    FirmwarePlanIdentity as WiringPlanIdentity, FirmwarePositionFeedbackTarget,
    FirmwarePositionIdentity, FirmwarePositionProjection, PlanLifecycle,
};
pub(in crate::presentation) use apply::{
    BoardWiringApplyActions, BoardWiringApplyFeedback, use_board_wiring_apply,
};
pub(in crate::presentation) use controller::{
    WiringResolutionNotice, use_firmware_position_edits, use_pcb_part_net_edits,
    use_pcb_wiring_controller, wiring_resolution_notice,
};
pub(in crate::presentation) use mode::{
    BoardWiringModeActions, BoardWiringModeEditRequest, BoardWiringModeFeedback,
    use_board_wiring_mode_edits,
};
pub(in crate::presentation) use part_input_settings::PartInputActions;
pub(in crate::presentation) use pins::{
    PcbWiringPinActions, PcbWiringPinEditRequest, PcbWiringPinFeedback, use_pcb_wiring_pin_edits,
};
pub(in crate::presentation) use remap::{
    ProtectedRemapActions, ProtectedRemapFeedback, use_protected_remap_review,
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
    pub scope_generation: u64,
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
        scope_generation: u64,
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
            scope_generation,
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
            && self.scope_generation == other.scope_generation
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
    pub part_input_actions: PartInputActions,
    pub on_firmware_edit: EventHandler<FirmwarePositionEditRequest>,
    pub on_resolve: EventHandler<()>,
    pub on_choose_controller: EventHandler<()>,
    pub on_edit_board_wiring: EventHandler<()>,
    pub mode_actions: BoardWiringModeActions,
    pub pin_actions: PcbWiringPinActions,
    pub apply_actions: BoardWiringApplyActions,
    pub protected_remap_actions: ProtectedRemapActions,
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

pub(in crate::presentation) fn use_part_input_edits(
    runtime: Rc<crate::runtime::Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
) -> PartInputActions {
    part_input_settings::use_part_input_edits(
        runtime,
        version,
        workspace,
        scope_generation,
        instance_is_current,
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
            Some(source) => crate::presentation::parts::is_generator_source(source).await,
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
    let body = match &display.selection {
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
    };
    rsx! {
        div { class: "m1-pcb-inspector-content",
            {body}
        }
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
            part_input_settings::PartInputInspector {
                source: source.clone(),
                actions: props.part_input_actions.clone(),
                board_details_and_connections: rsx! {
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
                },
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
    let resolution_notice = wiring_resolution_notice(&props.resolution, identity);
    let pending = matches!(resolution_notice, WiringResolutionNotice::Pending);
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
    let used_pins = matching_plan.map(used_pins).unwrap_or_default();
    let free_pins = matching_plan
        .map(|plan| plan.free_pins.as_slice())
        .unwrap_or(&[]);
    let on_resolve = props.on_resolve;
    let on_choose_controller = props.on_choose_controller;
    let mode_actions = props.mode_actions.clone();
    let selected_mode = match display.mode {
        ElectricalMode::Matrix => "matrix",
        ElectricalMode::Direct => "direct",
    };
    let change_mode = mode_actions.on_change;
    let mode_identity = mode_actions.identity.clone();
    let apply_actions = props.apply_actions.clone();
    let apply_identity = apply_actions.identity.clone();
    let on_apply = apply_actions.on_apply;
    let pin_actions = props.pin_actions.clone();
    let pin_identity = pin_actions.identity.clone();
    let on_pin_change = pin_actions.on_change;
    let pin_rows = matching_plan
        .map(|plan| pins::assignments(&props.source, plan))
        .unwrap_or_default();
    let protected_remap_actions = props.protected_remap_actions.clone();
    let protected_remap_identity = protected_remap_actions.identity.clone();
    let on_review_remap = protected_remap_actions.on_review;
    let existing_connections = matching_plan
        .and_then(|plan| connections::existing_connection_review(&props.source.document, plan));
    let existing_connection_names = existing_connections
        .as_ref()
        .map(|review| review.names().join(", "))
        .unwrap_or_default();
    let on_release_reviewed_connections = apply_actions.on_release_reviewed_connections;
    let review_connections_identity = mode_identity.clone();
    rsx! {
        section { class: "m1-pcb-wiring m1-pcb-board-wiring", aria_label: "Electrical wiring",
            p { class: "m1-pcb-wiring-breadcrumb", "{display.board_name} / PCB" }
            header { class: "m1-pcb-wiring-header",
                div {
                    h2 { "Wiring" }
                    p { "{controller_name}" }
                }
                span { class: "m1-pcb-wiring-topology", "{selected_mode}" }
            }
            label { class: "m1-pcb-wiring-mode-control",
                span { "Wiring mode" }
                select {
                    "aria-label": "Wiring mode",
                    value: "{selected_mode}",
                    disabled: !mode_actions.editable,
                    onchange: move |event| {
                        let Some(identity) = mode_identity.clone() else { return; };
                        let mode = match event.value().as_str() {
                            "matrix" => ElectricalMode::Matrix,
                            "direct" => ElectricalMode::Direct,
                            _ => return,
                        };
                        change_mode.call(BoardWiringModeEditRequest { identity, mode });
                    },
                    option { value: "matrix", "Matrix" }
                    option { value: "direct", "Direct GPIO" }
                }
            }
            if display.controller_choices.is_empty() {
                p { "Place a controller from Parts to assign this board’s wiring." }
                button { type: "button", onclick: move |_| on_choose_controller.call(()), "Add controller" }
            }
            if let Some(feedback) = &mode_actions.feedback {
                if matches!(feedback.state, BoardWiringModeFeedback::Pending) {
                    p { role: "status", "Saving wiring mode…" }
                } else if matches!(feedback.state, BoardWiringModeFeedback::Saved) {
                    p { role: "status", "Wiring mode saved." }
                } else if let BoardWiringModeFeedback::Failed(message) = &feedback.state {
                    p { role: "alert", "{message}" }
                }
            }
            if let Some(feedback) = &pin_actions.feedback {
                match &feedback.state {
                    PcbWiringPinFeedback::Pending => rsx! { p { role: "status", "Saving wiring pin…" } },
                    PcbWiringPinFeedback::Saved => rsx! { p { role: "status", "Wiring pin saved." } },
                    PcbWiringPinFeedback::Failed(message) => rsx! { p { role: "alert", "{message}" } },
                }
            }
            if protected_remap_actions.handoff_revision.is_some()
                || protected_remap_actions.feedback.is_some()
            {
                section { class: "m1-pcb-wiring-protected", aria_label: "Protected handoff",
                    strong { "Protected handoff" }
                    if let Some(revision) = protected_remap_actions.handoff_revision {
                        p { "Pins protected by PCB handoff at revision {revision}" }
                        details {
                            summary { "Review PCB remap" }
                            p { "Changing a protected assignment creates a new hardware revision. The old PCB and firmware must be regenerated before export." }
                            button {
                                type: "button",
                                disabled: !protected_remap_actions.editable,
                                onclick: move |_| {
                                    let Some(identity) = protected_remap_identity.clone() else { return; };
                                    on_review_remap.call(identity);
                                },
                                "Start a new PCB revision"
                            }
                        }
                    }
                    if let Some(feedback) = &protected_remap_actions.feedback {
                        if matches!(feedback.state, ProtectedRemapFeedback::Pending) {
                            p { role: "status", "Starting a new PCB revision…" }
                        } else if matches!(feedback.state, ProtectedRemapFeedback::Saved) {
                            p { role: "status", "New PCB revision started. Regenerate PCB and firmware before export." }
                        } else if let ProtectedRemapFeedback::Failed(message) = &feedback.state {
                            p { role: "alert", "{message}" }
                        }
                    }
                }
            }
            match &resolution_notice {
                WiringResolutionNotice::Pending => rsx! { p { role: resolution_notice.role(), "Resolving wiring…" } },
                WiringResolutionNotice::Failed(error) => rsx! { p { role: resolution_notice.role(), "{error}" } },
                WiringResolutionNotice::Waiting if matching_plan.is_none() => {
                    rsx! { p { role: "status", "Waiting for a current wiring plan." } }
                }
                WiringResolutionNotice::Waiting => rsx! {},
            }
            if matching_plan.is_some() && chosen_controller_id.is_some() {
                div { class: "m1-pcb-wiring-pin-summary",
                    div { strong { "Used pins" }
                        span { if used_pins.is_empty() { "None assigned" } else { "{used_pins.join(\" · \")}" } }
                    }
                    div { strong { "Free pins" }
                        span { if free_pins.is_empty() { "None available" } else { "{free_pins.join(\" · \")}" } }
                    }
                }
            }
            div { class: "m1-pcb-wiring-actions",
                button { type: "button", disabled: pending || display.controller_choices.is_empty(), onclick: move |_| on_resolve.call(()),
                    if pending { "Resolving…" } else { "Resolve automatically" }
                }
                button { class: "m1-pcb-wiring-apply", type: "button", disabled: !apply_actions.editable, onclick: move |_| {
                    let Some(identity) = apply_identity.clone() else { return; };
                    on_apply.call(identity);
                }, "Apply wiring" }
            }
            if let Some(feedback) = &apply_actions.feedback {
                if matches!(feedback.state, BoardWiringApplyFeedback::Pending) {
                    p { role: "status", "Applying wiring plan…" }
                } else if matches!(feedback.state, BoardWiringApplyFeedback::Saved) {
                    p { role: "status", "Wiring plan applied and saved." }
                } else if let BoardWiringApplyFeedback::Failed(message) = &feedback.state {
                    p { role: "alert", "{message}" }
                }
            }
            if let Some(plan) = matching_plan.filter(|plan| !plan.diagnostics.is_empty()) {
                div { class: "m1-pcb-wiring-findings", role: "alert",
                    strong { "Review before handoff" }
                    for (index, diagnostic) in plan.diagnostics.iter().enumerate() {
                        p { key: "{index}", "{diagnostic.severity}: {diagnostic.message}" }
                    }
                }
            }
            if let Some(review) = existing_connections.as_ref() {
                div { class: "m1-pcb-wiring-protected",
                    strong { "Review existing connections" }
                    p { "{review.pin_count} pin connections already belong to {existing_connection_names}. Switching them to automatic wiring removes these assignments so the board plan can replace them. Other connections stay in place. You can undo this change." }
                    button {
                        type: "button",
                        disabled: !mode_actions.editable || review_connections_identity.is_none(),
                        onclick: move |_| {
                            let Some(identity) = review_connections_identity.clone() else { return; };
                            on_release_reviewed_connections.call(identity);
                        },
                        "Use automatic wiring for these connections"
                    }
                }
            }
            {props.firmware_controls.clone()}
            if let Some(plan) = matching_plan {
                div { class: "m1-pcb-wiring-section m1-pcb-wiring-assignments",
                    div { class: "m1-pcb-wiring-section-heading",
                        h3 { "Assignments" }
                        span { "{pin_rows.len()}" }
                    }
                    for row in &pin_rows {
                        div { class: "m1-pcb-wiring-assignment m1-pcb-wiring-pin-assignment", key: "{row.id}",
                            div {
                                strong { "{row.label}" }
                                if let Some(detail) = row.detail.as_deref() {
                                    small { "{detail}" }
                                }
                            }
                            label { class: "m1-pcb-wiring-pin-control",
                                span { class: "m1-visually-hidden", "Pin for {row.label}" }
                                select {
                                    "aria-label": "Pin for {row.label}",
                                    value: "{row.value.as_deref().unwrap_or(\"\")}",
                                    disabled: !pin_actions.editable || row.locked,
                                    onchange: {
                                        let assignment_id = row.id.clone();
                                        let identity = pin_identity.clone();
                                        move |event| {
                                            let (Some(identity), value) = (identity.clone(), event.value()) else { return; };
                                            on_pin_change.call(PcbWiringPinEditRequest {
                                                identity,
                                                assignment_id: assignment_id.clone(),
                                                pin: Some(value),
                                            });
                                        }
                                    },
                                    {pin_assignment_options(row, pins::pin_choices(row, plan))}
                                }
                            }
                            button {
                                type: "button",
                                class: "m1-pcb-wiring-lock-button",
                                disabled: !pin_actions.editable,
                                "aria-label": "{pin_lock_action(row.locked)} {row.label}",
                                "aria-pressed": "{row.locked}",
                                title: "{pin_lock_action(row.locked)} assignment",
                                onclick: {
                                    let assignment_id = row.id.clone();
                                    let current_pin = row.value.clone();
                                    let locked = row.locked;
                                    let identity = pin_identity.clone();
                                    move |_| {
                                        let Some(identity) = identity.clone() else { return; };
                                        if locked || current_pin.is_some() {
                                            on_pin_change.call(PcbWiringPinEditRequest {
                                                identity,
                                                assignment_id: assignment_id.clone(),
                                                pin: if locked { None } else { current_pin.clone() },
                                            });
                                        }
                                    }
                                },
                                svg { view_box: "0 0 20 20", "aria-hidden": "true", fill: "none", stroke: "currentColor", stroke_width: "1.5",
                                    rect { x: "4", y: "9", width: "12", height: "9", rx: "2" }
                                    path { d: if row.locked { "M6 9V6a4 4 0 0 1 8 0v3" } else { "M6 9V6a4 4 0 0 1 8 0" } }
                                    path { d: "M10 12v3" }
                                }
                            }
                        }
                    }
                    if pin_rows.is_empty() {
                        p { class: "m1-pcb-wiring-empty", "Resolve the board to see controller, matrix, and peripheral assignments." }
                    }
                }
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

fn pin_lock_action(locked: bool) -> &'static str {
    if locked { "Unlock" } else { "Lock" }
}

fn pin_assignment_options(row: &pins::PcbWiringPinAssignment, choices: Vec<String>) -> Element {
    rsx! {
        option {
            value: "",
            selected: row.value.is_none(),
            "Unresolved"
        }
        for pin in choices {
            option {
                value: "{pin}",
                selected: row.value.as_deref() == Some(pin.as_str()),
                "{pin}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen::JsValue;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;
    #[cfg(target_arch = "wasm32")]
    use web_sys::Element as DomElement;

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test_configure!(run_in_browser);

    #[cfg(target_arch = "wasm32")]
    fn pin_option_fixture() -> Element {
        let retained = pins::PcbWiringPinAssignment {
            id: "row/0".into(),
            label: "Row 1".into(),
            detail: None,
            value: Some("P5".into()),
            locked: false,
        };
        let unresolved = pins::PcbWiringPinAssignment {
            id: "left/SW25/encoder-a".into(),
            label: "left/SW25/encoder-a".into(),
            detail: None,
            value: None,
            locked: false,
        };
        rsx! {
            select { id: "retained-pin", value: "P5",
                {pin_assignment_options(&retained, vec!["P5".into(), "P6".into()])}
            }
            select { id: "unresolved-pin", value: "",
                {pin_assignment_options(&unresolved, vec!["P1".into()])}
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn mount_pin_option_fixture() -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("pcb-wiring-pin-option-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(pin_option_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    #[cfg(target_arch = "wasm32")]
    fn selected_option(root: &web_sys::Element, selector: &str) -> Result<DomElement, JsValue> {
        root.query_selector(&format!("{selector} option:checked"))?
            .ok_or_else(|| JsValue::from_str("expected selected pin option"))
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn wiring_pin_options_select_saved_values_and_only_unresolved_for_empty_rows() {
        let root = mount_pin_option_fixture();
        gloo_timers::future::TimeoutFuture::new(40).await;

        assert_eq!(
            selected_option(&root, "#retained-pin")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("P5")
        );
        assert_eq!(
            selected_option(&root, "#unresolved-pin")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Unresolved")
        );
        root.remove();
    }
}
