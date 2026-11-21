use super::*;
fn config() -> MechanicalConfiguration {
    MechanicalConfiguration {
        internal_gasket: None,
        gasket_layout: None,
        hardware: None,
        critical_fits: None,
        bottom_style: None,
        middle_frame: None,
        gasket_travel: None,
        openings: None,
        opening_allowance: None,
        stabilizers: None,
        part_processes: None,
        gasket: None,
        closure_mounts: None,
        board_id: "board".into(),
        next_opening_identity: 0,
        next_critical_fit_identity: 0,
        next_hardware_identity: 0,
        next_mount_identity: 0,
        method: PlateMethod::Printed,
        mount: MechanicalMount::Tray,
        integrated_plate_frame: false,
        battery: None,
        mounts: vec![],
        plate_thickness: 1.5,
        plate_foam_thickness: 1.0,
        pcb_thickness: 1.6,
        bottom_foam_thickness: 0.5,
        battery_height: 3.0,
        bottom_thickness: 2.0,
        plate_to_pcb: 3.5,
        wall_thickness: 2.0,
        clearance: 0.2,
        profiles: vec![],
    }
}
#[test]
fn disabled_mechanical_configuration_has_no_stack() {
    let doc = ProjectDoc::empty("test", "test");
    assert!(
        serde_json::to_value(&doc)
            .unwrap()
            .get("mechanical")
            .is_none()
    );
    assert!(resolve(&doc, &[]).stack.is_empty());
}
#[test]
fn stack_is_explicit_and_revision_tagged() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.revision = 7;
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.mechanical = Some(config());
    let result = resolve(&doc, &[]);
    assert_eq!(result.revision, 7);
    assert_eq!(
        result.stack.iter().find(|l| l.id == "bottom").unwrap().z,
        -4.1
    );
    assert_eq!(result.case.bodies[0].contours, result.plate_contours);
}
#[test]
fn custom_dimensions_and_disabled_foam_survive_normalization() {
    let mut c = config();
    c.plate_thickness = 1.8;
    c.plate_foam_thickness = 0.0;
    c.bottom_foam_thickness = 0.0;
    c.bottom_thickness = 4.2;
    c.clearance = 0.45;
    let processes = normalized_processes(&c);
    assert_eq!(c.plate_thickness, 1.8);
    assert_eq!(c.bottom_thickness, 4.2);
    assert_eq!(c.clearance, 0.45);
    assert_eq!(
        processes
            .iter()
            .find(|item| item.part_id == "plate-foam")
            .unwrap()
            .thickness,
        0.0
    );
    assert_eq!(
        processes
            .iter()
            .find(|item| item.part_id == "bottom-foam")
            .unwrap()
            .thickness,
        0.0
    );
}
#[test]
fn no_model_foam_fallbacks_transform_union_and_warn() {
    let mut doc = ProjectDoc::empty("foam", "foam");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["front-a".into(), "front-b".into(), "back".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.extend([
        serde_json::from_value(serde_json::json!({
            "id":"front-a", "definitionId":"courtyard-part", "reference":"U1",
            "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"front-b", "definitionId":"courtyard-part", "reference":"U2",
            "pose":{"at":{"x":23,"y":20},"rotation":0}, "side":"front"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"back", "definitionId":"pad-only-part", "reference":"J1",
            "pose":{"at":{"x":70,"y":70},"rotation":0}, "side":"back"
        }))
        .unwrap(),
    ]);
    doc.definitions.extend([
        serde_json::from_value(serde_json::json!({
            "id":"courtyard-part", "name":"No model front component", "kind":"custom",
            "courtyard":[{"x":-4,"y":-3},{"x":4,"y":-3},{"x":4,"y":3},{"x":-4,"y":3}],
            "pads":[]
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"pad-only-part", "name":"No model back component", "kind":"connector",
            "courtyard":[], "pads":[{"id":"1","number":"1","at":{"x":1,"y":0},
            "size":{"x":4,"y":2},"shape":"rect","rotation":90}]
        }))
        .unwrap(),
    ]);
    let mut configuration = config();
    configuration.clearance = 0.3;
    doc.mechanical = Some(configuration);
    let outline = [Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 100.0, y: 0.0 },
            Vec2 { x: 100.0, y: 100.0 },
            Vec2 { x: 0.0, y: 100.0 },
        ],
    }];

    let assembly = resolve(&doc, &outline);
    let plate_foam = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate-foam")
        .unwrap();
    let bottom_foam = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "bottom-foam")
        .unwrap();
    let hole_bounds = |body: &CaseIR| {
        body.contours
            .iter()
            .filter(|contour| contour.hole)
            .map(|contour| {
                let min_x = contour
                    .points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min);
                let max_x = contour
                    .points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                (min_x, max_x)
            })
            .collect::<Vec<_>>()
    };
    let front_holes = hole_bounds(plate_foam);
    let back_holes = hole_bounds(bottom_foam);
    assert_eq!(
        front_holes.len(),
        1,
        "overlapping courtyard clearances are unioned"
    );
    assert!((front_holes[0].0 - 15.7).abs() < 0.01);
    assert!((front_holes[0].1 - 27.3).abs() < 0.01);
    assert_eq!(back_holes.len(), 1, "back-side pads clear only bottom foam");
    assert!((back_holes[0].0 - 67.7).abs() < 0.01);
    assert!((back_holes[0].1 - 70.3).abs() < 0.01);
    assert!(assembly.diagnostics.iter().any(|finding| {
        finding.id == "mechanical:foam-approximation:front-a"
            && finding.severity == Severity::Warning
    }));
    assert!(!assembly.generation_blocked);
}
#[test]
fn integrated_frame_uses_lid_and_separate_bottom() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut c = config();
    c.integrated_plate_frame = true;
    c.mount = MechanicalMount::Rigid;
    doc.mechanical = Some(c);
    let resolved = resolve(
        &doc,
        &[Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 50.0, y: 0.0 },
                Vec2 { x: 50.0, y: 50.0 },
                Vec2 { x: 0.0, y: 50.0 },
            ],
        }],
    );
    let plate = &resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "plate")
        .unwrap()
        .body;
    assert_eq!(plate.kind, CaseKind::Lid);
    assert!((plate.z.unwrap() + plate.wall_height.unwrap() - 3.5).abs() < 1e-9);
    assert_eq!(
        resolved
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "bottom")
            .unwrap()
            .body
            .kind,
        CaseKind::Plate
    );
}
#[test]
fn gasket_mode_places_discrete_supports_and_retaining_frame_from_defaults() {
    let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut settings = config();
    settings.mount = MechanicalMount::Gasket;
    doc.mechanical = Some(settings);
    let contours = vec![Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0., y: 0. },
            Vec2 { x: 130., y: 0. },
            Vec2 { x: 130., y: 90. },
            Vec2 { x: 0., y: 90. },
        ],
    }];
    let result = resolve(&doc, &contours);
    assert!(!result.generation_blocked, "{:?}", result.diagnostics);
    assert!(result.case.bodies.iter().any(|b| b.body.id == "retainer"));
    assert_eq!(
        result
            .case
            .bodies
            .iter()
            .filter(|b| b.body.id.starts_with("gasket:"))
            .count(),
        12
    );
}

