//! Mechanical resolution for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{case, generators, geometry, hash, matrix};
#[allow(unused_imports)]
pub(crate) use boardstudio_core_modules::modules;

pub mod mechanical;
pub mod mechanical_checks;
