//! The Layout objects tree and its matrix, mirrored-pair, keycap and alignment tools.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

// The objects tree shares selection, coordinate, and transform helpers with Layout.
#[cfg(target_arch = "wasm32")]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_catalogue::matrix_setup_operation;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_layout_core::outline as outline_lifecycle;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_layout_core::{
    anchor_pick, matrix_relations, mirror_existing_half, mirrored_pair_geometry,
    mirrored_pair_lifecycle, outline_settings, outline_snapping,
};
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_layout_core::{matrix_transform_operation, part_rail};
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_parts::parts;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::operation_outcomes;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::runtime;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::canvas_interaction;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::selection::{self, SelectionAdapter};
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::svg_coordinates::coordinates_with_scale;

#[cfg(target_arch = "wasm32")]
pub mod objects;

// Keycap resizing and alignment geometry are plain logic; compile them natively too so their
// unit tests run under `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
mod objects {
    #[allow(dead_code)]
    #[path = "keycap_resize.rs"]
    mod keycap_resize;

    #[cfg(test)]
    mod layout_align_logic;

    #[cfg(test)]
    mod keycap_size_logic;

    #[cfg(test)]
    mod tree_projection_logic;

    #[cfg(test)]
    mod layout_toolbar_logic;

    #[cfg(test)]
    mod matrix_setup_logic;

    #[cfg(test)]
    mod matrix_preset_logic;

    #[cfg(test)]
    #[path = "layout_align_geometry.rs"]
    mod layout_align_geometry_tests;

    #[cfg(test)]
    mod layout_selection_kind;

    #[cfg(test)]
    mod layout_rail_tool;
}
