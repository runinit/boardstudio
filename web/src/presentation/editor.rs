use super::*;
mod action_handlers;
mod camera;
mod canvas_adapter;
mod canvas_inspector_inputs;
mod canvas_pointer;
mod canvas_state;
mod commands;
mod findings_navigation;
mod inspector_bundle;
mod keyboard;
mod pointer_bundles;
mod selection_handlers;
mod setup_bundles;
mod setup_guide_state;
mod setup_shell;
mod setup_workspaces;
mod toolbar_inputs;
mod view;
mod view_scene;
mod workspace_inputs;

/// Ticket 18: the ghost of the previous spacing lives here, in the Editor,
/// next to the other Layout view state. The tracker below records the last
/// accepted snapshot; a committed spacing change raises the pre-commit scene
/// as a dashed ghost with its delta note until the next edit, undo/redo, or
/// selection change. Nothing here writes the document.
#[component]
pub(super) fn Editor() -> Element {
    let (setup_a, mut workspace_routing, mut matrix_setup, on_parts_select) =
        setup_bundles::use_editor_setup_shell();
    let (setup_b, layer_actions, binding_actions, macro_actions) =
        setup_bundles::use_setup_workspaces(&setup_a);
    let Some(render_scope) = setup_a.current_scope.clone() else {
        return rsx! {};
    };
    let Some(snapshot) = setup_a.model.accepted.as_ref() else {
        return rsx! {};
    };
    let views = setup_workspaces::derive_views(
        &setup_a.model,
        snapshot,
        setup_a
            .resolved_document
            .clone()
            .unwrap_or_else(|| snapshot.document.clone()),
    );
    let board = views
        .document
        .boards
        .iter()
        .find(|b| b.id == setup_a.model.active_board_id);
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
    let matrices: Vec<_> = views
        .document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix
                .board_id
                .as_deref()
                .is_some_and(|id| id == setup_a.model.active_board_id)
                || (matrix.board_id.is_none()
                    && (matrix
                        .part_ids
                        .iter()
                        .any(|id| board_part_ids.contains(id.as_str()))
                        || (matrix.part_ids.is_empty() && views.document.boards.len() <= 1)))
        })
        .collect();
    let (h_sel, select_tree) = selection_handlers::build_selection_handlers(
        &setup_a,
        snapshot,
        &render_scope,
        workspace_routing,
    );
    let cam = camera::use_camera_bundle(
        &setup_a,
        &setup_b,
        &views,
        snapshot,
        &render_scope,
        visible.clone(),
        &mut matrix_setup,
    );
    let (nav, navigate_box, instance_scope_pending) = findings_navigation::use_navigation_bundle(
        &setup_a,
        &setup_b,
        &cam,
        snapshot,
        &render_scope,
        workspace_routing,
    );
    let canvas = canvas_state::use_canvas_bundle(&setup_a, &views, &cam, snapshot, &render_scope);
    let (move_pointer, end_pointer, cancel_pointer) =
        pointer_bundles::use_pointer_bundle(&setup_a, &cam, &render_scope);
    let (keyboard, key_up, start_pan) =
        keyboard::build_keyboard_bundle(&setup_a, &cam, &render_scope);
    let (pk, wheel) = keyboard::build_keyboard_rest(&setup_a, &cam, snapshot, &render_scope);
    let (wir, objects_input) = workspace_inputs::use_workspace_wiring(
        workspace_inputs::WiringInput {
            setup_a: &setup_a,
            setup_b: &setup_b,
            snapshot,
            render_scope: &render_scope,
            matrix_setup: &matrix_setup,
            workspace_routing: &mut workspace_routing,
            instance_scope_pending,
            layer_actions: &layer_actions,
            cam: &cam,
            sel: h_sel,
            nav: &nav,
            pk,
            ops: on_parts_select,
        },
        select_tree,
        navigate_box,
        end_pointer.clone(),
        cancel_pointer.clone(),
        keyboard.clone(),
        wheel.clone(),
    );
    let inp = toolbar_inputs::use_workspace_inputs(
        toolbar_inputs::WsInputsInput {
            setup_a: &setup_a,
            setup_b: &setup_b,
            cam: &cam,
            canvas: &canvas,
            wir: &wir,
            snapshot,
            render_scope: &render_scope,
            matrices: &matrices,
            matrix_setup: &matrix_setup,
            instance_scope_pending,
        },
        start_pan.clone(),
        move_pointer.clone(),
        key_up.clone(),
    );
    let inspector_input = inspector_bundle::inspector_input_for(inspector_bundle::InspectorInput {
        setup_a: &setup_a,
        setup_b: &setup_b,
        views: &views,
        cam: &cam,
        nav: &nav,
        canvas: &canvas,
        wir: &wir,
        inp: &inp,
        snapshot,
        render_scope: &render_scope,
        workspace_routing,
        instance_scope_pending,
        binding_actions,
        macro_actions,
        layer_actions,
    });
    let guide = setup_guide_state::use_guide_bundle(setup_guide_state::GuideInput {
        setup_a: &setup_a,
        setup_b: &setup_b,
        views: &views,
        cam: &cam,
        canvas: &canvas,
        inp: &inp,
        snapshot,
        render_scope: &render_scope,
        visible: &visible,
        matrices: &matrices,
        matrix_setup: &matrix_setup,
    });
    let ptr = pointer_bundles::build_canvas_pointer(&setup_a, &cam, &views, &render_scope);
    let cmds = commands::use_view_commands(commands::CmdInput {
        setup_a: &setup_a,
        views: &views,
        cam: &cam,
        wir: &wir,
        inp: &inp,
        guide: &guide,
        snapshot,
        render_scope: &render_scope,
        workspace_routing,
    });
    view::view_root(
        view::ViewRootInput {
            setup_a: &setup_a,
            setup_b: &setup_b,
            views: &views,
            cam,
            nav: &nav,
            canvas: &canvas,
            wir: &wir,
            inp,
            guide,
            ptr,
            cmds,
            objects_input,
            inspector_input,
            matrix_setup,
            render_scope,
            snapshot,
        },
        view_scene::SvgNavHandlers {
            start_pan,
            move_pointer,
            end_pointer,
        },
        view_scene::SvgKeyHandlers {
            cancel_pointer,
            keyboard,
            key_up,
        },
        wheel,
    )
}
