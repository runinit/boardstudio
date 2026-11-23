use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(super) fn navigate_scoped_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    drag: Rc<RefCell<Option<Drag>>>,
    svg: Rc<RefCell<Option<SvgElement>>>,
    generation: u64,
) -> impl FnMut((Scope, String, Option<String>)) + Clone + 'static {
    let mut selected_context = adapter.selected_context;
    let mut anchor_scope = adapter.anchor_scope;
    move |(captured_scope, board_id, instance_id): (Scope, String, Option<String>)| {
        if runtime.scope().as_ref() != Some(&captured_scope) || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        let valid_board = model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == board_id)
                && instance_id.as_ref().is_none_or(|instance_id| {
                    snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                        hardware.instances.iter().any(|instance| {
                            instance.id == *instance_id && instance.board_id == board_id
                        })
                    })
                })
        });
        if !valid_board {
            return;
        }
        if captured_scope.board_id == board_id && captured_scope.instance_id == instance_id {
            return;
        }
        selection::cancel_scoped_drag(&runtime, &drag, &svg, Some(&captured_scope));
        selected_context.set(None);
        anchor_scope.set(None);
        runtime.submit(Event::Navigate {
            operation_id: runtime.operation(),
            board_id,
            instance_id,
        });
    }
}

pub(super) fn navigate_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    mut navigate_scoped: impl FnMut((Scope, String, Option<String>)) + Clone + 'static,
    generation: u64,
    instance_selection: InstanceSelection,
) -> impl FnMut((Scope, String, Option<String>)) + Clone + 'static {
    move |(captured_scope, board_id, requested_instance): (Scope, String, Option<String>)| {
        if runtime.scope().as_ref() != Some(&captured_scope) || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == board_id)
        {
            return;
        }
        if let Some(id) = requested_instance {
            let valid = snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                hardware
                    .instances
                    .iter()
                    .any(|instance| instance.id == id && instance.board_id == board_id)
            });
            if !valid {
                return;
            }
            instance_selection.reconcile(snapshot.session_epoch, snapshot.document.id.clone(), id);
        }
        let instance_id = instance_selection::resolve(
            &snapshot.document,
            snapshot.session_epoch,
            &board_id,
            instance_selection.0.read().as_ref(),
        )
        .map(str::to_owned);
        navigate_scoped((captured_scope, board_id, instance_id));
    }
}

pub(in crate::presentation) struct OnKeycapsFindingInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub generation: u64,
    pub workspace: Signal<Workspace>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub body_selection: Signal<Option<shared_viewer::BodySelection>>,
    pub case_selection_for_owner: shared_viewer::CaseSelection,
    pub navigation_alive: Rc<Cell<bool>>,
    pub inspector_settings: Signal<PanelSettings>,
    pub fit_state: Option<keycaps_fit::KeycapsFitState>,
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub source_surface: Rc<RefCell<Option<SvgElement>>>,
    pub select_tree: EventHandler<objects::TreeSelectRequest>,
    pub case_selection: shared_viewer::CaseSelection,
}

