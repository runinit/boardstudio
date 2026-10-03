//! Private Layout Align toolbar controls.
pub(in crate::presentation) use super::layout_align_geometry::{
    AlignCommand, PendingSettlementGate, alignment_delta, local_matrix_delta,
    pending_settlement_gate, reconcile_reference_choice, transformed_envelope,
};
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::tree::TreeContext;
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct AlignReference {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct AlignFeedback {
    pub reference: String,
    pub command: AlignCommand,
    pub message: String,
    pub succeeded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct AlignAction {
    pub workspace: &'static str,
    pub scope: Scope,
    pub scope_generation: u64,
    pub context: TreeContext,
    pub moving_ids: Vec<String>,
    pub reference_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub command: AlignCommand,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct LayoutAlignMount {
    pub references: Vec<AlignReference>,
    pub selected_reference: Option<String>,
    pub action: Option<AlignAction>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub busy: bool,
    pub feedback: Option<AlignFeedback>,
    pub on_reference: EventHandler<String>,
    pub on_align: EventHandler<AlignAction>,
}

#[component]
pub(in crate::presentation) fn LayoutAlignToolbar(
    mount: LayoutAlignMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    show_relationships: bool,
    has_selection_context: bool,
    on_show_relationships: EventHandler<()>,
) -> Element {
    let on_align = mount.on_align;
    let is_open = open_menu() == Some(LayoutCommandMenu::Align);
    rsx! {
        details {
            class: "m1-layout-command-menu m1-layout-align-menu",
            "data-layout-menu": "align",
            open: is_open,
            onkeydown: move |event: KeyboardEvent| {
                if event.data().key().to_string() == "Escape" && open_menu() == Some(LayoutCommandMenu::Align) {
                    event.prevent_default();
                    event.stop_propagation();
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Align);
                }
            },
            summary {
                id: "m1-layout-align-trigger",
                "aria-controls": "m1-layout-align-menu",
                "aria-expanded": "{is_open}",
                onclick: move |event: MouseEvent| {
                    event.prevent_default();
                    let mut open_menu = open_menu;
                    open_menu.set((open_menu() != Some(LayoutCommandMenu::Align)).then_some(LayoutCommandMenu::Align));
                },
                "Align"
            }
            div { id: "m1-layout-align-menu", class: "m1-layout-align-popover",
                super::layout_toolbar::LayoutCommandMenuHeader {
                    label: "Align".to_owned(),
                    close_label: "Close Align".to_owned(),
                    open_menu,
                    menu: LayoutCommandMenu::Align,
                }
                label { "Reference part"
                    select {
                        "aria-label": "Alignment reference",
                        value: mount.selected_reference.clone().unwrap_or_default(),
                        disabled: mount.references.is_empty(),
                        onchange: move |event: FormEvent| mount.on_reference.call(event.value()),
                        if mount.references.is_empty() {
                            option { value: "", disabled: true, "No independent reference parts" }
                        }
                        for part in &mount.references {
                            option { key: "{part.id}", value: "{part.id}", "{part.label}" }
                        }
                    }
                }
                if let Some(reason) = mount.disabled_reason.as_ref() {
                    p { class: "m1-layout-align-status", role: "status", "{reason}" }
                }
                div { class: "m1-layout-align-actions", role: "group", aria_label: "Align selection",
                    for command in AlignCommand::ALL {
                        button {
                            key: "{command.label()}",
                            r#type: "button",
                            disabled: !mount.enabled || mount.busy || mount.selected_reference.is_none(),
                            onclick: {
                                let action = mount.action.as_ref().map(|action| AlignAction { command, ..action.clone() });
                                move |_| if let Some(action) = action.clone() {
                                    on_align.call(action);
                                    close_layout_command_menu(open_menu, LayoutCommandMenu::Align);
                                }
                            },
                            "{command.label()}"
                        }
                    }
                }
                if let Some(feedback) = mount.feedback.as_ref() {
                    p {
                        title: "{feedback.command.label()} alignment",
                        class: if feedback.succeeded { "m1-layout-align-feedback" } else { "m1-layout-align-feedback is-error" },
                        role: if feedback.succeeded { "status" } else { "alert" },
                        "{feedback.message}"
                    }
                }
                if show_relationships {
                    button {
                        r#type: "button",
                        disabled: !has_selection_context,
                        onclick: move |_| {
                            on_show_relationships.call(());
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Align);
                        },
                        "Relationships"
                    }
                }
            }
        }
    }
}
