//! The bundled component catalogue: loading, generator normalization and the physical
//! setup proposals built from it. Plain logic (no Dioxus), shared by Parts, Layout, PCB and
//! Case so none of them depends on another for catalogue access.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

pub mod bundled;
pub mod catalogue;
pub mod matrix_setup_operation;
pub mod physical_setup;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
