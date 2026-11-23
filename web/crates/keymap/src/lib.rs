//! The Keymap workspace: layers, bindings, encoders and macros.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::runtime;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod pcb_wiring {
    pub use boardstudio_web_ui_model::wiring::PcbWiringResolution;
}

pub mod binding;
pub mod encoder;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod layer_edit;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod macro_edit;

#[cfg(target_arch = "wasm32")]
pub mod keymap;

#[cfg(all(not(target_arch = "wasm32"), test))]
pub mod keymap {
    #[path = "view.rs"]
    mod view;
    #[path = "view_controls.rs"]
    mod view_controls;

    pub use view::{KeymapView, project};
    pub use view_controls::{FitCamera, fit_camera, selected_bounds};
}
