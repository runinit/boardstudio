//! Stateless artifact operations for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{generators, hash, matrix};
#[allow(unused_imports)]
pub(crate) use boardstudio_core_mechanical::mechanical;
#[allow(unused_imports)]
pub(crate) use boardstudio_core_modules::modules;

pub mod artifact;
