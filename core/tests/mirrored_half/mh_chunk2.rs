use super::*;

#[test]
fn physical_setup_and_pair_creation_are_one_atomic_core_edit() {
    let mut input = synthetic_half::document();
    input["hardware"]["topology"] = json!("unibody");
    input["hardware"]["transport"] = json!("none");
    let (mut engine, opened) = open(&input);
    let source = opened["hardware"]["instances"][0].clone();
    let shared = source["mechanical"].clone();
    let mut request_op = operation(
        "pair/right",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    );
    request_op["physicalSetup"] = json!({
        "topology":"split", "transport":"wireless", "sharedConstruction":shared,
        "sourceInstance":source
    });
    request_op["labels"] = json!({
        "source":{"name":"Left half","half":"left","role":"central"},
        "mirror":{"name":"Right half","half":"right","role":"peripheral"}
    });
    let result = create_with(&mut engine, &opened, "commit", request_op.clone()).unwrap();
    assert_eq!(result["document"]["hardware"]["topology"], "split");
    assert_eq!(result["document"]["hardware"]["transport"], "wireless");
    assert_eq!(result["document"]["hardware"]["sharedConstruction"], shared);
    assert_eq!(
        result["document"]["hardware"]["pairs"][0]["id"],
        "pair/right"
    );
    assert_eq!(
        result["document"]["hardware"]["instances"][0]["name"],
        "Left half"
    );
    let mut undone = request(
        &mut engine,
        json!({"id":"undo-physical-setup","kind":"undo"}),
    )["document"]
        .clone();
    undone["revision"] = opened["revision"].clone();
    assert_eq!(undone, opened, "setup and pair creation undo as one edit");

    let (mut rejected_engine, rejected_opened) = open(&input);
    request_op["physicalSetup"]["transport"] = json!("none");
    assert!(create_with(&mut rejected_engine, &rejected_opened, "commit", request_op).is_err());
    assert_eq!(snapshot(&mut rejected_engine), rejected_opened);
}

#[test]
fn changed_ids_are_exactly_the_new_items_and_source_correspondences() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
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
    let record = &pair(document, "pair-right")["halves"];
    let mut expected: BTreeSet<String> = ["pair-right", "left", "right"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for correspondence in record["correspondences"].as_array().unwrap() {
        expected.insert(correspondence["sourceId"].as_str().unwrap().to_owned());
        expected.insert(correspondence["mirrorId"].as_str().unwrap().to_owned());
    }
    // Generated clearance holes and their definitions that appeared or went away.
    let generated = |doc: &Value| -> BTreeSet<String> {
        let ids = |key: &str| {
            doc[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["id"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        ids("parts")
            .into_iter()
            .chain(ids("definitions"))
            .filter(|id| id.starts_with("case-closure/") || id.starts_with("assembly-closure/"))
            .collect()
    };
    let (before, after) = (generated(&seed), generated(document));
    expected.extend(before.symmetric_difference(&after).cloned());
    let changed: BTreeSet<String> = reply["scene"]["changedIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(changed, expected);
    assert!(changed.contains("matrix/right-synthetic-keys/r0c0"));
    assert!(changed.contains("matrix/left-synthetic-keys/r0c0"));
    assert!(changed.contains("right/TRRS") && changed.contains("left/TRRS"));
    assert!(
        changed
            .iter()
            .any(|id| id.starts_with("case-closure/left/"))
    );
    assert!(
        changed
            .iter()
            .any(|id| id.starts_with("case-closure/right/"))
    );
}

#[test]
fn refused_requests_name_the_problem_and_leave_the_document_byte_identical() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let good = synthetic_mapping(&synthetic_input);
    let op = |pair: &str, source: &str, mode: Value| operation(pair, source, mode, good.clone());
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("right", "left"))
        )
        .contains("\"left\"")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "nowhere", new_board("right", "right"))
        )
        .contains("nowhere")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("left", "right"))
        )
        .contains("already exists")
    );
    assert!(
        refused(
            &mut engine,
            &seed,
            op(" ", "left", new_board("right", "right"))
        )
        .contains("pair named")
    );
    // An id the mirror would mint that another item already owns.
    assert!(
        refused(
            &mut engine,
            &seed,
            op("p", "left", new_board("left/U1", "x"))
        )
        .contains("already")
    );
    // The good request still succeeds after the refusals, and then cannot repeat.
    let (mut engine, seed) = open(&seed);
    let made = create_with(
        &mut engine,
        &seed,
        "commit",
        op("p", "left", new_board("right", "right")),
    )
    .unwrap();
    let after = made["document"].clone();
    assert!(
        refused(
            &mut engine,
            &after,
            op("p2", "left", new_board("second", "second"))
        )
        .contains("already belongs")
    );
    assert!(
        refused(
            &mut engine,
            &after,
            op("p", "right", new_board("third", "third"))
        )
        .contains("pair named")
    );
}

