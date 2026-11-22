#![forbid(unsafe_code)]
//! Browser host services for the BoardStudio page.

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

#[cfg(feature = "page")]
mod captured_case;

#[cfg(feature = "page")]
pub mod cad_jobs {
    pub use crate::captured_case::*;
    pub use boardstudio_web_worker_host::cad_jobs::*;
}

#[cfg(feature = "page")]
pub use boardstudio_web_worker_host::cad_qualification;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub use boardstudio_web_worker_host::cad_worker;

#[cfg(feature = "page")]
pub use boardstudio_web_worker_host::offline;

#[cfg(all(test, target_arch = "wasm32", feature = "page"))]
mod boundary_parity;
#[cfg(all(test, target_arch = "wasm32", feature = "page"))]
#[path = "../../../../core/tests/support/mod.rs"]
pub(crate) mod support;

#[cfg(feature = "page")]
pub mod in_process_core;
#[cfg(feature = "page")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod persistence_contract;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;

#[cfg(feature = "page")]
pub mod case_settings;
