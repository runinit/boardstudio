//! Shared Add-matrix field components for the single and mirrored-pair modes.
//!
//! Both the single-matrix form and the mirrored-pair form render these sliders,
//! assembly toggles and SVG preview so the two modes never drift apart again.
use crate::matrix_setup_operation::MatrixSetupPreset;
use dioxus::prelude::*;
use dioxus_primitives::{
    slider::{Slider, SliderRange, SliderThumb, SliderTrack},
    toggle_group::{ToggleGroup, ToggleItem},
};
use std::collections::HashSet;

/// Parse trimmed row/column text into dimensions capped at 4096 cells.
pub fn parse_matrix_dimensions(rows: &str, columns: &str) -> Option<(u32, u32)> {
    rows.trim()
        .parse::<u32>()
        .ok()
        .zip(columns.trim().parse::<u32>().ok())
        .filter(|(rows, columns)| {
            *rows > 0
                && *columns > 0
                && rows
                    .checked_mul(*columns)
                    .is_some_and(|cells| cells <= 4096)
        })
}

/// Derive the catalogue preset from the assembly toggle choices.
pub fn matrix_setup_preset(choc: bool, hotswap: bool, rgb: bool) -> MatrixSetupPreset {
    match (choc, hotswap, rgb) {
        (false, false, false) => MatrixSetupPreset::MxSolder,
        (false, true, false) => MatrixSetupPreset::MxHotswap,
        (true, false, false) => MatrixSetupPreset::ChocSolder,
        (true, true, false) => MatrixSetupPreset::ChocHotswap,
        (false, false, true) => MatrixSetupPreset::MxRgb,
        (true, false, true) => MatrixSetupPreset::ChocRgb,
        (false, true, true) => MatrixSetupPreset::MxHotswapRgb,
        (true, true, true) => MatrixSetupPreset::ChocHotswapRgb,
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixDimensionSlidersProps {
    pub rows: Signal<String>,
    pub columns: Signal<String>,
    pub row_slider: Signal<Option<f64>>,
    pub column_slider: Signal<Option<f64>>,
    pub rows_label: &'static str,
    pub columns_label: &'static str,
    pub editable: bool,
}

/// Rows (1-12) and columns (1-18) sliders shared by both Add-matrix modes.
#[component]
pub fn MatrixDimensionSliders(props: MatrixDimensionSlidersProps) -> Element {
    let rows_value = (props.rows)();
    let columns_value = (props.columns)();
    let mut rows = props.rows;
    let mut columns = props.columns;
    let mut row_slider = props.row_slider;
    let mut column_slider = props.column_slider;
    rsx! {
        div { class: "m1-matrix-dimension",
            label { "{props.rows_label}" strong { "{rows_value}" } }
            Slider {
                label: props.rows_label, value: row_slider, min: 1.0, max: 12.0, step: 1.0,
                disabled: !props.editable,
                on_value_change: move |value: f64| {
                    let value = value.round().clamp(1.0, 12.0);
                    row_slider.set(Some(value)); rows.set(format!("{value:.0}"));
                },
                SliderTrack { class: "m1-matrix-slider-track",
                    SliderRange { class: "m1-matrix-slider-range" }
                    SliderThumb { class: "m1-matrix-slider-thumb" }
                }
            }
        }
        div { class: "m1-matrix-dimension",
            label { "{props.columns_label}" strong { "{columns_value}" } }
            Slider {
                label: props.columns_label, value: column_slider, min: 1.0, max: 18.0, step: 1.0,
                disabled: !props.editable,
                on_value_change: move |value: f64| {
                    let value = value.round().clamp(1.0, 18.0);
                    column_slider.set(Some(value)); columns.set(format!("{value:.0}"));
                },
                SliderTrack { class: "m1-matrix-slider-track",
                    SliderRange { class: "m1-matrix-slider-range" }
                    SliderThumb { class: "m1-matrix-slider-thumb" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixAssemblyPickerProps {
    pub family: Signal<Option<HashSet<usize>>>,
    pub mounting: Signal<Option<HashSet<usize>>>,
    pub lighting: Signal<Option<HashSet<usize>>>,
    pub editable: bool,
}

/// Key assembly toggles shared by both Add-matrix modes.
#[component]
pub fn MatrixAssemblyPicker(props: MatrixAssemblyPickerProps) -> Element {
    let mut family = props.family;
    let mut mounting = props.mounting;
    let mut lighting = props.lighting;
    rsx! {
        div { class: "m1-matrix-assembly-picker",
            h3 { "Key assembly" }
            ToggleGroup {
                aria_label: "New matrix assembly",
                pressed: family,
                allow_multiple_pressed: false,
                disabled: !props.editable,
                on_pressed_change: move |pressed: HashSet<usize>| {
                    if !pressed.is_empty() { family.set(Some(pressed)); }
                },
                for (index, label) in ["MX", "Choc"].iter().enumerate() {
                    ToggleItem { index, class: "m1-matrix-assembly-choice",
                        svg { view_box: "0 0 32 32", "aria-hidden": "true",
                            rect { x: "4", y: if index == 0 { "5" } else { "12" }, width: "24", height: if index == 0 { "16" } else { "9" }, rx: "3" }
                            path { d: "M8 25h16M12 21v4m8-4v4" }
                        }
                        span { strong { "{label}" } small { if index == 0 { "Full height" } else { "Low profile" } } }
                    }
                }
            }
            h3 { "Mounting" }
            ToggleGroup {
                aria_label: "Switch mounting", pressed: mounting, allow_multiple_pressed: false,
                disabled: !props.editable,
                on_pressed_change: move |choice: HashSet<usize>| if !choice.is_empty() { mounting.set(Some(choice)); },
                ToggleItem { index: 0usize, "Solder" }
                ToggleItem { index: 1usize, "Hotswap" }
            }
            h3 { "Lighting" }
            ToggleGroup {
                aria_label: "Key lighting", pressed: lighting, allow_multiple_pressed: false,
                disabled: !props.editable,
                on_pressed_change: move |choice: HashSet<usize>| if !choice.is_empty() { lighting.set(Some(choice)); },
                ToggleItem { index: 0usize, "None" }
                ToggleItem { index: 1usize, "Per-key RGB" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixPreviewProps {
    pub rows: String,
    pub columns: String,
}

/// SVG key-grid preview shared by both Add-matrix modes.
#[component]
pub fn MatrixPreview(props: MatrixPreviewProps) -> Element {
    let preview = parse_matrix_dimensions(&props.rows, &props.columns);
    rsx! {
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
    }
}
