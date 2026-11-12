//! Private Layout selection and snap controls plus their shared value adapters.
use super::tree::{self, TreeContext};
use boardstudio_application::ReadModel;
use boardstudio_core::model::Vec2;
pub use boardstudio_web_ui_model::state::{LayoutSnapSettings, TreeCellAnchor};
use boardstudio_web_ui_shared::ui::{
    callout::Callout,
    checkbox::Checkbox,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    icons::{ChevronDown, ChevronRight},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    popover::{Popover, PopoverContent, PopoverTrigger},
    select::{Select, SelectOption},
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::{Toolbar, ToolbarButton},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Document, HtmlElement, Node, PointerEvent as WebPointerEvent};

const DEFAULT_PITCH_MM: f64 = 19.05;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutSelectionKind {
    Matrix,
    Row,
    Column,
    #[default]
    Key,
    Part,
}

impl LayoutSelectionKind {
    const ALL: [Self; 5] = [Self::Matrix, Self::Row, Self::Column, Self::Key, Self::Part];

    const fn label(self) -> &'static str {
        match self {
            Self::Matrix => "Matrix",
            Self::Row => "Row",
            Self::Column => "Column",
            Self::Key => "Key",
            Self::Part => "Part",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutSnapIntent {
    Fraction(f64),
    GeometrySnap(bool),
    GapSnap(bool),
    GapOverride(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureSnapInputs {
    pub pitch: Vec2,
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutCanvasGridStyle {
    pub spacing_mm: f64,
    pub radius_mm: f64,
}

/// Match the React canvas background grid, including its denser, screen-stable outline-edit grid.
pub fn layout_canvas_grid_style(
    view_width_mm: f64,
    surface_width_px: f64,
    pitch_x_mm: f64,
    snap_fraction: f64,
    outline_editing: bool,
) -> LayoutCanvasGridStyle {
    if !outline_editing {
        return LayoutCanvasGridStyle {
            spacing_mm: DEFAULT_PITCH_MM / 2.0,
            radius_mm: 0.12,
        };
    }

    let pitch_x_mm = finite_positive(pitch_x_mm).unwrap_or(DEFAULT_PITCH_MM);
    let snap_fraction = if snap_fraction.is_finite() {
        snap_fraction
    } else {
        0.25
    };
    let base_grid_mm = if snap_fraction < 0.0 {
        -snap_fraction
    } else {
        pitch_x_mm * snap_fraction
    }
    .max(0.1);
    let view_width_mm = finite_positive(view_width_mm).unwrap_or(1.0);
    let surface_width_px = finite_positive(surface_width_px).unwrap_or(1.0).max(1.0);
    let minimum_screen_spacing_mm = 8.0 * view_width_mm / surface_width_px;
    let spacing_mm = base_grid_mm * (minimum_screen_spacing_mm / base_grid_mm).ceil().max(1.0);

    LayoutCanvasGridStyle {
        spacing_mm,
        radius_mm: view_width_mm / surface_width_px * 0.7,
    }
}

/// Produce the exact existing GestureBegin snap values from current root-owned preferences.
pub fn gesture_snap_inputs(
    settings: &LayoutSnapSettings,
    accepted_matrix_pitch: Option<Vec2>,
    selected_matrix_edge_gap: Option<f64>,
) -> GestureSnapInputs {
    let pitch = accepted_matrix_pitch.unwrap_or(Vec2 {
        x: DEFAULT_PITCH_MM,
        y: DEFAULT_PITCH_MM,
    });
    let pitch = Vec2 {
        x: finite_positive(pitch.x).unwrap_or(DEFAULT_PITCH_MM),
        y: finite_positive(pitch.y).unwrap_or(DEFAULT_PITCH_MM),
    };
    let snap_fraction = if settings.snap_fraction.is_finite() {
        settings.snap_fraction
    } else {
        0.25
    };
    let geometry_snap = settings.geometry_snap;
    let gap = if geometry_snap && settings.gap_snap {
        settings
            .gap_override
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .or_else(|| selected_matrix_edge_gap.filter(|value| value.is_finite() && *value >= 0.0))
            .or(Some(1.0))
    } else {
        None
    };
    GestureSnapInputs {
        pitch,
        snap_fraction,
        geometry_snap,
        gap,
    }
}

fn finite_positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            include_str!("../../../../assets/workbench-base.css"),
            include_str!("../../../../assets/tailwind.css")
        )));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(snap_dock_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
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
        for (theme, stroke) in [("original", "1.5px"), ("shadcn", "2px")] {
            root.set_attribute("data-theme", theme).unwrap();
            for open in [false, true] {
                let style = window.get_computed_style(&trigger).unwrap().unwrap();
                assert_eq!(
                    style.get_property_value("font-size").unwrap(),
                    "14px",
                    "{theme}, open={open}"
                );
                assert_eq!(
                    style.get_property_value("font-weight").unwrap(),
                    "500",
                    "{theme}, open={open}"
                );
                let chevron = trigger
                    .query_selector("svg[data-slot=icon]")
                    .unwrap()
                    .expect("shared disclosure chevron");
                assert_eq!(
                    chevron.get_attribute("aria-hidden").as_deref(),
                    Some("true")
                );
                assert!(chevron.get_bounding_client_rect().width() > 0.0);
                assert_eq!(
                    window
                        .get_computed_style(&chevron)
                        .unwrap()
                        .unwrap()
                        .get_property_value("stroke-width")
                        .unwrap(),
                    stroke
                );
                assert_eq!(
                    trigger.get_attribute("aria-expanded").as_deref(),
                    Some(if open { "true" } else { "false" })
                );
                trigger.click();
                wait_until("snap disclosure toggles", || {
                    trigger.get_attribute("aria-expanded").as_deref()
                        == Some(if open { "false" } else { "true" })
                })
                .await;
            }
        }
        stylesheet.remove();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn snap_dock_uses_the_new_disclosure_and_keeps_options_accessible() {
        use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(snap_dock_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
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
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(snap_options_instances),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
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
    async fn direct_selection_choices_stay_inline_and_support_keyboard() {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(&format!(
            "{}\n{}",
            include_str!("../../../../assets/workbench-base.css"),
            include_str!("../../../../assets/tailwind.css")
        )));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let root = document.create_element("div").unwrap();
        root.set_attribute("data-theme", "original").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(direct_selection_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        let group = boardstudio_web_ui_shared::test_role::by_role_name(
            &root,
            "group",
            "Selection scope choices",
        )
        .unwrap();
        // DESIGN's filled Toggle Group: Original Light muted inset, background active item.
        // Known Theme role values, not a class/selector oracle (011 D5).
        assert_eq!(
            window
                .get_computed_style(&group)
                .unwrap()
                .unwrap()
                .get_property_value("background-color")
                .unwrap(),
            "rgb(247, 248, 250)"
        );
        for kind in LayoutSelectionKind::ALL {
            let button = boardstudio_web_ui_shared::test_role::by_role_name(
                &root,
                "button",
                &format!("Select {}", kind.label()),
            )
            .unwrap();
            button.focus().unwrap();
            button.click();
            gloo_timers::future::TimeoutFuture::new(30).await;
            assert_eq!(
                document.active_element().as_ref(),
                Some(button.as_ref()),
                "scope choice retains trigger focus"
            );
            assert_eq!(
                root.query_selector("#direct-selection-state")
                    .unwrap()
                    .unwrap()
                    .text_content(),
                Some(format!("{}:true", kind.label()))
            );
            assert_eq!(
                button.get_attribute("aria-pressed").as_deref(),
                Some("true")
            );
            assert_eq!(
                button.get_attribute("aria-haspopup"),
                None,
                "a scope choice is not a popup trigger"
            );
            button.click();
            gloo_timers::future::TimeoutFuture::new(30).await;
            assert_eq!(
                button.get_attribute("aria-pressed").as_deref(),
                Some("true")
            );
            assert!(
                root.query_selector("[role=dialog], [role=menu]")
                    .unwrap()
                    .is_none()
            );
            assert_eq!(document.active_element().as_ref(), Some(button.as_ref()));
            assert_eq!(button.get_attribute("data-state").as_deref(), Some("on"));
            assert_eq!(
                window
                    .get_computed_style(&button)
                    .unwrap()
                    .unwrap()
                    .get_property_value("background-color")
                    .unwrap(),
                "rgb(233, 238, 242)",
                "{} uses the background role when selected",
                kind.label()
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
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
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
        stylesheet.remove();
        root.remove();
    }

    #[derive(Props, Clone, PartialEq)]
    struct PillDuplicateProbeProps {
        open: Option<LayoutCommandMenu>,
        tools: bool,
        accordion: bool,
    }

    #[component]
    fn pill_duplicate_probe_host(props: PillDuplicateProbeProps) -> Element {
        let mut open_menu = use_signal(|| None::<LayoutCommandMenu>);
        let target = props.open;
        let mut kind = use_signal(|| LayoutSelectionKind::Column);
        let mut selected = use_signal(|| props.tools);
        let tools = selected();
        let transform = super::super::layout_transform_toolbar::LayoutTransformMenuMount {
            // A real selected Part has a Component Inspector, not the matrix
            // transform projection that sets these two availability flags.
            properties_available: tools && kind() != LayoutSelectionKind::Part,
            column_available: true,
            row_available: true,
            // The real pill keeps the scope row visible and only swaps the tools;
            // `tools` here models whether a selection backs them, not the row itself.
            pointer_tools_visible: true,
            pointer_tools_available: tools && kind() != LayoutSelectionKind::Part,
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
            busy: false,
            feedback: None,
            on_reference: EventHandler::new(|_: String| {}),
            on_align: EventHandler::new(|_: super::super::layout_align::AlignAction| {}),
        };
        let pill = rsx! {
            LayoutCommandPill {
                command_label: "Layout".to_owned(),
                menu_owner_key: "duplicate-probe".to_owned(),
                open_menu,
                selection_kind: kind(),
                snap_settings: LayoutSnapSettings::default(),
                transform,
                align,
                show_relationships: false,
                on_selection_kind: EventHandler::new(move |next| kind.set(next)),
                on_snap_intent: EventHandler::new(|_: LayoutSnapIntent| {}),
                has_selection_context: tools,
                on_show_relationships: EventHandler::new(|_: ()| {}),
                snap_in_dock: true,
                accordion: props.accordion,
                view: Some(rsx! { span { "View probe" } }),
                utilities: Some(rsx! { span { "Utilities probe" } }),
            }
        };
        rsx! {
          boardstudio_web_ui_shared::ui::toast::ToastProvider {
            button {
                id: "pill-probe-open",
                onclick: move |_| open_menu.set(target),
                "Open probe menu"
            }
            button {
                id: "scope-probe-select-column",
                onclick: move |_| { selected.set(true); kind.set(LayoutSelectionKind::Column); },
                "Simulate column selection"
            }
            if props.accordion {
                Toolbar { aria_label: "Layout tools", {pill} }
            } else {
                {pill}
            }
          }
        }
    }

    fn tool_button_count(root: &web_sys::Element, label: &str) -> usize {
        let buttons = root
            .query_selector_all("button, [role=menuitem], [role=menuitemradio]")
            .unwrap();
        (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter(|node| node.text_content().is_some_and(|text| text.trim() == label))
            .count()
    }

    async fn mount_pill(open: Option<LayoutCommandMenu>) -> web_sys::Element {
        mount_pill_with_tools(open, true).await
    }

    async fn mount_pill_with_tools(
        open: Option<LayoutCommandMenu>,
        tools: bool,
    ) -> web_sys::Element {
        mount_pill_with_options(open, tools, false).await
    }

    async fn mount_accordion_pill(open: Option<LayoutCommandMenu>) -> web_sys::Element {
        mount_pill_with_options(open, true, true).await
    }

    async fn mount_pill_with_options(
        open: Option<LayoutCommandMenu>,
        tools: bool,
        accordion: bool,
    ) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                pill_duplicate_probe_host,
                PillDuplicateProbeProps {
                    open,
                    tools,
                    accordion,
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn accordion_sections_keep_existing_tools_inline() {
        let root = mount_accordion_pill(Some(LayoutCommandMenu::Transform)).await;
        root.query_selector("#pill-probe-open")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector_all("[data-workbench-section]")
                .unwrap()
                .length(),
            5
        );
        for label in ["Select", "Transform", "Align", "Snap", "View"] {
            assert!(
                boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", label)
                    .is_some(),
                "{label} section trigger"
            );
        }
        assert_eq!(
            root.query_selector("[data-workbench-section='transform']")
                .unwrap()
                .unwrap()
                .get_attribute("data-open")
                .as_deref(),
            Some("true")
        );
        for label in ["Position", "Stagger", "Spread", "Offset", "Splay", "Origin"] {
            assert_eq!(
                tool_button_count(&root, label),
                1,
                "{label} occurs once inline"
            );
        }
        let select = boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Select")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        click(&select).await;
        assert_eq!(
            root.query_selector("[data-workbench-section='select']")
                .unwrap()
                .unwrap()
                .get_attribute("data-open")
                .as_deref(),
            Some("true")
        );
        assert!(root.query_selector("[data-workbench-section='transform'][data-open='false'] .workbench-tool-reveal[inert]").unwrap().is_some());
        let snap = boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Snap")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        click(&snap).await;
        for label in ["Off", "⅛u", "¼u", "½u", "1u", "1 mm", "0.5 mm", "0.1 mm"] {
            assert!(
                boardstudio_web_ui_shared::test_role::by_role_name(
                    &root,
                    "button",
                    format!("Snap increment {label}").as_str()
                )
                .is_some(),
                "{label} size button"
            );
        }
        assert!(
            root.query_selector("input[aria-label='Custom snap increment']")
                .unwrap()
                .is_some()
        );
        assert!(
            root.query_selector("#layout-snap-gap-inline")
                .unwrap()
                .is_some()
        );
        let view = boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "View")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        click(&view).await;
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("View probe")
        );
        assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Utilities probe")
        );
        root.remove();
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
        let root = mount_pill(Some(LayoutCommandMenu::Transform)).await;
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
    async fn scope_choices_keep_inline_guidance_when_nothing_selected() {
        let root = mount_pill_with_tools(None, false).await;
        for kind in LayoutSelectionKind::ALL {
            let trigger = scope_trigger(&root, kind.label());
            trigger.focus().unwrap();
            for _ in 0..2 {
                click(&trigger).await;
                assert_eq!(
                    trigger.get_attribute("aria-pressed").as_deref(),
                    Some("true")
                );
                assert!(
                    root.query_selector("[role=dialog], [role=menu]")
                        .unwrap()
                        .is_none()
                );
                let hint = root
                    .query_selector("[role=status]")
                    .unwrap()
                    .expect("inline selection guidance");
                assert!(hint.text_content().unwrap_or_default().contains(&format!(
                    "Select a {} on the canvas",
                    kind.label().to_lowercase()
                )));
                assert_eq!(
                    web_sys::window()
                        .unwrap()
                        .document()
                        .unwrap()
                        .active_element()
                        .as_ref(),
                    Some(trigger.as_ref())
                );
            }
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn scope_actions_appear_once_on_choice_reselection_and_transform_menu() {
        let root = mount_pill(Some(LayoutCommandMenu::Transform)).await;
        // Restore A4's Matrix/Row/Column guards and apply the same oracle to Key/Part.
        for (kind, labels) in [
            (
                "Matrix",
                &[
                    "Position", "Move", "Rotate", "Stagger", "Spread", "Offset", "Splay",
                ][..],
            ),
            ("Row", &["Row offsets", "Stagger", "Spread", "Offset"][..]),
            (
                "Column",
                &["Position", "Stagger", "Spread", "Offset", "Splay"][..],
            ),
            ("Key", &["Position", "Stagger", "Spread", "Offset"][..]),
            ("Part", &["Position"][..]),
        ] {
            let trigger = scope_trigger(&root, kind);
            trigger.focus().unwrap();
            for _ in 0..2 {
                click(&trigger).await;
                assert!(
                    root.query_selector("[role=dialog], [role=menu]")
                        .unwrap()
                        .is_none(),
                    "{kind} has no scope popup"
                );
                assert_eq!(
                    trigger.get_attribute("aria-pressed").as_deref(),
                    Some("true")
                );
                for label in labels {
                    assert_eq!(
                        tool_button_count(&root, label),
                        1,
                        "{kind}: {label} occurs once inline"
                    );
                }
            }
            let transform = root
                .query_selector("#m1-layout-transform-trigger")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            transform.focus().unwrap();
            click(&transform).await;
            for label in labels {
                assert_eq!(
                    tool_button_count(&root, label),
                    1,
                    "{kind}: {label} stays inline even with Transform open"
                );
            }
            let menu = root
                .query_selector("#m1-layout-transform-menu")
                .unwrap()
                .unwrap();
            assert_eq!(
                tool_button_count(&menu, "Origin"),
                1,
                "Transform retains its non-inline action"
            );
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Escape");
            init.set_bubbles(true);
            init.set_cancelable(true);
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
            gloo_timers::future::TimeoutFuture::new(40).await;
            assert!(root.query_selector("[role=menu]").unwrap().is_none());
            assert_eq!(
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .active_element()
                    .as_ref(),
                Some(transform.as_ref())
            );
        }
        // Align uses Popover (not DropdownMenu): its Escape relies on the explicit
        // restore_layout_command_focus, while DropdownMenu restores itself. This
        // catches removal of that restore; Transform alone would still pass.
        let align = root
            .query_selector("#m1-layout-align-trigger")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        align.focus().unwrap();
        click(&align).await;
        let align_menu = root
            .query_selector("#m1-layout-align-menu")
            .unwrap()
            .expect("Align popover opens");
        assert_eq!(
            align_menu.get_attribute("role").as_deref(),
            Some("dialog"),
            "Align uses the Popover dialog seam"
        );
        let close = align_menu
            .query_selector("button[aria-label='Close Align']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        close.focus().unwrap();
        gloo_timers::future::TimeoutFuture::new(20).await;
        let document = web_sys::window().unwrap().document().unwrap();
        assert_eq!(
            document.active_element().as_ref(),
            Some(close.as_ref()),
            "focus moves inside Align before Escape"
        );
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("Escape");
        init.set_bubbles(true);
        init.set_cancelable(true);
        align_menu
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert!(
            root.query_selector("[role=dialog]").unwrap().is_none(),
            "Align closes on Escape"
        );
        assert_eq!(
            document.active_element().as_ref(),
            Some(align.as_ref()),
            "Align trigger regains DOM focus after Escape"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn inline_guidance_yields_to_actions_without_displacing_compact_scope_menus() {
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(&format!(
            "{}\n{}\n{}",
            include_str!("../../../../assets/workbench-base.css"),
            include_str!("../../../../assets/tailwind.css"),
            include_str!("../../../../assets/layout-toolbar.css")
        )));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        for width in [800, 430, 280] {
            let root = mount_pill_with_tools(None, false).await;
            root.set_attribute("style", &format!("width: {width}px"))
                .unwrap();
            click(&scope_trigger(&root, "Column")).await;
            let scope = boardstudio_web_ui_shared::test_role::by_role_name(
                &root,
                "toolbar",
                "Selection scope",
            )
            .unwrap();
            let menus = boardstudio_web_ui_shared::test_role::by_role_name(
                &root,
                "group",
                "Selection actions",
            )
            .unwrap();
            let before = menus.get_bounding_client_rect();
            let scope_bounds = scope.parent_element().unwrap().get_bounding_client_rect();
            if width == 800 {
                assert!(
                    (before.top() - scope_bounds.top()).abs() <= 1.0,
                    "wide controls share a row"
                );
                assert!(
                    (0.0..=12.0).contains(&(before.left() - scope_bounds.right())),
                    "normal adjacent gap"
                );
            } else {
                assert!(
                    (0.0..=12.0).contains(&(before.top() - scope_bounds.bottom())),
                    "menus wrap immediately beneath scope, not beneath an empty row"
                );
            }
            assert!(
                root.query_selector("[role=status]").unwrap().is_some(),
                "inline guidance before selection"
            );
            let root_bounds = root.get_bounding_client_rect();
            let buttons = root
                .query_selector_all("#m1-layout-command-pill button")
                .unwrap();
            for index in 0..buttons.length() {
                let button = buttons
                    .item(index)
                    .unwrap()
                    .dyn_into::<web_sys::Element>()
                    .unwrap();
                let bounds = button.get_bounding_client_rect();
                assert!(
                    bounds.left() >= root_bounds.left() - 1.0
                        && bounds.right() <= root_bounds.right() + 1.0,
                    "{width}px: no horizontal control clipping"
                );
            }
            click(
                &root
                    .query_selector("#scope-probe-select-column")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::HtmlElement>()
                    .unwrap(),
            )
            .await;
            assert!(
                root.query_selector("[role=status]").unwrap().is_none(),
                "actions replace guidance"
            );
            for label in ["Position", "Stagger", "Spread", "Offset", "Splay"] {
                assert_eq!(tool_button_count(&root, label), 1);
            }
            let after = menus.get_bounding_client_rect();
            assert_eq!(
                (before.x(), before.y()),
                (after.x(), after.y()),
                "selection does not displace scope/menu grouping"
            );
            click(&scope_trigger(&root, "Column")).await;
            assert_eq!(
                scope_trigger(&root, "Column")
                    .get_attribute("aria-pressed")
                    .as_deref(),
                Some("true")
            );
            assert!(
                root.query_selector("[role=dialog], [role=menu]")
                    .unwrap()
                    .is_none()
            );
            root.remove();
        }
        stylesheet.remove();
    }

    #[wasm_bindgen_test]
    fn retained_cell_survives_key_row_column_and_key_mode_changes() {
        let anchor = TreeCellAnchor {
            matrix_id: "matrix-1".to_owned(),
            row: 2,
            column: 3,
        };
        let row = TreeContext::Row {
            matrix_id: "matrix-1".to_owned(),
            row: 2,
        };
        let column = TreeContext::Column {
            matrix_id: "matrix-1".to_owned(),
            column: 3,
        };
        let key = TreeContext::Key {
            matrix_id: "matrix-1".to_owned(),
            row: 2,
            column: 3,
        };
        assert_eq!(
            retained_coordinates(&row, &anchor),
            Some(("matrix-1".to_owned(), 2, 3))
        );
        assert_eq!(
            retained_coordinates(&column, &anchor),
            Some(("matrix-1".to_owned(), 2, 3))
        );
        assert_eq!(
            retained_coordinates(&key, &anchor),
            Some(("matrix-1".to_owned(), 2, 3))
        );

        let changed_row = TreeContext::Row {
            matrix_id: "matrix-1".to_owned(),
            row: 1,
        };
        assert_eq!(retained_coordinates(&changed_row, &anchor), None);
    }

    #[wasm_bindgen_test]
    fn part_scope_is_a_real_component_or_an_empty_semantic_cell() {
        let cell = TreeContext::Key {
            matrix_id: "matrix-1".to_owned(),
            row: 2,
            column: 3,
        };
        let selected = part_context(cell.clone(), &["part-real".to_owned()]).unwrap();
        assert_eq!(
            selected,
            TreeContext::Component {
                part_id: Some("part-real".to_owned()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            }
        );
        assert_eq!(part_context(cell.clone(), &[]), Some(cell));
        assert_eq!(
            part_context(
                TreeContext::Key {
                    matrix_id: "matrix-1".to_owned(),
                    row: 2,
                    column: 3,
                },
                &["first".to_owned(), "second".to_owned()],
            ),
            None
        );
    }

    #[wasm_bindgen_test]
    fn gesture_inputs_preserve_off_fractional_and_world_millimeter_modes() {
        let settings = LayoutSnapSettings {
            snap_fraction: 0.0,
            geometry_snap: true,
            gap_snap: true,
            gap_override: "0.7".to_owned(),
        };
        let off = gesture_snap_inputs(&settings, Some(Vec2 { x: 19.0, y: 18.0 }), Some(1.2));
        assert_eq!(off.snap_fraction, 0.0);
        assert_eq!(off.gap, Some(0.7));

        let millimeters = LayoutSnapSettings {
            snap_fraction: -0.5,
            gap_snap: false,
            ..settings
        };
        let mm = gesture_snap_inputs(&millimeters, None, None);
        assert_eq!(mm.pitch, Vec2 { x: 19.05, y: 19.05 });
        assert_eq!(mm.snap_fraction, -0.5);
        assert_eq!(mm.gap, None);
    }

    #[wasm_bindgen_test]
    fn gesture_gap_uses_live_matrix_then_one_millimeter_and_respects_toggles() {
        let mut settings = LayoutSnapSettings {
            gap_override: "not a number".to_owned(),
            ..LayoutSnapSettings::default()
        };
        assert_eq!(
            gesture_snap_inputs(&settings, None, Some(1.4)).gap,
            Some(1.4)
        );
        assert_eq!(gesture_snap_inputs(&settings, None, None).gap, Some(1.0));

        settings.geometry_snap = false;
        assert_eq!(gesture_snap_inputs(&settings, None, Some(1.4)).gap, None);
        settings.geometry_snap = true;
        settings.gap_snap = false;
        assert_eq!(gesture_snap_inputs(&settings, None, Some(1.4)).gap, None);
    }

    #[wasm_bindgen_test]
    fn snap_custom_input_accepts_pitch_and_millimetre_amounts() {
        assert_eq!(parse_snap_increment("0.5u"), Some(0.5));
        assert_eq!(parse_snap_increment("1u"), Some(1.0));
        assert_eq!(parse_snap_increment("0u"), Some(0.0));
        assert_eq!(parse_snap_increment("0.5"), Some(0.5));
        assert_eq!(parse_snap_increment("  ¼u "), Some(0.25));
        assert_eq!(parse_snap_increment("1/2u"), Some(0.5));
        assert_eq!(parse_snap_increment("2*0.25u"), Some(0.5));
        assert_eq!(parse_snap_increment("(1+2)/2u"), Some(1.5));
        assert_eq!(parse_snap_increment("0.5mm"), Some(-0.5));
        assert_eq!(parse_snap_increment("1 mm"), Some(-1.0));
        assert_eq!(parse_snap_increment(""), None);
        assert_eq!(parse_snap_increment("a lot"), None);
        assert_eq!(parse_snap_increment("-0.5u"), None);
        assert_eq!(parse_snap_increment("0mm"), None);
        assert_eq!(parse_snap_increment("1/0u"), None);
        assert_eq!(parse_snap_increment("0.5.2u"), None);
    }

    #[wasm_bindgen_test]
    fn snap_custom_input_round_trips_through_its_canonical_text() {
        assert_eq!(format_snap_increment(0.0), "0");
        assert_eq!(format_snap_increment(0.5), "0.5");
        assert_eq!(format_snap_increment(-0.5), "0.5mm");
        for fraction in [0.0, 0.125, 0.25, 0.5, 1.0, -1.0, -0.5, -0.1] {
            assert_eq!(
                parse_snap_increment(&format_snap_increment(fraction)),
                Some(fraction)
            );
        }
    }

    #[wasm_bindgen_test]
    fn invalid_pitch_axes_fall_back_independently_and_nonfinite_fraction_is_safe() {
        let settings = LayoutSnapSettings {
            snap_fraction: f64::NAN,
            ..LayoutSnapSettings::default()
        };
        let inputs = gesture_snap_inputs(
            &settings,
            Some(Vec2 {
                x: f64::INFINITY,
                y: 18.0,
            }),
            None,
        );
        assert_eq!(inputs.pitch, Vec2 { x: 19.05, y: 18.0 });
        assert_eq!(inputs.snap_fraction, 0.25);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutSelectionProjection {
    pub context: TreeContext,
    pub part_ids: Vec<String>,
}

fn retained_coordinates(
    current: &TreeContext,
    anchor: &TreeCellAnchor,
) -> Option<(String, u32, u32)> {
    let matches = match current {
        TreeContext::Matrix { matrix_id } => matrix_id == &anchor.matrix_id,
        TreeContext::Row { matrix_id, row } => matrix_id == &anchor.matrix_id && *row == anchor.row,
        TreeContext::Column { matrix_id, column } => {
            matrix_id == &anchor.matrix_id && *column == anchor.column
        }
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => matrix_id == &anchor.matrix_id && *row == anchor.row && *column == anchor.column,
        TreeContext::Component {
            matrix_id: Some(matrix_id),
            row: Some(row),
            column: Some(column),
            ..
        } => matrix_id == &anchor.matrix_id && *row == anchor.row && *column == anchor.column,
        _ => false,
    };
    matches.then(|| (anchor.matrix_id.clone(), anchor.row, anchor.column))
}

fn part_context(cell_context: TreeContext, part_ids: &[String]) -> Option<TreeContext> {
    match part_ids {
        [] => Some(cell_context),
        [part_id] => Some(TreeContext::Component {
            part_id: Some(part_id.clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        }),
        _ => None,
    }
}

/// Convert an existing matrix context to the current Select mode using fresh scene membership.
/// `None` means the current context cannot be converted; empty `part_ids` is valid for an empty cell.
pub fn context_for_selection_kind(
    model: &ReadModel,
    current: &TreeContext,
    kind: LayoutSelectionKind,
    retained_cell: Option<&TreeCellAnchor>,
) -> Option<LayoutSelectionProjection> {
    if kind == LayoutSelectionKind::Part
        && matches!(
            current,
            TreeContext::Component {
                part_id: Some(_),
                ..
            }
        )
    {
        let part_ids = tree::resolve_selection(model, current)?;
        if part_ids.len() > 1 {
            return None;
        }
        return Some(LayoutSelectionProjection {
            context: current.clone(),
            part_ids,
        });
    }

    let current_cell = match current {
        TreeContext::Matrix { matrix_id } => Some((matrix_id.clone(), 0, 0)),
        TreeContext::Row { matrix_id, row } => Some((matrix_id.clone(), *row, 0)),
        TreeContext::Column { matrix_id, column } => Some((matrix_id.clone(), 0, *column)),
        TreeContext::Key {
            matrix_id,
            row,
            column,
        } => Some((matrix_id.clone(), *row, *column)),
        TreeContext::Component {
            matrix_id: Some(matrix_id),
            row: Some(row),
            column: Some(column),
            ..
        } => Some((matrix_id.clone(), *row, *column)),
        TreeContext::Component {
            part_id: Some(part_id),
            matrix_id: None,
            ..
        } => {
            // After Part is selected, the semantic context is a real component. Reuse the
            // retained cell only when the live primary member still matches that component.
            let anchor = retained_cell?;
            let key = tree::context_for_cell(model, &anchor.matrix_id, anchor.row, anchor.column)?;
            let members = tree::resolve_selection(model, &key)?;
            (members.len() == 1 && members[0] == *part_id).then_some((
                anchor.matrix_id.clone(),
                anchor.row,
                anchor.column,
            ))
        }
        TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. }
        | TreeContext::MountedModule { .. }
        | TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. }
        | TreeContext::Component { .. } => None,
    }?;

    let (matrix_id, base_row, base_column) = current_cell;
    let (row, column) = retained_cell
        .and_then(|anchor| retained_coordinates(current, anchor))
        .filter(|(anchor_matrix, _, _)| anchor_matrix == &matrix_id)
        .map(|(_, row, column)| (row, column))
        .unwrap_or((base_row, base_column));
    let cell_context = tree::context_for_cell(model, &matrix_id, row, column)?;
    let context = match kind {
        LayoutSelectionKind::Matrix => TreeContext::Matrix {
            matrix_id: matrix_id.clone(),
        },
        LayoutSelectionKind::Row => TreeContext::Row {
            matrix_id: matrix_id.clone(),
            row,
        },
        LayoutSelectionKind::Column => TreeContext::Column {
            matrix_id: matrix_id.clone(),
            column,
        },
        LayoutSelectionKind::Key => cell_context.clone(),
        LayoutSelectionKind::Part => {
            let part_ids = tree::resolve_selection(model, &cell_context)?;
            // Empty/disabled cell is still a semantic cell context, with no invented ID.
            part_context(cell_context.clone(), &part_ids)?
        }
    };
    let part_ids = tree::resolve_selection(model, &context)?;
    if kind == LayoutSelectionKind::Part && part_ids.len() > 1 {
        return None;
    }
    Some(LayoutSelectionProjection { context, part_ids })
}

const SNAP_STEPS: [(f64, &str, &str); 8] = [
    (0.0, "0", "Off"),
    (0.125, "0.125", "⅛u"),
    (0.25, "0.25", "¼u"),
    (0.5, "0.5", "½u"),
    (1.0, "1", "1u"),
    (-1.0, "-1", "1 mm"),
    (-0.5, "-0.5", "0.5 mm"),
    (-0.1, "-0.1", "0.1 mm"),
];

pub fn snap_label(fraction: f64) -> &'static str {
    SNAP_STEPS
        .iter()
        .find(|(value, _, _)| *value == fraction)
        .map(|(_, _, label)| *label)
        .unwrap_or("Custom")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutCommandMenu {
    Select,
    Transform,
    Align,
    Snap,
    View,
}

impl LayoutCommandMenu {
    const fn key(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Transform => "transform",
            Self::Align => "align",
            Self::Snap => "snap",
            Self::View => "view",
        }
    }

    const fn trigger_id(self) -> &'static str {
        match self {
            Self::Select => "m1-layout-select-trigger",
            Self::Transform => "m1-layout-transform-trigger",
            Self::Align => "m1-layout-align-trigger",
            Self::Snap => "m1-layout-snap-trigger",
            Self::View => "m1-layout-view-trigger",
        }
    }
}

type MenuOutsideListener = Rc<RefCell<Option<(Document, Closure<dyn FnMut(WebPointerEvent)>)>>>;

fn remove_menu_outside_listener(listener: &MenuOutsideListener) {
    if let Some((document, callback)) = listener.borrow_mut().take() {
        let _ = document
            .remove_event_listener_with_callback("pointerdown", callback.as_ref().unchecked_ref());
    }
}

pub fn close_layout_command_menu(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
) {
    let menu = open_menu().unwrap_or(menu);
    open_menu.set(None);
    restore_layout_command_focus(menu);
}

#[component]
pub fn LayoutCommandMenuHeader(
    label: String,
    close_label: String,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
) -> Element {
    let trigger_label = close_label.clone();
    rsx! {
        div { class: "layout-popup-heading",
            strong { "{label}" }
            Tooltip {
                TooltipTrigger { r#as: move |attributes: Vec<Attribute>| {
                    let attributes = merge_attributes(vec![attributes, attributes!(button {
                        r#type: "button", class: "layout-popup-close", aria_label: trigger_label.clone(),
                        onclick: move |_| close_layout_command_menu(open_menu, menu),
                    })]);
                    rsx! { button { ..attributes,
                        span { aria_hidden: true, "×" }
                    } }
                } }
                TooltipContent { "{close_label}" }
            }
        }
    }
}

fn restore_layout_command_focus(menu: LayoutCommandMenu) {
    if let Some(trigger) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| {
            if menu == LayoutCommandMenu::Select {
                document
                    .query_selector(".layout-scope-trigger[aria-pressed='true']")
                    .ok()
                    .flatten()
                    .or_else(|| document.get_element_by_id(menu.trigger_id()))
            } else {
                document.get_element_by_id(menu.trigger_id())
            }
        })
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = trigger.focus();
    }
}

fn focus_layout_popup(id: &str) {
    if let Some(content) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(id))
        && content.get_attribute("data-state").as_deref() == Some("open")
        && !content.has_attribute("inert")
        && let Some(control) = content.query_selector("input:not(:disabled), [data-slot=select-trigger]:not(:disabled), [data-slot=toolbar-button]:not(:disabled)").ok().flatten()
            .or_else(|| content.query_selector("button:not(:disabled)").ok().flatten())
        && let Some(control) = control.dyn_ref::<HtmlElement>()
    { let _ = control.focus(); }
}

fn dismiss_layout_popup(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
    id: &str,
) {
    let restore = web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| {
            let active = document.active_element();
            active.is_none()
                || active == document.body().map(Into::into)
                || document.get_element_by_id(id).is_some_and(|content| {
                    content.contains(active.as_ref().map(|element| element.as_ref()))
                })
        });
    if restore {
        close_layout_command_menu(open_menu, menu);
    } else {
        open_menu.set(None);
    }
}

fn stop_layout_space_pan(event: KeyboardEvent) {
    if event.key().to_string() == " "
        && let Some(raw) = event.data().try_as_web_event()
        && raw.target().and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|target| target.id() == "m1-layout-snap-trigger"
                || (target.tag_name() == "BUTTON"
                    && target.closest("#m1-layout-command-pill").ok().flatten().is_some()
                    && target.matches(".layout-scope-trigger, .layout-action-control, .layout-geometry-control, .layout-popup-close, .workbench-tool-trigger, .workbench-action, .workbench-utility, .layout-menu-actions [data-slot=popover-trigger], .layout-menu-actions [data-slot=dropdown-menu-trigger], .layout-settings-popup [data-slot=button], .layout-settings-popup [data-slot=checkbox][aria-label='Geometry snap'], .layout-settings-popup [data-slot=checkbox][aria-label='Envelope gap']").unwrap_or(false)))
    {
        // Native control activation must not become the canvas's Space-pan shortcut.
        event.stop_propagation();
        raw.stop_propagation();
    }
}

#[component]
fn LayoutAccordionSection(
    menu: LayoutCommandMenu,
    index: usize,
    label: String,
    caption: Option<String>,
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    icon: Element,
    children: Element,
) -> Element {
    let expanded = open_menu() == Some(menu);
    let trigger_id = match menu {
        LayoutCommandMenu::Align => "m1-layout-align-section-trigger",
        LayoutCommandMenu::Snap => "m1-layout-snap-section-trigger",
        _ => menu.trigger_id(),
    };
    rsx! {
        div { class: "workbench-tool-group", "data-workbench-section": menu.key(), "data-open": expanded,
            ToolbarButton { index, id: trigger_id, class: "workbench-tool-trigger",
                "data-workbench-section-trigger": menu.key(),
                aria_label: label.clone(), aria_expanded: expanded,
                aria_controls: format!("workbench-section-{}", menu.key()),
                on_click: move |_| open_menu.set(if expanded { None } else { Some(menu) }),
                span { class: "workbench-tool-icon", "aria-hidden": "true", {icon} }
                span { class: "workbench-tool-heading",
                    span { class: "workbench-tool-name", "{label}" }
                    if let Some(caption) = caption { span { class: "workbench-tool-caption", "{caption}" } }
                }
                ChevronRight { class: "workbench-tool-chevron" }
            }
            div { class: "workbench-tool-reveal", id: format!("workbench-section-{}", menu.key()),
                inert: (!expanded).then_some(""), "aria-hidden": (!expanded).then_some("true"),
                div { class: "workbench-tool-reveal-inner",
                    div { class: "workbench-tool-track", {children} }
                }
            }
        }
    }
}

#[component]
pub fn LayoutCommandPill(
    command_label: String,
    menu_owner_key: String,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    snap_settings: LayoutSnapSettings,
    transform: super::layout_transform_toolbar::LayoutTransformMenuMount,
    align: super::layout_align::LayoutAlignMount,
    show_relationships: bool,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    has_selection_context: bool,
    on_show_relationships: EventHandler<()>,
    #[props(default)] snap_in_dock: bool,
    #[props(default)] accordion: bool,
    #[props(default)] view: Option<Element>,
    #[props(default)] utilities: Option<Element>,
) -> Element {
    let pointer_tools_visible = transform.pointer_tools_visible;
    let mut transform = transform;
    // Part's existing Component Inspector is a valid properties destination even
    // without a matrix-transform projection or matrix pointer tools.
    if pointer_tools_visible && selection_kind == LayoutSelectionKind::Part && has_selection_context
    {
        transform.properties_available = true;
    }
    let has_inline_actions = has_selection_context
        && (transform.properties_available || transform.pointer_tools_available);
    // The inline strip owns contextual actions. Transform keeps Origin, which has no
    // inline control; the non-Layout menu keeps its existing full set of actions.
    let mut menu_transform = transform.clone();
    if pointer_tools_visible {
        menu_transform.pointer_tools_visible = false;
    }
    let geometry_snap_enabled = snap_settings.geometry_snap;
    let workspace = try_use_context::<boardstudio_web_ui_model::state::WorkspaceState>();
    let mut commands = Vec::new();
    if workspace.is_some_and(|w| w.0() == "Layout") {
        use boardstudio_web_ui_shared::ui::command::Command;
        for kind in LayoutSelectionKind::ALL {
            commands.push(Command {
                group: "Current Workspace",
                label: format!("Select {}", kind.label()),
                reason: None,
                hint: None,
                on_select: EventHandler::new(move |()| {
                    on_selection_kind.call(kind);
                    let mut menu = open_menu;
                    menu.set(None);
                }),
            });
        }
        for (label, menu) in [
            ("Snap settings", LayoutCommandMenu::Snap),
            ("Align", LayoutCommandMenu::Align),
            ("Transform", LayoutCommandMenu::Transform),
        ] {
            commands.push(Command {
                group: "Current Workspace",
                label: label.into(),
                reason: None,
                hint: None,
                on_select: EventHandler::new(move |()| {
                    let mut open = open_menu;
                    open.set(Some(menu));
                    if menu == LayoutCommandMenu::Snap {
                        restore_layout_command_focus(menu);
                    }
                }),
            });
        }
        commands.push(Command {
            group: "Current Workspace",
            label: "Geometry snap".into(),
            reason: None,
            hint: None,
            on_select: EventHandler::new(move |()| {
                on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap_enabled))
            }),
        });
        let properties = transform.on_show_properties;
        commands.push(Command { group: "Current Workspace", label: "Position & rotation".into(),
            reason: (!transform.properties_available).then(|| "Select a matrix, row, column, key or part to open its current transform properties.".into()), hint: None, on_select: properties });
        for (tool, label, available) in [
            (
                super::layout_transform_toolbar::LayoutTransformTool::Stagger,
                "Stagger column",
                transform.column_available,
            ),
            (
                super::layout_transform_toolbar::LayoutTransformTool::Offset,
                "Offset",
                transform.column_available || transform.row_available,
            ),
            (
                super::layout_transform_toolbar::LayoutTransformTool::Splay,
                "Splay",
                transform.column_available,
            ),
            (
                super::layout_transform_toolbar::LayoutTransformTool::Origin,
                "Origin",
                transform.column_available,
            ),
        ] {
            let callback = transform.on_transform_tool;
            commands.push(Command { group: "Current Workspace", label: label.into(),
                reason: (!(transform.pointer_tools_available && available)).then(|| "Select a matrix, row, column, key or part to open its current transform properties.".into()), hint: None,
                on_select: EventHandler::new(move |()| callback.call(tool)) });
        }
    }
    boardstudio_web_ui_shared::ui::command::use_commands(commands);
    let outside_listener = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(Document, Closure<dyn FnMut(WebPointerEvent)>)>,
        ))
    });

    use_effect(use_reactive((&menu_owner_key,), move |_| {
        open_menu.set(None);
    }));
    use_effect(use_reactive((&open_menu(),), {
        let outside_listener = outside_listener.clone();
        move |(active_menu,)| {
            remove_menu_outside_listener(&outside_listener);
            if active_menu.is_none() {
                return;
            }
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let listener_document = document.clone();
            let mut open_menu = open_menu;
            let callback = Closure::wrap(Box::new(move |event: WebPointerEvent| {
                let target = event
                    .target()
                    .and_then(|target| target.dyn_into::<Node>().ok());
                let inside = target.as_ref().is_some_and(|target| {
                    listener_document
                        .get_element_by_id("m1-layout-command-pill")
                        .is_some_and(|pill| pill.contains(Some(target)))
                        || listener_document
                            .get_element_by_id("m1-layout-snap-trigger")
                            .and_then(|trigger| {
                                trigger.closest("[data-slot='collapsible']").ok().flatten()
                            })
                            .is_some_and(|dock| dock.contains(Some(target)))
                });
                if !inside {
                    open_menu.set(None);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("pointerdown", callback.as_ref().unchecked_ref());
            *outside_listener.borrow_mut() = Some((document, callback));
        }
    }));
    use_drop({
        let outside_listener = outside_listener.clone();
        move || remove_menu_outside_listener(&outside_listener)
    });

    if accordion {
        let transform_caption = transform.active_tool.map(|tool| {
            match tool {
                super::layout_transform_toolbar::LayoutTransformTool::Stagger => "Stagger",
                super::layout_transform_toolbar::LayoutTransformTool::Spread => "Spread",
                super::layout_transform_toolbar::LayoutTransformTool::Offset => "Offset",
                super::layout_transform_toolbar::LayoutTransformTool::Shift => "Shift",
                super::layout_transform_toolbar::LayoutTransformTool::Bend => "Bend",
                super::layout_transform_toolbar::LayoutTransformTool::Splay => "Splay",
                super::layout_transform_toolbar::LayoutTransformTool::Origin => "Origin",
                super::layout_transform_toolbar::LayoutTransformTool::Move => "Move",
                super::layout_transform_toolbar::LayoutTransformTool::Rotate => "Rotate",
                super::layout_transform_toolbar::LayoutTransformTool::Anchor => "Anchor",
            }
            .to_owned()
        });
        return rsx! {
            div { id: "m1-layout-command-pill", class: "layout-toolbar-strips layout-toolbar-accordion", role: "group", aria_label: "{command_label}", onkeydown: stop_layout_space_pan,
                LayoutAccordionSection { menu: LayoutCommandMenu::Select, index: 0usize, label: "Select".to_owned(), caption: Some(selection_kind.label().to_owned()), open_menu,
                    icon: rsx! { LayoutSelectionIcon { kind: selection_kind } },
                    LayoutSelectionToolbar { open_menu, selection_kind, on_selection_kind, bare: true, close_on_select: false }
                }
                LayoutAccordionSection { menu: LayoutCommandMenu::Transform, index: 10usize, label: "Transform".to_owned(), caption: transform_caption, open_menu,
                    icon: rsx! { svg { view_box: "0 0 20 20", path { d: "M16 7a7 7 0 1 0 1 7M16 3v4h-4" } } },
                    if pointer_tools_visible && has_inline_actions {
                        super::layout_transform_toolbar::LayoutContextActionToolbar {
                            mount: transform.clone(), open_menu, selection_kind, bare: true, first_index: 11usize, include_origin: true, close_on_action: false,
                        }
                    } else if pointer_tools_visible {
                        {
                            let guidance_text = format!("Select a {} on the canvas to edit it.", selection_kind.label().to_lowercase());
                            let guidance_trigger = guidance_text.clone();
                            let guidance_content = guidance_text;
                            rsx! {
                                Tooltip {
                                    TooltipTrigger { r#as: move |mut attrs: Vec<Attribute>| {
                                        // Preserve roving: Toolbar sync looks for data-slot toolbar-button,
                                        // while tooltip's data-slot would overwrite it (later wins). Filter it,
                                        // keeping tabindex/handlers/aria for tooltip function.
                                        attrs.retain(|a| a.name != "data-slot");
                                        let label = guidance_trigger.clone();
                                        rsx! {
                                            ToolbarButton {
                                                index: 11usize,
                                                class: "layout-selection-guidance",
                                                aria_label: label,
                                                attributes: attrs,
                                                span { role: "status", class: "layout-selection-guidance-text", "{guidance_trigger}" }
                                            }
                                        }
                                    } }
                                    TooltipContent { "{guidance_content}" }
                                }
                            }
                        }
                    } else {
                        super::layout_transform_toolbar::LayoutTransformToolbar { open_menu, mount: menu_transform.clone(), selection_kind: None }
                    }
                }
                LayoutAccordionSection { menu: LayoutCommandMenu::Align, index: 20usize, label: "Align".to_owned(), caption: None, open_menu,
                    icon: rsx! { svg { view_box: "0 0 20 20", path { d: "M3 2v16M6 4h10v4H6zM6 12h6v4H6z" } } },
                    super::layout_align::LayoutAlignToolbar { mount: align, open_menu, show_relationships, has_selection_context, on_show_relationships, inline_actions: true, first_index: 21usize }
                }
                LayoutAccordionSection { menu: LayoutCommandMenu::Snap, index: 30usize, label: "Snap".to_owned(), caption: Some(snap_label(snap_settings.snap_fraction).to_owned()), open_menu,
                    icon: rsx! { svg { view_box: "0 0 20 20", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } } },
                    LayoutSnapInline { settings: snap_settings, on_snap_intent, first_index: 31usize }
                }
                LayoutAccordionSection { menu: LayoutCommandMenu::View, index: 50usize, label: "View".to_owned(), caption: None, open_menu,
                    icon: rsx! { svg { view_box: "0 0 20 20", path { d: "M1 10s3-6 9-6 9 6 9 6-3 6-9 6-9-6-9-6M13 10a3 3 0 1 1-6 0 3 3 0 0 1 6 0" } } },
                    if let Some(view) = view { {view} }
                }
                if let Some(utilities) = utilities { {utilities} }
            }
        };
    }

    rsx! {
        div { id: "m1-layout-command-pill", class: "layout-toolbar-strips", role: "group", aria_label: "{command_label}", onkeydown: stop_layout_space_pan,
            div { class: "layout-command-strip",
                div { class: "layout-scope-strip",
                    if pointer_tools_visible {
                        LayoutSelectionToolbar { open_menu, selection_kind, on_selection_kind }
                    } else {
                        LayoutSelectMenu { open_menu, selection_kind, on_selection_kind }
                    }
                }
                div { class: "layout-menu-actions", role: "group", aria_label: "Selection actions",
                    super::layout_align::LayoutAlignToolbar {
                        mount: align, open_menu, show_relationships, has_selection_context, on_show_relationships,
                    }
                    super::layout_transform_toolbar::LayoutTransformToolbar {
                        open_menu, mount: menu_transform,
                        selection_kind: pointer_tools_visible.then_some(selection_kind),
                    }
                    if !snap_in_dock {
                        LayoutSnapMenu { open_menu, settings: snap_settings, on_snap_intent, show_layout_state: pointer_tools_visible }
                        if pointer_tools_visible {
                            Tooltip {
                                TooltipTrigger { r#as: move |attributes: Vec<Attribute>| {
                                    let attributes = merge_attributes(vec![attributes, attributes!(button {
                                        r#type: "button",
                                        class: "layout-geometry-control",
                                        aria_label: "Geometry snap", aria_pressed: geometry_snap_enabled,
                                        onclick: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap_enabled)),
                                    })]);
                                    rsx! { button { ..attributes,
                                        svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                            path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" }
                                        }
                                    } }
                                } }
                                TooltipContent { "Geometry snap" }
                            }
                        }
                    }
                }
            }
            if pointer_tools_visible {
                div { class: "layout-context-strip",
                    div { class: "layout-action-slot",
                        if has_inline_actions {
                            super::layout_transform_toolbar::LayoutContextActionToolbar {
                                mount: transform, open_menu, selection_kind,
                            }
                        } else {
                            p { class: "layout-selection-guidance", role: "status",
                                "Select a {selection_kind.label().to_lowercase()} on the canvas to edit it."
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Direct scope choices reuse the same retained-cell selection adapter as the menu.
#[component]
fn LayoutSelectionToolbar(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
    #[props(default)] bare: bool,
    #[props(default = true)] close_on_select: bool,
) -> Element {
    let choices = rsx! {
            ToggleGroup {
                aria_label: "Selection scope choices", horizontal: true,
                pressed: Some(std::collections::HashSet::from([LayoutSelectionKind::ALL.iter().position(|kind| *kind == selection_kind).unwrap()])),
                for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                    ToggleItem {
                        index, class: "layout-scope-trigger",
                        id: format!("m1-layout-scope-{}", kind.label()),
                        "data-workbench-action": "",
                        aria_label: format!("Select {}", kind.label()),
                        attributes: vec![onclick(move |_| {
                            on_selection_kind.call(kind);
                            if close_on_select { open_menu.set(None); }
                        })],
                        LayoutSelectionIcon { kind }
                        span { class: "workbench-action-label", "{kind.label()}" }
                    }
                }
            }
    };
    if bare {
        choices
    } else {
        rsx! { Toolbar { class: "layout-scope-toolbar", aria_label: "Selection scope", {choices} } }
    }
}

#[component]
fn LayoutSelectionIcon(kind: LayoutSelectionKind) -> Element {
    let (x, y, width, height, radius, path) = match kind {
        LayoutSelectionKind::Matrix => {
            ("3", "3", "14", "14", "1", "M3 8h14M3 12h14M8 3v14M12 3v14")
        }
        LayoutSelectionKind::Row => ("2", "7", "16", "6", "1", "M7 7v6M13 7v6"),
        LayoutSelectionKind::Column => ("7", "2", "6", "16", "1", "M7 7h6M7 13h6"),
        LayoutSelectionKind::Key => ("3", "3", "14", "14", "3", "M6 13h8"),
        LayoutSelectionKind::Part => (
            "6",
            "6",
            "8",
            "8",
            "1",
            "M7 2v4M13 2v4M7 14v4M13 14v4M2 7h4M2 13h4M14 7h4M14 13h4",
        ),
    };
    rsx! {
        svg { class: "block size-[22px]! shrink-0 fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
            rect { x, y, width, height, rx: radius }
            path { d: path }
        }
    }
}

/// Always-visible snapping preferences; the existing intent owner remains authoritative.
/// The Canvas handles checkbox below them flips workspace view state only: hiding
/// handles never submits an edit and never touches document or history.
#[component]
pub fn LayoutSnapDock(
    settings: LayoutSnapSettings,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    #[props(default = true)] handles_visible: bool,
    #[props(default)] on_toggle_handles: EventHandler<bool>,
) -> Element {
    let mut snap_open = use_signal(|| false);
    rsx! {
        Collapsible {
            class: "relative left-0 w-full min-w-0 flex-none border-t border-border bg-card p-0",
            open: Some(snap_open()), on_open_change: move |open| snap_open.set(open),
            CollapsibleTrigger {
                id: "m1-layout-snap-trigger", aria_label: "Snap",
                class: "flex w-full min-h-control-touch items-center justify-start gap-2 rounded-none border-0 bg-transparent px-3 py-2.5 text-left text-sm font-medium text-foreground hover:bg-accent",
                attributes: vec![dioxus::events::onkeydown(move |event: KeyboardEvent| {
                    // Space must activate the button, not the Layout window's pan shortcut.
                    if event.key().to_string() == " " {
                        event.stop_propagation();
                        if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                    }
                })],
                svg { class: "size-[18px] fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                span { "Snap" } small { class: "ml-auto text-muted-foreground", "{snap_label(settings.snap_fraction)}" }
                ChevronDown { class: "shrink-0" }
            }
            CollapsibleContent {
                LayoutSnapOptions { settings, on_snap_intent }
                div { class: "flex items-center gap-2 px-3 py-2 text-sm",
                    Checkbox {
                        aria_label: "Canvas handles",
                        checked: Some(if handles_visible { dioxus_primitives::checkbox::CheckboxState::Checked } else { dioxus_primitives::checkbox::CheckboxState::Unchecked }),
                        on_checked_change: move |value: dioxus_primitives::checkbox::CheckboxState| on_toggle_handles.call(value.into()),
                    }
                    span { "Canvas handles" }
                }
            }
        }
    }
}

#[component]
fn LayoutSelectMenu(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
) -> Element {
    rsx! {
        DropdownMenu {
            open: Some(open_menu() == Some(LayoutCommandMenu::Select)),
            on_open_change: move |next| {
                if next { open_menu.set(Some(LayoutCommandMenu::Select)); }
                else if open_menu() == Some(LayoutCommandMenu::Select) { open_menu.set(None); }
            },
            DropdownMenuTrigger { id: "m1-layout-select-trigger", aria_label: format!("Select: {}", selection_kind.label()),
                LayoutSelectionIcon { kind: selection_kind } span { "{selection_kind.label()}" }
            }
            DropdownMenuContent { id: "m1-layout-select-menu",
                for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                    DropdownMenuItem { index, value: kind, role: "menuitemradio", aria_checked: kind == selection_kind,
                        on_select: move |_: LayoutSelectionKind| { on_selection_kind.call(kind); close_layout_command_menu(open_menu, LayoutCommandMenu::Select); },
                        "{kind.label()}"
                    }
                }
            }
        }
    }
}

#[component]
fn LayoutSnapMenu(
    open_menu: Signal<Option<LayoutCommandMenu>>,
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    show_layout_state: bool,
    #[props(default)] detail: bool,
) -> Element {
    let trigger = if detail {
        rsx! { span { "Settings" } }
    } else {
        rsx! { span { "Snap" } if show_layout_state { small { "{snap_label(settings.snap_fraction)}" } } }
    };
    rsx! {
        LayoutCommandPopup {
            open_menu, menu: LayoutCommandMenu::Snap, label: "Snap settings",
            trigger,
            LayoutSnapOptions { settings, on_snap_intent }
        }
    }
}

#[component]
fn LayoutSnapIncrementSelect(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let fraction = use_memo(use_reactive((&settings.snap_fraction,), |(value,)| {
        Some(value.to_string())
    }));
    rsx! {
        Select::<String> {
            aria_label: "Snap increment", value: Some(fraction.into()),
            on_value_change: move |value: Option<String>| {
                if let Some(value) = value.and_then(|value| value.parse::<f64>().ok())
                    && value.is_finite() && SNAP_STEPS.iter().any(|(step, _, _)| *step == value)
                { on_snap_intent.call(LayoutSnapIntent::Fraction(value)); }
            },
            for (index, (_, value, label)) in SNAP_STEPS.into_iter().enumerate() {
                SelectOption::<String> { index, value: value.to_owned(), text_value: label, "{label}" }
            }
        }
    }
}

#[component]
fn LayoutSnapCustomInput(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let mut text = use_signal(|| format_snap_increment(settings.snap_fraction));
    let mut focused = use_signal(|| false);
    let mut invalid = use_signal(|| false);
    let mut sync_text = text;
    let mut sync_invalid = invalid;
    use_effect(use_reactive((&settings.snap_fraction,), move |(fraction,)| {
        if !focused() {
            // Compare before setting: an unconditional set would re-render and re-fire this effect.
            let canonical = format_snap_increment(fraction);
            if sync_text() != canonical {
                sync_text.set(canonical);
            }
            if sync_invalid() {
                sync_invalid.set(false);
            }
        }
    }));
    rsx! {
        input {
            r#type: "text",
            class: "layout-snap-custom",
            aria_label: "Custom snap increment",
            title: "Type a size such as 0.5u or 0.5mm, Enter to apply",
            spellcheck: false,
            autocomplete: "off",
            value: text(),
            aria_invalid: invalid(),
            onfocus: move |_| focused.set(true),
            onblur: move |_| focused.set(false),
            oninput: move |event: FormEvent| text.set(event.value()),
            onkeydown: move |event: KeyboardEvent| {
                // Typing must not drive canvas shortcuts or toolbar keys.
                event.stop_propagation();
                if let Some(raw) = event.data().try_as_web_event() {
                    raw.stop_propagation();
                }
                match event.key() {
                    Key::Enter => {
                        match parse_snap_increment(&text()) {
                            Some(value) => {
                                invalid.set(false);
                                text.set(format_snap_increment(value));
                                on_snap_intent.call(LayoutSnapIntent::Fraction(value));
                            }
                            None => invalid.set(true),
                        }
                    }
                    Key::Escape => {
                        text.set(format_snap_increment(settings.snap_fraction));
                        invalid.set(false);
                    }
                    _ => {}
                }
            },
        }
    }
}

/// Parse an inline snap increment such as `0.5u`, `1/2u` or `0.5mm` into the
/// stored [`LayoutSnapSettings::snap_fraction`] encoding: non-negative matrix-pitch
/// fractions stay positive, millimetre steps are stored negated.
fn parse_snap_increment(raw: &str) -> Option<f64> {
    let text = raw.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    let (amount, millimetres) = if let Some(amount) = text.strip_suffix("mm") {
        (amount, true)
    } else if let Some(amount) = text.strip_suffix('u') {
        (amount, false)
    } else {
        (text.as_str(), false)
    };
    let value = eval_snap_amount(amount.trim())?;
    if !value.is_finite() {
        return None;
    }
    if millimetres {
        (value > 0.0).then_some(-value)
    } else {
        (value >= 0.0).then_some(value)
    }
}

/// Canonical text for the custom input: plain pitch fractions, millimetres with suffix.
fn format_snap_increment(fraction: f64) -> String {
    if fraction == 0.0 {
        "0".to_owned()
    } else if fraction > 0.0 {
        fraction.to_string()
    } else {
        format!("{}mm", -fraction)
    }
}

/// Basic arithmetic for inline sizes: numbers, `+ - * /` and parentheses.
fn eval_snap_amount(expression: &str) -> Option<f64> {
    let normalized = expression
        .replace('⅛', "(1/8)")
        .replace('¼', "(1/4)")
        .replace('½', "(1/2)")
        .replace('¾', "(3/4)");
    let mut parser = SnapAmountParser {
        bytes: normalized.as_bytes(),
        pos: 0,
    };
    let value = parser.parse_sum()?;
    parser.skip_ws();
    (parser.pos == parser.bytes.len()).then_some(value)
}

struct SnapAmountParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl SnapAmountParser<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_sum(&mut self) -> Option<f64> {
        let mut value = self.parse_product()?;
        loop {
            self.skip_ws();
            let add = match self.bytes.get(self.pos) {
                Some(b'+') => true,
                Some(b'-') => false,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_product()?;
            value = if add { value + rhs } else { value - rhs };
        }
        Some(value)
    }

    fn parse_product(&mut self) -> Option<f64> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_ws();
            let multiply = match self.bytes.get(self.pos) {
                Some(b'*') => true,
                Some(b'/') => false,
                _ => break,
            };
            self.pos += 1;
            let rhs = self.parse_factor()?;
            value = if multiply { value * rhs } else { value / rhs };
        }
        Some(value)
    }

    fn parse_factor(&mut self) -> Option<f64> {
        self.skip_ws();
        match self.bytes.get(self.pos) {
            Some(b'(') => {
                self.pos += 1;
                let value = self.parse_sum()?;
                self.skip_ws();
                if self.bytes.get(self.pos) == Some(&b')') {
                    self.pos += 1;
                    Some(value)
                } else {
                    None
                }
            }
            Some(b'-') => {
                self.pos += 1;
                Some(-self.parse_factor()?)
            }
            Some(b'+') => {
                self.pos += 1;
                self.parse_factor()
            }
            Some(next) if next.is_ascii_digit() || *next == b'.' => {
                let start = self.pos;
                while self.pos < self.bytes.len()
                    && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
                {
                    self.pos += 1;
                }
                std::str::from_utf8(&self.bytes[start..self.pos])
                    .ok()?
                    .parse::<f64>()
                    .ok()
            }
            _ => None,
        }
    }
}

#[component]
fn LayoutSnapInline(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    first_index: usize,
) -> Element {
    rsx! {
        for (offset, (value, _, label)) in SNAP_STEPS.into_iter().enumerate() {
            ToolbarButton { index: first_index + offset, class: "layout-action-control workbench-action", "data-workbench-action": "",
                aria_label: format!("Snap increment {label}"), aria_pressed: settings.snap_fraction == value,
                on_click: move |_| on_snap_intent.call(LayoutSnapIntent::Fraction(value)),
                span { class: "workbench-action-label", "{label}" }
            }
        }
        LayoutSnapCustomInput { settings: settings.clone(), on_snap_intent }
        ToolbarButton { index: first_index + SNAP_STEPS.len(), class: "layout-action-control workbench-action", "data-workbench-action": "",
            aria_label: "Geometry snap", aria_pressed: settings.geometry_snap,
            on_click: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!settings.geometry_snap)),
            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
            span { class: "workbench-action-label", "Geometry snap" }
        }
        ToolbarButton { index: first_index + SNAP_STEPS.len() + 1, class: "layout-action-control workbench-action", "data-workbench-action": "",
            aria_label: "Envelope gap", aria_pressed: settings.gap_snap, disabled: !settings.geometry_snap,
            on_click: move |_| on_snap_intent.call(LayoutSnapIntent::GapSnap(!settings.gap_snap)),
            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M3 4v12M17 4v12M6 10h8M8 8l-2 2 2 2M12 8l2 2-2 2" } }
            span { class: "workbench-action-label", "Envelope gap" }
        }
        Label { html_for: "layout-snap-gap-inline", class: "layout-snap-gap-label", "Gap (mm)" }
        InputGroup { class: "layout-snap-gap-group",
            InputGroupInput { id: "layout-snap-gap-inline", r#type: InputType::Number, min: "0", step: "any", aria_label: "Snap gap",
                value: settings.gap_override.clone(),
                oninput: move |event: FormEvent| on_snap_intent.call(LayoutSnapIntent::GapOverride(event.value())),
            }
        }
    }
}

