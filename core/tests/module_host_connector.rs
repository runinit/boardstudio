use crate::support::fixture::{self, request};
use crate::support::query;
use boardstudio_core::CoreEngine;
use boardstudio_core::model::{
    Contour, ExportTarget, FinishExportRequest, PrepareExportRequest, ProjectDoc, Vec2,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const LIBRARY: &str = include_str!("../../catalogue/modules/imported-modules.json");

fn export_host_board(document: &Value, scene: &Value) -> String {
    let document: ProjectDoc = serde_json::from_value(document.clone()).unwrap();
    let contours = serde_json::from_value(scene["boardContours"][0]["contours"].clone()).unwrap();
    let mut model_paths = BTreeMap::new();
    model_paths.insert(
        "bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp".into(),
        "models/vik-connector-horizontal.step".into(),
    );
    let plan = boardstudio_core::artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "host-connector-review".into(),
        expected_revision: document.revision,
        document,
        target: ExportTarget::Board {
            board_id: "pcb".into(),
        },
        contours,
        model_paths,
    })
    .expect("prepare source-backed connector export");
    assert!(
        plan.jobs.is_empty(),
        "source-backed footprint should not need invented Ergogen jobs"
    );
    let artifact = boardstudio_core::artifact::kicad::finish_export(FinishExportRequest {
        plan,
        results: Vec::new(),
    })
    .expect("finish source-backed connector export");
    artifact
        .files
        .into_iter()
        .find(|file| file.filename.ends_with(".kicad_pcb"))
        .unwrap()
        .content
}

fn exported_footprint<'a>(board: &'a str, reference: &str) -> &'a str {
    let reference = format!("(property \"Reference\" \"{reference}\"");
    let reference_at = board
        .find(&reference)
        .expect("exported footprint reference");
    let start = board[..reference_at]
        .rfind("(footprint ")
        .expect("footprint root");
    let end = board[reference_at..]
        .find("\n  (footprint ")
        .map(|offset| reference_at + offset)
        .unwrap_or(board.len());
    &board[start..end]
}

/// An 80 mm host document is the default; `half` selects the outline half-extent.
fn host_document(half: f64) -> Value {
    let mut document = serde_json::to_value(ProjectDoc::empty("host", "Host")).unwrap();
    document["boards"] = json!([{"id":"pcb","name":"PCB","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":half * 2.0,"y":half * 2.0},"radius":0,"operation":"add"}]);
    document
}

