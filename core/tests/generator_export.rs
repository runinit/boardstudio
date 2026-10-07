//! KiCad export of the built-in footprint generators through Core's single
//! preview and export requests (ported from `kicad/test/ergogen.test.ts`, which
//! drove the JavaScript worker).
use std::collections::BTreeMap;
use std::process::Command;

use boardstudio_core::artifact::kicad;
use boardstudio_core::generators;
use boardstudio_core::model::{ExportTarget, PartDefinition, PrepareExportRequest, ProjectDoc};
use serde_json::{Value, json};

fn definitions() -> Vec<PartDefinition> {
    generators::catalogue().expect("catalogue")
}

fn definition(source: &str) -> PartDefinition {
    definitions()
        .into_iter()
        .find(|entry| entry.generator.as_ref().is_some_and(|g| g.source == source))
        .unwrap_or_else(|| panic!("missing bundled generator {source}"))
}

fn part(item: &PartDefinition, id: &str, x: f64, side: &str, parameters: Value) -> Value {
    json!({
        "id": id, "definitionId": item.id, "reference": id.to_uppercase(),
        "pose": { "at": { "x": x, "y": 0 }, "rotation": 0 }, "side": side,
        "generatorParameters": parameters,
    })
}

fn project(items: &[PartDefinition], parts: &[Value], nets: &[Value]) -> ProjectDoc {
    let mut doc =
        serde_json::to_value(ProjectDoc::empty("ergogen-export", "Ergogen export")).unwrap();
    doc["definitions"] = serde_json::to_value(items).unwrap();
    doc["parts"] = Value::Array(parts.to_vec());
    doc["nets"] = Value::Array(nets.to_vec());
    doc["boards"] = json!([{
        "id": "main", "name": "main", "outlineIds": [],
        "partIds": parts.iter().map(|p| p["id"].clone()).collect::<Vec<_>>(),
        "netIds": nets.iter().map(|n| n["id"].clone()).collect::<Vec<_>>(),
        "thickness": 1.6,
    }]);
    serde_json::from_value(doc).expect("project")
}

fn request(
    doc: ProjectDoc,
    target: ExportTarget,
    paths: BTreeMap<String, String>,
) -> PrepareExportRequest {
    let contours = if matches!(target, ExportTarget::Board { .. }) {
        serde_json::from_value(json!([{
            "hole": false,
            "points": [{ "x": -30, "y": -30 }, { "x": 90, "y": -30 }, { "x": 90, "y": 30 }, { "x": -30, "y": 30 }],
        }]))
        .unwrap()
    } else {
        Vec::new()
    };
    PrepareExportRequest {
        snapshot_token: "generator-export".into(),
        expected_revision: doc.revision,
        document: doc,
        target,
        contours,
        model_paths: paths,
    }
}

fn export_board(doc: ProjectDoc, paths: BTreeMap<String, String>) -> String {
    let name = doc.boards[0].name.clone();
    let artifact = kicad::export(request(
        doc,
        ExportTarget::Board {
            board_id: "main".into(),
        },
        paths,
    ))
    .expect("export board");
    artifact
        .files
        .into_iter()
        .find(|file| file.filename == format!("{name}.kicad_pcb"))
        .expect("board file")
        .content
}

fn model_paths(items: &[(&PartDefinition, Option<&Value>)]) -> BTreeMap<String, String> {
    let mut paths = BTreeMap::new();
    for (item, part) in items {
        let part = part.map(|value| serde_json::from_value(value.clone()).unwrap());
        for id in generators::model_asset_ids(item, part.as_ref()).unwrap_or_default() {
            paths.insert(
                id.clone(),
                format!("models/{}", id.replace([':', '/'], "_")),
            );
        }
    }
    paths
}

fn kicad10() -> bool {
    Command::new("kicad-cli")
        .arg("version")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .starts_with("10.")
        })
        .unwrap_or(false)
}

