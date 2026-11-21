use super::*;
use crate::mechanical_settings;

#[component]
pub(super) fn SharedViewer(
    projection: Rc<RendererSceneProjection>,
    assembly_layers: Vec<crate::presentation::case_assembly_layers::CaseAssemblyLayer>,
    component_layers: Vec<crate::presentation::case_assembly_layers::CaseComponentLayer>,
    selectable_layers: Vec<String>,
    canvas_context: ViewerCanvasContext,
    selected_layer: String,
    selected_reference: Option<String>,
    display: CaseDisplay,
    theme: String,
    owner: Rc<ViewerOwner>,
    current_source: SourceGuard,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
    mechanical_settings: Option<crate::presentation::MechanicalSettingsProps>,
    inline_case_controls: bool,
    #[props(default)] toolbar_leading: Option<Element>,
    handle_projection: Option<CaseHandleProjection>,
    handles: Vec<ViewerHandle>,
    handle_preview: Option<Vec<ViewerHandle>>,
    gesture_message: Option<String>,
    focus_request: Option<ViewerFocusRequest>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let mut editing_gaskets = use_signal(|| false);
    let mut editing_mounts = use_signal(|| false);
    let host = use_hook(|| Rc::new(RefCell::new(None::<RendererPageHost>)));
    let canvas = use_hook(|| Rc::new(RefCell::new(None::<HtmlCanvasElement>)));
    let live_projection = use_hook(|| Rc::new(RefCell::new(projection.clone())));
    *live_projection.borrow_mut() = projection.clone();
    let live_source = use_hook(|| Rc::new(RefCell::new(current_source.clone())));
    *live_source.borrow_mut() = current_source.clone();
    let mounted = use_signal(|| false);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let mut transient = use_signal(TransientView::default);
    let pointer = use_hook(|| Rc::new(RefCell::new(None::<PointerOwner>)));
    let applied_sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let applied_identity = use_hook(|| Rc::new(RefCell::new(None::<ViewerIdentity>)));
    #[cfg(all(test, target_arch = "wasm32"))]
    if let Some(probe) = try_consume_context::<SharedViewerEscapeProbe>() {
        let pointer = pointer.clone();
        let identity = projection.identity.clone();
        let handle_projection = handle_projection.clone();
        *probe.0.borrow_mut() = Some(Rc::new(move |handle_id| {
            *pointer.borrow_mut() = Some(PointerOwner::Handle {
                identity: identity.clone(),
                projection: handle_projection.clone(),
                pointer_id: 13,
                z: 0.8,
                id: handle_id,
            });
        }));
    }

    use_effect(use_reactive((&projection, &handle_projection), {
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let owner = owner.clone();
        let live_source = live_source.clone();
        move |(_projection, handle_projection)| {
            cancel_superseded_pointer(
                &pointer,
                &canvas,
                &owner,
                handle_projection.as_ref(),
                (live_source.borrow().0)(),
            );
        }
    }));

    use_drop({
        let host = host.clone();
        let owner = owner.clone();
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move || {
            owner.active.set(false);
            if let Some(pointer_id) = pointer
                .borrow_mut()
                .take()
                .map(|active| active.pointer_id())
            {
                release_pointer_capture(&canvas, pointer_id);
            }
            if let Some(host) = host.borrow_mut().take() {
                let _ = host.dispose();
            }
        }
    });

    use_effect(use_reactive((&projection, &mounted()), {
        let host = host.clone();
        let applied_sequence = applied_sequence.clone();
        let applied_identity = applied_identity.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, mounted)| {
            if !mounted || !owner.active.get() || !(current_source.0)() {
                return;
            }
            let identity = projection.identity.clone();
            if owner.identity.borrow().as_ref() != Some(&identity) {
                return;
            }
            if applied_sequence.get() >= identity.renderer_sequence {
                return;
            }
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| {
                    host.submit_scene(projection.input.clone(), identity.renderer_sequence)
                });
            match result {
                Ok(accepted) => {
                    applied_sequence.set(identity.renderer_sequence);
                    if accepted {
                        *applied_identity.borrow_mut() = Some(identity.clone());
                    }
                    if owner_is_current(&owner, &identity) && (current_source.0)() {
                        status.set(if accepted {
                            "3D preview updated.".to_owned()
                        } else {
                            "A stale 3D scene update was ignored.".to_owned()
                        });
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::SceneAccepted(accepted),
                        );
                    }
                }
                Err(error) => {
                    if owner_is_current(&owner, &identity) && (current_source.0)() {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::Failed(error),
                        );
                    }
                }
            }
        }
    }));

    use_effect(use_reactive(
        (
            &projection,
            &selected_layer,
            &display,
            &theme,
            &transient(),
            &mounted(),
        ),
        {
            let host = host.clone();
            let owner = owner.clone();
            let current_source = current_source.clone();
            let on_signal = on_signal;
            let mut status = status;
            move |(projection, selected_layer, display, theme, transient, mounted)| {
                if !mounted
                    || !owner.active.get()
                    || owner.identity.borrow().as_ref() != Some(&projection.identity)
                {
                    return;
                }
                let result =
                    display_state(&display, &selected_layer, &theme, transient).and_then(|state| {
                        host.borrow()
                            .as_ref()
                            .ok_or_else(|| "3D renderer is not mounted".to_owned())
                            .and_then(|host| host.set_display_state(state))
                    });
                if let Err(error) = result
                    && owner_is_current(&owner, &projection.identity)
                    && (current_source.0)()
                {
                    status.set(error.clone());
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        },
    ));

    use_effect(use_reactive((&projection, &focus_request, &mounted()), {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, request, mounted)| {
            if !mounted
                || !owner.active.get()
                || owner.identity.borrow().as_ref() != Some(&projection.identity)
                || !(current_source.0)()
            {
                return;
            }
            let target_ids = request
                .filter(|request| focus_request_matches(request, &projection.identity))
                .map_or_else(Vec::new, |request| request.target_ids.clone());
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| host.focus_objects(&target_ids));
            match result {
                Ok(true) => {}
                Ok(false) => {
                    status.set("Finding geometry is not available in this 3D scene.".into())
                }
                Err(error) => {
                    status.set(error.clone());
                    if owner_is_current(&owner, &projection.identity) && (current_source.0)() {
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                    }
                }
            }
        }
    }));

    use_effect(use_reactive(
        (
            &projection,
            &mounted(),
            &editing_gaskets(),
            &editing_mounts(),
            &handles,
            &handle_preview,
        ),
        {
            let host = host.clone();
            let owner = owner.clone();
            let current_source = current_source.clone();
            let on_signal = on_signal;
            let mut status = status;
            move |(projection, mounted, edit_gaskets, edit_mounts, handles, preview)| {
                if !mounted
                    || !owner.active.get()
                    || !(current_source.0)()
                    || owner.identity.borrow().as_ref() != Some(&projection.identity)
                {
                    return;
                }
                let active =
                    active_case_handles(&handles, preview.as_deref(), edit_gaskets, edit_mounts);
                let result = handles_value(&active).and_then(|handles| {
                    host.borrow()
                        .as_ref()
                        .ok_or_else(|| "3D renderer is not mounted".to_owned())
                        .and_then(|host| host.set_handles(handles).map(|_| ()))
                });
                if let Err(error) = result
                    && owner_is_current(&owner, &projection.identity)
                    && (current_source.0)()
                {
                    status.set(error.clone());
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        },
    ));

    let mount_host = super::mount::RendererMount {
        projection: projection.clone(),
        host: host.clone(),
        canvas_state: canvas.clone(),
        owner: owner.clone(),
        runtime: runtime.clone(),
        live_projection: live_projection.clone(),
        live_source: live_source.clone(),
        applied_identity: applied_identity.clone(),
        on_signal,
        status,
        applied_sequence: applied_sequence.clone(),
        mounted,
    }
    .handler();

    let active_handles = active_case_handles(
        &handles,
        handle_preview.as_deref(),
        editing_gaskets(),
        editing_mounts(),
    );
    let on_pointer_down = {
        let handle_projection = handle_projection.clone();
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let handles = active_handles.clone();
        let applied_identity = applied_identity.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) || pointer.borrow().is_some() {
                return;
            }
            event.prevent_default();
            let (x, y) = pointer_point(&event);
            let host_guard = host.borrow();
            let Some(host) = host_guard.as_ref() else {
                return;
            };
            let picked = if (current_source.0)()
                && applied_identity.borrow().as_ref() == Some(&projection.identity)
            {
                match host.pick_at_client(x, y) {
                    Ok(id) => id.filter(|id| !id.is_empty()),
                    Err(error) => {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                        None
                    }
                }
            } else {
                None
            };
            if let Some(handle) = picked
                .as_ref()
                .and_then(|id| handles.iter().find(|handle| &handle.id == id))
            {
                let point = match host.point_on_plane_at_client(x, y, handle.z) {
                    Ok(point) => point,
                    Err(error) => {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                        return;
                    }
                };
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::WorldPoint(point),
                );
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::HandleGesture {
                        phase: HandleGesturePhase::Start,
                        handle_id: handle.id.clone(),
                        point,
                    },
                );
                *pointer.borrow_mut() = Some(PointerOwner::Handle {
                    projection: handle_projection.clone(),
                    identity: projection.identity.clone(),
                    pointer_id: event.pointer_id(),
                    z: handle.z,
                    id: handle.id.clone(),
                });
            } else {
                *pointer.borrow_mut() = Some(PointerOwner::Orbit {
                    identity: projection.identity.clone(),
                    pointer_id: event.pointer_id(),
                    last_x: x,
                    last_y: y,
                    start_x: x,
                    start_y: y,
                    moved: false,
                    picked,
                });
            }
            if let Some(canvas) = canvas.borrow().as_ref()
                && let Err(error) = canvas.set_pointer_capture(event.pointer_id())
            {
                status.set(js_error(error));
            }
        }
    };

    let on_wheel = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |event: WheelEvent| {
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let Some(wheel) = event.data().try_as_web_event() else {
                return;
            };
            wheel.prevent_default();
            let delta = wheel.delta_y();
            let factor = (delta.signum() * (delta.abs() * 0.001).min(1.0)).exp();
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| host.zoom(factor));
            if let Err(error) = result {
                status.set(error.clone());
                if (current_source.0)() {
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        }
    };

    let on_pointer_move = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let applied_identity = applied_identity.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let host_guard = host.borrow();
            let Some(host) = host_guard.as_ref() else {
                return;
            };
            let (x, y) = pointer_point(&event);
            let Some(active) =
                take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity)
            else {
                return;
            };
            let identity = active.identity().clone();
            match active {
                PointerOwner::Orbit {
                    identity: pointer_identity,
                    pointer_id,
                    last_x,
                    last_y,
                    start_x,
                    start_y,
                    mut moved,
                    picked,
                } if pointer_id == event.pointer_id() => {
                    let delta_x = x - last_x;
                    let delta_y = y - last_y;
                    moved |= (x - start_x).abs() + (y - start_y).abs() > 4.0;
                    if let Err(error) = host.orbit(delta_x, delta_y) {
                        status.set(error.clone());
                        if (current_source.0)() {
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                        }
                    }
                    *pointer.borrow_mut() = Some(PointerOwner::Orbit {
                        identity: pointer_identity,
                        pointer_id,
                        last_x: x,
                        last_y: y,
                        start_x,
                        start_y,
                        moved,
                        picked,
                    });
                }
                PointerOwner::Handle {
                    identity: pointer_identity,
                    projection: handle_projection,
                    pointer_id,
                    z,
                    id,
                } if pointer_id == event.pointer_id() => {
                    if !(current_source.0)()
                        || applied_identity.borrow().as_ref() != Some(&identity)
                    {
                        release_pointer_capture(&canvas, pointer_id);
                        return;
                    }
                    let point = match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            status.set(error.clone());
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                            *pointer.borrow_mut() = Some(PointerOwner::Handle {
                                projection: handle_projection,
                                identity: pointer_identity,
                                pointer_id,
                                z,
                                id,
                            });
                            return;
                        }
                    };
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::WorldPoint(point),
                    );
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::HandleGesture {
                            phase: HandleGesturePhase::Move,
                            handle_id: id.clone(),
                            point,
                        },
                    );
                    *pointer.borrow_mut() = Some(PointerOwner::Handle {
                        projection: handle_projection,
                        identity: pointer_identity,
                        pointer_id,
                        z,
                        id,
                    });
                }
                _ => {}
            }
        }
    };

    let on_pointer_up = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let applied_identity = applied_identity.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let active = take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity);
            let Some(active) = active else {
                return;
            };
            release_pointer_capture(&canvas, event.pointer_id());
            let identity = active.identity().clone();
            if identity != projection.identity {
                return;
            }
            match active {
                PointerOwner::Orbit {
                    pointer_id,
                    moved,
                    picked,
                    ..
                } if pointer_id == event.pointer_id() => {
                    if !moved
                        && (current_source.0)()
                        && applied_identity.borrow().as_ref() == Some(&identity)
                        && runtime.scope().as_ref() == Some(&projection.identity.scope)
                        && let Some(id) = picked
                    {
                        if let Some(host) = host.borrow().as_ref() {
                            let _ = host.focus_objects(&[]);
                        }
                        emit_signal(&owner, on_signal, &identity, ViewerSignalKind::Picked(id));
                    }
                }
                PointerOwner::Handle {
                    pointer_id, z, id, ..
                } if pointer_id == event.pointer_id() => {
                    if !(current_source.0)()
                        || applied_identity.borrow().as_ref() != Some(&identity)
                    {
                        return;
                    }
                    if let Some(host) = host.borrow().as_ref() {
                        let (x, y) = pointer_point(&event);
                        let point = match host.point_on_plane_at_client(x, y, z) {
                            Ok(point) => point,
                            Err(error) => {
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &identity,
                                    ViewerSignalKind::Failed(error),
                                );
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
                                return;
                            }
                        };
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::WorldPoint(point),
                        );
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::HandleGesture {
                                phase: HandleGesturePhase::End,
                                handle_id: id,
                                point,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    };

    let on_pointer_cancel = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let applied_identity = applied_identity.clone();
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let active = take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity);
            let Some(active) = active else {
                return;
            };
            release_pointer_capture(&canvas, event.pointer_id());
            let identity = active.identity().clone();
            if identity != projection.identity
                || !(current_source.0)()
                || applied_identity.borrow().as_ref() != Some(&identity)
            {
                return;
            }
            if let PointerOwner::Handle {
                pointer_id, z, id, ..
            } = active
                && pointer_id == event.pointer_id()
            {
                let point = if let Some(host) = host.borrow().as_ref() {
                    let (x, y) = pointer_point(&event);
                    match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                emit_signal(
                    &owner,
                    on_signal,
                    &identity,
                    ViewerSignalKind::WorldPoint(point),
                );
                emit_signal(
                    &owner,
                    on_signal,
                    &identity,
                    ViewerSignalKind::HandleGesture {
                        phase: HandleGesturePhase::Cancel,
                        handle_id: id,
                        point,
                    },
                );
            }
        }
    };

    let on_key_down = {
        let projection = projection.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: KeyboardEvent| {
            let Some(keyboard) = event.data().try_as_web_event() else {
                return;
            };
            if keyboard.key() != "Escape"
                || !owner_is_current(&owner, &projection.identity)
                || !(current_source.0)()
            {
                return;
            }
            let active = pointer.borrow_mut().take();
            match active {
                Some(PointerOwner::Handle {
                    identity,
                    pointer_id,
                    id,
                    ..
                }) if identity == projection.identity => {
                    release_pointer_capture(&canvas, pointer_id);
                    keyboard.prevent_default();
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
                Some(active) => *pointer.borrow_mut() = Some(active),
                None => {}
            }
        }
    };

    let select_layer = {
        let projection = projection.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        move |id: String| {
            if (current_source.0)()
                && runtime.scope().as_ref() == Some(&projection.identity.scope)
                && owner_is_current(&owner, &projection.identity)
            {
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::LayerSelected(id),
                );
            }
        }
    };

    let change_display = {
        let projection = projection.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        move |next: CaseDisplay| {
            let identity = projection.identity.clone();
            if !(current_source.0)()
                || !owner_is_current(&owner, &identity)
                || runtime.scope().as_ref() != Some(&identity.scope)
            {
                return;
            }
            on_display_change.call(ScopedDisplayChange {
                identity,
                display: next,
                owner: Rc::downgrade(&owner),
            });
        }
    };

    let on_pointer_cancel = EventHandler::new(on_pointer_cancel);
    let on_key_down = EventHandler::new(on_key_down);
    let fit = host.clone();
    let top = host.clone();
    let top_for_gaskets = host.clone();
    let top_for_mounts = host.clone();
    let bottom = host.clone();
    let iso = host.clone();
    let case_fit = host.clone();
    let case_top = host.clone();
    let case_bottom = host.clone();
    let case_iso = host.clone();
    let left = host.clone();
    let right = host.clone();
    let zoom_in = host.clone();
    let zoom_out = host.clone();
    let assembly_change_display = change_display.clone();
    let selected_support = mechanical_settings.as_ref().and_then(|settings| {
        settings.gasket_supports.iter().find(|support| {
            !support.is_previous
                && (selected_layer == format!("gasket:{}:lower", support.id)
                    || selected_layer == format!("gasket:{}:upper", support.id))
        })
    });
    let can_edit_gaskets = mechanical_settings.as_ref().is_some_and(|settings| {
        handles
            .iter()
            .any(|handle| matches!(&handle.target, ViewerHandleTarget::Gasket { .. }))
            && settings.identity.scope == projection.identity.scope
            && settings.identity.snapshot_token == projection.identity.snapshot_token
            && settings.editable
    });
    let can_edit_mounts = mechanical_settings.as_ref().is_some_and(|settings| {
        handles.iter().any(|handle| {
            matches!(
                &handle.target,
                ViewerHandleTarget::Mount { .. } | ViewerHandleTarget::AuthoredMount { .. }
            )
        }) && settings.identity.scope == projection.identity.scope
            && settings.identity.snapshot_token == projection.identity.snapshot_token
            && settings.editable
    }) || handles
        .iter()
        .any(|handle| matches!(&handle.target, ViewerHandleTarget::AuthoredMount { .. }));
    let unlink_request = selected_support.and_then(|support| {
        mechanical_settings.as_ref().map(|settings| {
            let anchors = settings
                .gasket_supports
                .iter()
                .filter(|candidate| {
                    candidate.id == support.id
                        || candidate.pair_id.as_deref() == Some(support.id.as_str())
                })
                .map(mechanical_settings::MechanicalGasketSupportRow::saved_anchor)
                .collect::<Vec<_>>();
            (
                support.id.clone(),
                support.pair_id.clone(),
                anchors,
                settings.identity.clone(),
                settings.request_sequence,
                settings.on_request,
                settings.editable,
            )
        })
    });
    let unlink_feedback = selected_support.and_then(|support| {
        mechanical_settings.as_ref().and_then(|settings| {
            let field_id = format!("gasket-support:{}:link", support.id);
            settings
                .feedback
                .iter()
                .rev()
                .find(|feedback| feedback.field_id == field_id)
                .cloned()
        })
    });
    let unlink_message = unlink_feedback
        .as_ref()
        .and_then(|feedback| match feedback.state {
            mechanical_settings::MechanicalSettingsFeedbackState::Pending => {
                Some((false, "Unlinking selected support…".to_owned()))
            }
            mechanical_settings::MechanicalSettingsFeedbackState::Landed => None,
            mechanical_settings::MechanicalSettingsFeedbackState::Failed => Some((
                true,
                feedback
                    .message
                    .clone()
                    .unwrap_or_else(|| "The support could not be unlinked.".to_owned()),
            )),
        });
    let unlink_source = current_source.clone();
    let unlink_owner = owner.clone();
    let unlink_projection = projection.clone();
    let unlink_runtime = runtime.clone();
    let show_assembly_controls = projection.layers.iter().any(|(id, _)| id != "pcb");
    let gaskets_edit_disabled = mechanical_settings
        .as_ref()
        .is_none_or(|settings| !settings.editable);
    let select_assembly_layer = select_layer.clone();

    let section_plane = use_memo(move || Some(transient().section_plane.to_owned()));
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
                        Tooltip {
                            TooltipTrigger { as: move |attributes| { let fit = case_fit.clone(); rsx! {
                                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                                    class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                    aria_label: "Fit",
                                    onclick: move |_| run_host(&fit, |host| host.fit(), &mut status),
                                    {viewer_control_icon("Fit")}
                                }
                            } } }
                            TooltipContent { "Fit model" }
                        }
                        Tooltip {
                            TooltipTrigger { as: move |attributes| { let top = case_top.clone(); rsx! {
                                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                                    class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                    aria_label: "Top",
                                    onclick: move |_| run_host(&top, |host| host.view("top"), &mut status),
                                    {viewer_control_icon("Top")}
                                }
                            } } }
                            TooltipContent { "Top view" }
                        }
                        Tooltip {
                            TooltipTrigger { as: move |attributes| { let bottom = case_bottom.clone(); rsx! {
                                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                                    class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                    aria_label: "Bottom",
                                    onclick: move |_| run_host(&bottom, |host| host.view("bottom"), &mut status),
                                    {viewer_control_icon("Bottom")}
                                }
                            } } }
                            TooltipContent { "Bottom view" }
                        }
                        Tooltip {
                            TooltipTrigger { as: move |attributes| { let iso = case_iso.clone(); rsx! {
                                UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                                    class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                    aria_label: "Isometric",
                                    onclick: move |_| run_host(&iso, |host| host.view("isometric"), &mut status),
                                    {viewer_control_icon("Isometric")}
                                }
                            } } }
                            TooltipContent { "Isometric view" }
                        }
                    }
                }

                if inline_case_controls {
                    ToggleGroup {
                        aria_label: "Display mode",
                        pressed: Some([match transient().mode { RenderMode::Shaded => 0, RenderMode::Wireframe => 1, RenderMode::Hybrid => 2 }].into_iter().collect()),
                        on_pressed_change: move |next: std::collections::HashSet<usize>| {
                            if let Some(index) = next.iter().next() {
                                transient.with_mut(|view| view.mode = [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid][*index]);
                            }
                        },
                        for (index, mode) in [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid].into_iter().enumerate() {
                            {
                                let label = match mode {
                                    RenderMode::Shaded => "Shaded",
                                    RenderMode::Wireframe => "Wireframe",
                                    RenderMode::Hybrid => "Hybrid",
                                };
                                rsx! {
                                    Tooltip {
                                        TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                                            ToggleItem { index, aria_label: label,
                                                class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                                attributes,
                                                {viewer_control_icon(label)}
                                            }
                                        } }
                                        TooltipContent { "{label}" }
                                    }
                                }
                            }
                        }
                    }
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
                if show_assembly_controls {
                    ToggleGroup {
                        aria_label: "Mechanical assembly view",
                        pressed: Some([match transient().assembly { AssemblyView::Assembled => 0, AssemblyView::Exploded => 1, AssemblyView::Section => 2 }].into_iter().collect()),
                        on_pressed_change: move |next: std::collections::HashSet<usize>| {
                            if let Some(index) = next.iter().next() {
                                transient.with_mut(|view| view.assembly = [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section][*index]);
                            }
                        },
                        for (index, view) in [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section].into_iter().enumerate() {
                            {
                                let label = match view {
                                    AssemblyView::Assembled => "Assembled",
                                    AssemblyView::Exploded => "Exploded",
                                    AssemblyView::Section => "Section",
                                };
                                rsx! {
                                    Tooltip {
                                        TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                                            ToggleItem { index, aria_label: label,
                                                class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                                attributes,
                                                {viewer_control_icon(label)}
                                            }
                                        } }
                                        TooltipContent { "{label}" }
                                    }
                                }
                            }
                        }
                    }
                    }
                }
                if can_edit_gaskets || can_edit_mounts {
                    ToggleGroup {
                        aria_label: "Edit mode",
                        pressed: Some({
                            let mut pressed: std::collections::HashSet<usize> = Default::default();
                            if editing_gaskets() { pressed.insert(0); }
                            if editing_mounts() { pressed.insert(1); }
                            pressed
                        }),
                        on_pressed_change: move |next: std::collections::HashSet<usize>| {
                            let enable_gaskets = next.contains(&0);
                            let enable_mounts = !enable_gaskets && next.contains(&1);
                            if enable_gaskets != editing_gaskets() || enable_mounts != editing_mounts() {
                                editing_gaskets.set(enable_gaskets);
                                editing_mounts.set(enable_mounts);
                                if enable_gaskets {
                                    run_host(&top_for_gaskets, |host| host.view("top"), &mut status);
                                }
                                if enable_mounts {
                                    run_host(&top_for_mounts, |host| host.view("top"), &mut status);
                                }
                            }
                        },
                        if can_edit_gaskets {
                            Tooltip {
                                TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                                    ToggleItem { index: 0usize, aria_label: "Edit gaskets",
                                        disabled: gaskets_edit_disabled,
                                        class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                        attributes,
                                        {viewer_control_icon("Edit gaskets")}
                                    }
                                } }
                                TooltipContent { "Edit gaskets" }
                            }
                        }
                        if can_edit_mounts {
                            Tooltip {
                                TooltipTrigger { as: move |attributes: Vec<Attribute>| rsx! {
                                    ToggleItem { index: 1usize, aria_label: "Edit mounts",
                                        class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                                        attributes,
                                        {viewer_control_icon("Edit mounts")}
                                    }
                                } }
                                TooltipContent { "Edit mounts" }
                            }
                        }
                    }
                }
                if editing_gaskets() {
                    if let Some((support_id, pair_id, anchors, identity, mut sequence, on_request, editable)) = unlink_request.clone() {
                        UiButton { variant: UiButtonVariant::Secondary, button_type: "button",
                            class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                            disabled: !editable || selected_support.is_some_and(|support| support.unlinked),
                            onclick: move |_| {
                                if !(unlink_source.0)()
                                    || !owner_is_current(&unlink_owner, &unlink_projection.identity)
                                    || unlink_runtime.scope().as_ref() != Some(&identity.scope)
                                {
                                    return;
                                }
                                let Some(request_id) = sequence().checked_add(1) else { return };
                                sequence.set(request_id);
                                let patch = mechanical_settings::MechanicalSettingsPatch::SetGasketSupportUnlinked {
                                    support_id: support_id.clone(), pair_id: pair_id.clone(), anchors: anchors.clone(),
                                };
                                on_request.call(mechanical_settings::MechanicalSettingsRequest {
                                    identity: identity.clone(),
                                    request_id,
                                    field_id: patch.field_id(),
                                    patch,
                                    reviewed_fingerprint: None,
                                });
                            },
                            "Unlink selected support"
                        }
                        if let Some((alert, message)) = unlink_message {
                            p { role: if alert { "alert" } else { "status" }, "{message}" }
                        }
                        if selected_support.is_some_and(|support| support.unlinked) {
                            p { "This gasket is unlinked from its pair." }
                        }
                    } else {
                        p { "Select a gasket in Objects to unlink its mirrored support pair." }
                    }
                }
                if !inline_case_controls {
                Tooltip {
                    TooltipTrigger { as: move |attributes| { let fit = fit.clone(); rsx! {
                        UiButton { attributes, variant: UiButtonVariant::Secondary, size: UiButtonSize::IconSm,
                            class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]",
                            aria_label: canvas_context.fit_label(),
                            onclick: move |_| run_host(&fit, |host| host.fit(), &mut status),
                            {viewer_control_icon("Fit")}
                        }
                    } } }
                    TooltipContent { "{canvas_context.fit_label()}" }
                }
                Collapsible { keep_mounted: true,
                    Tooltip {
                        TooltipTrigger { as: move |attributes| rsx! {
                            CollapsibleTrigger { aria_label: "View controls", class: "size-control-compact justify-center max-[960px]:min-h-11 max-[960px]:min-w-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.5]", attributes, {viewer_control_icon("View controls")} }
                        } }
                        TooltipContent { "View controls" }
                    }
                    CollapsibleContent { class: "absolute left-0 top-full z-50 mt-2 max-h-[min(60vh,max(0px,calc(100cqh-5rem)))] w-[min(320px,calc(100vw-32px))] overflow-auto rounded-md bg-card p-3 shadow-md data-[open=false]:hidden max-[960px]:[&_[data-slot=button]]:min-h-11 max-[960px]:[&_[data-slot=toggle-group-item]]:min-h-11 max-[960px]:[&_[data-slot=select-trigger]]:min-h-11",
            div { role: "group", "aria-label": "{canvas_context.camera_label()}",
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&top, |host| host.view("top"), &mut status), "Top view" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&bottom, |host| host.view("bottom"), &mut status), "Bottom view" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&iso, |host| host.view("isometric"), &mut status), "Isometric view" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&left, |host| host.orbit(-50.0, 0.0), &mut status), "Rotate left" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&right, |host| host.orbit(50.0, 0.0), &mut status), "Rotate right" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&zoom_in, |host| host.zoom(0.85), &mut status), "Zoom in" }
                UiButton { variant: UiButtonVariant::Ghost, onclick: move |_| run_host(&zoom_out, |host| host.zoom(1.15), &mut status), "Zoom out" }
            }
            ToggleGroup {
                aria_label: canvas_context.display_label(), horizontal: true,
                pressed: Some([match transient().mode { RenderMode::Shaded => 0, RenderMode::Wireframe => 1, RenderMode::Hybrid => 2 }].into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if let Some(index) = next.iter().next() {
                        transient.with_mut(|view| view.mode = [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid][*index]);
                    }
                },
                for (index, mode) in [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid].into_iter().enumerate() {
                    ToggleItem { index, "{mode.renderer_value()}" }
                }
            }
            ToggleGroup {
                aria_label: canvas_context.assembly_label(), horizontal: true,
                pressed: Some([match transient().assembly { AssemblyView::Assembled => 0, AssemblyView::Exploded => 1, AssemblyView::Section => 2 }].into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if let Some(index) = next.iter().next() {
                        transient.with_mut(|view| view.assembly = [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section][*index]);
                    }
                },
                for (index, view) in [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section].into_iter().enumerate() {
                    ToggleItem { index, "{view.renderer_value()}" }
                }
            }
            if transient().assembly == AssemblyView::Exploded {
                Slider {
                    label: "Explode amount", min: 0.0, max: 10.0, step: 0.1, class: "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1",
                    value: Some(f64::from(transient().explode_amount)),
                    on_value_change: move |value: f64| transient.with_mut(|state| state.explode_amount = (value as f32).clamp(0.0, 10.0)),
                }
            }
            if transient().assembly == AssemblyView::Section {
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
                    label: "Section position", min: -100.0, max: 100.0, step: 1.0, class: "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1",
                    value: Some(f64::from(transient().section_position)),
                    on_value_change: move |value: f64| transient.with_mut(|state| state.section_position = (value as f32).clamp(-100.0, 100.0)),
                }
                Label { html_for: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), class: "max-[960px]:min-h-11",
                    UiCheckbox { id: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), aria_label: "Show section plane",
                        checked: Some(if transient().show_section_plane { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: move |value| transient.with_mut(|state| state.show_section_plane = bool::from(value)),
                    }
                    "Show section plane"
                }
            }
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
            if inline_case_controls && show_assembly_controls && transient().assembly == AssemblyView::Exploded {
                Slider {
                    label: "Explode amount", min: 0.0, max: 10.0, step: 0.1, class: "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1",
                    value: Some(f64::from(transient().explode_amount)),
                    on_value_change: move |value: f64| transient.with_mut(|state| state.explode_amount = (value as f32).clamp(0.0, 10.0)),
                }
            }
            if inline_case_controls && show_assembly_controls && transient().assembly == AssemblyView::Section {
                div { class: "flex flex-wrap items-center gap-3 border-b border-border px-2.5 py-1.5",
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
                        label: "Section position", min: -100.0, max: 100.0, step: 1.0, class: "min-h-control max-[960px]:min-h-11 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-thumb]]:absolute [&_[data-slot=slider-thumb]]:top-1/2 [&_[data-slot=slider-thumb]]:-translate-x-1/2 [&_[data-slot=slider-thumb]]:-translate-y-1/2 [&_[data-slot=slider-thumb]]:scroll-m-1",
                        value: Some(f64::from(transient().section_position)),
                        on_value_change: move |value: f64| transient.with_mut(|state| state.section_position = (value as f32).clamp(-100.0, 100.0)),
                    }
                    Label { html_for: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), class: "max-[960px]:min-h-11",
                        UiCheckbox { id: format!("viewer-show-plane-{}", dioxus::core::current_scope_id().0), aria_label: "Show plane",
                            checked: Some(if transient().show_section_plane { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                            on_checked_change: move |value| transient.with_mut(|state| state.show_section_plane = bool::from(value)),
                        }
                        "Show plane"
                    }
                }
            }
            div { class: "relative flex min-h-[160px] min-w-0 flex-1 [&>canvas]:h-full [&>canvas]:min-h-0 [&>canvas]:min-w-0 [&>canvas]:w-full [&>canvas]:flex-1 [&>canvas]:border-0",
                if let Some(reference) = selected_reference.as_deref() {
                    div { class: "pointer-events-none absolute bottom-14 left-3 z-[2] max-w-[min(70%,320px)] overflow-hidden truncate rounded-sm border border-border bg-card px-[9px] py-[7px] font-mono text-sm leading-[1.45] text-foreground", "aria-hidden": "true", "{reference}" }
                }
                canvas {
                    style: "width:100%;height:100%;display:block",
                    tabindex: "0", role: "img",
                    "aria-label": "{canvas_context.accessible_name()}",
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
