use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

fn import(source: String, id: &str) -> Value {
    serde_json::from_str(&boardstudio_core::artifact::request(&json!({
        "id":"import","kind":"import-module-board","definitionId":id,
        "name":"Fixture module","source":source,"family":"fixture","variant":"source",
        "provenance":{"repository":"https://example.invalid/repo","revision":"abc","path":"board.kicad_pcb","license":"MIT"}
    }).to_string())).unwrap()
}

#[test]
fn modern_outline_source_ids_match_legacy_source_geometry() {
    let legacy = source_board_with("");
    let outline_id = "11111111-1111-4111-8111-111111111111";
    let modern = legacy.replace(
        &format!("(tstamp {outline_id})"),
        &format!("(uuid {outline_id})"),
    );
    assert_ne!(legacy, modern, "the control changes only identifier keys");
    let read = |source: &str| -> Value {
        serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"identity", "kind":"read-board-source", "source":source,
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false})
            .to_string(),
        ))
        .unwrap()
    };
    let expected = read(&legacy);
    assert_eq!(expected["kind"], "read-board-source", "{expected}");
    let actual = read(&modern);
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
        let reply: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"identity-policy", "kind":"read-board-source",
                "source":format!(r#"(kicad_pcb (general (thickness 1.6))
                    (gr_circle (center 0 0) (end 2 0) (layer "Edge.Cuts") {identity}))"#),
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false})
            .to_string(),
        ))
        .unwrap();
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
        let reply: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"invalid-identity", "kind":"read-board-source",
                "source":format!("(kicad_pcb (general (thickness 1.6)) {curves})"),
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false})
            .to_string(),
        ))
        .unwrap();
        assert_eq!(reply["kind"], "error", "{identities:?}: {reply}");
        assert!(
            reply["error"]["message"].as_str().unwrap().contains(reason),
            "{reply}"
        );
    }
}