/// Whether KiCad 10 parses the board; skipped (true) without KiCad 10.
fn kicad_parses(content: &str) -> bool {
    if !kicad10() {
        return true;
    }
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "boardstudio-generator-kicad-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let board = directory.join("main.kicad_pcb");
    std::fs::write(&board, content).unwrap();
    let report = directory.join("stats.json");
    let status = Command::new("kicad-cli")
        .args(["pcb", "export", "stats", "--format", "json", "--output"])
        .arg(&report)
        .arg(&board)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&directory);
    status.status.success()
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn exports_front_and_back_switches_with_instance_nets_and_models() {
    let mx = definition("ceoloide/switch_mx");
    let model = "${KIPRJMOD}/models/boardstudio/";
    let shared = |from: &str, to: &str, side: &str| {
        json!({ "from": from, "to": to, "side": side, "hotswap": true,
            "switch_3dmodel_filename": format!("{model}switch.step"),
            "hotswap_3dmodel_filename": format!("{model}socket.step"),
            "keycap_3dmodel_filename": format!("{model}keycap.step") })
    };
    let parts = [
        part(&mx, "sw1", 0.0, "front", shared("ROW0", "COL0", "F")),
        part(&mx, "sw2", 40.0, "back", shared("ROW1", "COL1", "B")),
    ];
    let nets: Vec<Value> = ["ROW0", "COL0", "ROW1", "COL1"]
        .iter()
        .map(|name| json!({ "id": format!("net-{name}"), "name": name, "pins": [] }))
        .collect();
    let paths: BTreeMap<String, String> = [
        ("bundled-model:switch.step", "models/switch.step"),
        ("bundled-model:socket.step", "models/socket.step"),
        ("bundled-model:keycap.step", "models/keycap.step"),
    ]
    .iter()
    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
    .collect();
    let board = export_board(project(&[mx], &parts, &nets), paths);
    for net in ["ROW0", "COL0", "ROW1", "COL1"] {
        assert!(board.contains(&format!("\"{net}\")")), "{net}");
    }
    assert_eq!(count(&board, "(footprint "), 2);
    assert_eq!(count(&board, "(model \"${KIPRJMOD}/models/"), 6);
    assert!(board.contains("(layer \"F.Cu\")") && board.contains("(layer \"B.Cu\")"));
    assert!(kicad_parses(&board));
}

#[test]
fn exports_document_pad_net_assignments_for_terminal_groups() {
    let connector = definition("ceoloide/battery_connector_jst_ph_2");
    let placed = part(&connector, "j1", 0.0, "front", json!({}));
    let pad = connector.pads[0].clone();
    let nets =
        [json!({ "id": "gnd", "name": "GND", "pins": [{ "partId": "j1", "padId": pad.id }] })];
    let paths = model_paths(&[(&connector, None)]);
    let board = export_board(project(&[connector], &[placed], &nets), paths);
    assert!(board.contains("(net 1 \"GND\")"));
    assert!(board.contains(&format!("(pad \"{}\"", pad.number)));
    assert!(kicad_parses(&board));
}

#[test]
fn exports_zone_text_and_route_as_board_objects() {
    let (zone, text, router) = (
        definition("ceoloide/utility_filled_zone"),
        definition("ceoloide/utility_text"),
        definition("ceoloide/utility_router"),
    );
    let parts = [
        part(
            &zone,
            "zone",
            0.0,
            "front",
            json!({ "net": "GND", "side": "F", "points": [[-20, -20], [80, -20], [80, 20], [-20, 20]] }),
        ),
        part(
            &text,
            "label",
            0.0,
            "front",
            json!({ "text": "BOARD", "reversible": true }),
        ),
        part(
            &router,
            "route",
            0.0,
            "front",
            json!({ "net": "GND", "route": "f(0,0)(2,0)v(2,2)" }),
        ),
    ];
    let board = export_board(project(&[zone, text, router], &parts, &[]), BTreeMap::new());
    assert!(board.contains("\"GND\")") && board.contains("(zone"));
    assert_eq!(count(&board, "(gr_text \"BOARD\""), 2);
    assert_eq!(count(&board, "(segment"), 2);
    assert!(board.contains("(via"));
    assert!(kicad_parses(&board));
}

