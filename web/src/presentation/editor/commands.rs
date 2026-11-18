use super::super::*;

pub(super) struct CommandsInput<'a> {
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub split_focus: Signal<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub accepted_canvas_source: layout_canvas::SceneSource,
    pub snapshot: &'a AcceptedSnapshot,
    pub render_scope: Scope,
    pub render_generation: u64,
    pub active_workspace: &'static str,
    pub model: &'a ReadModel,
    pub workspace_routing: WorkspaceRoutingSlots,
    pub layout_assembly_3d: Signal<bool>,
    pub on_layout_view_mode: EventHandler<bool>,
    pub toggle_footprints: EventHandler<()>,
    pub layout_state: layout_workspace::LayoutWorkspaceState,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub objects_panel_settings: Signal<PanelSettings>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub guide: Option<setup_guide::SetupGuidePreferences>,
    pub has_inspector: bool,
}

pub(super) struct Commands {
    pub split_focused_half: Option<boardstudio_web_ui_model::split_focus::SplitHalf>,
    pub on_fit_keymap_board: EventHandler<()>,
    pub on_zoom_keymap_out: EventHandler<()>,
    pub on_zoom_keymap_in: EventHandler<()>,
}

pub(super) fn use_commands(
    input: CommandsInput<'_>,
    on_fit_keymap_board: impl FnMut(()) + 'static,
    on_zoom_keymap_out: impl FnMut(()) + 'static,
    on_zoom_keymap_in: impl FnMut(()) + 'static,
) -> Commands {
    let CommandsInput {
        document,
        split_focus,
        accepted_canvas_source,
        snapshot,
        render_scope,
        render_generation,
        active_workspace,
        model,
        workspace_routing,
        layout_assembly_3d,
        on_layout_view_mode,
        toggle_footprints,
        layout_state,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
        guide,
        has_inspector,
    } = input;
    let split_focused_half = if boardstudio_web_ui_model::split_focus::has_split(&document) {
        Some(split_focus())
    } else {
        None
    };
    let on_fit_keymap_board = EventHandler::new(on_fit_keymap_board);
    let on_zoom_keymap_out = EventHandler::new(on_zoom_keymap_out);
    let on_zoom_keymap_in = EventHandler::new(on_zoom_keymap_in);
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
    if matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps") && !layout_assembly_3d()
    {
        for (label, on_select) in [
            ("Fit board", on_fit_keymap_board),
            ("Zoom out", on_zoom_keymap_out),
            ("Zoom in", on_zoom_keymap_in),
        ] {
            palette_commands.push(boardstudio_web_ui_shared::ui::command::Command {
                group: "View",
                label: label.into(),
                reason: None,
                hint: None,
                on_select,
            });
        }
    }
    if active_workspace == "Layout" {
        for (label, mode) in [("2D", false), ("3D assembly", true)] {
            palette_commands.push(boardstudio_web_ui_shared::ui::command::Command {
                group: "View",
                label: label.into(),
                reason: None,
                hint: None,
                on_select: EventHandler::new(move |()| on_layout_view_mode.call(mode)),
            });
        }
        palette_commands.push(boardstudio_web_ui_shared::ui::command::Command {
            group: "View",
            label: "Footprints".into(),
            reason: None,
            hint: None,
            on_select: toggle_footprints,
        });
        let mut add_open = layout_state.add_object_menu_open();
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let mut objects_settings = objects_panel_settings;
        palette_commands.push(boardstudio_web_ui_shared::ui::command::Command {
            group: "Current Workspace",
            label: "Add object".into(),
            reason: None,
            hint: None,
            on_select: EventHandler::new(move |()| {
                if is_compact_viewport() {
                    inspect_open.set(false);
                    objects_open.set(true);
                } else {
                    let mut current = objects_settings();
                    current.mode = PanelMode::Pinned;
                    objects_settings.set(current);
                }
                add_open.set(true);
            }),
        });
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
    if guide.is_none() && active_workspace != "Export" {
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
    Commands {
        split_focused_half,
        on_fit_keymap_board,
        on_zoom_keymap_out,
        on_zoom_keymap_in,
    }
}
