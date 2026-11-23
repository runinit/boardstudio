use super::*;
use crate::presentation::test_mount_helpers;
use crate::presentation::test_mount_helpers::canvas::{scene, switch_document};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>) {
    let mut document = switch_document("canvas", "Canvas", &["key", "stationary"]);
    document.parts[1].pose.at.x = 30.0;
    (Arc::new(document), scene(&[]))
}

fn render_fixture() -> Element {
    let mut highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::super::RelationHighlight(highlight));
    let (document, initial_scene) = use_hook(fixture);
    let mut accepted_document = use_signal(|| document.clone());
    let mut displayed = use_signal(|| initial_scene.clone());
    let mut preview = use_signal(|| false);
    let mut generation = use_signal(|| 0_usize);
    let mut observed = use_signal(|| 0_usize);
    let mut selected = use_signal(Vec::<String>::new);
    let mut keys = use_signal(|| true);
    let captured_generation = generation();
    rsx! {
        button { id: "canvas-unrelated", onclick: move |_| generation += 1, "Unrelated update" }
        button { id: "canvas-highlight", onclick: move |_| highlight.set(Some(vec!["key".into()])), "Highlight relation" }
        button { id: "canvas-preview", onclick: move |_| {
            let mut next = (*displayed()).clone();
            next.transforms = vec![Transform { id: "key".into(), pose: Pose2 { at: Vec2 { x: 9.0, y: 4.0 }, rotation: 0.0 } }];
            displayed.set(Arc::new(next));
            preview.set(true);
        }, "Preview" }
        button { id: "canvas-preview-off", onclick: move |_| {
            displayed.set(initial_scene.clone());
            preview.set(false);
        }, "Clear preview" }
        button { id: "canvas-select", onclick: move |_| selected.set(vec!["key".into()]), "Select" }
        button { id: "canvas-accepted", onclick: move |_| {
            let mut next = accepted_document();
            Arc::make_mut(&mut next).parts[0].keycap = Some(Vec2 { x: 30.0, y: 18.0 });
            accepted_document.set(next);
        }, "Accepted geometry" }
        button { id: "canvas-hide", onclick: move |_| keys.set(false), "Hide" }
        output { id: "canvas-observed", "{observed()}" }
        svg {
            Scene {
                source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                preview_active: preview(),
                accepted_scene: None,
                selected_ids: selected(), selected_context: None,
                view_width_world: 200.0, surface_width_px: 800.0,
                keys_visible: keys(), components_visible: true, keycaps_visible: keys(), footprints_visible: false,
                on_matrix_pointer_down: |_| {},
                on_part_pointer_down: move |_| observed.set(captured_generation),
            }
        }
    }
}

#[wasm_bindgen_test]
async fn canvas_skips_unrelated_updates_but_refreshes_callbacks_preview_selection_and_layers() {
    let root = test_mount_helpers::mount(VirtualDom::new(render_fixture), None);
    gloo_timers::future::TimeoutFuture::new(5).await;
    let renders = SCENE_RENDERS.with(|count| count.get());
    let click = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click()
    };
    click("#canvas-unrelated");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        SCENE_RENDERS.with(|count| count.get()),
        renders,
        "unchanged canvas inputs must not rebuild the scene"
    );
    let pointer = js_sys::eval("new PointerEvent('pointerdown', {bubbles: true})")
        .unwrap()
        .dyn_into::<web_sys::Event>()
        .unwrap();
    root.query_selector("[data-geometry='part-hit-area']")
        .unwrap()
        .unwrap()
        .dispatch_event(&pointer)
        .unwrap();
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        root.query_selector("#canvas-observed")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("1"),
        "memoized canvas callbacks must use the latest parent capture"
    );
    click("#canvas-highlight");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='part-outline'][data-relation-highlight='true']")
            .unwrap()
            .is_some()
    );
    assert_eq!(
        SCENE_RENDERS.with(|count| count.get()),
        renders,
        "relation children remain reactive without rebuilding the canvas"
    );
    let part_renders = PART_CONTENT_RENDERS.with(|count| count.get());
    click("#canvas-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        PART_CONTENT_RENDERS.with(|count| count.get()),
        part_renders,
        "pose-only previews must not rebuild unchanged part graphics"
    );
    assert_eq!(
        root.query_selector("[data-geometry='scene-part']")
            .unwrap()
            .unwrap()
            .get_attribute("transform")
            .as_deref(),
        Some("translate(9,4) rotate(0) ")
    );
    click("#canvas-select");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='keycap'][data-selected='true']")
            .unwrap()
            .is_some()
    );
    click("#canvas-accepted");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert_eq!(
        root.query_selector("[data-geometry='part-hit-area']")
            .unwrap()
            .unwrap()
            .get_attribute("width")
            .as_deref(),
        Some("30"),
        "accepted geometry must refresh cached graphics"
    );
    click("#canvas-hide");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-geometry='scene-part']")
            .unwrap()
            .is_none()
    );
}

/// Ticket 07: the selected part's outline turns accent while unselected
/// part outlines stay violet.
#[wasm_bindgen_test]
async fn selected_part_outline_turns_accent_while_unselected_stays_violet() {
    let root = test_mount_helpers::mount(VirtualDom::new(render_fixture), None);
    gloo_timers::future::TimeoutFuture::new(5).await;
    let click = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click()
    };
    click("#canvas-select");
    gloo_timers::future::TimeoutFuture::new(5).await;
    let outlines = root
        .query_selector_all("[data-geometry='part-outline']")
        .unwrap();
    assert_eq!(outlines.length(), 2, "both parts draw outlines");
    let selected = root
        .query_selector_all("[data-geometry='part-outline'][data-selected='true']")
        .unwrap();
    assert_eq!(
        selected.length(),
        1,
        "exactly the selected part turns accent"
    );
}

/// Ticket 23 slice 9, RED: a transform-drag preview marks the effective
/// poses amber until the preview clears.
#[wasm_bindgen_test]
async fn transform_preview_marks_the_scene_amber_until_it_clears() {
    let root = test_mount_helpers::mount(VirtualDom::new(render_fixture), None);
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
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_none(),
        "the settled scene carries no preview mark"
    );
    click("#canvas-preview");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_some(),
        "the preview marks its effective poses amber"
    );
    click("#canvas-preview-off");
    gloo_timers::future::TimeoutFuture::new(5).await;
    assert!(
        root.query_selector("[data-scene-preview='true']")
            .unwrap()
            .is_none(),
        "clearing the preview restores the scene"
    );
}
