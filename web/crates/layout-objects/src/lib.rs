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

// `matrix_source` and the Matrix Setup owner run against the real Session/CoreEngine in native
// lifecycle tests; the catalogue operation and the headless test Runtime back them.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(crate) use boardstudio_web_catalogue::matrix_setup_operation;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(crate) use boardstudio_web_runtime::runtime;

// Keycap resizing and alignment geometry are plain logic; compile them natively too so their
// unit tests run under `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
mod objects {
    #[cfg(test)]
    pub use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext, resolve_selection};

    /// Shared matrix-template fixture for native lifecycle tests. It mirrors the catalogue
    /// crate's `test_catalogue.rs` without pulling that crate's `#[cfg(test)]` module across
    /// crates (inline test modules cannot `#[path]`-include it through a virtual directory).
    #[cfg(test)]
    mod test_fixtures {
        use boardstudio_core::model::{PartDefinition, PartGenerator, PartKind};
        use std::collections::BTreeMap;

        pub fn definition(id: &str, source: &str, kind: PartKind) -> PartDefinition {
            PartDefinition {
                hardware_profile: None,
                input_profile: None,
                mechanical_profile: None,
                id: id.into(),
                name: id.into(),
                kind,
                keycap: None,
                envelope_source: None,
                kicad_source: None,
                terminals: BTreeMap::new(),
                matrix_terminals: None,
                envelope_notice: None,
                courtyard: vec![],
                pads: vec![],
                models: None,
                generator: Some(PartGenerator {
                    source: source.into(),
                    version: "fixture".into(),
                    parameters: BTreeMap::new(),
                }),
            }
        }

        pub fn catalogue() -> Vec<PartDefinition> {
            vec![
                definition(
                    "generator:ceoloide/switch_mx",
                    "ceoloide/switch_mx",
                    PartKind::Switch,
                ),
                definition(
                    "generator:ceoloide/switch_choc_v1_v2",
                    "ceoloide/switch_choc_v1_v2",
                    PartKind::Switch,
                ),
                definition(
                    "generator:ceoloide/diode_tht_sod123",
                    "ceoloide/diode_tht_sod123",
                    PartKind::Passive,
                ),
                definition(
                    "generator:ceoloide/led_sk6812mini-e",
                    "ceoloide/led_sk6812mini-e",
                    PartKind::Passive,
                ),
            ]
        }
    }

    #[allow(dead_code)]
    #[path = "keycap_resize.rs"]
    mod keycap_resize;

    #[path = "mirror_links.rs"]
    mod mirror_links;

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
    mod matrix_preparation;

    #[cfg(test)]
    mod matrix_setup_controller;

    #[cfg(test)]
    mod matrix_preset_logic;

    #[cfg(test)]
    #[path = "layout_align_geometry.rs"]
    mod layout_align_geometry_tests;

    #[cfg(test)]
    mod layout_selection_kind;

    #[cfg(test)]
    mod layout_rail_tool;

    #[cfg(test)]
    mod workspace_state;
}
