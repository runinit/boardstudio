use super::super::*;

pub(super) fn use_toolbar_selection_availability(
    tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    layout_owner: LayoutOwnerIdentity,
    accepted_canvas_source: layout_canvas::SceneSource,
    snapshot: &AcceptedSnapshot,
    model: &ReadModel,
    selected_tree_context: Option<objects::ScopedTreeContext>,
    active_workspace: &'static str,
) -> (bool, bool) {
    let retained_toolbar_cell = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
    use_memo(use_reactive(
        (
            &accepted_canvas_source,
            &snapshot.token,
            &snapshot.session_epoch,
            &model.active_instance_id,
            &selected_tree_context,
            &retained_toolbar_cell,
            &active_workspace,
        ),
        |(source, token, session_epoch, active_instance_id, selected, retained, workspace)| {
            if !matches!(workspace, "Layout" | "PCB") {
                return (false, false);
            }
            let model = ReadModel {
                accepted: Some(AcceptedSnapshot {
                    token,
                    session_epoch,
                    document: source.document,
                    scene: source.scene,
                }),
                active_board_id: source.board_id,
                active_instance_id,
                ..ReadModel::default()
            };
            let supports = |kind| {
                selected.as_ref().is_some_and(|selected| {
                    objects::context_for_selection_kind(
                        &model,
                        &selected.context,
                        kind,
                        retained.as_ref(),
                    )
                    .is_some()
                })
            };
            (
                supports(objects::LayoutSelectionKind::Column),
                supports(objects::LayoutSelectionKind::Row),
            )
        },
    ))()
}

