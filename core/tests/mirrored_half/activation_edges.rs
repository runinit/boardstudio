//! Public Core acceptance edges for ticket 111's activated mirrored-half routes.
use super::*;
use std::collections::BTreeMap;

fn electrical_notice_id(kind: &str, identities: &[&str]) -> String {
    let mut id = format!("preview:electrical:{kind}");
    for identity in identities {
        id.push_str(&format!(":{}-", identity.len()));
        for byte in identity.as_bytes() {
            id.push_str(&format!("{byte:02x}"));
        }
    }
    id
}

fn assert_electrical_preview_notice(
    preview: &Value,
    kind: &str,
    identities: &[&str],
    message: String,
    targets: &[&str],
) {
    let expected = json!({
        "id": electrical_notice_id(kind, identities),
        "severity": "info",
        "scope": "pcb",
        "message": message,
        "targetIds": targets,
    });
    assert!(
        preview["scene"]["findings"]
            .as_array()
            .unwrap()
            .contains(&expected),
        "missing exact Preview notice {expected}; findings={}",
        preview["scene"]["findings"],
    );
}

fn exception_operation(pair_id: &str, state: &str) -> Value {
    exception_operation_for(pair_id, "part", "right/RST", state)
}

fn exception_operation_for(pair_id: &str, item_kind: &str, mirror_id: &str, state: &str) -> Value {
    json!({
        "kind": "set-half-exception",
        "pairId": pair_id,
        "itemKind": item_kind,
        "mirrorId": mirror_id,
        "state": state,
    })
}

fn commit_exception(
    engine: &mut CoreEngine,
    document: &Value,
    pair_id: &str,
    state: &str,
) -> Value {
    request(
        engine,
        json!({"id":format!("activation-{state}"),"kind":"edit","command":{
            "baseRevision":document["revision"],
            "transactionId":format!("activation-{state}"),
            "phase":"commit",
            "targetIds":["right/RST"],
            "operation":exception_operation(pair_id, state)
        }}),
    )
}

fn net<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == id)
        .unwrap_or_else(|| panic!("net {id} is unavailable"))
}

fn net_correspondence<'a>(document: &'a Value, pair_id: &str, source_id: &str) -> &'a str {
    pair(document, pair_id)["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "net" && item["sourceId"] == source_id)
        .and_then(|item| item["mirrorId"].as_str())
        .unwrap_or_else(|| panic!("plain net correspondence for {source_id} is missing"))
}

