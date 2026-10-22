//! Mounted ticket 008 regression tests. The exported fixture also supports trusted agent-browser keys.
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::{
    slider::Slider,
    toast::{ToastOptions, ToastProvider, use_toast},
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::{Toolbar, ToolbarButton, ToolbarSeparator},
};

fn fixture() -> Element {
    let mut actions = use_signal(|| 0);
    let mut slider_changes = use_signal(|| Vec::<f64>::new());
    let mut controlled = use_signal(|| Some(40.0));
    let mut toggle_changes = use_signal(|| 0);
    let mut pressed = use_signal(|| Some(std::collections::HashSet::<usize>::new()));
    rsx! {
        button { "Before toolbar" }
        Toolbar { aria_label: "History",
            ToolbarButton { index: 0usize, on_click: move |_| actions += 1, "Undo" }
            ToolbarButton { index: 1usize, disabled: true, "Unavailable" }
            ToolbarSeparator {}
            ToolbarButton { index: 2usize, on_click: move |_| actions += 1, "Redo" }
            ToolbarButton { index: 3usize, "Last action" }
        }
        button { "After toolbar" }
        Toolbar { aria_label: "Vertical tools", horizontal: false,
            ToolbarButton { index: 0usize, "Top tool" }
            ToolbarButton { index: 1usize, "Bottom tool" }
        }
        Toolbar { aria_label: "Disabled tools", disabled: true,
            ToolbarButton { index: 0usize, on_click: move |_| actions += 100, "Disabled action" }
        }
        ToggleGroup { aria_label: "Alignment", horizontal: true,
            on_pressed_change: move |_| toggle_changes += 1,
            ToggleItem { index: 0usize, "Left" }
            ToggleItem { index: 1usize, disabled: true, "Disabled alignment" }
            ToggleItem { index: 2usize, "Right" }
        }
        ToggleGroup { aria_label: "Formatting", horizontal: false, allow_multiple_pressed: true,
            pressed,
            on_pressed_change: move |next| { toggle_changes += 1; pressed.set(Some(next)); },
            ToggleItem { index: 0usize, "Bold" }
            ToggleItem { index: 1usize, "Italic" }
        }
        Slider { label: "Zoom", min: 10.0, max: 90.0, step: 2.0, default_value: 40.0,
            on_value_change: move |next| slider_changes.write().push(next),
        }
        Slider { label: "Controlled zoom", min: 10.0, max: 90.0, step: 2.0, value: controlled,
            on_value_change: move |next| { slider_changes.write().push(next); controlled.set(Some(next)); },
        }
        Slider { label: "Disabled zoom", disabled: true, default_value: 40.0,
            on_value_change: move |next| slider_changes.write().push(next),
        }
        output { role: "status", aria_label: "Action count", "{actions}" }
        output { role: "status", aria_label: "Slider changes", "{slider_changes:?}" }
        output { role: "status", aria_label: "Toggle changes", "{toggle_changes}" }
        ToastProvider { ToastTrigger {} }
    }
}

#[component]
fn ToastTrigger() -> Element {
    let toasts = use_toast();
    let mut actions = use_signal(|| 0);
    rsx! {
        button { onclick: move |_| {
            actions += 1;
            toasts.success("Saved".into(), ToastOptions::new().description("Project stored").permanent(true));
        }, "Notify" }
        button { "After notifications" }
        output { role: "status", aria_label: "Notification actions", "{actions}" }
    }
}

#[wasm_bindgen]
pub fn mount_ticket_008_fixture() {
    mount();
}

#[wasm_bindgen]
pub fn ticket_008_control(role: &str, name: &str) -> Option<web_sys::HtmlElement> {
    let root = web_sys::window()?
        .document()?
        .get_element_by_id("ticket-008-fixture")?;
    by_role_name(&root, role, name)
}

fn mount() -> web_sys::Element {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("ticket-008-fixture");
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    root
}

