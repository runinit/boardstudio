mod model;
pub use model::{
    build_assembly, build_case, build_keycaps, export_cached_assembly, preview_body,
    read_step_model,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn initialize_cadrum() {
    cadrum::__anchor_wasi_stub();
    unsafe {
        __wasm_call_ctors();
    }
}