#[test]
fn reuse_board_projects_authored_features_without_copying_board_data() {
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
    let document = &reply["document"];
    // Nothing is copied: boards, matrices, layouts, nets and every part but the generated holes.
    let owned = |doc: &Value, key: &str| -> Vec<Value> {
        doc[key]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| !v["id"].as_str().unwrap().starts_with("case-closure/"))
            .cloned()
            .collect()
    };
    assert_eq!(owned(document, "parts"), owned(&seed, "parts"));
    assert_eq!(document["matrices"], seed["matrices"]);
    assert_eq!(document["layouts"], seed["layouts"]);
    assert_eq!(document["nets"], seed["nets"]);
    assert_eq!(document["hardware"]["boards"], seed["hardware"]["boards"]);
    assert_eq!(document["boards"].as_array().unwrap().len(), 1);
    let flipped = instance(document, "flipped");
    assert_eq!(flipped["boardId"], "left");
    assert_eq!(flipped["half"], "right");
    assert_eq!(flipped["role"], "peripheral");
    assert_eq!(flipped["flipped"], true);
    assert_eq!(flipped["controllerPartId"], Value::Null);
    assert_eq!(flipped["constructionOwnership"], "pair");
    assert_eq!(instance(document, "left")["constructionOwnership"], "pair");
    // The linked features, not cleared copies: the flipped half reads the reflection.
    let record = pair(document, "pair-flipped");
    assert_eq!(record["members"], json!(["left", "flipped"]));
    assert_eq!(record["constructionSource"], "left");
    assert_eq!(record["halves"]["sourceBoardId"], "left");
    assert_eq!(record["halves"]["mirrorBoardId"], "left");
    assert_eq!(record["features"].as_array().unwrap().len(), 2);
    for half in ["left", "flipped"] {
        assert_eq!(
            instance(document, half)["mechanical"]["closureMounts"],
            json!([]),
            "{half}"
        );
        assert_eq!(
            instance(document, half)["mechanical"]["openings"],
            json!([]),
            "{half}"
        );
    }
    let (source, mirror) = (effective(document, "left"), effective(document, "flipped"));
    assert_eq!(source["openings"].as_array().unwrap().len(), 1);
    assert_eq!(mirror["openings"].as_array().unwrap().len(), 1);
    assert_eq!(closure_positions(&source), vec![(3.0, 5.0)]);
    assert_eq!(closure_positions(&mirror), vec![(-3.0, 5.0)]);
    assert_eq!(closure_holes(document, "left").len(), 1);
    // Reuse projects one physical board, so the reflected effective feature is not a second PCB part.
    assert!(closure_holes(document, "flipped").is_empty());
    let shared_board = document["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["id"] == "left")
        .unwrap();
    let shared_hole = closure_holes(document, "left").into_iter().next().unwrap();
    assert!(
        shared_board["partIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == &shared_hole)
    );
    // A reused board is one half of one record.
    let (_, reopened) = open(document);
    assert_eq!(pair(&reopened, "pair-flipped")["id"], "pair-flipped");
    assert!(
        refused(
            &mut engine,
            document,
            operation(
                "again",
                "left",
                json!({"kind":"reuse-board","instanceId":"again"}),
                mapping(0.0)
            )
        )
        .contains("already")
    );
}

/// A mirror circuit id that is already a circuit on the source board refuses the record by name, and
/// the document stays byte-identical.

#[test]
fn a_mirror_circuit_id_already_in_use_refuses_the_record_and_changes_nothing() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let one = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let two = embed_haptic(&mut engine, &one, "left", "right-haptic", json!({}));
    let error = refused(
        &mut engine,
        &two,
        operation(
            "pair-right",
            "left",
            new_board("right", "right"),
            json!({"at":{"x":300.0,"y":12.0},"rotation":27.0}),
        ),
    );
    assert!(
        error.contains("\"right-haptic\" already exists")
            && !error.contains("cannot be mirrored yet"),
        "{error}"
    );
}