fn module_definition(id: &str, name: &str, half_w: f64, half_h: f64) -> Value {
    json!({"id":id,"name":name,"family":"test","variant":"source",
        "source":{"repository":"https://github.com/sadekbaroudi/vik","revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553","path":"pcb/test","license":"CERN-OHL-S-2.0"},
        "board":{"contours":[{"hole":false,"points":[{"x":-half_w,"y":-half_h},{"x":half_w,"y":-half_h},{"x":half_w,"y":half_h},{"x":-half_w,"y":half_h}]}]},
        "mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[{"id":"input","role":"module","signals":["sclk","miso","cs","gpio2","mosi","gpio1","v5","rgb","scl","sda","gnd","v3v3"]}],
        "electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},"constituents":[]})
}

fn attach(
    engine: &mut CoreEngine,
    id: &str,
    target: &str,
    instance: Value,
    definition: Value,
    host: Option<Value>,
) -> Value {
    let mut operation =
        json!({"kind":"set-mounted-module","instance":instance,"definition":definition});
    if let Some(host) = host {
        operation["hostConnectorDefinition"] = host;
    }
    request(
        engine,
        json!({"id":id,"kind":"edit","command":{"baseRevision":0,"transactionId":id,"phase":"commit","targetIds":[target],
        "operation":operation}}),
    )
}

fn host_connector() -> Value {
    let library: Value = serde_json::from_str(LIBRARY).unwrap();
    library["modules"].as_array().unwrap().iter()
        .flat_map(|entry| entry["definition"]["circuit"]["definitions"].as_array().into_iter().flatten())
        .find(|definition| {
            definition["hardwareProfile"]["vikRole"] == "host"
                && definition["name"].as_str().is_some_and(|name| name.contains("horizontal"))
        })
        .map(|definition| {
            let mut definition = definition.clone();
            definition["models"] = json!([{"assetId":"bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp",
                "offset":{"x":-2.75,"y":2.3,"z":0},"rotation":{"x":0,"y":0,"z":0},"scale":{"x":1,"y":1,"z":1}}]);
            definition
        }).unwrap()
}

#[test]
fn attaching_a_module_adds_and_links_a_source_host_footprint_in_one_undoable_edit() {
    let mut document = host_document(40.0);
    document["boardOutlines"] = json!([{"boardId":"pcb","activeVersionId":"reviewed","versions":[{"id":"reviewed","name":"Reviewed board edge","source":{"revision":4,"versionId":"source-v4"},
        "geometry":{"features":[{"kind":"rect","id":"fixed-outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}],
        "settings":{"corners":"sharp","size":2,"bridgeWidth":10},"expectedRegions":1}}]}]);
    let mut engine = CoreEngine::new();
    let opened = fixture::wire_open(&mut engine, "open", &document);
    assert_eq!(opened["kind"], "scene");
    let reviewed_outline = opened["document"]["boardOutlines"][0].clone();
    let reviewed_contours = opened["scene"]["boardContours"].clone();

    let host_connector_id = "module/test/vik-host-connector";
    let definition = module_definition("module-test", "Test module", 10.0, 8.0);
    let instance = json!({"id":"module/test","definitionId":"module-test","hostBoardId":"pcb","hostFace":"front","facingFace":"back",
        "at":{"x":0,"y":0},"rotation":0,"gap":3,"attachment":"board","serviceClearance":0,
        "connection":{"hostConnectorPartId":host_connector_id,"modulePortId":"input","busId":"","assignments":{},"cableType":"type-a-12-0.5","railVoltages":{}}});
    let added = attach(
        &mut engine,
        "attach",
        "module/test",
        instance,
        definition,
        Some(host_connector()),
    );
    assert_eq!(added["kind"], "scene", "{added}");
    assert_eq!(
        added["document"]["boardOutlines"][0], reviewed_outline,
        "connector creation must preserve the authored outline snapshot"
    );
    assert_eq!(
        added["scene"]["boardContours"], reviewed_contours,
        "resolved contours must remain unchanged after connector creation"
    );
    assert_eq!(
        added["document"]["modules"][0]["connection"]["hostConnectorPartId"],
        host_connector_id
    );
    assert_eq!(
        added["document"]["boards"][0]["partIds"],
        json!([host_connector_id])
    );
    let part = added["document"]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["id"] == host_connector_id)
        .unwrap();
    assert_eq!(part["side"], "front");
    let connector_definition = added["document"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == part["definitionId"])
        .unwrap();
    assert_eq!(connector_definition["hardwareProfile"]["vikRole"], "host");
    assert_eq!(
        connector_definition["pads"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|pad| (1..=12).contains(
                &pad["number"]
                    .as_str()
                    .unwrap_or("")
                    .parse::<u8>()
                    .unwrap_or(0)
            ))
            .count(),
        12
    );
    assert!(
        added["document"]["modules"][0]["connection"]["assignments"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let pcb = export_host_board(&added["document"], &added["scene"]);
    let footprint = exported_footprint(&pcb, "J_VIK1");
    assert!(footprint.contains("(layer \"F.Cu\")"));
    assert!(
        footprint.contains("(pad \"1\" smd rect")
            && footprint.contains("(layers \"F.Cu\" \"F.Paste\" \"F.Mask\")")
    );
    assert!(footprint.contains("${KIPRJMOD}/models/vik-connector-horizontal.step"));
    assert!(!footprint.contains("../../kicad/3dmodels/vik-connector-horizontal.stp"));
    let pose = &part["pose"];
    let rotation = pose["rotation"].as_f64().unwrap().to_radians();
    let (sin, cos) = rotation.sin_cos();
    let outer_contour = added["scene"]["boardContours"][0]["contours"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contour| !contour["hole"].as_bool().unwrap())
        .unwrap()["points"]
        .as_array()
        .unwrap();
    let outer: Contour =
        serde_json::from_value(json!({"hole": false, "points": outer_contour})).unwrap();
    for point in connector_definition["courtyard"].as_array().unwrap() {
        let x = point["x"].as_f64().unwrap();
        let y = point["y"].as_f64().unwrap();
        let world = Vec2 {
            x: pose["at"]["x"].as_f64().unwrap() + x * cos - y * sin,
            y: pose["at"]["y"].as_f64().unwrap() + x * sin + y * cos,
        };
        assert!(
            query::contains(std::slice::from_ref(&outer), world.x, world.y),
            "source courtyard point {world:?} must lie within the resolved host PCB contour"
        );
    }

    let undo = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    assert_eq!(undo["kind"], "scene", "{undo}");
    assert!(
        undo["document"]
            .get("modules")
            .is_none_or(|items| items.as_array().unwrap().is_empty())
    );
    assert!(
        undo["document"]
            .get("parts")
            .is_none_or(|items| items.as_array().unwrap().is_empty())
    );
    assert!(
        undo["document"]["boards"][0]["partIds"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn reusing_a_host_connector_preserves_its_manual_pose_and_board_ownership() {
    let mut document = host_document(40.0);
    let mut host = host_connector();
    host["id"] = json!("vik:source:horizontal-host-connector");
    document["boards"] = json!([{"id":"pcb","name":"PCB","partIds":["manual-host"],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["definitions"] = json!([host]);
    document["parts"] = json!([{"id":"manual-host","definitionId":host["id"],"reference":"J9","pose":{"at":{"x":-17.5,"y":6.25},"rotation":90},"side":"back"}]);
    let mut engine = CoreEngine::new();
    fixture::wire_open(&mut engine, "open", &document);
    let module = json!({"id":"module/reuse","definitionId":"module-reuse","hostBoardId":"pcb","hostFace":"back","facingFace":"front",
        "at":{"x":10,"y":8},"rotation":15,"gap":3,"attachment":"board","serviceClearance":0,
        "connection":{"hostConnectorPartId":"manual-host","modulePortId":"input","busId":"manual-bus","assignments":{},"cableType":"type-a-12-0.5","railVoltages":{}}});
    let definition = module_definition("module-reuse", "Reuse module", 8.0, 6.0);
    let added = attach(
        &mut engine,
        "attach",
        "module/reuse",
        module,
        definition,
        None,
    );
    assert_eq!(added["kind"], "scene", "{added}");
    assert_eq!(added["document"]["parts"].as_array().unwrap().len(), 1);
    assert_eq!(
        added["document"]["parts"][0]["pose"],
        json!({"at":{"x":-17.5,"y":6.25},"rotation":90.0})
    );
    assert_eq!(added["document"]["parts"][0]["side"], "back");
    assert_eq!(
        added["document"]["boards"][0]["partIds"],
        json!(["manual-host"])
    );
    assert_eq!(
        added["document"]["modules"][0]["connection"]["hostConnectorPartId"],
        "manual-host"
    );
    let pcb = export_host_board(&added["document"], &added["scene"]);
    let footprint = exported_footprint(&pcb, "J9");
    assert!(footprint.contains("(layer \"B.Cu\")"));
    assert!(
        footprint.contains("(pad \"1\" smd rect")
            && footprint.contains("(layers \"B.Cu\" \"B.Paste\" \"B.Mask\")")
    );
}

#[test]
fn automatic_placement_rejects_a_host_connector_that_would_fall_outside_the_board() {
    let document = host_document(10.0);
    let mut engine = CoreEngine::new();
    fixture::wire_open(&mut engine, "open", &document);
    let definition = module_definition("module-tight", "Tight board module", 8.0, 6.0);
    let mut instance = json!({"id":"module/tight","definitionId":"module-tight","hostBoardId":"pcb","hostFace":"front","facingFace":"back",
        "at":{"x":0,"y":0},"rotation":0,"gap":3,"attachment":"board","serviceClearance":0,
        "connection":{"hostConnectorPartId":"module/tight/vik-host-connector","modulePortId":"input","busId":"tight","assignments":{},"cableType":"type-a-12-0.5","railVoltages":{}}});
    let rejected = attach(
        &mut engine,
        "rejected",
        "module/tight",
        instance.clone(),
        definition.clone(),
        Some(host_connector()),
    );
    assert_eq!(rejected["kind"], "error", "{rejected}");
    assert!(
        rejected["message"]
            .as_str()
            .unwrap()
            .contains("No clear nearby host-PCB position")
    );
    instance.as_object_mut().unwrap().remove("connection");
    let no_connector = attach(
        &mut engine,
        "plain",
        "module/tight",
        instance,
        definition,
        None,
    );
    assert_eq!(no_connector["kind"], "scene", "{no_connector}");
    assert!(
        no_connector["document"]["parts"]
            .as_array()
            .unwrap()
            .is_empty(),
        "the rejected edit left no partially inserted physical part"
    );
}
