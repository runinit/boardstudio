//! Private, source-parity projection for the new mirrored-pair placement.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
    parse_matrix_dimensions,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
use crate::mirrored_pair_geometry::MirroredPairProjection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Vec2;
use boardstudio_web_ui_shared::controls::BoardButton;
use dioxus::prelude::*;
use std::collections::HashSet;

mod canvas_overlay;
pub use canvas_overlay::MirroredPairCanvasOverlay;

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairRequest {
    pub owner: MirroredPairOwner,
    pub left_name: String,
    pub right_name: String,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormProjection {
    pub owner: MirroredPairOwner,
    pub values: MirroredPairFormValues,
    pub editable: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormValues {
    pub left_name: String,
    pub right_name: String,
    pub rows: String,
    pub columns: String,
    pub preset: MatrixSetupPreset,
    pub gap_mm: String,
}

impl Default for MirroredPairFormValues {
    fn default() -> Self {
        Self {
            left_name: "Left half".into(),
            right_name: "Right half".into(),
            rows: "3".into(),
            columns: "5".into(),
            preset: MatrixSetupPreset::MxSolder,
            gap_mm: "40".into(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct MirroredPairMount {
    pub form: Option<MirroredPairFormProjection>,
    pub placement: Option<MirroredPairPlacement>,
    pub can_open: bool,
    /// Owns the placement interaction from form admission through pending save settlement.
    pub owns_canvas: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
    pub on_move: EventHandler<MirroredPairMove>,
    pub on_commit: EventHandler<MirroredPairMove>,
    pub on_created: EventHandler<MirroredPairCreated>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairPlacement {
    pub owner: MirroredPairOwner,
    pub pair: MirroredPairProjection,
    pub left_scene: boardstudio_core::model::MatrixScene,
    pub right_scene: boardstudio_core::model::MatrixScene,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairMove {
    pub owner: MirroredPairOwner,
    pub center: Vec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairCreated {
    pub owner: MirroredPairOwner,
    pub result_token: SnapshotToken,
    pub result_revision: u64,
    pub scope: Scope,
    pub left_layout_id: String,
    pub right_layout_id: String,
    pub left_matrix_id: String,
    pub right_matrix_id: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct MirroredPairFormProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub fn MirroredPairForm(props: MirroredPairFormProps) -> Element {
    let values = props.projection.values.clone();
    let mut left_name = use_signal(|| values.left_name);
    let mut right_name = use_signal(|| values.right_name);
    let rows = use_signal(|| values.rows);
    let columns = use_signal(|| values.columns);
    let mut gap = use_signal(|| values.gap_mm);
    let (choc_default, hotswap_default, rgb_default) = preset_choices(values.preset);
    let family = use_signal(|| Some(HashSet::from([usize::from(choc_default)])));
    let mounting = use_signal(|| Some(HashSet::from([usize::from(hotswap_default)])));
    let lighting = use_signal(|| Some(HashSet::from([usize::from(rgb_default)])));
    let row_slider = use_signal(|| {
        Some(
            rows()
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| (1.0..=12.0).contains(value))
                .unwrap_or(3.0),
        )
    });
    let column_slider = use_signal(|| {
        Some(
            columns()
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| (1.0..=18.0).contains(value))
                .unwrap_or(5.0),
        )
    });
    let left_name_value = left_name();
    let right_name_value = right_name();
    let rows_value = rows();
    let columns_value = columns();
    let gap_value = gap();
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
    let dimensions = parse_matrix_dimensions(&rows_value, &columns_value);
    let gap_mm = gap_value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0);
    let valid_names = !left_name_value.trim().is_empty()
        && !right_name_value.trim().is_empty()
        && left_name_value.trim() != right_name_value.trim();
    let can_preview =
        props.projection.editable && dimensions.is_some() && gap_mm.is_some() && valid_names;
    let owner = props.projection.owner.clone();
    let on_preview = props.on_preview;
    let left_submit = left_name_value.clone();
    let right_submit = right_name_value.clone();
    let preset_submit = selected_preset;
    let gap_submit = gap_value.clone();
    let owner_submit = owner.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        let Some((rows, columns)) = dimensions else {
            return;
        };
        let Some(gap_mm) = gap_submit
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
        else {
            return;
        };
        let left_name = left_submit.trim().to_owned();
        let right_name = right_submit.trim().to_owned();
        if left_name.is_empty() || right_name.is_empty() || left_name == right_name {
            return;
        }
        on_preview.call(MirroredPairRequest {
            owner: owner_submit.clone(),
            left_name,
            right_name,
            rows,
            columns,
            preset: preset_submit,
            gap_mm,
        });
    };
    let error = props.projection.error.clone();
    let status = props.projection.status.clone();
    let on_cancel = props.on_cancel;

    rsx! {
        section { class: "m1-matrix-setup m1-mirrored-pair-setup", aria_label: "New mirrored pair",
            header { class: "m1-matrix-setup-heading", h2 { "Mirrored pair" } }
            p { class: "m1-mirrored-pair-intro", "Linked key assemblies, diode settings and components. Substitute a component on either half when needed." }
            form { onsubmit: submit,
                div { class: "m1-mirrored-pair-fields",
                    label { "Left layout"
                        input { aria_label: "Left layout name", required: true, onmounted: move |event| async move { let _ = event.set_focus(true).await; }, value: "{left_name_value}", disabled: !props.projection.editable, oninput: move |event| left_name.set(event.value()) }
                    }
                    label { "Right layout"
                        input { aria_label: "Right layout name", required: true, value: "{right_name_value}", disabled: !props.projection.editable, oninput: move |event| right_name.set(event.value()) }
                    }
                }
                MatrixDimensionSliders {
                    rows: rows,
                    columns: columns,
                    row_slider: row_slider,
                    column_slider: column_slider,
                    rows_label: "Rows per half",
                    columns_label: "Columns per half",
                    editable: props.projection.editable,
                }
                MatrixAssemblyPicker {
                    family: family,
                    mounting: mounting,
                    lighting: lighting,
                    editable: props.projection.editable,
                }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                label { "Gap between key edges (mm)"
                    input { aria_label: "Gap between key edges (mm)", r#type: "number", min: "0", step: "any", required: true, value: "{gap_value}", disabled: !props.projection.editable, oninput: move |event| gap.set(event.value()) }
                }
                p { class: "m1-mirrored-pair-policy", "Edit either half to update both. Unlink in the inspector for independent geometry." }
                if let Some(error) = error { p { role: "alert", class: "m1-mirrored-pair-error", "{error}" } }
                if let Some(status) = status { p { role: "status", class: "m1-mirrored-pair-status", "{status}" } }
                footer { class: "m1-mirrored-pair-actions",
                    BoardButton { r#type: "button", onclick: move |_| on_cancel.call(owner.clone()), "Cancel" }
                    button { r#type: "submit", disabled: !can_preview, "Preview placement" }
                }
            }
        }
    }
}

fn preset_choices(preset: MatrixSetupPreset) -> (bool, bool, bool) {
    match preset {
        MatrixSetupPreset::MxSolder => (false, false, false),
        MatrixSetupPreset::MxHotswap => (false, true, false),
        MatrixSetupPreset::ChocSolder => (true, false, false),
        MatrixSetupPreset::ChocHotswap => (true, true, false),
        MatrixSetupPreset::MxRgb => (false, false, true),
        MatrixSetupPreset::ChocRgb => (true, false, true),
        MatrixSetupPreset::MxHotswapRgb => (false, true, true),
        MatrixSetupPreset::ChocHotswapRgb => (true, true, true),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "mirrored_pair/canvas_overlay_tests.rs"]
mod canvas_overlay_tests;