#[test]
fn companion_placement_operation_updates_pair_default_and_preserves_item_override() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let made = create_with(
        &mut engine,
        &source,
        "commit",
        operation(
            "placement-edit",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        ),
    )
    .unwrap()["document"]
        .clone();
    let mut document = made.clone();
    let links = document["hardware"]["pairs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|pair| pair["id"] == "placement-edit")
        .unwrap()["halves"]["correspondences"]
        .as_array_mut()
        .unwrap();
    let selected = links
        .iter_mut()
        .find(|link| {
            link["kind"] == "part" && link["sourceId"].as_str().unwrap().ends_with("/diode")
        })
        .unwrap();
    selected["companionMode"] = json!("literal-mirror-with-tweak");
    selected["companionTweak"] = json!({"x": 1.25, "y": -0.5, "rotation": 4.0});
    let (mut engine, document) = open(&document);
    let selected_id = pair(&document, "placement-edit")["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["companionMode"] == "literal-mirror-with-tweak")
        .unwrap()["mirrorId"]
        .clone();
    let another_id = pair(&document, "placement-edit")["halves"]["correspondences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| {
            link["kind"] == "part"
                && link["sourceId"].as_str().unwrap().ends_with("/diode")
                && link["mirrorId"] != selected_id
        })
        .unwrap()["sourceId"]
        .clone();
    let reply = create_with(
        &mut engine,
        &document,
        "commit",
        json!({"kind":"set-companion-placement", "pairId":"placement-edit",
            "targets":[{"kind":"pair"},{"kind":"item","itemKind":"part","mirrorId":another_id}],
            "mode":"duplicate-position", "tweak":null}),
    )
    .expect("pair default mode edit");
    assert_eq!(
        reply["document"]["hardware"]["pairs"][0]["halves"]["companionMode"],
        "duplicate-position"
    );
    let updated = pair(&reply["document"], "placement-edit")["halves"]["correspondences"]
        .as_array()
        .unwrap();
    assert_eq!(
        updated
            .iter()
            .find(|link| link["mirrorId"] == selected_id)
            .unwrap()["companionMode"],
        "literal-mirror-with-tweak"
    );
    assert_eq!(
        updated
            .iter()
            .find(|link| link["mirrorId"] == selected_id)
            .unwrap()["companionTweak"],
        json!({"x": 1.25, "y": -0.5, "rotation": 4.0})
    );
    assert_eq!(
        updated
            .iter()
            .find(|link| link["sourceId"] == another_id)
            .unwrap()["companionMode"],
        "duplicate-position"
    );
    let undone = request(&mut engine, json!({"id":"undo-placement","kind":"undo"}));
    let mut before = document;
    before["revision"] = undone["document"]["revision"].clone();
    assert_eq!(undone["document"], before);
}

#[test]
fn companion_placement_item_and_matrix_selectors_deduplicate_and_apply_tweak() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let made = create_with(
        &mut engine,
        &source,
        "commit",
        operation(
            "placement-selectors",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        ),
    )
    .unwrap()["document"]
        .clone();
    let item = "matrix/left-synthetic-keys/r0c1/diode";
    let edit = json!({"kind":"set-companion-placement", "pairId":"placement-selectors",
        "targets":[
            {"kind":"item", "itemKind":"part", "mirrorId":item},
            {"kind":"row", "matrixId":"left-synthetic-keys", "row":0},
            {"kind":"column", "matrixId":"right-synthetic-keys", "column":1},
            {"kind":"matrix", "matrixId":"left-synthetic-keys"}
        ], "mode":"literal-mirror-with-tweak", "tweak":{"x":1.25,"y":-0.5,"rotation":4.0}});
    let reply = create_with(&mut engine, &made, "commit", edit).unwrap();
    let links = pair(&reply["document"], "placement-selectors")["halves"]["correspondences"]
        .as_array()
        .unwrap();
    let companions: Vec<_> = links
        .iter()
        .filter(|link| {
            link["kind"] == "part"
                && (link["sourceId"].as_str().unwrap().ends_with("/diode")
                    || link["sourceId"].as_str().unwrap().ends_with("/led"))
        })
        .collect();
    assert_eq!(companions.len(), 8);
    for link in companions {
        assert_eq!(link["companionMode"], "literal-mirror-with-tweak");
        assert_eq!(
            link["companionTweak"],
            json!({"x":1.25,"y":-0.5,"rotation":4.0})
        );
    }
    let expected_changed: BTreeSet<_> = links
        .iter()
        .filter(|link| {
            link["kind"] == "part"
                && (link["sourceId"].as_str().unwrap().ends_with("/diode")
                    || link["sourceId"].as_str().unwrap().ends_with("/led"))
        })
        .map(|link| link["mirrorId"].as_str().unwrap().to_owned())
        .collect();
    let expected_changed = expected_changed
        .into_iter()
        .chain([
            "placement-selectors".to_owned(),
            "right-synthetic-keys".to_owned(),
        ])
        .collect();
    let reported_changed: BTreeSet<_> = reply["scene"]["changedIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(reported_changed, expected_changed);
    assert!(
        links
            .iter()
            .filter(|link| link["sourceId"] == "left/U1")
            .all(|link| link.get("companionMode").is_none())
    );
    let refused = create_with(
        &mut engine,
        &reply["document"],
        "commit",
        json!({"kind":"set-companion-placement", "pairId":"placement-selectors",
            "targets":[{"kind":"pair"},{"kind":"item","itemKind":"part","mirrorId":item}],
            "mode":"literal-mirror", "tweak":{"x":1.0,"y":0.0,"rotation":0.0}}),
    );
    assert!(refused.is_err());
    assert_eq!(snapshot(&mut engine), reply["document"]);
}

