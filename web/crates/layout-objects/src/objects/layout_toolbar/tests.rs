use super::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[component]
fn snap_dock_fixture() -> Element {
    let menu = use_signal(|| None);
    let mut keys = use_signal(|| 0);
    rsx! {
        div { onkeydown: move |_| keys += 1,
            LayoutSnapDock { settings: LayoutSnapSettings::default(), open_menu: menu, on_snap_intent: |_| {} }
            output { role: "status", aria_label: "Pan keys", "{keys}" }
        }
    }
}

#[wasm_bindgen_test]
async fn snap_dock_keeps_its_touch_target_without_legacy_classes() {
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    let document = web_sys::window().unwrap().document().unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(&format!(
        "{}\n{}",
        include_str!("../../../../../assets/workbench-base.css"),
        include_str!("../../../../../assets/tailwind.css")
    )));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mut mounted = MountedFixture::new(root.clone(), VirtualDom::new(snap_dock_fixture));
    mounted.own_node(stylesheet.clone());
    wait_until("snap trigger", || {
        by_role_name(&root, "button", "Snap").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Snap").unwrap();
    let dock = trigger.parent_element().unwrap();
    let classes = dock.get_attribute("class").unwrap_or_default();
    dock.set_attribute(
        "class",
        &classes
            .split_whitespace()
            .filter(|class| !class.starts_with("m1-"))
            .collect::<Vec<_>>()
            .join(" "),
    )
    .unwrap();
    let window = web_sys::window().unwrap();
    let button_style = window.get_computed_style(&trigger).unwrap().unwrap();
    assert_eq!(
        button_style.get_property_value("min-height").unwrap(),
        "44px"
    );
    assert_eq!(
        button_style.get_property_value("padding-left").unwrap(),
        "12px"
    );
    assert_eq!(
        window
            .get_computed_style(&dock)
            .unwrap()
            .unwrap()
            .get_property_value("border-top-width")
            .unwrap(),
        "1px"
    );
    stylesheet.remove();
    root.remove();
}

#[wasm_bindgen_test]
async fn snap_dock_uses_the_new_disclosure_and_keeps_options_accessible() {
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(snap_dock_fixture));
    wait_until("snap trigger", || {
        by_role_name(&root, "button", "Snap").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Snap").unwrap();
    assert_eq!(
        trigger.get_attribute("data-slot").as_deref(),
        Some("collapsible-trigger")
    );
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(" ");
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    trigger.dispatch_event(&event).unwrap();
    assert!(
        !event.default_prevented(),
        "native activation remains available"
    );
    assert_eq!(
        by_role_name(&root, "status", "Pan keys")
            .unwrap()
            .text_content()
            .as_deref(),
        Some("0")
    );
    trigger.click();
    wait_until("snap options open", || {
        by_role_name(&root, "checkbox", "Geometry snap").is_some()
    })
    .await;
    assert_eq!(
        trigger.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
    root.remove();
}

#[component]
fn snap_options_instances() -> Element {
    rsx! {
        LayoutSnapOptions { settings: LayoutSnapSettings::default(), on_snap_intent: |_| {} }
        LayoutSnapOptions { settings: LayoutSnapSettings::default(), on_snap_intent: |_| {} }
    }
}

#[wasm_bindgen_test]
async fn snap_checkbox_labels_have_unique_targets_per_options_instance() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(snap_options_instances));
    gloo_timers::future::TimeoutFuture::new(40).await;
    let mut ids = std::collections::HashSet::new();
    for name in ["Geometry snap", "Envelope gap"] {
        let checkboxes =
            boardstudio_web_ui_shared::test_role::all_by_role_name(&root, "checkbox", name);
        assert_eq!(checkboxes.len(), 2);
        for checkbox in checkboxes {
            assert!(ids.insert(checkbox.id()));
            assert_eq!(
                checkbox
                    .closest("label")
                    .unwrap()
                    .unwrap()
                    .get_attribute("for"),
                Some(checkbox.id())
            );
            assert_eq!(
                document.get_element_by_id(&checkbox.id()).as_ref(),
                Some(checkbox.as_ref())
            );
        }
    }
    assert_eq!(ids.len(), 4);
    root.remove();
}

#[component]
fn direct_selection_fixture() -> Element {
    let open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
    let mut kind = use_signal(|| LayoutSelectionKind::Key);
    rsx! {
        LayoutSelectionToolbar { open_menu, selection_kind: kind(), on_selection_kind: move |next| kind.set(next) }
        output { id: "direct-selection-state", "{kind().label()}:{open_menu().is_none()}" }
    }
}

#[wasm_bindgen_test]
async fn direct_selection_choices_open_scope_panels_and_support_keyboard() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(direct_selection_fixture));
    gloo_timers::future::TimeoutFuture::new(40).await;
    for kind in LayoutSelectionKind::ALL {
        let button = boardstudio_web_ui_shared::test_role::by_role_name(
            &root,
            "button",
            &format!("Select {}", kind.label()),
        )
        .unwrap();
        button.click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        assert_eq!(
            root.query_selector("#direct-selection-state")
                .unwrap()
                .unwrap()
                .text_content(),
            Some(format!("{}:false", kind.label()))
        );
        assert_eq!(
            button.get_attribute("aria-pressed").as_deref(),
            Some("true")
        );
        assert_eq!(
            button.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
    }
    let matrix =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Select Matrix")
            .unwrap();
    matrix.focus().unwrap();
    let init = web_sys::KeyboardEventInit::new();
    init.set_key("ArrowRight");
    init.set_bubbles(true);
    init.set_cancelable(true);
    matrix
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
    gloo_timers::future::TimeoutFuture::new(30).await;
    assert_eq!(
        document
            .active_element()
            .unwrap()
            .get_attribute("aria-label")
            .as_deref(),
        Some("Select Row")
    );
    root.remove();
}

