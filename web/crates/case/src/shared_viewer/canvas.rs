use super::*;
use crate::mechanical_settings;
use boardstudio_web_host::renderer_host::{RendererStatus, renderer_module};

#[derive(Props, Clone, PartialEq)]
pub(super) struct ViewportCanvas {
    pub projection: Rc<RendererSceneProjection>,
    pub assembly_layers: Vec<crate::presentation::case_assembly_layers::CaseAssemblyLayer>,
    pub component_layers: Vec<crate::presentation::case_assembly_layers::CaseComponentLayer>,
    pub selectable_layers: Vec<String>,
    pub canvas_context: ViewerCanvasContext,
    pub selected_layer: String,
    pub selected_reference: Option<String>,
    pub display: CaseDisplay,
    pub theme: String,
    pub owner: Rc<ViewerOwner>,
    pub current_source: SourceGuard,
    pub on_signal: EventHandler<ScopedViewerSignal>,
    pub on_display_change: EventHandler<ScopedDisplayChange>,
    pub mechanical_settings: Option<crate::presentation::MechanicalSettingsProps>,
    pub inline_case_controls: bool,
    pub toolbar_leading: Option<Element>,
    pub handle_projection: Option<CaseHandleProjection>,
    pub handles: Vec<ViewerHandle>,
    pub handle_preview: Option<Vec<ViewerHandle>>,
    pub gesture_message: Option<String>,
    pub focus_request: Option<ViewerFocusRequest>,
}

