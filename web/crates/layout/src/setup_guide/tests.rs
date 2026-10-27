use super::*;
use crate::presentation::panels::{self, InspectorPanel, ObjectsPanel, PanelSettings};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn panel_composition() -> Element {
    let mut workspace = use_signal(|| "Layout");
    let mut stage = use_signal(|| SetupGuideStage::Project);
    let mut settings_target = use_signal(|| "");
    let objects_open = use_signal(|| true);
    let inspector_open = use_signal(|| false);
    let objects_settings = use_signal(|| PanelSettings {
        mode: panels::PanelMode::Pinned,
        width: None,
    });
    let inspector_settings = use_signal(|| PanelSettings {
        mode: panels::PanelMode::Collapsed,
        width: None,
    });
    let requested = panels::use_workspace_panel_defaults(workspace(), objects_open, inspector_open);
    rsx! {
        div { class: "m1-editor",
            button { id: "test-layout-route", onclick: move |_| workspace.set("Layout"), "Test ordinary Layout navigation" }
            button { id: "test-case-route", onclick: move |_| workspace.set("Case"), "Test ordinary Case navigation" }
            button { id: "test-reopen-guide", onclick: move |_| activate_stage(SetupGuideStage::Case, workspace, requested, objects_open, inspector_open, objects_settings, inspector_settings), "Test same-workspace guide reveal" }
            button { id: "test-reveal-settings", onclick: move |_| reveal_panels(crate::setup_guide_state::GuideReveal::Settings, objects_open, inspector_open, objects_settings, inspector_settings), "Test settings reveal policy" }
            output { id: "test-settings-target", "{settings_target}" }
            div { class: "m1-editor-body",
                ObjectsPanel { compact_open: objects_open, settings: objects_settings,
                    ProjectSetupGuide {
                        stage: stage(), stage_readiness: [false; 5], stage_detail: stage_detail(stage(), &ProjectDoc::empty("fixture", "Fixture"), "board"), project_name: "Fixture".to_owned(),
                        on_name_change: |_| {}, on_name_commit: |_| {},
                        on_stage_change: move |next| {
                            stage.set(next);
                            activate_stage(next, workspace, requested, objects_open, inspector_open, objects_settings, inspector_settings);
                        },
                        on_open_workspace: move |target| settings_target.set(target),
                        on_open_matrix_setup: None, on_choose_controller: None, on_dismiss: |_| {}, on_finish: |_| {}, project_controls: None,
                    }
                }
                section { class: "m1-workspace-content", "{workspace()}" }
                InspectorPanel { compact_open: inspector_open, settings: inspector_settings,
                    button { "Case settings control" }
                }
            }
        }
    }
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}
fn element(selector: &str) -> web_sys::Element {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(selector)
        .unwrap()
        .unwrap()
}
fn click(selector: &str) {
    element(selector)
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}
fn assert_panels(objects_visible: bool, inspector_visible: bool) {
    for (id, visible) in [
        ("m1-objects-panel", objects_visible),
        ("m1-inspector-panel", inspector_visible),
    ] {
        let panel = element(&format!("#{id}"));
        assert_eq!(
            panel.get_attribute("aria-hidden").as_deref(),
            Some(if visible { "false" } else { "true" }),
            "{id} must remain available on desktop"
        );
        assert_eq!(panel.has_attribute("inert"), !visible);
    }
}

#[wasm_bindgen_test]
async fn desktop_case_stage_and_panel_reveal_policy() {
    let window = web_sys::window().unwrap();
    assert!(
        window
            .match_media("(min-width: 981px)")
            .unwrap()
            .unwrap()
            .matches(),
        "run this composition test with a desktop browser viewport"
    );
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("guide-panel-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(panel_composition),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered().await;
    assert_panels(true, true);
    assert_eq!(
        element("#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("collapsed")
    );
    click(".m1-setup-stage-accordion > div:nth-child(4) > button");
    rendered().await;
    let project_trigger = element(".m1-setup-stage-accordion > div:nth-child(1) > button");
    let stale_clicks = Rc::new(Cell::new(0));
    let count_clicks = stale_clicks.clone();
    let listener = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        count_clicks.set(count_clicks.get() + 1);
    });
    project_trigger
        .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
    let generation = Rc::new(Cell::new(0));
    sync_stage_trigger(SetupGuideStage::Project, generation.clone());
    sync_stage_trigger(SetupGuideStage::Case, generation);
    rendered().await;
    assert_eq!(
        stale_clicks.get(),
        0,
        "a superseded stage must not be clicked"
    );
    project_trigger
        .remove_event_listener_with_callback("click", listener.as_ref().unchecked_ref())
        .unwrap();
    assert_panels(true, true);
    assert_eq!(
        element(".m1-workspace-content").text_content().as_deref(),
        Some("Case")
    );
    assert_eq!(
        element("#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("collapsed"),
        "choosing the guide stage must not open the settings panel"
    );
    let copy = element(".m1-setup-guide").text_content().unwrap();
    assert_eq!(
        copy.matches("Optional. Set the basics here, then refine your case in the full editor.")
            .count(),
        1,
        "Case readiness detail must appear once"
    );
    click(".m1-setup-guide__secondary");
    rendered().await;
    assert_eq!(
        element("#test-settings-target").text_content().as_deref(),
        Some("Case")
    );
    assert_eq!(
        element("#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("collapsed")
    );
    click("#test-reveal-settings");
    rendered().await;
    assert_panels(true, true);
    assert_eq!(
        element("#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
    click("#test-layout-route");
    rendered().await;
    click("#test-case-route");
    rendered().await;
    assert_panels(true, true);
    click("#test-reopen-guide");
    rendered().await;
    assert_panels(true, true);
    assert_eq!(
        element("#m1-objects-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
    assert_eq!(
        element("#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
    root.remove();
}
