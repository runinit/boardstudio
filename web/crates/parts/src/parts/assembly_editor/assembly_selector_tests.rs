use super::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    assembly: AssemblyDefinition,
    component_options: Vec<ModelOption>,
    asset_options: Vec<ModelOption>,
    component_signal: Rc<RefCell<Option<Signal<Vec<ModelOption>>>>>,
    asset_signal: Rc<RefCell<Option<Signal<Vec<ModelOption>>>>>,
}

#[component]
fn persisted_assembly_fixture() -> Element {
    let probe = use_context::<Probe>();
    let component_options_signal = use_signal(Vec::<ModelOption>::new);
    let asset_options_signal = use_signal(Vec::<ModelOption>::new);
    *probe.component_signal.borrow_mut() = Some(component_options_signal);
    *probe.asset_signal.borrow_mut() = Some(asset_options_signal);
    let assembly = probe.assembly.clone();
    let member = assembly.members.first().expect("persisted member").clone();
    rsx! {
        select {
            id: "persisted-component",
            value: member.definition_id.as_deref().unwrap_or(""),
            for (row_id, row_name) in component_option_rows(&component_options_signal(), member.definition_id.as_deref()) {
                option { key: "component-{row_id}", value: "{row_id}", "{row_name}" }
            }
        }
        for (index, model) in member.models.iter().enumerate() {
            select {
                id: "persisted-model-{index}",
                value: "{model.asset_id}",
                for (row_id, row_name) in model_asset_option_rows(&asset_options_signal(), &model.asset_id) {
                    option { key: "model-{index}-{row_id}", value: "{row_id}", "{row_name}" }
                }
            }
        }
    }
}

#[wasm_bindgen_test]
async fn persisted_member_selection_survives_dynamic_options_mount() {
    let assembly: AssemblyDefinition = serde_json::from_value(serde_json::json!({
        "id": "assembly-f46",
        "name": "F46 saved assembly",
        "members": [{
            "id": "switch",
            "definitionId": "generator:ceoloide/switch_mx",
            "modelMode": "custom",
            "parameters": { "hotswap": true },
            "pose": { "at": { "x": 4.0, "y": 0.0 }, "rotation": 0.0 },
            "side": "back",
            "models": [
                {
                    "assetId": "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp",
                    "offset": { "x": 0.0, "y": 0.0, "z": -1.8709399700164795 },
                    "rotation": { "x": 180.0, "y": 0.0, "z": 0.0 },
                    "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
                },
                {
                    "assetId": "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp",
                    "offset": { "x": 0.007089999970048666, "y": 0.007089999970048666, "z": -1.3574800491333008 },
                    "rotation": { "x": 180.0, "y": 0.0, "z": 0.0 },
                    "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
                }
            ]
        }]
    }))
    .expect("saved assembly archive shape");
    let member = &assembly.members[0];
    assert_eq!(
        member.definition_id.as_deref(),
        Some("generator:ceoloide/switch_mx")
    );
    assert_eq!(
        member.model_mode,
        Some(boardstudio_model::model::AssemblyModelMode::Custom)
    );
    assert_eq!(member.side, Side::Back);
    assert_eq!(member.pose.at.x, 4.0);

    let probe = Probe {
        assembly,
        component_options: vec![
            ModelOption {
                id: "assembly-preset-mx-hotswap-south-left-keys-0/definition/switch".into(),
                name: "switch mx".into(),
            },
            ModelOption {
                id: "generator:ceoloide/switch_mx".into(),
                name: "switch mx".into(),
            },
        ],
        asset_options: vec![
            ModelOption {
                id: "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp".into(),
                name: "SW_Hotswap_Kailh_MX.stp".into(),
            },
            ModelOption {
                id: "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp".into(),
                name: "SW_Cherry_MX_PCB.stp".into(),
            },
        ],
        component_signal: Rc::new(RefCell::new(None)),
        asset_signal: Rc::new(RefCell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("parts-assembly-persisted-select-test");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(persisted_assembly_fixture);
    dom.provide_root_context(probe.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    gloo_timers::future::TimeoutFuture::new(40).await;

    let mut component_signal = probe.component_signal.borrow().as_ref().copied().unwrap();
    component_signal.set(probe.component_options.clone());
    let mut asset_signal = probe.asset_signal.borrow().as_ref().copied().unwrap();
    asset_signal.set(probe.asset_options.clone());
    gloo_timers::future::TimeoutFuture::new(40).await;

    let selected_value = |selector: &str| {
        let select = root
            .query_selector(selector)
            .unwrap()
            .expect("persisted select");
        js_sys::Reflect::get(select.as_ref(), &JsValue::from_str("value"))
            .unwrap()
            .as_string()
            .unwrap()
    };
    assert_eq!(
        selected_value("#persisted-component"),
        "generator:ceoloide/switch_mx"
    );
    assert_eq!(
        selected_value("#persisted-model-0"),
        "bundled-model:kiswitch/SW_Hotswap_Kailh_MX.stp"
    );
    assert_eq!(
        selected_value("#persisted-model-1"),
        "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp"
    );
    root.remove();
}