#[test]
fn manual_gasket_anchors_are_exact_persistent_and_invalidated_by_outline_changes() {
    let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut settings = config();
    settings.mount = MechanicalMount::Gasket;
    doc.mechanical = Some(settings);
    let mut contours = vec![Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0., y: 0. },
            Vec2 { x: 130., y: 0. },
            Vec2 { x: 130., y: 90. },
            Vec2 { x: 0., y: 90. },
        ],
    }];
    let first = resolve(&doc, &contours);
    let support = &first.gasket_supports[0];
    let anchor = support.anchor + 0.0007;
    doc.mechanical.as_mut().unwrap().gasket_layout = Some(MechanicalGasketLayout {
        auto_size: None,
        adhesive_thickness: None,
        minimum_foam_thickness: None,
        preset_id: None,
        material: None,
        length: 12.,
        width: 3.,
        thickness: 2.,
        compression: 0.15,
        supports: vec![MechanicalGasketAnchor {
            length: None,
            width: None,
            placement: None,
            id: support.id.clone(),
            region_id: support.region_id.clone(),
            outline_key: support.outline_key.clone(),
            anchor,
            unlinked: false,
        }],
    });
    let moved = resolve(&doc, &contours);
    assert!(!moved.generation_blocked, "{:?}", moved.diagnostics);
    assert!((moved.gasket_supports[0].anchor - anchor).abs() < 1e-9);
    let saved = serde_json::to_string(&doc).unwrap();
    let reopened: ProjectDoc = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        moved.gasket_supports,
        resolve(&reopened, &contours).gasket_supports
    );
    contours[0].points[1].x += 1.;
    assert!(resolve(&doc, &contours).generation_blocked);
}

