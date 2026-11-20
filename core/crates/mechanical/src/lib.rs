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
pub mod keymap;
pub mod mechanical;
pub mod mechanical_checks;
pub mod modules;
