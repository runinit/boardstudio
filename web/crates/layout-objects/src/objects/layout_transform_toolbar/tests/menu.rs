//! The Transform command menu, the contextual action toolbar and the snap dock.
use super::*;
use boardstudio_web_ui_shared::ui::button::Button;

fn menu_mount(
    pointer_tools_visible: bool,
    pointer_tools_available: bool,
    active_tool: Option<LayoutTransformTool>,
    on_transform_tool: EventHandler<LayoutTransformTool>,
) -> LayoutTransformMenuMount {
    LayoutTransformMenuMount {
        properties_available: true,
        column_available: true,
        row_available: true,
        pointer_tools_visible,
        pointer_tools_available,
        active_tool,
        matrix_rotate_followed: None,
        on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
        on_show_properties: EventHandler::new(|()| {}),
        on_transform_tool,
    }
}

/// Where a menu host mounts its tools: the Transform popover, the contextual toolbar for a
/// selection kind, or the Matrix toolbar with its rotation Follow.
#[derive(Clone, Copy, PartialEq)]
enum MenuSite {
    Popover,
    Context(LayoutSelectionKind),
    Matrix { followed: bool },
}

#[derive(Props, Clone, PartialEq)]
struct MenuHostProps {
    site: MenuSite,
    available: bool,
}

/// Offset is active; the chosen tool reports through `#chosen-tool`.
#[component]
fn menu_host(props: MenuHostProps) -> Element {
    let open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
    let mut chosen = use_signal(String::new);
    let on_tool = EventHandler::new(move |tool: LayoutTransformTool| {
        chosen.set(format!("{tool:?}"));
    });
    let mount = menu_mount(
        true,
        props.available,
        Some(LayoutTransformTool::Offset),
        on_tool,
    );
    rsx! {
        match props.site {
            MenuSite::Popover => rsx! { LayoutTransformToolbar { mount, open_menu } },
            MenuSite::Context(selection_kind) => rsx! {
                LayoutContextActionToolbar { mount, open_menu, selection_kind }
            },
            MenuSite::Matrix { followed } => rsx! {
                LayoutContextActionToolbar {
                    mount: LayoutTransformMenuMount {
                        active_tool: None,
                        matrix_rotate_followed: Some(followed),
                        ..mount
                    },
                    open_menu,
                    selection_kind: LayoutSelectionKind::Matrix,
                }
            },
        }
        output { id: "chosen-tool", "{chosen()}" }
    }
}

async fn mount_menu(site: MenuSite, available: bool) -> (web_sys::Element, MountedFixture) {
    let (root, mounted) = fixture::mount_root(VirtualDom::new_with_props(
        menu_host,
        MenuHostProps { site, available },
    ));
    wait_until("the transform menu renders", || {
        query(&root, "button").is_some()
    })
    .await;
    (root, mounted)
}

fn button_labels(root: &web_sys::Element) -> Vec<String> {
    query_all(root, "button")
        .iter()
        .filter_map(|button| button.text_content())
        .collect()
}

fn button_named(root: &web_sys::Element, text: &str) -> web_sys::HtmlElement {
    query_all(root, "button")
        .into_iter()
        .find(|button| button.text_content().as_deref() == Some(text))
        .and_then(|button| button.dyn_into().ok())
        .unwrap_or_else(|| panic!("the {text} button"))
}

async fn chosen_after_click(root: &web_sys::Element, button: &web_sys::HtmlElement) -> String {
    button.click();
    frame().await;
    text_of(root, "#chosen-tool")
}

#[wasm_bindgen_test]
async fn transform_popover_preserves_escape_and_outside_focus() {
    let (root, _mounted) = mount_menu(MenuSite::Popover, true).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let trigger = by_role_name(&root, "button", "Transform").unwrap();
    let content = by_role_name(&root, "menu", "Transform").unwrap();
    assert_eq!(content.get_attribute("role").as_deref(), Some("menu"));
    assert_eq!(
        content.get_attribute("aria-labelledby").as_deref(),
        Some(trigger.id().as_str())
    );
    let expanded = || trigger.get_attribute("aria-expanded");
    let focused = |id: &str| {
        document
            .active_element()
            .is_some_and(|element| element.id() == id)
    };
    fixture::press_key(&content, "Escape");
    wait_until("Escape closes the menu and restores trigger focus", || {
        expanded().as_deref() == Some("false") && focused(&trigger.id())
    })
    .await;
    trigger.click();
    wait_until("the transform menu reopens", || {
        expanded().as_deref() == Some("true")
    })
    .await;
    let outside = document.create_element("button").unwrap();
    outside.set_id("transform-outside-target");
    root.append_child(&outside).unwrap();
    outside
        .unchecked_ref::<web_sys::HtmlElement>()
        .focus()
        .unwrap();
    wait_until("outside focus closes the menu and stays outside", || {
        expanded().as_deref() == Some("false") && focused("transform-outside-target")
    })
    .await;
    root.remove();
}