#[test]
fn gasket_support_reaches_plate_without_rigid_contact() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut c = config();
    c.mount = MechanicalMount::Gasket;
    c.gasket_travel = Some(0.5);
    c.gasket = Some(Gasket {
        inset: 0.2,
        width: 1.0,
        depth: 0.4,
    });
    doc.mechanical = Some(c);
    let resolved = resolve(
        &doc,
        &[Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 50.0, y: 0.0 },
                Vec2 { x: 50.0, y: 50.0 },
                Vec2 { x: 0.0, y: 50.0 },
            ],
        }],
    );
    let bottom = &resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "bottom")
        .unwrap()
        .body;
    let strips = resolved
        .case
        .bodies
        .iter()
        .filter(|b| b.body.id.starts_with("gasket:"))
        .collect::<Vec<_>>();
    assert_eq!(strips.len(), 12);
    for strip in strips {
        if strip.body.id.ends_with(":lower") {
            assert!((strip.body.z.unwrap() + strip.body.thickness - 3.5).abs() < 1e-9);
        } else {
            assert!((strip.body.z.unwrap() - 5.0).abs() < 1e-9);
        }
    }
    let retainer = resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "retainer")
        .unwrap();
    assert!((bottom.z.unwrap() + bottom.thickness - retainer.body.z.unwrap()).abs() < 1e-9);
    assert!(
        bottom
            .mounts
            .as_ref()
            .unwrap()
            .iter()
            .all(|m| m.id.starts_with("closure:"))
    );
    assert!(
        bottom
            .openings
            .as_ref()
            .unwrap()
            .iter()
            .any(|o| (o.z - 1.8).abs() < 1e-9)
    );
}
#[test]
fn cable_exit_regenerates_single_foam_envelope_without_mutation() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut c = config();
    c.battery = Some(MechanicalBattery {
        size: Vec3 {
            x: 8.0,
            y: 8.0,
            z: 3.0,
        },
        at: Vec2 { x: 20.0, y: 20.0 },
        cable_exit: Vec2 { x: 35.0, y: 20.0 },
        cable_width: Some(2.0),
    });
    doc.mechanical = Some(c);
    let outline = [Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 50.0, y: 0.0 },
            Vec2 { x: 50.0, y: 50.0 },
            Vec2 { x: 0.0, y: 50.0 },
        ],
    }];
    let saved = doc.clone();
    let first = resolve(&doc, &outline);
    assert_eq!(doc, saved);
    doc.mechanical
        .as_mut()
        .unwrap()
        .battery
        .as_mut()
        .unwrap()
        .cable_exit
        .y = 35.0;
    let second = resolve(&doc, &outline);
    let foam = |assembly: MechanicalAssembly| {
        assembly
            .case
            .bodies
            .into_iter()
            .find(|body| body.body.id == "bottom-foam")
            .unwrap()
            .contours
    };
    let a = foam(first);
    let b = foam(second);
    assert_ne!(a, b);
    assert_eq!(a.iter().filter(|contour| contour.hole).count(), 1);
    assert_eq!(b.iter().filter(|contour| contour.hole).count(), 1);
}
#[test]
fn part_owned_profiles_supply_case_cutouts_without_case_assignment() {
    let mut doc = ProjectDoc::empty("profiles", "Profiles");
    let profile = builtin_profile(
        "custom-switch".into(),
        MechanicalBuiltinProfile::MxSwitch,
        3.5,
    )
    .unwrap();
    doc.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "custom-switch", "name": "Qualified switch", "kind": "switch",
            "courtyard": [], "pads": [], "mechanicalProfile": profile
        }))
        .unwrap(),
    );
    doc.parts.push(
        serde_json::from_value(serde_json::json!({
            "id":"key", "definitionId":"custom-switch", "reference":"SW1",
            "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
        }))
        .unwrap(),
    );
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.2,
        traces: vec![],
        vias: vec![],
    });
    doc.mechanical = Some(config());
    let outline = Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0., y: 0. },
            Vec2 { x: 40., y: 0. },
            Vec2 { x: 40., y: 40. },
            Vec2 { x: 0., y: 40. },
        ],
    };
    let saved = doc.clone();
    let result = resolve(&doc, &[outline]);
    assert_eq!(
        doc, saved,
        "resolving must not copy inherited fits into the project"
    );
    assert_eq!(
        result
            .plate_contours
            .iter()
            .filter(|contour| contour.hole)
            .count(),
        1
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|finding| finding.id.contains("profile-missing"))
    );
    assert_eq!(
        result
            .stack
            .iter()
            .find(|layer| layer.id == "pcb")
            .unwrap()
            .thickness,
        1.2
    );
}

