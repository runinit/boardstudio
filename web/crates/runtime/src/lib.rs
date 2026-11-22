//! The page's non-UI layer: the browser runtime (workers, storage, CAD and renderer
//! hosts), model delivery and the operation/projection logic the presentation drives.
//! No Dioxus. Split from the `boardstudio-web` bin so presentation edits do not
//! recompile it.
//!
//! Modules that only run in the browser are `wasm32`-only; the rest also compile
//! natively so their unit tests run under `cargo test`. The real Session/Core native
//! Runtime exists only in test and test-support builds.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

pub mod archive_export {
    pub fn archive_filename(project_name: &str) -> String {
        format!("{project_name}.boardstudio")
    }
}
pub mod bundled_models;
pub mod case_generation_lifecycle;
pub mod case_model_lifecycle;
pub mod case_preview;
pub mod core_services;
pub(crate) mod export_lease;
pub mod firmware_position_projection;
#[cfg(any(test, feature = "test-support"))]
pub mod gate_driver;
pub(crate) mod keycap_package;
pub mod layout_viewer_source;
pub(crate) mod mechanical_package;
pub mod model_delivery;
pub(crate) mod native_model_mapping;
#[cfg(test)]
mod native_model_mapping_tests;
pub mod operation_outcomes;
pub mod parts_preview;
pub mod pcb_wiring_mode_operation;
pub mod portable_archive;
#[cfg(test)]
mod qualification_assessment_tests;

#[cfg(target_arch = "wasm32")]
pub mod case_gesture_preview;
#[cfg(target_arch = "wasm32")]
pub mod export_footprints;
#[cfg(target_arch = "wasm32")]
pub mod firmware_request_adapter;
#[cfg(target_arch = "wasm32")]
pub mod pcb_handoff;
#[cfg(target_arch = "wasm32")]
pub mod renderer_host_page;
#[cfg(target_arch = "wasm32")]
pub mod runtime;
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "test-support")))]
#[path = "runtime_test_stub.rs"]
pub mod runtime;

// Unit tests instantiate this crate's Runtime and inspect its private state. Compile the
// shared fixture wiring against that Runtime, as the external fixture crate does for callers.
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../../test-runtime/src/fixtures.rs"]
mod fixtures;

#[cfg(all(test, target_arch = "wasm32"))]
fn engine_core_services() -> core_services::CoreServices {
    core_services::CoreServices::engine()
}

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
