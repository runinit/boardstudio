#![forbid(unsafe_code)]
//! Browser worker host services shared by the BoardStudio workers and page.

pub mod offline;

#[cfg(feature = "cad")]
pub mod cad_jobs;

#[cfg(feature = "cad")]
pub mod cad_qualification;

#[cfg(all(target_arch = "wasm32", feature = "cad"))]
pub mod cad_worker;
