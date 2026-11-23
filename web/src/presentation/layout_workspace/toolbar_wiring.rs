use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct WsInputs {
    pub on_layout_view_mode: EventHandler<bool>,
    pub toolbar_input: layout_workspace::ToolbarInput,
    pub on_component_inspector_action: EventHandler<inspector::LayoutComponentInspectorAction>,
    pub on_companion_placement_action: EventHandler<inspector::CompanionPlacementAction>,
}

pub(in crate::presentation) fn use_layout_toolbar(
    state: &super::super::workbench::editor_session::session_state::SessionState,
    cam: &super::super::workbench::editor_session::camera::CamNamed,
    canvas: &super::inspector_wiring::CanvasNamed,
    wir: &super::super::workbench::editor_session::workspace_inputs::WsWiring,
    matrices: Vec<&boardstudio_core::model::Matrix>,
    matrix_setup: &objects::MatrixSetupMount,
) -> WsInputs {
    let snapshot = state.snapshot;
    let render_scope = state.render_scope;
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let adapter = &state.adapter;
    let layout_owner = &state.view.layout_owner;
    let workspace = editor.workspace;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let layout_state = state.layout_state;
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let interaction_version = cam.placement.interaction_version;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let canvas_interaction = &state.canvas_interaction;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let tree_cell_anchor = &state.tree_cell_anchor;
    let accepted_canvas_source = &cam.scene_bounds.accepted_canvas_source;
    let model = &state.view.model;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let active_workspace = state.active_workspace;
    let show_position_inspector = canvas.inspector.show_position_inspector;
    let matrix_transform_inspector = &state.matrix_transform_inspector;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let inspector_panel_settings = state.inspector_panel_settings;
    let layout_transform_tool = state.layout_state.transform_tool();
    let on_layout_selection_kind_handler = wir.on_layout_selection_kind_handler;
    let layer_visibility = editor.layers;
    let layout_snap_settings = state.layout_state.snap_settings();
    let layout_command_menu = state.layout_state.command_menu();
    let toggle_footprints = wir.toggle_footprints;
    let layout_selection_kind = state.layout_state.selection_kind();
    let component_inspector = &canvas.inspector.component_inspector;
    let layout_transform_target = &canvas.selection.layout_transform_target;
    let layout_align = &state.layout_align;
    let layout_snap_intent = wir.layout_snap_intent;
    let layout_component_inspector_lifetime = &state.layout_component_inspector_lifetime;
    let on_layout_view_mode = on_layout_view_mode(OnLayoutViewModeArgs {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        layout_owner: layout_owner.clone(),
        workspace,
        layout_assembly_3d,
        layout_state,
        drag: drag.clone(),
        svg: svg.clone(),
        render_scope: render_scope.clone(),
        interaction_version,
        part_placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        canvas_interaction: canvas_interaction.clone(),
        mirrored_pair: mirrored_pair.clone(),
        matrix_setup: matrix_setup.clone(),
    });
    let toolbar_selection_availability =
        super::super::workspace_composition::use_toolbar_selection_availability(
            tree_cell_anchor.clone(),
            layout_owner.clone(),
            layout_canvas::SceneSource {
                document: snapshot.document.clone(),
                ..accepted_canvas_source.clone()
            },
            snapshot,
            model,
            selected_tree_context.clone(),
            (active_workspace == Workspace::Layout).then_some(active_workspace),
        );
    let on_selection_kind = on_layout_selection_kind_handler;
    let mut objects_open = objects_open;
    let mut inspect_open = inspect_open;
    let toolbar_input = {
        let properties_available = selected_tree_context.as_ref().is_some_and(|selected| {
            matches!(
                &selected.context,
                objects::TreeContext::Matrix { .. }
                    | objects::TreeContext::Row { .. }
                    | objects::TreeContext::Column { .. }
                    | objects::TreeContext::Key { .. }
                    | objects::TreeContext::Component { .. }
            )
        }) && (show_position_inspector
            || matrix_transform_inspector.is_active());
        let on_show_properties = EventHandler::new({
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let owner = layout_owner.clone();
            move |_| {
                if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                    return;
                }
                pin_inspector_on_desktop(inspector_panel_settings);
                objects_open.set(false);
                inspect_open.set(true);
            }
        });
        let pointer_tools_available = layout_transform_target.is_some();
        let on_transform_tool = EventHandler::new({
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let owner = layout_owner.clone();
            let mut active_tool = layout_transform_tool;
            let matrix_ids: BTreeSet<_> = matrices.iter().map(|matrix| matrix.id.clone()).collect();
            move |tool| {
                if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                    return;
                }
                let Some(selected) = adapter.selected_context.read().clone().filter(|selected| {
                    owner.scope.as_ref() == Some(&selected.scope)
                        && selection::context_is_current(
                            &runtime.model(),
                            &selected.scope,
                            &selected.context,
                        )
                }) else {
                    return;
                };
                let matrix_id = match &selected.context {
                    objects::TreeContext::Matrix { matrix_id }
                    | objects::TreeContext::Row { matrix_id, .. }
                    | objects::TreeContext::Column { matrix_id, .. }
                    | objects::TreeContext::Key { matrix_id, .. } => matrix_id.as_str(),
                    objects::TreeContext::Component {
                        matrix_id: Some(matrix_id),
                        ..
                    } => matrix_id.as_str(),
                    _ => return,
                };
                if !matrix_ids.contains(matrix_id) {
                    active_tool.set(None);
                    return;
                }
                let activation =
                    objects::activate_transform_tool(active_tool(), tool, &selected.context);
                active_tool.set(activation.active);
                if let Some(kind) = activation.selection_kind {
                    on_selection_kind.call(kind);
                }
            }
        });
        let matrix_rotate_followed =
            matrix_transform_inspector
                .projection()
                .and_then(|projection| {
                    match projection.fields {
                boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields::Matrix {
                    rotation_followed,
                    ..
                } => Some(rotation_followed),
                _ => None,
            }
                });
        let transform = objects::LayoutTransformMenuMount {
            properties_available,
            column_available: toolbar_selection_availability.0,
            row_available: toolbar_selection_availability.1,
            pointer_tools_visible: true,
            pointer_tools_available,
            active_tool: layout_transform_tool(),
            matrix_rotate_followed,
            on_selection_kind,
            on_show_properties,
            on_transform_tool,
        };
        let footprints_pressed =
            (layer_visibility.footprints)() && !(layer_visibility.hidden)().contains("Footprints");
        layout_workspace::ToolbarInput {
            footprints_pressed,
            on_toggle_footprints: toggle_footprints,
            assembly_3d: layout_assembly_3d(),
            on_view_mode: on_layout_view_mode,
            selection_kind: layout_selection_kind(),
            snap_settings: layout_snap_settings.read().clone(),
            command_label: "Layout commands".to_owned(),
            align: layout_align.clone(),
            transform,
            menu_owner_key: format!("{layout_owner:?}"),
            open_menu: layout_command_menu,
            on_selection_kind,
            on_snap_intent: layout_snap_intent,
            show_relationships: true,
            has_selection_context: matrix_transform_inspector.is_active()
                || component_inspector.is_some(),
            on_show_relationships: EventHandler::new(move |()| {
                inspect_open.set(true);
            }),
        }
    };
    let on_component_inspector_action =
        super::inspector_wiring::on_component_inspector_action_handler(
            runtime.clone(),
            adapter.clone(),
            layout_owner.clone(),
            layout_component_inspector_lifetime.clone(),
            workspace,
            inspect_open,
            layout_state,
        );
    let on_companion_placement_action =
        super::inspector_wiring::on_companion_placement_action_handler(
            runtime.clone(),
            adapter.clone(),
            workspace,
            layout_state,
        );
    WsInputs {
        on_layout_view_mode,
        toolbar_input,
        on_component_inspector_action,
        on_companion_placement_action,
    }
}

