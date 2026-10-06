//! The Parts workspace: the component browser, definition details and naming, generator
//! settings, assemblies, custom and imported footprints, mechanical profiles and previews.
//! The workspace container stays in the page shell.
//!
//! Browser-only modules are `wasm32`-only; definition, naming, preset and profile logic
//! also compiles natively so its unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::SelectionAdapter`,
// `super::objects::TreeContext`, `super::shared_viewer::CaseSharedViewer`,
// `crate::footprint_forms`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_case::{case_display, shared_viewer};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_catalogue::{matrix_setup_operation, physical_setup};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::runtime;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{
    bundled_models, layout_viewer_source, model_delivery, operation_outcomes, parts_preview,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::selection::{self, SelectionAdapter};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::footprint_forms;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::{
    footprint_graphics, model_asset_import, panels,
    panels::browse_parts_workspace,
    panels::{PanelMode, PanelSettings},
};

#[allow(unused_imports)]
pub(crate) mod objects {
    #[cfg(target_arch = "wasm32")]
    pub use crate::parts::MatrixPlacementSource;
    pub use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext, context_for_part};
}

#[allow(unused_imports)]
pub(crate) mod part_placement {
    pub use boardstudio_web_ui_model::state::ComponentPlacementAction;
}

pub mod parts_assembly_preset_draft;
pub mod parts_custom_definition;
pub mod parts_definition_name;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod parts_import_footprint;
#[path = "parts/mechanical_profile.rs"]
pub mod parts_mechanical_profile;
pub mod parts_new_component;
pub mod parts_view_generation;

#[cfg(target_arch = "wasm32")]
pub mod parts;

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
