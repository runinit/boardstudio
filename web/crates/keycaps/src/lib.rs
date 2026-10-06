//! The Keycaps workspace's features: keycap fit and its findings, keycap settings, the
//! keycap scene and finding navigation. The workspace container that mounts them stays
//! in the page shell.
#![cfg(target_arch = "wasm32")]

// Modules keep the page bin's paths: `super::WorkspaceState`, `super::objects::TreeContext`,
// `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{model_delivery, operation_outcomes, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, selection};

pub(crate) mod objects {
    pub use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
}

pub mod keycaps_finding_marker;
pub mod keycaps_fit;
pub mod keycaps_navigation;
pub mod keycaps_scene;
pub mod keycaps_settings;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
