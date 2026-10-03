//! Shared browser file and verified-byte persistence path for attached model assets.

use crate::presentation::model_delivery::{ModelFormat, VerifiedModelBytes};
use boardstudio_web::host::{AssetBytes, BrowserStore};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use sha2::{Digest, Sha256};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
const MAX_MODEL_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImportedModelFile {
    pub(crate) filename: String,
    pub(crate) media_type: String,
    pub(crate) sha256: String,
    bytes: Vec<u8>,
}

impl ImportedModelFile {
    /// Persist through the existing verified project asset store. The document
    /// edit remains a separate normal accepted operation owned by the caller.
    pub(crate) async fn store(&self, store: &BrowserStore) -> Result<(), String> {
        VerifiedModelBytes::verify(self.bytes.clone(), &self.sha256)?;
        store
            .save_asset(AssetBytes {
                sha256: self.sha256.clone(),
                bytes: self.bytes.clone(),
            })
            .await
            .map_err(|error| format!("Could not store model asset: {error}"))
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn read_model_file(file: web_sys::File) -> Result<ImportedModelFile, String> {
    let filename = file.name();
    let format = ModelFormat::from_filename(&filename)?;
    if file.size() <= 0.0 || file.size() > MAX_MODEL_BYTES {
        return Err("Choose a nonempty model file no larger than 32 MiB.".into());
    }
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|error| format!("Model file could not be read: {}", js_error(error)))?;
    let bytes = Uint8Array::new(&buffer).to_vec();
    let sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    VerifiedModelBytes::verify(bytes.clone(), &sha256)?;
    let media_type = match format {
        ModelFormat::Stl => "model/stl",
        ModelFormat::Wrl => "model/vrml",
        ModelFormat::Step => "model/step",
    };
    Ok(ImportedModelFile {
        filename,
        media_type: media_type.into(),
        sha256,
        bytes,
    })
}

#[cfg(target_arch = "wasm32")]
fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
