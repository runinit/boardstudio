//! Private Layout selection and snap controls plus their shared value adapters.
use super::tree::{self, TreeContext};
use boardstudio_application::ReadModel;
use boardstudio_model::model::Vec2;
pub use boardstudio_web_ui_model::state::{LayoutSnapSettings, TreeCellAnchor};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    callout::Callout,
    checkbox::Checkbox,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    popover::{Popover, PopoverContent, PopoverTrigger},
    select::{Select, SelectOption},
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::Toolbar,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Document, HtmlElement, Node, PointerEvent as WebPointerEvent};

const DEFAULT_PITCH_MM: f64 = 19.05;

pub use super::layout_selection_kind::LayoutSelectionKind;

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
#[cfg(test)]
#[path = "layout_toolbar/tests.rs"]
mod tests;

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
}

impl LayoutCommandMenu {
    const fn key(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Transform => "transform",
            Self::Align => "align",
            Self::Snap => "snap",
        }
    }

    const fn trigger_id(self) -> &'static str {
        match self {
            Self::Select => "m1-layout-select-trigger",
            Self::Transform => "m1-layout-transform-trigger",
            Self::Align => "m1-layout-align-trigger",
            Self::Snap => "m1-layout-snap-trigger",
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
                TooltipTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                    Button { attributes, variant: ButtonVariant::Ghost, class: "layout-popup-close",
                        aria_label: trigger_label.clone(),
                        onclick: move |_| close_layout_command_menu(open_menu, menu),
                        span { aria_hidden: true, "×" }
                    }
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
) -> Element {
    let pointer_tools_visible = transform.pointer_tools_visible;
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
                    menu.set(Some(LayoutCommandMenu::Select));
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

    rsx! {
        div { id: "m1-layout-command-pill", class: "layout-toolbar-strips", role: "group", aria_label: "{command_label}",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " "
                    && let Some(raw) = event.data().try_as_web_event()
                    && raw.target().and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|target| target.id() == "m1-layout-snap-trigger"
                            || (target.tag_name() == "BUTTON"
                                && target.closest("#m1-layout-command-pill").ok().flatten().is_some()
                                && target.matches(".layout-scope-trigger, .layout-action-control, .layout-geometry-control, .layout-popup-close, .layout-menu-actions [data-slot=popover-trigger], .layout-menu-actions [data-slot=dropdown-menu-trigger], .layout-scope-popup [data-slot=button], .layout-settings-popup [data-slot=button], .layout-settings-popup [data-slot=checkbox][aria-label='Geometry snap'], .layout-settings-popup [data-slot=checkbox][aria-label='Envelope gap']").unwrap_or(false)))
                {
                    // Native control activation must not become the canvas's Space-pan shortcut.
                    event.stop_propagation();
                    raw.stop_propagation();
                }
            },
            div { class: "layout-scope-strip",
                if pointer_tools_visible {
                    LayoutSelectionToolbar { open_menu, selection_kind, on_selection_kind, transform: Some(transform.clone()) }
                } else {
                    LayoutSelectMenu { open_menu, selection_kind, on_selection_kind }
                }
            }
            div { class: "layout-context-strip",
                if pointer_tools_visible {
                div { class: "layout-action-slot",
                if has_selection_context {
                    super::layout_transform_toolbar::LayoutContextActionToolbar {
                        mount: transform.clone(), open_menu, selection_kind,
                    }
                }
                }
                }
                div { class: "layout-menu-actions", role: "group", aria_label: "Selection actions",
                    super::layout_align::LayoutAlignToolbar {
                        mount: align, open_menu, show_relationships, has_selection_context, on_show_relationships,
                    }
                    super::layout_transform_toolbar::LayoutTransformToolbar {
                        open_menu, mount: transform.clone(),
                        selection_kind: pointer_tools_visible.then_some(selection_kind),
                    }
                    if !snap_in_dock {
                        LayoutSnapMenu { open_menu, settings: snap_settings, on_snap_intent, show_layout_state: pointer_tools_visible }
                        if pointer_tools_visible {
                            Tooltip {
                                TooltipTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                                    Button { attributes, variant: ButtonVariant::Ghost,
                                        class: "layout-geometry-control",
                                        aria_label: "Geometry snap", aria_pressed: geometry_snap_enabled,
                                        onclick: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap_enabled)),
                                        svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                            path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" }
                                        }
                                    }
                                } }
                                TooltipContent { "Geometry snap" }
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
    #[props(default)] transform: Option<super::layout_transform_toolbar::LayoutTransformMenuMount>,
) -> Element {
    let has_actions = transform
        .as_ref()
        .is_some_and(|mount| mount.properties_available || mount.pointer_tools_available);
    use_effect(use_reactive((&selection_kind,), move |(kind,)| {
        if open_menu() == Some(LayoutCommandMenu::Select) {
            focus_layout_popup(&format!("m1-layout-scope-panel-{}", kind.label()));
        }
    }));
    rsx! {
        Toolbar { class: "layout-scope-toolbar", aria_label: "Selection scope",
            ToggleGroup {
                aria_label: "Selection scope choices", horizontal: true,
                pressed: Some(std::collections::HashSet::from([LayoutSelectionKind::ALL.iter().position(|kind| *kind == selection_kind).unwrap()])),
                for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                    Popover {
                        open: open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind,
                        is_modal: false,
                        on_open_change: move |next| {
                            if next {
                                on_selection_kind.call(kind);
                                open_menu.set(Some(LayoutCommandMenu::Select));
                            } else if kind == selection_kind && open_menu() == Some(LayoutCommandMenu::Select) {
                                dismiss_layout_popup(open_menu, LayoutCommandMenu::Select, &format!("m1-layout-scope-panel-{}", kind.label()));
                            }
                        },
                        ToggleItem {
                            index, class: "layout-scope-trigger",
                            id: format!("m1-layout-scope-{}", kind.label()),
                            aria_label: format!("Select {}", kind.label()),
                            aria_haspopup: "dialog",
                            aria_expanded: open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind,
                            aria_controls: (open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind).then(|| format!("m1-layout-scope-panel-{}", kind.label())),
                            attributes: vec![onclick(move |_| {
                                let was_open = kind == selection_kind && open_menu() == Some(LayoutCommandMenu::Select);
                                on_selection_kind.call(kind);
                                open_menu.set((!was_open).then_some(LayoutCommandMenu::Select));
                            })],
                            LayoutSelectionIcon { kind }
                            span { "{kind.label()}" }
                        }
                        PopoverContent {
                            id: format!("m1-layout-scope-panel-{}", kind.label()),
                            class: "layout-scope-popup", role: "dialog", aria_label: format!("{} actions", kind.label()),
                            aria_labelledby: Option::<String>::None,
                            inert: (open_menu() != Some(LayoutCommandMenu::Select) || kind != selection_kind).then_some(""),
                            attributes: vec![onmounted(move |_| {
                                if open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind {
                                    focus_layout_popup(&format!("m1-layout-scope-panel-{}", kind.label()));
                                }
                            })],
                            LayoutCommandMenuHeader {
                                label: format!("{} actions", kind.label()), close_label: format!("Close {} actions", kind.label()),
                                open_menu, menu: LayoutCommandMenu::Select,
                            }
                            if has_actions {
                                if let Some(mount) = transform.clone() {
                                    super::layout_transform_toolbar::LayoutContextActionToolbar { mount, open_menu, selection_kind: kind }
                                }
                            } else {
                                Callout { "Select a {kind.label().to_lowercase()} on the canvas to edit it." }
                                if let Some(mount) = transform.clone() {
                                    Button { variant: ButtonVariant::Secondary,
                                        onclick: move |_| {
                                            mount.on_show_properties.call(());
                                            close_layout_command_menu(open_menu, LayoutCommandMenu::Select);
                                        }, "Open Inspector"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
                class: "flex w-full min-h-control-touch items-center justify-start gap-2 rounded-none border-0 bg-transparent px-3 py-2.5 text-left text-foreground hover:bg-accent",
                attributes: vec![dioxus::events::onkeydown(move |event: KeyboardEvent| {
                    // Space must activate the button, not the Layout window's pan shortcut.
                    if event.key().to_string() == " " {
                        event.stop_propagation();
                        if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                    }
                })],
                svg { class: "size-[18px] fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                span { "Snap" } small { class: "ml-auto text-muted-foreground", "{snap_label(settings.snap_fraction)}" }
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
) -> Element {
    rsx! {
        LayoutCommandPopup {
            open_menu, menu: LayoutCommandMenu::Snap, label: "Snap settings",
            trigger: rsx! { span { "Snap" } if show_layout_state { small { "{snap_label(settings.snap_fraction)}" } } },
            LayoutSnapOptions { settings, on_snap_intent }
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
    let fraction = use_memo(use_reactive((&settings.snap_fraction,), |(value,)| {
        Some(value.to_string())
    }));
    rsx! {
        div { class: "layout-snap-fields",
            Label { "Snap increment" }
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
