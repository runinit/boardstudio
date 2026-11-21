use super::*;

#[test]
fn a_mirrored_mcu_keeps_its_source_heading_and_intended_origin() {
    // Issue 3: the MCU (a controller) must mirror to the plain reflection, not the +180
    // independent-part rule, so both halves share its heading and its intended origin.
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let reply = create(
        &mut engine,
        &seed,
        "pair-mcu",
        "left",
        new_board("mirror", "mirror"),
        mapping(300.0),
    )
    .unwrap();
    let document = &reply["document"];
    let source = part(document, "left/U1");
    let mirror = part(document, "mirror/U1");
    // The axis map at x = 300 sends (-24, 0) to (324, 0).
    assert_point(
        (
            mirror["pose"]["at"]["x"].as_f64().unwrap(),
            mirror["pose"]["at"]["y"].as_f64().unwrap(),
        ),
        (324.0, 0.0),
        "mirror MCU origin",
    );
    assert!(
        angles_match(
            mirror["pose"]["rotation"].as_f64().unwrap(),
            source["pose"]["rotation"].as_f64().unwrap()
        ),
        "mirror MCU shares its source heading: {} against {}",
        mirror["pose"]["rotation"],
        source["pose"]["rotation"]
    );
    // A genuine independent (the TRRS connector) still takes the half turn.
    let source_trrs = part(document, "left/TRRS");
    let mirror_trrs = part(document, "mirror/TRRS");
    assert_point(
        (
            mirror_trrs["pose"]["at"]["x"].as_f64().unwrap(),
            mirror_trrs["pose"]["at"]["y"].as_f64().unwrap(),
        ),
        (324.0, -16.0),
        "mirror TRRS origin",
    );
    assert!(
        angles_match(
            mirror_trrs["pose"]["rotation"].as_f64().unwrap(),
            180.0 - source_trrs["pose"]["rotation"].as_f64().unwrap()
        ),
        "mirror TRRS keeps the half turn: {}",
        mirror_trrs["pose"]["rotation"]
    );
}

#[test]
fn pairing_projects_authored_openings_and_closure_holes_to_the_mirror() {
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
    // Authored features move into pair ownership and remain effective on both halves.
    let record = pair(document, "pair-right");
    assert_eq!(record["features"].as_array().unwrap().len(), 2);
    for half in ["left", "right"] {
        let stored = &instance(document, half)["mechanical"];
        assert_eq!(stored["closureMounts"], json!([]), "{half}");
        assert_eq!(stored["openings"], json!([]), "{half}");
        assert_eq!(stored["boardId"], half, "{half}");
        assert_eq!(instance(document, half)["constructionOwnership"], "pair");
    }
    let source = effective(document, "left");
    let mirror = effective(document, "right");
    assert_eq!(source["openings"].as_array().unwrap().len(), 1);
    assert_eq!(mirror["openings"].as_array().unwrap().len(), 1);
    assert_eq!(closure_positions(&source), vec![(3.0, 5.0)]);
    assert_eq!(closure_positions(&mirror), vec![(197.0, 5.0)]);
    let opening_points = |configuration: &Value| {
        let mut points: Vec<_> = configuration["openings"][0]["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|vertex| {
                (
                    vertex["point"]["x"].as_f64().unwrap(),
                    vertex["point"]["y"].as_f64().unwrap(),
                )
            })
            .collect();
        points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        points
    };
    assert_eq!(
        opening_points(&source),
        vec![(-4.0, -3.0), (-4.0, 3.0), (4.0, -3.0), (4.0, 3.0)]
    );
    assert_eq!(
        opening_points(&mirror),
        vec![(196.0, -3.0), (196.0, 3.0), (204.0, -3.0), (204.0, 3.0)]
    );
    assert_eq!(closure_holes(document, "right").len(), 1);
    assert_eq!(closure_holes(document, "left").len(), 1);
    let source_holes: BTreeSet<String> = closure_holes(document, "left")
        .iter()
        .map(|id| id.replace("/left/", "/right/"))
        .collect();
    assert!(source_holes.is_disjoint(&closure_holes(document, "right")));
    for hole in closure_holes(document, "right") {
        let board = document["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == "right")
            .unwrap();
        assert!(
            board["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| *id == hole),
            "{hole}"
        );
    }
    for hole in closure_holes(document, "left") {
        let board = document["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["id"] == "left")
            .unwrap();
        assert!(
            board["partIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| *id == hole),
            "{hole}"
        );
    }
}

#[test]
fn one_undo_step_removes_the_half_and_redo_restores_it() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let before = snapshot(&mut engine);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let made = reply["document"]["revision"].as_u64().unwrap();
    assert_eq!(made, before["revision"].as_u64().unwrap() + 1);
    let mut undone = request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
    assert_eq!(undone["revision"].as_u64().unwrap(), made + 1);
    undone["revision"] = before["revision"].clone();
    assert_eq!(undone, before);
    let mut redone = request(&mut engine, json!({"id":"redo","kind":"redo"}))["document"].clone();
    redone["revision"] = reply["document"]["revision"].clone();
    assert_eq!(redone, reply["document"]);
    // The produced record reopens as a valid document.
    let (_, reopened) = open(&reply["document"]);
    assert_eq!(pair(&reopened, "pair-right")["constructionSource"], "left");
    assert_eq!(
        pair(&reopened, "pair-right")["halves"]["mirrorBoardId"],
        "right"
    );
}

#[test]
fn a_closure_hole_without_a_catalogue_definition_is_refused_atomically() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let mut op = operation(
        "pair-no-hole",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    );
    op["mountingHole"] = Value::Null;
    let error = refused(&mut engine, &seed, op);
    assert!(error.to_lowercase().contains("mounting"), "{error}");
}

#[test]
fn a_preview_changes_nothing_and_a_commit_still_follows() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let before = serde_json::to_string(&snapshot(&mut engine)).unwrap();
    let op = operation(
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    );
    let preview = create_with(&mut engine, &seed, "preview", op.clone()).unwrap();
    assert_eq!(preview["kind"], "preview");
    assert_eq!(
        serde_json::to_string(&snapshot(&mut engine)).unwrap(),
        before
    );
    let committed = create_with(&mut engine, &seed, "commit", op).unwrap();
    assert_eq!(committed["kind"], "scene");
}

