//! Rust entry points for BoardStudio Core.

pub use boardstudio_core_artifact::artifact;
pub use boardstudio_core_engine::{
    CommittedScene, CoreEngine, EditOutcome, ResolvedDoc, archive,
    companion_placement_disabled_reason, firmware, project_document,
};
pub use boardstudio_core_kernel::matrix::kernel as layout_kernel;
pub use boardstudio_core_kernel::matrix::layout::{
    evaluated_part_pose, half_offset, is_matrix_key, layout_half_offset, part_correspondence,
    proposed_creation_gap,
};
pub use boardstudio_core_kernel::{document_format, generators, hash, outline_edit};
pub use boardstudio_core_mechanical::{electrical, electrical_peripherals, mechanical};
pub use boardstudio_model::case_readiness::{authored_case_geometry_ready, case_preparation_ready};
pub use boardstudio_model::{electrical_jumpers, electrical_profiles, model};

#[cfg(target_arch = "wasm32")]
use model::ArchiveReply;
/// Handle an artifact request without mutating the stateful document engine.
pub fn artifact_request(json: &str) -> String {
    artifact::request(json)
}

/// Archive payloads cross WASM as typed buffers, independently of JSON metadata.
#[cfg(target_arch = "wasm32")]
pub fn archive_request(metadata: &str, buffers: js_sys::Array) -> js_sys::Array {
    use wasm_bindgen::JsCast;
    let inputs: Result<Vec<Vec<u8>>, _> = buffers
        .iter()
        .map(|value| {
            value
                .dyn_into::<js_sys::Uint8Array>()
                .map(|bytes| bytes.to_vec())
        })
        .collect();
    let (reply, outputs) = match inputs {
        Ok(inputs) => archive::request(metadata, &inputs),
        Err(_) => (
            boardstudio_model::wire::encode_archive_reply(&ArchiveReply::Error {
                message: "Archive buffers must be Uint8Array values".into(),
            }),
            Vec::new(),
        ),
    };
    let result = js_sys::Array::new();
    result.push(&wasm_bindgen::JsValue::from_str(&reply));
    let encoded = js_sys::Array::new();
    for bytes in outputs {
        encoded.push(&js_sys::Uint8Array::from(bytes.as_slice()));
    }
    result.push(&encoded);
    result
}