// The canvas can disappear when a source lease is revoked. Give its WebGL
// attachment a matching Dioxus lifetime, including when the viewport shows an error.
#[component]
pub(super) fn MountedCanvas(props: ViewportCanvas) -> Element {
    use_hook({
        let owner = props.owner.clone();
        move || owner.active.set(true)
    });

    let ViewportCanvas {
        projection,
        assembly_layers,
        component_layers,
        selectable_layers,
        canvas_context,
        selected_layer,
        selected_reference,
        display,
        theme,
        owner,
        current_source,
        on_signal,
        on_display_change,
        mechanical_settings,
        inline_case_controls,
        toolbar_leading,
        handle_projection,
        handles,
        handle_preview,
        gesture_message,
        focus_request,
    } = props;
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let mut edit_mode = use_signal(|| None::<usize>);
    let editing_gaskets = || edit_mode() == Some(0);
    let editing_mounts = || edit_mode() == Some(1);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let mut transient = use_signal(TransientView::default);
    let mut mounted = use_signal(|| false);
    let host = use_hook(|| Rc::new(RefCell::new(None::<Rc<RendererPageHost>>)));
    let canvas = use_hook(|| Rc::new(RefCell::new(None::<HtmlCanvasElement>)));
    let pointer = use_hook(|| Rc::new(RefCell::new(None::<PointerOwner>)));
    let applied = use_hook(|| Rc::new(RefCell::new(None::<ViewerIdentity>)));
    let submitted = use_hook(|| Rc::new(Cell::new(0)));
    let live = use_hook(|| Rc::new(RefCell::new((projection.clone(), current_source.clone()))));
    *live.borrow_mut() = (projection.clone(), current_source.clone());

    let interaction = super::interaction::ViewportInteraction {
        host: host.clone(),
        canvas: canvas.clone(),
        pointer: pointer.clone(),
        owner: owner.clone(),
        identity: projection.identity.clone(),
        source: current_source.clone(),
        applied,
        handles: active_case_handles(
            &handles,
            handle_preview.as_deref(),
            editing_gaskets(),
            editing_mounts(),
        ),
        handle_projection: handle_projection.clone(),
        on_signal,
        status,
    };
    use_drop({
        let interaction = interaction.clone();
        move || {
            interaction.owner.active.set(false);
            interaction.owner.identity.borrow_mut().take();
            interaction.cancel_capture();
            if let Some(host) = interaction.host.borrow_mut().take() {
                let _ = host.dispose();
            }
        }
    });
    let mount_host = {
        let runtime = runtime.clone();
        let interaction = interaction.clone();
        let submitted = submitted.clone();
        move |event: MountedEvent| {
            let Ok(element) = event
                .data()
                .try_as_web_event()
                .ok_or(())
                .and_then(|element| element.dyn_into::<HtmlCanvasElement>().map_err(|_| ()))
            else {
                interaction.fail("3D canvas could not be attached.".into());
                return;
            };
            *interaction.canvas.borrow_mut() = Some(element.clone());
            let interaction = interaction.clone();
            let live = live.clone();
            let submitted = submitted.clone();
            let runtime = runtime.clone();
            spawn(async move {
                let loaded = renderer_module().await;
                if !interaction.owner.active.get() {
                    return;
                }
                let projection = live.borrow().0.clone();
                if runtime.scope().as_ref() != Some(&projection.identity.scope) {
                    return;
                }
                let identity = projection.identity.clone();
                let report = {
                    let owner = Rc::downgrade(&interaction.owner);
                    let pointer = interaction.pointer.clone();
                    let canvas = interaction.canvas.clone();
                    let live = live.clone();
                    Rc::new(move |event| {
                        let mut mounted = mounted;
                        let mut status = status;
                        let Some(owner) = owner.upgrade().filter(|owner| owner.active.get()) else {
                            return;
                        };
                        let (projection, source) = live.borrow().clone();
                        match event {
                            RendererStatus::ContextRestored => {
                                mounted.set(true);
                                status.set("3D preview ready.".into());
                            }
                            RendererStatus::ContextLost => {
                                mounted.set(false);
                                status.set("WebGL context lost. Waiting for 3D recovery…".into());
                                if let Some(captured) = release_capture(&pointer, &canvas)
                                    && let PointerOwner::Handle { identity, id, .. } = captured
                                {
                                    emit_signal(
                                        &owner,
                                        on_signal,
                                        &identity,
                                        ViewerSignalKind::HandleGesture {
                                            phase: HandleGesturePhase::Cancel,
                                            handle_id: id,
                                            point: None,
                                        },
                                    );
                                }
                            }
                            RendererStatus::Error(error) if (source.0)() => {
                                status.set(error.clone());
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &projection.identity,
                                    ViewerSignalKind::Failed(error),
                                );
                            }
                            RendererStatus::Error(_) => {}
                        }
                    })
                };
                match loaded.and_then(|module| {
                    RendererPageHost::mount(
                        &module,
                        element,
                        projection.input.clone(),
                        identity.renderer_sequence,
                        report,
                    )
                }) {
                    Ok(host) => {
                        *interaction.host.borrow_mut() = Some(Rc::new(host));
                        submitted.set(identity.renderer_sequence);
                        *interaction.applied.borrow_mut() = Some(identity);
                        mounted.set(true);
                        status.set("3D preview ready.".into());
                    }
                    Err(error) => {
                        status.set(format!("3D preview unavailable: {error}"));
                        emit_signal(
                            &interaction.owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::Failed(error),
                        );
                    }
                }
            });
        }
    };
    use_effect(use_reactive(
        (
            &projection,
            &handle_projection,
            &display,
            &selected_layer,
            &theme,
            &transient(),
            &interaction.handles,
            &mounted(),
        ),
        {
            let interaction = interaction.clone();
            move |(
                projection,
                handle_projection,
                display,
                layer,
                theme,
                transient,
                handles,
                mounted,
            )| {
                let current = (interaction.source.0)();
                cancel_superseded_pointer(
                    &interaction.pointer,
                    &interaction.canvas,
                    &interaction.owner,
                    handle_projection.as_ref(),
                    current,
                );
                if !mounted || !current || !interaction.current() {
                    return;
                }
                let Some(host) = interaction.host.borrow().clone() else {
                    return;
                };
                let identity = &projection.identity;
                let result = (|| {
                    if submitted.get() < identity.renderer_sequence {
                        submitted.set(identity.renderer_sequence);
                        if host
                            .submit_scene(projection.input.clone(), identity.renderer_sequence)?
                        {
                            *interaction.applied.borrow_mut() = Some(identity.clone());
                        }
                    }
                    host.set_display_state(display_state(&display, &layer, &theme, transient)?)?;
                    host.set_handles(handles_value(&handles)?)
                })();
                if let Err(error) = result {
                    interaction.fail(error);
                }
            }
        },
    ));
    use_effect(use_reactive((&projection, &focus_request, &mounted()), {
        let interaction = interaction.clone();
        move |(projection, request, mounted)| {
            if !mounted || !interaction.current() || !interaction.can_select() {
                return;
            }
            let Some(host) = interaction.host.borrow().clone() else {
                return;
            };
            let request =
                request.filter(|request| focus_request_matches(request, &projection.identity));
            let ids = request
                .as_ref()
                .map_or(&[][..], |request| request.target_ids.as_slice());
            match host.focus_objects(ids) {
                Ok(false) => {
                    status.set("Finding geometry is not available in this 3D scene.".into())
                }
                Ok(true) => {}
                Err(error) => interaction.fail(error),
            }
        }
    }));

    let on_pointer_down = {
        let interaction = interaction.clone();
        move |event: dioxus::prelude::PointerEvent| {
            if let Some(event) = event.data().try_as_web_event() {
                interaction.down(&event);
            }
        }
    };
    let on_pointer_move = {
        let interaction = interaction.clone();
        move |event: dioxus::prelude::PointerEvent| {
            if let Some(event) = event.data().try_as_web_event() {
                interaction.drag(&event);
            }
        }
    };
    let finish_pointer = |phase| {
        let interaction = interaction.clone();
        move |event: dioxus::prelude::PointerEvent| {
            if let Some(event) = event.data().try_as_web_event() {
                interaction.finish(&event, phase);
            }
        }
    };
    let on_pointer_up = finish_pointer(HandleGesturePhase::End);
    let on_pointer_cancel = finish_pointer(HandleGesturePhase::Cancel);
    let on_key_down = {
        let interaction = interaction.clone();
        move |event: KeyboardEvent| {
            if let Some(event) = event.data().try_as_web_event() {
                interaction.key(&event);
            }
        }
    };
    let on_wheel = move |event: WheelEvent| {
        if let Some(event) = event.data().try_as_web_event() {
            interaction.wheel(&event);
        }
    };

    let select_layer = {
        let interaction = current_source.clone();
        let owner = owner.clone();
        let identity = projection.identity.clone();
        move |id| {
            if (interaction.0)() {
                emit_signal(
                    &owner,
                    on_signal,
                    &identity,
                    ViewerSignalKind::LayerSelected(id),
                );
            }
        }
    };
    let change_display = {
        let source = current_source.clone();
        let owner = owner.clone();
        let identity = projection.identity.clone();
        move |display| {
            if (source.0)() && owner_is_current(&owner, &identity) {
                on_display_change.call(ScopedDisplayChange {
                    identity: identity.clone(),
                    display,
                    owner: Rc::downgrade(&owner),
                });
            }
        }
    };

    let on_pointer_cancel = EventHandler::new(on_pointer_cancel);
    let on_key_down = EventHandler::new(on_key_down);
    let camera =
        EventHandler::new(move |action: CameraAction| run_host(&host, action, &mut status));
    let assembly_change_display = change_display.clone();
    let selected_support = mechanical_settings.as_ref().and_then(|settings| {
        settings.gasket_supports.iter().find(|support| {
            !support.is_previous
                && (selected_layer == format!("gasket:{}:lower", support.id)
                    || selected_layer == format!("gasket:{}:upper", support.id))
        })
    });
    let can_edit_gaskets = handles
        .iter()
        .any(|handle| matches!(handle.target, ViewerHandleTarget::Gasket { .. }));
    let can_edit_mounts = handles
        .iter()
        .any(|handle| !matches!(handle.target, ViewerHandleTarget::Gasket { .. }));
    let unlink = selected_support
        .zip(mechanical_settings.as_ref())
        .map(|(support, settings)| {
            let guard = current_source.clone();
            let owner = owner.clone();
            let identity = projection.identity.clone();
            let settings = settings.clone();
            let anchors = settings
                .gasket_supports
                .iter()
                .filter(|candidate| {
                    candidate.id == support.id
                        || candidate.pair_id.as_deref() == Some(support.id.as_str())
                })
                .map(mechanical_settings::MechanicalGasketSupportRow::saved_anchor)
                .collect();
            let patch = mechanical_settings::MechanicalSettingsPatch::SetGasketSupportUnlinked {
                support_id: support.id.clone(),
                pair_id: support.pair_id.clone(),
                anchors,
            };
            let field_id = patch.field_id();
            let feedback = settings
                .feedback
                .iter()
                .rev()
                .find(|feedback| feedback.field_id == field_id)
                .filter(|feedback| {
                    feedback.state == mechanical_settings::MechanicalSettingsFeedbackState::Failed
                })
                .map(|feedback| {
                    feedback
                        .message
                        .clone()
                        .unwrap_or_else(|| "The support could not be unlinked.".into())
                });
            let unlinked = support.unlinked;
            let disabled = !settings.editable || unlinked;
            let on_click = move |_| {
                if (guard.0)()
                    && owner_is_current(&owner, &identity)
                    && settings.identity.scope == identity.scope
                {
                    case::submit_settings_patch(&settings, patch.clone());
                }
            };
            rsx! {
                UiButton { variant: UiButtonVariant::Secondary, button_type: "button",
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    disabled, onclick: on_click, "Unlink selected support"
                }
                if let Some(message) = feedback { p { role: "alert", "{message}" } }
                if unlinked { p { "This gasket is unlinked from its pair." } }
            }
        });
    let show_assembly_controls = projection.layers.iter().any(|(id, _)| id != "pcb");
    let select_assembly_layer = select_layer.clone();

    let display_mode = view_choices(
        if inline_case_controls {
            "Display mode"
        } else {
            canvas_context.display_label()
        },
        inline_case_controls,
        transient().mode,
        [
            ("Shaded", RenderMode::Shaded),
            ("Wireframe", RenderMode::Wireframe),
            ("Hybrid", RenderMode::Hybrid),
        ],
        move |mode| transient.with_mut(|view| view.mode = mode),
    );
    let assembly_view = view_choices(
        if inline_case_controls {
            "Mechanical assembly view"
        } else {
            canvas_context.assembly_label()
        },
        inline_case_controls,
        transient().assembly,
        [
            ("Assembled", AssemblyView::Assembled),
            ("Exploded", AssemblyView::Exploded),
            ("Section", AssemblyView::Section),
        ],
        move |assembly| transient.with_mut(|view| view.assembly = assembly),
    );
    let section_plane = use_memo(move || Some(transient().section_plane.to_owned()));
    let assembly_adjustments = assembly_adjustments(transient, section_plane, inline_case_controls);
    let docked_layers = boardstudio_web_ui_shared::layer_dock::use_docked_layers(rsx! {
        CaseAssemblyLayers {
            initially_open: false,
            assembly: assembly_layers,
            components: component_layers,
            selectable_layers: selectable_layers.clone(),
            selected_layer: selected_layer.clone(),
            on_select_layer: select_assembly_layer,
            display: display.clone(),
            on_display_change: assembly_change_display,
        }
    if !inline_case_controls {
    Collapsible { keep_mounted: true,
        CollapsibleTrigger { "Case layers and colors" }
        CollapsibleContent { class: "data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11",
        div { role: "group", "aria-label": "Case layers",
        for (id, label) in &projection.layers {
            {
            let id = id.clone();
            let label = label.clone();
            let ids = preference_ids(&id);
            let visible = !ids.iter().all(|entry| display.hidden.contains(entry));
            let color = display.color(&id).unwrap_or("#b0b5bd").to_owned();
            let has_color = display.has_color(&id);
            let toggle_next = display.clone();
            let color_next = display.clone();
            let reset_next = display.clone();
            let toggle_display = change_display.clone();
            let select_layer = select_layer.clone();
            let set_color = change_display.clone();
            let reset_color = change_display.clone();
            let select_id = id.clone();
            let toggle_id = id.clone();
            let color_id = id.clone();
            let reset_id = id.clone();
            rsx! {
                div { class: "flex flex-wrap items-center gap-1.5 border-b border-border py-2 [&_label]:flex [&_label]:gap-1.5 [&_label]:text-muted-foreground [&_input[type=color]]:w-11 [&_input[type=color]]:p-[3px]", key: "{id}",
                    UiButton { variant: UiButtonVariant::Ghost,
                        aria_pressed: selected_layer == id,
                        onclick: move |_| select_layer(select_id.clone()),
                        "Select {label}"
                    }
                    ToggleGroup {
                        aria_label: format!("Visibility for {label}"), horizontal: true,
                        pressed: Some(if visible { [0].into_iter().collect() } else { Default::default() }),
                        on_pressed_change: move |_| {
                            let mut next_display = toggle_next.clone();
                            let aliases = preference_ids(&toggle_id);
                            let is_hidden = aliases.iter().all(|entry| next_display.hidden.contains(entry));
                            if is_hidden {
                                next_display.hidden.retain(|entry| !aliases.contains(entry));
                            } else {
                                for entry in aliases {
                                    if !next_display.hidden.contains(&entry) { next_display.hidden.push(entry); }
                                }
                            }
                            toggle_display(next_display.clone());
                        },
                        ToggleItem { index: 0usize, aria_label: format!("Toggle visibility for {label}"),
                            if visible { "Visible" } else { "Hidden" }
                        }
                    }
                    Field {
                        Label { html_for: format!("viewer-color-{}-{id}", dioxus::core::current_scope_id().0), "Color for {label}" }
                        InputGroup {
                            InputGroupInput {
                                id: format!("viewer-color-{}-{id}", dioxus::core::current_scope_id().0),
                                aria_label: format!("Color for {label}"),
                                r#type: InputType::Color, value: "{color}",
                                oninput: move |event: FormEvent| {
                                    let mut next_display = color_next.clone();
                                    next_display.set_color(&color_id, &event.value());
                                    set_color(next_display.clone());
                                }
                            }
                        }
                    }
                    UiButton { variant: UiButtonVariant::Secondary,
                        disabled: !has_color,
                        onclick: move |_| {
                            let mut next_display = reset_next.clone();
                            next_display.set_color(&reset_id, "");
                            reset_color(next_display.clone());
                        },
                        "Reset color"
                    }
                }
            }
            }
        }
        }
    }
    }
    }    });

    rsx! {
        div { class: "relative flex min-h-0 flex-1 flex-col gap-1.5 overflow-hidden",
            style: (!inline_case_controls).then_some("container-type: size"),
            div { class: if inline_case_controls { "relative z-[5] m-3 flex w-max max-w-[calc(100%-24px)] flex-none flex-wrap items-center self-start gap-1 overflow-visible rounded-2xl bg-toolbar p-[5px] shadow-[0_6px_18px_rgb(0_0_0/16%)] [&_[role=group]]:m-0 [&_[role=group]]:flex-none [&_[role=group]]:gap-0.5 [&_[role=group]]:p-0" } else { "relative flex flex-none flex-wrap items-center justify-center gap-1 rounded-md bg-toolbar p-1" },
                {toolbar_leading}
                if inline_case_controls {
                    div { class: "static flex flex-wrap items-center gap-0.5 border-0 bg-transparent p-0", role: "group", "aria-label": "Assembly camera",
                        {camera_button("Fit", "Fit", "Fit model", camera, RendererPageHost::fit)}
                        for (name, label, action) in CAMERA_VIEWS {
                            {camera_button(name, name, label, camera, action)}
                        }
                    }
                }
                if inline_case_controls {
                    {display_mode.clone()}
                    ToggleGroup {
                        aria_label: "Hidden lines",
                        pressed: Some(if transient().show_hidden { [0].into_iter().collect() } else { Default::default() }),
                        on_pressed_change: move |next: std::collections::HashSet<usize>| {
                            if next.contains(&0) != transient().show_hidden {
                                transient.with_mut(|view| view.show_hidden = !view.show_hidden);
                            }
                        },
                        Tooltip {
                            TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                                ToggleItem { index: 0usize, aria_label: "Show hidden lines",
                                    class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                    attributes,
                                    svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { d: "m12 2 9 5v10l-9 5-9-5V7ZM3 7l9 5 9-5M12 12v10" }
                                        path { stroke_dasharray: "2 2", d: "M12 2v10M3 17l9-5 9 5" }
                                    }
                                }
                            } }
                            TooltipContent { "Show hidden lines" }
                        }
                    }
                    if show_assembly_controls { {assembly_view.clone()} }
                }
                if can_edit_gaskets || can_edit_mounts {
                    ToggleGroup {
                        aria_label: "Edit mode",
                        pressed: Some(edit_mode().into_iter().collect()),
                        on_pressed_change: move |next: std::collections::HashSet<usize>| {
                            let next = next.into_iter().min();
                            if next != edit_mode() {
                                edit_mode.set(next);
                                if next.is_some() { camera.call(CAMERA_VIEWS[0].2); }
                            }
                        },
                        if can_edit_gaskets { {icon_toggle(0, "Edit gaskets")} }
                        if can_edit_mounts { {icon_toggle(1, "Edit mounts")} }
                    }
                }
                if editing_gaskets() {
                    if let Some(unlink) = unlink { {unlink} } else {
                        p { "Select a gasket in Objects to unlink its mirrored support pair." }
                    }
                }
                if !inline_case_controls {
                {camera_button(canvas_context.fit_label(), "Fit", canvas_context.fit_label(), camera, RendererPageHost::fit)}
                Collapsible { keep_mounted: true,
                    Tooltip {
                        TooltipTrigger { as: move |attributes| rsx! {
                            CollapsibleTrigger { aria_label: "View controls", class: "size-control-compact justify-center max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]", attributes, {viewer_control_icon("View controls")} }
                        } }
                        TooltipContent { "View controls" }
                    }
                    CollapsibleContent { class: "absolute left-0 top-full z-50 mt-2 max-h-[min(60vh,max(0px,calc(100cqh-5rem)))] w-[min(320px,calc(100vw-32px))] overflow-auto rounded-md bg-card p-3 shadow-md data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11 max-[960px]:[&_[data-slot=select-trigger]]:min-h-11",
            div { role: "group", "aria-label": "{canvas_context.camera_label()}",
                for (_, label, action) in CAMERA_VIEWS {
                    UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                }
                for (label, action) in CAMERA_NAVIGATION {
                    UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                }
            }
            {display_mode}
            {assembly_view}
            {assembly_adjustments.clone()}
            Label { html_for: format!("viewer-show-hidden-{}", dioxus::core::current_scope_id().0), class: "max-[960px]:min-h-11",
                UiCheckbox { id: format!("viewer-show-hidden-{}", dioxus::core::current_scope_id().0), aria_label: "Show hidden lines",
                    checked: Some(if transient().show_hidden { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                    on_checked_change: move |value| transient.with_mut(|state| state.show_hidden = bool::from(value)),
                }
                "Show hidden lines"
            }
                    }
                }
                }
            }
            if inline_case_controls && show_assembly_controls {
                {assembly_adjustments}
            }
            div { class: "relative flex min-h-[160px] min-w-0 flex-1 [&>canvas]:h-full [&>canvas]:min-h-0 [&>canvas]:min-w-0 [&>canvas]:w-full [&>canvas]:flex-1 [&>canvas]:border-0",
                if let Some(reference) = selected_reference.as_deref() {
                    div { class: "pointer-events-none absolute bottom-14 left-3 z-[2] max-w-[min(70%,320px)] overflow-hidden truncate rounded-sm border border-border bg-card px-[9px] py-[7px] font-mono text-sm leading-[1.45] text-foreground", "aria-hidden": "true", "{reference}" }
                }
                canvas {
                    style: "width:100%;height:100%;display:block",
                    tabindex: "0", role: "img",
                    "aria-label": "{canvas_context.accessible_name()}",
                    "data-case-viewer-canvas": (canvas_context == ViewerCanvasContext::Case).then_some("true"),
                    onmounted: mount_host,
                    onpointerdown: on_pointer_down,
                    onpointermove: on_pointer_move,
                    onpointerup: on_pointer_up,
                    onpointercancel: on_pointer_cancel,
                    onlostpointercapture: on_pointer_cancel,
                    onkeydown: on_key_down,
                    onwheel: on_wheel,
                }
                if let Some(message) = gesture_message.as_deref() {
                    p { class: "pointer-events-none absolute top-14 left-3 z-[3] m-0 max-w-[calc(100%-24px)] rounded-sm border border-border bg-card px-2 py-1.5 text-foreground", role: "status", "aria-label": "Case edit guidance", "{message}" }
                }
                if !status().is_empty() && !matches!(status().as_str(), "3D preview updated." | "3D preview ready." | "Starting the 3D renderer…" | "A stale 3D scene update was ignored.") {
                    p { class: "pointer-events-none absolute top-3 left-3 z-[3] m-0 max-w-[min(70%,420px)] rounded-md bg-card px-[7px] py-1 text-sm text-muted-foreground [&[role=alert]]:pointer-events-auto [&[role=alert]]:text-destructive", role: if status().to_ascii_lowercase().contains("unavailable") || status().contains("failed") { "alert" } else { "status" }, "aria-live": "polite", "{status()}" }
                }
            }
            {docked_layers}

        }
    }
}

