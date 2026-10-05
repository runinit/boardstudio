use boardstudio_core::CoreEngine;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn splitter() -> Value {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../catalogue/modules/imported-modules.json")).unwrap();
    catalogue["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["row"] == "vik-splitter")
        .unwrap()["definition"]
        .clone()
}

#[test]
fn splitter_service_clearance_is_bounded_and_preserves_only_holes_that_survive_offset() {
    let definition = splitter();
    assert_eq!(definition["board"]["holes"].as_array().unwrap().len(), 38);

    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "module-clearance",
        "Module clearance",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"board","name":"Host","partIds":[],"netIds":[],"outlineIds":["boundary"],"thickness":1.6}]);
    document["outline"] = json!([{"kind":"rect","id":"boundary","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    document["moduleDefinitions"] = json!([definition.clone()]);
    document["modules"] = json!([{"id":"mounted","definitionId":definition["id"],"hostBoardId":"board","hostFace":"back","facingFace":"back","at":{"x":27,"y":0},"rotation":0,"gap":3,"attachment":"board","detached":false,"serviceClearance":1}]);

    let start = Instant::now();
    let resolved = request(
        &mut CoreEngine::new(),
        json!({"kind":"resolve-modules","id":"clearance","document":document,"boardId":"board"}),
    );
    let elapsed = start.elapsed();
    assert_eq!(resolved["kind"], "modules-resolved", "{resolved}");
    assert!(
        elapsed < Duration::from_secs(3),
        "one source splitter clearance resolution took {elapsed:?}"
    );

    // The real source has 2.2 mm mount holes, 1.1 mm pad drills, and 0.3 mm vias.
    // At 1 mm positive clearance, the latter holes vanish while the mount holes
    // retain a 0.2 mm opening. Verify that fact through public PrepareCase.
    let source_holes = definition["board"]["holes"].as_array().unwrap();
    let mut contours = vec![definition["board"]["contours"][0].clone()];
    for index in [0, 1, 16] {
        contours.push(json!({"hole":true,"points":source_holes[index]}));
    }
    let prepared = request(
        &mut CoreEngine::new(),
        json!({"kind":"prepare-case","id":"clearance-profile","ir":{
            "revision":0,"bodies":[{"revision":0,"contours":contours,
                "body":{"id":"module-clearance-profile","name":"Module clearance profile","boardId":"board","kind":"plate","thickness":1,"clearance":1}}
            ]
        }}),
    );
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
    let holes = prepared["ir"]["bodies"][0]["regions"][0]["holes"]
        .as_array()
        .unwrap();
    assert_eq!(
        holes.len(),
        1,
        "the source mount hole survives; the smaller pad drill and via collapse"
    );
    for hole in holes {
        let points = hole.as_array().unwrap();
        let xs = points
            .iter()
            .map(|point| point["x"].as_f64().unwrap())
            .collect::<Vec<_>>();
        let ys = points
            .iter()
            .map(|point| point["y"].as_f64().unwrap())
            .collect::<Vec<_>>();
        let width = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - xs.iter().copied().fold(f64::INFINITY, f64::min);
        let height = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - ys.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(
            width > 0.1 && width < 0.3 && height > 0.1 && height < 0.3,
            "surviving source hole bounds {width} × {height}"
        );
    }
}