#[test]
fn upgrades_legacy_vendor_arcs() {
    let item = definition("infused-kim/nice_view");
    let placed = part(&item, "part", 0.0, "front", json!({}));
    let paths = model_paths(&[(&item, Some(&placed))]);
    let board = export_board(project(&[item], &[placed], &[]), paths);
    assert!(
        board.contains("(fp_arc (start ") && board.contains("(mid "),
        "arcs are upgraded to start/mid/end"
    );
    assert!(!board.contains("(angle "), "no legacy angle arcs remain");
    assert!(kicad_parses(&board));
}

#[test]
fn every_bundled_generator_exports_a_board_kicad_parses() {
    for item in definitions() {
        let source = item.generator.as_ref().unwrap().source.clone();
        let placed = part(&item, "part", 0.0, "front", json!({}));
        let paths = model_paths(&[(&item, Some(&placed))]);
        let board = export_board(project(&[item], &[placed], &[]), paths);
        assert!(kicad_parses(&board), "{source}");
    }
}

#[test]
fn standalone_export_batches_every_generator_and_reports_skipped_utilities() {
    let items = definitions();
    assert_eq!(items.len(), 36);
    let mut doc =
        serde_json::to_value(ProjectDoc::empty("ergogen-library", "Ergogen library")).unwrap();
    doc["definitions"] = serde_json::to_value(&items).unwrap();
    let doc: ProjectDoc = serde_json::from_value(doc).unwrap();
    let pairs: Vec<(&PartDefinition, Option<&Value>)> =
        items.iter().map(|item| (item, None)).collect();
    let paths = model_paths(&pairs);
    let ids = items.iter().map(|item| item.id.clone()).collect();
    let artifact = kicad::export(request(
        doc,
        ExportTarget::StandaloneFootprints {
            definition_ids: ids,
        },
        paths,
    ))
    .expect("standalone export");
    assert!(
        artifact
            .skipped_utilities
            .contains(&"utility text".to_owned())
    );
    assert!(
        artifact
            .skipped_utilities
            .contains(&"utility router".to_owned())
    );
    assert_eq!(
        artifact.files.len() + artifact.skipped_utilities.len(),
        items.len()
    );
    assert!(
        artifact
            .files
            .iter()
            .any(|file| file.content.contains("${KIPRJMOD}/models/"))
    );
}

#[test]
fn generator_failures_reach_the_caller_with_their_text() {
    let keepout = definition("ceoloide/utility_keepout_zone");
    let placed = part(&keepout, "zone", 0.0, "front", json!({ "hatch_pitch": 3 }));
    let doc = project(&[keepout], &[placed], &[]);
    let error = kicad::export(request(
        doc,
        ExportTarget::Board {
            board_id: "main".into(),
        },
        BTreeMap::new(),
    ))
    .unwrap_err();
    assert!(
        error
            .message
            .contains("hatch_pitch must be a positive number below 2mm"),
        "{}",
        error.message
    );
}

#[test]
fn previews_rotated_generators_on_both_faces_with_models_and_copper() {
    for source in [
        "ceoloide/switch_mx",
        "ceoloide/switch_choc_v1_v2",
        "ceoloide/diode_tht_sod123",
    ] {
        for side in ["front", "back"] {
            let item = definition(source);
            let mut placed = part(
                &item,
                "part",
                12.0,
                side,
                json!({"side":if side == "front" {"F"} else {"B"}}),
            );
            placed["pose"] = json!({"at":{"x":12,"y":-8},"rotation":37});
            let paths = model_paths(&[(&item, Some(&placed))]);
            let doc = project(&[item], &[placed], &[]);
            let preview = kicad::preview(request(
                doc,
                ExportTarget::Board {
                    board_id: "main".into(),
                },
                paths,
            ))
            .unwrap();
            assert!(!preview.models.is_empty(), "{source} {side}");
            assert!(
                preview
                    .surfaces
                    .iter()
                    .any(|surface| surface.layer.ends_with(".Cu"))
            );
            let expected = if side == "front" {
                boardstudio_core::model::Side::Front
            } else {
                boardstudio_core::model::Side::Back
            };
            assert!(preview.models.iter().all(|model| model.side == expected));
            assert_eq!(preview.models[0].pose.at.y, -8.0);
            assert_eq!(preview.contours.len(), 1);
        }
    }
}
