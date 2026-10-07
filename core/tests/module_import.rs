use serde_json::{Value, json};

fn import(source: String, id: &str) -> Value {
    serde_json::from_str(&boardstudio_core::artifact::request(&json!({
        "id":"import","kind":"import-module-board","definitionId":id,
        "name":"Fixture module","source":source,"family":"fixture","variant":"source",
        "provenance":{"repository":"https://example.invalid/repo","revision":"abc","path":"board.kicad_pcb","license":"MIT"}
    }).to_string())).unwrap()
}

#[test]
fn module_import_rejects_duplicate_source_references_before_namespacing_a_circuit() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    )
    .replace(
        "(fp_text reference \"J1002\"",
        "(fp_text reference \"J1001\"",
    );
    let result = import(source, "fixture:duplicate");
    assert_eq!(
        result["kind"], "error",
        "duplicate references must not collapse local part identities"
    );
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("duplicate footprint reference"),
        "{result}"
    );
}

#[test]
fn module_import_gives_unannotated_footprints_unique_circuit_identities() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    )
    .to_owned();
    let result = import(source, "fixture:splitter");
    assert_eq!(result["kind"], "import-module-board", "{result}");
    let parts = result["result"]["circuit"]["parts"].as_array().unwrap();
    let ids: std::collections::BTreeSet<_> = parts
        .iter()
        .map(|part| part["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids.len(),
        parts.len(),
        "every imported circuit part needs an independent identity"
    );
    assert!(
        ids.iter().any(|id| id.starts_with("PART")),
        "unannotated source references receive deterministic ids"
    );
}
