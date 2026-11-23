//! The Keycaps workspace's features: keycap fit and its findings, keycap settings, the
//! keycap scene and finding navigation. The workspace container that mounts them stays
//! in the page shell.
//!
//! Browser-only modules are `wasm32`-only; the keycap scene projection and the
//! keycap settings admission logic also compile natively so their unit tests run
//! under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_runtime::runtime;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod objects {
    pub use boardstudio_web_ui_model::tree::TreeContext;
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