/// Local/Omit/Rejoin and Unlink use the activated public Core path with materialized closure
/// geometry. The synthetic fixture's one authored closure mount is the source of expected IDs;
/// this test does not introduce new closure geometry.
#[test]
fn public_exception_lifecycle_and_unlink_materialize_nonempty_closure_once() {
    let source = synthetic_half::with_construction_features(runtime_supported_document());
    let (mut engine, seed) = seed(&source, "left");
    let created = create(
        &mut engine,
        &seed,
        "activation-closure-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let pair_id = "activation-closure-pair";
    let linked = created["document"].clone();
    let expected_closures: BTreeSet<_> = ["left", "right"]
        .into_iter()
        .flat_map(|board| closure_holes(&linked, board))
        .collect();
    assert!(
        !expected_closures.is_empty(),
        "fixture must have a linked closure"
    );
    let expected_configurations: BTreeMap<_, _> = ["left", "right"]
        .into_iter()
        .map(|instance_id| {
            let mut configuration = effective(&linked, instance_id);
            // Unlink persists the identity reservations required by materialization.
            // This legacy fixture has no reserved pair-owned opening or vertex IDs, so
            // the documented zero-means-unset floor initializes each counter to one.
            configuration["nextOpeningIdentity"] = json!(1);
            for opening in configuration["openings"].as_array_mut().unwrap() {
                opening["nextVertexIdentity"] = json!(1);
            }
            (instance_id, configuration)
        })
        .collect();
    let expected_mount_ids = BTreeSet::from(["synthetic-center-hole"]);
    for configuration in expected_configurations.values() {
        let mount_ids: BTreeSet<_> = configuration["closureMounts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|mount| mount["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            mount_ids, expected_mount_ids,
            "exact authored closure configuration ID"
        );
    }

    let localized = commit_exception(&mut engine, &linked, pair_id, "local");
    assert_eq!(localized["kind"], "scene", "{localized}");
    assert_eq!(
        pair(&localized["document"], pair_id)["halves"]["exceptions"][0]["state"],
        "local"
    );
    assert!(find_part(&localized["document"], "right/RST").is_some());

    let omitted = commit_exception(&mut engine, &localized["document"], pair_id, "omitted");
    assert_eq!(omitted["kind"], "scene", "{omitted}");
    assert!(find_part(&omitted["document"], "right/RST").is_none());
    assert_eq!(
        pair(&omitted["document"], pair_id)["halves"]["exceptions"][0]["state"],
        "omitted"
    );

    let rejoined = commit_exception(&mut engine, &omitted["document"], pair_id, "linked");
    assert_eq!(rejoined["kind"], "scene", "{rejoined}");
    assert!(find_part(&rejoined["document"], "right/RST").is_some());
    assert!(
        pair(&rejoined["document"], pair_id)["halves"]["exceptions"]
            .as_array()
            .is_none_or(|exceptions| exceptions.is_empty()),
        "Rejoin should clear the exception; absent and empty are both valid: {}",
        pair(&rejoined["document"], pair_id)["halves"]["exceptions"]
    );
    let before_unlink = rejoined["document"].clone();

    let unlinked = request(
        &mut engine,
        json!({"id":"activation-unlink","kind":"edit","command":{
            "baseRevision":before_unlink["revision"],
            "transactionId":"activation-unlink",
            "phase":"commit",
            "targetIds":[],
            "operation":{"kind":"unlink-halves","pairId":pair_id}
        }}),
    );
    assert_eq!(unlinked["kind"], "scene", "{unlinked}");
    let unlinked_document = &unlinked["document"];
    assert!(
        unlinked_document["hardware"]["pairs"]
            .as_array()
            .is_none_or(|pairs| pairs.iter().all(|record| record["id"] != pair_id))
    );
    assert_eq!(
        ["left", "right"]
            .into_iter()
            .map(|instance_id| effective(unlinked_document, instance_id))
            .collect::<Vec<_>>(),
        ["left", "right"]
            .into_iter()
            .map(|instance_id| expected_configurations[instance_id].clone())
            .collect::<Vec<_>>(),
        "Unlink materializes each linked effective configuration unchanged"
    );
    let actual_closures: BTreeSet<_> = ["left", "right"]
        .into_iter()
        .flat_map(|board| closure_holes(unlinked_document, board))
        .collect();
    assert_eq!(
        actual_closures, expected_closures,
        "exact materialized closure IDs"
    );
    for board_id in ["left", "right"] {
        for closure_id in closure_holes(unlinked_document, board_id) {
            let memberships = unlinked_document["boards"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|board| {
                    board["partIds"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(closure_id))
                })
                .count();
            assert_eq!(memberships, 1, "{closure_id} has one board membership");
        }
    }
    assert_eq!(
        unlinked_document["hardware"]["instances"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|instance| ["left", "right"].contains(&instance["id"].as_str().unwrap()))
            .map(|instance| instance["constructionOwnership"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["local", "local"]
    );

    let undone = request(
        &mut engine,
        json!({"id":"activation-unlink-undo","kind":"undo"}),
    );
    assert_eq!(undone["kind"], "scene", "{undone}");
    assert!(
        pair(&undone["document"], pair_id)["halves"]["exceptions"]
            .as_array()
            .is_none_or(|exceptions| exceptions.is_empty())
    );
    assert!(find_part(&undone["document"], "right/RST").is_some());
    let redone = request(
        &mut engine,
        json!({"id":"activation-unlink-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    assert_eq!(
        ["left", "right"]
            .into_iter()
            .map(|instance_id| effective(&redone["document"], instance_id))
            .collect::<Vec<_>>(),
        ["left", "right"]
            .into_iter()
            .map(|instance_id| expected_configurations[instance_id].clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        ["left", "right"]
            .into_iter()
            .flat_map(|board| closure_holes(&redone["document"], board))
            .collect::<BTreeSet<_>>(),
        expected_closures
    );

    // Core has no Save request: this is its public persisted-document handoff through serde/Open.
    let saved: Value =
        serde_json::from_str(&serde_json::to_string(&redone["document"]).unwrap()).unwrap();
    let (_reopened_engine, reopened_document) = open(&saved);
    assert_eq!(
        ["left", "right"]
            .into_iter()
            .map(|instance_id| effective(&reopened_document, instance_id))
            .collect::<Vec<_>>(),
        ["left", "right"]
            .into_iter()
            .map(|instance_id| expected_configurations[instance_id].clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        ["left", "right"]
            .into_iter()
            .flat_map(|board| closure_holes(&reopened_document, board))
            .collect::<BTreeSet<_>>(),
        expected_closures
    );
}

/// A Local controller may own a mirror-authored pin on a private net. Rejoining it to its source
/// drops that pin and net in the staged result; Core Preview marks the net while leaving the
/// accepted document untouched, and the same committed transition makes the loss observable.
#[test]
fn public_paired_preview_and_commit_expose_local_controller_pin_loss_on_rejoin() {
    let mut source = crate::support::replacement_electrical::supported_source_document();
    for cell in source["matrices"][0]["cells"].as_array_mut().unwrap() {
        let object = cell.as_object_mut().unwrap();
        object.remove("definitionId");
        object.remove("variant");
        object.remove("assembliesLocal");
    }
    source["hardware"]["boards"][0]["locks"] = json!({"row/0":"P10"});
    source["hardware"]["boards"][0]["assignments"] = json!({"row/0":"P10"});
    source["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "generator:ceoloide/mcu_nice_nano")
        .unwrap()["pads"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"pad-1",
            "number":"1",
            "at":{"x":0.0,"y":0.0},
            "size":{"x":1.0,"y":1.0},
            "shape":"circle"
        }));
    source["nets"].as_array_mut().unwrap().push(json!({
        "id":"left/omit-controller-net",
        "name":"OMIT_CONTROLLER",
        "pins":[{"partId":"left/RST","padId":"pad-1"}]
    }));
    source["boards"][0]["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/omit-controller-net"));
    let (mut engine, seed) = seed(&source, "left");
    let created = create(
        &mut engine,
        &seed,
        "activation-private-net-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let pair_id = "activation-private-net-pair";
    let created_document = &created["document"];
    let assert_wiring = |document: &Value, board_id: &str, controller_id: &str, pin: &str| {
        let board = document["hardware"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["boardId"] == board_id)
            .unwrap();
        assert_eq!(board["controllerPartId"], controller_id);
        assert_eq!(board["locks"]["row/0"], pin);
        assert_eq!(board["assignments"]["row/0"], pin);
    };
    assert_wiring(created_document, "right", "right/U1", "P10");
    let mirror_net_id =
        net_correspondence(created_document, pair_id, "left/omit-controller-net").to_owned();
    let localized = request(
        &mut engine,
        json!({"id":"activation-local-controller","kind":"edit","command":{
            "baseRevision":created_document["revision"],
            "transactionId":"activation-local-controller",
            "phase":"commit",
            "targetIds":["right/U1"],
            "operation":exception_operation_for(pair_id, "part", "right/U1", "local")
        }}),
    );
    assert_eq!(localized["kind"], "scene", "{localized}");
    let mut accepted_with_local_controller = localized["document"].clone();
    assert_wiring(&accepted_with_local_controller, "right", "right/U1", "P10");
    let mirror_net = accepted_with_local_controller["nets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|net| net["id"] == mirror_net_id)
        .unwrap();
    mirror_net["pins"].as_array_mut().unwrap().push(json!({
        "partId":"right/U1",
        "padId":"pad-1"
    }));
    for board in accepted_with_local_controller["hardware"]["boards"]
        .as_array_mut()
        .unwrap()
    {
        if board["boardId"] == "left" {
            board["locks"]["row/0"] = json!("P14");
            board["assignments"]["row/0"] = json!("P14");
        }
    }

    // Seed a pin on a Local controller: ReplaceDocument projects the current source mapping while
    // retaining Local-owned data; the linked Rejoin below repairs away the mirror-authored pin.
    let seeded = request(
        &mut engine,
        json!({"id":"activation-seed-mirror-pin","kind":"edit","command":{
            "baseRevision":accepted_with_local_controller["revision"],
            "transactionId":"activation-seed-mirror-pin",
            "phase":"commit",
            "targetIds":[],
            "operation":{"kind":"replace-document","document":accepted_with_local_controller}
        }}),
    );
    assert_eq!(seeded["kind"], "scene", "{seeded}");
    accepted_with_local_controller = seeded["document"].clone();
    assert_wiring(&accepted_with_local_controller, "left", "left/U1", "P14");
    assert_wiring(&accepted_with_local_controller, "right", "right/U1", "P14");
    assert!(
        net(&accepted_with_local_controller, &mirror_net_id)["pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == "pad-1")
    );

    let local_omit = request(
        &mut engine,
        json!({"id":"activation-private-net-omit","kind":"edit","command":{
            "baseRevision":accepted_with_local_controller["revision"],
            "transactionId":"activation-private-net-omit",
            "phase":"commit",
            "targetIds":["right/RST"],
            "operation":exception_operation(pair_id, "omitted")
        }}),
    );
    assert_eq!(local_omit["kind"], "scene", "{local_omit}");
    let accepted = local_omit["document"].clone();
    assert!(find_part(&accepted, "right/RST").is_none());
    assert_wiring(&accepted, "right", "right/U1", "P14");
    let accepted_mirror_net = accepted["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == mirror_net_id);
    assert!(
        accepted_mirror_net.is_some_and(|net| {
            net["pins"].as_array().is_some_and(|pins| {
                pins.iter()
                    .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == "pad-1")
            })
        }),
        "Omit should retain the Local controller's mirror pin on {mirror_net_id}; \
         nets={:?}; exceptions={}",
        accepted["nets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|net| net["id"].clone())
            .collect::<Vec<_>>(),
        pair(&accepted, pair_id)["halves"]["exceptions"]
    );

    let rejoin_controller = exception_operation_for(pair_id, "part", "right/U1", "linked");
    let preview = request(
        &mut engine,
        json!({"id":"activation-private-net-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"activation-private-net-preview",
            "phase":"preview",
            "targetIds":["right/U1"],
            "operation":rejoin_controller
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert!(
        preview["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .contains(&json!(mirror_net_id))
    );
    let controller_reference = find_part(&accepted, "right/U1").unwrap()["reference"]
        .as_str()
        .unwrap();
    let net_name = net(&accepted, &mirror_net_id)["name"].as_str().unwrap();
    let board_name = accepted["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "right")
        .unwrap()["name"]
        .as_str()
        .unwrap();
    assert_electrical_preview_notice(
        &preview,
        "pin",
        &[&mirror_net_id, "right/U1", "pad-1"],
        format!(
            "Preview removes pad pad-1 on {controller_reference} (right/U1) from net {net_name} ({mirror_net_id})."
        ),
        &[&mirror_net_id, "right/U1"],
    );
    assert_electrical_preview_notice(
        &preview,
        "net",
        &[&mirror_net_id],
        format!("Preview removes net {net_name} ({mirror_net_id})."),
        &[&mirror_net_id],
    );
    assert_electrical_preview_notice(
        &preview,
        "registration",
        &["right", &mirror_net_id],
        format!(
            "Preview removes net {net_name} ({mirror_net_id}) from board {board_name} (right)."
        ),
        &["right", &mirror_net_id],
    );
    assert_eq!(
        snapshot(&mut engine),
        accepted,
        "Preview must not publish the pin loss"
    );
    assert_wiring(&accepted, "right", "right/U1", "P14");

    let committed = request(
        &mut engine,
        json!({"id":"activation-private-net-rejoin","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"activation-private-net-rejoin",
            "phase":"commit",
            "targetIds":["right/U1"],
            "operation":rejoin_controller
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert_wiring(&committed["document"], "right", "right/U1", "P14");
    assert!(
        committed["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| !finding["id"]
                .as_str()
                .unwrap()
                .starts_with("preview:electrical:")),
        "committed findings must not retain Preview-only notices: {committed}"
    );
    assert!(
        net(&accepted, &mirror_net_id)["pins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == "pad-1")
    );
    assert!(
        committed["document"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|net| net["id"] != mirror_net_id)
    );
    assert!(
        committed["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "right")
            .unwrap()["netIds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id != &mirror_net_id)
    );
}

/// A Local mirror circuit owns a joined host-net reference. When Omit makes that corresponded
/// source net private, existing circuit-reference semantics keep the mirror net available.
#[test]
fn public_local_circuit_keeps_its_corresponded_plain_net_across_private_source_omit() {
    let mut source = runtime_supported_document();
    source["nets"].as_array_mut().unwrap().push(json!({
        "id":"left/gnd",
        "name":"GND",
        "pins":[{"partId":"left/RST","padId":"pad-1"}]
    }));
    source["boards"][0]["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/gnd"));
    for definition_id in ["synthetic/controller", "synthetic/reset"] {
        source["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|definition| definition["id"] == definition_id)
            .unwrap()["pads"] = json!([{
            "id":"pad-1",
            "number":"1",
            "at":{"x":0.0,"y":0.0},
            "size":{"x":1.0,"y":1.0},
            "shape":"circle"
        }]);
    }

    let (mut engine, seed) = seed(&source, "left");
    let created = create(
        &mut engine,
        &seed,
        "activation-circuit-net-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let pair_id = "activation-circuit-net-pair";
    let mirror_gnd = net_correspondence(&created["document"], pair_id, "left/gnd").to_owned();
    assert_eq!(
        net(&created["document"], "left/gnd")["pins"],
        json!([{"partId":"left/RST","padId":"pad-1"}])
    );
    let added = request(
        &mut engine,
        json!({"id":"activation-local-circuit-add","kind":"edit","command":{
            "baseRevision":created["document"]["revision"],
            "transactionId":"activation-local-circuit-add",
            "phase":"commit",
            "targetIds":["right/local-net-circuit"],
            "operation":{
                "kind":"embed-module-circuit",
                "id":"right/local-net-circuit",
                "definition":imported_haptic(),
                "hostBoardId":"right",
                "pose":{"at":{"x":120.0,"y":40.0},"rotation":0.0},
                "side":"front",
                "joins":{}
            }
        }}),
    );
    assert_eq!(added["kind"], "scene", "{added}");
    let mirror_circuit_id = pairs_of_kind(&added["document"], pair_id, "embedded-circuit")
        .into_iter()
        .find(|(_, mirror_id)| mirror_id == "right/local-net-circuit")
        .map(|(_, mirror_id)| mirror_id)
        .expect("the mirror circuit correspondence is created");
    let local = request(
        &mut engine,
        json!({"id":"activation-local-circuit","kind":"edit","command":{
            "baseRevision":added["document"]["revision"],
            "transactionId":"activation-local-circuit",
            "phase":"commit",
            "targetIds":[mirror_circuit_id],
            "operation":exception_operation_for(
                pair_id,
                "embedded-circuit",
                &mirror_circuit_id,
                "local"
            )
        }}),
    );
    assert_eq!(local["kind"], "scene", "{local}");
    let mut circuit_owned_document = local["document"].clone();
    let mirror_circuit = circuit_owned_document["embeddedCircuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|circuit| circuit["id"] == mirror_circuit_id)
        .expect("Local mirror circuit survives replacement setup");
    let circuit_private_gnd = mirror_circuit["ports"]["gnd"]
        .as_str()
        .expect("the unjoined circuit has a private gnd net")
        .to_owned();
    assert_ne!(circuit_private_gnd, mirror_gnd);
    mirror_circuit["ports"]["gnd"] = json!(mirror_gnd);
    if !mirror_circuit["netIds"]
        .as_array()
        .unwrap()
        .contains(&json!(mirror_gnd))
    {
        mirror_circuit["netIds"]
            .as_array_mut()
            .unwrap()
            .push(json!(mirror_gnd));
    }
    let seeded = request(
        &mut engine,
        json!({"id":"activation-local-circuit-join","kind":"edit","command":{
            "baseRevision":circuit_owned_document["revision"],
            "transactionId":"activation-local-circuit-join",
            "phase":"commit",
            "targetIds":[],
            "operation":{"kind":"replace-document","document":circuit_owned_document}
        }}),
    );
    assert_eq!(seeded["kind"], "scene", "{seeded}");
    let accepted = seeded["document"].clone();
    let accepted_circuit = accepted["embeddedCircuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|circuit| circuit["id"] == mirror_circuit_id)
        .expect("accepted ReplaceDocument retains the Local mirror circuit");
    let accepted_circuit_exception = pair(&accepted, pair_id)["halves"]["exceptions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|exception| {
            exception["kind"] == "embedded-circuit" && exception["mirrorId"] == mirror_circuit_id
        });
    assert_eq!(
        accepted_circuit_exception.map(|exception| exception["state"].as_str()),
        Some(Some("local")),
        "accepted circuit {mirror_circuit_id} must remain Local before Omit; exception={:?}",
        pair(&accepted, pair_id)["halves"]["exceptions"]
    );
    assert_eq!(
        accepted_circuit["ports"]["gnd"], mirror_gnd,
        "accepted Local circuit {mirror_circuit_id} must retain its mirror host-net port; \
         ports={}; netIds={}",
        accepted_circuit["ports"], accepted_circuit["netIds"]
    );
    assert!(
        accepted_circuit["netIds"]
            .as_array()
            .is_some_and(|net_ids| net_ids.contains(&json!(mirror_gnd))),
        "accepted Local circuit {mirror_circuit_id} must retain net {mirror_gnd}; \
         ports={}; netIds={}",
        accepted_circuit["ports"],
        accepted_circuit["netIds"]
    );
    assert_eq!(
        net(&accepted, "left/gnd")["pins"],
        json!([{"partId":"left/RST","padId":"pad-1"}]),
        "the source net's only external pin belongs to the item being Omitted"
    );
    assert_eq!(
        net(&accepted, &mirror_gnd)["pins"],
        json!([{"partId":"right/RST","padId":"pad-1"}])
    );

    let omit = exception_operation(pair_id, "omitted");
    let preview = request(
        &mut engine,
        json!({"id":"activation-circuit-net-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"activation-circuit-net-preview",
            "phase":"preview",
            "targetIds":["right/RST"],
            "operation":omit
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert!(
        preview["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .contains(&json!(mirror_gnd))
    );
    let reference = find_part(&accepted, "right/RST").unwrap()["reference"]
        .as_str()
        .unwrap();
    let net_name = net(&accepted, &mirror_gnd)["name"].as_str().unwrap();
    assert_electrical_preview_notice(
        &preview,
        "pin",
        &[&mirror_gnd, "right/RST", "pad-1"],
        format!(
            "Preview removes pad pad-1 on {reference} (right/RST) from net {net_name} ({mirror_gnd})."
        ),
        &[&mirror_gnd, "right/RST"],
    );
    for retained_notice in [
        electrical_notice_id("net", &[&mirror_gnd]),
        electrical_notice_id("registration", &["right", &mirror_gnd]),
    ] {
        assert!(
            preview["scene"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .all(|finding| finding["id"] != retained_notice),
            "Preview must not report loss of retained net {mirror_gnd}: {preview}"
        );
    }
    assert_eq!(
        snapshot(&mut engine),
        accepted,
        "Preview must not publish Omit"
    );

    let committed = request(
        &mut engine,
        json!({"id":"activation-circuit-net-omit","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"activation-circuit-net-omit",
            "phase":"commit",
            "targetIds":["right/RST"],
            "operation":omit
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert!(
        committed["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| !finding["id"]
                .as_str()
                .unwrap()
                .starts_with("preview:electrical:")),
        "committed findings must not retain Preview-only notices: {committed}"
    );
    assert!(find_part(&committed["document"], "right/RST").is_none());
    assert!(
        committed["document"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|net| net["id"] == mirror_gnd),
        "Omit removed net {mirror_gnd} despite Local circuit reference: circuits={:?}; \
         exceptions={}; registered_nets={:?}",
        committed["document"]["embeddedCircuits"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|circuit| circuit["id"] == mirror_circuit_id)
            .map(|circuit| json!({
                "id": circuit["id"],
                "ports": circuit["ports"],
                "netIds": circuit["netIds"],
            }))
            .collect::<Vec<_>>(),
        pair(&committed["document"], pair_id)["halves"]["exceptions"],
        committed["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "right")
            .map(|board| board["netIds"].clone())
    );
    assert!(
        net(&committed["document"], &mirror_gnd)["pins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|pin| pin["partId"] != "right/RST"),
        "Omit removes the linked pin but the circuit-owned net remains"
    );
    let circuit = committed["document"]["embeddedCircuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|circuit| circuit["id"] == mirror_circuit_id)
        .expect("Local circuit survives Omit");
    assert_eq!(circuit["ports"]["gnd"], mirror_gnd);
    assert!(
        circuit["netIds"]
            .as_array()
            .unwrap()
            .contains(&json!(mirror_gnd))
    );
    assert!(
        committed["document"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "right")
            .unwrap()["netIds"]
            .as_array()
            .unwrap()
            .contains(&json!(mirror_gnd))
    );
}
