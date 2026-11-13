//! PCB wiring preview plus contextual selected-part connection projection.
//!
//! Root owns the accepted-source controller and Runtime request. This leaf receives only a
//! cheap accepted-document handle, board scope, active selection, and guarded edit actions.
use crate::pcb_controls::{PcbChoice, PcbInput, PcbSection};
use boardstudio_application::Scope;
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{Net, PartDefinition, PartKind, ProjectDoc},
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
mod controller;
pub(crate) use controller::pending_binding;
mod mode;
mod part_connections;
mod part_input_settings;
mod part_net_admission;
mod pins;
mod remap;
use crate::firmware_position_projection;
pub use crate::firmware_position_projection::{
    FirmwarePlanIdentity as WiringPlanIdentity, FirmwarePositionFeedbackTarget,
    FirmwarePositionIdentity, FirmwarePositionProjection, PlanLifecycle,
};
pub use apply::{BoardWiringApplyActions, use_board_wiring_apply};
pub use controller::{
    WiringResolutionNotice, use_firmware_position_edits, use_pcb_part_net_edits,
    use_pcb_wiring_controller, wiring_resolution_notice,
};
pub use mode::{BoardWiringModeActions, BoardWiringModeEditRequest, use_board_wiring_mode_edits};
pub use part_input_settings::PartInputActions;
pub use pins::{PcbWiringPinActions, PcbWiringPinEditRequest, use_pcb_wiring_pin_edits};
pub use remap::{ProtectedRemapActions, ProtectedRemapFeedback, use_protected_remap_review};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartNetEditIdentity {
    pub board_id: String,
    pub ui_scope: Scope,
    pub part_id: String,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

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
    Pending,
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

pub fn use_part_input_edits(
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

// A receipt travels with the exact queued create, never inferred from accepted net names.
#[derive(Clone)]
struct NewNetSubmission {
    draft: Signal<String>,
    value: String,
    admitted: Rc<std::cell::Cell<bool>>,
}

#[derive(Clone)]
struct NewNetSubmitter(EventHandler<(PartNetEditRequest, Option<NewNetSubmission>)>);

#[component]
pub fn PcbWiringInspector(props: PcbWiringInspectorProps) -> Element {
    let new_net_name = use_signal(String::new);
    let new_net_submission = use_signal(|| None::<NewNetSubmission>);
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let workspace = use_context::<crate::WorkspaceState>().0;
    let guard = use_signal(|| None::<boardstudio_web_runtime::draft_owners::DraftOwnerGuard>);
    let owner = (
        props.source.document.id.clone(),
        props.source.ui_scope.session_epoch,
    );
    use_effect(use_reactive((&owner,), {
        let runtime = runtime.clone();
        let mut guard = guard;
        move |(owner,)| {
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
            guard.set(Some(runtime.register_draft_owner(
                boardstudio_web_runtime::draft_owners::DraftOwnerRegistration {
                    project_scope: owner,
                    workspace: "PCB",
                    owner: "new-net-name",
                    admission_key: "create",
                    area: "New net name".into(),
                    remedy: "Add the net or clear its name".into(),
                    is_unresolved: Rc::new(move || {
                        if *workspace.peek() != "PCB" || new_net_name.peek().is_empty() {
                            return false;
                        }
                        !new_net_submission.peek().as_ref().is_some_and(|receipt| {
                            receipt.admitted.get() && *new_net_name.peek() == receipt.value
                        })
                    }),
                },
            )));
        }
    }));
    use_drop({
        let runtime = runtime.clone();
        let mut guard = guard;
        move || {
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
        }
    });
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
            section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground", role: "status",
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
            new_net_submission,
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
    new_net_submission: Signal<Option<NewNetSubmission>>,
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
        mut new_net_submission,
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
    let add_handler = try_consume_context::<NewNetSubmitter>();
    let add_name = new_net_name;
    let add_net = move |event: FormEvent| {
        event.prevent_default();
        let name = add_name().trim().to_owned();
        if name.is_empty() {
            return;
        }
        if let (Some(identity), Some(handler)) = (add_identity.clone(), add_handler.as_ref()) {
            let receipt = NewNetSubmission {
                draft: add_name,
                value: add_name.peek().clone(),
                admitted: Rc::new(std::cell::Cell::new(false)),
            };
            handler.0.call((
                PartNetEditRequest {
                    identity,
                    action: PartNetEditAction::CreateNet { name },
                },
                Some(receipt.clone()),
            ));
            // Rejection or asynchronous preparation never exempts the draft.
            new_net_submission.set(Some(receipt));
        }
    };
    rsx! {
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{breadcrumb}" }
            h2 { "{title}" }
            part_input_settings::PartInputInspector {
                source: source.clone(),
                actions: props.part_input_actions.clone(),
                board_details_and_connections: rsx! {
            PcbSection { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs", label: "Board details",
                ul {
                    li { span { "Board thickness" } strong { "{thickness:.2} mm" } }
                    li { span { "Placed parts" } strong { "{placed_parts}" } }
                    li { span { "Net assignments" } strong { "{assigned_pins}" } }
                }
            }
            div { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs",
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
                            let value = identity.as_ref().and_then(|identity| controller::pending_net(identity, &row.pad_ids)).unwrap_or_else(|| row.selected_net_id.clone()).unwrap_or_default();
                            rsx! {
                                PcbChoice { class: "grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5 gap-y-[3px] py-2 border-b border-border text-xs [&_strong]:[overflow-wrap:anywhere] [&_small]:col-span-full [&_small]:text-muted-foreground", key: "{row.label}",
                                        label: row.label.clone(), aria_label: format!("Net for {}", row.label), value, searchable: true,
                                        disabled: !actions.editable || identity.is_none(),
                                        on_change: move |value: String| {
                                            let Some(identity) = identity.clone() else { return; };
                                            on_edit.call(PartNetEditRequest {
                                                identity,
                                                action: PartNetEditAction::AssignPads {
                                                    pad_ids: pad_ids.clone(),
                                                    net_id: (!value.is_empty()).then_some(value),
                                                },
                                            });
                                        },
                                        options: std::iter::once((String::new(), "Unmapped".into(), false)).chain(nets.iter().map(|net| (net.id.clone(), net.name.clone(), false))).collect(),
                                }
                            }
                        }
                    }
                }
            }
                },
            }
            form { class: "grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5 gap-y-[3px] py-2 border-b border-border text-xs [&_strong]:[overflow-wrap:anywhere] [&_small]:col-span-full [&_small]:text-muted-foreground !items-center !gap-2 mt-2.5", onsubmit: add_net,
                PcbInput { label: "New net name", aria_label: "New net name", r#type: InputType::Text,
                    placeholder: Some("New net name".into()), value: new_net_name(),
                    disabled: !actions.editable || identity.is_none(),
                    oninput: move |event: FormEvent| new_net_name.set(event.value()),
                }
                Button { button_type: "submit", variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !actions.editable || identity.is_none() || new_net_name().trim().is_empty() || actions.create_pending, "Add net" }
            }
            PcbSection { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs", label: format!("Electrical nets ({})", nets.len()),
                ul {
                    for net in nets {
                        li { key: "{net.id}", span { "{net.name}" } small { "{net.pin_count} pins" } }
                    }
                    if nets.is_empty() { li { "No nets are assigned to this board." } }
                }
            }
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
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{breadcrumb}" }
            h2 { "{title}" }
            p { "Switch wiring is inherited from its key assembly." }
            div { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs",
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
            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| on_edit_board_wiring.call(()), "Edit board wiring" }
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
    let selected_mode = (mode_actions.draft)();
    let mode_pending = mode_actions.pending;
    let mode_failure = (mode_actions.failure)();
    let change_mode = mode_actions.on_change;
    let mode_identity = mode_actions.identity.clone();
    let apply_actions = props.apply_actions.clone();
    let apply_identity = apply_actions.identity.clone();
    let apply_failure = apply_actions.failure.clone();
    let release_disabled = apply_actions.release_disabled;
    let on_apply = apply_actions.on_apply;
    let pin_actions = props.pin_actions.clone();
    let pin_pending = pin_actions.pending;
    let pin_failure = pin_actions.failure.clone();
    let pin_identity = pin_actions.identity.clone();
    let mut pin_rows = matching_plan
        .map(|plan| pins::assignments(&props.source, plan))
        .unwrap_or_default();
    if let Some(identity) = &pin_identity {
        for row in &mut pin_rows {
            if let Some(value) = pins::pending_pin(identity, &row.id) {
                row.value = value;
            }
        }
    }
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
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground gap-4", aria_label: "Electrical wiring",
            p { class: "!m-0 !text-[11px]", "{display.board_name} / PCB" }
            header { class: "flex items-start justify-between gap-3 [&_h2]:!m-0 [&_p]:!mt-[5px] [&_p]:!mb-0",
                div {
                    h2 { "Routing & nets" }
                    p { "{controller_name}" }
                }
                span { class: "border border-border rounded-[3px] px-1.5 py-[3px] text-muted-foreground font-mono text-[11px] uppercase", "{selected_mode}" }
            }
            PcbSection { class: "border-t border-border py-4 [&_ul]:list-none [&_ul]:m-0 [&_ul]:p-0 [&_li]:flex [&_li]:items-baseline [&_li]:justify-between [&_li]:gap-2 [&_li]:py-1.5 [&_li]:border-b [&_li]:border-border [&_li_span]:text-muted-foreground [&_li_span]:text-xs", default_open: true, label: "Nets",
                ul {
                    for net in props.source.document.nets.iter().filter(|net| props.source.document.boards.iter().find(|board| board.id == props.source.identity.scope.board_id).is_some_and(|board| board.net_ids.contains(&net.id) || net.pins.iter().any(|pin| board.part_ids.contains(&pin.part_id)))) {
                        li { key: "{net.id}", strong { "{net.name}" } span { "{net.pins.len()} pins" } }
                    }
                }
            }
            PcbChoice {
                label: "Wiring mode",
                class: "grid gap-[5px] text-xs text-muted-foreground",
                aria_label: "Wiring mode",
                value: "{selected_mode}",
                disabled: !mode_actions.editable,
                on_change: move |value: String| {
                        let Some(identity) = mode_identity.clone() else { return; };
                        let mode = match value.as_str() {
                            "matrix" => ElectricalMode::Matrix,
                            "direct" => ElectricalMode::Direct,
                            _ => return,
                        };
                        change_mode.call(BoardWiringModeEditRequest { identity, mode });
                    },
                options: vec![("matrix".into(), "Matrix".into(), false), ("direct".into(), "Direct GPIO".into(), false)],
            }
            if display.controller_choices.is_empty() {
                p { "Place a controller from Parts to assign this board’s wiring." }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| on_choose_controller.call(()), "Add controller" }
            }
            if mode_pending {
                p { role: "status", Spinner {} "Saving wiring mode…" }
            }
            if let Some(message) = &mode_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if pin_pending {
                p { role: "status", Spinner {} "Saving wiring pin…" }
            }
            if let Some(message) = &pin_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if protected_remap_actions.transfer_revision.is_some()
                || protected_remap_actions.feedback.is_some()
            {
                section { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0", aria_label: "Protected transfer",
                    strong { "Protected transfer" }
                    if let Some(revision) = protected_remap_actions.transfer_revision {
                        p { "Pins protected by PCB transfer at revision {revision}" }
                        PcbSection { label: "Review PCB remap",
                            p { "Changing a protected assignment creates a new hardware revision. The old PCB and firmware must be regenerated before export." }
                            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
                            p { role: "status", Spinner {} "Starting a new PCB revision…" }
                        } else if matches!(feedback.state, ProtectedRemapFeedback::Saved) {
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
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: pending || display.controller_choices.is_empty(), onclick: move |_| on_resolve.call(()),
                    if pending { "Resolving…" } else { "Resolve automatically" }
                }
                Button { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !apply_actions.editable, onclick: move |_| {
                    let Some(identity) = apply_identity.clone() else { return; };
                    on_apply.call(identity);
                }, "Apply wiring" }
            }
            if let Some(message) = &apply_failure {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if let Some(plan) = matching_plan.filter(|plan| !plan.diagnostics.is_empty()) {
                Alert { variant: AlertVariant::Destructive, class: "[&_p]:!m-0 [&_p]:!text-xs",
                    strong { "Review before transfer" }
                    for (index, diagnostic) in plan.diagnostics.iter().enumerate() {
                        p { key: "{index}", "{diagnostic.severity}: {diagnostic.message}" }
                    }
                }
            }
            if let Some(review) = existing_connections.as_ref() {
                div { class: "grid gap-1.5 my-2.5 p-2.5 border border-border rounded [&>strong]:text-xs [&>strong]:text-muted-foreground [&_p]:!m-0",
                    strong { "Review existing connections" }
                    p { "{review.pin_count} pin connections already belong to {existing_connection_names}. Switching them to automatic wiring removes these assignments so the board plan can replace them. Other connections stay in place. You can undo this change." }
                    Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        disabled: !mode_actions.editable || review_connections_identity.is_none() || release_disabled,
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
                PcbSection { default_open: true, class: "m-0 p-0 border-t-0", label: format!("Assignments ({})", pin_rows.len()),
                    if plan.mode == ElectricalMode::Matrix && !assignment_groups.rows.is_empty() {
                        PcbSection { class: "m-0 p-0 border-t border-border", label: format!("Row pins ({})", assignment_groups.rows.len()),
                            for row in assignment_groups.rows.iter() {
                                {pin_assignment_row(row, &pin_actions, plan)}
                            }
                        }
                    }
                    if plan.mode == ElectricalMode::Matrix && !assignment_groups.columns.is_empty() {
                        PcbSection { class: "m-0 p-0 border-t border-border", label: format!("Column pins ({})", assignment_groups.columns.len()),
                            for row in assignment_groups.columns.iter() {
                                {pin_assignment_row(row, &pin_actions, plan)}
                            }
                        }
                    }
                    if plan.mode == ElectricalMode::Matrix && !assignment_groups.other.is_empty() {
                        PcbSection { class: "m-0 p-0 border-t border-border", label: format!("Peripheral pins ({})", assignment_groups.other.len()),
                            for row in assignment_groups.other.iter() {
                                {pin_assignment_row(row, &pin_actions, plan)}
                            }
                        }
                    }
                    if plan.mode != ElectricalMode::Matrix {
                        for row in assignment_groups.other.iter() {
                            {pin_assignment_row(row, &pin_actions, plan)}
                        }
                    }
                    if pin_rows.is_empty() {
                        p { class: "text-muted-foreground", "Resolve the board to see controller, matrix, and peripheral assignments." }
                    }
                }
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
            PcbChoice { class: "min-w-0", label_class: "sr-only",
                    label: format!("Pin for {}", row.label), aria_label: format!("Pin for {}", row.label),
                    value: row.value.clone().unwrap_or_default(), searchable: true,
                    disabled: !actions.editable || row.locked,
                    options: std::iter::once((String::new(), "Unresolved".into(), false)).chain(pin_choices.into_iter().map(|pin| (pin.clone(), pin, false))).collect(),
                    on_change: {
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
                ToggleItem { index: 0usize, aria_label: "{pin_lock_action(row.locked)} {row.label}", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{pin_lock_action(row.locked)}" }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[cfg(target_arch = "wasm32")]
    #[derive(Clone)]
    struct PinCapture(Rc<std::cell::RefCell<Option<PcbWiringPinEditRequest>>>);

    #[cfg(target_arch = "wasm32")]
    fn mounted_pin_row() -> Element {
        let capture = use_context::<PinCapture>();
        let scope = boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let identity = crate::pcb_wiring_mode_operation::BoardWiringModeIdentity {
            plan: crate::firmware_position_projection::FirmwarePlanIdentity {
                scope: scope.clone(),
                token: boardstudio_application::SnapshotToken(1),
                revision: 0,
                executor_epoch: 0,
            },
            ui_scope: scope,
            selected_part_id: None,
            scope_generation: 0,
        };
        let actions = PcbWiringPinActions {
            identity: Some(identity),
            editable: true,
            pending: false,
            failure: None,
            on_change: EventHandler::new(move |request| *capture.0.borrow_mut() = Some(request)),
        };
        let row = pins::PcbWiringPinAssignment {
            id: "row/0".into(),
            label: "Row 1".into(),
            detail: None,
            value: Some("P5".into()),
            locked: false,
        };
        let mut plan = boardstudio_core::electrical::resolve(
            boardstudio_core::electrical::ElectricalPlanRequest {
                document: ProjectDoc::empty("project", "Fixture"),
                instance_id: None,
                mode: ElectricalMode::Matrix,
                locks: Default::default(),
                controller_profile: None,
                board_id: Some("board".into()),
                controller_part_id: None,
            },
        );
        plan.free_pins = vec!["P6".into()];
        pin_assignment_row(&row, &actions, &plan)
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn pin_assignment_filters_and_retains_accepted_choice() {
        use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
        use wasm_bindgen::JsCast;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let capture = Rc::new(std::cell::RefCell::new(None));
        let dom = VirtualDom::new(mounted_pin_row);
        dom.provide_root_context(PinCapture(capture.clone()));
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("pin picker mounts", || {
            by_role_name(&root, "combobox", "Pin for Row 1").is_some()
        })
        .await;
        let picker = by_role_name(&root, "combobox", "Pin for Row 1").unwrap();
        let label = root.query_selector("label").unwrap().unwrap();
        assert!(
            label
                .get_attribute("class")
                .unwrap_or_default()
                .split_whitespace()
                .any(|class| class == "sr-only"),
            "the accessible pin label uses the new visually-hidden utility"
        );
        let input: web_sys::HtmlInputElement = picker
            .clone()
            .dyn_into()
            .expect("long pin list is searchable");
        assert_eq!(input.value(), "P5");
        picker.focus().unwrap();
        let key = |value: &str| {
            let event = web_sys::KeyboardEventInit::new();
            event.set_key(value);
            event.set_bubbles(true);
            event.set_cancelable(true);
            picker
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                        .unwrap(),
                )
                .unwrap();
        };
        key("ArrowDown");
        input.set_value("P6");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        wait_until("pin list filters", || {
            by_role_name(&root, "option", "P6").is_some()
                && by_role_name(&root, "option", "P5").is_none()
        })
        .await;
        key("ArrowDown");
        wait_until("matching pin is active", || {
            by_role_name(&root, "option", "P6").is_some_and(|option| {
                picker.get_attribute("aria-activedescendant").as_deref()
                    == Some(option.id().as_str())
            })
        })
        .await;
        key("Enter");
        wait_until("pin request emitted", || {
            capture.borrow().as_ref().is_some_and(|request| {
                request.assignment_id == "row/0" && request.pin.as_deref() == Some("P6")
            })
        })
        .await;
        assert!(by_role_name(&root, "group", "Pin lock for Row 1").is_some());
        let lock = by_role_name(&root, "button", "Lock Row 1").unwrap();
        assert_eq!(lock.get_attribute("aria-pressed").as_deref(), Some("false"));
        lock.focus().unwrap();
        lock.click();
        wait_until("lock preserves the accepted pin", || {
            capture.borrow().as_ref().is_some_and(|request| {
                request.assignment_id == "row/0" && request.pin.as_deref() == Some("P5")
            })
        })
        .await;
        root.remove();
    }

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
        let actions = PcbWiringPinActions {
            identity: None,
            editable: false,
            pending: false,
            failure: None,
            on_change: EventHandler::new(|_| {}),
        };
        let mut plan = boardstudio_core::electrical::resolve(
            boardstudio_core::electrical::ElectricalPlanRequest {
                document: ProjectDoc::empty("project", "Fixture"),
                instance_id: None,
                mode: ElectricalMode::Matrix,
                locks: Default::default(),
                controller_profile: None,
                board_id: Some("board".into()),
                controller_part_id: None,
            },
        );
        plan.free_pins = vec!["P1".into(), "P6".into()];
        rsx! { {pin_assignment_row(&retained, &actions, &plan)} {pin_assignment_row(&unresolved, &actions, &plan)} }
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
    #[wasm_bindgen_test]
    async fn wiring_pin_options_select_saved_values_and_only_unresolved_for_empty_rows() {
        use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
        use wasm_bindgen::JsCast;
        let root = mount_pin_option_fixture();
        wait_until("pin rows mount", || {
            by_role_name(&root, "combobox", "Pin for Row 1").is_some()
                && by_role_name(&root, "combobox", "Pin for left/SW25/encoder-a").is_some()
        })
        .await;
        let value = |name| {
            by_role_name(&root, "combobox", name)
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .value()
        };
        assert_eq!(value("Pin for Row 1"), "P5");
        assert_eq!(value("Pin for left/SW25/encoder-a"), "Unresolved");
        root.remove();
    }
}

#[cfg(test)]
mod test_fixtures;

#[cfg(test)]
mod queued_edit_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod draft_update_tests {
    use super::*;
    use crate::runtime::{Runtime, project_name_test_support as support};
    use boardstudio_application::Event;
    use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog, UpdateOpener};
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    struct NetUpdate;
    impl boardstudio_web_host::host::UpdateSignals for NetUpdate {
        fn snapshot(&self) -> boardstudio_web_host::offline::UpdateSnapshot {
            boardstudio_web_host::offline::UpdateSnapshot {
                waiting: true,
                installing: false,
                controlled: true,
            }
        }
        fn watch(&self, _: Rc<dyn Fn()>) -> Box<dyn boardstudio_web_host::host::UpdateWatch> {
            struct Stop;
            impl boardstudio_web_host::host::UpdateWatch for Stop {
                fn stop(&mut self) {}
            }
            Box::new(Stop)
        }
        fn adopt_waiting(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<(), boardstudio_web_host::offline::AdoptError>,
                    >,
            >,
        > {
            Box::pin(async { Ok(()) })
        }
        fn reload(&self) {}
    }

    // Reuse the real wiring controllers and accepted-source fixture. Only worker
    // adoption is substituted: the new-net form and its admission are production.
    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let _ = version();
        let mut workspace = use_signal(|| "PCB");
        use_context_provider(|| crate::WorkspaceState(workspace));
        let generation = use_signal(|| 0_u64);
        let mut show = use_signal(|| true);
        let mut hidden = use_signal(|| false);
        let accepted = runtime.model().accepted.unwrap();
        let source = PcbWiringSource {
            identity: WiringPlanIdentity {
                scope: runtime.scope().unwrap(),
                token: accepted.token,
                revision: accepted.document.revision,
                executor_epoch: runtime.electrical_preview_executor_epoch(),
            },
            ui_scope: runtime.scope().unwrap(),
            scope_generation: 0,
            active_part_id: runtime.model().selected_part_ids.first().cloned(),
            document: accepted.document.clone(),
        };
        let resolution = use_signal(|| PcbWiringResolution::Idle);
        let mode_actions = use_board_wiring_mode_edits(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
            Some(source.clone()),
            resolution,
        );
        let pin_actions = use_pcb_wiring_pin_edits(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
            Some(source.clone()),
            resolution,
        );
        let apply_actions = use_board_wiring_apply(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
            Some(source.clone()),
            resolution,
        );
        let protected_remap_actions = use_protected_remap_review(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
            Some(source.clone()),
        );
        let part_net_actions = use_pcb_part_net_edits(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
        );
        let part_input_actions = part_input_settings::use_part_input_edits(
            runtime.clone(),
            version,
            workspace,
            generation,
            Rc::new(|| true),
        );
        let firmware_positions =
            firmware_position_projection(&source, 0, &PcbWiringResolution::Idle);
        let switch_runtime = runtime.clone();
        rsx! {
            div { hidden: hidden(),
                if show() {
                    PcbWiringInspector { source, resolution: PcbWiringResolution::Idle, firmware_positions, firmware_feedback: None, firmware_controls: rsx! {}, part_net_actions, part_input_actions, on_firmware_edit: |_| {}, on_resolve: |_| {}, on_choose_controller: |_| {}, on_edit_board_wiring: |_| {}, mode_actions, pin_actions, apply_actions, protected_remap_actions }
                }
            }
            button { onclick: move |_| switch_runtime.submit(Event::SelectParts { operation_id: switch_runtime.operation(), part_ids: vec!["diode-two".into()], range_part_ids: Vec::new(), mode: boardstudio_application::SelectionMode::Replace }), "Switch part" }
            button { onclick: move |_| hidden.set(!hidden()), "Collapse wiring" }
            button { onclick: move |_| workspace.set("Parts"), "Leave PCB" }
            button { onclick: move |_| show.set(false), "Unmount wiring" }
        }
    }

    async fn mounted() -> (Rc<Runtime>, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut document = test_fixtures::document();
        document
            .parts
            .push(test_fixtures::test_part("diode-two", "diode-definition"));
        document.boards[0].part_ids.push("diode-two".into());
        support::open_document(&runtime, document).await;
        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id: "left".into(),
            instance_id: None,
        });
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: vec!["matrix/m/r0c0/diode".into()],
            range_part_ids: Vec::new(),
            mode: boardstudio_application::SelectionMode::Replace,
        });
        let doc = web_sys::window().unwrap().document().unwrap();
        let root = doc.create_element("div").unwrap();
        doc.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("new-net name mounts", || {
            by_role_name(&root, "textbox", "New net name").is_some()
        })
        .await;
        runtime.observe_update_waiting(Rc::new(NetUpdate));
        (runtime, root)
    }

    fn enter(root: &web_sys::Element, name: &str) -> web_sys::HtmlInputElement {
        let input = by_role_name(root, "textbox", "New net name")
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        input.set_value(name);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
        input
    }

    fn areas(runtime: &Rc<Runtime>, opener: UpdateOpener) -> Vec<String> {
        runtime.cancel_update_dialog();
        runtime.begin_update(opener);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => blockers
                .into_iter()
                .filter_map(|blocker| match blocker {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "New net name");
                        assert_eq!(remedy, "Add the net or clear its name");
                        Some(area)
                    }
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
            other => panic!("unexpected update dialog: {other:?}"),
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_new_net_exact_admission_failure_newer_input_and_rejection_013() {
        use boardstudio_web_ui_shared::test_wait::wait_until_step;
        let (runtime, root) = mounted().await;
        let input = enter(&root, "  Submitted net  ");
        gloo_timers::future::TimeoutFuture::new(20).await;
        let (mut entered, release) = support::gate_next_core_reply(&runtime);
        by_role_name(&root, "button", "Add net").unwrap().click();
        assert_eq!(
            areas(&runtime, UpdateOpener::Header),
            vec!["New net name"],
            "queued preparation is not admission"
        );
        wait_until_step("new net enters Core", async || {
            support::drive_pending(&runtime);
            entered.try_recv().unwrap().is_some()
        })
        .await;
        assert!(
            areas(&runtime, UpdateOpener::Toast).is_empty(),
            "only the exact admitted value is exempt"
        );
        enter(&root, "Newer net");
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(areas(&runtime, UpdateOpener::Header), vec!["New net name"]);
        release.send(()).unwrap();
        wait_until_step("new-net landing preserves newer input", async || {
            support::run_pending(&runtime).await;
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .nets
                .iter()
                .any(|net| net.name == "Submitted net")
                && !by_role_name(&root, "button", "Add net")
                    .unwrap()
                    .has_attribute("disabled")
        })
        .await;
        assert_eq!(input.value(), "Newer net");
        assert_eq!(areas(&runtime, UpdateOpener::Toast), vec!["New net name"]);

        by_role_name(&root, "button", "Add net").unwrap().click();
        wait_until_step("own successful create clears its exact draft", async || {
            support::run_pending(&runtime).await;
            input.value().is_empty()
        })
        .await;

        enter(&root, "Rejected stale form");
        gloo_timers::future::TimeoutFuture::new(20).await;
        let count = runtime.model().accepted.unwrap().document.nets.len();
        // The old mounted form submits before selection's rerender: admission must reject it.
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: vec!["diode-two".into()],
            range_part_ids: Vec::new(),
            mode: boardstudio_application::SelectionMode::Replace,
        });
        by_role_name(&root, "button", "Add net").unwrap().click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&runtime).await;
        assert_eq!(runtime.model().accepted.unwrap().document.nets.len(), count);
        assert_eq!(input.value(), "Rejected stale form");
        assert_eq!(areas(&runtime, UpdateOpener::Header), vec!["New net name"]);

        // An unrelated accepted net with the same name is not evidence of success.
        enter(&root, "Submitted net");
        gloo_timers::future::TimeoutFuture::new(20).await;
        support::fail_next_core_reply(&runtime, "controlled new-net failure");
        by_role_name(&root, "button", "Add net").unwrap().click();
        wait_until_step("new-net executor failure requires recovery", async || {
            support::run_pending(&runtime).await;
            runtime.model().lifecycle == boardstudio_application::Lifecycle::RecoveryRequired
        })
        .await;
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(input.value(), "Submitted net");
        assert_eq!(runtime.model().accepted.unwrap().document.nets.len(), count);
        assert_eq!(areas(&runtime, UpdateOpener::Header), vec!["New net name"]);
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_new_net_pending_unmount_retires_without_touching_dropped_signals_013() {
        use boardstudio_web_ui_shared::test_wait::wait_until_step;
        let (runtime, root) = mounted().await;
        enter(&root, "Unmounted net");
        gloo_timers::future::TimeoutFuture::new(20).await;
        let (mut entered, release) = support::gate_next_core_reply(&runtime);
        by_role_name(&root, "button", "Add net").unwrap().click();
        wait_until_step("pending new net enters Core", async || {
            support::drive_pending(&runtime);
            entered.try_recv().unwrap().is_some()
        })
        .await;
        enter(&root, "Newer unsubmitted name");
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(areas(&runtime, UpdateOpener::Toast), vec!["New net name"]);
        by_role_name(&root, "button", "Unmount wiring")
            .unwrap()
            .click();
        wait_until("wiring unmounts", || {
            by_role_name(&root, "textbox", "New net name").is_none()
        })
        .await;
        assert!(areas(&runtime, UpdateOpener::Header).is_empty());
        release.send(()).unwrap();
        wait_until_step("unmounted new net still settles durably", async || {
            support::run_pending(&runtime).await;
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .nets
                .iter()
                .any(|net| net.name == "Unmounted net")
                && matches!(
                    runtime.model().durability,
                    boardstudio_application::Durability::Saved { .. }
                )
        })
        .await;
        assert!(areas(&runtime, UpdateOpener::Toast).is_empty());
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_new_net_own_landing_clears_update_deferral_013() {
        use boardstudio_web_ui_shared::test_wait::wait_until_step;
        let (runtime, root) = mounted().await;
        let input = enter(&root, "Own successful net");
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(areas(&runtime, UpdateOpener::Toast), vec!["New net name"]);
        by_role_name(&root, "button", "Add net").unwrap().click();
        wait_until_step("own net lands and its draft clears", async || {
            support::run_pending(&runtime).await;
            input.value().is_empty()
                && matches!(
                    runtime.model().durability,
                    boardstudio_application::Durability::Saved { .. }
                )
        })
        .await;
        assert!(areas(&runtime, UpdateOpener::Header).is_empty());
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_new_net_workspace_departure_retires_unsubmitted_name_013() {
        let (runtime, root) = mounted().await;
        let input = enter(&root, "Unsubmitted departing name");
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(areas(&runtime, UpdateOpener::Toast), vec!["New net name"]);
        by_role_name(&root, "button", "Switch part")
            .unwrap()
            .click();
        wait_until("departing name survives part switching", || {
            runtime.model().selected_part_ids == vec!["diode-two"]
        })
        .await;
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(areas(&runtime, UpdateOpener::Header), vec!["New net name"]);
        assert_eq!(input.value(), "Unsubmitted departing name");
        by_role_name(&root, "button", "Collapse wiring")
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert!(input.closest("[hidden]").unwrap().is_some());
        assert_eq!(areas(&runtime, UpdateOpener::Toast), vec!["New net name"]);
        assert_eq!(input.value(), "Unsubmitted departing name");
        by_role_name(&root, "button", "Leave PCB").unwrap().click();
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert!(areas(&runtime, UpdateOpener::Header).is_empty());
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_new_net_valid_invalid_lifetime_and_revert_013() {
        for opener in [UpdateOpener::Toast, UpdateOpener::Header] {
            for name in ["Unapplied net", "   "] {
                let (runtime, root) = mounted().await;
                let input = enter(&root, name);
                gloo_timers::future::TimeoutFuture::new(20).await;
                assert_eq!(areas(&runtime, opener), vec!["New net name"]);
                assert_eq!(input.value(), name);
                by_role_name(&root, "button", "Switch part")
                    .unwrap()
                    .click();
                wait_until("part switches", || {
                    runtime.model().selected_part_ids == vec!["diode-two"]
                })
                .await;
                gloo_timers::future::TimeoutFuture::new(20).await;
                assert_eq!(areas(&runtime, opener), vec!["New net name"]);
                by_role_name(&root, "button", "Collapse wiring")
                    .unwrap()
                    .click();
                gloo_timers::future::TimeoutFuture::new(20).await;
                assert_eq!(areas(&runtime, opener), vec!["New net name"]);
                enter(&root, "");
                gloo_timers::future::TimeoutFuture::new(20).await;
                assert!(areas(&runtime, opener).is_empty());
                runtime.unsubscribe();
                root.remove();
            }
        }
    }
}
