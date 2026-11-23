//! Keymap-owned canvas and Inspector composition.
use super::workbench::editor_session::action_handlers;
use super::workspace_composition::CanvasEventHandlers;
use super::workspace_composition::{
    WorkspaceInteraction, WorkspaceOutputs, WorkspaceSurfaces, design_toolbar, publish_surfaces,
    use_frame,
};
use super::*;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Contour;
use boardstudio_web_ui_model::editor::EditorContext;
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

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
    pub(super) selected_key_id: Option<String>,
    pub(super) on_export: EventHandler<()>,
    pub(super) firmware_export_enabled: bool,
    pub(super) binding_actions: keymap::BindingActions,
    pub(super) macro_actions: keymap::MacroActions,
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

#[derive(Clone, Copy)]
pub(super) struct ActiveLayer(pub Signal<String>);

#[component]
pub(super) fn KeymapWorkspace() -> Element {
    let editor = use_context::<EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let frame = use_frame();
    let interaction = use_context::<WorkspaceInteraction>();
    let outputs = use_context::<WorkspaceOutputs>();
    let ActiveLayer(layer_id) = use_context::<ActiveLayer>();
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let generation = editor.scope_generation;
    let instance_selection = editor.instance_selection;
    let current_instance = Rc::new({
        let runtime = runtime.clone();
        move || instance_selection.is_current(&runtime.model())
    });
    let model = runtime.model();
    let current_scope = frame.owner.scope.clone();
    let fit = keycaps_workspace::use_clearance_assessment(editor, &model, current_scope.as_ref());
    let layer_source =
        current_scope
            .clone()
            .zip(model.accepted.as_ref())
            .map(|(scope, snapshot)| keymap::LayerSource {
                scope,
                token: snapshot.token,
                revision: snapshot.document.revision,
            });
    let layer_actions = keymap::use_layer_operations(
        runtime.clone(),
        layer_source.clone(),
        layer_id,
        workspace,
        generation,
        current_instance.clone(),
    );
    let projection = use_projection(
        runtime.clone(),
        frame.owner.token,
        current_scope,
        model.active_board_id.clone(),
        layer_id(),
    );
    let (view, contours) = projection.map_or_else(
        || (None, Rc::from([])),
        |(view, contours)| (Some(view), contours),
    );
    let wiring = pcb_wiring::use_pcb_wiring_controller(runtime.clone(), editor.version);
    let encoders = keymap::use_encoder_inputs(
        runtime.clone(),
        layer_source.clone(),
        wiring.resolution_signal,
    );
    let binding_actions = keymap::use_binding_operations(
        runtime.clone(),
        keymap::BindingProjectionSources {
            source: layer_source.clone(),
            view: view.clone(),
            encoder_projection: encoders.projection,
            current_encoder_projection: encoders.current,
        },
        layer_id,
        workspace,
        generation,
        current_instance.clone(),
    );
    let macro_actions = keymap::use_macro_operations(
        runtime.clone(),
        layer_source,
        workspace,
        generation,
        current_instance,
    );
    let render_generation = frame.owner.generation;
    let scope = &frame.scope;
    let snapshot = &frame.snapshot;
    let on_select_key = on_select_key(editor, adapter.clone(), &frame, layer_id);
    let on_layer = action_handlers::on_keymap_layer_handler(
        runtime.clone(),
        adapter.clone(),
        scope.clone(),
        render_generation,
        workspace,
        layer_id,
    );
    let on_export =
        action_handlers::on_keymap_export_handler(action_handlers::OnKeymapExportInput {
            runtime,
            adapter: adapter.clone(),
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: render_generation,
            workspace,
            instance_selection,
        });
    let selected_ids = model.selected_part_ids.iter().cloned().collect();
    let selected_key_id = model.selected_part_ids.first().cloned();
    let (view_box, handlers, status) = frame.use_design_canvas(
        editor,
        &adapter,
        &model,
        view.as_deref()
            .and_then(|view| keymap_bounds(view, &contours)),
        view.as_deref()
            .and_then(|view| keymap::selected_bounds(view, &selected_ids)),
    );
    let toolbar = design_toolbar(
        &frame,
        editor,
        &adapter,
        "Layers & key behaviors",
        rsx! {
            workspace_navigation::InspectorActions {
                label: "Keymap tools", workspace, inspect_open: editor.panels.inspector_open,
                inspector_settings: interaction.inspector_settings, generation: adapter.generation,
                actions: vec![("Assign", "[aria-label='Keymap editors'] [aria-label='Keys']", true), ("Macros", "[aria-label='Keymap editors'] [aria-label='Macros']", true), ("Encoders", "[aria-label='Keymap editors'] [aria-label='Encoders']", true)],
                disabled: false,
            }
        },
    );
    let firmware_export_enabled = view.as_ref().is_some_and(|view| !view.keys.is_empty())
        || binding_actions
            .encoder_projection
            .as_ref()
            .is_some_and(|projection| !projection.rows.is_empty());
    let surfaces = WorkspaceSurfaces {
        objects: objects(NavigationInput {
            view: view.clone(),
            scope: scope.clone(),
            active_layer_id: layer_id(),
            selected_key_id: selected_key_id.clone(),
            layer_actions,
            on_layer,
            on_select_key,
        }),
        inspector: inspector(InspectorInput {
            view: view.clone(),
            selected_key_id,
            on_export,
            firmware_export_enabled,
            binding_actions,
            macro_actions,
        }),
        toolbar,
        canvas: frame.design_canvas(
            fit.state,
            canvas(CanvasInput {
                view,
                contours,
                view_box,
                selected_ids,
                handlers,
                on_select_key,
            }),
        ),
        status,
        layers: rsx! {},
    };
    publish_surfaces(outputs, Workspace::Keymap, surfaces);
    rsx! {}
}

