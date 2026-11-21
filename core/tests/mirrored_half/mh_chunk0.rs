use super::*;

#[test]
fn creation_reflects_every_part_for_axis_rotated_and_seeded_maps() {
    let input = synthetic_half::document();
    let mut maps = vec![
        mapping(300.0),
        json!({"at":{"x":310.0,"y":42.0},"rotation":27.0}),
    ];
    let mut seed_value = 109_u32;
    let mut coordinate = || {
        seed_value = seed_value.wrapping_mul(1664525).wrapping_add(1013904223);
        f64::from(seed_value % 721) - 360.0
    };
    for _ in 0..8 {
        maps.push(json!({"at":{"x":coordinate(),"y":coordinate()},"rotation":coordinate()}));
    }
    for map in maps {
        let (mut engine, seed) = seed(&input, "left");
        let reply = create(
            &mut engine,
            &seed,
            "pair",
            "left",
            new_board("mirror", "mirror"),
            map.clone(),
        )
        .unwrap();
        let document = &reply["document"];
        assert_eq!(pair(document, "pair")["mapping"], map);
        assert_eq!(part(document, "left/U1"), part(&seed, "left/U1"));
        assert_reflects_every_part(document, "pair", &map.to_string());
        assert_eq!(
            part(document, "mirror/U1")["generatorParameters"],
            part(&seed, "left/U1")["generatorParameters"]
        );
    }
}

