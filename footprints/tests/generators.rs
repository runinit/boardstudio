//! Every ported generator against its golden fixture: parameter schema, catalogue
//! entry, and every recorded case (render text, nets, geometry, normalization,
//! model references, export), apart from the deviations listed in
//! `docs/investigations/footprint-generators-rust.md`.
//!
//! Forms compare as parsed trees re-serialized (whitespace normalised, numbers
//! as the exact text the generator printed).
mod common;

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use boardstudio_footprints::definition::DefinitionInput;
use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::geometry::geometry;
use boardstudio_footprints::models::{model_asset_ids, model_bindings};
use boardstudio_footprints::nets::{NetAllocator, NetIndexer, NoNets};
use boardstudio_footprints::registry::Registry;
use boardstudio_footprints::sexpr::{Expr, serialize};
use boardstudio_footprints::types::{GeneratorRef, PartRef, ReservedNet, Side};
use boardstudio_footprints::{GeneratorError, bundled};
use common::{generator_fixtures, json_eq};
use serde_json::{Value, json};

const DIODE_SOD123_BASELINE_SOURCE: &str = "ceoloide/diode_tht_sod123";
const DIODE_SOD123_CASE_IDS: [&str; 43] = [
    "default",
    "placed-default",
    "flip:reversible",
    "flip:include_traces_vias",
    "flip:include_tht",
    "flip:include_thru_hole_smd_pads",
    "pose:front-F-0",
    "pose:back-B-0",
    "pose:front-F-90",
    "pose:back-B-90",
    "pose:front-F-37",
    "pose:back-B-37",
    "pose:front-B-0",
    "pose:back-F-0",
    "pose:front-unset-37",
    "pose:back-unset-37",
    "nets:assigned",
    "nets:reserved",
    "nets:empty",
    "merge:part-overrides-definition",
    "coerce:numeric-strings",
    "extra:smd-pads-reversible",
    "extra:hotswap-conflict-error",
    "demo-definition:sofle-v2:assembly-preset-mx-hotswap-south-left-keys-0/definition/diode",
    "demo-part:sofle-v2:matrix/left-keys/r0c0/diode",
    "demo-part:sofle-rgb:matrix/left-keys/r0c0/diode",
    "demo-part:sofle-choc:matrix/left-keys/r0c0/diode",
    "demo-part:measured-corne:matrix/left-keys/r0c0/diode",
    "demo-part:measured-lily58:matrix/left-keys/r0c0/diode",
    "demo-part:measured-sweep:matrix/left-keys/r0c0/diode",
    "demo-part:measured-chocofi:matrix/left-keys/r0c0/diode",
    "demo-part:measured-reviung41:matrix/main-right-keys/r0c0/diode",
    "demo-part:measured-totem:matrix/left-keys/r0c0/diode",
    "demo-part:measured-klor:matrix/left-keys/r0c0/diode",
    "demo-part:measured-cantor:matrix/left-keys/r0c0/diode",
    "demo-part:measured-gh60:matrix/main-keys/r0c0/diode",
    "demo-part:measured-discipline:matrix/main-keys/r0c0/diode",
    "demo-part:measured-mysterium:matrix/main-function/r0c0/diode",
    "demo-part:measured-voyager97:matrix/main-function/r0c0/diode",
    "demo-part:measured-voyager104:matrix/main-function/r0c0/diode",
    "demo-part:measured-plaid:matrix/main-keys/r0c0/diode",
    "demo-part:measured-lumberjack:matrix/main-left-keys/r0c0/diode",
    "demo-part:reviung41:matrix/main-right-keys/r0c0/diode",
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

#[derive(serde::Serialize)]
struct BaselineAllocatorSeed {
    reserved_nets: Vec<ReservedNet>,
    next_net_index: u32,
}

#[derive(serde::Serialize)]
struct StructuredBaselineError {
    variant: &'static str,
    fields: Value,
    message: String,
}

#[derive(serde::Serialize)]
struct DiodeSod123BaselineRecord {
    order: usize,
    id: String,
    source: String,
    qualification_flag_state: &'static str,
    raw_input: Value,
    effective_generator_ref: Value,
    effective_part_ref: Option<Value>,
    allocator_seed: Option<BaselineAllocatorSeed>,
    render_forms: Option<Vec<String>>,
    nets_after_allocator_snapshot: Option<Vec<ReservedNet>>,
    render_error: Option<StructuredBaselineError>,
    export_path_map: BTreeMap<String, String>,
    exported_footprints: Option<Vec<String>>,
    exported_objects: Option<Vec<String>>,
    export_error: Option<StructuredBaselineError>,
}

fn structured_baseline_error(error: &GeneratorError) -> StructuredBaselineError {
    let (variant, fields) = match error {
        GeneratorError::UnknownGenerator { source } => {
            ("UnknownGenerator", json!({"source": source}))
        }
        GeneratorError::UnsupportedVersion { version } => {
            ("UnsupportedVersion", json!({"version": version}))
        }
        GeneratorError::MissingGenerator => ("MissingGenerator", json!({})),
        GeneratorError::InvalidParameter {
            generator,
            parameter,
            expected,
        } => (
            "InvalidParameter",
            json!({"generator": generator, "parameter": parameter, "expected": expected}),
        ),
        GeneratorError::Rejected {
            generator,
            parameter,
            message,
        } => (
            "Rejected",
            json!({"generator": generator, "parameter": parameter, "message": message}),
        ),
        GeneratorError::Parse(message) => ("Parse", json!({"0": message})),
        GeneratorError::Geometry(message) => ("Geometry", json!({"0": message})),
        GeneratorError::Model(message) => ("Model", json!({"0": message})),
        GeneratorError::Export(message) => ("Export", json!({"0": message})),
        GeneratorError::Net(message) => ("Net", json!({"0": message})),
        GeneratorError::Internal(message) => ("Internal", json!({"0": message})),
    };
    StructuredBaselineError {
        variant,
        fields,
        message: error.message(),
    }
}

fn contains_qualification_flag(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields.contains_key("use_vishay_1n4148w_land_pattern")
                || fields.values().any(contains_qualification_flag)
        }
        Value::Array(items) => items.iter().any(contains_qualification_flag),
        _ => false,
    }
}

