//! Public acceptance flow for one mixed host/module project.
//! Module envelope values below are explicit test fixtures, not upstream hardware claims.
use boardstudio_core::{CoreEngine, artifact_request};
use serde_json::{Value, json};

const THQ_LIBRARY: &str = include_str!("../../catalogue/parts/imported-parts.json");
const VIK_LIBRARY: &str = include_str!("../../catalogue/modules/imported-modules.json");
const HAPTIC_SOURCE: &str = include_str!(
    "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb"
);

fn thq_definition(id: &str) -> Value {
    serde_json::from_str::<Value>(THQ_LIBRARY).unwrap()["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["definition"]["id"] == id)
        .unwrap()["definition"]
        .clone()
}

fn vik_definition(row: &str) -> Value {
    serde_json::from_str::<Value>(VIK_LIBRARY).unwrap()["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["row"] == row)
        .unwrap()["definition"]
        .clone()
}

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn base_document() -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "mixed-acceptance",
        "Mixed project acceptance",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"host","name":"Host PCB","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":110,"y":80},"radius":0,"operation":"add"}]);
    let rotation = thq_definition("thqwgd001:rotation-reversible");
    document["definitions"] = json!([
        {"id":"host-switch","name":"Host MX switch","kind":"switch","courtyard":[{"x":-9,"y":-9},{"x":9,"y":-9},{"x":9,"y":9},{"x":-9,"y":9}],"pads":[]},
        rotation
    ]);
    document["matrices"] = json!([{
        "id":"matrix","name":"Switch matrix","boardId":"host","rows":1,"columns":2,
        "pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"host-switch","partIds":[],
        "cells":[{"row":0,"column":0,"enabled":true},{"row":0,"column":1,"enabled":true}]
    }]);
    document["mechanical"] = json!({"boardId":"host","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
        "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
        "plateToPcb":3.5,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});

    // Explicit synthetic design fixtures exercise the transform and case paths without
    // qualifying the pinned VIK splitter assembly, whose upstream envelope remains unknown.
    let fixture = |id: &str| {
        json!({
            "id":id,"name":"Illustrative daughterboard fixture","family":"test-fixture","variant":"designer-entered profile",
            "source":{"repository":"https://example.invalid/boardstudio-test-fixture","revision":"fixture-v1","path":id,"license":"CC0-1.0"},
            "board":{"thickness":1.6,"contours":[{"hole":false,"points":[{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}]}]},
            "mounts":[],"volumes":[{"id":"body","purpose":"occupied","source":"test-only designer input","qualified":true,
              "geometry":{"points":[{"x":-4,"y":-3},{"x":4,"y":-3},{"x":4,"y":3},{"x":-4,"y":3}],"z":0.8,"height":2.0}}],
            "openings":[],"models":[],"candidateModels":[],"gates":[],"interfaces":[],
            "electrical":{"protocol":"pass-through","requiredSignals":[]},"constituents":[{"reference":"TEST1","name":"fixture-only part","footprint":"test:fixture"}]
        })
    };
    let above = fixture("fixture-above");
    let below = fixture("fixture-below");
    let mut rotary = vik_definition("ec11-evqwgd001");
    rotary["electrical"]["rotaryProfile"]["steps"] = json!(24);
    rotary["electrical"]["rotaryProfile"]["triggersPerRotation"] = json!(4);
    document["moduleDefinitions"] = json!([above, below, rotary]);
    document["modules"] = json!([
        {"id":"fixture/above","definitionId":"fixture-above","hostBoardId":"host","hostFace":"front","facingFace":"front","at":{"x":30,"y":20},"rotation":0,"gap":3,"attachment":"board","detached":false,"serviceClearance":0},
        {"id":"fixture/below","definitionId":"fixture-below","hostBoardId":"host","hostFace":"back","facingFace":"front","at":{"x":65,"y":20},"rotation":180,"gap":3,"attachment":"board","detached":false,"serviceClearance":0},
        {"id":"module/encoder","definitionId":rotary["id"],"hostBoardId":"host","hostFace":"front","facingFace":"back","at":{"x":96,"y":20},"rotation":0,"gap":3,"attachment":"board","detached":false,"serviceClearance":0}
    ]);
    document
}

