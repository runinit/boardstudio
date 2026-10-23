//! Mount the leased controls on SharedViewer; its transient values stay in the production owner.
use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen_test::wasm_bindgen_test;

fn viewer_fixture(inline_case_controls: bool) -> Element {
    let owner = use_hook(|| {
        let owner = ViewerOwner::new().unwrap();
        // This tests the real control owner; GPU/provider acceptance remains separate.
        owner.active.set(false);
        owner
    });
    let projection = use_hook(|| {
        Rc::new(RendererSceneProjection {
            identity: ViewerIdentity {
                scope: Scope {
                    session_epoch: SessionEpoch(1),
                    document_id: "parts-controls".into(),
                    board_id: "board".into(),
                    instance_id: None,
                },
                snapshot_token: SnapshotToken(1),
                revision: 0,
                viewer_instance: 1,
                projection_generation: 1,
                renderer_sequence: 1,
            },
            input: JsValue::NULL,
            layers: vec![
                ("pcb".into(), "PCB".into()),
                ("plate".into(), "Plate".into()),
            ],
        })
    });
    rsx! {
        SharedViewer {
            projection, owner, assembly_layers: vec![], component_layers: vec![],
            selectable_layers: vec!["pcb".into(), "plate".into()],
            canvas_context: ViewerCanvasContext::Parts, selected_layer: "pcb".to_owned(),
            selected_reference: None, display: CaseDisplay::default(), theme: "light".to_owned(),
            current_source: SourceGuard(Rc::new(|| false)), on_signal: |_| {},
            on_display_change: |_| {},
            mechanical_settings: None, inline_case_controls, handle_projection: None,
            handles: vec![], handle_preview: None, gesture_message: None, focus_request: None,
        }
    }
}

fn fixture() -> Element {
    viewer_fixture(false)
}

fn inline_fixture() -> Element {
    rsx! {
        style { {include_str!("../../../assets/m1.css")} }
        // Native inline controls use these maintained vanilla theme variables and legacy rules.
        style { {include_str!("../../../tailwind.css").replacen("@import \"tailwindcss\";", "", 1)} }
        section { class: "m1-case-panel m1-workspace-content", {viewer_fixture(true)} }
    }
}

fn control(root: &web_sys::Element, role: &str, name: &str) -> web_sys::HtmlElement {
    by_role_name(root, role, name).unwrap_or_else(|| panic!("missing {role} {name}"))
}

fn key(element: &web_sys::HtmlElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    element
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

#[wasm_bindgen_test]
async fn viewer_slot_exclusions_preserve_actual_native_inline_controls_in_four_themes() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let html = document.document_element().unwrap();
    let original_theme = html.get_attribute("data-theme");
    let original_appearance = html.get_attribute("data-appearance");
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(inline_fixture);
    dom.provide_root_context(crate::runtime::project_name_test_support::new_runtime());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the production inline controls mount", || {
        by_role_name(&root, "button", "Fit").is_some()
    })
    .await;
    let fit = control(&root, "button", "Fit");
    let shaded = control(&root, "button", "Shaded");
    shaded.click();
    wait_until("the native display control becomes selected", || {
        shaded.get_attribute("aria-pressed").as_deref() == Some("true")
    })
    .await;
    let exploded = control(&root, "button", "Exploded");
    exploded.click();
    wait_until(
        "the native control still updates its production transient owner",
        || exploded.get_attribute("aria-pressed").as_deref() == Some("true"),
    )
    .await;
    let probe = document.create_element("span").unwrap();
    probe
        .set_attribute("style", "background-color:var(--wb-selection-surface)")
        .unwrap();
    root.append_child(&probe).unwrap();
    for theme in ["original", "shadcn"] {
        for appearance in ["light", "dark"] {
            html.set_attribute("data-theme", theme).unwrap();
            html.set_attribute("data-appearance", appearance).unwrap();
            let style =
                |element: &web_sys::Element| window.get_computed_style(element).unwrap().unwrap();
            let fit_style = style(&fit);
            let shaded_style = style(&shaded);
            assert_eq!(fit.get_attribute("data-slot"), None);
            assert_eq!(
                fit.closest(".m1-case-view-toolbar.board-tool-pill")
                    .unwrap()
                    .unwrap()
                    .class_name(),
                "m1-case-view-toolbar board-tool-pill"
            );
            assert_eq!(
                fit_style.get_property_value("border-top-width").unwrap(),
                "0px"
            );
            assert_eq!(
                fit_style.get_property_value("background-color").unwrap(),
                "rgba(0, 0, 0, 0)"
            );
            assert_eq!(
                shaded_style.get_property_value("background-color").unwrap(),
                style(&probe)
                    .get_property_value("background-color")
                    .unwrap()
            );
            assert_eq!(fit.get_bounding_client_rect().width(), 40.0);
            assert_eq!(
                exploded.get_attribute("aria-pressed").as_deref(),
                Some("true")
            );
            assert_eq!(control(&root, "button", "Exploded"), exploded);
            wasm_bindgen_test::console_log!(
                "native inline {theme}/{appearance}: Fit height={}, border={}, background={}; selected={}",
                fit.get_bounding_client_rect().height(),
                fit_style.get_property_value("border-top-width").unwrap(),
                fit_style.get_property_value("background-color").unwrap(),
                shaded_style.get_property_value("background-color").unwrap()
            );
        }
    }
    root.remove();
    for (name, value) in [
        ("data-theme", original_theme),
        ("data-appearance", original_appearance),
    ] {
        if let Some(value) = value {
            html.set_attribute(name, &value).unwrap();
        } else {
            html.remove_attribute(name).unwrap();
        }
    }
}

