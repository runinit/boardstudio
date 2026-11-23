//! Shared canvas navigation state and the behavior that owns camera gestures.

use boardstudio_application::Scope;
use boardstudio_model::model::Vec2;
use boardstudio_web_ui_model::{state::Drag, svg_coordinates::PointerLocation};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

#[derive(Clone)]
pub struct CanvasNavigationState {
    surface_size: Signal<(f64, f64)>,
    drag: Rc<RefCell<Option<Drag>>>,
    space_down: Rc<Cell<bool>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomTarget {
    pub center: Vec2,
    pub zoom: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanStart {
    pub pointer_id: i64,
    pub scope: Scope,
    pub generation: u64,
    pub client: (f64, f64),
    pub camera: Vec2,
}

pub fn use_canvas_navigation_state(workspace: Signal<&'static str>) -> CanvasNavigationState {
    let state = CanvasNavigationState {
        surface_size: use_signal(|| (1.0, 1.0)),
        drag: use_hook(|| Rc::new(RefCell::new(None))),
        space_down: use_hook(|| Rc::new(Cell::new(false))),
    };
    #[cfg(target_arch = "wasm32")]
    {
        let space_down = state.space_down.clone();
        let listener = use_hook({
            move || {
                let is_layout: Rc<dyn Fn() -> bool> = Rc::new(move || workspace() == "Layout");
                boardstudio_web_ui_model::canvas_interaction::LayoutSpacePanWindowListener::install(
                    is_layout, space_down,
                )
                .map(Rc::new)
            }
        });
        use_drop(move || drop(listener));
    }
    state
}

impl CanvasNavigationState {
    pub fn surface_size(&self) -> Signal<(f64, f64)> {
        self.surface_size
    }

    /// Shared with page-level selection/placement arbitration; navigation owns
    /// pan lifecycle while workspace handlers own non-navigation drags.
    pub fn drag_interaction(&self) -> Rc<RefCell<Option<Drag>>> {
        self.drag.clone()
    }

    /// Shared with page-level keyboard and pointer arbitration.
    pub fn space_key_state(&self) -> Rc<Cell<bool>> {
        self.space_down.clone()
    }

    /// Build the common Space+primary-button pan handler. Page-specific pointer
    /// arbitration runs first; the navigation handle owns the shared pan gesture.
    pub fn space_pan_handler<P>(
        &self,
        scope: Scope,
        generation: u64,
        runtime: Rc<boardstudio_web_runtime::runtime::Runtime>,
        svg: Rc<RefCell<Option<SvgElement>>>,
        owner_is_current: Rc<dyn Fn() -> bool>,
        page_arbitration: P,
    ) -> impl FnMut(PointerEvent) + Clone + use<P>
    where
        P: FnMut(PointerEvent) + Clone + 'static,
    {
        let navigation = self.clone();
        let mut page_arbitration = page_arbitration;
        move |event| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            page_arbitration(event.clone());
            if pointer.default_prevented()
                || pointer.button() != 0
                || !navigation.space_down.get()
                || runtime.scope().as_ref() != Some(&scope)
                || !owner_is_current()
                || navigation.drag.borrow().is_some()
                || runtime.model().gesture.is_some()
            {
                return;
            }
            event.prevent_default();
            event.stop_propagation();
            if let Some(surface) = svg.borrow().as_ref() {
                let _ = surface.set_pointer_capture(pointer.pointer_id());
                let options = web_sys::FocusOptions::new();
                options.set_prevent_scroll(true);
                let _ = surface.focus_with_options(&options);
            }
            navigation.start_space_pan(PanStart {
                pointer_id: i64::from(pointer.pointer_id()),
                scope: scope.clone(),
                generation,
                client: (f64::from(pointer.client_x()), f64::from(pointer.client_y())),
                camera: runtime.model().camera.center,
            });
        }
    }

    pub fn mount_handler(
        surface_size: Signal<(f64, f64)>,
        runtime: Rc<boardstudio_web_runtime::runtime::Runtime>,
        svg: Rc<RefCell<Option<SvgElement>>>,
        focus_placement: bool,
    ) -> impl FnMut(MountedEvent) {
        let mut surface_size = surface_size;
        move |event: MountedEvent| {
            if let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|event| event.dyn_into::<SvgElement>().ok())
            {
                runtime.surface(element.clone());
                if focus_placement {
                    let options = web_sys::FocusOptions::new();
                    options.set_prevent_scroll(true);
                    let _ = element.focus_with_options(&options);
                }
                let rect = element.get_bounding_client_rect();
                surface_size.set((rect.width(), rect.height()));
                *svg.borrow_mut() = Some(element);
            }
        }
    }

