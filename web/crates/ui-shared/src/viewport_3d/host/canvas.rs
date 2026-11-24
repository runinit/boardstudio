use super::controls::{CameraAction, ViewerChrome};
use super::interaction::{
    PointerOwner, ViewportInteraction, cancel_handle_capture, cancel_superseded_pointer,
};
use super::*;
use boardstudio_web_host::renderer_host::{RendererStatus, renderer_module};

#[derive(Props, Clone, PartialEq)]
pub(super) struct ViewportCanvas {
    pub projection: Rc<RendererSceneProjection>,
    pub overlay: ViewerOverlayConfig,
    pub canvas_context: ViewerCanvasContext,
    pub theme: String,
    pub owner: Rc<ViewerOwner>,
    pub current_source: SourceGuard,
    pub on_signal: EventHandler<ScopedViewerSignal>,
    pub on_display_change: EventHandler<ScopedDisplayChange>,
    pub handle_projection: Option<CaseHandleProjection>,
    pub edit_mode: Option<Signal<Option<usize>>>,
    pub handle_preview: Option<Vec<ViewportHandle>>,
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

    let local_edit_mode = use_signal(|| None::<usize>);
    let edit_mode = props.edit_mode.unwrap_or(local_edit_mode);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let transient = use_signal(TransientView::default);
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
        handles: props
            .handle_preview
            .as_deref()
            .unwrap_or(&props.overlay.handles)
            .iter()
            .filter(|handle| edit_mode() == Some(handle.edit_group))
            .cloned()
            .collect(),
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
            if (interaction.source.0)() {
                interaction
                    .owner
                    .emit(on_display_change, &interaction.identity, (), display);
            }
        }
    });
    let camera = EventHandler::new(move |action: CameraAction| {
        if let Some(host) = host.borrow().as_ref()
            && let Err(error) = action(host)
        {
            status.set(error);
        }
    });
    rsx! {
        ViewerChrome {
            overlay: props.overlay,
            context: props.canvas_context,
            transient,
            edit_mode,
            status,
            camera,
            select_layer,
            change_display,
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
        }
    }
}

fn use_renderer_mount(
    interaction: &Rc<ViewportInteraction>,
    projection: &Rc<RendererSceneProjection>,
    mut mounted: Signal<bool>,
    submitted: Rc<Cell<u64>>,
) -> impl Fn(MountedEvent) + 'static {
    let runtime = use_context::<Rc<Runtime>>();
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
                            if let Some((identity, id)) = cancel_handle_capture(&pointer, &canvas) {
                                owner.emit(
                                    on_signal,
                                    &identity,
                                    ViewerSignalKind::HandleGesture {
                                        phase: HandleGesturePhase::Cancel,
                                        handle_id: id,
                                        point: None,
                                    },
                                    (),
                                );
                            }
                        }
                        RendererStatus::Error(error) if (source.0)() => {
                            status.set(error.clone());
                            owner.emit(
                                on_signal,
                                &projection.identity,
                                ViewerSignalKind::Failed(error),
                                (),
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
                    interaction.owner.emit(
                        on_signal,
                        &identity,
                        ViewerSignalKind::Failed(error),
                        (),
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
            &props.overlay.display,
            &props.overlay.selected_layer,
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
                let identity = &projection.identity;
                let request = request.filter(|request| {
                    request.scope == identity.scope
                        && request.snapshot_token == identity.snapshot_token
                        && request.revision == identity.revision
                        && !request.target_ids.is_empty()
                });
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
