use super::super::*;

pub(super) struct InspectorInput<'a> {
    pub setup_a: &'a super::setup_bundles::SetupA,
    pub setup_b: &'a super::setup_bundles::SetupB,
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
    pub binding_actions: keymap::BindingActions,
    pub macro_actions: keymap::MacroActions,
    pub layer_actions: keymap::LayerActions,
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
        binding_actions,
        macro_actions,
        layer_actions,
    } = input;
    let keymap_view = &setup_b.keymap_view;
    let keymap_layer_id = setup_a.keymap_layer_id;
    let model = &setup_a.model;
    let keymap_layer = wir.wiring.keymap_layer;
    let keymap_export = wir.wiring.keymap_export;
    let keymap_select = wir.wiring.keymap_select;
    let adapter = &setup_a.adapter;
    let physical_setup_mount = &setup_b.physical_setup_mount;
    let mechanical_settings = &setup_b.mechanical_settings;
    let current_scope = &setup_a.current_scope;
    let case_scene = &wir.pcb.case_scene;
    let case_selection = setup_a.case_selection;
    let case_selected_layer_id = &wir.pcb.case_selected_layer_id;
    let case_selected_context = &wir.pcb.case_selected_context;
    let selected_tree_context = &canvas.selection.selected_tree_context;
    let show_configured_board = wir.wiring.show_configured_board;
    let case_display = wir.wiring.case_display;
    let parts_query = setup_a.parts_query;
    let parts_selection = setup_a.parts_selection;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &setup_a.matrix_placement;
    let layout_target = setup_a.layout_target;
    let on_place_matrix_assembly = setup_a.on_place_matrix_assembly;
    let layout_selection_kind = setup_a.layout_selection_kind;
    let runtime = &setup_a.runtime;
    let render_generation = setup_a.render_generation;
    let resolution = &setup_b.resolution;
    let firmware_feedback = &setup_b.firmware_feedback;
    let firmware_editable = setup_b.firmware_editable;
    let firmware_on_edit = setup_b.firmware_on_edit;
    let part_net_actions = &setup_b.part_net_actions;
    let part_input_actions = &setup_b.part_input_actions;
    let pcb_wiring_edit_board = wir.wiring.pcb_wiring_edit_board;
    let pcb_wiring_source = &wir.pcb.pcb_wiring_source;
    let pcb_wiring_mode_actions = &wir.pcb.pcb_wiring_mode_actions;
    let pcb_wiring_pin_actions = &wir.pcb.pcb_wiring_pin_actions;
    let pcb_wiring_apply_actions = &wir.pcb.pcb_wiring_apply_actions;
    let pcb_wiring_protected_remap_actions = &wir.pcb.pcb_wiring_protected_remap_actions;
    let on_resolve = setup_b.on_resolve;
    let document = &views.document;
    let keycaps_view = &setup_b.keycaps_view;
    let keycaps_settings_actions = &setup_b.keycaps_settings_actions;
    let keycaps_select = wir.wiring.keycaps_select;
    let keycaps_fit_state = &setup_b.keycaps_fit_state;
    let keycaps_finding = wir.wiring.keycaps_finding;
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
    let outline_inspector = &setup_a.outline_inspector;
    let layout_findings = setup_a.layout_findings;
    let layout_owner = &setup_a.layout_owner;
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
        "Keymap" => super::canvas_inspector_inputs::keymap_inspector_input(
            super::canvas_inspector_inputs::KeymapInspectorArgs {
                keymap_view: keymap_view.clone(),
                render_scope: render_scope.clone(),
                keymap_layer_id,
                model,
                keymap_layer,
                keymap_export,
                binding_actions,
                macro_actions,
                keymap_select,
                snapshot,
                scope_generation: adapter.generation,
                layer_actions,
            },
        ),
        "Case" => super::canvas_inspector_inputs::case_inspector_input(
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
        "Parts" => super::canvas_inspector_inputs::parts_inspector_input(
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
        "PCB" => super::canvas_inspector_inputs::pcb_inspector_input(
            super::canvas_inspector_inputs::PcbInspectorArgs {
                selected_tree_context: selected_tree_context.clone(),
                render_scope: render_scope.clone(),
                runtime: runtime.clone(),
                snapshot,
                selected_context: adapter.selected_context,
                part_placement: part_placement.clone(),
                pcb_wiring_source: pcb_wiring_source.clone(),
                render_generation,
                resolution: resolution.clone(),
                firmware_feedback: firmware_feedback.clone(),
                firmware_editable,
                firmware_on_edit,
                part_net_actions: part_net_actions.clone(),
                part_input_actions: part_input_actions.clone(),
                pcb_wiring_edit_board,
                mode_actions: pcb_wiring_mode_actions.clone(),
                pin_actions: pcb_wiring_pin_actions.clone(),
                apply_actions: pcb_wiring_apply_actions.clone(),
                protected_remap_actions: pcb_wiring_protected_remap_actions.clone(),
                on_resolve,
            },
        ),
        "Keycaps" => super::canvas_inspector_inputs::keycaps_inspector_input(
            super::canvas_inspector_inputs::KeycapsInspectorArgs {
                model,
                current_scope: current_scope.clone(),
                runtime: runtime.clone(),
                document: document.clone(),
                keycaps_view: keycaps_view.clone(),
                keycaps_settings_actions: keycaps_settings_actions.clone(),
                keycaps_select,
                keycaps_fit_state: keycaps_fit_state.clone(),
                keycaps_finding,
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
