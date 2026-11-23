//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
};
use crate::matrix_setup_operation::MatrixSetupPreset;
pub use crate::presentation::parts::MatrixPlacementSource;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
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
    pub matrix: boardstudio_model::model::Matrix,
    pub scene: boardstudio_model::model::MatrixScene,
    pub definitions: Vec<boardstudio_model::model::PartDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementMove {
    pub owner: MatrixPlacementOwner,
    pub center: boardstudio_model::model::Vec2,
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
    pub workspace: Signal<Workspace>,
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
    let rows = use_signal(|| "5".to_owned());
    let columns = use_signal(|| "6".to_owned());
    let row_slider = use_signal(|| Some(5.0));
    let column_slider = use_signal(|| Some(6.0));
    let family = use_signal(|| Some(HashSet::from([0usize])));
    let mounting = use_signal(|| Some(HashSet::from([0usize])));
    let lighting = use_signal(|| Some(HashSet::from([0usize])));
    let return_focus = use_signal(|| None::<web_sys::HtmlElement>);
    let rows_value = rows();
    let columns_value = columns();
    let choc = family().is_some_and(|choice| choice.contains(&1));
    let hotswap = mounting().is_some_and(|choice| choice.contains(&1));
    let rgb = lighting().is_some_and(|choice| choice.contains(&1));
    let selected_preset = matrix_setup_preset(choc, hotswap, rgb);
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
        section {
            class: "absolute inset-x-2 top-[100px] bottom-2 z-6 flex flex-col gap-2.5 overflow-auto rounded-md border border-border-strong bg-card p-3 text-foreground shadow-md [&_form]:grid [&_form]:gap-2 [&_footer]:flex [&_footer]:justify-end [&_footer]:gap-2",
            aria_label: "New matrix",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                }
            },
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-[16px]!", h2 { "New matrix" } }
            form { onsubmit: on_submit,
                div {
                    id: "m1-matrix-rows-focus",
                    onmounted: move |_| async move {
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.active_element())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { return_focus_on_mount.set(Some(target)); }
                        if let Some(target) = web_sys::window()
                            .and_then(|window| window.document())
                            .and_then(|document| document.query_selector("#m1-matrix-rows-focus [role='slider'][aria-label='Rows']").ok().flatten())
                            .and_then(|target| target.dyn_into::<web_sys::HtmlElement>().ok())
                        { let _ = target.focus(); }
                    },
                    MatrixDimensionSliders {
                        rows, columns, row_slider, column_slider,
                        rows_label: "Rows", columns_label: "Columns", editable: props.projection.editable,
                    }
                }
                MatrixAssemblyPicker { family, mounting, lighting, editable: props.projection.editable }
                MatrixPreview { rows: rows_value.clone(), columns: columns_value.clone() }
                if let Some(message) = feedback_error {
                    Alert { variant: AlertVariant::Destructive, AlertDescription { "{message}" } }
                }
                if let Some(message) = feedback_status {
                    div { role: "status", Callout { variant: CalloutVariant::Info, "{message}" } }
                }
                footer {
                    if props.projection.can_cancel {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel(owner.clone()), "Cancel" }
                    }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !props.projection.editable, "Create matrix" }
                }
            }
        }
    }
}