#[test]
fn switch_profiles_drive_spacing_and_reject_incompatible_engagement() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into(), "key2".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    for (id, definition_id) in [("key", "switch"), ("key2", "switch2")] {
        doc.parts.push(serde_json::from_value(serde_json::json!({"id":id,"definitionId":definition_id,"reference":id,"pose":{"at":{"x":20,"y":20},"rotation":0},"side":"front"})).unwrap());
        doc.definitions.push(serde_json::from_value(serde_json::json!({"id":definition_id,"name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
    }
    let mut c = config();
    c.plate_to_pcb = 5.0;
    c.profiles
        .push(builtin_profile("switch".into(), MechanicalBuiltinProfile::MxSwitch, 4.0).unwrap());
    c.profiles
        .push(builtin_profile("switch2".into(), MechanicalBuiltinProfile::MxSwitch, 4.0).unwrap());
    doc.mechanical = Some(c);
    let first = resolve(&doc, &[]);
    assert_eq!(
        first
            .stack
            .iter()
            .find(|layer| layer.id == "plate")
            .unwrap()
            .z,
        3.5
    );
    assert_eq!(doc.mechanical.as_ref().unwrap().plate_to_pcb, 5.0);
    doc.mechanical.as_mut().unwrap().profiles[1].switch_family =
        Some(MechanicalSwitchFamily::ChocV1);
    doc.mechanical.as_mut().unwrap().profiles[1].plate_to_pcb = 2.0;
    let second = resolve(&doc, &[]);
    assert!(
        second
            .diagnostics
            .iter()
            .any(|finding| finding.id == "mechanical:switch-family"
                && finding.severity == Severity::Error)
    );
    assert!(second.generation_blocked);
}
#[test]
fn imported_stabilizers_use_the_mx_mounting_datum() {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../catalogue/parts/imported-parts.json"
    ))
    .unwrap();
    let stabilizers: Vec<_> = catalogue["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| {
            part["definition"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("kicad:marbastlib/STAB_MX_"))
        })
        .collect();
    assert_eq!(
        stabilizers.len(),
        2,
        "Both imported MX stabilizer sizes must retain datum coverage"
    );
    for imported in stabilizers {
        let definition: PartDefinition =
            serde_json::from_value(imported["definition"].clone()).unwrap();
        for (thickness, legacy) in [(1.4, false), (1.5, false), (1.6, false), (1.5, true)] {
            let mut doc = ProjectDoc::empty("test", "test");
            doc.boards.push(
                serde_json::from_value(serde_json::json!({
                    "id":"board", "name":"Board", "outlineIds":[],
                    "partIds":["key", "stab"], "netIds":[], "thickness":1.6
                }))
                .unwrap(),
            );
            doc.definitions.push(serde_json::from_value(serde_json::json!({
                "id":"switch", "name":"Switch", "kind":"switch", "courtyard":[], "pads":[],
                "generator":{"source":"ceoloide/switch_mx","version":"bundled-1","parameters":{}}
            })).unwrap());
            doc.definitions.push(definition.clone());
            if legacy {
                let profile = doc.definitions[1].mechanical_profile.as_mut().unwrap();
                profile.switch_family = None;
                profile.plate_to_pcb = 5.0;
            }
            for (id, definition_id) in [("key", "switch"), ("stab", definition.id.as_str())] {
                doc.parts.push(
                    serde_json::from_value(serde_json::json!({
                        "id":id,"definitionId":definition_id,"reference":id,
                        "pose":{"at":{"x":70,"y":30},"rotation":0},"side":"front"
                    }))
                    .unwrap(),
                );
            }
            let mut settings = config();
            settings.plate_thickness = thickness;
            doc.mechanical = Some(settings);
            let outline = Contour {
                hole: false,
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 140.0, y: 0.0 },
                    Vec2 { x: 140.0, y: 60.0 },
                    Vec2 { x: 0.0, y: 60.0 },
                ],
            };
            let saved = doc.clone();
            let result = resolve(&doc, std::slice::from_ref(&outline));
            assert_eq!(doc, saved);
            assert!(!result.generation_blocked, "{:?}", result.diagnostics);
            assert!(
                !result
                    .diagnostics
                    .iter()
                    .any(|d| d.id.contains("engagement:")),
                "{} at {thickness}: {:?}",
                definition.id,
                result.diagnostics
            );
            assert_eq!(result.plate_contours.iter().filter(|c| c.hole).count(), 3);
            let plate = result
                .stack
                .iter()
                .find(|layer| layer.id == "plate")
                .unwrap();
            assert!((plate.z + plate.thickness - 5.0).abs() < 1e-9);

            // Unknown/custom engagement remains authoritative, even for a
            // profile that uses the same stabilizer cutout geometry.
            let profile = doc.definitions[1].mechanical_profile.as_mut().unwrap();
            profile.switch_family = None;
            profile.plate_to_pcb = 4.0;
            let invalid = resolve(&doc, std::slice::from_ref(&outline));
            assert!(invalid.generation_blocked);
            assert!(
                invalid
                    .diagnostics
                    .iter()
                    .any(|d| d.id == "mechanical:engagement:stab")
            );

            // A low-profile switch cannot share the MX stabilizer stack.
            doc.definitions[1] = definition.clone();
            doc.mechanical.as_mut().unwrap().profiles.push(
                builtin_profile("switch".into(), MechanicalBuiltinProfile::ChocV1Switch, 2.0)
                    .unwrap(),
            );
            let incompatible = resolve(&doc, &[outline]);
            assert!(incompatible.generation_blocked);
            assert!(
                incompatible
                    .diagnostics
                    .iter()
                    .any(|d| d.id == "mechanical:engagement:stab")
            );
        }
    }
}

#[test]
fn canonical_switch_profiles_produce_seventy_square_openings() {
    for source in ["ceoloide/switch_mx", "ceoloide/switch_choc_v1_v2"] {
        let definition_id = "switch-def";
        let mut doc = ProjectDoc::empty("split", "split");
        doc.definitions.push(serde_json::from_value(serde_json::json!({
            "id":definition_id,"name":"Switch","kind":"switch","courtyard":[],"pads":[],
            "generator":{"source":source,"version":"bundled-1","parameters":{"choc_v1_support":true,"choc_v2_support":false}}
        })).unwrap());
        for half in 0..2 {
            for row in 0..5 {
                for column in 0..7 {
                    let index = half * 35 + row * 7 + column;
                    doc.parts.push(
                        serde_json::from_value(serde_json::json!({
                            "id":format!("switch-{index}"), "definitionId":definition_id,
                            "reference":format!("SW{index}"), "side":"back",
                            "pose":{"at":{"x":half as f64 * 160.0 + column as f64 * 19.0 + 15.0,
                                "y": row as f64 * 19.0 + 15.0},"rotation":0}
                        }))
                        .unwrap(),
                    );
                }
            }
        }
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: doc.parts.iter().map(|p| p.id.clone()).collect(),
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut settings = config();
        settings.plate_thickness = if source.ends_with("switch_mx") {
            1.5
        } else {
            1.3
        };
        settings.plate_to_pcb = if source.ends_with("switch_mx") {
            3.5
        } else {
            2.2
        };
        settings.plate_foam_thickness = 0.0;
        doc.mechanical = Some(settings);
        let outlines = (0..2)
            .map(|half| {
                let x = half as f64 * 160.0;
                Contour {
                    hole: false,
                    points: vec![
                        Vec2 { x, y: 0.0 },
                        Vec2 {
                            x: x + 145.0,
                            y: 0.0,
                        },
                        Vec2 {
                            x: x + 145.0,
                            y: 105.0,
                        },
                        Vec2 { x, y: 105.0 },
                    ],
                }
            })
            .collect::<Vec<_>>();
        let assembly = resolve(&doc, &outlines);
        assert!(!assembly.generation_blocked, "{:?}", assembly.diagnostics);
        let openings = assembly
            .plate_contours
            .iter()
            .filter(|c| c.hole)
            .collect::<Vec<_>>();
        assert_eq!(openings.len(), 70, "{definition_id}");
        for opening in openings {
            for axis in [true, false] {
                let values = opening.points.iter().map(|p| if axis { p.x } else { p.y });
                let min = values.clone().fold(f64::INFINITY, f64::min);
                let max = values.fold(f64::NEG_INFINITY, f64::max);
                assert!(
                    (max - min - 14.0).abs() < 0.001,
                    "{definition_id}: {min}..{max}"
                );
            }
        }
    }
}

