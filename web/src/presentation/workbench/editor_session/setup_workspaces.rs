use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) fn use_physical_setup(
    runtime: Rc<Runtime>,
    guide_preferences: Signal<Option<setup_guide::SetupGuidePreferences>>,
    version: Signal<u64>,
    adapter: SelectionAdapter,
    instance_selection: InstanceSelection,
    workspace: Signal<Workspace>,
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

pub(super) struct OnShowMechanicalFindingArgs<'a> {
    pub runtime: Rc<Runtime>,
    pub current_scope: Option<Scope>,
    pub model: &'a ReadModel,
    pub adapter_generation: Signal<u64>,
    pub workspace: Signal<Workspace>,
    pub inspect_open: Signal<bool>,
    pub case_selection: shared_viewer::CaseSelection,
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
            workspace.set(Workspace::Case);
        } else if let Some(body) = captured_document
            .case_bodies
            .iter()
            .find(|body| body.board_id == scope.board_id && finding.target_ids.contains(&body.id))
        {
            case_selection.focus_layer(scope.clone(), token, revision, body.id.clone());
            case_selection.body.set(Some(shared_viewer::BodySelection {
                scope: scope.clone(),
                body_id: body.id.clone(),
            }));
            workspace.set(Workspace::Case);
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
                workspace.set(Workspace::Layout);
            }
        }
        inspect_open.set(true);
        runtime.report(finding.message.clone());
    })
}

pub(in crate::presentation) struct Views {
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
