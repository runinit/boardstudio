//! KiCad export, import and source-preservation checks through Core's artifact
//! request boundary, with KiCad 10 as the independent parser and plotter.
//! Ported from `kicad/test/export.test.ts` and `kicad/test/source.test.ts`,
//! which drove the same boundary through a Node subprocess.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use boardstudio_core::model::ProjectDoc;
use regex::Regex;
use serde_json::{Value, json};

fn artifact(request: Value) -> Result<Value, String> {
    let reply: Value =
        serde_json::from_str(&boardstudio_core::artifact::request(&request.to_string()))
            .expect("artifact reply is JSON");
    if reply["kind"] == "error" {
        return Err(reply["error"]["message"]
            .as_str()
            .unwrap_or("Rust artifact request failed")
            .to_owned());
    }
    Ok(reply)
}

fn export(
    document: &Value,
    target: Value,
    contours: &Value,
    model_paths: Value,
) -> Result<Vec<Value>, String> {
    let reply = artifact(json!({
        "id": "export", "kind": "export-pcb", "request": {
            "snapshotToken": "snapshot", "expectedRevision": document["revision"],
            "document": document, "target": target, "contours": contours,
            "modelPaths": model_paths,
        },
    }))?;
    Ok(reply["result"]["files"].as_array().expect("files").clone())
}

