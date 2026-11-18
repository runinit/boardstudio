use super::*;

pub(crate) struct RuntimeBundle {
    pub(crate) version: Signal<u64>,
    pub(crate) project_created: Signal<Option<SetupGuideRequest>>,
    pub(crate) pending_new_keyboard: Signal<Option<PendingNewKeyboard>>,
    pub(crate) new_keyboard_error: Signal<String>,
    pub(crate) adapter: SelectionAdapter,
}

pub(crate) fn use_workbench_runtime(runtime: Rc<Runtime>) -> RuntimeBundle {
    let version = use_signal(|| 0u64);
    let project_created = use_signal(|| None::<SetupGuideRequest>);
    let pending_new_keyboard = use_signal(|| None::<PendingNewKeyboard>);
    let new_keyboard_error = use_signal(String::new);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 0u64);
    let adapter = use_hook({
        move || SelectionAdapter::new(selected_context, anchor_scope, scope_generation)
    });
    let selection_retention = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionRetention::default(),
        ))
    });
    let selection_membership = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionMembershipCache::default(),
        ))
    });
    let observed_scope = use_hook({
        let runtime = runtime.clone();
        move || Rc::new(RefCell::new(runtime.scope()))
    });
    let observed_token = use_hook({
        let runtime = runtime.clone();
        move || {
            Rc::new(RefCell::new(
                runtime.model().accepted.as_ref().map(|s| s.token),
            ))
        }
    });
    let reconciling_scope = use_hook(|| Rc::new(Cell::new(false)));
    use_hook({
        let runtime = runtime.clone();
        let active = active.clone();
        let weak_runtime = Rc::downgrade(&runtime);
        let adapter = adapter.clone();
        let selection_retention = selection_retention.clone();
        let selection_membership = selection_membership.clone();
        let observed_scope = observed_scope.clone();
        let reconciling_scope = reconciling_scope.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                if !active.get() {
                    return;
                }
                let Some(runtime) = weak_runtime.upgrade() else {
                    return;
                };
                let Some(_reset) = ReentrancyReset::enter(reconciling_scope.clone()) else {
                    return;
                };
                let mut selected_context = adapter.selected_context;
                let mut anchor_scope = adapter.anchor_scope;
                let mut generation = adapter.generation;
                let next_scope = runtime.scope();
                let next_token = runtime
                    .model()
                    .accepted
                    .as_ref()
                    .map(|snapshot| snapshot.token);
                let previous_token = {
                    let mut observed = observed_token.borrow_mut();
                    std::mem::replace(&mut *observed, next_token)
                };
                let previous_scope = {
                    let mut observed = observed_scope.borrow_mut();
                    std::mem::replace(&mut *observed, next_scope.clone())
                };
                if previous_scope != next_scope {
                    selected_context.set(None);
                    anchor_scope.set(None);
                    let previous_generation = *generation.peek();
                    generation += 1;
                    adapter.record_scope_transition(selection::ScopeTransition {
                        previous_scope,
                        previous_token,
                        next_scope: next_scope.clone(),
                        next_token,
                        previous_generation,
                        next_generation: *generation.peek(),
                    });
                    let cleanup = adapter
                        .cleanup
                        .borrow()
                        .as_ref()
                        .map(|(_, cleanup)| cleanup.clone());
                    if let Some(cleanup) = cleanup {
                        cleanup();
                    }
                }

                let model = runtime.model();
                let invalid_context =
                    adapter
                        .selected_context
                        .read()
                        .as_ref()
                        .is_some_and(|selected| {
                            next_scope.as_ref() != Some(&selected.scope)
                                || !selection::context_is_current(
                                    &model,
                                    &selected.scope,
                                    &selected.context,
                                )
                        });
                if invalid_context {
                    selected_context.set(None);
                    anchor_scope.set(None);
                }

                let membership = selection_membership.borrow_mut().project(&model, |model| {
                    crate::matrix_transform_lifecycle::SelectionMembership {
                        eligible: selection::eligible_live_ids(model),
                        live: selection::live_board_ids(model),
                    }
                });
                let selected_context = (adapter.selected_context)().filter(|selected| {
                    matches!(&selected.context, objects::TreeContext::Key { .. })
                });
                let selected_ids = selection_retention.borrow_mut().reconcile(
                    selected_context.as_ref(),
                    &model.selected_part_ids,
                    &membership.eligible,
                    &membership.live,
                );
                if selected_ids != model.selected_part_ids {
                    anchor_scope.set(None);
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: selected_ids,
                        range_part_ids: Vec::new(),
                        mode: SelectionMode::Replace,
                    });
                }
                if let Some(scope) = next_scope.as_ref() {
                    let current = runtime.model();
                    let membership = selection_membership
                        .borrow_mut()
                        .project(&current, |model| {
                            crate::matrix_transform_lifecycle::SelectionMembership {
                                eligible: selection::eligible_live_ids(model),
                                live: selection::live_board_ids(model),
                            }
                        });
                    if (adapter.anchor_scope)().as_ref() == Some(scope)
                        && current
                            .selection_anchor_id
                            .as_ref()
                            .is_none_or(|anchor| !membership.eligible.iter().any(|id| id == anchor))
                    {
                        anchor_scope.set(None);
                    }
                }
                let mut signal = version;
                signal += 1;
            }))
        }
    });
    use_drop({
        let runtime = runtime.clone();
        move || {
            active.set(false);
            runtime.unsubscribe();
            runtime.submit(Event::Close {
                operation_id: runtime.operation(),
            });
        }
    });
    RuntimeBundle {
        version,
        project_created,
        pending_new_keyboard,
        new_keyboard_error,
        adapter,
    }
}

