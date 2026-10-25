//! Private Layout selection and snap controls plus their shared value adapters.
use super::tree::{self, TreeContext};
use boardstudio_application::ReadModel;
use boardstudio_core::model::Vec2;
pub use boardstudio_web_ui_model::state::{LayoutSnapSettings, TreeCellAnchor};
use boardstudio_web_ui_shared::controls::{BoardButton, BoardCheckbox, BoardPopover, BoardToolbar};
use dioxus::prelude::*;
use dioxus_primitives::toolbar::ToolbarButton;
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
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(direct_selection_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        for kind in LayoutSelectionKind::ALL {
            let button = root
                .query_selector(&format!("button[aria-label='Select {}']", kind.label()))
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
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
        let matrix = root
            .query_selector("button[aria-label='Select Matrix']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
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
            busy: false,
            feedback: None,
            on_reference: EventHandler::new(|_: String| {}),
            on_align: EventHandler::new(|_: super::super::layout_align::AlignAction| {}),
        };
        rsx! {
            button {
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

    fn tool_button_count(root: &web_sys::Element, label: &str) -> usize {
        let buttons = root.query_selector_all("button").unwrap();
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
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                pill_duplicate_probe_host,
                PillDuplicateProbeProps { open, tools },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
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
    async fn scope_popover_keeps_hint_when_nothing_selected() {
        let root = mount_pill_with_tools(Some(LayoutCommandMenu::Select), false).await;
        click(
            &root
                .query_selector("#pill-probe-open")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap(),
        )
        .await;
        let hint = root
            .query_selector("#m1-layout-scope-panel-Column .m1-layout-scope-hint")
            .unwrap()
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
    async fn column_scope_trigger_never_opens_panel_when_tools_visible() {
        let root = mount_pill(Some(LayoutCommandMenu::Select)).await;
        click(&scope_trigger(&root, "Column")).await;
        assert!(
            root.query_selector("#m1-layout-scope-panel-Column")
                .unwrap()
                .is_none(),
            "Column actions panel must not render when the pill already carries the tools"
        );
        for label in ["Stagger", "Spread", "Offset"] {
            assert_eq!(
                tool_button_count(&root, label),
                1,
                "{label} must appear exactly once: inline in the toolbar, never in a scope panel"
            );
        }
        root.remove();
    }

    #[component]
    fn scope_reselect_probe_host() -> Element {
        let mut kind = use_signal(|| LayoutSelectionKind::Key);
        let mut has_selection = use_signal(|| false);
        let open_menu = use_signal(|| None::<LayoutCommandMenu>);
        let transform = has_selection().then(|| {
            super::super::layout_transform_toolbar::LayoutTransformMenuMount {
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
            }
        });
        rsx! {
            button {
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
    async fn scope_panel_closes_when_column_selection_arrives() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(scope_reselect_probe_host),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        click(&scope_trigger(&root, "Column")).await;
        assert!(
            root.query_selector("#m1-layout-scope-panel-Column")
                .unwrap()
                .is_some(),
            "the no-selection scope trigger still opens its hint panel"
        );
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
            root.query_selector("#m1-layout-scope-panel-Column")
                .unwrap()
                .is_none(),
            "selecting a column on the canvas must close the scope panel, not leave it over the pill"
        );
        click(&scope_trigger(&root, "Column")).await;
        assert!(
            root.query_selector("#m1-layout-scope-panel-Column")
                .unwrap()
                .is_none(),
            "re-selecting the Column scope while tools are visible selects without reopening a panel"
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
    rsx! {
        div { class: "m1-layout-command-menu-header",
            strong { "{label}" }
            button {
                r#type: "button",
                aria_label: "{close_label}",
                onclick: move |_| close_layout_command_menu(open_menu, menu),
                "×"
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
                    .query_selector(".layout-selection-action[aria-pressed='true']")
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
                            .query_selector(".m1-layout-snap-dock")
                            .ok()
                            .flatten()
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
    use_effect(use_reactive((&open_menu(),), move |(active_menu,)| {
        let Some(menu) = active_menu else {
            return;
        };
        let selector = format!(
            "[data-layout-menu='{}'][data-state='open'] .board-popover-content :is(input:not(:disabled), select:not(:disabled), button:not(:disabled))",
            menu.key()
        );
        if let Some(control) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.query_selector(&selector).ok().flatten())
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        {
            let _ = control.focus();
        }
    }));
    use_drop({
        let outside_listener = outside_listener.clone();
        move || remove_menu_outside_listener(&outside_listener)
    });

    rsx! {
        div { id: "m1-layout-command-pill", class: "m1-layout-command-pill layout-command-pill--contextual", role: "group", aria_label: "{command_label}",
            "data-compact": false,
            div { class: "m1-layout-primary-pill board-tool-pill",
            if pointer_tools_visible {
                LayoutSelectionToolbar { open_menu, selection_kind, on_selection_kind, transform: Some(transform.clone()) }
            } else {
                LayoutSelectMenu { open_menu, selection_kind, on_selection_kind }
            }
            div { class: "layout-edit-actions", role: "group", aria_label: "Selection actions",
            super::layout_transform_toolbar::LayoutTransformToolbar {
                open_menu,
                mount: transform.clone(),
            }
            super::layout_align::LayoutAlignToolbar {
                mount: align,
                open_menu,
                show_relationships,
                has_selection_context,
                on_show_relationships,
            }
            if !snap_in_dock { LayoutSnapMenu {
                open_menu,
                settings: snap_settings,
                on_snap_intent,
                show_layout_state: pointer_tools_visible,
            } }
            if pointer_tools_visible && !snap_in_dock {
                button {
                    r#type: "button",
                    class: "layout-geometry-snap-toggle px-2 py-1 text-xs font-medium",
                    aria_label: "Geometry snap",
                    title: "Geometry snap",
                    aria_pressed: geometry_snap_enabled,
                    onclick: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap_enabled)),
                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                        path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" }
                    }
                    span { class: "m1-visually-hidden", if geometry_snap_enabled { "Geometry on" } else { "Geometry off" } }
                }
            }
            }
            }
            if pointer_tools_visible && has_selection_context {
                super::layout_transform_toolbar::LayoutContextActionToolbar {
                    mount: transform.clone(), open_menu, selection_kind,
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
    // While a selection backs the inline tools, scope triggers only switch kind:
    // opening a panel here would cover the pill the panel points at. A canvas
    // selection arriving under an open hint panel lands here as well.
    use_effect(use_reactive((&has_actions,), move |(has_actions,)| {
        if has_actions {
            open_menu.set(None);
        }
    }));
    rsx! {
        BoardToolbar { class: "layout-selection-actions", label: "Selection scope",
            for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                if has_actions {
                    ToolbarButton {
                        index,
                        id: format!("m1-layout-scope-{}", kind.label()),
                        class: "board-popover-trigger layout-selection-action",
                        title: format!("Select {}", kind.label()),
                        aria_label: format!("Select {}", kind.label()),
                        aria_pressed: kind == selection_kind,
                        on_click: move |_| {
                            on_selection_kind.call(kind);
                            open_menu.set(None);
                        },
                        LayoutSelectionIcon { kind }
                        span { class: "layout-action-label", "{kind.label()}" }
                    }
                } else {
                BoardPopover {
                    open: open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind,
                    on_open_change: move |next| {
                        if next {
                            on_selection_kind.call(kind);
                            open_menu.set(Some(LayoutCommandMenu::Select));
                        } else if kind == selection_kind && open_menu() == Some(LayoutCommandMenu::Select) {
                            open_menu.set(None);
                        }
                    },
                    toolbar_index: index,
                    pressed: kind == selection_kind,
                    trigger_id: format!("m1-layout-scope-{}", kind.label()),
                    content_id: format!("m1-layout-scope-panel-{}", kind.label()),
                    trigger_label: format!("Select {}", kind.label()),
                    root_class: "m1-layout-command-menu m1-layout-scope-menu",
                    menu_kind: "select",
                    trigger_class: "layout-selection-action",
                    content_class: "m1-layout-scope-popover",
                    content_role: "group",
                    content_label: format!("{} actions", kind.label()),
                    trigger: rsx! {
                        LayoutSelectionIcon { kind }
                        span { class: "layout-action-label", "{kind.label()}" }
                    },
                    LayoutCommandMenuHeader {
                        label: format!("{} actions", kind.label()),
                        close_label: format!("Close {} actions", kind.label()),
                        open_menu, menu: LayoutCommandMenu::Select,
                    }
                    p { class: "m1-layout-scope-hint", "Select a {kind.label().to_lowercase()} on the canvas to edit it." }
                    if let Some(mount) = transform.clone() {
                        BoardButton {
                            onclick: move |_| {
                                mount.on_show_properties.call(());
                                close_layout_command_menu(open_menu, LayoutCommandMenu::Select);
                            },
                            "Open Inspector"
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
        svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
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
    on_toggle_handles: EventHandler<bool>,
) -> Element {
    rsx! {
        boardstudio_web_ui_shared::controls::BoardAccordionSection {
            class: "m1-layout-snap-dock", label: "Snap", id: "m1-layout-snap-trigger",
            heading: rsx! {
                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                span { "Snap" } small { "{snap_label(settings.snap_fraction)}" }
            },
            LayoutSnapOptions { settings, on_snap_intent }
            label { class: "m1-layout-snap-check",
                BoardCheckbox {
                    checked: handles_visible,
                    onchange: move |event: FormEvent| {
                        on_toggle_handles.call(event.checked());
                    },
                }
                "Canvas handles"
            }
        }
    }
}

#[component]
fn LayoutSelectMenu(
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Select);
    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                open_menu.with_mut(|owner| {
                    if next {
                        *owner = Some(LayoutCommandMenu::Select);
                    } else if *owner == Some(LayoutCommandMenu::Select) {
                        *owner = None;
                    }
                });
            },
            root_class: "m1-layout-command-menu m1-layout-select-menu",
            menu_kind: Some("select".to_owned()),
            trigger_id: "m1-layout-select-trigger",
            content_id: "m1-layout-select-menu",
            trigger_label: format!("Select: {}", selection_kind.label()),
            trigger_class: "m1-layout-select-trigger",
            content_class: "m1-layout-select-popover",
            content_role: "group",
            content_label: Some("Selection scope".to_owned()),
            trigger: rsx! {
                LayoutSelectionIcon { kind: selection_kind }
                span { "{selection_kind.label()}" }
            },
            children: rsx! {
                LayoutCommandMenuHeader {
                    label: format!("Select: {}", selection_kind.label()),
                    close_label: format!("Close Select: {}", selection_kind.label()),
                    open_menu,
                    menu: LayoutCommandMenu::Select,
                }
                for kind in LayoutSelectionKind::ALL {
                    BoardButton {
                        density: boardstudio_web_ui_shared::controls::BoardButtonDensity::Compact,
                        class: "m1-layout-select-option",
                        pressed: Some(kind == selection_kind),
                        onclick: move |_| {
                            on_selection_kind.call(kind);
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Select);
                        },
                        "{kind.label()}"
                    }
                }
            },
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
    let is_open = open_menu() == Some(LayoutCommandMenu::Snap);
    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                open_menu.with_mut(|owner| {
                    if next {
                        *owner = Some(LayoutCommandMenu::Snap);
                    } else if *owner == Some(LayoutCommandMenu::Snap) {
                        *owner = None;
                    }
                });
            },
            root_class: "m1-layout-command-menu m1-layout-snap-menu",
            menu_kind: Some("snap".to_owned()),
            trigger_id: "m1-layout-snap-trigger",
            content_id: "m1-layout-snap-menu",
            trigger_label: "Snap settings".to_owned(),
            trigger_class: "m1-layout-snap-trigger",
            content_class: "m1-layout-snap-popover",
            trigger: rsx! {
                svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M4 3v7a6 6 0 0 0 12 0V3h-4v7a2 2 0 0 1-4 0V3ZM4 7h4M12 7h4" }
                }
                span { "Snap" }
                if show_layout_state {
                    span { class: "text-xs text-muted", "{snap_label(settings.snap_fraction)}" }
                }
            },
            children: rsx! {
                LayoutCommandMenuHeader {
                    label: "Snap".to_owned(),
                    close_label: "Close Snap".to_owned(),
                    open_menu,
                    menu: LayoutCommandMenu::Snap,
                }
                LayoutSnapOptions { settings, on_snap_intent }
            },
        }
    }
}

#[component]
fn LayoutSnapOptions(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    rsx! {
                label { "Snap increment"
                    select {
                        "aria-label": "Snap increment",
                        value: "{settings.snap_fraction}",
                        onchange: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite()
                                && SNAP_STEPS.iter().any(|(step, _, _)| *step == value)
                            {
                                on_snap_intent.call(LayoutSnapIntent::Fraction(value));
                            }
                        },
                        for (fraction, value, label) in SNAP_STEPS {
                            option { value: "{value}", selected: settings.snap_fraction == fraction, "{label}" }
                        }
                    }
                }
                label { class: "m1-layout-snap-check",
                    BoardCheckbox {
                        checked: settings.geometry_snap,
                        onchange: move |event: FormEvent| {
                            on_snap_intent.call(LayoutSnapIntent::GeometrySnap(event.checked()));
                        },
                    }
                    "Geometry snap"
                }
                label { class: "m1-layout-snap-check",
                    BoardCheckbox {
                        checked: settings.gap_snap,
                        disabled: !settings.geometry_snap,
                        onchange: move |event: FormEvent| {
                            on_snap_intent.call(LayoutSnapIntent::GapSnap(event.checked()));
                        },
                    }
                    "Envelope gap"
                }
                label { class: "m1-layout-gap-field", "Gap (mm)"
                    input {
                        r#type: "number",
                        min: "0",
                        step: "any",
                        "aria-label": "Snap gap",
                        value: "{settings.gap_override}",
                        oninput: move |event: FormEvent| {
                            on_snap_intent.call(LayoutSnapIntent::GapOverride(event.value()));
                        },
                    }
                }
                p { "Shared by Layout, drawing and perimeter editing. Unit steps use matrix pitch; mm steps use world coordinates. Hold Alt to bypass snapping." }
    }
}
