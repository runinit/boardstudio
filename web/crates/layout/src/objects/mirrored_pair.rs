//! Private, source-parity projection for the new mirrored-pair placement.
use crate::matrix_setup_operation::MatrixSetupPreset;
use crate::mirrored_pair_geometry::MirroredPairProjection;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Vec2;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::{
    field::{Field, FieldLabel},
    select::{Select, SelectOption},
};
use boardstudio_web_ui_shared::ui::{
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
};

use dioxus::prelude::*;
use dioxus_web::WebEventExt;

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
    let mut rows = use_signal(|| values.rows);
    let mut columns = use_signal(|| values.columns);
    let mut gap = use_signal(|| values.gap_mm);
    let mut preset = use_signal(|| values.preset);
    let left_name_value = left_name();
    let right_name_value = right_name();
    let rows_value = rows();
    let columns_value = columns();
    let gap_value = gap();
    let preset_choice = use_memo(move || Some(preset().as_str().to_owned()));
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
                section {
                    class: "m1-matrix-setup m1-mirrored-pair-setup",
                    aria_label: "New mirrored pair",
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key().to_string() == " " {
                            event.stop_propagation();
                            if let Some(raw) = event.data().try_as_web_event() {
                                raw.stop_propagation();
                            }
                        }
                    },
                    header { class: "m1-matrix-setup-heading",
                        h2 { "Mirrored pair" }
                    }
                    p { class: "m1-mirrored-pair-intro",
                        "Linked key assemblies, diode settings and components. Substitute a component on either half when needed."
                    }
                    form { onsubmit: submit,
                        div { class: "m1-mirrored-pair-fields",
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-5",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    Field {
                                        FieldLabel { html_for: control_input_id.clone(), "Left layout" }
                                        InputGroup {
                                            InputGroupInput { aria_label: "Left layout name",
    required: true,
    value: "{left_name_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| left_name.set(event.value()),
    id: control_input_id.clone(),
    attributes: vec![dioxus::events::onmounted(move |event: MountedEvent| async move { let _ = event.set_focus(true).await; })], }
                                        }
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-4",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    Field {
                                        FieldLabel { html_for: control_input_id.clone(), "Right layout" }
                                        InputGroup {
                                            InputGroupInput { aria_label: "Right layout name",
    required: true,
    value: "{right_name_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| right_name.set(event.value()),
    id: control_input_id.clone(), }
                                        }
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-3",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    Field {
                                        FieldLabel { html_for: control_input_id.clone(), "Rows per half" }
                                        InputGroup {
                                            InputGroupInput { aria_label: "Rows per half",
    r#type: InputType::Number,
    min: "1",
    max: "4096",
    step: "1",
    required: true,
    value: "{rows_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| rows.set(event.value()),
    id: control_input_id.clone(), }
                                        }
                                    }
                                }
                            }
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-2",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    Field {
                                        FieldLabel { html_for: control_input_id.clone(), "Columns per half" }
                                        InputGroup {
                                            InputGroupInput { aria_label: "Columns per half",
    r#type: InputType::Number,
    min: "1",
    max: "4096",
    step: "1",
    required: true,
    value: "{columns_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| columns.set(event.value()),
    id: control_input_id.clone(), }
                                        }
                                    }
                                }
                            }
                        }
                        {
                            let control_label = "Key assembly".to_owned();
                            let control_aria_label = "Key assembly".to_owned();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let control_choices = [
                                ("mx-solder".to_owned(), "MX Solder".to_owned(), false),
                                ("mx-hotswap".to_owned(), "MX Hotswap".to_owned(), false),
                                ("choc-solder".to_owned(), "Choc V1 Solder".to_owned(), false),
                                ("choc-hotswap".to_owned(), "Choc V1 Hotswap".to_owned(), false),
                                ("mx-rgb".to_owned(), "MX RGB".to_owned(), false),
                                ("choc-rgb".to_owned(), "Choc V1 RGB".to_owned(), false),
                                ("mx-hotswap-rgb".to_owned(), "MX Hotswap RGB".to_owned(), false),
                                ("choc-hotswap-rgb".to_owned(), "Choc V1 Hotswap RGB".to_owned(), false),
                            ];
                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
            let Some(choice) = choice else {
                return;
            };
            if let Some(next) = parse_preset(&choice) {
                preset.set(next);
            }
        });
        rsx! {

                                Field { class: "",
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    Select::<String> {
                                        id: control_id,
                                        aria_label: control_aria_label,
                                        value: Some(preset_choice.into()),
                                        disabled: !props.projection.editable,
                                        on_value_change,
                                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                            SelectOption::<String> {
                                                index,
                                                value: value.clone(),
                                                text_value: label.clone(),
                                                disabled: *disabled,
                                                "{label}"
                                            }
                                        }
                                    }
                                }

                            }
                        }
                        {
                            let control_input_id = format!(
                                "layout-input-{}-1",
                                dioxus::core::current_scope_id().0,
                            );
                            rsx! {
                                Field {
                                    FieldLabel { html_for: control_input_id.clone(), "Gap between key edges (mm)" }
                                    InputGroup {
                                        InputGroupInput { aria_label: "Gap between key edges (mm)",
    r#type: InputType::Number,
    min: "0",
    step: "any",
    required: true,
    value: "{gap_value}",
    disabled: !props.projection.editable,
    oninput: move |event: FormEvent| gap.set(event.value()),
    id: control_input_id.clone(), }
                                    }
                                }
                            }
                        }
                        p { class: "m1-mirrored-pair-policy",
                            "Edit either half to update both. Unlink in the inspector for independent geometry."
                        }
                        if let Some(error) = error {
                            Alert { class: "m1-mirrored-pair-error",
        variant: AlertVariant::Destructive,AlertDescription {  "{error}"  } }
                        }
                        if let Some(status) = status {
                            div { role: "status",  Callout { class: "m1-mirrored-pair-status", variant: CalloutVariant::Info,  "{status}"  } }
                        }
                        footer { class: "m1-mirrored-pair-actions",
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                onclick: move |_| on_cancel.call(owner.clone()),
                                "Cancel"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "submit",
                                disabled: !can_preview,
                                "Preview placement"
                            }
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

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "mirrored_pair/canvas_overlay_tests.rs"]
mod canvas_overlay_tests;
