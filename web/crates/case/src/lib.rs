//! The Case workspace's features: the case viewer and bodies, case display and assembly
//! layers, mechanical settings, closure clearance, CAD presentation and the shared 3D
//! viewer that Parts and Keycaps also mount. The Case workspace container stays in the
//! page shell.
//!
//! Browser-only modules are `wasm32`-only; display, clearance, generation admission and
//! mechanical feedback logic also compile natively so their unit tests run under
//! `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::InstanceSelection`, `super::objects::TreeContext`,
// `crate::presentation::model_delivery`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_keycaps::keycaps_finding_marker;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{
    bundled_models, case_generation_lifecycle, case_model_lifecycle, case_preview,
    layout_viewer_source, model_delivery, operation_outcomes, parts_preview,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{case_gesture_preview, renderer_host_page, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, selection};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use case_viewer::{CasePreviewViewer, CaseViewer};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use mechanical_settings::{MechanicalSettings, MechanicalSettingsProps};

#[allow(unused_imports)]
pub(crate) mod objects {
    pub use boardstudio_web_ui_model::tree::{
        ScopedTreeContext, TreeContext, context_for_part, resolve_selection,
    };
}

#[allow(unused_imports)]
pub(crate) mod parts {
    pub use boardstudio_web_catalogue::bundled::load_mounting_hole_definition;
}

pub mod case_display;
pub mod case_generation_admission;
pub mod closure_clearance;
pub mod mechanical_feedback;

#[cfg(target_arch = "wasm32")]
pub mod cad_presentation;
#[cfg(target_arch = "wasm32")]
pub mod case_assembly_layers;
#[cfg(target_arch = "wasm32")]
pub mod case_bodies;
#[cfg(target_arch = "wasm32")]
pub mod case_controller;
#[cfg(target_arch = "wasm32")]
pub mod case_preview_lifecycle;
#[cfg(target_arch = "wasm32")]
pub mod case_selection;
#[cfg(target_arch = "wasm32")]
pub mod case_viewer;
#[cfg(target_arch = "wasm32")]
pub mod mechanical_settings;
#[cfg(target_arch = "wasm32")]
pub mod mechanical_settings_controller;
#[cfg(target_arch = "wasm32")]
pub mod mechanical_settings_mount;
#[cfg(target_arch = "wasm32")]
pub(crate) mod observed_edits;
#[cfg(target_arch = "wasm32")]
pub mod shared_viewer;
#[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
pub mod test_contexts;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
