//! Private Layout selection and snap controls plus their shared value adapters.
use super::tree::{self, TreeContext};
use boardstudio_application::ReadModel;
use boardstudio_core::model::Vec2;
pub use boardstudio_web_ui_model::state::{LayoutSnapSettings, TreeCellAnchor};
use boardstudio_web_ui_shared::controls::BoardCheckbox;
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardPopover, BoardSelectField, BoardToolbar, BoardToolbarButton,
};
use dioxus::prelude::*;
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
    fn pilot_popover_fixture() -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Select));
        let transition_trace = use_context_provider(|| Signal::new(Vec::<String>::new()));
        let mut selection_kind = use_signal(|| LayoutSelectionKind::Matrix);
        let open_menu_key = open_menu().map(LayoutCommandMenu::key).unwrap_or("none");
        let transition_trace_text = transition_trace().join("\n");
        rsx! {
            div { id: "pilot-popover-command-surface", class: "m1-layout-command-pill",
                LayoutSelectMenu {
                    open_menu,
                    selection_kind: selection_kind(),
                    on_selection_kind: move |kind| selection_kind.set(kind),
                }
                LayoutSnapMenu {
                    open_menu,
                    settings: LayoutSnapSettings::default(),
                    on_snap_intent: move |_| {},
                    show_layout_state: true,
                }
                output { id: "pilot-open-menu-state", "{open_menu_key}" }
                output { id: "pilot-menu-transition-trace", "{transition_trace_text}" }
            }
        }
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
    async fn direct_selection_choices_support_pointer_keyboard_and_close_specialist_menu() {
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
                Some(format!("{}:true", kind.label()))
            );
            assert_eq!(
                button.get_attribute("aria-pressed").as_deref(),
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

    fn popover_test_element(selector: &str) -> web_sys::Element {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("missing pilot popover test element {selector}"))
    }

    async fn mount_pilot_popovers() {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(previous) = document.get_element_by_id("pilot-popover-command-surface") {
            previous.remove();
        }
        if let Some(previous) = document.get_element_by_id("pilot-popover-test-styles") {
            previous.remove();
        }
        let styles = document.create_element("style").unwrap();
        styles.set_id("pilot-popover-test-styles");
        let css = format!(
            "{}\n{}",
            include_str!("../../../../assets/m1.css"),
            include_str!("../../../../assets/tailwind.css")
        );
        styles.set_text_content(Some(&css));
        document.body().unwrap().append_child(&styles).unwrap();

        let mount = document.create_element("div").unwrap();
        mount.set_id("pilot-popover-test-root");
        document.body().unwrap().append_child(&mount).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(pilot_popover_fixture),
            dioxus_web::Config::new().rootnode(mount.into()),
        );
        gloo_timers::future::TimeoutFuture::new(80).await;
        gloo_timers::future::TimeoutFuture::new(80).await;
    }

    #[wasm_bindgen_test]
    async fn contextual_toolbar_stays_inside_its_workspace() {
        mount_pilot_popovers().await;
        let document = web_sys::window().unwrap().document().unwrap();
        let workspace = document.create_element("section").unwrap();
        workspace.set_class_name("m1-workspace-content");
        workspace
            .set_attribute(
                "style",
                "position:fixed;left:200px;top:60px;width:800px;height:600px",
            )
            .unwrap();
        let region = document.create_element("div").unwrap();
        region.set_class_name("m1-layout-toolbar-region");
        let toolbar = document.create_element("div").unwrap();
        toolbar.set_class_name("m1-canvas-toolbar");
        let mount = document
            .get_element_by_id("pilot-popover-test-root")
            .unwrap();
        toolbar.append_child(&mount).unwrap();
        region.append_child(&toolbar).unwrap();
        workspace.append_child(&region).unwrap();
        document.body().unwrap().append_child(&workspace).unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        let bounds = workspace.get_bounding_client_rect();
        let controls = toolbar.get_bounding_client_rect();
        assert!(
            controls.left() >= bounds.left() && controls.right() <= bounds.right(),
            "context controls must remain within the canvas rather than translating underneath the Objects dock"
        );
        workspace.remove();
    }

    #[wasm_bindgen_test]
    async fn pilot_popovers_preserve_select_rows_and_snap_field_layout() {
        mount_pilot_popovers().await;

        let select = popover_test_element("#m1-layout-select-menu");
        let window = web_sys::window().unwrap();
        let select_style = window.get_computed_style(&select).unwrap().unwrap();
        assert_eq!(select_style.get_property_value("display").unwrap(), "grid");
        assert_eq!(select_style.get_property_value("gap").unwrap(), "5px");
        let options = select
            .query_selector_all(".m1-layout-select-option")
            .unwrap();
        assert_eq!(options.length(), 5);
        let mut previous_top = f64::NEG_INFINITY;
        for index in 0..options.length() {
            let option = options
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            assert_eq!(
                option.parent_element().unwrap().id(),
                "m1-layout-select-menu",
                "each selection option must be a direct grid row"
            );
            let top = option.get_bounding_client_rect().top();
            assert!(
                top > previous_top,
                "selection options must occupy distinct rows"
            );
            previous_top = top;
        }
        let option_style = window
            .get_computed_style(&popover_test_element(".m1-layout-select-option"))
            .unwrap()
            .unwrap();
        assert_eq!(
            option_style.get_property_value("border-radius").unwrap(),
            "5px"
        );

        let snap_trigger = popover_test_element("#m1-layout-snap-trigger")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let transition_trace = popover_test_element("#pilot-menu-transition-trace");
        transition_trace
            .set_attribute("data-phase", "pointerdown-start")
            .unwrap();
        let pointer_down = web_sys::EventInit::new();
        pointer_down.set_bubbles(true);
        let pointer_down =
            web_sys::Event::new_with_event_init_dict("pointerdown", &pointer_down).unwrap();
        snap_trigger.dispatch_event(&pointer_down).unwrap();
        transition_trace
            .set_attribute("data-phase", "click-start")
            .unwrap();
        snap_trigger.click();
        transition_trace
            .set_attribute("data-phase", "settle-wait")
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        let callback_trace = transition_trace.text_content().unwrap_or_default();
        let select_expanded = popover_test_element("#m1-layout-select-trigger")
            .get_attribute("aria-expanded")
            .unwrap_or_default();
        let snap_expanded = snap_trigger
            .get_attribute("aria-expanded")
            .unwrap_or_default();
        assert_eq!(
            popover_test_element("#pilot-open-menu-state")
                .text_content()
                .as_deref(),
            Some("snap"),
            "the real Snap trigger should update the shared menu owner; callbacks: {callback_trace}; rendered expanded(select={select_expanded},snap={snap_expanded}); active={}",
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .map(|element| element.id())
                .unwrap_or_default()
        );
        let snap = popover_test_element("#m1-layout-snap-menu");
        let snap_style = window.get_computed_style(&snap).unwrap().unwrap();
        assert_eq!(snap_style.get_property_value("display").unwrap(), "grid");
        assert_eq!(snap_style.get_property_value("gap").unwrap(), "5px");
        for selector in ["label", ".m1-layout-gap-field"] {
            let label = snap.query_selector(selector).unwrap().unwrap();
            assert_eq!(
                label.parent_element().unwrap().id(),
                "m1-layout-snap-menu",
                "Snap labels must remain direct children of the styled popover"
            );
            let style = window.get_computed_style(&label).unwrap().unwrap();
            assert_eq!(style.get_property_value("display").unwrap(), "grid");
            assert_eq!(style.get_property_value("gap").unwrap(), "4px");
        }

        popover_test_element("#m1-layout-select-trigger")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(80).await;
        let snap_trigger = popover_test_element("#m1-layout-snap-trigger")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        snap_trigger.focus().unwrap();
        transition_trace
            .set_attribute("data-phase", "focused-click-start")
            .unwrap();
        snap_trigger.click();
        gloo_timers::future::TimeoutFuture::new(80).await;
        let callback_trace = transition_trace.text_content().unwrap_or_default();
        assert_eq!(
            popover_test_element("#pilot-open-menu-state")
                .text_content()
                .as_deref(),
            Some("snap"),
            "focused Snap activation must survive Select focus dismissal; callbacks: {callback_trace}; active={}",
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .map(|element| element.id())
                .unwrap_or_default()
        );
        popover_test_element("#pilot-popover-test-root").remove();
        document_cleanup_styles();
    }

    fn document_cleanup_styles() {
        if let Some(styles) = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("pilot-popover-test-styles")
        {
            styles.remove();
        }
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

#[cfg(test)]
type LayoutMenuTransitionTrace = Signal<Vec<String>>;

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
        .and_then(|document| document.get_element_by_id(menu.trigger_id()))
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
    let mut compact = use_signal(|| {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| {
                storage
                    .get_item("boardstudio:v2:layout-toolbar-compact")
                    .ok()
                    .flatten()
            })
            .as_deref()
            == Some("true")
    });
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
            "[data-layout-menu='{}'] :is(input:not(:disabled), select:not(:disabled), button:not(:disabled))",
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
            "data-compact": pointer_tools_visible && compact(),
            if pointer_tools_visible && has_selection_context {
                LayoutSelectMenu { open_menu, selection_kind, on_selection_kind }
                super::layout_transform_toolbar::LayoutContextActionToolbar {
                    mount: transform.clone(), open_menu, selection_kind,
                }
            } else if pointer_tools_visible {
                LayoutSelectionToolbar { open_menu, selection_kind, on_selection_kind }
            } else {
                LayoutSelectMenu { open_menu, selection_kind, on_selection_kind }
            }
            super::layout_transform_toolbar::LayoutTransformToolbar {
                open_menu,
                mount: transform.clone(),
                selection_kind: pointer_tools_visible.then_some(selection_kind),
            }
            super::layout_align::LayoutAlignToolbar {
                mount: align,
                open_menu,
                show_relationships,
                has_selection_context,
                on_show_relationships,
            }
            if pointer_tools_visible {
                BoardButton {
                    class: "layout-toolbar-density",
                    aria_label: "Compact toolbar", title: if compact() { "Show toolbar labels" } else { "Use icon-only toolbar" },
                    pressed: compact(),
                    onclick: move |_| {
                        let next = !compact();
                        compact.set(next);
                        if let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten()) {
                            let _ = storage.set_item("boardstudio:v2:layout-toolbar-compact", if next { "true" } else { "false" });
                        }
                    },
                    svg { class: "m1-context-tool-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                        rect { x: "2", y: "4", width: "4", height: "4", rx: "1" }
                        rect { x: "2", y: "12", width: "4", height: "4", rx: "1" }
                        path { d: "M9 6h9M9 14h9" }
                    }
                }
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
                    aria_pressed: geometry_snap_enabled,
                    onclick: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap_enabled)),
                    span { if geometry_snap_enabled { "Geometry on" } else { "Geometry off" } }
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
) -> Element {
    rsx! {
        BoardToolbar { class: "layout-selection-actions", label: "Selection scope",
            for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                BoardToolbarButton {
                    index, class: "layout-selection-action", aria_label: "Select {kind.label()}",
                    title: "Select {kind.label()}", pressed: kind == selection_kind,
                    on_activate: move |_| { on_selection_kind.call(kind); open_menu.set(None); },
                    LayoutSelectionIcon { kind }
                    span { class: "layout-action-label", "{kind.label()}" }
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
#[component]
pub fn LayoutSnapDock(
    settings: LayoutSnapSettings,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let mut open_menu = open_menu;
    let open = open_menu() == Some(LayoutCommandMenu::Snap);
    rsx! {
        BoardPopover { open, on_open_change: move |next| { if next { open_menu.set(Some(LayoutCommandMenu::Snap)); } else if open_menu() == Some(LayoutCommandMenu::Snap) { open_menu.set(None); } },
            trigger_id: "m1-layout-snap-trigger", content_id: "m1-snap-dock-options", trigger_label: "Snap",
            root_class: "m1-layout-snap-dock", trigger_class: "m1-snap-dock-trigger", content_class: "m1-snap-dock-options", content_role: "region", content_label: "Snap options",
            trigger: rsx! {
                svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                span { "Snap" } small { "{snap_label(settings.snap_fraction)}" }
                span { class: "m1-dock-chevron", "aria-hidden": "true", if open { "⌄" } else { "⌃" } }
            },
            LayoutSnapOptions { settings, on_snap_intent }
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
    #[cfg(test)]
    let transition_trace = try_consume_context::<LayoutMenuTransitionTrace>();
    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                #[cfg(test)]
                let owner_before = open_menu();
                open_menu.with_mut(|owner| {
                    if next {
                        *owner = Some(LayoutCommandMenu::Select);
                    } else if *owner == Some(LayoutCommandMenu::Select) {
                        *owner = None;
                    }
                });
                #[cfg(test)]
                if let Some(trace) = transition_trace {
                    record_layout_menu_transition(
                        trace,
                        "Select",
                        next,
                        owner_before,
                        open_menu(),
                    );
                }
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

#[cfg(test)]
fn record_layout_menu_transition(
    mut trace: LayoutMenuTransitionTrace,
    menu: &str,
    next: bool,
    owner_before: Option<LayoutCommandMenu>,
    owner_after: Option<LayoutCommandMenu>,
) {
    let document = web_sys::window().and_then(|window| window.document());
    let expanded = |id| {
        document
            .as_ref()
            .and_then(|document| document.get_element_by_id(id))
            .and_then(|trigger| trigger.get_attribute("aria-expanded"))
            .unwrap_or_else(|| "missing".to_owned())
    };
    let active = document
        .as_ref()
        .and_then(|document| document.active_element())
        .map(|element| element.id())
        .unwrap_or_default();
    let select_expanded = expanded("m1-layout-select-trigger");
    let snap_expanded = expanded("m1-layout-snap-trigger");
    let phase = document
        .as_ref()
        .and_then(|document| document.get_element_by_id("pilot-menu-transition-trace"))
        .and_then(|trace| trace.get_attribute("data-phase"))
        .unwrap_or_else(|| "unknown".to_owned());
    trace.with_mut(|events| {
        events.push(format!(
            "phase={phase} callback={menu}.on_open_change({next}) owner={owner_before:?}->{owner_after:?} expanded(select={select_expanded},snap={snap_expanded}) active={active}"
        ));
    });
}

#[component]
fn LayoutSnapMenu(
    open_menu: Signal<Option<LayoutCommandMenu>>,
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    show_layout_state: bool,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Snap);
    #[cfg(test)]
    let transition_trace = try_consume_context::<LayoutMenuTransitionTrace>();
    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                #[cfg(test)]
                let owner_before = open_menu();
                open_menu.with_mut(|owner| {
                    if next {
                        *owner = Some(LayoutCommandMenu::Snap);
                    } else if *owner == Some(LayoutCommandMenu::Snap) {
                        *owner = None;
                    }
                });
                #[cfg(test)]
                if let Some(trace) = transition_trace {
                    record_layout_menu_transition(
                        trace,
                        "Snap",
                        next,
                        owner_before,
                        open_menu(),
                    );
                }
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