fn import_haptic() -> Value {
    let source = HAPTIC_SOURCE;
    let provenance = json!({"repository":"https://github.com/sadekbaroudi/vik","revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553",
        "path":"pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb","license":"CERN-OHL-S-2.0"});
    let reply: Value = serde_json::from_str(&artifact_request(&json!({
        "id":"haptic-import","kind":"import-module-board","definitionId":"vik:haptic-fixture","name":"DRV2605L fixture",
        "source":source,"provenance":provenance,"family":"feedback","variant":"source"
    }).to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    reply["result"].clone()
}

#[test]
fn mixed_host_project_keeps_module_ownership_and_public_export_gates_together() {
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":base_document()}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let mut replacement = opened["document"]["matrices"][0].clone();
    let tactile = thq_definition("thqwgd001:c-2pin-reversible");
    replacement["cells"][1]["definitionId"] = json!(tactile["id"]);
    replacement["cells"][1]["variant"] = json!(tactile["id"]);
    let replaced = request(
        &mut engine,
        json!({"id":"matrix-replace","kind":"edit","command":{
            "baseRevision":opened["document"]["revision"],"transactionId":"mixed/replace-matrix","phase":"commit","targetIds":["matrix"],
            "operation":{"kind":"set-matrix","matrix":replacement,"definitions":[tactile]}
        }}),
    );
    assert_eq!(replaced["kind"], "scene", "{replaced}");
    assert!(
        replaced["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["definitionId"] == "thqwgd001:c-2pin-reversible")
    );
    assert_eq!(replaced["document"]["modules"].as_array().unwrap().len(), 3);

    let haptic = import_haptic();
    let embedded = request(
        &mut engine,
        json!({"id":"embed-haptic","kind":"edit","command":{
            "baseRevision":replaced["document"]["revision"],"transactionId":"mixed/embed-haptic","phase":"commit","targetIds":["embedded/haptic"],
            "operation":{"kind":"embed-module-circuit","id":"embedded/haptic","definition":haptic,"hostBoardId":"host","pose":{"at":{"x":10,"y":30},"rotation":0},"side":"front","joins":{}}
        }}),
    );
    assert_eq!(embedded["kind"], "scene", "{embedded}");
    let document = &embedded["document"];
    let circuit = &document["embeddedCircuits"][0];
    assert!(!circuit["partIds"].as_array().unwrap().is_empty());
    assert!(document["parts"].as_array().unwrap().iter().any(|part| {
        circuit["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == &part["id"])
    }));
    assert!(
        document["moduleDefinitions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|definition| definition["id"] == "vik:haptic-fixture")
    );

    let resolved = request(
        &mut engine,
        json!({"id":"modules","kind":"resolve-modules","document":document,"boardId":"host","previewTopZ":1.6}),
    );
    assert_eq!(resolved["kind"], "modules-resolved", "{resolved}");
    let layers = resolved["result"]["modules"].as_array().unwrap();
    assert!(
        layers
            .iter()
            .any(|module| module["id"] == "fixture/above" && module["flipped"] == true)
    );
    assert!(
        layers
            .iter()
            .any(|module| module["id"] == "fixture/below" && module["flipped"] == false)
    );

    let contours = json!([{"hole":false,"points":[{"x":-55,"y":-40},{"x":55,"y":-40},{"x":55,"y":40},{"x":-55,"y":40}]}]);
    let mechanical = request(
        &mut engine,
        json!({"id":"case","kind":"resolve-mechanical","document":document,"contours":contours}),
    );
    assert_eq!(mechanical["kind"], "mechanical-resolved", "{mechanical}");
    assert_eq!(mechanical["assembly"]["generationBlocked"], true);
    assert!(
        mechanical["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"] == "module/fixture/above/case-material/plate"
                    && finding["targetIds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|id| id == "fixture/above")
            }),
        "{}",
        mechanical["assembly"]["diagnostics"]
    );
    assert!(
        mechanical["assembly"]["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|module| module["id"] == "fixture/above")
    );
    assert!(
        mechanical["assembly"]["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|module| module["id"] == "fixture/below")
    );
    let case = &mechanical["assembly"]["case"];
    let prepared = request(
        &mut engine,
        json!({"id":"case-prepare","kind":"prepare-case","ir":case}),
    );
    assert_eq!(
        prepared["kind"], "case-prepared",
        "{}",
        mechanical["assembly"]["diagnostics"]
    );
    assert!(prepared["ir"]["bodies"].as_array().unwrap().len() >= 2);

    let firmware = json!({"kind":"generate-firmware","id":"mixed-firmware","request":{
        "controller_profile":"ceoloide/mcu_nice_nano","board_name":"host","mode":"matrix",
        "rows":[{"terminal":"P21","gpio":"P0.31"}],"columns":[{"terminal":"P20","gpio":"P0.29"}],
        "keys":[{"id":"matrix/matrix/r0c0","row":0,"column":0},{"id":"matrix/matrix/r0c1","row":0,"column":1}],
        "diode_direction":"col2row","encoders":[{"id":"module/encoder","profile":document["moduleDefinitions"][2]["electrical"]["rotaryProfile"],"aGpio":"P0.02","bGpio":"P1.15"}],
        "encoder_ids":["module/encoder"],"keymap":{"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{"module/encoder":{"clockwise":{"kind":"key-press","keycode":"RIGHT"},"counterclockwise":{"kind":"key-press","keycode":"LEFT"}}}}],"macros":[]},
        "hardware":{"boardId":"host","modules":[
            {"id":"fixture/above","name":"Illustrative above","protocol":"pass-through","gates":[]},
            {"id":"fixture/below","name":"Illustrative below","protocol":"pass-through","gates":[]},
            {"id":"module/encoder","name":"EC11 VIK","catalogueRow":"ec11-evqwgd001","source":document["moduleDefinitions"][2]["source"],"protocol":"gpio","rotaryProfile":document["moduleDefinitions"][2]["electrical"]["rotaryProfile"],"gates":[]}
        ],"physicalInstances":[],"moduleFindings":[],"embeddedCircuitIds":["embedded/haptic"]}
    }});
    let firmware = request(&mut engine, firmware);
    assert_eq!(firmware["kind"], "error", "{}", firmware);
    assert!(
        firmware["message"]
            .as_str()
            .unwrap()
            .contains("Embedded module circuits")
    );

    // Host preview captures host-owned and explicitly embedded footprints;
    // above/below module BOMs remain separate while fabrication stays gated.
    let export_request = json!({
        "snapshotToken":"mixed-host","expectedRevision":document["revision"],"document":document,
        "target":{"kind":"board","boardId":"host"},"contours":contours,
        "modelPaths":{"ergogen:model:thqwgd001/THQWGD001C [2pin] #1.stp":"models/thq-tactile.step"}
    });
    let fabrication: Value = serde_json::from_str(&artifact_request(
        &json!({"id":"host-fabrication","kind":"export-pcb","request":export_request})
            .to_string(),
    ))
    .unwrap();
    assert_eq!(fabrication["kind"], "error", "{fabrication}");
    assert!(
        fabrication["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Physical encoder part identity")
    );
    // Preview planning captures only host-owned and embedded footprints.
    let plan = serde_json::to_value(
        boardstudio_core::artifact::kicad::prepare_preview(
            serde_json::from_value(export_request.clone()).unwrap(),
        )
        .expect("preview plan"),
    )
    .unwrap();
    assert_eq!(plan["target"]["boardId"], "host");
    assert!(
        plan["capturedDocument"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|part| part["id"].as_str().unwrap().starts_with("embedded/")
                || part["id"].as_str().unwrap().starts_with("matrix/"))
    );
    assert!(
        plan["capturedDocument"]["moduleDefinitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|definition| definition["id"] == "fixture-above")
            .unwrap()["constituents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["reference"] == "TEST1")
    );
    let preview: Value = serde_json::from_str(&artifact_request(
        &json!({"id":"host-preview-finish","kind":"preview-pcb","request":export_request}).to_string(),
    )).unwrap();
    assert_eq!(preview["kind"], "preview-board", "{preview}");
    assert!(preview["result"]["diagnostics"].as_array().is_some());

    let project_json = serde_json::to_string(document).unwrap();
    let (packed, buffers) = boardstudio_core::archive::request(
        &json!({"kind":"pack-project","projectJson":project_json,"assets":[]}).to_string(),
        &[],
    );
    let packed: Value = serde_json::from_str(&packed).unwrap();
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked, _) =
        boardstudio_core::archive::request(&json!({"kind":"unpack-project"}).to_string(), &buffers);
    let unpacked: Value = serde_json::from_str(&unpacked).unwrap();
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    let roundtrip: Value = serde_json::from_str(unpacked["projectJson"].as_str().unwrap()).unwrap();
    assert_eq!(roundtrip["embeddedCircuits"], document["embeddedCircuits"]);
    assert_eq!(roundtrip["modules"], document["modules"]);
    assert_eq!(roundtrip["matrices"], document["matrices"]);
}
