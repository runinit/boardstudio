use super::*;

#[test]
fn source_backed_module_board_is_one_complete_case_body() {
    let imported = import(FRONT_SLOT.into(), "fixture:two-islands");
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
    let valid = import(FRONT_SLOT.into(), "fixture:valid")["result"].clone();
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
    for (raw, expected_holes, expected_drills) in [(FRONT_SLOT, 1, 0), (VIK_SPLITTER, 38, 38)] {
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
            "CAD kernel still needs all exact round drill definitions"
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
