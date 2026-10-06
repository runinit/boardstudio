//! The Keycaps workspace's features: keycap fit and its findings, keycap settings, the
//! keycap scene and finding navigation. The workspace container that mounts them stays
//! in the page shell.
//!
//! Browser-only modules are `wasm32`-only; the keycap scene projection also compiles
//! natively so its unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::WorkspaceState`, `super::objects::TreeContext`,
// `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{model_delivery, operation_outcomes, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[allow(unused_imports)]
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, selection};

pub(crate) mod objects {
    pub use boardstudio_web_ui_model::tree::TreeContext;
}

pub mod keycaps_scene;

#[cfg(target_arch = "wasm32")]
pub mod keycaps_finding_marker;
#[cfg(target_arch = "wasm32")]
pub mod keycaps_fit;
#[cfg(target_arch = "wasm32")]
pub mod keycaps_navigation;
#[cfg(target_arch = "wasm32")]
pub mod keycaps_settings;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
