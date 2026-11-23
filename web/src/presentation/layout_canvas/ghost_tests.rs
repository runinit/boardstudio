use super::*;
use crate::presentation::test_mount_helpers;
use crate::presentation::test_mount_helpers::canvas::{
    column_at_origin, one_key_matrix, one_key_scene, scene, switch_document,
};
use boardstudio_core::model::SceneDelta;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn ghost_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
    let mut document = switch_document("canvas", "Canvas", &["matrix/m/r0c0"]);
    document
        .matrices
        .push(one_key_matrix("m", "matrix/m/r0c0", 0.0));
    let scene_with = |x, y| scene(&[one_key_scene("m", x, y, &[column_at_origin()])]);
    (
        Arc::new(document),
        scene_with(0.0, 0.0),
        scene_with(9.0, 4.0),
    )
}

fn render_ghost_fixture() -> Element {
    let highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    let (document, initial_scene, preview_scene) = use_hook(ghost_fixture);
    let accepted_document = use_signal(|| document.clone());
    let mut displayed = use_signal(|| initial_scene.clone());
    rsx! {
        button { id: "canvas-ghost-preview", onclick: move |_| {
            displayed.set(preview_scene.clone());
        }, "Preview" }
        button { id: "canvas-ghost-off", onclick: move |_| {
            displayed.set(initial_scene.clone());
        }, "Clear preview" }
        svg {
            Scene {
                source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                preview_active: false,
                accepted_scene: Some(initial_scene.clone()),
                selected_ids: Vec::new(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
            }
        }
    }
}

/// Ticket 23 slice 10, RED: a transform preview ghosts the pre-drag pose
/// of every moved matrix key until the preview clears.
#[wasm_bindgen_test]
async fn transform_preview_ghosts_the_pre_drag_pose() {
    let root = test_mount_helpers::mount(VirtualDom::new(render_ghost_fixture), None);
    gloo_timers::future::TimeoutFuture::new(5).await;
    let click = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click()
    };
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_none(),
        "the settled scene carries no ghosts"
    );
    click("#canvas-ghost-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    let ghost = root
        .query_selector("[data-ghost='scene']")
        .unwrap()
        .expect("the preview ghosts the pre-drag pose");
    assert!(
        ghost
            .get_attribute("transform")
            .as_deref()
            .unwrap_or_default()
            .contains("translate(0 0)"),
        "the ghost sits on the pre-drag pose"
    );
    click("#canvas-ghost-off");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-ghost='scene']")
            .unwrap()
            .is_none(),
        "clearing the preview retires its ghosts"
    );
}
