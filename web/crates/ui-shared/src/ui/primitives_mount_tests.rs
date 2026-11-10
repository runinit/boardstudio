//! Mounts a few primitives-backed copies to check the wiring: the primitive supplies the role and state, the copy
//! adds its rust-ui classes and `data-slot`. The accessibility floor rows are tickets 006 to 008.

use dioxus::prelude::*;
use wasm_bindgen_test::*;

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::button::Button;

#[wasm_bindgen_test]
fn role_names_skip_hidden_chevrons_without_rewriting_inline_text_or_labels() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_inner_html(r#"<button><span>4</span> <strong>Case <em>(optional)</em></strong><span aria-hidden="true">⌄</span></button>
        <button>Save<strong>As</strong></button><button>Plain name</button>
        <button aria-label="Explicit"><span aria-hidden="true">Ignored</span>Content</button>
        <span id="role-test-label">Referenced name</span><button aria-labelledby="role-test-label">Content</button>"#);
    document.body().unwrap().append_child(&root).unwrap();
    for name in [
        "4 Case (optional)",
        "SaveAs",
        "Plain name",
        "Explicit",
        "Referenced name",
    ] {
        assert!(
            by_role_name(&root, "button", name).is_some(),
            "missing accessible name {name}"
        );
    }
    root.remove();
}
use crate::ui::checkbox::Checkbox;
use crate::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use crate::ui::toolbar::{Toolbar, ToolbarButton};

#[wasm_bindgen_test]
async fn slider_round_thumb_tracks_value_and_remains_hit_testable_in_touch_rows() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    // Extend ticket 008's actual controlled/uncontrolled story, not another Slider fixture.
    crate::toolbar_toggle_slider_toast_tests::mount_ticket_008_fixture();
    let root = document.get_element_by_id("ticket-008-fixture").unwrap();
    // Match the setup guide's descendant-button touch target utility.
    root.set_attribute("class", "[&_button]:min-h-control-touch")
        .unwrap();
    root.set_attribute("style", "width:320px;margin:24px")
        .unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(include_str!("../../../../assets/tailwind.css")));
    root.append_child(&stylesheet).unwrap();
    wait_until("existing sliders render", || {
        by_role_name(&root, "slider", "Controlled zoom").is_some()
    })
    .await;
    for name in ["Zoom", "Controlled zoom"] {
        let thumb = by_role_name(&root, "slider", name).unwrap();
        thumb.scroll_into_view();
        let track = thumb.parent_element().unwrap();
        let activation = track.parent_element().unwrap();
        for (action, expected, fraction) in [
            (None, "40", 0.375),
            (Some("End"), "90", 1.0),
            (Some("Home"), "10", 0.0),
            (Some("PageUp"), "30", 0.25),
        ] {
            if let Some(action) = action {
                thumb.focus().unwrap();
                key(&thumb, action);
                wait_until("value keys update the existing slider", || {
                    thumb.get_attribute("aria-valuenow").as_deref() == Some(expected)
                })
                .await;
                assert!(focused(&thumb));
            }
            let rect = thumb.get_bounding_client_rect();
            let rail = track.get_bounding_client_rect();
            let style = window.get_computed_style(&thumb).unwrap().unwrap();
            let x = rect.x() + rect.width() / 2.0;
            let y = rect.y() + rect.height() / 2.0;
            let hit = document.element_from_point(x as f32, y as f32);
            let radius = style.get_property_value("border-top-left-radius").unwrap();
            let radius = radius.trim_end_matches("px").parse::<f64>().unwrap();
            assert!(
                rect.width() == 16.0
                    && rect.height() == 16.0
                    && radius >= 8.0
                    && (x - (rail.x() + rail.width() * fraction)).abs() < 1.0
                    && (y - (rail.y() + rail.height() / 2.0)).abs() < 1.0
                    && hit.as_ref() == Some(thumb.as_ref())
                    && activation.get_bounding_client_rect().height() >= 44.0,
                "{name} at {expected}: round 16px thumb must be centred at {fraction}, painted/hit-testable in a 44px activation row; size={}x{}, centre=({x},{y}), track=({},{},{}x{}), radius={radius}, hit={:?}, row_height={}",
                rect.width(),
                rect.height(),
                rail.x(),
                rail.y(),
                rail.width(),
                rail.height(),
                hit.map(|element| element.tag_name()),
                activation.get_bounding_client_rect().height(),
            );
        }
    }
    let disabled = by_role_name(&root, "slider", "Disabled zoom").unwrap();
    assert!(disabled.has_attribute("disabled"));
    assert_eq!(
        disabled.get_attribute("aria-disabled").as_deref(),
        Some("true")
    );
    root.remove();
}

