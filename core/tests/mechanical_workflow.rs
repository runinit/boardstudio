use boardstudio_core::{CoreEngine, model::ProjectDoc};
use serde_json::{Value, json};

fn configuration() -> Value {
    json!({"boardId":"board","method":"printed","mount":"rigid","integratedPlateFrame":false,
    "plateThickness":1.5,"plateFoamThickness":1.0,"pcbThickness":1.6,"bottomFoamThickness":0.5,
    "batteryHeight":3.0,"bottomThickness":2.0,"plateToPcb":3.5,"wallThickness":2.0,"clearance":0.2,"profiles":[],"mounts":[]})
}
fn document() -> Value {
    let mut doc = serde_json::to_value(ProjectDoc::empty("mechanical", "Mechanical")).unwrap();
    doc["boards"] = json!([{"id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6,"traces":[],"vias":[]}]);
    doc
}
fn contours() -> Value {
    json!([{"hole":false,"points":[{"x":0,"y":0},{"x":60,"y":0},{"x":60,"y":40},{"x":0,"y":40}]},
    {"hole":true,"points":[{"x":10,"y":10},{"x":20,"y":10},{"x":20,"y":20},{"x":10,"y":20}]}])
}
fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

#[test]
fn configuration_is_undoable_persistent_and_resolution_is_stateless() {
    let mut engine = CoreEngine::new();
    assert_eq!(
        request(
            &mut engine,
            json!({"kind":"open","id":"open","document":document()})
        )["kind"],
        "scene"
    );
    let changed = request(
        &mut engine,
        json!({"kind":"edit","id":"edit","command":{"baseRevision":0,"transactionId":"configure","phase":"commit","targetIds":[],"operation":{"kind":"set-mechanical","configuration":configuration()}}}),
    );
    assert_eq!(changed["kind"], "scene");
    assert_eq!(changed["document"]["mechanical"], configuration());
    let saved = changed["document"].clone();
    let resolved = request(
        &mut engine,
        json!({"kind":"resolve-mechanical","id":"resolve","document":saved,"contours":contours()}),
    );
    assert_eq!(resolved["assembly"]["revision"], 1);
    let snapshot = request(&mut engine, json!({"kind":"snapshot","id":"snapshot"}));
    assert_eq!(snapshot["document"], saved);
    let undone = request(&mut engine, json!({"kind":"undo","id":"undo"}));
    assert!(undone["document"].get("mechanical").is_none());
    let redone = request(&mut engine, json!({"kind":"redo","id":"redo"}));
    assert_eq!(redone["document"]["mechanical"], configuration());
    let reopened = request(
        &mut CoreEngine::new(),
        json!({"kind":"open","id":"reopen","document":saved}),
    );
    assert_eq!(reopened["document"]["mechanical"], configuration());
}

#[test]
fn opening_allowance_is_explicit_shared_and_preserves_nominal_geometry() {
    let mut doc = document();
    doc["mechanical"] = configuration();
    let nominal = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"nominal","document":doc,"contours":contours()}),
    );
    let mut config = configuration();
    config["openingAllowance"] = json!(0.2);
    doc["mechanical"] = config;
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"resolve","document":doc,"contours":contours()}),
    );
    let assembly = &result["assembly"];
    assert_eq!(
        assembly["nominalPlateContours"],
        nominal["assembly"]["plateContours"]
    );
    assert_eq!(
        assembly["plateContours"][0],
        nominal["assembly"]["plateContours"][0]
    );
    let points = assembly["plateContours"][1]["points"].as_array().unwrap();
    let min_x = points
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    assert!((min_x - 9.8).abs() < 0.002, "{result}");
    let plate = assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["body"]["id"] == "plate")
        .unwrap();
    assert_eq!(plate["contours"], assembly["plateContours"]);
    assert!(
        assembly["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "mechanical:fit-review")
    );
}

#[test]
fn negative_opening_allowance_shrinks_without_losing_nominal_dimensions() {
    let mut doc = document();
    let mut config = configuration();
    config["openingAllowance"] = json!(-0.2);
    doc["mechanical"] = config;
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"resolve","document":doc,"contours":contours()}),
    );
    let points = result["assembly"]["plateContours"][1]["points"]
        .as_array()
        .unwrap();
    let min_x = points
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    assert!((min_x - 10.2).abs() < 0.002, "{result}");
}

#[test]
fn holes_outside_the_material_are_diagnostics_not_silently_dropped() {
    let mut doc = document();
    doc["mechanical"] = configuration();
    let mut rings = contours();
    rings.as_array_mut().unwrap().push(json!({"hole":true,"points":[{"x":90,"y":90},{"x":95,"y":90},{"x":95,"y":95},{"x":90,"y":95}]}));
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"resolve","document":doc,"contours":rings}),
    );
    assert!(
        result["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "mechanical:process:outside:2" && f["severity"] == "error"),
        "{result}"
    );
}

