//! Every ported generator against its golden fixture: parameter schema, catalogue
//! entry, and every recorded case (render text, nets, geometry, normalization,
//! model references, export), apart from the deviations listed in
//! `docs/investigations/footprint-generators-rust.md`.
//!
//! Forms compare as parsed trees re-serialized (whitespace normalised, numbers
//! as the exact text the generator printed).
#[path = "generators/parameter_order.rs"]
mod parameter_order;
use parameter_order::parameter_key_order;

use std::collections::BTreeMap;

use crate::common::{self, generator_fixtures, json_eq};
use boardstudio_footprints::definition::DefinitionInput;
use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::geometry::geometry;
use boardstudio_footprints::models::{model_asset_ids, model_bindings};
use boardstudio_footprints::nets::{NetAllocator, NetIndexer, NoNets};
use boardstudio_footprints::registry::Registry;
use boardstudio_footprints::sexpr::{Expr, serialize};
use boardstudio_footprints::types::{GeneratorRef, PartRef, ReservedNet, Side};
use boardstudio_footprints::{GeneratorError, bundled};
use serde_json::{Value, json};

const DIODE_SOD123_BASELINE_SOURCE: &str = "ceoloide/diode_tht_sod123";
const DIODE_SOD123_QUALIFICATION_FLAG: &str = "use_vishay_1n4148w_land_pattern";
const DIODE_SOD123_SCHEMA_NAMES: [&str; 17] = [
    "side",
    "designator",
    "reversible",
    "include_traces_vias",
    "trace_distance",
    "trace_width",
    "via_size",
    "via_drill",
    "include_tht",
    "include_thru_hole_smd_pads",
    "diode_3dmodel_filename",
    "diode_3dmodel_xyz_offset",
    "diode_3dmodel_xyz_rotation",
    "diode_3dmodel_xyz_scale",
    "from",
    "to",
    DIODE_SOD123_QUALIFICATION_FLAG,
];
/// Corrected quirks: generator text that changes on purpose.
fn corrected_text(source: &str, text: &str) -> String {
    match source {
        // The old bodies read a point name that never existed and printed "undefined".
        "ceoloide/utility_point_debugger" => {
            text.replace("(fp_text user \"undefined\"", "(fp_text user \"\"")
        }
        "infused-kim/point_debugger" => {
            text.replace("(fp_text user undefined", "(fp_text user \"\"")
        }
        _ => text.to_owned(),
    }
}

struct Run<'a> {
    registry: &'static Registry,
    source: &'a str,
    fixture: &'a Value,
    /// Top-level key names of the fixture's `"parameters"` object in file
    /// order (see [`parameter_key_order`]). `None` when the order was never
    /// extracted; [`check_schema`] then records an order failure instead of
    /// comparing against the parsed map, whose key order is unspecified.
    parameter_order: Option<Vec<String>>,
    failures: Vec<String>,
}

impl Run<'_> {
    fn fail(&mut self, case: &str, what: &str, detail: String) {
        self.failures
            .push(format!("{} {case} {what}: {detail}", self.source));
    }
}

/// How a recorded output relates to the Rust result.
enum Expect<'a> {
    Ok(&'a Value),
    Error(&'a str),
}

fn recorded(output: &Value) -> Expect<'_> {
    match (output.get("ok"), output.get("error")) {
        (Some(ok), _) => Expect::Ok(ok),
        (_, Some(error)) => Expect::Error(error.as_str().expect("error text")),
        _ => panic!("malformed output {output}"),
    }
}

fn compare<T: serde::Serialize>(
    run: &mut Run,
    case: &str,
    what: &str,
    actual: Result<T, GeneratorError>,
    expected: &Value,
) {
    match (actual, recorded(expected)) {
        (Ok(value), Expect::Ok(expected)) => {
            let value = serde_json::to_value(value).unwrap();
            if !json_eq(&value, expected) {
                run.fail(
                    case,
                    what,
                    format!("\n  actual:   {value}\n  expected: {expected}"),
                );
            }
        }
        (Err(error), Expect::Error(message)) => {
            if error.message() != message {
                run.fail(
                    case,
                    what,
                    format!("error {:?} vs {message:?}", error.message()),
                );
            }
        }
        (Ok(_), Expect::Error(message)) => {
            run.fail(case, what, format!("expected error {message:?}"))
        }
        (Err(error), Expect::Ok(_)) => run.fail(case, what, format!("unexpected error {error:?}")),
    }
}

