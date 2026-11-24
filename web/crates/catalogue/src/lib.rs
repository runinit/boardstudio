//! The bundled component catalogue: loading, generator normalization and the physical
//! setup proposals built from it. Plain logic (no Dioxus), shared by Parts, Layout, PCB and
//! Case so none of them depends on another for catalogue access.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

pub mod bundled;
pub mod case_settings;
pub mod catalogue;
pub mod matrix_input;
pub mod matrix_setup_operation;
pub mod mirror_pair;
pub mod physical_setup;