#[test]
fn open_refuses_ambiguous_native_mirror_board_ownership_without_publication() {
    let synthetic_input = synthetic_half::with_construction_features(synthetic_half::document());
    let (mut source_engine, seed) = seed(&synthetic_input, "left");
    let created = create(
        &mut source_engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&synthetic_input),
    )
    .unwrap();
    let mut ambiguous = created["document"].clone();
    let mut extra_board = ambiguous["boards"][0].clone();
    extra_board["id"] = json!("board-extra");
    extra_board["name"] = json!("Extra source");
    extra_board["outlineIds"] = json!([]);
    extra_board["partIds"] = json!([]);
    extra_board["netIds"] = json!([]);
    ambiguous["boards"]
        .as_array_mut()
        .unwrap()
        .push(extra_board);
    let mut extra_source = ambiguous["hardware"]["instances"][0].clone();
    extra_source["id"] = json!("instance-extra-source");
    extra_source["name"] = json!("Extra source");
    extra_source["boardId"] = json!("board-extra");
    let mut extra_mirror = ambiguous["hardware"]["instances"][1].clone();
    extra_mirror["id"] = json!("instance-extra-mirror");
    extra_mirror["name"] = json!("Extra mirror");
    ambiguous["hardware"]["instances"]
        .as_array_mut()
        .unwrap()
        .extend([extra_source, extra_mirror]);
    let mut duplicate_pair = ambiguous["hardware"]["pairs"][0].clone();
    duplicate_pair["id"] = json!("ambiguous-pair");
    duplicate_pair["members"] = json!(["instance-extra-source", "instance-extra-mirror"]);
    duplicate_pair["constructionSource"] = json!("instance-extra-source");
    duplicate_pair["features"] = json!([]);
    duplicate_pair["halves"]["sourceBoardId"] = json!("board-extra");
    ambiguous["hardware"]["pairs"]
        .as_array_mut()
        .unwrap()
        .push(duplicate_pair);

    let mut engine = CoreEngine::new();
    let empty_before: Value = serde_json::from_str(
        &engine
            .request(&json!({"id":"snapshot-before-ambiguous-open","kind":"snapshot"}).to_string()),
    )
    .unwrap();
    let open_reply: Value =
        serde_json::from_str(&engine.request(
            &json!({"id":"ambiguous-open","kind":"open","document":ambiguous}).to_string(),
        ))
        .unwrap();
    assert_eq!(open_reply["kind"], "error", "{open_reply}");
    let snapshot_reply: Value = serde_json::from_str(
        &engine
            .request(&json!({"id":"snapshot-after-rejected-open","kind":"snapshot"}).to_string()),
    )
    .unwrap();
    assert_eq!(
        snapshot_reply["document"], empty_before["document"],
        "the rejected Open publishes no document"
    );
}

