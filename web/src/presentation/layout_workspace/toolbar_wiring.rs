use super::super::editor::selection_handlers;
use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct WsInputs {
    pub on_layout_view_mode: EventHandler<bool>,
    pub toolbar_input: layout_workspace::ToolbarInput,
    pub on_component_inspector_action: EventHandler<inspector::LayoutComponentInspectorAction>,
    pub on_companion_placement_action: EventHandler<inspector::CompanionPlacementAction>,
}

pub(in crate::presentation) fn use_layout_toolbar(
    frame: &super::super::workspace_composition::WorkspaceFrame,
    canvas: &super::inspector_wiring::CanvasNamed,
    mounts: &super::LayoutMounts,
    layout_component_inspector_lifetime: Rc<inspector::LayoutComponentInspectorLifetime>,
) -> WsInputs {
    let services = frame.layout_services();
    let snapshot = &frame.snapshot;
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let interaction = use_context::<super::super::workspace_composition::WorkspaceInteraction>();
    let layout_state = use_context::<layout_workspace::LayoutWorkspaceState>();
    let runtime = editor.runtime();
    let layout_owner = frame.owner.clone();
    let workspace = editor.workspace;
    let layout_assembly_3d = layout_state.assembly_3d();
    let tree_cell_anchor = interaction.tree_cell_anchor.clone();
    let accepted_canvas_source = frame.camera.accepted_canvas_source.clone();
    let model = &frame.model;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let active_workspace = Workspace::Layout;
    let show_position_inspector = canvas.inspector.show_position_inspector;
    let matrix_transform_inspector = services.matrix_transform_inspector.clone();
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let inspector_panel_settings = interaction.inspector_settings;
    let layout_transform_tool = layout_state.transform_tool();
    let layout_selection_kind = layout_state.selection_kind();
    let on_layout_selection_kind_handler = selection_handlers::on_selection_kind_handler(
        selection_handlers::SelectionKindInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            objects_open,
            inspect_open,
            selection_kind: layout_selection_kind,
            workspace,
            inspector_panel_settings,
        },
        Workspace::Layout,
    );
    let layer_visibility = editor.layers;
    let layout_snap_settings = layout_state.snap_settings();
    let layout_command_menu = layout_state.command_menu();
    let toggle_footprints = super::super::workspace_composition::toggle_footprints(editor.layers);
    let component_inspector = &canvas.inspector.component_inspector;
    let layout_transform_target = &canvas.selection.layout_transform_target;
    let layout_align = mounts.layout_align.clone();
    let layout_snap_intent = selection_handlers::on_snap_intent_handler(
        runtime.clone(),
        adapter.clone(),
        layout_owner.clone(),
        layout_snap_settings,
        workspace,
        Workspace::Layout,
    );
    let matrix_setup = frame.matrix_setup.clone();
    let board = frame
        .document
        .boards
        .iter()
        .find(|board| board.id == frame.model.active_board_id);
    let board_part_ids: HashSet<_> = board
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    let matrices = boardstudio_web_ui_model::tree::visible_matrices(
        &frame.document,
        &frame.model.active_board_id,
        &board_part_ids,
    );
    let on_layout_view_mode = {
        let input = use_context::<layout_input::LayoutInput>();
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let matrix_setup = matrix_setup.clone();
        EventHandler::new(move |assembly_3d| {
            if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            if assembly_3d && !layout_assembly_3d() {
                input.dispatch(CanvasEvent::ScopeChanged);
                if let Some(projection) = matrix_setup.projection.as_ref() {
                    matrix_setup.on_cancel.call(projection.owner.clone());
                }
            }
            layout_state.set_assembly_3d(assembly_3d);
        })
    };
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
