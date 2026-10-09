//! The included split keyboards as mechanical mirror pairs, and the reused-PCB case.
//!
//! The projects are built by the shipped recipes (`core/examples/demo_projects`) and
//! then driven only through Core's public request interface and its effective-input
//! read, as the Case workspace does: opening, typed mechanical edits, Undo and Redo,
//! and reopening a saved project. Key positions are the measured layout's and are
//! pinned separately by `demo_projects.rs`; here they only prove the pair map. Each
//! pair's closure holes are checked against what Core projects, and every half is
//! exported to KiCad with those holes present.
use crate::demo_projects::demo_projects;

use boardstudio_core::CoreEngine;
use boardstudio_core::artifact::kicad;
use boardstudio_core::generators;
use boardstudio_core::mechanical::inputs::effective_document;
use boardstudio_core::mechanical::{MechanicalTarget, effective_inputs};
use boardstudio_core::model::{ExportTarget, PrepareExportRequest, ProjectDoc};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

const MEASURED_SPLITS: [&str; 7] = [
    "corne", "lily58", "klor", "sweep", "chocofi", "totem", "cantor",
];
const SOFLE: [&str; 3] = ["v2", "rgb", "choc"];

/// Every split project and the reused-PCB demonstration, built once.
fn projects() -> &'static BTreeMap<String, Value> {
    static PROJECTS: OnceLock<BTreeMap<String, Value>> = OnceLock::new();
    PROJECTS.get_or_init(|| {
        let mut engine = CoreEngine::new();
        let layouts: Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../content/layouts/keyboard-layouts.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut built = BTreeMap::new();
        for id in MEASURED_SPLITS {
            let project = demo_projects::measured_project(&mut engine, id, &layouts[id]).unwrap();
            built.insert(id.to_owned(), project);
        }
        for variant in SOFLE {
            let project = demo_projects::sofle_project(&mut engine, variant).unwrap();
            built.insert(format!("sofle-{variant}"), project);
        }
        built.insert(
            "reused-pcb".to_owned(),
            demo_projects::reused_pcb_project(&mut engine).unwrap(),
        );
        built
    })
}

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    let reply: Value = serde_json::from_str(&engine.request(&input.to_string())).unwrap();
    assert_ne!(reply["kind"], "error", "{reply}");
    reply
}

/// A fresh engine with `project` open, as when a saved project is reopened.
fn reopen(project: &Value) -> (CoreEngine, Value) {
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":project}),
    );
    assert_eq!(reply["kind"], "scene");
    let document = reply["document"].clone();
    (engine, document)
}

fn mounting_hole() -> Value {
    let definition = boardstudio_core::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|d| {
            d.generator
                .as_ref()
                .is_some_and(|g| g.source == "ceoloide/mounting_hole_npth")
        })
        .unwrap();
    serde_json::to_value(definition).unwrap()
}

/// Submits one typed mechanical change for a physical instance and returns the
/// accepted document.
fn change(engine: &mut CoreEngine, document: &Value, instance: &str, change: Value) -> Value {
    let reply = request(
        engine,
        json!({"id":"edit","kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":"keyboard-test","phase":"commit","targetIds":[],
            "operation":{"kind":"apply-mechanical-edit","edit":{
                "target":{"kind":"instance","instanceId":instance},"change":change,"mountingHole":mounting_hole()}}}}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    reply["document"].clone()
}

fn undo(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"id":"undo","kind":"undo"}))["document"].clone()
}

fn redo(engine: &mut CoreEngine) -> Value {
    request(engine, json!({"id":"redo","kind":"redo"}))["document"].clone()
}

/// What the Case workspace reads for one physical half: its effective configuration.
fn effective(document: &Value, instance: &str) -> Value {
    let typed: ProjectDoc = serde_json::from_value(document.clone()).unwrap();
    let inputs = effective_inputs(
        &typed,
        &[],
        &MechanicalTarget::Instance {
            instance_id: instance.into(),
        },
    )
    .unwrap();
    serde_json::to_value(inputs.configuration).unwrap()
}

fn opening<'a>(configuration: &'a Value, id: &str) -> Option<&'a Value> {
    configuration["openings"]
        .as_array()?
        .iter()
        .find(|o| o["id"] == id)
}

