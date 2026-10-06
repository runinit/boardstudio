//! Native replacement for the retired Node/libcascade STEP acceptance tests.
//!
//! Inputs are built through the Core preparation driver and the production Cadrum code. Every
//! expected count, volume and bound is read from `cad/test/fixtures/step-expectations.json`
//! (original assertions with their original tolerances, plus the libcascade baseline), and every
//! export is re-read by the separate OCCT oracle in `cad/step-oracle`, never by Cadrum.

use super::super::step_oracle::{self as oracle, Report};
use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repository root")
        .to_path_buf()
}

fn fixture_path(relative: &str) -> PathBuf {
    root().join(relative)
}

fn core_request(request: &Value) -> Value {
    let driver = root().join("target/debug/examples/prepare_case");
    let output = Command::new(driver)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("driver stdin")
                .write_all(format!("{request}\n").as_bytes())?;
            child.wait_with_output()
        })
        .expect("run Core preparation driver");
    assert!(
        output.status.success(),
        "Core preparation driver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("Core driver JSON reply")
}

fn prepare(input: &Value, kind: &str) -> Value {
    let reply = core_request(&json!({ "id": "cad-native-validation", "kind": kind, "ir": input }));
    assert_eq!(reply["kind"], "case-prepared", "Core preparation reply");
    reply["ir"].clone()
}

fn raw_case(input: Value) -> CaseResultData {
    let assembly = json!({ "revision": input["revision"], "bodies": [input] });
    let prepared = prepare(&assembly, "prepare-case");
    let body: PreparedCase =
        serde_json::from_value(prepared["bodies"][0].clone()).expect("prepared case contract");
    build_case_data(body).expect("build prepared case")
}

/// Like `raw_case`, but reports a Core preparation or CAD build rejection instead of panicking.
fn try_raw_case(input: Value) -> Result<CaseResultData, String> {
    let assembly = json!({ "revision": input["revision"], "bodies": [input] });
    let reply = core_request(
        &json!({ "id": "cad-native-validation", "kind": "prepare-case", "ir": assembly }),
    );
    if reply["kind"] != "case-prepared" {
        return Err(reply["message"]
            .as_str()
            .unwrap_or("Core preparation failed")
            .to_string());
    }
    let body: PreparedCase =
        serde_json::from_value(reply["ir"]["bodies"][0].clone()).expect("prepared case contract");
    build_case_data(body)
}

fn raw_assembly(input: Value) -> CaseResultData {
    let prepared = prepare(&input, "prepare-case");
    let assembly: PreparedAssembly = serde_json::from_value(prepared).expect("prepared assembly");
    build_assembly_data(assembly).expect("build prepared assembly")
}

fn mechanical_document() -> Value {
    json!({
        "format":"boardstudio/v2", "id":"mechanical", "name":"Mechanical", "revision":21,
        "parameters":{}, "definitions":[], "parts":[], "matrices":[], "constraints":[], "nets":[],
        "outline":[], "caseBodies":[], "assets":[], "materials":[], "scripts":[],
        "boards":[{"id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6,"traces":[],"vias":[]}],
        "mechanical":{
            "boardId":"board", "method":"printed", "mount":"rigid", "plateThickness":1.5,
            "plateFoamThickness":1, "pcbThickness":1.6, "bottomFoamThickness":0.5, "batteryHeight":3,
            "bottomThickness":2, "plateToPcb":3.5, "wallThickness":2, "clearance":0.2, "profiles":[],
            "mounts":[{"id":"hole","kind":"hole","at":{"x":5,"y":5},"holeDiameter":2,"bossDiameter":5}]
        }
    })
}

fn resolve_mechanical(document: &Value, contours: &Value) -> Value {
    let reply = core_request(&json!({
        "id":"mechanical-cad-native-validation", "kind":"resolve-mechanical",
        "document":document, "contours":contours
    }));
    assert_eq!(
        reply["kind"], "mechanical-resolved",
        "Core mechanical resolver reply"
    );
    reply["assembly"].clone()
}

fn resolve_modules(document: &Value, board_id: &str) -> Value {
    let reply = core_request(&json!({
        "id":"module-cad-native-validation", "kind":"resolve-modules",
        "document":document, "boardId":board_id, "previewTopZ":0
    }));
    assert_eq!(
        reply["kind"], "modules-resolved",
        "Core module resolver reply"
    );
    reply["result"].clone()
}

fn polygon_area(points: &Value) -> f64 {
    let points = points.as_array().expect("polygon points");
    let area = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = &points[(index + 1) % points.len()];
            point["x"].as_f64().unwrap() * next["y"].as_f64().unwrap()
                - next["x"].as_f64().unwrap() * point["y"].as_f64().unwrap()
        })
        .sum::<f64>();
    area.abs() / 2.0
}

fn square(min: f64, max: f64) -> Value {
    json!([
        {"x": min, "y": min}, {"x": max, "y": min},
        {"x": max, "y": max}, {"x": min, "y": max}
    ])
}

fn expectations() -> &'static Value {
    static FILE: OnceLock<Value> = OnceLock::new();
    FILE.get_or_init(|| {
        serde_json::from_slice(
            &fs::read(fixture_path("cad/test/fixtures/step-expectations.json")).unwrap(),
        )
        .expect("expectation fixture")
    })
}

fn entry(id: &str) -> &'static Value {
    expectations()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("no expectation entry {id}"))
}

fn expect(id: &str) -> &'static Value {
    &entry(id)["expect"]
}

fn baseline(id: &str) -> &'static Value {
    &entry(id)["baseline"]
}

fn num(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("{value} is not a number"))
}

/// True when `actual` is within the tolerance carried by a `{value, tolerance}` expectation.
fn amount_matches(actual: f64, amount: &Value) -> bool {
    (actual - num(&amount["value"])).abs() <= num(&amount["tolerance"])
}

fn check_amount(actual: f64, amount: &Value, label: &str) {
    assert!(
        amount_matches(actual, amount),
        "{label}: expected {} ± {}, got {actual} ({})",
        amount["value"],
        amount["tolerance"],
        amount["basis"]
    );
}

fn check_bounds(min: [f64; 3], max: [f64; 3], bounds: &Value, label: &str) {
    let tolerance = num(&bounds["tolerance"]);
    for axis in 0..3 {
        assert_close(
            min[axis],
            num(&bounds["min"][axis]),
            tolerance,
            &format!("{label} min axis {axis}"),
        );
        assert_close(
            max[axis],
            num(&bounds["max"][axis]),
            tolerance,
            &format!("{label} max axis {axis}"),
        );
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected}, got {actual} (tolerance {tolerance})"
    );
}

/// The oracle's report must agree with the libcascade baseline: exact solid count, volume 0.1 mm³,
/// bounds 0.01 mm (the baseline suite's own tolerances).
fn check_baseline(report: &Report, baseline: &Value, label: &str) {
    assert_eq!(
        report.solids as u64,
        baseline["solids"].as_u64().unwrap(),
        "{label}: baseline solid count"
    );
    assert_close(
        report.volume,
        num(&baseline["volume"]),
        0.1,
        &format!("{label}: baseline volume"),
    );
    for axis in 0..3 {
        assert_close(
            report.min[axis],
            num(&baseline["min"][axis]),
            0.01,
            &format!("{label}: baseline min {axis}"),
        );
        assert_close(
            report.max[axis],
            num(&baseline["max"][axis]),
            0.01,
            &format!("{label}: baseline max {axis}"),
        );
    }
}

