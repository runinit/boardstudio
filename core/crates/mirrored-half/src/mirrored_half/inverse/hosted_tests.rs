//! Direct translation tests for the hosted, electrical and keycap inverse families (ticket 112).
//!
//! Every expectation is a literal worked from the pair map by hand. The axis map sends
//! `(x, y)` to `(200 - x, y)` and a heading `r` to `180 - r`. The rotated map `(100, -30,
//! 90 degrees)` sends the source point `(a, b)` to `(100 - b, -30 - a)` and a heading `r` to
//! `270 - r` (both modulo 360). No expectation calls the production mapping.

use super::super::canonicalize;
use crate::model::{EditOperation, MirroredExceptionState, Pose2, ProjectDoc, Vec2};
use serde_json::{Value, json};

const AXIS: Pose2 = Pose2 {
    at: Vec2 { x: 200.0, y: 0.0 },
    rotation: 0.0,
};
const ROTATED: Pose2 = Pose2 {
    at: Vec2 { x: 100.0, y: -30.0 },
    rotation: 90.0,
};

fn part(id: &str, definition: &str, reference: &str, x: f64, y: f64, rotation: f64) -> Value {
    json!({"id": id, "definitionId": definition, "reference": reference,
        "pose": {"at": {"x": x, "y": y}, "rotation": rotation}, "side": "front"})
}

fn corr(kind: &str, source: &str, mirror: &str) -> Value {
    json!({"kind": kind, "sourceId": source, "mirrorId": mirror})
}

fn module(
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

fn connection(connector: &str, bus: &str, upstream: Option<&str>) -> Value {
    json!({"hostConnectorPartId": connector, "modulePortId": "host", "busId": bus,
        "assignments": {}, "cableType": "cable", "railVoltages": {},
        "upstreamModuleId": upstream})
}

fn pin(part: &str, pad: &str) -> Value {
    json!({"partId": part, "padId": pad})
}

/// Board `left` is the source and `right` its mirror. Each half has a switch, a connector, a
/// module (the right one plugged into the right connector, on the same bus spelled for its
/// side), an upstream module, an embedded circuit with a private and a joined host net, and an
/// electrical configuration. `right/FREE` is a mirror-only part on the joined net, and
/// `left/SRC-ONLY` is a source-only part on it.
fn fixture() -> ProjectDoc {
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

fn with_mapping(mapping: Pose2) -> ProjectDoc {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
    document
}

fn with_exception(kind: &str, mirror: &str, state: MirroredExceptionState) -> ProjectDoc {
    let mut document = fixture();
    let state = match state {
        MirroredExceptionState::Local => "local",
        MirroredExceptionState::Omitted => "omitted",
    };
    let exception: crate::model::MirroredException =
        serde_json::from_value(json!({"kind": kind, "mirrorId": mirror, "state": state})).unwrap();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(exception);
    document
}

fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

fn assert_close(actual: &Value, expected: &Value, path: &str) {
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

fn run(document: &ProjectDoc, request: &Value) -> Result<super::super::Canonical, String> {
    canonicalize(document, &op(request.clone())).map_err(|refusal| refusal.to_string())
}

fn expect(document: &ProjectDoc, request: Value, expected: Value) {
    let result = run(document, &request).expect("the request translates");
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &serde_json::to_value(op(expected)).unwrap(),
        "operation",
    );
}

fn refused(document: &ProjectDoc, request: Value) -> String {
    run(document, &request).expect_err("the request is refused")
}

/// A request that executes exactly as written.
fn unchanged(document: &ProjectDoc, request: Value) {
    expect(document, request.clone(), request);
}

fn module_request(module: Value) -> Value {
    json!({"kind": "set-mounted-module", "instance": module})
}

/// A module request for the translated (source) module at `(x, 3)`.
fn source_request(x: f64, y: f64, rotation: f64) -> Value {
    let mut request = module_request(module(
        "left-module",
        "left",
        "inst-left",
        x,
        rotation,
        connection("left/CONN", "left/bus", Some("left-up")),
    ));
    request["instance"]["at"]["y"] = json!(y);
    request
}

// ---- mounted modules ------------------------------------------------------------------------

#[test]
fn a_mirror_module_edit_edits_the_source_module_through_the_pair_map() {
    // The mirror shows (180, 7) at heading 100: the source is (20, 7) at 180 - 100 = 80.
    let request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        180.0,
        100.0,
        connection("right/CONN", "right/bus", Some("right-up")),
    ));
    let mut request = request;
    request["instance"]["at"]["y"] = json!(7);
    expect(&fixture(), request, source_request(20.0, 7.0, 80.0));
}

#[test]
fn a_rotated_pair_map_places_and_turns_the_source_module() {
    // Mirror point (X, Y) = (60, -100): the source is (-30 - Y, 100 - X) = (70, 40); the
    // heading 270 - 200 = 70.
    let mut request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        60.0,
        200.0,
        connection("right/CONN", "right/bus", Some("right-up")),
    ));
    request["instance"]["at"]["y"] = json!(-100);
    expect(
        &with_mapping(ROTATED),
        request,
        source_request(70.0, 40.0, 70.0),
    );
}

