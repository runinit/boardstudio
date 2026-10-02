//! Read-only PCB wiring preview and selected-switch terminal projection.
//!
//! Root owns the accepted-source controller and Runtime request. This leaf receives only a
//! cheap accepted-document handle, board scope, ordered active selection, and one guarded plan.
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    model::{Net, PartDefinition, PartKind, ProjectDoc},
};
use dioxus::prelude::*;
use std::{rc::Rc, sync::Arc};

mod controller;
pub(in crate::presentation) use controller::{PcbWiringMount, use_pcb_wiring_controller};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct WiringPlanIdentity {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub executor_epoch: u64,
}

/// A cheap view of the accepted source. Arc identity avoids deep document comparisons in the
/// Dioxus props diff; the leaf never receives writable Session access.
#[derive(Clone)]
pub(in crate::presentation) struct PcbWiringSource {
    pub identity: WiringPlanIdentity,
    document: Arc<ProjectDoc>,
    active_part_id: Option<String>,
}

impl PcbWiringSource {
    pub(in crate::presentation) fn new(
        accepted: &AcceptedSnapshot,
        scope: &Scope,
        active_part_id: Option<&str>,
        executor_epoch: u64,
    ) -> Option<Self> {
        if scope.instance_id.is_some()
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum SelectionView {
    Board,
    Switch {
        title: String,
        breadcrumb: String,
        terminals: Vec<TerminalRow>,
    },
    Unsupported,
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
    pub on_resolve: EventHandler<()>,
    pub on_edit_board_wiring: EventHandler<()>,
}

#[component]
pub(in crate::presentation) fn PcbWiringInspector(props: PcbWiringInspectorProps) -> Element {
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
                _ => SelectionView::Unsupported,
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
            if let Some(plan) = matching_plan {
                let plan_ready = !plan
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == "error");
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
    use boardstudio_application::SessionEpoch;

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