#[test]
fn manual_mounts_must_leave_material_for_their_full_boss() {
    let mut doc = document();
    let mut config = configuration();
    config["mounts"] =
        json!([{"id":"edge","at":{"x":-1,"y":5},"kind":"hole","holeDiameter":3,"bossDiameter":6}]);
    doc["mechanical"] = config;
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"resolve","document":doc,"contours":contours()}),
    );
    assert!(
        result["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |f| f["id"] == "mechanical:process:mount-material:edge" && f["severity"] == "error"
            ),
        "{result}"
    );
}

#[test]
fn hardware_and_critical_fit_specs_require_real_targets_and_valid_dimensions() {
    let mut doc = document();
    let mut config = configuration();
    config["hardware"] = json!([{"id":"screw","partId":"plate","featureId":"missing","designation":"M2 screw","thread":"M2 x 0.4","length":6,"quantity":2}]);
    config["criticalFits"] = json!([{"id":"fit","partId":"missing","label":"Retention","from":{"x":0,"y":0},"to":{"x":0,"y":0},"tolerance":"+0.05/-0.00 mm"}]);
    doc["mechanical"] = config;
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"specs","document":doc,"contours":contours()}),
    );
    let findings = resolved["assembly"]["diagnostics"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["id"] == "mechanical:hardware:screw")
    );
    assert!(
        findings
            .iter()
            .any(|f| f["id"] == "mechanical:critical-fit:fit")
    );
}

#[test]
fn hardware_and_critical_fit_specs_persist_and_match_generated_parts() {
    let mut doc = document();
    let mut config = configuration();
    config["mounts"] =
        json!([{"id":"m1","at":{"x":40,"y":20},"kind":"hole","holeDiameter":2.2,"bossDiameter":6}]);
    config["hardware"] = json!([{"id":"screw","partId":"plate","featureId":"m1","designation":"Socket screw","thread":"M2 x 0.4","length":6,"quantity":1,"tolerance":"6g"}]);
    config["criticalFits"] = json!([{"id":"fit","partId":"plate","label":"Opening","from":{"x":10,"y":10},"to":{"x":20,"y":10},"tolerance":"+0.05/-0.00 mm"}]);
    doc["mechanical"] = config.clone();
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"kind":"open","id":"open","document":doc}),
    );
    assert_eq!(
        serde_json::from_value::<boardstudio_core::model::MechanicalConfiguration>(
            opened["document"]["mechanical"].clone()
        )
        .unwrap(),
        serde_json::from_value::<boardstudio_core::model::MechanicalConfiguration>(config).unwrap()
    );
    let resolved = request(
        &mut engine,
        json!({"kind":"resolve-mechanical","id":"specs","document":opened["document"],"contours":contours()}),
    );
    assert!(
        !resolved["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                let id = finding["id"].as_str().unwrap();
                id.starts_with("mechanical:hardware:") || id.starts_with("mechanical:critical-fit:")
            })
    );
}

#[test]
fn mechanical_diagnostics_keep_validation_order_before_outline_checks() {
    let mut doc = document();
    let mut config = configuration();
    config["plateThickness"] = json!(0.0);
    doc["mechanical"] = config;
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"order","document":doc,"contours":[]}),
    );
    let findings = result["assembly"]["diagnostics"].as_array().unwrap();
    assert_eq!(
        findings
            .iter()
            .map(|finding| finding["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["mechanical:frame-outline", "mechanical:dimensions"]
    );
}

#[test]
fn battery_stack_and_body_preserve_declared_layer_order() {
    let mut doc = document();
    let mut config = configuration();
    config["battery"] = json!({
        "size":{"x":20,"y":10,"z":3},
        "at":{"x":30,"y":20},
        "cableExit":{"x":30,"y":20},
        "cableWidth":2
    });
    doc["mechanical"] = config;
    let result = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-mechanical","id":"battery","document":doc,"contours":contours()}),
    );
    let assembly = &result["assembly"];
    let stack = assembly["stack"].as_array().unwrap();
    let ids: Vec<_> = stack
        .iter()
        .map(|layer| layer["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "plate",
            "plate-foam",
            "pcb",
            "bottom-foam",
            "battery",
            "bottom"
        ]
    );
    assert_eq!(
        assembly["case"]["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|body| body["body"]["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["plate", "bottom", "plate-foam", "bottom-foam"]
    );
}
