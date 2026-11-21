use super::*;

#[test]
fn mirror_only_part_refuses_a_reuse_board_pair_target() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-flipped",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        mapping(0.0),
    )
    .unwrap();
    let document = reply["document"].clone();
    let mut target_owned_part = document["parts"][0].clone();
    target_owned_part["id"] = json!("flipped/reuse-only");
    target_owned_part["reference"] = json!("REUSE1");
    let refusal: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"reuse-board-mirror-only","kind":"edit","command":{
                "baseRevision":document["revision"],
                "transactionId":"reuse-board-mirror-only",
                "phase":"commit",
                "targetIds":[],
                "operation":{
                    "kind":"add-mirror-only-part",
                    "part":target_owned_part,
                    "boardId":"left",
                    "definition":null,
                    "layoutId":null
                }
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(refusal["kind"], "error", "{refusal}");
    assert_eq!(snapshot(&mut engine), document, "refusal is atomic");
}

#[test]
fn parts_keep_their_side_and_every_generator_parameter_exactly() {
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    for part in doc["parts"].as_array_mut().unwrap() {
        match part["id"].as_str().unwrap() {
            "left/U1" => part["generatorParameters"] = json!({"reversible": false, "custom": 9}),
            "left/RST" => {
                part["side"] = json!("back");
                part["generatorParameters"] = json!({"reversible": true});
            }
            _ => {}
        }
    }
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    for (source, mirror) in [
        ("left/U1", "right/U1"),
        ("left/RST", "right/RST"),
        ("left/TRRS", "right/TRRS"),
    ] {
        assert_eq!(
            part(document, source)["side"],
            part(document, mirror)["side"],
            "{mirror}"
        );
        assert_eq!(
            part(document, source)["generatorParameters"],
            part(document, mirror)["generatorParameters"],
            "{mirror}"
        );
    }
    assert_eq!(
        part(document, "right/U1")["generatorParameters"],
        json!({"reversible": false, "custom": 9})
    );
    assert_eq!(
        part(document, "right/RST")["generatorParameters"],
        json!({"reversible": true})
    );
    assert_eq!(part(document, "right/RST")["side"], "back");
    // The source is untouched too.
    assert_eq!(
        part(document, "left/U1")["generatorParameters"],
        json!({"reversible": false, "custom": 9})
    );
}

#[test]
fn a_board_with_nothing_on_it_and_a_matrix_id_without_the_board_token_still_mirror() {
    // An empty board mirrors to an empty board.
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    doc["parts"] = json!([]);
    doc["matrices"] = json!([]);
    doc["layouts"] = json!([]);
    doc["outline"] = json!([]);
    doc["nets"] = json!([]);
    doc["boards"][0]["partIds"] = json!([]);
    doc["boards"][0]["outlineIds"] = json!([]);
    doc["boards"][0]["netIds"] = json!([]);
    doc["hardware"]["boards"] = json!([]);
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    assert_eq!(reply["document"]["boards"].as_array().unwrap().len(), 2);
    // A matrix whose id does not contain the board id follows its members.
    let mut doc = seed_document(&synthetic_input, "left");
    let text = serde_json::to_string(&doc)
        .unwrap()
        .replace("left-synthetic-keys", "alpha");
    doc = serde_json::from_str(&text).unwrap();
    let (mut engine, seed) = open(&doc);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let document = &reply["document"];
    assert!(
        document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "right-alpha")
    );
    assert!(
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "matrix/right-alpha/r0c0")
    );
    assert_reflects_every_part(document, "pair-right", "token-free matrix ids");
}

#[test]
fn a_mirrored_outline_identity_that_is_already_used_refuses_the_whole_record() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    document = outline_edit(
        &mut engine,
        &document,
        json!({"kind": "board", "boardId": "left"}),
        json!({"add-connection": {"connection": {
        "id": "left-link", "width": 3.0,
        "points": [{"at": {"x": 4.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 30.0, "y": -20.0}}]}}}),
        1,
    );
    // Another top-level feature already owns the id the mirror's connection would take.
    let mut stray = document.clone();
    stray["outline"].as_array_mut().unwrap().push(
        json!({"kind": "polygon", "id": "right-link", "operation": "add",
        "points": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]}),
    );
    let (mut engine, stray) = open(&stray);
    let error = refused(
        &mut engine,
        &stray,
        operation(
            "p",
            "left",
            new_board("right", "right"),
            synthetic_mapping(&synthetic_input),
        ),
    );
    assert!(error.contains("right-link"), "{error}");
}

/// A board with a mounted module and an embedded circuit mirrors both: the module by ticket 27's
/// rule, the circuit by ticket 28's, and the record names each of them.

