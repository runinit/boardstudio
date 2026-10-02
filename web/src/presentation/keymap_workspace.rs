//! Keymap-owned canvas and Inspector composition.
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use super::{keymap, objects};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Contour;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

pub(super) struct CanvasInput {
    pub(super) view: Option<Rc<keymap::KeymapView>>,
    pub(super) contours: Rc<[Contour]>,
    pub(super) view_box: String,
    pub(super) selected_ids: BTreeSet<String>,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_select_key: EventHandler<String>,
}

pub(super) struct InspectorInput {
    pub(super) view: Option<Rc<keymap::KeymapView>>,
    pub(super) scope: Scope,
    pub(super) layer_actions: keymap::LayerActions,
    pub(super) active_layer_id: String,
    pub(super) selected_key_id: Option<String>,
    pub(super) on_layer: EventHandler<String>,
    pub(super) on_select_key: EventHandler<String>,
    pub(super) on_export: EventHandler<()>,
    pub(super) firmware_export_enabled: bool,
    pub(super) binding_actions: keymap::BindingActions,
    pub(super) macro_actions: keymap::MacroActions,
    pub(super) admission_token: SnapshotToken,
    pub(super) admission_revision: u64,
    pub(super) scope_generation: u64,
}

pub(super) fn objects(input: SharedObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.selected_context,
            on_select: input.on_select,
            on_navigate: input.on_navigate,
            on_nudge: input.on_nudge,
            matrix_setup: None,
            mirrored_pair: None,
            pair_created: None,
            on_place_component: None,
            placement_error: None,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    if let Some(view) = input.view {
        let handlers = input.handlers;
        rsx! {
            svg {
                class: "m1-canvas m1-keymap-canvas",
                view_box: "{input.view_box}",
                preserve_aspect_ratio: "xMidYMid meet",
                tabindex: "0",
                role: "group",
                "aria-label": "Keymap layout; select a key with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: handlers.mount,
                onpointerdown: handlers.start_pan,
                onpointermove: handlers.move_pointer,
                onpointerup: handlers.end_pointer,
                onpointercancel: handlers.cancel_pointer,
                onlostpointercapture: handlers.cancel_pointer,
                onkeydown: handlers.keyboard,
                onkeyup: handlers.key_up,
                onwheel: handlers.wheel,
                g { transform: "scale(1,-1)",
                    keymap::KeymapCanvas {
                        view,
                        contours: input.contours,
                        selected_ids: input.selected_ids,
                        on_select_key: input.on_select_key,
                    }
                }
            }
        }
    } else {
        rsx! { p { class: "m1-keymap-unavailable", role: "status", "Keymap is unavailable for the current board." } }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    if let Some(view) = input.view {
        let render_scope = input.scope;
        let binding_actions = input.binding_actions;
        let macro_actions = input.macro_actions;
        let layer_actions = input.layer_actions;
        let keys_editor = if let (Some(binding), Some(key_id)) = (
            binding_actions.projection.as_ref(),
            input.selected_key_id.as_ref(),
        ) {
            rsx! {
                keymap::BindingEditor {
                    key: "{render_scope:?}:{binding.effective_layer_id}:{key_id}:{binding_actions.editor_instance_id}",
                    scope: render_scope.clone(),
                    admission_token: input.admission_token,
                    admission_revision: input.admission_revision,
                    active_layer_id: binding.effective_layer_id.clone(),
                    target: keymap::BindingTarget::Key { key_id: key_id.clone() },
                    input_identity: None,
                    key_label: binding.key_label.clone(),
                    editor_instance_id: binding_actions.editor_instance_id,
                    request_sequence: binding_actions.request_sequence,
                    value: binding.binding.clone(),
                    layers: binding.layers.clone(),
                    macros: binding.macros.clone(),
                    enabled: binding_actions.enabled,
                    feedback: binding_actions.feedback.clone(),
                    on_change: binding_actions.on_change,
                }
            }
        } else {
            rsx! {}
        };
        let macros_editor = if let Some(source) = macro_actions.source.as_ref() {
            rsx! {
                keymap::MacroEditor {
                    key: "{render_scope:?}:macros:{macro_actions.editor_instance_id}",
                    scope: render_scope.clone(),
                    scope_generation: input.scope_generation,
                    source: source.clone(),
                    sequences: macro_actions.sequences.clone(),
                    editor_instance_id: macro_actions.editor_instance_id,
                    request_sequence: macro_actions.request_sequence,
                    enabled: macro_actions.enabled,
                    feedback: macro_actions.feedback.clone(),
                    on_change: macro_actions.on_change,
                }
            }
        } else {
            rsx! {
                p { class: "m1-keymap-unavailable", role: "status", "Macro editor is unavailable for the current board." }
            }
        };
        let encoders_editor = if let Some(projection) = binding_actions.encoder_projection.as_ref()
        {
            rsx! {
                keymap::EncoderEditor {
                    key: "{render_scope:?}:encoders:{projection.effective_layer_id}:{projection.input_identity.projection_generation}:{binding_actions.editor_instance_id}",
                    projection: projection.clone(),
                    editor_instance_id: binding_actions.editor_instance_id,
                    request_sequence: binding_actions.request_sequence,
                    enabled: binding_actions.enabled,
                    feedback: binding_actions.feedback.clone(),
                    on_change: binding_actions.on_change,
                }
            }
        } else {
            rsx! {}
        };
        rsx! {
            keymap::KeymapPanel {
                view,
                scope: render_scope.clone(),
                layer_operations_enabled: layer_actions.enabled,
                layer_feedback: layer_actions.feedback.clone(),
                on_layer_operation: layer_actions.on_operation,
                active_layer_id: input.active_layer_id,
                selected_key_id: input.selected_key_id.clone(),
                on_layer: input.on_layer,
                on_select_key: input.on_select_key,
                on_export: input.on_export,
                firmware_export_enabled: input.firmware_export_enabled,
                keys_editor,
                macros_editor,
                encoders_editor,
            }
        }
    } else {
        rsx! {
            section { class: "m1-keymap-panel", "aria-label": "Keymap",
                p { class: "m1-keymap-unavailable", role: "status", "Keymap is unavailable for the current board." }
            }
        }
    }
}
