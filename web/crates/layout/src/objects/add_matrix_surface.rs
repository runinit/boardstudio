//! One shared Add-matrix surface for the single and mirrored-pair modes.
//!
//! The Layout (and PCB) Add menu exposes a single "Matrix…" entry. The surface
//! renders a Single | Mirrored pair toggle above the shared MatrixSetup fields;
//! pair mode adds the second layout name and gap field through MirroredPairForm.
//! Switching modes cancels the current owner before opening the other so the
//! two controllers never own the canvas at once.
use super::matrix_setup::{MatrixSetup, MatrixSetupMount};
use super::mirrored_pair::{MirroredPairCanvasOverlay, MirroredPairMount};
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddMatrixMode {
    Single,
    Pair,
}

#[derive(Props, Clone, PartialEq)]
pub struct AddMatrixSurfaceProps {
    pub matrix_setup: MatrixSetupMount,
    pub mirrored_pair: MirroredPairMount,
    /// The Setup Guide renders its own inline single-matrix form; while it does,
    /// the surface hides its single mode (and its switch) so one owner mounts once.
    pub guide_covering_single: bool,
}

#[component]
pub fn AddMatrixSurface(props: AddMatrixSurfaceProps) -> Element {
    let pair_form = props.mirrored_pair.form.clone();
    let matrix_projection = props.matrix_setup.projection.clone();
    let pair_active = pair_form.is_some();
    if !pair_active && (props.guide_covering_single || matrix_projection.is_none()) {
        return rsx! {};
    }
    let mode = if pair_active {
        AddMatrixMode::Pair
    } else {
        AddMatrixMode::Single
    };

    let to_single = {
        let matrix_setup = props.matrix_setup.clone();
        let mirrored_pair = props.mirrored_pair.clone();
        let pair_form = pair_form.clone();
        let matrix_projection = matrix_projection.clone();
        move |_| {
            if let Some(form) = pair_form.clone() {
                mirrored_pair.on_cancel.call(form.owner);
            }
            if matrix_projection.is_none() {
                matrix_setup.on_open.call(());
            }
        }
    };
    let to_pair = {
        let matrix_setup = props.matrix_setup.clone();
        let mirrored_pair = props.mirrored_pair.clone();
        let pair_form = pair_form.clone();
        let matrix_projection = matrix_projection.clone();
        move |_| {
            if let Some(projection) = matrix_projection.clone() {
                matrix_setup.on_cancel.call(projection.owner);
            }
            if pair_form.is_none() {
                mirrored_pair.on_open.call(());
            }
        }
    };
    let single_disabled = mode == AddMatrixMode::Single || props.guide_covering_single;
    let pair_disabled = mode == AddMatrixMode::Pair
        || !props.mirrored_pair.can_open
        || matrix_projection
            .as_ref()
            .is_some_and(|projection| !projection.can_cancel);

    rsx! {
        div { class: "m1-add-matrix-surface",
            div { class: "m1-add-matrix-mode", role: "group", aria_label: "Add matrix mode",
                button {
                    r#type: "button",
                    aria_pressed: mode == AddMatrixMode::Single,
                    disabled: single_disabled,
                    onclick: to_single,
                    "Single"
                }
                button {
                    r#type: "button",
                    aria_pressed: mode == AddMatrixMode::Pair,
                    disabled: pair_disabled,
                    onclick: to_pair,
                    "Mirrored pair"
                }
            }
            if mode == AddMatrixMode::Pair {
                if let Some(projection) = pair_form {
                    MirroredPairCanvasOverlay {
                        projection,
                        on_cancel: props.mirrored_pair.on_cancel,
                        on_preview: props.mirrored_pair.on_preview,
                    }
                }
            } else if let Some(projection) = matrix_projection {
                MatrixSetup {
                    projection,
                    on_cancel: props.matrix_setup.on_cancel,
                    on_create: props.matrix_setup.on_create,
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "add_matrix_surface_tests.rs"]
mod surface_tests;