pub(in crate::presentation) fn on_keycaps_finding_handler(
    input: OnKeycapsFindingInput,
) -> EventHandler<keycaps_fit::FindingNavigationRequest> {
    let OnKeycapsFindingInput {
        runtime,
        adapter,
        scope,
        generation,
        mut workspace,
        mut objects_open,
        mut inspect_open,
        mut body_selection,
        case_selection_for_owner,
        navigation_alive,
        inspector_settings,
        fit_state,
        keycaps_state,
        source_surface,
        select_tree,
        case_selection,
    } = input;
    EventHandler::new(move |request: keycaps_fit::FindingNavigationRequest| {
        let owner = keycaps_navigation::OwnerIdentity {
            scope: &scope,
            generation,
        };
        let model = runtime.model();
        let Some(snapshot) = model.accepted.as_ref() else {
            return;
        };
        let live_mechanical_layers = current_case_layer_ids(&runtime, &scope, snapshot);
        let Some(state) = fit_state.as_ref() else {
            return;
        };
        let Some(accepted_scope) = runtime.scope() else {
            return;
        };
        let accepted = keycaps_navigation::AcceptedNavigationSource {
            scope: accepted_scope,
            session_epoch: snapshot.session_epoch,
            token: snapshot.token,
            revision: snapshot.document.revision,
            active_board_id: model.active_board_id.clone(),
        };
        let Some(admitted) = keycaps_navigation::admit_accepted_request(
            &request,
            keycaps_navigation::NavigationAdmission {
                current_workspace: workspace(),
                owner,
                live_scope: runtime.scope().as_ref(),
                live_generation: (adapter.generation)(),
                accepted: &accepted,
                fit_state: state,
                document: &snapshot.document,
                live_mechanical_layers: live_mechanical_layers.as_deref(),
            },
            |target| match target {
                keycaps_fit::FindingNavigationTarget::Part { part_id, .. } => {
                    objects::component_context_for_finding_part(&model, part_id)
                }
                _ => None,
            },
        ) else {
            return;
        };
        let focused = keycaps_navigation::focused_finding_for_admitted_route(&request, &admitted);
        let effects = admitted.effects;
        let navigation_owner = admitted.owner;
        let source_basis = if workspace() == Workspace::Keycaps {
            source_surface.borrow().as_ref().and_then(|surface| {
                let rect = surface.get_bounding_client_rect();
                keycaps_fit::layout_camera_basis(
                    &snapshot.document,
                    &snapshot.scene,
                    &request.source.scope.board_id,
                    (rect.width(), rect.height()),
                )
            })
        } else {
            None
        };
        keycaps_state.set_focused_finding(focused);
        let focus_runtime = runtime.clone();
        let focus_adapter = adapter.clone();
        let focus_body_selection = body_selection;
        let focus_case_selection = case_selection_for_owner;
        let focus_workspace = workspace;
        keycaps_state.set_pending_layout_fit(None);
        keycaps_navigation::dispatch_route_with_camera_basis(
            effects,
            &request,
            source_basis,
            |effect| match effect {
                keycaps_navigation::RouteAction::SetWorkspace(name) => workspace.set(name),
                keycaps_navigation::RouteAction::SelectTree { scope, context } => {
                    select_tree.call(objects::TreeSelectRequest {
                        scope,
                        context,
                        mode: SelectionMode::Replace,
                        outline_action: None,
                    });
                }
                keycaps_navigation::RouteAction::SelectCaseLayer { scope, layer_id } => {
                    case_selection.select_layer(scope, layer_id);
                }
                keycaps_navigation::RouteAction::SelectCaseBody { scope, body_id } => {
                    // Dioxus retains body and layer selections independently; clear an older
                    // layer so the body finding becomes the active, reachable Inspector owner.
                    case_selection_for_owner.clear_layer_for_scope(&scope);
                    body_selection.set(Some(shared_viewer::BodySelection { scope, body_id }));
                }
                keycaps_navigation::RouteAction::CloseObjects => objects_open.set(false),
                keycaps_navigation::RouteAction::OpenInspector => inspect_open.set(true),
                keycaps_navigation::RouteAction::PinInspector => {
                    pin_inspector_on_desktop(inspector_settings);
                }
                keycaps_navigation::RouteAction::QueueLayoutFit(request) => {
                    keycaps_state.set_pending_layout_fit(Some(*request));
                }
                keycaps_navigation::RouteAction::Report(message) => runtime.report(message),
                keycaps_navigation::RouteAction::FocusInspector => {
                    let scheduled_runtime = focus_runtime.clone();
                    let scheduled_adapter = focus_adapter.clone();
                    focus_first_inspector_control_on_next_frame(
                        navigation_alive.clone(),
                        navigation_owner.clone(),
                        move || {
                            current_keycaps_navigation_owner(
                                focus_workspace,
                                &scheduled_runtime,
                                &scheduled_adapter,
                                focus_body_selection,
                                focus_case_selection,
                            )
                        },
                    );
                }
            },
        );
    })
}

pub(super) struct LayoutFindingCallbacksInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub layout_owner: LayoutOwnerIdentity,
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub inspect_open: Signal<bool>,
    pub objects_open: Signal<bool>,
    pub inspector_panel_settings: Signal<PanelSettings>,
    pub workspace: Signal<Workspace>,
    pub workspace_routing: WorkspaceRoutingSlots,
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub keycaps_navigation_alive: Rc<Cell<bool>>,
    pub case_selection: shared_viewer::CaseSelection,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
}

pub(super) struct LayoutFindingCallbacks {
    pub on_toggle_layout_findings: Callback<(), ()>,
    pub on_close_layout_findings: Callback<(), ()>,
    pub on_return_from_layout_finding: Callback<(), ()>,
    pub on_layout_finding: Callback<layout_findings::Request, ()>,
    pub resume_pending_layout_finding:
        Callback<(layout_findings::Request, LayoutOwnerIdentity), ()>,
}

