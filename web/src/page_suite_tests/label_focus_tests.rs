use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::ui::label::Label;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> Element {
    rsx! {
        Label { "Implicit target" input { aria_label: "Implicit target" } }
        Label { html_for: "explicit-label-target", "Explicit target" }
        input { id: "explicit-label-target", aria_label: "Explicit target" }
        Label { html_for: "", "Empty target" input { aria_label: "Empty target" } }
    }
}

#[wasm_bindgen_test]
async fn label_omits_none_and_preserves_explicit_id_and_empty_targets() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
    wait_until("Label association cases mount", || {
        by_role_name(&root, "textbox", "Empty target").is_some()
    })
    .await;
    let implicit = by_role_name(&root, "textbox", "Implicit target").unwrap();
    let implicit_label = implicit.closest("label").unwrap().unwrap();
    assert_eq!(implicit_label.get_attribute("for"), None);
    assert_eq!(
        js_sys::Reflect::get(implicit_label.as_ref(), &"control".into()).unwrap(),
        wasm_bindgen::JsValue::from(implicit.clone()),
    );
    implicit_label
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    assert_eq!(document.active_element(), Some(implicit.unchecked_into()));

    let explicit = by_role_name(&root, "textbox", "Explicit target").unwrap();
    let explicit_label = root
        .query_selector("label[for='explicit-label-target']")
        .unwrap()
        .unwrap();
    assert_eq!(
        explicit_label.get_attribute("for").as_deref(),
        Some("explicit-label-target")
    );
    assert_eq!(
        js_sys::Reflect::get(explicit_label.as_ref(), &"control".into()).unwrap(),
        wasm_bindgen::JsValue::from(explicit.clone()),
    );
    explicit_label
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    assert_eq!(document.active_element(), Some(explicit.unchecked_into()));

    let empty = by_role_name(&root, "textbox", "Empty target").unwrap();
    let empty_label = empty.closest("label").unwrap().unwrap();
    assert_eq!(empty_label.get_attribute("for").as_deref(), Some(""));
    assert!(
        js_sys::Reflect::get(empty_label.as_ref(), &"control".into())
            .unwrap()
            .is_null()
    );
    root.remove();
}
