use super::super::*;

pub(super) struct KeycapsPcbInput<'a> {
    pub model: &'a ReadModel,
    pub current_scope: Option<Scope>,
    pub runtime: Rc<Runtime>,
    pub workspace: Signal<&'static str>,
    pub version: Signal<u64>,
    pub generation: Signal<u64>,
    pub instance_selection: InstanceSelection,
    pub layout_owner: LayoutOwnerIdentity,
}

pub(super) struct KeycapsPcbState {
    pub active_board_id: String,
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub focused_keycaps_finding: Signal<Option<keycaps_finding_marker::FocusedFinding>>,
    pub keycaps_navigation_alive: Rc<Cell<bool>>,
    pub layer_source: Option<keymap::LayerSource>,
    pub keycaps_fit_state: keycaps_fit::KeycapsFitActions,
    pub keycaps_settings_actions: Option<keycaps_settings::KeycapsSettingsActions>,
    pub resolution_signal: Signal<pcb_wiring::PcbWiringResolution>,
    pub resolution: pcb_wiring::PcbWiringResolution,
    pub on_resolve: EventHandler<()>,
    pub firmware_feedback: Option<pcb_wiring::FirmwarePositionFeedback>,
    pub firmware_editable: bool,
    pub firmware_on_edit: EventHandler<pcb_wiring::FirmwarePositionEditRequest>,
    pub part_net_actions: pcb_wiring::PartNetActions,
    pub part_input_actions: pcb_wiring::PartInputActions,
}

pub(super) fn use_keycaps_pcb_state(input: KeycapsPcbInput<'_>) -> KeycapsPcbState {
    let KeycapsPcbInput {
        model,
        current_scope,
        runtime,
        workspace,
        version,
        generation,
        instance_selection,
        layout_owner,
    } = input;
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
            active_board_scope_matches(model, scope) && instance_selection.is_current(model)
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
        .filter(|scope| active_board_scope_matches(model, scope))
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
        generation,
    );
    let pcb_wiring_mount = pcb_wiring::use_pcb_wiring_controller(runtime.clone(), version);
    let pcb_part_net_actions =
        pcb_wiring::use_pcb_part_net_edits(runtime.clone(), version, workspace, generation, {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        });
    let firmware_position_actions = pcb_wiring::use_firmware_position_edits(
        runtime.clone(),
        version,
        workspace,
        generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_mount.resolution_signal,
    );
    let part_input_actions =
        pcb_wiring::use_part_input_edits(runtime.clone(), version, workspace, generation, {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        });
    KeycapsPcbState {
        active_board_id,
        keycaps_state,
        focused_keycaps_finding,
        keycaps_navigation_alive,
        layer_source,
        keycaps_fit_state,
        keycaps_settings_actions,
        resolution_signal: pcb_wiring_mount.resolution_signal,
        resolution: pcb_wiring_mount.resolution.clone(),
        on_resolve: pcb_wiring_mount.on_resolve,
        firmware_feedback: firmware_position_actions.feedback.clone(),
        firmware_editable: firmware_position_actions.editable,
        firmware_on_edit: firmware_position_actions.on_edit,
        part_net_actions: pcb_part_net_actions,
        part_input_actions,
    }
}

pub(super) fn use_physical_setup(
    runtime: Rc<Runtime>,
    guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    version: Signal<u64>,
    adapter: SelectionAdapter,
    instance_selection: InstanceSelection,
    workspace: Signal<&'static str>,
) -> pcb_physical_setup::PhysicalSetupMount {
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
    pcb_physical_setup::use_controller(
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
    )
}

pub(super) struct KeymapProjectionsInput {
    pub runtime: Rc<Runtime>,
    pub layer_source: Option<keymap::LayerSource>,
    pub keymap_layer_id: Signal<String>,
    pub workspace: Signal<&'static str>,
    pub generation: Signal<u64>,
    pub accepted_token: Option<boardstudio_application::SnapshotToken>,
    pub current_scope: Option<Scope>,
    pub active_board_id: String,
    pub instance_selection: InstanceSelection,
    pub resolution_signal: Signal<pcb_wiring::PcbWiringResolution>,
}

pub(super) struct KeymapProjections {
    pub layer_actions: keymap::LayerActions,
    pub binding_actions: keymap::BindingActions,
    pub keymap_view: Option<Rc<keymap::KeymapView>>,
    pub keymap_contours: Option<Rc<[Contour]>>,
    pub keycaps_view: Option<Rc<keycaps_scene::KeycapsView>>,
    pub keycaps_contours: Option<Rc<[Contour]>>,
    pub macro_actions: keymap::MacroActions,
}

