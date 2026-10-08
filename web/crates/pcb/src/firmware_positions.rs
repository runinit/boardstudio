//! Private presentation for the legacy PCB firmware-position map.
//!
//! The selected-board plan and bindings are projected by F5; this component only renders that
//! accepted value and turns a select change into the existing root-owned edit request.
use super::pcb_wiring::{
    FirmwarePositionEditRequest, FirmwarePositionFeedback, FirmwarePositionFeedbackState,
};
use crate::firmware_position_choices;
use crate::firmware_position_projection::{FirmwarePositionProjection, FirmwarePositionState};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FirmwareKeymapPanelProps {
    pub projection: FirmwarePositionProjection,
    pub feedback: Option<FirmwarePositionFeedback>,
    pub editable: bool,
    pub on_change: EventHandler<FirmwarePositionEditRequest>,
}

#[component]
pub fn FirmwareKeymapPanel(props: FirmwareKeymapPanelProps) -> Element {
    let keys = props.projection.keys.as_ref();
    let assigned = keys
        .iter()
        .filter(|key| {
            props
                .projection
                .bindings
                .get(&key.id)
                .is_some_and(|binding| binding != "&none")
        })
        .count();
    let ready = matches!(&props.projection.state, FirmwarePositionState::Current)
        && props.projection.identity.is_some();
    let disabled = !ready || !props.editable;

    let content = match &props.projection.state {
        FirmwarePositionState::Idle => rsx! {
            p { class: "firmware-keymap-message", role: "status",
                "Resolve a current wiring plan to edit firmware positions."
            }
        },
        FirmwarePositionState::Pending => rsx! {
            p { class: "firmware-keymap-message", role: "status", "Loading firmware positions…" }
        },
        FirmwarePositionState::Failed(message) => rsx! {
            p { class: "firmware-keymap-message", role: "alert", "Could not load firmware positions: {message}" }
        },
        FirmwarePositionState::Current if keys.is_empty() => rsx! {
            p { class: "firmware-keymap-message", role: "status", "The current plan has no editable key positions." }
        },
        FirmwarePositionState::Current => {
            let on_change = props.on_change;
            rsx! {
                div { class: "firmware-keymap-body",
                    div { class: "firmware-keymap-grid",
                        for key in keys {
                            {
                                let key_id = key.id.clone();
                                let label = key.label.clone();
                                let id = format!("firmware-key-{}", key.id);
                                let identity = props.projection.identity.clone();
                                let accepted_value = props.projection.bindings
                                    .get(&key.id)
                                    .map_or("&none", String::as_str)
                                    .to_owned();
                                let value = identity.as_ref().and_then(|identity| super::pcb_wiring::pending_binding(identity, &key.id)).unwrap_or(accepted_value);
                                let choices = firmware_position_choices::choices();
                                rsx! {
                                    label { class: "firmware-keymap-row", r#for: "{id}", key: "{key_id}",
                                        span { "{label}" }
                                        select {
                                            id: "{id}",
                                            aria_label: "Binding for {label}",
                                            disabled,
                                            onchange: move |event| {
                                                if let Some(identity) = identity.clone() {
                                                    on_change.call(FirmwarePositionEditRequest {
                                                        identity,
                                                        key_id: key_id.clone(),
                                                        binding: event.value(),
                                                    });
                                                }
                                            },
                                            for (choice_value, choice_label) in choices {
                                                option {
                                                    value: "{choice_value}",
                                                    selected: choice_value == value,
                                                    "{choice_label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "firmware-keymap-preview", aria_label: "Firmware keymap preview",
                        for key in keys {
                            {
                                let key_id = key.id.clone();
                                let label = key.label.clone();
                                let binding = props.projection.bindings.get(&key.id).map_or("&none", String::as_str).to_owned();
                                rsx! {
                                    div { class: "firmware-keymap-preview-row", key: "{key_id}",
                                        span { "{label}" }
                                        code { "{binding}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let feedback = props
        .feedback
        .as_ref()
        .map(|feedback| match &feedback.state {
            FirmwarePositionFeedbackState::Pending => rsx! {
                p { class: "firmware-keymap-feedback", role: "status", "Saving firmware position…" }
            },
            FirmwarePositionFeedbackState::Failed(message) => rsx! {
                p { class: "firmware-keymap-feedback", role: "alert", "{message}" }
            },
        });

    rsx! {
        details { class: "firmware-keymap-panel",
            summary {
                span { "Firmware keymap" }
                span { class: "firmware-keymap-summary", "{assigned}/{keys.len()} assigned" }
            }
            {content}
            {feedback}
        }
    }
}