/// Re-reads a generated export with the independent oracle. Generated geometry must be a valid
/// BRep with no faces outside a solid (a face lost from a shell shows up as an orphan).
fn export(step: &[u8], label: &str) -> Report {
    if let Some(dir) = std::env::var_os("STEP_DUMP_DIR") {
        let name = format!(
            "{}-{:.12}.step",
            label.replace([' ', '/'], "_"),
            format!("{:x}", Sha256::digest(step))
        );
        fs::write(PathBuf::from(dir).join(name), step).expect("dump STEP");
    }
    let report = oracle::inspect(step);
    assert_eq!(
        report.read_status,
        oracle::READ_DONE,
        "{label}: oracle read: {}",
        report.error
    );
    assert!(report.valid, "{label}: exported STEP is a valid BRep");
    assert_eq!(report.orphan_faces, 0, "{label}: no faces outside a solid");
    report
}

fn load_manifest(name: &str) -> Value {
    let bytes = fs::read(fixture_path(&format!("cad/test/fixtures/{name}")))
        .expect("read CAD fixture manifest");
    serde_json::from_slice(&bytes).expect("valid fixture manifest")
}

fn body_named<'a>(assembly: &'a Value, id: &str) -> &'a Value {
    assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == id)
        .unwrap_or_else(|| panic!("no body {id}"))
}

fn contour<'a>(contours: &'a Value, hole: bool) -> &'a Value {
    contours
        .as_array()
        .unwrap()
        .iter()
        .find(|contour| contour["hole"] == hole)
        .unwrap()
}

fn import_checks(step: Vec<u8>, label: &str) -> StepModelData {
    let imported =
        read_step_model_data(step).unwrap_or_else(|error| panic!("{label} Cadrum import: {error}"));
    assert!(
        !imported.mesh.positions.is_empty(),
        "{label}: imported mesh"
    );
    assert_eq!(
        imported.mesh.positions.len(),
        imported.mesh.normals.len(),
        "{label}: positions and normals"
    );
    imported
}

