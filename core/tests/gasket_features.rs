//! Public prepared-CAD contract for the internal gasket construction.
use boardstudio_core::{CoreEngine, model::*};
use serde_json::{Value, json};

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

fn support_owner_contour(hole: bool, points: &[[f64; 2]]) -> Value {
    json!({"hole":hole,"points":points.iter().map(|[x,y]| json!({"x":x,"y":y})).collect::<Vec<_>>()})
}

fn support_owner_case(points: &[[f64; 2]], contours: Vec<Value>) -> CaseAssemblyIR {
    let support = points
        .iter()
        .map(|[x, y]| json!({"x":x,"y":y}))
        .collect::<Vec<_>>();
    serde_json::from_value(json!({"revision":1,"bodies":[{
        "revision":1,"contours":contours,"body":{"id":"owner-case","name":"Owner case","boardId":"board","kind":"plate",
            "thickness":1.0,"clearance":0.0,"features":[{"kind":"support-prism","id":"owner-support","points":support,"z":0.0,"height":1.0}]}
    }]})).unwrap()
}

#[test]
fn support_owner_cutoff_is_exact_and_strict_for_prepared_inputs() {
    let exterior = [[-2.0, 0.0], [0.0, 0.0], [0.0, 10.0], [-2.0, 10.0]];
    for edge in [1e-8, f64::from_bits(1e-8_f64.to_bits() + 1)] {
        let support = [[-1.0, 2.0], [edge, 2.0], [edge, 3.0], [-1.0, 3.0]];
        let error = prepare(support_owner_case(
            &support,
            vec![support_owner_contour(false, &exterior)],
        ))
        .unwrap_err();
        assert!(error.contains("owner-support"), "{error}");
        assert!(error.contains("one case exterior"), "{error}");
    }

    let below = [[-1.0, 2.0], [5e-9, 2.0], [5e-9, 3.0], [-1.0, 3.0]];
    assert!(
        prepare(support_owner_case(
            &below,
            vec![support_owner_contour(false, &exterior)]
        ))
        .is_ok()
    );
    let touching = [[-1.0, 2.0], [0.0, 2.0], [0.0, 3.0], [-1.0, 3.0]];
    assert!(
        prepare(support_owner_case(
            &touching,
            vec![support_owner_contour(false, &exterior)]
        ))
        .is_ok()
    );
}

#[test]
fn support_owner_keeps_concavity_multiple_owners_and_hole_contract() {
    let concave = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 3.0],
        [3.0, 3.0],
        [3.0, 10.0],
        [0.0, 10.0],
    ];
    let crossing_notch = [[2.5, 2.5], [3.5, 2.5], [3.5, 3.5], [2.5, 3.5]];
    assert!(
        prepare(support_owner_case(
            &crossing_notch,
            vec![support_owner_contour(false, &concave)]
        ))
        .is_err()
    );

    let overlap_support = [[6.0, 2.0], [7.0, 2.0], [7.0, 3.0], [6.0, 3.0]];
    let left = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let right = [[5.0, 0.0], [15.0, 0.0], [15.0, 10.0], [5.0, 10.0]];
    assert!(
        prepare(support_owner_case(
            &overlap_support,
            vec![
                support_owner_contour(false, &left),
                support_owner_contour(false, &right)
            ]
        ))
        .is_err()
    );

    let hole_exterior = [[-2.0, 0.0], [0.0, 0.0], [0.0, 10.0], [-2.0, 10.0]];
    let hole = [[-0.75, 4.0], [-0.25, 4.0], [-0.25, 6.0], [-0.75, 6.0]];
    let in_hole = [[-0.6, 4.5], [-0.4, 4.5], [-0.4, 5.5], [-0.6, 5.5]];
    assert!(
        prepare(support_owner_case(
            &in_hole,
            vec![
                support_owner_contour(false, &hole_exterior),
                support_owner_contour(true, &hole)
            ]
        ))
        .is_ok()
    );
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
fn support_prism_rejects_self_crossing_polygon_at_prepare_boundary() {
    let points = (0..5)
        .map(|i| {
            let angle =
                std::f64::consts::TAU * ((i * 2) % 5) as f64 / 5.0 - std::f64::consts::FRAC_PI_2;
            json!({"x": 10.0 + 8.0 * angle.cos(), "y": 10.0 + 8.0 * angle.sin()})
        })
        .collect::<Vec<_>>();
    let error = prepare(assembly(json!([{
        "kind":"support-prism","id":"crossed-support","points":points,"z":1.0,"height":1.0
    }])))
    .unwrap_err();
    assert!(error.contains("crossed-support"), "{error}");
    assert!(
        error.contains("cross") || error.contains("intersect"),
        "{error}"
    );
}

