//! Page UI shared by several workspaces: panels, canvas layers, the layout camera,
//! footprint forms and graphics, geometry scripts, model import and board-reference
//! effects. Depends on `ui-model` only, never on a workspace crate or the page shell.
//!
//! Browser-only modules are `wasm32`-only; the rest also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules address shared types as the page bin does: `super::WorkspaceState`,
// `crate::presentation::model_delivery`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{model_delivery, runtime};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;

pub mod footprint_forms;
pub mod layout_camera;
pub mod pending_edit_helpers;

#[cfg(target_arch = "wasm32")]
pub mod board_reference_effect;
#[cfg(target_arch = "wasm32")]
pub mod canvas_layers;
#[cfg(target_arch = "wasm32")]
pub mod canvas_navigation;
#[cfg(target_arch = "wasm32")]
pub mod footprint_graphics;
#[cfg(target_arch = "wasm32")]
pub mod geometry_scripts;
#[cfg(target_arch = "wasm32")]
pub mod model_asset_import;
#[cfg(target_arch = "wasm32")]
pub mod object_options;
#[cfg(target_arch = "wasm32")]
pub mod panels;
#[cfg(target_arch = "wasm32")]
pub mod project_menu;
// Panel policy is plain logic; compile it natively too so its unit tests run under `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
mod panels {
    #[allow(dead_code)]
    #[path = "policy.rs"]
    mod policy;
}

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