#[wasm_bindgen_test]
async fn noninline_parts_viewer_keeps_transient_values_and_control_identity() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(fixture);
    dom.provide_root_context(crate::runtime::project_name_test_support::new_runtime());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the production Parts view controls mount", || {
        by_role_name(&root, "button", "View controls").is_some()
    })
    .await;
    let opener = control(&root, "button", "View controls");
    let content = document
        .get_element_by_id(&opener.get_attribute("aria-controls").unwrap())
        .unwrap();
    assert_eq!(content.get_attribute("data-open").as_deref(), Some("false"));
    opener.click();
    wait_until("the actual retained controls open", || {
        opener.get_attribute("aria-expanded").as_deref() == Some("true")
    })
    .await;
    control(&root, "button", "exploded").click();
    wait_until("exploded view exposes its production slider", || {
        by_role_name(&root, "slider", "Explode amount").is_some()
    })
    .await;
    let amount = control(&root, "slider", "Explode amount");
    key(&amount, "Home");
    wait_until("explode reaches its minimum before the next key", || {
        amount.get_attribute("aria-valuenow").as_deref() == Some("0")
    })
    .await;
    key(&amount, "ArrowRight");
    wait_until("fractional explode value reaches the f32 owner", || {
        amount
            .get_attribute("aria-valuenow")
            .and_then(|v| v.parse::<f64>().ok())
            == Some(f64::from(0.1_f32))
    })
    .await;
    opener.click();
    wait_until("retained controls close", || {
        content.get_attribute("data-open").as_deref() == Some("false")
    })
    .await;
    opener.click();
    wait_until("same controls reopen", || {
        content.get_attribute("data-open").as_deref() == Some("true")
    })
    .await;
    assert_eq!(amount, control(&root, "slider", "Explode amount"));
    assert_eq!(
        amount
            .get_attribute("aria-valuenow")
            .unwrap()
            .parse::<f64>()
            .unwrap(),
        f64::from(0.1_f32)
    );
    key(&amount, "End");
    wait_until("explode maximum preserves its existing bound", || {
        amount.get_attribute("aria-valuenow").as_deref() == Some("10")
    })
    .await;
    control(&root, "button", "section").click();
    wait_until("section view exposes its production controls", || {
        by_role_name(&root, "slider", "Section position").is_some()
    })
    .await;
    let position = control(&root, "slider", "Section position");
    key(&position, "Home");
    wait_until("section minimum stays minus one hundred", || {
        position.get_attribute("aria-valuenow").as_deref() == Some("-100")
    })
    .await;
    key(&position, "End");
    wait_until("section maximum stays one hundred", || {
        position.get_attribute("aria-valuenow").as_deref() == Some("100")
    })
    .await;
    let section = control(&root, "button", "Section plane");
    section.focus().unwrap();
    key(&section, "ArrowDown");
    wait_until(
        "section plane list is named and its first option focused",
        || {
            by_role_name(&root, "listbox", "Section plane").is_some()
                && document.active_element() == by_role_name(&root, "option", "XY").map(Into::into)
        },
    )
    .await;
    key(&control(&root, "option", "XY"), "Enter");
    wait_until("plane selection is shown by the same owner", || {
        control(&root, "button", "Section plane")
            .text_content()
            .is_some_and(|s| s.contains("XY"))
    })
    .await;
    let plane = control(&root, "checkbox", "Show section plane");
    plane.click();
    wait_until("the actual checkbox updates its transient owner", || {
        plane.get_attribute("aria-checked").as_deref() == Some("false")
    })
    .await;
    key(&plane, "Enter");
    assert_eq!(
        plane.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    opener.click();
    wait_until("section controls actually close", || {
        content.get_attribute("data-open").as_deref() == Some("false")
    })
    .await;
    opener.click();
    wait_until("the same section checkbox survives reopening", || {
        content.get_attribute("data-open").as_deref() == Some("true")
            && control(&root, "checkbox", "Show section plane") == plane
    })
    .await;
    assert_eq!(
        plane.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    root.remove();
}
