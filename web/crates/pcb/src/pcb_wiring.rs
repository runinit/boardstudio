//! PCB wiring controls and accepted board-plan edits.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope};
use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlan};

/// Shared outer section shell for the wiring panels.
pub(crate) const WIRING_SECTION_CLS: &str = "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground";
/// Shared bordered card wrapper for the wiring detail groups.
pub(crate) const WIRING_CARD_CLS: &str = "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs";
use boardstudio_model::model::{EditOperation, Net, PartDefinition, PartKind, ProjectDoc};
use boardstudio_web_ui_model::editor::{EditGuard, use_guarded_edit};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, SelectField, TOUCH_CLS, TextField,
};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
use dioxus::prelude::*;
use std::rc::Rc;

pub use boardstudio_web_ui_model::wiring::{PcbWiringResolution, PcbWiringSource};

mod apply;
mod connections;
mod mode;
mod part_connections;
mod part_input_projection;
mod part_input_settings;
mod part_net_identity;
pub use part_net_identity::PartNetEditIdentity;
mod pins;
mod remap;
use crate::firmware_position_projection::{self, FirmwarePositionAdmission, admits_edit};
pub use crate::firmware_position_projection::{
    FirmwarePlanIdentity as WiringPlanIdentity, FirmwarePositionFeedbackTarget,
    FirmwarePositionIdentity, FirmwarePositionProjection, PlanLifecycle,
};
use crate::runtime::Runtime;
pub use apply::{BoardWiringApplyActions, use_board_wiring_apply};
use mode::pcb_source;
pub(crate) use mode::ready_snapshot;
pub use mode::{BoardWiringModeActions, BoardWiringModeEditRequest, use_board_wiring_mode_edits};
use part_connections::PartNetIntent;
pub use part_input_settings::{PartInputActions, use_part_input_edits};
use part_net_identity::{admit_classified_part_net_edit, request_matches_current_part};
pub use pins::{PcbWiringPinActions, PcbWiringPinEditRequest, use_pcb_wiring_pin_edits};
pub use remap::{ProtectedRemapActions, ProtectedRemapFeedback, use_protected_remap_review};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartNetEditRequest {
    pub identity: PartNetEditIdentity,
    pub action: PartNetEditAction,
}

/// The stable target of one part's connection edits: no accepted token or revision, so a
/// pending submission keeps projecting and replacing while the revision advances.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartNetTarget {
    pub(crate) ui_scope: Scope,
    pub(crate) board_id: String,
    pub(crate) part_id: String,
    pub(crate) generation: u64,
}

impl PartNetTarget {
    pub(crate) fn of(identity: &PartNetEditIdentity) -> Self {
        Self {
            ui_scope: identity.ui_scope.clone(),
            board_id: identity.board_id.clone(),
            part_id: identity.part_id.clone(),
            generation: identity.generation,
        }
    }

