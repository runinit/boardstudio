#![forbid(unsafe_code)]
//! Browser host services for the BoardStudio page.

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod browser;

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

#[cfg(all(feature = "page", any(target_arch = "wasm32", test)))]
mod persistence_contract;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;

#[cfg(feature = "page")]
pub mod case_settings;
