#![forbid(unsafe_code)]
//! Browser host services shared by the page (`boardstudio-web` bin) and its workers.

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

#[cfg(any(feature = "page", feature = "cad-worker"))]
pub mod cad_jobs;

#[cfg(all(target_arch = "wasm32", any(feature = "page", feature = "cad-worker")))]
pub mod cad_worker;

pub mod offline;

#[cfg(all(test, target_arch = "wasm32", feature = "page"))]
mod boundary_parity;

pub const CORE_WORKER_FRAME_VERSION: u8 = 1;

#[cfg(feature = "page")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod core_protocol;
#[cfg(feature = "page")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod persistence_contract;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;

#[cfg(feature = "page")]
pub mod case_settings;
