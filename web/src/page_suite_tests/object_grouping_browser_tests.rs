use boardstudio_web_ui_shared::object_options::use_object_options;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_mount::SavedLocalStorage;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use dioxus::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> Element {
    use_object_options()
}

#[wasm_bindgen_test]
async fn grouping_choice_updates_and_persists() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let key = "boardstudio:v2:tree-grouping";
    let _saved = SavedLocalStorage::new(storage.clone(), &[key]).unwrap();
    storage.set_item(key, "column").unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
    wait_until("grouping picker mounts", || {
        by_role_name(&root, "button", "Tree grouping").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Tree grouping").unwrap();
    trigger.click();
    wait_until("grouping choices open", || {
        by_role_name(&root, "option", "Rows").is_some()
    })
    .await;
    let option = by_role_name(&root, "option", "Rows").unwrap();
    for phase in ["pointerdown", "pointerup"] {
        let event = web_sys::PointerEvent::new(phase).unwrap();
        event.init_event_with_bubbles_and_cancelable(phase, true, true);
        option.dispatch_event(&event).unwrap();
    }
    wait_until("grouping updates", || {
        trigger.text_content().as_deref() == Some("Rows")
    })
    .await;

    assert_eq!(storage.get_item(key).unwrap().as_deref(), Some("row"));
}
