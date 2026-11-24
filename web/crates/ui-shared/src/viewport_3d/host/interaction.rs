use super::*;
use crate::viewport_3d::camera::{crossed_drag_threshold, wheel_zoom};

pub(super) struct ViewportInteraction {
    pub host: Rc<RefCell<Option<Rc<RendererPageHost>>>>,
    pub canvas: Rc<RefCell<Option<HtmlCanvasElement>>>,
    pub pointer: Rc<RefCell<Option<PointerOwner>>>,
    pub owner: Rc<ViewerOwner>,
    pub identity: ViewerIdentity,
    pub source: SourceGuard,
    pub applied: Rc<RefCell<Option<ViewerIdentity>>>,
    pub handles: Vec<ViewportHandle>,
    pub handle_projection: Option<CaseHandleProjection>,
    pub on_signal: EventHandler<ScopedViewerSignal>,
    pub status: Signal<String>,
}

impl ViewportInteraction {
    pub(super) fn bind_event<T: WebEventExt + 'static>(
        self: &Rc<Self>,
        handle: impl Fn(&Self, &T::WebEvent) + 'static,
    ) -> impl Fn(Event<T>) + 'static {
        let interaction = self.clone();
        move |event: Event<T>| {
            if let Some(event) = event.data().try_as_web_event() {
                handle(&interaction, &event);
            }
        }
    }

    pub(super) fn current(&self) -> bool {
        self.owner.is_current(&self.identity)
    }

    pub(super) fn can_select(&self) -> bool {
        (self.source.0)() && self.applied.borrow().as_ref() == Some(&self.identity)
    }

    pub(super) fn emit(&self, kind: ViewerSignalKind) {
        self.owner.emit(self.on_signal, &self.identity, kind, ());
    }

    pub(super) fn fail(&self, error: String) {
        let mut status = self.status;
        status.set(error.clone());
        if (self.source.0)() {
            self.emit(ViewerSignalKind::Failed(error));
        }
    }

    fn handle_event(&self, phase: HandleGesturePhase, handle_id: String, point: Option<[f32; 3]>) {
        self.emit(ViewerSignalKind::HandleGesture {
            phase,
            handle_id,
            point,
        });
    }

    pub(super) fn cancel_capture(&self) {
        release_capture(&self.pointer, &self.canvas);
    }

    pub fn down(&self, event: &PointerEvent) {
        if !self.current() || self.pointer.borrow().is_some() {
            return;
        }
        event.prevent_default();
        let (x, y) = pointer_point(event);
        let Some(host) = self.host.borrow().as_ref().cloned() else {
            return;
        };
        let picked = if self.can_select() {
            match host.pick_at_client(x, y) {
                Ok(id) => id.filter(|id| !id.is_empty()),
                Err(error) => {
                    self.fail(error);
                    None
                }
            }
        } else {
            None
        };
        let pointer_id = event.pointer_id();
        let gesture = if let Some(handle) = picked
            .as_ref()
            .and_then(|id| self.handles.iter().find(|handle| &handle.id == id))
        {
            match host.point_on_plane_at_client(x, y, handle.z) {
                Ok(point) => self.handle_event(HandleGesturePhase::Start, handle.id.clone(), point),
                Err(error) => {
                    self.fail(error);
                    return;
                }
            }
            PointerGesture::Handle {
                projection: self.handle_projection.clone(),
                z: handle.z,
                id: handle.id.clone(),
            }
        } else {
            PointerGesture::Orbit {
                last: (x, y),
                start: (x, y),
                moved: false,
                picked,
            }
        };
        *self.pointer.borrow_mut() = Some(PointerOwner {
            identity: self.identity.clone(),
            pointer_id,
            gesture,
        });
        if let Some(canvas) = self.canvas.borrow().as_ref()
            && let Err(error) = canvas.set_pointer_capture(pointer_id)
        {
            self.fail(js_error(error));
        }
    }

    pub fn drag(&self, event: &PointerEvent) {
        if !self.current() {
            return;
        }
        let Some(host) = self.host.borrow().as_ref().cloned() else {
            return;
        };
        let Some(mut active) =
            take_pointer_for_event(&self.pointer, event.pointer_id(), &self.identity)
        else {
            return;
        };
        let point = pointer_point(event);
        match &mut active.gesture {
            PointerGesture::Orbit {
                last, start, moved, ..
            } => {
                *moved |= crossed_drag_threshold(*start, point);
                if let Err(error) = host.orbit(point.0 - last.0, point.1 - last.1) {
                    self.fail(error);
                }
                *last = point;
            }
            PointerGesture::Handle { id, z, .. } => {
                if !self.can_select() {
                    release_pointer_capture(&self.canvas, event.pointer_id());
                    return;
                }
                match host.point_on_plane_at_client(point.0, point.1, *z) {
                    Ok(point) => self.handle_event(HandleGesturePhase::Move, id.clone(), point),
                    Err(error) => self.fail(error),
                }
            }
        }
        *self.pointer.borrow_mut() = Some(active);
    }

    pub fn finish(&self, event: &PointerEvent, phase: HandleGesturePhase) {
        if !self.current() {
            return;
        }
        let Some(active) =
            take_pointer_for_event(&self.pointer, event.pointer_id(), &self.identity)
        else {
            return;
        };
        release_pointer_capture(&self.canvas, active.pointer_id);
        if !self.can_select() {
            return;
        }
        let host = self.host.borrow().clone();
        match active.gesture {
            PointerGesture::Orbit {
                moved: false,
                picked: Some(id),
                ..
            } if phase == HandleGesturePhase::End => {
                if let Some(host) = host {
                    let _ = host.focus_objects(&[]);
                }
                self.emit(ViewerSignalKind::Picked(id));
            }
            PointerGesture::Handle { id, z, .. } => {
                let (x, y) = pointer_point(event);
                let result = host
                    .map(|host| host.point_on_plane_at_client(x, y, z))
                    .transpose();
                match result {
                    Ok(point) => self.handle_event(phase, id, point.flatten()),
                    Err(error) => {
                        self.fail(error);
                        self.handle_event(HandleGesturePhase::Cancel, id, None);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn key(&self, event: &web_sys::KeyboardEvent) {
        if event.key() != "Escape" || !self.current() || !(self.source.0)() {
            return;
        }
        let is_handle = self.pointer.borrow().as_ref().is_some_and(|active| {
            active.identity == self.identity
                && matches!(active.gesture, PointerGesture::Handle { .. })
        });
        if !is_handle {
            return;
        }
        event.prevent_default();
        if let Some((_, id)) = cancel_handle_capture(&self.pointer, &self.canvas) {
            self.handle_event(HandleGesturePhase::Cancel, id, None);
        }
    }

    pub fn wheel(&self, event: &web_sys::WheelEvent) {
        if !self.current() {
            return;
        }
        event.prevent_default();
        if let Some(host) = self.host.borrow().as_ref().cloned()
            && let Err(error) = host.zoom(wheel_zoom(event.delta_y()))
        {
            self.fail(error);
        }
    }
}

pub(super) struct PointerOwner {
    identity: ViewerIdentity,
    pointer_id: i32,
    gesture: PointerGesture,
}

enum PointerGesture {
    Orbit {
        last: (f64, f64),
        start: (f64, f64),
        moved: bool,
        picked: Option<String>,
    },
    Handle {
        projection: Option<CaseHandleProjection>,
        z: f32,
        id: String,
    },
}

fn take_pointer_for_event(
    pointer: &RefCell<Option<PointerOwner>>,
    pointer_id: i32,
    identity: &ViewerIdentity,
) -> Option<PointerOwner> {
    let mut active = pointer.borrow_mut();
    active.take_if(|active| active.pointer_id == pointer_id && &active.identity == identity)
}

pub(super) fn cancel_superseded_pointer(
    pointer: &RefCell<Option<PointerOwner>>,
    canvas: &RefCell<Option<HtmlCanvasElement>>,
    owner: &ViewerOwner,
    handle_projection: Option<&CaseHandleProjection>,
    source_current: bool,
) {
    let stale_pointer = {
        let mut active = pointer.borrow_mut();
        let current_identity = owner.identity.borrow();
        if owner.active.get()
            && source_current
            && let Some(current) = current_identity.as_ref()
            && let Some(PointerOwner {
                identity,
                gesture:
                    PointerGesture::Handle {
                        projection: Some(previous),
                        ..
                    },
                ..
            }) = active.as_mut()
            && let Some(next) = handle_projection
            && identity.scope == current.scope
            && identity.snapshot_token == current.snapshot_token
            && identity.revision == current.revision
            && previous.continues_preview(next)
        {
            // Transfer this gesture to the new render; event admission remains
            // full-identity checked, so callbacks from the old render stay stale.
            *identity = current.clone();
            *previous = next.clone();
        }
        active.take_if(|active| {
            !owner.active.get()
                || (matches!(active.gesture, PointerGesture::Handle { .. }) && !source_current)
                || current_identity.as_ref() != Some(&active.identity)
        })
    };
    if let Some(active) = stale_pointer {
        release_pointer_capture(canvas, active.pointer_id);
    }
}

pub(super) fn cancel_handle_capture(
    pointer: &RefCell<Option<PointerOwner>>,
    canvas: &RefCell<Option<HtmlCanvasElement>>,
) -> Option<(ViewerIdentity, String)> {
    let active = release_capture(pointer, canvas)?;
    match active.gesture {
        PointerGesture::Handle { id, .. } => Some((active.identity, id)),
        PointerGesture::Orbit { .. } => None,
    }
}

fn release_capture(
    pointer: &RefCell<Option<PointerOwner>>,
    canvas: &RefCell<Option<HtmlCanvasElement>>,
) -> Option<PointerOwner> {
    let captured = pointer.borrow_mut().take();
    if let Some(captured) = &captured {
        release_pointer_capture(canvas, captured.pointer_id);
    }
    captured
}

fn release_pointer_capture(canvas: &RefCell<Option<HtmlCanvasElement>>, pointer_id: i32) {
    if let Some(canvas) = canvas.borrow().as_ref() {
        let _ = canvas.release_pointer_capture(pointer_id);
    }
}

fn pointer_point(event: &PointerEvent) -> (f64, f64) {
    (f64::from(event.client_x()), f64::from(event.client_y()))
}
