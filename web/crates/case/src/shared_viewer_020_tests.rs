//! Mount the leased controls on SharedViewer; its transient values stay in the production owner.
use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> Element {
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
            mechanical_settings: None, inline_case_controls: false, handle_projection: None,
            handles: vec![], handle_preview: None, gesture_message: None, focus_request: None,
        }
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
    control(&root, "button", "Section plane").click();
    wait_until("section plane list is named", || {
        by_role_name(&root, "listbox", "Section plane").is_some()
    })
    .await;
    control(&root, "option", "XY").click();
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
    opener.click();
    wait_until("the same section checkbox survives reopening", || {
        control(&root, "checkbox", "Show section plane") == plane
    })
    .await;
    assert_eq!(
        plane.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    root.remove();
}