type CameraAction = fn(&RendererPageHost) -> Result<(), String>;

const CAMERA_VIEWS: [(&str, &str, CameraAction); 3] = [
    ("Top", "Top view", |host| host.view("top")),
    ("Bottom", "Bottom view", |host| host.view("bottom")),
    ("Isometric", "Isometric view", |host| host.view("isometric")),
];

const CAMERA_NAVIGATION: [(&str, CameraAction); 4] = [
    ("Rotate left", |host| host.orbit(-50.0, 0.0)),
    ("Rotate right", |host| host.orbit(50.0, 0.0)),
    ("Zoom in", |host| host.zoom(0.85)),
    ("Zoom out", |host| host.zoom(1.15)),
];

const ICON_CONTROL: &str = "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]";

fn camera_button(
    name: &'static str,
    icon: &'static str,
    tooltip: &'static str,
    camera: EventHandler<CameraAction>,
    action: CameraAction,
) -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { as: move |attributes| rsx! {
                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                    class: ICON_CONTROL, aria_label: name, onclick: move |_| camera.call(action),
                    {viewer_control_icon(icon)}
                }
            } }
            TooltipContent { "{tooltip}" }
        }
    }
}

fn view_choices<T: Copy + PartialEq + 'static>(
    label: &'static str,
    inline: bool,
    selected: T,
    choices: [(&'static str, T); 3],
    on_change: impl FnMut(T) + 'static,
) -> Element {
    let mut on_change = on_change;
    let pressed = choices.iter().position(|(_, choice)| *choice == selected);
    rsx! {
        ToggleGroup {
            aria_label: label, horizontal: true,
            pressed: Some(pressed.into_iter().collect()),
            on_pressed_change: move |next: std::collections::HashSet<usize>| {
                if let Some((_, choice)) = next.iter().next().and_then(|index| choices.get(*index)) {
                    on_change(*choice);
                }
            },
            for (index, (name, _)) in choices.into_iter().enumerate() {
                if inline {
                    {icon_toggle(index, name)}
                } else {
                    ToggleItem { index, "{name.to_ascii_lowercase()}" }
                }
            }
        }
    }
}

fn icon_toggle(index: usize, name: &'static str) -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                ToggleItem { index, aria_label: name, class: ICON_CONTROL, attributes,
                    {viewer_control_icon(name)}
                }
            } }
            TooltipContent { "{name}" }
        }
    }
}

