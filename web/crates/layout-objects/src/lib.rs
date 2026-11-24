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
pub mod canvas_interaction;
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

    #[cfg(test)]
    mod test_fixtures {
        use super::super::runtime::Runtime;
        use boardstudio_application::Event;
        use boardstudio_core::model::{Board, ProjectDoc};
        pub use boardstudio_web_catalogue::matrix_setup_operation::test_catalogue::catalogue;

        /// An empty saved document with one board, optionally carrying the
        /// single configured physical instance the setup owners admit.
        pub fn board_document(id: &str, revision: u64, with_instance: bool) -> ProjectDoc {
            let mut document = ProjectDoc::empty(id, id);
            document.revision = revision;
            document.boards.push(Board {
                id: "board".into(),
                name: "Board".into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            if with_instance {
                document.hardware = Some(
                    serde_json::from_value(serde_json::json!({"instances":[{
                        "id":"primary", "name":"Primary", "boardId":"board", "half":"single", "role":"standalone", "flipped":false, "constructionLinked":false
                    }]})).unwrap(),
                );
            }
            document
        }

        /// Open `board_document` on the headless test Runtime.
        pub fn open_board_document(
            runtime: &Runtime,
            id: &str,
            revision: u64,
            with_instance: bool,
        ) {
            runtime.submit(Event::Open {
                operation_id: runtime.operation(),
                document: board_document(id, revision, with_instance),
            });
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