fn on_select_key(
    editor: EditorContext,
    adapter: SelectionAdapter,
    frame: &workspace_composition::WorkspaceFrame,
    keymap_layer_id: Signal<String>,
) -> EventHandler<String> {
    let runtime = editor.runtime();
    let workspace = editor.workspace;
    let scope = frame.scope.clone();
    let generation = frame.owner.generation;
    let CompactPanelState {
        mut objects_open,
        inspector_open: mut inspect_open,
    } = editor.panels;
    EventHandler::new(move |key_id: String| {
        if workspace() != Workspace::Keymap
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) {
            return;
        }
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        if key_id.is_empty() {
            let mut selected_context = adapter.selected_context;
            selected_context.set(None);
            let mut anchor_scope = adapter.anchor_scope;
            anchor_scope.set(None);
            runtime.submit(boardstudio_application::Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            return;
        }
        let layer_id = keymap_layer_id();
        let Some(view) = keymap::project(snapshot, Some(&scope), &model.active_board_id, &layer_id)
        else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        let Some(context) = objects::context_for_part(&model, &key_id) else {
            return;
        };
        if !selection::context_is_current(&model, &scope, &context) {
            return;
        }
        let Some(selected_ids) = selection::submit_canvas_selection(
            &runtime,
            &adapter,
            &scope,
            generation,
            context.clone(),
            SelectionMode::Replace,
            Vec::new(),
        ) else {
            return;
        };
        if selected_ids.is_empty()
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let current = runtime.model();
        if !active_board_scope_matches(&current, &scope)
            || !selection::context_is_current(&current, &scope, &context)
        {
            return;
        }
        let Some(snapshot) = current.accepted.as_ref() else {
            return;
        };
        let layer_id = keymap_layer_id();
        let Some(view) =
            keymap::project(snapshot, Some(&scope), &current.active_board_id, &layer_id)
        else {
            return;
        };
        if !view.keys.iter().any(|key| key.id.as_ref() == key_id) {
            return;
        }
        objects_open.set(false);
        inspect_open.set(true);
    })
}
