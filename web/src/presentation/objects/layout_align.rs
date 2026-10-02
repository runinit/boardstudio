//! Private Layout Align toolbar controls.
pub(in crate::presentation) use super::layout_align_geometry::{
    AlignCommand, PendingSettlementGate, alignment_delta, local_matrix_delta,
    pending_settlement_gate, reconcile_reference_choice, transformed_envelope,
};
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
pub(in crate::presentation) fn LayoutAlignToolbar(mount: LayoutAlignMount) -> Element {
    let on_align = mount.on_align;
    let mut open = use_signal(|| false);
    rsx! {
        details {
            class: "m1-layout-align-menu",
            open: open(),
            onkeydown: move |event: KeyboardEvent| {
                if event.data().key().to_string() == "Escape" && open() {
                    event.prevent_default();
                    event.stop_propagation();
                    open.set(false);
                }
            },
            summary {
                onclick: move |event: MouseEvent| {
                    event.prevent_default();
                    open.set(!open());
                },
                "Align"
            }
            div { class: "m1-layout-align-popover",
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
                                let mut open = open;
                                move |_| if let Some(action) = action.clone() {
                                    on_align.call(action);
                                    open.set(false);
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
            }
        }
    }
}