#[test]
fn a_circuit_and_a_module_on_one_board_mirror_together() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    let (mut engine, seed) = open(&source);
    let with_circuit = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let reply = create(
        &mut engine,
        &with_circuit,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let document = &reply["document"];
    let expected_modules: BTreeSet<(String, String)> = [
        ("left/splitter-above", "right/splitter-above"),
        ("left/splitter-below", "right/splitter-below"),
    ]
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .into();
    assert_eq!(module_pairs(document, "pair-right"), expected_modules);
    assert_eq!(
        pairs_of_kind(document, "pair-right", "embedded-circuit"),
        [("left-haptic", "right-haptic")]
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .into()
    );
    assert_eq!(
        module(document, "right/splitter-above")["connection"]["hostConnectorPartId"],
        "right/TRRS"
    );
    assert_eq!(record(document, "right-haptic")["hostBoardId"], "right");
}

/// The mirror of a board with embedded circuits is as valid as its source (ticket 28 verification):
/// each circuit finding of the source recurs on the mirror under the mirror circuit id with the same
/// severity and message, no finding is new, every finding target exists on the mirror, no part
/// reference repeats (a circuit member whose mirror reference a host part already holds takes the next
/// free one), and the same edit on the same source gives the same references.

#[test]
fn a_mirrored_circuit_reports_the_findings_of_its_source_and_no_reference_repeats() {
    let synthetic_input = synthetic_half::document();
    let build = || {
        let (mut engine, seed) = seed(&synthetic_input, "left");
        let joined = board_nets(&seed, "left")[0].clone();
        let one = embed_haptic(
            &mut engine,
            &seed,
            "left",
            "left-haptic",
            json!({"gnd": joined}),
        );
        let source = embed_haptic(&mut engine, &one, "left", "review/haptic", json!({}));
        let reply = create(
            &mut engine,
            &source,
            "pair-right",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        )
        .unwrap();
        (source, reply)
    };
    let (source, reply) = build();
    let document = &reply["document"];
    let opened: Value = serde_json::from_str(
        &CoreEngine::new().request(&json!({"id":"o","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let circuit_findings =
        |scene: &Value, circuits: [&str; 2]| -> BTreeSet<(String, String, String)> {
            scene["findings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| {
                    circuits.iter().any(|c| {
                        f["id"]
                            .as_str()
                            .unwrap()
                            .starts_with(&format!("embedded/{c}/"))
                    })
                })
                .map(|f| {
                    (
                        f["id"].as_str().unwrap().to_owned(),
                        f["severity"].as_str().unwrap().to_owned(),
                        f["message"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        };
    let on_source = circuit_findings(&opened["scene"], ["left-haptic", "review/haptic"]);
    let on_mirror = circuit_findings(&reply["scene"], ["right-haptic", "right/review/haptic"]);
    assert!(!on_source.is_empty(), "the fixture raises circuit findings");
    let renamed: BTreeSet<(String, String, String)> = on_source
        .iter()
        .map(|(id, severity, message)| {
            let id = id
                .replacen("embedded/left-haptic/", "embedded/right-haptic/", 1)
                .replacen(
                    "embedded/review/haptic/",
                    "embedded/right/review/haptic/",
                    1,
                );
            (id, severity.clone(), message.clone())
        })
        .collect();
    assert_eq!(on_mirror, renamed);

    let exists = |id: &str| {
        document["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == id)
            || document["embeddedCircuits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == id)
    };
    for finding in reply["scene"]["findings"].as_array().unwrap() {
        if !finding["id"]
            .as_str()
            .unwrap()
            .starts_with("embedded/right")
        {
            continue;
        }
        for target in finding["targetIds"].as_array().unwrap() {
            assert!(exists(target.as_str().unwrap()), "{finding}");
        }
    }

    let references = |doc: &Value| -> Vec<(String, String)> {
        doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["id"].as_str().unwrap().to_owned(),
                    p["reference"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let all = references(document);
    let distinct: BTreeSet<&String> = all.iter().map(|(_, reference)| reference).collect();
    assert_eq!(
        distinct.len(),
        all.len(),
        "a part reference repeats: {all:?}"
    );
}

/// A mounted module is mirrored by pose and id: `at` through the pair map, the heading through
/// the full rule `rotation + 180 - r` (never modulo 180, so the module is not flipped), and its
/// definition, faces, gap, attachment, supports and connection parameters unchanged. Every
/// reference is renamed by the board-prefix rule, and the host instance is the mirror instance.

#[test]
fn two_hosted_modules_mirror_with_exact_ids_poses_and_correspondences() {
    let synthetic_input = synthetic_half::document();
    for map in [
        mapping(300.0),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        let mut source = seed_document(&synthetic_input, "left");
        add_two_splitters(&mut source, "left", "left");
        let (mut engine, seed) = open(&source);
        let before = snapshot(&mut engine);
        let reply = create(
            &mut engine,
            &seed,
            "pair-right",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let document = &reply["document"];
        let ids: BTreeSet<String> = document["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_owned())
            .collect();
        let expected_ids: BTreeSet<String> = [
            "left/splitter-above",
            "left/splitter-below",
            "right/splitter-above",
            "right/splitter-below",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(ids, expected_ids, "map {map}");
        let expected_pairs: BTreeSet<(String, String)> = [
            ("left/splitter-above", "right/splitter-above"),
            ("left/splitter-below", "right/splitter-below"),
        ]
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .into();
        assert_eq!(module_pairs(document, "pair-right"), expected_pairs);

        let rotation = map["rotation"].as_f64().unwrap();
        for (source_id, mirror_id) in [
            ("left/splitter-above", "right/splitter-above"),
            ("left/splitter-below", "right/splitter-below"),
        ] {
            let original = module(&source, source_id);
            let mirror = module(document, mirror_id);
            let at = |m: &Value| {
                (
                    m["at"]["x"].as_f64().unwrap(),
                    m["at"]["y"].as_f64().unwrap(),
                )
            };
            assert_point(at(mirror), reflected_point(&map, at(original)), mirror_id);
            let heading = original["rotation"].as_f64().unwrap();
            assert!(
                angles_match(
                    mirror["rotation"].as_f64().unwrap(),
                    rotation + 180.0 - heading
                ),
                "{mirror_id}: {} from {heading} under map {rotation}",
                mirror["rotation"]
            );
            assert_eq!(mirror["hostBoardId"], "right", "{mirror_id}");
            for key in [
                "definitionId",
                "hostFace",
                "facingFace",
                "gap",
                "attachment",
                "detached",
                "serviceClearance",
                "mountSupports",
            ] {
                assert_eq!(mirror[key], original[key], "{mirror_id} {key}");
            }
            let (from, to) = (&original["connection"], &mirror["connection"]);
            assert_eq!(to["hostConnectorPartId"], "right/TRRS", "{mirror_id}");
            assert_eq!(to["modulePortId"], from["modulePortId"], "{mirror_id}");
            assert_eq!(to["cableType"], from["cableType"], "{mirror_id}");
            assert_eq!(to["assignments"], from["assignments"], "{mirror_id}");
            assert_eq!(to["railVoltages"], from["railVoltages"], "{mirror_id}");
        }
        // The pinned module moves to the mirror instance; the unpinned one stays unpinned.
        assert_eq!(
            module(document, "right/splitter-above")["hostInstanceId"],
            "right"
        );
        assert!(module(document, "right/splitter-below")["hostInstanceId"].is_null());
        // Bus ids and the chain are mapped by the same rule; the chain is the mirror's own.
        assert_eq!(
            module(document, "right/splitter-above")["connection"]["busId"],
            "right/bus-a"
        );
        assert_eq!(
            module(document, "right/splitter-below")["connection"]["busId"],
            "right/bus-b"
        );
        let downstream = &module(document, "right/splitter-below")["connection"];
        assert_eq!(downstream["upstreamModuleId"], "right/splitter-above");
        assert_eq!(downstream["upstreamPortId"], "J1004");
        // Every new id is reported, and the whole edit is one Undo step.
        let changed: BTreeSet<String> = reply["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        for id in ["right/splitter-above", "right/splitter-below", "right"] {
            assert!(changed.contains(id), "{id} missing from {changed:?}");
        }
        let undone = request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        assert_eq!(undone["modules"], before["modules"]);
        // An empty list is omitted from the document, so no pair is the same as none.
        assert!(
            undone["hardware"]["pairs"]
                .as_array()
                .into_iter()
                .flatten()
                .next()
                .is_none()
        );
    }
}

#[test]
fn handed_off_grid_footprint_warning_is_unique_located_and_pair_scoped() {
    let mut source = synthetic_half::document();
    source["definitions"].as_array_mut().unwrap().push(json!({
        "id":"synthetic/handed-connector", "name":"Handed connector", "kind":"connector",
        "courtyard":[{"x":-3,"y":-2},{"x":4,"y":-2},{"x":4,"y":2},{"x":-3,"y":2}],
        "pads":[{"id":"1","number":"1","at":{"x":-2,"y":0},"size":{"x":1,"y":1},"shape":"circle"}],
        "generator":{"source":"synthetic/handed-connector","version":"1","parameters":{}}
    }));
    source["parts"].as_array_mut().unwrap().push(json!({
        "id":"left/JH1", "definitionId":"synthetic/handed-connector", "reference":"JH1",
        "pose":{"at":{"x":-30,"y":20},"rotation":17}, "side":"front"
    }));
    source["boards"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/JH1"));
    let opened: Value = serde_json::from_str(
        &CoreEngine::new()
            .request(&json!({"id":"open","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let findings = |scene: &Value| -> Vec<Value> {
        scene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["id"] == "mirror/right/JH1/heading-uncertain")
            .cloned()
            .collect()
    };
    assert!(
        findings(&opened["scene"]).is_empty(),
        "no pair means no mirror heading warning"
    );
    let (mut engine, seed) = open(&opened["document"]);
    let reply = create(
        &mut engine,
        &seed,
        "pair-handed",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let warning = findings(&reply["scene"]);
    assert_eq!(warning.len(), 1);
    assert_eq!(warning[0]["severity"], "warning");
    assert_eq!(warning[0]["scope"], "pcb");
    assert_eq!(warning[0]["targetIds"], json!(["right/JH1"]));
    assert!(
        !reply["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"] == "mirror/right/RST/heading-uncertain"),
        "the symmetric synthetic switch is exempt"
    );
}

#[test]
fn companion_placement_updates_local_and_omitted_records_for_rejoin() {
    const PAIR: &str = "placement-exception";
    const MIRROR: &str = "matrix/right-synthetic-keys/r0c0/diode";
    let set_state = |state| {
        json!({"kind":"set-half-exception","pairId":PAIR,
        "itemKind":"part","mirrorId":MIRROR,"state":state})
    };
    let tweak = json!({"x":1.0,"y":2.0,"rotation":3.0});
    let placement = json!({"kind":"set-companion-placement","pairId":PAIR,
        "targets":[{"kind":"item","itemKind":"part","mirrorId":MIRROR}],
        "mode":"literal-mirror-with-tweak","tweak":tweak});
    for state in ["local", "omitted"] {
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let made = create(
            &mut engine,
            &source,
            PAIR,
            "left",
            new_board("right", "right"),
            mapping(300.0),
        )
        .unwrap();
        let exceptional =
            create_with(&mut engine, &made["document"], "commit", set_state(state)).unwrap();
        let before = &exceptional["document"];
        let edited = create_with(&mut engine, before, "commit", placement.clone()).unwrap();
        let rejoined = create_with(
            &mut engine,
            &edited["document"],
            "commit",
            set_state("linked"),
        )
        .unwrap();
        for (stage, document) in [
            ("edited", &edited["document"]),
            ("rejoined", &rejoined["document"]),
        ] {
            let halves = &pair(document, PAIR)["halves"];
            let link = halves["correspondences"]
                .as_array()
                .unwrap()
                .iter()
                .find(|link| link["mirrorId"] == MIRROR)
                .unwrap();
            assert_eq!(
                link["sourceId"], "matrix/left-synthetic-keys/r0c0/diode",
                "{state} {stage}"
            );
            assert_eq!(
                link["companionMode"], "literal-mirror-with-tweak",
                "{state} {stage}"
            );
            assert_eq!(link["companionTweak"], tweak, "{state} {stage}");
        }
        if state == "local" {
            assert_eq!(
                part(&edited["document"], MIRROR)["pose"],
                part(before, MIRROR)["pose"]
            );
        } else {
            assert!(find_part(&edited["document"], MIRROR).is_none());
        }
        let mirror_key = part(&rejoined["document"], "matrix/right-synthetic-keys/r0c0");
        let source_key = part(&made["document"], "matrix/left-synthetic-keys/r0c0");
        let source_diode = part(&made["document"], "matrix/left-synthetic-keys/r0c0/diode");
        let (x, y) = in_switch_frame(source_diode, source_key);
        assert_close_pair(
            in_switch_frame(part(&rejoined["document"], MIRROR), mirror_key),
            (1.0 - x, 2.0 + y),
            state,
        );
    }
}

#[test]
fn omitted_and_empty_creation_labels_preserve_unibody_defaults() {
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        for labels in [
            None,
            Some(json!({})),
            Some(json!({"source":{},"mirror":{}})),
        ] {
            let (mut engine, before) = unibody_label_seed();
            let mut op = operation("pair-labels", "left", mode.clone(), mapping(0.0));
            if let Some(labels) = labels {
                op["labels"] = labels;
            }
            let made = create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
            for (id, name, role) in [
                ("left", "Keyboard", "standalone"),
                ("right", "Keyboard mirror", "central"),
            ] {
                let half = instance(&made, id);
                assert_eq!(half["name"], name);
                assert_eq!(half["half"], "unibody");
                assert_eq!(half["role"], role);
            }
        }
    }
}

#[test]
fn production_prepared_routes_mirror_edits_preview_and_history() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let original = seed.clone();
    let created = create(
        &mut engine,
        &seed,
        "runtime-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let move_from_mirror = json!({
        "kind":"move-parts",
        "positions":[{"id":"right/U1","at":{"x":216.0,"y":14.0}}]
    });
    let preview = request(
        &mut engine,
        json!({"id":"runtime-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-preview",
            "phase":"preview",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let preview_transform = |id: &str| {
        preview["scene"]["transforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|transform| transform["id"] == id)
            .unwrap_or_else(|| panic!("preview transform for {id} is missing: {preview}"))
    };
    assert_eq!(preview_transform("left/U1")["pose"]["at"]["x"], -16.0);
    assert_eq!(preview_transform("left/U1")["pose"]["at"]["y"], 14.0);
    assert_eq!(preview_transform("right/U1")["pose"]["at"]["x"], 216.0);
    assert_eq!(preview_transform("right/U1")["pose"]["at"]["y"], 14.0);
    assert_eq!(snapshot(&mut engine), accepted, "preview must not publish");

    let committed = request(
        &mut engine,
        json!({"id":"runtime-commit","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-commit",
            "phase":"commit",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    assert_eq!(
        committed["document"]["revision"],
        accepted["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(
        part(&committed["document"], "left/U1")["pose"]["at"]["x"],
        -16.0
    );
    assert_eq!(
        part(&committed["document"], "right/U1")["pose"]["at"]["x"],
        216.0
    );

    let undone = request(&mut engine, json!({"id":"runtime-undo","kind":"undo"}));
    assert_eq!(
        part(&undone["document"], "left/U1")["pose"]["at"]["x"],
        -24.0
    );
    assert_eq!(
        part(&undone["document"], "right/U1")["pose"]["at"]["x"],
        224.0
    );

    let second_undo = request(
        &mut engine,
        json!({"id":"runtime-second-undo","kind":"undo"}),
    );
    assert_eq!(
        second_undo["document"]["hardware"]["pairs"], original["hardware"]["pairs"],
        "one prepared commit must publish exactly one history snapshot"
    );
    let first_redo = request(
        &mut engine,
        json!({"id":"runtime-first-redo","kind":"redo"}),
    );
    assert_eq!(
        pair(&first_redo["document"], "runtime-pair")["id"],
        "runtime-pair"
    );
    let redone = request(&mut engine, json!({"id":"runtime-redo","kind":"redo"}));
    assert_eq!(
        part(&redone["document"], "left/U1")["pose"]["at"]["x"],
        -16.0
    );
    assert_eq!(
        part(&redone["document"], "right/U1")["pose"]["at"]["x"],
        216.0
    );
}

#[test]
fn production_mirror_move_retunes_constraint_chain_through_preview_and_history() {
    let mut fixture = runtime_supported_document();
    let mut downstream = part(&fixture, "left/TRRS").clone();
    downstream["id"] = json!("left/J2");
    downstream["reference"] = json!("J2");
    downstream["pose"] = json!({"at":{"x":-14.0,"y":5.0},"rotation":0.0});
    fixture["parts"].as_array_mut().unwrap().push(downstream);
    fixture["boards"][0]["partIds"]
        .as_array_mut()
        .unwrap()
        .push(json!("left/J2"));
    fixture["constraints"] = json!([
        {"kind":"offset", "id":"left-rst-u1", "sourcePartId":"left/RST",
         "targetPartId":"left/U1", "offset":{"x":0.0,"y":-16.0}, "rotation":0.0},
        {"kind":"offset", "id":"left-u1-j2", "sourcePartId":"left/U1",
         "targetPartId":"left/J2", "offset":{"x":10.0,"y":5.0}, "rotation":0.0}
    ]);

    let (mut engine, source) = seed(&fixture, "left");
    let created = create(
        &mut engine,
        &source,
        "runtime-canonical-constraint-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    assert_eq!(accepted["constraints"].as_array().unwrap().len(), 2);
    assert_eq!(
        constraint_by_id(&accepted, "left-rst-u1")["offset"],
        json!({"x":0.0,"y":-16.0})
    );
    assert_eq!(
        constraint_by_id(&accepted, "left-u1-j2")["offset"],
        json!({"x":10.0,"y":5.0})
    );

    let move_from_mirror = json!({
        "kind":"move-parts",
        "positions":[{"id":"right/U1","at":{"x":216.0,"y":14.0}}]
    });
    let preview = request(
        &mut engine,
        json!({"id":"runtime-canonical-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-canonical-preview",
            "phase":"preview",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(preview["kind"], "preview", "{preview}");
    let preview_transform = |id: &str| {
        preview["scene"]["transforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|transform| transform["id"] == id)
            .unwrap_or_else(|| panic!("preview transform for {id} is missing: {preview}"))
    };
    // right/U1 is a controller, so it keeps heading 0 while the connector J2 keeps the turn.
    for (id, x, y, rotation) in [
        ("left/U1", -16.0, 14.0, 0.0),
        ("left/J2", -6.0, 19.0, 0.0),
        ("right/U1", 216.0, 14.0, 0.0),
        ("right/J2", 206.0, 19.0, 180.0),
    ] {
        assert_eq!(
            preview_transform(id)["pose"],
            json!({"at":{"x":x,"y":y},"rotation":rotation}),
            "literal Preview pose for {id}"
        );
    }
    assert_eq!(snapshot(&mut engine), accepted, "Preview must not publish");

    let committed = request(
        &mut engine,
        json!({"id":"runtime-canonical-commit","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-canonical-commit",
            "phase":"commit",
            "targetIds":["right/U1"],
            "operation":move_from_mirror
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    let committed_document = &committed["document"];
    assert_eq!(
        committed_document["revision"],
        accepted["revision"].as_u64().unwrap() + 1
    );
    for (id, x, y, rotation) in [
        ("left/U1", -16.0, 14.0, 0.0),
        ("left/J2", -6.0, 19.0, 0.0),
        ("right/U1", 216.0, 14.0, 0.0),
        ("right/J2", 206.0, 19.0, 180.0),
    ] {
        assert_eq!(
            part(committed_document, id)["pose"],
            json!({"at":{"x":x,"y":y},"rotation":rotation}),
            "literal committed pose for {id}"
        );
    }
    assert_eq!(
        constraint_by_id(committed_document, "left-rst-u1"),
        json!({"kind":"framed", "id":"left-rst-u1",
            "source":{"kind":"part","partId":"left/RST"},
            "target":{"kind":"part","partId":"left/U1"},
            "relation":{"kind":"offset","offset":{"x":8.0,"y":-2.0},"rotation":0.0}})
    );
    assert_eq!(
        constraint_by_id(committed_document, "left-u1-j2"),
        json!({"kind":"offset", "id":"left-u1-j2", "sourcePartId":"left/U1",
            "targetPartId":"left/J2", "offset":{"x":10.0,"y":5.0}, "rotation":0.0})
    );

    let undone = request(
        &mut engine,
        json!({"id":"runtime-canonical-undo","kind":"undo"}),
    );
    let undone_document = &undone["document"];
    let mut expected_undo = accepted.clone();
    expected_undo["revision"] = undone_document["revision"].clone();
    assert_eq!(
        undone_document, &expected_undo,
        "Undo restores the full accepted state"
    );
    assert_eq!(
        constraint_by_id(undone_document, "left-rst-u1"),
        constraint_by_id(&accepted, "left-rst-u1")
    );
    assert_eq!(
        constraint_by_id(undone_document, "left-u1-j2"),
        constraint_by_id(&accepted, "left-u1-j2")
    );
    for (id, x, y, rotation) in [
        ("left/U1", -24.0, 0.0, 0.0),
        ("left/J2", -14.0, 5.0, 0.0),
        ("right/U1", 224.0, 0.0, 0.0),
        ("right/J2", 214.0, 5.0, 180.0),
    ] {
        assert_eq!(
            part(undone_document, id)["pose"],
            json!({"at":{"x":x,"y":y},"rotation":rotation}),
            "literal Undo pose for {id}"
        );
    }

    let redone = request(
        &mut engine,
        json!({"id":"runtime-canonical-redo","kind":"redo"}),
    );
    let redone_document = &redone["document"];
    let mut expected_redo = committed_document.clone();
    expected_redo["revision"] = redone_document["revision"].clone();
    assert_eq!(
        redone_document, &expected_redo,
        "Redo restores the full committed state"
    );
    assert_eq!(
        constraint_by_id(redone_document, "left-rst-u1"),
        constraint_by_id(committed_document, "left-rst-u1")
    );
    assert_eq!(
        constraint_by_id(redone_document, "left-u1-j2"),
        constraint_by_id(committed_document, "left-u1-j2")
    );
    for (id, x, y, rotation) in [
        ("left/U1", -16.0, 14.0, 0.0),
        ("left/J2", -6.0, 19.0, 0.0),
        ("right/U1", 216.0, 14.0, 0.0),
        ("right/J2", 206.0, 19.0, 180.0),
    ] {
        assert_eq!(
            part(redone_document, id)["pose"],
            json!({"at":{"x":x,"y":y},"rotation":rotation}),
            "literal Redo pose for {id}"
        );
    }
}

#[test]
fn production_matrix_membership_transition_updates_pair_context_for_later_source_edits() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-membership-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let before = created["document"].clone();
    let removed = request(
        &mut engine,
        json!({"id":"runtime-remove-cell","kind":"edit","command":{
            "baseRevision":before["revision"],
            "transactionId":"runtime-remove-cell",
            "phase":"commit",
            "targetIds":[],
            "operation":{
                "kind":"remove-selected-matrix-cells",
                "cells":{"left-synthetic-keys":[[0,0]]}
            }
        }}),
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    let correspondences =
        pair(&removed["document"], "runtime-membership-pair")["halves"]["correspondences"]
            .as_array()
            .unwrap();
    assert!(correspondences.iter().all(|record| {
        record["sourceId"] != "matrix/left-synthetic-keys/r0c0"
            && record["mirrorId"] != "matrix/right-synthetic-keys/r0c0"
    }));
    assert!(correspondences.iter().any(|record| {
        record["sourceId"] == "matrix/left-synthetic-keys/r0c1"
            && record["mirrorId"] == "matrix/right-synthetic-keys/r0c1"
    }));

    let moved = request(
        &mut engine,
        json!({"id":"runtime-source-edit-after-membership","kind":"edit","command":{
            "baseRevision":removed["document"]["revision"],
            "transactionId":"runtime-source-edit-after-membership",
            "phase":"commit",
            "targetIds":["matrix/left-synthetic-keys/r0c1"],
            "operation":{
                "kind":"move-parts",
                "positions":[{"id":"matrix/left-synthetic-keys/r0c1","at":{"x":43.0,"y":7.0}}]
            }
        }}),
    );
    assert_eq!(moved["kind"], "scene", "{moved}");
    assert_eq!(
        part(&moved["document"], "matrix/left-synthetic-keys/r0c1")["pose"]["at"],
        json!({"x":43.0,"y":7.0})
    );
    assert_eq!(
        part(&moved["document"], "matrix/right-synthetic-keys/r0c1")["pose"]["at"],
        json!({"x":157.0,"y":7.0})
    );
}

#[test]
fn production_curved_key_move_uses_the_published_offset_inverse() {
    let mut input = runtime_supported_document();
    input["matrices"][0]["columnLayout"] = json!([{"rotation":90}]);
    input["matrices"][0]["rowLayout"] = json!([{"bend":90}]);
    let (mut engine, source) = seed(&input, "left");
    // Prepare the matrix through Core so edit comparisons include its canonical edge gap.
    let prepared = request(
        &mut engine,
        json!({"kind":"edit","id":"placement-setup","command":{
            "baseRevision":source["revision"],"transactionId":"placement-setup","phase":"commit","targetIds":[],
            "operation":{"kind":"set-matrix","matrix":source["matrices"][0]}
        }}),
    );
    let source = prepared["document"].clone();
    let created = create(
        &mut engine,
        &source,
        "curved-placement",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let before = created["document"].clone();
    let id = "matrix/left-synthetic-keys/r1c0";
    let old = part(&before, id)["pose"]["at"].clone();
    let x = old["x"].as_f64().unwrap() + 4.0;
    let y = old["y"].as_f64().unwrap() - 3.0;
    let operation = json!({"kind":"move-parts","positions":[{"id":id,"at":{"x":x,"y":y}}]});
    let preview = request(
        &mut engine,
        json!({"kind":"edit","id":"curved-preview","command":{
            "baseRevision":before["revision"],"transactionId":"curved","phase":"preview","targetIds":[id],"operation":operation
        }}),
    );
    assert_eq!(preview["kind"], "preview");
    assert_eq!(snapshot(&mut engine), before);
    let committed = request(
        &mut engine,
        json!({"kind":"edit","id":"curved-commit","command":{
            "baseRevision":before["revision"],"transactionId":"curved","phase":"commit","targetIds":[id],"operation":operation
        }}),
    );
    let after = &committed["document"];
    assert_close_pair(
        (
            part(after, id)["pose"]["at"]["x"].as_f64().unwrap(),
            part(after, id)["pose"]["at"]["y"].as_f64().unwrap(),
        ),
        (x, y),
        "requested source move",
    );
    assert_eq!(
        after["matrices"][0]["columnLayout"],
        before["matrices"][0]["columnLayout"]
    );
    assert_eq!(
        after["matrices"][0]["rowLayout"],
        before["matrices"][0]["rowLayout"]
    );
    for row in 0..2 {
        for column in 0..2 {
            if (row, column) == (1, 0) {
                continue;
            }
            let key = format!("matrix/left-synthetic-keys/r{row}c{column}");
            assert_eq!(part(after, &key), part(&before, &key));
            let mirror = format!("matrix/right-synthetic-keys/r{row}c{column}");
            assert_eq!(part(after, &mirror), part(&before, &mirror));
            assert!(
                !committed["scene"]["changedIds"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(key))
            );
        }
    }
    assert_reflects_every_part(after, "curved-placement", "curved move");
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert_eq!(undone["document"]["matrices"], before["matrices"]);
    assert_eq!(undone["document"]["parts"], before["parts"]);
    let swung = request(
        &mut engine,
        json!({"kind":"edit","id":"mirror-swing","command":{
            "baseRevision":undone["document"]["revision"],"transactionId":"mirror-swing","phase":"commit","targetIds":[],
            "operation":{"kind":"set-matrix-splay","matrixId":"right-synthetic-keys","column":0,
                "change":{"kind":"origin-swing","world":{"x":203.0,"y":2.0}}}
        }}),
    );
    let source_matrix = |document: &Value| {
        document["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|matrix| matrix["id"] == "left-synthetic-keys")
            .unwrap()
            .clone()
    };
    let mut expected = source_matrix(&before);
    expected["columnLayout"][0]["splayOrigin"] = json!({"x":-3.0,"y":2.0});
    assert_eq!(source_matrix(&swung["document"]), expected);
    let source_key = part(&swung["document"], "matrix/left-synthetic-keys/r0c0");
    assert_close_pair(
        (
            source_key["pose"]["at"]["x"].as_f64().unwrap(),
            source_key["pose"]["at"]["y"].as_f64().unwrap(),
        ),
        (-1.0, 5.0),
        "mirror Swing inverse",
    );
    assert_reflects_every_part(&swung["document"], "curved-placement", "mirror Swing");
}

#[test]
fn production_linked_rigid_translation_retains_membership_refusal() {
    // Linked constraint admission belongs to ticket 20; preserve its current atomic refusal.
    for column in [None, Some(0)] {
        let mut input = runtime_supported_document();
        input["constraints"] = json!([{"kind":"offset","id":"internal-keys",
            "sourcePartId":"matrix/left-synthetic-keys/r0c0","targetPartId":"matrix/left-synthetic-keys/r1c0",
            "offset":{"x":0,"y":21},"rotation":0}]);
        let (mut engine, source) = seed(&input, "left");
        let prepared = request(
            &mut engine,
            json!({"kind":"edit","id":"internal-setup","command":{
                "baseRevision":source["revision"],"transactionId":"internal-setup","phase":"commit","targetIds":[],
                "operation":{"kind":"set-matrix","matrix":source["matrices"][0]}
            }}),
        );
        let source = prepared["document"].clone();
        let created = create(
            &mut engine,
            &source,
            "internal-translation",
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let before = created["document"].clone();
        let change = match column {
            Some(column) => {
                json!({"kind":"column-translation","column":column,"delta":{"x":4,"y":-3}})
            }
            None => json!({"kind":"matrix-translation","delta":{"x":4,"y":-3}}),
        };
        let operation =
            json!({"kind":"set-matrix-placement","matrixId":"left-synthetic-keys","change":change});
        for phase in ["preview", "commit"] {
            let input = json!({"kind":"edit","id":format!("internal-{phase}"),"command":{
                "baseRevision":before["revision"],"transactionId":"internal","phase":phase,"targetIds":[],"operation":operation
            }});
            let refused: Value = serde_json::from_str(&engine.request(&input.to_string())).unwrap();
            assert_eq!(refused["kind"], "error");
            assert_eq!(
                refused["message"],
                "Linked matrix placements cannot retain authored or constrained geometry"
            );
            assert_eq!(refused["revision"], before["revision"]);
            assert_eq!(snapshot(&mut engine), before);
        }
        let undone = request(&mut engine, json!({"kind":"undo","id":"internal-undo"}));
        let mut expected = source.clone();
        expected["revision"] = undone["document"]["revision"].clone();
        assert_eq!(
            undone["document"], expected,
            "failed edits add no history entry"
        );
        let redone = request(&mut engine, json!({"kind":"redo","id":"internal-redo"}));
        expected = before;
        expected["revision"] = redone["document"]["revision"].clone();
        assert_eq!(redone["document"], expected);
    }
}
