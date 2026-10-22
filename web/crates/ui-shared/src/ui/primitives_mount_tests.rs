//! Mounts a few primitives-backed copies to check the wiring: the primitive supplies the role and state, the copy
//! adds its rust-ui classes and `data-slot`. The accessibility floor rows are tickets 006 to 008.

use dioxus::prelude::*;
use wasm_bindgen_test::*;

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::checkbox::Checkbox;
use crate::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use crate::ui::toolbar::{Toolbar, ToolbarButton};

fn fixture() -> Element {
    rsx! {
        Checkbox { aria_label: "Snap to grid" }
        Toolbar { aria_label: "History",
            ToolbarButton { index: 0usize, "Undo" }
        }
        Tabs { default_value: "objects", horizontal: true,
            TabsList {
                TabsTrigger { value: "objects", index: 0usize, "Objects" }
                TabsTrigger { value: "inspector", index: 1usize, "Inspector" }
            }
            TabsContent { value: "objects", index: 0usize, "Objects panel" }
            TabsContent { value: "inspector", index: 1usize, "Inspector panel" }
        }
    }
}

#[wasm_bindgen_test]
async fn primitives_backed_copies_carry_role_slot_and_classes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the copies render", || {
        by_role_name(&root, "checkbox", "Snap to grid").is_some()
    })
    .await;

    let checkbox = by_role_name(&root, "checkbox", "Snap to grid").unwrap();
    assert_eq!(
        checkbox.get_attribute("data-slot").as_deref(),
        Some("checkbox")
    );
    assert!(
        checkbox.class_name().contains("bg-field"),
        "rust-ui classes on the primitive: {}",
        checkbox.class_name()
    );
    assert_eq!(
        checkbox.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    checkbox.click();
    wait_until("the primitive toggles the checkbox", || {
        by_role_name(&root, "checkbox", "Snap to grid")
            .and_then(|c| c.get_attribute("aria-checked"))
            .as_deref()
            == Some("true")
    })
    .await;

    let toolbar = by_role_name(&root, "toolbar", "History").unwrap();
    assert_eq!(
        toolbar.get_attribute("data-slot").as_deref(),
        Some("toolbar")
    );
    let undo = by_role_name(&root, "button", "Undo").unwrap();
    assert_eq!(
        undo.get_attribute("data-slot").as_deref(),
        Some("toolbar-button")
    );

    let inspector = by_role_name(&root, "tab", "Inspector").unwrap();
    assert_eq!(
        inspector.get_attribute("data-slot").as_deref(),
        Some("tabs-trigger")
    );
    inspector.click();
    wait_until("the primitive selects the Inspector tab", || {
        by_role_name(&root, "tab", "Inspector")
            .and_then(|t| t.get_attribute("aria-selected"))
            .as_deref()
            == Some("true")
    })
    .await;
    root.remove();
}

use crate::ui::combobox::{Combobox, ComboboxOption};
use crate::ui::field::{Field, FieldLabel};
use crate::ui::select::{Select, SelectOption};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlInputElement, KeyboardEvent, KeyboardEventInit};

