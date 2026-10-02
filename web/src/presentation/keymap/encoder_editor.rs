//! Private encoder binding rows, reusing the shared KeyBinding editor.
use super::binding_controller::EncoderEditorProjection;
use super::binding_editor::{
    BindingEditFeedback, BindingEditRequest, BindingEditor, BindingTarget,
};
use boardstudio_core::model::EncoderDirection;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct EncoderEditorProps {
    pub(in crate::presentation) projection: Rc<EncoderEditorProjection>,
    pub(in crate::presentation) editor_instance_id: u64,
    pub(in crate::presentation) request_sequence: Signal<u64>,
    pub(in crate::presentation) enabled: bool,
    pub(in crate::presentation) feedback: Option<BindingEditFeedback>,
    pub(in crate::presentation) on_change: EventHandler<BindingEditRequest>,
}

#[component]
pub(in crate::presentation) fn EncoderEditor(props: EncoderEditorProps) -> Element {
    let projection = props.projection;
    let rows = projection.rows.clone();
    let input_identity = projection.input_identity.clone();
    let active_layer_id = projection.effective_layer_id.clone();
    let layers = projection.layers.clone();
    let macros = projection.macros.clone();
    let editor_instance_id = props.editor_instance_id;
    let request_sequence = props.request_sequence;
    let enabled = props.enabled;
    let feedback = props.feedback.clone();
    let on_change = props.on_change;
    let row_count = rows.len();

    rsx! {
        section { class: "m1-keymap-encoder-editor", "aria-label": "Encoders",
            header { class: "m1-keymap-encoder-heading",
                h3 { "Encoders" }
                span { "{row_count}" }
            }
            if rows.is_empty() {
                p { class: "m1-keymap-empty", role: "status",
                    "Add a rotary encoder in Layout. Configure its GPIOs in PCB, then assign rotation here."
                }
            } else {
                for row in rows.iter() {
                    {
                        let encoder_id = row.id.to_string();
                        let label = row.label.to_string();
                        let clockwise_target = BindingTarget::EncoderRotation {
                            encoder_id: encoder_id.clone(),
                            direction: EncoderDirection::Clockwise,
                        };
                        let counterclockwise_target = BindingTarget::EncoderRotation {
                            encoder_id: encoder_id.clone(),
                            direction: EncoderDirection::Counterclockwise,
                        };
                        let row_key = encoder_id.clone();
                        let clockwise_key = clockwise_target.stable_key();
                        let counterclockwise_key = counterclockwise_target.stable_key();
                        let clockwise_component_key = format!("{editor_instance_id}-{clockwise_key}");
                        let counterclockwise_component_key = format!("{editor_instance_id}-{counterclockwise_key}");
                        let clockwise_label = format!("{label} clockwise");
                        let counterclockwise_label = format!("{label} counterclockwise");
                        let clockwise = row.clockwise.clone();
                        let counterclockwise = row.counterclockwise.clone();
                        let push = match (&row.push_key_id, &row.push_binding) {
                            (Some(key_id), Some(binding)) => Some((key_id.to_string(), binding.clone())),
                            _ => None,
                        };
                        rsx! {
                            div { key: "{row_key}", class: "m1-keymap-encoder-row",
                                h4 { "{label}" }
                                details { key: "{clockwise_key}", open: true,
                                    summary { "Clockwise" }
                                    BindingEditor {
                                        key: "{clockwise_component_key}",
                                        scope: input_identity.scope.clone(),
                                        admission_token: input_identity.token,
                                        admission_revision: input_identity.revision,
                                        input_identity: Some(input_identity.clone()),
                                        active_layer_id: active_layer_id.clone(),
                                        target: clockwise_target,
                                        key_label: clockwise_label,
                                        editor_instance_id,
                                        request_sequence,
                                        value: clockwise,
                                        layers: layers.clone(),
                                        macros: macros.clone(),
                                        enabled,
                                        feedback: feedback.clone(),
                                        on_change,
                                    }
                                }
                                details { key: "{counterclockwise_key}", open: true,
                                    summary { "Counterclockwise" }
                                    BindingEditor {
                                        key: "{counterclockwise_component_key}",
                                        scope: input_identity.scope.clone(),
                                        admission_token: input_identity.token,
                                        admission_revision: input_identity.revision,
                                        input_identity: Some(input_identity.clone()),
                                        active_layer_id: active_layer_id.clone(),
                                        target: counterclockwise_target,
                                        key_label: counterclockwise_label,
                                        editor_instance_id,
                                        request_sequence,
                                        value: counterclockwise,
                                        layers: layers.clone(),
                                        macros: macros.clone(),
                                        enabled,
                                        feedback: feedback.clone(),
                                        on_change,
                                    }
                                }
                                if let Some((key_id, binding)) = push {
                                    {
                                        let target = BindingTarget::EncoderPush {
                                            encoder_id: encoder_id.clone(),
                                            key_id: key_id.clone(),
                                        };
                                        let target_key = target.stable_key();
                                        let key_label = format!("{label} push button");
                                        rsx! {
                                            details { key: "{target_key}",
                                                summary { "Push button" }
                                                BindingEditor {
                                                    key: "{editor_instance_id}-{target_key}",
                                                    scope: input_identity.scope.clone(),
                                                    admission_token: input_identity.token,
                                                    admission_revision: input_identity.revision,
                                                    input_identity: Some(input_identity.clone()),
                                                    active_layer_id: active_layer_id.clone(),
                                                    target,
                                                    key_label,
                                                    editor_instance_id,
                                                    request_sequence,
                                                    value: binding,
                                                    layers: layers.clone(),
                                                    macros: macros.clone(),
                                                    enabled,
                                                    feedback: feedback.clone(),
                                                    on_change,
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
