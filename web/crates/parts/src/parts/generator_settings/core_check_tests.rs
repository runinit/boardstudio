use super::*;
use boardstudio_core_kernel::generators;
use wasm_bindgen_test::*;

fn definition(source: &str) -> PartDefinition {
    generators::catalogue()
        .expect("catalogue")
        .into_iter()
        .find(|entry| {
            entry
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
        .expect("bundled generator")
}

fn submit(source: &str, edits: Value) -> Result<PartDefinition, String> {
    let base = definition(source);
    let schema = generators::parameter_schema(source).expect("schema");
    let edits: BTreeMap<String, Value> =
        serde_json::from_value(edits).expect("edits are an object");
    apply_input(&base, &schema, &edits)
}

#[wasm_bindgen_test]
fn submitted_numbers_and_lists_pass_the_core_check() {
    let hole = submit(
        "ceoloide/mounting_hole_npth",
        serde_json::json!({ "hole_size": " 2.5 ", "hole_drill": "2.2" }),
    )
    .expect("numeric text is coerced");
    assert_eq!(
        generators::definition_error(&hole),
        Ok(()),
        "the coerced mounting hole passes Core"
    );
    assert!(hole.generator.as_ref().unwrap().parameters["hole_size"].is_number());

    let mcu = submit(
        "ceoloide/mcu_nice_nano",
        serde_json::json!({ "mcu_3dmodel_xyz_offset": "[1, 2, 3]", "designator": "U9" }),
    )
    .expect("list text that parses as JSON is coerced");
    assert_eq!(generators::definition_error(&mcu), Ok(()));
    assert!(mcu.generator.as_ref().unwrap().parameters["mcu_3dmodel_xyz_offset"].is_array());
}

#[wasm_bindgen_test]
fn text_that_does_not_parse_is_refused_before_it_reaches_core() {
    assert!(
        submit(
            "ceoloide/mounting_hole_npth",
            serde_json::json!({ "hole_size": "two" })
        )
        .is_err()
    );
    assert!(
        submit(
            "ceoloide/mcu_nice_nano",
            serde_json::json!({ "mcu_3dmodel_xyz_offset": "[1, 2" })
        )
        .is_err()
    );
}

#[wasm_bindgen_test]
fn the_check_would_catch_text_the_form_did_not_coerce() {
    // Negative control: a value written past the form is the wrong type for Core.
    let mut written = definition("ceoloide/mounting_hole_npth");
    written
        .generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("hole_size".into(), Value::from("2"));
    let error = generators::definition_error(&written).expect_err("text for a number");
    assert!(error.contains("hole_size"), "{error}");
    assert!(error.contains("ceoloide/mounting_hole_npth"), "{error}");
}