#[test]
fn support_prism_keeps_existing_finite_and_height_errors_before_topology() {
    let crossed = (0..5)
        .map(|i| {
            let angle =
                std::f64::consts::TAU * ((i * 2) % 5) as f64 / 5.0 - std::f64::consts::FRAC_PI_2;
            boardstudio_core::model::Vec2 {
                x: 10.0 + 8.0 * angle.cos(),
                y: 10.0 + 8.0 * angle.sin(),
            }
        })
        .collect::<Vec<_>>();
    let invalid_height = serde_json::from_value::<CaseAssemblyIR>(json!({
        "revision":1,
        "bodies":[{
            "revision":1,
            "contours":[{"hole":false,"points":[{"x":0.0,"y":0.0},{"x":30.0,"y":0.0},{"x":30.0,"y":20.0},{"x":0.0,"y":20.0}]}],
            "body":{"id":"bottom","name":"Tray","boardId":"board","kind":"tray","thickness":2.0,"clearance":0.0,"z":0.0,"wallHeight":8.0,"wallThickness":2.0,
                "features":[{"kind":"support-prism","id":"bad-height-support","points":crossed,"z":1.0,"height":0.0}]}
        }]
    })).unwrap();
    let height_error = match CoreEngine::new().handle(CoreRequest::PrepareCase {
        id: "features".into(),
        ir: invalid_height,
    }) {
        CoreReply::Error { message, .. } => message,
        other => panic!("invalid support height was accepted: {other:?}"),
    };
    assert!(
        height_error.contains("bad-height-support"),
        "{height_error}"
    );
    assert!(
        height_error.contains("finite positive geometry"),
        "{height_error}"
    );

    let mut invalid_coordinate = assembly(json!([{
        "kind":"support-prism","id":"bad-coordinate-support","points":[
            {"x":1.0,"y":1.0},{"x":6.0,"y":1.0},{"x":6.0,"y":6.0},{"x":1.0,"y":6.0}
        ],"z":1.0,"height":1.0
    }]));
    if let CaseFeature::SupportPrism { points, .. } =
        &mut invalid_coordinate.bodies[0].body.features.as_mut().unwrap()[0]
    {
        points[0].x = f64::NAN;
    }
    let coordinate_error = prepare(invalid_coordinate).unwrap_err();
    assert!(
        coordinate_error.contains("bad-coordinate-support"),
        "{coordinate_error}"
    );
    assert!(
        coordinate_error.contains("finite positive geometry"),
        "{coordinate_error}"
    );
}

#[test]
fn support_prism_preserves_exact_duplicates_large_rings_and_nearby_edges() {
    let duplicates = json!([{
        "kind":"support-prism","id":"duplicate-support","points":[
            {"x":1.0,"y":1.0},{"x":1.0,"y":1.0},{"x":6.0,"y":1.0},
            {"x":6.0,"y":6.0},{"x":1.0,"y":6.0},{"x":1.0,"y":1.0}
        ],"z":1.0,"height":1.0
    }]);
    let prepared_duplicates = prepare(assembly(duplicates.clone())).unwrap();
    let encoded = serde_json::to_value(prepared_duplicates).unwrap();
    assert_eq!(
        encoded["bodies"][0]["body"]["features"][0]["points"],
        duplicates[0]["points"]
    );

    let nearby = json!([{
        "kind":"support-prism","id":"nearby-support","points":[
            {"x":1.0,"y":1.0},{"x":20.0,"y":1.0},{"x":20.0,"y":19.0},
            {"x":1.0000000001,"y":19.0},{"x":1.0000000001,"y":10.0},
            {"x":1.0,"y":10.0}
        ],"z":1.0,"height":1.0
    }]);
    assert!(prepare(assembly(nearby)).is_ok());

    let points = (0..1025)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / 1025.0;
            json!({"x": 10.0 + 8.0 * angle.cos(), "y": 10.0 + 8.0 * angle.sin()})
        })
        .collect::<Vec<_>>();
    assert!(
        prepare(assembly(json!([{
            "kind":"support-prism","id":"large-support","points":points,"z":1.0,"height":1.0
        }])))
        .is_ok()
    );
}