fn diode_sod123_baseline_records(fixture: &Value) -> Vec<u8> {
    assert_eq!(fixture["source"], DIODE_SOD123_BASELINE_SOURCE);
    let cases = fixture["cases"].as_array().expect("diode fixture cases");
    assert_eq!(cases.len(), DIODE_SOD123_CASE_IDS.len());
    let registry = bundled();
    let source = DIODE_SOD123_BASELINE_SOURCE;
    let run = Run {
        registry,
        source,
        fixture,
        failures: Vec::new(),
    };
    let mut records = Vec::with_capacity(cases.len() * 4096);
    for (index, case) in cases.iter().enumerate() {
        let id = case["id"].as_str().expect("diode case id");
        assert_eq!(
            id, DIODE_SOD123_CASE_IDS[index],
            "diode fixture order changed"
        );
        let input_value = case.get("input").expect("diode case input");
        assert!(
            !contains_qualification_flag(input_value),
            "phase-one baseline inputs must leave the opt-in flag absent: {id}"
        );
        let input = input_of(fixture, case);
        let allocator_seed =
            input
                .nets
                .as_ref()
                .map(|(reserved_nets, next_net_index)| BaselineAllocatorSeed {
                    reserved_nets: reserved_nets.clone(),
                    next_net_index: *next_net_index,
                });
        let (rendered, nets) = render_forms(&run, &input);
        let export_path_map = case["output"]["export"]["ok"]["modelPaths"]
            .as_object()
            .map(|_| {
                serde_json::from_value(case["output"]["export"]["ok"]["modelPaths"].clone())
                    .expect("fixture export path map")
            })
            .unwrap_or_default();
        let (render_forms, render_error, exported_footprints, exported_objects, export_error) =
            match rendered {
                Ok(forms) => {
                    let rendered_strings = forms.iter().map(serialize).collect();
                    match export_forms(forms, &export_path_map) {
                        Ok(exported) => (
                            Some(rendered_strings),
                            None,
                            Some(exported.footprints),
                            Some(exported.objects),
                            None,
                        ),
                        Err(error) => (
                            Some(rendered_strings),
                            None,
                            None,
                            None,
                            Some(structured_baseline_error(&error)),
                        ),
                    }
                }
                Err(error) => (
                    None,
                    Some(structured_baseline_error(&error)),
                    None,
                    None,
                    None,
                ),
            };
        let record = DiodeSod123BaselineRecord {
            order: index + 1,
            id: id.to_owned(),
            source: source.to_owned(),
            qualification_flag_state: "absent",
            raw_input: input_value.clone(),
            effective_generator_ref: serde_json::to_value(&input.generator)
                .expect("serialize adapted generator ref"),
            effective_part_ref: input
                .part
                .as_ref()
                .map(|part| serde_json::to_value(part).expect("serialize adapted part ref")),
            allocator_seed,
            render_forms,
            nets_after_allocator_snapshot: nets,
            render_error,
            export_path_map,
            exported_footprints,
            exported_objects,
            export_error,
        };
        serde_json::to_writer(&mut records, &record).expect("serialize baseline record");
        records.push(b'\n');
    }
    records
}

fn required_absolute_path(variable: &str) -> PathBuf {
    let value = std::env::var(variable)
        .unwrap_or_else(|error| panic!("{variable} must be set to an absolute path: {error}"));
    let path = PathBuf::from(value);
    assert!(path.is_absolute(), "{variable} must be an absolute path");
    path
}

struct TemporaryBaselinePath(PathBuf);

