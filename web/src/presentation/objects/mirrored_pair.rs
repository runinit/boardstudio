//! Private, source-parity projection for the new mirrored-pair placement.
use crate::matrix_setup_operation::MatrixSetupPreset;
use crate::mirrored_pair_geometry::MirroredPairProjection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Vec2;
use dioxus::prelude::*;

mod canvas_overlay;
pub(in crate::presentation) use canvas_overlay::MirroredPairCanvasOverlay;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MirroredPairRequest {
    pub owner: MirroredPairOwner,
    pub left_name: String,
    pub right_name: String,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
    pub gap_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MirroredPairOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MirroredPairFormProjection {
    pub owner: MirroredPairOwner,
    pub values: MirroredPairFormValues,
    pub editable: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MirroredPairFormValues {
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
            gap_mm: "24".into(),
        }
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct MirroredPairMount {
    pub form: Option<MirroredPairFormProjection>,
    pub placement: Option<MirroredPairPlacement>,
    pub can_open: bool,
    /// Owns the placement interaction from form admission through pending save settlement.
    pub owns_canvas: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_return_to_form: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
    pub on_move: EventHandler<MirroredPairMove>,
    pub on_commit: EventHandler<MirroredPairMove>,
    pub on_created: EventHandler<MirroredPairCreated>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MirroredPairPlacement {
    pub owner: MirroredPairOwner,
    pub pair: MirroredPairProjection,
    pub left_scene: boardstudio_core::model::MatrixScene,
    pub right_scene: boardstudio_core::model::MatrixScene,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MirroredPairMove {
    pub owner: MirroredPairOwner,
    pub center: Vec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MirroredPairCreated {
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
pub(in crate::presentation) struct MirroredPairFormProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub(in crate::presentation) fn MirroredPairForm(props: MirroredPairFormProps) -> Element {
    let values = props.projection.values.clone();
    let mut left_name = use_signal(|| values.left_name);
    let mut right_name = use_signal(|| values.right_name);
    let mut rows = use_signal(|| values.rows);
    let mut columns = use_signal(|| values.columns);
    let mut gap = use_signal(|| values.gap_mm);
    let mut preset = use_signal(|| values.preset);
    let left_name_value = left_name();
    let right_name_value = right_name();
    let rows_value = rows();
    let columns_value = columns();
    let gap_value = gap();
    let preset_value = preset();
    let dimensions = rows_value
        .trim()
        .parse::<u32>()
        .ok()
        .zip(columns_value.trim().parse::<u32>().ok())
        .filter(|(rows, columns)| {
            *rows > 0
                && *columns > 0
                && rows
                    .checked_mul(*columns)
                    .is_some_and(|cells| cells <= 4096)
        });
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
    let preset_submit = preset_value;
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
            p { "Linked key assemblies, diode settings and components. Substitute a component on either half when needed." }
            form { onsubmit: submit,
                label { "Left layout"
                    input { aria_label: "Left layout name", required: true, onmounted: move |event| async move { let _ = event.set_focus(true).await; }, value: "{left_name_value}", disabled: !props.projection.editable, oninput: move |event| left_name.set(event.value()) }
                }
                label { "Right layout"
                    input { aria_label: "Right layout name", required: true, value: "{right_name_value}", disabled: !props.projection.editable, oninput: move |event| right_name.set(event.value()) }
                }
                label { "Rows per half"
                    input { aria_label: "Mirrored pair rows", r#type: "number", min: "1", max: "4096", step: "1", required: true, value: "{rows_value}", disabled: !props.projection.editable, oninput: move |event| rows.set(event.value()) }
                }
                label { "Columns per half"
                    input { aria_label: "Mirrored pair columns", r#type: "number", min: "1", max: "4096", step: "1", required: true, value: "{columns_value}", disabled: !props.projection.editable, oninput: move |event| columns.set(event.value()) }
                }
                label { "Key assembly"
                    select { aria_label: "Mirrored pair key assembly", value: preset_value.as_str(), disabled: !props.projection.editable,
                        onchange: move |event| if let Some(next) = parse_preset(&event.value()) { preset.set(next); },
                        option { value: "mx-solder", "MX solder" }
                        option { value: "mx-hotswap", "MX hotswap" }
                        option { value: "choc-solder", "Choc V1 Solder" }
                        option { value: "choc-hotswap", "Choc V1 Hotswap" }
                        option { value: "mx-rgb", "MX RGB" }
                        option { value: "choc-rgb", "Choc V1 RGB" }
                        option { value: "mx-hotswap-rgb", "MX hotswap + RGB" }
                        option { value: "choc-hotswap-rgb", "Choc V1 Hotswap + RGB" }
                    }
                }
                label { "Gap between key edges (mm)"
                    input { aria_label: "Mirrored pair gap", r#type: "number", min: "0", step: "any", required: true, value: "{gap_value}", disabled: !props.projection.editable, oninput: move |event| gap.set(event.value()) }
                }
                p { "Edit either half to update both. Unlink in the inspector for independent geometry." }
                if let Some(error) = error { p { role: "alert", class: "m1-mirrored-pair-error", "{error}" } }
                if let Some(status) = status { p { role: "status", class: "m1-mirrored-pair-status", "{status}" } }
                footer {
                    button { r#type: "button", onclick: move |_| on_cancel.call(owner.clone()), "Cancel" }
                    button { r#type: "submit", disabled: !can_preview, "Preview placement" }
                }
            }
        }
    }
}

fn parse_preset(value: &str) -> Option<MatrixSetupPreset> {
    Some(match value {
        "mx-solder" => MatrixSetupPreset::MxSolder,
        "mx-hotswap" => MatrixSetupPreset::MxHotswap,
        "choc-solder" => MatrixSetupPreset::ChocSolder,
        "choc-hotswap" => MatrixSetupPreset::ChocHotswap,
        "mx-rgb" => MatrixSetupPreset::MxRgb,
        "choc-rgb" => MatrixSetupPreset::ChocRgb,
        "mx-hotswap-rgb" => MatrixSetupPreset::MxHotswapRgb,
        "choc-hotswap-rgb" => MatrixSetupPreset::ChocHotswapRgb,
        _ => return None,
    })
}
