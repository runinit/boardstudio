use boardstudio_web_ui_shared::model_asset_import::read_model_file;
use js_sys::{Array, Uint8Array};
use wasm_bindgen_test::wasm_bindgen_test;

/// SHA-256 of the fixture bytes below, as the lowercase hex every stored
/// asset identity uses.
const IMPORTED_SHA256: &str = "eb7d49011bcbe51022fe594256533e0d31adeb2a0c2278d959cf11460e7e5b0e";

fn model_file(name: &str, bytes: &[u8]) -> web_sys::File {
    web_sys::File::new_with_u8_array_sequence(&Array::of1(&Uint8Array::from(bytes).into()), name)
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