pub(super) fn use_keymap_projections(input: KeymapProjectionsInput) -> KeymapProjections {
    let KeymapProjectionsInput {
        runtime,
        layer_source,
        keymap_layer_id,
        workspace,
        generation,
        accepted_token,
        current_scope,
        active_board_id,
        instance_selection,
        resolution_signal,
    } = input;
    // Keep the operation observer alive even when the workspace panel is hidden.
    let layer_actions = keymap::use_layer_operations(
        runtime.clone(),
        layer_source.clone(),
        keymap_layer_id,
        workspace,
        generation,
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
    let encoder_input_actions =
        keymap::use_encoder_inputs(runtime.clone(), layer_source.clone(), resolution_signal);
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
        generation,
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
    let macro_actions =
        keymap::use_macro_operations(runtime.clone(), layer_source, workspace, generation, {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        });
    KeymapProjections {
        layer_actions,
        binding_actions,
        keymap_view,
        keymap_contours,
        keycaps_view,
        keycaps_contours,
        macro_actions,
    }
}

pub(super) struct OnShowMechanicalFindingArgs<'a> {
    pub runtime: Rc<Runtime>,
    pub current_scope: Option<Scope>,
    pub model: &'a ReadModel,
    pub adapter_generation: Signal<u64>,
    pub workspace: Signal<&'static str>,
    pub inspect_open: Signal<bool>,
    pub case_selection: case_viewer::CaseSelection,
    pub instance_selection: InstanceSelection,
}

pub(super) fn on_show_mechanical_finding_handler(
    input: OnShowMechanicalFindingArgs<'_>,
) -> EventHandler<String> {
    let OnShowMechanicalFindingArgs {
        runtime,
        current_scope,
        model,
        adapter_generation,
        mut workspace,
        mut inspect_open,
        mut case_selection,
        instance_selection,
    } = input;
    let captured_scope = current_scope.clone();
    let captured_token = model.accepted.as_ref().map(|snapshot| snapshot.token);
    let captured_generation = adapter_generation();
    let generation = adapter_generation;
    let captured_revision = model
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.revision);
    EventHandler::new(move |finding_id: String| {
        let Some(scope) = captured_scope.as_ref() else {
            return;
        };
        let model = runtime.model();
        if generation() != captured_generation
            || runtime.scope().as_ref() != Some(scope)
            || !instance_selection.is_current(&model)
            || model.accepted.as_ref().is_none_or(|current| {
                Some(current.token) != captured_token
                    || Some(current.document.revision) != captured_revision
            })
        {
            return;
        }
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        let (token, revision) = (snapshot.token, snapshot.document.revision);
        let resolved_assembly = runtime
            .resolve_mechanical_settings(
                snapshot.clone(),
                scope.clone(),
                snapshot.document.as_ref().clone(),
            )
            .ok()
            .map(|(assembly, _)| assembly);
        let scene = runtime
            .cad_scene()
            .filter(|scene| &scene.scope == scope && Some(scene.token) == captured_token);
        let assembly = resolved_assembly
            .as_ref()
            .or_else(|| scene.as_ref().and_then(|scene| scene.mechanical.as_ref()));
        let Some(assembly) = assembly else {
            return;
        };
        let captured_document = &snapshot.document;
        let Some(finding) = assembly
            .diagnostics
            .iter()
            .find(|finding| finding.id == finding_id)
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
            case_selection.focus_layer(scope.clone(), token, revision, layer.id.clone());
            workspace.set("Case");
        } else if let Some(body) = captured_document
            .case_bodies
            .iter()
            .find(|body| body.board_id == scope.board_id && finding.target_ids.contains(&body.id))
        {
            case_selection.focus_layer(scope.clone(), token, revision, body.id.clone());
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
    })
}

pub(super) struct Views {
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub scene: std::sync::Arc<boardstudio_core::model::SceneDelta>,
    pub findings: Vec<LayoutFinding>,
    pub visible_ids: Rc<Vec<String>>,
}

pub(super) fn derive_views(
    model: &ReadModel,
    snapshot: &AcceptedSnapshot,
    document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
) -> Views {
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
    let visible_ids: Rc<Vec<String>> =
        Rc::new(visible.iter().map(|part| part.id.clone()).collect());
    Views {
        document,
        scene,
        findings,
        visible_ids,
    }
}
