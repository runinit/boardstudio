//! The Layout workspace's features: the objects tree and its matrix, mirrored-pair, keycap
//! and alignment tools, outline versions and snapping, part placement, the layout viewer and
//! findings, the board inspector and the new-keyboard setup guide. The Layout workspace
//! container and the component inspector stay in the page shell.
//!
//! Browser-only modules are `wasm32`-only; transform, mirrored-pair, outline-setting and
//! setup-guide state logic also compiles natively so its unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::SelectionAdapter`, `super::parts::PartsQuery`,
// `super::keycaps_fit::KeycapsFitState`, `crate::presentation::coordinates`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_case::{case_display, shared_viewer};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_catalogue::matrix_setup_operation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_keycaps::{keycaps_finding_marker, keycaps_fit};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_parts::parts;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{
    case_generation_lifecycle, layout_viewer_source, model_delivery, operation_outcomes, runtime,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::selection::{
    self, SelectionAdapter, active_board_scope_matches, current_layout_owner,
    layout_owner_is_current,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::svg_coordinates::{
    PointerLocation, coordinates, coordinates_at, pointer_location,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::{canvas_interaction, instance_selection};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::{
    canvas_layers, footprint_graphics, layout_camera, model_asset_import, panels,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use part_placement::{LayoutPlacementCancellation, layout_view_mode_handler};

pub mod matrix_transform_lifecycle;
pub mod matrix_transform_operation;
pub mod mirrored_pair_geometry;
pub mod mirrored_pair_lifecycle;
pub mod outline;
pub mod outline_settings;
pub mod setup_guide_state;

#[cfg(target_arch = "wasm32")]
pub mod board_inspector;
#[cfg(target_arch = "wasm32")]
pub mod layout_findings;
#[cfg(target_arch = "wasm32")]
pub mod layout_findings_state;
#[cfg(target_arch = "wasm32")]
pub mod layout_viewer;
#[cfg(target_arch = "wasm32")]
pub mod objects;
#[cfg(target_arch = "wasm32")]
pub mod outline_lifecycle;
#[cfg(target_arch = "wasm32")]
pub mod outline_snapping;
#[cfg(target_arch = "wasm32")]
pub mod part_placement;
#[cfg(target_arch = "wasm32")]
pub mod setup_guide;

// Grid rounding, keycap resizing and alignment geometry are plain logic; compile them natively too so their
// unit tests run under `cargo test`.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "outline_grid_rounding.rs"]
mod outline_grid_rounding_tests;
#[cfg(not(target_arch = "wasm32"))]
mod objects {
    #[allow(dead_code)]
    #[path = "keycap_resize.rs"]
    mod keycap_resize;

    #[cfg(test)]
    #[path = "layout_align_geometry.rs"]
    mod layout_align_geometry_tests;
}

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
