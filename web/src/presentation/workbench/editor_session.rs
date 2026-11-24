use super::*;
use boardstudio_web_ui_model::editor::EditorContext;
pub(in crate::presentation) mod action_handlers;
mod camera;
mod commands;
pub(in crate::presentation) mod findings_navigation;
pub(in crate::presentation) mod keyboard;
mod selection_handlers;
pub(in crate::presentation) mod session_state;
pub(in crate::presentation) mod setup_guide_state;
pub(in crate::presentation) mod setup_workspaces;
mod view;
pub(in crate::presentation) mod workspace_inputs;

#[component]
pub(in crate::presentation) fn Editor() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let snapshot = runtime.with_model(|model| model.accepted.clone());
    let (Some(scope), Some(snapshot)) = (runtime.scope(), snapshot) else {
        return rsx! {};
    };
    rsx! { EditorBody { scope, snapshot } }
}

#[component]
fn EditorBody(scope: Scope, snapshot: AcceptedSnapshot) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let inst_pref = use_signal(|| {
        runtime.scope().and_then(|s| {
            s.instance_id
                .map(|explicit_id| instance_selection::Preference {
                    session_epoch: s.session_epoch,
                    document_id: s.document_id,
                    explicit_id,
                })
        })
    });
    let instance_selection = use_context_provider(|| InstanceSelection(inst_pref));
    let editor = EditorContext {
        runtime: use_hook(|| CopyValue::new(runtime)),
        version: use_context::<Signal<u64>>(),
        workspace: use_context::<WorkspaceState>().0,
        scope_generation: use_context::<SelectionAdapter>().generation,
        instance_selection,
        panels: use_context::<CompactPanelState>(),
        layers: use_context::<LayerVisibility>(),
        return_workspace: use_context::<ExportReturnWorkspace>().0,
    };
    use_context_provider(|| editor);
    rsx! { EditorSession { scope, snapshot } }
}