fn has_legacy_arc(forms: &[Expr]) -> bool {
    fn visit(node: &Expr) -> bool {
        let Expr::List(items) = node else {
            return false;
        };
        let arc =
            matches!(items.first(), Some(Expr::Atom(head)) if head == "fp_arc" || head == "gr_arc");
        let angle = items
            .iter()
            .any(|item| matches!(item, Expr::List(l) if matches!(l.first(), Some(Expr::Atom(h)) if h == "angle")));
        (arc && angle) || items.iter().any(visit)
    }
    forms.iter().any(visit)
}

/// Back-side parts used to inherit `side` B when none was saved; the 1→2
/// migration writes it explicitly.
fn migrate_side(part: &mut PartRef, definition_parameters: &BTreeMap<String, Value>) {
    let params = part
        .generator_parameters
        .get_or_insert_with(Default::default);
    if part.side == Side::Back
        && !params.contains_key("side")
        && !definition_parameters.contains_key("side")
    {
        params.insert("side".into(), json!("B"));
    }
}

struct Input {
    definition_id: String,
    generator: GeneratorRef,
    part: Option<PartRef>,
    nets: Option<(Vec<ReservedNet>, u32)>,
}

fn input_of(fixture: &Value, case: &Value) -> Input {
    let input = &case["input"];
    let catalogue = &fixture["catalogue"];
    let (definition_id, parameters) = if let Some(definition) = input.get("definition") {
        (
            definition["id"].as_str().unwrap().to_owned(),
            definition["generator"]["parameters"].clone(),
        )
    } else {
        (
            input
                .get("definitionId")
                .and_then(Value::as_str)
                .unwrap_or(catalogue["id"].as_str().unwrap())
                .to_owned(),
            input["definitionParameters"].clone(),
        )
    };
    let generator = GeneratorRef {
        source: catalogue["generator"]["source"]
            .as_str()
            .unwrap()
            .to_owned(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).unwrap(),
    };
    let mut part: Option<PartRef> = input
        .get("part")
        .filter(|part| !part.is_null())
        .map(|part| serde_json::from_value(part.clone()).unwrap());
    if let Some(part) = &mut part {
        migrate_side(part, &generator.parameters);
    }
    let nets = part.as_ref().map(|_| {
        let reserved: Vec<ReservedNet> =
            serde_json::from_value(input["reservedNets"].clone()).unwrap();
        (reserved, input["nextNetIndex"].as_u64().unwrap() as u32)
    });
    Input {
        definition_id,
        generator,
        part,
        nets,
    }
}

/// The definition normalization reads, rebuilt the way the recorder built it.
fn definition_of(fixture: &Value, case: &Value, input: &Input) -> DefinitionInput {
    let mut definition = fixture["catalogue"].clone();
    definition["id"] = json!(input.definition_id);
    definition["generator"]["parameters"] =
        serde_json::to_value(&input.generator.parameters).unwrap();
    let pads = definition["pads"].as_array().cloned().unwrap_or_default();
    if case["input"]["savedPads"].as_bool() == Some(true) {
        definition["pads"] = Value::Array(
            pads.iter()
                .enumerate()
                .map(|(index, pad)| {
                    let mut pad = pad.clone();
                    pad["id"] = json!(format!("saved-{index}"));
                    pad["netId"] = json!(format!("net-{index}"));
                    pad
                })
                .collect(),
        );
    } else if let Some(count) = case["input"]["definition"]["savedPadCount"].as_u64() {
        // Demo definitions kept the pad IDs and nets of the document that saved them.
        let expected = &case["output"]["normalized"]["ok"]["pads"];
        definition["pads"] = Value::Array(
            (0..count as usize)
                .map(|index| {
                    let mut pad = pads.first().cloned().unwrap_or(json!({}));
                    pad["id"] = expected[index]["id"].clone();
                    match expected[index].get("netId") {
                        Some(net) => pad["netId"] = net.clone(),
                        None => {
                            pad.as_object_mut().map(|object| object.remove("netId"));
                        }
                    }
                    pad
                })
                .collect(),
        );
    }
    serde_json::from_value(definition).expect("definition input")
}

