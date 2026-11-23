use super::*;

pub(crate) struct RuntimeBundle {
    pub(crate) version: Signal<u64>,
    pub(crate) durability_version: Signal<u64>,
    pub(crate) project_created: Signal<Option<SetupGuideRequest>>,
    pub(crate) pending_new_keyboard: Signal<Option<PendingNewKeyboard>>,
    pub(crate) new_keyboard_error: Signal<String>,
    pub(crate) adapter: SelectionAdapter,
}

pub(crate) fn use_workbench_runtime(runtime: Rc<Runtime>) -> RuntimeBundle {
    let version = use_signal(|| 0u64);
    let durability_version = use_signal(|| 0u64);
    use_context_provider(|| boardstudio_web_ui_shared::DurabilityVersion(durability_version));
    let project_created = use_signal(|| None::<SetupGuideRequest>);
    let pending_new_keyboard = use_signal(|| None::<PendingNewKeyboard>);
    let new_keyboard_error = use_signal(String::new);
    let active = use_hook(|| Rc::new(Cell::new(true)));
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| None);
    let scope_generation = use_signal(|| 0u64);
    let adapter =
        use_hook(move || SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    let retention = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionRetention::default(),
        ))
    });
    let membership_cache = use_hook(|| {
        Rc::new(RefCell::new(
            crate::matrix_transform_lifecycle::SelectionMembershipCache::default(),
        ))
    });
    let obs_scope = use_hook({
        let r = runtime.clone();
        move || Rc::new(RefCell::new(r.scope()))
    });
    let obs_token = use_hook({
        let r = runtime.clone();
        move || {
            Rc::new(RefCell::new(
                r.with_model(|m| m.accepted.as_ref().map(|s| s.token)),
            ))
        }
    });
    let reconciling = use_hook(|| Rc::new(Cell::new(false)));
    use_hook({
        let (rt, act, weak, ad, ret, mem, os, rec) = (
            runtime.clone(),
            active.clone(),
            Rc::downgrade(&runtime),
            adapter.clone(),
            retention.clone(),
            membership_cache.clone(),
            obs_scope.clone(),
            reconciling.clone(),
        );
        move || {
            let act_d = act.clone();
            let rt_d = rt.clone();
            rt.subscribe_durability(Rc::new(move || {
                if !act_d.get() {
                    return;
                }
                let mut dv = durability_version;
                dv += 1;
                if rt_d.with_model(|m| matches!(m.durability, Durability::Failed { .. })) {
                    let mut v = version;
                    v += 1;
                }
            }));
            rt.subscribe(Rc::new(move || {
                if !act.get() {
                    return;
                }
                let Some(rt) = weak.upgrade() else {
                    return;
                };
                let Some(_reset) = ReentrancyReset::enter(rec.clone()) else {
                    return;
                };
                let (mut sc, mut anc, mut generation) =
                    (ad.selected_context, ad.anchor_scope, ad.generation);
                let next_scope = rt.scope();
                let next_token = rt.with_model(|m| m.accepted.as_ref().map(|s| s.token));
                let prev_token = std::mem::replace(&mut *obs_token.borrow_mut(), next_token);
                let prev_scope = std::mem::replace(&mut *os.borrow_mut(), next_scope.clone());
                if prev_scope != next_scope {
                    sc.set(None);
                    anc.set(None);
                    let prev_gen = *generation.peek();
                    generation += 1;
                    ad.record_scope_transition(selection::ScopeTransition {
                        previous_scope: prev_scope,
                        previous_token: prev_token,
                        next_scope: next_scope.clone(),
                        next_token,
                        previous_generation: prev_gen,
                        next_generation: *generation.peek(),
                    });
                    if let Some(cleanup) = ad.cleanup.borrow().as_ref().map(|(_, c)| c.clone()) {
                        cleanup();
                    }
                }
                let (clear_ctx, submit_sel_ids) = rt.with_model(|model| {
                    let clear_ctx = ad.selected_context.read().as_ref().is_some_and(|s| {
                        next_scope.as_ref() != Some(&s.scope)
                            || !selection::context_is_current(model, &s.scope, &s.context)
                    });
                    let m = mem.borrow_mut().project(model, |m| {
                        crate::matrix_transform_lifecycle::SelectionMembership {
                            eligible: selection::eligible_live_ids(m),
                            live: selection::live_board_ids(m),
                        }
                    });
                    let key_ctx = if clear_ctx {
                        None
                    } else {
                        (ad.selected_context)()
                            .filter(|s| matches!(&s.context, objects::TreeContext::Key { .. }))
                    };
                    let sel_ids = ret.borrow_mut().reconcile(
                        key_ctx.as_ref(),
                        &model.selected_part_ids,
                        &m.eligible,
                        &m.live,
                    );
                    (
                        clear_ctx,
                        (sel_ids != model.selected_part_ids).then_some(sel_ids),
                    )
                });
                if clear_ctx {
                    sc.set(None);
                    anc.set(None);
                }
                if let Some(part_ids) = submit_sel_ids {
                    anc.set(None);
                    rt.submit(Event::SelectParts {
                        operation_id: rt.operation(),
                        part_ids,
                        range_part_ids: vec![],
                        mode: SelectionMode::Replace,
                    });
                }
                if let Some(scope) = next_scope.as_ref() {
                    let clear_anchor = rt.with_model(|cur| {
                        let m = mem.borrow_mut().project(cur, |m| {
                            crate::matrix_transform_lifecycle::SelectionMembership {
                                eligible: selection::eligible_live_ids(m),
                                live: selection::live_board_ids(m),
                            }
                        });
                        (ad.anchor_scope)().as_ref() == Some(scope)
                            && cur
                                .selection_anchor_id
                                .as_ref()
                                .is_none_or(|a| !m.eligible.contains(a))
                    });
                    if clear_anchor {
                        anc.set(None);
                    }
                }
                let mut v = version;
                v += 1;
            }))
        }
    });
    use_drop({
        let rt = runtime.clone();
        move || {
            active.set(false);
            rt.unsubscribe_durability();
            rt.unsubscribe();
            rt.submit(Event::Close {
                operation_id: rt.operation(),
            });
        }
    });
    RuntimeBundle {
        version,
        durability_version,
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
}

