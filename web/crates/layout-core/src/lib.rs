//! Pure Layout workspace state logic shared by the objects tree and the outline
//! lifecycle: matrix transforms and relations, mirrored-pair geometry, anchor
//! picking, outline planning and snapping. Everything here compiles natively so
//! its unit tests run under `cargo test`.

#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;

pub mod anchor_badge;
pub mod anchor_pick;
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
pub mod screen_scale;
pub mod setup_guide_state;
