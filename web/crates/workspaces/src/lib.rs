//! Combined Library, Keymap and Keycaps workspace features.
//!
//! Pure projection, rename-admission, keycap-fit, keymap-edit and settings modules
//! compile natively so their unit tests run under `cargo test`; Dioxus views and the
//! 3D assembly viewer stay `wasm32`-only.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_runtime::runtime;

#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::state::*;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod demo_projects;
#[cfg(target_arch = "wasm32")]
pub mod library;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod project_name;

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

pub mod keycaps_scene;

#[cfg(any(target_arch = "wasm32", test))]
pub mod keycaps_finding_marker;
#[cfg(any(target_arch = "wasm32", test))]
pub mod keycaps_fit;
#[cfg(any(target_arch = "wasm32", test))]
pub mod keycaps_navigation;
#[cfg(any(target_arch = "wasm32", test))]
pub mod keycaps_settings;
#[cfg(target_arch = "wasm32")]
pub mod keycaps_viewer;