fn vertex_x(opening: &Value, vertex: &str) -> f64 {
    opening["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == vertex)
        .unwrap()["point"]["x"]
        .as_f64()
        .unwrap()
}

fn pair(document: &Value) -> &Value {
    let pairs = document["hardware"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 1);
    &pairs[0]
}

fn member_state<'a>(pair: &'a Value, feature: &str, instance: &str) -> &'a str {
    pair["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == feature)
        .unwrap()["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["instanceId"] == instance)
        .unwrap()["state"]
        .as_str()
        .unwrap()
}

fn mapping_x(pair: &Value) -> f64 {
    assert_eq!(pair["mapping"]["rotation"], 0.0);
    assert_eq!(pair["mapping"]["at"]["y"], 0.0);
    pair["mapping"]["at"]["x"].as_f64().unwrap()
}

/// Switch positions and rotations of one half: `matrix/<half>-<cluster>/rXcY`.
fn keys(document: &Value, half: &str) -> Vec<(f64, f64, f64)> {
    document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            let id = p["id"].as_str().unwrap();
            let mut path = id.split('/');
            path.next() == Some("matrix")
                && path
                    .next()
                    .is_some_and(|c| c.starts_with(&format!("{half}-")))
                && path.next().is_some()
                && path.next().is_none()
        })
        .map(|p| {
            (
                p["pose"]["at"]["x"].as_f64().unwrap(),
                p["pose"]["at"]["y"].as_f64().unwrap(),
                p["pose"]["rotation"].as_f64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn every_split_keyboard_opens_as_an_explicit_mirror_pair_that_reflects_its_keys() {
    for (name, project) in projects() {
        if name == "reused-pcb" {
            continue;
        }
        let pair = pair(project);
        assert_eq!(pair["id"], "mirror-left-right", "{name}");
        assert_eq!(pair["members"], json!(["left", "right"]), "{name}");
        for instance in project["hardware"]["instances"].as_array().unwrap() {
            assert_eq!(instance["constructionOwnership"], "pair", "{name}");
            assert_eq!(
                instance["flipped"], false,
                "{name}: halves are separate boards"
            );
        }
        let offset = mapping_x(pair);
        // The pair map is the measured layout's own reflection: every left key lands
        // on a right key with the mirrored orientation. KLOR and Lily58 are authored
        // from their right half, and the same map holds.
        let (left, right) = (keys(project, "left"), keys(project, "right"));
        assert!(!left.is_empty(), "{name}");
        assert_eq!(left.len(), right.len(), "{name}");
        for (x, y, rotation) in &left {
            let matched = right.iter().any(|(rx, ry, rr)| {
                (offset - x - rx).abs() < 1e-6
                    && (y - ry).abs() < 1e-6
                    && ((-rotation - rr).rem_euclid(360.0) < 1e-6
                        || (-rotation - rr).rem_euclid(360.0) > 360.0 - 1e-6)
            });
            assert!(
                matched,
                "{name}: left key at ({x}, {y}) has no mirrored key"
            );
        }
        // Opening it again changes nothing: links are never inferred or normalized.
        let (_, reopened) = reopen(project);
        assert_eq!(reopened["hardware"], project["hardware"], "{name}");
    }
}

#[test]
fn deliberate_asymmetry_is_named_per_keyboard() {
    for id in MEASURED_SPLITS {
        let pair = pair(&projects()[id]);
        for feature in ["opening-1", "opening-2"] {
            assert_eq!(member_state(pair, feature, "left"), "linked", "{id}");
            assert_eq!(
                member_state(pair, feature, "right"),
                "local",
                "{id}: the generated bay is not the left bay's reflection"
            );
        }
    }
    for variant in SOFLE {
        let pair = pair(&projects()[&format!("sofle-{variant}")]);
        for feature in ["opening-1", "opening-2"] {
            for half in ["left", "right"] {
                assert_eq!(
                    member_state(pair, feature, half),
                    "linked",
                    "sofle-{variant}"
                );
            }
        }
    }
    // The Sofle v2 gasket mount is the pair's shared construction, not one half's.
    let v2 = &projects()["sofle-v2"];
    assert_eq!(pair(v2)["construction"]["mount"], "gasket");
    for half in ["left", "right"] {
        assert_eq!(effective(v2, half)["mount"], "gasket", "{half}");
    }
    assert!(v2.get("mechanical").is_none_or(Value::is_null));
}

#[test]
fn a_paired_value_edit_changes_both_halves_in_one_undo_step() {
    let (mut engine, document) = reopen(&projects()["corne"]);
    assert_eq!(effective(&document, "left")["plateThickness"], 1.5);
    assert_eq!(effective(&document, "right")["plateThickness"], 1.5);
    // Either half can author the shared value.
    let edited = change(
        &mut engine,
        &document,
        "right",
        json!({"set-dimension":{"field":"plate-thickness","value":2.0}}),
    );
    assert_eq!(
        edited["revision"].as_u64(),
        Some(document["revision"].as_u64().unwrap() + 1)
    );
    for half in ["left", "right"] {
        assert_eq!(effective(&edited, half)["plateThickness"], 2.0, "{half}");
    }
    let undone = undo(&mut engine);
    for half in ["left", "right"] {
        assert_eq!(effective(&undone, half)["plateThickness"], 1.5, "{half}");
    }
    let redone = redo(&mut engine);
    for half in ["left", "right"] {
        assert_eq!(effective(&redone, half)["plateThickness"], 2.0, "{half}");
    }
    assert_eq!(redone["hardware"], edited["hardware"]);
}

#[test]
fn a_linked_opening_follows_either_half_through_the_pair_map() {
    let sofle = &projects()["sofle-rgb"];
    let offset = mapping_x(pair(sofle));
    let (mut engine, document) = reopen(sofle);
    let moved_from = |document: &Value, half: &str| {
        vertex_x(
            opening(&effective(document, half), "opening-1").unwrap(),
            "vertex-1",
        )
    };
    let before = moved_from(&document, "right");
    // Edit from the right half; the left half, which authored the opening, reflects it.
    let edited = change(
        &mut engine,
        &document,
        "right",
        json!({"set-opening-dimension":{"opening_id":"opening-1","vertex_id":"vertex-1","field":"opening-point-x","value":before + 1.5}}),
    );
    assert!((moved_from(&edited, "right") - (before + 1.5)).abs() < 1e-9);
    assert!(
        (moved_from(&edited, "left") - (offset - (before + 1.5))).abs() < 1e-9,
        "the left half shows the reflected value"
    );
    // The second opening is untouched.
    assert_eq!(
        opening(&effective(&edited, "left"), "opening-2"),
        opening(&effective(&document, "left"), "opening-2")
    );
    let undone = undo(&mut engine);
    assert!((moved_from(&undone, "right") - before).abs() < 1e-9);
    assert!((moved_from(&undone, "left") - (offset - before)).abs() < 1e-9);
    let redone = redo(&mut engine);
    assert!((moved_from(&redone, "left") - (offset - (before + 1.5))).abs() < 1e-9);
}

#[test]
fn a_local_exception_stays_local_while_the_rest_of_the_pair_stays_linked() {
    let corne = &projects()["corne"];
    let (mut engine, document) = reopen(corne);
    let right_bay = |document: &Value| effective(document, "right");
    let left_bay = |document: &Value| effective(document, "left");
    // Moving the right half's local bay does not move the left half's.
    let local = vertex_x(
        opening(&right_bay(&document), "opening-1").unwrap(),
        "vertex-1",
    );
    let moved = change(
        &mut engine,
        &document,
        "right",
        json!({"set-opening-dimension":{"opening_id":"opening-1","vertex_id":"vertex-1","field":"opening-point-x","value":local + 2.0}}),
    );
    assert_eq!(
        opening(&left_bay(&moved), "opening-1"),
        opening(&left_bay(&document), "opening-1")
    );
    assert!(
        (vertex_x(
            opening(&right_bay(&moved), "opening-1").unwrap(),
            "vertex-1"
        ) - (local + 2.0))
            .abs()
            < 1e-9
    );
    // And the other way: the left bay's edit does not reach the local right bay.
    let left = vertex_x(opening(&left_bay(&moved), "opening-1").unwrap(), "vertex-1");
    let both = change(
        &mut engine,
        &moved,
        "left",
        json!({"set-opening-dimension":{"opening_id":"opening-1","vertex_id":"vertex-1","field":"opening-point-x","value":left - 3.0}}),
    );
    assert_eq!(
        opening(&right_bay(&both), "opening-1"),
        opening(&right_bay(&moved), "opening-1")
    );
    // The exception is per feature: the construction is still shared.
    let thicker = change(
        &mut engine,
        &both,
        "left",
        json!({"set-dimension":{"field":"wall-thickness","value":2.5}}),
    );
    for half in ["left", "right"] {
        assert_eq!(effective(&thicker, half)["wallThickness"], 2.5, "{half}");
    }
    assert_eq!(member_state(pair(&thicker), "opening-1", "right"), "local");
    // Undo walks back one edit at a time, down to the opened project.
    for _ in 0..3 {
        undo(&mut engine);
    }
    let snapshot = request(&mut engine, json!({"id":"snapshot","kind":"snapshot"}));
    assert_eq!(snapshot["document"]["hardware"], document["hardware"]);
}

#[test]
fn a_reused_flipped_pcb_pairs_through_the_reflection_its_opposite_flips_imply() {
    let project = &projects()["reused-pcb"];
    let instances = project["hardware"]["instances"].as_array().unwrap();
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0]["boardId"], instances[1]["boardId"]);
    assert_eq!(instances[0]["flipped"], false);
    assert_eq!(instances[1]["flipped"], true);
    let shared = pair(project);
    assert_eq!(shared["members"], json!(["main", "flipped"]));
    assert_eq!(mapping_x(shared), 0.0);

    let (mut engine, document) = reopen(project);
    // The bay is one linked value shown in each copy's own frame: the flipped copy
    // mirrors X and walks the polygon the other way.
    let main = effective(&document, "main");
    let flipped = effective(&document, "flipped");
    let (main_bay, flipped_bay) = (
        opening(&main, "opening-1").unwrap(),
        opening(&flipped, "opening-1").unwrap(),
    );
    let first = main_bay["vertices"].as_array().unwrap();
    let second = flipped_bay["vertices"].as_array().unwrap();
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter().rev()) {
        assert_eq!(a["id"], b["id"]);
        assert!(
            (a["point"]["x"].as_f64().unwrap() + b["point"]["x"].as_f64().unwrap()).abs() < 1e-9
        );
        assert!(
            (a["point"]["y"].as_f64().unwrap() - b["point"]["y"].as_f64().unwrap()).abs() < 1e-9
        );
    }
    // The reset access belongs to the unflipped copy only.
    assert!(opening(&main, "opening-2").is_some());
    assert!(opening(&flipped, "opening-2").is_none());
    assert_eq!(member_state(shared, "opening-2", "flipped"), "omitted");

    // An edit from the flipped copy reaches the unflipped one, and Undo restores both.
    let before = vertex_x(main_bay, "vertex-1");
    let flipped_before = vertex_x(flipped_bay, "vertex-1");
    let edited = change(
        &mut engine,
        &document,
        "flipped",
        json!({"set-opening-dimension":{"opening_id":"opening-1","vertex_id":"vertex-1","field":"opening-point-x","value":flipped_before - 1.0}}),
    );
    let main_after = vertex_x(
        opening(&effective(&edited, "main"), "opening-1").unwrap(),
        "vertex-1",
    );
    assert!(
        (main_after - (before + 1.0)).abs() < 1e-9,
        "{main_after} vs {before}"
    );
    let undone = undo(&mut engine);
    assert!(
        (vertex_x(
            opening(&effective(&undone, "main"), "opening-1").unwrap(),
            "vertex-1"
        ) - before)
            .abs()
            < 1e-9
    );
    // Reopening a saved copy keeps the pair, its identities and the omission.
    let (_, reopened) = reopen(&edited);
    assert_eq!(reopened["hardware"], edited["hardware"]);
    assert_eq!(
        member_state(pair(&reopened), "opening-2", "flipped"),
        "omitted"
    );
}