pub(super) struct LayoutToolbarArgs<'a> {
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub show_position_inspector: bool,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<&'static str>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub toolbar_selection_availability: (bool, bool),
    pub layer_visibility: LayerVisibility,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_assembly_3d: Signal<bool>,
    pub on_layout_view_mode: EventHandler<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub layout_transform_target: Option<(
        std::rc::Rc<boardstudio_core::model::Matrix>,
        std::rc::Rc<boardstudio_core::model::MatrixScene>,
        objects::TreeContext,
    )>,
    pub layout_align: objects::LayoutAlignMount,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) fn layout_toolbar_input(
    input: LayoutToolbarArgs<'_>,
) -> workspace_composition::WorkspaceToolbarInput {
    let LayoutToolbarArgs {
        selected_tree_context,
        show_position_inspector,
        matrix_transform_inspector,
        runtime,
        adapter,
        workspace,
        mut objects_open,
        mut inspect_open,
        inspector_panel_settings,
        layout_owner,
        layout_transform_tool,
        on_selection_kind,
        matrices,
        toolbar_selection_availability,
        layer_visibility,
        layout_snap_settings,
        layout_command_menu,
        layout_assembly_3d,
        on_layout_view_mode,
        toggle_footprints,
        layout_selection_kind,
        component_inspector,
        layout_transform_target,
        layout_align,
        layout_snap_intent,
    } = input;
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
    let matrix_rotate_followed = matrix_transform_inspector
        .projection()
        .and_then(|projection| match projection.fields {
            boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields::Matrix {
                rotation_followed,
                ..
            } => Some(rotation_followed),
            _ => None,
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
    workspace_composition::WorkspaceToolbarInput::Layout(Box::new(layout_workspace::ToolbarInput {
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
    }))
}

pub(super) struct PcbToolbarArgs {
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub show_position_inspector: bool,
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<&'static str>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub layout_owner: LayoutOwnerIdentity,
    pub toolbar_selection_availability: (bool, bool),
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub pcb_align: objects::LayoutAlignMount,
}

pub(super) fn pcb_toolbar_input(
    input: PcbToolbarArgs,
) -> workspace_composition::WorkspaceToolbarInput {
    let PcbToolbarArgs {
        selected_tree_context,
        show_position_inspector,
        runtime,
        adapter,
        workspace,
        mut objects_open,
        mut inspect_open,
        inspector_panel_settings,
        layout_owner,
        toolbar_selection_availability,
        layout_snap_settings,
        layout_command_menu,
        layout_selection_kind,
        pcb_selection_kind,
        pcb_snap_intent,
        pcb_align,
    } = input;
    let properties_available = selected_tree_context.as_ref().is_some_and(|selected| {
        matches!(
            &selected.context,
            objects::TreeContext::Matrix { .. }
                | objects::TreeContext::Row { .. }
                | objects::TreeContext::Column { .. }
                | objects::TreeContext::Key { .. }
                | objects::TreeContext::Component {
                    part_id: Some(_),
                    ..
                }
        )
    }) && show_position_inspector;
    let on_show_properties = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        move |_| {
            if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) || !properties_available
            {
                return;
            }
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let transform = objects::LayoutTransformMenuMount {
        properties_available,
        column_available: toolbar_selection_availability.0,
        row_available: toolbar_selection_availability.1,
        pointer_tools_visible: false,
        pointer_tools_available: false,
        active_tool: None,
        matrix_rotate_followed: None,
        on_selection_kind: pcb_selection_kind,
        on_show_properties: EventHandler::new(on_show_properties),
        on_transform_tool: EventHandler::new(|_| {}),
    };
    workspace_composition::WorkspaceToolbarInput::Pcb(Box::new(pcb_workspace::ToolbarInput {
        command_label: "PCB commands".to_owned(),
        menu_owner_key: format!("{layout_owner:?}"),
        open_menu: layout_command_menu,
        selection_kind: layout_selection_kind(),
        snap_settings: layout_snap_settings.read().clone(),
        transform,
        align: pcb_align.clone(),
        on_selection_kind: pcb_selection_kind,
        on_snap_intent: pcb_snap_intent,
        show_relationships: false,
        has_selection_context: false,
        on_show_relationships: EventHandler::new(|()| {}),
    }))
}

pub(super) struct KeymapToolbarArgs {
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<&'static str>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_assembly_3d: Signal<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub layer_visibility: LayerVisibility,
}

pub(super) fn keymap_toolbar_input(
    input: KeymapToolbarArgs,
) -> workspace_composition::WorkspaceToolbarInput {
    let KeymapToolbarArgs {
        runtime,
        workspace,
        adapter,
        layout_owner,
        layout_assembly_3d,
        toggle_footprints,
        inspect_open,
        inspector_panel_settings,
        layer_visibility,
    } = input;
    let on_view_mode = design_consumer_view_mode_handler(
        runtime.clone(),
        workspace,
        adapter.clone(),
        layout_owner.clone(),
        layout_assembly_3d,
    );
    workspace_composition::WorkspaceToolbarInput::Keymap(shared_viewer::DesignViewToolbarProps {
        contextual_tools: Some(rsx! { workspace_navigation::InspectorActions {
            label: "Keymap tools", workspace, inspect_open, inspector_settings: inspector_panel_settings,
            generation: adapter.generation, actions: vec![("Assign", "[aria-label='Keymap editors'] [aria-label='Keys']", true), ("Macros", "[aria-label='Keymap editors'] [aria-label='Macros']", true), ("Encoders", "[aria-label='Keymap editors'] [aria-label='Encoders']", true)],
            disabled: false,
        } }),
        label: "Keymap".into(),
        detail: "Layers & key behaviors".into(),
        assembly_3d: layout_assembly_3d(),
        footprints_visible: (layer_visibility.footprints)()
            && !(layer_visibility.hidden)().contains("Footprints"),
        on_view_mode,
        on_toggle_footprints: toggle_footprints,
    })
}

pub(super) struct KeycapsToolbarArgs<'a> {
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<&'static str>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_assembly_3d: Signal<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub layer_visibility: LayerVisibility,
    pub model: &'a ReadModel,
}

pub(super) fn keycaps_toolbar_input(
    input: KeycapsToolbarArgs<'_>,
) -> workspace_composition::WorkspaceToolbarInput {
    let KeycapsToolbarArgs {
        runtime,
        workspace,
        adapter,
        layout_owner,
        layout_assembly_3d,
        toggle_footprints,
        inspect_open,
        inspector_panel_settings,
        layer_visibility,
        model,
    } = input;
    let on_view_mode = design_consumer_view_mode_handler(
        runtime.clone(),
        workspace,
        adapter.clone(),
        layout_owner.clone(),
        layout_assembly_3d,
    );
    workspace_composition::WorkspaceToolbarInput::Keycaps(shared_viewer::DesignViewToolbarProps {
        contextual_tools: Some(rsx! { workspace_navigation::InspectorActions {
            label: "Keycaps tools", workspace, inspect_open, inspector_settings: inspector_panel_settings,
            generation: adapter.generation, actions: vec![("Profile", "[aria-label^='Keycap overrides for '] [data-slot='select-trigger'][aria-label^='Profile override for ']", false), ("Legend", "[aria-label^='Keycap overrides for '] [aria-label^='Legend for ']", false), ("Color", "[aria-label^='Keycap overrides for '] [aria-label^='Keycap color for ']", false)],
            disabled: model.selected_part_ids.is_empty(),
        } }),
        label: "Keycaps".into(),
        detail: "Profiles, legends & fit".into(),
        assembly_3d: layout_assembly_3d(),
        footprints_visible: (layer_visibility.footprints)()
            && !(layer_visibility.hidden)().contains("Footprints"),
        on_view_mode,
        on_toggle_footprints: toggle_footprints,
    })
}

pub(super) struct ToolbarInputArgs<'a> {
    pub active_workspace: &'static str,
    pub toolbar_selection_availability: (bool, bool),
    pub selected_tree_context: Option<objects::ScopedTreeContext>,
    pub show_position_inspector: bool,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<&'static str>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    pub on_layout_selection_kind_handler: EventHandler<objects::LayoutSelectionKind>,
    pub matrices: Vec<&'a boardstudio_core::model::Matrix>,
    pub layer_visibility: LayerVisibility,
    pub model: &'a ReadModel,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_assembly_3d: Signal<bool>,
    pub on_layout_view_mode: EventHandler<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub layout_selection_kind: Signal<objects::LayoutSelectionKind>,
    pub component_inspector: Option<inspector::LayoutComponentInspectorProjection>,
    pub pcb_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub pcb_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub layout_transform_target: Option<(
        std::rc::Rc<boardstudio_core::model::Matrix>,
        std::rc::Rc<boardstudio_core::model::MatrixScene>,
        objects::TreeContext,
    )>,
    pub layout_align: objects::LayoutAlignMount,
    pub layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub pcb_align: objects::LayoutAlignMount,
}