#[test]
fn outlines_reflect_anchored_geometry_in_the_part_frame_and_keep_unique_identities() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    let board = json!({"kind": "board", "boardId": "left"});
    let live = json!({"kind": "board", "boardId": "left"});
    // Live outline: an authored connection and a kept gap, some points attached to the controller.
    document = outline_edit(
        &mut engine,
        &document,
        board.clone(),
        json!({"add-connection": {"connection": {
        "id": "conn-live", "width": 3.0,
        "points": [{"at": {"x": 4.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 30.0, "y": -20.0}}]}}}),
        1,
    );
    document = outline_edit(
        &mut engine,
        &document,
        live,
        json!({"set-protected-gap": {"gap_id": "gap-live", "points": [
        {"at": {"x": 1.0, "y": 2.0}, "partId": "left/U1"}, {"at": {"x": 6.0, "y": 2.0}, "partId": "left/U1"},
        {"at": {"x": 6.0, "y": 8.0}, "partId": "left/U1"}]}}),
        2,
    );
    // A fixed copy with an anchored cutout and an anchored polygon.
    document = outline_edit(
        &mut engine,
        &document,
        board,
        json!({"copy": {"version_id": "fixed", "name": "Fixed"}}),
        3,
    );
    let version = json!({"kind": "version", "boardId": "left", "versionId": "fixed"});
    document = outline_edit(
        &mut engine,
        &document,
        version.clone(),
        json!({"add-feature": {"feature": {
        "kind": "polygon", "id": "cut-poly", "anchorPartId": "left/U1",
        "points": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}, {"x": 5.0, "y": 7.0}], "operation": "subtract"}}}),
        5,
    );
    document = outline_edit(
        &mut engine,
        &document,
        version,
        json!({"set-protected-gap": {"gap_id": "gap-fixed", "points": [
        {"at": {"x": 40.0, "y": -40.0}}, {"at": {"x": 50.0, "y": -40.0}}, {"at": {"x": 50.0, "y": -30.0}}]}}),
        6,
    );
    // A version-local edit can only add polygons; an anchored cutout reaches a version from older
    // data, so it is placed in the stored document directly.
    document["boardOutlines"][0]["versions"][0]["geometry"]["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind": "rect", "id": "cut-rect", "anchorPartId": "left/RST", "center": {"x": 2.0, "y": 3.0},
            "size": {"x": 4.0, "y": 5.0}, "radius": 0.0, "rotation": 20.0, "operation": "subtract"}));
    let (mut engine, seed_doc) = open(&document);
    let map = json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0});
    let reply = create(
        &mut engine,
        &seed_doc,
        "pair-right",
        "left",
        new_board("right", "right"),
        map.clone(),
    )
    .unwrap();
    let out = &reply["document"];
    let find = |list: &Value, id: &str| {
        list.as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == id)
            .unwrap_or_else(|| panic!("missing {id}; available: {list}"))
            .clone()
    };

    // Live outline, in the part frame: U1 is a controller, so it keeps the reflected heading
    // and its attached point is (-x, y) in that frame; the free one maps.
    let live = find(&out["outline"], "right/synthetic-envelope");
    let connection = &live["connections"][0];
    assert_eq!(connection["points"][0]["partId"], "right/U1");
    assert_eq!(connection["points"][0]["at"], json!({"x": -4.0, "y": 2.0}));
    assert_point(
        world_of(out, "right/U1", &connection["points"][0]["at"]),
        reflected_point(&map, world_of(out, "left/U1", &json!({"x": 4.0, "y": 2.0}))),
        "attached connection point",
    );
    let free = &connection["points"][1];
    assert!(free["partId"].is_null());
    assert_point(
        (
            free["at"]["x"].as_f64().unwrap(),
            free["at"]["y"].as_f64().unwrap(),
        ),
        reflected_point(&map, (30.0, -20.0)),
        "free connection point",
    );
    let gap = &live["settings"]["repair"]["keepGaps"][0];
    for (index, source) in [
        (0, json!({"x": 1.0, "y": 2.0})),
        (1, json!({"x": 6.0, "y": 2.0})),
        (2, json!({"x": 6.0, "y": 8.0})),
    ] {
        assert_eq!(gap["points"][index]["partId"], "right/U1");
        assert_point(
            world_of(out, "right/U1", &gap["points"][index]["at"]),
            reflected_point(&map, world_of(out, "left/U1", &source)),
            "kept gap point",
        );
    }

    // The fixed copy: anchored cutouts reflect in the part frame, world geometry through the map.
    let state = out["boardOutlines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["boardId"] == "right")
        .unwrap();
    assert_eq!(state["activeVersionId"], "right/fixed");
    let geometry = &state["versions"][0]["geometry"];
    let rect = find(&geometry["features"], "right/cut-rect");
    assert_eq!(rect["anchorPartId"], "right/RST");
    assert_point(
        world_of(out, "right/RST", &rect["center"]),
        reflected_point(
            &map,
            world_of(out, "left/RST", &json!({"x": 2.0, "y": 3.0})),
        ),
        "anchored rect centre",
    );
    assert_eq!(rect["rotation"].as_f64().unwrap(), -20.0);
    let polygon = find(&geometry["features"], "right/cut-poly");
    let source_points = [(0.0, 0.0), (5.0, 0.0), (5.0, 7.0)];
    let mirrored_world: Vec<(f64, f64)> = polygon["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| world_of(out, "right/U1", p))
        .collect();
    for (x, y) in source_points {
        let expected = reflected_point(&map, world_of(out, "left/U1", &json!({"x": x, "y": y})));
        assert!(
            mirrored_world
                .iter()
                .any(|w| (w.0 - expected.0).abs() < 1e-9 && (w.1 - expected.1).abs() < 1e-9),
            "{x},{y}"
        );
    }
    // Reflection reverses the winding, so the points come back in the reverse order. U1 is a
    // controller with the reflected heading, so the local point is (-x, y), not (x, -y).
    assert_eq!(polygon["points"][0], json!({"x": -5.0, "y": 7.0}));
    let fixed_gap = &geometry["protectedGaps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "right/gap-fixed")
        .unwrap();
    assert_point(
        (
            fixed_gap["points"][0]["at"]["x"].as_f64().unwrap(),
            fixed_gap["points"][0]["at"]["y"].as_f64().unwrap(),
        ),
        reflected_point(&map, (40.0, -40.0)),
        "protected gap",
    );

    // One namespace: the only repeated identity is the gap kept on the live outline and
    // protected in the fixed copy, which is one gap with one identity on each half.
    let ids = outline_identities(out);
    let repeated: BTreeSet<&String> = ids
        .iter()
        .filter(|id| ids.iter().filter(|other| other == id).count() > 1)
        .collect();
    let expected: BTreeSet<String> = ["gap-live", "right/gap-live"].map(str::to_owned).into();
    assert_eq!(
        repeated,
        expected.iter().collect::<BTreeSet<_>>(),
        "{ids:?}"
    );
    assert!(ids.iter().any(|id| id == "right/conn-live") && ids.iter().any(|id| id == "conn-live"));
    assert_eq!(
        find(&out["outline"], "left/synthetic-envelope"),
        find(&seed_doc["outline"], "left/synthetic-envelope")
    );
}

/// Reuse-board copies no circuit. An embedded circuit belongs to its board, not to an instance (the
/// model has no instance pin), so the flipped instance shows it without a copy.

#[test]
fn reuse_board_copies_no_embedded_circuit_because_it_belongs_to_the_board() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, seed) = seed(&synthetic_input, "left");
    let with_circuit = embed_haptic(&mut engine, &seed, "left", "left-haptic", json!({}));
    let before = snapshot(&mut engine);
    let reply = create(
        &mut engine,
        &with_circuit,
        "pair-flip",
        "left",
        json!({"kind":"reuse-board","instanceId":"flipped"}),
        json!({"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}),
    )
    .unwrap();
    let document = &reply["document"];
    assert_eq!(document["embeddedCircuits"], before["embeddedCircuits"]);
    let circuit_parts = |doc: &Value| -> Vec<Value> {
        doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"].as_str().unwrap().starts_with("embedded/"))
            .cloned()
            .collect()
    };
    assert_eq!(circuit_parts(document), circuit_parts(&before));
    assert!(instance(document, "flipped")["flipped"].as_bool().unwrap());
    assert!(pairs_of_kind(document, "pair-flip", "embedded-circuit").is_empty());
}

/// A mirrored module is one Warning per module, located on the mirror, asking a person to confirm
/// its handedness. A source module, or a module that is not part of a pair, raises none.

#[test]
fn every_mirrored_module_raises_one_handed_module_warning_and_a_source_module_raises_none() {
    let synthetic_input = synthetic_half::document();
    let mut source = seed_document(&synthetic_input, "left");
    add_two_splitters(&mut source, "left", "left");
    let opened: Value = serde_json::from_str(
        &CoreEngine::new().request(&json!({"id":"o","kind":"open","document":source}).to_string()),
    )
    .unwrap();
    let handed = |scene: &Value| -> Vec<Value> {
        scene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["id"].as_str().unwrap().ends_with("/handed-module"))
            .cloned()
            .collect()
    };
    assert!(
        handed(&opened["scene"]).is_empty(),
        "a source module alone is not a finding"
    );

    let (mut engine, seed) = open(&source);
    let reply = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        mapping(300.0),
    )
    .unwrap();
    let findings = handed(&reply["scene"]);
    let mut ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "mirror/right/splitter-above/handed-module",
            "mirror/right/splitter-below/handed-module",
        ]
    );
    for finding in &findings {
        assert_eq!(finding["severity"], "warning");
        assert_eq!(finding["scope"], "pcb");
        let target = finding["targetIds"].as_array().unwrap();
        assert_eq!(target.len(), 1);
        assert!(target[0].as_str().unwrap().starts_with("right/"));
    }
}

