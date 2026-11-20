//! Exercise the actual Parts picker composition; the primitive owns its search draft.
use super::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn picker_host() -> Element {
    let mut selected = use_signal(|| "0".to_owned());
    let mut count = use_signal(|| 12_usize);
    let mut renders = use_signal(|| 0);
    rsx! {
        PartsChoiceField {
            label: "Host connector", aria_label: "Module host connector", value: selected(),
            options: (0..count()).map(|index| (index.to_string(),
                if index == 0 && renders() > 0 { "Renamed connector".to_owned() }
                else { format!("Connector {index}") })).collect(),
            on_value_change: move |value| selected.set(value),
        }
        Button { onclick: move |_| count.set(13), "More connectors" }
        Button { onclick: move |_| renders += 1, "Refresh caller" }
        output { role: "status", aria_label: "Chosen connector", "{selected}:{renders}" }
    }
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
async fn parts_picker_uses_select_through_twelve_then_keeps_the_combobox_owned_query() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(picker_host));
    wait_until("twelve connectors use the actual Parts Select", || {
        by_role_name(&root, "button", "Module host connector").is_some()
    })
    .await;
    let select = by_role_name(&root, "button", "Module host connector").unwrap();
    assert!(!select.id().is_empty());
    let select_label = document
        .query_selector(&format!("label[for='{}']", select.id()))
        .unwrap()
        .unwrap();
    assert_eq!(
        select_label.text_content().as_deref(),
        Some("Host connector")
    );
    select_label
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until(
        "the actual short-picker label activates its trigger",
        || document.active_element() == Some(select.clone().unchecked_into()),
    )
    .await;
    key(&select, "ArrowDown");
    wait_until(
        "ArrowDown opens the Select and focuses its first option",
        || {
            select.get_attribute("aria-expanded").as_deref() == Some("true")
                && document.active_element()
                    == by_role_name(&root, "option", "Connector 0").map(|e| e.unchecked_into())
        },
    )
    .await;
    assert!(by_role_name(&root, "listbox", "Module host connector").is_some());
    let option = by_role_name(&root, "option", "Connector 0").unwrap();
    key(&option, "End");
    wait_until("End reaches the last connector", || {
        document.active_element()
            == by_role_name(&root, "option", "Connector 11").map(|e| e.unchecked_into())
    })
    .await;
    key(
        &by_role_name(&root, "option", "Connector 11").unwrap(),
        "Enter",
    );
    wait_until("the Parts Select returns focus after selection", || {
        document.active_element() == Some(select.clone().unchecked_into())
    })
    .await;
    assert!(
        by_role_name(&root, "status", "Chosen connector")
            .unwrap()
            .text_content()
            .unwrap()
            .starts_with("11:")
    );
    by_role_name(&root, "button", "More connectors")
        .unwrap()
        .click();
    wait_until("thirteen connectors use the actual Parts Combobox", || {
        by_role_name(&root, "combobox", "Module host connector").is_some()
    })
    .await;
    let combo = by_role_name(&root, "combobox", "Module host connector").unwrap();
    let input: web_sys::HtmlInputElement = combo.clone().unchecked_into();
    let label = combo
        .closest("label")
        .unwrap()
        .expect("the actual Parts picker supplies its visible label");
    assert_eq!(
        js_sys::Reflect::get(label.as_ref(), &"control".into()).unwrap(),
        wasm_bindgen::JsValue::from(input.clone()),
        "the actual Parts long-picker label must associate with its nested Combobox input"
    );
    label.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    wait_until("the actual long-picker label activates its input", || {
        document.active_element() == Some(combo.clone().unchecked_into())
    })
    .await;
    input.set_value("Connector 12");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    wait_until("typing filters inside the actual primitive", || {
        by_role_name(&root, "option", "Connector 12").is_some()
    })
    .await;
    by_role_name(&root, "button", "Refresh caller")
        .unwrap()
        .click();
    wait_until("caller refresh completes", || {
        by_role_name(&root, "status", "Chosen connector")
            .unwrap()
            .text_content()
            .as_deref()
            == Some("11:1")
    })
    .await;
    assert_eq!(
        by_role_name(&root, "combobox", "Module host connector").unwrap(),
        combo
    );
    assert_eq!(
        input.value(),
        "Connector 12",
        "the primitive's local query survives its caller rendering"
    );
    combo.focus().unwrap();
    key(&combo, "ArrowDown");
    wait_until(
        "the actual Combobox identifies its filtered active option",
        || {
            input
                .get_attribute("aria-activedescendant")
                .and_then(|id| document.get_element_by_id(&id))
                .and_then(|option| option.text_content())
                .is_some_and(|text| text.contains("Connector 12"))
        },
    )
    .await;
    key(&combo, "Enter");
    wait_until(
        "the selected connector reaches the caller unchanged",
        || {
            by_role_name(&root, "status", "Chosen connector")
                .unwrap()
                .text_content()
                .as_deref()
                == Some("12:1")
        },
    )
    .await;
    assert_eq!(document.active_element(), Some(combo.unchecked_into()));
    root.remove();
}