fn assembly_adjustments(
    mut transient: Signal<TransientView>,
    section_plane: Memo<Option<String>>,
    inline: bool,
) -> Element {
    const SLIDER_CLASS: &str = "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1";
    if transient().assembly == AssemblyView::Exploded {
        return rsx! {
            Slider {
                label: "Explode amount", min: 0.0, max: 10.0, step: 0.1, class: SLIDER_CLASS,
                value: Some(f64::from(transient().explode_amount)),
                on_value_change: move |value: f64| transient.with_mut(|state| state.explode_amount = (value as f32).clamp(0.0, 10.0)),
            }
        };
    }
    if transient().assembly != AssemblyView::Section {
        return rsx! {};
    }
    let show_plane = if inline {
        "Show plane"
    } else {
        "Show section plane"
    };
    let section = rsx! {
        Field {
            Label { html_for: format!("viewer-section-plane-{}", dioxus::core::current_scope_id().0), "Section plane" }
            Select::<String> {
                id: format!("viewer-section-plane-{}", dioxus::core::current_scope_id().0), aria_label: "Section plane",
                value: Some(section_plane.into()),
                on_value_change: move |value: Option<String>| {
                    if let Some(value) = value {
                        transient.with_mut(|state| state.section_plane = match value.as_str() { "XY" => "XY", "XZ" => "XZ", _ => "YZ" });
                    }
                },
                for (index, plane) in ["XY", "XZ", "YZ"].into_iter().enumerate() {
                    SelectOption::<String> { index, value: plane.to_owned(), text_value: plane, "{plane}" }
                }
            }
        }
        Slider {
            label: "Section position", min: -100.0, max: 100.0, step: 1.0, class: SLIDER_CLASS,
            value: Some(f64::from(transient().section_position)),
            on_value_change: move |value: f64| transient.with_mut(|state| state.section_position = (value as f32).clamp(-100.0, 100.0)),
        }
        Label { html_for: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), class: "max-[960px]:min-h-11",
            UiCheckbox { id: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), aria_label: show_plane,
                checked: Some(if transient().show_section_plane { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                on_checked_change: move |value| transient.with_mut(|state| state.show_section_plane = bool::from(value)),
            }
            "{show_plane}"
        }
    };
    if inline {
        rsx! { div { class: "flex flex-wrap items-center gap-3 border-b border-border px-2.5 py-1.5", {section} } }
    } else {
        section
    }
}