/// The popover's column tools follow availability and the active tool, and dispatch.
#[wasm_bindgen_test]
async fn the_menu_lists_stagger_offset_splay_origin_and_follows_availability() {
    for available in [true, false] {
        let (root, _mounted) = mount_menu(MenuSite::Popover, available).await;
        let names = ["Stagger", "Offset", "Splay", "Origin"];
        let tools = names.map(|name| {
            by_role_name(&root, "menuitemradio", name)
                .expect("each existing tool remains a named menu item")
        });
        for (button, name) in tools.iter().zip(names) {
            assert_eq!(button.text_content().as_deref(), Some(name));
            assert_eq!(
                button.get_attribute("aria-disabled").as_deref(),
                Some(if available { "false" } else { "true" })
            );
            assert_eq!(
                button.get_attribute("aria-checked").as_deref(),
                Some(if name == "Offset" { "true" } else { "false" })
            );
        }
        if available {
            assert_eq!(chosen_after_click(&root, &tools[1]).await, "Offset");
        }
        root.remove();
    }
}

/// Ticket 23 slice: the Matrix contextual tools offer Move always, and Rotate enabled only
/// when the matrix follows rotation; otherwise Rotate stays visible but disabled with the
/// reason. The Transform popover no longer duplicates these tools, so the host mounts them
/// directly at their owning seam.
#[wasm_bindgen_test]
async fn the_matrix_menu_disables_rotate_without_follow_rotation() {
    for followed in [true, false] {
        let (root, _mounted) = mount_menu(MenuSite::Matrix { followed }, true).await;
        wait_until("the Matrix menu renders its Rotate action", || {
            button_labels(&root).iter().any(|label| label == "Rotate")
        })
        .await;
        assert!(
            button_labels(&root).iter().any(|label| label == "Move"),
            "the Matrix menu offers Move"
        );
        let rotate = button_named(&root, "Rotate");
        assert_eq!(
            rotate.has_attribute("disabled"),
            !followed,
            "Rotate follows the rotation Follow"
        );
        assert_eq!(
            rotate.get_attribute("aria-label").as_deref(),
            Some(if followed {
                "Activate Rotate for the selected matrix"
            } else {
                "Rotate matrix (unavailable: Follow rotation is off)"
            }),
            "the Rotate entry explains itself"
        );
        root.remove();
    }
}

/// The contextual toolbar omits unavailable tools instead of disabling them.
#[wasm_bindgen_test]
async fn contextual_column_tools_follow_availability_and_dispatch() {
    let site = MenuSite::Context(LayoutSelectionKind::Column);
    let (root, _mounted) = mount_menu(site, true).await;
    let labels = ["Position", "Stagger", "Offset", "Spread", "Splay"];
    assert_eq!(button_labels(&root), labels);
    for label in labels {
        let button = button_named(&root, label);
        assert!(!button.has_attribute("disabled"));
        let expected = match label {
            "Offset" => Some("true"),
            "Position" => None,
            _ => Some("false"),
        };
        assert_eq!(
            button.get_attribute("aria-pressed").as_deref(),
            expected,
            "pressed follows the active tool; the Position action carries no pressed state"
        );
    }
    let offset = button_named(&root, "Offset");
    assert_eq!(chosen_after_click(&root, &offset).await, "Offset");
    root.remove();

    let (root, _mounted) = mount_menu(site, false).await;
    assert_eq!(
        button_labels(&root),
        ["Position"],
        "unavailable pointer tools are omitted, leaving only the Position action"
    );
    root.remove();
}

/// Properties shortcuts must select the Properties tab before the selection changes owner.
#[component]
fn shortcut_test_host() -> Element {
    let mut properties_tab = use_signal(|| false);
    let mut owner_current = use_signal(|| true);
    let mut selection_changes = use_signal(|| 0_u32);
    let mut open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
    let mount = LayoutTransformMenuMount {
        on_show_properties: EventHandler::new(move |()| {
            if owner_current() {
                properties_tab.set(true);
            }
        }),
        on_selection_kind: EventHandler::new(move |_: LayoutSelectionKind| {
            owner_current.set(false);
            selection_changes += 1;
        }),
        ..menu_mount(false, false, None, EventHandler::new(|_| {}))
    };
    rsx! {
        LayoutTransformToolbar { mount, open_menu }
        Button {
            id: "reset-shortcut-probe",
            onclick: move |_| {
                properties_tab.set(false);
                owner_current.set(true);
                selection_changes.set(0);
                open_menu.set(Some(LayoutCommandMenu::Transform));
            },
            "Reset"
        }
        output { id: "shortcut-probe-state", "{properties_tab()}:{selection_changes()}" }
    }
}