/// Free-form settings use Popover; action-only menus use DropdownMenu.
#[component]
pub(super) fn LayoutCommandPopup(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
    label: String,
    trigger: Element,
    children: Element,
) -> Element {
    let close_label = if menu == LayoutCommandMenu::Snap {
        "Close Snap".to_owned()
    } else {
        format!("Close {label}")
    };
    use_effect(move || {
        if open_menu() == Some(menu) {
            focus_layout_popup(&format!("m1-layout-{}-menu", menu.key()));
        }
    });
    rsx! {
        Popover {
            open: open_menu() == Some(menu), is_modal: false,
            on_open_change: move |next| {
                if next { open_menu.set(Some(menu)); }
                else if open_menu() == Some(menu) { dismiss_layout_popup(open_menu, menu, &format!("m1-layout-{}-menu", menu.key())); }
            },
            PopoverTrigger { id: menu.trigger_id(), aria_label: label.clone(), aria_haspopup: "dialog",
                aria_expanded: open_menu() == Some(menu),
                aria_controls: (open_menu() == Some(menu)).then(|| format!("m1-layout-{}-menu", menu.key())),
                {trigger}
            }
            PopoverContent { id: format!("m1-layout-{}-menu", menu.key()), class: "layout-settings-popup", role: "dialog", aria_label: label.clone(),
                inert: (open_menu() != Some(menu)).then_some(""),
                attributes: vec![onmounted(move |_| {
                    if open_menu() == Some(menu) { focus_layout_popup(&format!("m1-layout-{}-menu", menu.key())); }
                })],
                LayoutCommandMenuHeader { label, close_label, open_menu, menu }
                {children}
            }
        }
    }
}

