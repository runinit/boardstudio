use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) fn camera_commands(
    camera: super::camera::CameraActions,
) -> impl Iterator<Item = boardstudio_web_ui_shared::ui::command::Command> {
    [
        ("Fit board", camera.fit_board),
        ("Zoom out", camera.zoom_out),
        ("Zoom in", camera.zoom_in),
    ]
    .into_iter()
    .map(
        |(label, on_select)| boardstudio_web_ui_shared::ui::command::Command {
            group: "View",
            label: label.into(),
            reason: None,
            hint: None,
            on_select,
        },
    )
}

pub(super) fn use_view_commands(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
    on_layout_view_mode: EventHandler<bool>,
    toggle_footprints: EventHandler<()>,
    guide: Option<setup_guide::SetupGuidePreferences>,
    workspace_routing: WorkspaceRoutingSlots,
) -> Option<boardstudio_web_ui_model::split_focus::SplitHalf> {
    let views = &state.views;
    let snapshot = state.snapshot;
    let render_scope = state.render_scope.clone();
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let document = views.document.clone();
    let split_focus = state.layout_state.split_focus();
    let accepted_canvas_source = cam.scene_bounds.accepted_canvas_source.clone();
    let render_generation = state.render_generation;
    let active_workspace = state.active_workspace;
    let model = &state.view.model;
    let layout_assembly_3d = state.layout_state.assembly_3d();
    let layout_state = state.layout_state;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let objects_panel_settings = state.objects_panel_settings;
    let inspector_panel_settings = state.inspector_panel_settings;
    let has_inspector = state.view.has_inspector;
    let camera = cam.actions;
    let split_focused_half = layout_workspace::scene::split_focused_half(&document, split_focus);
    let object_commands = use_memo(use_reactive(
        (
            &accepted_canvas_source,
            &snapshot.token,
            &render_scope,
            &render_generation,
            &active_workspace,
        ),
        {
            let model = model.clone();
            let on_select = workspace_routing.select_tree;
            move |(_, _, scope, generation, workspace)| {
                command_palette::editor_commands(&model, workspace, &scope, generation, on_select)
            }
        },
    ));
    let mut palette_commands = object_commands();
    if matches!(active_workspace, Workspace::Layout | Workspace::Pcb) && !layout_assembly_3d() {
        palette_commands.extend(camera_commands(camera));
    }
    if active_workspace == Workspace::Layout {
        palette_commands.extend(layout_workspace::view_commands(
            on_layout_view_mode,
            toggle_footprints,
            layout_state,
            objects_open,
            inspect_open,
            objects_panel_settings,
        ));
    }
    let toggle_panel = |mut requested: Signal<bool>,
                        mut opposite: Signal<bool>,
                        mut settings: Signal<PanelSettings>| {
        if is_compact_viewport() {
            let was_open = requested();
            opposite.set(false);
            requested.set(!was_open);
        } else {
            let mut current = settings();
            current.mode = match current.mode {
                PanelMode::Pinned => PanelMode::Collapsed,
                PanelMode::Collapsed => PanelMode::Pinned,
            };
            settings.set(current);
        }
    };
    let toggle_objects =
        use_callback(move |()| toggle_panel(objects_open, inspect_open, objects_panel_settings));
    let toggle_inspector =
        use_callback(move |()| toggle_panel(inspect_open, objects_open, inspector_panel_settings));
    if guide.is_none() && active_workspace != Workspace::Export {
        for (label, on_select, available) in [
            ("Show/hide Objects", toggle_objects, true),
            ("Show/hide Inspector", toggle_inspector, has_inspector),
        ] {
            if !available {
                continue;
            }
            palette_commands.push(boardstudio_web_ui_shared::ui::command::Command {
                group: "View",
                label: label.into(),
                reason: None,
                hint: None,
                on_select,
            });
        }
    }
    boardstudio_web_ui_shared::ui::command::use_commands(palette_commands);
    split_focused_half
}
