//! Public Core acceptance edges for ticket 111's activated mirrored-half routes.
use super::parent_acceptance::{assert_history_document, commit_operation};
use super::*;

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

/// Exercise Unlink both with a live Local exception and after Omit/Rejoin. The second
/// case adds the fixture's authored closure mount and checks its materialization.
#[test]
fn public_exception_lifecycle_sequences_preserve_state_through_unlink() {
    let source = runtime_supported_document();
    for (pair_id, source, states, authored_mount) in [
        (
            "runtime-lifecycle-pair",
            source.clone(),
            &["local"][..],
            None,
        ),
        (
            "activation-closure-pair",
            synthetic_half::with_construction_features(source),
            &["local", "omitted", "linked"][..],
            Some("synthetic-center-hole"),
        ),
    ] {
        let (mut engine, seed) = seed(&source, "left");
        let created = create(
            &mut engine,
            &seed,
            pair_id,
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let linked = &created["document"];
        let closure_ids = |document| ["left", "right"].map(|board| closure_holes(document, board));
        let expected_closures = closure_ids(linked);
        let expected_configurations = ["left", "right"].map(|instance_id| {
            let mut configuration = effective(linked, instance_id);
            // Unlink initializes the legacy zero-means-unset identity reservations.
            configuration["nextOpeningIdentity"] = json!(1);
            for opening in configuration["openings"].as_array_mut().unwrap() {
                opening["nextVertexIdentity"] = json!(1);
            }
            configuration
        });
        if let Some(mount_id) = authored_mount {
            assert!(expected_closures.iter().any(|ids| !ids.is_empty()));
            for configuration in &expected_configurations {
                let mount_ids: BTreeSet<_> = configuration["closureMounts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|mount| mount["id"].as_str().unwrap())
                    .collect();
                assert_eq!(mount_ids, BTreeSet::from([mount_id]));
            }
        }

        let mut before_unlink = linked.clone();
        for &state in states {
            let reply = commit_operation(
                &mut engine,
                &before_unlink,
                &format!("{pair_id}-{state}"),
                &["right/RST"],
                exception_operation(pair_id, state),
            );
            assert_eq!(reply["kind"], "scene", "{pair_id} {state}: {reply}");
            before_unlink = reply["document"].clone();
            let exceptions = pair(&before_unlink, pair_id)["halves"]["exceptions"]
                .as_array()
                .map_or(&[][..], Vec::as_slice);
            let expected_target = (state != "omitted").then(|| part(linked, "right/RST"));
            assert_eq!(
                find_part(&before_unlink, "right/RST"),
                expected_target,
                "{pair_id} {state}"
            );
            if state == "linked" {
                assert!(exceptions.is_empty(), "{pair_id} Rejoin: {exceptions:?}");
            } else {
                assert_eq!(exceptions.len(), 1, "{pair_id} {state}");
                assert_eq!(exceptions[0]["kind"], "part");
                assert_eq!(exceptions[0]["mirrorId"], "right/RST");
                assert_eq!(exceptions[0]["state"], state);
            }
        }

        let unlinked = commit_operation(
            &mut engine,
            &before_unlink,
            &format!("{pair_id}-unlink"),
            &[],
            json!({"kind":"unlink-halves","pairId":pair_id}),
        );
        assert_eq!(unlinked["kind"], "scene", "{pair_id}: {unlinked}");
        let unlinked_document = &unlinked["document"];
        assert!(
            unlinked_document["hardware"]["pairs"]
                .as_array()
                .is_none_or(|pairs| pairs.iter().all(|record| record["id"] != pair_id))
        );
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
        for closure_id in closure_ids(unlinked_document).into_iter().flatten() {
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

        for (action, expected) in [("undo", &before_unlink), ("redo", unlinked_document)] {
            let reply = request(
                &mut engine,
                json!({"id":format!("{pair_id}-unlink-{action}"),"kind":action}),
            );
            assert_eq!(reply["kind"], "scene", "{pair_id} {action}: {reply}");
            assert_history_document(&reply["document"], expected, action);
        }

        let redone = snapshot(&mut engine);
        // Core's persistence boundary is serialized project data handed to Open.
        let saved: Value = serde_json::from_str(&redone.to_string()).unwrap();
        let (_, reopened) = open(&saved);
        for (stage, document) in [
            ("unlink", unlinked_document),
            ("redo", &redone),
            ("open", &reopened),
        ] {
            assert_eq!(
                ["left", "right"].map(|instance_id| effective(document, instance_id)),
                expected_configurations,
                "{pair_id} {stage} materializes the effective configurations"
            );
            assert_eq!(
                closure_ids(document),
                expected_closures,
                "{pair_id} {stage} closure IDs"
            );
        }
    }
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
