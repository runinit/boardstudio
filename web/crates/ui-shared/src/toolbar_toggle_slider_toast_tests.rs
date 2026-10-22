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
    let mut slider_changes = use_signal(Vec::<f64>::new);
    let mut controlled = use_signal(|| Some(40.0));
    let mut toggle_changes = use_signal(|| 0);
    let mut pressed = use_signal(|| Some(std::collections::HashSet::<usize>::new()));
    let mut caller_mounts = use_signal(|| 0);
    let mut caller_keys = use_signal(|| 0);
    let mut caller_focus = use_signal(|| 0);
    let mut slider_keys = use_signal(|| 0);
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
        Toolbar { aria_label: "Tool modes",
            ToolbarButton { index: 0usize, "Pointer tool" }
            ToggleGroup { aria_label: "Tool scope", horizontal: true,
                ToggleItem { index: 0usize, "Keys scope" }
                ToggleItem { index: 1usize, "Points scope" }
            }
            ToolbarButton { index: 1usize, "Finish tool" }
        }
        Toolbar { aria_label: "Caller tools", id: "caller-tools", title: "Caller title", class: "caller-class", "data-caller": "kept",
            attributes: vec![
                dioxus::events::onmounted(move |_| caller_mounts += 1),
                dioxus::events::onfocusin(move |_| caller_focus += 1),
                dioxus::events::onkeydown(move |_| caller_keys += 1),
            ],
            ToolbarButton { index: 0usize, "Caller first" }
            ToolbarButton { index: 1usize, "Caller last" }
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
        Slider { label: "Caller zoom", default_value: 40.0,
            attributes: vec![dioxus::events::onkeydown(move |_| slider_keys += 1)],
        }
        button { "After sliders" }
        output { role: "status", aria_label: "Action count", "{actions}" }
        output { role: "status", aria_label: "Slider changes", "{slider_changes:?}" }
        output { role: "status", aria_label: "Toggle changes", "{toggle_changes}" }
        output { role: "status", aria_label: "Caller mounts", "{caller_mounts}" }
        output { role: "status", aria_label: "Caller keys", "{caller_keys}" }
        output { role: "status", aria_label: "Caller focus", "{caller_focus}" }
        output { role: "status", aria_label: "Slider keys", "{slider_keys}" }
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
        1,
        "{}",
        toolbar.outer_html()
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
async fn toolbar_roves_across_an_embedded_toggle_group() {
    let root = mount();
    ready(&root).await;
    let toolbar = control(&root, "toolbar", "Tool modes");
    assert_eq!(
        toolbar
            .query_selector_all("[tabindex='0']")
            .unwrap()
            .length(),
        1,
        "{}; caller mounts {}",
        toolbar.outer_html(),
        control(&root, "status", "Caller mounts")
            .text_content()
            .unwrap()
    );
    let pointer = control(&root, "button", "Pointer tool");
    pointer.focus().unwrap();
    key(&pointer, "ArrowRight");
    wait_until("toolbar enters toggle group", || {
        control(&root, "button", "Keys scope")
            .matches(":focus")
            .unwrap()
    })
    .await;
    key(&control(&root, "button", "Keys scope"), "ArrowRight");
    wait_until("toolbar crosses toggle items", || {
        control(&root, "button", "Points scope")
            .matches(":focus")
            .unwrap()
    })
    .await;
    key(&control(&root, "button", "Points scope"), "End");
    wait_until("toolbar End leaves toggle group", || {
        control(&root, "button", "Finish tool")
            .matches(":focus")
            .unwrap()
    })
    .await;
    assert_eq!(
        toolbar
            .query_selector_all("[tabindex='0']")
            .unwrap()
            .length(),
        1
    );
    root.remove();
}

fn changing_toolbar() -> Element {
    let mut show_first = use_signal(|| true);
    let mut show_toolbar = use_signal(|| true);
    let mut first_disabled = use_signal(|| false);
    rsx! {
        button { onclick: move |_| show_first.set(false), "Remove first action" }
        button { onclick: move |_| show_toolbar.set(false), "Unmount tools" }
        button { onclick: move |_| first_disabled.set(true), "Disable current action" }
        if show_toolbar() {
            Toolbar { aria_label: "Changing tools",
                if show_first() { ToolbarButton { index: 0usize, disabled: first_disabled, "First changing action" } }
                ToolbarButton { index: 1usize, "Remaining action" }
            }
        }
    }
}

#[wasm_bindgen_test]
async fn toolbar_preserves_caller_attributes_and_callbacks() {
    let root = mount();
    ready(&root).await;
    let toolbar = control(&root, "toolbar", "Caller tools");
    assert_eq!(toolbar.id(), "caller-tools");
    assert_eq!(
        toolbar.get_attribute("title").as_deref(),
        Some("Caller title")
    );
    assert_eq!(
        toolbar.get_attribute("data-caller").as_deref(),
        Some("kept")
    );
    assert!(toolbar.class_name().contains("caller-class"));
    assert!(toolbar.class_name().contains("bg-background"));
    assert_eq!(
        control(&root, "status", "Caller mounts")
            .text_content()
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        toolbar
            .query_selector_all("[tabindex='0']")
            .unwrap()
            .length(),
        1
    );
    let first = control(&root, "button", "Caller first");
    first.focus().unwrap();
    key(&first, "End");
    wait_until("caller toolbar End", || {
        control(&root, "button", "Caller last")
            .matches(":focus")
            .unwrap()
    })
    .await;
    wait_until("caller key notification committed", || {
        control(&root, "status", "Caller keys")
            .text_content()
            .as_deref()
            == Some("1")
    })
    .await;
    assert_eq!(
        control(&root, "status", "Caller keys")
            .text_content()
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        control(&root, "status", "Caller focus")
            .text_content()
            .as_deref(),
        Some("2")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn slider_preserves_caller_key_handler() {
    let root = mount();
    ready(&root).await;
    let slider = control(&root, "slider", "Caller zoom");
    key(&slider, "Home");
    wait_until("caller slider Home", || {
        slider.get_attribute("aria-valuenow").as_deref() == Some("0")
    })
    .await;
    assert_eq!(
        control(&root, "status", "Slider keys")
            .text_content()
            .as_deref(),
        Some("1")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn toolbar_reassigns_the_entry_when_the_current_item_is_disabled() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(changing_toolbar),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("changing toolbar mounted", || {
        by_role_name(&root, "toolbar", "Changing tools").is_some()
    })
    .await;
    let first = control(&root, "button", "First changing action");
    first.focus().unwrap();
    control(&root, "button", "Disable current action").click();
    wait_until("current item disabled", || first.has_attribute("disabled")).await;
    assert_eq!(first.tab_index(), -1);
    assert_eq!(control(&root, "button", "Remaining action").tab_index(), 0);
    root.remove();
}

#[wasm_bindgen_test]
async fn toolbar_releases_its_observer_when_unmounted() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(changing_toolbar),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("changing toolbar mounted", || {
        by_role_name(&root, "toolbar", "Changing tools").is_some()
    })
    .await;
    let toolbar = control(&root, "toolbar", "Changing tools");
    let first = control(&root, "button", "First changing action");
    let remaining = control(&root, "button", "Remaining action");
    control(&root, "button", "Unmount tools").click();
    wait_until("toolbar unmounted", || {
        by_role_name(&root, "toolbar", "Changing tools").is_none()
    })
    .await;
    toolbar.append_child(&first).unwrap();
    toolbar.append_child(&remaining).unwrap();
    first.set_tab_index(0);
    remaining.set_tab_index(0);
    // Cross the mutation-observer microtask checkpoint; a leaked observer would repair these detached nodes.
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::UNDEFINED))
        .await
        .unwrap();
    assert_eq!(first.tab_index(), 0);
    assert_eq!(remaining.tab_index(), 0);
    root.remove();
}

#[wasm_bindgen_test]
async fn toolbar_keeps_a_tab_stop_when_the_current_item_is_removed() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(changing_toolbar),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("changing toolbar mounted", || {
        by_role_name(&root, "button", "First changing action").is_some()
    })
    .await;
    control(&root, "button", "First changing action")
        .focus()
        .unwrap();
    control(&root, "button", "Remove first action").click();
    wait_until("current action removed", || {
        by_role_name(&root, "button", "First changing action").is_none()
    })
    .await;
    assert_eq!(control(&root, "button", "Remaining action").tab_index(), 0);
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
    assert_eq!(root.query_selector_all("[role=alert]").unwrap().length(), 0);
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
