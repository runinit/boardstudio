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
        document.parts.push(serde_json::from_value(json!({"id":"mount","reference":"CUT1","definitionId":helper,"side":"front","pose":{"at":{"x":0,"y":0},"rotation":0}})).unwrap());
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
    fn requires_eq<T: Eq>(_: &T) {}
    requires_eq(&request);
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(error.contains("Encoder pulses per rotation"), "{error}");
}