#[test]
fn an_unchanged_mirror_module_translates_to_exactly_the_source_module() {
    for mapping in [AXIS, ROTATED] {
        let mut document = with_mapping(mapping);
        // Put the mirror module where the map says the source puts it.
        let source = document.modules[0].clone();
        let mirror = &mut document.modules[1];
        mirror.at = crate::mechanical::pair::forward(mapping, source.at);
        mirror.rotation = (mapping.rotation - source.rotation + 180.0).rem_euclid(360.0);
        let request = serde_json::to_value(EditOperation::SetMountedModule {
            instance: Box::new(document.modules[1].clone()),
            definition: None,
            host_connector_definition: None,
        })
        .unwrap();
        let result = run(&document, &request).unwrap();
        let EditOperation::SetMountedModule { instance, .. } = result.operation else {
            panic!("a module request stays a module request");
        };
        assert_eq!(*instance, source, "{mapping:?}");
    }
}

#[test]
fn connector_bus_and_upstream_references_follow_the_source_spellings() {
    // A connector, bus and upstream the mirror changes map back: the new connector is the
    // mirror of `left/CONN2`, the bus `right/other` of `left/other`.
    let mut document = fixture();
    document
        .parts
        .push(serde_json::from_value(part("left/CONN2", "conn", "J2", 0.0, 0.0, 0.0)).unwrap());
    document
        .parts
        .push(serde_json::from_value(part("right/CONN2", "conn", "RJ2", 200.0, 0.0, 0.0)).unwrap());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("part", "left/CONN2", "right/CONN2")).unwrap());
    let mut request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        196.0,
        168.0,
        connection("right/CONN2", "right/other", None),
    ));
    request["instance"]["at"]["y"] = json!(3);
    let result = run(&document, &request).unwrap();
    let EditOperation::SetMountedModule { instance, .. } = result.operation else {
        panic!();
    };
    let connection = instance.connection.unwrap();
    assert_eq!(connection.host_connector_part_id, "left/CONN2");
    assert_eq!(connection.bus_id, "left/other");
    assert_eq!(connection.upstream_module_id, None);
    assert_eq!(instance.id, "left-module");
    assert_eq!(instance.host_board_id, "left");
    assert_eq!(instance.host_instance_id.as_deref(), Some("inst-left"));
}

#[test]
fn a_reference_the_source_cannot_follow_refuses_the_request() {
    let mut request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        196.0,
        168.0,
        connection("right/FREE", "right/bus", Some("right-up")),
    ));
    request["instance"]["at"]["y"] = json!(3);
    assert!(refused(&fixture(), request.clone()).contains("right/FREE"));
    request["instance"]["connection"]["hostConnectorPartId"] = json!("right/CONN");
    request["instance"]["connection"]["upstreamModuleId"] = json!("mirror-only-module");
    assert!(refused(&fixture(), request.clone()).contains("mirror-only-module"));
    request["instance"]["connection"]["upstreamModuleId"] = json!("right-up");
    request["instance"]["hostBoardId"] = json!("left");
    assert!(refused(&fixture(), request.clone()).contains("stays on board"));
    request["instance"]["hostBoardId"] = json!("right");
    request["instance"]["hostInstanceId"] = json!("inst-left");
    assert!(refused(&fixture(), request).contains("not the mirror's"));
}

#[test]
fn source_local_and_omitted_module_targets_follow_the_established_rules() {
    let source = module_request(module(
        "left-module",
        "left",
        "inst-left",
        9.0,
        1.0,
        connection("left/CONN", "left/bus", None),
    ));
    unchanged(&fixture(), source);
    let mirror = module_request(module(
        "right-module",
        "right",
        "inst-right",
        9.0,
        1.0,
        connection("right/CONN", "right/bus", None),
    ));
    unchanged(
        &with_exception("module", "right-module", MirroredExceptionState::Local),
        mirror.clone(),
    );
    let message = refused(
        &with_exception("module", "right-module", MirroredExceptionState::Omitted),
        mirror,
    );
    assert!(message.contains("omitted"), "{message}");
    unchanged(
        &fixture(),
        json!({"kind": "remove-mounted-module", "id": "left-module"}),
    );
}

#[test]
fn removing_a_mirror_module_removes_its_source_module() {
    expect(
        &fixture(),
        json!({"kind": "remove-mounted-module", "id": "right-module"}),
        json!({"kind": "remove-mounted-module", "id": "left-module"}),
    );
    let message = refused(
        &with_exception("module", "right-module", MirroredExceptionState::Omitted),
        json!({"kind": "remove-mounted-module", "id": "right-module"}),
    );
    assert!(message.contains("omitted"));
}