fn export_board(document: &Value, board_id: &str, contours: &Value) -> Result<String, String> {
    let name = document["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == board_id)
        .map(|board| board["name"].as_str().unwrap().to_owned())
        .unwrap();
    let files = export(
        document,
        json!({ "kind": "board", "boardId": board_id }),
        contours,
        json!({}),
    )?;
    Ok(files
        .iter()
        .find(|file| file["filename"] == format!("{name}.kicad_pcb"))
        .expect("native artifact omitted the board")["content"]
        .as_str()
        .unwrap()
        .to_owned())
}

fn export_footprint(
    document: &Value,
    definition_id: &str,
    model_paths: Value,
) -> Result<(String, String), String> {
    let files = export(
        document,
        json!({ "kind": "standalone-footprints", "definitionIds": [definition_id] }),
        &json!([]),
        model_paths,
    )?;
    let file = files
        .first()
        .expect("native artifact omitted the footprint");
    Ok((
        file["filename"].as_str().unwrap().to_owned(),
        file["content"].as_str().unwrap().to_owned(),
    ))
}

fn import_footprint(source: &str, id: &str) -> Result<Value, String> {
    Ok(artifact(json!({
        "id": format!("import:{id}"), "kind": "import-footprint", "definitionId": id, "source": source,
    }))?["result"]
        .clone())
}

fn compile_footprint(definition: &Value, side: &str) -> Value {
    artifact(json!({
        "id": "compile", "kind": "compile-footprints",
        "jobs": [{ "id": definition["id"], "definition": definition, "side": side }],
    }))
    .expect("compile")["result"][0]
        .clone()
}

fn empty_project(id: &str, name: &str) -> Value {
    serde_json::to_value(ProjectDoc::empty(id, name)).unwrap()
}

fn fixture() -> (Value, Value) {
    let mut doc = empty_project("project", "Test");
    doc["revision"] = json!(2);
    doc["definitions"] = json!([{
        "id": "switch", "name": "Switch", "kind": "switch",
        "courtyard": [{ "x": -7, "y": -7 }, { "x": 7, "y": -7 }, { "x": 7, "y": 7 }, { "x": -7, "y": 7 }],
        "pads": [
            { "id": "left", "number": "1", "at": { "x": -3, "y": 0 }, "size": { "x": 1.8, "y": 1.8 }, "shape": "circle", "drill": 0.9 },
            { "id": "right", "number": "2", "at": { "x": 3, "y": 0 }, "size": { "x": 1.8, "y": 1.8 }, "shape": "circle", "drill": 0.9 },
        ],
    }]);
    doc["parts"] = json!([{ "id": "s1", "definitionId": "switch", "reference": "SW1", "pose": { "at": { "x": 0, "y": 0 }, "rotation": 0 }, "side": "front" }]);
    doc["nets"] =
        json!([{ "id": "row", "name": "ROW0", "pins": [{ "partId": "s1", "padId": "left" }] }]);
    doc["boards"] = json!([{ "id": "main", "name": "main", "outlineIds": [], "partIds": ["s1"], "netIds": ["row"], "thickness": 1.6 }]);
    let contours = json!([{ "hole": false, "points": [{ "x": -12, "y": -12 }, { "x": 12, "y": -12 }, { "x": 12, "y": 12 }, { "x": -12, "y": 12 }] }]);
    (doc, contours)
}

fn project_with(definition: &Value) -> Value {
    let mut doc = empty_project(
        definition["id"].as_str().unwrap(),
        definition["name"].as_str().unwrap(),
    );
    doc["definitions"] = json!([definition]);
    doc
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap()
}

fn assert_match(text: &str, pattern: &str) {
    assert!(
        re(pattern).is_match(text),
        "expected /{pattern}/ in:\n{text}"
    );
}

fn assert_error(result: Result<impl std::fmt::Debug, String>, pattern: &str) {
    let message = result.expect_err("expected the request to be rejected");
    assert_match(&message, pattern);
}

fn temp_dir(label: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "boardstudio-kicad-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn kicad_cli(arguments: &[&str]) {
    let output = Command::new("kicad-cli")
        .args(arguments)
        .output()
        .expect("kicad-cli (KiCad 10) is required for the KiCad integration checks");
    assert!(
        output.status.success(),
        "kicad-cli {arguments:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn path_str(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn drc(doc: &Value, contours: &Value) -> Value {
    let directory = temp_dir("drc");
    let board = directory.join("main.kicad_pcb");
    let report = directory.join("drc.json");
    std::fs::write(&board, export_board(doc, "main", contours).unwrap()).unwrap();
    kicad_cli(&[
        "pcb",
        "drc",
        "--format",
        "json",
        "--output",
        path_str(&report),
        path_str(&board),
    ]);
    serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap()
}

fn item_types(report: &Value, key: &str) -> Vec<String> {
    report[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["type"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn exports_a_deterministic_board_with_flipped_y_coordinates_and_net_pads() {
    let (doc, contours) = fixture();
    let first = export_board(&doc, "main", &contours).unwrap();
    assert_eq!(first, export_board(&doc, "main", &contours).unwrap());
    assert_match(&first, r#"\(net 1 "ROW0"\)"#);
    assert_match(&first, r"\(start -12 12\) \(end 12 12\)");
    assert_match(&first, r#"\(pad "1" thru_hole circle .*\(net 1 "ROW0"\)"#);
}

#[test]
fn compiled_geometry_is_immutable_and_ignores_net_labels() {
    let (doc, _) = fixture();
    let mut definition = doc["definitions"][0].clone();
    let first = compile_footprint(&definition, "front");
    definition["pads"][0]["netId"] = json!("row");
    let second = compile_footprint(&definition, "front");
    assert_eq!(first["geometry"], second["geometry"]);
    assert!(first["geometry"]["pads"][0].get("netId").is_none());
    assert_match(
        first["previewSvg"].as_str().unwrap_or(""),
        r"(?s)<svg.*<rect",
    );
    assert_ne!(
        compile_footprint(&definition, "back")["geometry"],
        first["geometry"]
    );
}

#[test]
fn exports_front_and_back_copper_traces_and_vias() {
    let (mut doc, contours) = fixture();
    doc["boards"][0]["traces"] = json!([
        { "id": "front", "start": { "x": -3, "y": 0 }, "end": { "x": 0, "y": 0 }, "width": 0.25, "layer": "front", "netId": "row" },
        { "id": "back", "start": { "x": 0, "y": 0 }, "end": { "x": 3, "y": 0 }, "width": 0.25, "layer": "back", "netId": "row" },
    ]);
    doc["boards"][0]["vias"] = json!([{ "id": "through", "at": { "x": 0, "y": 0 }, "size": 0.8, "drill": 0.4, "netId": "row" }]);
    let board = export_board(&doc, "main", &contours).unwrap();
    assert_match(
        &board,
        r#"\(segment \(start -3 0\) \(end 0 0\).*\(layer "F.Cu"\) \(net 1\)"#,
    );
    assert_match(
        &board,
        r#"\(segment \(start 0 0\) \(end 3 0\).*\(layer "B.Cu"\) \(net 1\)"#,
    );
    assert_match(
        &board,
        r"\(via \(at 0 0\) \(size 0.8\) \(drill 0.4\).*\(net 1\)",
    );
    assert!(drc(&doc, &contours)["violations"].is_array());
}

#[test]
fn rejects_stale_revisions_and_conflicting_net_assignments() {
    let (mut doc, contours) = fixture();
    assert_error(
        artifact(json!({ "id": "stale", "kind": "export-pcb", "request": {
            "snapshotToken": "stale", "expectedRevision": 1, "document": doc,
            "target": { "kind": "board", "boardId": "main" }, "contours": contours, "modelPaths": {},
        } })),
        "committed current v2 revision",
    );
    doc["definitions"][0]["pads"][0]["netId"] = json!("another");
    assert_error(export_board(&doc, "main", &contours), "Conflicting net");
}

#[test]
fn requires_relative_model_paths_and_emits_standalone_footprints() {
    let (doc, _) = fixture();
    let mut definition = doc["definitions"][0].clone();
    definition["models"] = json!([{
        "assetId": "switch-model",
        "offset": { "x": 0, "y": 0, "z": 0 }, "rotation": { "x": 0, "y": 0, "z": 0 }, "scale": { "x": 1, "y": 1, "z": 1 },
    }]);
    let project = project_with(&definition);
    assert_error(
        export_footprint(
            &project,
            "switch",
            json!({ "switch-model": "../secret.step" }),
        ),
        "safe relative path",
    );
    let (_, content) = export_footprint(
        &project,
        "switch",
        json!({ "switch-model": "models/switch.step" }),
    )
    .unwrap();
    assert_match(&content, r"\$\{KIPRJMOD\}/models/switch.step");
}

#[test]
fn footprint_filename_matches_the_serialized_kicad_name() {
    let (doc, _) = fixture();
    let mut definition = doc["definitions"][0].clone();
    definition["name"] = json!("../Switch / Test");
    let project = project_with(&definition);
    let (filename, content) = export_footprint(&project, "switch", json!({})).unwrap();
    let name = re(r#"(?m)^\(footprint "([^"]+)""#)
        .captures(&content)
        .unwrap()[1]
        .to_owned();
    let imported = import_footprint(&content, "roundtrip").unwrap();
    assert_eq!(filename, format!("{name}.kicad_mod"));
    assert_eq!(imported["definition"]["name"], name);
}

#[test]
fn standalone_footprint_rejects_invalid_courtyard_and_pad_geometry() {
    let (doc, _) = fixture();
    let mut definition = doc["definitions"][0].clone();
    // JSON cannot carry NaN or Infinity; null stands in for the non-finite value.
    definition["courtyard"][0]["x"] = Value::Null;
    assert_error(
        export_footprint(&project_with(&definition), "switch", json!({})),
        "courtyard|Invalid|invalid type",
    );

    definition["courtyard"][0]["x"] = json!(-7);
    definition["pads"][0]["size"]["x"] = Value::Null;
    assert_error(
        export_footprint(&project_with(&definition), "switch", json!({})),
        "Invalid pad|Invalid|invalid type",
    );

    definition["pads"][0]["size"]["x"] = json!(1.8);
    definition["pads"][0]["drill"] = json!(0);
    assert_error(
        export_footprint(&project_with(&definition), "switch", json!({})),
        "Invalid pad|Invalid",
    );
}

#[test]
fn standalone_footprints_retain_repeated_logical_numbers_with_unique_physical_pad_identities() {
    let (doc, _) = fixture();
    let mut definition = doc["definitions"][0].clone();
    definition["pads"][0]["number"] = json!(" ");
    assert_error(
        export_footprint(&project_with(&definition), "switch", json!({})),
        "Empty electrical pad number",
    );

    definition["pads"][0]["number"] = json!("2");
    let (_, content) = export_footprint(&project_with(&definition), "switch", json!({})).unwrap();
    let imported = import_footprint(&content, "repeated-pads").unwrap();
    let pads = imported["definition"]["pads"].as_array().unwrap();
    assert_eq!(
        pads.iter()
            .map(|pad| pad["number"].clone())
            .collect::<Vec<_>>(),
        [json!("2"), json!("2")]
    );
    assert_ne!(pads[0]["id"], pads[1]["id"]);

    definition["pads"][1]["id"] = definition["pads"][0]["id"].clone();
    assert_error(
        export_footprint(&project_with(&definition), "switch", json!({})),
        "Duplicate .*pad",
    );
}

fn switch_source() -> String {
    let (doc, _) = fixture();
    export_footprint(&doc, "switch", json!({})).unwrap().1
}

#[test]
fn imports_supported_external_footprint_pads_and_courtyard() {
    let imported = import_footprint(&switch_source(), "external-switch").unwrap();
    assert_eq!(imported["definition"]["name"], "Switch");
    assert_eq!(imported["definition"]["pads"].as_array().unwrap().len(), 2);
    assert_eq!(
        imported["definition"]["pads"][0]["at"]["x"].as_f64(),
        Some(-3.0)
    );
    assert_eq!(
        imported["definition"]["pads"][0]["at"]["y"].as_f64(),
        Some(0.0)
    );
    assert_eq!(
        imported["definition"]["courtyard"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

fn retained_source(source: &str, id: &str) -> Value {
    import_footprint(source, id).unwrap()["definition"]["kicadSource"]["source"].clone()
}

#[test]
fn native_source_import_preserves_rotated_pads_and_external_model_references() {
    let source = switch_source();
    let rotated = source.replace("(at -3 0)", "(at -3 0 45)");
    assert_eq!(retained_source(&rotated, "rotated"), json!(rotated));

    let model = re(r"\n  \)\s*$")
        .replace(
            &source,
            "\n    (model \"${KIPRJMOD}/models/switch.step\")\n  )",
        )
        .into_owned();
    assert_eq!(retained_source(&model, "model"), json!(model));
}

#[test]
fn imports_shuffled_courtyard_lines_by_connected_endpoints() {
    let source = switch_source();
    let line = re(
        r#"\(fp_line \(start [^)]+\) \(end [^)]+\) \(stroke \(width [^)]+\) \(type solid\)\) \(layer "F\.CrtYd"\) \(uuid "[^"]+"\)\)"#,
    );
    let lines: Vec<String> = line
        .find_iter(&source)
        .map(|found| found.as_str().to_owned())
        .collect();
    assert_eq!(lines.len(), 4);
    let markers: Vec<String> = (0..4)
        .map(|index| format!("__courtyard_{index}__"))
        .collect();
    let mut shuffled = source.clone();
    for (index, found) in lines.iter().enumerate() {
        shuffled = shuffled.replacen(found, &markers[index], 1);
    }
    for (position, line_index) in [2usize, 0, 3, 1].into_iter().enumerate() {
        shuffled = shuffled.replacen(&markers[position], &lines[line_index], 1);
    }
    let courtyard = |text: &str, id: &str| -> Vec<Value> {
        import_footprint(text, id).unwrap()["definition"]["courtyard"]
            .as_array()
            .unwrap()
            .clone()
    };
    let imported = courtyard(&shuffled, "shuffled");
    let original = courtyard(&source, "original");
    let start = imported
        .iter()
        .position(|point| *point == original[0])
        .unwrap();
    let rotated: Vec<Value> = imported[start..]
        .iter()
        .chain(&imported[..start])
        .cloned()
        .collect();
    assert_eq!(rotated, original);
}

#[test]
fn native_import_preserves_disconnected_and_curved_courtyard_source_with_diagnostics() {
    let source = switch_source();
    let broken = source.replacen("(end 7 7)", "(end 8 7)", 1);
    assert_eq!(retained_source(&broken, "broken"), json!(broken));

    let arc = re(r"\n  \)\s*$")
        .replace(
            &source,
            "\n    (fp_arc (start 0 0) (mid 1 1) (end 2 0) (layer \"F.CrtYd\"))\n  )",
        )
        .into_owned();
    assert_eq!(retained_source(&arc, "arc"), json!(arc));
}

#[test]
fn rejects_a_truncated_footprint_without_hanging() {
    assert!(import_footprint("(footprint \"broken\"", "broken").is_err());
}

#[test]
fn native_import_preserves_pad_features_beyond_the_static_preview_projection() {
    let source = switch_source();
    for modified in [
        source.replacen("(drill 0.9)", "(drill oval 0.9 1.1)", 1),
        source.replacen(
            "(layers \"*.Cu\" \"*.Mask\")",
            "(layers \"B.Cu\" \"B.Mask\")",
            1,
        ),
        source.replacen("(drill 0.9)", "(drill 0.9) (solder_mask_margin 0.2)", 1),
    ] {
        assert_eq!(retained_source(&modified, "preserved"), json!(modified));
    }
}

#[test]
fn kicad_10_parses_the_exported_board() {
    let (doc, contours) = fixture();
    let report = drc(&doc, &contours);
    assert_eq!(report["violations"], json!([]));
    assert_eq!(report["unconnected_items"], json!([]));
    let directory = temp_dir("parse");
    let board = directory.join("main.kicad_pcb");
    std::fs::write(&board, export_board(&doc, "main", &contours).unwrap()).unwrap();

    let stats = directory.join("stats.json");
    kicad_cli(&[
        "pcb",
        "export",
        "stats",
        "--format",
        "json",
        "--output",
        path_str(&stats),
        path_str(&board),
    ]);
    let parsed: Value = serde_json::from_str(&std::fs::read_to_string(&stats).unwrap()).unwrap();
    assert_eq!(parsed["board"]["has_outline"], true);
    assert_eq!(parsed["board"]["width"], "24.0000 mm");
    assert_eq!(parsed["pads"]["through_hole"], 2);
    assert_eq!(parsed["components"]["total"]["front"], 1);

    let netlist = directory.join("netlist.d356");
    kicad_cli(&[
        "pcb",
        "export",
        "ipcd356",
        "--output",
        path_str(&netlist),
        path_str(&board),
    ]);
    let nets = std::fs::read_to_string(&netlist).unwrap();
    assert_match(&nets, r"317ROW0\s+SW1\s+-1\s");
    assert_match(&nets, r"317N/C\s+SW1\s+-2\s");
}

#[test]
fn kicad_reports_unrouted_nets_independently_of_serializer_validity() {
    let (mut doc, mut contours) = fixture();
    doc["parts"].as_array_mut().unwrap().push(json!({ "id": "s2", "definitionId": "switch", "reference": "SW2", "pose": { "at": { "x": 0, "y": 18 }, "rotation": 0 }, "side": "front" }));
    doc["boards"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("s2"));
    doc["nets"][0]["pins"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "partId": "s2", "padId": "left" }));
    contours[0]["points"][2]["y"] = json!(30);
    contours[0]["points"][3]["y"] = json!(30);
    let report = drc(&doc, &contours);
    assert_eq!(
        item_types(&report, "unconnected_items"),
        ["unconnected_items"]
    );
    assert_eq!(report["violations"], json!([]));
}

#[test]
fn kicad_reports_copper_clearance_independently_of_unrouted_nets() {
    let (mut doc, contours) = fixture();
    doc["definitions"][0]["pads"][1]["at"]["x"] = json!(-1.1);
    doc["nets"].as_array_mut().unwrap().push(
        json!({ "id": "column", "name": "COL0", "pins": [{ "partId": "s1", "padId": "right" }] }),
    );
    doc["boards"][0]["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("column"));
    let report = drc(&doc, &contours);
    assert_eq!(report["unconnected_items"], json!([]));
    assert!(
        item_types(&report, "violations")
            .iter()
            .any(|kind| kind == "clearance")
    );
}

#[test]
fn kicad_10_plots_the_standalone_footprint() {
    let directory = temp_dir("footprint");
    let library = directory.join("BoardStudio.pretty");
    let output = directory.join("svg");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(library.join("Switch.kicad_mod"), switch_source()).unwrap();
    kicad_cli(&[
        "fp",
        "export",
        "svg",
        path_str(&library),
        "--output",
        path_str(&output),
    ]);
    assert!(output.join("Switch.svg").exists());
}

// --- imported source preservation (from source.test.ts) ---

const RICH_SOURCE: &str = r#"(footprint "Rich Imported Ω" (version 20240108) (generator "pcbnew")
  (layer "F.Cu") (uuid 00000000-0000-4000-8000-000000000001)
  (at 0 0)
  (property "Reference" "REF**" (at 0 0 0) (layer "F.SilkS") (effects (font (size 1 1) (thickness 0.15))))
  (property "Value" "Rich Imported Ω" (at 0 1 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (property "Vendor note" "café 東京 & Ω" (at 0 2 0) (layer "F.Fab") (effects (font (size 1 1) (thickness 0.15))))
  (attr through_hole)
  (fp_arc (start -3 -2) (mid -2 -3) (end -1 -2) (stroke (width 0.12) (type default)) (layer "F.CrtYd") (uuid 00000000-0000-4000-8000-000000000002))
  (fp_line (start -3 -2) (end 3 -2) (stroke (width 0.12) (type default)) (layer "F.Fab") (uuid 00000000-0000-4000-8000-000000000003))
  (pad "1" thru_hole oval (at -1.5 0 37) (size 2 1) (drill oval 1.2 0.6 (offset 0.2 0)) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000004))
  (pad "1" thru_hole circle (at 2 3 30) (size 1.8 1.8) (drill 0.8) (layers "*.Cu" "*.Mask") (uuid 00000000-0000-4000-8000-000000000005))
  (model "${KIPRJMOD}/models/rich.step" (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))
  (group "vendor geometry" (uuid 00000000-0000-4000-8000-000000000006) (members 00000000-0000-4000-8000-000000000003))
)"#;

fn import_source(id: &str, source: &str) -> Value {
    let reply = artifact(json!({ "id": format!("import-{id}"), "kind": "import-footprint", "definitionId": id, "source": source })).unwrap();
    assert_eq!(reply["kind"], "import-footprint");
    assert_eq!(reply["id"], format!("import-{id}"));
    reply["result"].clone()
}

fn board_document(definition: &Value, conflicting: bool) -> Value {
    let mut doc = empty_project("source-test", "Imported source fixture");
    doc["revision"] = json!(4);
    doc["definitions"] = json!([definition]);
    let id = definition["id"].clone();
    doc["parts"] = json!([
        { "id": "front-part", "definitionId": id, "reference": "J1", "pose": { "at": { "x": 11, "y": 13 }, "rotation": 37 }, "side": "front" },
        { "id": "back-part", "definitionId": id, "reference": "J2", "pose": { "at": { "x": 11, "y": 13 }, "rotation": 37 }, "side": "back" },
    ]);
    let mut pins = vec![
        json!({ "partId": "front-part", "padId": "pad-0" }),
        json!({ "partId": "front-part", "padId": "pad-1" }),
        json!({ "partId": "back-part", "padId": "pad-0" }),
    ];
    if !conflicting {
        pins.push(json!({ "partId": "back-part", "padId": "pad-1" }));
    }
    let mut nets = vec![json!({ "id": "shared", "name": "SHARED", "pins": pins })];
    let mut net_ids = vec![json!("shared")];
    if conflicting {
        nets.push(json!({ "id": "conflict", "name": "CONFLICT", "pins": [{ "partId": "back-part", "padId": "pad-1" }] }));
        net_ids.push(json!("conflict"));
    }
    doc["nets"] = Value::Array(nets);
    doc["boards"] = json!([{ "id": "board", "name": "source-board", "outlineIds": [], "partIds": ["front-part", "back-part"], "netIds": net_ids, "thickness": 1.6 }]);
    doc
}

fn source_contours() -> Value {
    json!([{ "hole": false, "points": [{ "x": -30, "y": -24 }, { "x": 30, "y": -24 }, { "x": 30, "y": 24 }, { "x": -30, "y": 24 }] }])
}

fn export_source_board(doc: &Value) -> Result<String, String> {
    export_board(doc, "board", &source_contours())
}

/// The balanced form whose `(property "Reference" ...)` names `reference`.
fn footprint_for_reference(board: &str, reference: &str) -> String {
    let bytes = board.as_bytes();
    let mut from = 0;
    while let Some(offset) = [
        board[from..].find("(footprint "),
        board[from..].find("(module "),
    ]
    .into_iter()
    .flatten()
    .min()
    {
        let start = from + offset;
        let (mut depth, mut quoted, mut escaped) = (0i32, false, false);
        for index in start..bytes.len() {
            let character = bytes[index];
            if quoted {
                if escaped {
                    escaped = false;
                } else if character == b'\\' {
                    escaped = true;
                } else if character == b'"' {
                    quoted = false;
                }
            } else if character == b'"' {
                quoted = true;
            } else if character == b'(' {
                depth += 1;
            } else if character == b')' {
                depth -= 1;
                if depth == 0 {
                    let form = &board[start..=index];
                    if form.contains(&format!("(property \"Reference\" \"{reference}\"")) {
                        return form.to_owned();
                    }
                    break;
                }
            }
        }
        from = start + 1;
    }
    panic!("Footprint with reference {reference} is missing");
}

fn global_kicad(root: (f64, f64, f64), local: (f64, f64)) -> (f64, f64) {
    let theta = root.2.to_radians();
    (
        root.0 + theta.cos() * local.0 + theta.sin() * local.1,
        root.1 - theta.sin() * local.0 + theta.cos() * local.1,
    )
}

fn plot(board: &str, layers: &str, label: &str) -> (PathBuf, String) {
    let directory = temp_dir(label);
    let path = directory.join("board.kicad_pcb");
    std::fs::write(&path, board).unwrap();
    let svg = directory.join("board.svg");
    kicad_cli(&[
        "pcb",
        "export",
        "svg",
        "--layers",
        layers,
        "--output",
        path_str(&svg),
        path_str(&path),
    ]);
    (path, std::fs::read_to_string(&svg).unwrap())
}

/// Pad rows as KiCad's own Python API reads them; `None` without `pcbnew`.
fn pcbnew_pads(path: &Path) -> Option<Vec<Value>> {
    let has = Command::new("python3")
        .args(["-c", "import pcbnew"])
        .output()
        .ok()?
        .status
        .success();
    if !has {
        return None;
    }
    let script = "import json, pcbnew, sys\nboard = pcbnew.LoadBoard(sys.argv[1])\nrows = [{\"ref\": fp.GetReference(), \"number\": pad.GetNumber(), \"x\": pcbnew.ToMM(pad.GetPosition().x), \"y\": pcbnew.ToMM(pad.GetPosition().y), \"angle\": pad.GetOrientationDegrees()} for fp in board.GetFootprints() for pad in fp.Pads()]\nprint(json.dumps(rows))";
    let output = Command::new("python3")
        .args(["-c", script, path_str(path)])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(serde_json::from_slice(&output.stdout).unwrap())
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() < tolerance
}

#[test]
fn native_source_import_projects_asymmetric_pads_and_retains_exact_kicad_source() {
    let imported = import_source("rich-source", RICH_SOURCE);
    assert_eq!(imported["definition"]["name"], "Rich Imported Ω");
    assert_eq!(imported["definition"]["kicadSource"]["source"], RICH_SOURCE);
    assert_eq!(imported["geometry"]["pads"].as_array().unwrap().len(), 2);
    assert_eq!(
        imported["geometry"]["courtyard"].as_array().unwrap().len(),
        4
    );
    assert!(
        imported["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["message"]
                .as_str()
                .unwrap()
                .contains("models/rich.step"))
    );
    assert!(RICH_SOURCE.contains("(drill oval 1.2 0.6 (offset 0.2 0))"));
    assert!(RICH_SOURCE.contains("café 東京 & Ω"));
}

#[test]
fn native_board_export_preserves_untouched_source_spans_and_kicad_10_parses_and_plots_both_sides() {
    let imported = import_source("board-source", RICH_SOURCE);
    let board = export_source_board(&board_document(&imported["definition"], false)).unwrap();
    let front = footprint_for_reference(&board, "J1");
    let back = footprint_for_reference(&board, "J2");
    assert_match(&front, r"\(at 11 -13 37\)");
    assert_match(&front, r#"\(pad "1" thru_hole circle \(at 2 3 67\)"#);
    assert_match(&back, r"\(at 11 -13 -143\)");
    assert_match(&back, r#"\(pad "1" thru_hole circle \(at 2 -3 187\)"#);
    let front_global = global_kicad((11.0, -13.0, 37.0), (2.0, 3.0));
    let back_global = global_kicad((11.0, -13.0, -143.0), (2.0, -3.0));
    assert!(close(front_global.0, 14.402716, 1e-6) && close(front_global.1, -11.807724, 1e-6));
    assert!(close(back_global.0, 11.208174, 1e-6) && close(back_global.1, -9.400463, 1e-6));
    assert_match(&board, r"\(drill oval 1\.2 0\.6 \(offset 0\.2 0\)\)");
    assert_match(&board, r#"\(property "Vendor note" "café 東京 & Ω""#);
    assert_match(&board, r#"\(model "\$\{KIPRJMOD\}/models/rich\.step""#);
    assert_match(
        &board,
        r"\(fp_arc \(start -3 -2\) \(mid -2 -3\) \(end -1 -2\)",
    );
    let uuids: Vec<String> = re(r#"\(uuid "([0-9a-f-]{36})"\)"#)
        .captures_iter(&board)
        .map(|found| found[1].to_owned())
        .collect();
    let group = re(
        r#"\(group "vendor geometry" \(uuid "([0-9a-f-]{36})"\) \(members "([0-9a-f-]{36})"\)\)"#,
    )
    .captures(&board)
    .expect("group and member UUIDs should be retained and rewritten consistently");
    assert!(uuids.contains(&group[1].to_owned()));
    assert!(uuids.contains(&group[2].to_owned()));
    assert_eq!(board.matches("(net 1 \"SHARED\")").count(), 5);

    let (path, plotted) = plot(
        &board,
        "F.Cu,B.Cu,F.SilkS,B.SilkS,F.CrtYd,B.CrtYd",
        "source",
    );
    assert!(plotted.contains("<svg"));
    if let Some(rows) = pcbnew_pads(&path) {
        for (reference, x, y, angle) in [
            ("J1", 14.402716, -11.807724, 67.0),
            ("J2", 11.208174, -9.400463, 187.0),
        ] {
            let actual = rows
                .iter()
                .find(|row| {
                    row["ref"] == reference
                        && row["number"] == "1"
                        && close(row["angle"].as_f64().unwrap(), angle, 1e-3)
                })
                .unwrap_or_else(|| panic!("pcbnew should report {reference} pad angle {angle}"));
            assert!(close(actual["x"].as_f64().unwrap(), x, 1e-6));
            assert!(close(actual["y"].as_f64().unwrap(), y, 1e-6));
        }
    }
}

#[test]
fn imported_source_without_a_pad_angle_preserves_the_absolute_angle_on_both_board_sides() {
    let angled = r#"(footprint "Angled" (layer "F.Cu") (at 20 30 37)
    (fp_line (start -4 -4) (end 4 -4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start 4 -4) (end 4 4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start 4 4) (end -4 4) (layer "F.CrtYd") (width 0.05))
    (fp_line (start -4 4) (end -4 -4) (layer "F.CrtYd") (width 0.05))
    (pad 1 smd rect (at 2 3) (size 2 1) (layers F.Cu F.Paste F.Mask))
    (pad 2 smd rect (at 3 3) (size 1 1) (layers F.Cu F.Paste F.Mask)))"#;
    let imported = import_source("angled-source", angled);
    assert_eq!(imported["geometry"]["pads"].as_array().unwrap().len(), 2);
    assert_eq!(imported["definition"]["kicadSource"]["source"], angled);
    let pad = imported["definition"]["pads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pad| pad["id"] == "pad-0")
        .unwrap();
    assert_eq!(
        pad["rotation"].as_f64(),
        Some(-37.0),
        "root angle 37 and omitted pad angle (0) normalize to a local offset of -37 degrees"
    );

    let board = export_source_board(&board_document(&imported["definition"], false)).unwrap();
    assert_match(
        &footprint_for_reference(&board, "J1"),
        r"\(pad 1 smd rect \(at 2 3 0\)",
    );
    assert_match(
        &footprint_for_reference(&board, "J2"),
        r"\(pad 1 smd rect \(at 2 -3 254\)",
    );

    let (path, plotted) = plot(&board, "F.Cu,B.Cu,F.CrtYd,B.CrtYd", "angle");
    assert!(plotted.contains("<svg"));
    if let Some(rows) = pcbnew_pads(&path) {
        let angle = |reference: &str| {
            rows.iter().find(|row| row["ref"] == reference).unwrap()["angle"]
                .as_f64()
                .unwrap()
        };
        assert!(angle("J1").abs() < 1e-3);
        assert!(close(angle("J2"), 254.0, 1e-3));
    }
}

#[test]
fn legacy_kicad_arc_preserves_sweep_geometry_in_a_kicad_10_board_export() {
    let legacy = r#"(module LegacyArc (layer F.Cu)
    (fp_line (start -4 -4) (end 4 -4) (layer F.CrtYd) (width 0.05))
    (fp_line (start 4 -4) (end 4 4) (layer F.CrtYd) (width 0.05))
    (fp_line (start 4 4) (end -4 4) (layer F.CrtYd) (width 0.05))
    (fp_line (start -4 4) (end -4 -4) (layer F.CrtYd) (width 0.05))
    (fp_arc (start 0 0) (end 2 0) (angle 90) (layer F.SilkS) (width 0.15))
    (pad 1 smd rect (at 0 0) (size 1 1) (layers F.Cu F.Paste F.Mask))
    (pad 2 smd rect (at 3 0) (size 1 1) (layers F.Cu F.Paste F.Mask)))"#;
    let imported = import_source("legacy-arc", legacy);
    assert_eq!(imported["definition"]["kicadSource"]["source"], legacy);
    assert_eq!(
        imported["geometry"]["courtyard"].as_array().unwrap().len(),
        4
    );
    assert_eq!(imported["geometry"]["pads"].as_array().unwrap().len(), 2);

    let board = export_source_board(&board_document(&imported["definition"], false)).unwrap();
    assert_match(
        &footprint_for_reference(&board, "J1"),
        r"\(fp_arc \(start 2 0\) \(mid 1\.414214 1\.414214\) \(end 0 2\)",
    );
    assert_match(
        &footprint_for_reference(&board, "J2"),
        r"\(fp_arc \(start 2 0\) \(mid 1\.414214 -1\.414214\) \(end 0 -2\)",
    );
    let (_, plotted) = plot(
        &board,
        "F.Cu,B.Cu,F.SilkS,B.SilkS,F.CrtYd,B.CrtYd",
        "legacy-arc",
    );
    assert!(plotted.contains("<svg"));
}

#[test]
fn native_source_export_rejects_repeated_pad_numbers_assigned_to_different_nets() {
    let imported = import_source("conflict-source", RICH_SOURCE);
    assert_error(
        export_source_board(&board_document(&imported["definition"], true)),
        "Repeated logical pad number 1 has conflicting net assignments",
    );
}
