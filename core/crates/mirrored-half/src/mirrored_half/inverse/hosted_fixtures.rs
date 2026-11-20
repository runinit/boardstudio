use super::*;

pub(super) const AXIS: Pose2 = Pose2 {
    at: Vec2 { x: 200.0, y: 0.0 },
    rotation: 0.0,
};

pub(super) const ROTATED: Pose2 = Pose2 {
    at: Vec2 { x: 100.0, y: -30.0 },
    rotation: 90.0,
};

pub(super) fn part(
    id: &str,
    definition: &str,
    reference: &str,
    x: f64,
    y: f64,
    rotation: f64,
) -> Value {
    json!({"id": id, "definitionId": definition, "reference": reference,
        "pose": {"at": {"x": x, "y": y}, "rotation": rotation}, "side": "front"})
}

pub(super) fn corr(kind: &str, source: &str, mirror: &str) -> Value {
    json!({"kind": kind, "sourceId": source, "mirrorId": mirror})
}

pub(super) fn module(
    id: &str,
    board: &str,
    instance: &str,
    x: f64,
    rotation: f64,
    connection: Value,
) -> Value {
    json!({"id": id, "definitionId": "module-definition", "hostBoardId": board,
        "hostInstanceId": instance, "hostFace": "front", "facingFace": "back",
        "at": {"x": x, "y": 3}, "rotation": rotation, "gap": 0.4, "attachment": "board",
        "detached": false, "serviceClearance": 0, "connection": connection})
}

pub(super) fn connection(connector: &str, bus: &str, upstream: Option<&str>) -> Value {
    json!({"hostConnectorPartId": connector, "modulePortId": "host", "busId": bus,
        "assignments": {}, "cableType": "cable", "railVoltages": {},
        "upstreamModuleId": upstream})
}

pub(super) fn pin(part: &str, pad: &str) -> Value {
    json!({"partId": part, "padId": pad})
}