fn picker_fixture() -> Element {
    let mut selected = use_signal(|| None::<String>);
    let mut combo_selected = use_signal(|| None::<String>);
    let mut open = use_signal(|| Some(false));
    rsx! {
        Field {
            FieldLabel { html_for: "fruit-trigger", span { id: "fruit-label", "Fruit" } }
            Select::<String> { id: "fruit-trigger", aria_labelledby: "fruit-label",
                value: Some(selected.into()), on_value_change: move |value| selected.set(value),
                open, on_open_change: move |value| open.set(Some(value)),
                SelectOption::<String> { index: 0usize, value: "a", text_value: "Apple", "Apple" }
                SelectOption::<String> { index: 1usize, value: "x", text_value: "Apricot", disabled: true, "Apricot" }
                SelectOption::<String> { index: 2usize, value: "b", text_value: "Banana", "Banana" }
                SelectOption::<String> { index: 3usize, value: "c", text_value: "Cherry", "Cherry" }
            }
        }
        Combobox::<String> { aria_label: "Search fruit", list_aria_label: "Fruit matches",
            value: Some(combo_selected.into()), on_value_change: move |value| combo_selected.set(value),
            ComboboxOption::<String> { index: 0usize, value: "a", text_value: "Apple", "Apple" }
            ComboboxOption::<String> { index: 1usize, value: "x", text_value: "Apricot", disabled: true, "Apricot" }
            ComboboxOption::<String> { index: 2usize, value: "b", text_value: "Banana", "Banana" }
            ComboboxOption::<String> { index: 3usize, value: "c", text_value: "Cherry", "Cherry" }
        }
        Select::<String> { disabled: true,
            SelectOption::<String> { index: 0usize, value: "disabled", "Disabled" }
        }
        Combobox::<String> { disabled: true, aria_label: "Unavailable fruit" }
        button { onclick: move |_| open.set(Some(false)), "Close fruit" }
        output { role: "status", aria_label: "Selected fruit", "{selected():?}" }
        output { role: "status", aria_label: "Combobox selection", "{combo_selected():?}" }
    }
}