/// A key switch takes the pair's heading rule with no half turn: a switch tilted by `r` mirrors to
/// `forward_angle(map, r)`, which is `-r` on an axis map (the two rules differ whenever the map
/// rotation is not 0 modulo 360, which the 27 degree cases cover). A companion is mirrored in
/// position and takes the negated heading: in the switch's own frame its local x flips and its
/// local y does not. A `180 - r` rule would move the LED to the other side, which this test catches.
///
/// Literal-mirror is the default: the diode's authored source-local offset becomes its reflected
/// offset. Duplicate-position keeps the original local offset;
/// that optional mode is tested separately below.

#[test]
fn key_switches_mirror_their_tilt_without_a_half_turn_and_keep_their_companion_side() {
    let cases: [(&str, Option<f64>); 2] = [("synthetic", None), ("synthetic", Some(27.0))];
    for (name, turn) in cases {
        let synthetic_input = &synthetic_half::document();
        let (mut engine, seed) = seed(synthetic_input, "left");
        // Two switches carry a 15 and a -20 degree cell turn (the other two stay flat), so the
        // tilts are not the zero the synthetic_input matrices use.
        let tilted = with_cell_turns(
            &mut engine,
            &seed,
            "left-synthetic-keys",
            &[(0, 0, 15.0), (1, 0, 0.0), (0, 1, -20.0), (1, 1, 0.0)],
        );
        let map = match turn {
            None => mapping(300.0),
            Some(degrees) => json!({"at": {"x": 310.0, "y": 42.0}, "rotation": degrees}),
        };
        let label = format!("{name} with map rotation {:?}", turn.unwrap_or(0.0));
        let reply = create(
            &mut engine,
            &tilted,
            "pair-tilt",
            "left",
            new_board("right", "right"),
            map.clone(),
        )
        .unwrap();
        let mirrored = &reply["document"];
        let source =
            |r: u64, c: u64| part(&tilted, &format!("matrix/left-synthetic-keys/r{r}c{c}"));
        let mirror =
            |r: u64, c: u64| part(mirrored, &format!("matrix/right-synthetic-keys/r{r}c{c}"));
        let tilt = |r: u64, c: u64| source(r, c)["pose"]["rotation"].as_f64().unwrap();
        assert!(
            angles_match(tilt(0, 0) - tilt(1, 0), 15.0)
                && angles_match(tilt(0, 1) - tilt(1, 1), -20.0),
            "{label}: the synthetic source switches must carry the 15 and -20 degree turns"
        );
        for (r, c) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (position, heading) = mapped_pose(&map, &source(r, c)["pose"]);
            assert_close_pair(
                (
                    mirror(r, c)["pose"]["at"]["x"].as_f64().unwrap(),
                    mirror(r, c)["pose"]["at"]["y"].as_f64().unwrap(),
                ),
                position,
                &format!("{label} switch r{r}c{c} position"),
            );
            assert!(
                angles_match(mirror(r, c)["pose"]["rotation"].as_f64().unwrap(), heading),
                "{label} switch r{r}c{c}: heading {} from the pair rule {heading}",
                mirror(r, c)["pose"]["rotation"]
            );
            // The authored diode accompanies every synthetic switch.
            for (suffix, local) in [("diode", (8.0, -2.0))] {
                let source_id = format!("matrix/left-synthetic-keys/r{r}c{c}/{suffix}");
                let Some(companion) = find_part(&tilted, &source_id) else {
                    continue;
                };
                let mirror_id = format!("matrix/right-synthetic-keys/r{r}c{c}/{suffix}");
                let mirror_companion = find_part(mirrored, &mirror_id)
                    .unwrap_or_else(|| panic!("{label}: no mirror part {mirror_id}"));
                let (lx, ly) = in_switch_frame(companion, source(r, c));
                assert!(
                    (lx - local.0).abs() < 1e-6 && (ly - local.1).abs() < 1e-6,
                    "{label} {source_id}: the synthetic component offset changed to ({lx}, {ly})"
                );
                let (position, heading) = mapped_pose(&map, &companion["pose"]);
                assert_close_pair(
                    (
                        mirror_companion["pose"]["at"]["x"].as_f64().unwrap(),
                        mirror_companion["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                    position,
                    &format!("{label} {mirror_id} position"),
                );
                assert!(
                    angles_match(
                        mirror_companion["pose"]["rotation"].as_f64().unwrap(),
                        heading
                    ),
                    "{label} {mirror_id}: heading from the pair rule {heading}"
                );
                let (mx, my) = in_switch_frame(mirror_companion, mirror(r, c));
                assert!(
                    (mx + local.0).abs() < 1e-6 && (my - local.1).abs() < 1e-6,
                    "{label} {mirror_id}: local ({mx}, {my}) is not the mirrored side ({}, {})",
                    -local.0,
                    local.1
                );
            }
        }
    }
}

/// Each part in the authored synthetic half maps through the pair rule, including companions.

#[test]
fn anchored_geometry_lands_on_the_mapped_world_point_at_every_heading_and_map() {
    let synthetic_input = synthetic_half::document();
    let (mut engine, mut document) = seed(&synthetic_input, "left");
    document = outline_edit(
        &mut engine,
        &document,
        json!({"kind": "board", "boardId": "left"}),
        json!({"copy": {"version_id": "fixed", "name": "Fixed"}}),
        3,
    );
    let companion = "left/TRRS";
    // U1 is a controller, so it shares the switch rule with matrix keys: only RST still
    // takes the full independent rule.
    let anchors = [
        ("left/U1", false),
        ("left/RST", true),
        ("matrix/left-synthetic-keys/r0c0", false),
        (companion, false),
    ];
    for map in [
        json!({"at": {"x": 300.0, "y": 0.0}, "rotation": 0.0}),
        json!({"at": {"x": 310.0, "y": 42.0}, "rotation": 27.0}),
    ] {
        for heading in [0.0, 90.0, 180.0, 270.0, 37.0] {
            let mut source = document.clone();
            for part in source["parts"].as_array_mut().unwrap() {
                if part["id"] == "left/U1" || part["id"] == "left/RST" {
                    part["pose"]["rotation"] = json!(heading);
                }
            }
            let features = source["boardOutlines"][0]["versions"][0]["geometry"]["features"]
                .as_array_mut()
                .unwrap();
            for (index, (anchor, _)) in anchors.iter().enumerate() {
                features.push(json!({"kind": "rect", "id": format!("cut-{index}"),
                    "anchorPartId": anchor, "center": {"x": 2.0, "y": 3.5},
                    "size": {"x": 4.0, "y": 5.0}, "radius": 0.0, "rotation": 20.0,
                    "operation": "subtract"}));
            }
            let (mut engine, seed_doc) = open(&source);
            let out = create(
                &mut engine,
                &seed_doc,
                "pair-right",
                "left",
                new_board("right", "right"),
                map.clone(),
            )
            .unwrap();
            let out = &out["document"];
            let state = out["boardOutlines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["boardId"] == "right")
                .unwrap();
            let features = &state["versions"][0]["geometry"]["features"];
            for (index, (anchor, independent)) in anchors.iter().enumerate() {
                let rect = features
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["id"] == format!("right/cut-{index}"))
                    .unwrap();
                let mirror_id = rect["anchorPartId"].as_str().unwrap();
                let at = json!({"x": 2.0, "y": 3.5});
                assert_point(
                    world_of(out, mirror_id, &rect["center"]),
                    reflected_point(&map, world_of(out, anchor, &at)),
                    &format!("{anchor} heading {heading} map {}", map["rotation"]),
                );
                if *independent {
                    // The full rule, read independently: `rotation + 180 - heading`.
                    let expected = map["rotation"].as_f64().unwrap() + 180.0 - heading;
                    assert!(
                        angles_match(
                            part(out, mirror_id)["pose"]["rotation"].as_f64().unwrap(),
                            expected
                        ),
                        "{anchor} heading {heading}: {}",
                        part(out, mirror_id)["pose"]["rotation"]
                    );
                }
            }
        }
    }
}

/// Duplicate placement retains the authored switch-local pose under either pair-map rotation.

#[test]
fn duplicate_companions_keep_the_source_local_pose() {
    for turn in [0.0, 27.0] {
        let source_fixture = synthetic_half::document();
        let (mut engine, source) = seed(&source_fixture, "left");
        let source = with_cell_turns(
            &mut engine,
            &source,
            "left-synthetic-keys",
            &[(0, 0, 15.0), (0, 1, -20.0)],
        );
        let mut op = operation(
            "pair-mode",
            "left",
            new_board("right", "right"),
            json!({"at":{"x":310.0,"y":42.0},"rotation":turn}),
        );
        op["companionMode"] = json!("duplicate-position");
        let made = create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
        assert_eq!(
            pair(&made, "pair-mode")["halves"]["companionMode"],
            "duplicate-position"
        );
        let mut count = 0;
        for link in pair(&made, "pair-mode")["halves"]["correspondences"]
            .as_array()
            .unwrap()
        {
            if link["kind"] != "part" {
                continue;
            }
            let source_id = link["sourceId"].as_str().unwrap();
            if !source_id.ends_with("/diode") {
                continue;
            }
            let source_key_id = source_id.rsplit_once('/').unwrap().0;
            let mirror_id = link["mirrorId"].as_str().unwrap();
            let mirror_key_id = mirror_id.rsplit_once('/').unwrap().0;
            let original = part(&source, source_id);
            let original_key = part(&source, source_key_id);
            let companion = part(&made, mirror_id);
            let key = part(&made, mirror_key_id);
            assert_close_pair(
                in_switch_frame(original, original_key),
                (8.0, -2.0),
                source_id,
            );
            assert_close_pair(in_switch_frame(companion, key), (8.0, -2.0), mirror_id);
            let relative = original["pose"]["rotation"].as_f64().unwrap()
                - original_key["pose"]["rotation"].as_f64().unwrap();
            let actual = companion["pose"]["rotation"].as_f64().unwrap()
                - key["pose"]["rotation"].as_f64().unwrap();
            assert!(
                (actual - relative).abs() < 1e-9,
                "{mirror_id}: {actual} != {relative}"
            );
            assert_eq!(
                companion["generatorParameters"],
                original["generatorParameters"]
            );
            assert_eq!(companion["side"], original["side"]);
            count += 1;
        }
        assert_eq!(count, 4, "one diode for each authored switch");
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = source["revision"].clone();
        assert_eq!(undone, source);
        let (_, reopened) = open(&made);
        assert_eq!(pair(&reopened, "pair-mode"), pair(&made, "pair-mode"));
    }
}

#[test]
fn typed_creation_refuses_non_finite_tweaks_without_mutation() {
    use boardstudio_core::model::{
        CoreReply, CoreRequest, EditCommand, EditOperation, MirroredCompanionTweak,
    };
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    for field in ["x", "y", "rotation"] {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut op = operation(
                "pair-finite",
                "left",
                new_board("right", "right"),
                mapping(300.0),
            );
            op["companionMode"] = json!("literal-mirror-with-tweak");
            let mut command: EditCommand = serde_json::from_value(json!({"baseRevision":source["revision"],"transactionId":"finite","phase":"commit","targetIds":[],"operation":op})).unwrap();
            let mut tweak = MirroredCompanionTweak {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            };
            match field {
                "x" => tweak.x = value,
                "y" => tweak.y = value,
                _ => tweak.rotation = value,
            }
            let EditOperation::CreateMirroredHalf {
                companion_tweak, ..
            } = &mut command.operation
            else {
                panic!("creation operation")
            };
            *companion_tweak = Some(tweak);
            match engine.handle(CoreRequest::Edit {
                id: "finite".into(),
                command,
            }) {
                CoreReply::Error { message, .. } => {
                    assert!(message.contains("finite"), "{message}")
                }
                other => panic!("Non-finite tweak accepted: {other:?}"),
            }
            assert_eq!(snapshot(&mut engine), source);
        }
    }
}