impl Drop for TemporaryBaselinePath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_new_file_atomically(path: &Path, bytes: &[u8]) {
    assert!(path.is_absolute(), "baseline output path must be absolute");
    let parent = path.parent().expect("baseline output parent");
    fs::create_dir_all(parent).expect("create baseline output directory");
    let filename = path.file_name().expect("baseline output filename");

    let mut temporary = None;
    for suffix in 0..100 {
        let candidate = parent.join(format!(
            ".{}.capture-{}-{suffix}",
            filename.to_string_lossy(),
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                let cleanup = TemporaryBaselinePath(candidate.clone());
                file.write_all(bytes)
                    .expect("write baseline temporary file");
                file.sync_all().expect("sync baseline temporary file");
                temporary = Some((candidate, cleanup));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create baseline temporary file: {error}"),
        }
    }
    let (temporary, cleanup) = temporary.expect("allocate unique baseline temporary file");
    fs::hard_link(&temporary, path)
        .unwrap_or_else(|error| panic!("exclusive atomic baseline creation failed: {error}"));
    drop(cleanup);
    OpenOptions::new()
        .read(true)
        .open(parent)
        .and_then(|directory| directory.sync_all())
        .expect("sync baseline output directory");
}

fn first_byte_difference(actual: &[u8], expected: &[u8]) -> usize {
    actual
        .iter()
        .zip(expected)
        .position(|(left, right)| left != right)
        .unwrap_or(actual.len().min(expected.len()))
}

#[test]
#[ignore = "manual pre-change baseline capture; run only in an authorized capture window"]
fn diode_sod123_baseline_capture() {
    let output = required_absolute_path("BOARDSTUDIO_DIODE_BASELINE_OUTPUT");
    let fixture = generator_fixtures()
        .into_iter()
        .find(|fixture| fixture["source"] == DIODE_SOD123_BASELINE_SOURCE)
        .expect("diode SOD-123 generator fixture");
    let bytes = diode_sod123_baseline_records(&fixture);
    write_new_file_atomically(&output, &bytes);
}

#[test]
#[ignore = "manual frozen-byte comparison; run only in an authorized comparison window"]
fn diode_sod123_baseline_matches_frozen_expected_bytes() {
    let expected = required_absolute_path("BOARDSTUDIO_DIODE_BASELINE_EXPECTED");
    let fixture = generator_fixtures()
        .into_iter()
        .find(|fixture| fixture["source"] == DIODE_SOD123_BASELINE_SOURCE)
        .expect("diode SOD-123 generator fixture");
    let expected_bytes = fs::read(&expected)
        .unwrap_or_else(|error| panic!("frozen expected baseline must already exist: {error}"));
    let actual_bytes = diode_sod123_baseline_records(&fixture);
    assert_eq!(
        first_byte_difference(&actual_bytes, &expected_bytes),
        actual_bytes.len().min(expected_bytes.len()),
        "baseline bytes differ at the first unequal byte or length"
    );
    assert_eq!(
        actual_bytes, expected_bytes,
        "the frozen legacy baseline must match byte-for-byte"
    );
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
    let order: Vec<&str> = actual
        .iter()
        .map(|p| p.name.as_str())
        .filter(|n| *n != "side")
        .collect();
    let wanted: Vec<&str> = expected
        .keys()
        .map(String::as_str)
        .filter(|n| *n != "side")
        .collect();
    if order != wanted {
        run.fail("schema", "order", format!("{order:?} vs {wanted:?}"));
    }
    for parameter in &actual {
        let mut value = serde_json::to_value(parameter).unwrap();
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

fn run_generator(registry: &'static Registry, fixture: &Value) -> (usize, Vec<String>) {
    let source = fixture["source"].as_str().unwrap().to_owned();
    let mut run = Run {
        registry,
        source: &source,
        fixture,
        failures: Vec::new(),
    };
    check_schema(&mut run);
    check_catalogue(&mut run);
    let cases = fixture["cases"].as_array().unwrap();
    for case in cases {
        check_case(&mut run, case);
    }
    (cases.len(), run.failures)
}

#[test]
fn bundled_generators_match_the_golden_baseline() {
    let registry = bundled();
    let mut checked = 0;
    let mut cases = 0;
    let mut failures = Vec::new();
    let mut pending = Vec::new();
    for fixture in generator_fixtures() {
        let source = fixture["source"].as_str().unwrap();
        if !registry.contains(source) {
            pending.push(source.to_owned());
            continue;
        }
        let (count, mut found) = run_generator(registry, &fixture);
        checked += 1;
        cases += count;
        failures.append(&mut found);
    }
    println!(
        "{checked} generators, {cases} cases checked; {} not ported yet: {pending:?}",
        pending.len()
    );
    if !failures.is_empty() {
        let shown: Vec<String> = failures.iter().take(12).cloned().collect();
        panic!(
            "{} mismatches (showing {}):\n{}",
            failures.len(),
            shown.len(),
            shown.join("\n")
        );
    }
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
