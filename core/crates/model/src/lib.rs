//! Shared document, request and reply types and their intrinsic operations.
pub mod case_readiness;
pub mod constraints;
pub mod electrical;
pub mod electrical_jumpers;
pub mod electrical_peripherals;
pub mod electrical_profiles;
pub mod firmware;
pub mod hardware;
pub mod inputs;
pub mod keymap;
pub mod matrix;
pub mod migrate;
pub mod mirrored_half;
pub mod model;
pub mod modules;
pub mod outline_edit;
pub mod wire;
pub mod mechanical {
    pub mod change;
    pub mod features;
    pub mod halves;
    pub mod pair;
}

/// Evaluates document scripts at the engine's Open and ReplaceDocument boundaries.
pub type ScriptRunner = fn(&mut model::ProjectDoc) -> Result<(), String>;
