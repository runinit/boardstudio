//! Mirrored-half lifecycle for BoardStudio Core.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{
    constraints, generators, geometry, matrix, outline_controls, outline_edit, outline_versions,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_core_mechanical::mechanical;
#[allow(unused_imports)]
pub(crate) use boardstudio_core_mechanical::{electrical, electrical_peripherals, keymap, modules};
#[allow(unused_imports)]
pub(crate) use boardstudio_model::ScriptRunner;

pub mod mirrored_half;
