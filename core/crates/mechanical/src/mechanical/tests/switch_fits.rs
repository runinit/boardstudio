use super::*;
use crate::test_support::*;

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
    doc.boards.push(board("board", vec!["key".into()], 1.2));
    doc.mechanical = Some(config());
    let outline = rectangle(0., 0., 40., 40.);
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
    doc.boards
        .push(board("board", vec!["key".into(), "key2".into()], 1.6));
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
        "../../../../../../catalogue/parts/imported-parts.json"
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
            let outline = rectangle(0.0, 0.0, 140.0, 60.0);
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
        doc.boards.push(board(
            "board",
            doc.parts.iter().map(|p| p.id.clone()).collect(),
            1.6,
        ));
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
        doc.boards.push(board("board", vec!["switch".into()], 1.6));
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
        let outline = [rectangle(0.0, 0.0, 100.0, 100.0)];

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
    doc.boards.push(board("board", vec!["switch".into()], 1.6));
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
    let outline = [rectangle(0.0, 0.0, 100.0, 100.0)];

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
    let two_u = include_str!("../../../../../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod");
    let wide = include_str!("../../../../../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod");
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
    doc.boards.push(board(
        "board",
        parts.iter().map(|part| part.id.clone()).collect(),
        1.6,
    ));
    doc.parts = parts;
    doc.definitions = definitions;
    doc
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
