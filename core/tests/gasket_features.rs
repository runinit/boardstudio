//! Public prepared-CAD contract for the internal gasket construction.
use boardstudio_core::{model::*, CoreEngine};
use serde_json::{json, Value};

fn prepare(ir: CaseAssemblyIR) -> Result<PreparedCaseAssemblyIR, String> {
    match CoreEngine::new().handle(CoreRequest::PrepareCase {
        id: "features".into(),
        ir,
    }) {
        CoreReply::CasePrepared { ir, .. } => Ok(ir),
        CoreReply::Error { message, .. } => Err(message),
        other => panic!("unexpected reply: {other:?}"),
    }
}

fn assembly(features: Value) -> CaseAssemblyIR {
    serde_json::from_value(json!({"revision":1,"bodies":[{
        "revision":1,"contours":[{"hole":false,"points":[
            {"x":0.0,"y":0.0},{"x":30.0,"y":0.0},{"x":30.0,"y":20.0},{"x":0.0,"y":20.0}
        ]}],"body":{"id":"bottom","name":"Tray","boardId":"board","kind":"tray",
            "thickness":2.0,"clearance":0.0,"z":0.0,"wallHeight":8.0,"wallThickness":2.0,
            "features":features}
    }]}))
    .unwrap()
}

#[test]
fn wall_supports_and_blind_seats_survive_preparation() {
    let features = json!([
        {"kind":"support-prism","id":"support:0","points":[
            {"x":1.0,"y":5.0},{"x":6.0,"y":5.0},{"x":6.0,"y":15.0},{"x":1.0,"y":15.0}
        ],"z":2.0,"height":4.0},
        {"kind":"round-seat","id":"insert:0","at":{"x":10.0,"y":1.0},
            "z":4.0,"height":3.0,"diameter":3.2}
    ]);
    let prepared = prepare(assembly(features.clone())).unwrap();
    let encoded = serde_json::to_value(prepared).unwrap();
    assert_eq!(encoded["bodies"][0]["body"]["features"], features);
}

#[test]
fn invalid_feature_dimensions_are_rejected_before_cad() {
    let invalid = json!([{"kind":"round-seat","id":"insert:bad","at":{"x":10.0,"y":1.0},
        "z":4.0,"height":-3.0,"diameter":3.2}]);
    let error = prepare(assembly(invalid)).unwrap_err();
    assert!(error.contains("insert:bad"), "{error}");
}

fn internal_document() -> Value {
    serde_json::from_str::<Value>(include_str!(
        "../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
    ))
    .unwrap()["document"]
        .clone()
}

fn resolve_internal(doc: Value) -> Value {
    let reply = CoreEngine::new().request(
        &json!({"kind":"resolve-mechanical","id":"resolve",
        "document":doc,"contours":[{"hole":false,"points":[
            {"x":0,"y":0},{"x":100,"y":0},{"x":100,"y":70},{"x":0,"y":70}
        ]}]})
        .to_string(),
    );
    serde_json::from_str::<Value>(&reply).unwrap()["assembly"].clone()
}

#[test]
fn internal_case_has_nominal_plate_stable_exterior_and_independent_insert_closures() {
    let original = resolve_internal(internal_document());
    assert_eq!(original["generationBlocked"], false, "{original}");
    let plate = &original["nominalPlateContours"][0]["points"];
    assert!(plate
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["x"].as_f64().unwrap() >= 0.));
    let bodies = original["case"]["bodies"].as_array().unwrap();
    let top = bodies
        .iter()
        .find(|b| b["body"]["name"] == "Top case")
        .expect("top case");
    assert!(top["body"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["kind"] == "round-seat"));
    assert_eq!(original["gasketSupports"].as_array().unwrap().len(), 4);
    let support = &original["gasketSupports"][0];
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":support["outlineKey"],
        "anchor":support["anchor"].as_f64().unwrap()+0.002,"unlinked":true
    }]);
    let moved = resolve_internal(doc);
    assert_eq!(moved["generationBlocked"], false, "{moved}");
    let changed = moved["case"]["bodies"].as_array().unwrap();
    assert_eq!(
        top,
        changed
            .iter()
            .find(|b| b["body"]["id"] == "retainer")
            .unwrap()
    );
    assert_eq!(original["generatedHardware"], moved["generatedHardware"]);
    assert_eq!(original["pcbReference"], moved["pcbReference"]);
    let bottom = |bs: &Vec<Value>| {
        bs.iter().find(|b| b["body"]["id"] == "bottom").unwrap()["contours"].clone()
    };
    assert_eq!(bottom(bodies), bottom(changed));
}