#[wasm_bindgen_test]
async fn all_transform_property_shortcuts_route_to_properties_before_selection_changes_owner() {
    let (root, _mounted) = fixture::mount_root(VirtualDom::new(shortcut_test_host));
    frame().await;
    for (label, expected) in [
        ("Position & rotation", "true:0"),
        ("Splay & origin", "true:1"),
        ("Row offsets", "true:1"),
    ] {
        by_role_name(&root, "menuitem", label)
            .expect("transform shortcut menu item exists")
            .click();
        frame().await;
        assert_eq!(
            text_of(&root, "#shortcut-probe-state"),
            expected,
            "{label} must select Properties before any selection transition"
        );
        query(&root, "#reset-shortcut-probe")
            .unwrap()
            .unchecked_into::<web_sys::HtmlElement>()
            .click();
        frame().await;
    }
    root.remove();
}

#[derive(Props, Clone, PartialEq)]
struct ContextHostProps {
    selection_kind: LayoutSelectionKind,
    pointer_tools_visible: bool,
    pointer_tools_available: bool,
}

/// The contextual toolbar over the real activation rules; `#context-action-state` reports
/// `<selection kind>:<active tool>`.
#[component]
fn context_test_host(props: ContextHostProps) -> Element {
    let mut active_tool = use_signal(|| None::<LayoutTransformTool>);
    let mut selection_kind = use_signal(|| props.selection_kind);
    let open_menu = use_signal(|| None::<LayoutCommandMenu>);
    let on_transform_tool = EventHandler::new(move |tool| {
        let context = match selection_kind() {
            LayoutSelectionKind::Row => TreeContext::Row {
                matrix_id: "matrix-1".to_owned(),
                row: 0,
            },
            _ => TreeContext::Column {
                matrix_id: "matrix-1".to_owned(),
                column: 0,
            },
        };
        let activation = activate_transform_tool(active_tool(), tool, &context);
        active_tool.set(activation.active);
        if let Some(next_kind) = activation.selection_kind {
            selection_kind.set(next_kind);
        }
    });
    let mount = LayoutTransformMenuMount {
        active_tool: active_tool(),
        on_selection_kind: EventHandler::new(move |kind| selection_kind.set(kind)),
        ..menu_mount(
            props.pointer_tools_visible,
            props.pointer_tools_available,
            None,
            on_transform_tool,
        )
    };
    let active_label = active_tool().map_or("None", feedback_tool_name);
    rsx! {
        LayoutContextActionToolbar { mount, open_menu, selection_kind: selection_kind() }
        output { id: "context-action-state", "{selection_kind():?}:{active_label}" }
    }
}

async fn mount_context_actions(
    selection_kind: LayoutSelectionKind,
    pointer_tools_visible: bool,
    pointer_tools_available: bool,
) -> (web_sys::Element, MountedFixture) {
    let (root, mounted) = fixture::mount_root(VirtualDom::new_with_props(
        context_test_host,
        ContextHostProps {
            selection_kind,
            pointer_tools_visible,
            pointer_tools_available,
        },
    ));
    wait_until(
        "contextual action fixture has rendered its selection",
        || query(&root, "#context-action-state").is_some(),
    )
    .await;
    (root, mounted)
}

fn active_label() -> Option<String> {
    web_sys::window()?
        .document()?
        .active_element()?
        .get_attribute("aria-label")
}

#[wasm_bindgen_test]
async fn contextual_toolbar_has_one_tab_stop_and_end_reaches_last_action() {
    let (root, _mounted) = mount_context_actions(LayoutSelectionKind::Column, true, true).await;
    let toolbar = by_role_name(&root, "toolbar", "Actions for Column").unwrap();
    assert_eq!(
        query_all(&toolbar, "[tabindex='0']").len(),
        1,
        "the contextual toolbar must expose a single tab stop"
    );
    let position = by_role_name(
        &toolbar,
        "button",
        "Open Position and rotation properties for Column",
    )
    .unwrap();
    position.focus().unwrap();
    fixture::press_key(&position, "End");
    wait_until("End focuses the Splay action", || {
        active_label().as_deref() == Some("Activate Splay for the selected column")
    })
    .await;
    root.remove();
}

