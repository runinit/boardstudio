//! Small authored source half for mirrored-half operation contracts.
use super::CURRENT_VERSION;
use serde_json::{Value, json};

pub(crate) fn document() -> Value {
    let switch = "generator:ceoloide/switch_mx";
    let diode = "generator:ceoloide/diode_tht_sod123";
    let mut document = json!({
        "format": "boardstudio/v2", "formatVersion": CURRENT_VERSION,
        "id": "synthetic-mirror", "name": "Synthetic mirror contract", "revision": 0,
        "parameters": {},
        "definitions": [
            { "id": switch, "name": "Synthetic switch", "kind": "switch",
              "keycap": { "x": 18, "y": 18 }, "courtyard": [
                {"x":-9,"y":-9},{"x":9,"y":-9},{"x":9,"y":9},{"x":-9,"y":9}], "pads": [],
              "terminals": {"row":["pad-1"],"column":["pad-2"]},
              "matrixTerminals": {"row":"row","column":"column"},
              "generator": {"source":"ceoloide/switch_mx","version":"bundled-1","parameters":{"hotswap":true,"solder":false}} },
            { "id": diode, "name": "Synthetic diode", "kind": "passive", "courtyard": [
                {"x":-2,"y":-1},{"x":2,"y":-1},{"x":2,"y":1},{"x":-2,"y":1}], "pads": [],
              "generator": {"source":"ceoloide/diode_tht_sod123","version":"bundled-1","parameters":{"side":"B"}} },
            { "id": "synthetic/led", "name": "Synthetic indicator", "kind": "passive", "courtyard": [
              {"x":-1.5,"y":-1.5},{"x":1.5,"y":-1.5},{"x":1.5,"y":1.5},{"x":-1.5,"y":1.5}], "pads": [] },
            { "id": "synthetic/controller", "name": "Synthetic controller", "kind": "controller",
              "courtyard": [{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}], "pads": [] },
            { "id": "synthetic/reset", "name": "Synthetic reset", "kind": "switch",
              "courtyard": [{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}], "pads": [] },
            { "id": "synthetic/connector", "name": "Synthetic connector", "kind": "connector",
              "courtyard": [{"x":-4,"y":-2},{"x":4,"y":-2},{"x":4,"y":2},{"x":-4,"y":2}], "pads": [] }
        ],
        "parts": [], "matrices": [], "layouts": [], "nets": [], "outline": [],
        "boards": [], "caseBodies": [], "materials": [], "assets": [], "scripts": [], "constraints": [],
        "hardware": {"topology":"split","transport":"wired","instances":[
            {"id":"left","name":"Synthetic left","boardId":"left","half":"left","role":"central","flipped":false,"controllerPartId":"left/U1"}
        ],"boards":[{"boardId":"left","controllerPartId":"left/U1","mode":"matrix"}],"pairs":[]}
    });
    document["hardware"]["instances"][0]["mechanical"] = json!({
        "boardId":"left", "method":"printed", "mount":"rigid", "integratedPlateFrame":false,
        "bottomStyle":"shell", "plateThickness":1.5, "plateFoamThickness":0,
        "pcbThickness":1.6, "bottomFoamThickness":0, "batteryHeight":0, "bottomThickness":2,
        "plateToPcb":3.5, "wallThickness":2, "clearance":0.2, "profiles":[], "mounts":[],
        "closureMounts":[], "openings":[]
    });
    document["outline"] = json!([{
        "kind":"part-envelope", "id":"left/synthetic-envelope",
        "settings":{"corners":"fillet","size":2,"bridgeWidth":10},
        "partIds":["left/U1","left/RST","left/TRRS"], "margin":4, "operation":"add"
    }, {
        "kind":"rect", "id":"synthetic-boundary", "center":{"x":0,"y":0},
        "size":{"x":100,"y":80}, "radius":0, "operation":"add"
    }]);

    let mut part_ids = Vec::new();
    let mut cells = Vec::new();
    let mut nets = Vec::new();
    for row in 0..2 {
        for column in 0..2 {
            let id = format!("matrix/left-synthetic-keys/r{row}c{column}");
            part_ids.push(id.clone());
            let x = f64::from(column) * 19.0;
            let y = f64::from(row) * 21.0;
            let rotation = 0.0;
            document["parts"].as_array_mut().unwrap().push(
                json!({"id":id,"definitionId":switch,"reference":format!("SW{}{}",row,column),
                   "pose":{"at":{"x":x,"y":y},"rotation":rotation},"side":"front"}),
            );
            cells.push(
                json!({"row":row,"column":column,"enabled":true,"definitionId":switch,
            "assembliesLocal":true,"assemblies":[{"id":"diode","definitionId":diode,
            "offset":{"x":8,"y":-2},"rotation":90,"side":"back"},
            {"id":"led","definitionId":"synthetic/led","offset":{"x":-3,"y":4},"rotation":0,"side":"front"}]}),
            );
            nets.push(json!({"id":format!("synthetic/column/{row}/{column}"),
                "name":format!("Synthetic column {row}/{column}"),"pins":[]}));
        }
    }
    for (id, definition, reference, x, y) in [
        ("left/U1", "synthetic/controller", "U1", -24.0, 0.0),
        ("left/RST", "synthetic/reset", "RST", -24.0, 16.0),
        ("left/TRRS", "synthetic/connector", "J1", -24.0, -16.0),
    ] {
        part_ids.push(id.into());
        document["parts"].as_array_mut().unwrap().push(
            json!({"id":id,"definitionId":definition,"reference":reference,
            "pose":{"at":{"x":x,"y":y},"rotation":0},"side":"front"}),
        );
    }
    document["matrices"] = json!([{"id":"left-synthetic-keys","name":"Synthetic keys","rows":2,"columns":2,
        "pitch":{"x":19,"y":21},"origin":{"x":0,"y":0},"definitionId":switch,"boardId":"left",
        "partIds":["matrix/left-synthetic-keys/r0c0","matrix/left-synthetic-keys/r0c1","matrix/left-synthetic-keys/r1c0","matrix/left-synthetic-keys/r1c1"],"cells":cells}]);
    document["layouts"] = json!([{"id":"left-synthetic-keys-layout","name":"Synthetic keys","boardId":"left",
        "matrixIds": ["left-synthetic-keys"],"partIds":[]}]);
    let net_ids: Vec<Value> = nets.iter().map(|net| net["id"].clone()).collect();
    document["nets"] = Value::Array(nets);
    document["boards"] = json!([{"id":"left","name":"Synthetic left","partIds":part_ids,
        "outlineIds":["synthetic-boundary","left/synthetic-envelope"],"netIds":net_ids,"thickness":1.6}]);
    document
}