#[test]
fn invalid_feature_dimensions_are_rejected_before_cad() {
    let invalid = json!([{"kind":"round-seat","id":"insert:bad","at":{"x":10.0,"y":1.0},
        "z":4.0,"height":-3.0,"diameter":3.2}]);
    let error = prepare(assembly(invalid)).unwrap_err();
    assert!(error.contains("insert:bad"), "{error}");
}

fn internal_document() -> Value {
    serde_json::from_str::<Value>(include_str!("fixtures/internal-gasket-v1/rectangle.json"))
        .unwrap()["document"]
        .clone()
}

fn resolve_internal(doc: Value) -> Value {
    resolve_internal_sized(doc, 100., 70.)
}

fn resolve_internal_sized(doc: Value, width: f64, height: f64) -> Value {
    let reply = CoreEngine::new().request(
        &json!({"kind":"resolve-mechanical","id":"resolve",
        "document":doc,"contours":[{"hole":false,"points":[
            {"x":0,"y":0},{"x":width,"y":0},{"x":width,"y":height},{"x":0,"y":height}
        ]}]})
        .to_string(),
    );
    serde_json::from_str::<Value>(&reply).unwrap()["assembly"].clone()
}

fn adoption_reachability_case(id: &str) -> (Value, Value, PreparedCaseIR) {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/adoption-reachability.json")).unwrap();
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["prototypeId"] == id)
        .unwrap()
        .clone();
    let mut original =
        serde_json::to_value(serde_json::from_value::<ProjectDoc>(internal_document()).unwrap())
            .unwrap();
    original["formatVersion"] = json!(boardstudio_core::migrate::CURRENT_VERSION);
    // Ticket 37: raw SetMechanical is deleted. The case mechanical is setup, not
    // behavior: open the document with it in place instead of writing it by edit.
    let mut authored = original.clone();
    authored["mechanical"] = case["request"]["document"]["mechanical"].clone();
    let mut engine = CoreEngine::new();
    let call = |engine: &mut CoreEngine, input: Value| {
        serde_json::from_str::<Value>(&engine.request(&input.to_string())).unwrap()
    };
    let opened = call(
        &mut engine,
        json!({"kind":"open","id":"open","document":authored}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let edited = opened;
    assert_eq!(edited["kind"], "scene", "{edited}");
    assert_eq!(
        edited["document"]["mechanical"], case["request"]["document"]["mechanical"],
        "the authored mechanical is established"
    );
    let resolved = call(&mut engine, case["request"].clone());
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(assembly["generationBlocked"], false, "{resolved}");
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["severity"] != "error")
    );
    let prepared = prepare(serde_json::from_value(assembly["case"].clone()).unwrap()).unwrap();
    let body = prepared
        .bodies
        .into_iter()
        .find(|body| body.body.id == case["preparedBody"]["body"]["id"])
        .unwrap();
    assert_eq!(serde_json::to_value(&body).unwrap(), case["preparedBody"]);
    let mut changed = case["request"]["document"]["mechanical"].clone();
    if id.contains("opening") {
        changed.as_object_mut().unwrap().remove("openings");
    } else {
        let fields: &[&str] = if id == "cone-wall" {
            &["headProfile", "lengthDatum"]
        } else {
            &["seatDepth"]
        };
        for field in fields {
            changed["internalGasket"]["hardware"][field] =
                original["mechanical"]["internalGasket"]["hardware"][field].clone();
        }
    }
    assert_eq!(
        changed, original["mechanical"],
        "only the relevant authoring fields change"
    );
    (fixture, case, body)
}

