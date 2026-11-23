//! The page's non-UI layer: the browser runtime (workers, storage, CAD and renderer
//! hosts), model delivery and the operation/projection logic the presentation drives.
//! No Dioxus. Split from the `boardstudio-web` bin so presentation edits do not
//! recompile it.
//!
//! Modules that only run in the browser are `wasm32`-only; the rest also compile
//! natively so their unit tests run under `cargo test`. The real Session/Core native
//! Runtime exists only in test and test-support builds.

pub mod archive_export {
    pub fn archive_filename(project_name: &str) -> String {
        format!("{project_name}.boardstudio")
    }
}
pub mod bundled_models;
pub mod case_generation_lifecycle;
pub mod case_model_lifecycle;
pub mod case_preview;
pub mod core_services;
pub mod firmware_position_projection;
#[cfg(any(test, feature = "test-support"))]
pub mod gate_driver;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) mod keycap_package;
pub mod layout_viewer_source;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) mod mechanical_package;
pub mod model_delivery;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) mod native_model_mapping;
#[cfg(test)]
mod native_model_mapping_tests;
pub mod operation_outcomes;
pub mod parts_preview;
#[cfg(any(test, target_arch = "wasm32"))]
mod physical_pcb;
pub mod portable_archive;
#[cfg(test)]
mod qualification_assessment_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod source_pcb_export_tests;

#[cfg(any(test, target_arch = "wasm32"))]
pub mod case_gesture_preview;
#[cfg(target_arch = "wasm32")]
pub mod export_footprints;
#[cfg(any(test, target_arch = "wasm32"))]
pub mod firmware_request_adapter;
#[cfg(target_arch = "wasm32")]
pub mod pcb_handoff;
#[cfg(target_arch = "wasm32")]
pub mod renderer_host_page;
#[cfg(target_arch = "wasm32")]
pub mod runtime;
#[cfg(all(not(target_arch = "wasm32"), any(test, feature = "test-support")))]
#[path = "runtime_test_stub.rs"]
pub mod runtime;

#[cfg(any(test, target_arch = "wasm32"))]
mod runtime_report;

use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlanRequest};
use boardstudio_model::model::{CoreRequest, ProjectDoc};

pub fn electrical_preview_request(
    request_id: &str,
    document: &ProjectDoc,
    board_id: &str,
) -> CoreRequest {
    let configuration = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|item| item.board_id == board_id)
    });
    CoreRequest::ResolveElectrical {
        id: request_id.to_owned(),
        request: ElectricalPlanRequest {
            document: document.clone(),
            instance_id: None,
            mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
            locks: configuration.map_or_else(Default::default, |item| item.locks.clone()),
            controller_profile: None,
            board_id: Some(board_id.to_owned()),
            controller_part_id: configuration.and_then(|item| item.controller_part_id.clone()),
        },
    }
}
