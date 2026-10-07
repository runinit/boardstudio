use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

const MODULE_LIBRARY: &str = include_str!("../../catalogue/modules/imported-modules.json");

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn surface_anchor_in_host(surface: &Value, footprint: &Value) -> (f64, f64) {
    let local = &surface["points"][0];
    let pose = &footprint["pose"];
    let angle = pose["rotation"].as_f64().unwrap().to_radians();
    let (sin, cos) = angle.sin_cos();
    let x = local["x"].as_f64().unwrap();
    let y = local["y"].as_f64().unwrap();
    (
        pose["at"]["x"].as_f64().unwrap() + x * cos - y * sin,
        pose["at"]["y"].as_f64().unwrap() + x * sin + y * cos,
    )
}

#[test]
fn mounted_module_resolves_source_footprints_in_host_frame_without_host_ownership() {
    let library: Value = serde_json::from_str(MODULE_LIBRARY).unwrap();
    let definition = library["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["row"] == "haptic-drv2605l")
        .unwrap()["definition"]
        .clone();

    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "module-footprints",
        "Module footprint preview",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    document["moduleDefinitions"] = json!([definition]);
    document["modules"] = json!([
        {
            "id":"mounted/haptic",
            "definitionId":definition["id"],
            "hostBoardId":"host",
            "hostFace":"front",
            "facingFace":"front",
            "at":{"x":15,"y":7},
            "rotation":90,
            "gap":3,
            "attachment":"board",
            "detached":false,
            "serviceClearance":0
        },
        {
            "id":"mounted/haptic-unflipped",
            "definitionId":definition["id"],
            "hostBoardId":"host",
            "hostFace":"back",
            "facingFace":"front",
            "at":{"x":45,"y":7},
            "rotation":90,
            "gap":3,
            "attachment":"board",
            "detached":false,
            "serviceClearance":0
        }
    ]);

    let mut engine = CoreEngine::new();
    let resolved = request(
        &mut engine,
        json!({"id":"resolve","kind":"resolve-modules","document":document.clone(),"boardId":"host"}),
    );
    assert_eq!(resolved["kind"], "modules-resolved", "{resolved}");
    let modules = resolved["result"]["modules"].as_array().unwrap();
    let module = modules
        .iter()
        .find(|module| module["id"] == "mounted/haptic")
        .unwrap();
    assert_eq!(module["id"], "mounted/haptic");
    let footprints = module["footprints"]
        .as_array()
        .expect("resolved preview footprints");
    assert_eq!(footprints.len(), 14);

    let resistor = footprints
        .iter()
        .find(|footprint| footprint["reference"] == "R1")
        .unwrap();
    assert_eq!(resistor["id"], "mounted/haptic/footprint/R1");
    assert_eq!(resistor["sourcePartId"], "R1");
    assert_eq!(
        resistor["definitionId"],
        "vik:haptic-drv2605l:pcb-haptic-drv2605l-haptic-drv2605l/component/7"
    );
    assert_eq!(resistor["pose"]["at"]["x"], 17.5);
    assert_eq!(resistor["pose"]["at"]["y"], -2.0);
    assert_eq!(resistor["pose"]["rotation"], -90.0);
    assert_eq!(resistor["side"], "front");
    assert_eq!(module["flipped"], true);
    assert!(
        resistor["courtyard"]
            .as_array()
            .is_some_and(|points| !points.is_empty())
    );
    let surfaces = resistor["surfaces"].as_array().unwrap();
    assert!(
        surfaces
            .iter()
            .any(|surface| surface["layer"] == "F.SilkS" && surface["text"] == "R1")
    );
    assert!(
        surfaces
            .iter()
            .any(|surface| surface["layer"] == "F.Fab" && surface["text"] == "4.7k")
    );
    assert!(surfaces.iter().all(|surface| {
        matches!(
            surface["layer"].as_str(),
            Some("F.SilkS" | "B.SilkS" | "F.Fab" | "B.Fab")
        )
    }));
    let pad = resistor["pads"].as_array().unwrap().first().unwrap();
    assert_eq!(pad["at"]["x"], -1.4625);
    assert!(
        pad.get("netId").is_none(),
        "module-local nets must not be presented as host nets"
    );

    let unflipped = modules
        .iter()
        .find(|module| module["id"] == "mounted/haptic-unflipped")
        .unwrap();
    let unflipped_resistor = unflipped["footprints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|footprint| footprint["reference"] == "R1")
        .unwrap();
    assert_eq!(unflipped_resistor["pose"]["at"]["x"], 47.5);
    assert_eq!(unflipped_resistor["pose"]["at"]["y"], 16.0);
    assert_eq!(unflipped_resistor["pose"]["rotation"], 270.0);
    assert_eq!(unflipped_resistor["side"], "back");
    assert_eq!(unflipped["flipped"], false);
    let unflipped_reference = unflipped_resistor["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|surface| surface["text"] == "R1")
        .unwrap();
    assert_eq!(unflipped_reference["layer"], "B.SilkS");
    let flipped_reference = surfaces
        .iter()
        .find(|surface| surface["text"] == "R1")
        .unwrap();
    let flipped_anchor = surface_anchor_in_host(flipped_reference, resistor);
    let unflipped_anchor = surface_anchor_in_host(unflipped_reference, unflipped_resistor);
    assert!((flipped_anchor.0 - 15.68).abs() < 1e-6);
    assert!((flipped_anchor.1 + 2.0).abs() < 1e-6);
    assert!((unflipped_anchor.0 - 45.68).abs() < 1e-6);
    assert!((unflipped_anchor.1 - 16.0).abs() < 1e-6);

    assert!(document["parts"].as_array().unwrap().is_empty());
    assert!(document["nets"].as_array().unwrap().is_empty());
}
