//! Acceptance baseline recorded at 323967ff; its source hashes also match 3cdeb2ac2.
//! Identifiers are mechanically
//! renamed; placements, source references, matrix recipes, wiring and model hashes are frozen.
#[path = "../examples/demo_projects/mod.rs"]
#[allow(clippy::module_inception)] // The module is the shared example source, included by path.
mod demo_projects;
use serde_json::Value;
use std::fs;
fn equivalent(actual: &Value, expected: &Value, path: &str) {
    match expected {
        Value::Number(number) => {
            let expected = number.as_f64().unwrap();
            let actual = actual
                .as_f64()
                .unwrap_or_else(|| panic!("{path}: expected number"));
            assert!(
                (actual - expected).abs() <= 1e-9 * expected.abs().max(1.),
                "{path}: {actual} != {expected}"
            );
        }
        Value::Object(object) => {
            for (key, value) in object {
                equivalent(&actual[key], value, &format!("{path}/{key}"));
            }
        }
        Value::Array(items) => {
            let actual = actual
                .as_array()
                .unwrap_or_else(|| panic!("{path}: expected array"));
            assert_eq!(actual.len(), items.len(), "{path}");
            for (i, (actual, expected)) in actual.iter().zip(items).enumerate() {
                equivalent(actual, expected, &format!("{path}/{i}"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}
#[test]
fn native_recipes_preserve_all_examples_and_embedded_model_bytes() {
    let output = std::env::temp_dir().join(format!(
        "boardstudio-demo-acceptance-{}",
        std::process::id()
    ));
    assert!(!output.exists(), "Use a fresh acceptance directory");
    demo_projects::prepare(output.to_str()).unwrap();
    let result = std::panic::catch_unwind(|| {
        let baseline: Value =
            serde_json::from_str(include_str!("fixtures/demo-projects-baseline.json")).unwrap();
        let fixtures = baseline["fixtures"].as_object().unwrap();
        assert_eq!(fixtures.len(), 20);
        for (name, row) in fixtures {
            let mut doc: Value =
                serde_json::from_slice(&fs::read(output.join(format!("{name}.json"))).unwrap())
                    .unwrap();
            assert_eq!(doc["formatVersion"], 2, "{name}");
            doc["assets"]
                .as_array_mut()
                .unwrap()
                .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            equivalent(&doc, &row["expected"], name);
            let bytes = fs::read(output.join(format!("{name}.boardstudio"))).unwrap();
            let (reply, buffers) =
                boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &[bytes]);
            let reply: Value = serde_json::from_str(&reply).unwrap();
            assert_eq!(reply["kind"], "unpacked", "{name}: {reply}");
            let unpacked: Value =
                serde_json::from_str(reply["projectJson"].as_str().unwrap()).unwrap();
            let typed: boardstudio_core::model::ProjectDoc =
                serde_json::from_value(unpacked).unwrap();
            for definition in &typed.definitions {
                if definition.generator.is_some() {
                    boardstudio_core::generators::render_forms(definition, None).unwrap();
                }
            }
            for asset in reply["assets"].as_array().unwrap() {
                let hash = asset["sha256"].as_str().unwrap();
                assert_eq!(
                    buffers[asset["bufferIndex"].as_u64().unwrap() as usize],
                    fs::read(output.join(hash)).unwrap()
                );
            }
        }
        let provenance: Value =
            serde_json::from_slice(&fs::read(output.join("provenance.json")).unwrap()).unwrap();
        assert_eq!(provenance["fixtures"].as_array().unwrap().len(), 20);
        assert!(
            provenance["source_hashes"]
                .get("content/layouts/module-review.json")
                .is_some()
        );
    });
    fs::remove_dir_all(&output).unwrap();
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
