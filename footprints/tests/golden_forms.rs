//! Geometry, model references and export, checked against the golden fixtures by
//! feeding the recorded render text through the Rust pipeline. No generator is
//! involved, so this verifies the framework independently of any port.
mod common;

use std::collections::BTreeMap;

use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::geometry::geometry;
use boardstudio_footprints::models::{model_asset_ids, model_bindings, preview_model_paths};
use boardstudio_footprints::sexpr::{Expr, parse_forms, serialize};
use common::{assert_json_eq, generator_fixtures, read_json};
use serde_json::Value;

struct Recorded {
    generator: String,
    id: String,
    forms: Vec<Expr>,
    case: Value,
}

/// Every case whose render succeeded, with its recorded forms parsed.
fn recorded_cases() -> Vec<Recorded> {
    let mut cases = Vec::new();
    for fixture in generator_fixtures() {
        for case in fixture["cases"].as_array().unwrap() {
            let Some(forms) = case["output"]["forms"]["ok"]["forms"].as_array() else {
                continue;
            };
            let text: Vec<&str> = forms.iter().map(|form| form.as_str().unwrap()).collect();
            let forms = parse_forms(&text.join("\n")).expect("recorded forms parse");
            cases.push(Recorded {
                generator: fixture["source"].as_str().unwrap().to_owned(),
                id: case["id"].as_str().unwrap().to_owned(),
                forms,
                case: case.clone(),
            });
        }
    }
    cases
}

/// Compare a computed result with a recorded `{ok}` or `{error}` output.
fn check<T: serde::Serialize>(
    actual: boardstudio_footprints::Result<T>,
    recorded: &Value,
    context: &str,
) {
    match (&actual, recorded.get("ok"), recorded.get("error")) {
        (Ok(value), Some(expected), _) => {
            assert_json_eq(&serde_json::to_value(value).unwrap(), expected, context)
        }
        (Err(error), _, Some(expected)) => {
            assert_eq!(&error.message(), expected.as_str().unwrap(), "{context}")
        }
        (Ok(_), _, Some(expected)) => panic!("{context}: expected error {expected}"),
        (Err(error), Some(_), _) => panic!("{context}: unexpected error {error}"),
        _ => panic!("{context}: malformed record"),
    }
}

