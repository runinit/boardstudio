//! Browser coverage of the real form/guide mount boundary used by Editor.
use super::*;
use crate::presentation::setup_guide::{ProjectSetupGuide, SetupGuideStage};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn composition() -> Element {
    let mut open = use_signal(|| false);
    let mut stage = use_signal(|| SetupGuideStage::Layout);
    let on_open = use_callback(move |_| open.set(true));
    let owner = MatrixSetupOwner {
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
    };
    rsx! {
        div {
            if open() {
                MatrixSetup {
                    projection: MatrixSetupProjection { owner, editable: true, can_cancel: true, status: None, error: None },
                    on_cancel: move |_| open.set(false), on_create: |_| {},
                }
            } else {
                ProjectSetupGuide {
                    stage: stage(), stage_detail: "Ready".to_owned(), project_name: "Focus fixture".to_owned(),
                    on_name_change: |_| {}, on_name_commit: |_| {}, on_stage_change: move |next| stage.set(next),
                    on_open_workspace: |_| {}, on_open_matrix_setup: Some(on_open),
                    on_choose_controller: None, on_dismiss: |_| {}, project_controls: None,
                }
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
        dioxus_web::Config::new().rootnode(root.into()),
    );
    rendered().await;
    let initial_heading = focused().tag_name() == "H2";
    element("#matrix-focus-test-root .m1-setup-guide__content .m1-setup-guide__primary").click();
    rendered().await;
    let rows_focused = focused().get_attribute("aria-label").as_deref() == Some("New matrix rows");
    // Keep Columns focused through a real input render. Mount-only focus must not steal it.
    let columns = element("#matrix-focus-test-root input[aria-label='New matrix columns']");
    columns.focus().unwrap();
    columns
        .dyn_ref::<web_sys::HtmlInputElement>()
        .unwrap()
        .set_value("3");
    let input_event = web_sys::Event::new("input").unwrap();
    input_event.init_event_with_bubbles("input", true);
    columns.dispatch_event(&input_event).unwrap();
    rendered().await;
    let columns_preserved =
        focused().get_attribute("aria-label").as_deref() == Some("New matrix columns");
    let cancel = element("#matrix-focus-test-root .m1-matrix-setup footer button[type='button']");
    cancel.focus().unwrap();
    cancel.click();
    rendered().await;
    let returned_heading = focused().tag_name() == "H2"
        && focused().text_content().as_deref() == Some("Keyboard setup");
    assert_eq!(
        (
            initial_heading,
            rows_focused,
            columns_preserved,
            returned_heading
        ),
        (true, true, true, true),
        "guide mount, form mount, draft rerender, and Cancel must preserve their actual focus lifetimes; current focus: {}",
        focused().tag_name()
    );
    // A guide-stage render must preserve the user's focused stage button.
    let stage = element("#matrix-focus-test-root .m1-setup-guide__steps button:nth-child(4)");
    stage.focus().unwrap();
    stage.click();
    rendered().await;
    assert_eq!(focused(), web_sys::Element::from(stage));
}
