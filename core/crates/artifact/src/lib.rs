#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Stateless artifact operations for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{generators, hash, matrix};
#[allow(unused_imports)]
pub(crate) use boardstudio_core_mechanical::mechanical;
#[allow(unused_imports)]
pub(crate) use boardstudio_core_mechanical::modules;

pub mod artifact;
