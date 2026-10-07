//! Shared canvas navigation state and the behavior that owns camera gestures.

use boardstudio_application::Scope;
use boardstudio_core::model::Vec2;
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

    pub fn wheel_target(
        bounds: (f64, f64, f64, f64),
        location: PointerLocation,
        old_zoom: f64,
        scale_ratio: f64,
        delta_y: f64,
    ) -> ZoomTarget {
        let zoom = super::layout_camera::wheel_zoom(old_zoom, scale_ratio, delta_y);
        let center = Self::zoom_center_at(bounds, location, zoom);
        ZoomTarget { center, zoom }
    }

    pub fn apply_wheel_zoom(
        runtime: &Rc<boardstudio_web_runtime::runtime::Runtime>,
        bounds: (f64, f64, f64, f64),
        location: PointerLocation,
        scale_ratio: f64,
        delta_y: f64,
    ) -> ZoomTarget {
        let old = runtime.model().camera;
        let target = Self::wheel_target(bounds, location, old.zoom, scale_ratio, delta_y);
        Self::apply_zoom_target(runtime, target);
        target
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
        let base_width = (max_x - min_x).max(50.0);
        let base_height = (max_y - min_y).max(50.0);
        Vec2 {
            x: location.world.x
                - (min_x + max_x - base_width / zoom) * 0.5
                - location.x_fraction * base_width / zoom,
            y: location.world.y - (min_y + max_y + base_height / zoom) * 0.5
                + location.y_fraction * base_height / zoom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn fixture() -> Element {
        let workspace = use_signal(|| "Layout");
        let state = use_canvas_navigation_state(workspace);
        let surface_size = state.surface_size;
        let navigation = state.clone();
        let runtime =
            use_hook(|| boardstudio_web_runtime::runtime::project_name_test_support::new_runtime());
        let svg = use_hook(|| Rc::new(RefCell::new(None)));
        let mount = EventHandler::new(CanvasNavigationState::mount_handler(
            surface_size,
            runtime.clone(),
            svg.clone(),
            false,
        ));
        let mut outcome = use_signal(String::new);
        rsx! {
            svg {
                id: "canvas-navigation-surface",
                width: "640",
                height: "480",
                style: "width: 640px; height: 480px",
                onmounted: mount,
            }
            button {
                id: "canvas-navigation-size",
                onclick: move |_| {
                    navigation.space_down.set(true);
                    let scope = boardstudio_application::Scope {
                        session_epoch: boardstudio_application::SessionEpoch(1),
                        document_id: "navigation-test".into(),
                        board_id: "board".into(),
                        instance_id: None,
                    };
                    let camera_before = runtime.model().camera;
                    let started = navigation.start_space_pan(PanStart {
                        pointer_id: 7,
                        scope: scope.clone(),
                        generation: 3,
                        client: (100.0, 100.0),
                        camera: camera_before.center,
                    });
                    let pan_active = navigation.drag.borrow().as_ref().is_some_and(|drag| drag.pan);
                    let pan_move_consumed =
                        navigation.consume_pan_pointer_move(7, &scope, 3);
                    let rect = svg.borrow().as_ref().unwrap().get_bounding_client_rect();
                    let scale = (rect.width() / 640.0).min(rect.height() / 480.0);
                    let expected_center = CanvasNavigationState::pan_center(
                        camera_before.center,
                        (20.0, -10.0),
                        scale,
                    )
                    .unwrap();
                    let finished = navigation.finish_pan(
                        &runtime,
                        &svg,
                        7,
                        (120.0, 90.0),
                        (640.0, 480.0),
                    );
                    let camera = runtime.model().camera;
                    let camera_updated = (camera.center.x - expected_center.x).abs() < 0.001
                        && (camera.center.y - expected_center.y).abs() < 0.001;
                    let pan_center = CanvasNavigationState::pan_center(
                        boardstudio_core::model::Vec2 { x: 4.0, y: 6.0 },
                        (20.0, -10.0),
                        2.0,
                    ).unwrap();
                    navigation.release_space_key(" ", "Space");
                    let zoom = CanvasNavigationState::apply_wheel_zoom(
                        &runtime,
                        (-10.0, 10.0, -10.0, 10.0),
                        PointerLocation {
                            world: boardstudio_core::model::Vec2::default(),
                            x_fraction: 0.5,
                            y_fraction: 0.5,
                        },
                        1.0,
                        -400.0,
                    );
                    let applied_camera = runtime.model().camera;
                    let zoom_applied = (applied_camera.zoom - zoom.zoom).abs() < 0.001
                        && (applied_camera.center.x - zoom.center.x).abs() < 0.001
                        && (applied_camera.center.y - zoom.center.y).abs() < 0.001;
                    outcome.set(format!(
                        "{started}:{pan_active}:{pan_move_consumed}:{finished}:{camera_updated}:{:.0}:{:.0}:{}:{zoom_applied}:{:.3}:{:.0}:{:.0}",
                        pan_center.x,
                        pan_center.y,
                        navigation.space_down.get(),
                        zoom.zoom,
                        zoom.center.x,
                        zoom.center.y,
                    ));
                },
                "Resize canvas"
            }
            output { id: "canvas-navigation-result", "{surface_size().0}x{surface_size().1}" }
            output { id: "canvas-navigation-behavior", "{outcome}" }
        }
    }

    #[wasm_bindgen_test(async)]
    async fn handle_keeps_the_canvas_surface_measurement_owned_by_navigation_state() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(20).await;
        let button = root
            .query_selector("#canvas-navigation-size")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        button.click();
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(
            root.query_selector("#canvas-navigation-result")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("640x480")
        );
        assert_eq!(
            root.query_selector("#canvas-navigation-behavior")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("true:true:true:true:true:-6:1:false:true:1.492:0:0")
        );
        root.remove();
    }
}
