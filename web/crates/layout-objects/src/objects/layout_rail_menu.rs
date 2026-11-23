//! The contextual action toolbar and the Transform command menu.
use super::layout_rail::LayoutTransformTool;
use super::layout_selection_kind::LayoutSelectionKind;
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use boardstudio_web_ui_shared::ui::{
    callout::Callout,
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    kbd::Kbd,
    toolbar::{Toolbar, ToolbarButton},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

/// What the Transform menu and the contextual strip need from the workspace.
#[derive(Clone, PartialEq)]
pub struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub pointer_tools_visible: bool,
    pub pointer_tools_available: bool,
    pub active_tool: Option<LayoutTransformTool>,
    /// `None` when the selection is not a Matrix; otherwise whether the matrix follows rotation,
    /// which decides whether Rotate is offered enabled or disabled with an explanation.
    pub matrix_rotate_followed: Option<bool>,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
    pub on_transform_tool: EventHandler<LayoutTransformTool>,
}

/// One contextual pointer tool: the shared source data both the menu-item and the toolbar
/// presentations render, so their labels, names and order cannot drift.
struct ContextToolSpec {
    tool: LayoutTransformTool,
    label: &'static str,
    accessible_label: String,
    disabled: bool,
}

/// The Properties shortcut beside the tools, when the selection offers one.
struct ContextPropertySpec {
    label: &'static str,
    accessible_label: String,
}

fn context_selection_label(selection_kind: LayoutSelectionKind) -> &'static str {
    match selection_kind {
        LayoutSelectionKind::Matrix => "Matrix",
        LayoutSelectionKind::Row => "Row",
        LayoutSelectionKind::Column => "Column",
        LayoutSelectionKind::Key => "Key",
        LayoutSelectionKind::Part => "Part",
    }
}

fn context_property_spec(
    selection_kind: LayoutSelectionKind,
    selection_label: &str,
    mount: &LayoutTransformMenuMount,
) -> Option<ContextPropertySpec> {
    let offers = mount.pointer_tools_visible
        && mount.properties_available
        && (matches!(
            selection_kind,
            LayoutSelectionKind::Matrix
                | LayoutSelectionKind::Column
                | LayoutSelectionKind::Key
                | LayoutSelectionKind::Part
        ) || (selection_kind == LayoutSelectionKind::Row && mount.row_available));
    if !offers {
        return None;
    }
    if selection_kind == LayoutSelectionKind::Row {
        Some(ContextPropertySpec {
            label: "Row offsets",
            accessible_label: "Open Row offsets properties for Row".to_owned(),
        })
    } else {
        Some(ContextPropertySpec {
            label: "Position",
            accessible_label: format!(
                "Open Position and rotation properties for {selection_label}"
            ),
        })
    }
}

/// The Stagger/Offset/Spread column parameters every retargeting selection shares:
/// (tool, label). The accessible label derives as `Activate {label}; selection
/// changes to Column`, so the triple is written once.
const RETARGET_COLUMN_PARAMS: [(LayoutTransformTool, &str); 3] = [
    (LayoutTransformTool::Stagger, "Stagger column"),
    (LayoutTransformTool::Offset, "Offset column"),
    (LayoutTransformTool::Spread, "Spread column"),
];

/// The Column selection's own parameters: (tool, label, short name). The accessible
/// label derives as `Activate {short} for the selected column`.
const SELECTED_COLUMN_PARAMS: [(LayoutTransformTool, &str, &str); 3] = [
    (LayoutTransformTool::Stagger, "Stagger column", "Stagger"),
    (LayoutTransformTool::Offset, "Offset column", "Offset"),
    (LayoutTransformTool::Spread, "Spread column", "Spread"),
];

/// One retargeting column parameter: choosing it changes the selection to its Column.
fn retarget_spec(tool: LayoutTransformTool, label: &'static str) -> ContextToolSpec {
    ContextToolSpec {
        tool,
        label,
        accessible_label: format!("Activate {label}; selection changes to Column"),
        disabled: false,
    }
}

