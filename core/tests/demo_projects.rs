//! Acceptance baseline recorded at 323967ff; its source hashes also match 3cdeb2ac2.
//! Identifiers are mechanically
//! renamed; placements, source references, matrix recipes, wiring and model hashes are frozen.
//! Typed-core-edits ticket 04 deliberately re-recorded `revision` and `hardware` (and the
//! Sofle v2 project-level `mechanical`) for the ten split keyboards, which now carry a
//! mechanical mirror pair; see that ticket's Outcome for the per-keyboard record. Every
//! other recorded value, including all key positions, is unchanged. Generator-parameters
//! ticket 02 re-recorded `revision`, `hardware`, `parts`, `definitions` and the boards'
//! `partIds` for the same ten: each half keeps its own `case-closure/*` holes (none on
//! Sofle v2, whose gasket mount carries no closure mounts). No existing part moved.
//! Generator-parameters ticket 06 (ADR-0008) re-recorded `revision`, `hardware`, `parts`,
//! `definitions` and `boards` for the same nine paired splits: the second half's controller
//! bay, reset switch and TRRS jack moved to the reflection of the first half's, and every
//! closure mount is now one linked pair feature, so each half's holes are regenerated from it.
//! Sofle v2 is unchanged. Every key and matrix value is still the recorded one.
#[path = "../examples/demo_projects/mod.rs"]
#[allow(clippy::module_inception)] // The module is the shared example source, included by path.
pub(crate) mod demo_projects; // Shared with `included_keyboards`, which cannot include the files twice.
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

fn pairs_of(
    document: &boardstudio_core::model::ProjectDoc,
) -> &[boardstudio_core::model::MechanicalMirrorPair] {
    document
        .hardware
        .as_ref()
        .map_or(&[], |hardware| hardware.pairs.as_slice())
}

/// A document saved before format 5 has no `constructionSource` or `halves`. Migrating it
/// changes only `formatVersion`, and decoding then encoding it adds nothing: a pair without the
/// new fields stays unpaired (ADR-0004, ADR-0010). The shipped projects stand for saved ones.
fn version_4_form_migrates_unchanged(name: &str, current: &Value) {
    let mut saved = current.clone();
    saved["formatVersion"] = 4.into();
    for pair in saved
        .pointer_mut("/hardware/pairs")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        pair.as_object_mut().unwrap().remove("constructionSource");
        assert!(
            pair.get("halves").is_none(),
            "{name}: no shipped record yet"
        );
    }
    let (migrated, changed) = boardstudio_core::migrate::migrate_value(saved.clone()).unwrap();
    assert!(changed, "{name}");
    let mut expected = saved;
    expected["formatVersion"] = 5.into();
    assert_eq!(migrated, expected, "{name}: only formatVersion changes");
    let document: boardstudio_core::model::ProjectDoc = serde_json::from_value(migrated).unwrap();
    for pair in pairs_of(&document) {
        assert_eq!(
            (&pair.construction_source, &pair.halves),
            (&None, &None),
            "{name}"
        );
    }
    assert_eq!(
        serde_json::to_value(pairs_of(&document)).unwrap(),
        expected
            .pointer("/hardware/pairs")
            .cloned()
            .unwrap_or(Value::Array(Vec::new())),
        "{name}: a version 4 document's pairs round-trip without new fields"
    );
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
            assert_eq!(doc["formatVersion"], 5, "{name}");
            doc["assets"]
                .as_array_mut()
                .unwrap()
                .sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            equivalent(&doc, &row["expected"], name);
            version_4_form_migrates_unchanged(name, &doc);
            let bytes = fs::read(output.join(format!("{name}.boardstudio"))).unwrap();
            let (reply, buffers) =
                boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &[bytes]);
            let reply: Value = serde_json::from_str(&reply).unwrap();
            assert_eq!(reply["kind"], "unpacked", "{name}: {reply}");
            let unpacked: Value =
                serde_json::from_str(reply["projectJson"].as_str().unwrap()).unwrap();
            // The exported archive reopens with its mechanical pairs and exceptions intact.
            let mut engine = boardstudio_core::CoreEngine::new();
            let opened: Value = serde_json::from_str(&engine.request(
                &serde_json::json!({"id":"open","kind":"open","document":unpacked}).to_string(),
            ))
            .unwrap();
            assert_eq!(opened["kind"], "scene", "{name}: {opened}");
            // Shipped projects carry no generator parameter the schema rejects (ADR-0009).
            assert!(
                opened["scene"]["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|finding| !finding["id"]
                        .as_str()
                        .unwrap()
                        .starts_with("generator-parameter:")),
                "{name}: {opened}"
            );
            // (Opening may normalize a prebuilt archive's other hardware fields.)
            let pairs = |document: &Value| {
                document["hardware"]["pairs"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            };
            assert_eq!(pairs(&opened["document"]), pairs(&doc), "{name}");
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
