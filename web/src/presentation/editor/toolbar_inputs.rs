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
    pub model: &'a ReadModel,
    pub layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    pub layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub layout_assembly_3d: Signal<bool>,
    pub on_layout_view_mode: EventHandler<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub retry_save: EventHandler<()>,
    pub download_pending: EventHandler<()>,
    pub recover_saved: EventHandler<()>,
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
        model,
        layout_snap_settings,
        layout_command_menu,
        layout_assembly_3d,
        on_layout_view_mode,
        toggle_footprints,
        retry_save,
        download_pending,
        recover_saved,
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
        || matrix_transform_inspector.projection.is_some());
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
            .projection
            .as_ref()
            .and_then(|projection| {
                match &projection.fields {
                boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields::Matrix {
                    rotation_followed,
                    ..
                } => Some(*rotation_followed),
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
    workspace_composition::WorkspaceToolbarInput::Layout(Box::new(layout_workspace::ToolbarInput {
        save_failure: match &model.durability {
            Durability::Failed { reason, .. } => Some(reason.clone()),
            _ => None,
        },
        recovery_required: model.lifecycle == boardstudio_application::Lifecycle::RecoveryRequired,
        footprints_pressed,
        on_toggle_footprints: toggle_footprints,
        assembly_3d: layout_assembly_3d(),
        on_view_mode: on_layout_view_mode,
        on_retry_save: retry_save,
        save_conflict: runtime.has_save_conflict(),
        on_download_pending: download_pending,
        on_recover_saved: recover_saved,
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
        has_selection_context: matrix_transform_inspector.projection.is_some()
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
    pub retry_save: EventHandler<()>,
    pub download_pending: EventHandler<()>,
    pub recover_saved: EventHandler<()>,
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
        retry_save,
        download_pending,
        recover_saved,
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
            model,
            layout_snap_settings,
            layout_command_menu,
            layout_assembly_3d,
            on_layout_view_mode,
            toggle_footprints,
            retry_save,
            download_pending,
            recover_saved,
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