#[derive(Props, Clone, PartialEq)]
struct PillDuplicateProbeProps {
    open: Option<LayoutCommandMenu>,
    tools: bool,
}

#[component]
fn pill_duplicate_probe_host(props: PillDuplicateProbeProps) -> Element {
    let mut open_menu = use_signal(|| None::<LayoutCommandMenu>);
    let target = props.open;
    let tools = props.tools;
    let transform = super::super::layout_transform_toolbar::LayoutTransformMenuMount {
        properties_available: tools,
        column_available: true,
        row_available: true,
        // The real pill keeps the scope row visible and only swaps the tools;
        // `tools` here models whether a selection backs them, not the row itself.
        pointer_tools_visible: true,
        pointer_tools_available: tools,
        active_tool: None,
        matrix_rotate_followed: None,
        on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
        on_show_properties: EventHandler::new(|()| {}),
        on_transform_tool: EventHandler::new(
            |_: super::super::layout_transform_toolbar::LayoutTransformTool| {},
        ),
    };
    let align = super::super::layout_align::LayoutAlignMount {
        references: Vec::new(),
        selected_reference: None,
        action: None,
        enabled: false,
        disabled_reason: None,
        feedback: None,
        on_reference: EventHandler::new(|_: String| {}),
        on_align: EventHandler::new(|_: super::super::layout_align::AlignAction| {}),
    };
    rsx! {
      boardstudio_web_ui_shared::ui::toast::ToastProvider {
        Button {
            id: "pill-probe-open",
            onclick: move |_| open_menu.set(target),
            "Open probe menu"
        }
        LayoutCommandPill {
            command_label: "Layout".to_owned(),
            menu_owner_key: "duplicate-probe".to_owned(),
            open_menu,
            selection_kind: LayoutSelectionKind::Column,
            snap_settings: LayoutSnapSettings::default(),
            transform,
            align,
            show_relationships: false,
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_snap_intent: EventHandler::new(|_: LayoutSnapIntent| {}),
            has_selection_context: true,
            on_show_relationships: EventHandler::new(|_: ()| {}),
        }
      }
    }
}

fn tool_button_count(root: &web_sys::Element, label: &str) -> usize {
    let buttons = root.query_selector_all("button").unwrap();
    (0..buttons.length())
        .filter_map(|index| buttons.item(index))
        .filter(|node| node.text_content().is_some_and(|text| text.trim() == label))
        .count()
}

async fn mount_pill(open: Option<LayoutCommandMenu>) -> (web_sys::Element, MountedFixture) {
    mount_pill_with_tools(open, true).await
}

async fn mount_pill_with_tools(
    open: Option<LayoutCommandMenu>,
    tools: bool,
) -> (web_sys::Element, MountedFixture) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mounted = MountedFixture::new(
        root.clone(),
        VirtualDom::new_with_props(
            pill_duplicate_probe_host,
            PillDuplicateProbeProps { open, tools },
        ),
    );
    gloo_timers::future::TimeoutFuture::new(40).await;
    (root, mounted)
}