#[test]
fn a_module_new_on_the_mirror_board_is_a_planned_source_addition() {
    let mut request = module_request(module(
        "right-new",
        "right",
        "inst-right",
        180.0,
        100.0,
        connection("right/CONN", "right/new-bus", None),
    ));
    request["instance"]["at"]["y"] = json!(7);
    let result = run(&fixture(), &request).unwrap();
    let EditOperation::SetMountedModule { instance, .. } = &result.operation else {
        panic!();
    };
    // The board token `right` of `right-new` becomes `left`.
    assert_eq!(instance.id, "left-new");
    assert_eq!(instance.host_board_id, "left");
    assert_eq!(
        (instance.at.x, instance.at.y, instance.rotation),
        (20.0, 7.0, 80.0)
    );
    assert_eq!(
        instance.connection.as_ref().unwrap().host_connector_part_id,
        "left/CONN"
    );
    assert_eq!(instance.connection.as_ref().unwrap().bus_id, "left/new-bus");
    assert_eq!(
        result.hosted.modules,
        vec![super::ModuleAddition {
            source_id: "left-new".into(),
            mirror_id: "right-new".into()
        }]
    );
    // A request on the source board is its own addition and plans nothing.
    let mut source = module_request(module(
        "left-new",
        "left",
        "inst-left",
        1.0,
        2.0,
        connection("left/CONN", "left/new-bus", None),
    ));
    source["instance"]["at"]["y"] = json!(7);
    let result = run(&fixture(), &source).unwrap();
    assert!(result.hosted.modules.is_empty());
}

#[test]
fn a_host_connector_created_with_a_mirror_module_is_a_planned_part_pair() {
    let connector = json!({"id": "vik-connector", "name": "VIK", "kind": "connector",
        "courtyard": [], "pads": []});
    let mut request = json!({"kind": "set-mounted-module",
        "instance": module("right-new", "right", "inst-right", 180.0, 100.0,
            connection("", "right/new-bus", None)),
        "hostConnectorDefinition": connector});
    request["instance"]["at"]["y"] = json!(7);
    let result = run(&fixture(), &request).unwrap();
    assert_eq!(result.additions.len(), 1);
    assert_eq!(result.additions[0].source_id, "left-new/vik-host-connector");
    assert_eq!(
        result.additions[0].mirror_id,
        "right-new/vik-host-connector"
    );
    assert_eq!(result.additions[0].mirror_definition_id, "vik-connector");
    assert_eq!(result.additions[0].mirror_reference, "J_VIK1");
    // The shared connector definition rides unchanged.
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["hostConnectorDefinition"]["id"], "vik-connector");
}

#[test]
fn a_module_definition_payload_is_shared_and_carried_unchanged() {
    let mut request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        196.0,
        168.0,
        connection("right/CONN", "right/bus", Some("right-up")),
    ));
    request["instance"]["at"]["y"] = json!(3);
    request["definition"] = serde_json::to_value(&fixture().module_definitions[0]).unwrap();
    let result = run(&fixture(), &request).unwrap();
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["definition"], request["definition"]);
}

// ---- embedded circuits ----------------------------------------------------------------------

fn circuit_definition() -> Value {
    json!({"id": "circuit-definition", "name": "Circuit", "family": "f", "variant": "v",
        "source": {"repository": "https://example.invalid/c", "revision": "r",
                   "path": "c.kicad_pcb", "license": "MIT"},
        "board": {"contours": []}, "mounts": [], "volumes": [], "openings": [], "models": [],
        "gates": [], "interfaces": [],
        "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
        "constituents": [],
        "circuit": {
            "definitions": [{"id": "cd", "name": "CD", "kind": "custom", "courtyard": [],
                "pads": [{"id": "pad-1", "number": "1", "at": {"x": 0, "y": 0},
                          "size": {"x": 1, "y": 1}, "shape": "circle"}]}],
            "parts": [part("U1", "cd", "U", 1.0, 0.0, 0.0), part("U2", "cd", "U", 2.0, 0.0, 0.0)],
            "nets": [{"id": "N1", "name": "N1", "pins": [pin("U1", "pad-1")]},
                     {"id": "SIG", "name": "SIG", "pins": [pin("U2", "pad-1")]}],
            "ports": {"signal": "SIG"}, "adaptations": []}})
}

#[test]
fn a_circuit_embedded_on_the_mirror_board_is_a_source_embed_with_a_planned_map() {
    let request = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 180, "y": 7}, "rotation": 100}, "side": "back",
        "joins": {"signal": "net-right"}});
    let document = fixture();
    let result = run(&document, &request).unwrap();
    expect(
        &document,
        request.clone(),
        json!({"kind": "embed-module-circuit", "id": "left-extra",
            "definition": circuit_definition(), "hostBoardId": "left",
            "pose": {"at": {"x": 20, "y": 7}, "rotation": 80}, "side": "back",
            "joins": {"signal": "net-left"}}),
    );
    let plan = &result.hosted.circuits[0];
    assert_eq!(plan.source_id, "left-extra");
    assert_eq!(plan.mirror_id, "right-extra");
    // Standalone mirror embedding uses its board-local allocator. The reconciled plan uses a
    // document-wide set plus the staged source references, so these allocations intentionally differ.
    let mut mirror_run = document.clone();
    crate::modules::embed(
        &mut mirror_run,
        "right-extra",
        &serde_json::from_value(circuit_definition()).unwrap(),
        "right",
        Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 100.0,
        },
        crate::model::Side::Back,
        &[("signal".to_owned(), "net-right".to_owned())]
            .into_iter()
            .collect(),
    )
    .unwrap();
    for (source_member, mirror_member) in &plan.member_ids {
        let created = mirror_run
            .parts
            .iter()
            .find(|part| &part.id == mirror_member)
            .expect("the mirror embed creates the planned member");
        let standalone_reference = if source_member.ends_with("/U1") {
            "U"
        } else {
            "U1"
        };
        assert_eq!(created.reference, standalone_reference);
    }
    assert_eq!(plan.member_references["embedded/left-extra/U1"], "U3");
    assert_eq!(plan.member_references["embedded/left-extra/U2"], "U4");
    assert_eq!(plan.member_ids.len(), 2);
    assert_eq!(
        plan.member_ids["embedded/left-extra/U1"],
        "embedded/right-extra/U1"
    );
    assert_eq!(
        plan.definition_ids["embedded/left-extra/definition/cd"],
        "embedded/right-extra/definition/cd"
    );
    // The joined host net maps through its correspondence; the private net by namespace.
    assert_eq!(plan.net_ids["net-left"], "net-right");
    assert_eq!(
        plan.net_ids["embedded/left-extra/N1"],
        "embedded/right-extra/N1"
    );
    let mirror_circuit = mirror_run
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "right-extra")
        .unwrap();
    let mut mirror_nets: Vec<_> = plan.net_ids.values().cloned().collect();
    let mut created_nets = mirror_circuit.net_ids.clone();
    mirror_nets.sort();
    created_nets.sort();
    assert_eq!(mirror_nets, created_nets);
}

