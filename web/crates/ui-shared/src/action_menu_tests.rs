//! Ticket 007: exercise the copied menu, not a test implementation of its behaviour.
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Element as DomElement, HtmlElement, KeyboardEvent, KeyboardEventInit};

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::dropdown_menu::{DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem, DropdownMenuGroup, DropdownMenuLabel};

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
        DropdownMenu { open: controlled(), on_open_change: move |value| controlled.set(value),
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
    element.dispatch_event(&KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap()).unwrap();
}

fn active() -> HtmlElement {
    web_sys::window().unwrap().document().unwrap().active_element().unwrap().dyn_into().unwrap()
}

async fn focused(root: &DomElement, role: &str, name: &str) {
    wait_until(&format!("focus reaches {role} {name}"), || {
        by_role_name(root, role, name).is_some_and(|item| item == active())
    }).await;
}

async fn opened(root: &DomElement, name: &str) {
    let trigger = control(root, "button", name);
    trigger.focus().unwrap();
    key(&trigger, "Enter");
    wait_until(&format!("{name} menu opens"), || {
        by_role_name(root, "menu", name).is_some()
    }).await;
    assert_eq!(trigger.get_attribute("aria-haspopup").as_deref(), Some("menu"));
    assert_eq!(trigger.get_attribute("aria-expanded").as_deref(), Some("true"));
}

async fn closed(root: &DomElement, name: &str) {
    wait_until(&format!("{name} closes and returns focus"), || {
        let trigger = control(root, "button", name);
        trigger.get_attribute("aria-expanded").as_deref() == Some("false") && trigger == active()
            && by_role_name(root, "menu", name).is_none()
    }).await;
}

#[wasm_bindgen_test]
async fn action_menu_keyboard_roles_focus_and_action_outcomes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()));
    wait_until("representative menus render", || by_role_name(&root, "button", "Project").is_some()).await;

    // default_open must enter the first enabled item, just like an interactive open.
    focused(&root, "menuitem", "Align").await;
    key(&active(), "Escape");
    closed(&root, "Layout").await;

    opened(&root, "Project").await;
    focused(&root, "menuitem", "Open").await;
    assert_eq!(control(&root, "menuitem", "Save").get_attribute("aria-disabled").as_deref(), Some("true"));
    for (pressed, role, name) in [
        ("ArrowDown", "menuitem", "Export"),
        ("ArrowUp", "menuitem", "Open"),
        ("End", "menuitemradio", "Dark"),
        ("ArrowDown", "menuitem", "Open"),
        ("Home", "menuitem", "Open"),
        ("e", "menuitem", "Export"),
        ("x", "menuitem", "Export"),
        ("a", "menuitem", "Example"),
    ] {
        key(&active(), pressed);
        focused(&root, role, name).await;
    }
    key(&active(), "Enter");
    closed(&root, "Project").await;
    assert_eq!(root.query_selector("output").unwrap().unwrap().text_content().as_deref(), Some("Example"));

    opened(&root, "Project").await;
    focused(&root, "menuitem", "Open").await;
    control(&root, "menuitemcheckbox", "Snap").click();
    closed(&root, "Project").await;
    opened(&root, "Project").await;
    assert_eq!(control(&root, "menuitemcheckbox", "Snap").get_attribute("aria-checked").as_deref(), Some("true"));
    key(&active(), "End");
    focused(&root, "menuitemradio", "Dark").await;
    key(&active(), "Enter");
    closed(&root, "Project").await;
    opened(&root, "Project").await;
    assert_eq!(control(&root, "menuitemradio", "Dark").get_attribute("aria-checked").as_deref(), Some("true"));
    assert_eq!(control(&root, "menuitemradio", "Light").get_attribute("aria-checked").as_deref(), Some("false"));
    control(&root, "menuitem", "Save").click();
    assert_eq!(control(&root, "button", "Project").get_attribute("aria-expanded").as_deref(), Some("true"));
    key(&active(), "Escape");
    closed(&root, "Project").await;

    opened(&root, "Transform").await;
    focused(&root, "menuitem", "Rotate").await;
    key(&active(), "ArrowDown");
    focused(&root, "menuitem", "Mirror").await;
    key(&active(), "Enter");
    closed(&root, "Transform").await;
    assert_eq!(root.query_selector("output").unwrap().unwrap().text_content().as_deref(), Some("Mirror"));
    opened(&root, "Transform").await;
    focused(&root, "menuitem", "Rotate").await;
    control(&root, "button", "Close externally").click();
    closed(&root, "Transform").await;
    root.remove();
}
