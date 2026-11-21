//! The canvas-owned transform key listener: one document `keydown`/`keyup` and
//! window `blur` set per canvas, shared by every rail overlay that canvas draws.
//!
//! Overlays subscribe through [`use_canvas_escape`] and [`use_canvas_control`]
//! for their lifetime. The listener installs with the first route and is
//! removed with the last one, or when the canvas drops its svg handle, so a
//! second canvas (or a remount with a fresh handle) owns its own set.
use super::*;
use boardstudio_web_layout_core::canvas_keys::{CanvasKeyRoutes, KeyRouteId};
use std::cell::Cell;

/// An overlay's Escape route. It returns true when it claims the Escape; the
/// canvas then prevents the default, so later routes and document listeners
/// leave it alone.
type EscapeRoute = Rc<RefCell<dyn FnMut(&web_sys::KeyboardEvent) -> bool>>;
type KeyRoutes = CanvasKeyRoutes<EscapeRoute, Signal<bool>>;

#[derive(Default)]
pub(super) struct CanvasKeys {
    routes: Rc<RefCell<KeyRoutes>>,
    listener: RefCell<Option<KeyListener>>,
}

impl CanvasKeys {
    fn subscribe(&self, add: impl FnOnce(&mut KeyRoutes) -> KeyRouteId) -> KeyRouteId {
        let id = add(&mut self.routes.borrow_mut());
        let mut listener = self.listener.borrow_mut();
        if listener.is_none() {
            *listener = KeyListener::install(&self.routes);
        }
        id
    }

    fn unsubscribe(&self, id: KeyRouteId) {
        let empty = {
            let mut routes = self.routes.borrow_mut();
            routes.remove(id);
            routes.is_empty()
        };
        if empty {
            // Dropping the listener removes it from the document and window.
            let retired = self.listener.borrow_mut().take();
            drop(retired);
        }
    }
}

struct KeyListener {
    document: web_sys::Document,
    window: web_sys::Window,
    down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    blur: Closure<dyn FnMut(web_sys::Event)>,
}

impl KeyListener {
    fn install(routes: &Rc<RefCell<KeyRoutes>>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let down = {
            let routes = Rc::downgrade(routes);
            Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                let Some(routes) = routes.upgrade() else {
                    return;
                };
                match event.key().as_str() {
                    // The modifier changes presentation immediately; pointer-down latches the edit intent.
                    "Control" => set_control(&routes, true),
                    "Escape" => route_escape(&routes, &event),
                    _ => {}
                }
            }) as Box<dyn FnMut(_)>)
        };
        let up = {
            let routes = Rc::downgrade(routes);
            Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == "Control"
                    && let Some(routes) = routes.upgrade()
                {
                    set_control(&routes, false);
                }
            }) as Box<dyn FnMut(_)>)
        };
        let blur = {
            let routes = Rc::downgrade(routes);
            Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Some(routes) = routes.upgrade() {
                    set_control(&routes, false);
                }
            }) as Box<dyn FnMut(_)>)
        };
        let listener = Self {
            document,
            window,
            down,
            up,
            blur,
        };
        listener
            .document
            .add_event_listener_with_callback("keydown", listener.down.as_ref().unchecked_ref())
            .ok()?;
        listener
            .document
            .add_event_listener_with_callback("keyup", listener.up.as_ref().unchecked_ref())
            .ok()?;
        listener
            .window
            .add_event_listener_with_callback("blur", listener.blur.as_ref().unchecked_ref())
            .ok()?;
        Some(listener)
    }
}

impl Drop for KeyListener {
    fn drop(&mut self) {
        let _ = self
            .document
            .remove_event_listener_with_callback("keydown", self.down.as_ref().unchecked_ref());
        let _ = self
            .document
            .remove_event_listener_with_callback("keyup", self.up.as_ref().unchecked_ref());
        let _ = self
            .window
            .remove_event_listener_with_callback("blur", self.blur.as_ref().unchecked_ref());
    }
}

fn set_control(routes: &RefCell<KeyRoutes>, pressed: bool) {
    let targets = routes.borrow().control_targets();
    for mut target in targets {
        target.set(pressed);
    }
}

/// Hands the Escape to each route in subscription order until one claims it
/// (or an earlier document listener already prevented it).
fn route_escape(routes: &RefCell<KeyRoutes>, event: &web_sys::KeyboardEvent) {
    let escape = routes.borrow().escape_routes();
    for route in escape {
        if event.default_prevented() {
            return;
        }
        let claimed = (*route.borrow_mut())(event);
        if claimed {
            event.prevent_default();
        }
    }
}

/// Subscribes the route `add` builds on the canvas listener after the
/// overlay's first render, and unsubscribes it when the overlay drops.
fn use_canvas_route(
    svg: &LayoutTransformSvg,
    add: Option<impl FnOnce(&mut KeyRoutes) -> KeyRouteId + 'static>,
) {
    let route = use_hook(|| Rc::new(Cell::new(None::<KeyRouteId>)));
    let mut add = add;
    use_effect({
        let svg = svg.clone();
        let route = route.clone();
        move || {
            if let Some(add) = add.take() {
                route.set(Some(svg.0.keys.subscribe(add)));
            }
        }
    });
    let svg = svg.clone();
    use_drop(move || {
        if let Some(id) = route.take() {
            svg.0.keys.unsubscribe(id);
        }
    });
}

/// Hands every document Escape no earlier route or listener prevented to
/// `handler` for the overlay's lifetime; returning true claims it. The first
/// render's handler is the one subscribed, so it holds clones of the
/// overlay's handles rather than per-render values.
pub(super) fn use_canvas_escape(
    svg: &LayoutTransformSvg,
    handler: impl FnMut(&web_sys::KeyboardEvent) -> bool + 'static,
) {
    use_canvas_route(
        svg,
        Some(move |routes: &mut KeyRoutes| {
            let route: EscapeRoute = Rc::new(RefCell::new(handler));
            routes.add_escape(route)
        }),
    );
}

/// Whether Control is held, for an overlay whose first render `enabled` it;
/// otherwise it stays false and subscribes nothing.
pub(super) fn use_canvas_control(svg: &LayoutTransformSvg, enabled: bool) -> Signal<bool> {
    let control = use_signal(|| false);
    let enabled = use_hook(|| enabled);
    use_canvas_route(
        svg,
        enabled.then_some(move |routes: &mut KeyRoutes| routes.add_control(control)),
    );
    control
}

/// Whether a rail overlay claims an Escape: never one aimed at a text field,
/// and while no drag of its own is live, only when no other visible handle
/// owns the gesture (that handle's route cancels it).
pub(super) fn rail_escape_claims(
    event: &web_sys::KeyboardEvent,
    drag_live: bool,
    arbiter: &CanvasInteractionArbiter,
) -> bool {
    !escape_targets_text_entry(event) && (drag_live || arbiter.current().is_none())
}

/// Whether a document Escape belongs to a text field rather than to the rail.
fn escape_targets_text_entry(event: &web_sys::KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|target| {
            target
                .closest("input, textarea, select, [contenteditable='true']")
                .ok()
                .flatten()
                .is_some()
        })
}
