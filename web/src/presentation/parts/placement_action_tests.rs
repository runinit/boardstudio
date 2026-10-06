use super::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    action: Rc<RefCell<Option<super::super::part_placement::ComponentPlacementAction>>>,
    definition: boardstudio_core::model::PartDefinition,
    apply_to_key: bool,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    rsx! {
        PartsInspectorPlacementAction {
            entry: CatalogEntry {
                definition: Rc::new(probe.definition.clone()),
                source: catalogue::CatalogueSource::Imported,
            },
            apply_to_key: probe.apply_to_key,
            busy: false,
            on_place: EventHandler::new(move |action| *probe.action.borrow_mut() = Some(action)),
        }
    }
}

fn definition(id: &str, kind: &str) -> boardstudio_core::model::PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": kind,
        "kind": kind,
        "courtyard": [{"x": -2.0, "y": -2.0}, {"x": 2.0, "y": 2.0}],
        "pads": []
    }))
    .unwrap()
}

async fn mounted_action(
    root_id: &str,
    definition: boardstudio_core::model::PartDefinition,
    apply_to_key: bool,
) -> (
    Rc<RefCell<Option<super::super::part_placement::ComponentPlacementAction>>>,
    web_sys::HtmlElement,
) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let action = Rc::new(RefCell::new(None));
    let dom = VirtualDom::new(host);
    dom.provide_root_context(Probe {
        action: action.clone(),
        definition,
        apply_to_key,
    });
    dioxus_web::launch::launch_virtual_dom(dom, dioxus_web::Config::new().rootnode(root.into()));
    gloo_timers::future::TimeoutFuture::new(50).await;
    let button = document
        .query_selector(&format!("#{root_id} .m1-parts-place-component"))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    (action, button)
}

#[wasm_bindgen_test]
async fn inspector_disables_a_non_input_definition_for_a_selected_key() {
    let passive = definition("imported:passive", "passive");
    assert!(!matrix_input_available(&passive));
    let (action, button) = mounted_action("parts-passive-test", passive, true).await;

    assert!(button.has_attribute("disabled"));
    assert!(
        button
            .parent_element()
            .unwrap()
            .text_content()
            .unwrap()
            .contains("Clear the key selection to place it as a standalone component.")
    );
    button.click();
    gloo_timers::future::TimeoutFuture::new(20).await;
    assert!(action.borrow().is_none());
}

#[wasm_bindgen_test]
async fn inspector_click_routes_a_switch_to_the_selected_key_action() {
    let mut switch = definition("generator:switch", "switch");
    switch.pads = vec![input_pad("one", "1"), input_pad("two", "2")];
    assert!(matrix_input_available(&switch));
    let (action, button) = mounted_action("parts-switch-test", switch, true).await;

    assert!(!button.has_attribute("disabled"));
    assert_eq!(
        button.text_content().as_deref(),
        Some("Apply to selected key")
    );
    button.click();
    gloo_timers::future::TimeoutFuture::new(20).await;
    assert_eq!(
        action.borrow().as_ref(),
        Some(
            &super::super::part_placement::ComponentPlacementAction::PartsInspector {
                definition_id: "generator:switch".into(),
                kind: boardstudio_core::model::PartKind::Switch,
            }
        )
    );
}

#[wasm_bindgen_test]
async fn inspector_disables_a_switch_without_independent_press_contacts() {
    let switch = definition("generator:switch-no-input", "switch");
    assert!(!matrix_input_available(&switch));
    let (action, button) = mounted_action("parts-switch-no-input-test", switch, true).await;

    assert!(button.has_attribute("disabled"));
    button.click();
    gloo_timers::future::TimeoutFuture::new(20).await;
    assert!(action.borrow().is_none());
}

fn input_pad(id: &str, number: &str) -> boardstudio_core::model::Pad {
    boardstudio_core::model::Pad {
        id: id.into(),
        number: number.into(),
        at: boardstudio_core::model::Vec2::default(),
        size: boardstudio_core::model::Vec2 { x: 1.0, y: 1.0 },
        shape: boardstudio_core::model::PadShape::Rect,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    }
}
