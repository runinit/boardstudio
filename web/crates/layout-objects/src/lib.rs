//! The Layout objects tree and its matrix, mirrored-pair, keycap and alignment tools.
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
pub(crate) use boardstudio_web_layout_core::outline as outline_lifecycle;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_layout_core::{
    anchor_pick, matrix_relations, matrix_transform_operation, mirror_existing_half,
    mirrored_pair_geometry, mirrored_pair_lifecycle, outline_settings, outline_snapping, part_rail,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_parts::parts;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::runtime;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{
    case_generation_lifecycle, layout_viewer_source, model_delivery, operation_outcomes,
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
    PointerLocation, coordinates, coordinates_at, coordinates_with_scale, pointer_location,
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
pub mod objects;

// Keycap resizing and alignment geometry are plain logic; compile them natively too so their
// unit tests run under `cargo test`.
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
