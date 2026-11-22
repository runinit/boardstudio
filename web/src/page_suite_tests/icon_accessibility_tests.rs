use boardstudio_web_ui_shared::test_mount::MountedFixture;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

use boardstudio_web_ui_shared::test_role::{accessible_name, by_role_name};
use boardstudio_web_ui_shared::test_wait::wait_until;
use boardstudio_web_ui_shared::ui::button::Button;
use boardstudio_web_ui_shared::ui::icons::{Check, ChevronDown};

fn fixture() -> Element {
    rsx! {
        Button { Check { class: "size-3.5" } "Save" }
        ChevronDown {}
    }
}

#[wasm_bindgen_test]
async fn vendored_icons_are_decorative_and_use_class_size_and_theme_stroke() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
    wait_until("the icon button renders", || {
        by_role_name(&root, "button", "Save").is_some()
    })
    .await;

    let save = by_role_name(&root, "button", "Save").unwrap();
    assert_eq!(accessible_name(&save), "Save");
    let check = save.query_selector("svg").unwrap().unwrap();
    assert_eq!(check.get_attribute("aria-hidden").as_deref(), Some("true"));
    assert_eq!(check.get_attribute("class").as_deref(), Some("size-3.5"));
    assert_eq!(
        check.get_attribute("stroke-width").as_deref(),
        Some("var(--icon-stroke, 2)")
    );
    let default_icon: web_sys::Element = root
        .query_selector_all("svg")
        .unwrap()
        .item(1)
        .unwrap()
        .dyn_into()
        .unwrap();
    assert!(
        default_icon
            .get_attribute("class")
            .unwrap_or_default()
            .contains("size-4")
    );
    root.remove();
}
