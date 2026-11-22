//! Private, source-parity projection for the new mirrored-pair placement.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
    parse_matrix_dimensions,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
use crate::mirrored_pair_geometry::MirroredPairProjection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::Vec2;
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
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
    pub left_scene: boardstudio_model::model::MatrixScene,
    pub right_scene: boardstudio_model::model::MatrixScene,
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
    let mut gap = use_signal(|| {
        values
            .gap_mm
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or_default()
    });
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
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
    let dimensions = parse_matrix_dimensions(&rows_value, &columns_value);
    let valid_names = !left_name_value.trim().is_empty()
        && !right_name_value.trim().is_empty()
        && left_name_value.trim() != right_name_value.trim();
    let can_preview = props.projection.editable && dimensions.is_some() && valid_names;
    let owner = props.projection.owner.clone();
    let on_preview = props.on_preview;
    let left_submit = left_name_value.clone();
    let right_submit = right_name_value.clone();
    let preset_submit = selected_preset;
    let owner_submit = owner.clone();
    let submit = move |event: FormEvent| {
        event.prevent_default();
        let Some((rows, columns)) = dimensions else {
            return;
        };
        let gap_mm = gap();
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

    let left_id = format!("layout-input-{}-left", dioxus::core::current_scope_id().0);
    let right_id = format!("layout-input-{}-right", dioxus::core::current_scope_id().0);
    let gap_id = format!("layout-input-{}-gap", dioxus::core::current_scope_id().0);
    rsx! {
        section {
            class: "pointer-events-auto relative box-border flex max-h-full w-[min(380px,100%)] flex-col gap-0 overflow-y-auto rounded-md border border-border-strong bg-card p-6 text-foreground shadow-md max-[760px]:w-[min(380px,calc(100%-8px))] max-[760px]:p-[18px]",
            aria_label: "New mirrored pair",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                }
            },
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-xl!", h2 { "Mirrored pair" } }
            p { class: "mt-2.5 mb-5 text-[13px] leading-normal text-muted-foreground",
                "Linked key assemblies, diode settings and components. Substitute a component on either half when needed."
            }
            form { onsubmit: submit, class: "grid gap-2",
                div { class: "grid grid-cols-2 gap-x-4 gap-y-3", role: "group", aria_label: "Mirrored pair dimensions",
                    Field {
                        FieldLabel { html_for: left_id.clone(), "Left layout" }
                        InputGroup { class: "max-[760px]:h-control-touch",
                            InputGroupInput {
                                class: "max-[760px]:h-control-touch", aria_label: "Left layout name", id: left_id,
                                required: true, value: "{left_name_value}", disabled: !props.projection.editable,
                                oninput: move |event: FormEvent| left_name.set(event.value()),
                                attributes: vec![dioxus::events::onmounted(move |event: MountedEvent| async move { let _ = event.set_focus(true).await; })],
                            }
                        }
                    }
                    Field {
                        FieldLabel { html_for: right_id.clone(), "Right layout" }
                        InputGroup { class: "max-[760px]:h-control-touch",
                            InputGroupInput {
                                class: "max-[760px]:h-control-touch", aria_label: "Right layout name", id: right_id,
                                required: true, value: "{right_name_value}", disabled: !props.projection.editable,
                                oninput: move |event: FormEvent| right_name.set(event.value()),
                            }
                        }
                    }
                }
                MatrixDimensionSliders {
                    rows, columns, row_slider, column_slider,
                    rows_label: "Rows per half", columns_label: "Columns per half", editable: props.projection.editable,
                }
                MatrixAssemblyPicker { family, mounting, lighting, editable: props.projection.editable }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                NumericField {
                    id: gap_id, label: "Gap between key edges (mm)", aria_label: "Gap between key edges (mm)",
                    min: Some(0.0), value: gap(),
                    disabled_reason: (!props.projection.editable).then(|| "Placement is unavailable".to_owned()),
                    on_commit: move |value| gap.set(value),
                }
                p { class: "mt-2.5 mb-5 border-t border-border pt-3.5 text-[13px] leading-normal text-muted-foreground",
                    "Edit either half to update both. Unlink in the inspector for independent geometry."
                }
                if let Some(error) = error { Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } } }
                if let Some(status) = status { div { role: "status", Callout { variant: CalloutVariant::Info, "{status}" } } }
                footer { class: "flex justify-end gap-2",
                    Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel.call(owner.clone()), "Cancel" }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !can_preview, "Preview placement" }
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
