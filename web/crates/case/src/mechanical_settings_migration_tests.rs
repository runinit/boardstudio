//! Tests mount the production field so draft and validation ownership remain in Case.
use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen_test::wasm_bindgen_test;

fn identity() -> MechanicalSettingsIdentity {
    MechanicalSettingsIdentity {
        editor_instance_id: 1,
        scope_generation: 1,
        presentation_generation: 1,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        snapshot_token: SnapshotToken(1),
        revision: 1,
        active_board_id: "board".into(),
        configuration_board_id: "board".into(),
    }
}

fn dimension_page() -> Element {
    let sequence = use_signal(|| 0);
    rsx! {
        DimensionField {
            identity: identity(), request_sequence: sequence,
            on_request: move |_| panic!("invalid draft must not submit"),
            feedback: Rc::from([]), field: MechanicalDimension::PlateThickness,
            label: "Plate thickness", value: 1.5, editable: true,
        }
    }
}

#[wasm_bindgen_test]
async fn dimension_error_is_described_by_its_precise_input() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(dimension_page),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("production dimension mounts", || {
        by_role_name(&root, "spinbutton", "Plate thickness").is_some()
    }).await;
    let input: HtmlInputElement = by_role_name(&root, "spinbutton", "Plate thickness")
        .unwrap().dyn_into().unwrap();
    input.set_value("-1");
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap()).unwrap();
    input.dispatch_event(&web_sys::Event::new_with_event_init_dict("focusout", &init).unwrap()).unwrap();
    wait_until("invalid dimension is rejected locally", || {
        input.get_attribute("aria-invalid").as_deref() == Some("true")
    }).await;
    assert_eq!(input.value(), "-1", "the field owns the rejected draft");
    let described = input.get_attribute("aria-describedby")
        .expect("validation must describe the precise input");
    let error = document.get_element_by_id(&described).expect("description resolves");
    assert!(!error.text_content().unwrap().is_empty());
    assert!(root.contains(Some(&error)));
    root.remove();
}
