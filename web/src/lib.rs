#![forbid(unsafe_code)]

#[cfg(all(target_arch = "wasm32", feature = "core-worker"))]
mod core_worker;
#[cfg(all(target_arch = "wasm32", feature = "service-worker"))]
mod service_worker;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

#[cfg(any(feature = "page", feature = "cad-worker"))]
pub mod cad_jobs;

#[cfg(all(target_arch = "wasm32", any(feature = "page", feature = "cad-worker")))]
pub mod cad_worker;

pub mod offline;

pub const CORE_WORKER_FRAME_VERSION: u8 = 1;

#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod core_protocol;
#[cfg(all(test, feature = "page"))]
mod matrix_transform_operation;
#[cfg(all(test, feature = "page"))]
#[allow(dead_code)] // The native test adapter compiles the page module without its origin-discovery call sites.
#[path = "presentation/outline_snapping.rs"]
mod outline_snapping_tests;
#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod persistence_contract;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;

#[cfg(feature = "page")]
pub mod case_settings;

#[cfg(all(test, feature = "core-worker"))]
#[path = "presentation/objects/layout_align_geometry.rs"]
mod layout_align_geometry_tests;
