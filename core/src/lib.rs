//! Rust and WebAssembly entry points for BoardStudio Core.

pub use boardstudio_core_artifact::artifact;
pub use boardstudio_core_engine::{
    ResolvedDoc, archive, companion_placement_disabled_reason, firmware, project_document,
};
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub use boardstudio_core_engine::{apply_for_test, validate_document_for_test};
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
use model::{CoreReply, CoreRequest, ProjectDoc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct CoreEngine {
    engine: boardstudio_core_engine::CoreEngine,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl CoreEngine {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self::default()
    }

    // Worker transport keeps the Rust and TypeScript protocols identical.
    pub fn request(&mut self, json: &str) -> String {
        self.engine.request(json)
    }
}

impl Default for CoreEngine {
    fn default() -> Self {
        Self {
            engine: boardstudio_core_engine::CoreEngine::new(),
        }
    }
}

impl CoreEngine {
    /// Validate a typed Open candidate before retiring editor work, without changing Core.
    pub fn validate_open(document: &ProjectDoc) -> Result<(), String> {
        boardstudio_core_engine::CoreEngine::validate_open(document)
    }

    pub fn handle(&mut self, request: CoreRequest) -> CoreReply {
        self.engine.handle(request)
    }
}

/// Handle an artifact request without mutating the stateful document engine.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn artifact_request(json: &str) -> String {
    artifact::request(json)
}

/// Archive payloads cross WASM as typed buffers, independently of JSON metadata.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
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
    result.push(&JsValue::from_str(&reply));
    let encoded = js_sys::Array::new();
    for bytes in outputs {
        encoded.push(&js_sys::Uint8Array::from(bytes.as_slice()));
    }
    result.push(&encoded);
    result
}