pub(super) fn use_layout_finding_callbacks(
    input: LayoutFindingCallbacksInput,
) -> LayoutFindingCallbacks {
    let LayoutFindingCallbacksInput {
        runtime,
        adapter,
        layout_owner,
        layout_findings,
        inspect_open,
        objects_open,
        inspector_panel_settings,
        workspace,
        workspace_routing,
        keycaps_state,
        keycaps_navigation_alive,
        case_selection,
        svg,
    } = input;
    let on_toggle_layout_findings = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut findings_open = layout_findings.open;
        let mut inspect_open = inspect_open;
        let mut objects_open = objects_open;
        let inspector_settings = inspector_panel_settings;
        move |()| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            findings_open.set(!findings_open());
            inspect_open.set(true);
            objects_open.set(false);
            pin_inspector_on_desktop(inspector_settings);
        }
    });
    let on_close_layout_findings = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let mut findings_open = layout_findings.open;
        move |()| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner) {
                return;
            }
            findings_open.set(false);
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-layout-findings-trigger"))
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        }
    });
    let on_return_from_layout_finding = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let select_tree = workspace_routing.select_tree;
        let mut findings_open = layout_findings.open;
        let mut return_target = layout_findings.return_target;
        let mut return_focus = layout_findings.return_focus;
        move |()| {
            let Some(target) = return_target.peek().clone() else {
                return;
            };
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner)
                || !layout_finding_return_is_current(
                    &runtime.model(),
                    &owner,
                    adapter.selected_context.read().as_ref(),
                    &target,
                )
            {
                return_target.set(None);
                return;
            }
            let Some(scope) = owner.scope.clone() else {
                return_target.set(None);
                return;
            };
            findings_open.set(false);
            select_tree.call(objects::TreeSelectRequest {
                scope,
                context: target.selection.context,
                mode: SelectionMode::Replace,
                outline_action: None,
            });
            return_target.set(None);
            return_focus.set(true);
        }
    });
    let on_layout_finding = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        let findings_open = layout_findings.open;
        let inspector_settings = inspector_panel_settings;
        let mut pending = layout_findings.pending;
        let mut return_target = layout_findings.return_target;
        let keycaps_state = keycaps_state.clone();
        let svg = svg.clone();
        let alive = keycaps_navigation_alive.clone();
        let body_selection = case_selection.body;
        let select_tree = workspace_routing.select_tree;
        let navigate = workspace_routing.navigate;
        let mut active_workspace = workspace;
        move |request: layout_findings::Request| {
            if !findings_owner_is_current(&runtime, workspace, &adapter, &owner)
                || !source_matches_layout_owner(&request.source, &owner, false)
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !keycaps_fit::presented_findings(&snapshot.scene.findings, &snapshot.document)
                .iter()
                .any(|finding| finding == &request.finding)
                || keycaps_fit::finding_navigation_target(&request.finding, &snapshot.document)
                    .as_ref()
                    != Some(&request.target)
                || !layout_findings::target_has_live_layout_destination(
                    &request.target,
                    &snapshot.document,
                )
            {
                return;
            }
            let target_board = keycaps_fit::target_board_id(&request.target).to_owned();
            let Some(scope) = owner.scope.as_ref() else {
                return;
            };
            if target_board != scope.board_id {
                return_target.set(None);
                pending.set(Some(request));
                active_workspace.set(Workspace::Layout);
                navigate.call((scope.clone(), target_board, None));
                return;
            }
            if owner.workspace != Workspace::Layout {
                return_target.set(None);
                pending.set(Some(request));
                active_workspace.set(Workspace::Layout);
                return;
            }
            let selected = adapter.selected_context.read().clone().filter(|selected| {
                selected.scope == *scope
                    && selection::context_is_current(&model, scope, &selected.context)
            });
            let Some(destination) = layout_finding_context(&model, &request.target) else {
                return;
            };
            return_target.set(selected.map(|selection| LayoutFindingReturnTarget {
                owner: owner.clone(),
                selection,
                destination,
            }));
            perform_layout_finding_navigation(
                LayoutFindingNavigationContext {
                    runtime: runtime.clone(),
                    adapter: adapter.clone(),
                    owner: owner.clone(),
                    workspace,
                    objects_open,
                    inspect_open,
                    findings_open,
                    inspector_settings,
                    keycaps_state: keycaps_state.clone(),
                    svg: svg.clone(),
                    alive: alive.clone(),
                    body_selection,
                    case_selection,
                    select_tree,
                    resumed_after_board_navigation: false,
                },
                request,
            );
        }
    });
    let resume_pending_layout_finding = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let findings_open = layout_findings.open;
        let inspector_settings = inspector_panel_settings;
        let keycaps_state = keycaps_state.clone();
        let svg = svg.clone();
        let alive = keycaps_navigation_alive.clone();
        let body_selection = case_selection.body;
        let select_tree = workspace_routing.select_tree;
        move |(request, owner): (layout_findings::Request, LayoutOwnerIdentity)| {
            perform_layout_finding_navigation(
                LayoutFindingNavigationContext {
                    runtime: runtime.clone(),
                    adapter: adapter.clone(),
                    owner,
                    workspace,
                    objects_open,
                    inspect_open,
                    findings_open,
                    inspector_settings,
                    keycaps_state: keycaps_state.clone(),
                    svg: svg.clone(),
                    alive: alive.clone(),
                    body_selection,
                    case_selection,
                    select_tree,
                    resumed_after_board_navigation: true,
                },
                request,
            );
        }
    });
    LayoutFindingCallbacks {
        on_toggle_layout_findings,
        on_close_layout_findings,
        on_return_from_layout_finding,
        on_layout_finding,
        resume_pending_layout_finding,
    }
}

