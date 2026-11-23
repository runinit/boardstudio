use super::super::*;

pub(super) fn use_pointer_bundle(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> (
    impl FnMut(PointerEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
) {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let canvas_navigation = &state.canvas_navigation;
    let runtime = &editor.runtime();
    let svg = &cam.svg_surface.svg;
    let drag = &cam.placement.drag;
    let adapter = &state.adapter;
    let layout_owner = &state.view.layout_owner;
    let layout_snap_settings = state.layout_state.snap_settings();
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let tree_cell_anchor = &state.tree_cell_anchor;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let canvas_interaction = &state.canvas_interaction;
    let workspace = editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = state.render_generation;
    let interaction_version = cam.placement.interaction_version;
    let active_workspace = state.active_workspace;
    let space_down = &cam.placement.space_down;
    let move_pointer =
        super::canvas_adapter::move_pointer_handler(super::canvas_adapter::MovePointerInput {
            navigation: canvas_navigation.clone(),
            runtime: runtime.clone(),
            svg: svg.clone(),
            drag: drag.clone(),
            adapter: adapter.clone(),
            render_scope: render_scope.clone(),
            owner: layout_owner.clone(),
            snap_settings: layout_snap_settings,
            placement: part_placement.clone(),
            matrix_placement: matrix_placement.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            mirrored_pair: mirrored_pair.clone(),
            canvas_interaction: canvas_interaction.clone(),
            workspace,
            view_x,
            view_y,
            width,
            height,
            render_generation,
        });
    let end_pointer =
        super::canvas_adapter::end_pointer_handler(super::canvas_adapter::EndPointerInput {
            navigation: canvas_navigation.clone(),
            runtime: runtime.clone(),
            svg: svg.clone(),
            drag: drag.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            placement: part_placement.clone(),
            matrix_placement: matrix_placement.clone(),
            snap_settings: layout_snap_settings,
            render_scope: render_scope.clone(),
            canvas_interaction: canvas_interaction.clone(),
            workspace,
            view_x,
            view_y,
            width,
            height,
            render_generation,
            interaction_version,
        });
    let cancel_pointer =
        super::canvas_adapter::cancel_pointer_handler(super::canvas_adapter::CancelPointerInput {
            runtime: runtime.clone(),
            svg: svg.clone(),
            drag: drag.clone(),
            placement: part_placement.clone(),
            matrix_placement: matrix_placement.clone(),
            render_scope: render_scope.clone(),
            canvas_interaction: canvas_interaction.clone(),
            render_generation,
            interaction_version,
        });
    super::canvas_adapter::use_pointer_workspace_effect(
        active_workspace,
        runtime.clone(),
        drag.clone(),
        svg.clone(),
        space_down.clone(),
        render_scope.clone(),
        interaction_version,
    );
    (move_pointer, end_pointer, cancel_pointer)
}
