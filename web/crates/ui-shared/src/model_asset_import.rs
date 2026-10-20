//! Shared browser file and verified-byte persistence path for attached model assets.

use crate::presentation::model_delivery::{ModelFormat, VerifiedModelBytes};
use boardstudio_core::hash::sha256_hex;
use boardstudio_web_host::host::{AssetBytes, BrowserStore};
#[cfg(target_arch = "wasm32")]
use js_sys::Uint8Array;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
const MAX_MODEL_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedModelFile {
    pub filename: String,
    pub media_type: String,
    pub sha256: String,
    bytes: Vec<u8>,
}

impl ImportedModelFile {
    /// Persist through the existing verified project asset store. The document
    /// edit remains a separate normal accepted operation owned by the caller.
    pub async fn store(&self, store: &BrowserStore) -> Result<(), String> {
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
pub async fn read_model_file(file: web_sys::File) -> Result<ImportedModelFile, String> {
    let filename = file.name();
    let format = ModelFormat::from_filename(&filename)?;
    if file.size() <= 0.0 || file.size() > MAX_MODEL_BYTES {
        return Err("Choose a nonempty model file no larger than 32 MiB.".into());
    }
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|error| format!("Model file could not be read: {}", js_error(error)))?;
    let bytes = Uint8Array::new(&buffer).to_vec();
    // One hash pass over the imported bytes: the digest is the identity the
    // file itself carries, so verifying it against itself would only repeat
    // the size check that already ran above.
    let sha256 = sha256_hex(&bytes);
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

#[cfg(test)]
mod tests {
    use super::read_model_file;
    use js_sys::{Array, Uint8Array};
    use wasm_bindgen_test::wasm_bindgen_test;

    /// SHA-256 of the fixture bytes below, as the lowercase hex every stored
    /// asset identity uses.
    const IMPORTED_SHA256: &str =
        "eb7d49011bcbe51022fe594256533e0d31adeb2a0c2278d959cf11460e7e5b0e";

    fn model_file(name: &str, bytes: &[u8]) -> web_sys::File {
        web_sys::File::new_with_u8_array_sequence(
            &Array::of1(&Uint8Array::from(bytes).into()),
            name,
        )
        .expect("supported model File fixture constructs")
    }

    #[wasm_bindgen_test]
    async fn imported_model_file_carries_the_lowercase_digest_of_its_bytes() {
        let imported = read_model_file(model_file("imported.stl", b"solid imported.stl\n"))
            .await
            .expect("a supported nonempty model file is imported");
        assert_eq!(imported.sha256, IMPORTED_SHA256);
        assert_eq!(imported.media_type, "model/stl");
    }

    #[wasm_bindgen_test]
    async fn imported_model_file_rejects_an_empty_file() {
        let error = read_model_file(model_file("empty.stl", b""))
            .await
            .expect_err("an empty model file is rejected");
        assert_eq!(error, "Choose a nonempty model file no larger than 32 MiB.");
    }
}