#[test]
fn generator_switches_derive_their_fit_and_plate_gap() {
    for (source, parameters, family, plate_thickness, gap) in [
        (
            "ceoloide/switch_mx",
            serde_json::json!({}),
            MechanicalSwitchFamily::Mx,
            1.5,
            3.5,
        ),
        (
            "ceoloide/switch_choc_v1_v2",
            serde_json::json!({"choc_v1_support":true,"choc_v2_support":false}),
            MechanicalSwitchFamily::ChocV1,
            1.3,
            2.2,
        ),
        (
            "ceoloide/switch_choc_v1_v2",
            serde_json::json!({"choc_v1_support":{"value":false},"choc_v2_support":{"value":true}}),
            MechanicalSwitchFamily::ChocV2,
            1.5,
            3.5,
        ),
    ] {
        let mut doc = ProjectDoc::empty("switch-fit", "switch-fit");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["switch".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(
            serde_json::from_value(serde_json::json!({
                "id":"switch", "definitionId":"switch-def", "reference":"SW1",
                "pose":{"at":{"x":50,"y":50},"rotation":0}, "side":"front"
            }))
            .unwrap(),
        );
        doc.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id":"switch-def", "name":"Switch", "kind":"switch", "courtyard":[], "pads":[],
                "generator":{"source":source,"version":"1","parameters":parameters}
            }))
            .unwrap(),
        );
        let mut configuration = config();
        configuration.plate_thickness = plate_thickness;
        configuration.plate_to_pcb = gap;
        configuration.plate_foam_thickness = 0.0;
        doc.mechanical = Some(configuration);
        let outline = [Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 100.0, y: 0.0 },
                Vec2 { x: 100.0, y: 100.0 },
                Vec2 { x: 0.0, y: 100.0 },
            ],
        }];

        let assembly = resolve(&doc, &outline);
        assert!(
            !assembly.generation_blocked,
            "{source}: {:#?}",
            assembly.diagnostics
        );
        let plate = assembly
            .stack
            .iter()
            .find(|layer| layer.id == "plate")
            .unwrap();
        assert_eq!(plate.z, gap, "{source} mounting datum gap");
        assert_eq!(plate.thickness, plate_thickness, "{source} plate preset");
        let cutout = assembly
            .plate_contours
            .iter()
            .find(|contour| contour.hole)
            .unwrap();
        let min_x = cutout
            .points
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = cutout
            .points
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let expected_width = match family {
            MechanicalSwitchFamily::Mx => 14.0,
            MechanicalSwitchFamily::ChocV1 => 14.0,
            MechanicalSwitchFamily::ChocV2 => 14.0,
        };
        assert!(
            (max_x - min_x - expected_width).abs() < 0.01,
            "{source} cutout width"
        );
    }
}