fn render_forms(
    run: &Run,
    input: &Input,
) -> (Result<Vec<Expr>, GeneratorError>, Option<Vec<ReservedNet>>) {
    match &input.nets {
        Some((reserved, next)) => {
            let mut nets = NetAllocator::new(reserved, *next);
            let forms = run.registry.render(
                &input.definition_id,
                &input.generator,
                input.part.as_ref(),
                &mut nets as &mut dyn NetIndexer,
            );
            (forms, Some(nets.snapshot()))
        }
        None => (
            run.registry.render(
                &input.definition_id,
                &input.generator,
                input.part.as_ref(),
                &mut NoNets,
            ),
            None,
        ),
    }
}

fn check_case(run: &mut Run, case: &Value) {
    let id = case["id"].as_str().unwrap().to_owned();
    let output = &case["output"];
    let input = input_of(run.fixture, case);
    let (forms, nets) = render_forms(run, &input);

    // The old message was V8's JSON parser text, which is not a baseline.
    if (run.source, id.as_str()) == ("ceoloide/utility_router", "extra:error-bad-position") {
        if !matches!(forms, Err(GeneratorError::Rejected { .. })) {
            run.fail(&id, "forms", format!("expected a rejection, got {forms:?}"));
        }
        return;
    }

    // Numeric strings for number parameters are rejected now (listed deviation).
    if id == "coerce:numeric-strings"
        || (id == "extra:string-values" && run.source == "ceoloide/mounting_hole_npth")
    {
        let has_numbers = run
            .registry
            .parameters(run.source)
            .unwrap()
            .iter()
            .any(|p| p.kind == boardstudio_footprints::params::ParamKind::Number);
        match &forms {
            Err(GeneratorError::InvalidParameter { .. }) => {}
            other if has_numbers => run.fail(
                &id,
                "numeric strings",
                format!("expected InvalidParameter, got {other:?}"),
            ),
            _ => {}
        }
        if has_numbers {
            return;
        }
    }

    // Forms and nets.
    let expected_forms = &output["forms"];
    match (&forms, recorded(expected_forms)) {
        (Ok(forms), Expect::Ok(expected)) => {
            let text: Vec<Value> = forms.iter().map(|f| Value::String(serialize(f))).collect();
            let wanted: Vec<Value> = expected["forms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| Value::String(corrected_text(run.source, f.as_str().unwrap())))
                .collect();
            if text != wanted {
                let first = text
                    .iter()
                    .zip(&wanted)
                    .position(|(a, b)| a != b)
                    .unwrap_or(text.len().min(wanted.len()));
                let at = |list: &[Value]| {
                    list.get(first)
                        .map(|v| v.as_str().unwrap_or("").to_owned())
                        .unwrap_or_default()
                };
                let (a, b) = (at(&text), at(&wanted));
                let offset = a
                    .bytes()
                    .zip(b.bytes())
                    .position(|(x, y)| x != y)
                    .unwrap_or(a.len().min(b.len()));
                let window =
                    |s: &str| s[offset.saturating_sub(60)..(offset + 100).min(s.len())].to_owned();
                run.fail(
                    &id,
                    "forms",
                    format!(
                        "form {first} differs near byte {offset}\n  actual:   …{}\n  expected: …{}",
                        window(&a),
                        window(&b)
                    ),
                );
            }
            if let (Some(actual), Some(expected)) = (&nets, expected.get("nets"))
                && !json_eq(&serde_json::to_value(actual).unwrap(), expected)
            {
                run.fail(&id, "nets", format!("{actual:?} vs {expected}"));
            }
        }
        (Err(error), Expect::Error(message)) => {
            if error.message() != message {
                run.fail(
                    &id,
                    "forms error",
                    format!("{:?} vs {message:?}", error.message()),
                );
            }
        }
        (Ok(_), Expect::Error(message)) => {
            run.fail(&id, "forms", format!("expected error {message:?}"))
        }
        (Err(error), Expect::Ok(_)) => run.fail(
            &id,
            "forms",
            format!("unexpected error {:?}", error.message()),
        ),
    }

    if let Ok(forms) = &forms
        && output["forms"].get("ok").is_some()
    {
        {
            compare(run, &id, "geometry", geometry(forms), &output["geometry"]);
            compare(
                run,
                &id,
                "modelAssetIds",
                model_asset_ids(forms),
                &output["modelAssetIds"],
            );
            compare(
                run,
                &id,
                "modelBindings",
                model_bindings(forms),
                &output["modelBindings"],
            );
            if !has_legacy_arc(forms) {
                let paths: BTreeMap<String, String> = match output["export"].get("ok") {
                    Some(ok) => serde_json::from_value(ok["modelPaths"].clone()).unwrap(),
                    None => BTreeMap::new(),
                };
                let actual = export_forms(forms.clone(), &paths).map(|e| json!({ "footprints": e.footprints, "objects": e.objects, "modelPaths": paths }));
                let mut expected = output["export"].clone();
                if let Some(ok) = expected.get_mut("ok").and_then(Value::as_object_mut) {
                    ok.remove("nets");
                    if run.source == "ceoloide/utility_point_debugger"
                        || run.source == "infused-kim/point_debugger"
                    {
                        for key in ["footprints", "objects"] {
                            if let Some(list) = ok.get_mut(key).and_then(Value::as_array_mut) {
                                for text in list {
                                    *text = Value::String(corrected_text(
                                        run.source,
                                        text.as_str().unwrap(),
                                    ));
                                }
                            }
                        }
                    }
                }
                compare(run, &id, "export", actual, &expected);
            }
        }
    }

    // Normalization.
    if output.get("normalized").is_some() {
        let definition = definition_of(run.fixture, case, &input);
        let actual = run
            .registry
            .normalize(&definition)
            .map(|n| n.expect("generator definition"));
        let mut expected = output["normalized"].clone();
        if let Some(ok) = expected.get_mut("ok").and_then(Value::as_object_mut) {
            // Only the fields normalization owns.
            ok.retain(|key, _| {
                matches!(
                    key.as_str(),
                    "pads"
                        | "courtyard"
                        | "keycap"
                        | "envelopeSource"
                        | "terminals"
                        | "envelopeNotice"
                )
            });
        }
        compare(run, &id, "normalized", actual, &expected);
    }
}

/// Deviations in the parameter schema (plan: Deviations).
fn check_schema(run: &mut Run) {
    let actual = run.registry.parameters(run.source).unwrap();
    let expected = run.fixture["parameters"].as_object().unwrap();
    let diode_schema = run.source == DIODE_SOD123_BASELINE_SOURCE;
    if diode_schema {
        if actual.len() != DIODE_SOD123_SCHEMA_NAMES.len() {
            run.fail(
                "schema",
                "length",
                format!("{} vs {}", actual.len(), DIODE_SOD123_SCHEMA_NAMES.len()),
            );
        }
        let names: Vec<&str> = actual
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect();
        if names != DIODE_SOD123_SCHEMA_NAMES {
            run.fail(
                "schema",
                "names",
                format!("{names:?} vs {DIODE_SOD123_SCHEMA_NAMES:?}"),
            );
        }
    }
    let side_deviates = matches!(
        run.source,
        "ceoloide/diode_tht_sod123"
            | "ceoloide/led_sk6812mini-e"
            | "ceoloide/switch_choc_v1_v2"
            | "ceoloide/switch_gateron_ks27_ks33"
            | "ceoloide/switch_mx"
            | "ceoloide/utility_keepout_zone"
            | "infused-kim/trackpoint_mount"
    );
    if !diode_schema {
        let order: Vec<&str> = actual
            .iter()
            .map(|p| p.name.as_str())
            .filter(|n| *n != "side")
            .collect();
        match &run.parameter_order {
            // File order from the raw fixture text: the parsed `Value` map
            // no longer preserves insertion order.
            Some(recorded) => {
                let wanted: Vec<&str> = recorded
                    .iter()
                    .map(String::as_str)
                    .filter(|n| *n != "side")
                    .collect();
                if order != wanted {
                    run.fail("schema", "order", format!("{order:?} vs {wanted:?}"));
                }
            }
            None => run.fail("schema", "order", "parameter file order unavailable".into()),
        }
    }
    for parameter in &actual {
        let mut value = serde_json::to_value(parameter).unwrap();
        if parameter.name == DIODE_SOD123_QUALIFICATION_FLAG {
            if diode_schema
                && value
                    == json!({"name": DIODE_SOD123_QUALIFICATION_FLAG, "type": "boolean", "value": false})
            {
                continue;
            }
            run.fail(
                "schema",
                &parameter.name,
                format!("unexpected declaration {value}"),
            );
            continue;
        }
        value.as_object_mut().unwrap().remove("name");
        let Some(recorded) = expected.get(&parameter.name) else {
            if parameter.name != "side" {
                run.fail("schema", &parameter.name, "not in the old schema".into());
            }
            continue;
        };
        let vector = parameter.name.contains("3dmodel_xyz_")
            && recorded == &json!({"type": "string", "value": ""})
            && value == json!({"type": "array", "value": []});
        let hole = run.source == "ceoloide/mounting_hole_npth"
            && parameter.name.starts_with("hole_")
            && value == json!({"type": "number", "value": 2.2})
            && recorded == &json!({"type": "string", "value": "2.2"});
        let side = parameter.name == "side"
            && side_deviates
            && value == json!({"type": "string", "value": "F"});
        if !(vector || hole || side || json_eq(&value, recorded)) {
            run.fail("schema", &parameter.name, format!("{value} vs {recorded}"));
        }
    }
}

fn check_catalogue(run: &mut Run) {
    let entry = run
        .registry
        .catalogue()
        .unwrap()
        .into_iter()
        .find(|e| e.generator.source == run.source)
        .unwrap();
    let mut value = serde_json::to_value(entry).unwrap();
    let mut expected = run.fixture["catalogue"].clone();
    if let (Some(a), Some(b)) = (value.as_object_mut(), expected.as_object_mut()) {
        // The recorded catalogue carries the old display of unset optional fields.
        for key in ["envelopeNotice"] {
            if a.get(key).is_none() && b.get(key).is_none() {
                a.remove(key);
            }
        }
    }
    if !json_eq(&value, &expected) {
        run.fail(
            "catalogue",
            "entry",
            format!("\n  actual:   {value}\n  expected: {expected}"),
        );
    }
}

fn run_generator(registry: &'static Registry, fixture: &Value, fixture_raw: &str) -> Vec<String> {
    let source = fixture["source"].as_str().unwrap().to_owned();
    let mut run = Run {
        registry,
        source: &source,
        fixture,
        parameter_order: None,
        failures: Vec::new(),
    };
    match parameter_key_order(fixture_raw) {
        Ok(order) => run.parameter_order = Some(order),
        Err(error) => run.fail("schema", "order", error),
    }
    check_schema(&mut run);
    check_catalogue(&mut run);
    let cases = fixture["cases"].as_array().unwrap();
    for case in cases {
        check_case(&mut run, case);
    }
    run.failures
}

fn assert_generator_matches_golden(
    registry: &'static Registry,
    fixture: &Value,
    fixture_raw: &str,
) {
    let failures = run_generator(registry, fixture, fixture_raw);
    if !failures.is_empty() {
        let shown: Vec<_> = failures.iter().take(12).cloned().collect();
        panic!(
            "{}: {} mismatches (showing {}):\n{}",
            fixture["source"],
            failures.len(),
            shown.len(),
            shown.join("\n")
        );
    }
}

macro_rules! generator_goldens {
    ($($test:ident: $file:literal),+ $(,)?) => {
        $(
            #[test]
            fn $test() {
                let (fixture, fixture_raw) = common::read_json_raw(concat!("generators/", $file));
                assert_generator_matches_golden(bundled(), &fixture, &fixture_raw);
            }
        )+

        #[test]
        fn unlisted_bundled_generators_match_the_golden_baseline() {
            let listed = [$(common::read_json(concat!("generators/", $file))),+];
            let registry = bundled();
            for (fixture, fixture_raw) in common::generator_fixture_raws() {
                let source = fixture["source"].as_str().unwrap();
                if registry.contains(source) && !listed.contains(&fixture) {
                    assert_generator_matches_golden(registry, &fixture, &fixture_raw);
                }
            }
        }
    };
}

generator_goldens! {
    golden_ceoloide_battery_connector_jst_ph_2: "ceoloide__battery_connector_jst_ph_2.json",
    golden_ceoloide_battery_connector_molex_pico_ezmate_1x02: "ceoloide__battery_connector_molex_pico_ezmate_1x02.json",
    golden_ceoloide_diode_tht_sod123: "ceoloide__diode_tht_sod123.json",
    golden_ceoloide_display_nice_view: "ceoloide__display_nice_view.json",
    golden_ceoloide_display_ssd1306: "ceoloide__display_ssd1306.json",
    golden_ceoloide_led_sk6812mini_e: "ceoloide__led_sk6812mini-e.json",
    golden_ceoloide_mcu_nice_nano: "ceoloide__mcu_nice_nano.json",
    golden_ceoloide_mcu_supermini_nrf52840: "ceoloide__mcu_supermini_nrf52840.json",
    golden_ceoloide_mounting_hole_npth: "ceoloide__mounting_hole_npth.json",
    golden_ceoloide_mounting_hole_plated: "ceoloide__mounting_hole_plated.json",
    golden_ceoloide_power_switch_smd_side: "ceoloide__power_switch_smd_side.json",
    golden_ceoloide_reset_switch_smd_side: "ceoloide__reset_switch_smd_side.json",
    golden_ceoloide_reset_switch_tht_top: "ceoloide__reset_switch_tht_top.json",
    golden_ceoloide_rotary_encoder_ec11_ec12: "ceoloide__rotary_encoder_ec11_ec12.json",
    golden_ceoloide_switch_choc_v1_v2: "ceoloide__switch_choc_v1_v2.json",
    golden_ceoloide_switch_gateron_ks27_ks33: "ceoloide__switch_gateron_ks27_ks33.json",
    golden_ceoloide_switch_mx: "ceoloide__switch_mx.json",
    golden_ceoloide_trrs_pj320a: "ceoloide__trrs_pj320a.json",
    golden_ceoloide_utility_filled_zone: "ceoloide__utility_filled_zone.json",
    golden_ceoloide_utility_keepout_zone: "ceoloide__utility_keepout_zone.json",
    golden_ceoloide_utility_logo: "ceoloide__utility_logo.json",
    golden_ceoloide_utility_point_debugger: "ceoloide__utility_point_debugger.json",
    golden_ceoloide_utility_router: "ceoloide__utility_router.json",
    golden_ceoloide_utility_text: "ceoloide__utility_text.json",
    golden_infused_kim_conn_molex_pico_ezmate_1x02: "infused-kim__conn_molex_pico_ezmate_1x02.json",
    golden_infused_kim_conn_molex_pico_ezmate_1x05: "infused-kim__conn_molex_pico_ezmate_1x05.json",
    golden_infused_kim_icon_bat: "infused-kim__icon_bat.json",
    golden_infused_kim_mounting_hole: "infused-kim__mounting_hole.json",
    golden_infused_kim_nice_view: "infused-kim__nice_view.json",
    golden_infused_kim_pads: "infused-kim__pads.json",
    golden_infused_kim_point_debugger: "infused-kim__point_debugger.json",
    golden_infused_kim_smd_0805: "infused-kim__smd_0805.json",
    golden_infused_kim_switch_power: "infused-kim__switch_power.json",
    golden_infused_kim_switch_reset: "infused-kim__switch_reset.json",
    golden_infused_kim_text: "infused-kim__text.json",
    golden_infused_kim_trackpoint_mount: "infused-kim__trackpoint_mount.json",
}

/// `parameter_key_order` reads the exact key sequence, not sorted order.
#[test]
fn parameter_key_order_reads_file_order() {
    let raw = r#"{"source": "x", "parameters": {"zeta": 1, "alpha": 2, "mid": 3}}"#;
    assert_eq!(
        parameter_key_order(raw).unwrap(),
        vec!["zeta".to_owned(), "alpha".to_owned(), "mid".to_owned()]
    );
}

/// Inner keys of nested objects and arrays of objects must not leak out, and
/// braces, quotes, commas, or colons inside string values must not confuse
/// the scanner.
#[test]
fn parameter_key_order_ignores_nested_keys_and_string_punctuation() {
    let raw = r#"{
        "source": "x",
        "parameters": {
            "first": {"inner": 1, "list": [{"deep": true}, 2]},
            "second": "} \", : , { not json",
            "third": [1, {"nope": []}],
            "fourth": null
        },
        "cases": []
    }"#;
    assert_eq!(
        parameter_key_order(raw).unwrap(),
        vec![
            "first".to_owned(),
            "second".to_owned(),
            "third".to_owned(),
            "fourth".to_owned()
        ]
    );
}

