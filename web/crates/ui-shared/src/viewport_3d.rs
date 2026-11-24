//! The shared 3D viewport host mounted by Case, Layout 3D, Keycaps 3D and Parts.
//!
//! Lives below every workspace crate: it depends only on the runtime, host and
//! ui-model layers, never on a workspace. The pure camera, geometry and assembly
//! projections also compile natively so their unit tests run under `cargo test`;
//! the Dioxus/WebGL host itself is `wasm32`-only.

pub mod assembly_projection;
pub mod camera;
pub mod geometry;

#[cfg(target_arch = "wasm32")]
pub mod assembly_layers;
#[cfg(target_arch = "wasm32")]
mod host;
#[cfg(target_arch = "wasm32")]
pub mod sources;

#[cfg(target_arch = "wasm32")]
pub use host::*;

pub use crate::case_display::CaseDisplay;
