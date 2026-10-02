//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use crate::matrix_setup_operation::MatrixSetupPreset;
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixSetupOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixSetupProjection {
    pub owner: MatrixSetupOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixSetupCreateRequest {
    pub owner: MatrixSetupOwner,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct MatrixSetupMount {
    pub projection: Option<MatrixSetupProjection>,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct MatrixSetupProps {
    pub projection: MatrixSetupProjection,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[component]
pub(in crate::presentation) fn MatrixSetup(props: MatrixSetupProps) -> Element {
    let mut rows = use_signal(String::new);
    let mut columns = use_signal(String::new);
    let mut preset = use_signal(|| MatrixSetupPreset::MxSolder);
    let rows_value = rows();
    let columns_value = columns();
    let preview = rows_value
        .parse::<u32>()
        .ok()
        .zip(columns_value.parse::<u32>().ok())
        .filter(|(rows, columns)| {
            *rows > 0
                && *columns > 0
                && rows
                    .checked_mul(*columns)
                    .is_some_and(|cells| cells <= 4096)
        });
    let selected_preset = preset();
    let owner = props.projection.owner.clone();
    let feedback_error = props.projection.error.clone();
    let feedback_status = props.projection.status.clone();
    let create_handler = props.on_create;
    let submit_owner = owner.clone();
    let submit_rows = rows_value.clone();
    let submit_columns = columns_value.clone();
    let on_submit = move |event: FormEvent| {
        event.prevent_default();
        let Ok(rows) = submit_rows.trim().parse::<u32>() else {
            return;
        };
        let Ok(columns) = submit_columns.trim().parse::<u32>() else {
            return;
        };
        create_handler.call(MatrixSetupCreateRequest {
            owner: submit_owner.clone(),
            rows,
            columns,
            preset: selected_preset,
        });
    };
    let on_cancel = props.on_cancel;

    rsx! {
        section { class: "m1-matrix-setup", aria_label: "New matrix",
            header { class: "m1-matrix-setup-heading",
                h2 { "New matrix" }
                span { "{owner.board_id}" }
            }
            form { onsubmit: on_submit,
                label { "Rows"
                    input {
                        aria_label: "New matrix rows",
                        r#type: "number", min: "1", max: "4096", step: "1", required: true,
                        value: "{rows_value}", disabled: !props.projection.editable,
                        oninput: move |event| rows.set(event.value()),
                    }
                }
                label { "Columns"
                    input {
                        aria_label: "New matrix columns",
                        r#type: "number", min: "1", max: "4096", step: "1", required: true,
                        value: "{columns_value}", disabled: !props.projection.editable,
                        oninput: move |event| columns.set(event.value()),
                    }
                }
                label { "Key assembly"
                    select {
                        aria_label: "New matrix assembly",
                        value: selected_preset.as_str(),
                        disabled: !props.projection.editable,
                        onchange: move |event| {
                            preset.set(MatrixSetupPreset::from_str(event.value().as_str()).unwrap_or(MatrixSetupPreset::MxSolder));
                        },
                        option { value: "mx-solder", "MX solder" }
                        option { value: "mx-hotswap", "MX hotswap" }
                        option { value: "choc-solder", "Choc V1 Solder" }
                        option { value: "choc-hotswap", "Choc V1 Hotswap" }
                        option { value: "mx-rgb", "MX RGB" }
                        option { value: "choc-rgb", "Choc V1 RGB" }
                        option { value: "mx-hotswap-rgb", "MX hotswap + RGB" }
                        option { value: "choc-hotswap-rgb", "Choc V1 Hotswap RGB" }
                    }
                }
                if let Some((preview_rows, preview_columns)) = preview {
                    figure { class: "wb-matrix-preview",
                        svg {
                            view_box: "-1 -1 {preview_columns * 10 + 1} {preview_rows * 10 + 1}",
                            role: "img",
                            "aria-label": "{preview_rows} rows by {preview_columns} columns matrix preview",
                            for index in 0..(preview_rows * preview_columns) {
                                rect {
                                    key: "{index}",
                                    x: "{(index % preview_columns) * 10}",
                                    y: "{(index / preview_columns) * 10}",
                                    width: "8", height: "8", rx: "1",
                                }
                            }
                        }
                        figcaption { "{preview_rows * preview_columns} keys · {preview_rows} × {preview_columns}" }
                    }
                }
                p { "Starts at X 0, Y 0. Move and edit it on the canvas." }
                if let Some(message) = feedback_error {
                    p { class: "m1-matrix-setup-error", role: "alert", "{message}" }
                }
                if let Some(message) = feedback_status {
                    p { class: "m1-matrix-setup-status", role: "status", "{message}" }
                }
                footer {
                    button { r#type: "button", disabled: !props.projection.can_cancel,
                        onclick: move |_| on_cancel.call(owner.clone()), "Cancel"
                    }
                    button { r#type: "submit", disabled: !props.projection.editable,
                        "Create matrix"
                    }
                }
            }
        }
    }
}

impl MatrixSetupPreset {
    fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "mx-solder" => Self::MxSolder,
            "mx-hotswap" => Self::MxHotswap,
            "choc-solder" => Self::ChocSolder,
            "choc-hotswap" => Self::ChocHotswap,
            "mx-rgb" => Self::MxRgb,
            "choc-rgb" => Self::ChocRgb,
            "mx-hotswap-rgb" => Self::MxHotswapRgb,
            "choc-hotswap-rgb" => Self::ChocHotswapRgb,
            _ => return None,
        })
    }
}