fn scope_trigger(root: &web_sys::Element, kind: &str) -> web_sys::HtmlElement {
    root.query_selector(&format!("button[aria-label='Select {kind}']"))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

async fn click(element: &web_sys::HtmlElement) {
    element.click();
    gloo_timers::future::TimeoutFuture::new(40).await;
}

#[wasm_bindgen_test]
async fn column_tools_appear_once_when_transform_popover_opens() {
    let (root, _mounted) = mount_pill(Some(LayoutCommandMenu::Transform)).await;
    root.query_selector("#pill-probe-open")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    gloo_timers::future::TimeoutFuture::new(40).await;
    for label in ["Stagger", "Spread", "Offset"] {
        assert_eq!(
            tool_button_count(&root, label),
            1,
            "{label} must appear exactly once: inline in the toolbar, not duplicated in the Transform popover"
        );
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn scope_popover_keeps_hint_when_nothing_selected() {
    let (root, _mounted) = mount_pill_with_tools(Some(LayoutCommandMenu::Select), false).await;
    click(
        &root
            .query_selector("#pill-probe-open")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap(),
    )
    .await;
    let hint =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "dialog", "Column actions")
            .expect("the no-selection scope popover still opens with a hint");
    assert!(
        hint.text_content()
            .unwrap_or_default()
            .contains("Select a column on the canvas"),
        "the no-selection hint points at the canvas"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn column_scope_trigger_opens_migration_actions_when_tools_visible() {
    let (root, _mounted) = mount_pill(Some(LayoutCommandMenu::Select)).await;
    click(&scope_trigger(&root, "Column")).await;
    let dialog =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "dialog", "Column actions")
            .expect("the migrated scope action popover opens");
    for name in [
        "Activate Stagger for the selected column",
        "Activate Spread for the selected column",
        "Activate Offset for the selected column",
    ] {
        assert!(
            boardstudio_web_ui_shared::test_role::by_role_name(&dialog, "button", name).is_some()
        );
    }
    root.remove();
}

#[component]
fn scope_reselect_probe_host() -> Element {
    let mut kind = use_signal(|| LayoutSelectionKind::Key);
    let mut has_selection = use_signal(|| false);
    let open_menu = use_signal(|| None::<LayoutCommandMenu>);
    let transform =
        has_selection().then(
            || super::super::layout_transform_toolbar::LayoutTransformMenuMount {
                properties_available: true,
                column_available: true,
                row_available: true,
                pointer_tools_visible: true,
                pointer_tools_available: true,
                active_tool: None,
                matrix_rotate_followed: None,
                on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
                on_show_properties: EventHandler::new(|()| {}),
                on_transform_tool: EventHandler::new(
                    |_: super::super::layout_transform_toolbar::LayoutTransformTool| {},
                ),
            },
        );
    rsx! {
        Button {
            id: "scope-probe-select-column",
            onclick: move |_| {
                has_selection.set(true);
                kind.set(LayoutSelectionKind::Column);
            },
            "Simulate column selection"
        }
        LayoutSelectionToolbar {
            open_menu,
            selection_kind: kind(),
            on_selection_kind: move |next| kind.set(next),
            transform,
        }
    }
}

#[wasm_bindgen_test]
async fn scope_panel_updates_when_column_selection_arrives_and_can_close() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(scope_reselect_probe_host));
    gloo_timers::future::TimeoutFuture::new(40).await;
    click(&scope_trigger(&root, "Column")).await;
    assert!(
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "dialog", "Column actions")
            .is_some(),
        "the no-selection scope trigger still opens its hint panel"
    );
    click(
        &boardstudio_web_ui_shared::test_role::by_role_name(
            &root,
            "button",
            "Simulate column selection",
        )
        .unwrap(),
    )
    .await;
    assert!(
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "dialog", "Column actions")
            .is_some(),
        "the migrated scope popover stays open and updates to the selected column's actions"
    );
    click(&scope_trigger(&root, "Column")).await;
    assert!(
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "dialog", "Column actions")
            .is_none(),
        "re-selecting the active Column scope closes its actions"
    );
    assert_eq!(
        scope_trigger(&root, "Column")
            .get_attribute("aria-pressed")
            .as_deref(),
        Some("true"),
        "the Column scope stays selected"
    );
    root.remove();
}
