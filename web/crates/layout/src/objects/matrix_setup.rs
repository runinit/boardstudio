//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use crate::matrix_setup_operation::MatrixSetupPreset;
pub use crate::presentation::parts::MatrixPlacementSource;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};

use boardstudio_web_ui_shared::ui::{
    slider::Slider,
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupProjection {
    pub owner: MatrixSetupOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupCreateRequest {
    pub owner: MatrixSetupOwner,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}

#[derive(Clone, PartialEq)]
pub struct MatrixSetupMount {
    pub projection: Option<MatrixSetupProjection>,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixPlacementOwner {
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementProjection {
    pub owner: MatrixPlacementOwner,
    pub matrix: boardstudio_core::model::Matrix,
    pub scene: boardstudio_core::model::MatrixScene,
    pub definitions: Vec<boardstudio_core::model::PartDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementMove {
    pub owner: MatrixPlacementOwner,
    pub center: boardstudio_core::model::Vec2,
}

#[derive(Clone, PartialEq)]
pub struct MatrixPlacementMount {
    pub cancel_owner: Option<MatrixPlacementOwner>,
    pub placement: Option<MatrixPlacementProjection>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_place: EventHandler<MatrixPlacementSource>,
    pub on_move: EventHandler<MatrixPlacementMove>,
    pub on_commit: EventHandler<MatrixPlacementMove>,
    pub on_cancel: EventHandler<MatrixPlacementOwner>,
}

#[derive(Clone)]
pub struct MatrixPlacementInput {
    pub version: Signal<u64>,
    pub selected_context: Signal<Option<super::ScopedTreeContext>>,
    pub anchor_scope: Signal<Option<Scope>>,
    pub workspace: Signal<&'static str>,
    pub scope_generation: Signal<u64>,
    pub assembly_orientation: crate::presentation::parts::PartsAssemblyOrientation,
    pub canvas_interaction: crate::presentation::canvas_interaction::CanvasInteractionArbiter,
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixSetupProps {
    pub projection: MatrixSetupProjection,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

#[component]
pub fn MatrixSetup(props: MatrixSetupProps) -> Element {
    let mut rows = use_signal(|| "5".to_owned());
    let mut columns = use_signal(|| "6".to_owned());
    let mut row_slider = use_signal(|| Some(5.0));
    let mut column_slider = use_signal(|| Some(6.0));
    let mut family = use_signal(|| Some(HashSet::from([0usize])));
    let mut mounting = use_signal(|| Some(HashSet::from([0usize])));
    let mut lighting = use_signal(|| Some(HashSet::from([0usize])));
    let return_focus = use_signal(|| None::<web_sys::HtmlElement>);
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
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = match (choc, hotswap, rgb) {
        (false, false, false) => MatrixSetupPreset::MxSolder,
        (false, true, false) => MatrixSetupPreset::MxHotswap,
        (true, false, false) => MatrixSetupPreset::ChocSolder,
        (true, true, false) => MatrixSetupPreset::ChocHotswap,
        (false, false, true) => MatrixSetupPreset::MxRgb,
        (true, false, true) => MatrixSetupPreset::ChocRgb,
        (false, true, true) => MatrixSetupPreset::MxHotswapRgb,
        (true, true, true) => MatrixSetupPreset::ChocHotswapRgb,
    };
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
    let cancel_handler = props.on_cancel;
    let cancel_owner = owner.clone();
    let on_cancel = move |_| {
        cancel_handler.call(cancel_owner.clone());
        let return_focus = return_focus();
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            if let Some(target) = return_focus.filter(|target| target.is_connected()) {
                let _ = target.focus();
            } else if let Some(target) = document
                .get_element_by_id("m1-setup-guide-heading")
                .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = target.focus();
            }
        });
    };
    let mut return_focus_on_mount = return_focus;

    rsx! {
                section { class: "absolute inset-x-2 top-[100px] bottom-2 z-6 flex flex-col gap-2.5 overflow-auto rounded-md border border-border-strong bg-card p-3 text-foreground shadow-md [&_form]:grid [&_form]:gap-2 [&_footer]:flex [&_footer]:justify-end [&_footer]:gap-2", aria_label: "New matrix",
                onkeydown: move |event: KeyboardEvent| {
                    if event.key().to_string() == " " {
                        event.stop_propagation();
                        if let Some(raw) = event.data().try_as_web_event() {
                            raw.stop_propagation();
                        }
                    }
                },
                    header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-[16px]!",
                        h2 { "New matrix" }
                    }
                    form { onsubmit: on_submit,
                        div { class: "grid grid-cols-1 gap-1 [&>label]:flex [&>label]:justify-between [&>label]:font-semibold [&>[role=group]]:w-full",
                            label {
                                "Rows"
                                strong { "{rows_value}" }
                            }
                            div {
                                id: "m1-matrix-rows-focus",
                                onmounted: move |_| async move {
                                    if let Some(target) = web_sys::window()
                                        .and_then(|window| window.document())
                                        .and_then(|document| document.active_element())
                                        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        return_focus_on_mount.set(Some(target));
                                    }
                                    if let Some(target) = web_sys::window()
                                        .and_then(|window| window.document())
                                        .and_then(|document| {
                                            document
                                                .query_selector(
                                                    "#m1-matrix-rows-focus [role='slider'][aria-label='Rows']",
                                                )
                                                .ok()
                                                .flatten()
                                        })
                                        .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        let _ = target.focus();
                                    }
                                },
                                Slider { label: "Rows",
    value: row_slider,
    min: 1.0,
    max: 12.0,
    step: 1.0,
    disabled: !props.projection.editable,
    on_value_change: move |value: f64| {
                                    let value = value.round().clamp(1.0, 12.0);
                                    row_slider.set(Some(value)); rows.set(format!("{value:.0}"));
                                },

                                 }
                            }
                        }
                        div { class: "grid grid-cols-1 gap-1 [&>label]:flex [&>label]:justify-between [&>label]:font-semibold [&>[role=group]]:w-full",
                            label {
                                "Columns"
                                strong { "{columns_value}" }
                            }
                            Slider { label: "Columns",
    value: column_slider,
    min: 1.0,
    max: 18.0,
    step: 1.0,
    disabled: !props.projection.editable,
    on_value_change: move |value: f64| {
                                    let value = value.round().clamp(1.0, 18.0);
                                    column_slider.set(Some(value)); columns.set(format!("{value:.0}"));
                                },

                             }
                        }
                        div { class: "grid gap-2 [&_h3]:m-0 [&_h3]:text-xs [&_h3:not(:first-child)]:mt-2 [&>[data-orientation]]:grid [&>[data-orientation]]:grid-cols-2 [&>[data-orientation]]:gap-1.5",
                            h3 { "Key assembly" }
                            ToggleGroup {
                                aria_label: "New matrix assembly",
                                pressed: family,
                                allow_multiple_pressed: false,
                                disabled: !props.projection.editable,
                                on_pressed_change: move |pressed: HashSet<usize>| {
                                    if !pressed.is_empty() {
                                        family.set(Some(pressed));
                                    }
                                },
                                for (index, label) in ["MX", "Choc"].iter().enumerate() {
                                    ToggleItem { index, class: "grid min-h-[58px] grid-cols-[28px_max-content] items-center justify-center gap-2 rounded-md p-1.5 text-left data-[state=on]:bg-selection data-[state=on]:text-selection-foreground [&_span]:grid [&_span]:min-w-0 [&_span]:gap-0.5 [&_small]:text-xs [&_small]:text-muted-foreground [&_strong]:font-semibold [&_svg]:size-[26px] [&_svg]:fill-none [&_svg]:stroke-current [&_svg]:stroke-[1.6]",
                                        svg {
                                            view_box: "0 0 32 32",
                                            "aria-hidden": "true",
                                            rect {
                                                x: "4",
                                                y: if index == 0 { "5" } else { "12" },
                                                width: "24",
                                                height: if index == 0 { "16" } else { "9" },
                                                rx: "3",
                                            }
                                            path { d: "M8 25h16M12 21v4m8-4v4" }
                                        }
                                        span {
                                            strong { "{label}" }
                                            small {
                                                if index == 0 {
                                                    "Full height"
                                                } else {
                                                    "Low profile"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            h3 { "Mounting" }
                            ToggleGroup {
                                aria_label: "Switch mounting",
                                pressed: mounting,
                                allow_multiple_pressed: false,
                                disabled: !props.projection.editable,
                                on_pressed_change: move |choice: HashSet<usize>| {
                                    if !choice.is_empty() {
                                        mounting.set(Some(choice));
                                    }
                                },
                                ToggleItem { index: 0usize, "Solder" }
                                ToggleItem { index: 1usize, "Hotswap" }
                            }
                            h3 { "Lighting" }
                            ToggleGroup {
                                aria_label: "Key lighting",
                                pressed: lighting,
                                allow_multiple_pressed: false,
                                disabled: !props.projection.editable,
                                on_pressed_change: move |choice: HashSet<usize>| {
                                    if !choice.is_empty() {
                                        lighting.set(Some(choice));
                                    }
                                },
                                ToggleItem { index: 0usize, "None" }
                                ToggleItem { index: 1usize, "Per-key RGB" }
                            }
                        }
                        if let Some((preview_rows, preview_columns)) = preview {
                            figure { class: "m-0 rounded-md border border-border bg-muted p-2 [&_svg]:mx-auto [&_svg]:block [&_svg]:max-h-[128px] [&_svg]:w-[min(100%,220px)] [&_svg]:fill-primary [&_figcaption]:mt-1 [&_figcaption]:text-center [&_figcaption]:text-[11px] [&_figcaption]:text-muted-foreground",
                                svg {
                                    view_box: "-1 -1 {preview_columns * 10 + 1} {preview_rows * 10 + 1}",
                                    role: "img",
                                    "aria-label": "{preview_rows} rows by {preview_columns} columns matrix preview",
                                    for index in 0..(preview_rows * preview_columns) {
                                        rect {
                                            key: "{index}",
                                            x: "{(index % preview_columns) * 10}",
                                            y: "{(index / preview_columns) * 10}",
                                            width: "8",
                                            height: "8",
                                            rx: "1",
                                        }
                                    }
                                }
                                figcaption {
                                    "{preview_rows * preview_columns} keys · {preview_rows} × {preview_columns}"
                                }
                            }
                        }
                        if let Some(message) = feedback_error {
                            Alert {
        variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
                        }
                        if let Some(message) = feedback_status {
                            div { role: "status",  Callout { variant: CalloutVariant::Info,  "{message}"  } }
                        }
                        footer {
                            if props.projection.can_cancel {
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    button_type: "button",
                                    onclick: move |_| on_cancel(owner.clone()),
                                    "Cancel"
                                }
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "submit",
                                disabled: !props.projection.editable,

                                "Create matrix"

                            }
                        }
                    }
                }
            }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "matrix_setup_focus_tests.rs"]
mod focus_tests;
