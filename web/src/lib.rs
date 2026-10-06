#![forbid(unsafe_code)]
//! Worker entry points packaged by wasm-pack (`core-worker`, `cad-worker`, `service-worker`).
//! Shared host services live in `boardstudio-web-host`; the page is the `boardstudio-web` bin.

#[cfg(all(target_arch = "wasm32", feature = "core-worker"))]
mod core_worker;
#[cfg(all(target_arch = "wasm32", feature = "service-worker"))]
mod service_worker;

// The CAD worker's `start_cad_worker` export is defined in the host crate.
#[cfg(all(target_arch = "wasm32", feature = "cad-worker"))]
pub use boardstudio_web_host::cad_worker::start_cad_worker;

#[cfg(all(test, feature = "page"))]
mod matrix_transform_operation;

#[cfg(all(test, feature = "core-worker"))]
#[path = "presentation/objects/layout_align_geometry.rs"]
mod layout_align_geometry_tests;