/// Escaped key names decode exactly as `serde_json` parses them.
#[test]
fn parameter_key_order_decodes_escaped_key_names() {
    let raw = r#"{"parameters": {"a\"b": 1, "c\\d": 2, "\"e\"": 3, "\u0041bc": 4}}"#;
    assert_eq!(
        parameter_key_order(raw).unwrap(),
        vec![
            "a\"b".to_owned(),
            "c\\d".to_owned(),
            "\"e\"".to_owned(),
            "Abc".to_owned()
        ]
    );
    // The decoded names match what the parsed `Value` map holds.
    let parsed: Value = serde_json::from_str(raw).unwrap();
    let mut from_map: Vec<String> = parsed["parameters"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    from_map.sort();
    let mut from_raw = parameter_key_order(raw).unwrap();
    from_raw.sort();
    assert_eq!(from_raw, from_map);
}

/// Whitespace around tokens and an empty `"parameters"` object are fine,
/// while a missing or non-object `"parameters"` is an error.
#[test]
fn parameter_key_order_handles_whitespace_and_reports_missing() {
    let raw = "{ \n \"parameters\" \t : { } , \"cases\" : [ ] }";
    assert_eq!(parameter_key_order(raw).unwrap(), Vec::<String>::new());
    assert!(parameter_key_order(r#"{"source": "x"}"#).is_err());
    assert!(parameter_key_order(r#"{"parameters": []}"#).is_err());
    assert!(parameter_key_order(r#"{"parameters": {"a": 1"#).is_err());
}

/// Swapping two keys in the raw text changes the reported order, so
/// `check_schema` still detects reordered `"parameters"` keys.
#[test]
fn parameter_key_order_detects_reordered_keys() {
    let raw = r#"{"parameters": {"one": 1, "two": 2, "three": 3}}"#;
    let swapped = r#"{"parameters": {"three": 3, "two": 2, "one": 1}}"#;
    assert_eq!(
        parameter_key_order(raw).unwrap(),
        vec!["one".to_owned(), "two".to_owned(), "three".to_owned()]
    );
    assert_eq!(
        parameter_key_order(swapped).unwrap(),
        vec!["three".to_owned(), "two".to_owned(), "one".to_owned()]
    );
    assert_ne!(
        parameter_key_order(raw).unwrap(),
        parameter_key_order(swapped).unwrap()
    );
}

/// RED: `check_schema` fails when the recorded file order is swapped, even
/// though every parameter name and value still matches the fixture.
#[test]
fn check_schema_rejects_reordered_parameters() {
    let (fixture, fixture_raw) = common::read_json_raw("generators/ceoloide__switch_mx.json");
    let mut swapped = parameter_key_order(&fixture_raw).expect("parameter order");
    assert!(swapped.len() >= 2, "fixture needs at least two parameters");
    let last = swapped.len() - 1;
    swapped.swap(0, last);
    assert_ne!(
        swapped,
        parameter_key_order(&fixture_raw).expect("parameter order")
    );
    let registry = bundled();
    let source = fixture["source"].as_str().unwrap().to_owned();
    let mut run = Run {
        registry,
        source: &source,
        fixture: &fixture,
        parameter_order: Some(swapped),
        failures: Vec::new(),
    };
    check_schema(&mut run);
    assert!(
        run.failures
            .iter()
            .any(|failure| failure.contains("schema order")),
        "swapped parameters must fail the order check, got: {run_failures:?}",
        run_failures = run.failures
    );
}

#[test]
fn every_registered_generator_has_a_fixture() {
    let sources: Vec<String> = generator_fixtures()
        .iter()
        .map(|f| f["source"].as_str().unwrap().to_owned())
        .collect();
    for spec in bundled().specs() {
        assert!(
            sources.iter().any(|s| s == spec.source),
            "no golden fixture for {}",
            spec.source
        );
    }
}