/// Closure holes per half, `[left, right]`, as Core projects them for each shipped split.
/// A closure mount linked through the pair projects one hole on each half; a half's own
/// closure mount projects on that half only. The measured splits' left halves carry three
/// local mounts and four linked ones; their right halves get only the linked four. Sofle
/// RGB and Choc have no right-half closures, and Sofle v2 uses the internal gasket mount,
/// which carries no closure mounts. Recorded as Core gives them: see ticket
/// generator-parameters 02.
const CLOSURE_HOLES: [(&str, [usize; 2]); 10] = [
    ("corne", [7, 4]),
    ("lily58", [7, 4]),
    ("klor", [7, 4]),
    ("sweep", [7, 4]),
    ("chocofi", [7, 4]),
    ("totem", [7, 4]),
    ("cantor", [7, 4]),
    ("sofle-v2", [0, 0]),
    ("sofle-rgb", [4, 0]),
    ("sofle-choc", [4, 0]),
];

/// Core's refusal when a linked closure mount maps outside a half's outline. It is
/// pinned, not worked around: moving the mount would hide a fit conflict.
const OUTSIDE_OUTLINE: &str = "lies outside this half's board outline";

/// Each half's export, `[left, right]`: `None` exports, `Some(refusal)` is refused.
/// The left half of every measured split is refused, because a right-half closure linked
/// through the pair maps to a point beyond the left outline.
const EXPORT_REFUSALS: [(&str, [Option<&str>; 2]); 10] = [
    ("corne", [Some(OUTSIDE_OUTLINE), None]),
    ("lily58", [Some(OUTSIDE_OUTLINE), None]),
    ("klor", [Some(OUTSIDE_OUTLINE), None]),
    ("sweep", [Some(OUTSIDE_OUTLINE), None]),
    ("chocofi", [Some(OUTSIDE_OUTLINE), None]),
    ("totem", [Some(OUTSIDE_OUTLINE), None]),
    ("cantor", [Some(OUTSIDE_OUTLINE), None]),
    ("sofle-v2", [None, None]),
    ("sofle-rgb", [None, None]),
    ("sofle-choc", [None, None]),
];

