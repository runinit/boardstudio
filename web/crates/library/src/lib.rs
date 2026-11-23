//! The Library page: project list, examples and project actions.
//!
//! The page is `wasm32`-only; project rename admission also compiles natively so its
//! tests run against the real Session and Core under `cargo test`.

#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::runtime;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_shared::project_menu::close_project_menu;

#[cfg(target_arch = "wasm32")]
pub(crate) mod setup_guide {
    pub use boardstudio_web_ui_model::state::{PendingNewKeyboard, SetupGuideRequest};
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod demo_projects;
#[cfg(target_arch = "wasm32")]
pub mod library;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod project_name;
