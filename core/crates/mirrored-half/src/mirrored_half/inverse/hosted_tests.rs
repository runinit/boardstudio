//! Direct translation tests for the hosted, electrical and keycap inverse families (ticket 112).
//!
//! Every expectation is a literal worked from the pair map by hand. The axis map sends
//! `(x, y)` to `(200 - x, y)` and a heading `r` to `180 - r`. The rotated map `(100, -30,
//! 90 degrees)` sends the source point `(a, b)` to `(100 - b, -30 - a)` and a heading `r` to
//! `270 - r` (both modulo 360). No expectation calls the production mapping.

use super::super::{Canonical, canonicalize};
#[path = "hosted_fixtures.rs"]
mod fixtures;
use crate::model::{EditOperation, MirroredExceptionState, Pose2, ProjectDoc, Vec2};
use fixtures::*;
use serde_json::{Value, json};

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