#[test]
fn internal_stack_rejects_lost_preload_and_respects_wall_budget() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketTravel"] = json!(0.3);
    let invalid = resolve_internal(doc);
    assert_eq!(invalid["generationBlocked"], true);
    assert!(invalid["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["message"].as_str().unwrap().contains("loses contact")));
    let mut doc = internal_document();
    doc["mechanical"]["internalGasket"]["minimumWall"] = json!(4.1);
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}

#[test]
fn deeper_insert_grows_top_without_moving_seam_or_pads() {
    let base = resolve_internal(internal_document());
    let mut doc = internal_document();
    doc["mechanical"]["internalGasket"]["hardware"]["seatDepth"] = json!(6.5);
    let grown = resolve_internal(doc);
    assert_eq!(grown["generationBlocked"], false, "{grown}");
    let top = |assembly: &Value| {
        assembly["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["body"]["id"] == "retainer")
            .unwrap()["body"]
            .clone()
    };
    assert_eq!(top(&base)["z"], top(&grown)["z"]);
    assert_eq!(
        top(&grown)["thickness"].as_f64().unwrap() - top(&base)["thickness"].as_f64().unwrap(),
        2.
    );
    assert_eq!(base["gasketSupports"], grown["gasketSupports"]);
}

#[test]
fn structural_features_cannot_extend_the_exterior() {
    let outside = json!([{"kind":"support-prism","id":"outside","points":[
        {"x":-1,"y":5},{"x":6,"y":5},{"x":6,"y":15},{"x":-1,"y":15}],"z":2,"height":4}]);
    let error = prepare(assembly(outside)).unwrap_err();
    assert!(error.contains("outside"), "{error}");
}

#[test]
fn adopted_closures_survive_support_count_changes() {
    let original = resolve_internal(internal_document());
    let mut doc = internal_document();
    assert_eq!(original["suggestedMounts"].as_array().unwrap().len(), 4);
    doc["mechanical"]["closureMounts"] = original["suggestedMounts"].clone();
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(6);
    let changed = resolve_internal(doc);
    assert_eq!(changed["generationBlocked"], false, "{changed}");
    assert!(!changed["diagnostics"].as_array().unwrap().iter().any(|finding| finding["severity"] == "error"), "{changed}");
    assert_eq!(original["generatedHardware"], changed["generatedHardware"]);
    assert_eq!(original["suggestedMounts"], changed["suggestedMounts"]);
    assert_eq!(changed["gasketSupports"].as_array().unwrap().len(), 6);
}

#[test]
fn split_case_preserves_one_regions_closures_and_generates_the_other() {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
    ))
    .unwrap();
    let mut second = input["contours"][0].clone();
    for point in second["points"].as_array_mut().unwrap() {
        point["x"] = json!(point["x"].as_f64().unwrap() + 200.);
    }
    input["contours"].as_array_mut().unwrap().push(second);
    let resolve = |request: &Value| {
        serde_json::from_str::<Value>(&CoreEngine::new().request(&request.to_string())).unwrap()
            ["assembly"]
            .clone()
    };
    let original = resolve(&input);
    assert_eq!(original["generationBlocked"], false, "{original}");
    assert_eq!(original["suggestedMounts"].as_array().unwrap().len(), 8);
    input["document"]["mechanical"]["closureMounts"] =
        json!(original["suggestedMounts"].as_array().unwrap()[..4].to_vec());
    let adopted = resolve(&input);
    assert_eq!(adopted["generationBlocked"], false, "{adopted}");
    assert_eq!(adopted["suggestedMounts"], original["suggestedMounts"]);
}

