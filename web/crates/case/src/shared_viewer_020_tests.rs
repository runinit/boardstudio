//! Mount the leased controls on SharedViewer; its transient values stay in the production owner.
use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
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
        style { {include_str!("../../../assets/workbench-base.css")} }
        // Use the same generated utilities and theme tokens as the page.
        style { {include_str!("../../../assets/tailwind.css")} }
        section { class: "relative flex min-h-0 flex-col overflow-hidden bg-canvas", {viewer_fixture(true)} }
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
async fn viewer_slot_exclusions_preserve_actual_inline_controls_in_four_themes() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let html = document.document_element().unwrap();
    let original_theme = html.get_attribute("data-theme");
    let original_appearance = html.get_attribute("data-appearance");
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(inline_fixture);
    dom.provide_root_context(crate::runtime::project_name_test_support::new_runtime());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the production inline controls mount", || {
        by_role_name(&root, "button", "Fit").is_some()
    })
    .await;
    let fit = control(&root, "button", "Fit");
    assert_eq!(
        fit.query_selector(".sr-only")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Fit")
    );
    // Copied controls carry data-slot so the legacy element rules skip them.
    assert_eq!(fit.get_attribute("data-slot").as_deref(), Some("button"));
    let shaded = control(&root, "button", "Shaded");
    assert_eq!(
        shaded.get_attribute("data-slot").as_deref(),
        Some("tooltip-trigger")
    );
    shaded.click();
    wait_until("the display control becomes selected", || {
        shaded.get_attribute("aria-pressed").as_deref() == Some("true")
    })
    .await;
    assert_eq!(shaded.get_attribute("data-state").as_deref(), Some("on"));
    let exploded = control(&root, "button", "Exploded");
    exploded.click();
    wait_until("exploded view exposes its production slider", || {
        by_role_name(&root, "slider", "Explode amount").is_some()
    })
    .await;
    let amount = control(&root, "slider", "Explode amount");
    key(&amount, "Home");
    wait_until("explode reaches its minimum", || {
        amount.get_attribute("aria-valuenow").as_deref() == Some("0")
    })
    .await;
    // Icon-only camera buttons describe themselves through their tooltips.
    fit.focus().unwrap();
    wait_until("camera tooltip describes Fit", || {
        fit.get_attribute("aria-describedby")
            .and_then(|id| document.get_element_by_id(&id))
            .is_some_and(|tip| tip.text_content().unwrap_or_default().contains("Fit model"))
    })
    .await;
    let section = control(&root, "button", "Section");
    section.click();
    wait_until("section view exposes its production controls", || {
        by_role_name(&root, "slider", "Section position").is_some()
            && by_role_name(&root, "button", "Section plane").is_some()
            && by_role_name(&root, "checkbox", "Show plane").is_some()
    })
    .await;
    let plane = control(&root, "button", "Section plane");
    plane.focus().unwrap();
    key(&plane, "ArrowDown");
    wait_until("section plane opens its named list", || {
        by_role_name(&root, "listbox", "Section plane").is_some()
            && document.active_element() == by_role_name(&root, "option", "XY").map(Into::into)
    })
    .await;
    key(&control(&root, "option", "XY"), "ArrowDown");
    wait_until("section plane reaches XZ", || {
        document.active_element() == by_role_name(&root, "option", "XZ").map(Into::into)
    })
    .await;
    key(&control(&root, "option", "XZ"), "Enter");
    wait_until("plane selection is shown by the same owner", || {
        control(&root, "button", "Section plane")
            .text_content()
            .is_some_and(|s| s.contains("XZ"))
    })
    .await;
    let show_plane = control(&root, "checkbox", "Show plane");
    show_plane.click();
    wait_until("show plane toggles its transient owner", || {
        show_plane.get_attribute("aria-checked").as_deref() == Some("false")
    })
    .await;
    exploded.click();
    wait_until("exploded stays selected", || {
        exploded.get_attribute("aria-pressed").as_deref() == Some("true")
    })
    .await;
    for theme in ["original", "shadcn"] {
        for appearance in ["light", "dark"] {
            html.set_attribute("data-theme", theme).unwrap();
            html.set_attribute("data-appearance", appearance).unwrap();
            let style =
                |element: &web_sys::Element| window.get_computed_style(element).unwrap().unwrap();
            let fit_style = style(&fit);
            let toolbar = fit
                .closest("[role='group'][aria-label='Assembly camera']")
                .unwrap()
                .unwrap()
                .parent_element()
                .unwrap();
            assert_eq!(
                style(&toolbar).get_property_value("display").unwrap(),
                "flex"
            );
            assert_eq!(
                fit_style.get_property_value("border-top-width").unwrap(),
                "0px"
            );
            assert_eq!(control(&root, "button", "Fit"), fit);
            assert_eq!(exploded.get_attribute("data-state").as_deref(), Some("on"));
            wasm_bindgen_test::console_log!(
                "inline {theme}/{appearance}: Fit height={}, border={}; selected={}",
                fit.get_bounding_client_rect().height(),
                fit_style.get_property_value("border-top-width").unwrap(),
                style(&exploded)
                    .get_property_value("background-color")
                    .unwrap()
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
    let _mounted = MountedFixture::new(root.clone(), dom);
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

fn toolbar_fixture() -> Element {
    let mut mode_3d = use_signal(|| false);
    let mut footprints = use_signal(|| true);
    rsx! {
        DesignViewToolbar {
            label: "Case",
            detail: "detail",
            assembly_3d: mode_3d(),
            footprints_visible: footprints(),
            on_view_mode: move |next: bool| mode_3d.set(next),
            on_toggle_footprints: move |_| footprints.set(!footprints()),
        }
        p { id: "toolbar-probe", "{mode_3d}:{footprints}" }
    }
}

#[wasm_bindgen_test]
async fn design_view_toolbar_uses_toggle_groups_and_keeps_view_state() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(toolbar_fixture));
    wait_until("the view toolbar mounts", || {
        by_role_name(&root, "button", "2D").is_some()
    })
    .await;
    let mode_2d = control(&root, "button", "2D");
    assert_eq!(
        mode_2d.get_attribute("data-slot").as_deref(),
        Some("toggle-group-item")
    );
    assert!(by_role_name(&root, "group", "View mode").is_some());
    assert_eq!(
        mode_2d.get_attribute("aria-pressed").as_deref(),
        Some("true")
    );
    assert_eq!(
        control(&root, "button", "3D assembly")
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("false")
    );
    let probe = || {
        root.query_selector("#toolbar-probe")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
    };
    control(&root, "button", "3D assembly").click();
    wait_until("3D assembly mode is requested", || probe() == "true:true").await;
    assert!(by_role_name(&root, "button", "Footprints").is_none());
    control(&root, "button", "2D").click();
    wait_until("2D mode is requested", || probe() == "false:true").await;
    control(&root, "button", "Footprints").click();
    wait_until("footprints toggle is requested", || {
        probe() == "false:false"
    })
    .await;
    root.remove();
}
