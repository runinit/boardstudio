use super::*;

#[test]
fn modern_outline_source_ids_match_legacy_source_geometry() {
    let legacy = source_board_with("");
    let outline_id = "11111111-1111-4111-8111-111111111111";
    let modern = legacy.replace(
        &format!("(tstamp {outline_id})"),
        &format!("(uuid {outline_id})"),
    );
    assert_ne!(legacy, modern, "the control changes only identifier keys");
    let expected = read_source(&legacy);
    assert_eq!(expected["kind"], "read-board-source", "{expected}");
    let actual = read_source(&modern);
    assert_eq!(actual["kind"], "read-board-source", "{actual}");
    assert_eq!(actual["result"], expected["result"]);
    let primitives = actual["result"]["pcbSource"]["contours"][0]["primitives"]
        .as_array()
        .unwrap();
    assert_eq!(primitives.len(), 4);
    assert!(
        primitives
            .iter()
            .all(|primitive| { primitive["sourceId"] == "11111111-1111-4111-8111-111111111111" })
    );
}

#[test]
fn outline_source_identity_policy_keeps_nonempty_legacy_precedence() {
    for (identity, expected) in [
        (r#"(uuid "modern")"#, "modern"),
        (r#"(tstamp legacy) (uuid "modern")"#, "legacy"),
        (r#"(uuid "modern") (tstamp legacy)"#, "legacy"),
        (r#"(tstamp "") (uuid "modern")"#, "modern"),
        (r#"(uuid "") (tstamp legacy)"#, "legacy"),
    ] {
        let reply = read_source(&format!(
            r#"(kicad_pcb (general (thickness 1.6))
                    (gr_circle (center 0 0) (end 2 0) (layer "Edge.Cuts") {identity}))"#
        ));
        assert_eq!(reply["kind"], "read-board-source", "{identity}: {reply}");
        assert_eq!(
            reply["result"]["pcbSource"]["contours"][0]["primitives"][0]["sourceId"], expected,
            "{identity}"
        );
    }
}

#[test]
fn outline_source_rejects_missing_empty_and_duplicate_modern_identities() {
    for (identities, reason) in [
        (vec![""], "missing its stable"),
        (vec![r#"(uuid "")"#], "missing its stable"),
        (vec![r#"(tstamp "") (uuid " ")"#], "missing its stable"),
        (
            vec!["(uuid same)", "(uuid same)"],
            "duplicate Edge.Cuts source IDs",
        ),
        (
            vec!["(uuid same)", "(tstamp same)"],
            "duplicate Edge.Cuts source IDs",
        ),
    ] {
        let curves: String = identities
            .iter()
            .enumerate()
            .map(|(index, identity)| {
                let x = index * 10;
                format!(
                    r#"(gr_circle (center {x} 0) (end {} 0) (layer "Edge.Cuts") {identity})"#,
                    x + 2
                )
            })
            .collect();
        let reply = read_source(&format!("(kicad_pcb (general (thickness 1.6)) {curves})"));
        assert_eq!(reply["kind"], "error", "{identities:?}: {reply}");
        assert!(
            reply["error"]["message"].as_str().unwrap().contains(reason),
            "{reply}"
        );
    }
}

#[test]
fn direct_board_source_refuses_unsupported_via_depths() {
    for (kind, layers, reason) in [
        ("blind", "(layers \"F.Cu\" \"In1.Cu\")", "type 'blind'"),
        ("blind", "(layers \"In1.Cu\" \"In2.Cu\")", "type 'blind'"),
        ("micro", "(layers \"F.Cu\" \"In1.Cu\")", "type 'micro'"),
        ("micro", "(layers \"F.Cu\" \"B.Cu\")", "type 'micro'"),
        ("future", "(layers \"F.Cu\" \"B.Cu\")", "type 'future'"),
        ("", "(layers \"F.Cu\" \"In1.Cu\")", "layer span"),
        ("", "(layers \"In1.Cu\" \"In2.Cu\")", "layer span"),
        ("", "", "layer span"),
        ("", "(layers \"*.Cu\")", "layer span"),
        ("", "(layers \"F.Cu\" \"unknown\")", "layer span"),
        ("", "(layers \"F.Cu\" \"B.Cu\" \"In1.Cu\")", "layer span"),
        (
            "",
            "(layers \"F.Cu\" \"B.Cu\") (layers \"F.Cu\" \"In1.Cu\")",
            "layer span",
        ),
    ] {
        let source = source_board_with(&format!(
            "(via {kind} (at 10 10) (size 0.8) (drill 0.4) {layers} (uuid via-depth))"
        ));
        let reply = read_placed_source(&source, (100, 200), 90, true);
        assert_eq!(
            reply["kind"], "error",
            "{kind} must refuse physical drilling: {reply}"
        );
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("via-depth") && message.contains(reason),
            "{message}"
        );
    }
}

#[test]
fn through_vias_keep_uuid_centres_and_diameters_with_slots_and_source_placement() {
    for (flag, layers, identity) in [
        ("", r#""F.Cu" "B.Cu""#, "tstamp"),
        ("locked", r#""B.Cu" "F.Cu""#, "uuid"),
        ("(locked yes)", r#""F.Cu" "B.Cu""#, "uuid"),
    ] {
        let source = source_board_with(&format!(
            "(via {flag} (at 8 6) (size 0.8) (drill 0.4) (layers {layers}) ({identity} through-via))"
        ));
        let reply = read_placed_source(&source, (100, 200), 90, true);
        assert_eq!(reply["kind"], "read-board-source", "{reply}");
        assert_eq!(
            reply["result"]["roundDrills"],
            json!([{
                "id":"through-via", "at":{"x":106.,"y":192.}, "kind":"hole", "holeDiameter":0.4
            }])
        );
        assert_eq!(
            reply["result"]["pcbSource"]["slots"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let imported = import(source, "fixture:through-via");
        assert_eq!(
            imported["result"]["board"]["roundDrills"][0]["id"],
            "through-via"
        );
        assert!(
            !imported["result"]["gates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gate| gate["code"] == "board-source-profile")
        );
    }
}

#[test]
fn direct_board_source_preserves_a_circular_outline_hole_without_refitting() {
    let result = read_source(
        r#"(kicad_pcb (general (thickness 1.6))
                (gr_circle (center 10 20) (end 11 20) (layer "Edge.Cuts") (tstamp inner))
                (gr_circle (center 10 20) (end 15 20) (layer "Edge.Cuts") (tstamp outer)))"#,
    );
    assert_eq!(result["kind"], "read-board-source", "{result}");
    let board = &result["result"];
    let contours = board["pcbSource"]["contours"].as_array().unwrap();
    assert_eq!(contours.len(), 2);
    for (id, radius, owner) in [
        ("outline/inner", 1., json!("outline/outer")),
        ("outline/outer", 5., Value::Null),
    ] {
        let contour = contours.iter().find(|item| item["id"] == id).unwrap();
        assert_eq!(contour["outerId"], owner);
        let primitive = &contour["primitives"][0];
        assert_eq!(primitive["geometry"]["kind"], "circle");
        assert_eq!(
            primitive["curveSupport"]["center"],
            json!({"x":10.,"y":-20.})
        );
        assert_eq!(primitive["curveSupport"]["radiusMm"], radius);
        assert_eq!(
            primitive["curveSupport"]["sweepRadians"],
            std::f64::consts::TAU
        );
    }
    assert_eq!(board["contours"].as_array().unwrap().len(), 2);
    assert_eq!(
        board["contours"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["hole"] == true)
            .count(),
        1
    );
}

#[test]
fn direct_board_source_names_intersecting_touching_and_ambiguous_outline_sources() {
    let outer = r#"(gr_circle (center 0 0) (end 5 0) (layer "Edge.Cuts") (tstamp outer))"#;
    for (outline, expected_ids) in [
        (format!(r#"{outer} (gr_circle (center 4 0) (end 5 0) (layer "Edge.Cuts") (tstamp tangent))"#), vec!["outer", "tangent"]),
        (format!(r#"{outer} (gr_circle (center 5 0) (end 7 0) (layer "Edge.Cuts") (tstamp crossing))"#), vec!["outer", "crossing"]),
        (format!(r#"{outer} (gr_circle (center 0 0) (end -5 0) (layer "Edge.Cuts") (tstamp coincident))"#), vec!["outer", "coincident"]),
        (r#"(gr_poly (pts (xy 0 0) (xy 10 0) (xy 5 5) (xy 10 10) (xy 0 10) (xy 5 5)) (layer "Edge.Cuts") (tstamp self-touch))"#.into(), vec!["self-touch"]),
    ] {
        let result = read_source(&format!("(kicad_pcb (general (thickness 1.6)) {outline})"));
        assert_eq!(result["kind"], "error", "ambiguous geometry must refuse: {result}");
        let message = result["error"]["message"].as_str().unwrap();
        for id in expected_ids {
            assert!(message.contains(id), "missing source {id}: {message}");
        }
    }
}

#[test]
fn direct_board_source_preserves_nested_islands_and_nearest_hole_owners() {
    let loops = [
        r#"(gr_circle (center 10 20) (end 11 20) (layer "Edge.Cuts") (tstamp island-hole))"#,
        r#"(gr_circle (center 10 20) (end 20 20) (layer "Edge.Cuts") (tstamp outer))"#,
        r#"(gr_circle (center 10 20) (end 12 20) (layer "Edge.Cuts") (tstamp island))"#,
        r#"(gr_circle (center 10 20) (end 15 20) (layer "Edge.Cuts") (tstamp moat))"#,
    ];
    for (index, flipped) in [false, true].into_iter().enumerate() {
        let mut ordered = loops;
        if flipped {
            ordered.reverse();
        }
        let result = read_placed_source(
            &format!(
                "(kicad_pcb (general (thickness 1.6)) {})",
                ordered.join("\n")
            ),
            (0, 0),
            0,
            flipped,
        );
        assert_eq!(result["kind"], "read-board-source", "{result}");
        let board = &result["result"];
        let contours = board["pcbSource"]["contours"].as_array().unwrap();
        assert_eq!(contours.len(), 4);
        for (id, radius, owner) in [
            ("outer", 10., Value::Null),
            ("moat", 5., json!("outline/outer")),
            ("island", 2., Value::Null),
            ("island-hole", 1., json!("outline/island")),
        ] {
            let contour = contours
                .iter()
                .find(|c| c["id"] == format!("outline/{id}"))
                .unwrap();
            assert_eq!(contour["outerId"], owner);
            let support = &contour["primitives"][0]["curveSupport"];
            assert_eq!(support["radiusMm"], radius);
            assert_eq!(
                support["center"],
                json!({"x":if flipped {-10.} else {10.},"y":-20.})
            );
        }
        let body = json!({"id":"nested-islands","name":"Nested islands","boardId":"board",
            "kind":"plate","thickness":1.6,"clearance":0.,"z":0.,"pcb_source":board["pcbSource"]});
        let reply: Value = serde_json::from_str(
            &CoreEngine::new().request(
                &json!({
                    "id":"prepare-nested","kind":"prepare-case","ir":{"revision":3,"bodies":[{
                        "revision":3,"body":body,"contours":board["contours"]
                    }]}
                })
                .to_string(),
            ),
        )
        .unwrap();
        assert_eq!(reply["kind"], "case-prepared", "{reply}");
        let prepared = &reply["ir"]["bodies"][0];
        assert_eq!(prepared["body"], body);
        let regions = prepared["regions"].as_array().unwrap();
        assert_eq!(regions.len(), 2);
        assert!(
            regions
                .iter()
                .all(|region| region["holes"].as_array().unwrap().len() == 1)
        );
        if let Some(directory) = std::env::var_os("SOURCE_PCB_LOOP_EVIDENCE_OUTPUT") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(
                directory.join(format!("nested-{index}.json")),
                serde_json::to_vec_pretty(prepared).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn direct_board_source_classifies_a_hole_inside_a_major_arc_without_changing_the_arc() {
    let result = read_source(
        r#"(kicad_pcb (general (thickness 1.6))
                (gr_arc (start 55 50) (mid 45 50) (end 50 55) (layer "Edge.Cuts") (tstamp major))
                (gr_line (start 50 55) (end 55 50) (layer "Edge.Cuts") (tstamp chord))
                (gr_circle (center 50 48) (end 50.5 48) (layer "Edge.Cuts") (tstamp hole)))"#,
    );
    assert_eq!(result["kind"], "read-board-source", "{result}");
    let contours = result["result"]["pcbSource"]["contours"]
        .as_array()
        .unwrap();
    let outer = contours
        .iter()
        .find(|c| c["id"] == "outline/major")
        .unwrap();
    let hole = contours.iter().find(|c| c["id"] == "outline/hole").unwrap();
    assert_eq!(hole["outerId"], "outline/major");
    let arc = &outer["primitives"][0];
    assert_eq!(arc["curveSupport"]["radiusMm"], 5.);
    assert_eq!(
        arc["curveSupport"]["sweepRadians"],
        1.5 * std::f64::consts::PI
    );
    assert_eq!(arc["geometry"]["mid"], json!({"x":45.,"y":-50.}));
    assert_eq!(outer["primitives"].as_array().unwrap().len(), 2);
}

#[test]
fn direct_board_source_enforces_the_existing_module_contour_budget() {
    let loops = (0..1001)
        .map(|i| {
            format!(
                "(gr_circle (center {} 0) (end {} 0) (layer \"Edge.Cuts\") (tstamp loop-{i}))",
                i * 3,
                i * 3 + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let result = read_source(&format!("(kicad_pcb (general (thickness 1.6)) {loops})"));
    assert_eq!(result["kind"], "error", "{result}");
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("1000-contour")
    );
}

#[test]
fn direct_board_source_keeps_unknown_thickness_and_refuses_incomplete_outlines() {
    let source = source_board_with("");
    let missing_thickness = source.replace("(general (thickness 1.6))", "");
    assert_ne!(
        source, missing_thickness,
        "fixture contains authored thickness"
    );
    let result = read_source(&missing_thickness);
    assert_eq!(result["kind"], "read-board-source", "{result}");
    assert!(
        result["result"].get("thickness").is_none(),
        "preview defaults must not become source dimensions"
    );
    for extra in [
        r#"(gr_curve (pts (xy 0 0) (xy 1 1) (xy 2 2) (xy 3 3)) (layer "Edge.Cuts") (tstamp unsupported))"#,
        r#"(gr_line (start 30 30) (end 40 40) (layer "Edge.Cuts") (tstamp unmatched-edge))"#,
    ] {
        let result = read_source(&source_board_with(extra));
        assert_eq!(
            result["kind"], "error",
            "incomplete source must refuse as a whole: {result}"
        );
    }
}

#[test]
fn direct_board_source_retains_raw_origin_curves_and_drills_through_placement() {
    let source = r#"(kicad_pcb (general (thickness 1.6))
      (gr_arc (start 10 20) (end 15 20) (angle 180)
        (layer "Edge.Cuts") (width 0.05) (uuid arc))
      (gr_line (start 5 20) (end 15 20)
        (layer "Edge.Cuts") (width 0.05) (uuid line))
      (via (at 10 22) (size 2) (drill 1) (layers "F.Cu" "B.Cu") (tstamp drill)))"#;
    for (index, (flipped, rotation, tx, expected_x, expected_y, drill_x, expected_sweep)) in [
        (false, 90, 100, 120., 210., 122., -std::f64::consts::PI),
        (true, 90, 100, 120., 190., 122., std::f64::consts::PI),
        // Reflect the already placed source in world X, as the physical-half capture does.
        (true, -90, -100, -120., 210., -122., std::f64::consts::PI),
    ]
    .into_iter()
    .enumerate()
    {
        let result = read_placed_source(source, (tx, 200), rotation, flipped);
        assert_eq!(result["kind"], "read-board-source", "{result}");
        let board = &result["result"];
        assert_eq!(board["thickness"], 1.6);
        let primitives = board["pcbSource"]["contours"][0]["primitives"]
            .as_array()
            .unwrap();
        assert_eq!(primitives.len(), 2);
        let arc = primitives
            .iter()
            .find(|edge| edge["sourceId"] == "arc")
            .unwrap();
        let support = &arc["curveSupport"];
        assert!((support["center"]["x"].as_f64().unwrap() - expected_x).abs() < 1e-12);
        assert!((support["center"]["y"].as_f64().unwrap() - expected_y).abs() < 1e-12);
        assert_eq!(support["radiusMm"], 5.0);
        assert_eq!(support["sweepRadians"].as_f64(), Some(expected_sweep));
        let geometry = &arc["geometry"];
        assert!(geometry["angleDegrees"].is_null());
        let midpoint_x = if index == 2 { -125.0 } else { 125.0 };
        assert!((geometry["mid"]["x"].as_f64().unwrap() - midpoint_x).abs() < 1e-12);
        assert!((geometry["mid"]["y"].as_f64().unwrap() - expected_y).abs() < 1e-12);
        let drills = board["roundDrills"].as_array().unwrap();
        assert_eq!(drills.len(), 1);
        assert_eq!(drills[0]["id"], "drill");
        assert!((drills[0]["at"]["x"].as_f64().unwrap() - drill_x).abs() < 1e-12);
        assert!((drills[0]["at"]["y"].as_f64().unwrap() - expected_y).abs() < 1e-12);
        assert_eq!(drills[0]["holeDiameter"], 1.0);
        let hole = board["holes"][0].as_array().unwrap();
        let average = |axis: &str| {
            hole.iter()
                .map(|point| point[axis].as_f64().unwrap())
                .sum::<f64>()
                / hole.len() as f64
        };
        assert!((average("x") - drill_x).abs() < 1e-12);
        assert!((average("y") - expected_y).abs() < 1e-12);
        let mut contours = board["contours"].as_array().unwrap().clone();
        contours.extend(
            board["holes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|hole| json!({"points":hole,"hole":true})),
        );
        let reply: Value = serde_json::from_str(&CoreEngine::new().request(&json!({
            "id":"prepare-source", "kind":"prepare-case", "ir":{"revision":7,"bodies":[{
                "revision":7,"contours":contours,"body":{
                    "id":"pcb-reference","name":"Routed PCB","boardId":"board","kind":"plate",
                    "thickness":1.6,"clearance":0.,"z":1.4,
                    "pcb_source":board["pcbSource"],"mounts":board["roundDrills"]
                }
            }]}
        }).to_string())).unwrap();
        assert_eq!(reply["kind"], "case-prepared", "{reply}");
        let prepared = &reply["ir"]["bodies"][0];
        assert_eq!(prepared["body"]["pcb_source"], board["pcbSource"]);
        assert_eq!(prepared["body"]["mounts"], board["roundDrills"]);
        if let Some(directory) = std::env::var_os("SOURCE_PCB_EVIDENCE_OUTPUT") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(
                directory.join(format!("source-case-{index}.json")),
                serde_json::to_vec_pretty(prepared).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn legacy_board_arcs_preserve_compatibility_holes_and_exact_support() {
    for (arc_form, hole_y, center_y, sweep) in [
        ("angle 180", 22, 2.5, -std::f64::consts::PI),
        ("angle -180", 18, -2.5, std::f64::consts::PI),
    ] {
        let source = format!(
            r#"(kicad_pcb (general (thickness 1.6))
          (gr_arc (start 10 20) (end 15 20) ({arc_form})
            (layer "Edge.Cuts") (width 0.05) (tstamp outline-arc))
          (gr_line (start 5 20) (end 15 20)
            (layer "Edge.Cuts") (width 0.05) (tstamp outline-line))
          (via (at 10 {hole_y}) (size 2) (drill 1) (layers "F.Cu" "B.Cu") (tstamp drill)))"#
        );
        let result = import(source, "fixture:legacy-arc");
        assert_eq!(result["kind"], "import-module-board", "{result}");
        let board = &result["result"]["board"];
        assert_eq!(
            board["holes"].as_array().map(Vec::len),
            Some(1),
            "a legacy outline must not drop the current provider's drilled-hole view: {result}"
        );
        assert_eq!(board["roundDrills"].as_array().unwrap().len(), 1);
        assert_eq!(board["roundDrills"][0]["holeDiameter"], 1.0);
        let primitives = board["pcbSource"]["contours"][0]["primitives"]
            .as_array()
            .unwrap();
        let arc = primitives
            .iter()
            .find(|primitive| primitive["sourceId"] == "outline-arc")
            .unwrap();
        let support = &arc["curveSupport"];
        // Authored half discs span x=5..15, KiCad y=20..25 or 15..20; import centers them.
        assert!(support["center"]["x"].as_f64().unwrap().abs() < 1e-12);
        assert!((support["center"]["y"].as_f64().unwrap() - center_y).abs() < 1e-12);
        assert_eq!(support["radiusMm"], 5.0);
        assert_eq!(support["sweepRadians"].as_f64(), Some(sweep));
        assert!(
            result["result"]["gates"]
                .as_array()
                .unwrap()
                .iter()
                .all(|gate| {
                    !matches!(
                        gate["code"].as_str(),
                        Some("board-hole-geometry" | "board-source-profile")
                    )
                })
        );
    }
}

#[test]
fn board_preview_preserves_midpoint_arcs_and_refuses_invalid_arc_definitions() {
    for (definition, valid) in [
        ("(start 15 20) (mid 10 25) (end 5 20)", true),
        ("(start 10 20) (end 15 20) (angle 0)", false),
        ("(start 10 20) (end 15 20) (angle 361)", false),
        ("(start 10 20) (mid 10 25) (end 15 20) (angle 180)", false),
        ("(start 10 20) (end 15 20)", false),
        ("(start 10 20) (end 10 20) (angle 90)", false),
    ] {
        // A separate closed outline isolates arc validation from edge joining.
        let source = format!(
            r#"(kicad_pcb
              (gr_rect (start 0 0) (end 30 30) (layer "Edge.Cuts"))
              (gr_arc {definition} (layer "F.SilkS") (width 0.15)))"#
        );
        let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"preview","kind":"preview-board","source":source,"revision":17})
                .to_string(),
        ))
        .unwrap();
        if !valid {
            assert_eq!(result["kind"], "error", "{definition}: {result}");
            continue;
        }
        assert_eq!(result["kind"], "preview-board", "{result}");
        assert_eq!(result["result"]["revision"], 17);
        let points = result["result"]["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|surface| surface["layer"] == "F.SilkS")
            .unwrap()["points"]
            .as_array()
            .unwrap();
        let mut bounds = [
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for point in points {
            let x = point["x"].as_f64().unwrap();
            let y = point["y"].as_f64().unwrap();
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].max(x);
            bounds[2] = bounds[2].min(y);
            bounds[3] = bounds[3].max(y);
        }
        for (actual, expected) in bounds.into_iter().zip([5., 15., -25., -20.]) {
            assert!(
                (actual - expected).abs() < 1e-12,
                "midpoint arc bounds: {result}"
            );
        }
    }
}

#[test]
fn board_source_operations_keep_their_own_error_envelopes_and_validation_order() {
    for (source, message) in [
        ("(kicad_pcb", None),
        ("", Some("expected single root, got 0")),
        ("(footprint \"x\")", Some("Expected a .kicad_pcb board")),
    ] {
        let read = error_of(&read_source(source));
        assert_eq!(
            read,
            error_of(&preview(source, 3)),
            "read reports the preview parse"
        );
        assert_eq!(read.0, "parse-error");
        if let Some(message) = message {
            assert_eq!(read.1, message);
        }
        let imported = error_of(&import(source.to_owned(), "fixture:bad"));
        assert_eq!(imported.0, "validation");
        match message {
            Some("Expected a .kicad_pcb board") => {
                assert_eq!(imported.1, "Expected a KiCad PCB source")
            }
            Some(message) => assert_eq!(imported.1, message),
            None => {}
        }
    }
    // The preview size limit applies before the raw source is parsed.
    let oversized = format!("(kicad_pcb{} ", " ".repeat(32 * 1024 * 1024));
    let limit = "Board exceeds the 32 MiB preview limit";
    assert_eq!(
        error_of(&read_source(&oversized)),
        ("parse-error".into(), limit.into())
    );
    assert_eq!(error_of(&preview(&oversized, 3)).1, limit);
    // Import has no whole-source cap: only its hole geometry is gated.
    let padded = format!("{}{}", source_board_with(""), " ".repeat(32 * 1024 * 1024));
    let imported = import(padded, "fixture:padded");
    assert_eq!(imported["kind"], "import-module-board", "{imported}");
    assert!(
        imported["result"]["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gate| {
                gate["code"] == "board-hole-geometry"
                    && gate["message"].as_str().unwrap().contains(limit)
            }),
        "{imported}"
    );
    assert!(!imported["result"]["board"]["pcbSource"].is_null());
}
