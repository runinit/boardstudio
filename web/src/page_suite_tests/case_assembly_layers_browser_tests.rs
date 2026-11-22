use boardstudio_web_case::case_assembly_layers::*;
use boardstudio_web_case::case_display::CaseDisplay;
use boardstudio_web_runtime::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

use boardstudio_core::model::{PcbModel, Pose2, Side, Vec2, Vec3};

fn model(id: &str, reference: &str, path: &str) -> PcbModel {
    PcbModel {
        id: id.into(),
        reference: reference.into(),
        path: path.into(),
        pose: Pose2 {
            at: Vec2 { x: 1.0, y: 2.0 },
            rotation: 0.25,
        },
        side: Side::Front,
        offset: Vec3::default(),
        rotation: Vec3::default(),
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }
}

fn layer_menu_composition() -> Element {
    let mut display = use_signal(CaseDisplay::default);
    let mut selected_layer = use_signal(String::new);
    let assembly = assembly_layers_with_stack(
        [("plate".into(), "Plate body".into())],
        ["battery".into(), "plate".into()],
    );
    let models = vec![
        model("switch-mesh-1", "SW1", "models/switch.step"),
        model("companion-mesh-1", "SW1", "models/companion.step"),
        model("diode-mesh-1", "D1", "models/diode.step"),
    ];
    let delivery = ModelDeliveryRows {
        delivered: models[..2]
            .iter()
            .map(|model| DeliveredModel {
                id: model.id.clone(),
                mesh: Rc::new(ValidatedMesh {
                    positions: Rc::from([0.0_f32; 9]),
                    normals: Rc::from([0.0_f32; 9]),
                    colors: None,
                    step_bounds: None,
                    vendor_step: None,
                }),
                matrix: None,
            })
            .collect(),
        pending: Vec::new(),
        failures: Vec::new(),
    };
    let components = physical_component_layers(&models, Some(&delivery));
    rsx! {
        CaseAssemblyLayers {
            assembly,
            components,
            selectable_layers: vec!["pcb".into(), "plate".into()],
            selected_layer: selected_layer(),
            on_select_layer: move |id| selected_layer.set(id),
            display: display(),
            on_display_change: move |next| display.set(next),
        }
        output { "aria-label": "Selected layer", "{selected_layer()}" }
        output { "aria-label": "Hidden model ids", {display().hidden.join(",")} }
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

fn computed_display(element: &web_sys::Element) -> String {
    web_sys::window()
        .unwrap()
        .get_computed_style(element)
        .unwrap()
        .unwrap()
        .get_property_value("display")
        .unwrap()
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}

#[wasm_bindgen_test]
fn physical_rows_keep_preview_order_and_exact_model_id_join() {
    let preview_models = vec![
        model("mesh-id-switch", "SW1", "models/switch.step"),
        model("mesh-id-diode", "D1", "models/diode.step"),
    ];
    let delivered = ModelDeliveryRows {
        delivered: vec![DeliveredModel {
            id: "mesh-id-diode".into(),
            mesh: Rc::new(ValidatedMesh {
                positions: Rc::from([0.0_f32; 9]),
                normals: Rc::from([0.0_f32; 9]),
                colors: None,
                step_bounds: None,
                vendor_step: None,
            }),
            matrix: None,
        }],
        pending: vec!["mesh-id-switch".into()],
        failures: Vec::new(),
    };

    let rows = physical_component_layers(&preview_models, Some(&delivered));

    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["mesh-id-switch", "mesh-id-diode"]
    );
    assert_eq!(
        (rows[0].reference.as_str(), rows[0].filename.as_str()),
        ("SW1", "switch.step")
    );
    assert_eq!(rows[0].availability, LayerAvailability::Pending);
    assert_eq!(
        (rows[1].reference.as_str(), rows[1].filename.as_str()),
        ("D1", "diode.step")
    );
    assert_eq!(rows[1].availability, LayerAvailability::Available);
}

#[wasm_bindgen_test]
fn missing_delivery_never_marks_a_preview_model_available() {
    let models = [model("mesh-id", "SW1", "assets/switch.step")];
    let rows = physical_component_layers(&models, None);
    assert_eq!(rows[0].availability, LayerAvailability::Pending);
}

#[wasm_bindgen_test]
async fn layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked() {
    let document = web_sys::window().unwrap().document().unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(concat!(
        include_str!("../../assets/workbench-base.css"),
        include_str!("../../assets/tailwind.css")
    )));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-assembly-layer-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let mut mounted = MountedFixture::new(root.clone(), VirtualDom::new(layer_menu_composition));
    mounted.own_node(stylesheet.clone());
    rendered().await;

    let scope = root.clone();
    let list = element("#m1-case-assembly-layers-list");
    assert!(list.has_attribute("hidden"));
    assert_eq!(computed_display(&list), "none");
    assert_eq!(list.get_bounding_client_rect().height(), 0.0);
    let trigger =
        by_role_name(&scope, "button", "Assembly visibility").expect("assembly visibility trigger");
    assert_eq!(
        trigger.text_content().unwrap().trim(),
        "Assembly visibility"
    );
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    let battery = by_role_name(&scope, "button", "Show battery").expect("unavailable battery row");
    assert!(battery.has_attribute("disabled"));
    assert_eq!(
        battery.get_attribute("aria-pressed").as_deref(),
        Some("false")
    );
    let plate = by_role_name(&scope, "button", "Hide Plate").expect("plate visibility toggle");
    assert!(!plate.has_attribute("disabled"));
    assert_eq!(plate.get_attribute("aria-pressed").as_deref(), Some("true"));

    trigger.click();
    rendered().await;
    assert!(!list.has_attribute("hidden"));
    assert_eq!(computed_display(&list), "grid");
    assert!(list.get_bounding_client_rect().height() > 0.0);
    assert_eq!(
        by_role_name(&scope, "button", "Assembly visibility")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("true")
    );
    // The icon-only visibility toggle describes itself through its tooltip.
    plate.focus().unwrap();
    wait_until("visibility tooltip describes the toggle", || {
        by_role_name(&scope, "button", "Hide Plate").is_some_and(|toggle| {
            toggle
                .get_attribute("aria-describedby")
                .and_then(|id| document.get_element_by_id(&id))
                .is_some_and(|tip| {
                    tip.text_content()
                        .unwrap_or_default()
                        .contains("Hide Plate")
                })
        })
    })
    .await;
    let select_plate = by_role_name(&scope, "button", "Select Plate for appearance")
        .expect("plate appearance row");
    select_plate.click();
    rendered().await;
    assert_eq!(
        element("#case-assembly-layer-test-root output[aria-label='Selected layer']")
            .text_content()
            .as_deref(),
        Some("plate")
    );
    assert_eq!(
        by_role_name(&scope, "button", "Select Plate for appearance")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    assert_eq!(
        by_role_name(&scope, "button", "Hide Plate")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    let hidden_component = by_role_name(&scope, "button", "Hide SW1 · switch.step")
        .expect("component visibility toggle");
    assert_eq!(
        hidden_component.get_attribute("aria-pressed").as_deref(),
        Some("true")
    );
    assert_eq!(
        by_role_name(&scope, "button", "Hide SW1 · companion.step")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    hidden_component.click();
    rendered().await;
    assert_eq!(
        by_role_name(&scope, "button", "Show SW1 · switch.step")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("false")
    );
    assert_eq!(
        by_role_name(&scope, "button", "Hide SW1 · companion.step")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true"),
        "hiding one model must keep its same-reference sibling visible"
    );
    assert_eq!(
        element("#case-assembly-layer-test-root output[aria-label='Hidden model ids']")
            .text_content()
            .as_deref(),
        Some("switch-mesh-1")
    );
    by_role_name(&scope, "button", "Show SW1 · switch.step")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        by_role_name(&scope, "button", "Hide SW1 · switch.step")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    assert_eq!(
        element("#case-assembly-layer-test-root output[aria-label='Hidden model ids']")
            .text_content()
            .as_deref(),
        Some("")
    );
    assert_eq!(
        by_role_name(&scope, "button", "Hide SW1 · companion.step")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    let unavailable =
        by_role_name(&scope, "button", "Show D1 · diode.step").expect("missing model row");
    assert!(unavailable.has_attribute("disabled"));
    assert_eq!(
        unavailable.get_attribute("aria-pressed").as_deref(),
        Some("false")
    );

    let event_init = web_sys::KeyboardEventInit::new();
    event_init.set_key("Escape");
    event_init.set_bubbles(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event_init).unwrap();
    by_role_name(&scope, "button", "Hide SW1 · switch.step")
        .unwrap()
        .dispatch_event(&event)
        .unwrap();
    rendered().await;
    assert!(list.has_attribute("hidden"));
    assert_eq!(computed_display(&list), "none");
    assert_eq!(list.get_bounding_client_rect().height(), 0.0);
    assert_eq!(
        document.active_element().unwrap().id(),
        "m1-case-assembly-layers-trigger"
    );
    root.remove();
    stylesheet.remove();
}
