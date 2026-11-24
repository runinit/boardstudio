//! The PCB workspace's features: wiring, routed-board references, module footprints and
//! the module inspector, physical setup, layers, the PCB scene and the legacy
//! firmware-position map. The workspace container stays in the page shell.
//!
//! Browser-only modules are `wasm32`-only; wiring mode, connection, apply, pin and remap
//! logic, firmware position choices and remap proposals also compile natively so their
//! unit tests run under `cargo test`.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

// Modules keep the page bin's paths: `super::SelectionAdapter`, `crate::runtime`.
#[cfg(target_arch = "wasm32")]
pub(crate) use board_reference_owner::{
    board_reference_owner_is_current, board_reference_owner_lineage_is_current,
    dispatch_board_reference_action, submit_board_reference_document,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_catalogue::physical_setup;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_runtime::firmware_position_projection;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) use boardstudio_web_runtime::runtime;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::selection::{
    self, SelectionAdapter, active_board_scope_matches, current_layout_owner,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_model::state::*;
pub(crate) use boardstudio_web_ui_shared::footprint_graphics;
#[cfg(target_arch = "wasm32")]
pub(crate) use boardstudio_web_ui_shared::{
    board_reference_effect, canvas_layers, model_asset_import,
};

pub mod connector_admission;
pub mod firmware_position_choices;
mod pcb_layer_inventory;
mod pcb_layer_projection;
#[cfg(target_arch = "wasm32")]
pub mod pcb_physical_setup;

#[cfg(all(not(target_arch = "wasm32"), test))]
#[path = "pcb_physical_setup/reassignment_owner.rs"]
mod physical_setup_owner_tests;

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

// Native tests use the runtime stub supplied by the dev-dependency and cover
// only the pure wiring-domain modules; the declarative inspector is wasm-only.
#[cfg(all(not(target_arch = "wasm32"), test))]
pub(crate) mod pcb_wiring {
    pub(crate) use boardstudio_web_ui_model::wiring::{
        PcbWiringResolution, PcbWiringSource, WiringPlanIdentity,
    };

    #[path = "mode.rs"]
    mod mode;

    #[path = "connections.rs"]
    mod connections;

    #[path = "part_connections.rs"]
    mod part_connections;

    #[path = "part_input_projection.rs"]
    mod part_input_projection;

    #[path = "pins.rs"]
    mod pins;
}
