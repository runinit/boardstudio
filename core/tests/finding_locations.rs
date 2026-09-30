use boardstudio_core::{CoreEngine, model::*};
#[test]
fn fitted_corner_warning_has_local_geometry_through_public_scene() {
    let mut doc: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/reviung41-outline-original.json")).unwrap();
    for feature in &mut doc.outline {
        if let OutlineFeature::PartEnvelope { settings, .. } = feature {
            settings.corners = CornerStyle::Fillet;
            settings.size = 2.0;
        }
    }
    let reply = CoreEngine::new().handle(CoreRequest::Open {
        id: "located".into(),
        document: doc,
    });
    let CoreReply::Scene { scene, .. } = reply else {
        panic!("{reply:?}")
    };
    let warning = scene
        .findings
        .iter()
        .find(|f| f.id.ends_with("outline:corners:fitted"))
        .expect("fixture has fitted corners");
    let marker = scene
        .finding_markers
        .iter()
        .find(|m| m.finding_id == warning.id)
        .expect("corner warning must carry its exact affected locations");
    assert!(!marker.contours.is_empty());
    assert!(marker.contours.iter().all(
        |c| c.points.len() >= 3 && c.points.iter().all(|p| p.x.is_finite() && p.y.is_finite())
    ));
    // Local markers cannot substitute the complete board perimeter.
    assert!(marker.contours.iter().all(|c| {
        let min = c.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max = c
            .points
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        max - min <= 6.0
    }));
}
#[test]
fn keycap_clearance_warning_locates_both_affected_inputs() {
    let mut value = serde_json::to_value(ProjectDoc::empty("caps", "Caps")).unwrap();
    value["definitions"] = serde_json::json!([{"id":"mx","name":"MX","kind":"switch","keycap":{"x":18.2,"y":18.2},"courtyard":[],"pads":[]}]);
    value["parts"] = serde_json::json!([{"id":"a","reference":"SW1","definitionId":"mx","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},{"id":"b","reference":"SW2","definitionId":"mx","pose":{"at":{"x":18.3,"y":0},"rotation":0},"side":"front"}]);
    value["boards"] = serde_json::json!([{"id":"board","name":"Board","partIds":["a","b"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    value["keycaps"] = serde_json::json!({"matrices":{},"keys":{"a":{"profile":"dsa","mount":"mx"},"b":{"profile":"dsa","mount":"mx"}},"boards":{}});
    let doc = serde_json::from_value(value).unwrap();
    let CoreReply::Scene { scene, .. } = CoreEngine::new().handle(CoreRequest::Open {
        id: "caps".into(),
        document: doc,
    }) else {
        panic!("open")
    };
    let warning = scene
        .findings
        .iter()
        .find(|finding| finding.message.contains("keycap clearance"))
        .expect("fixture clearance warning");
    let marker = scene
        .finding_markers
        .iter()
        .find(|marker| marker.finding_id == warning.id)
        .expect("clearance warning must identify the affected keycaps");
    assert_eq!(marker.contours.len(), 2);
}