#[test]
fn creation_labels_apply_atomically_in_both_modes_and_undo_redo_together() {
    for mode in [
        new_board("right", "right"),
        json!({"kind":"reuse-board","instanceId":"right"}),
    ] {
        let (mut engine, before) = unibody_label_seed();
        let mut op = operation("pair-labels", "left", mode, mapping(0.0));
        op["labels"] = split_labels();
        let preview = create_with(&mut engine, &before, "preview", op.clone()).unwrap();
        assert_eq!(preview["kind"], "preview");
        assert_eq!(
            snapshot(&mut engine),
            before,
            "preview must not mutate the accepted source"
        );
        let reply = create_with(&mut engine, &before, "commit", op).unwrap();
        let made = &reply["document"];
        for (id, side) in [("left", "source"), ("right", "mirror")] {
            for field in ["name", "half", "role"] {
                assert_eq!(
                    instance(made, id)[field],
                    split_labels()[side][field],
                    "{id}.{field}"
                );
            }
            assert!(
                reply["scene"]["changedIds"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(id))
            );
        }
        assert_eq!(
            made["revision"].as_u64().unwrap(),
            before["revision"].as_u64().unwrap() + 1
        );
        let (_, reopened) = open(made);
        assert_eq!(
            reopened["hardware"]["instances"],
            made["hardware"]["instances"]
        );
        let mut undone =
            request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"].clone();
        undone["revision"] = before["revision"].clone();
        assert_eq!(undone, before);
        let mut redone =
            request(&mut engine, json!({"id":"redo","kind":"redo"}))["document"].clone();
        redone["revision"] = made["revision"].clone();
        assert_eq!(&redone, made);
    }
}