#[test]
fn mirror_circuit_references_reserve_unrelated_board_parts_and_all_members() {
    let mut document = fixture();
    let mut unrelated_board = document.boards[0].clone();
    unrelated_board.id = "other".into();
    unrelated_board.name = "Other".into();
    unrelated_board.net_ids.clear();
    unrelated_board.part_ids = vec!["other/C3".into()];
    document.boards.push(unrelated_board);

    let mut unrelated_part = document.parts[0].clone();
    unrelated_part.id = "other/C3".into();
    unrelated_part.reference = "C3".into();
    document.parts.push(unrelated_part);

    let mut definition = circuit_definition();
    for member in definition["circuit"]["parts"].as_array_mut().unwrap() {
        member["reference"] = json!("C1");
    }
    let request = EditOperation::EmbedModuleCircuit {
        id: "right-extra".into(),
        definition: serde_json::from_value(definition).unwrap(),
        host_board_id: "right".into(),
        pose: Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 100.0,
        },
        side: crate::model::Side::Back,
        joins: Default::default(),
    };

    let result = canonicalize(&document, &request).expect("the embed is accepted");
    let plan = &result.hosted.circuits[0];
    let references: Vec<_> = plan.member_references.values().cloned().collect();
    assert_eq!(references, ["C4", "C5"]);
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "other/C3")
            .unwrap()
            .reference,
        "C3",
        "reference planning leaves unrelated board parts unchanged"
    );
    assert_eq!(plan.member_ids.len(), 2, "every member gets a planned id");
    assert_eq!(
        plan.member_references.len(),
        2,
        "every member gets a planned reference"
    );
    assert!(
        references
            .iter()
            .all(|reference| reference != "C1" && reference != "C2" && reference != "C3")
    );
}

#[test]
fn a_rotated_map_places_the_source_circuit() {
    let request = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 60, "y": -100}, "rotation": 200}, "side": "front", "joins": {}});
    let result = run(&with_mapping(ROTATED), &request).unwrap();
    let EditOperation::EmbedModuleCircuit { pose, .. } = result.operation else {
        panic!();
    };
    for (actual, expected) in [(pose.at.x, 70.0), (pose.at.y, 40.0), (pose.rotation, 70.0)] {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{actual} is not {expected}"
        );
    }
}

#[test]
fn circuit_embeds_on_the_source_board_run_as_written_and_unjoinable_hosts_refuse() {
    let source = json!({"kind": "embed-module-circuit", "id": "left-extra",
        "definition": circuit_definition(), "hostBoardId": "left",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 0}, "side": "front", "joins": {}});
    let source_result = run(&fixture(), &source).expect("source-authored circuit is planned");
    assert_close(
        &serde_json::to_value(&source_result.operation).unwrap(),
        &source,
        "source-authored operation remains written as authored",
    );
    let source_plan = &source_result.hosted.circuits[0];
    assert_eq!(source_plan.source_id, "left-extra");
    assert_eq!(source_plan.mirror_id, "right-extra");
    assert_eq!(source_plan.member_ids.len(), 2);
    assert_eq!(
        source_plan.member_ids["embedded/left-extra/U1"],
        "embedded/right-extra/U1"
    );
    assert_eq!(
        source_plan.member_ids["embedded/left-extra/U2"],
        "embedded/right-extra/U2"
    );
    assert_eq!(
        source_plan.definition_ids["embedded/left-extra/definition/cd"],
        "embedded/right-extra/definition/cd"
    );
    assert_eq!(
        source_plan.net_ids["embedded/left-extra/SIG"],
        "embedded/right-extra/SIG"
    );
    assert_eq!(
        source_plan.net_ids["embedded/left-extra/N1"],
        "embedded/right-extra/N1"
    );
    assert!(
        source_plan.member_poses.is_empty(),
        "source-authored member poses remain as written"
    );
    let mut joined_source = source.clone();
    joined_source["joins"] = json!({"signal": "net-left"});
    let joined_result =
        run(&fixture(), &joined_source).expect("a joined source net reuses its exact counterpart");
    assert_eq!(
        joined_result.hosted.circuits[0].net_ids["net-left"],
        "net-right"
    );

    let mut unjoinable_source = source.clone();
    unjoinable_source["joins"] = json!({"signal": "net-right"});
    let message = run(&fixture(), &unjoinable_source)
        .expect_err("a source circuit cannot join a mirror-only net")
        .to_string();
    assert!(
        message.contains("net-right") && message.contains("no mirror"),
        "{message}"
    );

    for (state, label) in [
        (MirroredExceptionState::Local, "local"),
        (MirroredExceptionState::Omitted, "omitted"),
    ] {
        let message = run(&with_exception("net", "net-right", state), &joined_source)
            .expect_err("source circuits cannot join a Local or Omitted counterpart")
            .to_string();
        assert!(message.contains(label), "{message}");
    }

    let mut mirror = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 0}, "side": "front",
        "joins": {"signal": "right-only-net"}});
    let message = refused(&fixture(), mirror.clone());
    assert!(message.contains("right-only-net"), "{message}");
    // The mirror namespace must be free so the forward transition can create it.
    mirror["joins"] = json!({});
    mirror["id"] = json!("right-circuit");
    unchanged(&fixture(), mirror);
}

