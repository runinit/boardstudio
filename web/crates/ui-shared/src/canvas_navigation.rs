//! Shared canvas navigation state and the behavior that owns camera gestures.

use boardstudio_application::Scope;
use boardstudio_model::model::Vec2;
use boardstudio_web_ui_model::{state::Drag, svg_coordinates::PointerLocation};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

#[derive(Clone)]
pub struct CanvasNavigationState {
    surface_size: Signal<(f64, f64)>,
    drag: Rc<RefCell<Option<Drag>>>,
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

pub fn use_canvas_navigation_state() -> CanvasNavigationState {
    CanvasNavigationState {
        surface_size: use_signal(|| (1.0, 1.0)),
        drag: use_hook(|| Rc::new(RefCell::new(None))),
    }
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
            last_sample: None,
            active: true,
            pan: true,
            camera: start.camera,
        });
        true
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
        let current = {
            let mut drag = self.drag.borrow_mut();
            if !drag
                .as_ref()
                .is_some_and(|drag| drag.pan && drag.pointer == i64::from(pointer_id))
            {
                return false;
            }
            drag.take().unwrap()
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
        true
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