/// Board `left` is the source and `right` its mirror. Each half has a switch, a connector, a
/// module (the right one plugged into the right connector, on the same bus spelled for its
/// side), an upstream module, an embedded circuit with a private and a joined host net, and an
/// electrical configuration. `right/FREE` is a mirror-only part on the joined net, and
/// `left/SRC-ONLY` is a source-only part on it.
pub(super) fn fixture() -> ProjectDoc {
    let nets = json!([
        {"id": "net-left", "name": "SIGNAL", "pins": [
            pin("embedded/left-circuit/U1", "pad-1"), pin("left/CONN", "pad-two"),
            pin("left/SRC-ONLY", "pad-s")]},
        {"id": "net-right", "name": "SIGNAL", "pins": [
            pin("embedded/right-circuit/U1", "pad-1"), pin("right/CONN", "pad-two"),
            pin("right/FREE", "pad-f")]},
        {"id": "embedded/left-circuit/N$1", "name": "PRIVATE",
         "pins": [pin("embedded/left-circuit/U1", "pad-2")]},
        {"id": "embedded/right-circuit/N$1", "name": "PRIVATE",
         "pins": [pin("embedded/right-circuit/U1", "pad-2")]}
    ]);
    let boards = json!([
        {"id": "left", "name": "left", "outlineIds": [], "thickness": 1.6,
         "netIds": ["net-left", "embedded/left-circuit/N$1"],
         "partIds": ["matrix/left-keys/r0c0", "left/CONN", "left/SRC-ONLY",
                     "embedded/left-circuit/U1"]},
        {"id": "right", "name": "right", "outlineIds": [], "thickness": 1.6,
         "netIds": ["net-right", "embedded/right-circuit/N$1"],
         "partIds": ["matrix/right-keys/r0c0", "right/CONN", "right/FREE",
                     "embedded/right-circuit/U1"]}
    ]);
    let parts = json!([
        part("matrix/left-keys/r0c0", "sw", "SW1", 0.0, 0.0, 0.0),
        part("matrix/right-keys/r0c0", "sw", "RSW1", 200.0, 0.0, 0.0),
        part("left/CONN", "conn", "J1", -24.0, 30.0, 0.0),
        part("right/CONN", "conn", "RJ1", 224.0, 30.0, 180.0),
        part("left/SRC-ONLY", "u", "S1", -10.0, 0.0, 0.0),
        part("right/FREE", "u", "RF1", 300.0, 0.0, 0.0),
        part("embedded/left-circuit/U1", "cu", "U1", 10.0, 5.0, 30.0),
        part("embedded/right-circuit/U1", "cu", "RU1", 190.0, 5.0, 150.0)
    ]);
    let matrices = json!([
        {"id": "left-keys", "name": "Left keys", "rows": 1, "columns": 1,
         "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0}, "definitionId": "sw",
         "boardId": "left", "cells": [{"row": 0, "column": 0, "enabled": true}],
         "partIds": ["matrix/left-keys/r0c0"]},
        {"id": "right-keys", "name": "Right keys", "rows": 1, "columns": 1,
         "pitch": {"x": 19, "y": 19}, "origin": {"x": 200, "y": 0}, "definitionId": "sw",
         "boardId": "right", "mirror": "x", "cells": [{"row": 0, "column": 0, "enabled": true}],
         "partIds": ["matrix/right-keys/r0c0"]}
    ]);
    let module_definitions = json!([{
            "id": "module-definition", "name": "Module", "family": "fixture", "variant": "v",
            "source": {"repository": "https://example.invalid/m", "revision": "r",
                       "path": "m.kicad_pcb", "license": "MIT"},
            "board": {"contours": []}, "mounts": [], "volumes": [], "openings": [], "models": [],
            "gates": [], "interfaces": [],
            "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
            "constituents": []}]);
    let modules = json!([
        module(
            "left-module",
            "left",
            "inst-left",
            4.0,
            12.0,
            connection("left/CONN", "left/bus", Some("left-up"))
        ),
        module(
            "right-module",
            "right",
            "inst-right",
            196.0,
            168.0,
            connection("right/CONN", "right/bus", Some("right-up"))
        ),
        module("left-up", "left", "inst-left", 50.0, 0.0, Value::Null),
        module("right-up", "right", "inst-right", 150.0, 180.0, Value::Null)
    ]);
    let circuits = json!([
        {"id": "left-circuit", "definitionId": "module-definition", "hostBoardId": "left",
         "partIds": ["embedded/left-circuit/U1"],
         "netIds": ["embedded/left-circuit/N$1", "net-left"],
         "ports": {"signal": "net-left"}},
        {"id": "right-circuit", "definitionId": "module-definition", "hostBoardId": "right",
         "partIds": ["embedded/right-circuit/U1"],
         "netIds": ["embedded/right-circuit/N$1", "net-right"],
         "ports": {"signal": "net-right"}}
    ]);
    let hardware = json!({"topology": "split", "transport": "wired",
    "instances": [
        {"id": "inst-left", "name": "Left", "boardId": "left", "half": "left",
         "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null},
        {"id": "inst-right", "name": "Right", "boardId": "right", "half": "right",
         "role": "peripheral", "flipped": false, "controllerPartId": null,
         "mechanical": null}],
    "boards": [{"boardId": "left", "mode": "matrix"}, {"boardId": "right", "mode": "matrix"}],
    "pairs": [{
        "id": "pair", "members": ["left", "right"],
        "mapping": {"at": {"x": 200, "y": 0}, "rotation": 0},
        "construction": {"method": "printed", "mount": "rigid",
            "integratedPlateFrame": false, "plateThickness": 1.5,
            "plateFoamThickness": 0, "bottomFoamThickness": 0, "bottomThickness": 2,
            "plateToPcb": 3.5, "wallThickness": 2, "clearance": 0.2},
        "halves": {
            "sourceBoardId": "left", "mirrorBoardId": "right",
            "correspondences": [
                corr("matrix", "left-keys", "right-keys"),
                corr("part", "matrix/left-keys/r0c0", "matrix/right-keys/r0c0"),
                corr("part", "left/CONN", "right/CONN"),
                corr("part", "embedded/left-circuit/U1", "embedded/right-circuit/U1"),
                corr("module", "left-module", "right-module"),
                corr("module", "left-up", "right-up"),
                corr("embedded-circuit", "left-circuit", "right-circuit"),
                corr("net", "net-left", "net-right"),
                corr("net", "embedded/left-circuit/N$1", "embedded/right-circuit/N$1"),
                corr("electrical-board", "left", "right")
            ],
            "exceptions": []
        }
    }]});
    let document = json!({
        "format": "boardstudio/v2", "formatVersion": 5, "id": "hosted", "name": "Hosted",
        "revision": 0, "parameters": {},
        "definitions": [], "layouts": [], "outline": [], "caseBodies": [],
        "materials": [], "assets": [], "scripts": [], "constraints": [],
        "nets": nets, "boards": boards, "parts": parts, "matrices": matrices,
        "moduleDefinitions": module_definitions, "modules": modules,
        "embeddedCircuits": circuits, "hardware": hardware
    });
    serde_json::from_value(document).expect("the fixture is a valid project")
}

pub(super) fn with_mapping(mapping: Pose2) -> ProjectDoc {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
    document
}

pub(super) fn with_exception(
    kind: &str,
    mirror: &str,
    state: MirroredExceptionState,
) -> ProjectDoc {
    let mut document = fixture();
    let state = match state {
        MirroredExceptionState::Local => "local",
        MirroredExceptionState::Omitted => "omitted",
    };
    let exception: boardstudio_model::model::MirroredException =
        serde_json::from_value(json!({"kind": kind, "mirrorId": mirror, "state": state})).unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(exception);
    document
}

pub(super) fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

pub(super) fn assert_close(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!((a - e).abs() < 1e-9, "{path}: {a} is not {e}");
        }
        (Value::Object(a), Value::Object(e)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                e.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (key, value) in e {
                assert_close(&a[key], value, &format!("{path}.{key}"));
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(
                a.len(),
                e.len(),
                "{path}: length of {actual} against {expected}"
            );
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                assert_close(a, e, &format!("{path}[{index}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}

pub(super) fn run(document: &ProjectDoc, request: &Value) -> Result<Canonical, String> {
    canonicalize(document, &op(request.clone())).map_err(|refusal| refusal.to_string())
}

pub(super) fn expect(document: &ProjectDoc, request: Value, expected: Value) {
    let result = run(document, &request).expect("the request translates");
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &serde_json::to_value(op(expected)).unwrap(),
        "operation",
    );
}

pub(super) fn refused(document: &ProjectDoc, request: Value) -> String {
    run(document, &request).expect_err("the request is refused")
}

/// A request that executes exactly as written.
pub(super) fn unchanged(document: &ProjectDoc, request: Value) {
    expect(document, request.clone(), request);
}