#[test]
fn unknown_switch_requires_a_fit_choice_before_plate_generation() {
    let mut doc = ProjectDoc::empty("unknown-switch", "unknown-switch");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["switch".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(
        serde_json::from_value(serde_json::json!({
            "id":"switch", "definitionId":"imported", "reference":"SW1",
            "pose":{"at":{"x":50,"y":50},"rotation":0}, "side":"front"
        }))
        .unwrap(),
    );
    doc.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id":"imported", "name":"Imported switch", "kind":"switch", "courtyard":[], "pads":[]
        }))
        .unwrap(),
    );
    let configuration = config();
    doc.mechanical = Some(configuration);
    let outline = [Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 100.0, y: 0.0 },
            Vec2 { x: 100.0, y: 100.0 },
            Vec2 { x: 0.0, y: 100.0 },
        ],
    }];

    let unresolved = resolve(&doc, &outline);
    assert!(unresolved.generation_blocked);
    assert!(
        unresolved
            .diagnostics
            .iter()
            .any(|finding| finding.id == "mechanical:switch-family")
    );

    let family = MechanicalSwitchFamily::ChocV1;
    let mut profile = builtin_profile(
        "imported".into(),
        profile_source_for_family(family),
        switch_mounting_datum(family) - 1.3,
    )
    .unwrap();
    profile.switch_family = Some(family);
    let configuration = doc.mechanical.as_mut().unwrap();
    configuration.plate_thickness = 1.3;
    configuration.plate_to_pcb = 2.2;
    configuration.plate_foam_thickness = 2.0;
    configuration.profiles.push(profile);
    let selected = resolve(&doc, &outline);
    assert!(!selected.generation_blocked, "{:#?}", selected.diagnostics);
    assert_eq!(
        selected
            .stack
            .iter()
            .find(|layer| layer.id == "plate")
            .unwrap()
            .z,
        2.2
    );
}
#[test]
fn switch_family_inference_follows_the_generator_and_part_overrides() {
    use MechanicalSwitchFamily::{ChocV1, ChocV2, Mx};
    use serde_json::json;
    let choc = "ceoloide/switch_choc_v1_v2";
    for (source, definition_parameters, part_parameters, expected) in [
        ("ceoloide/switch_mx", json!({}), None, Some(Mx)),
        ("Ceoloide/Switch_MX", json!({}), None, Some(Mx)),
        ("someone/else", json!({}), None, None),
        (choc, json!({}), None, None),
        (
            choc,
            json!({"choc_v1_support":true,"choc_v2_support":false}),
            None,
            Some(ChocV1),
        ),
        (
            choc,
            json!({"choc_v1_support":{"value":false},"choc_v2_support":{"value":true}}),
            None,
            Some(ChocV2),
        ),
        (
            choc,
            json!({"choc_v1_support":false,"choc_v2_support":false}),
            None,
            None,
        ),
        // A part-level value replaces the definition's value for that key.
        (
            choc,
            json!({"choc_v1_support":true,"choc_v2_support":false}),
            Some(json!({"choc_v1_support":false,"choc_v2_support":true})),
            Some(ChocV2),
        ),
        // A part-level value that is not a boolean reads as the default (true) instead
        // of falling back to the definition's value.
        (
            choc,
            json!({"choc_v1_support":false,"choc_v2_support":true}),
            Some(json!({"choc_v1_support":"yes"})),
            None,
        ),
    ] {
        let definition = generator_definition("def", source, definition_parameters);
        let part = switch_part("key", "def", part_parameters);
        assert_eq!(
            inferred_switch_family(&part, &definition),
            expected,
            "{source}"
        );
    }
}

#[test]
fn legacy_mx_stabilizer_profiles_are_recognized_only_by_reviewed_source_and_original_gap() {
    let two_u = include_str!("../../../../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod");
    let wide = include_str!("../../../../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod");
    for (text, gap, family, spacing) in [
        (Some(two_u), 5.0, Some(MechanicalSwitchFamily::Mx), 3.5),
        (Some(wide), 5.0, Some(MechanicalSwitchFamily::Mx), 3.5),
        (Some(two_u), 4.0, None, 4.0),
        (Some("custom"), 5.0, None, 5.0),
        (None, 5.0, None, 5.0),
    ] {
        let mut configuration = config();
        configuration.profiles.push(MechanicalPartProfile {
            source_geometry: text.map(|text| MechanicalProfileSource {
                text: text.into(),
                sha256: String::new(),
                mappings: vec![],
                source_ids: vec![],
            }),
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: None,
            definition_id: "stab".into(),
            source: "test".into(),
            cutouts: vec![],
            plate_to_pcb: gap,
        });
        let doc = switch_document(
            vec![switch_part("key", "def", None)],
            vec![generator_definition(
                "def",
                "ceoloide/switch_mx",
                serde_json::json!({}),
            )],
        );
        let inputs = resolve_profiles(&doc, &configuration).unwrap();
        let stabilizer = inputs
            .config
            .profiles
            .iter()
            .find(|profile| profile.definition_id == "stab")
            .unwrap();
        assert_eq!(stabilizer.switch_family, family, "{gap}");
        assert_eq!(stabilizer.plate_to_pcb, spacing, "{gap}");
    }
}