pub(crate) struct PanelsBundle {
    pub(crate) workspace: Signal<&'static str>,
    pub(crate) project_browser: Signal<bool>,
    pub(crate) return_workspace: Signal<&'static str>,
    pub(crate) observed_version: u64,
}

pub(crate) fn use_workbench_panels(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    project_created: Signal<Option<SetupGuideRequest>>,
    pending_new_keyboard: Signal<Option<PendingNewKeyboard>>,
    new_keyboard_error: Signal<String>,
    adapter: SelectionAdapter,
) -> PanelsBundle {
    let observed_version = version();
    use_context_provider(|| runtime.clone());
    use_context_provider(|| version);
    use_context_provider(|| project_created);
    use_context_provider(|| pending_new_keyboard);
    use_context_provider(|| new_keyboard_error);
    use_context_provider(|| adapter.clone());
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let project_browser = use_signal(|| false);
    use_context_provider(|| library::ProjectBrowserState(project_browser));
    let mut objects_open = use_signal(|| false);
    let mut inspector_open = use_signal(|| false);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let mut return_workspace = use_signal(|| "Layout");
    use_context_provider(|| ExportReturnWorkspace(return_workspace));
    use_effect(move || {
        let active = workspace();
        if active != "Export" {
            return_workspace.set(active);
        } else {
            objects_open.set(false);
            inspector_open.set(false);
        }
    });
    PanelsBundle {
        workspace,
        project_browser,
        return_workspace,
        observed_version,
    }
}

pub(crate) fn use_keyboard_creation(
    runtime: Rc<Runtime>,
    observed_version: u64,
    pending_new_keyboard: Signal<Option<PendingNewKeyboard>>,
    project_created: Signal<Option<SetupGuideRequest>>,
    new_keyboard_error: Signal<String>,
) {
    let case_generation = CaseGenerationState {
        live_preview: use_signal(|| true),
        automatic: use_signal(AutomaticCaseGeneration::new),
    };
    use_context_provider(|| case_generation);
    let runtime_for_creation = runtime.clone();
    use_effect(use_reactive!(|observed_version| {
        let _ = observed_version;
        let mut pending_new_keyboard = pending_new_keyboard;
        let mut project_created = project_created;
        let mut new_keyboard_error = new_keyboard_error;
        let Some(pending) = pending_new_keyboard() else {
            return;
        };
        let Some(outcome) = pending.outcome.borrow().clone() else {
            return;
        };
        let model = runtime_for_creation.model();
        match outcome {
            TerminalOutcome::Completed => {
                let settlement = crate::setup_guide_state::creation_settlement(
                    &pending.project_id,
                    model
                        .accepted
                        .as_ref()
                        .map(|snapshot| snapshot.document.id.as_str()),
                    model.lifecycle == Lifecycle::Ready
                        && model.accepted.as_ref().is_some_and(|snapshot| {
                            model.durability
                                == Durability::Saved {
                                    revision: snapshot.document.revision,
                                }
                        }),
                );
                match settlement {
                    crate::setup_guide_state::CreationSettlement::Wait => return,
                    crate::setup_guide_state::CreationSettlement::Retire => {
                        new_keyboard_error.set(
                            "The new keyboard was superseded before it could open. Try again."
                                .into(),
                        );
                        pending_new_keyboard.set(None);
                        return;
                    }
                    crate::setup_guide_state::CreationSettlement::Reveal => {}
                }
                project_created.set(Some(SetupGuideRequest {
                    project_id: pending.project_id.clone(),
                    request_id: pending.project_id,
                    start_at_project: true,
                }));
                pending_new_keyboard.set(None);
                new_keyboard_error.set(String::new());
            }
            TerminalOutcome::Rejected(message)
            | TerminalOutcome::PersistenceFailed(message)
            | TerminalOutcome::BlockedByRecovery(message)
            | TerminalOutcome::ExecutorFailed(message) => {
                new_keyboard_error.set(message);
                pending_new_keyboard.set(None);
            }
            TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
                new_keyboard_error.set("The new keyboard could not be opened. Try again.".into());
                pending_new_keyboard.set(None);
            }
        }
    }));
}

