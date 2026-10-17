//! Browser coverage of the real form/guide mount boundary used by Editor.
use super::*;
use crate::presentation::setup_guide::{ProjectSetupGuide, SetupGuideStage};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn owner() -> MatrixSetupOwner {
    MatrixSetupOwner {
        editor_instance_id: 1,
        open_id: 1,
        scope_generation: 1,
        scope: boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "focus-fixture".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        board_id: "board".into(),
        snapshot_token: boardstudio_application::SnapshotToken(1),
        revision: 1,
    }
}

fn composition() -> Element {
    let mut open = use_signal(|| false);
    let mut stage = use_signal(|| SetupGuideStage::Layout);
    let on_open = use_callback(move |_| open.set(true));
    let owner = owner();
    rsx! {
        div {
            ProjectSetupGuide {
                stage: stage(), stage_readiness: [false; 5], stage_detail: "Ready".to_owned(), project_name: "Focus fixture".to_owned(),
                on_name_change: |_| {}, on_name_commit: |_| {}, on_stage_change: move |next| stage.set(next),
                on_open_workspace: |_| {}, on_open_matrix_setup: Some(on_open),
                on_choose_controller: None, on_dismiss: |_| {}, on_finish: |_| {}, project_controls: None,
            }
            if open() {
                MatrixSetup {
                    projection: MatrixSetupProjection { owner, editable: true, can_cancel: true, status: None, error: None },
                    on_cancel: move |_| open.set(false), on_create: |_| {},
                }
            }
        }
    }
}

fn canvas_opener_composition() -> Element {
    let mut open = use_signal(|| false);
    let owner = owner();
    rsx! {
        button { id: "matrix-canvas-opener", onclick: move |_| open.set(true), "Add key matrix" }
        if open() {
            MatrixSetup {
                projection: MatrixSetupProjection { owner, editable: true, can_cancel: true, status: None, error: None },
                on_cancel: move |_| open.set(false), on_create: |_| {},
            }
        }
    }
}

fn element(selector: &str) -> web_sys::HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(selector)
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn focused() -> web_sys::Element {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .active_element()
        .unwrap()
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(80).await;
}

#[wasm_bindgen_test]
async fn matrix_cancel_restores_guide_focus_and_new_form_focuses_rows() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("matrix-focus-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(composition),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered().await;
    let initial_heading = focused().tag_name() == "H2";
    let add_matrix = element("#matrix-focus-test-root .m1-setup-guide__add-matrix");
    add_matrix.focus().unwrap();
    add_matrix.click();
    rendered().await;
    let rows_focused = focused().get_attribute("aria-label").as_deref() == Some("Rows");
    let guide_stays_mounted = root.query_selector_all(".m1-setup-guide").unwrap().length() == 1;
    let cancel = element("#matrix-focus-test-root .m1-matrix-setup footer button[type='button']");
    cancel.focus().unwrap();
    cancel.click();
    rendered().await;
    let returned_to_opener = focused().get_attribute("class").is_some_and(|class| {
        class
            .split_whitespace()
            .any(|name| name == "m1-setup-guide__add-matrix")
    });
    assert_eq!(
        (
            initial_heading,
            rows_focused,
            guide_stays_mounted,
            returned_to_opener
        ),
        (true, true, true, true),
        "the guide stays mounted while the form opens, preserves its draft and restores focus to its opener on Cancel; current focus: {}",
        focused().tag_name()
    );
    // A guide-stage render must preserve the user's focused stage button.
    let stage =
        element("#matrix-focus-test-root .m1-setup-stage-accordion > div:nth-child(4) > button");
    stage.focus().unwrap();
    stage.click();
    rendered().await;
    assert_eq!(focused(), web_sys::Element::from(stage));
    // Do not leave an outside focus owner for the next mounted browser test.
    root.remove();
}

#[wasm_bindgen_test]
async fn matrix_cancel_restores_the_canvas_opener_without_a_guide() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("matrix-focus-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(canvas_opener_composition),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered().await;
    let opener = element("#matrix-focus-test-root #matrix-canvas-opener");
    opener.focus().unwrap();
    opener.click();
    rendered().await;
    let cancel = element("#matrix-focus-test-root .m1-matrix-setup footer button[type='button']");
    cancel.focus().unwrap();
    cancel.click();
    rendered().await;
    assert!(
        focused().is_same_node(Some(&opener.clone().into())),
        "without a guide, Cancel returns focus to the canvas action"
    );
    root.remove();
}