/// The visible pointer tools for one selection in canonical order, with Splay last.
/// Availability filters a row out; disabled keeps it visible but inert. Hidden or
/// unavailable pointer tools hide the whole strip, Splay included.
fn context_tool_specs(
    selection_kind: LayoutSelectionKind,
    mount: &LayoutTransformMenuMount,
    rotate_accessible_label: &str,
) -> Vec<ContextToolSpec> {
    if !(mount.pointer_tools_visible && mount.pointer_tools_available) {
        return Vec::new();
    }
    let mut specs = Vec::new();
    if selection_kind == LayoutSelectionKind::Matrix {
        specs.push(ContextToolSpec {
            tool: LayoutTransformTool::Move,
            label: "Move matrix",
            accessible_label: "Activate Move for the selected matrix".to_owned(),
            disabled: false,
        });
        specs.push(ContextToolSpec {
            tool: LayoutTransformTool::Rotate,
            label: "Rotate matrix",
            accessible_label: rotate_accessible_label.to_owned(),
            disabled: !mount.matrix_rotate_followed.unwrap_or(true),
        });
    }
    if selection_kind == LayoutSelectionKind::Row && mount.row_available {
        specs.push(ContextToolSpec {
            tool: LayoutTransformTool::Offset,
            label: "Offset row",
            accessible_label: "Activate Offset for the selected row".to_owned(),
            disabled: false,
        });
    }
    if mount.column_available {
        match selection_kind {
            LayoutSelectionKind::Column => {
                specs.extend(SELECTED_COLUMN_PARAMS.iter().map(|(tool, label, short)| {
                    ContextToolSpec {
                        tool: *tool,
                        label,
                        accessible_label: format!("Activate {short} for the selected column"),
                        disabled: false,
                    }
                }))
            }
            LayoutSelectionKind::Matrix | LayoutSelectionKind::Key | LayoutSelectionKind::Part => {
                specs.extend(
                    RETARGET_COLUMN_PARAMS
                        .iter()
                        .map(|(tool, label)| retarget_spec(*tool, label)),
                );
            }
            // A Row keeps its own Offset above and only retargets Stagger and Spread.
            LayoutSelectionKind::Row => {
                specs.push(retarget_spec(
                    LayoutTransformTool::Stagger,
                    "Stagger column",
                ));
                specs.push(retarget_spec(LayoutTransformTool::Spread, "Spread column"));
            }
        }
        if matches!(
            selection_kind,
            LayoutSelectionKind::Column | LayoutSelectionKind::Matrix
        ) {
            specs.push(ContextToolSpec {
                tool: LayoutTransformTool::Splay,
                label: "Splay",
                accessible_label: "Activate Splay for the selected column".to_owned(),
                disabled: false,
            });
        }
    }
    specs
}

/// The toolbar's short visible label for one full menu label.
fn short_action_label(label: &str) -> &str {
    match label {
        "Stagger column" => "Stagger",
        "Offset column" | "Offset row" => "Offset",
        "Spread column" => "Spread",
        "Move matrix" => "Move",
        "Rotate matrix" => "Rotate",
        _ => label,
    }
}