pub(in crate::presentation) struct OnLayoutViewModeArgs {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub workspace: Signal<Workspace>,
    pub layout_assembly_3d: Signal<bool>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub drag: Rc<RefCell<Option<Drag>>>,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub render_scope: Scope,
    pub interaction_version: Signal<u64>,
    pub part_placement: part_placement::PartPlacementMount,
    pub matrix_placement: objects::MatrixPlacementMount,
    pub canvas_interaction: CanvasInteractionArbiter,
    pub mirrored_pair: objects::MirroredPairMount,
    pub matrix_setup: objects::MatrixSetupMount,
}

pub(in crate::presentation) fn on_layout_view_mode(
    input: OnLayoutViewModeArgs,
) -> EventHandler<bool> {
    let OnLayoutViewModeArgs {
        runtime,
        adapter,
        layout_owner,
        workspace,
        layout_assembly_3d,
        layout_state,
        drag,
        svg,
        render_scope,
        interaction_version,
        part_placement,
        matrix_placement,
        canvas_interaction,
        mirrored_pair,
        matrix_setup,
    } = input;
    layout_view_mode_handler(
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let owner = layout_owner.clone();
            move || layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        },
        layout_assembly_3d,
        {
            move |assembly_3d| {
                layout_state.set_assembly_3d(assembly_3d);
            }
        },
        LayoutPlacementCancellation {
            parts: part_placement.clone(),
            matrices: matrix_placement.clone(),
            interactions: canvas_interaction.clone(),
        },
        {
            let runtime = runtime.clone();
            let drag = drag.clone();
            let svg = svg.clone();
            let render_scope = render_scope.clone();
            move || {
                selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&render_scope));
                let mut interaction_version = interaction_version;
                interaction_version += 1;
            }
        },
        {
            let mirrored_pair = mirrored_pair.clone();
            let matrix_setup = matrix_setup.clone();
            let matrix_placement = matrix_placement.clone();
            move || {
                if let Some(active) = mirrored_pair.placement.as_ref() {
                    mirrored_pair.on_cancel.call(active.owner.clone());
                } else if let Some(form) = mirrored_pair.form.as_ref() {
                    mirrored_pair.on_cancel.call(form.owner.clone());
                }
                if let Some(projection) = matrix_setup.projection.as_ref() {
                    matrix_setup.on_cancel.call(projection.owner.clone());
                }
                if let Some(owner) = matrix_placement.cancel_owner.clone() {
                    matrix_placement.on_cancel.call(owner);
                }
            }
        },
    )
}
