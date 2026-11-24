//! Pure Layout workspace state logic shared by the objects tree and the outline
//! lifecycle: matrix transforms and relations, mirrored-pair geometry, anchor
//! picking, outline planning and snapping. Everything here compiles natively so
//! its unit tests run under `cargo test`.

pub mod anchor_badge;
pub mod anchor_pick;
pub mod canvas_projection;
pub mod companion_placement;
pub mod component_inspector;
pub use component_inspector::layout_component_edits;
pub mod component_offset;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod constraint_edit_tests;
#[cfg(test)]
mod engine_test_support;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod inspector_fixture;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod inspector_projection_tests;
pub mod inspector_types;
pub mod interaction;
pub mod matrix_relations;
pub mod matrix_transform_lifecycle;
pub mod matrix_transform_operation;
pub mod mirror_existing_half;
pub mod mirrored_pair_geometry;
pub mod mirrored_pair_lifecycle;
pub mod outline;
pub mod outline_settings;
pub mod outline_snapping;
pub mod part_rail;
pub mod rail_drag;
pub mod rail_gesture;
pub mod screen_scale;
pub mod setup_guide_state;

use boardstudio_model::model::{Matrix, MatrixScene, MatrixSceneCell, ProjectDoc};

pub fn matrix_by_id<'a>(document: &'a ProjectDoc, matrix_id: &str) -> Option<&'a Matrix> {
    document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
}

pub fn scene_by_id<'a>(scenes: &'a [MatrixScene], matrix_id: &str) -> Option<&'a MatrixScene> {
    scenes.iter().find(|scene| scene.matrix_id == matrix_id)
}

pub fn scene_cell_in(scene: &MatrixScene, row: u32, column: u32) -> Option<&MatrixSceneCell> {
    scene
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
}

pub fn scene_cell<'a>(
    scenes: &'a [MatrixScene],
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<&'a MatrixSceneCell> {
    scene_cell_in(scene_by_id(scenes, matrix_id)?, row, column)
}

pub fn canonical_matrix_member(matrix_id: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix_id}/r{row}c{column}")
}
