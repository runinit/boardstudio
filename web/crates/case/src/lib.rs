//! The Case workspace's features: the case viewer and bodies, mechanical settings,
//! closure clearance and CAD presentation. The shared 3D viewer (`<Viewport3D />`),
//! case display preferences and assembly layers live in `boardstudio-web-ui-shared`
//! below every workspace; this crate re-exports them under their historical paths.
//! The Case workspace container stays in the page shell.
//!
//! Browser-only modules are `wasm32`-only; display, clearance, generation admission,
//! mechanical feedback and inspector-selection logic also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

// Modules keep the page bin's paths: `super::InstanceSelection`, `super::objects::TreeContext`,
// `crate::presentation::model_delivery`, `crate::runtime`.
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::case_generation_lifecycle;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::{
    case_gesture_preview, case_preview, model_delivery, runtime,
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

pub mod case_body_edit;
pub use boardstudio_web_ui_shared::case_display;
pub mod case_generation_admission;
pub mod case_gesture_math;
mod case_handles;
pub mod closure_clearance;
pub mod mechanical_pair;
pub mod mechanical_projection;
pub mod mechanical_rules;

#[cfg(target_arch = "wasm32")]
pub mod cad_presentation;
#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_ui_shared::viewport_3d::assembly_layers as case_assembly_layers;
#[cfg(target_arch = "wasm32")]
pub mod case_bodies;
#[cfg(target_arch = "wasm32")]
mod case_collision;
#[cfg(target_arch = "wasm32")]
pub mod case_controller;
#[cfg(any(target_arch = "wasm32", test))]
pub mod case_preview_lifecycle;
pub mod case_selection;
#[cfg(target_arch = "wasm32")]
mod case_viewport;
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
/// The shared 3D viewer now lives in `ui-shared`; Case selection state and
/// the typed Case edit handles stay here and are re-exported alongside it so
/// existing `shared_viewer::…` paths in the page shell keep resolving.
#[cfg(target_arch = "wasm32")]
pub mod shared_viewer {
    pub use boardstudio_web_ui_shared::viewport_3d::*;

    pub use crate::case_handles::{CaseMountTarget, ViewerHandle, ViewerHandleTarget};
    pub use crate::case_viewport::{
        BodySelection, CaseBodyEditDispatch, CaseBodyEditPortal, CaseSelection, LayerSelection,
    };
    pub(crate) use crate::case_viewport::{CaseViewport, CaseViewportSource};
}
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_shared::viewport_3d::camera as viewport_camera;