#[test]
fn removing_a_mirror_circuit_removes_the_source_circuit() {
    expect(
        &fixture(),
        json!({"kind": "remove-embedded-circuit", "id": "right-circuit"}),
        json!({"kind": "remove-embedded-circuit", "id": "left-circuit"}),
    );
    unchanged(
        &fixture(),
        json!({"kind": "remove-embedded-circuit", "id": "left-circuit"}),
    );
    unchanged(
        &with_exception(
            "embedded-circuit",
            "right-circuit",
            MirroredExceptionState::Local,
        ),
        json!({"kind": "remove-embedded-circuit", "id": "right-circuit"}),
    );
    let mut locked = fixture();
    locked
        .parts
        .iter_mut()
        .find(|part| part.id == "embedded/right-circuit/U1")
        .unwrap()
        .locked = Some(true);
    let message = refused(
        &locked,
        json!({"kind": "remove-embedded-circuit", "id": "right-circuit"}),
    );
    assert!(message.contains("Unlock"), "{message}");
}

#[test]
fn a_circuit_member_inherits_its_associated_local_owner_state() {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(json!({
                "kind": "embedded-circuit",
                "mirrorId": "right-circuit",
                "state": "local"
            }))
            .unwrap(),
        );
    let request = json!({"kind": "move-parts", "positions": [
        {"id": "embedded/right-circuit/U1", "at": {"x": 180, "y": 9}}]});
    unchanged(&document, request);
}

#[test]
fn circuit_member_moves_and_removals_translate_with_the_circuit_member_map() {
    expect(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "embedded/right-circuit/U1", "at": {"x": 180, "y": 9}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "embedded/left-circuit/U1", "at": {"x": 20, "y": 9}}]}),
    );
    expect(
        &with_mapping(ROTATED),
        json!({"kind": "move-parts", "positions": [
            {"id": "embedded/right-circuit/U1", "at": {"x": 60, "y": -100}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "embedded/left-circuit/U1", "at": {"x": 70, "y": 40}}]}),
    );
    expect(
        &fixture(),
        json!({"kind": "remove-parts",
               "ids": ["embedded/right-circuit/U1", "embedded/left-circuit/U1"]}),
        json!({"kind": "remove-parts", "ids": ["embedded/left-circuit/U1"]}),
    );
    // A Local circuit's members run as written; an Omitted circuit's are refused.
    let moved = json!({"kind": "move-parts", "positions": [
        {"id": "embedded/right-circuit/U1", "at": {"x": 180, "y": 9}}]});
    unchanged(
        &with_exception(
            "embedded-circuit",
            "right-circuit",
            MirroredExceptionState::Local,
        ),
        moved.clone(),
    );
    let message = refused(
        &with_exception(
            "embedded-circuit",
            "right-circuit",
            MirroredExceptionState::Omitted,
        ),
        moved.clone(),
    );
    assert!(message.contains("omitted"), "{message}");
    // A member whose source is outside the source circuit's namespace is not translated.
    let mut broken = fixture();
    // The wrong source is even a member of the source circuit: only the namespace is wrong.
    broken
        .embedded_circuits
        .iter_mut()
        .find(|circuit| circuit.id == "left-circuit")
        .unwrap()
        .part_ids
        .push("left/SRC-ONLY".into());
    broken.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .find(|c| c.mirror_id == "embedded/right-circuit/U1")
        .unwrap()
        .source_id = "left/SRC-ONLY".into();
    let message = refused(&broken, moved);
    assert!(message.contains("embedded/left-circuit/"), "{message}");
}

