//! Browser half of the native/WASM boundary parity check: replays the shared
//! requests through the real `CoreEngine` and `archive_request` WASM exports
//! (typed-array marshalling included) and must reproduce the transcript the
//! native test recorded in `core/tests/fixtures/boundary/transcript.json`.
use boardstudio_core::{CoreEngine, archive_request};
use js_sys::{Array, Uint8Array};
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[path = "../../../../core/tests/support/boundary.rs"]
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
        "../../../../core/tests/fixtures/boundary/transcript.json"
    ))
    .unwrap();
    boundary::assert_golden(&transcript, &golden);
}

/// The same conservative active-dependency budget the native tests pin, exercised
/// through the packaged WASM build: the boundary chain resolves, one more edge is
/// refused, and a much deeper chain is refused without trapping the WASM stack.
#[wasm_bindgen_test]
fn wasm_constraint_depth_boundary_is_conservative() {
    // Mirrors Core's private budget; this pinned WASM boundary fails if it drifts.
    const MAX_DEPENDENCY_DEPTH: usize = 256;
    fn chain(count: usize) -> Value {
        let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
            "wasm-depth",
            "WASM depth",
        ))
        .unwrap();
        document["definitions"] = json!([{
            "id":"def","name":"Switch","kind":"switch","courtyard":[],"pads":[]
        }]);
        document["parts"] = json!(
            (0..count)
                .map(|index| json!({
                    "id": format!("p{index:05}"), "definitionId": "def",
                    "reference": format!("SW{index}"),
                    "pose": {"at": {"x": index as f64 * 10.0, "y": 0.0}, "rotation": 0.0},
                    "side": "front"
                }))
                .collect::<Vec<_>>()
        );
        document["constraints"] = json!(
            (0..count.saturating_sub(1))
                .map(|index| json!({
                    "kind": "offset", "id": format!("c{index:05}"),
                    "sourcePartId": format!("p{:05}", index + 1),
                    "targetPartId": format!("p{index:05}"),
                    "offset": {"x": 10.0, "y": 0.0}, "rotation": 0.0
                }))
                .collect::<Vec<_>>()
        );
        document
    }
    fn open(document: Value) -> Value {
        let mut engine = CoreEngine::new();
        let request = json!({"kind":"open","id":"depth","document":document});
        serde_json::from_str(&engine.request(&request.to_string())).unwrap()
    }

    assert_eq!(open(chain(MAX_DEPENDENCY_DEPTH))["kind"], "scene");
    let above = open(chain(MAX_DEPENDENCY_DEPTH + 1));
    assert_eq!(above["kind"], "error");
    assert!(
        above["message"].as_str().unwrap().contains("depth exceeds"),
        "{}",
        above["message"]
    );
    let deep = open(chain(2_000));
    assert_eq!(deep["kind"], "error");
    assert!(
        deep["message"].as_str().unwrap().contains("depth exceeds"),
        "{}",
        deep["message"]
    );
}
