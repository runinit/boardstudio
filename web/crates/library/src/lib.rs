//! The Library page: project list, examples and project actions.
#![cfg(target_arch = "wasm32")]

// Modules keep the page bin's paths: `super::ThemePicker`, `super::setup_guide`,
// `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{operation_outcomes, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::project_menu::{
    ThemePicker, close_project_menu, durability_label,
};

pub(crate) mod setup_guide {
    pub use boardstudio_web_ui_model::state::{PendingNewKeyboard, SetupGuideRequest};
}

pub mod library;

#[cfg(test)]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
