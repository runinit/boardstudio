//! Electrical planning for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{hash, inputs};
#[allow(unused_imports)]
pub(crate) use boardstudio_core_modules::modules;
#[allow(unused_imports)]
pub(crate) use boardstudio_model::{electrical_jumpers, electrical_profiles};

pub mod electrical;
pub mod electrical_peripherals;
pub mod keymap;