#[test]
fn layouts_keycaps_and_electrical_configuration_follow_the_mapped_ids() {
    let synthetic_input = synthetic_half::document();
    let mut doc = seed_document(&synthetic_input, "left");
    doc["keycaps"] = json!({
        "boards": {"left": {"color": "#111111", "legendColor": "#222222", "clearance": 0.6}},
        "matrices": {"left-synthetic-keys": {"profile": "dsa", "mount": "mx", "firstRow": 2, "wallThickness": 1.4}},
        "keys": {"matrix/left-synthetic-keys/r0c0": {"legend": "Q", "color": "#ff0000", "row": 3}},
    });
    doc["layouts"][0]["partIds"] = json!(["left/U1", "left/RST"]);
    doc["nets"][0]["id"] = json!("left/column/0/0");
    doc["nets"][0]["name"] = json!("LEFT/column/0/0");
    doc["nets"][0]["pins"] = json!([{"partId":"matrix/left-synthetic-keys/r0c0","padId":"pad-1"}]);
    doc["boards"][0]["netIds"][0] = json!("left/column/0/0");
    let board = &mut doc["hardware"]["boards"][0];
    board["locks"] = json!({"row/0": "P10", "peripheral/left/TRRS/split-rx": "P14"});
    board["assignments"] =
        json!({"row/0":"P3", "column/0":"P4", "peripheral/left/TRRS/split-rx":"P14"});
    board["keyBindings"] = json!({"matrix/left-synthetic-keys/r0c0": "&kp Q"});
    board["jumperStates"] = json!({});
    board["protectedHandoff"] =
        json!({"fingerprint": "f", "revision": 1, "assignments": {"row/0": "P10"}});
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
    // Layouts: mapped ids and board, parts in order, no layout link, the source's untouched.
    let layout = |id: &str| {
        document["layouts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == id)
            .unwrap()
            .clone()
    };
    let expected = json!({"boardId": "right", "id": "right-synthetic-keys-layout", "matrixIds": ["right-synthetic-keys"], "name": "Synthetic keys", "partIds": ["right/U1", "right/RST"]});
    assert_eq!(layout("right-synthetic-keys-layout"), expected);
    assert_eq!(layout("left-synthetic-keys-layout"), seed["layouts"][0]);
    assert_eq!(document["layouts"].as_array().unwrap().len(), 2);
    assert_eq!(
        layout("left-synthetic-keys-layout")["partIds"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(document["nets"].as_array().unwrap().len(), 8);
    let mapped_net = document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == "right/column/0/0")
        .unwrap();
    assert_eq!(mapped_net["name"], "RIGHT/column/0/0");
    assert_eq!(
        mapped_net["pins"],
        json!([{"partId":"matrix/right-synthetic-keys/r0c0","padId":"pad-1"}])
    );
    // Keycaps: settings follow the mapped ids; a legend is copied as it is (a binding is not).
    let keycaps = &document["keycaps"];
    assert_eq!(keycaps["boards"]["right"], keycaps["boards"]["left"]);
    assert_eq!(
        keycaps["matrices"]["right-synthetic-keys"],
        keycaps["matrices"]["left-synthetic-keys"]
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["color"],
        "#ff0000"
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["row"],
        3
    );
    assert_eq!(
        keycaps["keys"]["matrix/right-synthetic-keys/r0c0"]["legend"],
        "Q"
    );
    assert_eq!(
        keycaps["keys"]["matrix/left-synthetic-keys/r0c0"]["legend"],
        "Q"
    );
    // Electrical: the same pins on the renamed functions, nothing of the source in the names.
    let boards = document["hardware"]["boards"].as_array().unwrap();
    let (left, right) = (&boards[0], &boards[1]);
    assert_eq!(right["boardId"], "right");
    assert_eq!(right["controllerPartId"], "right/U1");
    assert_eq!(
        right["assignments"],
        json!({"row/0":"P3", "column/0":"P4", "peripheral/right/TRRS/split-rx":"P14"})
    );
    assert_eq!(right["keyBindings"], json!({}));
    assert_eq!(right["protectedHandoff"], Value::Null);
    assert_eq!(
        left["keyBindings"],
        json!({"matrix/left-synthetic-keys/r0c0": "&kp Q"})
    );
    assert_eq!(
        right["locks"],
        json!({"row/0": "P10", "peripheral/right/TRRS/split-rx": "P14"})
    );
    assert_eq!(
        left["keyBindings"]["matrix/left-synthetic-keys/r0c0"],
        "&kp Q"
    );
    assert_eq!(right["keyBindings"], json!({}));
}

#[test]
fn reuse_board_refuses_a_map_other_than_the_flip_itself() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reuse = json!({"kind":"reuse-board","instanceId":"flipped"});
    for map in [
        json!({"at": {"x": 12.0, "y": 0.0}, "rotation": 0.0}),
        json!({"at": {"x": 0.0, "y": 3.0}, "rotation": 0.0}),
        json!({"at": {"x": 0.0, "y": 0.0}, "rotation": 90.0}),
    ] {
        let error = refused(
            &mut engine,
            &seed,
            operation("p", "left", reuse.clone(), map),
        );
        assert!(
            error.contains("flipped instance") && error.contains("plain reflection"),
            "{error}"
        );
    }
}

/// An embedded circuit mirrors with its namespace intact (ticket 28). `left-haptic` carries a board
/// token, so it mirrors as `right-haptic`; `review/haptic` carries none, so it mirrors as
/// `right/review/haptic`. Under the board-prefix rule alone a member would become
/// `right/embedded/...`, outside the `embedded/<id>/` namespace that `modules::circuit` reads. Each
/// member and private net moves under its circuit's mirror namespace, each member keeps its side and
/// generator parameters, takes the pair map and the full heading rule, and each copied definition
/// is the source's with only its id changed. A port that joins a host net mirrors to the host
/// mirror net.

#[test]
fn a_board_with_embedded_circuits_mirrors_with_valid_namespaces_and_records() {
    let synthetic_input = synthetic_half::document();
    for map in [
        mapping(300.0),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        let (mut engine, seed) = seed(&synthetic_input, "left");
        // The first host net of the left board stands for a host net the circuit joins.
        let joined = board_nets(&seed, "left")[0].clone();
        let one = embed_haptic(
            &mut engine,
            &seed,
            "left",
            "left-haptic",
            json!({"gnd": joined}),
        );
        let source = embed_haptic(&mut engine, &one, "left", "review/haptic", json!({}));
        let before = snapshot(&mut engine);
        let reply = create(
            &mut engine,
            &source,
            "pair-right",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let document = &reply["document"];
        let rotation = map["rotation"].as_f64().unwrap();
        let mirror_joined = pairs_of_kind(document, "pair-right", "net")
            .into_iter()
            .find(|(source, _)| *source == joined)
            .expect("the joined host net has a mirror")
            .1;
        let at = |p: &Value| {
            (
                p["pose"]["at"]["x"].as_f64().unwrap(),
                p["pose"]["at"]["y"].as_f64().unwrap(),
            )
        };

        for (source_id, mirror_id) in [
            ("left-haptic", "right-haptic"),
            ("review/haptic", "right/review/haptic"),
        ] {
            let original = record(&source, source_id);
            let mirrored = record(document, mirror_id);
            let from = format!("embedded/{source_id}/");
            let to = format!("embedded/{mirror_id}/");
            let mapped = |id: &str| -> String {
                id.strip_prefix(&from)
                    .map_or_else(|| id.to_owned(), |rest| format!("{to}{rest}"))
            };
            assert_eq!(mirrored["hostBoardId"], "right", "{mirror_id}");
            assert_eq!(
                mirrored["definitionId"], original["definitionId"],
                "{mirror_id}"
            );

            let source_parts: Vec<&str> = original["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_str().unwrap())
                .collect();
            let mirror_parts: Vec<String> = source_parts.iter().map(|p| mapped(p)).collect();
            assert_eq!(
                mirrored["partIds"],
                json!(mirror_parts),
                "{mirror_id} parts"
            );
            assert!(
                mirror_parts.iter().all(|p| p.starts_with(&to)),
                "{mirror_id}: a member is outside {to}"
            );

            // Private nets move under the namespace; the joined host ground maps to the host mirror.
            let mut mirror_nets = Vec::new();
            for net_id in original["netIds"].as_array().unwrap() {
                let net_id = net_id.as_str().unwrap();
                if net_id.starts_with(&from) {
                    mirror_nets.push(mapped(net_id));
                } else {
                    assert_eq!(
                        net_id, joined,
                        "{mirror_id}: only the joined net is a host net"
                    );
                    mirror_nets.push(mirror_joined.clone());
                }
            }
            assert_eq!(mirrored["netIds"], json!(mirror_nets), "{mirror_id} nets");
            let mirror_ports: serde_json::Map<String, Value> = original["ports"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(role, net_id)| {
                    let net_id = net_id.as_str().unwrap();
                    let mirrored_net = if net_id == joined {
                        mirror_joined.clone()
                    } else {
                        mapped(net_id)
                    };
                    (role.clone(), json!(mirrored_net))
                })
                .collect();
            assert_eq!(
                mirrored["ports"],
                Value::Object(mirror_ports),
                "{mirror_id} ports"
            );

            for (source_part, mirror_part) in source_parts.iter().zip(&mirror_parts) {
                let (s, m) = (part(&source, source_part), part(document, mirror_part));
                assert_eq!(m["side"], s["side"], "{mirror_part} side");
                assert_eq!(
                    m["generatorParameters"], s["generatorParameters"],
                    "{mirror_part} parameters"
                );
                // The component definition moves with the namespace and keeps its content.
                let (source_def, mirror_def) = (
                    s["definitionId"].as_str().unwrap(),
                    mapped(s["definitionId"].as_str().unwrap()),
                );
                assert_eq!(
                    m["definitionId"],
                    json!(mirror_def),
                    "{mirror_part} definition"
                );
                let mut expected_def = definition(&source, source_def).clone();
                expected_def["id"] = json!(mirror_def);
                if expected_def["mechanicalProfile"].is_object() {
                    expected_def["mechanicalProfile"]["definitionId"] = json!(mirror_def);
                }
                assert_eq!(
                    definition(document, &mirror_def),
                    &expected_def,
                    "{mirror_part} definition copy"
                );
                // Position through the pair map, heading by the full rule (180 - r on an axis map).
                let source_at = at(s);
                let expected_at = reflected_point(&map, source_at);
                assert_point(at(m), expected_at, mirror_part);
                let heading = s["pose"]["rotation"].as_f64().unwrap();
                assert!(
                    angles_match(
                        m["pose"]["rotation"].as_f64().unwrap(),
                        rotation + 180.0 - heading
                    ),
                    "{mirror_part}: {} from {heading} under map {rotation}",
                    m["pose"]["rotation"]
                );
            }

            // Each private net keeps its pins on the mirrored members.
            for net_id in original["netIds"].as_array().unwrap() {
                let net_id = net_id.as_str().unwrap();
                if !net_id.starts_with(&from) {
                    continue;
                }
                let pins: Vec<Value> = net(&source, net_id)["pins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pin| json!({"partId": mapped(pin["partId"].as_str().unwrap()), "padId": pin["padId"]}))
                    .collect();
                assert_eq!(
                    net(document, &mapped(net_id))["pins"],
                    json!(pins),
                    "{mapped_net} pins",
                    mapped_net = mapped(net_id)
                );
            }
            // The mirror of the joined host net carries every member pin this circuit puts on it.
            if original["ports"]["gnd"] == joined.as_str() {
                let mirror_pins = net(document, &mirror_joined)["pins"].as_array().unwrap();
                for pin in net(&source, &joined)["pins"].as_array().unwrap() {
                    let partid = pin["partId"].as_str().unwrap();
                    if partid.starts_with(&from) {
                        let expected = json!({"partId": mapped(partid), "padId": pin["padId"]});
                        assert!(
                            mirror_pins.contains(&expected),
                            "{mirror_id}: {expected} on the mirror of the joined net"
                        );
                    }
                }
            }
            assert!(
                pairs_of_kind(document, "pair-right", "embedded-circuit")
                    .contains(&(source_id.to_owned(), mirror_id.to_owned())),
                "{mirror_id} correspondence"
            );
        }

        // No mirror id leaves the namespace, and the part and circuit correspondences are recorded.
        assert!(
            !document.to_string().contains("right/embedded/"),
            "a mirror id left the namespace"
        );
        assert_eq!(
            pairs_of_kind(document, "pair-right", "embedded-circuit"),
            [
                ("left-haptic", "right-haptic"),
                ("review/haptic", "right/review/haptic"),
            ]
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .into()
        );
        assert!(pairs_of_kind(document, "pair-right", "part").contains(&(
            "embedded/left-haptic/C1".into(),
            "embedded/right-haptic/C1".into()
        )));

        // Every new circuit member, net and definition is reported as changed.
        let changed: BTreeSet<String> = reply["scene"]["changedIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        for id in [
            "right-haptic",
            "right/review/haptic",
            "embedded/right-haptic/C1",
        ] {
            assert!(changed.contains(id), "{id} missing from {changed:?}");
        }
        // The whole edit is one Undo step, and the record reopens as a valid document.
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = before["revision"].clone();
        assert_eq!(undone, before);
        let (_, reopened) = open(document);
        assert_eq!(record(&reopened, "right-haptic")["hostBoardId"], "right");
    }
}

/// A module the mirror cannot place names the module and changes nothing: a pin to an instance
/// other than the source, a connector off the source board, and an upstream module off it.

#[test]
fn a_module_the_mirror_cannot_place_refuses_the_record_and_changes_nothing() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    type ModuleEdit = fn(&mut Value);
    let cases: [(&str, ModuleEdit, &str); 3] = [
        (
            "pinned elsewhere",
            |m| m["hostInstanceId"] = json!("elsewhere"),
            "elsewhere",
        ),
        (
            "connector off the board",
            |m| m["connection"]["hostConnectorPartId"] = json!("other/TRRS"),
            "other/TRRS",
        ),
        (
            "upstream off the board",
            |m| m["connection"]["upstreamModuleId"] = json!("other/splitter-above"),
            "other/splitter-above",
        ),
    ];
    for (label, edit, named) in cases {
        let document = with_module_edit(&source, edit);
        let (mut engine, seed) = open(&document);
        let error = refused(
            &mut engine,
            &seed,
            operation(
                "pair-right",
                "left",
                new_board("right", "right"),
                synthetic_mapping(&synthetic_input),
            ),
        );
        assert!(
            error.contains("splitter-below") && error.contains(named),
            "{label}: {error}"
        );
    }
}

/// Reuse-board copies no module: a module hosted on the board is reflected by the flipped view
/// (resolve), so the record holds no module correspondence. A module pinned to the source
/// instance is not visible from the flipped instance, so the operation refuses it by name.

#[test]
fn reuse_board_copies_no_module_and_refuses_one_pinned_to_the_source() {
    let synthetic_input = synthetic_half::document();
    let mut unpinned = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut unpinned, "left", "left");
    unpinned["modules"][0]
        .as_object_mut()
        .unwrap()
        .remove("hostInstanceId");
    let (mut engine, seed) = open(&unpinned);
    let reply = create(
        &mut engine,
        &seed,
        "pair-reuse",
        "left",
        json!({"kind":"reuse-board","instanceId":"right"}),
        mapping(0.0),
    )
    .unwrap();
    let document = &reply["document"];
    assert_eq!(document["modules"], seed["modules"]);
    assert!(module_pairs(document, "pair-reuse").is_empty());

    let mut pinned = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut pinned, "left", "left");
    let (mut engine, seed) = open(&pinned);
    let error = refused(
        &mut engine,
        &seed,
        operation(
            "pair-reuse",
            "left",
            json!({"kind":"reuse-board","instanceId":"right"}),
            mapping(0.0),
        ),
    );
    assert!(
        error.contains("left/splitter-above") && error.contains("reuse-board"),
        "{error}"
    );
}

#[test]
fn each_handed_mirror_has_one_warning_and_symmetric_footprints_have_none() {
    let mut source = synthetic_half::document();
    source["definitions"].as_array_mut().unwrap().push(json!({
        "id":"synthetic/handed-module", "name":"Handed module", "kind":"connector",
        "courtyard":[{"x":-2,"y":-1},{"x":3,"y":-1},{"x":3,"y":1},{"x":-2,"y":1}],
        "pads":[{"id":"1","number":"1","at":{"x":-1,"y":0},"size":{"x":1,"y":1},"shape":"circle"}],
        "generator":{"source":"synthetic/handed-module","version":"1","parameters":{}}
    }));
    for (id, y) in [("left/JH1", 15.0), ("left/JH2", 24.0)] {
        source["parts"].as_array_mut().unwrap().push(json!({
            "id":id, "definitionId":"synthetic/handed-module", "reference":id,
            "pose":{"at":{"x":-30,"y":y},"rotation":17}, "side":"front"
        }));
        source["boards"][0]["partIds"]
            .as_array_mut()
            .unwrap()
            .push(json!(id));
    }
    let (mut engine, seed) = open(&source);
    let reply = create(
        &mut engine,
        &seed,
        "pair-two-handed",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let mut warnings: Vec<_> = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .ends_with("/heading-uncertain")
        })
        .cloned()
        .collect();
    warnings.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    assert_eq!(
        warnings
            .iter()
            .map(|finding| finding["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "mirror/right/JH1/heading-uncertain",
            "mirror/right/JH2/heading-uncertain"
        ]
    );
    for (warning, target) in warnings.iter().zip(["right/JH1", "right/JH2"]) {
        assert_eq!(warning["severity"], "warning");
        assert_eq!(warning["scope"], "pcb");
        assert_eq!(warning["targetIds"], json!([target]));
    }
    assert!(
        !warnings
            .iter()
            .any(|warning| warning["id"] == "mirror/right/RST/heading-uncertain")
    );
}

#[test]
fn companion_placement_selectors_match_the_authored_cells() {
    let item =
        json!({"kind":"item","itemKind":"part","mirrorId":"matrix/left-synthetic-keys/r0c1/diode"});
    let row = json!({"kind":"row","matrixId":"left-synthetic-keys","row":0});
    let column = json!({"kind":"column","matrixId":"left-synthetic-keys","column":1});
    let matrix = json!({"kind":"matrix","matrixId":"left-synthetic-keys"});
    let all_cells = &["r0c0", "r0c1", "r1c0", "r1c1"][..];
    for (label, targets, cells) in [
        ("item", vec![item.clone()], &["r0c1"][..]),
        ("row", vec![row.clone()], &["r0c0", "r0c1"][..]),
        ("column", vec![column], &["r0c1", "r1c1"][..]),
        ("matrix", vec![matrix.clone()], all_cells),
        (
            "overlapping",
            vec![
                item.clone(),
                row,
                json!({"kind":"column","matrixId":"right-synthetic-keys","column":1}),
                matrix,
            ],
            all_cells,
        ),
    ] {
        let overlapping = label == "overlapping";
        let (map, mode, tweak) = if overlapping {
            (
                mapping(300.0),
                "literal-mirror-with-tweak",
                json!({"x":1.25,"y":-0.5,"rotation":4.0}),
            )
        } else {
            (
                json!({"at":{"x":300.0,"y":12.0},"rotation":27.0}),
                "duplicate-position",
                Value::Null,
            )
        };
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let made = create(
            &mut engine,
            &source,
            "placement",
            "left",
            new_board("right", "right"),
            map,
        )
        .unwrap()["document"]
            .clone();
        let edit = json!({"kind":"set-companion-placement","pairId":"placement",
            "targets":targets,"mode":mode,"tweak":tweak});
        let preview = create_with(&mut engine, &made, "preview", edit.clone()).unwrap();
        assert_eq!(preview["kind"], "preview", "{label}");
        assert_eq!(snapshot(&mut engine), made, "{label} Preview isolation");
        let reply = create_with(&mut engine, &made, "commit", edit).unwrap();
        let links = pair(&reply["document"], "placement")["halves"]["correspondences"]
            .as_array()
            .unwrap();

        // Build the expected identities from the fixture's cell coordinates and assembly IDs.
        let selected: BTreeSet<_> = cells
            .iter()
            .flat_map(|cell| {
                let children = if label == "item" {
                    &["diode"][..]
                } else {
                    &["diode", "led"][..]
                };
                children.iter().map(move |child| format!("{cell}/{child}"))
            })
            .collect();
        let updated: BTreeSet<_> = links
            .iter()
            .filter(|link| link["companionMode"] == mode)
            .map(|link| {
                let suffix = link["sourceId"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("matrix/left-synthetic-keys/")
                    .unwrap();
                assert_eq!(
                    link["mirrorId"],
                    format!("matrix/right-synthetic-keys/{suffix}"),
                    "{label}"
                );
                assert_eq!(link["companionTweak"], tweak, "{label}: {suffix}");
                suffix.to_owned()
            })
            .collect();
        assert_eq!(updated, selected, "{label}");
        for suffix in &selected {
            let (cell, _) = suffix.split_once('/').unwrap();
            let source_key = part(&made, &format!("matrix/left-synthetic-keys/{cell}"));
            let source_child = part(&made, &format!("matrix/left-synthetic-keys/{suffix}"));
            let mirror_key = part(
                &reply["document"],
                &format!("matrix/right-synthetic-keys/{cell}"),
            );
            let mirror_child = part(
                &reply["document"],
                &format!("matrix/right-synthetic-keys/{suffix}"),
            );
            let (x, y) = in_switch_frame(source_child, source_key);
            let expected = if overlapping {
                (1.25 - x, y - 0.5)
            } else {
                (x, y)
            };
            assert_close_pair(in_switch_frame(mirror_child, mirror_key), expected, suffix);
        }
        assert!(
            links
                .iter()
                .any(|link| link["sourceId"] == "left/U1" && link.get("companionMode").is_none()),
            "{label}"
        );

        if overlapping {
            let mut changed: Vec<_> = selected
                .iter()
                .map(|suffix| format!("matrix/right-synthetic-keys/{suffix}"))
                .chain(["placement".into(), "right-synthetic-keys".into()])
                .collect();
            changed.sort();
            for phase in [&preview, &reply] {
                assert_eq!(phase["scene"]["changedIds"], json!(changed), "{label}");
            }
            let refused = create_with(
                &mut engine,
                &reply["document"],
                "commit",
                json!({"kind":"set-companion-placement","pairId":"placement",
                    "targets":[{"kind":"pair"},item],
                    "mode":"literal-mirror","tweak":{"x":1.0,"y":0.0,"rotation":0.0}}),
            );
            assert_eq!(
                refused.unwrap_err(),
                "A pair default cannot carry a companion tweak"
            );
            assert_eq!(snapshot(&mut engine), reply["document"]);
        }
    }
}

#[test]
fn changing_pair_default_preserves_explicit_and_inherited_tweaks() {
    const SELECTED: &str = "matrix/right-synthetic-keys/r0c0/diode";
    const NEIGHBOR: &str = "matrix/left-synthetic-keys/r0c1/diode";
    let (mut creator, source) = seed(&runtime_supported_document(), "left");
    let made = create(
        &mut creator,
        &source,
        "placement-default",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap()["document"]
        .clone();
    for inherited in [false, true] {
        let mut configured = made.clone();
        let halves = &mut configured["hardware"]["pairs"][0]["halves"];
        let mut targets = vec![json!({"kind":"pair"})];
        if inherited {
            halves["companionMode"] = json!("literal-mirror-with-tweak");
        } else {
            targets.push(json!({"kind":"item","itemKind":"part","mirrorId":NEIGHBOR}));
        }
        let selected = halves["correspondences"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|link| link["mirrorId"] == SELECTED)
            .unwrap();
        if inherited {
            selected.as_object_mut().unwrap().remove("companionMode");
        } else {
            selected["companionMode"] = json!("literal-mirror-with-tweak");
        }
        let tweak = if inherited {
            json!({"x":0.75,"y":-0.25,"rotation":3.0})
        } else {
            json!({"x":1.25,"y":-0.5,"rotation":4.0})
        };
        selected["companionTweak"] = tweak.clone();
        let (mut engine, mut before) = open(&configured);
        let reply = create_with(
            &mut engine,
            &before,
            "commit",
            json!({"kind":"set-companion-placement","pairId":"placement-default",
                "targets":targets,"mode":"duplicate-position","tweak":null}),
        )
        .unwrap();
        let halves = &pair(&reply["document"], "placement-default")["halves"];
        assert_eq!(halves["companionMode"], "duplicate-position");
        let links = halves["correspondences"].as_array().unwrap();
        let selected = links
            .iter()
            .find(|link| link["mirrorId"] == SELECTED)
            .unwrap();
        assert_eq!(
            selected["companionMode"], "literal-mirror-with-tweak",
            "inherited={inherited}"
        );
        assert_eq!(selected["companionTweak"], tweak);
        if !inherited {
            let neighbor = links
                .iter()
                .find(|link| link["sourceId"] == NEIGHBOR)
                .unwrap();
            assert_eq!(neighbor["companionMode"], "duplicate-position");
        }
        let undone = request(&mut engine, json!({"id":"undo-placement","kind":"undo"}));
        before["revision"] = undone["document"]["revision"].clone();
        assert_eq!(undone["document"], before);
    }
}

#[test]
fn invalid_companion_modes_and_tweaks_are_refused_without_mutation() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let tweak = json!({"x":1.0,"y":2.0,"rotation":3.0});
    for mode in ["literal-mirror", "duplicate-position", "not-a-mode"] {
        let mut op = operation(
            "pair-mode",
            "left",
            new_board("right", "right"),
            mapping(300.0),
        );
        op["companionMode"] = json!(mode);
        op["companionTweak"] = tweak.clone();
        let error = refused(&mut engine, &source, op);
        assert!(
            error.contains("companion") || error.contains("unknown variant"),
            "{error}"
        );
    }
    let made = create(
        &mut engine,
        &source,
        "pair-mode",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap()["document"]
        .clone();
    for exception in [false, true] {
        for mode in [
            None,
            Some("literal-mirror"),
            Some("duplicate-position"),
            Some("unknown"),
        ] {
            let mut invalid = made.clone();
            let record = &mut invalid["hardware"]["pairs"][0]["halves"];
            let index = record["correspondences"]
                .as_array()
                .unwrap()
                .iter()
                .position(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c0/diode")
                .unwrap();
            let item = if exception {
                let mut item = record["correspondences"]
                    .as_array_mut()
                    .unwrap()
                    .remove(index);
                item.as_object_mut().unwrap().remove("sourceId");
                item["state"] = json!("local");
                record["exceptions"] = json!([item]);
                &mut record["exceptions"][0]
            } else {
                &mut record["correspondences"][index]
            };
            if let Some(mode) = mode {
                item["companionMode"] = json!(mode);
            }
            item["companionTweak"] = tweak.clone();
            let error = refused(
                &mut engine,
                &made,
                json!({"kind":"replace-document","document":invalid}),
            );
            assert!(
                error.contains("companion") || error.contains("unknown variant"),
                "{error}"
            );
        }
    }
    // The typed boundary can carry non-finite floats, even though JSON cannot.
    for field in ["x", "y", "rotation"] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut doc: ProjectDoc = serde_json::from_value(made.clone()).unwrap();
            let record = doc.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
            record.companion_mode =
                boardstudio_core::model::MirroredCompanionMode::LiteralMirrorWithTweak;
            let mut tweak = boardstudio_core::model::MirroredCompanionTweak {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            };
            match field {
                "x" => tweak.x = value,
                "y" => tweak.y = value,
                _ => tweak.rotation = value,
            }
            record
                .correspondences
                .iter_mut()
                .find(|c| c.mirror_id.ends_with("/diode"))
                .unwrap()
                .companion_tweak = Some(tweak);
            record
                .correspondences
                .iter_mut()
                .find(|c| c.mirror_id.ends_with("/diode"))
                .unwrap()
                .companion_mode =
                Some(boardstudio_core::model::MirroredCompanionMode::LiteralMirrorWithTweak);
            let error = boardstudio_core::mechanical::halves::validate_document(&doc).unwrap_err();
            assert!(error.contains("finite"), "{error}");
        }
    }
}

#[test]
fn a_plain_led_companion_uses_its_matrix_relationship_in_every_mode() {
    for (mode, offset, heading) in [
        (None, (-2.5, 3.25), -37.0),
        (Some("literal-mirror"), (-2.5, 3.25), -37.0),
        (Some("duplicate-position"), (2.5, 3.25), 37.0),
        (Some("literal-mirror-with-tweak"), (-1.25, 1.25), -24.0),
    ] {
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let mut matrix = source["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "left-synthetic-keys")
            .unwrap()
            .clone();
        let cell = matrix["cells"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["row"] == 0 && c["column"] == 0)
            .unwrap();
        cell["rotation"] = json!(15.0);
        cell["assemblies"].as_array_mut().unwrap().push(json!({"id":"status-light","definitionId":"plain-led","offset":{"x":2.5,"y":3.25},"rotation":37.0,"side":"back"}));
        let source = create_with(&mut engine, &source, "commit", json!({"kind":"set-matrix","matrix":matrix,"definitions":[{"id":"plain-led","name":"Single colour LED","kind":"passive","pads":[],"courtyard":[]}]})).unwrap()["document"].clone();
        let mut op = operation(
            "pair-led",
            "left",
            new_board("right", "right"),
            json!({"at":{"x":310.0,"y":42.0},"rotation":27.0}),
        );
        if let Some(mode) = mode {
            op["companionMode"] = json!(mode);
        }
        if mode == Some("literal-mirror-with-tweak") {
            op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
        }
        let made = create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
        let led = part(&made, "matrix/right-synthetic-keys/r0c0/status-light");
        let switch = part(&made, "matrix/right-synthetic-keys/r0c0");
        assert_close_pair(in_switch_frame(led, switch), offset, "plain LED");
        let relative = led["pose"]["rotation"].as_f64().unwrap()
            - switch["pose"]["rotation"].as_f64().unwrap();
        assert!(
            (relative - heading).abs() < 1e-9,
            "plain LED in {mode:?}: {relative} != {heading}"
        );
        let assembly = made["matrices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "right-synthetic-keys")
            .unwrap()["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["row"] == 0 && c["column"] == 0)
            .unwrap()["assemblies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "status-light")
            .unwrap();
        assert_close_pair(
            (
                assembly["offset"]["x"].as_f64().unwrap(),
                assembly["offset"]["y"].as_f64().unwrap(),
            ),
            offset,
            "stored plain LED assembly",
        );
        assert!((assembly["rotation"].as_f64().unwrap() - heading).abs() < 1e-9);
        let link = pair(&made, "pair-led")["halves"]["correspondences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["mirrorId"] == led["id"])
            .unwrap();
        assert_eq!(
            link["companionTweak"].is_object(),
            mode == Some("literal-mirror-with-tweak")
        );
    }
}

#[test]
fn companion_overrides_and_tweaks_survive_linked_local_and_omitted_records() {
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let made = create(
        &mut engine,
        &source,
        "pair-mode",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap()["document"]
        .clone();
    for default in ["duplicate-position", "literal-mirror-with-tweak"] {
        for state in ["linked", "local", "omitted"] {
            let mut document = made.clone();
            let halves = &mut document["hardware"]["pairs"][0]["halves"];
            halves["companionMode"] = json!(default);
            let links = halves["correspondences"].as_array_mut().unwrap();
            let index = links
                .iter()
                .position(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c0/diode")
                .unwrap();
            links[index]["companionMode"] = json!("literal-mirror-with-tweak");
            links[index]["companionTweak"] = json!({"x":1.0,"y":2.0,"rotation":3.0});
            let other = links
                .iter_mut()
                .find(|c| c["mirrorId"] == "matrix/right-synthetic-keys/r0c1/diode")
                .unwrap();
            other["companionMode"] = json!("literal-mirror");
            if state != "linked" {
                let mut item = links.remove(index);
                item.as_object_mut().unwrap().remove("sourceId");
                item["state"] = json!(state);
                halves["exceptions"] = json!([item]);
            }
            let (_, reopened) = open(&document);
            assert_eq!(
                pair(&reopened, "pair-mode")["halves"],
                pair(&document, "pair-mode")["halves"]
            );
        }
    }
}

#[test]
fn production_open_repairs_native_pair_and_reports_located_finding() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-open-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let mut edited_mirror = created["document"].clone();
    let mirror = edited_mirror["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "right/U1")
        .unwrap();
    mirror["pose"]["at"]["x"] = json!(231.0);
    let mut reopened_engine = CoreEngine::new();
    let reopened = request(
        &mut reopened_engine,
        json!({"id":"runtime-reopen","kind":"open","document":edited_mirror}),
    );
    assert_eq!(reopened["kind"], "scene", "{reopened}");
    assert_eq!(
        part(&reopened["document"], "right/U1")["pose"]["at"]["x"],
        224.0
    );
    assert!(
        reopened["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"] == "mirrored-half:open-repaired:runtime-open-pair")
    );
}

#[test]
fn production_linked_diode_move_refuses_in_preview_and_commit_without_history_loss() {
    for target_id in [
        "matrix/left-synthetic-keys/r0c0/diode",
        "matrix/right-synthetic-keys/r0c0/diode",
    ] {
        let (mut engine, source) = seed(&runtime_supported_document(), "left");
        let before_creation = snapshot(&mut engine);
        let created = create(
            &mut engine,
            &source,
            "runtime-linked-diode-refusal-pair",
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let accepted = created["document"].clone();
        let target = part(&accepted, target_id);
        let position = target["pose"]["at"].clone();
        let requested = json!({
            "kind":"move-parts",
            "positions":[{"id":target_id,"at":{
                "x":position["x"].as_f64().unwrap() + 4.0,
                "y":position["y"].as_f64().unwrap() - 1.0
            }}]
        });
        let command = |phase: &str, transaction_id: &str| {
            json!({
                "id":transaction_id,
                "kind":"edit",
                "command":{
                    "baseRevision":accepted["revision"],
                    "transactionId":transaction_id,
                    "phase":phase,
                    "targetIds":[target_id],
                    "operation":requested
                }
            })
        };

        let expected_refusal =
            "Linked matrix placements cannot retain authored or constrained geometry";
        for (phase, transaction_id) in [
            ("preview", "runtime-linked-diode-preview"),
            ("commit", "runtime-linked-diode-commit"),
        ] {
            let refusal: Value =
                serde_json::from_str(&engine.request(&command(phase, transaction_id).to_string()))
                    .unwrap();
            assert_eq!(refusal["kind"], "error", "{target_id}: {refusal}");
            assert_eq!(
                refusal["message"], expected_refusal,
                "{target_id}: {refusal}"
            );
            assert_eq!(
                snapshot(&mut engine),
                accepted,
                "{target_id} {phase} refusal must preserve the full accepted document and revision"
            );
        }

        let undone = request(
            &mut engine,
            json!({"id":"runtime-linked-diode-undo","kind":"undo"}),
        )["document"]
            .clone();
        let mut expected_undo = before_creation;
        expected_undo["revision"] = undone["revision"].clone();
        assert_eq!(
            undone, expected_undo,
            "{target_id}: Undo still reaches the pre-pair document after both refusals"
        );

        let redone = request(
            &mut engine,
            json!({"id":"runtime-linked-diode-redo","kind":"redo"}),
        )["document"]
            .clone();
        let mut expected_redo = accepted;
        expected_redo["revision"] = redone["revision"].clone();
        assert_eq!(
            redone, expected_redo,
            "{target_id}: Redo still restores the accepted pair after both refusals"
        );
    }
}

#[test]
fn production_paired_noop_commit_preserves_revision_history_and_redo() {
    let (mut engine, seed) = seed(&runtime_supported_document(), "left");
    let created = create(
        &mut engine,
        &seed,
        "runtime-noop-pair",
        "left",
        new_board("right", "right"),
        mapping(200.0),
    )
    .unwrap();
    let accepted = created["document"].clone();
    let committed = request(
        &mut engine,
        json!({"id":"runtime-noop-history-setup","kind":"edit","command":{
            "baseRevision":accepted["revision"],
            "transactionId":"runtime-noop-history-setup",
            "phase":"commit",
            "targetIds":["left/U1"],
            "operation":{"kind":"move-parts","positions":[{"id":"left/U1","at":{"x":-22.0,"y":4.0}}]}
        }}),
    );
    assert_eq!(committed["kind"], "scene", "{committed}");
    let undone = request(
        &mut engine,
        json!({"id":"runtime-noop-history-undo","kind":"undo"}),
    );
    assert_eq!(
        part(&undone["document"], "left/U1")["pose"]["at"],
        json!({"x":-24.0,"y":0.0})
    );
    assert_eq!(
        part(&undone["document"], "right/U1")["pose"]["at"],
        json!({"x":224.0,"y":0.0})
    );
    let before_noop = snapshot(&mut engine);

    let noop = request(
        &mut engine,
        json!({"id":"runtime-noop-commit","kind":"edit","command":{
            "baseRevision":before_noop["revision"],
            "transactionId":"runtime-noop-commit",
            "phase":"commit",
            "targetIds":["left/U1"],
            "operation":{"kind":"apply-mechanical-edit","edit":{
                "target":{"kind":"instance","instanceId":"left"},
                "change":{"set-dimension":{"field":"wall-thickness","value":2.0}}
            }}
        }}),
    );
    assert_eq!(noop["kind"], "scene", "{noop}");
    assert_eq!(noop["document"]["revision"], before_noop["revision"]);
    assert_eq!(
        snapshot(&mut engine),
        before_noop,
        "no-op must leave document and history position unchanged"
    );

    let redone = request(
        &mut engine,
        json!({"id":"runtime-noop-history-redo","kind":"redo"}),
    );
    assert_eq!(redone["kind"], "scene", "{redone}");
    assert_eq!(
        part(&redone["document"], "left/U1")["pose"]["at"],
        json!({"x":-22.0,"y":4.0})
    );
    assert_eq!(
        part(&redone["document"], "right/U1")["pose"]["at"],
        json!({"x":222.0,"y":4.0})
    );
}