pub(crate) fn use_workbench_panels(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    project_created: Signal<Option<SetupGuideRequest>>,
    pending_new_keyboard: Signal<Option<PendingNewKeyboard>>,
    new_keyboard_error: Signal<String>,
    adapter: SelectionAdapter,
) -> PanelsBundle {
    let _ = version();
    use_context_provider(|| runtime);
    use_context_provider(|| version);
    use_context_provider(|| project_created);
    use_context_provider(|| pending_new_keyboard);
    use_context_provider(|| new_keyboard_error);
    use_context_provider(|| adapter);
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    let project_browser = use_signal(|| false);
    use_context_provider(|| library::ProjectBrowserState(project_browser));
    let (mut obj_open, mut insp_open) = (use_signal(|| false), use_signal(|| false));
    use_context_provider(|| CompactPanelState {
        objects_open: obj_open,
        inspector_open: insp_open,
    });
    let mut return_workspace = use_signal(|| "Layout");
    use_context_provider(|| ExportReturnWorkspace(return_workspace));
    use_effect(move || {
        let active = workspace();
        if active != "Export" {
            return_workspace.set(active);
        } else {
            obj_open.set(false);
            insp_open.set(false);
        }
    });
    PanelsBundle {
        workspace,
        project_browser,
        return_workspace,
    }
}

pub(crate) fn use_keyboard_creation(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    durability_version: Signal<u64>,
    mut pending_new: Signal<Option<PendingNewKeyboard>>,
    mut project_created: Signal<Option<SetupGuideRequest>>,
    mut new_error: Signal<String>,
) {
    let live_preview = use_signal(|| true);
    let automatic = use_signal(AutomaticCaseGeneration::new);
    use_context_provider(move || CaseGenerationState {
        live_preview,
        automatic,
    });
    use_effect(move || {
        let _ = version();
        let Some(pending) = pending_new() else {
            return;
        };
        let _ = durability_version();
        let Some(outcome) = pending.outcome.borrow().clone() else {
            return;
        };
        match outcome {
            TerminalOutcome::Completed => {
                let settled = runtime.with_model(|model| {
                    crate::setup_guide_state::creation_settlement(
                        &pending.project_id,
                        model.accepted.as_ref().map(|s| s.document.id.as_str()),
                        model.lifecycle == Lifecycle::Ready
                            && model.accepted.as_ref().is_some_and(|s| {
                                model.durability
                                    == Durability::Saved {
                                        revision: s.document.revision,
                                    }
                            }),
                    )
                });
                match settled {
                    crate::setup_guide_state::CreationSettlement::Wait => return,
                    crate::setup_guide_state::CreationSettlement::Retire => {
                        new_error.set(
                            "The new keyboard was superseded before it could open. Try again."
                                .into(),
                        );
                        pending_new.set(None);
                        return;
                    }
                    crate::setup_guide_state::CreationSettlement::Reveal => {}
                }
                project_created.set(Some(SetupGuideRequest {
                    project_id: pending.project_id.clone(),
                    request_id: pending.project_id,
                    start_at_project: true,
                }));
                pending_new.set(None);
                new_error.set(String::new());
            }
            TerminalOutcome::Rejected(m)
            | TerminalOutcome::PersistenceFailed(m)
            | TerminalOutcome::ExecutorFailed(m) => {
                new_error.set(m);
                pending_new.set(None);
            }
            TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
                new_error.set("The new keyboard could not be opened. Try again.".into());
                pending_new.set(None);
            }
        }
    });
}