    pub(crate) fn matches(&self, identity: &PartNetEditIdentity) -> bool {
        self.ui_scope == identity.ui_scope
            && self.board_id == identity.board_id
            && self.part_id == identity.part_id
            && self.generation == identity.generation
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartNetFeedback {
    pub(crate) target: PartNetTarget,
    pub(crate) failure: String,
}

#[derive(Clone, PartialEq)]
pub struct PartNetActions {
    pub identity: Option<PartNetEditIdentity>,
    pub editable: bool,
    /// The one-shot "Add net" action is queued or pending for the current part.
    pub(crate) create_pending: bool,
    pub feedback: Option<PartNetFeedback>,
    pub on_edit: EventHandler<PartNetEditRequest>,
}

/// Commit one scoped resolver and settle its failure (wrapped for its target)
/// onto the hook's feedback signal; success clears it.
pub(crate) fn settle_commit<F: 'static>(
    runtime: &Rc<Runtime>,
    label: &'static str,
    resolver: EditResolver,
    mut feedback: Signal<Option<F>>,
    wrap: impl Fn(String) -> F,
) {
    feedback.set(runtime.commit(label, resolver).err().map(wrap));
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
pub struct PcbWiringInspectorProps {
    #[props(default)]
    pub board_overview: bool,
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

#[component]
pub fn PcbWiringInspector(props: PcbWiringInspectorProps) -> Element {
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
    let display = use_memo(use_reactive(
        (&props.source, &props.board_overview),
        move |(mut source, overview)| {
            if overview {
                source.active_part_id = None;
            }
            project_display(&source)
        },
    ));
    let Some(display) = display() else {
        return rsx! {
            section { class: "{WIRING_SECTION_CLS}", role: "status",
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
        div {
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
                            net_name: first_terminal_net(document, board, part_id, pad_ids)
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
                    let nets = part_connections::board_nets(document, board);
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
        section { class: "{WIRING_SECTION_CLS}",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{breadcrumb}" }
            h2 { "{title}" }
            part_input_settings::PartInputInspector {
                source: source.clone(),
                actions: props.part_input_actions.clone(),
                board_details_and_connections: rsx! {
            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: false, title: "Board details",
                ul {
                    li { span { "Board thickness" } strong { "{thickness:.2} mm" } }
                    li { span { "Placed parts" } strong { "{placed_parts}" } }
                    li { span { "Net assignments" } strong { "{assigned_pins}" } }
                }
            } }
            div { class: "{WIRING_CARD_CLS}",
                h3 { "Connections" }
                if connections.is_empty() {
                    p { class: "text-muted-foreground", "This part has no assignable terminals or pads." }
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
                                SelectField::<String> { key: "{row.label}",
                                        label: row.label.clone(), aria_label: format!("Net for {}", row.label), value,
                                        disabled: !actions.editable || identity.is_none(),
                                        on_commit: move |value: String| {
                                            let Some(identity) = identity.clone() else { return; };
                                            on_edit.call(PartNetEditRequest {
                                                identity,
                                                action: PartNetEditAction::AssignPads {
                                                    pad_ids: pad_ids.clone(),
                                                    net_id: (!value.is_empty()).then_some(value),
                                                },
                                            });
                                        },
                                        options: std::iter::once((String::new(), "Unmapped".into())).chain(nets.iter().map(|net| (net.id.clone(), net.name.clone()))).collect(),
                                }
                            }
                        }
                    }
                }
            }
                },
            }
            form { class: "grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5 gap-y-[3px] py-2 border-b border-border text-xs [&_strong]:[overflow-wrap:anywhere] [&_small]:col-span-full [&_small]:text-muted-foreground !items-center !gap-2 mt-2.5", onsubmit: add_net,
                TextField { id: "pcb-new-net".to_owned(), label: "New net name", r#type: InputType::Text,
                    placeholder: Some("New net name".into()), value: new_net_name(),
                    disabled: !actions.editable || identity.is_none(),
                    oninput: move |event: FormEvent| new_net_name.set(event.value()),
                }
                Button { button_type: "submit", variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: !actions.editable || identity.is_none() || new_net_name().trim().is_empty() || actions.create_pending, "Add net" }
            }
            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: false, title: format!("Electrical nets ({})", nets.len()),
                ul {
                    for net in nets {
                        li { key: "{net.id}", span { "{net.name}" } small { "{net.pin_count} pins" } }
                    }
                    if nets.is_empty() { li { "No nets are assigned to this board." } }
                }
            } }
            if let Some(feedback) = actions.feedback {
                Alert { variant: AlertVariant::Destructive, "{feedback.failure}" }
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
    board: &'a boardstudio_model::model::Board,
    part_id: &str,
    pad_ids: &[String],
) -> Option<&'a Net> {
    document.nets.iter().find(|net| {
        part_connections::net_on_board(net, board)
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
        section { class: "{WIRING_SECTION_CLS}",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{breadcrumb}" }
            h2 { "{title}" }
            p { "Switch wiring is inherited from its key assembly." }
            div { class: "{WIRING_CARD_CLS}",
                h3 { "Named terminals" }
                if terminals.is_empty() {
                    p { class: "text-muted-foreground", "This switch has no named terminals." }
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
            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |_| on_edit_board_wiring.call(()), "Edit board wiring" }
        }
    }
}

fn board_wiring(props: &PcbWiringInspectorProps, display: &WiringDisplayProjection) -> Element {
    let identity = &props.source.identity;
    let matching_plan = mode::current_plan_for(&props.resolution, identity);
    let matching_plan = matching_plan.as_deref();
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
    let selected_mode = mode::mode_text(mode_actions.mode);
    let mode_failure = mode_actions.failure.clone();
    let change_mode = mode_actions.on_change;
    let mode_identity = mode_actions.identity.clone();
    let apply_actions = props.apply_actions.clone();
    let apply_identity = apply_actions.identity.clone();
    let apply_failure = apply_actions.failure.clone();
    let on_apply = apply_actions.on_apply;
    let pin_actions = props.pin_actions.clone();
    let pin_failure = pin_actions.failure.clone();
    let pin_rows = matching_plan
        .map(|plan| pins::assignments(&props.source, plan))
        .unwrap_or_default();
    let assignment_groups = pins::group_assignments(&pin_rows);

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
        section { class: "{WIRING_SECTION_CLS} gap-4", aria_label: "Electrical wiring",
            p { class: "!m-0 !text-[11px]", "{display.board_name} / PCB" }
            header { class: "flex items-start justify-between gap-3 [&_h2]:!m-0 [&_p]:!mt-[5px] [&_p]:!mb-0",
                div {
                    h2 { "Routing & nets" }
                    p { "{controller_name}" }
                }
                span { class: "border border-border rounded-[3px] px-1.5 py-[3px] text-muted-foreground font-mono text-[11px] uppercase", "{selected_mode}" }
            }
            div { class: "border-t border-border py-4 [&_ul]:list-none [&_ul]:m-0 [&_ul]:p-0 [&_li]:flex [&_li]:items-baseline [&_li]:justify-between [&_li]:gap-2 [&_li]:py-1.5 [&_li]:border-b [&_li]:border-border [&_li_span]:text-muted-foreground [&_li_span]:text-xs", InspectorSection { default_open: true, title: "Nets",
                ul {
                    for net in props.source.document.nets.iter().filter(|net| props.source.document.boards.iter().find(|board| board.id == props.source.identity.scope.board_id).is_some_and(|board| part_connections::net_on_board(net, board))) {
                        li { key: "{net.id}", strong { "{net.name}" } span { "{net.pins.len()} pins" } }
                    }
                }
            } }
            SelectField::<ElectricalMode> {
                label: "Wiring mode", value: mode_actions.mode, disabled: !mode_actions.editable,
                on_commit: move |mode| {
                    if let Some(identity) = mode_identity.clone() {
                        change_mode.call(BoardWiringModeEditRequest { identity, mode });
                    }
                },
                options: vec![(ElectricalMode::Matrix, "Matrix".into()), (ElectricalMode::Direct, "Direct GPIO".into())],
            }
            if display.controller_choices.is_empty() {
                p { "Place a controller from Parts to assign this board’s wiring." }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |_| on_choose_controller.call(()), "Add controller" }
            }
            if let Some(message) = &mode_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if let Some(message) = &pin_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if protected_remap_actions.handoff_revision.is_some()
                || protected_remap_actions.feedback.is_some()
            {
                section { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0", aria_label: "Protected handoff",
                    strong { "Protected handoff" }
                    if let Some(revision) = protected_remap_actions.handoff_revision {
                        p { "Pins protected by PCB handoff at revision {revision}" }
                        InspectorSection { default_open: false, title: "Review PCB remap",
                            p { "Changing a protected assignment creates a new hardware revision. The old PCB and firmware must be regenerated before export." }
                            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS,
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
                        if matches!(feedback.state, ProtectedRemapFeedback::Applied) {
                            p { role: "status", "New PCB revision started. Regenerate PCB and firmware before export." }
                        } else if let ProtectedRemapFeedback::Failed(message) = &feedback.state {
                            Alert { variant: AlertVariant::Destructive, "{message}" }
                        }
                    }
                }
            }
            match &resolution_notice {
                WiringResolutionNotice::Pending => rsx! { p { role: resolution_notice.role(), Spinner {} "Resolving wiring…" } },
                WiringResolutionNotice::Failed(error) => rsx! { Alert { variant: AlertVariant::Destructive, "{error}" } },
                WiringResolutionNotice::Waiting if matching_plan.is_none() => {
                    rsx! { p { role: "status", "Waiting for a current wiring plan." } }
                }
                WiringResolutionNotice::Waiting => rsx! {},
            }
            if matching_plan.is_some() && chosen_controller_id.is_some() {
                div { class: "grid gap-2 p-3 border border-border rounded-lg bg-secondary [&>div]:grid [&>div]:gap-[3px] [&>div]:min-w-0 [&_strong]:text-xs [&_strong]:text-muted-foreground [&_span]:text-[11px] [&_span]:font-mono [&_span]:text-muted-foreground [&_span]:[overflow-wrap:anywhere]",
                    div { strong { "Used pins" }
                        span { if used_pins.is_empty() { "None assigned" } else { "{used_pins.join(\" · \")}" } }
                    }
                    div { strong { "Free pins" }
                        span { if free_pins.is_empty() { "None available" } else { "{free_pins.join(\" · \")}" } }
                    }
                }
            }
            div { class: "flex flex-wrap gap-2",
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: pending || display.controller_choices.is_empty(), onclick: move |_| on_resolve.call(()),
                    if pending { "Resolving…" } else { "Resolve automatically" }
                }
                Button { class: TOUCH_CLS, disabled: !apply_actions.editable, onclick: move |_| {
                    let Some(identity) = apply_identity.clone() else { return; };
                    on_apply.call(identity);
                }, "Apply wiring" }
            }
            if let Some(message) = &apply_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if let Some(plan) = matching_plan.filter(|plan| !plan.diagnostics.is_empty()) {
                Alert { variant: AlertVariant::Destructive, class: "[&_p]:!m-0 [&_p]:!text-xs",
                    strong { "Review before handoff" }
                    for (index, diagnostic) in plan.diagnostics.iter().enumerate() {
                        p { key: "{index}", "{diagnostic.severity}: {diagnostic.message}" }
                    }
                }
            }
            if let Some(review) = existing_connections.as_ref() {
                div { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0",
                    strong { "Review existing connections" }
                    p { "{review.pin_count} pin connections already belong to {existing_connection_names}. Switching them to automatic wiring removes these assignments so the board plan can replace them. Other connections stay in place. You can undo this change." }
                    Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS,
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
                div { class: "m-0 p-0 border-t-0", InspectorSection { default_open: true, title: format!("Assignments ({})", pin_rows.len()),
                    if plan.mode == ElectricalMode::Matrix {
                        {assignment_group("Row pins", &assignment_groups.rows, plan, &pin_actions)}
                        {assignment_group("Column pins", &assignment_groups.columns, plan, &pin_actions)}
                        {assignment_group("Peripheral pins", &assignment_groups.other, plan, &pin_actions)}
                    } else {
                        for row in assignment_groups.other.iter() {
                            {pin_assignment_row(row, &pin_actions, plan)}
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

fn pin_assignment_row(
    row: &pins::PcbWiringPinAssignment,
    actions: &PcbWiringPinActions,
    plan: &ElectricalPlan,
) -> Element {
    let identity = actions.identity.clone();
    let on_pin_change = actions.on_change;
    let assignment_id = row.id.clone();
    let pin_choices = pins::pin_choices(row, plan);
    rsx! {
        div { class: "grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5 gap-y-[3px] py-2 border-b border-border text-xs [&_strong]:[overflow-wrap:anywhere] [&_small]:col-span-full [&_small]:text-muted-foreground !grid-cols-[minmax(0,1fr)_minmax(84px,auto)_auto] !items-center !gap-x-2 [&>div]:min-w-0 [&>div_small]:block", key: "{row.id}",
            div {
                strong { "{row.label}" }
                if let Some(detail) = row.detail.as_deref() {
                    small { "{detail}" }
                }
            }
            SelectField::<String> {
                    label: format!("Pin for {}", row.label), aria_label: format!("Pin for {}", row.label),
                    value: row.value.clone().unwrap_or_default(),
                    disabled: !actions.editable || row.locked,
                    options: std::iter::once((String::new(), "Unresolved".into())).chain(pin_choices.into_iter().map(|pin| (pin.clone(), pin))).collect(),
                    on_commit: {
                        let assignment_id = assignment_id.clone();
                        let identity = identity.clone();
                        move |value: String| {
                            let Some(identity) = identity.clone() else { return; };
                            on_pin_change.call(PcbWiringPinEditRequest {
                                identity,
                                assignment_id: assignment_id.clone(),
                                pin: Some(value),
                            });
                        }
                    },
            }
            ToggleGroup { aria_label: format!("Pin lock for {}", row.label), horizontal: true,
                pressed: Some(if row.locked { [0usize].into_iter().collect() } else { std::collections::HashSet::new() }),
                disabled: !actions.editable,
                on_pressed_change: {
                    let assignment_id = assignment_id.clone();
                    let current_pin = row.value.clone();
                    let locked = row.locked;
                    let identity = identity.clone();
                    move |_: std::collections::HashSet<usize>| {
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
                ToggleItem { index: 0usize, aria_label: "{pin_lock_action(row.locked)} {row.label}", class: TOUCH_CLS, "{pin_lock_action(row.locked)}" }
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

/// Snapshot, wiring source, plan identity and firmware projection for the exact
/// current plan, shared by firmware-position admission and editability.
fn current_firmware_projection(
    runtime: &Runtime,
    generation: u64,
    resolution: &PcbWiringResolution,
) -> Option<(
    AcceptedSnapshot,
    PcbWiringSource,
    WiringPlanIdentity,
    FirmwarePositionProjection,
)> {
    let (snapshot, source) = pcb_source(runtime, generation)?;
    let PcbWiringResolution::Current { identity, plan } = resolution else {
        return None;
    };
    let projection = firmware_position_projection::project(
        &snapshot.document,
        &source.identity,
        &source.ui_scope,
        source.scope_generation,
        PlanLifecycle::Current(identity, plan),
    );
    Some((snapshot, source, identity.clone(), projection))
}

/// Editor-lifetime root owner for legacy SetKeyBinding admission and exact outcome settlement.
pub fn use_firmware_position_edits(
    guard: EditGuard,
    resolution: Signal<PcbWiringResolution>,
) -> FirmwarePositionActions {
    let feedback = use_signal(|| None::<FirmwarePositionFeedback>);
    let generation = guard.editor.scope_generation;
    let latest = feedback;
    let on_edit = use_guarded_edit(
        guard,
        move |runtime, request: FirmwarePositionEditRequest| {
            let Some((snapshot, source, identity, current)) =
                current_firmware_projection(runtime, generation(), &resolution.read())
            else {
                return;
            };
            if !admits_edit(
                &request.identity,
                &request.key_id,
                FirmwarePositionAdmission {
                    current_generation: generation(),
                    runtime_scope: Some(&source.ui_scope),
                    accepted: &snapshot,
                    current_plan: Some(&identity),
                    current_projection: &current,
                },
            ) {
                return;
            }
            let target = firmware_key_target(&request.identity, &request.key_id);
            settle_commit(
                runtime,
                "firmware-position",
                firmware_position_resolver(request),
                latest,
                |message| FirmwarePositionFeedback {
                    target: target.clone(),
                    state: FirmwarePositionFeedbackState::Failed(message),
                },
            );
        },
    );

    let runtime = guard.editor.runtime();
    let editable = guard.workspace_is_current()
        && guard.instance_is_current(&runtime)
        && current_firmware_projection(&runtime, generation(), &resolution.read()).is_some_and(
            |(_, _, _, projection)| projection.identity.is_some() && !projection.keys.is_empty(),
        );
    FirmwarePositionActions {
        feedback: feedback(),
        editable,
        on_edit,
    }
}

/// One bordered assignment group; empty groups render nothing.
fn assignment_group(
    title: &str,
    rows: &[&pins::PcbWiringPinAssignment],
    plan: &ElectricalPlan,
    actions: &PcbWiringPinActions,
) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "m-0 p-0 border-t border-border", InspectorSection { default_open: false, title: format!("{title} ({})", rows.len()),
            for row in rows.iter() {
                {pin_assignment_row(row, actions, plan)}
            }
        } }
    }
}

fn firmware_position_resolver(request: FirmwarePositionEditRequest) -> EditResolver {
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

pub fn use_pcb_part_net_edits(guard: EditGuard) -> PartNetActions {
    let feedback = use_signal(|| None::<PartNetFeedback>);
    let preparing_create = use_signal(|| false);
    let generation = (guard.editor.scope_generation)();
    let on_edit = use_guarded_edit(guard, move |runtime, request: PartNetEditRequest| {
        if !guard.route_is_current(generation) {
            return;
        }
        let Some((current, definition_id, source)) = current_part_net_target(runtime, generation)
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
        let mut feedback = feedback;
        let target = PartNetTarget::of(&request.identity);
        let seed = runtime.operation().0;
        if source.is_none() {
            settle_commit(
                runtime,
                "pcb-part-net",
                part_net_resolver(request, definition_id, None, false, seed),
                feedback,
                |failure| PartNetFeedback {
                    target: target.clone(),
                    failure,
                },
            );
            return;
        }

        let mut preparing_create = preparing_create;
        if create {
            preparing_create.set(true);
        }
        let runtime = runtime.clone();
        spawn(async move {
            let classified =
                crate::presentation::parts::is_generator_source(source.clone().unwrap()).await;
            if create {
                preparing_create.set(false);
            }
            if !admit_classified_part_net_edit(
                guard.route_is_current(generation),
                &request.identity,
                || live_part_net_identity(guard, &runtime, generation),
            ) {
                return;
            }
            let result = classified.and_then(|is_generator_source| {
                let resolver =
                    part_net_resolver(request, definition_id, source, is_generator_source, seed);
                runtime.commit("pcb-part-net", resolver)
            });
            feedback.set(
                result
                    .err()
                    .map(|failure| PartNetFeedback { target, failure }),
            );
        });
    });
    let identity = live_part_net_identity(guard, &guard.editor.runtime(), generation);
    let editable = identity.is_some() && guard.workspace_is_current();
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
    mode::replace_document_resolver("pcb-part-net", move |accepted, replacement| {
        let part = replacement
            .parts
            .iter()
            .find(|part| part.id == request.identity.part_id);
        let definition = part.and_then(|part| {
            replacement
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
            &request.identity.board_id,
            &request.identity.part_id,
            is_generator_source,
            intent,
        )?;
        Ok(vec![
            request.identity.board_id.clone(),
            request.identity.part_id.clone(),
        ])
    })
}

fn live_part_net_identity(
    guard: EditGuard,
    runtime: &Runtime,
    generation: u64,
) -> Option<PartNetEditIdentity> {
    if !guard.instance_is_current(runtime) {
        return None;
    }
    current_part_net_target(runtime, generation).map(|(identity, _, _)| identity)
}

/// The selected part's net-edit identity plus its definition identity and
/// generator source, resolved in one accepted-document lookup.
fn current_part_net_target(
    runtime: &Runtime,
    generation: u64,
) -> Option<(PartNetEditIdentity, String, Option<String>)> {
    let (accepted, source) = pcb_source(runtime, generation)?;
    let scope = source.ui_scope;
    let model = runtime.model();
    let [part_id] = model.selected_part_ids.as_slice() else {
        return None;
    };
    if !accepted
        .document
        .boards
        .iter()
        .any(|board| board.id == scope.board_id && board.part_ids.contains(part_id))
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
    let source = definition
        .generator
        .as_ref()
        .map(|generator| generator.source.clone());
    Some((
        PartNetEditIdentity {
            board_id: scope.board_id.clone(),
            ui_scope: scope,
            part_id: part_id.clone(),
            token: accepted.token,
            revision: accepted.document.revision,
            generation,
        },
        definition.id.clone(),
        source,
    ))
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