#[test]
fn adoption_reachability_countersunk_hardware_still_emits_contained_surface_seats() {
    let (_, case, prepared) = adoption_reachability_case("cone-wall");
    let hardware = &case["request"]["document"]["mechanical"]["internalGasket"]["hardware"];
    assert_eq!(hardware["headProfile"], "countersunk");
    assert_eq!(hardware["lengthDatum"], "overall");
    let features = prepared.body.features.unwrap();
    let heads: Vec<_> = features
        .iter()
        .filter_map(|feature| match feature {
            CaseFeature::ConicalSeat {
                at,
                z,
                height,
                diameter,
                ..
            } => Some((at, z, height, diameter)),
            _ => None,
        })
        .collect();
    assert_eq!(heads.len(), 4);
    for (at, z, height, diameter) in heads {
        assert_eq!(Some(*z), prepared.body.z);
        assert!(*z + height < z + prepared.body.thickness);
        let radius = diameter / 2.;
        // This retained source has a rectangular floor; every head fits inside it.
        for axis in [0, 1] {
            let coordinates: Vec<_> = prepared.regions[0]
                .outer
                .iter()
                .map(|p| if axis == 0 { p.x } else { p.y })
                .collect();
            let centre = if axis == 0 { at.x } else { at.y };
            assert!(centre - radius > coordinates.iter().copied().fold(f64::INFINITY, f64::min));
            assert!(
                centre + radius
                    < coordinates
                        .iter()
                        .copied()
                        .fold(f64::NEG_INFINITY, f64::max)
            );
        }
    }
    let attempt = &case["blockedWallAxisAttempt"];
    let blocked: Value =
        serde_json::from_str(&CoreEngine::new().request(&attempt["request"].to_string())).unwrap();
    assert_eq!(blocked["assembly"]["generationBlocked"], true);
    assert!(
        blocked["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["severity"] == "error"
                    && finding["message"]
                        .as_str()
                        .unwrap()
                        .contains(attempt["expectedReason"].as_str().unwrap())
            })
    );
}

#[test]
fn adoption_reachability_deeper_insert_seats_still_start_at_the_part_surface() {
    let (_, _, prepared) = adoption_reachability_case("sealed-round-seat");
    let seats: Vec<_> = prepared
        .body
        .features
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|feature| match feature {
            CaseFeature::RoundSeat { z, height, .. } => Some((z, height)),
            _ => None,
        })
        .collect();
    assert_eq!(seats.len(), 4);
    for (z, height) in seats {
        assert_eq!(Some(*z), prepared.body.z);
        assert_eq!(*height, 5.);
        assert_eq!(prepared.body.thickness - height, 1.5);
    }
}

fn check_authored_opening_reachability(id: &str, original: &str, removed_volume: f64) {
    let (fixture, case, prepared) = adoption_reachability_case(id);
    let prototype: PreparedCaseIR = serde_json::from_str(original).unwrap();
    let openings = prepared.body.openings.as_ref().unwrap();
    let source = prototype.body.openings.as_ref().unwrap();
    for (actual, expected) in openings.iter().zip(source) {
        assert_eq!(actual.points, expected.points);
        assert_eq!(actual.height, expected.height);
        assert!(actual.z > prepared.body.z.unwrap());
        assert!(actual.z + actual.height <= prepared.body.z.unwrap() + prepared.body.thickness);
    }
    assert_eq!(case["expectedBackend"]["removedVolumeMm3"], removed_volume);
    let mut body = serde_json::to_value(&prepared).unwrap();
    body["body"]["openings"] = json!(&openings[source.len()..]);
    assert_eq!(body["body"], fixture["baselinePreparedBottom"]["body"]);
    assert_eq!(
        body["regions"],
        fixture["baselinePreparedBottom"]["regions"]
    );
}

#[test]
fn adoption_reachability_authored_sealed_opening_survives_real_tray_preparation() {
    check_authored_opening_reachability(
        "sealed-opening",
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/sealed-opening.json"),
        16.,
    );
}

#[test]
fn adoption_reachability_authored_ring_island_survives_real_tray_preparation() {
    check_authored_opening_reachability(
        "opening-ring-island",
        include_str!("../../cad/monstertruck/tests/fixtures/adoption/opening-ring-island.json"),
        164.,
    );
}