async fn mount_picker() -> web_sys::Element {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(picker_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("pickers render", || {
        by_role_name(&root, "combobox", "Search fruit").is_some()
    })
    .await;
    root
}

fn key(element: &HtmlElement, value: &str) {
    let init = KeyboardEventInit::new();
    init.set_key(value);
    init.set_bubbles(true);
    init.set_cancelable(true);
    element
        .dispatch_event(
            &KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

fn focused(element: &HtmlElement) -> bool {
    element.owner_document().unwrap().active_element().as_ref() == Some(element.as_ref())
}

async fn focus_option(root: &web_sys::Element, name: &str) -> HtmlElement {
    wait_until("the named option receives focus", || {
        by_role_name(root, "option", name).is_some_and(|e| focused(&e))
    })
    .await;
    by_role_name(root, "option", name).unwrap()
}

#[wasm_bindgen_test]
async fn select_floor_label_navigation_selection_and_focus_return() {
    let root = mount_picker().await;
    let trigger =
        by_role_name(&root, "button", "Fruit").expect("Select trigger is named by the Field label");
    assert_eq!(trigger.id(), "fruit-trigger");
    assert_eq!(
        trigger.get_attribute("aria-haspopup").as_deref(),
        Some("listbox")
    );
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    trigger.focus().unwrap();
    key(&trigger, "ArrowDown");
    let apple = focus_option(&root, "Apple").await;
    let list = by_role_name(&root, "listbox", "Fruit").expect("listbox takes the Field label");
    assert_eq!(
        trigger.get_attribute("aria-controls").as_deref(),
        Some(list.id().as_str())
    );
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
    assert_eq!(
        apple.get_attribute("aria-selected").as_deref(),
        Some("false")
    );
    let disabled = by_role_name(&root, "option", "Apricot").unwrap();
    assert_eq!(
        disabled.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    key(&apple, "ArrowDown");
    let banana = focus_option(&root, "Banana").await;
    key(&banana, "ArrowUp");
    let apple = focus_option(&root, "Apple").await;
    key(&apple, "End");
    let cherry = focus_option(&root, "Cherry").await;
    key(&cherry, "Home");
    let apple = focus_option(&root, "Apple").await;
    key(&apple, "b");
    let banana = focus_option(&root, "Banana").await;
    key(&banana, "Enter");
    wait_until("selection closes and restores the trigger", || {
        trigger.get_attribute("aria-expanded").as_deref() == Some("false") && focused(&trigger)
    })
    .await;
    assert_eq!(
        trigger.text_content().unwrap().trim(),
        "Banana",
        "display the label, not b or icon text"
    );
    assert_eq!(
        by_role_name(&root, "status", "Selected fruit")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Some(\"b\")")
    );
    for opening_key in ["Enter", " ", "ArrowDown"] {
        key(&trigger, opening_key);
        wait_until("keyboard opens Select", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let selected = by_role_name(&root, "option", "Banana").unwrap();
        assert_eq!(
            selected.get_attribute("aria-selected").as_deref(),
            Some("true")
        );
        let active = root
            .owner_document()
            .unwrap()
            .active_element()
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        key(&active, "Escape");
        wait_until("Escape closes and restores the trigger", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("false") && focused(&trigger)
        })
        .await;
    }
    key(&trigger, "ArrowDown");
    focus_option(&root, "Apple").await;
    by_role_name(&root, "button", "Close fruit")
        .unwrap()
        .click();
    wait_until("a controlled close restores trigger focus", || {
        trigger.get_attribute("aria-expanded").as_deref() == Some("false") && focused(&trigger)
    })
    .await;
    let disabled = by_role_name(&root, "button", "Select an option").unwrap();
    assert!(disabled.has_attribute("disabled"));
    disabled.click();
    assert_eq!(
        disabled.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn combobox_floor_filter_active_descendant_selection_and_escape() {
    let root = mount_picker().await;
    let input = by_role_name(&root, "combobox", "Search fruit").unwrap();
    assert_eq!(
        input.get_attribute("aria-haspopup").as_deref(),
        Some("listbox")
    );
    assert_eq!(
        input.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    input.focus().unwrap();
    key(&input, "ArrowDown");
    wait_until("combobox opens and highlights Apple", || {
        let option = by_role_name(&root, "option", "Apple");
        option.is_some_and(|o| {
            input.get_attribute("aria-activedescendant").as_deref() == Some(o.id().as_str())
        })
    })
    .await;
    let list = by_role_name(&root, "listbox", "Fruit matches").unwrap();
    assert_eq!(
        input.get_attribute("aria-controls").as_deref(),
        Some(list.id().as_str())
    );
    assert!(focused(&input));
    key(&input, "ArrowDown");
    wait_until("combobox skips disabled Apricot", || {
        by_role_name(&root, "option", "Banana").is_some_and(|o| {
            input.get_attribute("aria-activedescendant").as_deref() == Some(o.id().as_str())
        })
    })
    .await;
    assert!(focused(&input));
    let html_input = input.clone().dyn_into::<HtmlInputElement>().unwrap();
    html_input.set_value("ch");
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
    wait_until("typing filters the list", || {
        by_role_name(&root, "option", "Cherry").is_some()
            && by_role_name(&root, "option", "Apple").is_none()
            && by_role_name(&root, "option", "Banana").is_none()
    })
    .await;
    key(&input, "ArrowDown");
    wait_until("filtered Cherry is the active descendant", || {
        by_role_name(&root, "option", "Cherry").is_some_and(|o| {
            input.get_attribute("aria-activedescendant").as_deref() == Some(o.id().as_str())
        })
    })
    .await;
    assert!(focused(&input));
    key(&input, "Enter");
    wait_until("Enter selects the label and closes", || {
        html_input.value() == "Cherry"
            && input.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
    assert!(focused(&input));
    assert!(input.get_attribute("aria-activedescendant").is_none());
    key(&input, "ArrowDown");
    wait_until("combobox reopens", || {
        input.get_attribute("aria-expanded").as_deref() == Some("true")
    })
    .await;
    key(&input, "Escape");
    wait_until("Escape closes combobox", || {
        input.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
    assert!(focused(&input));
    assert_eq!(html_input.value(), "Cherry");
    assert_eq!(
        by_role_name(&root, "status", "Combobox selection")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("Some(\"c\")")
    );
    let disabled = by_role_name(&root, "combobox", "Unavailable fruit").unwrap();
    assert!(disabled.has_attribute("disabled"));
    disabled.click();
    assert_eq!(
        disabled.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    root.remove();
}
