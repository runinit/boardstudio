//! The page's non-UI layer: the browser runtime (workers, storage, CAD and renderer
//! hosts), model delivery and the operation/projection logic the presentation drives.
//! No Dioxus. Split from the `boardstudio-web` bin so presentation edits do not
//! recompile it.
//!
//! Modules that only run in the browser are `wasm32`-only; the rest also compile
//! natively so their unit tests run under `cargo test`. `runtime_test_stub` stands
//! in for `runtime` natively, for the presentation's native tests.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

pub mod archive_export;
pub mod bundled_models;
pub mod case_generation_lifecycle;
pub mod case_model_lifecycle;
pub mod case_preview;
pub mod edit_ticket;
pub(crate) mod export_lease;
pub mod firmware_position_projection;
pub mod layout_viewer_source;
pub(crate) mod mechanical_package;
pub mod model_delivery;
pub mod operation_outcomes;
pub mod parts_preview;
pub mod pcb_wiring_mode_operation;
pub mod portable_archive;

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
#[cfg(not(target_arch = "wasm32"))]
#[path = "runtime_test_stub.rs"]
pub mod runtime;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod renderer_host_source_sync;