#[test]
fn assigned_and_inferred_families_decide_compatibility_before_resolution() {
    use MechanicalSwitchFamily::{ChocV1, ChocV2, Mx};
    use serde_json::json;
    let assigned = |definition_id: &str, family: MechanicalSwitchFamily| {
        let mut profile = builtin_profile(
            definition_id.into(),
            profile_source_for_family(family),
            switch_mounting_datum(family) - 1.5,
        )
        .unwrap();
        profile.switch_family = Some(family);
        profile
    };
    let mx = || generator_definition("def", "ceoloide/switch_mx", json!({}));
    let single = |definition: PartDefinition, profiles: Vec<MechanicalPartProfile>| {
        let mut configuration = config();
        configuration.plate_to_pcb = 4.2;
        configuration.profiles = profiles;
        let doc = switch_document(vec![switch_part("key", "def", None)], vec![definition]);
        resolve_profiles(&doc, &configuration).unwrap()
    };

    // An inferred family creates its profile and drives the plate spacing.
    let created = single(mx(), vec![]);
    assert!(created.incompatible_switches.is_empty());
    assert_eq!(created.config.profiles.len(), 1);
    assert_eq!(created.config.profiles[0].switch_family, Some(Mx));
    assert_eq!(created.config.plate_to_pcb, 3.5);

    // An assignment that disagrees with the generator is incompatible even at the same datum.
    let disagreeing = single(mx(), vec![assigned("def", ChocV2)]);
    assert_eq!(disagreeing.incompatible_switches, ["key"]);
    let agreeing = single(mx(), vec![assigned("def", Mx)]);
    assert!(agreeing.incompatible_switches.is_empty());

    // The definition's own profile replaces a configuration profile for the same definition.
    let mut owned = mx();
    owned.mechanical_profile = Some(assigned("def", Mx));
    let overridden = single(owned, vec![assigned("def", ChocV2)]);
    assert!(overridden.incompatible_switches.is_empty());
    assert_eq!(overridden.config.profiles.len(), 1);
    assert_eq!(overridden.config.profiles[0].switch_family, Some(Mx));

    // An unknown family adds no profile and leaves the configured spacing alone.
    let unknown = single(
        serde_json::from_value(
            json!({"id":"def","name":"def","kind":"switch","courtyard":[],"pads":[]}),
        )
        .unwrap(),
        vec![],
    );
    assert!(unknown.incompatible_switches.is_empty());
    assert!(unknown.config.profiles.is_empty());
    assert_eq!(unknown.config.plate_to_pcb, 4.2);

    // Switches whose mounting datum differs from the first resolved family are listed.
    let doc = switch_document(
        vec![
            switch_part("a", "def-mx", None),
            switch_part("b", "def-choc", None),
        ],
        vec![
            generator_definition("def-mx", "ceoloide/switch_mx", json!({})),
            generator_definition(
                "def-choc",
                "ceoloide/switch_choc_v1_v2",
                json!({"choc_v1_support":true,"choc_v2_support":false}),
            ),
        ],
    );
    let mixed = resolve_profiles(&doc, &config()).unwrap();
    assert_eq!(mixed.incompatible_switches, ["b"]);
    assert_eq!(mixed.config.plate_to_pcb, 3.5);
    assert_eq!(
        mixed
            .config
            .profiles
            .iter()
            .find(|profile| profile.definition_id == "def-choc")
            .and_then(|profile| profile.switch_family),
        Some(ChocV1)
    );
}

fn switch_part(id: &str, definition_id: &str, parameters: Option<serde_json::Value>) -> Part {
    let mut part = serde_json::json!({
        "id":id, "definitionId":definition_id, "reference":id,
        "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
    });
    if let Some(parameters) = parameters {
        part["generatorParameters"] = parameters;
    }
    serde_json::from_value(part).unwrap()
}

fn generator_definition(id: &str, source: &str, parameters: serde_json::Value) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":id, "name":id, "kind":"switch", "courtyard":[], "pads":[],
        "generator":{"source":source,"version":"1","parameters":parameters}
    }))
    .unwrap()
}

fn switch_document(parts: Vec<Part>, definitions: Vec<PartDefinition>) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("family", "family");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: parts.iter().map(|part| part.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts = parts;
    doc.definitions = definitions;
    doc
}