pub(crate) fn use_workbench_appearance() -> Signal<bool> {
    let layer_visibility = LayerVisibility {
        hidden: use_signal(BTreeSet::new),
        modules_hidden: use_signal(pcb_module_footprints::default_hidden_layers),
        footprints: use_signal(|| false),
    };
    use_context_provider(|| layer_visibility);
    let preference_warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(preference_warning));
    let appearance = use_signal(move || panels::read_appearance_preference(preference_warning));
    let system_appearance = use_signal(read_system_appearance);
    use_context_provider(|| AppearanceState(appearance));
    let theme = use_signal(move || panels::read_workbench_theme_preference(preference_warning));
    use_context_provider(|| ThemeState(theme));
    let resolved_appearance = use_memo(move || {
        let preference = appearance();
        if preference == "system" {
            system_appearance()
        } else {
            preference
        }
    });
    use_context_provider(|| ResolvedAppearance(resolved_appearance));
    let media_listener = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(web_sys::MediaQueryList, Closure<dyn FnMut(web_sys::Event)>)>,
        ))
    });
    use_effect({
        let mut system_appearance = system_appearance;
        let media_listener = media_listener.clone();
        move || {
            if let Some(query) = web_sys::window().and_then(|window| {
                window
                    .match_media("(prefers-color-scheme: dark)")
                    .ok()
                    .flatten()
            }) {
                let observed_query = query.clone();
                let listener = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    system_appearance.set(if observed_query.matches() {
                        "dark"
                    } else {
                        "light"
                    });
                }) as Box<dyn FnMut(_)>);
                let _ = query
                    .add_event_listener_with_callback("change", listener.as_ref().unchecked_ref());
                *media_listener.borrow_mut() = Some((query, listener));
            }
        }
    });
    use_drop({
        let media_listener = media_listener.clone();
        move || {
            if let Some((query, listener)) = media_listener.borrow_mut().take() {
                let _ = query.remove_event_listener_with_callback(
                    "change",
                    listener.as_ref().unchecked_ref(),
                );
            }
        }
    });
    use_effect(use_reactive((&resolved_appearance(), &theme()), {
        move |(effective, theme)| {
            if let Some(root) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.document_element())
            {
                let _ = root.set_attribute("data-appearance", effective);
                let _ = root.set_attribute("data-theme", theme);
            }
        }
    }));
    use_effect(|| {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            document.set_title("BoardStudio");
        }
    });
    preference_warning
}

pub(crate) struct HeaderBundle {
    pub(crate) project_name: String,
    pub(crate) save_state: &'static str,
    pub(crate) settings_open: Signal<bool>,
    pub(crate) palette: command_palette::PaletteState,
    pub(crate) on_undo: EventHandler<()>,
    pub(crate) on_redo: EventHandler<()>,
    pub(crate) on_export: EventHandler<()>,
}

pub(crate) fn use_workbench_header(
    runtime: Rc<Runtime>,
    mut workspace: Signal<&'static str>,
    return_workspace: Signal<&'static str>,
    project_browser: Signal<bool>,
) -> HeaderBundle {
    let project_name = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| snapshot.document.name.clone())
        .unwrap_or_else(|| "Open project".into());
    let save_state = if runtime.model().accepted.is_none() {
        "ready"
    } else {
        durability_state(&runtime.model().durability)
    };
    let settings_open = use_signal(|| false);
    let palette = command_palette::use_palette_state();
    boardstudio_web_ui_shared::ui::command::use_command_registry();
    library::use_project_tools();
    let header_undo = runtime.clone();
    let header_redo = runtime.clone();
    let on_undo = EventHandler::new(move |()| {
        header_undo.submit(Event::Undo {
            operation_id: header_undo.operation(),
        })
    });
    let on_redo = EventHandler::new(move |()| {
        header_redo.submit(Event::Redo {
            operation_id: header_redo.operation(),
        })
    });
    let on_export = EventHandler::new(move |()| {
        if workspace() == "Export" {
            workspace.set(return_workspace())
        } else {
            workspace.set("Export")
        }
    });
    command_palette::use_navigation_commands(
        workspace,
        project_browser,
        runtime.model().accepted.is_some(),
        on_undo,
        on_redo,
        on_export,
    );
    HeaderBundle {
        project_name,
        save_state,
        settings_open,
        palette,
        on_undo,
        on_redo,
        on_export,
    }
}

pub(crate) fn read_system_appearance() -> &'static str {
    if web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
    {
        "dark"
    } else {
        "light"
    }
}
