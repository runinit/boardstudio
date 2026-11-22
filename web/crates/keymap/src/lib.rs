//! The Keymap workspace: layers, bindings, encoders and macros.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{operation_outcomes, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, selection};

#[allow(unused_imports)]
pub(crate) mod pcb_wiring {
    pub use boardstudio_web_ui_model::wiring::{
        PcbWiringResolution, PcbWiringSource, WiringPlanIdentity,
    };
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

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