#[component]
pub fn LayoutContextActionToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    #[props(default)] menu_items: bool,
) -> Element {
    let selection_label = context_selection_label(selection_kind);
    let rotate_followed = mount.matrix_rotate_followed.unwrap_or(true);
    let rotate_accessible_label = if rotate_followed {
        "Activate Rotate for the selected matrix"
    } else {
        "Rotate matrix (unavailable: Follow rotation is off)"
    };
    let property = context_property_spec(selection_kind, selection_label, &mount);
    let property_action = property.is_some();
    let visible_tools = context_tool_specs(selection_kind, &mount, rotate_accessible_label);
    if !property_action && visible_tools.is_empty() {
        return rsx! {};
    }
    let on_show_properties = mount.on_show_properties;
    let on_selection_kind = mount.on_selection_kind;
    let position_menu = open_menu;
    let (property_label, property_accessible_label) = property
        .map_or(("", String::new()), |property| {
            (property.label, property.accessible_label)
        });

    rsx! {
        if menu_items {
            if property_action {
                DropdownMenuItem::<()> {
                    index: 0usize, value: (), aria_label: property_accessible_label,
                    on_select: move |_| {
                        on_show_properties.call(());
                        if selection_kind == LayoutSelectionKind::Row { on_selection_kind.call(LayoutSelectionKind::Row); }
                        close_layout_command_menu(position_menu, LayoutCommandMenu::Transform);
                    },
                    "{property_label}"
                }
            }
            for (index, spec) in visible_tools.into_iter().enumerate() {
                DropdownMenuItem {
                    index: index + usize::from(property_action), value: spec.tool,
                    role: "menuitemradio", aria_checked: mount.active_tool == Some(spec.tool),
                    aria_label: spec.accessible_label, disabled: spec.disabled,
                    on_select: move |_: LayoutTransformTool| {
                        mount.on_transform_tool.call(spec.tool);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "{spec.label}"
                }
            }
        } else {
        Toolbar {
            class: "layout-action-strip",
            aria_label: "Actions for {selection_label}",
            if property_action {
                ToolbarButton {
                    index: 0usize,
                    class: "layout-action-control",
                    aria_label: property_accessible_label.clone(),
                    on_click: move |_| {
                        on_show_properties.call(());
                        if selection_kind == LayoutSelectionKind::Row {
                            on_selection_kind.call(LayoutSelectionKind::Row);
                        }
                        close_layout_command_menu(position_menu, LayoutCommandMenu::Transform);
                    },
                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                        path { d: "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3M5 7l-3 3 3 3M15 7l3 3-3 3" }
                    }
                    span { "{property_label}" }
                }
            }
            for (index, spec) in visible_tools.into_iter().enumerate() {
                LayoutContextToolButton {
                    index: index + usize::from(property_action),
                    mount: mount.clone(),
                    open_menu,
                    tool: spec.tool,
                    label: spec.label.to_owned(),
                    accessible_label: spec.accessible_label,
                    pressed: mount.active_tool == Some(spec.tool),
                    disabled: spec.disabled,
                }
            }
        }
        }
    }
}

#[component]
fn LayoutContextToolButton(
    index: usize,
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    tool: LayoutTransformTool,
    label: String,
    accessible_label: String,
    pressed: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let on_transform_tool = mount.on_transform_tool;
    let glyph = match tool {
        LayoutTransformTool::Stagger => "M10 2v16M6 6l4-4 4 4M6 14l4 4 4-4",
        LayoutTransformTool::Spread => "M2 10h16M4 6l-2 4 2 4M16 6l2 4-2 4",
        LayoutTransformTool::Offset => "M2 10h16M6 6l-4 4 4 4M14 6l4 4-4 4",
        // Ticket 05: Shift shares the four-way arrow; Bend is a curved arrow.
        // The Row/Key context bars do not list them yet (rail-only ticket).
        LayoutTransformTool::Shift => "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3",
        LayoutTransformTool::Bend => "M4 16C4 9 8 5 15 5M12 2l3 3-3 3",
        LayoutTransformTool::Splay => "M4 16h13M4 16l8-12M7 16a7 7 0 0 0 3-6",
        LayoutTransformTool::Origin => "M10 1v18M1 10h18M5 5h10v10H5z",
        LayoutTransformTool::Move => "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3",
        LayoutTransformTool::Rotate => "M17 10a7 7 0 1 1-2-4.9M17 2v5h-5",
        LayoutTransformTool::Anchor => {
            "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0"
        }
    };
    rsx! {
        Tooltip {
        TooltipTrigger { r#as: move |mut attributes: Vec<Attribute>| {
            // The toolbar owns its single tab stop; the tooltip supplies only description/events.
            attributes.retain(|attribute| attribute.name != "tabindex");
            rsx! {
        ToolbarButton {
            index,
            attributes,
            "data-slot": "toolbar-button",
            class: "layout-action-control",
            aria_label: accessible_label.clone(),
            aria_pressed: pressed,
            disabled,
            on_click: move |_| {
                on_transform_tool.call(tool);
                close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
            },
            svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: glyph } }
            span { {short_action_label(label.as_str())} }
        }
        } } }
        TooltipContent { class: "layout-tool-hint",
            Kbd { "Alt" } " bypass snapping · " Kbd { "Shift" } " coarser steps"
            if tool == LayoutTransformTool::Stagger { " · " Kbd { "Ctrl" } " quick offset" }
        }
        }
    }
}