pub(super) fn toolbar_input(
    input: ToolbarInputArgs<'_>,
) -> workspace_composition::WorkspaceToolbarInput {
    let ToolbarInputArgs {
        active_workspace,
        toolbar_selection_availability,
        selected_tree_context,
        show_position_inspector,
        matrix_transform_inspector,
        runtime,
        adapter,
        workspace,
        objects_open,
        inspect_open,
        inspector_panel_settings,
        layout_owner,
        layout_transform_tool,
        on_layout_selection_kind_handler,
        matrices,
        layer_visibility,
        model,
        layout_snap_settings,
        layout_command_menu,
        layout_assembly_3d,
        on_layout_view_mode,
        toggle_footprints,
        layout_selection_kind,
        component_inspector,
        pcb_selection_kind,
        pcb_snap_intent,
        layout_transform_target,
        layout_align,
        layout_snap_intent,
        pcb_align,
    } = input;
    match active_workspace {
        "Layout" => layout_toolbar_input(LayoutToolbarArgs {
            selected_tree_context,
            show_position_inspector,
            matrix_transform_inspector,
            runtime,
            adapter,
            workspace,
            objects_open,
            inspect_open,
            inspector_panel_settings,
            layout_owner,
            layout_transform_tool,
            on_selection_kind: on_layout_selection_kind_handler,
            matrices,
            toolbar_selection_availability,
            layer_visibility,
            layout_snap_settings,
            layout_command_menu,
            layout_assembly_3d,
            on_layout_view_mode,
            toggle_footprints,
            layout_selection_kind,
            component_inspector,
            layout_transform_target,
            layout_align,
            layout_snap_intent,
        }),
        "PCB" => pcb_toolbar_input(PcbToolbarArgs {
            selected_tree_context,
            show_position_inspector,
            runtime,
            adapter,
            workspace,
            objects_open,
            inspect_open,
            inspector_panel_settings,
            layout_owner,
            toolbar_selection_availability,
            layout_snap_settings,
            layout_command_menu,
            layout_selection_kind,
            pcb_selection_kind,
            pcb_snap_intent,
            pcb_align,
        }),
        "Keymap" => keymap_toolbar_input(KeymapToolbarArgs {
            runtime,
            workspace,
            adapter,
            layout_owner,
            layout_assembly_3d,
            toggle_footprints,
            inspect_open,
            inspector_panel_settings,
            layer_visibility,
        }),
        "Keycaps" => keycaps_toolbar_input(KeycapsToolbarArgs {
            runtime,
            workspace,
            adapter,
            layout_owner,
            layout_assembly_3d,
            toggle_footprints,
            inspect_open,
            inspector_panel_settings,
            layer_visibility,
            model,
        }),
        "Case" => workspace_composition::WorkspaceToolbarInput::Case,
        "Parts" => workspace_composition::WorkspaceToolbarInput::Parts,
        "Export" => workspace_composition::WorkspaceToolbarInput::Export,
        _ => workspace_composition::WorkspaceToolbarInput::Other,
    }
}

