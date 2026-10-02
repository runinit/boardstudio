//! Private Layout selection and snap controls plus their shared value adapters.
use super::tree::{self, TreeContext};
use boardstudio_application::ReadModel;
use boardstudio_core::model::Vec2;
use dioxus::prelude::*;

const DEFAULT_PITCH_MM: f64 = 19.05;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::presentation) enum LayoutSelectionKind {
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
pub(in crate::presentation) struct LayoutSnapSettings {
    /// Zero disables the grid, positive values are fractions of pitch, negative values are mm.
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap_snap: bool,
    /// Kept as an editable draft; blank or invalid input uses the current context's fallback.
    pub gap_override: String,
}

impl Default for LayoutSnapSettings {
    fn default() -> Self {
        Self {
            snap_fraction: 0.25,
            geometry_snap: true,
            gap_snap: true,
            gap_override: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) enum LayoutSnapIntent {
    SetFraction(f64),
    SetGeometrySnap(bool),
    SetGapSnap(bool),
    SetGapOverride(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::presentation) struct GestureSnapInputs {
    pub pitch: Vec2,
    pub snap_fraction: f64,
    pub geometry_snap: bool,
    pub gap: Option<f64>,
}

/// Produce the exact existing GestureBegin snap values from current root-owned preferences.
pub(in crate::presentation) fn gesture_snap_inputs(
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

    #[test]
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

    #[test]
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

    #[test]
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

    #[test]
    fn gesture_gap_uses_live_matrix_then_one_millimeter_and_respects_toggles() {
        let mut settings = LayoutSnapSettings::default();
        settings.gap_override = "not a number".to_owned();
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

    #[test]
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
pub(in crate::presentation) struct LayoutSelectionProjection {
    pub context: TreeContext,
    pub part_ids: Vec<String>,
}

/// Retained cell coordinates let mode changes preserve the full matrix scope after Row/Column
/// contexts intentionally omit one coordinate. The root owns and clears this with its live owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct TreeCellAnchor {
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
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
pub(in crate::presentation) fn context_for_selection_kind(
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
        TreeContext::Board { .. }
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

fn snap_label(fraction: f64) -> &'static str {
    SNAP_STEPS
        .iter()
        .find(|(value, _, _)| *value == fraction)
        .map(|(_, _, label)| *label)
        .unwrap_or("Custom")
}

#[component]
pub(in crate::presentation) fn LayoutSelectionSnapToolbar(
    selection_kind: LayoutSelectionKind,
    snap_settings: LayoutSnapSettings,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let settings = snap_settings;
    rsx! {
        div { class: "m1-layout-command-controls", role: "toolbar", aria_label: "Layout commands",
            details { class: "m1-layout-select-menu",
                summary { "Select: {selection_kind.label()}" }
                div { class: "m1-layout-select-popover", role: "group", aria_label: "Selection scope",
                    for kind in LayoutSelectionKind::ALL {
                        button {
                            r#type: "button",
                            aria_pressed: "{kind == selection_kind}",
                            onclick: move |_| on_selection_kind.call(kind),
                            "{kind.label()}"
                        }
                    }
                }
            }
            details { class: "m1-layout-snap-menu",
                summary { "Snap" }
                div { class: "m1-layout-snap-popover",
                    label { "Snap increment"
                        select {
                            "aria-label": "Snap increment",
                            value: "{settings.snap_fraction}",
                            onchange: move |event: FormEvent| {
                                if let Ok(value) = event.value().parse::<f64>()
                                    && value.is_finite()
                                    && SNAP_STEPS.iter().any(|(step, _, _)| *step == value)
                                {
                                    on_snap_intent.call(LayoutSnapIntent::SetFraction(value));
                                }
                            },
                            for (fraction, value, label) in SNAP_STEPS {
                                option { value: "{value}", selected: settings.snap_fraction == fraction, "{label}" }
                            }
                        }
                    }
                    label { class: "m1-layout-snap-check",
                        input {
                            r#type: "checkbox",
                            checked: settings.geometry_snap,
                            onchange: move |event: FormEvent| {
                                on_snap_intent.call(LayoutSnapIntent::SetGeometrySnap(event.checked()));
                            },
                        }
                        "Geometry snap"
                    }
                    label { class: "m1-layout-snap-check",
                        input {
                            r#type: "checkbox",
                            checked: settings.gap_snap,
                            disabled: !settings.geometry_snap,
                            onchange: move |event: FormEvent| {
                                on_snap_intent.call(LayoutSnapIntent::SetGapSnap(event.checked()));
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
                                on_snap_intent.call(LayoutSnapIntent::SetGapOverride(event.value()));
                            },
                        }
                    }
                    p { "Shared by Layout, drawing and perimeter editing. Unit steps use matrix pitch; mm steps use world coordinates. Hold Alt to bypass snapping." }
                }
            }
        }
    }
}

#[component]
pub(in crate::presentation) fn LayoutSelectionSnapStatus(
    snap_settings: LayoutSnapSettings,
    active_part_position: Option<Vec2>,
) -> Element {
    let settings = snap_settings;
    let (x, y) = active_part_position
        .map(|position| (position.x, position.y))
        .unwrap_or((0.0, 0.0));
    rsx! {
        div { class: "m1-layout-coordinates", aria_label: "Selected part coordinates",
            span { "mm" }
            span { "X " b { "{x:.2}" } }
            span { "Y " b { "{y:.2}" } }
        }
        div { class: "m1-layout-snap-status",
            span { "Grid {snap_label(settings.snap_fraction)}" }
            span { if settings.geometry_snap { "Geometry snap on" } else { "Geometry snap off" } }
        }
    }
}