#[test]
fn owner_ids_map_and_scalar_payloads_are_unchanged() {
    for (request, expected) in [
        (
            json!({"kind": "set-input-scan-mode", "partId": "matrix/right-keys/r0c0",
                "mode": "direct"}),
            json!({"kind": "set-input-scan-mode", "partId": "matrix/left-keys/r0c0",
                "mode": "direct"}),
        ),
        (
            json!({"kind": "set-wiring-mode", "boardId": "right", "mode": "direct"}),
            json!({"kind": "set-wiring-mode", "boardId": "left", "mode": "direct"}),
        ),
        (
            json!({"kind": "set-keycap-board", "boardId": "right",
                "change": {"kind": "legend-color", "value": "#112233"}}),
            json!({"kind": "set-keycap-board", "boardId": "left",
                "change": {"kind": "legend-color", "value": "#112233"}}),
        ),
        (
            json!({"kind": "set-keycap-board", "boardId": "right",
                "change": {"kind": "clearance", "value": 0.75}}),
            json!({"kind": "set-keycap-board", "boardId": "left",
                "change": {"kind": "clearance", "value": 0.75}}),
        ),
        (
            json!({"kind": "set-matrix-keycaps", "matrixId": "right-keys",
                "change": {"kind": "wall-thickness", "value": 1.25}}),
            json!({"kind": "set-matrix-keycaps", "matrixId": "left-keys",
                "change": {"kind": "wall-thickness", "value": 1.25}}),
        ),
        (
            json!({"kind": "set-keycap-key", "keyId": "matrix/right-keys/r0c0",
                "change": {"kind": "legend", "value": "Esc"}}),
            json!({"kind": "set-keycap-key", "keyId": "matrix/left-keys/r0c0",
                "change": {"kind": "legend", "value": "Esc"}}),
        ),
        (
            json!({"kind": "set-keycap-key", "keyId": "embedded/right-circuit/U1",
                "change": {"kind": "units", "value": {"x": 1.5, "y": 1}}}),
            json!({"kind": "set-keycap-key", "keyId": "embedded/left-circuit/U1",
                "change": {"kind": "units", "value": {"x": 1.5, "y": 1}}}),
        ),
    ] {
        expect(&fixture(), request, expected);
    }
}

#[test]
fn source_local_and_omitted_owners_follow_the_established_rules() {
    for request in [
        json!({"kind": "set-input-scan-mode", "partId": "matrix/left-keys/r0c0", "mode": "matrix"}),
        json!({"kind": "set-wiring-mode", "boardId": "left", "mode": "direct"}),
        json!({"kind": "set-keycap-board", "boardId": "left",
            "change": {"kind": "color", "value": "#000000"}}),
        json!({"kind": "set-matrix-keycaps", "matrixId": "left-keys",
            "change": {"kind": "first-row", "value": 3}}),
        json!({"kind": "set-keycap-key", "keyId": "matrix/left-keys/r0c0",
            "change": {"kind": "legend", "value": null}}),
    ] {
        unchanged(&fixture(), request);
    }
    // A key the mirror keeps as its own, a matrix kept local, an Omitted key.
    let key = json!({"kind": "set-keycap-key", "keyId": "matrix/right-keys/r0c0",
        "change": {"kind": "legend", "value": "Esc"}});
    unchanged(
        &with_exception(
            "part",
            "matrix/right-keys/r0c0",
            MirroredExceptionState::Local,
        ),
        key.clone(),
    );
    assert!(
        refused(
            &with_exception(
                "part",
                "matrix/right-keys/r0c0",
                MirroredExceptionState::Omitted
            ),
            key
        )
        .contains("omitted")
    );
    unchanged(
        &with_exception("matrix", "right-keys", MirroredExceptionState::Local),
        json!({"kind": "set-matrix-keycaps", "matrixId": "right-keys",
            "change": {"kind": "first-row", "value": 3}}),
    );
    unchanged(
        &with_exception("electrical-board", "right", MirroredExceptionState::Local),
        json!({"kind": "set-wiring-mode", "boardId": "right", "mode": "direct"}),
    );
}

#[test]
fn an_unpaired_document_runs_every_hosted_request_as_written() {
    let mut unpaired = fixture();
    unpaired.hardware.as_mut().unwrap().pairs[0].halves = None;
    for request in [
        json!({"kind": "remove-mounted-module", "id": "right-module"}),
        json!({"kind": "remove-embedded-circuit", "id": "right-circuit"}),
        json!({"kind": "set-wiring-mode", "boardId": "right", "mode": "direct"}),
        json!({"kind": "set-keycap-key", "keyId": "matrix/right-keys/r0c0",
            "change": {"kind": "legend", "value": "A"}}),
        json!({"kind": "move-parts", "positions": [
            {"id": "embedded/right-circuit/U1", "at": {"x": 1, "y": 2}}]}),
    ] {
        unchanged(&unpaired, request);
    }
}

// ---- review round: connection, generated connector, placement, circuit exceptions ----------

