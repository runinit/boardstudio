use boardstudio_core::artifact;
use serde_json::{Value, json};

fn catalogue_definition(id: &str) -> Value {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../app/src/parts/imported-parts.json")).unwrap();
    catalogue["parts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["definition"]["id"] == id)
        .unwrap()["definition"]
        .clone()
}

#[test]
fn unqualified_thqwgd001_cannot_be_exported_as_a_fabrication_ready_footprint() {
    let definition = catalogue_definition("thqwgd001:c-4pin-reversible");
    let mut document = boardstudio_core::model::ProjectDoc::empty("qualification", "Qualification");
    document
        .definitions
        .push(serde_json::from_value(definition).unwrap());
    let reply: Value = serde_json::from_str(&artifact::request(&json!({
        "id":"unqualified", "kind":"prepare-export", "request":{
            "snapshotToken":"committed", "expectedRevision":0,
            "document":document,
            "target":{"kind":"standalone-footprints","definitionIds":["thqwgd001:c-4pin-reversible"]},
            "contours":[],"modelPaths":{}
        }
    }).to_string())).unwrap();
    assert_eq!(
        reply["kind"], "error",
        "Published files do not establish reversible hardware qualification"
    );
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Physical encoder part identity"),
        "{reply}"
    );
}

#[test]
fn routed_and_drilled_helpers_keep_distinct_public_export_gates() {
    let route = catalogue_definition("thqwgd001:slitc");
    let drill = catalogue_definition("thqwgd001:slitc-drill");
    let gates = |definition: &Value| {
        definition["hardwareProfile"]["gates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|gate| gate["code"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    let has_gate =
        |definition: &Value, code: &str| gates(definition).iter().any(|gate| gate == code);
    assert!(has_gate(
        &route,
        "confirm-helper-placement-and-closed-contour-machining"
    ));
    assert!(!has_gate(
        &drill,
        "confirm-helper-placement-and-closed-contour-machining"
    ));
    assert!(has_gate(&drill, "confirm-helper-placement-assumption"));
    assert!(!has_gate(&route, "confirm-helper-placement-assumption"));
    assert!(has_gate(
        &route,
        "fabricator-minimum-slot-radius-and-web-width"
    ));
    assert!(has_gate(
        &drill,
        "fabricator-minimum-slot-radius-and-web-width"
    ));
    assert_eq!(drill["pads"].as_array().unwrap().len(), 8);
    assert!(
        !drill["kicadSource"]["source"]
            .as_str()
            .unwrap()
            .contains("Edge.Cuts")
    );

    for (id, definition, expected) in [
        ("thqwgd001:slitc", route, "routed source geometry is open"),
        (
            "thqwgd001:slitc-drill",
            drill,
            "publishes no companion transform",
        ),
    ] {
        let mut document = boardstudio_core::model::ProjectDoc::empty("helper-gate", "Helper gate");
        document
            .definitions
            .push(serde_json::from_value(definition).unwrap());
        let reply: Value = serde_json::from_str(&artifact::request(
            &json!({
                "id":id, "kind":"prepare-export", "request":{
                    "snapshotToken":"helper-gate", "expectedRevision":0,
                    "document":document,
                    "target":{"kind":"standalone-footprints","definitionIds":[id]},
                    "contours":[], "modelPaths":{}
                }
            })
            .to_string(),
        ))
        .unwrap();
        assert_eq!(reply["kind"], "error", "{id}: {reply}");
        assert!(
            reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{id}: {reply}"
        );
    }
}

fn placed_project(pitch: f64) -> boardstudio_core::model::ProjectDoc {
    let mut value =
        serde_json::to_value(boardstudio_core::model::ProjectDoc::empty("fit", "Fit")).unwrap();
    value["definitions"] = json!([catalogue_definition("thqwgd001:c-4pin-reversible")]);
    value["parts"] = json!([
        {"id":"a","reference":"ENC1","definitionId":"thqwgd001:c-4pin-reversible","side":"front","pose":{"at":{"x":0,"y":0},"rotation":0}},
        {"id":"b","reference":"ENC2","definitionId":"thqwgd001:c-4pin-reversible","side":"front","pose":{"at":{"x":pitch,"y":0},"rotation":0}}
    ]);
    value["boards"] = json!([{"id":"board","name":"Board","partIds":["a","b"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    serde_json::from_value(value).unwrap()
}

fn single_project(
    id: &str,
    side: &str,
    at: (f64, f64),
    rotation: f64,
) -> boardstudio_core::model::ProjectDoc {
    let mut value =
        serde_json::to_value(boardstudio_core::model::ProjectDoc::empty("datum", "Datum")).unwrap();
    value["definitions"] = json!([catalogue_definition(id)]);
    value["parts"] = json!([{
        "id":"encoder", "reference":"ENC1", "definitionId":id, "side":side,
        "pose":{"at":{"x":at.0,"y":at.1},"rotation":rotation}
    }]);
    value["boards"] = json!([{
        "id":"board", "name":"Board", "partIds":["encoder"], "outlineIds":[], "netIds":[], "thickness":1.6
    }]);
    serde_json::from_value(value).unwrap()
}

fn hardware_fit_findings(document: boardstudio_core::model::ProjectDoc) -> Vec<String> {
    use boardstudio_core::{CoreEngine, model::*};
    let reply = CoreEngine::new().handle(CoreRequest::Open {
        id: "fit-geometry".into(),
        document,
    });
    let CoreReply::Scene { scene, .. } = reply else {
        panic!("{reply:?}")
    };
    scene
        .findings
        .iter()
        .filter(|finding| finding.id.starts_with("hardware-fit/"))
        .map(|finding| finding.id.clone())
        .collect()
}

fn exported_preview(id: &str, side: &str) -> Value {
    use boardstudio_core::model::*;
    let mut document = single_project(id, side, (11.0, 13.0), 37.0);
    for definition in &mut document.definitions {
        definition.hardware_profile.as_mut().unwrap().gates.clear();
    }
    let model_id = document.definitions[0].models.as_ref().unwrap()[0]
        .asset_id
        .clone();
    let plan = artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "source-datum".into(),
        expected_revision: 0,
        document,
        target: ExportTarget::Board {
            board_id: "board".into(),
        },
        contours: vec![Contour {
            hole: false,
            points: vec![
                Vec2 { x: -30., y: -30. },
                Vec2 { x: 40., y: -30. },
                Vec2 { x: 40., y: 45. },
                Vec2 { x: -30., y: 45. },
            ],
        }],
        model_paths: std::collections::BTreeMap::from([(model_id, "models/thq.stp".into())]),
    })
    .unwrap();
    let output = artifact::kicad::finish_export(FinishExportRequest {
        plan,
        results: vec![],
    })
    .unwrap();
    let board = &output
        .files
        .iter()
        .find(|file| file.filename.ends_with(".kicad_pcb"))
        .unwrap()
        .content;
    serde_json::from_str(&artifact::request(
        &json!({"id":"preview-thq","kind":"preview-board","revision":1,"source":board}).to_string(),
    ))
    .unwrap()
}

fn paired_project(
    side: &str,
    rotation: f64,
    second_at: (f64, f64),
) -> boardstudio_core::model::ProjectDoc {
    let mut value = serde_json::to_value(placed_project(25.0)).unwrap();
    for (index, at) in [(0, (0.0, 0.0)), (1, second_at)] {
        value["parts"][index]["side"] = json!(side);
        value["parts"][index]["pose"] = json!({
            "at":{"x":at.0,"y":at.1},"rotation":rotation
        });
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn nominal_model_bounds_locate_a_collision_even_at_mx_pitch() {
    use boardstudio_core::{CoreEngine, model::*};
    let reply = CoreEngine::new().handle(CoreRequest::Open {
        id: "fit".into(),
        document: placed_project(19.05),
    });
    let CoreReply::Scene { scene, .. } = reply else {
        panic!("{reply:?}")
    };
    let finding = scene
        .findings
        .iter()
        .find(|f| f.id.starts_with("hardware-fit/"))
        .expect("C4 assembly width is 19.419158001 mm, exceeding nominal MX pitch");
    assert_eq!(finding.target_ids, vec!["a", "b"]);
    let marker = scene
        .finding_markers
        .iter()
        .find(|m| m.finding_id == finding.id)
        .expect("Finding points to the overlap rather than whole components");
    assert_eq!(marker.board_id, "board");
    let xs: Vec<_> = marker
        .contours
        .iter()
        .flat_map(|c| c.points.iter().map(|p| p.x))
        .collect();
    let width = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        - xs.iter().copied().fold(f64::INFINITY, f64::min);
    assert!((width - 0.369158001).abs() < 0.0001);
    let reply = CoreEngine::new().handle(CoreRequest::Open {
        id: "clear".into(),
        document: placed_project(25.0),
    });
    let CoreReply::Scene { scene, .. } = reply else {
        panic!("{reply:?}")
    };
    assert!(
        !scene
            .findings
            .iter()
            .any(|f| f.id.starts_with("hardware-fit/"))
    );
}

#[test]
fn exported_front_and_back_pad_holes_and_model_datums_match_source_placement() {
    for id in [
        "thqwgd001:rotation-reversible",
        "thqwgd001:c-2pin-reversible",
        "thqwgd001:c-4pin-reversible",
    ] {
        let definition = catalogue_definition(id);
        for side in ["front", "back"] {
            let preview = exported_preview(id, side);
            assert_eq!(preview["kind"], "preview-board", "{id} {side}: {preview}");
            let holes = preview["result"]["holes"].as_array().unwrap();
            let centers: Vec<(f64, f64)> = holes
                .iter()
                .map(|hole| {
                    let points = hole.as_array().unwrap();
                    let center = points.iter().fold((0.0, 0.0), |sum, point| {
                        (
                            sum.0 + point["x"].as_f64().unwrap(),
                            sum.1 + point["y"].as_f64().unwrap(),
                        )
                    });
                    (
                        center.0 / points.len() as f64,
                        center.1 / points.len() as f64,
                    )
                })
                .collect();
            let yaw = 37.0_f64.to_radians();
            for pad in definition["pads"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|pad| pad["drill"].is_number())
            {
                let source_x = pad["at"]["x"].as_f64().unwrap();
                let source_y = pad["at"]["y"].as_f64().unwrap();
                // Canonical back placement reflects local X before board yaw.
                let local_x = if side == "back" { -source_x } else { source_x };
                let expected = (
                    11.0 + local_x * yaw.cos() - source_y * yaw.sin(),
                    13.0 + local_x * yaw.sin() + source_y * yaw.cos(),
                );
                let closest = centers
                    .iter()
                    .map(|point| (point.0 - expected.0).hypot(point.1 - expected.1))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    closest < 1e-5,
                    "{id} {side} hole {} at ({}, {}) differs by {closest}",
                    pad["id"],
                    expected.0,
                    expected.1
                );
            }
            let models = preview["result"]["models"].as_array().unwrap();
            assert_eq!(models.len(), 1, "{id} {side}");
            assert_eq!(models[0]["side"], side);
            assert_eq!(models[0]["pose"]["at"]["x"], 11.0);
            assert_eq!(models[0]["pose"]["at"]["y"], 13.0);
            let expected_yaw = if side == "back" { -143.0 } else { 37.0 };
            assert_eq!(models[0]["pose"]["rotation"], expected_yaw);
            assert_eq!(models[0]["rotation"]["z"], 0.0);
            assert_eq!(models[0]["offset"]["z"], 0.0);
        }
    }
}

#[test]
fn c4_model_fit_tracks_mx_choc_rotation_and_back_mounting() {
    let fit_at = |project| !hardware_fit_findings(project).is_empty();

    // Source bounds are 19.419158001 mm wide: both nominal switch pitches
    // overlap when aligned with the encoder's long axis.
    assert!(fit_at(paired_project("front", 0.0, (19.05, 0.0))));
    assert!(fit_at(paired_project("front", 0.0, (18.0, 0.0))));

    // Rotating the two assemblies swaps their occupied extents. They clear
    // horizontally at MX pitch, but still overlap when stacked vertically.
    assert!(!fit_at(paired_project("front", 90.0, (19.05, 0.0))));
    assert!(fit_at(paired_project("front", 90.0, (0.0, 19.05))));

    // The reversible back-side transform preserves the same nominal bounds.
    assert!(fit_at(paired_project("back", 0.0, (19.05, 0.0))));
}

#[test]
fn switched_variants_preserve_published_repeated_contacts_through_electrical_planning() {
    for id in ["thqwgd001:c-2pin-reversible", "thqwgd001:c-4pin-reversible"] {
        let mut document = placed_project(25.0);
        document.definitions = vec![serde_json::from_value(catalogue_definition(id)).unwrap()];
        document.parts.truncate(1);
        document.parts[0].definition_id = id.into();
        document.boards[0].part_ids = vec!["a".into()];
        let definition = &document.definitions[0];
        for number in ["A", "B", "C"] {
            let pads: Vec<_> = definition
                .pads
                .iter()
                .filter(|pad| pad.number == number)
                .collect();
            assert_eq!(pads.len(), 2);
            assert_ne!(pads[0].id, pads[1].id);
            let xs: Vec<_> = pads.iter().map(|p| p.at.x).collect();
            assert!(xs.contains(&-7.8) && xs.contains(&-4.9));
        }
        let plan = boardstudio_core::electrical::resolve(
            serde_json::from_value(json!({"document":document,"boardId":"board","mode":"matrix"}))
                .unwrap(),
        );
        let encoder = plan.peripherals.iter().find(|p| p.part_id == "a").unwrap();
        assert_eq!(
            encoder.fixed_terminals,
            vec![("C".into(), "GND".into()), ("2".into(), "GND".into())]
        );
        assert_eq!(
            encoder
                .gpio_terminals
                .iter()
                .map(|p| p.0.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "1"]
        );
        assert!(
            encoder.rotary.as_ref().unwrap().steps.is_none(),
            "No supplier pulse count was established"
        );
    }
}

#[test]
fn reviewed_geometry_fixture_exports_portable_models_and_selected_slot_helpers() {
    use boardstudio_core::model::*;
    for helper in ["thqwgd001:slitc", "thqwgd001:slitc-drill"] {
        let mut document = placed_project(25.0);
        document.parts.truncate(1);
        document.boards[0].part_ids = vec!["a".into(), "mount".into()];
        document
            .definitions
            .push(serde_json::from_value(catalogue_definition(helper)).unwrap());
        // A geometry-only fixture explicitly removes qualification blockers;
        // this does not certify these library assets for physical manufacture.
        for definition in &mut document.definitions {
            definition.hardware_profile.as_mut().unwrap().gates.clear();
        }
        document.parts.push(serde_json::from_value(json!({"id":"mount","reference":"CUT1","definitionId":helper,"side":"front","pose":{"at":{"x":12,"y":-8},"rotation":90}})).unwrap());
        let asset_id = document.definitions[0].models.as_ref().unwrap()[0]
            .asset_id
            .clone();
        let plan = artifact::kicad::prepare_export(PrepareExportRequest {
            snapshot_token: "geometry-only".into(),
            expected_revision: 0,
            document,
            target: ExportTarget::Board {
                board_id: "board".into(),
            },
            contours: vec![Contour {
                hole: false,
                points: vec![
                    Vec2 { x: -25., y: -25. },
                    Vec2 { x: 25., y: -25. },
                    Vec2 { x: 25., y: 25. },
                    Vec2 { x: -25., y: 25. },
                ],
            }],
            model_paths: std::collections::BTreeMap::from([(
                asset_id,
                "models/encoder.stp".into(),
            )]),
        })
        .unwrap();
        let output = artifact::kicad::finish_export(FinishExportRequest {
            plan,
            results: vec![],
        })
        .unwrap();
        let board = &output
            .files
            .iter()
            .find(|f| f.filename.ends_with(".kicad_pcb"))
            .unwrap()
            .content;
        assert!(board.contains("${KIPRJMOD}/models/encoder.stp"));
        assert!(!board.contains("/Users/hayashi/"));
        assert!(board.contains("B.Mask") && board.contains("F.Mask"));
        if helper.ends_with("-drill") {
            assert!(board.contains("np_thru_hole oval"));
        } else {
            assert!(board.contains("fp_line") && board.contains("Edge.Cuts"));
        }
        let preview: Value = serde_json::from_str(&artifact::request(
            &json!({"id":"preview","kind":"preview-board","revision":0,"source":board}).to_string(),
        ))
        .unwrap();
        if helper.ends_with("-drill") {
            assert_eq!(preview["kind"], "preview-board", "{preview}");
            assert_eq!(preview["result"]["models"].as_array().unwrap().len(), 1);
            assert_eq!(preview["result"]["models"][0]["rotation"]["z"], 0.0);
            let holes = preview["result"]["holes"].as_array().unwrap();
            for pad in catalogue_definition(helper)["pads"].as_array().unwrap() {
                let x = pad["at"]["x"].as_f64().unwrap();
                let y = pad["at"]["y"].as_f64().unwrap();
                let expected = (12.0 - y, -8.0 + x);
                let closest = holes
                    .iter()
                    .map(|hole| {
                        let points = hole.as_array().unwrap();
                        let center = points.iter().fold((0.0, 0.0), |sum, point| {
                            (
                                sum.0 + point["x"].as_f64().unwrap(),
                                sum.1 + point["y"].as_f64().unwrap(),
                            )
                        });
                        (center.0 / points.len() as f64 - expected.0)
                            .hypot(center.1 / points.len() as f64 - expected.1)
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    closest < 1e-5,
                    "{helper} source hole {} differs by {closest}",
                    pad["id"]
                );
            }
        } else {
            // The published routed helper is open and needs an explicitly
            // reviewed contour integration; preserve this gate rather than
            // manufacturing a closed slot from the author's construction lines.
            assert_eq!(preview["kind"], "error");
            assert!(
                preview["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("outline is open")
            );
        }
    }
}

#[test]
fn nominal_preview_keeps_unqualified_models_visible_without_enabling_fabrication_export() {
    let document = placed_project(25.0);
    let asset_id = document.definitions[0].models.as_ref().unwrap()[0]
        .asset_id
        .clone();
    let request = json!({"snapshotToken":"nominal","expectedRevision":0,"document":document,"target":{"kind":"board","boardId":"board"},"contours":[{"hole":false,"points":[{"x":-30,"y":-30},{"x":55,"y":-30},{"x":55,"y":30},{"x":-30,"y":30}]}],"modelPaths":{asset_id:"models/encoder.stp"}});
    let prepared: Value = serde_json::from_str(&artifact::request(
        &json!({"id":"nominal","kind":"prepare-preview","request":request}).to_string(),
    ))
    .unwrap();
    assert_eq!(prepared["kind"], "prepare-preview", "{prepared}");
    let finish = json!({"plan":prepared["result"],"results":[]});
    let preview: Value = serde_json::from_str(&artifact::request(
        &json!({"id":"preview","kind":"finish-preview","request":finish}).to_string(),
    ))
    .unwrap();
    assert_eq!(preview["kind"], "preview-board", "{preview}");
    assert_eq!(preview["result"]["models"].as_array().unwrap().len(), 2);
    assert!(
        preview["result"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["path"] == "${KIPRJMOD}/models/encoder.stp")
    );
    let exported: Value = serde_json::from_str(&artifact::request(
        &json!({"id":"cannot-bypass","kind":"finish-export","request":finish}).to_string(),
    ))
    .unwrap();
    assert_eq!(
        exported["kind"], "error",
        "Preview preparation must not bypass export qualification"
    );
}

#[test]
fn case_preparation_uses_the_footprint_surface_datum_and_retains_qualification_gates() {
    use boardstudio_core::CoreEngine;
    for (side, reaches_plate) in [("front", true), ("back", false)] {
        let mut document = serde_json::to_value(placed_project(25.)).unwrap();
        document["parts"].as_array_mut().unwrap().truncate(1);
        document["parts"][0]["side"] = json!(side);
        document["boards"][0]["partIds"] = json!(["a"]);
        document["mechanical"] = json!({"boardId":"board","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell","plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,"plateToPcb":3.5,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
        let reply:Value=serde_json::from_str(&CoreEngine::new().request(&json!({"id":"case","kind":"resolve-mechanical","document":document,"contours":[{"hole":false,"points":[{"x":-30,"y":-30},{"x":30,"y":-30},{"x":30,"y":30},{"x":-30,"y":30}]}]}).to_string())).unwrap();
        assert_eq!(reply["kind"], "mechanical-resolved", "{reply}");
        assert_eq!(reply["assembly"]["generationBlocked"], true);
        let findings = reply["assembly"]["diagnostics"].as_array().unwrap();
        assert!(
            findings
                .iter()
                .any(|f| f["id"] == "mechanical:hardware/a/both-side-model-hole-alignment")
        );
        assert_eq!(
            findings
                .iter()
                .any(|f| f["id"] == "mechanical:component-plate:a"),
            reaches_plate,
            "{findings:?}"
        );
        assert!(
            !findings
                .iter()
                .any(|f| f["id"] == "mechanical:engagement:a"),
            "An encoder occupancy profile with no plate aperture must not claim MX plate engagement"
        );
    }
}

#[test]
fn firmware_export_retains_unknown_encoder_timing_as_a_blocker() {
    use boardstudio_core::firmware::{
        FirmwarePartQualification, FirmwareQualification, FirmwareRequest,
    };
    let document = placed_project(25.);
    let definition = &document.definitions[0];
    let profile = definition.hardware_profile.as_ref().unwrap();
    let request = FirmwareRequest {
        qualification: Some(FirmwareQualification {
            board_id: "board".into(),
            parts: vec![FirmwarePartQualification {
                part_id: "a".into(),
                name: definition.name.clone(),
                source: profile.source.clone(),
                gates: profile.gates.clone(),
            }],
        }),
        ..Default::default()
    };
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(error.contains("Encoder pulses per rotation"), "{error}");
}

#[test]
fn firmware_export_accepts_explicit_encoder_config_without_clearing_electrical_assumption() {
    use boardstudio_core::firmware::{
        FirmwareEncoder, FirmwareKey, FirmwarePartQualification, FirmwareQualification,
        FirmwareRequest, ScanPin,
    };
    use boardstudio_core::model::{EncoderDriver, HardwareGate, HardwareOutput, RotaryProfile};

    let document = placed_project(25.);
    let definition = &document.definitions[0];
    let profile = definition.hardware_profile.as_ref().unwrap();
    let mut gates = profile.gates.clone();
    gates.retain(|gate| gate.output != HardwareOutput::Firmware);
    gates.extend([
        HardwareGate {
            output: HardwareOutput::Firmware,
            code: "encoder-pulses-per-rotation".into(),
            message: "Enter explicit rotary timing".into(),
        },
        HardwareGate {
            output: HardwareOutput::Firmware,
            code: "detents-and-quadrature-direction".into(),
            message: "Select encoder timing and direction".into(),
        },
        HardwareGate {
            output: HardwareOutput::Firmware,
            code: "driver-and-config-hardware-validation".into(),
            message: "Select a firmware driver and configuration".into(),
        },
        HardwareGate {
            output: HardwareOutput::Electrical,
            code: "reversible-contact-mapping-unverified".into(),
            message: "Verify selected-variant contact mapping".into(),
        },
    ]);
    let request = FirmwareRequest {
        qualification: Some(FirmwareQualification {
            board_id: "board".into(),
            parts: vec![FirmwarePartQualification {
                part_id: "a".into(),
                name: definition.name.clone(),
                source: profile.source.clone(),
                gates,
            }],
        }),
        encoders: vec![FirmwareEncoder {
            id: "a".into(),
            profile: RotaryProfile {
                a: "A".into(),
                b: "B".into(),
                common: "C".into(),
                steps: Some(20),
                triggers_per_rotation: Some(20),
                driver: Some(EncoderDriver::Ec11),
            },
            a_gpio: "P0.02".into(),
            b_gpio: "P1.15".into(),
        }],
        encoder_ids: vec!["a".into()],
        controller_profile: "ceoloide/mcu_nice_nano".into(),
        board_name: "THQ test".into(),
        rows: vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }],
        columns: vec![ScanPin {
            terminal: "P20".into(),
            gpio: "P0.29".into(),
        }],
        keys: vec![FirmwareKey {
            id: "key".into(),
            row: 0,
            column: 0,
        }],
        diode_direction: "col2row".into(),
        ..Default::default()
    };

    let generated = boardstudio_core::firmware::generate(&request);
    assert!(
        request.qualification.as_ref().unwrap().parts[0]
            .source
            .path
            .starts_with("KiCad/footprints/THQWGD001.pretty/")
    );
    assert_eq!(
        request.qualification.as_ref().unwrap().parts[0]
            .source
            .repository,
        "https://github.com/Taro-Hayashi/THQWGD001"
    );
    assert_eq!(
        request.qualification.as_ref().unwrap().parts[0]
            .source
            .revision,
        "78e1c42dbebca1a9e28cf057d7d84f7eb786aa15"
    );
    let generated = generated.unwrap();
    assert!(generated.warnings.iter().any(|warning| {
        warning.contains("designer-supplied digital configuration")
            && warning.contains("remain unverified")
    }));

    let mut missing_counts = request.clone();
    missing_counts.encoders[0].profile.steps = None;
    let missing_counts = boardstudio_core::firmware::generate(&missing_counts).unwrap_err();
    assert!(missing_counts.contains("needs verified steps per rotation"));

    let mut unrelated_gate = request;
    unrelated_gate
        .qualification
        .as_mut()
        .unwrap()
        .parts
        .push(FirmwarePartQualification {
            part_id: "b".into(),
            name: "Unrelated part".into(),
            source: profile.source.clone(),
            gates: vec![HardwareGate {
                output: HardwareOutput::Firmware,
                code: "unrelated-firmware-gate".into(),
                message: "Unrelated qualification remains required".into(),
            }],
        });
    let unrelated_gate = boardstudio_core::firmware::generate(&unrelated_gate).unwrap_err();
    assert!(
        unrelated_gate.contains("Unrelated part"),
        "{unrelated_gate}"
    );
    assert!(
        unrelated_gate.contains("Unrelated qualification remains required"),
        "{unrelated_gate}"
    );
}