#[test]
fn recorded_forms_round_trip_through_the_parser() {
    for recorded in recorded_cases() {
        let text: Vec<String> = recorded.forms.iter().map(serialize).collect();
        let expected: Vec<&str> = recorded.case["output"]["forms"]["ok"]["forms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|form| form.as_str().unwrap())
            .collect();
        assert_eq!(text, expected, "{} {}", recorded.generator, recorded.id);
    }
}

#[test]
fn geometry_matches_the_recorded_pads_and_courtyards() {
    let mut compared = 0;
    let mut errors = 0;
    for recorded in recorded_cases() {
        let context = format!("{} {}", recorded.generator, recorded.id);
        let expected = &recorded.case["output"]["geometry"];
        errors += usize::from(expected.get("error").is_some());
        check(geometry(&recorded.forms), expected, &context);
        compared += 1;
    }
    println!("geometry: {compared} cases, {errors} recorded errors");
    assert!(compared > 700);
}

#[test]
fn model_references_match() {
    let mut compared = 0;
    for recorded in recorded_cases() {
        let context = format!("{} {}", recorded.generator, recorded.id);
        let output = &recorded.case["output"];
        check(
            model_asset_ids(&recorded.forms),
            &output["modelAssetIds"],
            &format!("{context} ids"),
        );
        check(
            model_bindings(&recorded.forms),
            &output["modelBindings"],
            &format!("{context} bindings"),
        );
        compared += 1;
    }
    assert!(compared > 700);
}

fn has_legacy_arc(forms: &[Expr]) -> bool {
    fn visit(node: &Expr) -> bool {
        let Expr::List(items) = node else {
            return false;
        };
        let arc =
            matches!(items.first(), Some(Expr::Atom(head)) if head == "fp_arc" || head == "gr_arc");
        let angle = items.iter().any(|item| matches!(item, Expr::List(l) if matches!(l.first(), Some(Expr::Atom(h)) if h == "angle")));
        (arc && angle) || items.iter().any(visit)
    }
    forms.iter().any(visit)
}

#[test]
fn export_matches_apart_from_arcs_core_upgrades() {
    let mut compared = 0;
    let mut with_arcs = 0;
    for recorded in recorded_cases() {
        let context = format!("{} {}", recorded.generator, recorded.id);
        let export = &recorded.case["output"]["export"];
        if has_legacy_arc(&recorded.forms) {
            // The recorded text had its arcs upgraded by the old worker; Core's
            // upgrade is checked against these in core's own tests.
            with_arcs += 1;
            continue;
        }
        let paths: BTreeMap<String, String> = match export.get("ok") {
            Some(ok) => serde_json::from_value(ok["modelPaths"].clone()).unwrap(),
            None => BTreeMap::new(),
        };
        let actual = export_forms(recorded.forms.clone(), &paths).map(|exported| {
            let mut value = serde_json::json!({ "footprints": exported.footprints, "objects": exported.objects });
            if let Some(ok) = export.get("ok") {
                value["modelPaths"] = ok["modelPaths"].clone();
            }
            value
        });
        // Net snapshots come from rendering, which these cases do not repeat.
        let mut expected = export.clone();
        if let Some(ok) = expected.get_mut("ok").and_then(Value::as_object_mut) {
            ok.remove("nets");
        }
        check(actual, &expected, &context);
        compared += 1;
    }
    println!("export: {compared} compared, {with_arcs} with legacy arcs deferred to core");
    assert!(compared > 600);
}

#[test]
fn preview_paths_follow_the_recorded_rules() {
    let mut compared = 0;
    for recorded in recorded_cases() {
        let Some(ok) = recorded.case["output"]["export"].get("ok") else {
            continue;
        };
        let Ok(bindings) = model_bindings(&recorded.forms) else {
            continue;
        };
        let supplied: BTreeMap<String, String> = bindings
            .iter()
            .filter(|binding| !binding.asset_id.starts_with("unresolved-model:"))
            .map(|binding| {
                (
                    binding.asset_id.clone(),
                    format!("models/{}", binding.asset_id.replace([':', '/'], "_")),
                )
            })
            .collect();
        let paths = preview_model_paths(&bindings, &supplied).unwrap();
        assert_json_eq(
            &serde_json::to_value(&paths).unwrap(),
            &ok["modelPaths"],
            &format!("{} {}", recorded.generator, recorded.id),
        );
        compared += 1;
    }
    assert!(compared > 600);
}

#[test]
fn parser_behaviour_matches_the_provider_fixture() {
    let provider = read_json("provider.json");
    for entry in provider["parseForms"].as_array().unwrap() {
        let source = entry["source"].as_str().unwrap();
        let recorded = &entry["result"];
        match (
            parse_forms(source),
            recorded.get("ok"),
            recorded.get("error"),
        ) {
            (Ok(forms), Some(ok), _) => {
                let text: Vec<Value> = forms
                    .iter()
                    .map(|form| Value::String(serialize(form)))
                    .collect();
                assert_eq!(&Value::Array(text), ok, "{source:?}");
            }
            (Err(error), _, Some(message)) => {
                assert_eq!(error.message(), message.as_str().unwrap(), "{source:?}")
            }
            (result, ..) => panic!("{source:?}: {result:?} vs {recorded}"),
        }
    }
    let ids = &provider["modelAssetIdsForPaths"];
    let paths: Vec<&str> = ids["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    let expected: Vec<Option<String>> = ids["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect();
    assert_eq!(
        boardstudio_footprints::models::model_asset_ids_for_paths(&paths),
        expected
    );
}