fn control(root: &web_sys::Element, role: &str, name: &str) -> web_sys::HtmlElement {
    by_role_name(root, role, name).unwrap_or_else(|| panic!("missing {role} {name}"))
}

fn key(target: &web_sys::HtmlElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    target
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

async fn ready(root: &web_sys::Element) {
    wait_until("008 fixture mounted", || {
        by_role_name(root, "slider", "Zoom").is_some()
    })
    .await;
}

#[wasm_bindgen_test]
async fn toolbar_has_one_tab_stop() {
    let root = mount();
    ready(&root).await;
    let toolbar = control(&root, "toolbar", "History");
    assert_eq!(
        toolbar
            .query_selector_all("[tabindex='0']")
            .unwrap()
            .length(),
        1
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn toolbar_end_reaches_last_item() {
    let root = mount();
    ready(&root).await;
    let undo = control(&root, "button", "Undo");
    undo.focus().unwrap();
    key(&undo, "End");
    wait_until("End focuses last toolbar item", || {
        control(&root, "button", "Last action")
            .matches(":focus")
            .unwrap()
    })
    .await;
    root.remove();
}

#[wasm_bindgen_test]
async fn toggle_group_has_named_group_role() {
    let root = mount();
    ready(&root).await;
    control(&root, "group", "Alignment");
    assert_eq!(
        control(&root, "button", "Left")
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("false")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn slider_home_end_and_page_keys_notify_once() {
    let root = mount();
    ready(&root).await;
    let slider = control(&root, "slider", "Zoom");
    for (key_name, expected) in [
        ("Home", "10"),
        ("End", "90"),
        ("PageDown", "70"),
        ("PageUp", "90"),
    ] {
        key(&slider, key_name);
        wait_until(key_name, || {
            slider.get_attribute("aria-valuenow").as_deref() == Some(expected)
        })
        .await;
    }
    assert_eq!(
        control(&root, "status", "Slider changes")
            .text_content()
            .as_deref(),
        Some("[10.0, 90.0, 70.0, 90.0]")
    );
    root.remove();
}

async fn slider_key_reaches(key_name: &str, expected: &str) {
    let root = mount();
    ready(&root).await;
    let slider = control(&root, "slider", "Zoom");
    key(&slider, key_name);
    wait_until(key_name, || {
        slider.get_attribute("aria-valuenow").as_deref() == Some(expected)
    })
    .await;
    root.remove();
}

#[wasm_bindgen_test]
async fn slider_home_reaches_minimum() {
    slider_key_reaches("Home", "10").await;
}
#[wasm_bindgen_test]
async fn slider_end_reaches_maximum() {
    slider_key_reaches("End", "90").await;
}
#[wasm_bindgen_test]
async fn slider_page_up_changes_value() {
    slider_key_reaches("PageUp", "60").await;
}
#[wasm_bindgen_test]
async fn slider_page_down_changes_value() {
    slider_key_reaches("PageDown", "20").await;
}

#[wasm_bindgen_test]
async fn disabled_slider_is_not_a_tab_stop() {
    let root = mount();
    ready(&root).await;
    let slider = control(&root, "slider", "Disabled zoom");
    assert!(slider.has_attribute("disabled"));
    assert_eq!(
        slider.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn toast_is_an_announcement_with_reachable_dismiss() {
    let root = mount();
    ready(&root).await;
    control(&root, "button", "Notify").click();
    wait_until("toast mounted", || {
        by_role_name(&root, "button", "close").is_some()
    })
    .await;
    assert_eq!(
        root.query_selector_all("[role=alertdialog]")
            .unwrap()
            .length(),
        0
    );
    let toast = control(&root, "status", "Saved");
    assert_eq!(toast.get_attribute("aria-atomic").as_deref(), Some("true"));
    let close = control(&root, "button", "close");
    assert_eq!(close.tab_index(), 0);
    close.click();
    wait_until("dismiss removes toast", || {
        by_role_name(&root, "status", "Saved").is_none()
    })
    .await;
    root.remove();
}
