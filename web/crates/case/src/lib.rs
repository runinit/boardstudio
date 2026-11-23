//! The Case workspace's features: the case viewer and bodies, case display and assembly
//! layers, mechanical settings, closure clearance, CAD presentation and the shared 3D
//! viewer that Parts and Keycaps also mount. The Case workspace container stays in the
//! page shell.
//!
//! Browser-only modules are `wasm32`-only; display, clearance, generation admission,
//! mechanical feedback and inspector-selection logic also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

// Modules keep the page bin's paths: `super::InstanceSelection`, `super::objects::TreeContext`,
// `crate::presentation::model_delivery`, `crate::runtime`.
#[cfg(target_arch = "wasm32")]
pub(crate) use crate as presentation;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_runtime::case_generation_lifecycle;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::{
    case_gesture_preview, case_preview, layout_viewer_source, model_delivery, parts_preview,
    runtime,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::selection;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use mechanical_settings::MechanicalSettingsProps;

pub(crate) mod objects {
    #[cfg(target_arch = "wasm32")]
    pub use boardstudio_web_ui_model::tree::context_for_part;
    pub use boardstudio_web_ui_model::tree::{TreeContext, resolve_selection};
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod parts {
    pub use boardstudio_web_catalogue::bundled::load_mounting_hole_definition;
}

mod case_assembly_projection;
pub mod case_body_edit;
pub mod case_display;
pub mod case_generation_admission;
pub mod case_gesture_math;
pub mod closure_clearance;
pub mod mechanical_pair;
pub mod mechanical_projection;
pub mod mechanical_rules;

#[cfg(target_arch = "wasm32")]
pub mod cad_presentation;
#[cfg(target_arch = "wasm32")]
pub mod case_assembly_layers;
#[cfg(target_arch = "wasm32")]
pub mod case_bodies;
#[cfg(target_arch = "wasm32")]
mod case_collision;
#[cfg(target_arch = "wasm32")]
pub mod case_controller;
#[cfg(target_arch = "wasm32")]
pub mod case_preview_lifecycle;
pub mod case_selection;
#[cfg(target_arch = "wasm32")]
mod mechanical_fits;
#[cfg(target_arch = "wasm32")]
mod mechanical_layers;
#[cfg(target_arch = "wasm32")]
mod mechanical_profiles;
#[cfg(target_arch = "wasm32")]
pub mod mechanical_settings;
#[cfg(target_arch = "wasm32")]
mod pair_controls;
#[cfg(target_arch = "wasm32")]
pub mod shared_viewer;
#[cfg(any(test, target_arch = "wasm32"))]
mod viewport_camera;

#[cfg(any(test, target_arch = "wasm32"))]
mod viewport_geometry;
