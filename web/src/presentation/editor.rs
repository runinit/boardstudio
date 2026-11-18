use super::*;
mod action_handlers;
mod camera;
mod findings_navigation;
mod keyboard;
mod pointer;
mod selection_handlers;

/// Ticket 18: the ghost of the previous spacing lives here, in the Editor,
/// next to the other Layout view state. The tracker below records the last
/// accepted snapshot; a committed spacing change raises the pre-commit scene
/// as a dashed ghost with its delta note until the next edit, undo/redo, or
/// selection change. Nothing here writes the document.
#[component]
pub(super) fn Editor() -> Element {
    boardstudio_web_ui_shared::layer_dock::use_layer_dock_provider();
    parts::use_assembly_action_provider();
    let mut workspace_routing = use_hook(|| WorkspaceRoutingSlots {
        select_tree: EventHandler::new(|_: objects::TreeSelectRequest| {}),
        navigate: EventHandler::new(|_: (Scope, String, Option<String>)| {}),
    });
    let runtime = use_context::<Rc<Runtime>>();
    let compact_panel_state = use_context::<CompactPanelState>();
    let objects_open = compact_panel_state.objects_open;
    let mut inspect_open = compact_panel_state.inspector_open;
    let preference_warning = use_context::<PreferenceStorageWarning>().0;
    let objects_panel_settings = use_panel_settings(PanelSide::Objects, preference_warning);
    let inspector_panel_settings = use_panel_settings(PanelSide::Inspector, preference_warning);
    let created_request_signal = use_context::<Signal<Option<SetupGuideRequest>>>();
    let setup_state = setup_workspace::use_setup_workspace_state(
        runtime.clone(),
        created_request_signal,
        objects_open,
        inspect_open,
        objects_panel_settings,
        inspector_panel_settings,
    );
    let guide_preferences = setup_state.guide_preferences();
    let geometry_scripts_open = setup_state.geometry_scripts_open();
    let guide_name_edit = use_setup_project_name(runtime.clone());
    let guide_name_draft = guide_name_edit.draft;
    let guide_name_failure = guide_name_edit.failure;
    let adapter = use_context::<SelectionAdapter>();
    let layout_component_inspector_lifetime =
        use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
    let version = use_context::<Signal<u64>>();
    let observed_version = version();
    let instance_preference = setup_state.instance_preference();
    let instance_selection = InstanceSelection(instance_preference);
    let case_workspace_state = case_workspace::use_case_workspace_state();
    let case_selection = case_workspace_state.selection();
    let case_tree_expanded = case_workspace_state.tree_expanded();
    let workspace = use_context::<WorkspaceState>().0;
    let layout_state =
        layout_workspace::use_layout_workspace_state(adapter.selected_context, workspace);
    let canvas_navigation = use_canvas_navigation_state(workspace);
    let return_workspace = use_context::<ExportReturnWorkspace>().0;
    let active_workspace = workspace();
    let requested_workspace_panel =
        panels::use_workspace_panel_defaults(active_workspace, objects_open, inspect_open);
    let render_generation = (adapter.generation)();
    let layout_selection_kind = layout_state.selection_kind();
    // Ticket 14: view-only split editing focus. Like Handles, this signal
    // never writes the document; only canvas clicks move it.
    let split_focus = layout_state.split_focus();
    // Ticket 18: ghost of the previous spacing. The tracker records the last
    // accepted snapshot it processed; a committed spacing change raises the
    // pre-commit scene as the ghost. The effect re-runs on every runtime
    // change (Runtime::submit always notifies, including selection), but only
    // writes the signals when the tracker advances — selection-only paints
    // resolve to Keep.
    let mut last_spacing_seen = use_signal(|| None::<spacing_ghost::SeenAccepted>);
    let mut spacing_ghost_state = use_signal(|| None::<spacing_ghost::SpacingGhostState>);
    let ghost_effect_runtime = runtime.clone();
    let ghost_effect_selection = adapter.selected_context;
    use_effect(move || {
        let _ = version();
        let tree_context = (ghost_effect_selection)();
        let model = ghost_effect_runtime.model();
        let Some(snapshot) = model.accepted.clone() else {
            return;
        };
        let current = spacing_ghost::AcceptedView {
            revision: snapshot.document.revision,
            transaction: snapshot.scene.transaction_id.clone(),
            document: snapshot.document.clone(),
            scene: snapshot.scene.clone(),
            selected_ids: model.selected_part_ids.clone(),
            selected_context: tree_context,
        };
        // Clone the settled values first: the tracker borrows nothing, and the
        // guarded writes below only fire on real advances.
        let seen_before = last_spacing_seen.peek().clone();
        let ghost_before = spacing_ghost_state.peek().clone();
        match spacing_ghost::advance_spacing_ghost(
            seen_before.as_ref(),
            &current,
            ghost_before.as_ref(),
        ) {
            spacing_ghost::GhostAdvance::Keep => {}
            spacing_ghost::GhostAdvance::Set { seen, ghost } => {
                if Some(&seen) != seen_before.as_ref() {
                    last_spacing_seen.set(Some(seen));
                }
                if ghost != ghost_before {
                    spacing_ghost_state.set(ghost);
                }
            }
        }
    });
    let layout_snap_settings = layout_state.snap_settings();
    let layout_command_menu = layout_state.command_menu();
    let mut layout_transform_tool = layout_state.transform_tool();
    let layout_assembly_3d = layout_state.assembly_3d();
    let tree_cell_anchor = use_hook(|| Rc::new(RefCell::new(None::<OwnedTreeCellAnchor>)));
    let matrix_inspector = objects::use_matrix_inspector(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let board_inspector = board_inspector::use_board_inspector(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let key_size = objects::use_key_size(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let matrix_splay_affect = layout_state.splay_affect();
    let matrix_transform_inspector = objects::use_workspace_matrix_transform(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        matrix_splay_affect,
        "Layout",
    );
    let pending_splay_origin_pick = layout_state.pending_splay_origin_pick();
    let pending_anchor_pick = layout_state.pending_anchor_pick();
    let layout_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        "Layout",
    );
    let pcb_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        "PCB",
    );
    let mut matrix_setup = objects::use_matrix_setup(
        runtime.clone(),
        version,
        adapter.selected_context,
        adapter.anchor_scope,
        workspace,
        adapter.generation,
    );
    let parts_state = parts_workspace::use_parts_workspace_state();
    let parts_assembly_orientation = parts_state.assembly_orientation();
    let canvas_interaction = use_hook(CanvasInteractionArbiter::default);
    let matrix_placement = objects::use_matrix_placement(
        runtime.clone(),
        objects::MatrixPlacementInput {
            version,
            selected_context: adapter.selected_context,
            anchor_scope: adapter.anchor_scope,
            workspace,
            scope_generation: adapter.generation,
            assembly_orientation: parts::PartsAssemblyOrientation(parts_assembly_orientation),
            canvas_interaction: canvas_interaction.clone(),
        },
    );
    let on_place_matrix_assembly = {
        let on_place = matrix_placement.on_place;
        let runtime = runtime.clone();
        let mut workspace = workspace;
        let mut selected_context = adapter.selected_context;
        let mut anchor_scope = adapter.anchor_scope;
        let mut layout_assembly_3d = layout_assembly_3d;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let canvas_interaction = canvas_interaction.clone();
        EventHandler::new(move |source: objects::MatrixPlacementSource| {
            if workspace() != "Parts" || canvas_interaction.current().is_some() {
                return;
            }
            on_place.call(source);
            workspace.set("Layout");
            layout_assembly_3d.set(false);
            selected_context.set(None);
            anchor_scope.set(None);
            objects_open.set(false);
            inspect_open.set(false);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: Vec::new(),
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
        })
    };
    let pair_created_selection = layout_state.pair_created_selection();
    let on_mirrored_pair_created = use_callback({
        let runtime = runtime.clone();
        let mut adapter = adapter.clone();
        let mut pair_created_selection = pair_created_selection;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |created: objects::MirroredPairCreated| {
            let current_scope = runtime.scope();
            if workspace() != "Layout"
                || (adapter.generation)() != created.owner.scope_generation
                || current_scope.as_ref() != Some(&created.owner.scope)
                || created.scope != created.owner.scope
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if snapshot.token != created.result_token
                || snapshot.document.revision != created.result_revision
                || snapshot.token == created.owner.snapshot_token
                || snapshot.document.revision <= created.owner.revision
                || snapshot.document.id != created.owner.scope.document_id
                || snapshot.session_epoch != created.owner.scope.session_epoch
                || model.active_board_id != created.owner.board_id
                || model.active_instance_id != created.owner.scope.instance_id
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: created.result_revision,
                    })
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.left_matrix_id)
                || !snapshot
                    .document
                    .matrices
                    .iter()
                    .any(|matrix| matrix.id == created.right_matrix_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.left_layout_id)
                || !snapshot
                    .document
                    .layouts
                    .iter()
                    .any(|layout| layout.id == created.right_layout_id)
            {
                return;
            }
            let context = objects::TreeContext::Matrix {
                matrix_id: created.left_matrix_id.clone(),
            };
            let Some(part_ids) = objects::resolve_selection(&model, &context)
                .filter(|part_ids| !part_ids.is_empty())
            else {
                return;
            };
            adapter
                .selected_context
                .set(Some(objects::ScopedTreeContext {
                    scope: created.scope.clone(),
                    context,
                }));
            adapter.anchor_scope.set(None);
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids,
                range_part_ids: Vec::new(),
                mode: SelectionMode::Replace,
            });
            pair_created_selection.set(Some(created));
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    });
    let mut mirrored_pair = objects::use_mirrored_pair(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        on_mirrored_pair_created,
        canvas_interaction.clone(),
    );
    if mirrored_pair.owns_canvas {
        matrix_setup.can_open = false;
    }
    if matrix_setup.projection.is_some() {
        mirrored_pair.can_open = false;
    }
    let (outline_inspector, outline_activation) = outline_lifecycle::use_outline_lifecycle(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let layer_visibility = use_context::<LayerVisibility>();
    let parts_query: PartsQuery = parts_state.query();
    let parts_selection: PartsSelection = parts_state.selected();
    let parts_selection_generation = parts_state.selection_generation();
    let layout_target: Signal<Option<String>> = use_signal(|| None);
    let keymap_state = keymap_workspace::use_keymap_workspace_state();
    let keymap_layer_id = keymap_state.active_layer_id();
    let has_inspector = matches!(
        active_workspace,
        "Layout" | "Parts" | "Keymap" | "Keycaps" | "Case" | "PCB"
    );
    let on_parts_select = {
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        move |_| {
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let model = runtime.model();
    let current_scope = runtime.scope();
    let accepted_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let layout_owner = LayoutOwnerIdentity {
        scope: current_scope.clone(),
        token: accepted_token,
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: render_generation,
        workspace: active_workspace,
    };
    let layout_findings = use_layout_findings_state(
        runtime.clone(),
        workspace,
        adapter.clone(),
        layout_owner.clone(),
    );
    use_effect(use_reactive((&layout_owner,), {
        let tree_cell_anchor = tree_cell_anchor.clone();
        move |(owner,)| {
            let stale = tree_cell_anchor
                .borrow()
                .as_ref()
                .is_some_and(|anchor| anchor.owner != owner);
            if stale {
                tree_cell_anchor.borrow_mut().take();
            }
        }
    }));
    // The Relations highlight is transient view state. A selection, workspace or board change
    // clears it; an accepted revision that drops its parts is handled by the canvas filter.
    let relation_highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| RelationHighlight(relation_highlight));
    let highlight_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
    );
    use_effect(use_reactive((&highlight_context,), move |(_,)| {
        RelationHighlight(relation_highlight).clear();
    }));
    // Ticket 16: the amber at-rest field preview is transient view state too,
    // but separate from the violet reference above: hovering or focusing an
    // Inspector number field shows it, and leave, blur, unmount, a selection
    // change or any accepted revision clears it (the set is computed from
    // the accepted document, so a commit always stales it).
    let field_preview = use_signal(|| None::<FieldPreviewState>);
    use_context_provider(|| FieldPreview(field_preview));
    let field_preview_source = model.accepted.as_ref().map(|snapshot| FieldPreviewSource {
        document: snapshot.document.clone(),
        scene: snapshot.scene.clone(),
    });
    use_context_provider(|| field_preview_source.clone());
    let field_preview_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
        model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
    );
    use_effect(use_reactive((&field_preview_context,), move |(_,)| {
        FieldPreview(field_preview).clear();
    }));
    let active_board_id = model.active_board_id.clone();
    let keycaps_state = keycaps_workspace::use_keycaps_workspace_state(
        layout_owner.workspace,
        layout_owner.scope.clone(),
        layout_owner.token,
        layout_owner.revision,
        active_board_id.clone(),
    );
    let focused_keycaps_finding = keycaps_state.focused_finding();
    let keycaps_navigation_alive = keycaps_state.navigation_alive();
    let layer_source =
        current_scope
            .clone()
            .zip(model.accepted.as_ref())
            .map(|(scope, snapshot)| keymap::LayerSource {
                scope,
                token: snapshot.token,
                revision: snapshot.document.revision,
            });
    let keycaps_edit_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(&model, scope) && instance_selection.is_current(&model)
        })
        .and_then(|scope| {
            model
                .accepted
                .as_ref()
                .map(|snapshot| keycaps_settings::KeycapsEditSource {
                    scope: scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                })
        });
    let keycaps_fit_source = current_scope
        .as_ref()
        .filter(|scope| active_board_scope_matches(&model, scope))
        .and_then(|scope| {
            model.accepted.as_ref().map(|snapshot| {
                let case_preview_current = runtime.cad_scene().is_some_and(|scene| {
                    scene.exact
                        && scene.scope == *scope
                        && scene.token == snapshot.token
                        && scene.prepared.revision == snapshot.document.revision
                });
                keycaps_fit::KeycapsFitSource {
                    scope: scope.clone(),
                    token: snapshot.token,
                    revision: snapshot.document.revision,
                    case_preview_current,
                }
            })
        });
    let keycaps_fit_state = keycaps_fit::use_keycaps_fit(runtime.clone(), keycaps_fit_source);
    let keycaps_settings_actions = keycaps_settings::use_keycaps_settings_actions(
        runtime.clone(),
        keycaps_edit_source,
        workspace,
        adapter.generation,
    );
    let pcb_wiring_mount = pcb_wiring::use_pcb_wiring_controller(runtime.clone(), version);
    let pcb_part_net_actions = pcb_wiring::use_pcb_part_net_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let firmware_position_actions = pcb_wiring::use_firmware_position_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_mount.resolution_signal,
    );
    let part_input_actions = pcb_wiring::use_part_input_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    // Read guide state at every admission: an async operation may outlive the rendered guide
    // stage or switch to a different accepted project.
    let project_setup_active: Rc<dyn Fn() -> bool> = Rc::new({
        let runtime = runtime.clone();
        move || {
            let model = runtime.model();
            let Some(project_id) = model
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.id.as_str())
            else {
                return false;
            };
            guide_preferences
                .peek()
                .as_ref()
                .is_some_and(|preferences| {
                    preferences.open
                        && preferences.current_stage == SetupGuideStage::Project
                        && preferences.project_id == project_id
                })
        }
    });
    let physical_setup_mount = pcb_physical_setup::use_controller(
        runtime.clone(),
        version,
        adapter.generation,
        project_setup_active.clone(),
        instance_selection,
        Rc::new({
            let runtime = runtime.clone();
            let generation = adapter.generation;
            let project_setup_active = project_setup_active;
            let adapter = adapter.clone();
            move |identity: &pcb_physical_setup::OwnerIdentity, strict: bool| {
                let model = runtime.model();
                let Some(accepted) = model.accepted.as_ref() else {
                    return false;
                };
                let accepted_transition_current =
                    identity.scope_transition.as_ref().is_none_or(|expected| {
                        identity.context == pcb_physical_setup::OwnerContext::CaseInspector
                            && !strict
                            && expected.matches_current(
                                adapter.scope_transition().as_ref(),
                                Some(accepted.token),
                                generation(),
                            )
                            && runtime.scope() == expected.next_scope
                    });
                let context_current = match identity.context {
                    pcb_physical_setup::OwnerContext::ProjectGuide => {
                        project_setup_active()
                            && model.active_board_id == identity.board_id
                            && (!strict || model.active_instance_id == identity.instance_id)
                    }
                    pcb_physical_setup::OwnerContext::CaseInspector => {
                        case_setup_context_is_current(
                            identity,
                            strict,
                            workspace(),
                            runtime.scope().as_ref(),
                        )
                    }
                };
                accepted_transition_current
                    && context_current
                    && generation() == identity.generation
                    && accepted.session_epoch == identity.session_epoch
                    && accepted.document.id == identity.document_id
                    && (!strict
                        || (accepted.token == identity.token
                            && accepted.document.revision == identity.revision
                            && (identity.context
                                != pcb_physical_setup::OwnerContext::CaseInspector
                                || instance_selection.is_current(&model))))
            }
        }),
        Rc::new(|accepted, intent| {
            Box::pin(parts::prepare_physical_setup_resources(accepted, intent))
        }),
    );
    // Keep the operation observer alive even when the workspace panel is hidden.
    let layer_actions = keymap::use_layer_operations(
        runtime.clone(),
        layer_source.clone(),
        keymap_layer_id,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let keymap_layer_value = keymap_layer_id();
    let keymap_projection = keymap_workspace::use_projection(
        runtime.clone(),
        accepted_token,
        current_scope.clone(),
        active_board_id.clone(),
        keymap_layer_value,
    );
    let keycaps_projection = keycaps_workspace::use_projection(
        runtime.clone(),
        accepted_token,
        current_scope.clone(),
        active_board_id.clone(),
    );
    let encoder_input_actions = keymap::use_encoder_inputs(
        runtime.clone(),
        layer_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let binding_actions = keymap::use_binding_operations(
        runtime.clone(),
        keymap::BindingProjectionSources {
            source: layer_source.clone(),
            view: keymap_projection.as_ref().map(|(view, _)| view.clone()),
            encoder_projection: encoder_input_actions.projection,
            current_encoder_projection: encoder_input_actions.current,
        },
        keymap_layer_id,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let keymap_view = keymap_projection.as_ref().map(|(view, _)| view.clone());
    let keymap_contours = keymap_projection.map(|(_, contours)| contours);
    let keycaps_view = keycaps_projection.as_ref().map(|(view, _)| view.clone());
    let keycaps_contours = keycaps_projection.map(|(_, contours)| contours);
    // Keep macro operation observation alive when another workspace hides the panel.
    let macro_actions = keymap::use_macro_operations(
        runtime.clone(),
        layer_source,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
    );
    let on_show_mechanical_finding = {
        let runtime = runtime.clone();
        let captured_scope = current_scope.clone();
        let captured_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
        let captured_generation = (adapter.generation)();
        let generation = adapter.generation;
        let mut workspace = workspace;
        let mut inspect_open = inspect_open;
        let mut case_selection = case_selection;
        let captured_revision = model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision);
        EventHandler::new(
            move |request: mechanical_settings_mount::MechanicalFindingNavigation| {
                let Some(scope) = captured_scope.as_ref() else {
                    return;
                };
                if request.scope != *scope
                    || Some(request.token) != captured_token
                    || Some(request.revision) != captured_revision
                {
                    return;
                }
                let model = runtime.model();
                if generation() != captured_generation
                    || runtime.scope().as_ref() != Some(scope)
                    || !instance_selection.is_current(&model)
                    || model.accepted.as_ref().is_none_or(|current| {
                        Some(current.token) != captured_token
                            || current.document.revision != request.revision
                    })
                {
                    return;
                }
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                let scene = runtime
                    .cad_scene()
                    .filter(|scene| &scene.scope == scope && Some(scene.token) == captured_token);
                let assembly = request
                    .resolution
                    .as_ref()
                    .map(|resolution| &resolution.assembly)
                    .or_else(|| scene.as_ref().and_then(|scene| scene.mechanical.as_ref()));
                let Some(assembly) = assembly else {
                    return;
                };
                let captured_document = &snapshot.document;
                let Some(finding) = assembly
                    .diagnostics
                    .iter()
                    .find(|finding| finding.id == request.finding_id)
                else {
                    return;
                };
                if let Some(layer) = assembly.stack.iter().find(|layer| {
                    finding.target_ids.contains(&layer.id)
                        || assembly.case.bodies.iter().any(|entry| {
                            entry.body.id == layer.id
                                && entry.body.mounts.as_ref().is_some_and(|mounts| {
                                    mounts
                                        .iter()
                                        .any(|mount| finding.target_ids.contains(&mount.id))
                                })
                        })
                }) {
                    case_selection.focus_layer(
                        scope.clone(),
                        request.token,
                        request.revision,
                        layer.id.clone(),
                    );
                    workspace.set("Case");
                } else if let Some(body) = captured_document.case_bodies.iter().find(|body| {
                    body.board_id == scope.board_id && finding.target_ids.contains(&body.id)
                }) {
                    case_selection.focus_layer(
                        scope.clone(),
                        request.token,
                        request.revision,
                        body.id.clone(),
                    );
                    case_selection.body.set(Some(case_viewer::BodySelection {
                        scope: scope.clone(),
                        body_id: body.id.clone(),
                    }));
                    workspace.set("Case");
                } else {
                    let Some(board) = captured_document
                        .boards
                        .iter()
                        .find(|board| board.id == scope.board_id)
                    else {
                        return;
                    };
                    let part_ids = finding
                        .target_ids
                        .iter()
                        .filter(|id| board.part_ids.contains(id))
                        .cloned()
                        .collect::<Vec<_>>();
                    if !part_ids.is_empty() {
                        runtime.submit(Event::SelectParts {
                            operation_id: runtime.operation(),
                            part_ids,
                            range_part_ids: Vec::new(),
                            mode: SelectionMode::Replace,
                        });
                        workspace.set("Layout");
                    }
                }
                inspect_open.set(true);
                runtime.report(finding.message.clone());
            },
        )
    };
    let mechanical_settings = mechanical_settings_mount::use_mechanical_settings_mount(
        runtime.clone(),
        adapter.generation,
        workspace,
        instance_selection,
        case_selection,
        on_show_mechanical_finding,
    );
    let Some(render_scope) = current_scope.clone() else {
        return rsx! {};
    };
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = snapshot.document.clone();
    let scene = model
        .display_preview
        .clone()
        .unwrap_or_else(|| snapshot.scene.clone());
    let findings = layout_findings_for_display(
        &snapshot.scene.findings,
        model
            .display_preview
            .as_ref()
            .map(|preview| preview.findings.as_slice()),
    );
    let board = document
        .boards
        .iter()
        .find(|b| b.id == model.active_board_id);
    let board_part_ids: HashSet<_> = board
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    let visible: Vec<_> = document
        .parts
        .iter()
        .filter(|part| board_part_ids.contains(part.id.as_str()))
        .collect();
    let matrices: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix
                .board_id
                .as_deref()
                .is_some_and(|id| id == model.active_board_id)
                || (matrix.board_id.is_none()
                    && (matrix
                        .part_ids
                        .iter()
                        .any(|id| board_part_ids.contains(id.as_str()))
                        || (matrix.part_ids.is_empty() && document.boards.len() <= 1)))
        })
        .collect();
    let visible_ids: Rc<Vec<String>> =
        Rc::new(visible.iter().map(|part| part.id.clone()).collect());
    let select_tree =
        selection_handlers::select_tree_handler(selection_handlers::SelectTreeInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            generation: render_generation,
            workspace,
            inspector_settings: inspector_panel_settings,
            objects_open,
            inspect_open,
            layout_state,
            outline_activation,
        });
    let on_layout_selection_kind = selection_handlers::on_layout_selection_kind_handler(
        selection_handlers::OnLayoutSelectionKindInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            objects_open,
            inspect_open,
            selection_kind: layout_selection_kind,
            workspace,
            inspector_panel_settings,
        },
    );
    let on_layout_snap_intent = selection_handlers::on_layout_snap_intent_handler(
        runtime.clone(),
        adapter.clone(),
        layout_owner.clone(),
        layout_snap_settings,
        workspace,
    );
    let on_pcb_selection_kind = selection_handlers::on_pcb_selection_kind_handler(
        selection_handlers::OnPcbSelectionKindInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            objects_open,
            inspect_open,
            selection_kind: layout_selection_kind,
            workspace,
            inspector_panel_settings,
        },
    );
    let on_pcb_snap_intent = selection_handlers::on_pcb_snap_intent_handler(
        runtime.clone(),
        adapter.clone(),
        layout_owner.clone(),
        layout_snap_settings,
        workspace,
    );
    let on_keymap_select =
        selection_handlers::on_keymap_select_handler(selection_handlers::OnKeymapSelectInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            objects_open,
            inspect_open,
            workspace,
            keymap_layer_id,
        });
    let on_keycaps_select =
        selection_handlers::on_keycaps_select_handler(selection_handlers::OnKeycapsSelectInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            objects_open,
            inspect_open,
            workspace,
            instance_selection,
        });
    let on_pcb_part_hit =
        selection_handlers::on_pcb_part_hit_handler(selection_handlers::OnPcbPartHitInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            workspace,
            instance_selection,
            layout_selection_kind,
        });
    let on_mounted_module_select = action_handlers::on_mounted_module_select_handler(
        action_handlers::OnMountedModuleSelectInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            revision: snapshot.document.revision,
            inspect_open,
            workspace,
            instance_selection,
            parts_state,
            objects_open,
            objects_panel_settings,
        },
    );
    let on_open_module_placement = action_handlers::on_open_module_placement_handler(
        action_handlers::OnOpenModulePlacementInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            token: snapshot.token,
            revision: snapshot.document.revision,
            workspace,
            objects_open,
            inspect_open,
            instance_selection,
            parts_selection,
        },
    );
    let on_module_attached =
        action_handlers::on_module_attached_handler(action_handlers::OnModuleAttachedInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            workspace,
            objects_open,
            inspect_open,
            parts_selection_generation,
            parts_selection,
            instance_selection,
        });
    let on_pcb_wiring_edit_board = action_handlers::on_pcb_wiring_edit_board_handler(
        runtime.clone(),
        adapter.clone(),
        render_scope.clone(),
        render_generation,
        snapshot.token,
        workspace,
        instance_selection,
    );
    let case_admission = action_handlers::case_admission(
        runtime.clone(),
        adapter.clone(),
        case_workspace_state,
        instance_selection,
        render_scope.clone(),
        snapshot.token,
        render_generation,
    );
    let on_case_action = action_handlers::on_case_action_handler(
        case_admission.clone(),
        workspace_routing.navigate,
        workspace,
        case_workspace_state,
    );
    let on_case_display =
        action_handlers::on_case_display_handler(case_admission, workspace, case_workspace_state);
    let on_keymap_layer = action_handlers::on_keymap_layer_handler(
        runtime.clone(),
        adapter.clone(),
        render_scope.clone(),
        render_generation,
        workspace,
        keymap_state,
    );
    let on_keymap_export =
        action_handlers::on_keymap_export_handler(action_handlers::OnKeymapExportInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: render_generation,
            workspace,
            instance_selection,
        });
    let nudge_tree = action_handlers::nudge_tree_handler(
        runtime.clone(),
        adapter.clone(),
        render_generation,
        inspect_open,
    );
    let camera::SvgSurface {
        svg,
        zoom_surface_size,
    } = camera::use_svg_surface(&canvas_navigation, workspace);

    let camera::SceneBounds {
        accepted_canvas_source,
        canvas_source,
        displayed_layout_bounds,
        workspace_rect_bounds,
        bounds,
        min_x,
        max_x,
        min_y,
        max_y,
        keymap_scale_ratio,
        zoom_percent,
        width,
        height,
        view_x,
        view_y,
        view_box,
    } = camera::use_scene_bounds(camera::SceneBoundsInput {
        model: &model,
        snapshot,
        document: document.clone(),
        scene: scene.clone(),
        render_scope: render_scope.clone(),
        active_workspace,
        keymap_view: keymap_view.clone(),
        keymap_contours: keymap_contours.clone(),
        keycaps_view: keycaps_view.clone(),
        keycaps_contours: keycaps_contours.clone(),
        visible: visible.clone(),
        layout_assembly_3d,
        svg: svg.clone(),
        zoom_surface_size,
    });
    let camera::FooterBounds {
        footer_camera_bounds,
        footer_board_bounds,
        footer_selection_bounds,
        footer_selection_available,
    } = camera::footer_bounds(camera::FooterBoundsInput {
        min_x,
        max_x,
        min_y,
        max_y,
        workspace_rect_bounds,
        displayed_layout_bounds,
        active_workspace,
        keymap_view: keymap_view.clone(),
        document: document.clone(),
        scene: scene.clone(),
        board_id: render_scope.board_id.clone(),
        model: &model,
    });
    let on_fit_keymap_board = camera::on_fit_keymap_board_handler(
        layout_owner.clone(),
        runtime.clone(),
        workspace,
        adapter.clone(),
        svg.clone(),
        footer_board_bounds,
        footer_camera_bounds,
    );
    let on_fit_keymap_selection =
        camera::on_fit_keymap_selection_handler(camera::OnFitKeymapSelectionInput {
            layout_owner: layout_owner.clone(),
            runtime: runtime.clone(),
            workspace,
            adapter: adapter.clone(),
            svg: svg.clone(),
            footer_camera_bounds,
            selection_ids: model.selected_part_ids.clone(),
            footer_selection_bounds,
        });
    let mut zoom_keymap = camera::zoom_keymap_handler(camera::ZoomKeymapInput {
        zoom_runtime: runtime.clone(),
        zoom_workspace: workspace,
        zoom_scope: render_scope.clone(),
        zoom_owner: layout_owner.clone(),
        zoom_adapter: adapter.clone(),
        zoom_svg: svg.clone(),
        zoom_token: snapshot.token,
        zoom_revision: snapshot.document.revision,
        zoom_ratio: keymap_scale_ratio,
        zoom_bounds: bounds,
        zoom_view: (view_x, view_y, width, height),
        render_generation,
    });
    let mut zoom_out = zoom_keymap.clone();
    let on_zoom_keymap_out = move |_| zoom_out(-1.0);
    let on_zoom_keymap_in = move |_| zoom_keymap(1.0);
    let camera::PlacementBindings {
        part_placement,
        drag,
        space_down,
        interaction_version,
        observed_interaction_version,
    } = camera::use_placement_bindings(camera::PlacementBindingsInput {
        min_x,
        max_x,
        min_y,
        max_y,
        camera_center: model.camera.center,
        runtime: runtime.clone(),
        workspace,
        adapter: adapter.clone(),
        version,
        layout_selection_kind,
        guide_preferences,
        parts_query,
        parts_selection,
        layout_snap_settings,
        layout_target,
        objects_open,
        inspect_open,
        canvas_interaction: canvas_interaction.clone(),
        matrix_setup: &mut matrix_setup,
        mirrored_pair: mirrored_pair.clone(),
        svg: svg.clone(),
        canvas_navigation: &canvas_navigation,
    });
    let navigate_scoped = findings_navigation::navigate_scoped_handler(
        runtime.clone(),
        adapter.clone(),
        drag.clone(),
        svg.clone(),
        render_generation,
    );
    let navigate = findings_navigation::navigate_handler(
        runtime.clone(),
        adapter.clone(),
        navigate_scoped.clone(),
        render_generation,
        setup_state,
        instance_preference,
    );
    let on_keycaps_finding = findings_navigation::on_keycaps_finding_handler(
        findings_navigation::OnKeycapsFindingInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            workspace,
            objects_open,
            inspect_open,
            body_selection: case_selection.body,
            case_selection_for_owner: case_selection,
            navigation_alive: keycaps_navigation_alive.clone(),
            inspector_settings: inspector_panel_settings,
            fit_state: keycaps_fit_state.state.clone(),
            keycaps_state: keycaps_state.clone(),
            source_surface: svg.clone(),
            select_tree: workspace_routing.select_tree,
            case_selection,
        },
    );
    let findings_navigation::LayoutFindingCallbacks {
        on_toggle_layout_findings,
        on_close_layout_findings,
        on_return_from_layout_finding,
        on_layout_finding,
        resume_pending_layout_finding,
    } = findings_navigation::use_layout_finding_callbacks(
        findings_navigation::LayoutFindingCallbacksInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            layout_owner: layout_owner.clone(),
            layout_findings,
            inspect_open,
            objects_open,
            inspector_panel_settings,
            workspace,
            workspace_routing,
            setup_state,
            keycaps_state: keycaps_state.clone(),
            keycaps_navigation_alive: keycaps_navigation_alive.clone(),
            case_selection,
            svg: svg.clone(),
        },
    );
    findings_navigation::use_layout_finding_resume(findings_navigation::LayoutFindingResumeInput {
        layout_findings,
        resume_pending_layout_finding,
        adapter: adapter.clone(),
        workspace,
        version,
        runtime: runtime.clone(),
        keycaps_state: keycaps_state.clone(),
        case_selection,
        svg: svg.clone(),
        keycaps_navigation_alive: keycaps_navigation_alive.clone(),
        keycaps_fit_state: keycaps_fit_state.clone(),
    });
    findings_navigation::use_physical_setup_reconcile(
        findings_navigation::PhysicalSetupReconcileInput {
            instance_preference,
            physical_setup_busy: physical_setup_mount.busy,
            observed_version,
            observed_interaction_version,
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            scope: render_scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: render_generation,
            drag: drag.clone(),
        },
        navigate_scoped.clone(),
    );
    let instance_scope_pending = !instance_selection.is_current(&model);
    let on_show_configured_board = findings_navigation::on_show_configured_board_handler(
        navigate.clone(),
        render_scope.clone(),
    );
    findings_navigation::use_selection_cleanup(
        runtime.clone(),
        adapter.clone(),
        drag.clone(),
        svg.clone(),
        space_down.clone(),
    );
    let selected_tree_context = adapter.selected_context.read().clone().filter(|selected| {
        selected.scope == render_scope
            && selection::context_is_current(&model, &selected.scope, &selected.context)
    });
    // Ticket 02: the anchor badge and the "Placed from" readout share one
    // projection of the displayed scene. It clears with the selection, and
    // the canvas ring reads the same value through its scene prop.
    let canvas_anchor_badge = (!(layer_visibility.hidden)().contains("Keys"))
        .then(|| {
            selected_tree_context.as_ref().and_then(|selected| {
                let scene_refs: Vec<_> = canvas_source.scene.matrix_scenes.iter().collect();
                anchor_badge::anchor_badge(
                    &canvas_source.document,
                    &scene_refs,
                    Some(&selected.context),
                )
            })
        })
        .flatten();
    // Ticket 18: the live spacing ghost, if it still describes this paint —
    // the revision and selection it was born with, and no drag preview up.
    // The revision check covers the paint between a commit landing and the
    // tracker effect recording it; the selection check covers the paint
    // between a selection landing and the effect retiring the ghost.
    let spacing_ghost_prop = spacing_ghost_state
        .read()
        .clone()
        .filter(|state| {
            state.revision == snapshot.document.revision
                && state.selected_ids == model.selected_part_ids
                && state.selected_context == selected_tree_context
                && model.display_preview.is_none()
        })
        .map(|state| layout_canvas::SpacingGhost {
            scene: state.scene.clone(),
            delta_mm: state.delta_mm,
            axis_x: state.axis_x,
        });
    let transform_matrix_id = selected_tree_context
        .as_ref()
        .filter(|selected| selected.scope == render_scope && active_workspace == "Layout")
        .and_then(|selected| {
            let matrix_id = match &selected.context {
                objects::TreeContext::Matrix { matrix_id }
                | objects::TreeContext::Row { matrix_id, .. }
                | objects::TreeContext::Column { matrix_id, .. }
                | objects::TreeContext::Key { matrix_id, .. } => matrix_id.as_str(),
                objects::TreeContext::Component {
                    matrix_id: Some(matrix_id),
                    ..
                } => matrix_id.as_str(),
                _ => return None,
            };
            Some(matrix_id.to_owned())
        });
    let transform_matrix = use_memo(use_reactive(
        (&accepted_canvas_source, &transform_matrix_id),
        |(source, matrix_id)| {
            let matrix_id = matrix_id?;
            source
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .cloned()
                .map(Rc::new)
        },
    ))();
    let transform_projection = use_memo(use_reactive(
        (&canvas_source, &transform_matrix_id),
        |(source, matrix_id)| {
            let matrix_id = matrix_id?;
            source
                .scene
                .matrix_scenes
                .iter()
                .find(|matrix| matrix.matrix_id == matrix_id)
                .cloned()
                .map(Rc::new)
        },
    ))();
    let layout_transform_target = transform_matrix
        .zip(transform_projection)
        .zip(selected_tree_context.as_ref())
        .map(|((matrix, projection), selected)| (matrix, projection, selected.context.clone()));
    // Ticket 07: the part rail facts for an independent part selection behind
    // `transform_tools` and the part rail below. Members and companions keep
    // their matrix's tools: the facts name them and the pure list stays empty.
    let part_rail_selection =
        selected_tree_context
            .as_ref()
            .and_then(|selected| match &selected.context {
                objects::TreeContext::Component {
                    part_id: Some(part_id),
                    matrix_id: None,
                    ..
                } => {
                    let part = document.parts.iter().find(|part| &part.id == part_id)?;
                    Some((
                        part_id.clone(),
                        boardstudio_web_layout::part_rail::PartRailFacts {
                            anchored: part.anchor.is_some(),
                            anchor_column: part.anchor.as_ref().map_or(0, |anchor| anchor.column),
                            locked: part.locked == Some(true),
                            member_or_companion: document
                                .matrices
                                .iter()
                                .any(|matrix| matrix.part_ids.contains(part_id)),
                        },
                    ))
                }
                _ => None,
            });
    // Ticket 07: the part rail drag snap: anchored drags snap in the anchor
    // matrix's pitch, like the matrix tools snap in their own.
    let part_rail_snap = part_rail_selection.as_ref().map(|(part_id, _)| {
        let anchor_pitch = document
            .parts
            .iter()
            .find(|part| &part.id == part_id)
            .and_then(|part| part.anchor.as_ref())
            .and_then(|anchor| {
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == anchor.matrix_id)
            })
            .map(|matrix| matrix.pitch);
        objects::gesture_snap_inputs(&layout_snap_settings.read().clone(), anchor_pitch, None)
    });
    let transform_tool_owner = (
        layout_owner.scope.clone(),
        layout_owner.generation,
        layout_transform_target
            .as_ref()
            .map(|(matrix, _, _)| matrix.id.clone()),
        active_workspace,
        layout_assembly_3d(),
    );
    layout_state.observe_transform_tool_owner(transform_tool_owner);
    let component_inspector_key = layout_component_inspector_owner_key(
        &model,
        active_workspace,
        selected_tree_context.as_ref(),
    );
    let component_inspector_generation =
        layout_component_inspector_lifetime.update(component_inspector_key);
    let component_inspector_scope_generation = (adapter.generation)();
    let component_inspector_identity = (
        accepted_token,
        current_scope.clone(),
        model.active_board_id.clone(),
        model.active_instance_id.clone(),
        model.selected_part_ids.clone(),
        selected_tree_context.clone(),
        component_inspector_generation,
        component_inspector_scope_generation,
        active_workspace,
    );
    let component_inspector = use_memo(use_reactive((&component_inspector_identity,), {
        let runtime = runtime.clone();
        move |(identity,)| {
            let (
                token,
                _scope,
                active_board_id,
                active_instance_id,
                selected_part_ids,
                selected,
                context_generation,
                scope_generation,
                workspace,
            ) = identity;
            if workspace != "Layout" {
                return None;
            }
            let model = runtime.model();
            if model.accepted.as_ref().map(|snapshot| snapshot.token) != token
                || model.active_board_id != active_board_id
                || model.active_instance_id != active_instance_id
                || model.selected_part_ids != selected_part_ids
            {
                return None;
            }
            layout_component_inspector_projection(
                &model,
                selected.as_ref(),
                context_generation,
                scope_generation,
            )
        }
    }))
    .read()
    .clone();
    let companion_placement = (active_workspace == "Layout")
        .then(|| {
            companion_placement_projection(
                &model,
                selected_tree_context.as_ref(),
                render_generation,
                render_generation,
            )
        })
        .flatten();
    let board_inspector_projection = board_inspector.projection.clone().filter(|_| {
        component_inspector.is_none()
            && matrix_inspector.projection.is_none()
            && key_size.projection.is_none()
            && matrix_transform_inspector.projection.is_none()
            && outline_inspector.is_none()
    });
    let show_position_inspector = board_inspector_projection.is_none()
        && component_inspector.is_none()
        && selected_tree_context.as_ref().is_none_or(|selected| {
            let resolved = selection::resolve_context(&model, &selected.context);
            resolved.is_some_and(|ids| {
                ids.len() == 1 && model.selected_part_ids.as_slice() == ids.as_slice()
            }) && matches!(&selected.context, objects::TreeContext::Component { .. })
        });
    let context_summary = selected_tree_context
        .as_ref()
        .and_then(|selected| context_summary::summarize(&model, &selected.context));
    let mount = EventHandler::new(CanvasNavigationState::mount_handler(
        zoom_surface_size,
        runtime.clone(),
        svg.clone(),
        part_placement.projection.is_some(),
    ));
    let placement_active = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy;
    let canvas_aria_label = if matrix_placement.placement.is_some() || matrix_placement.busy {
        "Matrix placement canvas. Move the pointer or use arrow keys; click or press Enter to place, Escape to cancel."
    } else {
        "Keyboard layout; drag components, hold Shift for range selection, hold Space and drag to pan, or use position controls"
    };
    let focus_svg = svg.clone();
    use_effect(use_reactive!(|placement_active| {
        if !placement_active {
            return;
        }
        if let Some(element) = focus_svg.borrow().as_ref() {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = element.focus_with_options(&options);
        }
    }));
    let move_pointer = pointer::move_pointer_handler(pointer::MovePointerInput {
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
    let end_pointer = pointer::end_pointer_handler(pointer::EndPointerInput {
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
    let cancel_pointer = pointer::cancel_pointer_handler(pointer::CancelPointerInput {
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
    pointer::use_pointer_workspace_effect(
        active_workspace,
        runtime.clone(),
        drag.clone(),
        svg.clone(),
        space_down.clone(),
        render_scope.clone(),
        interaction_version,
    );
    let keyboard = keyboard::keyboard_handler(keyboard::KeyboardInput {
        runtime: runtime.clone(),
        drag: drag.clone(),
        svg: svg.clone(),
        render_scope: render_scope.clone(),
        space_down: space_down.clone(),
        mirrored_pair: mirrored_pair.clone(),
        placement: part_placement.clone(),
        matrix_placement: matrix_placement.clone(),
        canvas_interaction: canvas_interaction.clone(),
        snap_settings: layout_snap_settings,
        pending_splay_pick: pending_splay_origin_pick,
        pending_anchor_pick,
        transform_inspector: matrix_transform_inspector.clone(),
        interaction_version,
    });
    let key_up = keyboard::key_up_handler(canvas_navigation.clone());
    let start_pan = keyboard::start_pan_handler(keyboard::StartPanInput {
        navigation: canvas_navigation.clone(),
        render_scope: render_scope.clone(),
        render_generation,
        runtime: runtime.clone(),
        svg: svg.clone(),
        adapter: adapter.clone(),
        mirrored_pair: mirrored_pair.clone(),
        canvas_interaction: canvas_interaction.clone(),
        pending_splay_pick: pending_splay_origin_pick,
        transform_inspector: matrix_transform_inspector.clone(),
        layout_owner_for_pick: layout_owner.clone(),
        workspace,
        view_x,
        view_y,
        width,
        height,
    });
    let on_pcb_part_pointer_down =
        keyboard::on_pcb_part_pointer_down_handler(keyboard::OnPcbPartPointerDownInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            svg: svg.clone(),
            drag: drag.clone(),
            owner: layout_owner.clone(),
            scope: render_scope.clone(),
            accepted_token: snapshot.token,
            accepted_revision: snapshot.document.revision,
            generation: render_generation,
            navigation: canvas_navigation.clone(),
            space_down: space_down.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            canvas_interaction: canvas_interaction.clone(),
            workspace,
            view_x,
            view_y,
            width,
            height,
            instance_selection,
            layout_selection_kind,
        });
    let on_pcb_empty_hit = keyboard::on_pcb_empty_hit_handler(keyboard::OnPcbEmptyHitInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        scope: render_scope.clone(),
        generation: render_generation,
        token: snapshot.token,
        space_down: space_down.clone(),
        drag: drag.clone(),
        canvas_interaction: canvas_interaction.clone(),
        workspace,
        instance_selection,
    });
    let wheel = keyboard::wheel_handler(keyboard::WheelInput {
        runtime: runtime.clone(),
        svg: svg.clone(),
        scope: render_scope.clone(),
        adapter: adapter.clone(),
        render_generation,
        view_x,
        view_y,
        width,
        height,
        keymap_scale_ratio,
        min_x,
        max_x,
        min_y,
        max_y,
    });
    let undo = runtime.clone();
    let redo = runtime.clone();
    let retry = runtime.clone();
    let recover = runtime.clone();
    workspace_routing.select_tree.replace(Box::new(select_tree));
    workspace_routing.navigate.replace(Box::new(navigate));
    let nudge_tree = EventHandler::new(nudge_tree);
    let parts_select = EventHandler::new(on_parts_select);
    let keymap_select = EventHandler::new(on_keymap_select.clone());
    let keycaps_select = EventHandler::new(on_keycaps_select.clone());
    let keycaps_finding = EventHandler::new(on_keycaps_finding);
    let pcb_empty_hit = EventHandler::new(on_pcb_empty_hit);
    let pcb_part_hit = EventHandler::new(on_pcb_part_hit);
    let mounted_module_select = EventHandler::new(on_mounted_module_select);
    let pcb_part_pointer_down = EventHandler::new(on_pcb_part_pointer_down);
    let pcb_wiring_edit_board = EventHandler::new(on_pcb_wiring_edit_board);
    let on_layout_selection_kind_handler = EventHandler::new(on_layout_selection_kind);
    let layout_snap_intent = EventHandler::new(on_layout_snap_intent);
    let pcb_selection_kind = EventHandler::new(on_pcb_selection_kind);
    let pcb_snap_intent = EventHandler::new(on_pcb_snap_intent);
    let case_action = EventHandler::new(on_case_action);
    let case_display = EventHandler::new(on_case_display);
    let keymap_layer = EventHandler::new(on_keymap_layer);
    let keymap_export = EventHandler::new(on_keymap_export);
    let show_configured_board = EventHandler::new(on_show_configured_board);
    let open_geometry_scripts = {
        let mut workspace = workspace;
        let mut objects_open = objects_open;
        let mut inspect_open = inspect_open;
        let inspector_settings = inspector_panel_settings;
        EventHandler::new(move |_| {
            workspace.set("Layout");
            objects_open.set(false);
            inspect_open.set(true);
            setup_state.set_geometry_scripts_open(true);
            pin_inspector_on_desktop(inspector_settings);
        })
    };
    let toggle_footprints = EventHandler::new(move |_| {
        let mut footprints = layer_visibility.footprints;
        footprints.set(!footprints());
    });
    let retry_save = {
        let retry = retry.clone();
        EventHandler::new(move |_| {
            retry.submit(Event::RetrySave {
                operation_id: retry.operation(),
            });
        })
    };
    let download_pending = {
        let pending = runtime.clone();
        EventHandler::new(move |_| pending.download_pending_edits())
    };
    let recover_saved = {
        let recover = recover.clone();
        EventHandler::new(move |_| recover.recover_saved())
    };
    let canvas_end_pointer = EventHandler::new(end_pointer.clone());
    let canvas_cancel_pointer = EventHandler::new(cancel_pointer.clone());
    let canvas_keyboard = EventHandler::new(keyboard.clone());
    let canvas_wheel = EventHandler::new(wheel.clone());
    let board_setup = objects::use_board_setup(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        workspace_routing.navigate,
    );
    let shared_objects = workspace_composition::SharedObjectsInput {
        selected_context: adapter.selected_context,
        on_select: workspace_routing.select_tree,
        on_navigate: workspace_routing.navigate,
        on_nudge: nudge_tree,
        on_open_geometry_scripts: open_geometry_scripts,
        board_setup,
    };
    let case_scene = case_workspace::workspace_display_scene(runtime.cad_scene(), &render_scope);
    let pcb_wiring_source = current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(&model, scope) && instance_selection.is_current(&model)
        })
        .and_then(|scope| {
            let mut board_scope = scope.clone();
            board_scope.instance_id = None;
            pcb_wiring::PcbWiringSource::new(
                snapshot,
                &board_scope,
                scope,
                model.selected_part_ids.first().map(String::as_str),
                runtime.electrical_preview_executor_epoch(),
                (adapter.generation)(),
            )
        });
    let pcb_wiring_mode_actions = pcb_wiring::use_board_wiring_mode_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let pcb_wiring_pin_actions = pcb_wiring::use_pcb_wiring_pin_edits(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let pcb_wiring_protected_remap_actions = pcb_wiring::use_protected_remap_review(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
    );
    let pcb_wiring_apply_actions = pcb_wiring::use_board_wiring_apply(
        runtime.clone(),
        version,
        workspace,
        adapter.generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
    );
    let zmk_firmware_export_panel = use_export_panel_input(
        runtime.clone(),
        workspace,
        adapter.generation,
        pcb_wiring_source.clone(),
        pcb_wiring_mount.resolution_signal,
        instance_selection,
    );
    let case_selected_body_id = case_selection
        .body
        .read()
        .as_ref()
        .filter(|selection| selection.scope == render_scope)
        .map(|selection| selection.body_id.clone());
    let case_selected_layer_id = case_selection.layer_id(&render_scope);
    let case_selected_context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| selected.scope == render_scope);
    let on_browse_parts = EventHandler::new(move |_| {
        browse_parts_workspace(
            workspace,
            objects_open,
            objects_panel_settings,
            is_compact_viewport(),
        );
    });
    let on_empty_board_matrix = use_callback({
        let runtime = runtime.clone();
        let owner_scope = render_scope.clone();
        let owner_token = snapshot.token;
        let owner_revision = snapshot.document.revision;
        let generation = adapter.generation;
        let mut workspace = workspace;
        let on_open = matrix_setup.on_open;
        move |_| {
            let model = runtime.model();
            if workspace() != active_workspace
                || generation() != render_generation
                || runtime.scope().as_ref() != Some(&owner_scope)
                || model.lifecycle != Lifecycle::Ready
                || model.durability
                    != (Durability::Saved {
                        revision: owner_revision,
                    })
                || model.gesture.is_some()
                || model.display_preview.is_some()
                || !model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == owner_token && accepted.document.revision == owner_revision
                })
            {
                return;
            }
            workspace.set("Layout");
            let mut layout_assembly_3d = layout_assembly_3d;
            layout_assembly_3d.set(false);
            on_open.call(());
        }
    });
    let objects_input = match active_workspace {
        "PCB" => {
            let pcb_add_owner = LayoutOwnerIdentity {
                scope: Some(render_scope.clone()),
                token: Some(snapshot.token),
                revision: Some(snapshot.document.revision),
                generation: render_generation,
                workspace: "PCB",
            };
            let pcb_add_is_current =
                pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &pcb_add_owner);
            let mut pcb_shared_objects = shared_objects;
            pcb_shared_objects.on_select = pcb_add_outline_select_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_select,
            );
            pcb_shared_objects.on_open_geometry_scripts = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                pcb_shared_objects.on_open_geometry_scripts,
            );
            let mut pcb_matrix_setup = matrix_setup.clone();
            pcb_matrix_setup.can_open = pcb_add_is_current
                && pcb_matrix_setup.projection.is_none()
                && canvas_interaction.current().is_none();
            pcb_matrix_setup.on_open = pcb_add_layout_open_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                pcb_add_owner.clone(),
                layout_assembly_3d,
                matrix_setup.on_open,
            );
            workspace_composition::WorkspaceObjectsInput::Pcb(Box::new(
                pcb_workspace::ObjectsInput {
                    shared: pcb_shared_objects,
                    matrix_setup: pcb_matrix_setup,
                    on_place_component: part_placement.on_place_component,
                    layout_target,
                    parts_query,
                    on_browse_parts,
                    placement_error: matrix_placement
                        .error
                        .clone()
                        .or_else(|| part_placement.error.clone()),
                },
            ))
        }
        "Keymap" => workspace_composition::WorkspaceObjectsInput::Keymap(
            keymap_workspace::NavigationInput {
                view: keymap_view.clone(),
                scope: render_scope.clone(),
                active_layer_id: keymap_layer_id(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                layer_actions: keymap::LayerActions {
                    enabled: layer_actions.enabled,
                    feedback: layer_actions.feedback.clone(),
                    on_operation: layer_actions.on_operation,
                },
                on_layer: keymap_layer,
                on_select_key: keymap_select,
            },
        ),
        "Keycaps" => workspace_composition::WorkspaceObjectsInput::Keycaps(
            keycaps_workspace::NavigationInput {
                shared: shared_objects,
                view: keycaps_view.clone(),
                settings_actions: keycaps_settings_actions.clone(),
            },
        ),
        "Case" => workspace_composition::WorkspaceObjectsInput::Case(Box::new(
            case_workspace::ObjectsInput {
                model: &model,
                scope: current_scope.clone(),
                instance_scope_pending,
                scene: case_scene.clone(),
                selected_context: adapter.selected_context,
                selected_body_id: case_selected_body_id.clone(),
                selected_layer_id: case_selected_layer_id.clone(),
                case_selection,
                expanded: case_tree_expanded,
                on_action: case_action,
                on_select: workspace_routing.select_tree,
                on_display: case_display,
            },
        )),
        "Parts" => {
            workspace_composition::WorkspaceObjectsInput::Parts(parts_workspace::ObjectsInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                scope_generation: adapter.generation,
                workspace,
                query: parts_query,
                selected: parts_selection,
                on_select: parts_select,
            })
        }
        _ => workspace_composition::WorkspaceObjectsInput::Layout(Box::new(
            layout_workspace::ObjectsInput {
                shared: shared_objects,
                matrix_setup: matrix_setup.clone(),
                matrix_inspector: matrix_inspector.clone(),
                pair_created: pair_created_selection,
                on_place_component: part_placement.on_place_component,
                layout_target,
                parts_query,
                on_browse_parts,
                placement_error: matrix_placement
                    .error
                    .clone()
                    .or_else(|| part_placement.error.clone()),
                add_object_menu_open: layout_state.add_object_menu_open(),
            },
        )),
    };
    let on_layout_view_mode = layout_view_mode_handler(
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
    );
    let retained_toolbar_cell = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
    let toolbar_selection_availability = use_memo(use_reactive(
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
    ))();
    let toolbar_input = match active_workspace {
        "Layout" => {
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
                let mut objects_open = objects_open;
                let mut inspect_open = inspect_open;
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
                let on_selection_kind = on_layout_selection_kind_handler;
                let matrix_ids: BTreeSet<_> =
                    matrices.iter().map(|matrix| matrix.id.clone()).collect();
                move |tool| {
                    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
                        return;
                    }
                    let Some(selected) =
                        adapter.selected_context.read().clone().filter(|selected| {
                            owner.scope.as_ref() == Some(&selected.scope)
                                && selection::context_is_current(
                                    &runtime.model(),
                                    &selected.scope,
                                    &selected.context,
                                )
                        })
                    else {
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
                    .and_then(|projection| match &projection.fields {
                        boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields::Matrix {
                            rotation_followed,
                            ..
                        } => Some(*rotation_followed),
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
                on_selection_kind: on_layout_selection_kind_handler,
                on_show_properties,
                on_transform_tool,
            };
            let footprints_pressed = (layer_visibility.footprints)()
                && !(layer_visibility.hidden)().contains("Footprints");
            workspace_composition::WorkspaceToolbarInput::Layout(Box::new(
                layout_workspace::ToolbarInput {
                    save_failure: match &model.durability {
                        Durability::Failed { reason, .. } => Some(reason.clone()),
                        _ => None,
                    },
                    recovery_required: model.lifecycle
                        == boardstudio_application::Lifecycle::RecoveryRequired,
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
                    on_selection_kind: on_layout_selection_kind_handler,
                    on_snap_intent: layout_snap_intent,
                    show_relationships: true,
                    has_selection_context: matrix_transform_inspector.projection.is_some()
                        || component_inspector.is_some(),
                    on_show_relationships: EventHandler::new(move |()| {
                        inspect_open.set(true);
                    }),
                },
            ))
        }
        "PCB" => {
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
                let mut objects_open = objects_open;
                let mut inspect_open = inspect_open;
                move |_| {
                    if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner)
                        || !properties_available
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
            workspace_composition::WorkspaceToolbarInput::Pcb(Box::new(
                pcb_workspace::ToolbarInput {
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
                },
            ))
        }
        "Keymap" => {
            let on_view_mode = design_consumer_view_mode_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                layout_owner.clone(),
                layout_assembly_3d,
            );
            workspace_composition::WorkspaceToolbarInput::Keymap(
                shared_viewer::DesignViewToolbarProps {
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
                },
            )
        }
        "Keycaps" => {
            let on_view_mode = design_consumer_view_mode_handler(
                runtime.clone(),
                workspace,
                adapter.clone(),
                layout_owner.clone(),
                layout_assembly_3d,
            );
            workspace_composition::WorkspaceToolbarInput::Keycaps(
                shared_viewer::DesignViewToolbarProps {
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
                },
            )
        }
        "Case" => workspace_composition::WorkspaceToolbarInput::Case,
        "Parts" => workspace_composition::WorkspaceToolbarInput::Parts,
        "Export" => workspace_composition::WorkspaceToolbarInput::Export,
        _ => workspace_composition::WorkspaceToolbarInput::Other,
    };
    let canvas_handlers = workspace_composition::CanvasEventHandlers {
        mount,
        start_pan: EventHandler::new(start_pan.clone()),
        move_pointer: EventHandler::new(move_pointer.clone()),
        end_pointer: canvas_end_pointer,
        cancel_pointer: canvas_cancel_pointer,
        keyboard: canvas_keyboard,
        key_up: EventHandler::new(key_up.clone()),
        wheel: canvas_wheel,
    };
    let canvas_input = match active_workspace {
        "PCB" => Some(workspace_composition::WorkspaceCanvasInput::Pcb(Box::new(
            pcb_workspace::CanvasInput {
                snapshot: snapshot.clone(),
                scope: render_scope.clone(),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.clone(),
                generation: render_generation,
                handlers: canvas_handlers,
                on_empty_hit: pcb_empty_hit,
                on_part_hit: pcb_part_hit,
                on_part_pointer_down: pcb_part_pointer_down,
                on_module_select: mounted_module_select,
            },
        ))),
        "Keymap" => Some(workspace_composition::WorkspaceCanvasInput::Keymap(
            Box::new(keymap_workspace::CanvasInput {
                view: keymap_view.clone(),
                contours: keymap_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: keymap_select,
            }),
        )),
        "Keycaps" => Some(workspace_composition::WorkspaceCanvasInput::Keycaps(
            Box::new(keycaps_workspace::CanvasInput {
                view: keycaps_view.clone(),
                contours: keycaps_contours
                    .clone()
                    .unwrap_or_else(|| Rc::<[Contour]>::from(Vec::new())),
                view_box: view_box.clone(),
                selected_ids: model.selected_part_ids.iter().cloned().collect(),
                handlers: canvas_handlers,
                on_select_key: keycaps_select,
            }),
        )),
        "Case" => Some(workspace_composition::WorkspaceCanvasInput::Case(Box::new(
            case_workspace::CanvasInput {
                generation_ready: mechanical_settings.generation_ready,
                instance_scope_pending,
                mechanical_settings: mechanical_settings.props.clone(),
            },
        ))),
        "Parts" => Some(workspace_composition::WorkspaceCanvasInput::Parts(
            parts_workspace::CanvasInput {
                controller_back: part_placement.controller_back,
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
            },
        )),
        "Layout" | "Export" => None,
        _ => Some(workspace_composition::WorkspaceCanvasInput::Other(
            workspace_composition::PlaceholderInput {
                workspace,
                name: active_workspace,
                message: "Parts library editing is not available yet in the Rust interface.",
            },
        )),
    };
    let component_inspector_pending_edits = layout_state.component_inspector_edits();
    let (
        component_inspector_x,
        component_inspector_y,
        component_inspector_margin,
        component_inspector_error,
    ) = layout_state.component_inspector_drafts();
    let (
        component_inspector_x_failure,
        component_inspector_y_failure,
        component_inspector_margin_failure,
    ) = layout_state.component_inspector_field_failures();
    let (
        component_inspector_layout_pending,
        component_inspector_constraint_pending,
        component_inspector_remove_pending,
        component_inspector_mirrored_half_pending,
    ) = layout_state.component_inspector_pending_controls();
    let (companion_placement_pending, mut companion_placement_error, companion_placement_edits) =
        layout_state.companion_placement_controls();
    let on_component_inspector_action = EventHandler::new({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let lifetime = layout_component_inspector_lifetime.clone();
        move |action: inspector::LayoutComponentInspectorAction| {
            dispatch_layout_component_inspector_action(
                &runtime,
                &adapter,
                &owner,
                &lifetime,
                (workspace, inspect_open),
                component_inspector_pending_edits,
                action,
            )
        }
    });
    let on_companion_placement_action = EventHandler::new({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner_workspace = workspace;
        move |action: inspector::CompanionPlacementAction| {
            if !companion_placement_owner_is_current(
                &runtime,
                owner_workspace,
                &adapter,
                &action.owner,
                &action.pair_id,
                &action.targets,
            ) {
                companion_placement_error.set(Some(
                    "The selection changed before this edit ran; placement was not changed.".into(),
                ));
                return;
            }
            companion_placement_error.set(None);
            submit_layout_component_edit(
                &runtime,
                companion_placement_edits,
                layout_component_edits::InspectorField::CompanionPlacement,
                layout_component_edits::companion_placement_resolver(
                    action.pair_id,
                    action.targets,
                    action.mode,
                    action.tweak,
                ),
                None,
            );
        }
    });
    let inspector_input = match active_workspace {
        "Keymap" => workspace_composition::WorkspaceInspectorInput::Keymap(Box::new(
            keymap_workspace::InspectorInput {
                view: keymap_view.clone(),
                scope: render_scope.clone(),
                layer_actions,
                active_layer_id: keymap_layer_id(),
                selected_key_id: model.selected_part_ids.first().cloned(),
                on_layer: keymap_layer,
                on_export: keymap_export,
                firmware_export_enabled: keymap_view
                    .as_ref()
                    .is_some_and(|view| !view.keys.is_empty())
                    || binding_actions
                        .encoder_projection
                        .as_ref()
                        .is_some_and(|projection| !projection.rows.is_empty()),
                on_select_key: keymap_select,
                binding_actions,
                macro_actions,
                admission_token: snapshot.token,
                admission_revision: snapshot.document.revision,
                scope_generation: (adapter.generation)(),
            },
        )),
        "Case" => workspace_composition::WorkspaceInspectorInput::Case(Box::new(
            case_workspace::InspectorInput {
                physical_setup: physical_setup_mount.clone(),
                mechanical_settings: mechanical_settings.clone(),
                instance_scope_pending,
                scope: current_scope.clone(),
                scene: case_scene,
                case_selection,
                selected_layer_id: case_selected_layer_id,
                selected_context: case_selected_context,
                selected_part_summary: case_workspace::selected_part_summary(
                    &model,
                    selected_tree_context.as_ref(),
                ),
                on_show_configured_board: show_configured_board,
                on_display: case_display,
            },
        )),
        "Parts" => workspace_composition::WorkspaceInspectorInput::Parts(Box::new(
            parts_workspace::InspectorInput {
                snapshot: snapshot.clone(),
                scope: current_scope.clone(),
                query: parts_query,
                selected: parts_selection,
                selected_context: adapter.selected_context,
                on_place_controller: part_placement.on_place_controller,
                on_place_component: part_placement.on_place_component,
                controller_placement_enabled: part_placement.controller_placement_enabled,
                placement_busy: part_placement.busy || matrix_placement.busy,
                placement_error: matrix_placement
                    .error
                    .clone()
                    .or_else(|| part_placement.error.clone()),
                layout_target,
                on_place_assembly: on_place_matrix_assembly,
                on_board_placed: EventHandler::new(move |_| {
                    let mut selection_kind = layout_selection_kind;
                    selection_kind.set(objects::LayoutSelectionKind::Part);
                }),
                on_open_module_placement,
                on_module_attached,
            },
        )),
        "PCB" => {
            let selected_module = selected_tree_context.as_ref().and_then(|selected| {
                if selected.scope != render_scope {
                    return None;
                }
                match &selected.context {
                    objects::TreeContext::MountedModule {
                        board_id,
                        module_id,
                    } if board_id == &render_scope.board_id => Some(module_id.clone()),
                    _ => None,
                }
            });
            if let Some(module_id) = selected_module {
                workspace_composition::WorkspaceInspectorInput::PcbModule(Box::new(
                    pcb_module_inspector::InspectorInput {
                        runtime: runtime.clone(),
                        snapshot: snapshot.clone(),
                        scope: render_scope.clone(),
                        module_id,
                        selected_context: adapter.selected_context,
                        on_place_component: part_placement.on_place_component,
                    },
                ))
            } else {
                workspace_composition::WorkspaceInspectorInput::Pcb(pcb_wiring_source.map(
                    |source| {
                        let firmware_position_projection = pcb_wiring::firmware_position_projection(
                            &source,
                            render_generation,
                            &pcb_wiring_mount.resolution,
                        );
                        let firmware_feedback =
                            firmware_position_actions
                                .feedback
                                .clone()
                                .filter(|feedback| {
                                    feedback.target.is_visible(
                                        &source.ui_scope,
                                        render_generation,
                                        &firmware_position_projection,
                                    )
                                });
                        let firmware_controls = rsx! {
                            firmware_positions::FirmwareKeymapPanel {
                                projection: firmware_position_projection.clone(),
                                feedback: firmware_feedback.clone(),
                                editable: firmware_position_actions.editable,
                                on_change: firmware_position_actions.on_edit,
                            }
                        };
                        Box::new(pcb_wiring::PcbWiringInspectorProps {
                            board_overview: true,
                            source,
                            resolution: pcb_wiring_mount.resolution.clone(),
                            firmware_positions: firmware_position_projection,
                            firmware_feedback,
                            firmware_controls,
                            part_net_actions: pcb_part_net_actions.clone(),
                            part_input_actions: part_input_actions.clone(),
                            on_firmware_edit: firmware_position_actions.on_edit,
                            on_resolve: pcb_wiring_mount.on_resolve,
                            on_choose_controller: part_placement.on_choose_controller,
                            on_edit_board_wiring: pcb_wiring_edit_board,
                            mode_actions: pcb_wiring_mode_actions.clone(),
                            pin_actions: pcb_wiring_pin_actions.clone(),
                            apply_actions: pcb_wiring_apply_actions.clone(),
                            protected_remap_actions: pcb_wiring_protected_remap_actions.clone(),
                        })
                    },
                ))
            }
        }
        "Keycaps" => {
            let selected_key_id = model.selected_part_ids.first().cloned();
            let mechanical_layer_ids = current_scope
                .as_ref()
                .zip(model.accepted.as_ref())
                .and_then(|(scope, snapshot)| current_case_layer_ids(&runtime, scope, snapshot))
                .unwrap_or_else(|| Rc::from([]));
            let settings_editor = keycaps_view.as_deref().and_then(|view| {
                let selected = keycaps_settings::project_selected_key(
                    &document,
                    view,
                    selected_key_id.as_deref(),
                )?;
                keycaps_settings_actions
                    .clone()
                    .map(|actions| (selected, actions))
            });
            workspace_composition::WorkspaceInspectorInput::Keycaps(Box::new(
                keycaps_workspace::InspectorInput {
                    view: keycaps_view.clone(),
                    document: Rc::new((*document).clone()),
                    selected_key_id,
                    on_select_key: keycaps_select,
                    settings_editor,
                    settings_actions: keycaps_settings_actions.clone(),
                    fit_state: keycaps_fit_state.state.clone(),
                    mechanical_layer_ids,
                    fit_retry: keycaps_fit_state.on_retry,
                    fit_navigate: keycaps_finding,
                },
            ))
        }
        _ => workspace_composition::WorkspaceInspectorInput::Layout(Box::new(
            layout_workspace::InspectorInput {
                geometry_scripts_open: geometry_scripts_open(),
                on_close_geometry_scripts: EventHandler::new(move |()| {
                    setup_state.set_geometry_scripts_open(false)
                }),
                context_title: context_summary
                    .as_ref()
                    .map(|summary| summary.title.clone())
                    .or_else(|| {
                        board_inspector_projection
                            .as_ref()
                            .map(|board| board.board_name.clone())
                    }),
                context_detail: context_summary
                    .as_ref()
                    .and_then(|summary| summary.detail.clone())
                    .or_else(|| {
                        board_inspector_projection
                            .as_ref()
                            .map(|_| "Layout".to_owned())
                    }),
                show_position_inspector,
                component_inspector: component_inspector.clone(),
                component_inspector_pending_edits,
                component_inspector_x,
                component_inspector_y,
                component_inspector_margin,
                component_inspector_error,
                component_inspector_x_failure,
                component_inspector_y_failure,
                component_inspector_margin_failure,
                component_inspector_layout_pending,
                component_inspector_constraint_pending,
                component_inspector_remove_pending,
                component_inspector_mirrored_half_pending,
                companion_placement: companion_placement.clone(),
                companion_placement_pending,
                companion_placement_error,
                companion_placement_edits,
                on_companion_placement_action,
                on_component_inspector_action,
                matrix_inspector,
                key_size,
                matrix_transform_inspector: matrix_transform_inspector.clone(),
                on_pick_splay_origin: {
                    let mount = matrix_transform_inspector.clone();
                    let mut pending = pending_splay_origin_pick;
                    let mut anchor_pending = pending_anchor_pick;
                    EventHandler::new(move |()| {
                        let Some(projection) = mount.projection.as_ref() else {
                            return;
                        };
                        if matches!(
                            &projection.owner.context,
                            objects::TreeContext::Column { .. }
                        ) && mount.editable
                        {
                            anchor_pending.set(None);
                            pending.set(Some(projection.owner.clone()));
                        }
                    })
                },
                on_pick_anchor: {
                    let mount = matrix_transform_inspector.clone();
                    let mut pending = pending_anchor_pick;
                    let mut splay_pending = pending_splay_origin_pick;
                    EventHandler::new(move |()| {
                        let Some(projection) = mount.projection.as_ref() else {
                            return;
                        };
                        if matches!(
                            &projection.owner.context,
                            objects::TreeContext::Matrix { .. }
                        ) && mount.editable
                        {
                            splay_pending.set(None);
                            mount.clear_aggregate_pick();
                            pending.set(Some(projection.owner.clone()));
                        }
                    })
                },
                anchor_pick_pending: pending_anchor_pick().is_some(),
                on_cancel_anchor_pick: {
                    let mount = matrix_transform_inspector.clone();
                    let mut pending = pending_anchor_pick;
                    EventHandler::new(move |()| {
                        mount.clear_aggregate_pick();
                        pending.set(None);
                    })
                },
                splay_origin_pick_pending: pending_splay_origin_pick().is_some(),
                on_cancel_splay_origin_pick: {
                    let mut pending = pending_splay_origin_pick;
                    EventHandler::new(move |()| pending.set(None))
                },
                inspector_tab: layout_state.inspector_tab(),
                matrix_relations: matrix_context_relations(&model, selected_tree_context.as_ref()),
                relation_target: {
                    let runtime = runtime.clone();
                    Rc::new(move |request| relation_target_request(&runtime, request))
                },
                relation_scope: selected_tree_context
                    .as_ref()
                    .map(|selected| selected.scope.clone()),
                on_navigate_anchor_part: {
                    let runtime = runtime.clone();
                    let scope = selected_tree_context
                        .as_ref()
                        .map(|selected| selected.scope.clone());
                    let select = workspace_routing.select_tree;
                    EventHandler::new(move |context: objects::TreeContext| {
                        let Some(scope) = scope.clone() else {
                            return;
                        };
                        let request = objects::TreeSelectRequest {
                            scope,
                            context,
                            mode: boardstudio_application::SelectionMode::Replace,
                            outline_action: None,
                        };
                        // A part key removed since the section was projected:
                        // do nothing; the next accepted revision re-projects.
                        let Some(request) = relation_target_request(&runtime, request) else {
                            return;
                        };
                        select.call(request);
                    })
                },
                on_select_context: workspace_routing.select_tree,
                outline_inspector: outline_inspector.clone().map(Box::new),
                findings_return_available: (layout_findings.return_target)().as_ref().is_some_and(
                    |target| {
                        layout_finding_return_is_current(
                            &model,
                            &layout_owner,
                            selected_tree_context.as_ref(),
                            target,
                        )
                    },
                ),
                on_findings_return: on_return_from_layout_finding,
                board_inspector_mount: board_inspector.clone(),
                findings_page: Some(layout_findings::InspectorMount {
                    open: (layout_findings.open)(),
                    document: Rc::new(document.as_ref().clone()),
                    findings: findings.clone(),
                    source: layout_findings::Source {
                        workspace: active_workspace,
                        scope: render_scope.clone(),
                        token: snapshot.token,
                        revision: snapshot.document.revision,
                        generation: render_generation,
                    },
                    on_close: on_close_layout_findings,
                    on_navigate: on_layout_finding,
                }),
            },
        )),
    };
    let _board_reference_editor = if matches!(active_workspace, "Layout" | "Case") {
        let reference = document
            .board_references
            .iter()
            .find(|reference| reference.board_id == render_scope.board_id)
            .cloned();
        let owner = LayoutOwnerIdentity {
            scope: Some(render_scope.clone()),
            token: Some(snapshot.token),
            revision: Some(snapshot.document.revision),
            generation: render_generation,
            workspace: active_workspace,
        };
        let assets = document.assets.clone();
        let editable = boardstudio_web_pcb::board_reference_owner::board_reference_owner_is_current(
            &runtime, workspace, &adapter, &owner,
        );
        Some((reference, assets, editable, owner))
    } else {
        None
    };
    let selected_bridge = selected_tree_context.as_ref().and_then(|selected| {
        let objects::TreeContext::Bridge {
            board_id,
            bridge_id,
        } = &selected.context
        else {
            return None;
        };
        snapshot
            .scene
            .board_outline_scenes
            .iter()
            .find(|scene| scene.board_id == *board_id)
            .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == *bridge_id))
    });
    let active_part_position = model.selected_part_ids.first().and_then(|selected_id| {
        visible
            .iter()
            .find(|part| part.id == *selected_id)
            .map(|part| {
                scene
                    .transforms
                    .iter()
                    .find(|transform| transform.id == part.id)
                    .map(|transform| transform.pose.at)
                    .unwrap_or(part.pose.at)
            })
    });
    let controller_guide_hidden = part_placement.projection.is_some()
        || part_placement.busy
        || matrix_placement.placement.is_some()
        || matrix_placement.busy
        || (active_workspace == "Parts"
            && parts_query().trim() == "controller"
            && guide_preferences()
                .as_ref()
                .is_some_and(|preferences| preferences.current_stage == SetupGuideStage::Wiring));
    let guide = guide_preferences().filter(|preferences| {
        preferences.project_id == document.id
            && preferences.open
            // Keep the guide visible while matrix setup overlays the Layout canvas.
            && !(active_workspace == "Layout"
                && (matrix_placement.placement.is_some()
                    || matrix_placement.busy
                    || mirrored_pair.form.is_some()
                    || mirrored_pair.placement.is_some()))
            && !controller_guide_hidden
    });
    let has_matrix = !matrices.is_empty();
    let setup_guide_active = guide.is_some();
    let mut setup_wiring_error = use_signal(|| None::<String>);
    use_effect(use_reactive(
        (&guide_preferences(), &pcb_wiring_mount.resolution),
        {
            let runtime = runtime.clone();
            move |(guide, resolution)| {
                if !guide.as_ref().is_some_and(|guide| {
                    guide.open && guide.current_stage == SetupGuideStage::Wiring
                }) {
                    return;
                }
                let pcb_wiring::PcbWiringResolution::Current { identity, plan } = resolution else {
                    return;
                };
                let Some(accepted) = runtime.model().accepted else {
                    return;
                };
                if identity.token != accepted.token
                    || plan.controller_part_id.is_none()
                    || plan
                        .diagnostics
                        .iter()
                        .any(|finding| finding.severity == "error")
                {
                    return;
                }
                let mut proposal = (*accepted.document).clone();
                if let Err(error) = boardstudio_core::electrical::materialize(&mut proposal, &plan)
                {
                    setup_wiring_error.set(Some(error));
                    return;
                }
                setup_wiring_error.set(None);
                if proposal == *accepted.document {
                    return;
                }
                runtime.submit(boardstudio_application::Event::ResolveEdit {
                    operation_id: runtime.operation(),
                    label: "Setup wiring".into(),
                    resolver: boardstudio_application::EditResolver::new(
                        "setup-wiring",
                        move |current| {
                            if current.token != identity.token {
                                return boardstudio_application::Resolution::Retire(
                                    "The board changed while wiring was being prepared.".into(),
                                );
                            }
                            boardstudio_application::Resolution::submit(
                                vec![identity.scope.board_id.clone()],
                                EditOperation::ReplaceDocument {
                                    document: Box::new(proposal.clone()),
                                },
                            )
                        },
                    ),
                });
            }
        },
    ));
    let matrix_count = matrices.len();
    let placing_matrix = matrix_placement.placement.is_some() || matrix_placement.busy;
    let auto_open_matrix = use_reactive(
        (
            &guide_preferences(),
            &workspace(),
            &matrix_setup.can_open,
            &matrix_count,
            &placing_matrix,
        ),
        {
            let project_id = document.id.clone();
            let on_open = matrix_setup.on_open;
            move |(preferences, active_workspace, can_open, matrix_count, placing)| {
                if active_workspace == "Layout"
                    && can_open
                    && matrix_count == 0
                    && !placing
                    && preferences.as_ref().is_some_and(|preferences| {
                        preferences.project_id == project_id
                            && preferences.open
                            && preferences.current_stage == SetupGuideStage::Layout
                    })
                {
                    on_open.call(());
                }
            }
        },
    );
    use_effect(auto_open_matrix);
    let inline_matrix_setup = if guide.is_some() {
        matrix_setup.projection.clone().map(|mut projection| {
            projection.can_cancel &= has_matrix;
            rsx! {
                objects::MatrixSetup {
                    projection,
                    on_cancel: matrix_setup.on_cancel,
                    on_create: matrix_setup.on_create,
                }
            }
        })
    } else {
        None
    };
    let inline_case_basics = guide
        .as_ref()
        .and_then(|_| mechanical_settings.props.clone())
        .map(|settings| {
            rsx! {
                boardstudio_web_case::mechanical_settings::MechanicalSetupBasics { settings }
            }
        });
    let inline_controller_picker = guide
        .as_ref()
        .filter(|preferences| {
            preferences.current_stage == SetupGuideStage::Wiring
        })
        .map(|_| {
            rsx! {
                div {
                    parts::ControllerSetupPicker {
                        snapshot: snapshot.clone(),
                        scope: current_scope.clone(),
                        selected: parts_selection,
                        on_place: part_placement.on_place_controller,
                        enabled: part_placement.controller_placement_enabled,
                        busy: part_placement.busy || matrix_placement.busy,
                        error: setup_wiring_error().or_else(|| matrix_placement.error.clone()).or_else(|| part_placement.error.clone()),
                    }
                }
            }
        });
    let show_empty_board = guide.is_none()
        && matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps")
        && !layout_assembly_3d()
        && visible.is_empty()
        && matrices.is_empty()
        && !scene.board_contours.iter().any(|contours| {
            contours.board_id == model.active_board_id && !contours.contours.is_empty()
        })
        && mirrored_pair.placement.is_none()
        && part_placement.projection.is_none()
        && !part_placement.busy;
    let show_empty_board =
        show_empty_board && matrix_placement.placement.is_none() && !matrix_placement.busy;
    let outline_pitch = {
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &layout_owner);
        matrix_snap_parameters(&model, &render_scope, &adapter, retained.as_ref())
            .0
            .unwrap_or(Vec2 { x: 19.05, y: 19.05 })
    };
    let outline_snap_origins = use_memo(use_reactive((&accepted_canvas_source,), |(source,)| {
        Rc::new(layout_canvas::snap_origins(
            &source.document,
            &source.board_id,
            &source.scene,
        ))
    }))();
    let outline_snap_settings = layout_snap_settings.read().clone();
    let outline_grid_active = outline_inspector
        .as_ref()
        .is_some_and(|projection| (projection.editing_points)());
    let canvas_grid = objects::layout_canvas_grid_style(
        width,
        zoom_surface_size().0,
        outline_pitch.x,
        outline_snap_settings.snap_fraction,
        outline_grid_active,
    );
    let outline_overlay_key = outline_inspector
        .as_ref()
        .map(outline_lifecycle::OutlineInspectorProjection::canvas_edit_key);
    let name_value = guide_name_draft()
        .filter(|(project_id, _)| project_id == &document.id)
        .map(|(_, name)| name)
        .unwrap_or_else(|| document.name.clone());
    let on_name_change = guide_name_edit.on_change;
    let on_name_commit = guide_name_edit.on_commit;
    let guide_runtime = runtime.clone();
    let mut guide_adapter = adapter.clone();
    let setup_state_for_stage = setup_state;
    let guide_workspace_for_stage = workspace;
    let guide_project_id = document.id.clone();
    let on_stage_change = move |stage: SetupGuideStage| {
        if !setup_state_for_stage.update_guide_stage(&guide_project_id, stage) {
            return;
        }
        setup_guide::activate_stage(
            stage,
            guide_workspace_for_stage,
            requested_workspace_panel,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        guide_adapter.selected_context.set(None);
        guide_adapter.anchor_scope.set(None);
        guide_runtime.submit(Event::SelectParts {
            operation_id: guide_runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    };
    let mut guide_workspace_for_action = workspace;
    let mut guide_adapter_for_action = adapter.clone();
    let guide_runtime_for_action = runtime.clone();
    let settings_project_id = document.id.clone();
    let mut full_case_settings = use_signal(|| false);
    let on_open_workspace = move |target: &'static str| {
        if target == "Case" {
            full_case_settings.set(true);
            guide_workspace_for_action.set("Case");
            return;
        }
        guide_workspace_for_action.set(target);
        setup_guide::reveal_panels(
            crate::setup_guide_state::GuideReveal::Settings,
            objects_open,
            inspect_open,
            objects_panel_settings,
            inspector_panel_settings,
        );
        let focus_runtime = guide_runtime_for_action.clone();
        let focus_project_id = settings_project_id.clone();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if *guide_workspace_for_action.peek() == target
                && focus_runtime
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.document.id == focus_project_id)
            {
                setup_guide::focus_settings(target);
            }
        });
        guide_adapter_for_action.selected_context.set(None);
        guide_adapter_for_action.anchor_scope.set(None);
        guide_runtime_for_action.submit(Event::SelectParts {
            operation_id: guide_runtime_for_action.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    };
    let setup_state_for_dismiss = setup_state;
    let guide_project_id_for_dismiss = document.id.clone();
    let on_dismiss_guide = move |_| {
        setup_state_for_dismiss.dismiss_guide(&guide_project_id_for_dismiss);
    };
    let setup_state_for_finish = setup_state;
    let guide_project_id_for_finish = document.id.clone();
    let mut guide_workspace_for_finish = workspace;
    let on_finish_guide = move |_| {
        guide_workspace_for_finish.set("Layout");
        setup_state_for_finish.dismiss_guide(&guide_project_id_for_finish);
    };
    let (guide_stage_readiness, guide_stage_detail) = guide
        .as_ref()
        .map(|preferences| {
            let guide_statuses = setup_guide::stage_statuses(
                &document,
                &model.active_board_id,
                &snapshot.scene,
                match &pcb_wiring_mount.resolution {
                    pcb_wiring::PcbWiringResolution::Current { identity, plan }
                        if identity.scope.document_id == document.id
                            && identity.scope.board_id == model.active_board_id
                            && identity.scope.instance_id.is_none()
                            && identity.token == snapshot.token
                            && identity.revision == document.revision =>
                    {
                        Some(plan.as_ref())
                    }
                    _ => None,
                },
            );
            let guide_stage_index = match preferences.current_stage {
                SetupGuideStage::Project => 0,
                SetupGuideStage::Layout => 1,
                SetupGuideStage::Wiring => 2,
                SetupGuideStage::Case => 3,
                SetupGuideStage::Review => 4,
            };
            let guide_stage_detail = guide_statuses[guide_stage_index].detail.clone();
            (
                guide_statuses.map(|status| status.ready),
                guide_stage_detail,
            )
        })
        .unwrap_or_default();
    let canvas_transform_menu = match &toolbar_input {
        workspace_composition::WorkspaceToolbarInput::Layout(input) => {
            Some(input.transform.clone())
        }
        _ => None,
    };
    let selection_geometry_anchor = (!layout_assembly_3d() && active_workspace == "Layout")
        .then(|| {
            let points: Vec<_> = visible
                .iter()
                .filter(|part| model.selected_part_ids.contains(&part.id))
                .map(|part| {
                    scene
                        .transforms
                        .iter()
                        .find(|t| t.id == part.id)
                        .map_or(part.pose.at, |t| t.pose.at)
                })
                .collect();
            if points.is_empty() {
                return None;
            }
            let scale = svg.borrow().as_ref().map_or(1.0, |_surface| {
                let (surface_width, surface_height) = zoom_surface_size();
                (width / surface_width.max(1.0)).max(height / surface_height.max(1.0))
            });
            let x = (points.iter().map(|p| p.x).sum::<f64>() / points.len() as f64).clamp(
                view_x + 60.0 * scale,
                (view_x + width - 60.0 * scale).max(view_x + 60.0 * scale),
            );
            let y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min) - 12.0 - 6.0 * scale;
            Some((x, y, scale))
        })
        .flatten();
    let matrix_pointer_down =
        pointer::matrix_pointer_down_handler(pointer::MatrixPointerDownInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            svg: svg.clone(),
            space_down: space_down.clone(),
            scope: render_scope.clone(),
            generation: render_generation,
            range_ids: visible_ids.clone(),
            selection_kind: layout_selection_kind,
            owner: layout_owner.clone(),
            tree_cell_anchor: tree_cell_anchor.clone(),
            pending_splay_pick: pending_splay_origin_pick,
            pending_anchor_pick,
            transform_inspector: matrix_transform_inspector.clone(),
            canvas_interaction: canvas_interaction.clone(),
            mirrored_pair: mirrored_pair.clone(),
            split_focus,
            workspace,
            view_x,
            view_y,
            width,
            height,
        });
    let part_pointer_down = pointer::part_pointer_down_handler(pointer::PartPointerDownInput {
        runtime: runtime.clone(),
        svg: svg.clone(),
        drag: drag.clone(),
        space_down: space_down.clone(),
        canvas_interaction: canvas_interaction.clone(),
        navigation: canvas_navigation.clone(),
        adapter: adapter.clone(),
        render_scope: render_scope.clone(),
        generation: render_generation,
        range_ids: visible_ids.clone(),
        selection_kind: layout_selection_kind,
        owner: layout_owner.clone(),
        tree_cell_anchor: tree_cell_anchor.clone(),
        mirrored_pair: mirrored_pair.clone(),
        pending_splay_pick: pending_splay_origin_pick,
        transform_inspector: matrix_transform_inspector.clone(),
        split_focus,
        workspace,
        view_x,
        view_y,
        width,
        height,
    });

    // Only a linked Layout split has an unfocused half.
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

    rsx! {
        section { class: "flex min-h-0 flex-1 flex-col", "aria-label": "Keyboard editor",
            div { class: if active_workspace == "Export" { "dock-desk relative flex min-h-0 flex-1 flex-row gap-2 overflow-hidden bg-background p-2 [&_[id=workspace-panel-sheet]]:hidden!" } else { "dock-desk relative flex min-h-0 flex-1 flex-row gap-2 overflow-hidden bg-background p-2" },
                DockLayout {
                    objects: if guide.is_none() { Some(rsx! {
                ObjectsPanel { compact_open: objects_open, settings: objects_panel_settings,
                    header_action: if active_workspace == "Layout" {
                        Some(layout_workspace::add_object_header_action(layout_state.add_object_menu_open()))
                    } else { None },
                    div { class: "min-h-0 min-w-0 flex-[1_1_auto] overflow-x-hidden overflow-y-auto",
                    {workspace_composition::objects(objects_input)}
                    }
                    div { class: "relative mt-auto min-w-0 flex-none border-t border-border max-[760px]:[&_button]:min-h-control-touch [&>[data-slot=collapsible]]:overflow-visible [&_[id=m1-layers-list]]:absolute [&_[id=m1-layers-list]]:bottom-full [&_[id=m1-layers-list]]:left-0 [&_[id=m1-layers-list]]:z-30 [&_[id=m1-layers-list]]:w-full [&_[id=m1-layers-list]]:max-h-[min(65vh,520px)] [&_[id=m1-layers-list]]:overflow-auto [&_[id=m1-layers-list]]:rounded-t-xl [&_[id=m1-layers-list]]:border [&_[id=m1-layers-list]]:border-border-strong [&_[id=m1-layers-list]]:bg-card [&_[id=m1-layers-list]]:p-3",
                        if active_workspace == "Layout" && !layout_assembly_3d() {
                            CanvasLayers {
                                initially_open: false,
                                trigger_label: "Canvas visibility".to_owned(),
                                trigger_id: String::from("m1-layers-trigger"),
                                list_id: String::from("m1-layers-list"),
                                groups: canvas_layers::layout_groups(),
                            }
                        }
                        if active_workspace == "PCB" {
                            pcb_layers::PcbLayerControls { initially_open: false, snapshot: snapshot.clone(), scope: render_scope.clone() }
                        }
                        boardstudio_web_ui_shared::layer_dock::LayerDockOutlet {}
                    }
                    if active_workspace == "Layout" && !layout_assembly_3d() {
                        objects::LayoutSnapDock {
                            settings: layout_snap_settings.read().clone(),
                            open_menu: layout_command_menu,
                            on_snap_intent: layout_snap_intent,
                            handles_visible: layout_state.transform_handles()(),
                            on_toggle_handles: EventHandler::new(move |visible| layout_state.transform_handles().set(visible)),
                        }
                    }
                }
                    }) } else { None },
                    inspector: if has_inspector && guide.is_none() { Some(rsx! {
                    InspectorPanel { compact_open: inspect_open, settings: inspector_panel_settings,
                        if active_workspace != "Layout" && (layout_findings.open)() {
                            layout_findings::LayoutFindingsInspector {
                                open: true,
                                document: Rc::new(document.as_ref().clone()),
                                findings: findings.clone(),
                                source: layout_findings::Source {
                                    workspace: active_workspace,
                                    scope: render_scope.clone(),
                                    token: snapshot.token,
                                    revision: snapshot.document.revision,
                                    generation: render_generation,
                                },
                                on_close: on_close_layout_findings,
                                on_navigate: on_layout_finding,
                            }
                        } else {
                            {workspace_composition::inspector(inspector_input)}
                        }

                    }
                    }) } else { None },
                if guide.is_some() {
                    section { class: "absolute inset-y-0 left-0 z-24 flex w-[min(380px,calc(100vw-32px))] max-w-[calc(100%-16px)] flex-col overflow-hidden border-r border-border-strong bg-card shadow-[8px_0_24px_rgb(0_0_0/16%)] [&>[role=alert]]:mx-4 [&>[role=alert]]:mt-3 [&>[role=alert]]:text-destructive max-[760px]:z-40 max-[760px]:w-[min(380px,calc(100vw-16px))]", aria_label: "Keyboard setup",
                    if full_case_settings() && active_workspace == "Case" {
                        aside { class: "flex min-h-0 flex-1 flex-col bg-card p-4 [&>header]:flex [&>header]:items-center [&>header]:gap-3 [&>header]:pb-4 [&>header_h2]:m-0 [&>header_h2]:text-lg", aria_label: "Full case settings",
                            header {
                                Button { variant: ButtonVariant::Secondary,
                                    class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                                    aria_label: "Back to setup guide",
                                    onclick: move |_| full_case_settings.set(false),
                                    svg { width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8", "aria-hidden": "true",
                                        path { d: "M19 12H5m7-7-7 7 7 7" }
                                    }
                                    "Back to guide"
                                }
                                h2 { "Case settings" }
                            }
                            div { class: "min-h-0 overflow-y-auto",
                                if let Some(settings) = mechanical_settings.props.clone() {
                                    boardstudio_web_case::mechanical_settings::MechanicalSettings { ..settings }
                                }
                            }
                        }
                    } else {
                    if let Some(preferences) = guide.clone() {
                        if let Some(message) = guide_name_failure() { FieldError { "{message}" } }
                        setup_guide::ProjectSetupGuide {
                            stage: preferences.current_stage,
                            stage_readiness: guide_stage_readiness,
                            stage_detail: guide_stage_detail,
                            project_name: name_value,
                            on_name_change,
                            on_name_commit,
                            on_stage_change,
                            on_open_workspace,
                            on_open_matrix_setup: Some(matrix_setup.on_open),
                            matrix_setup: inline_matrix_setup,
                            case_basics: inline_case_basics,
                            controller_picker: inline_controller_picker,
                            has_matrix,
                            on_choose_controller: Some(part_placement.on_choose_controller),
                            on_dismiss: on_dismiss_guide,
                            on_finish: on_finish_guide,
                            project_controls: if preferences.current_stage
                                == SetupGuideStage::Project
                            {
                                Some(pcb_physical_setup::controller::project_setup_controls(
                                    physical_setup_mount.clone(),
                                ))
                            } else { None },
                        }
                    }
                    }
                    }
                }
                section { class: if setup_guide_active { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size] [&>[data-workspace-toolbar]]:hidden" } else if active_workspace == "Export" { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-auto rounded-xl border-0 bg-card shadow-sm" } else { "relative order-2 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border-0 bg-card shadow-sm [container:layout-canvas/inline-size]" }, role: "region", id: "m1-workspace-panel", aria_label: "{active_workspace} workspace",
                    onfocusin: {
                        let mut objects_open = objects_open;
                        let mut inspect_open = inspect_open;
                        move |_| {
                        let compact = web_sys::window()
                            .and_then(|window| window.match_media("(max-width: 760px)").ok().flatten())
                            .is_some_and(|query| query.matches());
                        if workspace() == "Case" && compact {
                            if objects_open() { objects_open.set(false); }
                            if inspect_open() { inspect_open.set(false); }
                        }
                        }
                    },
                    if active_workspace == "Layout" {
                        objects::AddMatrixSurface {
                            matrix_setup: matrix_setup.clone(),
                            mirrored_pair: mirrored_pair.clone(),
                            guide_covering_single: guide.is_some(),
                        }
                    }
                    {workspace_composition::toolbar(toolbar_input)}
                    if guide.as_ref().is_some_and(|guide| guide.current_stage == SetupGuideStage::Project) {
                        setup_guide::SetupHardwarePreview {
                            split: physical_setup_mount.projection.topology == boardstudio_core::model::HardwareTopology::Split,
                            wired: physical_setup_mount.projection.transport == boardstudio_core::model::HardwareTransport::Wired,
                        }
                    } else if matches!(active_workspace, "Layout" | "Keymap" | "Keycaps")
                        && layout_assembly_3d()
                    {
                        layout_viewer::LayoutCanonicalViewer {
                            on_mounted_module_pick: (active_workspace == "Layout")
                                .then_some(mounted_module_select),
                            keycaps_fit: if matches!(active_workspace, "Keymap" | "Keycaps") {
                                keycaps_fit_state.state.clone()
                            } else {
                                None
                            },
                            focused_finding: if active_workspace == "Layout" {
                                focused_keycaps_finding()
                            } else {
                                None
                            },
                        }
                    } else if active_workspace == "Layout" {
                    svg { class: "mt-[76px] block h-[calc(100%-120px)] min-h-0 w-full touch-none bg-canvas max-[760px]:mt-0 max-[760px]:h-full", view_box: "{view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group", "aria-label": "{canvas_aria_label}", onmounted: mount,
                    onpointerdown: start_pan, onpointermove: move_pointer, onpointerup: end_pointer, onpointercancel: cancel_pointer.clone(), onlostpointercapture: cancel_pointer, onkeydown: keyboard, onkeyup: key_up, onwheel: wheel,
                    defs {
                        pattern { id: "m1-layout-grid-small", width: "{canvas_grid.spacing_mm}", height: "{canvas_grid.spacing_mm}", pattern_units: "userSpaceOnUse",
                            circle { cx: "0", cy: "0", r: "{canvas_grid.radius_mm}", fill: "var(--grid-large)", stroke: "none" }
                        }
                    }
                    rect { x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}", fill: "url(#m1-layout-grid-small)" }
                    g { transform: "scale(1,-1)",
                        if !(layer_visibility.hidden)().contains("Board") {
                            for contour in scene.board_contours.iter().filter(|b| b.board_id == model.active_board_id).flat_map(|b| &b.contours) {
                                polygon { points: polygon_points(&contour.points), class: if contour.hole { "fill-canvas stroke-destructive stroke-[0.3] [stroke-dasharray:0.8_0.55]" } else { "fill-geometry-fill stroke-geometry-board stroke-[0.3]" } }
                            }
                        }
                        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.editing_points)()) {
                            outline_lifecycle::OutlinePointCanvasOverlay {
                                key: "{outline_overlay_key.as_deref().unwrap_or_default()}",
                                projection,
                                runtime: outline_lifecycle::OutlineRuntimeHandle::new(
                                    runtime.clone(),
                                ),
                                arbiter: canvas_interaction.clone(),
                                svg: svg.clone(),
                                view_x,
                                view_y,
                                width,
                                height,
                                snap_settings: outline_snap_settings.clone(),
                                pitch: outline_pitch,
                                origins: outline_snap_origins.as_ref().clone(),
                            }
                        }
                        if let Some(bridge) = selected_bridge {
                            polygon { points: polygon_points(&bridge.points), class: "pointer-events-none fill-none stroke-primary stroke-[1.25]", "data-outline-bridge": bridge.id.clone() }
                        }
                        if !(layer_visibility.hidden)().contains("Keys") {
                            if let Some(active) = matrix_placement.placement.as_ref() {
                                {
                                    let cells = crate::mirrored_pair_geometry::preview_cells(&active.scene, &active.matrix);
                                    rsx! {
                                        g {
                                            class: "pointer-events-none",
                                            "aria-label": "Matrix placement preview",
                                            transform: "translate({active.matrix.origin.x} {active.matrix.origin.y})",
                                            for cell in cells {
                                                g { key: "{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(active_pair) = mirrored_pair.placement.as_ref() {
                                {
                                    let pair = &active_pair.pair;
                                    let left_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.left_scene, &pair.matrix);
                                    let right_cells = crate::mirrored_pair_geometry::preview_cells(&active_pair.right_scene, &pair.right_preview);
                                    let line_half_height = f64::from(pair.matrix.rows) * pair.matrix.pitch.y / 2.0 + 16.0;
                                    rsx! {
                                        g { class: "pointer-events-none", "aria-label": "Mirrored pair placement preview", transform: "translate({pair.center.x} {pair.center.y})",
                                            line { class: "stroke-primary stroke-[0.35] [stroke-dasharray:2_2]", x1: "0", x2: "0", y1: "{-line_half_height}", y2: "{line_half_height}" }
                                            for cell in left_cells {
                                                g { key: "left-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                            for cell in right_cells {
                                                g { key: "right-{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                                                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        layout_canvas::Scene {
                            source: canvas_source,
                            focused_half: split_focused_half,
                            preview_active: model.display_preview.is_some(),
                            accepted_scene: model
                                .display_preview
                                .as_ref()
                                .map(|_| snapshot.scene.clone()),
                            spacing_ghost: spacing_ghost_prop.clone(),
                            selected_ids: model.selected_part_ids.clone(),
                            selected_context: selected_tree_context.clone(),
                            view_width_world: width,
                            surface_width_px: zoom_surface_size().0,
                            anchor_badge: canvas_anchor_badge.clone(),
                            keys_visible: !(layer_visibility.hidden)().contains("Keys"),
                            components_visible: !(layer_visibility.hidden)().contains("Components"),
                            keycaps_visible: !(layer_visibility.hidden)().contains("Keycaps"),
                            footprints_visible: (layer_visibility.footprints)()
                                && !(layer_visibility.hidden)().contains("Footprints"),
                            on_matrix_pointer_down: matrix_pointer_down,
                            on_part_pointer_down: part_pointer_down,
                        }
                        if let Some((x, y, scale)) = selection_geometry_anchor
                            && let Some(tools) = canvas_transform_menu.clone()
                            && matrix_transform_inspector.projection.is_some()
                        {
                            g {
                                foreignObject { class: "overflow-visible", transform: "translate({x - 60.0 * scale} {y - 66.0 * scale}) scale({scale} {-scale})", width: "120", height: "44",
                                objects::SelectionDeleteControl { mount: matrix_transform_inspector.clone() }
                            }
                            if !matches!(selected_tree_context.as_ref().map(|selected| &selected.context), Some(objects::TreeContext::Column { .. } | objects::TreeContext::Row { .. })) {
                            { let runtime = runtime.clone(); let svg = svg.clone(); let drag = drag.clone();
                                  let owner = layout_owner.clone(); let adapter = adapter.clone();
                                  let arbiter = canvas_interaction.clone(); let scope = render_scope.clone();
                                  rsx! {
                                    g {
                                        class: "cursor-grab touch-none fill-card stroke-current text-primary outline-none [pointer-events:all] [stroke-width:0.4] active:cursor-grabbing focus-visible:stroke-foreground [&_path]:fill-none",
                                        transform: "translate({x} {y - 18.0 * scale}) scale({scale * 3.0})",
                                        role: "button", tabindex: "0", "aria-label": "Move selection",
                                        "aria-description": "Drag to move. Press Enter to edit the exact position.",
                                        onkeydown: move |event: KeyboardEvent| {
                                            if event.key() == Key::Enter || event.key() == Key::Character(" ".into()) {
                                                event.prevent_default(); event.stop_propagation();
                                                tools.on_show_properties.call(());
                                            }
                                        },
                                        onpointerdown: move |event: PointerEvent| {
                                            let Some(pointer) = event.data().try_as_web_event() else { return; };
                                            if pointer.button() != 0 || arbiter.current().is_some()
                                                || !layout_owner_is_current(&runtime, workspace, &adapter, &owner)
                                                || drag.borrow().is_some() || runtime.model().gesture.is_some() { return; }
                                            pointer.prevent_default(); pointer.stop_propagation();
                                            let Some(origin) = coordinates(&svg, &pointer, view_x, view_y, width, height) else { return; };
                                            let current = runtime.model();
                                            let Some(snapshot) = current.accepted else { return; };
                                            let positions: Vec<_> = snapshot.document.parts.iter().filter(|p| current.selected_part_ids.contains(&p.id)).map(|p| Position { id: p.id.clone(), at: p.pose.at }).collect();
                                            if positions.is_empty() { return; }
                                            if let Some(surface) = svg.borrow().as_ref() {
                                                let _ = surface.set_pointer_capture(pointer.pointer_id());
                                                let options = web_sys::FocusOptions::new(); options.set_prevent_scroll(true);
                                                let _ = surface.focus_with_options(&options);
                                            }
                                            *drag.borrow_mut() = Some(Drag { pointer: i64::from(pointer.pointer_id()), scope: scope.clone(), generation: owner.generation, gesture_generation: None, origin, client_x: f64::from(pointer.client_x()), client_y: f64::from(pointer.client_y()), positions, active: false, pan: false, camera: Vec2::default() });
                                        },
                                        circle { r: "5" }
                                        path { d: "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4" }
                                        title { "Move selection" }
                                    }
                                  }
                                }
                            }
                            }
                        }
                        if active_workspace == "Layout"
                            && !layout_assembly_3d()
                            && let Some((matrix, projection, context)) = layout_transform_target.clone()
                        {
                            for tool in layout_canvas::transform_tools(&context, layout_transform_tool(), layout_state.transform_handles()(), None) {
                            // Ticket 08: the anchored matrix's Anchor token
                            // arms canvas pick mode instead of a drag; an
                            // unanchored matrix offers no token.
                            if tool == objects::LayoutTransformTool::Anchor && matrix.anchor.is_some() {
                            objects::LayoutAnchorPickOverlay {
                                key: "{tool:?}",
                                runtime: objects::LayoutTransformRuntime(runtime.clone()),
                                svg: objects::LayoutTransformSvg(svg.clone()),
                                arbiter: canvas_interaction.clone(),
                                owner: layout_owner.clone(),
                                selected_context: adapter.selected_context,
                                scope_generation: adapter.generation,
                                surface_size: Some(zoom_surface_size),
                                workspace,
                                pick_owner: objects::AnchorPickOwner::Matrix {
                                    matrix: matrix.clone(),
                                    projection: projection.clone(),
                                    context: context.clone(),
                                },
                                view_x,
                                view_y,
                                width,
                                height,
                            }
                            }
                            if tool != objects::LayoutTransformTool::Anchor {
                            objects::LayoutTransformToolOverlay {
                                key: "{tool:?}",
                                runtime: objects::LayoutTransformRuntime(runtime.clone()),
                                svg: objects::LayoutTransformSvg(svg.clone()),
                                arbiter: canvas_interaction.clone(),
                                owner: layout_owner.clone(),
                                selected_context: adapter.selected_context,
                                scope_generation: adapter.generation,
                                surface_size: Some(zoom_surface_size),
                                workspace,
                                matrix: matrix.clone(),
                                projection: projection.clone(),
                                context: context.clone(),
                                tool,
                                snap_settings: layout_snap_settings.read().clone(),
                                snap_origins: outline_snap_origins.clone(),
                                splay_affect: matrix_splay_affect,
                                view_x,
                                view_y,
                                width,
                                height,
                                on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
                            }
                            }
                            }
                        }
                        // Ticket 07: the independent part's own rail. Move and
                        // Rotate edit the local Offset and Rotation; Spread and
                        // Stagger edit the values carried from the anchor key.
                        // The strip above is unchanged: it keeps offering
                        // Position for a part selection.
                        if active_workspace == "Layout"
                            && !layout_assembly_3d()
                            && let Some((part_id, facts)) = part_rail_selection.clone()
                            && let Some(selected) = selected_tree_context.clone()
                            && let Some(part_snap) = part_rail_snap
                        {
                            {
                                let part_context = selected.context.clone();
                                rsx! {
                                    for tool in layout_canvas::transform_tools(&part_context, layout_transform_tool(), layout_state.transform_handles()(), Some(facts)) {
                                    // Ticket 08: the part rail ends in the
                                    // Anchor token, which arms canvas pick
                                    // mode instead of a drag.
                                    if tool == objects::LayoutTransformTool::Anchor {
                                    objects::LayoutAnchorPickOverlay {
                                        key: "{tool:?}",
                                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                                        svg: objects::LayoutTransformSvg(svg.clone()),
                                        arbiter: canvas_interaction.clone(),
                                        owner: layout_owner.clone(),
                                        selected_context: adapter.selected_context,
                                        scope_generation: adapter.generation,
                                        workspace,
                                        surface_size: Some(zoom_surface_size),
                                        pick_owner: objects::AnchorPickOwner::Part {
                                            part_id: part_id.clone(),
                                        },
                                        view_x,
                                        view_y,
                                        width,
                                        height,
                                    }
                                    }
                                    if tool != objects::LayoutTransformTool::Anchor {
                                    objects::LayoutPartToolOverlay {
                                        key: "{tool:?}",
                                        runtime: objects::LayoutTransformRuntime(runtime.clone()),
                                        svg: objects::LayoutTransformSvg(svg.clone()),
                                        arbiter: canvas_interaction.clone(),
                                        owner: layout_owner.clone(),
                                        selected_context: adapter.selected_context,
                                        scope_generation: adapter.generation,
                                        workspace,
                                        surface_size: Some(zoom_surface_size),
                                        part_id: part_id.clone(),
                                        tool,
                                        snap: part_snap,
                                        view_x,
                                        view_y,
                                        width,
                                        height,
                                        on_finish: EventHandler::new(move |_| layout_transform_tool.set(None)),
                                        on_focus_half: {
                                            let document = document.clone();
                                            let mut split_focus = split_focus;
                                            let part_context = part_context.clone();
                                            EventHandler::new(move |_| {
                                                if let Some(half) = boardstudio_web_ui_model::split_focus::context_half(&document, &part_context)
                                                    && half != split_focus()
                                                {
                                                    split_focus.set(half);
                                                }
                                            })
                                        },
                                    }
                                    }
                                    }
                                }
                            }
                        }
                        if let Some(placement) = part_placement.projection.as_ref() {
                            { let definition = placement.pending.definition.clone();
                              let at = placement.pending.part.pose.at;
                              let courtyard = polygon_points(&definition.courtyard);
                              let reference = placement.pending.part.reference.clone();
                              // Ticket 02: the preview name keeps the part-label
                              // rule: at the top of the part at a fixed 11 px.
                              let preview_courtyard_top = definition.courtyard.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max);
                              let preview_courtyard_top = if preview_courtyard_top.is_finite() { preview_courtyard_top } else { 0.0 };
                              let preview_label_font = boardstudio_web_layout::screen_scale::px_to_world(11.0, width, zoom_surface_size().0);
                              let preview_label_y = preview_courtyard_top - boardstudio_web_layout::screen_scale::px_to_world(12.0, width, zoom_surface_size().0);
                              rsx! {
                                g {
                                    class: "pointer-events-none opacity-[0.68]",
                                    "data-placement-preview": "part",
                                    transform: "translate({at.x},{at.y})",
                                    "aria-hidden": "true",
                                    polygon { points: "{courtyard}", class: "fill-selected stroke-primary stroke-[0.7] [stroke-dasharray:2_1]" }
                                    FootprintGraphics {
                                        definition: definition.clone(),
                                        parameters: placement.pending.part.generator_parameters.clone(),
                                    }
                                    for pad in &definition.pads {
                                        g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                                            if pad.plated != Some(false) {
                                                rect { class: "fill-geometry-part stroke-none", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}" }
                                            }
                                            if let Some(drill) = pad.drill { circle { class: "fill-canvas stroke-geometry-outline stroke-[0.1]", r: "{drill / 2.0}" } }
                                        }
                                    }
                                    text { transform: "scale(1,-1)", text_anchor: "middle", class: "pointer-events-none fill-geometry-label font-mono", style: "font-size:{preview_label_font}px", x: "0", y: "{preview_label_y}", "{reference}" }
                                }
                              }
                            }
                        }
                        keycaps_finding_marker::FocusedFindingMarker {
                            workspace: active_workspace.to_owned(),
                            scope: Some(render_scope.clone()),
                            token: Some(snapshot.token),
                            revision: Some(snapshot.document.revision),
                            active_board_id: model.active_board_id.clone(),
                            finding: focused_keycaps_finding(),
                            markers: Rc::from(snapshot.scene.finding_markers.clone()),
                        }
                        if let Some(projection) = outline_inspector.clone().filter(|projection| (projection.drawing_operation)().is_some()) {
                            outline_lifecycle::OutlineDraftCanvasOverlay {
                                key: "{outline_overlay_key.as_deref().unwrap_or_default()}-draft",
                                projection,
                                runtime: outline_lifecycle::OutlineRuntimeHandle::new(runtime.clone()),
                                arbiter: canvas_interaction.clone(),
                                svg: svg.clone(),
                                view_x,
                                view_y,
                                width,
                                height,
                                snap_settings: outline_snap_settings.clone(),
                                pitch: outline_pitch,
                                origins: outline_snap_origins.as_ref().clone(),
                            }
                        }
                    }
                        }
                    } else if active_workspace == "Export" {
                        export_workspace::ExportWorkspace {
                            zmk_firmware: Some(zmk_firmware_export_panel),
                            workspace,
                            return_workspace,
                            inspect_open,
                            inspector_settings: inspector_panel_settings,
                        }
                    } else if let Some(input) = canvas_input {
                        {workspace_composition::canvas(input)}
                    } else {
                        span {}
                    }
                    if show_empty_board {
                        empty_board_canvas::EmptyBoardCanvas {
                            editable: model.lifecycle == Lifecycle::Ready
                                && model.durability == (Durability::Saved { revision: snapshot.document.revision })
                                && model.gesture.is_none()
                                && model.display_preview.is_none(),
                            on_matrix: on_empty_board_matrix,
                            on_parts: on_browse_parts,
                        }
                    }
                    if active_workspace == "Layout"
                        && let Some(placement) = part_placement.projection.as_ref()
                    {
                        div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                            {format!("Place {}", parts::placement_label(&placement.pending.definition))}
                            if let Some(layout_name) = placement.owner.layout_id.as_ref()
                                .and_then(|id| document.layouts.iter().find(|layout| &layout.id == id))
                                .map(|layout| layout.name.as_str())
                            {
                                " in {layout_name}"
                            }
                            " · Click or Enter to place · Esc cancels"
                        }
                    }
                    if active_workspace == "Layout" && pending_splay_origin_pick().is_some() {
                        div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                            "Pick splay origin · Click the canvas · Esc cancels"
                        }
                    }
                    if active_workspace == "Layout" && pending_anchor_pick().is_some() {
                        div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] text-muted-foreground shadow-sm", role: "status",
                            "Pick anchor key · Click a key · Esc cancels"
                        }
                    }
                    // Ticket 16: the at-rest field preview names the keys the
                    // hovered or focused Inspector number field would move.
                    if active_workspace == "Layout"
                        && let Some(state) = field_preview() {
                        div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] font-semibold text-warning shadow-sm", role: "status", "data-field-preview-hint": "",
                            "{state.hint}"
                        }
                    }
                    // Ticket 02: the "Placed from" readout (prototype variant
                    // D) names what the selection is placed from at the
                    // canvas's bottom left. It shares the canvas ring's badge
                    // projection and clears with the selection.
                    if active_workspace == "Layout"
                        && !layout_assembly_3d()
                        && let Some(badge) = canvas_anchor_badge.as_ref()
                    {
                        div { class: "pointer-events-none absolute bottom-[62px] left-3 flex items-center gap-1.5 rounded border border-border bg-card px-2 py-1.5 font-mono text-[11px] text-geometry-part shadow-sm", "data-canvas-anchor-readout": "",
                            "Placed from {badge.label}"
                        }
                    }
                }
                }
            }
            footer { class: "shell-footer", hidden: active_workspace == "Export", aria_label: "Canvas status",
                boardstudio_web_ui_shared::ui::toolbar::Toolbar { class: "shell-history", aria_label: "Footer edit history",
                    boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 0usize, aria_label: "Undo", on_click: move |_| undo.submit(Event::Undo { operation_id: undo.operation() }), "Undo" }
                    boardstudio_web_ui_shared::ui::toolbar::ToolbarButton { index: 1usize, aria_label: "Redo", on_click: move |_| redo.submit(Event::Redo { operation_id: redo.operation() }), "Redo" }
                }
                if matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps") && !layout_assembly_3d() {
                    canvas_status_footer::CanvasStatusFooter {
                        snap_settings: layout_snap_settings.read().clone(),
                        active_part_position,
                        snap_menu_available: matches!(active_workspace, "Layout" | "PCB"),
                        open_menu: layout_command_menu,
                        board_available: true,
                        selection_available: footer_selection_available,
                        zoom_percent,
                        findings_count: Some(keycaps_fit::presented_findings(
                            &findings,
                            &snapshot.document,
                        ).len()),
                        on_toggle_findings: on_toggle_layout_findings,
                        on_fit_board: on_fit_keymap_board,
                        on_fit_selection: on_fit_keymap_selection,
                        on_zoom_out: on_zoom_keymap_out,
                        on_zoom_in: on_zoom_keymap_in,
                    }
                    if let Some(guide) = model.snap_guide.as_ref() {
                        span { class: "ml-auto max-w-[min(35vw,340px)] truncate", role: "status", "{guide.label}" }
                    }
                } else {
                    span { "{model.camera.zoom * 100.0:.0}%" }
                }
                if active_workspace != "Export"
                    && !(matches!(active_workspace, "Layout" | "PCB" | "Keymap" | "Keycaps")
                        && !layout_assembly_3d())
                {
                    canvas_status_footer::FindingsButton {
                        count: keycaps_fit::presented_findings(
                            &findings,
                            &snapshot.document,
                        ).len(),
                        on_toggle: on_toggle_layout_findings,
                    }
                }
            }
        }
    }
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}