pub(super) struct LayoutFindingResumeInput {
    pub layout_findings: boardstudio_web_layout::layout_findings_state::LayoutFindingsState,
    pub resume_pending_layout_finding:
        Callback<(layout_findings::Request, LayoutOwnerIdentity), ()>,
    pub adapter: SelectionAdapter,
    pub workspace: Signal<Workspace>,
    pub version: Signal<u64>,
    pub runtime: Rc<Runtime>,
    pub keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub case_selection: shared_viewer::CaseSelection,
    pub svg: Rc<RefCell<Option<SvgElement>>>,
    pub keycaps_navigation_alive: Rc<Cell<bool>>,
}

pub(super) fn use_layout_finding_resume(input: LayoutFindingResumeInput) {
    let LayoutFindingResumeInput {
        mut layout_findings,
        resume_pending_layout_finding,
        adapter,
        workspace,
        version,
        runtime,
        keycaps_state,
        case_selection,
        svg,
        keycaps_navigation_alive,
    } = input;
    layout_findings.resume.replace(Box::new(move |event| {
        resume_pending_layout_finding.call(event);
    }));
    let observed_navigation_selection = (adapter.selected_context)();
    let observed_navigation_generation = (adapter.generation)();
    keycaps_state.use_pending_layout_fit(
        (
            workspace(),
            observed_navigation_selection,
            observed_navigation_generation,
            version(),
        ),
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let body_selection = case_selection.body;
            move || {
                current_keycaps_navigation_owner(
                    workspace,
                    &runtime,
                    &adapter,
                    body_selection,
                    case_selection,
                )
            }
        },
        {
            let runtime = runtime.clone();
            let fit = keycaps_state.fit;
            let surface = svg.clone();
            move |pending_fit| {
                let request = &pending_fit.request;
                let model = runtime.model();
                let snapshot = model.accepted.as_ref().filter(|snapshot| {
                    snapshot.token == request.source.token
                        && snapshot.document.id == request.source.scope.document_id
                        && snapshot.document.revision == request.source.revision
                        && snapshot.session_epoch == request.source.scope.session_epoch
                        && model.active_board_id == request.source.scope.board_id
                })?;
                let live_layers =
                    current_case_layer_ids(&runtime, &pending_fit.owner.scope, snapshot);
                if live_layers.is_none() && request.source.case_preview_current {
                    return None;
                }
                let live_layers = live_layers.unwrap_or_else(|| Rc::from([]));
                let fit = fit.peek();
                let state = fit.as_ref()?;
                if keycaps_fit::accepted_navigation_target(
                    state,
                    request,
                    &snapshot.document,
                    &live_layers,
                ) != Some(request.target.clone())
                {
                    return None;
                }
                let target_bounds = keycaps_fit::finding_navigation_bounds(
                    &snapshot.document,
                    &snapshot.scene,
                    &request.finding.id,
                    &request.target,
                )?;
                let surface = surface.borrow().clone()?;
                let rect = surface.get_bounding_client_rect();
                let destination_surface = (rect.width(), rect.height());
                let basis = if let Some(basis) = pending_fit.source_basis {
                    basis
                } else {
                    let destination_base = keycaps_fit::layout_canvas_bounds(
                        &snapshot.document,
                        &snapshot.scene,
                        &request.source.scope.board_id,
                    )?;
                    keycaps_fit::CameraBasis {
                        bounds: keycaps_fit::aspect_bounds(destination_base, destination_surface),
                        surface: destination_surface,
                    }
                };
                Some(keycaps_navigation::DestinationFitGeometry {
                    base: basis.bounds,
                    target: target_bounds,
                    surface: basis.surface,
                })
            }
        },
        {
            let runtime = runtime.clone();
            let adapter = adapter.clone();
            let body_selection = case_selection.body;
            let alive = keycaps_navigation_alive.clone();
            move |action, expected| match action {
                keycaps_navigation::FitAction::SetCamera(camera_fit) => {
                    runtime.submit(Event::SetCamera {
                        operation_id: runtime.operation(),
                        center: camera_fit.center,
                        zoom: camera_fit.zoom,
                    });
                }
                keycaps_navigation::FitAction::FocusInspector => {
                    let focus_runtime = runtime.clone();
                    let focus_adapter = adapter.clone();
                    focus_first_inspector_control_on_next_frame(
                        alive.clone(),
                        expected,
                        move || {
                            current_keycaps_navigation_owner(
                                workspace,
                                &focus_runtime,
                                &focus_adapter,
                                body_selection,
                                case_selection,
                            )
                        },
                    );
                }
            }
        },
    );
}