fn focus_empty_transform_menu(content: &web_sys::Element) {
    let Some(document) = content.owner_document() else {
        return;
    };
    let rect = content.get_bounding_client_rect();
    if document
        .get_element_by_id("m1-layout-transform-menu")
        .as_ref()
        == Some(content)
        && content.is_connected()
        && content.get_attribute("data-state").as_deref() == Some("open")
        && content
            .closest("[hidden], [inert], [aria-hidden=true]")
            .ok()
            .flatten()
            .is_none()
        && rect.width() > 0.0
        && rect.height() > 0.0
        && document
            .default_view()
            .and_then(|window| window.get_computed_style(content).ok().flatten())
            .is_some_and(|style| style.get_property_value("visibility").as_deref() == Ok("visible"))
        && content
            .query_selector("[role^=menuitem]:not([aria-disabled=true])")
            .ok()
            .flatten()
            .is_none()
        && let Some(content) = content.dyn_ref::<web_sys::HtmlElement>()
    {
        let _ = content.focus();
    }
}

#[component]
pub fn LayoutTransformToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
) -> Element {
    use_effect(move || {
        if open_menu() == Some(LayoutCommandMenu::Transform)
            && let Some(content) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-layout-transform-menu"))
        {
            focus_empty_transform_menu(&content);
        }
    });
    let on_menu_item_mounted = move |event: MountedEvent| {
        if open_menu() == Some(LayoutCommandMenu::Transform)
            && let Some(content) = event
                .data()
                .downcast::<web_sys::Element>()
                .and_then(|item| item.closest("#m1-layout-transform-menu").ok().flatten())
        {
            focus_empty_transform_menu(&content);
        }
    };
    rsx! {
        DropdownMenu {
            open: Some(open_menu() == Some(LayoutCommandMenu::Transform)),
            on_open_change: move |next: bool| {
                let mut open_menu = open_menu;
                if next { open_menu.set(Some(LayoutCommandMenu::Transform)); }
                else if open_menu() == Some(LayoutCommandMenu::Transform) { open_menu.set(None); }
            },
            DropdownMenuTrigger { id: "m1-layout-transform-trigger", aria_label: "Transform", "Transform" }
            DropdownMenuContent { id: "m1-layout-transform-menu", tabindex: "-1",
                LayoutContextActionToolbar { mount: mount.clone(), open_menu, selection_kind, menu_items: true }
                DropdownMenuItem {
                        index: 10usize, value: LayoutTransformTool::Origin,
                        attributes: vec![onmounted(on_menu_item_mounted)],
                        role: "menuitemradio", aria_checked: mount.active_tool == Some(LayoutTransformTool::Origin),
                        disabled: !mount.pointer_tools_available || !mount.column_available,
                        on_select: move |_: LayoutTransformTool| {
                            mount.on_transform_tool.call(LayoutTransformTool::Origin);
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                        },
                        "Origin"
                    }
                if !mount.properties_available {
                    Callout { "Select a matrix, row, column, key or part to open its current transform properties." }
                }
            }
        }
    }
}
