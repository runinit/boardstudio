//! The PCB workspace's features: wiring, routed-board references, module footprints and
//! the module inspector, physical setup, layers, the PCB scene and the legacy
//! firmware-position map. The workspace container stays in the page shell.
//!
//! Browser-only modules are `wasm32`-only; wiring mode, connection, apply, pin and remap
//! logic, firmware position choices and remap proposals also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

// Modules keep the page bin's paths: `super::SelectionAdapter`, `super::objects::TreeContext`,
// `super::parts::is_generator_source`, `crate::runtime`.
#[allow(unused_imports)]
pub(crate) use crate as presentation;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use board_reference_owner::{
    board_reference_owner_is_current, board_reference_owner_lineage_is_current,
    board_reference_target_is_current, dispatch_board_reference_action,
    submit_board_reference_document,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_catalogue::physical_setup;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::runtime;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_runtime::{
    firmware_position_projection, model_delivery, operation_outcomes, pcb_wiring_mode_operation,
};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::selection::{
    self, SelectionAdapter, active_board_scope_matches, current_layout_owner,
    layout_owner_is_current,
};
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_model::state::*;
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::footprint_forms;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) use boardstudio_web_ui_shared::{
    board_reference_effect, canvas_layers, footprint_graphics, model_asset_import,
};

#[allow(unused_imports)]
pub(crate) mod objects {
    pub use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
}

#[allow(unused_imports)]
pub(crate) mod part_placement {
    pub use boardstudio_web_ui_model::state::ComponentPlacementAction;
}

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
pub(crate) mod parts {
    pub use boardstudio_web_catalogue::bundled::{generator_parameter_schema, is_generator_source};
    pub use boardstudio_web_parts::parts::load_horizontal_host_connector_definition;
}

pub mod firmware_position_choices;
pub mod pcb_wiring_remap_operation;

#[cfg(target_arch = "wasm32")]
pub mod board_reference_owner;
#[cfg(target_arch = "wasm32")]
pub mod firmware_positions;
#[cfg(target_arch = "wasm32")]
pub mod pcb_board_reference;
#[cfg(target_arch = "wasm32")]
pub mod pcb_layers;
#[cfg(target_arch = "wasm32")]
pub mod pcb_module_footprints;
#[cfg(target_arch = "wasm32")]
pub mod pcb_module_inspector;
#[cfg(target_arch = "wasm32")]
pub mod pcb_physical_setup;
#[cfg(target_arch = "wasm32")]
pub mod pcb_scene;
#[cfg(target_arch = "wasm32")]
pub mod pcb_wiring;

// Routed-board model matching is plain Rust; compile it natively too for its unit tests.
#[cfg(not(target_arch = "wasm32"))]
mod pcb_board_reference {
    #[allow(dead_code)]
    #[path = "matching.rs"]
    mod matching;
}

// Wiring logic is plain Rust; compile it natively too so its unit tests run under `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod pcb_wiring {
    #[allow(unused_imports)]
    use crate::firmware_position_projection::FirmwarePlanIdentity;
    #[allow(unused_imports)]
    use boardstudio_application::Scope;
    #[allow(unused_imports)]
    use boardstudio_core::{electrical::ElectricalPlan, model::ProjectDoc};
    pub(crate) use boardstudio_web_ui_model::wiring::{
        PcbWiringResolution, PcbWiringSource, WiringPlanIdentity,
    };
    #[allow(unused_imports)]
    use std::{rc::Rc, sync::Arc};

    #[path = "mode.rs"]
    mod mode;

    #[path = "connections.rs"]
    mod connections;

    #[path = "apply.rs"]
    mod apply;

    #[path = "pins.rs"]
    mod pins;

    #[path = "remap.rs"]
    mod remap;

    #[cfg(test)]
    #[path = "mode_owner_tests.rs"]
    mod mode_owner_tests;
}

#[cfg(all(test, target_arch = "wasm32"))]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