/// A public-wire `CreateMirroredHalf` request shared by the boundary and performance harnesses.
pub(crate) fn create_mirrored_half_operation(pair_id: &str, mirror_board_id: &str) -> Value {
    json!({
        "kind":"create-mirrored-half",
        "pairId":pair_id,
        "sourceInstanceId":"left",
        "mode":{"kind":"new-board","boardId":mirror_board_id,"instanceId":mirror_board_id},
        "mapping":{"at":{"x":200,"y":0},"rotation":0},
        "idRule":"board-prefix",
        "companionMode":"literal-mirror",
        "mountingHole":{
            "id":"generator:ceoloide/mounting_hole_npth",
            "name":"mounting hole npth",
            "kind":"custom",
            "pads":[{"id":"pad-0","number":"","at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2},"shape":"circle","drill":2.2,
                "plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},
                {"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth","version":"bundled-1","parameters":{}}
        }
    })
}

/// Adds small, explicitly authored construction features for projection contracts.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn with_construction_features(mut document: Value) -> Value {
    document["hardware"]["instances"][0]["mechanical"]["closureMounts"] = json!([{
        "id": "synthetic-center-hole", "at": {"x": 3.0, "y": 5.0},
        "kind": "hole", "holeDiameter": 2.2
    }]);
    document["hardware"]["instances"][0]["mechanical"]["openings"] = json!([{
        "id": "synthetic-battery-opening", "z": 0.0, "height": 2.0,
        "vertices": [
            {"id":"v1","point":{"x":-4.0,"y":-3.0}},
            {"id":"v2","point":{"x":4.0,"y":-3.0}},
            {"id":"v3","point":{"x":4.0,"y":3.0}},
            {"id":"v4","point":{"x":-4.0,"y":3.0}}
        ]
    }]);
    document
}