pub(super) struct WsInputs {
    pub on_layout_view_mode: EventHandler<bool>,
    pub toolbar_input: workspace_composition::WorkspaceToolbarInput,
    pub canvas_input: Option<workspace_composition::WorkspaceCanvasInput>,
    pub on_component_inspector_action: EventHandler<inspector::LayoutComponentInspectorAction>,
    pub on_companion_placement_action: EventHandler<inspector::CompanionPlacementAction>,
}

pub(super) struct WsInputsInput<'a> {
    pub setup_a: &'a super::setup_bundles::SetupA,
    pub setup_b: &'a super::setup_bundles::SetupB,
    pub cam: &'a super::camera::CamNamed,
    pub canvas: &'a super::canvas_state::CanvasNamed,
    pub wir: &'a super::workspace_inputs::WsWiring,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: &'a Scope,
    pub matrices: &'a Vec<&'a boardstudio_core::model::Matrix>,
    pub matrix_setup: &'a objects::MatrixSetupMount,
    pub instance_scope_pending: bool,
}

pub(super) fn use_workspace_inputs(
    input: WsInputsInput<'_>,
    start_pan: impl FnMut(PointerEvent) + Clone + 'static,
    move_pointer: impl FnMut(PointerEvent) + Clone + 'static,
    key_up: impl FnMut(KeyboardEvent) + Clone + 'static,
) -> WsInputs {
    let WsInputsInput {
        setup_a,
        setup_b,
        cam,
        canvas,
        wir,
        snapshot,
        render_scope,
        matrices,
        matrix_setup,
        instance_scope_pending,
    } = input;
    let runtime = &setup_a.runtime;
    let adapter = &setup_a.adapter;
    let layout_owner = &setup_a.layout_owner;
    let workspace = setup_a.workspace;
    let layout_assembly_3d = setup_a.layout_assembly_3d;
    let layout_state = setup_a.layout_state;
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let interaction_version = cam.placement.interaction_version;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_placement;
    let canvas_interaction = &setup_a.canvas_interaction;
    let mirrored_pair = &setup_a.mirrored_pair;
    let tree_cell_anchor = &setup_a.tree_cell_anchor;
    let accepted_canvas_source = &cam.scene_bounds.accepted_canvas_source;
    let model = &setup_a.model;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let active_workspace = setup_a.active_workspace;
    let show_position_inspector = canvas.inspector.show_position_inspector;
    let matrix_transform_inspector = &setup_a.matrix_transform_inspector;
    let objects_open = setup_a.objects_open;
    let inspect_open = setup_a.inspect_open;
    let inspector_panel_settings = setup_a.inspector_panel_settings;
    let layout_transform_tool = setup_a.layout_transform_tool;
    let on_layout_selection_kind_handler = wir.wiring.on_layout_selection_kind_handler;
    let layer_visibility = setup_a.layer_visibility;
    let layout_snap_settings = setup_a.layout_snap_settings;
    let layout_command_menu = setup_a.layout_command_menu;
    let toggle_footprints = wir.wiring.toggle_footprints;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let component_inspector = &canvas.inspector.component_inspector;
    let pcb_selection_kind = wir.wiring.pcb_selection_kind;
    let pcb_snap_intent = wir.wiring.pcb_snap_intent;
    let layout_transform_target = &canvas.selection.layout_transform_target;
    let layout_align = &setup_a.layout_align;
    let layout_snap_intent = wir.wiring.layout_snap_intent;
    let pcb_align = &setup_a.pcb_align;
    let mount = canvas.inspector.mount;
    let canvas_end_pointer = wir.wiring.canvas_end_pointer;
    let canvas_cancel_pointer = wir.wiring.canvas_cancel_pointer;
    let canvas_keyboard = wir.wiring.canvas_keyboard;
    let canvas_wheel = wir.wiring.canvas_wheel;
    let view_box = &cam.scene_bounds.view_box;
    let render_generation = setup_a.render_generation;
    let keymap_view = &setup_b.keymap_view;
    let keymap_contours = &setup_b.keymap_contours;
    let keycaps_view = &setup_b.keycaps_view;
    let keycaps_contours = &setup_b.keycaps_contours;
    let keymap_select = wir.wiring.keymap_select;
    let keycaps_select = wir.wiring.keycaps_select;
    let mechanical_settings = &setup_b.mechanical_settings;
    let current_scope = &setup_a.current_scope;
    let parts_query = setup_a.parts_query;
    let parts_selection = setup_a.parts_selection;
    let pcb_empty_hit = wir.wiring.pcb_empty_hit;
    let pcb_part_hit = wir.wiring.pcb_part_hit;
    let pcb_part_pointer_down = wir.wiring.pcb_part_pointer_down;
    let mounted_module_select = wir.wiring.mounted_module_select;
    let layout_component_inspector_lifetime = &setup_a.layout_component_inspector_lifetime;
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
    let toolbar_selection_availability = use_toolbar_selection_availability(
        tree_cell_anchor.clone(),
        layout_owner.clone(),
        layout_canvas::SceneSource {
            document: snapshot.document.clone(),
            ..accepted_canvas_source.clone()
        },
        snapshot,
        model,
        selected_tree_context.clone(),
        active_workspace,
    );
    let toolbar_input = toolbar_input(ToolbarInputArgs {
        active_workspace,
        toolbar_selection_availability,
        selected_tree_context: selected_tree_context.clone(),
        show_position_inspector,
        matrix_transform_inspector: matrix_transform_inspector.clone(),
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        workspace,
        objects_open,
        inspect_open,
        inspector_panel_settings,
        layout_owner: layout_owner.clone(),
        layout_transform_tool,
        on_layout_selection_kind_handler,
        matrices: matrices.clone(),
        layer_visibility,
        model,
        layout_snap_settings,
        layout_command_menu,
        layout_assembly_3d,
        on_layout_view_mode,
        toggle_footprints,
        layout_selection_kind,
        component_inspector: component_inspector.clone(),
        pcb_selection_kind,
        pcb_snap_intent,
        layout_transform_target: layout_transform_target.clone(),
        layout_align: layout_align.clone(),
        layout_snap_intent,
        pcb_align: pcb_align.clone(),
    });
    let canvas_input = super::canvas_inspector_inputs::canvas_input(
        super::canvas_inspector_inputs::CanvasInputArgs {
            mount,
            end_pointer: canvas_end_pointer,
            cancel_pointer: canvas_cancel_pointer,
            keyboard: canvas_keyboard,
            wheel: canvas_wheel,
            snapshot,
            render_scope: render_scope.clone(),
            view_box: view_box.clone(),
            model,
            render_generation,
            keymap_view: keymap_view.clone(),
            keymap_contours: keymap_contours.clone(),
            keycaps_view: keycaps_view.clone(),
            keycaps_contours: keycaps_contours.clone(),
            keymap_select,
            keycaps_select,
            mechanical_settings: mechanical_settings.clone(),
            instance_scope_pending,
            part_placement: part_placement.clone(),
            current_scope: current_scope.clone(),
            parts_query,
            parts_selection,
            workspace,
            active_workspace,
            pcb_empty_hit,
            pcb_part_hit,
            pcb_part_pointer_down,
            mounted_module_select,
        },
        start_pan.clone(),
        move_pointer.clone(),
        key_up.clone(),
    );
    let on_component_inspector_action =
        super::canvas_inspector_inputs::on_component_inspector_action_handler(
            runtime.clone(),
            adapter.clone(),
            layout_owner.clone(),
            layout_component_inspector_lifetime.clone(),
            workspace,
            inspect_open,
            layout_state,
        );
    let on_companion_placement_action =
        super::canvas_inspector_inputs::on_companion_placement_action_handler(
            runtime.clone(),
            adapter.clone(),
            workspace,
            layout_state,
        );
    WsInputs {
        on_layout_view_mode,
        toolbar_input,
        canvas_input,
        on_component_inspector_action,
        on_companion_placement_action,
    }
}

pub(super) struct OnLayoutViewModeArgs {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub workspace: Signal<&'static str>,
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

pub(super) fn on_layout_view_mode(input: OnLayoutViewModeArgs) -> EventHandler<bool> {
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