pub(super) struct PhysicalSetupReconcileInput {
    pub instance_preference: Signal<Option<instance_selection::Preference>>,
    pub physical_setup_busy: Signal<Option<boardstudio_application::OperationId>>,
    pub observed_version: u64,
    pub observed_interaction_version: u64,
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub scope: Scope,
    pub token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub generation: u64,
    pub drag: Rc<RefCell<Option<Drag>>>,
}

pub(super) fn use_physical_setup_reconcile(
    input: PhysicalSetupReconcileInput,
    mut navigate_scoped: impl FnMut((Scope, String, Option<String>)) + 'static,
) {
    let PhysicalSetupReconcileInput {
        instance_preference,
        physical_setup_busy,
        observed_version,
        observed_interaction_version,
        runtime,
        adapter,
        scope,
        token,
        revision,
        generation,
        drag,
    } = input;
    let preference_version = instance_preference();
    let physical_setup_busy_value = physical_setup_busy();
    let durability_version = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>();
    use_effect(use_reactive(
        (
            &observed_version,
            &preference_version,
            &observed_interaction_version,
            &physical_setup_busy_value,
        ),
        {
            move |_| {
                let _ = durability_version.map(|d| d.0());
                let model = runtime.model();
                if runtime.scope().as_ref() != Some(&scope)
                    || (adapter.generation)() != generation
                    || !instance_selection::can_reconcile(&model)
                    || physical_setup_busy.peek().is_some()
                    || drag.borrow().is_some()
                {
                    return;
                }
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                if snapshot.token != token || snapshot.document.revision != revision {
                    return;
                }
                let instance_id = instance_selection::resolve(
                    &snapshot.document,
                    snapshot.session_epoch,
                    &scope.board_id,
                    instance_preference.read().as_ref(),
                )
                .map(str::to_owned);
                if instance_id != scope.instance_id {
                    navigate_scoped((scope.clone(), scope.board_id.clone(), instance_id));
                }
            }
        },
    ));
}

pub(super) fn on_show_configured_board_handler(
    mut navigate: impl FnMut((Scope, String, Option<String>)) + 'static,
    scope: Scope,
) -> EventHandler<String> {
    EventHandler::new(move |board_id: String| navigate((scope.clone(), board_id, None)))
}

