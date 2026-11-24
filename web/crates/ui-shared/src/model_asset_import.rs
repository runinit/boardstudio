//! Shared browser file and verified-byte persistence path for attached model assets.

use boardstudio_core_kernel::hash::sha256_hex;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_host::host::{AssetBytes, BrowserStore};
use boardstudio_web_runtime::model_delivery::ModelFormat;
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

const MAX_MODEL_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedModelFile {
    pub filename: String,
    pub media_type: String,
    pub sha256: String,
    bytes: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
impl ImportedModelFile {
    /// Persist through the existing verified project asset store. The document
    /// edit remains a separate normal accepted operation owned by the caller.
    pub async fn store(&self, store: &BrowserStore) -> Result<(), String> {
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
pub async fn read_model_file(file: web_sys::File) -> Result<ImportedModelFile, String> {
    let filename = file.name();
    let format = ModelFormat::from_filename(&filename)?;
    validate_size(file.size())?;
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|error| format!("Model file could not be read: {}", js_error(error)))?;
    Ok(imported_model(
        filename,
        format,
        Uint8Array::new(&buffer).to_vec(),
    ))
}

fn validate_size(size: f64) -> Result<(), String> {
    if size <= 0.0 || size > MAX_MODEL_BYTES {
        return Err("Choose a nonempty model file no larger than 32 MiB.".into());
    }
    Ok(())
}

fn imported_model(filename: String, format: ModelFormat, bytes: Vec<u8>) -> ImportedModelFile {
    let sha256 = sha256_hex(&bytes);
    let media_type = match format {
        ModelFormat::Stl => "model/stl",
        ModelFormat::Wrl => "model/vrml",
        ModelFormat::Step => "model/step",
    };
    ImportedModelFile {
        filename,
        media_type: media_type.into(),
        sha256,
        bytes,
    }
}

#[cfg(target_arch = "wasm32")]
use boardstudio_web_host::browser::js_error;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_file_preflight_and_import_identity() {
        for size in [0.0, MAX_MODEL_BYTES + 1.0] {
            assert_eq!(
                validate_size(size).unwrap_err(),
                "Choose a nonempty model file no larger than 32 MiB."
            );
        }
        assert!(validate_size(MAX_MODEL_BYTES).is_ok());
        let name = "imported.stl";
        let model = imported_model(
            name.into(),
            ModelFormat::from_filename(name).unwrap(),
            b"solid imported.stl\n".to_vec(),
        );
        assert_eq!(model.media_type, "model/stl");
        assert_eq!(
            model.sha256,
            "eb7d49011bcbe51022fe594256533e0d31adeb2a0c2278d959cf11460e7e5b0e"
        );
    }
}