#[test]
fn duplicate_rings_ignore_winding_start_and_closure() {
    let a = Contour {
        hole: true,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
        ],
    };
    let b = Contour {
        hole: true,
        points: vec![a.points[1], a.points[0], a.points[2], a.points[1]],
    };
    assert!(same_ring(&a, &b));
}
#[test]
fn profiles_follow_rotation_and_back_side_mirror() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":10,"y":20},"rotation":90},"side":"back"})).unwrap());
    let mut configuration = config();
    configuration.profiles.push(MechanicalPartProfile {
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        switch_family: None,
        definition_id: "switch".into(),
        source: "custom".into(),
        cutouts: vec![vec![
            Vec2 { x: 1.0, y: 0.0 },
            Vec2 { x: 2.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
        ]],
        plate_to_pcb: 3.5,
        supported_thickness: None,
    });
    doc.mechanical = Some(configuration);
    let resolved = resolve(&doc, &[]);
    assert!((resolved.plate_contours[0].points[0].x - 10.0).abs() < 1e-9);
    assert!((resolved.plate_contours[0].points[0].y - 19.0).abs() < 1e-9);
}
#[test]
fn combined_switch_stabilizer_deduplicates_and_regenerates_with_pose() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":30,"y":30},"rotation":0},"side":"front"})).unwrap());
    let mut c = config();
    let mut profile =
        builtin_profile("switch".into(), MechanicalBuiltinProfile::MxSwitch, 3.5).unwrap();
    profile.cutouts.extend(
        builtin_profile("switch".into(), MechanicalBuiltinProfile::MxStab2u, 3.5)
            .unwrap()
            .cutouts,
    );
    let expected = profile.cutouts.len();
    c.profiles.push(profile);
    c.stabilizers = Some(vec![MechanicalStabilizerOverride {
        part_id: "key".into(),
        kind: MechanicalStabilizerKind::PcbMount,
        units: 2.0,
        rotation: None,
        profile: None,
    }]);
    doc.mechanical = Some(c);
    let first = resolve(&doc, &[]);
    assert_eq!(first.plate_contours.len(), expected);
    doc.parts[0].pose.at.x += 10.0;
    doc.revision += 1;
    let second = resolve(&doc, &[]);
    assert_eq!(second.plate_contours.len(), expected);
    assert!(
        (second.plate_contours[0].points[0].x - first.plate_contours[0].points[0].x - 10.0).abs()
            < 1e-9
    );
    assert_eq!(second.revision, 1);
}
#[test]
fn outer_expansion_preserves_nominal_holes() {
    let outer = Contour {
        hole: false,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 20.0, y: 0.0 },
            Vec2 { x: 20.0, y: 20.0 },
            Vec2 { x: 0.0, y: 20.0 },
        ],
    };
    let hole = Contour {
        hole: true,
        points: vec![
            Vec2 { x: 5.0, y: 5.0 },
            Vec2 { x: 5.0, y: 10.0 },
            Vec2 { x: 10.0, y: 10.0 },
            Vec2 { x: 10.0, y: 5.0 },
        ],
    };
    let expanded = expanded_outline(&[outer, hole.clone()], 2.0, 1).unwrap();
    assert!(expanded.contains(&hole));
    assert!(
        expanded
            .iter()
            .filter(|c| !c.hole)
            .flat_map(|c| c.points.iter())
            .any(|p| p.x < 0.0)
    );
}
#[test]
fn builtin_profiles_have_library_cutouts() {
    for source in [
        MechanicalBuiltinProfile::MxStab2u,
        MechanicalBuiltinProfile::MxStab625u,
    ] {
        let profile = builtin_profile("switch".into(), source, 3.5).unwrap();
        assert!(!profile.cutouts.is_empty());
        assert!(profile.cutouts.iter().all(|p| p.len() >= 3));
        assert!(profile.supported_thickness.is_none());
    }
    for (source, family, cutout_width, provenance) in [
        (
            MechanicalBuiltinProfile::MxSwitch,
            MechanicalSwitchFamily::Mx,
            14.0,
            "Cherry MX datasheet",
        ),
        (
            MechanicalBuiltinProfile::ChocV1Switch,
            MechanicalSwitchFamily::ChocV1,
            14.0,
            "CPG135001D01-16",
        ),
        (
            MechanicalBuiltinProfile::ChocV2Switch,
            MechanicalSwitchFamily::ChocV2,
            14.0,
            "CPG135301D03",
        ),
    ] {
        let profile = builtin_profile("switch".into(), source, 2.0).unwrap();
        assert_eq!(profile.switch_family, Some(family));
        assert!(profile.source.contains(provenance));
        let min_x = profile.cutouts[0]
            .iter()
            .map(|point| point.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = profile.cutouts[0]
            .iter()
            .map(|point| point.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_x - min_x - cutout_width).abs() < 0.001);
        assert!(profile.supported_thickness.is_some());
    }
}
#[test]
fn rejects_impossible_foam_and_rigid_gasket() {
    let mut doc = ProjectDoc::empty("test", "test");
    let mut c = config();
    c.plate_foam_thickness = 4.0;
    c.mount = MechanicalMount::Gasket;
    c.integrated_plate_frame = true;
    doc.mechanical = Some(c);
    let result = resolve(&doc, &[]);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.id == "mechanical:foam-engagement")
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.id == "mechanical:gasket-rigid-frame")
    );
}

#[test]
fn mechanical_defaults_have_their_reference_values() {
    assert_eq!(mounting_datum(MechanicalSwitchFamily::Mx), 5.0);
    assert_eq!(mounting_datum(MechanicalSwitchFamily::ChocV1), 3.5);
    assert_eq!(mounting_datum(MechanicalSwitchFamily::ChocV2), 5.0);
    assert_eq!(default_plate_thickness(MechanicalSwitchFamily::Mx), 1.5);
    assert_eq!(default_plate_thickness(MechanicalSwitchFamily::ChocV1), 1.3);
    assert_eq!(default_plate_foam_thickness(3.5), 3.0);
    assert_eq!(default_plate_foam_thickness(0.0), 0.0);
    assert_eq!(default_material("plate", &PlateMethod::Printed), "PLA");
    assert_eq!(default_material("plate-foam", &PlateMethod::Printed), "EVA");
    let process = default_layer_process("plate-foam", PlateMethod::Printed, 2.0);
    assert_eq!(process.method, PlateMethod::CutSheet);
    assert_eq!(process.material, "EVA");
    assert_eq!(process.thickness, 2.0);
    let layout = default_gasket_layout();
    assert_eq!(layout.length, 80.0);
    assert_eq!(layout.width, 3.0);
    assert_eq!(layout.compression, 0.15);
    assert!(layout.supports.is_empty());
    let gasket = default_internal_gasket();
    assert_eq!(gasket.version, GasketConstructionVersion::InternalV1);
    assert_eq!(gasket.hardware.id, "custom-m2");
    assert_eq!(
        gasket.hardware.screw_lengths,
        vec![8.0, 10.0, 12.0, 14.0, 15.0, 16.0]
    );
    assert_eq!(gasket.hardware.drive, ScrewDrive::Hex);
    assert_eq!(gasket.hardware.installation, InsertInstallation::HeatSet);
    assert_eq!(gasket.hardware.head_profile, ScrewHeadProfile::Flat);
    assert_eq!(gasket.hardware.length_datum, ScrewLengthDatum::UnderHead);
}
