//! The Keymap workspace: layers, bindings, encoders and macros. The workspace container
//! that mounts it stays in the page shell.
//!
//! Browser-only modules are `wasm32`-only; macro control naming also compiles natively
//! so its unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::InstanceSelection`,
// `super::super::pcb_wiring::PcbWiringResolution`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{operation_outcomes, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, selection};

#[allow(unused_imports)]
pub(crate) mod pcb_wiring {
    pub use boardstudio_web_ui_model::wiring::{
        PcbWiringResolution, PcbWiringSource, WiringPlanIdentity,
    };
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod layer_edit;
pub mod macro_accessible_names;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod macro_edit;

#[cfg(target_arch = "wasm32")]
pub mod keymap;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