#[component]
pub(in crate::presentation) fn EditorSession(scope: Scope, snapshot: AcceptedSnapshot) -> Element {
    let editor = use_context::<EditorContext>();
    let render_scope = scope;
    let snapshot = &snapshot;
    let (state, workspace_routing, mut matrix_setup) =
        session_state::use_session_state(snapshot, &render_scope);
    let (physical_setup, mechanical_settings) = session_state::use_case_and_project_setup(&state);
    let outputs = workspace_composition::use_workspace_outputs();
    use_context_provider(|| outputs);
    // One wiring solve for PCB/Keymap/Export and the Setup Guide. The hook
    // never reads the resolution signal here, so this session re-renders on
    // wiring changes only while the guide (below) is reading it.
    let guide_wiring_active = state.setup_state.guide_preferences()()
        .as_ref()
        .is_some_and(|guide| guide.open);
    let shared_wiring = pcb_wiring::use_pcb_wiring_shared(
        editor.runtime(),
        editor.version,
        guide_wiring_active
            || matches!(
                state.active_workspace,
                Workspace::Pcb | Workspace::Keymap | Workspace::Export
            ),
    );
    use_context_provider(|| shared_wiring);
    use_context_provider(|| workspace_composition::WorkspaceInteraction {
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
    let visible: Vec<_> = workspace_composition::board_visible_parts(
        &views.document,
        &state.view.model.active_board_id,
    );
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
    let mount = camera::use_session_canvas_mount(
        &cam.svg_surface.svg,
        cam.svg_surface.zoom_surface_size,
        editor.runtime(),
        &cam.placement.part_placement,
        &state.matrix_pair.matrix_placement,
    );
    let session_selected_tree_context =
        workspace_composition::current_tree_context(&state.adapter, model, &render_scope);
    let show_position_inspector =
        camera::session_show_position_inspector(model, session_selected_tree_context.as_ref());
    let (keyboard, key_up) = keyboard::build_keyboard_handlers(&state, &cam);
    let interaction = consume_context::<LayoutInteraction>();
    let handlers = super::super::editor::canvas_adapter::use_canvas_surface(
        super::super::editor::canvas_adapter::CanvasAdapter {
            surface: super::super::editor::canvas_adapter::CanvasSurface {
                runtime: editor.runtime(),
                svg: cam.svg_surface.svg.clone(),
                adapter: state.adapter.clone(),
                interaction,
                scope: render_scope.clone(),
                generation: state.render_generation,
                owner: state.view.layout_owner.clone(),
                workspace: editor.workspace,
                bounds: cam.scene_bounds.canvas_bounds(),
                tree_cell_anchor: state.tree_cell_anchor.clone(),
                layout_state: state.layout_state,
                transform_inspector: state.matrix_transform_inspector.clone(),
                range_ids: Rc::default(),
            },
            navigation: state.canvas_navigation.clone(),
            part_placement: cam.placement.part_placement.clone(),
            matrix_placement: state.matrix_pair.matrix_placement.clone(),
            mirrored_pair: state.matrix_pair.mirrored_pair.clone(),
            interaction_version: cam.placement.interaction_version,
            active_workspace: state.active_workspace,
        },
        mount,
        keyboard,
        key_up,
    );
    let mut layout_input = use_context::<layout_input::LayoutInput>();
    let layout_keyboard = handlers.keyboard;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let pair = &state.matrix_pair.mirrored_pair;
    let part_cancel = cam.placement.part_placement.on_cancel;
    let matrix_cancel = matrix_placement.on_cancel;
    let matrix_owner = matrix_placement.cancel_owner.clone();
    let pair_cancel = pair.on_cancel;
    let pair_owner = pair
        .placement
        .as_ref()
        .map(|active| active.owner.clone())
        .or_else(|| pair.form.as_ref().map(|form| form.owner.clone()));
    let cancel_placement = use_callback(move |mode| match mode {
        CanvasMode::PlacingPart => part_cancel.call(()),
        CanvasMode::PlacingMatrix => {
            if let Some(owner) = matrix_owner.clone() {
                matrix_cancel.call(owner);
            }
        }
        CanvasMode::PlacingMirroredPair => {
            if let Some(owner) = pair_owner.clone() {
                pair_cancel.call(owner);
            }
        }
        _ => {}
    });
    use_hook(move || {
        layout_input.keyboard.set(Some(layout_keyboard));
        layout_input.cancel_placement.set(Some(cancel_placement));
    });
    use_effect(move || {
        let _ = interaction.version();
        layout_input::focus_workbench_if_body();
    });
    use_effect(use_reactive((&snapshot.token,), |_| {
        layout_input::focus_workbench_if_body();
    }));
    use_drop(move || {
        if *layout_input.keyboard.peek() == Some(layout_keyboard) {
            layout_input.dispatch(CanvasEvent::ScopeChanged);
            layout_input.keyboard.set(None);
            layout_input.cancel_placement.set(None);
        }
    });
    let wir = workspace_inputs::use_workspace_wiring(
        &state,
        &matrix_setup,
        workspace_routing,
        h_sel,
        select_tree,
        navigate_box,
    );
    let guide = setup_guide_state::use_setup_guide(
        shared_wiring,
        &state,
        &mechanical_settings,
        &cam,
        (&visible, &matrices),
        &matrix_setup,
    );
    // Layout owns its canvas bundle, toolbar, pointer handlers and palette
    // commands inside `LayoutWorkspace`; the session only routes by workspace.
    commands::use_view_commands(&state, &cam, guide.state.guide.clone(), workspace_routing);
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

    let payload = match workspace {
        Workspace::Case => {
            workspace_composition::WorkspacePayload::Case(case_workspace::CaseFrame {
                instance_scope_pending,
                mechanical_settings: mechanical_settings.clone(),
                physical_setup: physical_setup.clone(),
                on_show_configured_board: nav.show_configured_board,
            })
        }
        Workspace::Parts => {
            workspace_composition::WorkspacePayload::Parts(parts_workspace::PartsFrame {
                matrix_placement_busy: state.matrix_pair.matrix_placement.busy,
                on_place_assembly: state.matrix_pair.on_place_matrix_assembly,
                on_open_module_placement: wir.on_open_module_placement,
                on_module_attached: wir.on_module_attached,
            })
        }
        Workspace::Layout => workspace_composition::WorkspacePayload::Layout(
            workspace_composition::LayoutSessionServices {
                matrix_inspector: state.matrix_inspector.clone(),
                matrix_transform_inspector: state.matrix_transform_inspector.clone(),
                matrix_placement: state.matrix_pair.matrix_placement.clone(),
                mirrored_pair: state.matrix_pair.mirrored_pair.clone(),
                outline_inspector: state.view.outline_inspector.clone(),
                layout_findings: state.view.layout_findings,
                on_return_from_layout_finding: nav.callbacks.on_return_from_layout_finding,
                on_close_layout_findings: nav.callbacks.on_close_layout_findings,
                on_layout_finding: nav.callbacks.on_layout_finding,
                visible_ids: views.visible_ids.clone(),
                objects_panel_settings: state.objects_panel_settings,
            },
        ),
        Workspace::Pcb => workspace_composition::WorkspacePayload::Pcb,
        Workspace::Keymap => workspace_composition::WorkspacePayload::Keymap,
        Workspace::Keycaps => workspace_composition::WorkspacePayload::Keycaps,
        Workspace::Export => workspace_composition::WorkspacePayload::Export,
    };
    let export_objects = (workspace == Workspace::Export).then(|| {
        layout_workspace::objects(workspace_inputs::layout_objects_input(
            &state,
            &cam,
            &wir,
            &matrix_setup,
        ))
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
            handlers,
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
            show_position_inspector,
            camera: cam.scene_bounds.clone(),
            status,
            payload,
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
                        export_objects,
                        layout_state: state.layout_state,
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
                    EventHandler::new({
                        let dismiss = guide.on_dismiss_guide;
                        move |_| {
                            if let Some(main) = web_sys::window().and_then(|window| window.document())
                                .and_then(|document| document.get_element_by_id("m1-workbench"))
                                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
                            {
                                let options = web_sys::FocusOptions::new();
                                options.set_prevent_scroll(true);
                                let _ = main.focus_with_options(&options);
                            }
                            dismiss.call(());
                        }
                    }),
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
                        return_workspace: editor.return_workspace,
                        inspector_panel_settings: state.inspector_panel_settings,
                        snapshot,
                        on_empty_board_matrix: wir.on_empty_board_matrix,
                        on_browse_parts: wir.on_browse_parts,
                        show_empty_board: guide.show_empty_board,
                    },
                )}
            }
            {footer}
        }
    }
}
