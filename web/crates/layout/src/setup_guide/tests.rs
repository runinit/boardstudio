use super::*;
use crate::presentation::panels::{self, InspectorPanel, ObjectsPanel, PanelSettings};
use boardstudio_web_ui_shared::test_mount::{MountedFixture, SavedLocalStorage};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn panel_composition() -> Element {
    let mut workspace = use_signal(|| "Layout");
    let mut stage = use_signal(|| SetupGuideStage::Project);
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
        div { class: "flex min-h-0 flex-1 flex-col",
            button { id: "test-layout-route", onclick: move |_| workspace.set("Layout"), "Test ordinary Layout navigation" }
            button { id: "test-case-route", onclick: move |_| workspace.set("Case"), "Test ordinary Case navigation" }
            button { id: "test-reopen-guide", onclick: move |_| activate_stage(SetupGuideStage::Case, workspace, requested, objects_open, inspector_open, objects_settings, inspector_settings), "Test same-workspace guide reveal" }
            div { class: "relative grid min-h-0 flex-1 grid-cols-[264px_minmax(0,1fr)_360px] overflow-hidden max-[760px]:flex max-[760px]:flex-col max-[760px]:overflow-visible",
                ObjectsPanel { compact_open: objects_open, settings: objects_settings,
                    ProjectSetupGuide {
                        stage: stage(), stage_readiness: [false; 5], stage_detail: stage_detail(stage(), &ProjectDoc::empty("fixture", "Fixture"), "board"), project_name: "Fixture".to_owned(),
                        on_name_change: |_| {}, on_name_commit: |_| {},
                        on_stage_change: move |next| {
                            stage.set(next);
                            activate_stage(next, workspace, requested, objects_open, inspector_open, objects_settings, inspector_settings);
                        },
                        on_open_workspace: move |_| reveal_panels(crate::setup_guide_state::GuideReveal::Settings, objects_open, inspector_open, objects_settings, inspector_settings),
                        on_open_matrix_setup: None, on_choose_controller: None, on_dismiss: |_| {}, on_finish: |_| {}, project_controls: None,
                    }
                }
                section { class: "relative col-start-2 row-start-1 flex min-h-0 min-w-0 flex-col overflow-auto bg-canvas [container-type:inline-size] max-[760px]:order-1 max-[760px]:min-h-[54svh] max-[760px]:overflow-visible", role: "region", aria_label: "Fixture workspace", "{workspace()}" }
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
fn element(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector).unwrap().unwrap()
}

fn click(root: &web_sys::Element, name: &str) {
    boardstudio_web_ui_shared::test_role::by_role_name(root, "button", name)
        .unwrap()
        .click();
}

fn assert_panels(root: &web_sys::Element, objects_visible: bool, inspector_visible: bool) {
    for (id, visible) in [
        ("m1-objects-panel", objects_visible),
        ("m1-inspector-panel", inspector_visible),
    ] {
        let panel = element(root, &format!("#{id}"));
        assert_eq!(
            panel.get_attribute("aria-hidden").as_deref(),
            Some(if visible { "false" } else { "true" }),
            "{id} must remain available on desktop"
        );
        assert_eq!(panel.has_attribute("inert"), !visible);
    }
}

#[wasm_bindgen_test]
async fn desktop_case_stage_opens_settings_and_keeps_the_guide_available() {
    let window = web_sys::window().unwrap();
    assert!(
        window
            .match_media("(min-width: 980px)")
            .unwrap()
            .unwrap()
            .matches(),
        "run this composition test with a desktop browser viewport"
    );
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    let _preferences = SavedLocalStorage::new(
        window.local_storage().unwrap().unwrap(),
        &[
            "boardstudio:v2:panel:left",
            "boardstudio:v2:panel:right",
            "boardstudio:v2:setup-guide:fixture",
        ],
    )
    .unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(panel_composition));
    rendered().await;
    assert_panels(&root, true, true);
    assert_eq!(
        element(&root, "#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("collapsed")
    );
    click(&root, "Case (optional)");
    rendered().await;
    assert_panels(&root, true, true);
    assert_eq!(
        element(&root, "[role=region][aria-label='Fixture workspace']")
            .text_content()
            .as_deref(),
        Some("Case")
    );
    assert_eq!(
        element(&root, "#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("collapsed"),
        "choosing the guide stage must not open the settings panel"
    );
    let copy = element(&root, "[aria-label='Project setup guide']")
        .text_content()
        .unwrap();
    assert_eq!(
        copy.matches("Optional. Set the basics here, then refine your case in the full editor.")
            .count(),
        1,
        "Case readiness detail must appear once"
    );
    click(&root, "Open full case settings");
    rendered().await;
    assert_panels(&root, true, true);
    assert_eq!(
        element(&root, "#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
    click(&root, "Test ordinary Layout navigation");
    rendered().await;
    click(&root, "Test ordinary Case navigation");
    rendered().await;
    assert_panels(&root, true, true);
    click(&root, "Test same-workspace guide reveal");
    rendered().await;
    assert_panels(&root, true, true);
    assert_eq!(
        element(&root, "#m1-objects-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
    assert_eq!(
        element(&root, "#m1-inspector-panel")
            .get_attribute("data-mode")
            .as_deref(),
        Some("pinned")
    );
}

fn freely_navigable_guide() -> Element {
    let mut stage = use_signal(|| SetupGuideStage::Wiring);
    let mut name = use_signal(|| "Guide fixture".to_owned());
    rsx! {
        ProjectSetupGuide {
            stage: stage(), stage_readiness: [false, true, false, true, false],
            stage_detail: "Fixture prerequisites".to_owned(), project_name: name(),
            on_name_change: move |value| name.set(value), on_name_commit: |_| {},
            on_stage_change: move |value| stage.set(value),
            on_open_workspace: |_| {}, on_open_matrix_setup: None,
            on_choose_controller: None, on_dismiss: |_| {}, on_finish: |_| {},
            project_controls: None,
        }
    }
}

#[wasm_bindgen_test]
async fn guide_current_and_ready_steps_remain_freely_navigable_without_losing_the_name_draft() {
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(freely_navigable_guide));
    wait_until("the named current guide step is available", || {
        by_role_name(&root, "button", "Controller & wiring").is_some()
    })
    .await;
    let current = by_role_name(&root, "button", "Controller & wiring").unwrap();
    assert_eq!(
        current.get_attribute("aria-current").as_deref(),
        Some("step")
    );
    for (title, ready) in [
        ("Project & hardware", false),
        ("Layout & assemblies", true),
        ("Controller & wiring", false),
        ("Case (optional)", true),
    ] {
        let trigger = by_role_name(&root, "button", title).unwrap();
        assert!(!trigger.has_attribute("disabled"));
        assert_eq!(
            trigger.get_attribute("aria-description").as_deref(),
            Some(if ready { "Ready" } else { "Not ready" })
        );
    }
    assert_eq!(
        root.query_selector_all("[data-slot=stepper][data-orientation=Vertical]")
            .unwrap()
            .length(),
        4
    );
    by_role_name(&root, "button", "Project & hardware")
        .unwrap()
        .click();
    wait_until("the project step becomes current", || {
        by_role_name(&root, "button", "Project & hardware")
            .unwrap()
            .get_attribute("aria-current")
            .as_deref()
            == Some("step")
    })
    .await;
    let field: web_sys::HtmlInputElement = by_role_name(&root, "textbox", "Project name")
        .unwrap()
        .dyn_into()
        .unwrap();
    field.set_value("Uncommitted guide draft");
    let event = web_sys::Event::new("input").unwrap();
    event.init_event_with_bubbles_and_cancelable("input", true, true);
    field.dispatch_event(&event).unwrap();
    by_role_name(&root, "button", "Case (optional)")
        .unwrap()
        .click();
    wait_until("a later ready stage is current", || {
        by_role_name(&root, "button", "Case (optional)")
            .unwrap()
            .get_attribute("aria-current")
            .as_deref()
            == Some("step")
    })
    .await;
    by_role_name(&root, "button", "Project & hardware")
        .unwrap()
        .click();
    wait_until(
        "returning retains the original field owner and draft",
        || {
            let current = by_role_name(&root, "textbox", "Project name").unwrap();
            current.is_same_node(Some(&field)) && field.value() == "Uncommitted guide draft"
        },
    )
    .await;
}