fn icon_geometry_fixture() -> Element {
    use crate::canvas_layers::LayerToggleButton;
    use crate::ui::toggle_group::{ToggleGroup, ToggleItem};
    rsx! {
        style { {include_str!("../../../../assets/tailwind.css")} }
        for (name, href) in [("Button geometry", None), ("Link geometry", Some("#geometry".to_owned()))] {
            Button { aria_label: name, href,
                svg { view_box: "0 0 20 20", "aria-hidden": "true" }
                svg { class: "size-6", view_box: "0 0 20 20", "aria-hidden": "true" }
                span { class: "[&_svg]:w-full [&_svg]:h-full", style: "display:block;width:152px;height:116px",
                    svg { view_box: "0 0 20 20", "aria-hidden": "true" }
                }
            }
        }
        ToggleGroup { aria_label: "Geometry modes",
            ToggleItem { index: 0usize, aria_label: "Toggle geometry",
                svg { view_box: "0 0 20 20", "aria-hidden": "true" }
                svg { class: "size-6", view_box: "0 0 20 20", "aria-hidden": "true" }
                span { class: "[&_svg]:w-full [&_svg]:h-full", style: "display:block;width:152px;height:116px",
                    svg { view_box: "0 0 20 20", "aria-hidden": "true" }
                }
            }
        }
        for visible in [true, false] {
            LayerToggleButton { label: "Keys", swatch: "Keys",
                aria_label: if visible { "Hide fixture keys" } else { "Show fixture keys" },
                visible, on_toggle: move |_| {},
            }
        }
    }
}

