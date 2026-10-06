//! UI vocabulary shared by the page's presentation crates: tree contexts, selection,
//! workspace and view state, and canvas interaction ownership. Feature crates depend
//! on this crate instead of on each other or on the page shell, so it holds types and
//! small helpers only; an edit here recompiles every presentation crate.
//!
//! Browser-only modules are `wasm32`-only; the rest also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Presentation code addresses the runtime as `crate::runtime`, as in the page bin.
pub(crate) use boardstudio_web_runtime::{case_generation_lifecycle, runtime};

pub mod instance_selection;
pub mod state;
pub mod tree;
pub mod wiring;

#[cfg(target_arch = "wasm32")]
pub mod canvas_interaction;
#[cfg(target_arch = "wasm32")]
pub mod selection;
#[cfg(target_arch = "wasm32")]
pub mod svg_coordinates;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