#[test]
fn internal_case_has_nominal_plate_stable_exterior_and_independent_insert_closures() {
    let original = resolve_internal(internal_document());
    assert_eq!(original["generationBlocked"], false, "{original}");
    let plate = &original["nominalPlateContours"][0]["points"];
    assert!(
        plate
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["x"].as_f64().unwrap() >= 0.)
    );
    let bodies = original["case"]["bodies"].as_array().unwrap();
    let top = bodies
        .iter()
        .find(|b| b["body"]["name"] == "Top case")
        .expect("top case");
    assert!(
        top["body"]["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["kind"] == "round-seat")
    );
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
    assert!(
        invalid["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("loses contact"))
    );
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
    assert!(
        !changed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["severity"] == "error"),
        "{changed}"
    );
    assert_eq!(original["generatedHardware"], changed["generatedHardware"]);
    assert_eq!(original["suggestedMounts"], changed["suggestedMounts"]);
    assert_eq!(changed["gasketSupports"].as_array().unwrap().len(), 6);
}

#[test]
fn split_case_preserves_one_regions_closures_and_generates_the_other() {
    let mut input: Value =
        serde_json::from_str(include_str!("fixtures/internal-gasket-v1/rectangle.json")).unwrap();
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
    let mut input: Value =
        serde_json::from_str(include_str!("fixtures/internal-gasket-v1/rectangle.json")).unwrap();
    let concave = [
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
        input["contours"][0]["points"] = json!(
            concave
                .iter()
                .map(|(x, y)| json!({"x":x*cos-y*sin,"y":x*sin+y*cos}))
                .collect::<Vec<_>>()
        );
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
    let mut input: Value =
        serde_json::from_str(include_str!("fixtures/internal-gasket-v1/rectangle.json")).unwrap();
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
        {"id":"left","name":"Left","boardId":"board","matrixIds":["left"],"partIds":["left-marker"]},
        {"id":"right","name":"Right","boardId":"board","matrixIds":["right"],"partIds":["right-marker"],"mirrorLink":{"sourceId":"left","axisX":150}}]);
    for automatic in [false, true] {
        input["document"]["mechanical"]["gasketLayout"]["autoSize"] = json!(automatic);
        input["document"]["mechanical"]["internalGasket"]["autoCount"] = json!(automatic);
        let result = serde_json::from_str::<Value>(&CoreEngine::new().request(&input.to_string()))
            .unwrap()["assembly"]
            .clone();
        assert_eq!(result["generationBlocked"], false, "{result}");
        let supports = result["gasketSupports"].as_array().unwrap();
        for slot in 0..supports.len() / 2 {
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
                (left["at"]["x"].as_f64().unwrap() + right["at"]["x"].as_f64().unwrap() - 300.)
                    .abs()
                    < 1e-6
            );
            assert!(
                (left["at"]["y"].as_f64().unwrap() - right["at"]["y"].as_f64().unwrap()).abs()
                    < 1e-6
            );
        }
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
    assert!(
        !top(&no_boss)["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["kind"] == "support-prism")
    );
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
        let mut input: Value =
            serde_json::from_str(include_str!("fixtures/internal-gasket-v1/rectangle.json"))
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
    let mut doc = internal_document();
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(6);
    let original = resolve_internal(doc.clone());
    let support = original["gasketSupports"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":support["outlineKey"],
        "anchor":support["anchor"],"unlinked":false,"placement":"generated"}]);
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(4);
    let changed = resolve_internal(doc.clone());
    assert_eq!(changed["generationBlocked"], false, "{changed}");
    doc["mechanical"]["gasketLayout"]["supports"][0]["placement"] = json!("user");
    assert_eq!(resolve_internal(doc)["generationBlocked"], true);
}

#[test]
fn automatic_gaskets_use_mixed_cut_lengths_and_support_four_sides() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["gasketLayout"]["length"] = json!(80);
    let result = resolve_internal(doc);
    assert_eq!(
        result["generationBlocked"], false,
        "{}",
        result["diagnostics"]
    );
    let supports = result["gasketSupports"].as_array().unwrap();
    let mut lengths = vec![];
    let mut sides = std::collections::HashSet::new();
    for support in supports {
        let x = support["normal"]["x"].as_f64().unwrap();
        let y = support["normal"]["y"].as_f64().unwrap();
        sides.insert(if x.abs() > y.abs() {
            if x > 0. { 0 } else { 1 }
        } else if y > 0. {
            2
        } else {
            3
        });
        let length = support["length"].as_f64().unwrap();
        assert!((5. ..=80.).contains(&length) && (length % 10. == 0. || length == 5.));
        lengths.push(length as u32);
    }
    lengths.sort();
    lengths.dedup();
    assert!(lengths.len() > 1, "{supports:?}");
    assert_eq!(sides.len(), 4, "{supports:?}");
}