#[test]
fn modern_outline_primitives_retain_exact_legacy_geometry_through_module_import() {
    for (name, graphics) in [
        (
            "lines",
            r#"(gr_line (start 0 0) (end 10 0) (layer "Edge.Cuts") (tstamp a))
            (gr_line (start 10 0) (end 0 10) (layer "Edge.Cuts") (tstamp b))
            (gr_line (start 0 10) (end 0 0) (layer "Edge.Cuts") (tstamp c))"#,
        ),
        (
            "arc",
            r#"(gr_arc (start 0 0) (mid 5 5) (end 10 0) (layer "Edge.Cuts") (tstamp a))
            (gr_line (start 10 0) (end 0 0) (layer "Edge.Cuts") (tstamp b))"#,
        ),
        (
            "circle",
            r#"(gr_circle (center 0 0) (end 5 0) (layer "Edge.Cuts") (tstamp a))"#,
        ),
        (
            "rectangle",
            r#"(gr_rect (start 0 0) (end 10 10) (layer "Edge.Cuts") (tstamp a))"#,
        ),
        (
            "polygon",
            r#"(gr_poly (pts (xy 0 0) (xy 10 0) (xy 0 10)) (layer "Edge.Cuts") (tstamp a))"#,
        ),
    ] {
        let legacy = format!("(kicad_pcb (general (thickness 1.6)) {graphics})");
        let modern = legacy.replace("(tstamp ", "(uuid ");
        let expected = import(legacy, name);
        let actual = import(modern, name);
        assert_eq!(
            expected["kind"], "import-module-board",
            "{name}: {expected}"
        );
        assert_eq!(actual["kind"], "import-module-board", "{name}: {actual}");
        assert!(
            !actual["result"]["board"]["pcbSource"].is_null(),
            "{name}: {actual}"
        );
        assert_eq!(
            actual["result"]["board"], expected["result"]["board"],
            "{name}"
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
        let reply: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"via-depth", "kind":"read-board-source", "source":source,
                "placement":{"at":{"x":100,"y":200},"rotation":90},"flipped":true})
            .to_string(),
        ))
        .unwrap();
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
        let reply: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"through", "kind":"read-board-source", "source":source,
                "placement":{"at":{"x":100,"y":200},"rotation":90},"flipped":true})
            .to_string(),
        ))
        .unwrap();
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
fn unsupported_via_depth_gates_module_exact_geometry_and_retains_source_provenance() {
    for (kind, layers) in [
        ("blind", r#""F.Cu" "In1.Cu""#),
        ("blind", r#""In1.Cu" "In2.Cu""#),
        ("micro", r#""F.Cu" "In1.Cu""#),
        ("", r#""F.Cu" "unknown""#),
    ] {
        let source = source_board_with(&format!(
            "(via {kind} (at 10 10) (size 0.8) (drill 0.4) (layers {layers}) (uuid refused-via))"
        ));
        let imported = import(source, "fixture:refused-via");
        assert_source_gate(&imported, "refused-via");
        let definition = &imported["result"];
        assert_eq!(
            definition["source"]["repository"],
            "https://example.invalid/repo"
        );
        assert_eq!(definition["source"]["revision"], "abc");
        assert_eq!(definition["source"]["path"], "board.kicad_pcb");
        assert_eq!(definition["source"]["sha256"].as_str().unwrap().len(), 64);
        assert!(
            !definition["board"]["contours"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let typed: boardstudio_core::model::ModuleDefinition =
            serde_json::from_value(definition.clone()).unwrap();
        assert!(typed.board.round_drills.is_empty());
        assert!(
            !definition["circuit"]["parts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let resolved = resolve_source_modules(vec![definition.clone()]);
        assert_eq!(resolved["kind"], "modules-resolved", "{resolved}");
        assert!(resolved["result"]["preview"].is_null(), "{resolved}");
        assert_eq!(resolved["result"]["modules"].as_array().unwrap().len(), 1);
        assert!(
            resolved["result"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["id"]
                    .as_str()
                    .unwrap()
                    .contains("board-source-profile"))
        );
    }
}

#[test]
fn direct_board_source_preserves_a_circular_outline_hole_without_refitting() {
    let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
        &json!({
            "id":"annulus", "kind":"read-board-source",
            "source":r#"(kicad_pcb (general (thickness 1.6))
                (gr_circle (center 10 20) (end 11 20) (layer "Edge.Cuts") (tstamp inner))
                (gr_circle (center 10 20) (end 15 20) (layer "Edge.Cuts") (tstamp outer)))"#,
            "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false
        })
        .to_string(),
    ))
    .unwrap();
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
        let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"invalid-loops","kind":"read-board-source",
                "source":format!("(kicad_pcb (general (thickness 1.6)) {outline})"),
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false
            }).to_string(),
        )).unwrap();
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
        let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"nested","kind":"read-board-source",
                "source":format!("(kicad_pcb (general (thickness 1.6)) {})", ordered.join("\n")),
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":flipped
            })
            .to_string(),
        ))
        .unwrap();
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
    let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
        &json!({"id":"major-arc","kind":"read-board-source",
            "source":r#"(kicad_pcb (general (thickness 1.6))
                (gr_arc (start 55 50) (mid 45 50) (end 50 55) (layer "Edge.Cuts") (tstamp major))
                (gr_line (start 50 55) (end 55 50) (layer "Edge.Cuts") (tstamp chord))
                (gr_circle (center 50 48) (end 50.5 48) (layer "Edge.Cuts") (tstamp hole)))"#,
            "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false
        })
        .to_string(),
    ))
    .unwrap();
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
    let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
        &json!({
            "id":"too-many-loops","kind":"read-board-source",
            "source":format!("(kicad_pcb (general (thickness 1.6)) {loops})"),
            "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false
        })
        .to_string(),
    ))
    .unwrap();
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
    let read = |source: String| -> Value {
        serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({
                "id":"read","kind":"read-board-source","source":source,
                "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":false
            })
            .to_string(),
        ))
        .unwrap()
    };
    let source = source_board_with("");
    let missing_thickness = source.replace("(general (thickness 1.6))", "");
    assert_ne!(
        source, missing_thickness,
        "fixture contains authored thickness"
    );
    let result = read(missing_thickness);
    assert_eq!(result["kind"], "read-board-source", "{result}");
    assert!(
        result["result"].get("thickness").is_none(),
        "preview defaults must not become source dimensions"
    );
    for extra in [
        r#"(gr_curve (pts (xy 0 0) (xy 1 1) (xy 2 2) (xy 3 3)) (layer "Edge.Cuts") (tstamp unsupported))"#,
        r#"(gr_line (start 30 30) (end 40 40) (layer "Edge.Cuts") (tstamp unmatched-edge))"#,
    ] {
        let result = read(source_board_with(extra));
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
        let result: Value = serde_json::from_str(&boardstudio_core::artifact::request(
            &json!({"id":"read-board","kind":"read-board-source","source":source,
                "placement":{"at":{"x":tx,"y":200},"rotation":rotation},"flipped":flipped})
            .to_string(),
        ))
        .unwrap();
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
fn module_import_rejects_duplicate_source_references_before_namespacing_a_circuit() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    )
    .replace(
        "(fp_text reference \"J1002\"",
        "(fp_text reference \"J1001\"",
    );
    let result = import(source, "fixture:duplicate");
    assert_eq!(
        result["kind"], "error",
        "duplicate references must not collapse local part identities"
    );
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("duplicate footprint reference"),
        "{result}"
    );
}