#[test]
fn overhanging_keycap_grows_capture_space_without_changing_pcb() {
    let base = resolve_internal(internal_document());
    let mut doc = internal_document();
    doc["boards"][0]["partIds"] = json!(["key"]);
    doc["definitions"] = json!([{"id":"cap","name":"Cap envelope","kind":"custom","keycap":{"x":20,"y":20},"courtyard":[],"pads":[]}]);
    doc["parts"] = json!([{"id":"key","definitionId":"cap","reference":"K1","pose":{"at":{"x":2,"y":35},"rotation":0},"side":"front"}]);
    let with_cap = resolve_internal(doc);
    assert_eq!(with_cap["generationBlocked"], false, "{with_cap}");
    let min_x = |a: &Value| {
        a["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["body"]["id"] == "bottom")
            .unwrap()["contours"][0]["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .fold(f64::INFINITY, f64::min)
    };
    assert!(min_x(&with_cap) < min_x(&base) - 4.);
    assert_eq!(base["pcbReference"], with_cap["pcbReference"]);
}

#[test]
fn downward_component_envelope_lowers_floor_without_moving_foam_contacts() {
    let base = resolve_internal(internal_document());
    let mut doc = internal_document();
    doc["boards"][0]["partIds"] = json!(["component"]);
    doc["definitions"] = json!([{"id":"component","name":"Component envelope","kind":"custom","courtyard":[],"pads":[]}]);
    doc["parts"] = json!([{"id":"component","definitionId":"component","reference":"U1","pose":{"at":{"x":50,"y":35},"rotation":0},"side":"front"}]);
    doc["mechanical"]["profiles"] = json!([{"definitionId":"component","source":"fixture","plateToPcb":5,"cutouts":[],"clearanceVolumes":[{"points":[{"x":-2,"y":-2},{"x":2,"y":-2},{"x":2,"y":2},{"x":-2,"y":2}],"z":-4,"height":2}]}]);
    doc["mechanical"]["internalGasket"]["hardware"]["screwLengths"] = json!([15, 17.5]);
    let resolved = resolve_internal(doc);
    assert_eq!(resolved["generationBlocked"], false, "{resolved}");
    assert_eq!(base["gasketSupports"], resolved["gasketSupports"]);
    let bottom = &resolved["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["body"]["id"] == "bottom")
        .unwrap()["body"];
    assert!(bottom["z"].as_f64().unwrap() + bottom["thickness"].as_f64().unwrap() <= -4.15 + 1e-9);
}

#[test]
fn zero_pcb_gap_cannot_bypass_rigid_contact_validation() {
    let mut doc = internal_document();
    doc["mechanical"]["internalGasket"]["supportClearance"] = json!(0);
    doc["mechanical"]["internalGasket"]["tolerance"] = json!(0);
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}

#[test]
fn rotated_and_concave_regions_keep_internal_features_within_the_wall_budget() {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
    ))
    .unwrap();
    let concave = vec![
        (0., 0.),
        (180., 0.),
        (180., 100.),
        (120., 100.),
        (120., 70.),
        (60., 70.),
        (60., 100.),
        (0., 100.),
    ];
    for angle in [0_f64, 17., -32.] {
        let (sin, cos) = angle.to_radians().sin_cos();
        input["contours"][0]["points"] = json!(concave
            .iter()
            .map(|(x, y)| json!({"x":x*cos-y*sin,"y":x*sin+y*cos}))
            .collect::<Vec<_>>());
        let resolved =
            serde_json::from_str::<Value>(&CoreEngine::new().request(&input.to_string())).unwrap();
        assert_eq!(
            resolved["assembly"]["generationBlocked"], false,
            "angle {angle}: {resolved}"
        );
    }
}

#[test]
fn explicit_overall_length_and_countersink_are_resolved_together() {
    let mut doc = internal_document();
    let hardware = &mut doc["mechanical"]["internalGasket"]["hardware"];
    hardware["lengthDatum"] = json!("overall");
    hardware["headProfile"] = json!("countersunk");
    hardware["screwLengths"] = json!([16]);
    let resolved = resolve_internal(doc);
    assert_eq!(resolved["generationBlocked"], false, "{resolved}");
    let bottom = &resolved["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["body"]["id"] == "bottom")
        .unwrap()["body"];
    assert_eq!(
        bottom["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["kind"] == "conical-seat")
            .count(),
        4
    );
}

#[test]
fn generated_anchors_repair_but_user_positions_stay_protected() {
    let original = resolve_internal(internal_document());
    let support = &original["gasketSupports"][0];
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":"previous outline",
        "anchor":support["anchor"],"unlinked":false,"placement":"generated"}]);
    assert_eq!(resolve_internal(doc.clone())["generationBlocked"], false);
    doc["mechanical"]["gasketLayout"]["supports"][0]["placement"] = json!("user");
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}

