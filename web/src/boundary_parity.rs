//! Browser half of the native/WASM boundary parity check: replays the shared
//! requests through the real `CoreEngine` and `archive_request` WASM exports
//! (typed-array marshalling included) and must reproduce the transcript the
//! native test recorded in `core/tests/fixtures/boundary/transcript.json`.
use boardstudio_core::{CoreEngine, archive_request};
use js_sys::{Array, Uint8Array};
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[path = "../../core/tests/support/boundary.rs"]
mod boundary;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn wasm_exports_reproduce_the_native_transcript() {
    let mut engine = CoreEngine::new();
    let transcript = boundary::run(
        &mut |request| engine.request(request),
        &mut |request, buffers| {
            let inputs = Array::new();
            for bytes in buffers {
                inputs.push(&Uint8Array::from(bytes.as_slice()));
            }
            let result = archive_request(&request.to_string(), inputs);
            let reply = result.get(0).as_string().expect("reply JSON");
            let outputs = result
                .get(1)
                .dyn_into::<Array>()
                .expect("output buffers")
                .iter()
                .map(|value| value.dyn_into::<Uint8Array>().expect("Uint8Array").to_vec())
                .collect();
            (serde_json::from_str(&reply).unwrap(), outputs)
        },
    );
    let golden: Value = serde_json::from_str(include_str!(
        "../../core/tests/fixtures/boundary/transcript.json"
    ))
    .unwrap();
    boundary::assert_golden(&transcript, &golden);
}