#[test]
fn an_unrelated_edit_to_a_module_whose_connector_is_omitted_keeps_the_source_connection() {
    // Forward derivation clears the mirror's connection when its connector is Omitted, so the
    // mirror shows `connection: null`. An edit that does not touch it must not clear the source's.
    let mut document = with_exception("part", "right/CONN", MirroredExceptionState::Omitted);
    document.modules[1].connection = None;
    let mut request = module_request(module(
        "right-module",
        "right",
        "inst-right",
        196.0,
        168.0,
        Value::Null,
    ));
    request["instance"]["at"]["y"] = json!(3);
    request["instance"]["gap"] = json!(0.9);
    let result = run(&document, &request).unwrap();
    let EditOperation::SetMountedModule { instance, .. } = result.operation else {
        panic!();
    };
    assert_eq!(instance.gap, 0.9);
    assert_eq!(instance.connection, document.modules[0].connection);
    assert_eq!(
        (instance.at, instance.rotation),
        (document.modules[0].at, document.modules[0].rotation)
    );
    // Disconnecting a module that the mirror still shows connected is a real edit.
    let mut disconnect = module_request(module(
        "right-module",
        "right",
        "inst-right",
        196.0,
        168.0,
        Value::Null,
    ));
    disconnect["instance"]["at"]["y"] = json!(3);
    let result = run(&fixture(), &disconnect).unwrap();
    let EditOperation::SetMountedModule { instance, .. } = result.operation else {
        panic!();
    };
    assert_eq!(instance.connection, None);
}

fn vik_connector() -> Value {
    serde_json::from_str(include_str!("vik_host_connector.json")).unwrap()
}

/// A paired document whose mirror board `pcb` can place a real VIK host connector.
fn vik_document() -> ProjectDoc {
    let mut document = serde_json::to_value(ProjectDoc::empty("vik", "Vik")).unwrap();
    document["boards"] = json!([
        {"id": "src", "name": "src", "partIds": [], "netIds": [], "outlineIds": [], "thickness": 1.6},
        {"id": "pcb", "name": "pcb", "partIds": ["pcb/existing"], "netIds": [],
         "outlineIds": ["outline"], "thickness": 1.6}]);
    document["parts"] = json!([part("pcb/existing", "x", "J_VIK1", 30.0, 30.0, 0.0)]);
    document["outline"] = json!([{"kind": "rect", "id": "outline", "center": {"x": 0, "y": 0},
        "size": {"x": 80, "y": 80}, "radius": 0, "operation": "add"}]);
    document["hardware"] = json!({"topology": "split", "transport": "wired", "instances": [],
        "boards": [], "pairs": [{
            "id": "pair", "members": ["src", "pcb"],
            "mapping": {"at": {"x": 200, "y": 0}, "rotation": 0},
            "construction": {"method": "printed", "mount": "rigid",
                "integratedPlateFrame": false, "plateThickness": 1.5, "plateFoamThickness": 0,
                "bottomFoamThickness": 0, "bottomThickness": 2, "plateToPcb": 3.5,
                "wallThickness": 2, "clearance": 0.2},
            "halves": {"sourceBoardId": "src", "mirrorBoardId": "pcb",
                "correspondences": [], "exceptions": []}}]});
    serde_json::from_value(document).unwrap()
}

fn vik_module_definition() -> Value {
    json!({"id": "module-test", "name": "Test module", "family": "test", "variant": "source",
        "source": {"repository": "https://github.com/sadekbaroudi/vik",
                   "revision": "cd5d16e4cd9137a229fc673412a89d75f4e64553",
                   "path": "pcb/test", "license": "CERN-OHL-S-2.0"},
        "board": {"contours": [{"hole": false, "points": [
            {"x": -10, "y": -8}, {"x": 10, "y": -8}, {"x": 10, "y": 8}, {"x": -10, "y": 8}]}]},
        "mounts": [], "volumes": [], "openings": [], "models": [], "gates": [], "interfaces": [],
        "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
        "constituents": []})
}

fn vik_request(connector_id: &str) -> Value {
    json!({"kind": "set-mounted-module",
        "instance": {"id": "pcb-module", "definitionId": "module-test", "hostBoardId": "pcb",
            "hostFace": "front", "facingFace": "back", "at": {"x": 0, "y": 0}, "rotation": 0,
            "gap": 3, "attachment": "board", "serviceClearance": 0, "detached": false,
            "connection": {"hostConnectorPartId": connector_id, "modulePortId": "input",
                "busId": "", "assignments": {}, "cableType": "type-a-12-0.5",
                "railVoltages": {}}},
        "definition": vik_module_definition(), "hostConnectorDefinition": vik_connector()})
}

#[test]
fn an_explicit_generated_connector_id_is_planned_like_an_empty_one() {
    let document = vik_document();
    for connector_id in ["pcb-module/vik-host-connector", ""] {
        let result = run(&document, &vik_request(connector_id))
            .unwrap_or_else(|error| panic!("{connector_id:?}: {error}"));
        assert_eq!(result.additions.len(), 1, "{connector_id:?}");
        assert_eq!(
            result.additions[0].mirror_id,
            "pcb-module/vik-host-connector"
        );
        assert_eq!(
            result.additions[0].source_id,
            "src-module/vik-host-connector"
        );
        let EditOperation::SetMountedModule { instance, .. } = &result.operation else {
            panic!();
        };
        let expected = if connector_id.is_empty() {
            ""
        } else {
            "src-module/vik-host-connector"
        };
        assert_eq!(
            instance.connection.as_ref().unwrap().host_connector_part_id,
            expected
        );
    }
}