#[test]
fn stabilisers_keep_their_source_heading_and_mirrored_position_in_every_mode() {
    let imported: Value =
        serde_json::from_str(include_str!("../../../catalogue/parts/imported-parts.json")).unwrap();
    let definitions: Vec<Value> = imported["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["definition"].clone())
        .filter(|definition| {
            [
                "kicad:marbastlib/STAB_MX_2u",
                "kicad:marbastlib/STAB_MX_6.25u",
            ]
            .contains(&definition["id"].as_str().unwrap())
        })
        .collect();
    assert_eq!(definitions.len(), 2);
    let stabilisers: [(&str, f64); 3] = [("stab-0", 0.0), ("stab-90", 90.0), ("stab-180", 180.0)];
    let (mut engine, source) = seed(&runtime_supported_document(), "left");
    let mut matrix = source["matrices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|matrix| matrix["id"] == "left-synthetic-keys")
        .unwrap()
        .clone();
    let cell = matrix["cells"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|cell| cell["row"] == 0 && cell["column"] == 0)
        .unwrap();
    cell["rotation"] = json!(15.0);
    for (index, (id, relative_heading)) in stabilisers.iter().enumerate() {
        cell["assemblies"].as_array_mut().unwrap().push(json!({
            "id":id, "definitionId":definitions[index % definitions.len()]["id"],
            "offset":{"x":2.5 + index as f64,"y":3.25}, "rotation":relative_heading, "side":"back"
        }));
    }
    let source = create_with(
        &mut engine,
        &source,
        "commit",
        json!({"kind":"set-matrix","matrix":matrix,"definitions":definitions}),
    )
    .unwrap()["document"]
        .clone();
    for turn in [0.0, 27.0] {
        for mode in [
            "literal-mirror",
            "duplicate-position",
            "literal-mirror-with-tweak",
        ] {
            let (mut engine, source) = open(&source);
            let map = json!({"at":{"x":310.0,"y":42.0},"rotation":turn});
            let mut op = operation(
                "pair-stabilisers",
                "left",
                new_board("right", "right"),
                map.clone(),
            );
            op["companionMode"] = json!(mode);
            if mode == "literal-mirror-with-tweak" {
                op["companionTweak"] = json!({"x":1.25,"y":-2.0,"rotation":13.0});
            }
            let made = create_with(&mut engine, &source, "commit", op).unwrap()["document"].clone();
            let typed: ProjectDoc = serde_json::from_value(made.clone()).unwrap();
            for (id, _) in stabilisers {
                let source_id = format!("matrix/left-synthetic-keys/r0c0/{id}");
                let mirror_id = format!("matrix/right-synthetic-keys/r0c0/{id}");
                let reason = Some(
                    "Stabilisers keep their mirrored-X position and unchanged heading in every mode.",
                );
                assert_eq!(
                    boardstudio_core::companion_placement_disabled_reason(
                        &typed,
                        "pair-stabilisers",
                        &source_id,
                    ),
                    reason,
                    "Core projects stabiliser eligibility from the source id"
                );
                assert_eq!(
                    boardstudio_core::companion_placement_disabled_reason(
                        &typed,
                        "pair-stabilisers",
                        &mirror_id,
                    ),
                    reason,
                    "Core projects stabiliser eligibility from the mirror id"
                );
                let original = part(&source, &source_id);
                let mirrored = part(&made, &mirror_id);
                let label = format!("{mode} map {turn}: {mirror_id}");
                assert_close_pair(
                    (
                        mirrored["pose"]["at"]["x"].as_f64().unwrap(),
                        mirrored["pose"]["at"]["y"].as_f64().unwrap(),
                    ),
                    mapped_pose(&map, &original["pose"]).0,
                    &label,
                );
                assert_eq!(
                    mirrored["pose"]["rotation"], original["pose"]["rotation"],
                    "{label}"
                );
                for field in ["definitionId", "side", "generatorParameters"] {
                    assert_eq!(mirrored[field], original[field], "{label}: {field}");
                }
            }
            let (_, reopened) = open(&made);
            assert_eq!(reopened["matrices"], made["matrices"]);
            assert_eq!(reopened["parts"], made["parts"]);
        }
    }
}
