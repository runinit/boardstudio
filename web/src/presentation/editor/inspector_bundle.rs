use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) struct InspectorInput<'a> {
    pub setup_a: &'a super::setup_bundles::SetupA,
    pub setup_b: &'a super::setup_bundles::SharedSetup,
    pub views: &'a super::setup_workspaces::Views,
    pub cam: &'a super::camera::CamNamed,
    pub nav: &'a super::findings_navigation::NavNamed,
    pub canvas: &'a super::canvas_state::CanvasNamed,
    pub wir: &'a super::workspace_inputs::WsWiring,
    pub inp: &'a super::toolbar_inputs::WsInputs,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: &'a Scope,
    pub workspace_routing: WorkspaceRoutingSlots,
    pub instance_scope_pending: bool,
}

pub(super) fn inspector_input_for(
    input: InspectorInput<'_>,
) -> workspace_composition::WorkspaceInspectorInput {
    let InspectorInput {
        setup_a,
        setup_b,
        views,
        cam,
        nav,
        canvas,
        wir,
        inp,
        snapshot,
        render_scope,
        workspace_routing,
        instance_scope_pending,
    } = input;
    let model = &setup_a.view.model;
    let adapter = &setup_a.adapter;
    let physical_setup_mount = &setup_b.physical_setup_mount;
    let mechanical_settings = &setup_b.mechanical_settings;
    let current_scope = &setup_a.view.current_scope;
    let case_scene = &wir.case_scene;
    let case_selection = setup_a.case_selection;
    let case_selected_layer_id = &wir.case_selected_layer_id;
    let case_selected_context = &wir.case_selected_context;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let show_configured_board = wir.show_configured_board;
    let case_display = wir.case_display;
    let parts_query = setup_a.view.parts_query;
    let parts_selection = setup_a.view.parts_selection;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_pair.matrix_placement;
    let layout_target = setup_a.view.layout_target;
    let on_place_matrix_assembly = setup_a.matrix_pair.on_place_matrix_assembly;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let runtime = &setup_a.editor.runtime();
    let render_generation = setup_a.render_generation;
    let document = &views.document;
    let layout_state = setup_a.layout_state;
    let context_summary = &canvas.inspector.context_summary;
    let board_inspector_projection = &canvas.inspector.board_inspector_projection;
    let show_position_inspector = canvas.inspector.show_position_inspector;
    let component_inspector = &canvas.inspector.component_inspector;
    let companion_placement = &canvas.inspector.companion_placement;
    let matrix_inspector = &setup_a.matrix_inspector;
    let key_size = &setup_a.key_size;
    let matrix_transform_inspector = &setup_a.matrix_transform_inspector;
    let pending_anchor_pick = setup_a.pending_anchor_pick;
    let pending_splay_origin_pick = setup_a.pending_splay_origin_pick;
    let outline_inspector = &setup_a.view.outline_inspector;
    let layout_findings = setup_a.view.layout_findings;
    let layout_owner = &setup_a.view.layout_owner;
    let board_inspector = &setup_a.board_inspector;
    let findings = &views.findings;
    let active_workspace = setup_a.active_workspace;
    let on_close_layout_findings = nav.callbacks.on_close_layout_findings;
    let on_layout_finding = nav.callbacks.on_layout_finding;
    let on_return_from_layout_finding = nav.callbacks.on_return_from_layout_finding;
    let on_companion_placement_action = inp.on_companion_placement_action;
    let on_component_inspector_action = inp.on_component_inspector_action;
    let on_open_module_placement = wir.on_open_module_placement;
    let on_module_attached = wir.on_module_attached;
    match active_workspace {
        Workspace::Case => super::canvas_inspector_inputs::case_inspector_input(
            super::canvas_inspector_inputs::CaseInspectorArgs {
                physical_setup_mount: physical_setup_mount.clone(),
                mechanical_settings: mechanical_settings.clone(),
                instance_scope_pending,
                current_scope: current_scope.clone(),
                case_scene: case_scene.clone(),
                case_selection,
                case_selected_layer_id: case_selected_layer_id.clone(),
                case_selected_context: case_selected_context.clone(),
                model,
                selected_tree_context: selected_tree_context.clone(),
                show_configured_board,
                case_display,
            },
        ),
        Workspace::Parts => super::canvas_inspector_inputs::parts_inspector_input(
            super::canvas_inspector_inputs::PartsInspectorArgs {
                snapshot,
                current_scope: current_scope.clone(),
                parts_query,
                parts_selection,
                selected_context: adapter.selected_context,
                part_placement: part_placement.clone(),
                matrix_placement: matrix_placement.clone(),
                layout_target,
                on_place_matrix_assembly,
                on_open_module_placement,
                on_module_attached,
                layout_selection_kind,
            },
        ),
        _ => super::canvas_inspector_inputs::layout_inspector_input(
            super::canvas_inspector_inputs::LayoutInspectorArgs {
                layout_state,
                runtime: runtime.clone(),
                context_summary,
                board_inspector_projection: board_inspector_projection.clone(),
                show_position_inspector,
                component_inspector: component_inspector.clone(),
                companion_placement: companion_placement.clone(),
                matrix_inspector: matrix_inspector.clone(),
                key_size: key_size.clone(),
                matrix_transform_inspector: matrix_transform_inspector.clone(),
                pending_anchor_pick,
                pending_splay_origin_pick,
                selected_tree_context: selected_tree_context.clone(),
                model,
                select_tree: workspace_routing.select_tree,
                outline_inspector: outline_inspector.clone(),
                layout_findings,
                layout_owner: layout_owner.clone(),
                board_inspector: board_inspector.clone(),
                document: document.clone(),
                findings: findings.clone(),
                active_workspace,
                render_scope: render_scope.clone(),
                snapshot,
                render_generation,
                on_close_layout_findings,
                on_layout_finding,
                on_return_from_layout_finding,
                on_companion_placement_action,
                on_component_inspector_action,
            },
        ),
    }
}