#[wasm_bindgen_test]
async fn action_icons_keep_default_and_explicit_sizes_without_resizing_nested_geometry() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(icon_geometry_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("geometry controls render", || {
        by_role_name(&root, "button", "Toggle geometry").is_some()
    })
    .await;
    for (role, name) in [
        ("button", "Button geometry"),
        ("link", "Link geometry"),
        ("button", "Toggle geometry"),
    ] {
        let control = by_role_name(&root, role, name).unwrap();
        let svgs = control.query_selector_all("svg").unwrap();
        let dimensions = (0..svgs.length())
            .map(|index| {
                let svg = svgs
                    .item(index)
                    .unwrap()
                    .dyn_into::<web_sys::Element>()
                    .unwrap();
                let rect = svg.get_bounding_client_rect();
                (rect.width(), rect.height())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            dimensions,
            [(16.0, 16.0), (24.0, 24.0), (152.0, 116.0)],
            "{name}"
        );
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn visibility_eyes_use_unfilled_current_color_strokes_in_both_states() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(icon_geometry_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("visibility controls render", || {
        by_role_name(&root, "button", "Show fixture keys").is_some()
    })
    .await;
    for (theme, stroke_width) in [("original", 1.5), ("shadcn", 2.0)] {
        root.set_attribute("data-theme", theme).unwrap();
        for name in ["Hide fixture keys", "Show fixture keys"] {
            let control = by_role_name(&root, "button", name).unwrap();
            let svg = control.query_selector("svg").unwrap().unwrap();
            let style = window.get_computed_style(&svg).unwrap().unwrap();
            assert_eq!(
                style.get_property_value("fill").unwrap(),
                "none",
                "{theme}: {name}"
            );
            assert_eq!(
                style.get_property_value("stroke").unwrap(),
                style.get_property_value("color").unwrap(),
                "{theme}: {name}"
            );
            assert_eq!(
                style
                    .get_property_value("stroke-width")
                    .unwrap()
                    .trim_end_matches("px")
                    .parse::<f64>()
                    .unwrap(),
                stroke_width,
                "{theme}: {name}"
            );
            assert_eq!(
                svg.get_bounding_client_rect().width(),
                16.0,
                "{theme}: {name}"
            );
        }
    }
    root.remove();
}

fn fixture() -> Element {
    rsx! {
        Button { "Plain button" }
        Button { href: "#plain", "Plain link" }
        Button { aria_expanded: true, aria_controls: "popup", aria_haspopup: "dialog", "Popup button" }
        Button { href: "#popup", aria_expanded: false, aria_controls: "popup", aria_haspopup: "dialog", "Popup link" }
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

    for (role, name, expanded) in [
        ("button", "Popup button", "true"),
        ("link", "Popup link", "false"),
    ] {
        let element = by_role_name(&root, role, name).unwrap();
        assert_eq!(
            element.get_attribute("aria-expanded").as_deref(),
            Some(expanded)
        );
        assert_eq!(
            element.get_attribute("aria-controls").as_deref(),
            Some("popup")
        );
        assert_eq!(
            element.get_attribute("aria-haspopup").as_deref(),
            Some("dialog")
        );
    }
    for (role, name) in [("button", "Plain button"), ("link", "Plain link")] {
        let element = by_role_name(&root, role, name).unwrap();
        for attribute in ["aria-expanded", "aria-controls", "aria-haspopup"] {
            assert_eq!(element.get_attribute(attribute), None);
        }
    }

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

fn select_event_fixture() -> Element {
    let mut events = use_signal(Vec::<String>::new);
    rsx! {
        div { onkeydown: move |event: dioxus::events::KeyboardEvent| events.write().push(format!("parent:{}", event.key())),
            Select::<String> { aria_label: "Composed choice",
                on_open_change: move |open| events.write().push(format!("open:{open}")),
                attributes: vec![dioxus::events::onclick(move |_| events.write().push("caller:click".to_owned()))],
                SelectOption::<String> { index: 0usize, value: "a", "Apple" }
            }
            output { role: "status", aria_label: "Select event trace", "{events():?}" }
        }
    }
}

#[wasm_bindgen_test]
async fn select_opening_space_isolates_raw_event_without_changing_enter_or_caller_events() {
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::closure::Closure;

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(select_event_fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("event fixture mounts", || {
        by_role_name(&root, "button", "Composed choice").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Composed choice").unwrap();
    let raw_events = Rc::new(RefCell::new(Vec::new()));
    let observed = raw_events.clone();
    let listener = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        observed
            .borrow_mut()
            .push((event.key(), event.default_prevented()));
    }) as Box<dyn FnMut(_)>);
    window
        .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .unwrap();
    trigger.focus().unwrap();
    key(&trigger, "q");
    for opening_key in ["Enter", " "] {
        key(&trigger, opening_key);
        wait_until("one key opens the Select", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        let list = by_role_name(&root, "listbox", "Composed choice").unwrap();
        key(&list, "Escape");
        wait_until("Escape restores the original trigger", || {
            focused(&trigger) && trigger.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
    }
    trigger.click();
    wait_until("caller click and internal open are both retained", || {
        by_role_name(&root, "status", "Select event trace")
            .unwrap()
            .text_content()
            .is_some_and(|trace| trace.contains("caller:click"))
    })
    .await;
    let trace = by_role_name(&root, "status", "Select event trace")
        .unwrap()
        .text_content()
        .unwrap();
    assert_eq!(trace.matches("parent:q").count(), 1);
    assert!(!trace.contains("parent:Enter") && !trace.contains("parent: "));
    assert_eq!(trace.matches("open:true").count(), 3);
    assert_eq!(trace.matches("open:false").count(), 2);
    assert_eq!(trace.matches("caller:click").count(), 1);
    let raw = raw_events.borrow();
    assert_eq!(
        raw.iter()
            .filter(|(key, _)| key == "Enter")
            .collect::<Vec<_>>(),
        vec![&("Enter".to_owned(), true)]
    );
    assert!(raw.contains(&("q".to_owned(), false)));
    assert!(!raw.iter().any(|(key, _)| key == " "));
    window
        .remove_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .unwrap();
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

#[component]
fn CloseTracePicker(controlled: bool) -> Element {
    let mut open = use_signal(|| Some(false));
    let mut requests = use_signal(Vec::<bool>::new);
    rsx! {
        Select::<String> {
            open: if controlled { open() } else { None },
            on_open_change: move |value| {
                requests.write().push(value);
                open.set(Some(value));
            },
            SelectOption::<String> { index: 0usize, value: "a", text_value: "Apple", "Apple" }
        }
        button { onclick: move |_| requests.clear(), "Clear requests" }
        button { onclick: move |_| open.set(Some(false)), "External close" }
        output { role: "status", aria_label: "Open requests", "{requests():?}" }
    }
}

fn animation_method(animation: &wasm_bindgen::JsValue, method: &str) {
    js_sys::Reflect::get(animation, &method.into())
        .unwrap()
        .dyn_into::<js_sys::Function>()
        .unwrap()
        .call0(animation)
        .unwrap();
}

fn hold_popup_animation(list: &web_sys::Element) -> wasm_bindgen::JsValue {
    let animate = js_sys::Reflect::get(list, &"animate".into())
        .unwrap()
        .dyn_into::<js_sys::Function>()
        .unwrap();
    let frames = js_sys::JSON::parse(r#"[{"opacity":1},{"opacity":0}]"#).unwrap();
    let animation = animate.call2(list, &frames, &1000.into()).unwrap();
    // Pause before yielding: retention ends only when the test explicitly finishes it.
    animation_method(&animation, "pause");
    animation
}

#[wasm_bindgen_test]
async fn select_close_requests_are_not_duplicated_while_popup_is_retained() {
    let document = web_sys::window().unwrap().document().unwrap();
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for controlled in [false, true] {
        for action in ["Escape", "Enter", "external"] {
            if !controlled && action == "external" {
                continue;
            }
            let root = document.create_element("div").unwrap();
            document.body().unwrap().append_child(&root).unwrap();
            dioxus_web::launch::launch_virtual_dom(
                VirtualDom::new_with_props(CloseTracePicker, CloseTracePickerProps { controlled }),
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            wait_until("trace picker mounts", || {
                by_role_name(&root, "button", "Select an option").is_some()
            })
            .await;
            let trigger = by_role_name(&root, "button", "Select an option").unwrap();
            trigger.focus().unwrap();
            key(&trigger, "ArrowDown");
            let apple = focus_option(&root, "Apple").await;
            let list_id = trigger.get_attribute("aria-controls").unwrap();
            let list = document.get_element_by_id(&list_id).unwrap();
            let animation = hold_popup_animation(&list);
            by_role_name(&root, "button", "Clear requests")
                .unwrap()
                .click();
            wait_until("trace is cleared", || {
                by_role_name(&root, "status", "Open requests")
                    .unwrap()
                    .text_content()
                    .as_deref()
                    == Some("[]")
            })
            .await;
            if action == "external" {
                by_role_name(&root, "button", "External close")
                    .unwrap()
                    .click();
            } else {
                key(&apple, action);
            }
            wait_until(
                "close restores focus while the popup remains mounted",
                || {
                    trigger.get_attribute("aria-expanded").as_deref() == Some("false")
                        && focused(&trigger)
                },
            )
            .await;
            assert!(
                list.is_connected(),
                "the closing animation must retain the option"
            );
            assert!(apple.is_connected());
            animation_method(&animation, "finish");
            wait_until("finishing the animation removes the popup", || {
                !list.is_connected()
            })
            .await;
            let trace = by_role_name(&root, "status", "Open requests")
                .unwrap()
                .text_content()
                .unwrap();
            console_log!("controlled={controlled}, action={action}, close requests={trace}");
            observed.push((controlled, action, trace));
            expected.push((
                controlled,
                action,
                if action == "external" {
                    "[]"
                } else {
                    "[false]"
                }
                .to_string(),
            ));
            root.remove();
        }
    }
    assert_eq!(
        observed, expected,
        "one callback per close action; an external close must not echo a request"
    );
}
