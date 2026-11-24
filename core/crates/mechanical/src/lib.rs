#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Mechanical resolution for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{
    case, generators, geometry, hardware, hash, inputs, matrix,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_model::{electrical_jumpers, electrical_profiles};

pub mod electrical;
pub mod electrical_peripherals;
pub(crate) mod geometry2d;
pub mod keymap;
pub mod mechanical;
pub mod mechanical_checks;
pub mod modules;
#[cfg(test)]
pub(crate) mod test_support;
