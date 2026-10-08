//! The Library page: project list, examples and project actions.
//!
//! The page is `wasm32`-only; project rename admission also compiles natively so its
//! tests run against the real Session and Core under `cargo test`.

// Modules keep the page bin's paths: `super::ThemePicker`, `super::setup_guide`,
// `crate::runtime`.
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{operation_outcomes, runtime};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::project_menu::{
    ThemePicker, close_project_menu, durability_label,
};

#[cfg(target_arch = "wasm32")]
pub(crate) mod setup_guide {
    pub use boardstudio_web_ui_model::state::{PendingNewKeyboard, SetupGuideRequest};
}

#[cfg(target_arch = "wasm32")]
pub mod library;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod project_name;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