pub(crate) fn use_workbench_appearance() -> Signal<bool> {
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(pcb_module_footprints::default_hidden_layers);
    let footprints = use_signal(|| false);
    use_context_provider(move || LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    let pref_warn = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(pref_warn));
    let appearance = use_signal(move || panels::read_appearance_preference(pref_warn));
    let mut sys_app = use_signal(read_system_appearance);
    use_context_provider(|| AppearanceState(appearance));
    let theme = use_signal(move || panels::read_workbench_theme_preference(pref_warn));
    use_context_provider(|| ThemeState(theme));
    let resolved = use_memo(move || {
        let p = appearance();
        if p == "system" { sys_app() } else { p }
    });
    use_context_provider(|| ResolvedAppearance(resolved));
    let media_listener = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(web_sys::MediaQueryList, Closure<dyn FnMut(web_sys::Event)>)>,
        ))
    });
    use_effect({
        let ml = media_listener.clone();
        move || {
            if let Some(q) = web_sys::window()
                .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
            {
                let obs = q.clone();
                let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    sys_app.set(if obs.matches() { "dark" } else { "light" })
                }) as Box<dyn FnMut(_)>);
                let _ = q.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
                *ml.borrow_mut() = Some((q, cb));
            }
        }
    });
    use_drop(move || {
        if let Some((q, cb)) = media_listener.borrow_mut().take() {
            let _ = q.remove_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
        }
    });
    use_effect(use_reactive((&resolved(), &theme()), move |(eff, th)| {
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = root.set_attribute("data-appearance", eff);
            let _ = root.set_attribute("data-theme", th);
        }
    }));
    use_effect(|| {
        if let Some(d) = web_sys::window().and_then(|w| w.document()) {
            d.set_title("BoardStudio");
        }
    });
    pref_warn
}

pub(crate) struct HeaderBundle {
    pub(crate) project_name: String,
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
    let (project_name, has_accepted) = runtime.with_model(|model| {
        (
            model
                .accepted
                .as_ref()
                .map_or_else(|| "Open project".into(), |s| s.document.name.clone()),
            model.accepted.is_some(),
        )
    });
    let settings_open = use_signal(|| false);
    let palette = command_palette::use_palette_state();
    boardstudio_web_ui_shared::ui::command::use_command_registry();
    library::use_project_tools();
    let (ru, rr) = (runtime.clone(), runtime);
    let on_undo = EventHandler::new(move |()| {
        ru.submit(Event::Undo {
            operation_id: ru.operation(),
        })
    });
    let on_redo = EventHandler::new(move |()| {
        rr.submit(Event::Redo {
            operation_id: rr.operation(),
        })
    });
    let on_export = EventHandler::new(move |()| {
        workspace.set(if workspace() == "Export" {
            return_workspace()
        } else {
            "Export"
        })
    });
    command_palette::use_navigation_commands(
        workspace,
        project_browser,
        has_accepted,
        on_undo,
        on_redo,
        on_export,
    );
    HeaderBundle {
        project_name,
        settings_open,
        palette,
        on_undo,
        on_redo,
        on_export,
    }
}

pub(crate) fn read_system_appearance() -> &'static str {
    if web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
        .is_some_and(|q| q.matches())
    {
        "dark"
    } else {
        "light"
    }
}