pub(super) fn use_selection_cleanup(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    drag: Rc<RefCell<Option<Drag>>>,
    svg: Rc<RefCell<Option<SvgElement>>>,
    space_down: Rc<Cell<bool>>,
) {
    let cleanup_registration = use_hook({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let drag = drag.clone();
        let svg = svg.clone();
        let space_down = space_down.clone();
        move || {
            let id = adapter.next_cleanup_id.get();
            adapter.next_cleanup_id.set(id.wrapping_add(1).max(1));
            let weak_runtime = Rc::downgrade(&runtime);
            let weak_drag = Rc::downgrade(&drag);
            let weak_svg = Rc::downgrade(&svg);
            let weak_space = Rc::downgrade(&space_down);
            let cleanup: selection::Cleanup = Rc::new(move || {
                let (Some(runtime), Some(drag), Some(svg), Some(space_down)) = (
                    weak_runtime.upgrade(),
                    weak_drag.upgrade(),
                    weak_svg.upgrade(),
                    weak_space.upgrade(),
                ) else {
                    return;
                };
                space_down.set(false);
                let pending = drag.borrow_mut().take();
                if let Some(pending) = pending {
                    if let Some(element) = svg.borrow().as_ref() {
                        let _ = element.release_pointer_capture(pending.pointer as i32);
                    }
                    if pending.active && !pending.pan {
                        selection::cancel_drag_if_owned(&runtime, &pending);
                    }
                }
            });
            *adapter.cleanup.borrow_mut() = Some((id, cleanup));
            id
        }
    });
    use_drop({
        let adapter = adapter.clone();
        move || {
            let mut registered = adapter.cleanup.borrow_mut();
            if registered
                .as_ref()
                .is_some_and(|(id, _)| id == &cleanup_registration)
            {
                registered.take();
            }
        }
    });
}

pub(super) type NavigateFn = Box<dyn FnMut((Scope, String, Option<String>))>;

pub(super) struct NavNamed {
    pub callbacks: LayoutFindingCallbacks,
    pub show_configured_board: EventHandler<String>,
}

pub(super) fn use_navigation_bundle(
    state: &super::session_state::SessionState,
    physical_setup_mount: &pcb_physical_setup::PhysicalSetupMount,
    keycaps_state: &keycaps_workspace::KeycapsWorkspaceState,
    cam: &super::camera::CamNamed,
    snapshot: &AcceptedSnapshot,
    render_scope: &Scope,
    workspace_routing: WorkspaceRoutingSlots,
) -> (NavNamed, NavigateFn, bool) {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let adapter = &state.adapter;
    let drag = &cam.placement.drag;
    let svg = &cam.svg_surface.svg;
    let render_generation = state.render_generation;
    let instance_preference = editor.instance_selection.0;
    let workspace = editor.workspace;
    let objects_open = editor.panels.objects_open;
    let inspect_open = editor.panels.inspector_open;
    let case_selection = state.case_selection;
    let keycaps_navigation_alive = keycaps_state.navigation_alive();
    let inspector_panel_settings = state.inspector_panel_settings;
    let layout_owner = &state.view.layout_owner;
    let layout_findings = state.view.layout_findings;
    let version = editor.version;
    let observed_version = state.observed_version;
    let observed_interaction_version = cam.placement.observed_interaction_version;
    let instance_selection = editor.instance_selection;
    let model = &state.view.model;
    let space_down = &cam.placement.space_down;
    let navigate_scoped = navigate_scoped_handler(
        runtime.clone(),
        adapter.clone(),
        drag.clone(),
        svg.clone(),
        render_generation,
    );
    let navigate = navigate_handler(
        runtime.clone(),
        adapter.clone(),
        navigate_scoped.clone(),
        render_generation,
        instance_selection,
    );
    let LayoutFindingCallbacks {
        on_toggle_layout_findings,
        on_close_layout_findings,
        on_return_from_layout_finding,
        on_layout_finding,
        resume_pending_layout_finding,
    } = use_layout_finding_callbacks(LayoutFindingCallbacksInput {
        runtime: runtime.clone(),
        adapter: adapter.clone(),
        layout_owner: layout_owner.clone(),
        layout_findings,
        inspect_open,
        objects_open,
        inspector_panel_settings,
        workspace,
        workspace_routing,
        keycaps_state: keycaps_state.clone(),
        keycaps_navigation_alive: keycaps_navigation_alive.clone(),
        case_selection,
        svg: svg.clone(),
    });
    use_layout_finding_resume(LayoutFindingResumeInput {
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
    });
    use_physical_setup_reconcile(
        PhysicalSetupReconcileInput {
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
    let instance_scope_pending = !instance_selection.is_current(model);
    let on_show_configured_board =
        on_show_configured_board_handler(navigate.clone(), render_scope.clone());
    use_selection_cleanup(
        runtime.clone(),
        adapter.clone(),
        drag.clone(),
        svg.clone(),
        space_down.clone(),
    );
    (
        NavNamed {
            callbacks: LayoutFindingCallbacks {
                on_toggle_layout_findings,
                on_close_layout_findings,
                on_return_from_layout_finding,
                on_layout_finding,
                resume_pending_layout_finding,
            },
            show_configured_board: on_show_configured_board,
        },
        Box::new(navigate.clone()),
        instance_scope_pending,
    )
}