#[test]
fn oversized_saved_gasket_remains_editable_and_resizing_repairs_it() {
    let mut doc = internal_document();
    let initial = resolve_internal(doc.clone());
    let support = &initial["gasketSupports"][3];
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":support["outlineKey"],
        "anchor":support["anchor"],"placement":"user","length":200,"width":3
    }]);
    let invalid = resolve_internal(doc.clone());
    assert_eq!(invalid["generationBlocked"], true);
    let retained = invalid["gasketSupports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == support["id"])
        .unwrap();
    assert_eq!(retained["length"], 200.);
    assert!(retained["fitError"].is_string());
    assert_eq!(retained["anchor"], support["anchor"]);
    assert!(!invalid["gasketTracks"].as_array().unwrap().is_empty());
    doc["mechanical"]["gasketLayout"]["supports"][0]["length"] = json!(10);
    let repaired = resolve_internal(doc.clone());
    assert_eq!(
        repaired["generationBlocked"], false,
        "{}",
        repaired["diagnostics"]
    );
    let saved = repaired["gasketSupports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == support["id"])
        .unwrap();
    assert_eq!(saved["length"], 10.);
    assert_eq!(
        initial["gasketSupports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].clone())
            .collect::<Vec<_>>(),
        repaired["gasketSupports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(saved["anchor"], support["anchor"]);
    assert!(saved["fitError"].is_null());
    assert!(
        repaired["generatedMaterials"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["featureId"] == support["id"] && m["size"]["x"] == 10.)
    );
}

#[test]
fn dense_automatic_gaskets_balance_all_sides_without_exhausting_short_edges() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(12);
    let result = resolve_internal(doc);
    assert_eq!(
        result["generationBlocked"], false,
        "{}",
        result["diagnostics"]
    );
    let mut sides = [0; 4];
    for support in result["gasketSupports"].as_array().unwrap() {
        let x = support["normal"]["x"].as_f64().unwrap();
        let y = support["normal"]["y"].as_f64().unwrap();
        let side = if x.abs() > y.abs() {
            if x > 0. { 0 } else { 1 }
        } else if y > 0. {
            2
        } else {
            3
        };
        sides[side] += 1;
    }
    assert_eq!(sides.iter().sum::<usize>(), 12);
    assert!(sides.iter().all(|count| *count > 0));
}

#[test]
fn automatic_support_count_adds_coverage_and_manual_count_remains_available() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["internalGasket"]["autoCount"] = json!(true);
    let automatic = resolve_internal(doc.clone());
    assert_eq!(automatic["generationBlocked"], false, "{automatic}");
    assert_eq!(automatic["gasketSupports"].as_array().unwrap().len(), 6);
    doc["mechanical"]["internalGasket"]["autoCount"] = json!(false);
    let manual = resolve_internal(doc);
    assert_eq!(manual["gasketSupports"].as_array().unwrap().len(), 4);
}

#[test]
fn automatic_count_scales_with_outline_and_respects_older_manual_counts() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["internalGasket"]
        .as_object_mut()
        .unwrap()
        .remove("autoCount");
    let larger = resolve_internal_sized(doc.clone(), 150., 100.);
    assert_eq!(larger["generationBlocked"], false, "{larger}");
    assert_eq!(larger["gasketSupports"].as_array().unwrap().len(), 8);
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(6);
    assert_eq!(
        resolve_internal(doc)["gasketSupports"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
}

#[test]
fn automatic_count_keeps_saved_support_slots_when_outline_target_is_smaller() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["internalGasket"]["supportCount"] = json!(12);
    let initial = resolve_internal(doc.clone());
    let support = initial["gasketSupports"][11].clone();
    doc["mechanical"]["gasketLayout"]["supports"] = json!([{
        "id":support["id"],"regionId":support["regionId"],"outlineKey":support["outlineKey"],
        "anchor":support["anchor"],"length":support["length"],"width":support["width"],"placement":"user","unlinked":false
    }]);
    doc["mechanical"]["internalGasket"]["autoCount"] = json!(true);
    let result = resolve_internal(doc);
    let retained = result["gasketSupports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == support["id"])
        .expect("pinned slot retained");
    assert_eq!(retained["anchor"], support["anchor"]);
}

#[test]
fn automatic_long_edge_pairs_are_centered_and_symmetric() {
    let mut doc = internal_document();
    doc["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
    doc["mechanical"]["internalGasket"]["autoCount"] = json!(true);
    let result = resolve_internal_sized(doc, 150., 70.);
    assert_eq!(
        result["generationBlocked"], false,
        "{}",
        result["diagnostics"]
    );
    let supports = result["gasketSupports"].as_array().unwrap();
    let bottom: Vec<_> = supports
        .iter()
        .filter(|s| s["normal"]["y"].as_f64().unwrap() < -0.9)
        .collect();
    assert_eq!(bottom.len(), 2, "{bottom:?}");
    assert_eq!(bottom[0]["length"], bottom[1]["length"]);
    let centers: Vec<_> = bottom
        .iter()
        .map(|s| s["at"]["x"].as_f64().unwrap())
        .collect();
    assert!(
        (centers[0] + centers[1] - 150.).abs() < 0.001,
        "{centers:?}"
    );
    assert!((centers[0] - centers[1]).abs() > bottom[0]["length"].as_f64().unwrap() + 1.);
}

#[test]
fn run_placement_handles_rotated_reversed_and_split_outlines() {
    for fixture in [
        include_str!("fixtures/internal-gasket-v1/rectangle.json"),
        include_str!("fixtures/internal-gasket-v1/rotated-concave.json"),
        include_str!("fixtures/internal-gasket-v1/split.json"),
    ] {
        for angle in [0_f64, 0.37, 1.57] {
            let mut request: Value = serde_json::from_str(fixture).unwrap();
            request["document"]["mechanical"]["gasketLayout"]["autoSize"] = json!(true);
            request["document"]["mechanical"]["internalGasket"]["autoCount"] = json!(true);
            for contour in request["contours"].as_array_mut().unwrap() {
                let points = contour["points"].as_array_mut().unwrap();
                for p in points.iter_mut() {
                    let x = p["x"].as_f64().unwrap();
                    let y = p["y"].as_f64().unwrap();
                    p["x"] = json!(x * angle.cos() - y * angle.sin() + 240.);
                    p["y"] = json!(x * angle.sin() + y * angle.cos() - 170.);
                }
                points.reverse();
            }
            let reply: Value =
                serde_json::from_str(&CoreEngine::new().request(&request.to_string())).unwrap();
            let result = &reply["assembly"];
            assert_eq!(
                result["generationBlocked"], false,
                "angle {angle}: {}",
                result["diagnostics"]
            );
            assert!(
                result["gasketSupports"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["fitError"].is_null())
            );
        }
    }
}

#[test]
fn automatic_closures_scale_with_case_perimeter() {
    for (width, height, expected) in [
        (100., 70., 4),
        (150., 110., 6),
        (380., 170., 10),
        (500., 180., 12),
    ] {
        let resolved = resolve_internal_sized(internal_document(), width, height);
        assert_eq!(resolved["generationBlocked"], false, "{resolved}");
        assert_eq!(
            resolved["suggestedMounts"].as_array().unwrap().len(),
            expected,
            "{width} by {height} case"
        );
        let mounts = resolved["suggestedMounts"].as_array().unwrap();
        for (i, a) in mounts.iter().enumerate() {
            for b in &mounts[..i] {
                let dx = a["at"]["x"].as_f64().unwrap() - b["at"]["x"].as_f64().unwrap();
                let dy = a["at"]["y"].as_f64().unwrap() - b["at"]["y"].as_f64().unwrap();
                assert!(
                    dx.hypot(dy) > 25.,
                    "Closures cluster on {width} by {height}"
                );
            }
        }
    }
}

#[test]
fn oversized_automatic_closure_layout_is_rejected() {
    let resolved = resolve_internal_sized(internal_document(), 3900., 100.);
    assert_eq!(resolved["generationBlocked"], true);
    assert!(
        resolved["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["message"]
                .as_str()
                .unwrap()
                .contains("more than 64 automatic closure screws"))
    );
}