#[test]
fn a_reused_board_cannot_take_a_tweak_that_it_cannot_store_per_item() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let mut op = operation(
        "pair-reuse",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        mapping(0.0),
    );
    op["companionMode"] = json!("literal-mirror-with-tweak");
    op["companionTweak"] = json!({"x":1.0,"y":2.0,"rotation":3.0});
    let error = refused(&mut engine, &source, op);
    assert!(
        error.contains("reuse-board") && error.contains("companionTweak"),
        "{error}"
    );
}

#[test]
fn partial_creation_labels_only_replace_the_supplied_fields() {
    for labels in [
        json!({"source":{"name":"Primary"}}),
        json!({"mirror":{"half":"right"}}),
        json!({"source":{"half":"right"},"mirror":{"role":"standalone"}}),
        json!({"source":{"role":"central"},"mirror":{"half":"right"}}),
    ] {
        let (mut engine, before) = unibody_label_seed();
        let mut op = operation(
            "pair-labels",
            "left",
            new_board("right", "right"),
            mapping(0.0),
        );
        op["labels"] = labels.clone();
        let made = create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
        for (id, side, name, role) in [
            ("left", "source", "Keyboard", "standalone"),
            ("right", "mirror", "Keyboard mirror", "central"),
        ] {
            for (field, default) in [("name", name), ("half", "unibody"), ("role", role)] {
                assert_eq!(
                    instance(&made, id)[field],
                    labels[side].get(field).cloned().unwrap_or(json!(default)),
                    "{id}.{field}"
                );
            }
        }
    }
}

#[test]
fn invalid_creation_labels_are_located_and_refused_without_mutation() {
    let (mut engine, before) = unibody_label_seed();
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        for (labels, location) in [
            (json!({"source":{"name":" \t"}}), "labels.source.name"),
            (json!({"mirror":{"name":""}}), "labels.mirror.name"),
            (json!({"source":{"half":"middle"}}), "labels.source.half"),
            (json!({"mirror":{"half":""}}), "labels.mirror.half"),
            (json!({"source":{"role":"primary"}}), "labels.source.role"),
            (json!({"mirror":{"role":"unknown"}}), "labels.mirror.role"),
            (
                json!({"source":{"half":"left","role":"central"},"mirror":{"half":"left","role":"central"}}),
                "labels.mirror",
            ),
            (json!({"source":{"role":"central"}}), "labels.mirror"),
            (json!({"mirror":{"role":"standalone"}}), "labels.mirror"),
        ] {
            let mut op = operation("pair-labels", "left", mode.clone(), mapping(0.0));
            op["labels"] = labels;
            let error = refused(&mut engine, &before, op);
            assert!(error.contains(location), "{location}: {error}");
        }
    }
}