#[test]
fn linked_split_supports_use_mirrored_tracks() {
    let mut input: Value = serde_json::from_str(include_str!(
        "../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
    ))
    .unwrap();
    let mut right = input["contours"][0].clone();
    for point in right["points"].as_array_mut().unwrap() {
        point["x"] = json!(300. - point["x"].as_f64().unwrap());
    }
    input["contours"].as_array_mut().unwrap().push(right);
    input["document"]["definitions"] =
        json!([{"id":"marker","name":"Marker","kind":"custom","courtyard":[],"pads":[]}]);
    input["document"]["parts"] = json!([
        {"id":"left-marker","definitionId":"marker","reference":"L","pose":{"at":{"x":50,"y":35},"rotation":0},"side":"front"},
        {"id":"right-marker","definitionId":"marker","reference":"R","pose":{"at":{"x":250,"y":35},"rotation":0},"side":"front"}]);
    input["document"]["layouts"] = json!([
        {"id":"left","name":"Left","boardId":"board","matrixId":"left","partIds":["left-marker"]},
        {"id":"right","name":"Right","boardId":"board","matrixId":"right","partIds":["right-marker"],"mirrorLink":{"sourceId":"left","axisX":150}}]);
    let result = serde_json::from_str::<Value>(&CoreEngine::new().request(&input.to_string()))
        .unwrap()["assembly"]
        .clone();
    assert_eq!(result["generationBlocked"], false, "{result}");
    let supports = result["gasketSupports"].as_array().unwrap();
    for slot in 0..4 {
        let left = supports
            .iter()
            .find(|s| s["id"] == format!("left:{slot}"))
            .unwrap();
        let right = supports
            .iter()
            .find(|s| s["id"] == format!("right:{slot}"))
            .unwrap();
        assert_eq!(left["pairId"], right["id"]);
        assert!(
            (left["at"]["x"].as_f64().unwrap() + right["at"]["x"].as_f64().unwrap() - 300.).abs()
                < 1e-6
        );
        assert!(
            (left["at"]["y"].as_f64().unwrap() - right["at"]["y"].as_f64().unwrap()).abs() < 1e-6
        );
    }
}