#[test]
fn the_planned_connector_reference_is_what_the_module_edit_allocates() {
    let document = vik_document();
    let request = vik_request("pcb-module/vik-host-connector");
    let result = run(&document, &request).unwrap();
    let EditOperation::SetMountedModule {
        instance,
        definition,
        host_connector_definition,
    } = op(request)
    else {
        panic!();
    };
    let mut executed = document.clone();
    crate::modules::set(
        &mut executed,
        &instance,
        definition.as_deref(),
        host_connector_definition.as_deref(),
    )
    .expect("the mirror edit executes");
    let created = executed
        .parts
        .iter()
        .find(|part| part.id == "pcb-module/vik-host-connector")
        .expect("the connector is created");
    assert_eq!(created.reference, "J_VIK2", "J_VIK1 is taken on the board");
    assert_eq!(result.additions[0].mirror_reference, created.reference);
}

/// Ticket 146: a circuit whose members sit off its x axis and are turned inside it.
fn circuit_with_placed_members() -> Value {
    let mut definition = circuit_definition();
    definition["circuit"]["parts"][0]["pose"] = json!({"at": {"x": 1.5, "y": 2.0}, "rotation": 30});
    definition["circuit"]["parts"][1]["pose"] =
        json!({"at": {"x": -2.0, "y": -1.25}, "rotation": 75});
    definition
}

/// Heading difference modulo 360.
fn heading_gap(a: f64, b: f64) -> f64 {
    let gap = (a - b).rem_euclid(360.0);
    gap.min(360.0 - gap)
}

#[test]
fn a_mirror_embed_lands_every_member_exactly_as_typed() {
    let maps = [
        AXIS,
        ROTATED,
        Pose2 {
            at: Vec2 { x: 50.0, y: 20.0 },
            rotation: 30.0,
        },
        Pose2 {
            at: Vec2 { x: -40.0, y: 15.0 },
            rotation: 0.0,
        },
    ];
    let definition: crate::modules::ModuleDefinition =
        serde_json::from_value(circuit_with_placed_members()).unwrap();
    for mapping in maps {
        for side in [crate::model::Side::Front, crate::model::Side::Back] {
            let document = with_mapping(mapping);
            let typed = Pose2 {
                at: Vec2 { x: 180.0, y: 7.0 },
                rotation: 100.0,
            };
            let request = EditOperation::EmbedModuleCircuit {
                id: "right-extra".into(),
                definition: definition.clone(),
                host_board_id: "right".into(),
                pose: typed,
                side: side.clone(),
                joins: Default::default(),
            };
            let result = canonicalize(&document, &request).expect("the embed is accepted");
            let plan = &result.hosted.circuits[0];
            let EditOperation::EmbedModuleCircuit {
                id,
                definition: shared,
                host_board_id,
                pose,
                side: source_side,
                joins,
            } = result.operation
            else {
                panic!();
            };
            // The shared definition travels unchanged and registers once.
            assert_eq!(shared, definition);
            // What the user typed, member by member: a real embed on the mirror half.
            let mut mirror = document.clone();
            crate::modules::embed(
                &mut mirror,
                "right-extra",
                &definition,
                "right",
                typed,
                side.clone(),
                &Default::default(),
            )
            .unwrap();
            // The prepared source embed with its overrides applied, then the forward rule.
            let mut source = document.clone();
            crate::modules::embed(
                &mut source,
                &id,
                &shared,
                &host_board_id,
                pose,
                source_side,
                &joins,
            )
            .unwrap();
            assert_eq!(plan.member_poses.len(), 2, "{mapping:?} {side:?}");
            for part in &mut source.parts {
                if let Some(over) = plan.member_poses.get(&part.id) {
                    part.pose = *over;
                }
            }
            for member in ["U1", "U2"] {
                let typed_member = mirror
                    .parts
                    .iter()
                    .find(|part| part.id == format!("embedded/right-extra/{member}"))
                    .unwrap();
                let source_member = source
                    .parts
                    .iter()
                    .find(|part| part.id == format!("embedded/{id}/{member}"))
                    .unwrap();
                let landed = crate::mirrored_half::mirror_pose(mapping, source_member.pose);
                let context = format!("{member} {mapping:?} {side:?}");
                assert!(
                    (landed.at.x - typed_member.pose.at.x).abs() < 1e-9,
                    "{context}"
                );
                assert!(
                    (landed.at.y - typed_member.pose.at.y).abs() < 1e-9,
                    "{context}"
                );
                assert!(
                    heading_gap(landed.rotation, typed_member.pose.rotation) < 1e-9,
                    "{context}: {} against {}",
                    landed.rotation,
                    typed_member.pose.rotation
                );
                assert_eq!(source_member.side, typed_member.side, "{context}");
            }
        }
    }
}

#[test]
fn a_rotated_map_source_half_embed_runs_as_written() {
    let request = json!({"kind": "embed-module-circuit", "id": "left-extra",
        "definition": circuit_definition(), "hostBoardId": "left",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 33}, "side": "front", "joins": {}});
    unchanged(&with_mapping(ROTATED), request);
}