#[test]
fn missing_source_companions_materialise_with_a_literal_fallback_in_modes_two_and_three() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let intact = with_cell_turns(&mut engine, &source, "left-synthetic-keys", &[(0, 0, 15.0)]);
    let listed_id = "matrix/left-synthetic-keys/r0c0/diode";
    let unlisted_id = "matrix/left-synthetic-keys/r0c0/led";
    let ids = [listed_id, unlisted_id];
    // One pass places both missing companions: the diode stays owned by its board and matrix
    // ("listed"), the LED is also dropped from them ("unlisted").
    let mut source = intact.clone();
    source["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| !ids.contains(&part["id"].as_str().unwrap()));
    for board in source["boards"].as_array_mut().unwrap() {
        board["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| id.as_str() != Some(unlisted_id));
    }
    for matrix in source["matrices"].as_array_mut().unwrap() {
        matrix["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| id.as_str() != Some(unlisted_id));
    }
    for turn in [0.0, 27.0] {
        for mode in ["duplicate-position", "literal-mirror-with-tweak"] {
            let (mut engine, before) = open(&source);
            assert!(ids.iter().all(|id| find_part(&before, id).is_none()));
            let map = json!({"at":{"x":310.0,"y":42.0},"rotation":turn});
            let mut op = operation(
                "pair-fallback",
                "left",
                new_board("right", "right"),
                map.clone(),
            );
            op["companionMode"] = json!(mode);
            if mode == "literal-mirror-with-tweak" {
                op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
            }
            let made = create_with(&mut engine, &before, "commit", op).unwrap()["document"].clone();
            for id in ids {
                let mirror_id = id.replace("left-synthetic-keys", "right-synthetic-keys");
                let original = part(&intact, id);
                let mirrored = part(&made, &mirror_id);
                let expected = mapped_pose(&map, &original["pose"]);
                let label = format!("{mode} map {turn}: {mirror_id}");
                assert_close_pair(
                    (
                        mirrored["pose"]["at"]["x"].as_f64().unwrap(),
                        mirrored["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                    expected.0,
                    &label,
                );
                assert!(
                    (mirrored["pose"]["rotation"].as_f64().unwrap() - expected.1).abs() < 1e-9,
                    "{label}"
                );
                // The source is repaired too, so both sides have a real, owned part to link.
                assert_eq!(
                    part(&made, id)["pose"],
                    original["pose"],
                    "{label}: source pose"
                );
                let link = pair(&made, "pair-fallback")["halves"]["correspondences"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|link| link["mirrorId"] == mirror_id)
                    .unwrap();
                assert_eq!(
                    link["companionMode"], "literal-mirror",
                    "{label}: durable fallback"
                );
                assert!(
                    link.get("companionTweak").is_none(),
                    "{label}: fallback ignores the tweak"
                );
            }
            for matrix in made["matrices"].as_array().unwrap() {
                let board = made["boards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|board| board["id"] == matrix["boardId"])
                    .unwrap();
                for id in matrix["partIds"].as_array().unwrap() {
                    assert!(
                        find_part(&made, id.as_str().unwrap()).is_some(),
                        "unmaterialised {id}"
                    );
                    assert!(
                        board["partIds"].as_array().unwrap().contains(id),
                        "unowned {id}"
                    );
                }
            }
            for original in before["parts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|part| part["id"].as_str().unwrap().starts_with("matrix/"))
            {
                assert_eq!(part(&made, original["id"].as_str().unwrap()), original);
            }
            let (_, reopened) = open(&made);
            assert_eq!(reopened["parts"], made["parts"]);
            assert_eq!(
                pair(&reopened, "pair-fallback"),
                pair(&made, "pair-fallback")
            );
            let mut undone =
                request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
            undone["revision"] = before["revision"].clone();
            assert_eq!(undone, before);
        }
    }
}

#[test]
fn a_missing_switch_is_refused_without_mutation_even_with_missing_companions() {
    let mut source = seed_document(&synthetic_half::document(), "left");
    let key_id = "matrix/left-synthetic-keys/r0c0";
    let belongs_to_cell = |id: &str| id == key_id || id.starts_with(&format!("{key_id}/"));
    source["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| !belongs_to_cell(part["id"].as_str().unwrap()));
    for board in source["boards"].as_array_mut().unwrap() {
        board["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !belongs_to_cell(id.as_str().unwrap()));
    }
    for matrix in source["matrices"].as_array_mut().unwrap() {
        matrix["partIds"]
            .as_array_mut()
            .unwrap()
            .retain(|id| !belongs_to_cell(id.as_str().unwrap()));
    }
    for mode in [
        "literal-mirror",
        "duplicate-position",
        "literal-mirror-with-tweak",
    ] {
        let (mut engine, before) = open(&source);
        assert!(find_part(&before, key_id).is_none());
        let mut op = operation(
            "pair-missing-switch",
            "left",
            new_board("right", "right"),
            mapping(310.0),
        );
        op["companionMode"] = json!(mode);
        if mode == "literal-mirror-with-tweak" {
            op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
        }
        let error = refused(&mut engine, &before, op);
        assert!(
            error.contains("source switch") && error.contains(key_id),
            "{mode}: {error}"
        );
    }
}

#[test]
fn production_rotated_pair_map_mirror_move_preview_commit_and_history() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &source,
        "runtime-rotated-edit-pair",
        "left",
        new_board("right", "right"),
        json!({"at":{"x":200.0,"y":30.0},"rotation":90.0}),
    )
    .unwrap();
    let accepted = created["document"].clone();
    assert_eq!(
        part(&accepted, "left/U1")["pose"],
        json!({"at":{"x":-24.0,"y":0.0},"rotation":0.0})
    );
    // U1 is a controller: the rotated map reflects its heading without the half turn
    // (90 - 0 = 90), while the position still goes through the pair map.
    assert_eq!(
        part(&accepted, "right/U1")["pose"],
        json!({"at":{"x":200.0,"y":54.0},"rotation":90.0})
    );

    let move_from_mirror = json!({
        "kind":"move-parts",
        "positions":[{"id":"right/U1","at":{"x":186.0,"y":46.0}}]
    });
    let preview = request(
        &mut engine,
        json!({"id":"runtime-rotated-preview","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-rotated-preview",
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
    let source_preview_pose = &preview_transform("left/U1")["pose"];
    assert_point(
        (
            source_preview_pose["at"]["x"].as_f64().unwrap(),
            source_preview_pose["at"]["y"].as_f64().unwrap(),
        ),
        (-16.0, 14.0),
        "literal source Preview pose",
    );
    assert_eq!(source_preview_pose["rotation"], 0.0);
    let mirror_preview_pose = &preview_transform("right/U1")["pose"];
    assert_point(
        (
            mirror_preview_pose["at"]["x"].as_f64().unwrap(),
            mirror_preview_pose["at"]["y"].as_f64().unwrap(),
        ),
        (186.0, 46.0),
        "literal mirror Preview pose",
    );
    assert_eq!(mirror_preview_pose["rotation"], 90.0);
    assert_eq!(snapshot(&mut engine), accepted, "Preview must not publish");

    let committed = request(
        &mut engine,
        json!({"id":"runtime-rotated-commit","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-rotated-commit",
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
    let source_committed_pose = &part(committed_document, "left/U1")["pose"];
    assert_point(
        (
            source_committed_pose["at"]["x"].as_f64().unwrap(),
            source_committed_pose["at"]["y"].as_f64().unwrap(),
        ),
        (-16.0, 14.0),
        "literal committed source pose",
    );
    assert_eq!(source_committed_pose["rotation"], 0.0);
    let mirror_committed_pose = &part(committed_document, "right/U1")["pose"];
    assert_point(
        (
            mirror_committed_pose["at"]["x"].as_f64().unwrap(),
            mirror_committed_pose["at"]["y"].as_f64().unwrap(),
        ),
        (186.0, 46.0),
        "literal committed mirror pose",
    );
    assert_eq!(mirror_committed_pose["rotation"], 90.0);

    let undone = request(
        &mut engine,
        json!({"id":"runtime-rotated-undo","kind":"undo"}),
    )["document"]
        .clone();
    let mut expected_undo = accepted;
    expected_undo["revision"] = undone["revision"].clone();
    assert_eq!(
        undone, expected_undo,
        "Undo restores the full accepted document"
    );

    let redone = request(
        &mut engine,
        json!({"id":"runtime-rotated-redo","kind":"redo"}),
    )["document"]
        .clone();
    let mut expected_redo = committed_document.clone();
    expected_redo["revision"] = redone["revision"].clone();
    assert_eq!(
        redone, expected_redo,
        "Redo restores the full committed document"
    );
}

#[test]
fn production_prepared_circuit_addition_preserves_member_frames_and_bidirectional_edits() {
    for side in ["front", "back"] {
        let (mut engine, seed) = seed(&runtime_supported_document(), "left");
        let created = create(
            &mut engine,
            &seed,
            &format!("runtime-circuit-{side}"),
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let mut definition = imported_haptic();
        let member = &mut definition["circuit"]["parts"][0];
        member["pose"] = json!({"at":{"x":3.75,"y":8.5},"rotation":359.0});
        member["side"] = json!("back");
        let added = request(
            &mut engine,
            json!({"id":"runtime-circuit-add","kind":"edit","command":{
                "baseRevision":created["document"]["revision"],
                "transactionId":"runtime-circuit-add",
                "phase":"commit",
                "targetIds":["right/runtime-circuit"],
                "operation":{
                    "kind":"embed-module-circuit",
                    "id":"right/runtime-circuit",
                    "definition":definition,
                    "hostBoardId":"right",
                    "pose":{"at":{"x":120.0,"y":40.0},"rotation":370.0},
                    "side":side,
                    "joins":{}
                }
            }}),
        );
        assert_eq!(added["kind"], "scene", "{added}");
        let document = &added["document"];
        let circuit_pair = pairs_of_kind(
            document,
            &format!("runtime-circuit-{side}"),
            "embedded-circuit",
        )
        .into_iter()
        .find(|(_, mirror_id)| mirror_id == "right/runtime-circuit")
        .unwrap_or_else(|| panic!("the new circuit pair is absent: {document}"));
        let typed_member = &definition["circuit"]["parts"][0];
        let typed_member_id = typed_member["id"].as_str().unwrap();
        let typed_member_reference = typed_member["reference"].as_str().unwrap();
        let member_candidates: Vec<_> =
            pairs_of_kind(document, &format!("runtime-circuit-{side}"), "part")
                .into_iter()
                .filter(|(source_id, mirror_id)| {
                    source_id.starts_with(&format!("embedded/{}/", circuit_pair.0))
                        && source_id.rsplit('/').next() == Some(typed_member_id)
                        && part(document, source_id)["reference"] == typed_member_reference
                        && mirror_id.starts_with("embedded/right/runtime-circuit/")
                })
                .collect();
        assert_eq!(
            member_candidates.len(),
            1,
            "the typed member must resolve to exactly one source/mirror correspondence: {member_candidates:?}"
        );
        let member_pair = member_candidates.into_iter().next().unwrap();
        let source_member = part(document, &member_pair.0);
        let mirror_member = part(document, &member_pair.1);
        assert_eq!(
            typed_member["side"], "back",
            "the fixture exercises an already-Back member"
        );
        assert_eq!(typed_member["pose"]["rotation"], 359.0);
        let local_x = if side == "back" { -3.75 } else { 3.75 };
        let (sin, cos) = 370.0_f64.to_radians().sin_cos();
        let expected_typed_at = (
            120.0 + local_x * cos - 8.5 * sin,
            40.0 + local_x * sin + 8.5 * cos,
        );
        assert_point(
            (
                mirror_member["pose"]["at"]["x"].as_f64().unwrap(),
                mirror_member["pose"]["at"]["y"].as_f64().unwrap(),
            ),
            expected_typed_at,
            "the mirror circuit member must land at the originally requested typed pose",
        );
        let expected_typed_heading = if side == "back" { 11.0 } else { 729.0 };
        assert!(
            angles_match(
                mirror_member["pose"]["rotation"].as_f64().unwrap(),
                expected_typed_heading,
            ),
            "the typed member heading must wrap from {expected_typed_heading}"
        );
        assert_eq!(
            mirror_member["side"],
            if side == "back" { "front" } else { "back" },
            "the already-Back definition member flips with the host side"
        );
        assert_ne!(
            source_member["pose"]["at"]["y"], 0,
            "the member's local y must participate"
        );
        assert_eq!(source_member["side"], mirror_member["side"]);
        assert_point(
            (
                mirror_member["pose"]["at"]["x"].as_f64().unwrap(),
                mirror_member["pose"]["at"]["y"].as_f64().unwrap(),
            ),
            reflected_point(
                &mapping(200.0),
                (
                    source_member["pose"]["at"]["x"].as_f64().unwrap(),
                    source_member["pose"]["at"]["y"].as_f64().unwrap(),
                ),
            ),
            "new circuit member mirror position",
        );
        assert!(angles_match(
            mirror_member["pose"]["rotation"].as_f64().unwrap(),
            180.0 - source_member["pose"]["rotation"].as_f64().unwrap(),
        ));

        for (source_id, target_id, at) in [
            (&member_pair.1, &member_pair.0, json!({"x":72.0,"y":31.0})),
            (&member_pair.0, &member_pair.1, json!({"x":-32.0,"y":-13.0})),
        ] {
            let before = snapshot(&mut engine);
            let moved = request(
                &mut engine,
                json!({"id":"runtime-circuit-member-move","kind":"edit","command":{
                    "baseRevision":before["revision"],
                    "transactionId":format!("runtime-circuit-member-move-{source_id}"),
                    "phase":"commit",
                    "targetIds":[source_id],
                    "operation":{"kind":"move-parts","positions":[{"id":source_id,"at":at}]}
                }}),
            );
            assert_eq!(moved["kind"], "scene", "{moved}");
            let moved_source = part(&moved["document"], target_id);
            let moved_mirror = part(
                &moved["document"],
                if target_id == &member_pair.0 {
                    &member_pair.1
                } else {
                    &member_pair.0
                },
            );
            assert_point(
                (
                    moved_mirror["pose"]["at"]["x"].as_f64().unwrap(),
                    moved_mirror["pose"]["at"]["y"].as_f64().unwrap(),
                ),
                reflected_point(
                    &mapping(200.0),
                    (
                        moved_source["pose"]["at"]["x"].as_f64().unwrap(),
                        moved_source["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                ),
                "subsequent circuit-member edit",
            );
        }
    }
}

#[test]
fn production_prepared_refusal_keeps_document_revision_and_history_untouched() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    create(
        &mut engine,
        &seed,
        "runtime-refusal-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let undone_creation = request(
        &mut engine,
        json!({"id":"runtime-refusal-undo-first","kind":"undo"}),
    );
    assert_eq!(undone_creation["document"]["hardware"], seed["hardware"]);
    let before = undone_creation["document"].clone();
    let refused: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"runtime-prepared-refusal","kind":"edit","command":{
                "baseRevision":before["revision"],
                "transactionId":"runtime-prepared-refusal",
                "phase":"commit",
                "targetIds":[],
                "operation":{
                    "kind":"set-half-exception",
                    "pairId":"missing-pair",
                    "itemKind":"part",
                    "mirrorId":"right/U1",
                    "state":"local"
                }
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(refused["kind"], "error", "{refused}");
    let after_refusal = snapshot(&mut engine);
    assert_eq!(after_refusal["revision"], before["revision"]);
    assert_eq!(after_refusal, before, "a prepared refusal must not publish");

    let redone = request(
        &mut engine,
        json!({"id":"runtime-refusal-redo","kind":"redo"}),
    );
    assert!(
        redone["document"]["hardware"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["id"] == "runtime-refusal-pair")
    );
}

#[test]
fn production_targeted_instance_set_mechanical_uses_preparation_for_unpaired_instances() {
    for initially_disabled in [false, true] {
        let mut document = runtime_supported_document();
        if initially_disabled {
            document["hardware"]["instances"][0]["mechanical"] = Value::Null;
        }
        let (mut engine, before) = open(&document);
        // Ticket 37: raw SetMechanical is deleted. A disabled instance is
        // established with Enable before the typed dimension write.
        let mut base = before["revision"].clone();
        if initially_disabled {
            let hole = mounting_hole();
            let enabled = request(
                &mut engine,
                json!({"id":"runtime-instance-mechanical-enable","kind":"edit","command":{
                    "baseRevision":base,
                    "transactionId":"runtime-instance-mechanical-enable",
                    "phase":"commit",
                    "targetIds":[],
                    "operation":{
                        "kind":"apply-mechanical-edit",
                        "edit":{"target":{"kind":"instance","instanceId":"left"},
                            "change":"enable", "mountingHole": hole}}
                }}),
            );
            assert_eq!(
                enabled["kind"], "scene",
                "disabled={initially_disabled}: {enabled}"
            );
            base = enabled["document"]["revision"].clone();
        }
        let configured = request(
            &mut engine,
            json!({"id":"runtime-instance-mechanical-configure","kind":"edit","command":{
                "baseRevision":base,
                "transactionId":format!("runtime-instance-mechanical-configure-{initially_disabled}"),
                "phase":"commit",
                "targetIds":[],
                // Ticket 37: raw SetMechanical is deleted; SetDimension on the
                // instance is the live typed wall-thickness write.
                "operation":{
                    "kind":"apply-mechanical-edit",
                    "edit":{"target":{"kind":"instance","instanceId":"left"},
                        "change":{"set-dimension":{"field":"wall-thickness","value":3.0}}}
                }
            }}),
        );
        assert_eq!(
            configured["kind"], "scene",
            "disabled={initially_disabled}: {configured}"
        );
        assert_eq!(
            instance(&configured["document"], "left")["mechanical"]["wallThickness"],
            3.0
        );
        assert_eq!(
            configured["document"]["revision"],
            base.as_u64().unwrap() + 1
        );
    }
}