#[component]
fn LayoutSnapOptions(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let geometry_id = format!(
        "m1-layout-snap-geometry-{}",
        dioxus::core::current_scope_id().0
    );
    let envelope_id = format!(
        "m1-layout-snap-envelope-gap-{}",
        dioxus::core::current_scope_id().0
    );
    rsx! {
        div { class: "layout-snap-fields",
            Label { "Snap increment" }
            LayoutSnapIncrementSelect { settings: settings.clone(), on_snap_intent }
            Label { html_for: geometry_id.clone(),
                Checkbox { id: geometry_id, aria_label: "Geometry snap", checked: Some(if settings.geometry_snap { dioxus_primitives::checkbox::CheckboxState::Checked } else { dioxus_primitives::checkbox::CheckboxState::Unchecked }),
                    on_checked_change: move |value: dioxus_primitives::checkbox::CheckboxState| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(value.into())) }
                "Geometry snap"
            }
            Label { html_for: envelope_id.clone(),
                Checkbox { id: envelope_id, aria_label: "Envelope gap", checked: Some(if settings.gap_snap { dioxus_primitives::checkbox::CheckboxState::Checked } else { dioxus_primitives::checkbox::CheckboxState::Unchecked }), disabled: !settings.geometry_snap,
                    on_checked_change: move |value: dioxus_primitives::checkbox::CheckboxState| on_snap_intent.call(LayoutSnapIntent::GapSnap(value.into())) }
                "Envelope gap"
            }
            Label { html_for: "layout-snap-gap", "Gap (mm)" }
            InputGroup {
                InputGroupInput { id: "layout-snap-gap", r#type: InputType::Number, min: "0", step: "any", aria_label: "Snap gap",
                    value: settings.gap_override,
                    oninput: move |event: FormEvent| on_snap_intent.call(LayoutSnapIntent::GapOverride(event.value())),
                }
            }
            Callout { "Shared by Layout, drawing and perimeter editing. Unit steps use matrix pitch; mm steps use world coordinates. Hold Alt to bypass snapping." }
        }
    }
}