#[wasm_bindgen_test]
async fn contextual_tools_show_for_layout_selection_and_follow_existing_activation_scope() {
    let (root, _mounted) = mount_context_actions(LayoutSelectionKind::Row, true, true).await;
    let toolbar = by_role_name(&root, "toolbar", "Actions for Row").unwrap();
    let names = [
        "Open Row offsets properties for Row",
        "Activate Offset for the selected row",
        "Activate Stagger column; selection changes to Column",
    ];
    let labels = names.map(|name| {
        by_role_name(&toolbar, "button", name)
            .unwrap()
            .text_content()
            .unwrap_or_default()
    });
    assert_eq!(labels, ["Row offsets", "Offset", "Stagger"]);
    by_role_name(&toolbar, "button", names[2]).unwrap().click();
    wait_until(
        "Stagger activates and changes the selection to Column",
        || text_of(&root, "#context-action-state") == "Column:Stagger",
    )
    .await;
    let stagger =
        by_role_name(&root, "button", "Activate Stagger for the selected column").unwrap();
    assert_eq!(
        stagger.get_attribute("aria-pressed").as_deref(),
        Some("true")
    );
    stagger.focus().unwrap();
    fixture::press_key(&stagger, "ArrowRight");
    // Toolbar keyboard navigation must follow the updated selection's action order.
    wait_until(
        "ArrowRight focuses Offset for the new column selection",
        || active_label().as_deref() == Some("Activate Offset for the selected column"),
    )
    .await;
    root.remove();

    let (pcb, _mounted) = mount_context_actions(LayoutSelectionKind::Matrix, false, true).await;
    assert!(
        by_role_name(&pcb, "toolbar", "Actions for Matrix").is_none(),
        "PCB must not inherit Layout-only contextual quick actions"
    );
    pcb.remove();
}

/// Ticket 07: a part selection keeps the Position strip without canvas tools, like
/// production (the anchor pick arrives with ticket 08).
#[wasm_bindgen_test]
async fn part_selection_keeps_the_position_strip_without_canvas_tools() {
    let (root, _mounted) = mount_context_actions(LayoutSelectionKind::Part, true, false).await;
    let toolbar = by_role_name(&root, "toolbar", "Actions for Part").expect("part actions toolbar");
    let position = by_role_name(
        &toolbar,
        "button",
        "Open Position and rotation properties for Part",
    )
    .expect("the Part Position action remains available");
    assert_eq!(position.text_content().as_deref(), Some("Position"));
    assert_eq!(text_of(&root, "#context-action-state"), "Part:None");
    root.remove();
}

/// Ticket 23: the Handles toggle flips view state and reports the intent, so hiding handles
/// never touches the document. The host owns no Runtime and no edit machinery: the intent
/// carries a plain bool to workspace state.
#[component]
fn snap_dock_host() -> Element {
    let mut handles = use_signal(|| true);
    let mut toggled = use_signal(|| None::<bool>);
    rsx! {
        super::super::super::layout_toolbar::LayoutSnapDock {
            settings: super::super::super::layout_toolbar::LayoutSnapSettings::default(),
            open_menu: use_signal(|| None),
            on_snap_intent: EventHandler::new(|_| {}),
            handles_visible: handles(),
            on_toggle_handles: EventHandler::new(move |visible| {
                toggled.set(Some(visible));
                handles.set(visible);
            }),
        }
        output { id: "snap-dock-toggled", "{toggled():?}" }
    }
}

#[wasm_bindgen_test]
async fn handles_toggle_reports_intent_and_flips_view_only() {
    let (root, _mounted) = fixture::mount_root(VirtualDom::new(snap_dock_host));
    wait_until("snap dock trigger is mounted", || {
        query(&root, "#m1-layout-snap-trigger").is_some()
    })
    .await;
    query(&root, "#m1-layout-snap-trigger")
        .unwrap()
        .unchecked_into::<web_sys::HtmlElement>()
        .click();
    let checkbox = || by_role_name(&root, "checkbox", "Canvas handles");
    wait_until("the snap dock exposes the Canvas handles checkbox", || {
        checkbox().is_some()
    })
    .await;
    let checked = || checkbox().and_then(|checkbox| checkbox.get_attribute("aria-checked"));
    assert_eq!(checked().as_deref(), Some("true"), "handles start visible");
    for (visible, reported) in [("false", "Some(false)"), ("true", "Some(true)")] {
        checkbox().unwrap().click();
        // The toggle reports the flipped view state and the checkbox follows it.
        wait_until("the handles toggle reports its intent", || {
            text_of(&root, "#snap-dock-toggled") == reported
                && checked().as_deref() == Some(visible)
        })
        .await;
    }
    root.remove();
}