/// The generated closure-hole parts of one board.
fn closure_parts<'a>(project: &'a Value, board: &str) -> Vec<&'a Value> {
    let prefix = format!("case-closure/{board}/");
    project["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["id"].as_str().unwrap().starts_with(&prefix))
        .collect()
}

/// The authored mounting holes of one board: Sofle's measured `TH` parts.
fn authored_holes<'a>(project: &'a Value, board: &str) -> Vec<&'a Value> {
    let prefix = format!("{board}/TH");
    project["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["id"].as_str().unwrap().starts_with(&prefix))
        .collect()
}

/// A hole part's drill diameter, from its own definition.
fn drill(project: &Value, part: &Value) -> f64 {
    project["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == part["definitionId"])
        .unwrap()["pads"][0]["drill"]
        .as_f64()
        .unwrap()
}

fn centre(part: &Value) -> (f64, f64) {
    (
        part["pose"]["at"]["x"].as_f64().unwrap(),
        part["pose"]["at"]["y"].as_f64().unwrap(),
    )
}

/// Clearance from a round hole to a key's 14 mm switch cutout, which turns with the key.
/// Negative means the hole cuts into the cutout.
fn cutout_clearance(hole: (f64, f64), radius: f64, key: (f64, f64, f64)) -> f64 {
    const HALF: f64 = 7.0;
    let (dx, dy) = (hole.0 - key.0, hole.1 - key.1);
    let angle = (-key.2).to_radians();
    let (lx, ly) = (
        dx * angle.cos() - dy * angle.sin(),
        dx * angle.sin() + dy * angle.cos(),
    );
    let (ox, oy) = ((lx.abs() - HALF).max(0.0), (ly.abs() - HALF).max(0.0));
    if ox > 0.0 || oy > 0.0 {
        ox.hypot(oy) - radius
    } else {
        -(HALF - lx.abs()).min(HALF - ly.abs()) - radius
    }
}

/// Exports one half's board to KiCad with that half's effective outline, as the Case
/// workspace does: the `.kicad_pcb` text, or Core's refusal message.
fn export_board(project: &Value, half: &str) -> Result<String, String> {
    let typed: ProjectDoc = serde_json::from_value(project.clone()).unwrap();
    let (_, contours) = effective_document(
        &typed,
        &MechanicalTarget::Instance {
            instance_id: half.into(),
        },
    )
    .unwrap();
    // Every bundled model the parts use, exported under the project's models folder.
    let mut model_paths = BTreeMap::new();
    for definition in typed.definitions.iter().filter(|d| d.generator.is_some()) {
        for part in typed
            .parts
            .iter()
            .filter(|p| p.definition_id == definition.id)
        {
            for id in generators::model_asset_ids(definition, Some(part)).unwrap() {
                if let Some(path) = id.strip_prefix("bundled-model:") {
                    model_paths.insert(id.clone(), format!("models/boardstudio/{path}"));
                }
            }
        }
    }
    let request = PrepareExportRequest {
        snapshot_token: "included-keyboards".into(),
        expected_revision: typed.revision,
        document: typed.clone(),
        target: ExportTarget::Board {
            board_id: half.into(),
        },
        contours,
        model_paths,
    };
    let artifact = kicad::export(request).map_err(|error| error.message)?;
    Ok(artifact
        .files
        .into_iter()
        .find(|file| file.filename.ends_with(".kicad_pcb"))
        .expect("board file")
        .content)
}

#[test]
fn every_split_half_projects_its_closure_holes_and_exports_or_is_refused_as_pinned() {
    for ((name, counts), (refusal_name, refusals)) in CLOSURE_HOLES.into_iter().zip(EXPORT_REFUSALS)
    {
        assert_eq!(name, refusal_name);
        let project = &projects()[name];
        let mut holes = [0; 2];
        for (index, half) in ["left", "right"].into_iter().enumerate() {
            holes[index] = closure_parts(project, half).len();
            match (export_board(project, half), refusals[index]) {
                (Ok(board), None) => assert_eq!(
                    board.matches("ceoloide:mounting_hole_npth").count(),
                    holes[index] + authored_holes(project, half).len(),
                    "{name} {half}: every closure hole and authored mounting hole is exported"
                ),
                (Err(message), Some(expected)) => assert!(
                    message.contains(expected),
                    "{name} {half}: refused for another reason: {message}"
                ),
                (Err(message), None) => panic!("{name} {half}: export refused: {message}"),
                (Ok(_), Some(_)) => panic!(
                    "{name} {half}: export now succeeds; update EXPORT_REFUSALS with the new record"
                ),
            }
        }
        assert_eq!(holes, counts, "{name}: closure holes per half");
        // A linked mount projects one hole per half; a local mount projects on its half.
        let linked = pair(project)["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|feature| feature["kind"] == "closure-mount")
            .count();
        let local: usize = project["hardware"]["instances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|instance| {
                instance["mechanical"]["closureMounts"]
                    .as_array()
                    .map_or(0, Vec::len)
            })
            .sum();
        assert_eq!(
            holes.iter().sum::<usize>(),
            local + 2 * linked,
            "{name}: holes = local mounts + two per linked mount"
        );
    }
}

#[test]
fn every_linked_closure_mount_lands_on_its_mirror_on_the_other_half() {
    for (name, project) in projects() {
        if name == "reused-pcb" {
            continue;
        }
        let offset = mapping_x(pair(project));
        for feature in pair(project)["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|feature| feature["kind"] == "closure-mount")
        {
            let mount = &feature["value"]["mount"]["at"];
            let (x, y) = (mount["x"].as_f64().unwrap(), mount["y"].as_f64().unwrap());
            // The map is its own inverse: the author's point maps to (offset - x, y).
            for half in ["left", "right"] {
                let expected = if feature["author"] == half {
                    (x, y)
                } else {
                    (offset - x, y)
                };
                assert!(
                    closure_parts(project, half).iter().any(|hole| {
                        let (hx, hy) = centre(hole);
                        (hx - expected.0).abs() < 1e-6 && (hy - expected.1).abs() < 1e-6
                    }),
                    "{name}: {half} has no hole at {expected:?} for linked {}",
                    feature["id"]
                );
            }
        }
    }
}

#[test]
fn closure_holes_clear_the_authored_mounting_holes_they_share_a_board_with() {
    for (name, project) in projects() {
        if name == "reused-pcb" {
            continue;
        }
        for half in ["left", "right"] {
            for hole in closure_parts(project, half) {
                for authored in authored_holes(project, half) {
                    let (hx, hy) = centre(hole);
                    let (ax, ay) = centre(authored);
                    let gap = (hx - ax).hypot(hy - ay)
                        - (drill(project, hole) + drill(project, authored)) / 2.0;
                    assert!(
                        gap > 0.0,
                        "{name} {half}: {} overlaps {} by {gap}",
                        hole["id"],
                        authored["id"]
                    );
                }
            }
        }
    }
}

#[test]
fn no_closure_hole_cuts_a_switch_cutout_on_its_half() {
    for (name, project) in projects() {
        if name == "reused-pcb" {
            continue;
        }
        for half in ["left", "right"] {
            let switches = keys(project, half);
            for hole in closure_parts(project, half) {
                let radius = drill(project, hole) / 2.0;
                for key in &switches {
                    let gap = cutout_clearance(centre(hole), radius, *key);
                    assert!(
                        gap > 0.0,
                        "{name} {half}: {} cuts a switch cutout by {}",
                        hole["id"],
                        -gap
                    );
                }
            }
        }
    }
}
