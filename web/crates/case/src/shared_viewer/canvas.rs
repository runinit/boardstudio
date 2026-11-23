use super::interaction::ViewportInteraction;
use super::*;
use crate::case_handles::active_case_handles;
use crate::case_viewport::controls::{
    edit_controls, inline_view_controls, layer_controls, use_assembly_adjustments,
};
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

    let edit_mode = use_signal(|| None::<usize>);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let mut transient = use_signal(TransientView::default);
    let mounted = use_signal(|| false);
    let host = use_hook(|| Rc::new(RefCell::new(None::<Rc<RendererPageHost>>)));
    let canvas = use_hook(|| Rc::new(RefCell::new(None::<HtmlCanvasElement>)));
    let pointer = use_hook(|| Rc::new(RefCell::new(None::<PointerOwner>)));
    let applied = use_hook(|| Rc::new(RefCell::new(None::<ViewerIdentity>)));
    let submitted = use_hook(|| Rc::new(Cell::new(0)));
    let interaction = Rc::new(ViewportInteraction {
        host: host.clone(),
        canvas,
        pointer,
        owner: props.owner.clone(),
        identity: props.projection.identity.clone(),
        source: props.current_source.clone(),
        applied,
        handles: active_case_handles(&props.handles, props.handle_preview.as_deref(), edit_mode()),
        handle_projection: props.handle_projection.clone(),
        on_signal: props.on_signal,
        status,
    });
    let mount_host =
        use_renderer_mount(&interaction, &props.projection, mounted, submitted.clone());
    use_renderer_updates(&props, transient(), mounted(), &interaction, submitted);

    let on_pointer_cancel = EventHandler::new(
        interaction.bind_event(|view, event| view.finish(event, HandleGesturePhase::Cancel)),
    );
    let select_layer = EventHandler::new({
        let interaction = interaction.clone();
        move |id| {
            if (interaction.source.0)() {
                interaction.emit(ViewerSignalKind::LayerSelected(id));
            }
        }
    });
    let on_display_change = props.on_display_change;
    let change_display = EventHandler::new({
        let interaction = interaction.clone();
        move |display| {
            if (interaction.source.0)() && interaction.current() {
                on_display_change.call(ScopedDisplayChange {
                    identity: interaction.identity.clone(),
                    display,
                    owner: Rc::downgrade(&interaction.owner),
                });
            }
        }
    });
    let camera =
        EventHandler::new(move |action: CameraAction| run_host(&host, action, &mut status));
    let accepts_case_edit = {
        let interaction = interaction.clone();
        move |scope: &Scope| {
            (interaction.source.0)()
                && interaction.current()
                && scope == &interaction.identity.scope
        }
    };
    let case_edit_controls = edit_controls(
        &props.selected_layer,
        props.mechanical_settings.as_ref(),
        &props.handles,
        edit_mode,
        camera,
        accepts_case_edit,
    );
    let show_assembly_controls = props.projection.layers.iter().any(|(id, _)| id != "pcb");

    let display_mode = view_choices(
        if props.inline_case_controls {
            "Display mode".to_owned()
        } else {
            format!("{} display mode", props.canvas_context.name())
        },
        props.inline_case_controls,
        transient().mode,
        [
            ("Shaded", RenderMode::Shaded),
            ("Wireframe", RenderMode::Wireframe),
            ("Hybrid", RenderMode::Hybrid),
        ],
        move |mode| transient.with_mut(|view| view.mode = mode),
    );
    let assembly_view = view_choices(
        if props.inline_case_controls {
            "Mechanical assembly view".to_owned()
        } else if props.canvas_context == ViewerCanvasContext::Parts {
            "Parts sample view".to_owned()
        } else {
            format!("{} assembly view", props.canvas_context.name())
        },
        props.inline_case_controls,
        transient().assembly,
        [
            ("Assembled", AssemblyView::Assembled),
            ("Exploded", AssemblyView::Exploded),
            ("Section", AssemblyView::Section),
        ],
        move |assembly| transient.with_mut(|view| view.assembly = assembly),
    );
    let assembly_adjustments = use_assembly_adjustments(transient, props.inline_case_controls);
    let docked_layers = boardstudio_web_ui_shared::layer_dock::use_docked_layers(rsx! {
        CaseAssemblyLayers {
            initially_open: false,
            assembly: props.assembly_layers,
            components: props.component_layers,
            selectable_layers: props.selectable_layers,
            selected_layer: props.selected_layer.clone(),
            on_select_layer: select_layer,
            display: props.display.clone(),
            on_display_change: change_display,
        }
        if !props.inline_case_controls {
            {layer_controls(&props.projection.layers, &props.selected_layer, &props.display, select_layer, change_display)}
        }
    });

    rsx! {
        div { class: "relative flex min-h-0 flex-1 flex-col gap-1.5 overflow-hidden",
            style: (!props.inline_case_controls).then_some("container-type: size"),
            div { class: if props.inline_case_controls { "relative z-[5] m-3 flex w-max max-w-[calc(100%-24px)] flex-none flex-wrap items-center self-start gap-1 overflow-visible rounded-2xl bg-toolbar p-[5px] shadow-[0_6px_18px_rgb(0_0_0/16%)] [&_[role=group]]:m-0 [&_[role=group]]:flex-none [&_[role=group]]:gap-0.5 [&_[role=group]]:p-0" } else { "relative flex flex-none flex-wrap items-center justify-center gap-1 rounded-md bg-toolbar p-1" },
                {props.toolbar_leading}
                if props.inline_case_controls { {inline_view_controls(camera, transient, display_mode.clone(), assembly_view.clone(), show_assembly_controls)} }
                {case_edit_controls}
                if !props.inline_case_controls {
                    {compact_view_controls(props.canvas_context, camera, transient, display_mode, assembly_view, assembly_adjustments.clone())}
                }
            }
            if props.inline_case_controls && show_assembly_controls {
                {assembly_adjustments}
            }
            div { class: "relative flex min-h-[160px] min-w-0 flex-1 [&>canvas]:h-full [&>canvas]:min-h-0 [&>canvas]:min-w-0 [&>canvas]:w-full [&>canvas]:flex-1 [&>canvas]:border-0",
                if let Some(reference) = props.selected_reference.as_deref() {
                    div { class: "pointer-events-none absolute bottom-14 left-3 z-[2] max-w-[min(70%,320px)] overflow-hidden truncate rounded-sm border border-border bg-card px-[9px] py-[7px] font-mono text-sm leading-[1.45] text-foreground", "aria-hidden": "true", "{reference}" }
                }
                canvas {
                    style: "width:100%;height:100%;display:block",
                    tabindex: "0", role: "img",
                    "aria-label": "{props.canvas_context.accessible_name()}",
                    "data-case-viewer-canvas": (props.canvas_context == ViewerCanvasContext::Case).then_some("true"),
                    onmounted: mount_host,
                    onpointerdown: interaction.bind_event(ViewportInteraction::down),
                    onpointermove: interaction.bind_event(ViewportInteraction::drag),
                    onpointerup: interaction.bind_event(|view, event| view.finish(event, HandleGesturePhase::End)),
                    onpointercancel: on_pointer_cancel,
                    onlostpointercapture: on_pointer_cancel,
                    onkeydown: interaction.bind_event(ViewportInteraction::key),
                    onwheel: interaction.bind_event(ViewportInteraction::wheel),
                }
                if let Some(message) = props.gesture_message.as_deref() {
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

fn use_renderer_mount(
    interaction: &Rc<ViewportInteraction>,
    projection: &Rc<RendererSceneProjection>,
    mut mounted: Signal<bool>,
    submitted: Rc<Cell<u64>>,
) -> impl Fn(MountedEvent) + 'static {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let mut status = interaction.status;
    let on_signal = interaction.on_signal;
    let latest = || (projection.clone(), interaction.source.clone());
    let live = use_hook(|| Rc::new(RefCell::new(latest())));
    *live.borrow_mut() = latest();

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
    let interaction = interaction.clone();
    move |event: MountedEvent| {
        let Some(element) = event
            .data()
            .try_as_web_event()
            .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
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
}

fn use_renderer_updates(
    props: &ViewportCanvas,
    transient: TransientView,
    mounted: bool,
    interaction: &Rc<ViewportInteraction>,
    submitted: Rc<Cell<u64>>,
) {
    let mut status = interaction.status;
    use_effect(use_reactive(
        (
            &props.projection,
            &interaction.handle_projection,
            &props.display,
            &props.selected_layer,
            &props.theme,
            &transient,
            &interaction.handles,
            &mounted,
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
    use_effect(use_reactive(
        (&props.projection, &props.focus_request, &mounted),
        {
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
        },
    ));
}

fn compact_view_controls(
    context: ViewerCanvasContext,
    camera: EventHandler<CameraAction>,
    mut transient: Signal<TransientView>,
    display_mode: Element,
    assembly_view: Element,
    adjustments: Element,
) -> Element {
    rsx! {
        {camera_button(context.fit_label(), "Fit", context.fit_label(), camera, RendererPageHost::fit)}
        Collapsible { keep_mounted: true,
            Tooltip {
                TooltipTrigger { as: move |attributes| rsx! {
                        CollapsibleTrigger { aria_label: "View controls", class: "size-control-compact justify-center max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]", attributes, {viewer_control_icon("View controls")} }
                    } }
                TooltipContent { "View controls" }
            }
            CollapsibleContent { class: "absolute left-0 top-full z-50 mt-2 max-h-[min(60vh,max(0px,calc(100cqh-5rem)))] w-[min(320px,calc(100vw-32px))] overflow-auto rounded-md bg-card p-3 shadow-md data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11 max-[960px]:[&_[data-slot=select-trigger]]:min-h-11",
                div { role: "group", "aria-label": format!("{} camera", context.name()),
                    for (_, label, action) in CAMERA_VIEWS {
                        UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                    }
                    for (label, action) in CAMERA_NAVIGATION {
                        UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| camera.call(action), "{label}" }
                    }
                }
                {display_mode}
                {assembly_view}
                {adjustments}
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

pub(crate) type CameraAction = fn(&RendererPageHost) -> Result<(), String>;

pub(crate) const CAMERA_VIEWS: [(&str, &str, CameraAction); 3] = [
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

pub(crate) fn camera_button(
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
    label: String,
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

pub(crate) fn icon_toggle(index: usize, name: &'static str) -> Element {
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