#[test]
fn longer_screw_precedes_a_supported_downward_boss_fallback() {
    let mut doc = internal_document();
    doc["mechanical"]["internalGasket"]["hardware"]["screwLengths"] = json!([12, 15]);
    let no_boss = resolve_internal(doc.clone());
    assert_eq!(no_boss["generationBlocked"], false, "{no_boss}");
    let top = |a: &Value| {
        a["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["body"]["id"] == "retainer")
            .unwrap()["body"]
            .clone()
    };
    assert!(!top(&no_boss)["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["kind"] == "support-prism"));
    doc["mechanical"]["internalGasket"]["hardware"]["screwLengths"] = json!([12]);
    let fallback = resolve_internal(doc);
    assert_eq!(fallback["generationBlocked"], false, "{fallback}");
    assert_eq!(
        top(&fallback)["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["kind"] == "support-prism")
            .count(),
        4
    );
    assert_eq!(top(&fallback)["z"], top(&no_boss)["z"]);
}

#[test]
fn custom_insert_length_process_and_fixed_screw_are_not_inferred() {
    let mut doc = internal_document();
    let hardware = &mut doc["mechanical"]["internalGasket"]["hardware"];
    hardware["insertLength"] = json!(4);
    hardware["installation"] = json!("heat-set");
    hardware["screwLengths"] = json!([12, 15]);
    hardware["fixedLength"] = json!(12);
    let result = resolve_internal(doc.clone());
    assert_eq!(result["generationBlocked"], false, "{result}");
    for item in result["generatedHardware"].as_array().unwrap() {
        if item["id"].as_str().unwrap().starts_with("insert:") {
            assert_eq!(item["length"].as_f64().unwrap(), 4.);
        }
        if item["id"].as_str().unwrap().starts_with("screw:") {
            assert_eq!(item["length"].as_f64().unwrap(), 12.);
        }
    }
    doc["mechanical"]["method"] = json!("cnc");
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}

#[test]
fn every_supplied_foam_size_keeps_free_material_dimensions() {
    for (id, length, width, thickness) in [
        ("A2", 20., 3., 2.),
        ("A3", 20., 3., 3.),
        ("A4", 20., 3., 4.),
        ("B2", 20., 4., 2.),
        ("B3", 20., 4., 3.),
        ("B4", 20., 4., 4.),
        ("E2", 80., 3., 2.),
        ("E3", 80., 3., 3.),
        ("E4", 80., 3., 4.),
        ("F2", 80., 4., 2.),
        ("F3", 80., 4., 3.),
        ("F4", 80., 4., 4.),
        ("F5", 80., 4., 5.),
    ] {
        let mut input: Value = serde_json::from_str(include_str!(
            "../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
        ))
        .unwrap();
        input["contours"][0]["points"] =
            json!([{"x":0,"y":0},{"x":420,"y":0},{"x":420,"y":280},{"x":0,"y":280}]);
        let foam = &mut input["document"]["mechanical"]["gasketLayout"];
        foam["length"] = json!(length);
        foam["width"] = json!(width);
        foam["thickness"] = json!(thickness);
        foam["presetId"] = json!(id);
        let result = serde_json::from_str::<Value>(&CoreEngine::new().request(&input.to_string()))
            .unwrap()["assembly"]
            .clone();
        assert_eq!(result["generationBlocked"], false, "{id}: {result}");
        let materials = result["generatedMaterials"]
            .as_array()
            .expect("foam cutting list");
        assert_eq!(materials.len(), 4);
        for item in materials {
            assert_eq!(item["quantity"], 2);
            assert_eq!(item["size"], json!({"x":length,"y":width,"z":thickness}));
            assert_eq!(item["presetId"], id);
        }
    }
}

#[test]
fn separately_specified_adhesive_is_not_compressed_as_foam() {
    let base = resolve_internal(internal_document());
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["adhesiveThickness"] = json!(0.1);
    let adhesive = resolve_internal(doc);
    assert_eq!(adhesive["generationBlocked"], false, "{adhesive}");
    let seam = |a: &Value| {
        a["stack"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["id"] == "retainer")
            .unwrap()["z"]
            .as_f64()
            .unwrap()
    };
    assert!((seam(&adhesive) - seam(&base) - 0.1).abs() < 1e-8);
}

#[test]
fn omitted_advanced_clearances_use_the_spec_defaults() {
    let mut doc = internal_document();
    let settings = doc["mechanical"]["internalGasket"].as_object_mut().unwrap();
    settings.remove("minimumWall");
    settings.remove("supportClearance");
    let defaulted = resolve_internal(doc);
    let explicit = resolve_internal(internal_document());
    assert_eq!(defaulted["case"], explicit["case"]);
}

#[test]
fn reducing_count_discards_generated_supports_but_preserves_user_supports() {
    let original = resolve_internal(internal_document());
    let mut doc = internal_document();
    let support = original["gasketSupports"].as_array().unwrap().last().unwrap();
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":support["outlineKey"],
        "anchor":support["anchor"],"unlinked":false,"placement":"generated"}]);
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(3);
    let changed = resolve_internal(doc.clone());
    assert_eq!(changed["generationBlocked"], false, "{changed}");
    doc["mechanical"]["gasketLayout"]["supports"][0]["placement"] = json!("user");
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}
