use super::*;
use boardstudio_web_ui_model::editor::EditorContext;
pub(in crate::presentation) mod action_handlers;
pub(in crate::presentation) mod camera;
mod canvas_adapter;
mod canvas_inspector_inputs;
mod canvas_pointer;
mod canvas_state;
pub(in crate::presentation) mod commands;
pub(in crate::presentation) mod findings_navigation;
pub(in crate::presentation) mod keyboard;
mod pointer_bundles;
pub(in crate::presentation) mod selection_handlers;
mod session_state;
mod setup_guide_state;
mod setup_workspaces;
pub(in crate::presentation) mod toolbar_inputs;
mod view;
mod workspace_inputs;

#[component]
pub(in crate::presentation) fn EditorSession(scope: Scope, snapshot: AcceptedSnapshot) -> Element {
    let editor = use_context::<EditorContext>();
    let render_scope = scope;
    let snapshot = &snapshot;
    let (state, workspace_routing, mut matrix_setup) =
        session_state::use_session_state(snapshot, &render_scope);
    let (physical_setup, mechanical_settings) = session_state::use_case_and_project_setup(&state);
    let outputs = use_signal(|| None);
    use_context_provider(|| workspace_composition::WorkspaceOutputs(outputs));
    use_context_provider(|| workspace_composition::WorkspaceInteraction {
        navigation: state.canvas_navigation.clone(),
        arbiter: state.canvas_interaction.clone(),
        tree_cell_anchor: state.tree_cell_anchor.clone(),
        inspector_settings: state.inspector_panel_settings,
    });
    let model = &state.view.model;
    let owner = &state.view.layout_owner;
    // A finding can switch workspaces before its destination canvas mounts.
    let keycaps_state = keycaps_workspace::use_keycaps_workspace_state(
        owner.workspace,
        owner.scope.clone(),
        owner.token,
        owner.revision,
        model.active_board_id.clone(),
    );
    use_context_provider(|| keycaps_state.clone());
    let keymap_layer = use_signal(|| "base".to_owned());
    use_context_provider(|| keymap_workspace::ActiveLayer(keymap_layer));
    let views = &state.views;
    let board = views
        .document
        .boards
        .iter()
        .find(|b| b.id == state.view.model.active_board_id);
    let board_part_ids: HashSet<_> = board
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    let visible: Vec<_> = views
        .document
        .parts
        .iter()
        .filter(|part| board_part_ids.contains(part.id.as_str()))
        .collect();
    let matrices = boardstudio_web_ui_model::tree::visible_matrices(
        &views.document,
        &state.view.model.active_board_id,
        &board_part_ids,
    );
    let (h_sel, select_tree) =
        selection_handlers::build_selection_handlers(&state, snapshot, &render_scope);
    let visible_positions = camera::visible_positions(&visible);
    let cam = camera::use_camera_bundle(
        &state,
        views,
        snapshot,
        &render_scope,
        visible_positions,
        &mut matrix_setup,
    );
    let (nav, navigate_box, instance_scope_pending) = findings_navigation::use_navigation_bundle(
        &state,
        &physical_setup,
        &keycaps_state,
        &cam,
        snapshot,
        &render_scope,
        workspace_routing,
    );
    let canvas = canvas_state::use_canvas_bundle(&state, views, &cam, snapshot, &render_scope);
    let (move_pointer, end_pointer, cancel_pointer) =
        pointer_bundles::use_pointer_bundle(&state, &cam, &render_scope);
    let (keyboard, key_up, start_pan) =
        keyboard::build_keyboard_bundle(&state, &cam, &render_scope);
    let wheel = keyboard::build_wheel(
        editor,
        &state.adapter,
        &render_scope,
        state.render_generation,
        &cam.svg_surface.svg,
        &cam.scene_bounds,
    );
    let wir = workspace_inputs::use_workspace_wiring(
        &state,
        &matrix_setup,
        workspace_routing,
        h_sel,
        select_tree,
        navigate_box,
    );
    let inp = toolbar_inputs::use_layout_toolbar(
        &state,
        &cam,
        &canvas,
        &wir,
        matrices.clone(),
        &matrix_setup,
    );
    let guide = setup_guide_state::use_setup_guide(
        &state,
        &mechanical_settings,
        &cam,
        &canvas.selection.selected_tree_context,
        inp.toolbar_input.as_ref(),
        (&visible, &matrices),
        &matrix_setup,
    );
    let ptr = pointer_bundles::build_canvas_pointer(&state, &cam, views, &render_scope);
    let cmds = commands::use_view_commands(
        &state,
        &cam,
        inp.on_layout_view_mode,
        wir.toggle_footprints,
        guide.state.guide.clone(),
        workspace_routing,
    );
    let guide_state = &guide.state;
    let model = &state.view.model;
    let workspace = state.active_workspace;
    let status = workspace_composition::StatusFooter {
        count: keycaps_fit::presented_findings(&views.findings, &snapshot.document).len(),
        toggle_findings: nav.callbacks.on_toggle_layout_findings,
        layout: state.layout_state,
        part_position: guide_state.active_part_position,
    };
    let footer = view::view_footer(
        workspace,
        state.layout_state.assembly_3d(),
        editor.runtime(),
        &status,
        model,
        &cam,
    );

    let layout_objects = || layout_workspace::ObjectsInput {
        shared: wir.shared_objects.clone(),
        matrix_setup: matrix_setup.clone(),
        matrix_inspector: state.matrix_inspector.clone(),
        pair_created: state.matrix_pair.pair_created_selection,
        on_place_component: cam.placement.part_placement.on_place_component,
        layout_target: state.view.layout_target,
        parts_query: state.view.parts_query,
        on_browse_parts: wir.on_browse_parts,
        placement_error: state
            .matrix_pair
            .matrix_placement
            .error
            .clone()
            .or_else(|| cam.placement.part_placement.error.clone()),
        add_object_menu_open: state.layout_state.add_object_menu_open(),
    };
    let export_objects = (workspace == Workspace::Export).then(layout_objects);
    let layout = inp.toolbar_input.clone().map(|toolbar| {
        let inspector = canvas_inspector_inputs::layout_inspector_input(
            editor,
            &state,
            &canvas,
            &nav.callbacks,
            &inp,
            workspace_routing.select_tree,
        );
        layout_workspace::LayoutFrame {
            objects: layout_objects(),
            inspector,
            toolbar,
            scene: layout_workspace::scene::LayoutScene {
                canvas_aria_label: canvas.inspector.canvas_aria_label,
                canvas_grid: guide.canvas_grid,
                outline_inspector: state.view.outline_inspector.clone(),
                outline_overlay_key: guide.outline_overlay_key.clone(),
                outline_snap_settings: guide.outline_snap_settings.clone(),
                outline_pitch: guide.outline_pitch,
                outline_snap_origins: guide.outline_snap_origins.clone(),
                selected_bridge: guide_state.selected_bridge.cloned(),
                matrix_placement: state.matrix_pair.matrix_placement.clone(),
                mirrored_pair: state.matrix_pair.mirrored_pair.clone(),
                split_focused_half: cmds,
                spacing_ghost_prop: canvas.selection.spacing_ghost_prop.clone(),
                selected_tree_context: canvas.selection.selected_tree_context.clone(),
                canvas_anchor_badge: canvas.selection.canvas_anchor_badge.clone(),
                matrix_pointer_down: ptr.matrix_pointer_down,
                part_pointer_down: ptr.part_pointer_down,
                selection_geometry_anchor: guide.selection_geometry_anchor,
                canvas_transform_menu: guide.canvas_transform_menu,
                matrix_transform_inspector: state.matrix_transform_inspector.clone(),
                drag: cam.placement.drag.clone(),
                layout_transform_target: canvas.selection.layout_transform_target.clone(),
                part_rail_selection: canvas.selection.part_rail_selection.clone(),
                part_rail_snap: canvas.selection.part_rail_snap,
            },
        }
    });
    let frame = (workspace != Workspace::Export).then(|| {
        workspace_composition::Frame::new(workspace_composition::WorkspaceFrame {
            scope: render_scope.clone(),
            snapshot: snapshot.clone(),
            document: views.document.clone(),
            scene: views.scene.clone(),
            model: model.clone(),
            visible_positions,
            owner: state.view.layout_owner.clone(),
            objects: wir.shared_objects.clone(),
            handlers: workspace_composition::CanvasEventHandlers {
                mount: canvas.inspector.mount,
                start_pan: EventHandler::new(start_pan.clone()),
                move_pointer: EventHandler::new(move_pointer.clone()),
                end_pointer: EventHandler::new(end_pointer.clone()),
                cancel_pointer: EventHandler::new(cancel_pointer.clone()),
                keyboard: EventHandler::new(keyboard.clone()),
                key_up: EventHandler::new(key_up.clone()),
                wheel: EventHandler::new(wheel.clone()),
            },
            svg: cam.svg_surface.svg.clone(),
            surface_size: cam.svg_surface.zoom_surface_size,
            matrix_setup: matrix_setup.clone(),
            part_placement: cam.placement.part_placement.clone(),
            placement_error: state
                .matrix_pair
                .matrix_placement
                .error
                .clone()
                .or_else(|| cam.placement.part_placement.error.clone()),
            layout_target: state.view.layout_target,
            parts_query: state.view.parts_query,
            browse_parts: wir.on_browse_parts,
            select_module: wir.mounted_module_select,
            show_position_inspector: canvas.inspector.show_position_inspector,
            camera: cam.scene_bounds.clone(),
            status,
            case: (workspace == Workspace::Case).then(|| case_workspace::CaseFrame {
                instance_scope_pending,
                mechanical_settings: mechanical_settings.clone(),
                physical_setup: physical_setup.clone(),
                on_show_configured_board: nav.show_configured_board,
            }),
            parts: (workspace == Workspace::Parts).then_some(parts_workspace::PartsFrame {
                matrix_placement_busy: state.matrix_pair.matrix_placement.busy,
                on_place_assembly: state.matrix_pair.on_place_matrix_assembly,
                on_open_module_placement: wir.on_open_module_placement,
                on_module_attached: wir.on_module_attached,
            }),
            layout,
        })
    });
    rsx! {
        if let Some(frame) = frame {
            workspace_composition::RoutedWorkspace { workspace, frame }
        }
        section { class: "flex min-h-0 flex-1 flex-col", "aria-label": "Keyboard editor",
            div { class: if state.active_workspace == Workspace::Export { "dock-desk relative flex min-h-0 flex-1 flex-row gap-2 overflow-hidden bg-background p-2 [&_[id=workspace-panel-sheet]]:hidden!" } else { "dock-desk relative flex min-h-0 flex-1 flex-row gap-2 overflow-hidden bg-background p-2" },
                DockLayout {
                    objects: view::objects_panel(view::ObjectsPanelInput {
                        guide_none: guide_state.guide.is_none(),
                        objects_open: editor.panels.objects_open,
                        objects_panel_settings: state.objects_panel_settings,
                        active_workspace: state.active_workspace,
                        layout_state: state.layout_state,
                        export_objects,
                        layout_assembly_3d: state.layout_state.assembly_3d(),
                        layout_snap_settings: state.layout_state.snap_settings(),
                        layout_command_menu: state.layout_state.command_menu(),
                        layout_snap_intent: wir.layout_snap_intent,
                    }),
                    inspector: view::inspector_panel(view::InspectorPanelInput {
                        guide_none: guide_state.guide.is_none(),
                        has_inspector: state.view.has_inspector,
                        inspect_open: editor.panels.inspector_open,
                        inspector_panel_settings: state.inspector_panel_settings,
                        active_workspace: state.active_workspace,
                        layout_findings: state.view.layout_findings,
                        document: views.document.clone(),
                        findings: &views.findings,
                        render_scope: render_scope.clone(),
                        snapshot,
                        render_generation: state.render_generation,
                        on_close_layout_findings: nav.callbacks.on_close_layout_findings,
                        on_layout_finding: nav.callbacks.on_layout_finding,
                    }),
                }
                {view::guide_panel(
                    view::GuidePanelInput {
                        guide: &guide_state.guide,
                        full_case_settings: guide.full_case_settings,
                        active_workspace: state.active_workspace,
                        mechanical_settings: &mechanical_settings,
                        guide_name_failure: state.guide_name_edit.failure,
                        guide_stage_readiness: guide.guide_stage_readiness,
                        guide_stage_detail: guide.guide_stage_detail,
                        name_value: guide.name_value,
                        on_name_change: guide.on_name_change,
                        on_name_commit: guide.on_name_commit,
                        matrix_setup_on_open: matrix_setup.on_open,
                        inline_matrix_setup: guide.inline_matrix_setup,
                        inline_case_basics: guide.inline_case_basics,
                        inline_controller_picker: guide.inline_controller_picker,
                        has_matrix: guide_state.has_matrix,
                        on_choose_controller: cam.placement.part_placement.on_choose_controller,
                        physical_setup_mount: &physical_setup,
                    },
                    guide.on_stage_change,
                    guide.on_open_workspace,
                    guide.on_dismiss_guide,
                    guide.on_finish_guide,
                )}
                {view::canvas_section(
                    view::CanvasSectionInput {
                        setup_guide_active: guide_state.setup_guide_active,
                        active_workspace: state.active_workspace,
                        objects_open: editor.panels.objects_open,
                        inspect_open: editor.panels.inspector_open,
                        workspace: editor.workspace,
                        matrix_setup,
                        mirrored_pair: state.matrix_pair.mirrored_pair.clone(),
                        guide_none: guide_state.guide.is_none(),
                        guide_project_stage: guide_state.guide.as_ref().is_some_and(|guide| {
                            guide.current_stage == SetupGuideStage::Project
                        }),
                        hardware_split: physical_setup.projection.topology
                            == boardstudio_core::model::HardwareTopology::Split,
                        hardware_wired: physical_setup.projection.transport
                            == boardstudio_core::model::HardwareTransport::Wired,
                        layout_assembly_3d: state.layout_state.assembly_3d(),
                        return_workspace: editor.return_workspace,
                        inspector_panel_settings: state.inspector_panel_settings,
                        snapshot,
                        on_empty_board_matrix: wir.on_empty_board_matrix,
                        on_browse_parts: wir.on_browse_parts,
                        show_empty_board: guide.show_empty_board,
                        part_placement: &cam.placement.part_placement,
                        document: views.document.clone(),
                        pending_splay_origin_pick: state.layout_state.pending_splay_origin_pick(),
                        pending_anchor_pick: state.layout_state.pending_anchor_pick(),
                        field_preview: state.view.field_preview,
                        canvas_anchor_badge: &canvas.selection.canvas_anchor_badge,
                    },
                )}
            }
            {footer}
        }
    }
}