#[test]
fn module_import_gives_unannotated_footprints_unique_circuit_identities() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    )
    .to_owned();
    let result = import(source, "fixture:splitter");
    assert_eq!(result["kind"], "import-module-board", "{result}");
    let parts = result["result"]["circuit"]["parts"].as_array().unwrap();
    let ids: std::collections::BTreeSet<_> = parts
        .iter()
        .map(|part| part["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids.len(),
        parts.len(),
        "every imported circuit part needs an independent identity"
    );
    assert!(
        ids.iter().any(|id| id.starts_with("PART")),
        "unannotated source references receive deterministic ids"
    );
}

#[test]
fn module_import_retains_source_outline_support_and_every_round_drill() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    );
    let result = import(source.to_owned(), "fixture:splitter");
    assert_eq!(result["kind"], "import-module-board", "{result}");

    let board = &result["result"]["board"];
    let source_profile = board.get("pcbSource").unwrap_or_else(|| {
        panic!("the source board outline must survive import as exact authored curves")
    });
    let contours = source_profile["contours"].as_array().unwrap();
    assert_eq!(
        contours.len(),
        1,
        "VIK splitter has one authored Edge.Cuts loop"
    );
    let primitives = contours[0]["primitives"].as_array().unwrap();
    assert_eq!(
        primitives.len(),
        8,
        "retain the four authored lines and four arcs"
    );

    let expected = serde_json::from_str::<Value>(include_str!(
        "fixtures/pcb-source/vik-splitter-expectations.json"
    ))
    .unwrap();
    let parse = |value: &Value| value.as_str().unwrap().parse::<f64>().unwrap();
    // These are the extrema of the authored straight Edge.Cuts spans. The
    // compatibility polygon centers from its projected bounds; the exact
    // arcs extend beyond this box slightly and must not move other geometry.
    let source_center = boardstudio_core::model::Vec2 {
        x: (69.084194 + 94.084194) / 2.0,
        y: -(98.212271 + 123.242271) / 2.0,
    };
    let compatibility_points = board["contours"][0]["points"].as_array().unwrap();
    let mut contour_min = [f64::INFINITY; 2];
    let mut contour_max = [f64::NEG_INFINITY; 2];
    for point in compatibility_points {
        let x = point["x"].as_f64().unwrap();
        let y = point["y"].as_f64().unwrap();
        contour_min[0] = contour_min[0].min(x);
        contour_min[1] = contour_min[1].min(y);
        contour_max[0] = contour_max[0].max(x);
        contour_max[1] = contour_max[1].max(y);
    }
    assert!(((contour_min[0] + contour_max[0]) / 2.0).abs() < 5e-12);
    assert!(((contour_min[1] + contour_max[1]) / 2.0).abs() < 5e-12);
    let mut actual_outline_ids = primitives
        .iter()
        .map(|primitive| primitive["sourceId"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    actual_outline_ids.sort();
    let mut expected_outline_ids = expected["arc_supports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|arc| arc["id"].as_str().unwrap().to_owned())
        .chain(
            [
                "27f679de-df87-49fa-a42e-d9d2cb3dd7ea",
                "2e356080-7f8f-4d51-81a1-6e35eecaff69",
                "3138b20f-7d9d-4ddb-a8d3-6d3944a0de81",
                "4f68febc-398c-4592-a1c4-57ab8bc51941",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .collect::<Vec<_>>();
    expected_outline_ids.sort();
    assert_eq!(actual_outline_ids, expected_outline_ids);

    for expected_arc in expected["arc_supports"].as_array().unwrap() {
        let source_id = expected_arc["id"].as_str().unwrap();
        let primitive = primitives
            .iter()
            .find(|primitive| primitive["sourceId"] == source_id)
            .unwrap();
        let support = &primitive["curveSupport"];
        let expected_center_x = parse(&expected_arc["center_kicad"][0]) - source_center.x;
        let expected_center_y = -parse(&expected_arc["center_kicad"][1]) - source_center.y;
        assert!((support["center"]["x"].as_f64().unwrap() - expected_center_x).abs() < 5e-12);
        assert!((support["center"]["y"].as_f64().unwrap() - expected_center_y).abs() < 5e-12);
        assert!(
            (support["radiusMm"].as_f64().unwrap() - parse(&expected_arc["radius_mm"])).abs()
                < 5e-12,
            "authored support radius changed for {source_id}"
        );
    }

    let actual_drills = board["roundDrills"].as_array().unwrap();
    assert_eq!(
        actual_drills.len(),
        38,
        "retain every authored pad and via drill"
    );
    let mut actual_drill_ids = actual_drills
        .iter()
        .map(|drill| drill["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    actual_drill_ids.sort();
    let mut expected_drill_ids = expected["round_drills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|drill| drill["source_id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    expected_drill_ids.sort();
    assert_eq!(actual_drill_ids, expected_drill_ids);
    let mut sum_squared_diameters = 0.0;
    for expected_drill in expected["round_drills"].as_array().unwrap() {
        let source_id = expected_drill["source_id"].as_str().unwrap();
        let actual = actual_drills
            .iter()
            .find(|drill| drill["id"] == source_id)
            .unwrap();
        let source_at = &expected_drill["center_kicad_mm"];
        let expected_x = parse(&source_at[0]) - source_center.x;
        let expected_y = -parse(&source_at[1]) - source_center.y;
        let expected_diameter = 2.0 * parse(&expected_drill["radius_mm"]);
        assert!((actual["at"]["x"].as_f64().unwrap() - expected_x).abs() < 5e-12);
        assert!((actual["at"]["y"].as_f64().unwrap() - expected_y).abs() < 5e-12);
        assert!((actual["holeDiameter"].as_f64().unwrap() - expected_diameter).abs() < 5e-12);
        sum_squared_diameters += expected_diameter * expected_diameter;
    }
    assert!((sum_squared_diameters - 26.5).abs() < 1e-10);

    let compatibility_holes = board["holes"].as_array().unwrap();
    assert_eq!(compatibility_holes.len(), 38);
    let compatibility_hole_centers = compatibility_holes
        .iter()
        .map(|hole| {
            let points = hole.as_array().unwrap();
            let (x, y) = points.iter().fold((0.0, 0.0), |(x, y), point| {
                (
                    x + point["x"].as_f64().unwrap(),
                    y + point["y"].as_f64().unwrap(),
                )
            });
            (x / points.len() as f64, y / points.len() as f64)
        })
        .collect::<Vec<_>>();
    for expected_drill in expected["round_drills"].as_array().unwrap() {
        let source_at = &expected_drill["center_kicad_mm"];
        let expected_x = parse(&source_at[0]) - source_center.x;
        let expected_y = -parse(&source_at[1]) - source_center.y;
        assert!(
            compatibility_hole_centers
                .iter()
                .any(|(x, y)| { (x - expected_x).abs() < 5e-12 && (y - expected_y).abs() < 5e-12 }),
            "compatibility hole projection must use the shared centered frame"
        );
    }

    let selected_mounts = result["result"]["mounts"].as_array().unwrap();
    let selected_mounts = selected_mounts
        .iter()
        .filter(|mount| (mount["diameter"].as_f64().unwrap() - 2.2).abs() < 1e-12)
        .collect::<Vec<_>>();
    assert_eq!(selected_mounts.len(), 2, "retain both selected M2.2 mounts");
    for selected in selected_mounts {
        assert!(
            actual_drills.iter().any(|drill| {
                (drill["holeDiameter"].as_f64().unwrap() - 2.2).abs() < 1e-12
                    && (drill["at"]["x"].as_f64().unwrap() - selected["at"]["x"].as_f64().unwrap())
                        .abs()
                        < 5e-12
                    && (drill["at"]["y"].as_f64().unwrap() - selected["at"]["y"].as_f64().unwrap())
                        .abs()
                        < 5e-12
            }),
            "selected mechanical mount metadata must share the exact drill frame"
        );
    }

    let profile =
        serde_json::from_value::<boardstudio_core::model::CasePcbSource>(source_profile.clone())
            .unwrap();
    let mounts =
        serde_json::from_value::<Vec<boardstudio_core::model::Mount>>(board["roundDrills"].clone())
            .unwrap();
    let contours =
        serde_json::from_value::<Vec<boardstudio_core::model::Contour>>(board["contours"].clone())
            .unwrap();
    let prepared =
        match CoreEngine::new().handle(boardstudio_core::model::CoreRequest::PrepareCase {
            id: "prepare-vik-splitter".into(),
            ir: boardstudio_core::model::CaseAssemblyIR {
                revision: 1,
                bodies: vec![boardstudio_core::model::CaseIR {
                    revision: 1,
                    body: boardstudio_core::model::CaseBody {
                        features: None,
                        openings: None,
                        id: "vik-splitter".into(),
                        name: "VIK splitter PCB".into(),
                        board_id: "fixture-board".into(),
                        kind: boardstudio_core::model::CaseKind::Plate,
                        thickness: board["thickness"].as_f64().unwrap(),
                        clearance: 0.0,
                        material_id: None,
                        z: Some(0.0),
                        wall_height: None,
                        wall_thickness: None,
                        mounts: Some(mounts.clone()),
                        pcb_source: Some(profile.clone()),
                        gasket: None,
                    },
                    contours,
                }],
            },
        }) {
            boardstudio_core::model::CoreReply::CasePrepared { ir, .. } => {
                ir.bodies.into_iter().next().unwrap()
            }
            other => panic!("expected prepared case, got {other:?}"),
        };
    assert_eq!(prepared.body.pcb_source, Some(profile));
    assert_eq!(prepared.body.mounts, Some(mounts));
    assert_eq!(prepared.regions.len(), 1);
    println!(
        "PREPARED_CASE_JSON={}",
        serde_json::to_string(&prepared).unwrap()
    );
}

#[test]
fn module_import_preserves_absolute_pad_angle_and_offset_on_both_board_sides() {
    let back = import(
        include_str!("fixtures/pcb_back_slot.kicad_pcb").to_owned(),
        "fixture:back-offset-slot",
    );
    let front = import(
        include_str!("fixtures/pcb_front_slot.kicad_pcb").to_owned(),
        "fixture:front-offset-slot",
    );
    assert_eq!(back["kind"], "import-module-board", "{back}");
    assert_eq!(front["kind"], "import-module-board", "{front}");
    let board = &back["result"]["board"];
    let front_board = &front["result"]["board"];
    assert!(
        board["roundDrills"]
            .as_array()
            .is_none_or(|drills| drills.is_empty())
    );
    let profile: boardstudio_core::model::CasePcbSource =
        serde_json::from_value(board["pcbSource"].clone()).unwrap();
    let front_profile: boardstudio_core::model::CasePcbSource =
        serde_json::from_value(front_board["pcbSource"].clone()).unwrap();
    assert_eq!(
        profile.slots, front_profile.slots,
        "raw board side does not mirror the pad"
    );
    assert_eq!(profile.slots.len(), 1);

    let slot = &profile.slots[0];
    assert_eq!(
        slot.id,
        "22222222-2222-4222-8222-222222222222/33333333-3333-4333-8333-333333333333"
    );
    let boardstudio_core::model::MechanicalShape::Drill {
        at,
        size,
        offset,
        rotation_degrees,
        shape,
        plated,
    } = &slot.geometry
    else {
        panic!("source slot must retain an oval drill primitive");
    };
    assert_eq!(*size, boardstudio_core::model::Vec2 { x: 1.2, y: 0.8 });
    assert_eq!(*offset, boardstudio_core::model::Vec2 { x: 0.3, y: 0.2 });
    assert_eq!(*shape, boardstudio_core::model::MechanicalDrillShape::Oval);
    assert_eq!(*plated, Some(false));

    // Independent raw-source oracle: the 20 mm rectangle centers at (10,-10).
    assert!((at.x - 4.476910593535113).abs() < 5e-12);
    assert!((at.y + 4.638913564179708).abs() < 5e-12);
    assert!((*rotation_degrees - 23.0).abs() < 1e-12);
    let radians = rotation_degrees.to_radians();
    let resolved_center = boardstudio_core::model::Vec2 {
        x: at.x + offset.x * radians.cos() - offset.y * radians.sin(),
        y: at.y + offset.x * radians.sin() + offset.y * radians.cos(),
    };
    assert!((resolved_center.x - 4.674915823872991).abs() < 5e-12);
    assert!((resolved_center.y + 4.337593254942438).abs() < 5e-12);
    assert!((radians.cos() - 0.9205048534524404).abs() < 5e-12);
    assert!((radians.sin() - 0.39073112848927377).abs() < 5e-12);

    let contours = serde_json::from_value(board["contours"].clone()).unwrap();
    let prepared =
        match CoreEngine::new().handle(boardstudio_core::model::CoreRequest::PrepareCase {
            id: "prepare-back-offset-slot".into(),
            ir: boardstudio_core::model::CaseAssemblyIR {
                revision: 1,
                bodies: vec![boardstudio_core::model::CaseIR {
                    revision: 1,
                    body: boardstudio_core::model::CaseBody {
                        features: None,
                        openings: None,
                        id: "back-offset-slot".into(),
                        name: "Back offset slot".into(),
                        board_id: "fixture-board".into(),
                        kind: boardstudio_core::model::CaseKind::Plate,
                        thickness: board["thickness"].as_f64().unwrap(),
                        clearance: 0.0,
                        material_id: None,
                        z: Some(0.0),
                        wall_height: None,
                        wall_thickness: None,
                        mounts: None,
                        pcb_source: Some(profile.clone()),
                        gasket: None,
                    },
                    contours,
                }],
            },
        }) {
            boardstudio_core::model::CoreReply::CasePrepared { ir, .. } => {
                ir.bodies.into_iter().next().unwrap()
            }
            other => panic!("expected prepared case, got {other:?}"),
        };
    assert_eq!(prepared.body.pcb_source, Some(profile));
    assert_eq!(
        serde_json::to_value(&prepared).unwrap(),
        serde_json::from_str::<Value>(include_str!("fixtures/prepared-case-absolute-slot.json"))
            .unwrap()
    );
}

fn source_board_with(extra: &str) -> String {
    let source = include_str!("fixtures/pcb_front_slot.kicad_pcb").trim_end();
    format!("{}\n{extra}\n)", source.strip_suffix(')').unwrap())
}

#[test]
fn module_import_retains_drilled_pads_in_legacy_module_containers() {
    let source =
        include_str!("fixtures/pcb_front_slot.kicad_pcb").replace("(footprint ", "(module ");
    let result = import(source, "fixture:legacy-slot");
    assert_eq!(result["kind"], "import-module-board", "{result}");
    let slots = result["result"]["board"]["pcbSource"]["slots"]
        .as_array()
        .unwrap();
    assert_eq!(slots.len(), 1, "legacy pad drill must not disappear");
    assert_eq!(slots[0]["geometry"]["rotationDegrees"], 23.0);
}

fn assert_source_gate(result: &Value, detail: &str) {
    assert_eq!(result["kind"], "import-module-board", "{result}");
    let definition = &result["result"];
    assert!(definition["board"]["pcbSource"].is_null(), "{result}");
    assert!(
        definition["gates"].as_array().unwrap().iter().any(|gate| {
            gate["output"] == "mechanical"
                && gate["code"] == "board-source-profile"
                && gate["message"].as_str().unwrap().contains(detail)
        }),
        "missing named source gate: {result}"
    );
}

#[test]
fn module_import_refuses_unsupported_outline_records_instead_of_partial_source() {
    let result = import(
        source_board_with(
            "(gr_curve (pts (xy 1 1) (xy 2 3) (xy 4 5) (xy 6 6)) (layer \"Edge.Cuts\") (tstamp unsupported-curve))",
        ),
        "fixture:unsupported-outline",
    );
    assert_source_gate(&result, "unsupported-curve");

    let source = include_str!("fixtures/pcb_front_slot.kicad_pcb").replace(
        "(property \"Reference\" \"J1\")",
        "(property \"Reference\" \"J1\") (fp_line (start 0 0) (end 1 0) (layer \"Edge.Cuts\") (tstamp nested-outline))",
    );
    assert_source_gate(&import(source, "fixture:nested-outline"), "nested-outline");
}

#[test]
fn module_import_keeps_nested_holes_and_disconnected_islands_in_all_views() {
    for (x, id) in [(10, "nested-loop"), (30, "disconnected-loop")] {
        let extra = format!(
            "(gr_circle (center {x} 10) (end {} 10) (layer \"Edge.Cuts\") (tstamp {id}))",
            x + 2
        );
        let result = import(source_board_with(&extra), id);
        assert_eq!(result["kind"], "import-module-board", "{result}");
        let board = &result["result"]["board"];
        let source = board["pcbSource"]["contours"].as_array().unwrap();
        assert_eq!(source.len(), 2);
        let circle = source
            .iter()
            .find(|c| c["id"] == format!("outline/{id}"))
            .unwrap();
        let expected_owner = if x == 10 {
            json!("outline/11111111-1111-4111-8111-111111111111")
        } else {
            Value::Null
        };
        assert_eq!(circle["outerId"], expected_owner);
        let compatibility = board["contours"].as_array().unwrap();
        assert_eq!(compatibility.len(), 2);
        assert_eq!(
            compatibility.iter().filter(|c| c["hole"] == true).count(),
            usize::from(x == 10)
        );
        assert_eq!(board["pcbSource"]["slots"].as_array().unwrap().len(), 1);
        assert!(
            !result["result"]["gates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gate| {
                    gate["code"] == "board-source-profile" || gate["code"] == "board-hole-geometry"
                })
        );
    }
}

fn resolve_source_modules(definitions: Vec<Value>) -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "source-preview",
        "Source preview",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":200,"y":200},"radius":0,"operation":"add"}]);
    document["modules"] = definitions.iter().enumerate().map(|(index, definition)| json!({
        "id":format!("mounted-{index}"),"definitionId":definition["id"],"hostBoardId":"host",
        "hostFace":"front","facingFace":"back","at":{"x":index*60,"y":0},"rotation":0,
        "gap":3,"attachment":"board","detached":false,"serviceClearance":0
    })).collect();
    document["moduleDefinitions"] = definitions.into();
    serde_json::from_str(
        &CoreEngine::new().request(
            &json!({
                "id":"resolve","kind":"resolve-modules","document":document,"boardId":"host"
            })
            .to_string(),
        ),
    )
    .unwrap()
}

#[test]
fn source_backed_module_board_is_one_complete_case_body() {
    let imported = import(
        include_str!("fixtures/pcb_front_slot.kicad_pcb").into(),
        "fixture:two-islands",
    );
    let mut definition = imported["result"].clone();
    let mut second = definition["board"]["contours"][0].clone();
    for point in second["points"].as_array_mut().unwrap() {
        point["x"] = (point["x"].as_f64().unwrap() + 30.0).into();
    }
    definition["board"]["contours"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let mut second = definition["board"]["pcbSource"]["contours"][0].clone();
    second["id"] = "second-exterior".into();
    for primitive in second["primitives"].as_array_mut().unwrap() {
        primitive["sourceId"] = "second-rectangle".into();
        for key in ["start", "end"] {
            let x = primitive["geometry"][key]["x"].as_f64().unwrap();
            primitive["geometry"][key]["x"] = (x + 30.0).into();
        }
    }
    definition["board"]["pcbSource"]["contours"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let result = resolve_source_modules(vec![definition]);
    assert_eq!(result["kind"], "modules-resolved", "{result}");
    let bodies = result["result"]["preview"]["bodies"].as_array().unwrap();
    assert_eq!(
        bodies.len(),
        1,
        "one physical PCB source must not be cloned per polygon"
    );
    assert_eq!(bodies[0]["regions"].as_array().unwrap().len(), 2);
    assert_eq!(
        bodies[0]["body"]["pcb_source"]["contours"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn source_failure_blocks_whole_exact_preview_and_retains_model_placements() {
    let valid = import(
        include_str!("fixtures/pcb_front_slot.kicad_pcb").into(),
        "fixture:valid",
    )["result"]
        .clone();
    let mut invalid = valid.clone();
    invalid["id"] = "fixture:invalid".into();
    invalid["board"]
        .as_object_mut()
        .unwrap()
        .remove("pcbSource");
    invalid["gates"].as_array_mut().unwrap().push(json!({
        "output":"mechanical","code":"board-source-profile","message":"Unqualified source outline"
    }));
    invalid["models"] = json!([{"assetId":"retained-model","offset":{"x":0,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0},"scale":{"x":1,"y":1,"z":1}}]);
    let result = resolve_source_modules(vec![valid, invalid]);
    assert_eq!(result["kind"], "modules-resolved", "{result}");
    assert!(
        result["result"]["preview"].is_null(),
        "must not accept a partial exact assembly: {result}"
    );
    assert_eq!(result["result"]["modules"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["result"]["modelPlacements"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        result["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["id"]
                    .as_str()
                    .unwrap()
                    .contains("board-source-profile")
            })
    );
}

#[test]
fn source_backed_preview_retains_legacy_voids_without_duplicate_mount_cuts() {
    for (raw, expected_holes, expected_drills) in [
        (include_str!("fixtures/pcb_front_slot.kicad_pcb"), 1, 0),
        (
            include_str!(
                "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
            ),
            38,
            38,
        ),
    ] {
        let definition = import(raw.into(), "fixture:compatibility-voids")["result"].clone();
        let result = resolve_source_modules(vec![definition]);
        assert_eq!(result["kind"], "modules-resolved", "{result}");
        let bodies = result["result"]["preview"]["bodies"].as_array().unwrap();
        assert_eq!(bodies.len(), 1);
        let body = &bodies[0];
        let regions = body["regions"].as_array().unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0]["holes"].as_array().unwrap().len(),
            expected_holes,
            "the current CAD reader needs the complete polygon void projection"
        );
        assert!(
            regions[0]["mounts"]
                .as_array()
                .is_none_or(|mounts| mounts.is_empty()),
            "the projected round voids must not be cut a second time"
        );
        assert_eq!(
            body["body"]["mounts"].as_array().map_or(0, Vec::len),
            expected_drills,
            "Monstertruck still needs all exact round drill definitions"
        );
        assert!(body["body"]["pcb_source"].is_object());
        if expected_drills == 0 {
            assert_eq!(
                body["body"]["pcb_source"]["slots"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
    }
}