    pub fn start_space_pan(&self, start: PanStart) -> bool {
        let mut drag = self.drag.borrow_mut();
        if drag.is_some() {
            return false;
        }
        *drag = Some(Drag {
            pointer: start.pointer_id,
            scope: start.scope,
            generation: start.generation,
            gesture_generation: None,
            origin: Vec2::default(),
            client_x: start.client.0,
            client_y: start.client.1,
            positions: Vec::new(),
            active: true,
            pan: true,
            camera: start.camera,
        });
        true
    }

    /// Consume moves for the camera-only pan gesture. Pan applies its camera
    /// delta on pointer-up, so this intentionally does not submit Session samples.
    pub fn consume_pan_pointer_move(
        &self,
        pointer_id: i64,
        scope: &Scope,
        generation: u64,
    ) -> bool {
        self.drag.borrow().as_ref().is_some_and(|drag| {
            drag.pan
                && drag.pointer == pointer_id
                && &drag.scope == scope
                && drag.generation == generation
        })
    }

    /// Finish a pan gesture by applying its accumulated camera delta and
    /// releasing the captured pointer. Returns true when this handle owned it.
    pub fn finish_pan(
        &self,
        runtime: &Rc<boardstudio_web_runtime::runtime::Runtime>,
        svg: &Rc<RefCell<Option<SvgElement>>>,
        pointer_id: i32,
        client: (f64, f64),
        view_box: (f64, f64),
    ) -> bool {
        let Some(current) = self
            .drag
            .borrow()
            .clone()
            .filter(|drag| drag.pan && drag.pointer == i64::from(pointer_id))
        else {
            return false;
        };
        if let Some(element) = svg.borrow().as_ref() {
            let rect = element.get_bounding_client_rect();
            if rect.width() > 0.0 && rect.height() > 0.0 {
                let scale = (rect.width() / view_box.0).min(rect.height() / view_box.1);
                if let Some(center) = Self::pan_center(
                    current.camera,
                    (client.0 - current.client_x, client.1 - current.client_y),
                    scale,
                ) {
                    let camera = runtime.model().camera;
                    runtime.submit(boardstudio_application::Event::SetCamera {
                        operation_id: runtime.operation(),
                        center,
                        zoom: camera.zoom,
                    });
                }
            }
            let _ = element.release_pointer_capture(pointer_id);
        }
        let mut drag = self.drag.borrow_mut();
        if drag.as_ref().is_some_and(|stored| {
            stored.pointer == current.pointer
                && stored.scope == current.scope
                && stored.generation == current.generation
        }) {
            drag.take();
        }
        true
    }

    pub fn release_space_key(&self, key: &str, code: &str) {
        if key == " " || code == "Space" {
            self.space_down.set(false);
        }
    }

    pub fn pan_center(start: Vec2, client_delta: (f64, f64), scale: f64) -> Option<Vec2> {
        (scale.is_finite() && scale > 0.0).then_some(Vec2 {
            x: start.x - client_delta.0 / scale,
            y: start.y + client_delta.1 / scale,
        })
    }

    pub fn apply_zoom_target(
        runtime: &Rc<boardstudio_web_runtime::runtime::Runtime>,
        target: ZoomTarget,
    ) {
        runtime.submit(boardstudio_application::Event::SetCamera {
            operation_id: runtime.operation(),
            center: target.center,
            zoom: target.zoom,
        });
    }

    pub fn zoom_center_at(
        (min_x, max_x, min_y, max_y): (f64, f64, f64, f64),
        location: PointerLocation,
        zoom: f64,
    ) -> Vec2 {
        // Single copy of the anchor math lives in layout_camera so burst
        // coalescing stays unit-testable on every target.
        super::layout_camera::zoom_center_for_anchor(
            (min_x, max_x, min_y, max_y),
            super::layout_camera::WheelAnchor {
                world: location.world,
                x_fraction: location.x_fraction,
                y_fraction: location.y_fraction,
            },
            zoom,
        )
    }
}