#[test]
fn manifests_retain_fixture_hashes_and_independent_step_reports() {
    let baselines = expectations()["manifestBaselines"].as_object().unwrap();
    let mut checked = 0;
    for manifest_name in ["manifest.json", "followup-manifest.json"] {
        let manifest = load_manifest(manifest_name);
        assert_eq!(manifest["units"], "millimetres");
        for fixture in manifest["fixtures"].as_array().expect("fixtures array") {
            let id = fixture["id"].as_str().expect("fixture id");
            let file = fixture["file"].as_str().expect("fixture file");
            let is_step = fixture["kind"] == "step";
            let path = if is_step {
                fixture_path(file)
            } else {
                fixture_path(&format!("cad/test/fixtures/{file}"))
            };
            let source = fs::read(&path).unwrap_or_else(|error| panic!("{id}: {error}"));
            assert_eq!(
                format!("{:x}", Sha256::digest(&source)),
                fixture["sha256"].as_str().unwrap(),
                "{id} input SHA-256"
            );

            let step = if is_step {
                source
            } else {
                let request: Value = serde_json::from_slice(&source).expect("fixture JSON");
                let prepared = prepare(&request, "prepare-case");
                if let Some(expected) = fixture["expectedCavities"].as_u64() {
                    let actual = prepared["bodies"][0]["regions"][0]["cavities"]
                        .as_array()
                        .expect("prepared cavities")
                        .len();
                    assert_eq!(actual as u64, expected, "{id} prepared cavity count");
                }
                let assembly: PreparedAssembly =
                    serde_json::from_value(prepared).expect("prepared assembly contract");
                build_assembly_data(assembly)
                    .unwrap_or_else(|error| panic!("{id} CAD build: {error}"))
                    .step
            };

            // Only BoardStudio's own exports must be valid BReps; vendor models are third-party.
            let report = if is_step {
                oracle::inspect(&step)
            } else {
                export(&step, &format!("fixture-{id}"))
            };
            assert_eq!(report.read_status, oracle::READ_DONE, "{id}: oracle read");
            assert_eq!(
                report.solids as u64,
                fixture["expectedSolids"].as_u64().unwrap(),
                "{id} solid count"
            );
            assert!(report.volume > 0.0, "{id} positive STEP volume");
            if let Some(volume) = fixture["expectedVolume"].as_f64() {
                assert_close(report.volume, volume, 0.1, &format!("{id} volume"));
            }
            let expected = &fixture["expectedBounds"];
            for axis in 0..3 {
                assert_close(
                    report.min[axis],
                    num(&expected[0][axis]),
                    0.01,
                    &format!("{id} oracle min {axis}"),
                );
                assert_close(
                    report.max[axis],
                    num(&expected[1][axis]),
                    0.01,
                    &format!("{id} oracle max {axis}"),
                );
            }
            check_baseline(&report, &baselines[id], id);

            let imported = import_checks(step, id);
            for axis in 0..3 {
                assert_close(
                    imported.min[axis],
                    num(&expected[0][axis]),
                    0.01,
                    &format!("{id} Cadrum min {axis}"),
                );
                assert_close(
                    imported.max[axis],
                    num(&expected[1][axis]),
                    0.01,
                    &format!("{id} Cadrum max {axis}"),
                );
            }
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        baselines.len(),
        "every manifest fixture was checked"
    );
}

#[test]
fn transformed_multi_solid_vendor_step_keeps_count_bounds_and_mesh() {
    let id = "trackpoint-vendor-multi-solid";
    let want = expect(id);
    let bytes = fs::read(fixture_path("ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step")).unwrap();
    let report = oracle::inspect(&bytes);
    assert_eq!(report.read_status, oracle::READ_DONE);
    assert_eq!(
        report.solids as u64,
        want["solids"].as_u64().unwrap(),
        "independent solid count"
    );
    assert_eq!(report.shells, report.solids, "one shell per solid");
    check_bounds(report.min, report.max, &want["bounds"], "oracle bounds");
    // Informational baseline volume (no original assertion): allow 0.5 mm³.
    assert_close(
        report.volume,
        num(&baseline(id)["volume"]),
        0.5,
        "baseline volume",
    );

    let imported = import_checks(bytes, id);
    check_bounds(imported.min, imported.max, &want["bounds"], "Cadrum bounds");
    assert!(imported.mesh.positions.len() as u64 > want["minMeshPositionFloats"].as_u64().unwrap());
    assert!(
        imported.mesh.positions.iter().all(|v| v.is_finite()),
        "finite positions"
    );
    let (mesh_min, mesh_max) = {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for point in imported.mesh.positions.chunks_exact(3) {
            for axis in 0..3 {
                min[axis] = min[axis].min(f64::from(point[axis]));
                max[axis] = max[axis].max(f64::from(point[axis]));
            }
        }
        (min, max)
    };
    let contain = num(&want["meshBoundsWithinModelBounds"]);
    for axis in 0..3 {
        assert_close(
            mesh_min[axis],
            imported.min[axis],
            contain,
            &format!("mesh min {axis}"),
        );
        assert_close(
            mesh_max[axis],
            imported.max[axis],
            contain,
            &format!("mesh max {axis}"),
        );
    }
    let unit = num(&want["normalUnitLengthTolerance"]);
    for (index, normal) in imported.mesh.normals.chunks_exact(3).enumerate() {
        let length = f64::from(normal[0])
            .hypot(f64::from(normal[1]))
            .hypot(f64::from(normal[2]));
        assert!(
            length.is_finite() && (length - 1.0).abs() < unit,
            "normal {index} has unit length"
        );
    }
}

#[test]
fn generated_exports_keep_analytic_volume_and_step_bounds() {
    let contours = json!([
        { "hole": false, "points": square(0.0, 20.0) },
        { "hole": true, "points": square(5.0, 15.0) }
    ]);
    let plate = raw_case(json!({
        "revision": 7,
        "body": { "id": "case", "name": "plate", "boardId": "board", "kind": "plate", "thickness": 2, "clearance": 0.5 },
        "contours": contours
    }));
    let id = "holed-plate-export";
    assert_eq!(plate.revision, expect(id)["revision"].as_u64().unwrap());
    assert!(!plate.step.is_empty());
    assert!(!plate.mesh.positions.is_empty());
    assert_eq!(plate.mesh.positions.len(), plate.mesh.normals.len());
    let report = export(&plate.step, id);
    assert_eq!(report.solids, 1);
    check_amount(report.volume, &expect(id)["volume"], "holed plate volume");
    check_baseline(&report, baseline(id), id);

    let simple = raw_case(json!({
        "revision": 1,
        "body": { "id": "case", "name": "plate", "boardId": "board", "kind": "plate", "thickness": 2, "clearance": 0 },
        "contours": [{ "hole": false, "points": square(0.0, 20.0) }]
    }));
    let want = expect("plate-generated-20x20x2");
    let imported = import_checks(simple.step, "plate-generated");
    check_bounds(
        imported.min,
        imported.max,
        &want["cadrumBounds"],
        "generated plate",
    );
    let rejection = want["malformedBytesRejectedWith"].as_str().unwrap();
    let malformed = read_step_model_data(vec![1, 2, 3])
        .err()
        .expect("malformed STEP must be rejected");
    assert!(
        malformed.contains(rejection),
        "malformed STEP rejection: {malformed}"
    );
}

#[test]
fn step_import_rejects_empty_oversize_and_garbage_input() {
    for (label, bytes) in [
        ("empty", Vec::new()),
        ("oversize", vec![0u8; MAX_STEP_BYTES + 1]),
    ] {
        let error = read_step_model_data(bytes)
            .err()
            .unwrap_or_else(|| panic!("{label} input accepted"));
        assert!(error.contains("invalid file size"), "{label}: {error}");
    }
    let error = read_step_model_data(b"ISO-10303-21; this is not a STEP file".to_vec())
        .err()
        .expect("garbage accepted");
    assert!(error.contains("STEP import failed"), "garbage: {error}");
}

#[test]
fn module_boss_and_standoff_exports_preserve_annuli_and_drills() {
    let hole = json!({ "id": "module:module-1:mh1", "at": { "x": 0, "y": 0 }, "kind": "hole", "holeDiameter": 2.2 });
    let boss = json!({ "id": "module:module-1:mh1", "at": { "x": 0, "y": 0 }, "kind": "boss", "holeDiameter": 2.2, "bossDiameter": 6, "height": 1 });
    let body = json!({ "id": "bottom", "name": "Module support fixture", "boardId": "board", "kind": "plate", "thickness": 2, "clearance": 0, "z": -9.2 });
    let contours = json!([{ "hole": false, "points": square(-40.0, 40.0) }]);
    let with_mount = |mount: &Value| {
        let mut body = body.clone();
        body["mounts"] = json!([mount]);
        raw_case(json!({ "revision": 1, "body": body, "contours": contours }))
    };
    let id = "module-support-annular-boss";
    let want = expect(id);
    let plain_report = export(&with_mount(&hole).step, "module-plain");
    let boss_report = export(&with_mount(&boss).step, "module-boss");
    check_bounds(
        boss_report.min,
        boss_report.max,
        &want["bounds"],
        "boss bounds",
    );
    check_amount(
        boss_report.volume - plain_report.volume,
        &want["volumeDeltaVersusPlain"],
        "annular support volume delta",
    );
    check_baseline(&plain_report, &baseline(id)["plain"], "plain plate");
    check_baseline(&boss_report, &baseline(id)["boss"], "boss plate");

    let id = "module-standoff-annulus";
    let want = expect(id);
    let (outer, inner) = (3.0_f64, 1.4_f64);
    let circle = |radius: f64| -> Vec<Value> {
        (0..96)
            .map(|index| {
                let angle = index as f64 / 96.0 * std::f64::consts::TAU;
                json!({ "x": radius * angle.cos(), "y": radius * angle.sin() })
            })
            .collect()
    };
    let standoff = raw_case(json!({
        "revision": 1,
        "body": { "id": "module-standoff/module-1/mh1", "name": "PCB standoff", "boardId": "host", "kind": "plate", "thickness": 3, "clearance": 0 },
        "contours": [ { "hole": false, "points": circle(outer) }, { "hole": true, "points": circle(inner) } ]
    }));
    let report = export(&standoff.step, id);
    assert_eq!(report.solids as u64, want["solids"].as_u64().unwrap());
    check_bounds(report.min, report.max, &want["bounds"], "standoff bounds");
    check_amount(report.volume, &want["volume"], "annular standoff volume");
    check_baseline(&report, baseline(id), id);
}

#[test]
fn concave_clearance_tray_lid_and_cavity_exports_retain_material() {
    let notch = json!([
        {"x":0,"y":0},{"x":30,"y":0},{"x":30,"y":30},{"x":18,"y":30},
        {"x":18,"y":5},{"x":12,"y":5},{"x":12,"y":30},{"x":0,"y":30}
    ]);
    let cleared = raw_case(json!({
        "revision": 8,
        "body": {"id":"case","name":"plate","boardId":"board","kind":"plate","thickness":2,"clearance":4},
        "contours": [{"hole":false,"points":notch}]
    }));
    let id = "concave-notch-clearance";
    let report = export(&cleared.step, id);
    check_amount(
        report.volume,
        &expect(id)["volume"],
        "concave clearance volume",
    );
    check_baseline(&report, baseline(id), id);

    let concave = json!([
        {"x":0,"y":0},{"x":40,"y":0},{"x":40,"y":30},{"x":26,"y":30},
        {"x":26,"y":7},{"x":14,"y":7},{"x":14,"y":30},{"x":0,"y":30}
    ]);
    let id = "concave-tray-and-lid-inset";
    let range = &expect(id)["volumeExclusiveRange"];
    for kind in ["tray", "lid"] {
        let result = raw_case(json!({
            "revision": 9,
            "body": {"id":kind,"name":kind,"boardId":"board","kind":kind,"thickness":2,"clearance":1,"wallHeight":5,"wallThickness":2},
            "contours": [{"hole":false,"points":concave}]
        }));
        let report = export(&result.step, &format!("{id}-{kind}"));
        assert!(
            report.volume > num(&range[0]) && report.volume < num(&range[1]),
            "concave {kind} volume {}",
            report.volume
        );
        check_baseline(&report, &baseline(id)[kind], kind);
    }

    let contour = json!([{ "hole": false, "points": square(0.0, 40.0) }]);
    let plate_body = json!({ "id":"body", "name":"body", "boardId":"board", "kind":"plate", "thickness":2, "clearance":0, "z":3 });
    let tray_body = json!({ "id":"body", "name":"body", "boardId":"board", "kind":"tray", "thickness":2, "clearance":0, "z":3, "wallHeight":6, "wallThickness":3,
        "mounts":[{"id":"boss","kind":"boss","at":{"x":10,"y":10},"holeDiameter":2,"bossDiameter":5,"height":3}],
        "gasket":{"inset":0.5,"width":1,"depth":0.5} });
    let lid_body = json!({ "id":"body", "name":"body", "boardId":"board", "kind":"lid", "thickness":2, "clearance":0, "z":3, "wallHeight":6, "wallThickness":3 });
    let id = "tray-lid-cavity-mount";
    let want = expect(id);
    let build = |revision: u64, body: &Value, label: &str| {
        export(
            &raw_case(json!({ "revision": revision, "body": body, "contours": contour })).step,
            &format!("{id}-{label}"),
        )
    };
    let plate = build(1, &plate_body, "plate");
    let tray = build(2, &tray_body, "tray");
    let lid = build(3, &lid_body, "lid");
    check_amount(
        plate.volume,
        &want["plateVolume"],
        "cavity fixture plate volume",
    );
    assert!(
        tray.volume > plate.volume,
        "tray adds material over the plate"
    );
    assert!(
        tray.volume < num(&want["trayVolumeBelow"]),
        "tray stays inside its bounding prism"
    );
    check_amount(lid.volume, &want["lidVolume"], "cavity fixture lid volume");
    let z_tolerance = num(&want["zTolerance"]);
    for (report, key, label) in [(&tray, "trayZ", "tray"), (&lid, "lidZ", "lid")] {
        assert_close(
            report.min[2],
            num(&want[key][0]),
            z_tolerance,
            &format!("{label} min Z"),
        );
        assert_close(
            report.max[2],
            num(&want[key][1]),
            z_tolerance,
            &format!("{label} max Z"),
        );
    }
    check_baseline(&plate, &baseline(id)["plate"], "plate");
    check_baseline(&tray, &baseline(id)["tray"], "tray");
    check_baseline(&lid, &baseline(id)["lid"], "lid");
}

#[test]
fn vertical_compound_corner_cutout_and_additive_support_exports_are_exact() {
    let contours = json!([{ "hole": false, "points": square(0.0, 20.0) }]);
    let bodies = [0.0, 8.0].into_iter().enumerate().map(|(index,z)| json!({
            "revision":9,
            "body":{"id":format!("plate-{index}"),"name":format!("plate-{index}"),"boardId":"board","kind":"plate","thickness":2,"clearance":0,"z":z},
            "contours":contours
        })).collect::<Vec<_>>();
    let compound = raw_assembly(json!({ "revision":9, "bodies":bodies }));
    let id = "compound-two-offset-plates";
    let want = expect(id);
    let report = export(&compound.step, id);
    assert_eq!(compound.revision, want["revision"].as_u64().unwrap());
    check_amount(report.volume, &want["volume"], "two-plate compound volume");
    let z_tolerance = num(&want["zTolerance"]);
    assert_close(
        report.min[2],
        num(&want["zRange"][0]),
        z_tolerance,
        "compound min Z",
    );
    assert_close(
        report.max[2],
        num(&want["zRange"][1]),
        z_tolerance,
        "compound max Z",
    );
    assert!(!compound.mesh.positions.is_empty());
    assert_eq!(compound.mesh.positions.len(), compound.mesh.normals.len());
    check_baseline(&report, baseline(id), id);

    let cut_contours = json!([
        {"hole":false,"points":[{"x":10,"y":0},{"x":30,"y":0},{"x":30,"y":30},{"x":0,"y":30},{"x":0,"y":10},{"x":10,"y":10}]},
        {"hole":true,"points":square(15.0,20.0)}
    ]);
    let cut = raw_case(
        json!({"revision":12,"body":{"id":"plate","name":"plate","boardId":"board","kind":"plate","thickness":2,"clearance":0},"contours":cut_contours}),
    );
    let id = "deleted-corner-and-cutout";
    let report = export(&cut.step, id);
    assert_eq!(cut.revision, expect(id)["revision"].as_u64().unwrap());
    check_amount(report.volume, &expect(id)["volume"], "corner/cutout volume");
    check_baseline(&report, baseline(id), id);

    let supported = raw_case(json!({
        "revision":1,
        "body":{"id":"supported","name":"Supported plate","boardId":"board","kind":"plate","thickness":2,"clearance":0,"mounts":[{"id":"drill","kind":"hole","at":{"x":10,"y":10},"holeDiameter":4}],"features":[{"id":"tower","kind":"support-prism","points":square(7.0,13.0),"z":1,"height":2}]},
        "contours":contours
    }));
    let id = "additive-support-refills-drill";
    let want = expect(id);
    let report = export(&supported.step, id);
    check_bounds(
        report.min,
        report.max,
        &want["bounds"],
        "additive support bounds",
    );
    assert_eq!(report.solids as u64, want["solids"].as_u64().unwrap());
    check_amount(report.volume, &want["volume"], "additive support volume");
    check_baseline(&report, baseline(id), id);
}

#[test]
fn core_mechanical_plate_and_battery_stack_roundtrip_with_nominal_dimensions() {
    let id = "mechanical-plate-battery-stack";
    let want = expect(id);
    let contours = json!([
        {"hole":false,"points":square(0.0,40.0)},
        {"hole":true,"points":square(12.0,26.0)}
    ]);
    let assembly = resolve_mechanical(&mechanical_document(), &contours);
    assert_eq!(assembly["nominalPlateContours"], assembly["plateContours"]);
    let holes = |contours: &Value| -> Vec<Value> {
        contours
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["hole"] == true)
            .cloned()
            .collect()
    };
    assert_eq!(
        holes(&assembly["nominalPlateContours"]),
        holes(&assembly["plateContours"])
    );
    let outer = contour(&assembly["plateContours"], false);
    let min_x = outer["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| num(&p["x"]))
        .fold(f64::INFINITY, f64::min);
    assert_close(
        min_x,
        num(&want["outerMinX"]),
        num(&want["outerMinXTolerance"]),
        "mechanical plate clearance min X",
    );
    let plate = body_named(&assembly, "plate").clone();
    assert_eq!(plate["contours"], assembly["plateContours"]);
    let actual_hole = contour(&assembly["plateContours"], true);
    let report = export(&raw_case(plate).step, "mechanical-plate");
    // (polygonArea(outer plate contour) - 196 - pi) * 1.5, derived from the resolver's own contour.
    let hole_area = polygon_area(&actual_hole["points"]);
    assert_close(hole_area, 196.0, 1e-6, "nominal hole area");
    assert_close(
        report.volume,
        (polygon_area(&outer["points"]) - hole_area - std::f64::consts::PI) * 1.5,
        num(&want["plateVolumeTolerance"]),
        "resolved plate STEP volume",
    );
    check_baseline(&report, baseline(id), id);

    let full = raw_assembly(assembly["case"].clone());
    let imported = import_checks(full.step.clone(), id);
    let bottom_z = num(&assembly["stack"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layer| layer["id"] == "bottom")
        .unwrap()["z"]);
    let z_tolerance = num(&want["zTolerance"]);
    assert_close(imported.min[2], bottom_z, z_tolerance, "stack bottom z");
    assert_close(
        imported.max[2],
        num(&want["stackMaxZ"]),
        z_tolerance,
        "stack top z",
    );
    assert_eq!(full.revision, want["revision"].as_u64().unwrap());
    assert!(full.bodies.as_ref().unwrap().len() as u64 >= want["minBodies"].as_u64().unwrap());
    export(&full.step, "mechanical-stack");
}

#[test]
fn integrated_lid_and_component_opening_have_exact_exported_subtraction() {
    let id = "integrated-lid-frame-side-access";
    let want = expect(id);
    let contours = json!([{ "hole":false, "points":square(0.0,40.0) }]);
    let body = json!({"id":"plate-frame","name":"plate-frame","boardId":"board","kind":"lid","thickness":1.5,"clearance":0,"z":-5,"wallHeight":8.5,"wallThickness":2});
    let plain = raw_case(json!({"revision":30,"body":body,"contours":contours}));
    let plain_report = export(&plain.step, &format!("{id}-plain"));
    check_amount(
        plain_report.volume,
        &want["plainVolume"],
        "integrated frame volume",
    );
    let z_tolerance = num(&want["zTolerance"]);
    assert_close(
        plain_report.min[2],
        num(&want["zRange"][0]),
        z_tolerance,
        "integrated frame min Z",
    );
    assert_close(
        plain_report.max[2],
        num(&want["zRange"][1]),
        z_tolerance,
        "integrated frame max Z",
    );
    check_baseline(&plain_report, &baseline(id)["plain"], "plain frame");

    let mut with_opening = body.clone();
    with_opening["openings"] = json!([{"points":[{"x":12,"y":-1},{"x":22,"y":-1},{"x":22,"y":3},{"x":12,"y":3}],"z":-2,"height":3}]);
    let cut = raw_case(json!({"revision":30, "body":with_opening, "contours":contours}));
    let cut_report = export(&cut.step, &format!("{id}-cut"));
    check_amount(
        plain_report.volume - cut_report.volume,
        &want["openingVolumeRemoved"],
        "side-access subtraction",
    );
    check_baseline(&cut_report, &baseline(id)["cut"], "cut frame");
    assert!(!import_checks(cut.step.clone(), id)
        .mesh
        .positions
        .is_empty());

    let mut invalid = body.clone();
    invalid["openings"] = json!([{"points":square(0.0,2.0),"z":0,"height":-1}]);
    let message = try_raw_case(json!({"revision":30, "body":invalid, "contours":contours}))
        .err()
        .expect("negative-height opening must be rejected");
    let needle = want["negativeHeightOpeningRejectedWith"].as_str().unwrap();
    assert!(
        message.contains(needle),
        "rejection names the opening: {message}"
    );
}

#[test]
fn core_internal_gasket_fixtures_export_connected_positive_regions() {
    for name in [
        "rectangle",
        "countersunk",
        "downward-boss",
        "rotated-concave",
        "split",
        "mixed-cuts",
        "automatic-count",
        "sofle-outline",
    ] {
        let id = format!("internal-gasket-{name}");
        let want = expect(&id);
        let fixture_name = if matches!(name, "mixed-cuts" | "automatic-count") {
            "rectangle"
        } else {
            name
        };
        let bytes = fs::read(fixture_path(&format!(
            "cad/test/fixtures/internal-gasket-v1/{fixture_name}.json"
        )))
        .unwrap();
        let mut input: Value = serde_json::from_slice(&bytes).unwrap();
        if matches!(name, "mixed-cuts" | "automatic-count") {
            input["document"]["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
        }
        if name == "automatic-count" {
            input["document"]["mechanical"]["internalGasket"]["autoCount"] = json!(true);
        }
        let assembly = resolve_mechanical(&input["document"], &input["contours"]);
        assert_eq!(
            assembly["generationBlocked"],
            want["resolverNotBlocked"].as_bool().map(|ok| !ok).unwrap(),
            "{name} resolver diagnostics"
        );
        let supports = assembly["gasketSupports"].as_array().unwrap();
        if let Some(floor) = want.get("gasketSupportsGreaterThan") {
            assert!(
                supports.len() as u64 > floor.as_u64().unwrap(),
                "{name} automatic support count {}",
                supports.len()
            );
        }
        if let Some(floor) = want.get("distinctSupportLengthsGreaterThan") {
            let lengths: std::collections::BTreeSet<_> = supports
                .iter()
                .map(|s| num(&s["length"]).to_bits())
                .collect();
            assert!(
                lengths.len() as u64 > floor.as_u64().unwrap(),
                "{name} uses different support lengths"
            );
        }
        let expected_solids = want["solidsPerBodyEqualsInputContours"].as_u64().unwrap();
        assert_eq!(
            input["contours"].as_array().unwrap().len() as u64,
            expected_solids,
            "{name} fixture region count"
        );
        for (index, body_id) in want["bodies"].as_array().unwrap().iter().enumerate() {
            let body_id = body_id.as_str().unwrap();
            let result = raw_case(body_named(&assembly, body_id).clone());
            let report = export(&result.step, &format!("{id}-{body_id}"));
            assert_eq!(
                report.solids as u64, expected_solids,
                "{name}/{body_id} connected regions"
            );
            assert!(report.volume > 0.0, "{name}/{body_id} positive volume");
            assert!(
                result.mesh.positions.iter().all(|value| value.is_finite()),
                "{name}/{body_id} finite mesh"
            );
            check_baseline(
                &report,
                &baseline(&id)[if index == 0 { "bottom" } else { "retainer" }],
                &format!("{name}/{body_id}"),
            );
        }
    }
}

#[test]
fn component_local_connector_profile_cuts_the_expected_side_wall_volume() {
    let id = "component-local-connector-opening";
    let want = expect(id);
    let contours = json!([{ "hole":false, "points":square(0.0,40.0) }]);
    let mut document = mechanical_document();
    let original = resolve_mechanical(&document, &contours);
    let original_bottom = body_named(&original, "bottom").clone();
    document["parts"] = json!([{"id":"connector","definitionId":"connector","reference":"J1","side":"front","pose":{"at":{"x":0,"y":20},"rotation":90}}]);
    document["boards"][0]["partIds"] = json!(["connector"]);
    document["mechanical"]["profiles"] = json!([{
        "definitionId":"connector", "source":"Explicit connector datasheet fixture", "cutouts":[], "plateToPcb":3.5,
        "openings":[{"points":[{"x":-5,"y":-1},{"x":5,"y":-1},{"x":5,"y":4},{"x":-5,"y":4}],"z":-2,"height":3}]
    }]);
    let resolved = resolve_mechanical(&document, &contours);
    let bottom = body_named(&resolved, "bottom").clone();
    assert_eq!(bottom["body"]["openings"].as_array().unwrap().len(), 1);
    let tolerance = num(&want["pointTolerance"]);
    let point = &bottom["body"]["openings"][0]["points"][0];
    assert_close(
        num(&point["x"]),
        num(&want["transformedOpeningFirstPoint"][0]),
        tolerance,
        "connector opening x",
    );
    assert_close(
        num(&point["y"]),
        num(&want["transformedOpeningFirstPoint"][1]),
        tolerance,
        "connector opening y",
    );
    let before = export(&raw_case(original_bottom).step, &format!("{id}-before"));
    let after = export(&raw_case(bottom).step, &format!("{id}-after"));
    check_amount(
        before.volume - after.volume,
        &want["openingVolumeRemoved"],
        "component-local wall opening volume",
    );
    check_baseline(&before, &baseline(id)["before"], "before");
    check_baseline(&after, &baseline(id)["after"], "after");
}

#[test]
fn source_module_pcb_keeps_front_and_back_placement_in_exported_step() {
    let catalogue: Value = serde_json::from_slice(
        &fs::read(fixture_path("catalogue/modules/imported-modules.json")).unwrap(),
    )
    .unwrap();
    let definition = catalogue["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["row"] == "vik-splitter")
        .unwrap()["definition"]
        .clone();
    for (host_face, facing_face) in [("front", "back"), ("back", "front")] {
        let id = format!("source-module-pcb-{host_face}");
        let want = expect(&id);
        let mut document = mechanical_document();
        document["moduleDefinitions"] = json!([definition]);
        document["modules"] = json!([{
            "id":"splitter", "definitionId":definition["id"], "hostBoardId":"board",
            "hostFace":host_face, "facingFace":facing_face, "at":{"x":15,"y":7},
            "rotation":0, "gap":3, "attachment":"board", "detached":false, "serviceClearance":0
        }]);
        let resolved = resolve_modules(&document, "board");
        assert!(
            resolved["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["scope"] == "case" && finding["severity"] == "error"),
            "unknown {host_face}/{facing_face} assembly dimensions retain qualification gates"
        );
        // The module preview is already a prepared assembly (the Node test passed it straight to buildAssembly).
        let prepared: PreparedAssembly =
            serde_json::from_value(resolved["preview"].clone()).expect("prepared module preview");
        let built = build_assembly_data(prepared).expect("build module preview");
        let report = export(&built.step, &id);
        check_bounds(report.min, report.max, &want["bounds"], &id);
        assert_eq!(report.solids as u64, want["solids"].as_u64().unwrap());
        check_amount(report.volume, &want["volume"], "drilled source PCB volume");
        check_baseline(&report, baseline(&id), &id);
    }
}

#[test]
fn resolved_allowance_frames_and_gasket_contacts_survive_step_export() {
    let id = "allowance-frame-gasket-assembly";
    let want = expect(id);
    let z_tolerance = num(&want["zTolerance"]);
    let contours = json!([
        {"hole":false,"points":square(0.0,40.0)},
        {"hole":true,"points":square(12.0,26.0)}
    ]);
    let mut document = mechanical_document();
    document["mechanical"]["openingAllowance"] = json!(0.2);
    let adjusted = resolve_mechanical(&document, &contours);
    let nominal_opening = contour(&adjusted["nominalPlateContours"], true);
    let actual_opening = contour(&adjusted["plateContours"], true);
    assert_close(
        polygon_area(&nominal_opening["points"]),
        num(&want["nominalOpeningArea"]["value"]),
        num(&want["nominalOpeningArea"]["tolerance"]),
        "nominal opening area",
    );
    check_amount(
        polygon_area(&actual_opening["points"]),
        &want["actualOpeningArea"],
        "allowance-adjusted opening area",
    );
    let plate = body_named(&adjusted, "plate").clone();
    assert_eq!(plate["contours"], adjusted["plateContours"]);
    let plate_outer = contour(&plate["contours"], false);
    let report = export(
        &raw_case(plate.clone()).step,
        &format!("{id}-allowance-plate"),
    );
    assert_close(
        report.volume,
        (polygon_area(&plate_outer["points"])
            - polygon_area(&actual_opening["points"])
            - std::f64::consts::PI)
            * 1.5,
        num(&want["plateVolumeTolerance"]),
        "allowance plate STEP volume",
    );
    check_baseline(&report, &baseline(id)["allowancePlate"], "allowance plate");
    let foam = body_named(&adjusted, "plate-foam");
    let nominal_hole = num(&want["foamHoleAreaNominal"]);
    assert!(
        foam["contours"].as_array().unwrap().iter().any(|c| c["hole"] == true && (polygon_area(&c["points"]) - nominal_hole).abs() < 0.001),
        "plate foam retains the nominal hole"
    );

    for mount in ["rigid", "gasket"] {
        let mut document = mechanical_document();
        document["mechanical"]["openingAllowance"] = json!(0.0);
        document["mechanical"]["integratedPlateFrame"] = json!(mount == "rigid");
        document["mechanical"]["mount"] = json!(mount);
        if mount == "gasket" {
            document["mechanical"]["gasketTravel"] = json!(0.5);
            document["mechanical"]["gasket"] = json!({"inset":0.5,"width":1,"depth":0.5});
        }
        let input_contours = if mount == "gasket" {
            json!([{"hole":false,"points":square(0.0,80.0)}, {"hole":true,"points":square(12.0,26.0)}])
        } else {
            contours.clone()
        };
        let resolved = resolve_mechanical(&document, &input_contours);
        let plate = body_named(&resolved, "plate");
        let kind_key = if mount == "rigid" {
            "rigidPlateBodyKind"
        } else {
            "gasketPlateBodyKind"
        };
        assert_eq!(plate["body"]["kind"], want[kind_key]);
        if mount == "gasket" {
            let bodies = resolved["case"]["bodies"].as_array().unwrap();
            let strips = bodies
                .iter()
                .filter(|b| {
                    b["body"]["id"]
                        .as_str()
                        .unwrap_or_default()
                        .starts_with("gasket:")
                })
                .count();
            assert_eq!(strips as u64, want["gasketStripCount"].as_u64().unwrap());
            assert_eq!(
                resolved["generatedHardware"].as_array().unwrap().len() as u64,
                want["generatedHardwareCount"].as_u64().unwrap()
            );
            let find_body = |suffix: &str| {
                bodies
                    .iter()
                    .find(|b| {
                        b["body"]["id"] == suffix
                            || b["body"]["id"]
                                .as_str()
                                .unwrap_or_default()
                                .ends_with(suffix)
                    })
                    .unwrap()
                    .clone()
            };
            let lower = export(&raw_case(find_body(":lower")).step, &format!("{id}-lower"));
            let upper = export(&raw_case(find_body(":upper")).step, &format!("{id}-upper"));
            let tray = export(&raw_case(find_body("bottom")).step, &format!("{id}-tray"));
            let retainer = export(
                &raw_case(find_body("retainer")).step,
                &format!("{id}-retainer"),
            );
            let plate_shape = export(&raw_case(plate.clone()).step, &format!("{id}-gasket-plate"));
            assert_close(
                lower.min[2],
                num(&want["lowerStripMinZ"]),
                z_tolerance,
                "lower gasket ledge contact",
            );
            assert_close(
                lower.max[2],
                plate_shape.min[2],
                z_tolerance,
                "lower strip to plate contact",
            );
            assert_close(
                upper.min[2],
                plate_shape.max[2],
                z_tolerance,
                "upper strip to plate contact",
            );
            assert_close(
                upper.max[2],
                retainer.min[2],
                z_tolerance,
                "upper strip to retainer contact",
            );
            assert_close(
                tray.max[2],
                retainer.min[2],
                z_tolerance,
                "retainer to tray contact",
            );
            check_amount(
                lower.volume,
                &want["lowerStripVolume"],
                "lower strip volume",
            );
            assert_close(
                retainer.max[2],
                num(&want["retainerMaxZ"]),
                z_tolerance,
                "retainer maximum Z",
            );
            for (report, key) in [
                (&lower, "lowerStrip"),
                (&upper, "upperStrip"),
                (&tray, "tray"),
                (&retainer, "retainer"),
                (&plate_shape, "gasketPlate"),
            ] {
                check_baseline(report, &baseline(id)[key], key);
            }
        }
        let built = raw_assembly(resolved["case"].clone());
        let imported = import_checks(built.step.clone(), &format!("{id}-{mount}"));
        assert_close(
            imported.max[2],
            num(&want["assemblyTopZ"][mount]),
            z_tolerance,
            "resolved assembly maximum Z",
        );
        export(&built.step, &format!("{id}-{mount}-assembly"));
    }
}

// ---- Oracle qualification: units, negative controls -------------------------------------------

/// A planar-only export (30x30x2 plate with a deleted corner and a cutout) to derive controls from.
fn control_step() -> String {
    let cut_contours = json!([
        {"hole":false,"points":[{"x":10,"y":0},{"x":30,"y":0},{"x":30,"y":30},{"x":0,"y":30},{"x":0,"y":10},{"x":10,"y":10}]},
        {"hole":true,"points":square(15.0,20.0)}
    ]);
    let result = raw_case(
        json!({"revision":1,"body":{"id":"plate","name":"plate","boardId":"board","kind":"plate","thickness":2,"clearance":0},"contours":cut_contours}),
    );
    String::from_utf8(result.step).expect("STEP is text")
}

/// Rewrites every 3D `CARTESIAN_POINT` through `f`.
fn map_points(step: &str, f: impl Fn([f64; 3]) -> [f64; 3]) -> String {
    const OPEN: &str = "CARTESIAN_POINT('',(";
    let mut out = String::new();
    let mut rest = step;
    while let Some(start) = rest.find(OPEN) {
        let after = start + OPEN.len();
        out.push_str(&rest[..after]);
        let end = rest[after..].find("))").expect("closed point") + after;
        let numbers: Vec<f64> = rest[after..end]
            .split(',')
            .map(|n| n.trim().parse().expect("coordinate"))
            .collect();
        if numbers.len() == 3 {
            let p = f([numbers[0], numbers[1], numbers[2]]);
            out.push_str(&format!("{:?},{:?},{:?}", p[0], p[1], p[2]));
        } else {
            // 2D parameter-space points scale with the same factor only when the caller scales
            // uniformly; planar faces here make them irrelevant, so keep them untouched.
            out.push_str(&rest[after..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn replace_length_unit(step: &str, replacement: &str) -> String {
    const UNIT: &str = "( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );";
    let at = step.find(UNIT).expect("millimetre unit declaration");
    let start = step[..at].rfind('#').expect("entity id");
    let id: String = step[start + 1..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    format!(
        "{}#{id} = {replacement}{}",
        &step[..start],
        &step[at + UNIT.len()..]
    )
}

fn control_expectation() -> (f64, f64) {
    let want = expect("deleted-corner-and-cutout");
    (
        num(&want["volume"]["value"]),
        num(&want["volume"]["tolerance"]),
    )
}

#[test]
fn oracle_applies_declared_units_to_inch_and_metre_exports() {
    let step = control_step();
    let (volume, tolerance) = control_expectation();
    let mm = oracle::inspect(step.as_bytes());
    assert_close(mm.volume, volume, tolerance, "millimetre control volume");

    let inch = replace_length_unit(
        &map_points(&step, |p| p.map(|v| v / 25.4)),
        "( CONVERSION_BASED_UNIT('INCH',#9001) LENGTH_UNIT() NAMED_UNIT(#9002) );\n#9001 = LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#9003);\n#9002 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n#9003 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
    );
    let metre = replace_length_unit(
        &map_points(&step, |p| p.map(|v| v / 1000.0)),
        "( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.) );",
    );
    for (label, text) in [("inch", inch), ("metre", metre)] {
        assert_ne!(
            text, step,
            "{label} variant differs from the millimetre file"
        );
        let report = oracle::inspect(text.as_bytes());
        assert_eq!(report.read_status, oracle::READ_DONE, "{label} read");
        assert_eq!(report.solids, mm.solids, "{label} solid count");
        assert_close(
            report.volume,
            volume,
            tolerance,
            &format!("{label} volume in mm³"),
        );
        for axis in 0..3 {
            assert_close(
                report.min[axis],
                mm.min[axis],
                0.01,
                &format!("{label} min {axis}"),
            );
            assert_close(
                report.max[axis],
                mm.max[axis],
                0.01,
                &format!("{label} max {axis}"),
            );
        }
        // Without the declared unit the same coordinates would be 25.4x / 1000x off.
        assert!(report.max[0] > 29.0, "{label} bounds are in millimetres");
    }
}

#[test]
fn oracle_rejects_bad_syntax_for_the_right_reason() {
    for (label, bytes) in [
        ("truncated", control_step()[..4000].as_bytes().to_vec()),
        ("garbage", (0..=255u8).cycle().take(1024).collect()),
    ] {
        let report = oracle::inspect(&bytes);
        assert_eq!(
            report.read_status,
            oracle::READ_FAIL,
            "{label}: reader fails"
        );
        assert!(
            report.error.contains("ReadFile"),
            "{label}: failure comes from the reader: {}",
            report.error
        );
        assert_eq!(report.solids, 0, "{label}");
    }
}

#[test]
fn oracle_flags_a_file_with_no_solids() {
    let step = control_step().replace("MANIFOLD_SOLID_BREP", "UNKNOWN_SOLID_THING");
    let report = oracle::inspect(step.as_bytes());
    assert_eq!(
        report.read_status,
        oracle::READ_DONE,
        "the file itself is well formed"
    );
    assert_eq!(report.solids, 0, "no solid survives the transfer");
    assert_eq!(report.faces, 0, "nothing else sneaks in as faces");
}

#[test]
fn oracle_flags_invalid_topology_as_a_lost_solid_with_orphan_faces() {
    let step = control_step();
    let intact = oracle::inspect(step.as_bytes());
    assert_eq!((intact.solids, intact.orphan_faces), (1, 0));
    // Drop the first face from the closed shell: the shell can no longer bound a solid.
    let open = {
        let at = step.find("CLOSED_SHELL('',(").expect("closed shell") + "CLOSED_SHELL('',(".len();
        let comma = step[at..].find(',').expect("face list") + at;
        format!("{}{}", &step[..at], &step[comma + 1..])
    };
    let report = oracle::inspect(open.as_bytes());
    assert_eq!(
        report.read_status,
        oracle::READ_DONE,
        "syntax is fine; the defect is topological"
    );
    assert_eq!(report.solids, 0, "the open shell is not a solid");
    assert_eq!(
        report.orphan_faces, report.faces,
        "every remaining face is orphaned"
    );
    assert!(report.orphan_faces > 0);
}

#[test]
fn oracle_accepts_valid_wrong_geometry_and_only_the_fixture_comparison_rejects_it() {
    let step = control_step();
    let lifted = map_points(&step, |p| {
        if (p[2] - 2.0).abs() < 1e-9 {
            [p[0], p[1], 3.0]
        } else {
            p
        }
    });
    let report = oracle::inspect(lifted.as_bytes());
    assert_eq!(report.read_status, oracle::READ_DONE);
    assert!(
        report.valid && report.orphan_faces == 0 && report.solids == 1,
        "structurally sound"
    );
    let want = expect("deleted-corner-and-cutout");
    assert!(
        !amount_matches(report.volume, &want["volume"]),
        "volume {} must miss the fixture",
        report.volume
    );
    assert_close(
        report.volume,
        2325.0,
        0.1,
        "the lifted plate really is 1.5x thicker",
    );
}

// ---- Kernel interface ---------------------------------------------------------------------

/// The acceptance gate for a `CadKernel`: build, export, import and mesh the fixture cases and
/// judge them with the independent oracle and the shared expectations, never with the kernel's own
/// reader. A future kernel (or a refactor) passes this or it does not ship.
fn assert_kernel_meets_the_gate<K: CadKernel>(kernel: &K) {
    let prepared = |input: Value| -> PreparedCase {
        let assembly = json!({ "revision": input["revision"], "bodies": [input] });
        let prepared = prepare(&assembly, "prepare-case");
        serde_json::from_value(prepared["bodies"][0].clone()).expect("prepared case contract")
    };
    let holed = prepared(json!({
        "revision": 7,
        "body": { "id": "case", "name": "plate", "boardId": "board", "kind": "plate", "thickness": 2, "clearance": 0.5 },
        "contours": [
            { "hole": false, "points": square(0.0, 20.0) },
            { "hole": true, "points": square(5.0, 15.0) }
        ]
    }));
    let model = kernel
        .build_case(&holed)
        .expect("kernel builds the holed plate");
    let step = kernel
        .export_step(std::slice::from_ref(&model))
        .expect("kernel exports STEP");
    let id = "holed-plate-export";
    let report = export(&step, id);
    assert_eq!(report.solids, 1);
    check_amount(report.volume, &expect(id)["volume"], "kernel export volume");
    check_baseline(&report, baseline(id), id);

    let mesh = kernel.mesh(&model).expect("kernel meshes the plate");
    assert!(!mesh.positions.is_empty() && mesh.positions.len() % 9 == 0);
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert!(mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .all(|value| value.is_finite()));

    let imported = kernel
        .import_step(&step)
        .expect("kernel re-imports its own export");
    assert_eq!(imported.solid_count, 1);
    for axis in 0..3 {
        assert_close(
            imported.min[axis],
            report.min[axis],
            0.01,
            &format!("import min {axis}"),
        );
        assert_close(
            imported.max[axis],
            report.max[axis],
            0.01,
            &format!("import max {axis}"),
        );
    }
    assert!(!kernel
        .mesh(&imported.model)
        .expect("imported mesh")
        .positions
        .is_empty());

    // Two bodies in one STEP compound: counts and volume come from the oracle, not the kernel.
    let plate = |index: usize, z: f64| {
        prepared(json!({
            "revision": 9,
            "body": { "id": format!("plate-{index}"), "name": format!("plate-{index}"), "boardId": "board", "kind": "plate", "thickness": 2, "clearance": 0, "z": z },
            "contours": [{ "hole": false, "points": square(0.0, 20.0) }]
        }))
    };
    let parts = [
        kernel.build_case(&plate(0, 0.0)).unwrap(),
        kernel.build_case(&plate(1, 8.0)).unwrap(),
    ];
    let id = "compound-two-offset-plates";
    let compound = export(&kernel.export_step(&parts).expect("compound export"), id);
    assert_eq!(compound.solids, 2);
    check_amount(
        compound.volume,
        &expect(id)["volume"],
        "kernel compound volume",
    );

    assert!(
        kernel.import_step(b"ISO-10303-21; not really").is_err(),
        "garbage must not import"
    );
    let no_solids = control_step().replace("MANIFOLD_SOLID_BREP", "UNKNOWN_SOLID_THING");
    assert!(
        kernel.import_step(no_solids.as_bytes()).is_err(),
        "a file without solids must not import"
    );
}

#[test]
fn kernel_interface_meets_the_same_gate() {
    assert_kernel_meets_the_gate(&CadrumKernel);
}

#[test]
fn kernel_interface_preserves_production_behaviour_and_policy() {
    // The thin production wrappers must agree with the generic seam, error text included.
    let step = control_step().into_bytes();
    let through_wrapper = read_step_model_data(step.clone()).expect("wrapper import");
    let through_seam = read_step_model_data_with(&CadrumKernel, &step).expect("seam import");
    assert_eq!(through_wrapper.mesh.positions, through_seam.mesh.positions);
    assert_eq!(through_wrapper.min, through_seam.min);
    assert_eq!(through_wrapper.max, through_seam.max);
    assert_eq!(through_wrapper.solid_count, 1);
    // The size policy stays outside the kernel.
    let oversize = read_step_model_data_with(&CadrumKernel, &vec![0u8; MAX_STEP_BYTES + 1])
        .err()
        .unwrap();
    assert_eq!(oversize, "STEP import failed: invalid file size");
    let empty = CadrumKernel.import_step(b"").err().expect("empty input");
    assert!(empty.starts_with("STEP import failed"), "{empty}");
}
