//! Ticket 010 adoption: caller-supplied Button state and real preference-menu outcomes.
use super::*;
use crate::{test_role::by_role_name, test_wait::wait_until, ui::button::Button};
use wasm_bindgen_test::*;

fn fixture() -> Element {
    let warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(warning));
    let appearance = use_signal(|| "light");
    use_context_provider(|| AppearanceState(appearance));
    let theme = use_signal(|| "original");
    use_context_provider(|| ThemeState(theme));
    rsx! {
        Button { "Plain" }
        Button { aria_current: "page", aria_pressed: false, "Workspace" }
        Button { aria_pressed: true, "Export" }
        Button { href: "#export", aria_current: "page", "Export link" }
        ThemePicker {}
    }
}

#[wasm_bindgen_test]
async fn header_button_state_and_preference_radio_groups_follow_their_owners() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("header controls render", || {
        by_role_name(&root, "button", "Settings").is_some()
    })
    .await;
    let plain = by_role_name(&root, "button", "Plain").unwrap();
    assert!(!plain.has_attribute("aria-current"));
    assert!(!plain.has_attribute("aria-pressed"));
    for (role, name) in [("button", "Workspace"), ("link", "Export link")] {
        let control = by_role_name(&root, role, name).unwrap();
        assert_eq!(
            control.get_attribute("aria-current").as_deref(),
            Some("page")
        );
    }
    assert_eq!(
        by_role_name(&root, "button", "Workspace")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("false")
    );
    assert_eq!(
        by_role_name(&root, "button", "Export")
            .unwrap()
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true")
    );
    assert!(
        !by_role_name(&root, "link", "Export link")
            .unwrap()
            .has_attribute("aria-pressed")
    );
    let settings = by_role_name(&root, "button", "Settings").unwrap();
    settings.click();
    wait_until("the Settings menu enters the first radio", || {
        document
            .active_element()
            .is_some_and(|e| e.get_attribute("role").as_deref() == Some("menuitemradio"))
    })
    .await;
    for group in ["Appearance", "Theme"] {
        assert!(by_role_name(&root, "group", group).is_some());
    }
    for (label, checked) in [
        ("Light", "true"),
        ("System", "false"),
        ("Dark", "false"),
        ("Original", "true"),
        ("shadcn", "false"),
    ] {
        assert_eq!(
            by_role_name(&root, "menuitemradio", label)
                .unwrap()
                .get_attribute("aria-checked")
                .as_deref(),
            Some(checked)
        );
    }
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    let previous = storage.get_item("boardstudio:v2:appearance").unwrap();
    by_role_name(&root, "menuitemradio", "Dark")
        .unwrap()
        .click();
    wait_until("selection closes Settings and returns focus", || {
        settings.get_attribute("aria-expanded").as_deref() == Some("false")
            && document.active_element().as_ref() == Some(settings.as_ref())
    })
    .await;
    assert_eq!(
        storage
            .get_item("boardstudio:v2:appearance")
            .unwrap()
            .as_deref(),
        Some("dark")
    );
    settings.click();
    wait_until("the owner publishes Dark as checked", || {
        by_role_name(&root, "menuitemradio", "Dark")
            .is_some_and(|e| e.get_attribute("aria-checked").as_deref() == Some("true"))
    })
    .await;
    assert_eq!(
        by_role_name(&root, "menuitemradio", "Light")
            .unwrap()
            .get_attribute("aria-checked")
            .as_deref(),
        Some("false")
    );
    if let Some(previous) = previous {
        storage
            .set_item("boardstudio:v2:appearance", &previous)
            .unwrap();
    } else {
        storage.remove_item("boardstudio:v2:appearance").unwrap();
    }
    root.remove();
}
