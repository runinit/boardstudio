use boardstudio_web_ui_model::svg_coordinates::PointerLocation;
use boardstudio_web_ui_shared::canvas_navigation::{
    CanvasNavigationState, PanStart, use_canvas_navigation_state,
};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> Element {
    let workspace = use_signal(|| "Layout");
    let state = use_canvas_navigation_state(workspace);
    let surface_size = state.surface_size();
    let navigation = state.clone();
    let runtime = use_hook(boardstudio_web_test_runtime::project_name_test_support::new_runtime);
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
                navigation.space_key_state().set(true);
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
                let pan_active = navigation.drag_interaction().borrow().as_ref().is_some_and(|drag| drag.pan);
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
                    navigation.space_key_state().get(),
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

#[wasm_bindgen_test]
async fn handle_keeps_the_canvas_surface_measurement_owned_by_navigation_state() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
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
