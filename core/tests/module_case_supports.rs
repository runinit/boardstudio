use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn document(supports: Value, above: bool) -> (Value, Value) {
    let mut doc = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "modules", "Modules",
    ))
    .unwrap();
    doc["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    doc["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    doc["moduleDefinitions"] = json!([{
        "id":"reference","name":"Mounted module","family":"test","variant":"support",
        "source":{"repository":"https://example.invalid/module","revision":"test","path":"pcb","license":"CC0-1.0"},
        "board":{"thickness":1.6,"contours":[{"hole":false,"points":[{"x":-12.5,"y":-12.5},{"x":12.5,"y":-12.5},{"x":12.5,"y":12.5},{"x":-12.5,"y":12.5}]}]},
        "mounts":[{"sourceId":"mh1","at":{"x":0,"y":0},"diameter":2.2}],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[]},"constituents":[]
    }]);
    doc["modules"] = json!([{
        "id":"module-1","definitionId":"reference","hostBoardId":"host","hostFace":if above {"front"} else {"back"},"facingFace":"front",
        "at":{"x":0,"y":0},"rotation":0,"gap":3,"attachment":"case","mountSupports":supports
    }]);
    doc["mechanical"] = json!({"boardId":"host","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
        "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
        "plateToPcb":if above {0.5} else {3.5},"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
    let contours = json!([{"hole":false,"points":[{"x":-40,"y":-40},{"x":40,"y":-40},{"x":40,"y":40},{"x":-40,"y":40}]}]);
    (doc, contours)
}

#[test]
fn explicit_module_support_becomes_a_preparable_case_boss() {
    let (doc, contours) = document(
        json!([{"mountId":"mh1","outerDiameter":6,"holeDiameter":2.2,"z":-1.8,"height":1}]),
        false,
    );
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], false,
        "{}",
        assembly["diagnostics"]
    );
    let bottom = assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "bottom")
        .unwrap();
    let boss = bottom["body"]["mounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|mount| mount["id"] == "module:module-1:mh1")
        .unwrap();
    assert_eq!(boss["kind"], "boss");
    assert!(boss["at"]["x"].as_f64().unwrap().abs() < 1e-8);
    assert!(boss["at"]["y"].as_f64().unwrap().abs() < 1e-8);
    assert!((boss["holeDiameter"].as_f64().unwrap() - 2.2).abs() < 1e-8);
    assert!((boss["bossDiameter"].as_f64().unwrap() - 6.0).abs() < 1e-8);
    assert!((boss["height"].as_f64().unwrap() - 1.0).abs() < 1e-8);
    assert!((bottom["body"]["z"].as_f64().unwrap() + 9.2).abs() < 1e-8);
    let prepared = request(
        &mut CoreEngine::new(),
        json!({"id":"prepare","kind":"prepare-case","ir":assembly["case"]}),
    );
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
    let prepared_boss = prepared["ir"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "bottom")
        .unwrap()["regions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|region| region["mounts"].as_array().unwrap())
        .find(|mount| mount["id"] == "module:module-1:mh1")
        .unwrap();
    assert_eq!(prepared_boss["kind"], "boss");
}

#[test]
fn above_module_support_bridges_to_case_plate_and_prepares() {
    let (doc, contours) = document(
        json!([{"mountId":"mh1","outerDiameter":6,"holeDiameter":2.2,"z":0.8,"height":1}]),
        true,
    );
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let assembly = &resolved["assembly"];
    assert_eq!(
        assembly["generationBlocked"], false,
        "{}",
        assembly["diagnostics"]
    );
    let plate = assembly["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|body| body["body"]["id"] == "plate")
        .unwrap();
    let boss = plate["body"]["mounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|mount| mount["id"] == "module:module-1:mh1")
        .unwrap();
    assert!((boss["height"].as_f64().unwrap() - 1.0).abs() < 1e-8);
    assert!(
        (plate["body"]["z"].as_f64().unwrap() + plate["body"]["thickness"].as_f64().unwrap() - 2.0)
            .abs()
            < 1e-8
    );
    let prepared = request(
        &mut CoreEngine::new(),
        json!({"id":"prepare","kind":"prepare-case","ir":assembly["case"]}),
    );
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
}

#[test]
fn case_attached_module_with_mounts_cannot_export_without_designer_supports() {
    let (doc, contours) = document(json!([]), false);
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let findings = resolved["assembly"]["diagnostics"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|finding| finding["id"] == "module/module-1/mount-support/mh1"),
        "{findings:?}"
    );
    assert_eq!(resolved["assembly"]["generationBlocked"], true);
    assert!(
        resolved["assembly"]["findingMarkers"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |marker| marker["findingId"] == "module/module-1/mount-support/mh1"
                    && marker["contours"][0]["points"].as_array().unwrap().len() == 128
            )
    );
}

#[test]
fn annular_support_must_have_uninterrupted_case_material_at_its_anchor_face() {
    let (doc, mut contours) = document(
        json!([{"mountId":"mh1","outerDiameter":6,"holeDiameter":2.2,"z":-1.8,"height":1}]),
        false,
    );
    contours
        .as_array_mut()
        .unwrap()
        .push(json!({"hole":true,"points":[
            {"x":1.8,"y":-0.2},{"x":2.3,"y":-0.2},{"x":2.3,"y":0.2},{"x":1.8,"y":0.2}
        ]}));
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let findings = resolved["assembly"]["diagnostics"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|finding| finding["id"] == "module/module-1/mount-support/mh1"),
        "{findings:?}"
    );
    assert_eq!(resolved["assembly"]["generationBlocked"], true);
}
