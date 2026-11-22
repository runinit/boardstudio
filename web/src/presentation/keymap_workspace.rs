//! Keymap-owned canvas and Inspector composition.
use super::Runtime;
use super::keymap;
use super::workspace_composition::CanvasEventHandlers;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Contour;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct KeymapWorkspaceState {
    layer_id: Signal<String>,
}

pub(super) fn use_keymap_workspace_state() -> KeymapWorkspaceState {
    KeymapWorkspaceState {
        layer_id: use_signal(|| "base".to_owned()),
    }
}

impl KeymapWorkspaceState {
    pub(super) fn active_layer_id(self) -> Signal<String> {
        self.layer_id
    }

    pub(super) fn select_layer(&self, layer_id: String) {
        let mut active = self.layer_id;
        active.set(layer_id);
    }
}

pub(super) fn use_projection(
    runtime: Rc<Runtime>,
    accepted_token: Option<SnapshotToken>,
    current_scope: Option<Scope>,
    active_board_id: String,
    layer_id: String,
) -> Option<(Rc<keymap::KeymapView>, Rc<[Contour]>)> {
    use_memo(use_reactive(
        (&accepted_token, &current_scope, &active_board_id, &layer_id),
        move |(token, scope, board_id, layer_id)| {
            let model = runtime.model();
            let snapshot = model.accepted.as_ref()?;
            if token.as_ref() != Some(&snapshot.token) {
                return None;
            }
            let view = keymap::project(snapshot, scope.as_ref(), &board_id, layer_id.as_ref())?;
            Some((view, super::accepted_board_contours(snapshot, &board_id)))
        },
    ))
    .read()
    .clone()
}

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

pub(super) struct NavigationInput {
    pub(super) view: Option<Rc<keymap::KeymapView>>,
    pub(super) scope: Scope,
    pub(super) active_layer_id: String,
    pub(super) selected_key_id: Option<String>,
    pub(super) layer_actions: keymap::LayerActions,
    pub(super) on_layer: EventHandler<String>,
    pub(super) on_select_key: EventHandler<String>,
}

pub(super) fn objects(input: NavigationInput) -> Element {
    rsx! {
        if let Some(view) = input.view {
            keymap::KeymapNavigation {
                view, scope: input.scope, active_layer_id: input.active_layer_id,
                selected_key_id: input.selected_key_id,
                layer_operations_enabled: input.layer_actions.enabled,
                layer_feedback: input.layer_actions.feedback,
                on_layer_operation: input.layer_actions.on_operation,
                on_layer: input.on_layer, on_select_key: input.on_select_key,
            }
        } else {
            p { role: "status", "Layers are unavailable for the current board." }
        }
    }
}

pub(super) fn toolbar(input: super::shared_viewer::DesignViewToolbarProps) -> Element {
    rsx! {
        super::shared_viewer::DesignViewToolbar {
            contextual_tools: input.contextual_tools, label: input.label, detail: input.detail,
            assembly_3d: input.assembly_3d, footprints_visible: input.footprints_visible,
            on_view_mode: input.on_view_mode, on_toggle_footprints: input.on_toggle_footprints,
        }
    }
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    if let Some(view) = input.view {
        let h = input.handlers;
        rsx! {
            svg {
                class: "block w-full h-full min-h-[250px] bg-canvas touch-none max-[760px]:h-[54svh] max-[760px]:min-h-[270px]",
                view_box: "{input.view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group",
                "aria-label": "Keymap layout; select a key with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: h.mount, onpointerdown: h.start_pan, onpointermove: h.move_pointer,
                onpointerup: h.end_pointer, onpointercancel: h.cancel_pointer, onlostpointercapture: h.cancel_pointer,
                onkeydown: h.keyboard, onkeyup: h.key_up, onwheel: h.wheel,
                g { transform: "scale(1,-1)",
                    keymap::KeymapCanvas { view, contours: input.contours, selected_ids: input.selected_ids, on_select_key: input.on_select_key }
                }
            }
        }
    } else {
        rsx! { p { class: "my-2 text-base text-muted-foreground", role: "status", "Keymap is unavailable for the current board." } }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    let _ = (
        input.scope,
        input.layer_actions,
        input.active_layer_id,
        input.on_layer,
        input.on_select_key,
        input.admission_token,
        input.admission_revision,
        input.scope_generation,
    );
    let Some(view) = input.view else {
        return rsx! {
            section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto px-6 py-2 text-foreground bg-card", "aria-label": "Keymap",
                p { class: "my-2 text-base text-muted-foreground", role: "status", "Keymap is unavailable for the current board." }
            }
        };
    };
    let ba = input.binding_actions;
    let ma = input.macro_actions;
    let keys_editor = if let (Some(proj), Some(key_id)) =
        (ba.projection.as_ref(), input.selected_key_id.as_ref())
    {
        rsx! {
            keymap::BindingEditor {
                label: proj.key_label.clone(), active_layer_id: proj.effective_layer_id.clone(),
                target: keymap::BindingTarget::Key { key_id: key_id.clone() },
                value: proj.binding.clone(), layers: proj.layers.clone(), macros: proj.macros.clone(),
                enabled: ba.enabled, on_commit: ba.on_commit,
            }
        }
    } else {
        rsx! {}
    };
    let encoders_editor = if let Some(proj) = ba.encoder_projection.as_ref() {
        rsx! { keymap::EncoderEditor { projection: proj.clone(), enabled: ba.enabled, on_commit: ba.on_commit } }
    } else {
        rsx! {}
    };
    let macros_editor = rsx! {
        keymap::MacroEditor { macros: ma.macros, enabled: ma.enabled, error: ma.error, on_action: ma.on_action }
    };
    rsx! {
        keymap::KeymapPanel {
            view, selected_key_id: input.selected_key_id, on_export: input.on_export,
            firmware_export_enabled: input.firmware_export_enabled, binding_error: ba.error,
            keys_editor, macros_editor, encoders_editor,
        }
    }
}
