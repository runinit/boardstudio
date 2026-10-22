//! Ticket 007: exercise the copied menu, not a test implementation of its behaviour.
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Element as DomElement, HtmlElement, KeyboardEvent, KeyboardEventInit};

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::dropdown_menu::{
    DropdownMenu, DropdownMenuContent, DropdownMenuGroup, DropdownMenuItem, DropdownMenuLabel,
    DropdownMenuTrigger,
};

fn fixture() -> Element {
    let mut outcome = use_signal(|| "None".to_string());
    let mut snap = use_signal(|| false);
    let mut appearance = use_signal(|| "Light".to_string());
    let mut controlled = use_signal(|| false);
    rsx! {
        DropdownMenu {
            DropdownMenuTrigger { "Project" }
            DropdownMenuContent {
                DropdownMenuItem::<&'static str> { value: "New", index: 0usize, disabled: true, "New" }
                DropdownMenuItem { value: "Open", index: 1usize,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Open" }
                DropdownMenuItem { value: "Save", index: 2usize, disabled: true,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Save" }
                DropdownMenuItem { value: "Export", index: 3usize,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Export" }
                DropdownMenuItem { value: "Example", index: 4usize,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Example" }
                DropdownMenuItem::<&'static str> { value: "Snap", index: 5usize,
                    role: "menuitemcheckbox", aria_checked: snap(),
                    on_select: move |_| snap.toggle(), "Snap" }
                DropdownMenuGroup {
                    DropdownMenuLabel { "Appearance" }
                    DropdownMenuItem { value: "Light", index: 6usize,
                        role: "menuitemradio", aria_checked: appearance() == "Light",
                        on_select: move |value: &'static str| appearance.set(value.to_string()), "Light" }
                    DropdownMenuItem { value: "Dark", index: 7usize,
                        role: "menuitemradio", aria_checked: appearance() == "Dark",
                        on_select: move |value: &'static str| appearance.set(value.to_string()), "Dark" }
                }
            }
        }
        DropdownMenu { id: "transform-actions", open: controlled(), on_open_change: move |value| controlled.set(value),
            DropdownMenuTrigger { "Transform" }
            DropdownMenuContent {
                DropdownMenuItem { value: "Rotate", index: 0usize,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Rotate" }
                DropdownMenuItem { value: "Mirror", index: 1usize,
                    on_select: move |value: &'static str| outcome.set(value.to_string()), "Mirror" }
            }
        }
        DropdownMenu { default_open: true,
            DropdownMenuTrigger { "Layout" }
            DropdownMenuContent {
                DropdownMenuItem::<&'static str> { value: "Align", index: 0usize, "Align" }
            }
        }
        button { onclick: move |_| controlled.set(false), "Close externally" }
        output { aria_label: "Action outcome", "{outcome}" }
    }
}

fn control(root: &DomElement, role: &str, name: &str) -> HtmlElement {
    by_role_name(root, role, name).unwrap_or_else(|| panic!("missing {role} {name}"))
}

fn key(element: &HtmlElement, key: &str) {
    let init = KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    element
        .dispatch_event(
            &KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

fn active() -> HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .active_element()
        .unwrap()
        .dyn_into()
        .unwrap()
}

async fn focused(root: &DomElement, role: &str, name: &str) {
    wait_until(&format!("focus reaches {role} {name}"), || {
        by_role_name(root, role, name).is_some_and(|item| item == active())
    })
    .await;
}

async fn opened(root: &DomElement, name: &str) {
    let trigger = control(root, "button", name);
    trigger.focus().unwrap();
    key(&trigger, "Enter");
    wait_until(&format!("{name} menu opens"), || {
        by_role_name(root, "menu", name).is_some_and(|menu| {
            trigger.get_attribute("aria-controls").as_deref() == Some(menu.id().as_str())
        })
    })
    .await;
    assert_eq!(
        trigger.get_attribute("aria-haspopup").as_deref(),
        Some("menu")
    );
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
}

async fn closed(root: &DomElement, name: &str) {
    wait_until(&format!("{name} closes and returns focus"), || {
        let trigger = control(root, "button", name);
        trigger.get_attribute("aria-expanded").as_deref() == Some("false")
            && trigger == active()
            && by_role_name(root, "menu", name).is_none()
    })
    .await;
}

#[wasm_bindgen_test]
async fn action_menu_keyboard_roles_focus_and_action_outcomes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("representative menus render", || {
        by_role_name(&root, "button", "Project").is_some()
    })
    .await;

    // default_open must enter the first enabled item, just like an interactive open.
    focused(&root, "menuitem", "Align").await;
    key(&active(), "Escape");
    closed(&root, "Layout").await;

    opened(&root, "Project").await;
    focused(&root, "menuitem", "Open").await;
    assert_eq!(
        control(&root, "menuitem", "Save")
            .get_attribute("aria-disabled")
            .as_deref(),
        Some("true")
    );
    for (pressed, role, name) in [
        ("ArrowDown", "menuitem", "Export"),
        ("ArrowUp", "menuitem", "Open"),
        ("End", "menuitemradio", "Dark"),
        ("ArrowDown", "menuitem", "Open"),
        ("ArrowUp", "menuitemradio", "Dark"),
        ("Home", "menuitem", "Open"),
        ("E", "menuitem", "Export"),
        ("e", "menuitem", "Example"),
        ("e", "menuitem", "Export"),
        ("x", "menuitem", "Export"),
        ("a", "menuitem", "Example"),
    ] {
        key(&active(), pressed);
        focused(&root, role, name).await;
    }
    key(&active(), "Enter");
    closed(&root, "Project").await;
    assert_eq!(
        root.query_selector("output")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Example")
    );

    opened(&root, "Project").await;
    focused(&root, "menuitem", "Open").await;
    control(&root, "menuitemcheckbox", "Snap").click();
    closed(&root, "Project").await;
    opened(&root, "Project").await;
    assert_eq!(
        control(&root, "menuitemcheckbox", "Snap")
            .get_attribute("aria-checked")
            .as_deref(),
        Some("true")
    );
    key(&active(), "End");
    focused(&root, "menuitemradio", "Dark").await;
    key(&active(), "Enter");
    closed(&root, "Project").await;
    opened(&root, "Project").await;
    assert_eq!(
        control(&root, "menuitemradio", "Dark")
            .get_attribute("aria-checked")
            .as_deref(),
        Some("true")
    );
    assert_eq!(
        control(&root, "menuitemradio", "Light")
            .get_attribute("aria-checked")
            .as_deref(),
        Some("false")
    );
    control(&root, "menuitem", "Save").click();
    assert_eq!(
        control(&root, "button", "Project")
            .get_attribute("aria-expanded")
            .as_deref(),
        Some("true")
    );
    key(&active(), "Escape");
    closed(&root, "Project").await;

    opened(&root, "Transform").await;
    focused(&root, "menuitem", "Rotate").await;
    key(&active(), "ArrowDown");
    focused(&root, "menuitem", "Mirror").await;
    key(&active(), "Enter");
    closed(&root, "Transform").await;
    assert_eq!(
        root.query_selector("output")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Mirror")
    );
    opened(&root, "Transform").await;
    focused(&root, "menuitem", "Rotate").await;
    control(&root, "button", "Close externally").click();
    closed(&root, "Transform").await;
    opened(&root, "Transform").await;
    focused(&root, "menuitem", "Rotate").await;
    let outside = control(&root, "button", "Close externally");
    outside.focus().unwrap();
    wait_until("outside focus dismisses without being stolen", || {
        control(&root, "button", "Transform")
            .get_attribute("aria-expanded")
            .as_deref()
            == Some("false")
            && active() == outside
    })
    .await;
    root.remove();
}

fn bounded_fixture() -> Element {
    let mut requested = use_signal(|| false);
    rsx! {
        DropdownMenu { disabled: true,
            DropdownMenuTrigger { "Unavailable" }
            DropdownMenuContent {
                DropdownMenuItem::<&'static str> { value: "Hidden", index: 0usize, "Hidden" }
            }
        }
        DropdownMenu { open: Some(false), on_open_change: move |_| requested.set(true),
            DropdownMenuTrigger { "Caller controlled" }
            DropdownMenuContent {
                DropdownMenuItem::<&'static str> { value: "Hidden", index: 0usize, "Hidden" }
            }
        }
        div { role: "status", aria_label: "Open request", "{requested}" }
        DropdownMenu { roving_loop: false,
            DropdownMenuTrigger {
                r#as: move |attributes: Vec<Attribute>| rsx! { button { ..attributes, "Settings" } },
            }
            DropdownMenuContent { id: "bounded-settings-menu",
                DropdownMenuItem::<&'static str> { value: "Zoom", index: 10usize, aria_label: "Zoom in", "+" }
                DropdownMenuItem::<&'static str> { value: "Mirror", index: 2usize, "Mirror" }
            }
        }
    }
}

#[wasm_bindgen_test]
async fn action_menu_respects_control_disabled_custom_trigger_and_nonlooping_order() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(bounded_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("bounded menu fixtures render", || {
        by_role_name(&root, "button", "Settings").is_some()
    })
    .await;
    let disabled = control(&root, "button", "Unavailable");
    assert!(disabled.has_attribute("disabled"));
    key(&disabled, "Enter");
    disabled.click();
    assert_eq!(
        disabled.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );

    let controlled = control(&root, "button", "Caller controlled");
    controlled.click();
    wait_until("the caller receives an open request", || {
        control(&root, "status", "Open request")
            .text_content()
            .as_deref()
            == Some("true")
    })
    .await;
    assert_eq!(
        controlled.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    assert!(by_role_name(&root, "menu", "Caller controlled").is_none());

    opened(&root, "Settings").await;
    focused(&root, "menuitem", "Mirror").await;
    assert_eq!(
        control(&root, "menu", "Settings").id(),
        "bounded-settings-menu"
    );
    for (pressed, name) in [
        ("ArrowUp", "Mirror"),
        ("End", "Zoom in"),
        ("ArrowDown", "Zoom in"),
        ("Home", "Mirror"),
        ("z", "Zoom in"),
    ] {
        key(&active(), pressed);
        focused(&root, "menuitem", name).await;
    }
    key(&active(), " ");
    closed(&root, "Settings").await;
    root.remove();
}
