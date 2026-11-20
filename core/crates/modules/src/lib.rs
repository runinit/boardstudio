//! Mounted-module resolution for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{case, geometry, hardware, matrix};
#[allow(unused_imports)]
pub(crate) use boardstudio_model::electrical_profiles;

pub mod modules;
