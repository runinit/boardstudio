//! The Layout workspace's features: the objects tree and its matrix, mirrored-pair, keycap
//! and alignment tools, outline versions and snapping, part placement, the layout viewer and
//! findings, the board inspector and the new-keyboard setup guide. The Layout workspace
//! container and the component inspector stay in the page shell.
//!
//! Browser-only modules are `wasm32`-only; transform, mirrored-pair, outline-setting and
//! setup-guide state logic also compiles natively so its unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Shared imports keep the paths used by Layout's view and controller modules.
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_layout_objects::canvas_interaction;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_parts::parts;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::runtime;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::selection::{self, SelectionAdapter};
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::svg_coordinates::coordinates;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_shared::panels;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_workspaces::keycaps_fit;

pub use boardstudio_web_layout_core::{
    anchor_badge, anchor_pick, canvas_projection, companion_placement, component_inspector,
    component_offset, inspector_types, matrix_relations, matrix_transform_lifecycle,
    matrix_transform_operation, mirror_existing_half, mirrored_pair_geometry,
    mirrored_pair_lifecycle, outline_settings, part_rail, screen_scale, setup_guide_state,
};
mod findings_return;
pub mod outline;

#[cfg(target_arch = "wasm32")]
pub mod board_inspector;
#[cfg(target_arch = "wasm32")]
pub mod layout_findings;
#[cfg(target_arch = "wasm32")]
pub mod layout_findings_state;
#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_layout_core::outline_snapping;
#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_layout_objects::objects;
pub mod part_placement;
#[cfg(target_arch = "wasm32")]
pub mod setup_guide;
