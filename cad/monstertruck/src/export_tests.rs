use crate::{ProfileInput, ProfileKind, ProfileRegion, build_profile, export_step};
use boardstudio_contracts::Vec2;
use step_p21::ast::{EntityInstance, Exchange};

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> ProfileRegion {
    ProfileRegion {
        outer: [
            (x, y),
            (x + width, y),
            (x + width, y + height),
            (x, y + height),
        ]
        .map(|(x, y)| Vec2 { x, y })
        .into(),
        holes: vec![],
    }
}

#[test]
fn exact_step_exports_one_mm_solid() {
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 4.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![rectangle(3.0, -5.0, 20.0, 10.0)],
    })
    .unwrap();
    let bytes = export_step(&model).expect("exact model exports successfully");
    let text = std::str::from_utf8(&bytes).unwrap();
    let parsed: Exchange = text.parse().expect("independent Part21 syntax parser");
    let count = parsed.data.iter().flat_map(|s| &s.entities).filter(|e| {
        matches!(e, EntityInstance::Simple { record, .. } if record.name == "MANIFOLD_SOLID_BREP")
    }).count();
    assert_eq!(count, 1);
    assert!(text.contains("SI_UNIT(.MILLI.,.METRE.)"));
}

#[test]
fn named_step_keeps_body_identity_and_all_disconnected_solids() {
    use crate::{StepBody, export_step_bodies};
    use step_p21::ast::Parameter;
    let build = |regions| {
        build_profile(ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 4.0,
            thickness: 2.0,
            wall_height: 0.0,
            regions,
        })
        .unwrap()
    };
    let first = build(vec![rectangle(3.0, -5.0, 20.0, 10.0)]);
    let second = build(vec![
        rectangle(40.0, 0.0, 4.0, 3.0),
        rectangle(50.0, 0.0, 4.0, 3.0),
    ]);
    let bytes = export_step_bodies(&[
        StepBody {
            id: "first",
            name: "Plate's first body",
            model: &first,
        },
        StepBody {
            id: "second",
            name: "two islands",
            model: &second,
        },
    ])
    .expect("named exact bodies export successfully");
    let parsed: Exchange = std::str::from_utf8(&bytes).unwrap().parse().unwrap();
    let products = parsed
        .data
        .iter()
        .flat_map(|s| &s.entities)
        .filter_map(|e| match e {
            EntityInstance::Simple { record, .. } if record.name == "PRODUCT" => {
                let Parameter::List(p) = &record.parameter else {
                    panic!("product parameters")
                };
                Some((p[0].clone(), p[1].clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        products,
        vec![
            (
                Parameter::String("first".into()),
                Parameter::String("Plate's first body".into())
            ),
            (
                Parameter::String("second".into()),
                Parameter::String("two islands".into())
            ),
        ]
    );
    assert_eq!(parsed.data.iter().flat_map(|s| &s.entities).filter(|e| {
        matches!(e, EntityInstance::Simple { record, .. } if record.name == "MANIFOLD_SOLID_BREP")
    }).count(), 3);
    let text = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(
        crate::step_accuracy::product_solid_counts(text).unwrap(),
        vec![
            ("first".into(), "Plate's first body".into(), 1),
            ("second".into(), "two islands".into(), 2),
        ]
    );
    let solids = crate::step_accuracy::inspect(text).unwrap();
    assert_eq!(
        solids.iter().map(|s| s.volume).collect::<Vec<_>>(),
        vec![400.0, 24.0, 24.0]
    );
    assert_eq!(solids[1].bounds, [[40.0, 0.0, 4.0], [44.0, 3.0, 6.0]]);
    assert_eq!(solids[2].bounds, [[50.0, 0.0, 4.0], [54.0, 3.0, 6.0]]);
}

#[test]
fn raw_step_measures_exact_hole_material_bounds_and_closed_topology() {
    let mut region = rectangle(3.0, -5.0, 20.0, 10.0);
    region.holes.push(rectangle(8.0, -3.0, 4.0, 5.0).outer);
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 4.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![region],
    })
    .unwrap();
    let step = String::from_utf8(export_step(&model).unwrap()).unwrap();
    let solids =
        crate::step_accuracy::inspect(&step).expect("independent planar B-rep measurement");
    assert_eq!(solids.len(), 1);
    let solid = &solids[0];
    // Analytic: (20*10 - 4*5)*2 = 360 mm3.
    assert!((solid.volume - 360.0).abs() < 0.1);
    assert_eq!(solid.bounds, [[3.0, -5.0, 4.0], [23.0, 5.0, 6.0]]);
    assert_eq!((solid.vertices, solid.edges, solid.faces), (16, 24, 10));
    assert!(solid.contains([4.0, -4.0, 5.0]));
    assert!(
        !solid.contains([10.0, 0.0, 5.0]),
        "through-hole contains no material"
    );
    assert!(
        !solid.contains([4.0, -4.0, 7.0]),
        "no material above the exact top face"
    );
}

#[test]
fn independent_external_freecad_cube_keeps_reference_geometry() {
    let step = include_str!("../tests/fixtures/freecad-cube.step");
    let solids = crate::step_accuracy::inspect(step).expect("external FreeCAD planar reference");
    assert_eq!(solids.len(), 1);
    assert!((solids[0].volume - 1000.0).abs() < 0.1);
    assert_eq!(solids[0].bounds, [[0.0; 3], [10.0; 3]]);
    assert_eq!(
        (solids[0].vertices, solids[0].edges, solids[0].faces),
        (8, 12, 6)
    );
    assert!(solids[0].contains([5.0; 3]));
    assert!(!solids[0].contains([11.0, 5.0, 5.0]));
}

fn control_step() -> String {
    let outer = [
        (10.0, 0.0),
        (30.0, 0.0),
        (30.0, 30.0),
        (0.0, 30.0),
        (0.0, 10.0),
        (10.0, 10.0),
    ]
    .map(|(x, y)| Vec2 { x, y })
    .into();
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![ProfileRegion {
            outer,
            holes: vec![rectangle(15.0, 15.0, 5.0, 5.0).outer],
        }],
    })
    .unwrap();
    String::from_utf8(export_step(&model).unwrap()).unwrap()
}

fn expected(id: &str) -> serde_json::Value {
    let all: serde_json::Value =
        serde_json::from_str(include_str!("../../test/fixtures/step-expectations.json")).unwrap();
    all["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

#[test]
fn deleted_corner_and_cutout_keep_existing_independent_acceptance() {
    let solids = crate::step_accuracy::inspect(&control_step()).unwrap();
    let want = expected("deleted-corner-and-cutout");
    let amount = &want["expect"]["volume"];
    assert!(
        (solids[0].volume - amount["value"].as_f64().unwrap()).abs()
            <= amount["tolerance"].as_f64().unwrap()
    );
    assert_eq!(solids[0].bounds, [[0.0, 0.0, 0.0], [30.0, 30.0, 2.0]]);
    assert!(
        !solids[0].contains([5.0, 5.0, 1.0]),
        "deleted corner stays absent"
    );
    assert!(
        !solids[0].contains([17.0, 17.0, 1.0]),
        "authored cutout stays absent"
    );
    assert!(solids[0].contains([25.0, 5.0, 1.0]));
}

#[test]
fn prepared_holed_plate_keeps_the_kernel_acceptance_reference() {
    let mut region = rectangle(-0.5, -0.5, 21.0, 21.0);
    region.holes.push(rectangle(5.5, 5.5, 9.0, 9.0).outer);
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![region],
    })
    .unwrap();
    let bytes = export_step(&model).unwrap();
    let solids = crate::step_accuracy::inspect(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let want = expected("holed-plate-export");
    let amount = &want["expect"]["volume"];
    assert_eq!(solids.len(), 1);
    assert!(
        (solids[0].volume - amount["value"].as_f64().unwrap()).abs()
            <= amount["tolerance"].as_f64().unwrap()
    );
    assert_eq!(solids[0].bounds, [[-0.5, -0.5, 0.0], [20.5, 20.5, 2.0]]);
    assert!(!solids[0].contains([10.0, 10.0, 1.0]));
    assert!(solids[0].contains([0.0, 0.0, 1.0]));
}

#[test]
fn offset_products_keep_existing_compound_volume_and_placements() {
    use crate::{StepBody, export_step_bodies};
    let model = |z| {
        build_profile(ProfileInput {
            kind: ProfileKind::Plate,
            base_z: z,
            thickness: 2.0,
            wall_height: 0.0,
            regions: vec![rectangle(0.0, 0.0, 20.0, 20.0)],
        })
        .unwrap()
    };
    let lower = model(0.0);
    let upper = model(8.0);
    let bytes = export_step_bodies(&[
        StepBody {
            id: "plate-0",
            name: "lower",
            model: &lower,
        },
        StepBody {
            id: "plate-1",
            name: "upper",
            model: &upper,
        },
    ])
    .unwrap();
    let solids = crate::step_accuracy::inspect(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let want = expected("compound-two-offset-plates");
    let amount = &want["expect"]["volume"];
    assert_eq!(
        solids.len(),
        want["baseline"]["solids"].as_u64().unwrap() as usize
    );
    assert!(
        (solids.iter().map(|s| s.volume).sum::<f64>() - amount["value"].as_f64().unwrap()).abs()
            <= amount["tolerance"].as_f64().unwrap()
    );
    assert_eq!(solids[0].bounds, [[0.0; 3], [20.0, 20.0, 2.0]]);
    assert_eq!(solids[1].bounds, [[0.0, 0.0, 8.0], [20.0, 20.0, 10.0]]);
    assert!(
        !solids.iter().any(|s| s.contains([10.0, 10.0, 5.0])),
        "placement gap remains empty"
    );
}

fn map_points(step: &str, transform: impl Fn([f64; 3]) -> [f64; 3]) -> String {
    const OPEN: &str = "CARTESIAN_POINT(";
    let mut changed = 0;
    let result = step
        .lines()
        .map(|line| {
            let Some(args) = line.find(OPEN).map(|i| i + OPEN.len()) else {
                return line.to_owned();
            };
            let start = line[args..].find('(').unwrap() + args + 1;
            let end = line[start..].find(')').unwrap() + start;
            let coordinates = line[start..end]
                .split(',')
                .map(|s| s.trim().parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            if coordinates.len() != 3 {
                return line.to_owned();
            }
            changed += 1;
            let [x, y, z] = transform(coordinates.try_into().unwrap());
            format!(
                "{}{x:.17E},{y:.17E},{z:.17E}{}",
                &line[..start],
                &line[end..]
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(changed > 0, "control must actually rewrite geometry");
    result
}

#[test]
fn independent_accuracy_preserves_inch_and_metre_unit_controls() {
    let step = control_step();
    let metre = map_points(&step, |p| p.map(|x| x / 1000.0))
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let inch = map_points(&step, |p| p.map(|x| x / 25.4))
        .replace("SI_UNIT(.MILLI.,.METRE.)", "CONVERSION_BASED_UNIT('INCH',#9001)")
        .replace("ENDSEC;\nEND-ISO-10303-21;", "#9001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#9002);\n#9002=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\nENDSEC;\nEND-ISO-10303-21;");
    for text in [step, metre, inch] {
        let solids = crate::step_accuracy::inspect(&text).unwrap();
        assert!((solids[0].volume - 1550.0).abs() <= 0.1);
        for axis in 0..3 {
            assert!(solids[0].bounds[0][axis].abs() <= 0.01);
            assert!((solids[0].bounds[1][axis] - [30.0, 30.0, 2.0][axis]).abs() <= 0.01);
        }
    }
}

#[test]
fn independent_accuracy_distinguishes_reader_solid_and_topology_failures() {
    use crate::step_accuracy::{Error, inspect};
    let step = control_step();
    assert!(matches!(
        inspect(&step[..step.len() / 2]),
        Err(Error::Syntax)
    ));
    assert!(matches!(inspect("not a STEP file"), Err(Error::Syntax)));
    assert!(matches!(
        inspect(&step.replace("MANIFOLD_SOLID_BREP", "UNKNOWN_SOLID_THING")),
        Err(Error::NoSolids)
    ));
    let args = step.find("CLOSED_SHELL(").unwrap() + "CLOSED_SHELL(".len();
    let at = step[args..].find('(').unwrap() + args + 1;
    let comma = step[at..].find(',').unwrap() + at;
    let open = format!("{}{}", &step[..at], &step[comma + 1..]);
    assert!(matches!(inspect(&open), Err(Error::Topology)));
    assert!(matches!(
        inspect(&step.replace("LENGTH_UNIT()", "UNKNOWN_LENGTH_UNIT()")),
        Err(Error::Units)
    ));
}

#[test]
fn valid_wrong_geometry_fails_existing_reference_measurements() {
    let lifted = map_points(&control_step(), |p| {
        if p[2] == 2.0 { [p[0], p[1], 3.0] } else { p }
    });
    let solids =
        crate::step_accuracy::inspect(&lifted).expect("lifted shape remains a closed planar solid");
    assert!((solids[0].volume - 2325.0).abs() <= 0.1);
    let want = expected("deleted-corner-and-cutout");
    let amount = &want["expect"]["volume"];
    assert!(
        (solids[0].volume - amount["value"].as_f64().unwrap()).abs()
            > amount["tolerance"].as_f64().unwrap()
    );
}

#[test]
fn empty_body_geometry_is_refused_without_silently_dropping_identity() {
    use crate::{ExactModel, StepBody, export_step_bodies};
    let empty = ExactModel { solids: vec![] };
    assert!(export_step(&empty).is_err());
    assert!(export_step_bodies(&[]).is_err());
    assert!(
        export_step_bodies(&[StepBody {
            id: "empty",
            name: "empty body",
            model: &empty
        }])
        .is_err()
    );
}

#[test]
fn caller_identity_is_encoded_as_part21_strings() {
    use crate::{StepBody, export_step_bodies};
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![rectangle(0.0, 0.0, 2.0, 1.0)],
    })
    .unwrap();
    let bytes = export_step_bodies(&[StepBody {
        id: "plate's",
        name: "Ω\\name\n",
        model: &model,
    }])
    .unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains(r"PRODUCT('plate''s', '\X4\000003A9\X0\\\name\X\0A'"));
    assert!(text.parse::<Exchange>().is_ok());
}

fn ring(value: &serde_json::Value) -> Vec<Vec2> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|point| Vec2 {
            x: point[0].as_f64().unwrap(),
            y: point[1].as_f64().unwrap(),
        })
        .collect()
}

#[test]
fn current_profiles_export_all_concavity_holes_and_separate_regions() {
    let concave: serde_json::Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/rotated-concave.json"
    ))
    .unwrap();
    let holes: serde_json::Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/multi-hole-region.json"
    ))
    .unwrap();
    let split: serde_json::Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/split-regions.json"
    ))
    .unwrap();
    let cases = [
        (
            ProfileKind::Tray,
            vec![ProfileRegion {
                outer: ring(&concave["outer"]),
                holes: vec![],
            }],
            vec![81000.0],
        ),
        (
            ProfileKind::Plate,
            vec![ProfileRegion {
                outer: ring(&holes["outer"]),
                holes: holes["holes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(ring)
                    .collect(),
            }],
            vec![19280.0],
        ),
        (
            ProfileKind::Lid,
            split["islands"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| ProfileRegion {
                    outer: ring(r),
                    holes: vec![],
                })
                .collect(),
            vec![35000.0, 35000.0],
        ),
    ];
    for (kind, mut regions, expected_volumes) in cases {
        for region in &mut regions {
            region.outer.reverse();
            for hole in &mut region.holes {
                hole.reverse();
            }
        }
        let model = build_profile(ProfileInput {
            kind,
            base_z: 4.0,
            thickness: 2.0,
            wall_height: 3.0,
            regions,
        })
        .unwrap();
        let bytes = export_step(&model).unwrap();
        let solids = crate::step_accuracy::inspect(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(solids.len(), expected_volumes.len());
        for (solid, expected) in solids.iter().zip(expected_volumes) {
            assert!(
                (solid.volume - expected).abs() <= 0.1,
                "{} != {expected}",
                solid.volume
            );
        }
        if kind == ProfileKind::Plate {
            assert_eq!(
                solids[0].faces, 146,
                "all35 through-holes retain their side faces"
            );
            assert!(!solids[0].contains([15.0, 15.0, 5.0]));
        }
    }
}

#[test]
fn curved_exact_solids_are_written_and_never_measured_as_vertex_chords() {
    use monstertruck_modeling::{Point3, Solid, Vector3, builder};
    let vertex = builder::vertex(Point3::new(2.0, 0.0, 0.0));
    let circle = builder::revolve(
        &vertex,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_z(),
        builder::SweepAngle::Closed,
        2,
    );
    let disk = builder::try_attach_plane(&[circle]).unwrap();
    let solid: Solid = builder::extrude(&disk, Vector3::new(0.0, 0.0, 3.0));
    let bytes = export_step(&crate::ExactModel {
        solids: vec![solid],
    })
    .unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("RATIONAL_B_SPLINE_CURVE") || text.contains("CIRCLE("));
    assert!(!text.contains("TRIANGULATED"));
    assert!(text.parse::<Exchange>().is_ok());
    assert!(matches!(
        crate::step_accuracy::inspect(text),
        Err(crate::step_accuracy::Error::Unsupported)
    ));
}

#[test]
fn raw_accuracy_binds_actual_representation_context() {
    use crate::step_accuracy::{Error, inspect};
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    let broken = original.replace("(#11,#15),#345)", "(#11,#15),#999999)");
    assert_ne!(broken, original);
    assert!(matches!(
        inspect(&broken),
        Err(Error::Units | Error::Topology)
    ));
}

#[test]
fn raw_accuracy_conditions_volume_on_world_placement() {
    let angle: f64 = 0.3;
    let outer = [(0.0, 0.0), (2.0, 0.0), (2.0, 3.0), (0.0, 3.0)]
        .map(|(x, y)| Vec2 {
            x: 1000000.1 + x * angle.cos() - y * angle.sin(),
            y: 1000000.2 + x * angle.sin() + y * angle.cos(),
        })
        .into();
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 1000000.3,
        thickness: 4.0,
        wall_height: 0.0,
        regions: vec![ProfileRegion {
            outer,
            holes: vec![],
        }],
    })
    .unwrap();
    let text = String::from_utf8(export_step(&model).unwrap()).unwrap();
    let solids = crate::step_accuracy::inspect(&text).expect("valid rotated placed box");
    assert!((solids[0].volume - 24.0).abs() <= 0.1);
}

fn append_step_records(step: &str, records: &str) -> String {
    let marker = "ENDSEC;\nEND-ISO-10303-21;";
    let normalized = step.replace("\r\n", "\n");
    assert_eq!(normalized.matches(marker).count(), 1);
    normalized.replace(marker, &format!("{records}\n{marker}"))
}

#[test]
fn raw_accuracy_ignores_unrelated_units_but_rejects_ambiguous_owners() {
    use crate::step_accuracy::{Error, inspect};
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    let extra = "#9000=(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNIT_ASSIGNED_CONTEXT((#9001)) REPRESENTATION_CONTEXT('unused','3D'));\n#9001=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));";
    let unrelated = append_step_records(original, extra);
    assert!((inspect(&unrelated).unwrap()[0].volume - 1000.0).abs() <= 0.1);
    let duplicate_owner =
        append_step_records(original, "#9000=SHAPE_DEFINITION_REPRESENTATION(#4,#10);");
    assert!(matches!(inspect(&duplicate_owner), Err(Error::Units)));
    assert_eq!(unrelated.matches("(#346,#347,#348)").count(), 1);
    let ambiguous = unrelated.replace("(#346,#347,#348)", "(#346,#347,#348,#9001)");
    assert!(matches!(inspect(&ambiguous), Err(Error::Units)));
}

#[test]
fn raw_accuracy_measures_each_product_in_its_own_length_context() {
    use crate::{StepBody, export_step_bodies};
    use step_p21::ast::{Name, Parameter};
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 1.0,
        wall_height: 0.0,
        regions: vec![rectangle(0.0, 0.0, 1.0, 1.0)],
    })
    .unwrap();
    let text = String::from_utf8(
        export_step_bodies(&[
            StepBody {
                id: "mm",
                name: "millimetre",
                model: &model,
            },
            StepBody {
                id: "m",
                name: "metre",
                model: &model,
            },
        ])
        .unwrap(),
    )
    .unwrap();
    let parsed: Exchange = text.parse().unwrap();
    let representations = parsed
        .data
        .iter()
        .flat_map(|s| &s.entities)
        .filter_map(|e| match e {
            EntityInstance::Simple { id, record } if record.name == "SHAPE_REPRESENTATION" => {
                Some((*id, &record.parameter))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let (id, Parameter::List(params)) = representations[1] else {
        panic!("representation")
    };
    let Parameter::Ref(Name::Entity(old_context)) = &params[2] else {
        panic!("context")
    };
    let prefix = format!("#{id} = SHAPE_REPRESENTATION");
    let rewritten = text
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                let old = format!(", #{old_context});");
                assert!(line.ends_with(&old));
                line.strip_suffix(&old).unwrap().to_owned() + ", #9000);"
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let mixed = append_step_records(
        &rewritten,
        "#9000=(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNIT_ASSIGNED_CONTEXT((#9001,#4,#5)) REPRESENTATION_CONTEXT('metre','3D'));\n#9001=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));",
    );
    let solids = crate::step_accuracy::inspect(&mixed).unwrap();
    assert!((solids[0].volume - 1.0).abs() < 1.0e-8);
    assert!((solids[1].volume - 1.0e9).abs() < 0.1);
    assert_eq!(solids[0].bounds, [[0.0; 3], [1.0; 3]]);
    assert_eq!(solids[1].bounds, [[0.0; 3], [1000.0; 3]]);
}

#[test]
fn raw_accuracy_does_not_measure_orphan_or_unresolved_nested_solids() {
    use crate::step_accuracy::{Error, inspect};
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    let orphan = original.replace("(#11,#15),#345)", "(#11),#345)");
    assert_ne!(orphan, original);
    assert!(matches!(inspect(&orphan), Err(Error::Topology)));
    let dangling = original.replace("(#11,#15),#345)", "(#11,#999999),#345)");
    assert!(matches!(inspect(&dangling), Err(Error::Topology)));
    let nested = append_step_records(
        &original.replace("(#11,#15),#345)", "(#11,#9000),#345)"),
        "#9000=MAPPED_ITEM('',#9001,#9002);\n#9001=REPRESENTATION_MAP(#11,#9003);\n#9002=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#13,#14,#12,1.,#13);\n#9003=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#15),#999999);",
    );
    assert!(matches!(inspect(&nested), Err(Error::Unsupported)));
    let broken_product = original.replace(
        "SHAPE_DEFINITION_REPRESENTATION(#4,#10)",
        "SHAPE_DEFINITION_REPRESENTATION(#999999,#10)",
    );
    assert!(matches!(inspect(&broken_product), Err(Error::Topology)));
}

#[test]
fn raw_accuracy_retains_external_cube_volume_at_large_world_offsets() {
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    for offset in [0.0, 1.0e6, 1.0e9] {
        let translated = map_points(original, |p| p.map(|v| v + offset));
        let solids = crate::step_accuracy::inspect(&translated).unwrap();
        assert!((solids[0].volume - 1000.0).abs() <= 0.1);
        assert_eq!(solids[0].bounds, [[offset; 3], [offset + 10.0; 3]]);
    }
}

#[test]
fn raw_accuracy_refuses_cyclic_and_dangling_unit_conversions() {
    use crate::step_accuracy::{Error, inspect};
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    let conversion = original.replace(
        "SI_UNIT(.MILLI.,.METRE.)",
        "CONVERSION_BASED_UNIT('cycle',#9001)",
    );
    let cyclic = append_step_records(
        &conversion,
        "#9001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#346);",
    );
    assert!(matches!(inspect(&cyclic), Err(Error::Units)));
    let dangling = conversion.replace("#9001", "#999999");
    assert!(matches!(inspect(&dangling), Err(Error::Topology)));
}

#[test]
fn raw_accuracy_checks_authored_plane_pcurves_against_edge_geometry() {
    use crate::step_accuracy::{Error, inspect};
    let original = include_str!("../tests/fixtures/freecad-cube.step");
    let shifted = original.replace(
        "#39 = CARTESIAN_POINT('',(0.,0.));",
        "#39 = CARTESIAN_POINT('',(0.,0.01));",
    );
    assert_ne!(shifted, original);
    assert!(matches!(inspect(&shifted), Err(Error::Geometry)));
    let curve_master = shifted.replace(".PCURVE_S1.", ".CURVE_3D.");
    assert!(
        matches!(inspect(&curve_master), Err(Error::Geometry)),
        "an inconsistent alternate representation is not discarded"
    );
    let second_master = original.replace(
        "#26 = SURFACE_CURVE('',#27,(#31,#43),.PCURVE_S1.);",
        "#26 = SURFACE_CURVE('',#27,(#31,#43),.PCURVE_S2.);",
    );
    assert_ne!(second_master, original);
    assert!((inspect(&second_master).unwrap()[0].volume - 1000.0).abs() <= 0.1);
    let bad_second = second_master.replace(
        "#51 = CARTESIAN_POINT('',(0.,0.));",
        "#51 = CARTESIAN_POINT('',(0.,0.01));",
    );
    assert!(matches!(inspect(&bad_second), Err(Error::Geometry)));
    let invalid_master_type = original.replace(
        "#26 = SURFACE_CURVE('',#27,(#31,#43),.PCURVE_S1.);",
        "#26 = SURFACE_CURVE('',#27,(#32,#43),.PCURVE_S1.);",
    );
    assert!(matches!(
        inspect(&invalid_master_type),
        Err(Error::Unsupported)
    ));
    let missing_master = original.replace(".PCURVE_S1.", ".UNKNOWN_MASTER.");
    assert!(matches!(inspect(&missing_master), Err(Error::Unsupported)));
    assert!((inspect(original).unwrap()[0].volume - 1000.0).abs() <= 0.1);
    let default_ref = original.replace(
        "#73 = AXIS2_PLACEMENT_3D('',#74,#75,#76);",
        "#73 = AXIS2_PLACEMENT_3D('',#74,#75,$);",
    );
    assert_ne!(default_ref, original);
    assert!((inspect(&default_ref).unwrap()[0].volume - 1000.0).abs() <= 0.1);
}

#[test]
fn raw_accuracy_refuses_unresolved_assembly_occurrences() {
    use crate::step_accuracy::{Error, inspect};
    let placed = include_str!("../tests/fixtures/placed-tetra-occurrences.step");
    assert!(
        matches!(inspect(placed), Err(Error::Unsupported)),
        "two placed occurrences must not become one identity part definition"
    );
    assert!(matches!(
        crate::step_accuracy::product_solid_counts(placed),
        Err(Error::Unsupported)
    ));
    let direct = include_str!("../tests/fixtures/freecad-cube.step");
    for relationship in [
        "NEXT_ASSEMBLY_USAGE_OCCURRENCE('','','',#5,#5,$)",
        "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#10,#4)",
        "REPRESENTATION_RELATIONSHIP('','',#10,#10)",
        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#11)",
        "SHAPE_REPRESENTATION_RELATIONSHIP()",
        "ITEM_DEFINED_TRANSFORMATION('','',#11,#11)",
        "MAPPED_ITEM('',#10,#11)",
        "REPRESENTATION_MAP(#11,#10)",
        "CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#12,1.,$)",
        "UNREVIEWED_REPRESENTATION_EXTENSION(#10)",
    ] {
        let placed = append_step_records(direct, &format!("#9000={relationship};"));
        assert!(matches!(inspect(&placed), Err(Error::Unsupported)));
    }
    assert!((inspect(direct).unwrap()[0].volume - 1000.0).abs() <= 0.1);
}

fn circular_extrusion_step(shape: crate::step_accuracy::CircularExtrusion) -> String {
    use monstertruck_modeling::{Curve, Point3, Vector3, primitive, profile};
    let circle = primitive::circle::<Curve>(
        Point3::new(shape.center[0] + shape.radius, shape.center[1], shape.z[0]),
        Point3::new(shape.center[0], shape.center[1], shape.z[0]),
        Vector3::unit_z(),
        2,
    );
    let solid = profile::solid_from_planar_profile::<Curve, monstertruck_modeling::Surface>(
        vec![circle],
        Vector3::new(0.0, 0.0, shape.z[1] - shape.z[0]),
    )
    .unwrap();
    String::from_utf8(
        export_step(&crate::ExactModel {
            solids: vec![solid],
        })
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn exact_circle_extrusion_has_independent_volume_material_and_topology_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let text = circular_extrusion_step(CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 3.0],
    });
    let certificate = certify_circular_extrusion(
        &text,
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 2.0,
            z: [0.0, 3.0],
        },
    )
    .expect("independent complete circular-extrusion source certificate");
    let analytic_volume = 12.0 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 1.0e-5);
    assert_eq!(certificate.bounds, [[-2.0, -2.0, 0.0], [2.0, 2.0, 3.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.topology, [4, 6, 4]);
    assert_eq!(certificate.contains([0.0, 0.0, 1.0]), Some(true));
    assert_eq!(certificate.contains([1.0, 1.0, 2.0]), Some(true));
    assert_eq!(certificate.contains([1.5, 1.5, 1.0]), Some(false));
    assert_eq!(certificate.contains([0.0, 0.0, 3.1]), Some(false));
    assert_eq!(certificate.contains([2.0, 0.0, 1.0]), None);
}

#[test]
fn circular_certificate_checks_interior_source_weights_and_complete_trim_topology() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 3.0],
    };
    let original = circular_extrusion_step(shape);
    let distort_interior = map_points(&original, |mut p| {
        if (p[1] - 2.0 / 3.0).abs() < 1.0e-12 {
            p[1] += 0.01;
        }
        p
    });
    assert_ne!(distort_interior, original);
    assert!(certify_circular_extrusion(&distort_interior, shape).is_err());
    for (old, new) in [
        (
            "RATIONAL_B_SPLINE_CURVE((1.0, 0.75",
            "RATIONAL_B_SPLINE_CURVE((1.0, 0.76",
        ),
        (
            "RATIONAL_B_SPLINE_SURFACE(((1.0, 1.0), (0.75, 0.75)",
            "RATIONAL_B_SPLINE_SURFACE(((1.0, 1.0), (0.75, 0.76)",
        ),
        (
            "CLOSED_SHELL('', (#21, #26, #33, #40))",
            "CLOSED_SHELL('', (#21, #26, #33))",
        ),
        (
            "ORIENTED_EDGE('', *, *, #45, .T.)",
            "ORIENTED_EDGE('', *, *, #45, .F.)",
        ),
        (".CURVE_3D.", ".PCURVE_S1."),
        (
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#3, #4, #5))",
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#999999, #4, #5))",
        ),
    ] {
        assert!(
            original.contains(old),
            "negative control must edit actual source: {old}"
        );
        let bad = original.replacen(old, new, 1);
        assert!(certify_circular_extrusion(&bad, shape).is_err(), "{old}");
    }
    let extra_graph = append_step_records(
        &original,
        "#9000=REPRESENTATION_RELATIONSHIP('','',#18,#18);",
    );
    assert!(certify_circular_extrusion(&extra_graph, shape).is_err());
    let wrong_radius = CircularExtrusion {
        radius: 1.9,
        ..shape
    };
    assert!(certify_circular_extrusion(&original, wrong_radius).is_err());
    let wrong_height = CircularExtrusion {
        z: [0.0, 3.01],
        ..shape
    };
    assert!(certify_circular_extrusion(&original, wrong_height).is_err());
}

#[test]
fn circular_certificate_preserves_fractional_translated_circle_geometry() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [11.25, -7.5],
        radius: 1.25,
        z: [-1.0, 1.5],
    };
    let text = circular_extrusion_step(shape);
    let certificate = certify_circular_extrusion(&text, shape).unwrap();
    let analytic_volume = 3.90625 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.1);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([11.25, -7.5, 0.0]), Some(true));
    assert_eq!(certificate.contains([12.25, -6.5, 0.0]), Some(false));
}

#[test]
fn external_freecad_cylinder_has_independent_analytic_and_selected_chart_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../tests/fixtures/freecad-cylinder.step");
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 10.0],
    };
    let certificate =
        certify_circular_extrusion(source, shape).expect("external analytic cylinder source");
    let analytic_volume = 40.0 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.1);
    assert_eq!(certificate.topology, [2, 3, 3]);
    assert_eq!(certificate.bounds, [[-2.0, -2.0, 0.0], [2.0, 2.0, 10.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([0.0, 0.0, 5.0]), Some(true));
    assert_eq!(certificate.contains([1.5, 1.5, 5.0]), Some(false));
    let reparametrized = source.replace(
        "#38 = CARTESIAN_POINT('',(0.,10.));",
        "#38 = CARTESIAN_POINT('',(13.4,10.));",
    );
    assert_ne!(reparametrized, source);
    assert!(certify_circular_extrusion(&reparametrized, shape).is_ok());
    let second_master = source.replace(".PCURVE_S1.", ".PCURVE_S2.");
    assert_ne!(second_master, source);
    assert!(certify_circular_extrusion(&second_master, shape).is_ok());
    let angle_units = source.replace("SI_UNIT($,.RADIAN.)", "SI_UNIT($,.DEGREE.)");
    assert_ne!(angle_units, source);
    assert!(certify_circular_extrusion(&angle_units, shape).is_err());
    for (old, new) in [
        (
            "#38 = CARTESIAN_POINT('',(0.,10.));",
            "#38 = CARTESIAN_POINT('',(0.,10.01));",
        ),
        (
            "#66 = CARTESIAN_POINT('',(6.28318530718,-0.));",
            "#66 = CARTESIAN_POINT('',(6.28,-0.));",
        ),
        (
            "#51 = CARTESIAN_POINT('',(0.,0.));",
            "#51 = CARTESIAN_POINT('',(0.01,0.));",
        ),
        (
            "#31 = CYLINDRICAL_SURFACE('',#32,2.);",
            "#31 = CYLINDRICAL_SURFACE('',#32,2.01);",
        ),
    ] {
        assert!(source.contains(old));
        let bad = source.replacen(old, new, 1);
        assert!(certify_circular_extrusion(&bad, shape).is_err(), "{old}");
    }
}

#[test]
fn circular_certificate_refuses_malformed_source_records_without_panicking() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../tests/fixtures/freecad-cylinder.step");
    let duplicate = source.replace(
        "#25 = CIRCLE('',#26,2.);",
        "#25 = (CIRCLE('',#26,2.)CIRCLE('',#26,3.));",
    );
    assert_ne!(duplicate, source);
    assert!(
        certify_circular_extrusion(
            &duplicate,
            CircularExtrusion {
                center: [0.0, 0.0],
                radius: 2.0,
                z: [0.0, 10.0],
            }
        )
        .is_err(),
        "conflicting geometry records must not hide behind the first matching record"
    );
    let conflicting_types = source.replace(
        "#25 = CIRCLE('',#26,2.);",
        "#25 = (CIRCLE('',#26,2.)PCURVE('',#31,#36));",
    );
    assert_ne!(conflicting_types, source);
    assert!(
        certify_circular_extrusion(
            &conflicting_types,
            CircularExtrusion {
                center: [0.0, 0.0],
                radius: 2.0,
                z: [0.0, 10.0],
            }
        )
        .is_err()
    );
    let malformed = source.replace("CYLINDRICAL_SURFACE('',#32,2.)", "CYLINDRICAL_SURFACE()");
    assert_ne!(malformed, source);
    assert!(
        certify_circular_extrusion(
            &malformed,
            CircularExtrusion {
                center: [0.0, 0.0],
                radius: 2.0,
                z: [0.0, 10.0],
            }
        )
        .is_err()
    );
}

#[test]
fn original_standoff_keeps_authored_96_gon_drill_and_existing_tolerances() {
    use crate::{StepBody, export_step_bodies};
    let polygon = |radius: f64| -> Vec<Vec2> {
        (0..96)
            .map(|index| {
                let angle = index as f64 / 96.0 * std::f64::consts::TAU;
                Vec2 {
                    x: radius * angle.cos(),
                    y: radius * angle.sin(),
                }
            })
            .collect()
    };
    let model = |drill| {
        build_profile(ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 0.0,
            thickness: 3.0,
            wall_height: 0.0,
            regions: vec![ProfileRegion {
                outer: polygon(3.0),
                holes: vec![polygon(drill)],
            }],
        })
        .unwrap()
    };
    let correct = model(1.4);
    let bytes = export_step_bodies(&[StepBody {
        id: "module-standoff/module-1/mh1",
        name: "PCB standoff",
        model: &correct,
    }])
    .unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let solids = crate::step_accuracy::inspect(text).unwrap();
    let want = expected("module-standoff-annulus");
    assert_eq!(
        solids.len(),
        want["expect"]["solids"].as_u64().unwrap() as usize
    );
    let amount = &want["expect"]["volume"];
    assert!(
        (solids[0].volume - amount["value"].as_f64().unwrap()).abs()
            <= amount["tolerance"].as_f64().unwrap()
    );
    assert!((solids[0].volume - want["baseline"]["volume"].as_f64().unwrap()).abs() <= 0.1);
    let bounds = &want["expect"]["bounds"];
    for (i, key) in ["min", "max"].into_iter().enumerate() {
        for axis in 0..3 {
            assert!(
                (solids[0].bounds[i][axis] - bounds[key][axis].as_f64().unwrap()).abs()
                    <= bounds["tolerance"].as_f64().unwrap()
            );
        }
    }
    assert_eq!(
        solids[0].faces, 194,
        "all96 outer and96 inner planar sides survive"
    );
    assert!(!solids[0].contains([0.0, 0.0, 1.5]));
    assert!(solids[0].contains([2.0, 0.0, 1.5]));
    assert!(!text.contains("B_SPLINE_CURVE(") && !text.contains("CIRCLE("));
    assert_eq!(
        crate::step_accuracy::product_solid_counts(text).unwrap(),
        vec![(
            "module-standoff/module-1/mh1".into(),
            "PCB standoff".into(),
            1
        )]
    );
    let edited = export_step(&model(1.3)).unwrap();
    let edited = crate::step_accuracy::inspect(std::str::from_utf8(&edited).unwrap()).unwrap();
    assert!(
        (edited[0].volume - amount["value"].as_f64().unwrap()).abs()
            > amount["tolerance"].as_f64().unwrap(),
        "a valid but wrong authored drill cannot reuse the original geometry"
    );
}

#[test]
fn original_holed_and_large_plate_manifests_keep_all_authored_holes() {
    use crate::{StepBody, export_step_bodies};
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../test/fixtures/manifest.json")).unwrap();
    let followup: serde_json::Value =
        serde_json::from_str(include_str!("../../test/fixtures/followup-manifest.json")).unwrap();
    for (id, input, expected_holes) in [
        (
            "holed-plate",
            include_str!("../../test/fixtures/holed-plate.json"),
            24,
        ),
        (
            "large-plate",
            include_str!("../../test/fixtures/large-plate.json"),
            96,
        ),
    ] {
        let input: serde_json::Value = serde_json::from_str(input).unwrap();
        let bodies = input["bodies"].as_array().unwrap();
        assert_eq!(bodies.len(), 1);
        let body = &bodies[0]["body"];
        assert_eq!(body["kind"], "plate");
        assert_eq!(body["clearance"].as_f64().unwrap(), 0.0);
        let contours = bodies[0]["contours"].as_array().unwrap();
        let points = |contour: &serde_json::Value| -> Vec<Vec2> {
            contour["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| Vec2 {
                    x: p["x"].as_f64().unwrap(),
                    y: p["y"].as_f64().unwrap(),
                })
                .collect()
        };
        let outers: Vec<_> = contours.iter().filter(|c| c["hole"] == false).collect();
        assert_eq!(outers.len(), 1);
        let holes: Vec<_> = contours
            .iter()
            .filter(|c| c["hole"] == true)
            .map(points)
            .collect();
        assert_eq!(holes.len(), expected_holes);
        let model = build_profile(ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 0.0,
            thickness: body["thickness"].as_f64().unwrap(),
            wall_height: 0.0,
            regions: vec![ProfileRegion {
                outer: points(outers[0]),
                holes,
            }],
        })
        .unwrap();
        let bytes = export_step_bodies(&[StepBody {
            id: body["id"].as_str().unwrap(),
            name: body["name"].as_str().unwrap(),
            model: &model,
        }])
        .unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let solids = crate::step_accuracy::inspect(text).unwrap();
        let want = manifest["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .chain(followup["fixtures"].as_array().unwrap())
            .find(|f| f["id"] == id)
            .unwrap();
        assert_eq!(
            solids.len(),
            want["expectedSolids"].as_u64().unwrap() as usize
        );
        assert!((solids[0].volume - want["expectedVolume"].as_f64().unwrap()).abs() <= 0.1);
        for bound in 0..2 {
            for axis in 0..3 {
                assert!(
                    (solids[0].bounds[bound][axis]
                        - want["expectedBounds"][bound][axis].as_f64().unwrap())
                    .abs()
                        <= 0.01
                );
            }
        }
        assert_eq!(solids[0].faces, 6 + expected_holes * 4);
        for contour in contours.iter().filter(|c| c["hole"] == true) {
            let points = points(contour);
            let center = points.iter().fold([0.0, 0.0], |a, p| {
                [
                    a[0] + p.x / points.len() as f64,
                    a[1] + p.y / points.len() as f64,
                ]
            });
            assert!(!solids[0].contains([center[0], center[1], 1.0]));
        }
        assert!(solids[0].contains([1.0, 1.0, 1.0]));
        assert_eq!(
            crate::step_accuracy::product_solid_counts(text).unwrap(),
            vec![(
                body["id"].as_str().unwrap().into(),
                body["name"].as_str().unwrap().into(),
                1
            )]
        );
    }
}

#[test]
fn circular_certificate_checks_both_seam_endpoints_within_reported_bound() {
    use crate::step_accuracy::{CircularExtrusion, Error, certify_circular_extrusion};
    let broken = include_str!("../tests/fixtures/off-seam-top-vertex.step");
    assert!(
        matches!(
            certify_circular_extrusion(
                broken,
                CircularExtrusion {
                    center: [0.0, 0.0],
                    radius: 2.0,
                    z: [0.0, 10.0],
                }
            ),
            Err(Error::Geometry)
        ),
        "top seam vertex cannot exceed the reported physical bound"
    );

    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 10.0],
    };
    let within_bound = broken.replace("(2.,0.00000005,10.)", "(2.,0.00000000005,10.)");
    assert_ne!(within_bound, broken);
    let accepted = certify_circular_extrusion(&within_bound, shape).unwrap();
    assert_eq!(
        accepted.bound_error, 8.0e-10,
        "the physical bound was not enlarged"
    );
    let opposite = broken.replace("(2.,0.00000005,10.)", "(2.,-0.00000005,10.)");
    assert_ne!(opposite, broken);
    assert!(matches!(
        certify_circular_extrusion(&opposite, shape),
        Err(Error::Geometry)
    ));
    let original = include_str!("../tests/fixtures/freecad-cylinder.step");
    let bottom = original.replace(
        "#57 = CARTESIAN_POINT('',(2.,-4.898587196589E-16,0.));",
        "#57 = CARTESIAN_POINT('',(2.,0.00000005,0.));",
    );
    assert_ne!(bottom, original);
    assert!(matches!(
        certify_circular_extrusion(&bottom, shape),
        Err(Error::Geometry)
    ));
}

fn annular_extrusion_step(shape: crate::step_accuracy::CircularExtrusion, inner: f64) -> String {
    use monstertruck_modeling::{Curve, Point3, Vector3, primitive, profile};
    let circle = |radius| {
        primitive::circle::<Curve>(
            Point3::new(shape.center[0] + radius, shape.center[1], shape.z[0]),
            Point3::new(shape.center[0], shape.center[1], shape.z[0]),
            Vector3::unit_z(),
            2,
        )
    };
    let solid = profile::solid_from_planar_profile::<Curve, monstertruck_modeling::Surface>(
        vec![circle(shape.radius), circle(inner)],
        Vector3::new(0.0, 0.0, shape.z[1] - shape.z[0]),
    )
    .unwrap();
    String::from_utf8(
        export_step(&crate::ExactModel {
            solids: vec![solid],
        })
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn true_annulus_export_has_independent_curved_drill_volume_and_material() {
    use crate::step_accuracy::{CircularExtrusion, certify_annular_extrusion};
    let text = annular_extrusion_step(
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 3.0,
            z: [0.0, 3.0],
        },
        1.4,
    );
    let certificate = certify_annular_extrusion(
        &text,
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 3.0,
            z: [0.0, 3.0],
        },
        1.4,
    )
    .expect("complete independent true circular annulus source certificate");
    let analytic_volume = 21.12 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 1.0e-5);
    assert_eq!(certificate.topology, [8, 12, 6]);
    assert_eq!(certificate.bounds, [[-3.0, -3.0, 0.0], [3.0, 3.0, 3.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([0.0, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.3, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.45, 0.0, 1.5]), Some(true));
    assert_eq!(certificate.contains([3.1, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.4, 0.0, 1.5]), None);
}

#[test]
fn annular_certificate_rejects_filled_drill_wrong_winding_and_interior_drift() {
    use crate::step_accuracy::{
        CircularExtrusion, certify_annular_extrusion, certify_circular_extrusion,
    };
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 3.0,
        z: [0.0, 3.0],
    };
    let source = annular_extrusion_step(shape, 1.4);
    assert!(
        certify_circular_extrusion(&source, shape).is_err(),
        "an annulus is not a filled disk"
    );
    assert!(
        certify_annular_extrusion(&circular_extrusion_step(shape), shape, 1.4).is_err(),
        "a requested drill must exist in source"
    );
    assert!(
        certify_annular_extrusion(&annular_extrusion_step(shape, 1.3), shape, 1.4).is_err(),
        "wrong drill geometry must not reuse expected geometry"
    );
    let drift = map_points(&source, |mut point| {
        if (point[1] - 1.4 / 3.0).abs() < 1.0e-12 {
            point[1] += 0.01;
        }
        point
    });
    assert_ne!(drift, source);
    assert!(
        certify_annular_extrusion(&drift, shape, 1.4).is_err(),
        "interior drill curvature is checked beyond its endpoints"
    );
    for (old, new) in [
        (
            "#90 = DIRECTION('', (0.0, 0.0, 1.0));",
            "#90 = DIRECTION('', (0.0, 0.00000001, 1.0));",
        ),
        (
            "#89 = CARTESIAN_POINT('', (3.0, 3.0000000000000004, 0.0));",
            "#89 = CARTESIAN_POINT('', (3.0, 3.0000000000000004, 0.00000005));",
        ),
        (
            "ADVANCED_FACE('', (#45), #118, .F.)",
            "ADVANCED_FACE('', (#45), #118, .T.)",
        ),
        ("FACE_BOUND('', #27, .F.)", "FACE_BOUND('', #27, .T.)"),
    ] {
        assert!(source.contains(old));
        let broken = source.replacen(old, new, 1);
        assert!(certify_annular_extrusion(&broken, shape, 1.4).is_err());
    }

    let rebased = source.replace(
        "#89 = CARTESIAN_POINT('', (3.0, 3.0000000000000004, 0.0));",
        "#89 = CARTESIAN_POINT('', (0.0, 0.0, 0.0));",
    );
    assert_ne!(rebased, source);
    assert!(
        certify_annular_extrusion(&rebased, shape, 1.4).is_ok(),
        "an in-plane origin rebase preserves the cap geometry"
    );
    let centered_tilt = rebased.replace(
        "#90 = DIRECTION('', (0.0, 0.0, 1.0));",
        "#90 = DIRECTION('', (0.0, 0.00000001, 1.0));",
    );
    assert_ne!(centered_tilt, rebased);
    assert!(
        certify_annular_extrusion(&centered_tilt, shape, 1.4).is_err(),
        "the entire cap plane is checked, not just its height at the center"
    );
    let bounded_tilt = source.replace(
        "#90 = DIRECTION('', (0.0, 0.0, 1.0));",
        "#90 = DIRECTION('', (0.0, 0.000000000001, 1.0));",
    );
    assert_ne!(bounded_tilt, source);
    let bounded = certify_annular_extrusion(&bounded_tilt, shape, 1.4).unwrap();
    assert!(bounded.bound_error < 0.01);
    let analytic_volume = 21.12 * std::f64::consts::PI;
    assert!(bounded.volume[0] <= analytic_volume && bounded.volume[1] >= analytic_volume);
}

#[test]
fn annular_boss_component_keeps_legacy_analytic_volume_delta() {
    use crate::step_accuracy::{CircularExtrusion, certify_annular_extrusion};
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 3.0,
        z: [-7.2, -6.2],
    };
    let source = annular_extrusion_step(shape, 1.1);
    let certificate = certify_annular_extrusion(&source, shape, 1.1).unwrap();
    let amount = &expected("module-support-annular-boss")["expect"]["volumeDeltaVersusPlain"];
    let value = amount["value"].as_f64().unwrap();
    let tolerance = amount["tolerance"].as_f64().unwrap();
    assert!(
        (certificate.volume[0] - value).abs() <= tolerance
            && (certificate.volume[1] - value).abs() <= tolerance
    );
    assert_eq!(certificate.contains([0.0, 0.0, -6.5]), Some(false));
    assert_eq!(certificate.contains([2.0, 0.0, -6.5]), Some(true));
    assert_eq!(certificate.bounds, [[-3.0, -3.0, -7.2], [3.0, 3.0, -6.2]]);
}

#[test]
fn frozen_013_countersink_has_independent_raw_volume_material_and_complete_trims() {
    use crate::step_accuracy::{Countersink, certify_countersink};
    // Captured unchanged public writer output; dimensions are the frozen PreparedCaseIR workload.
    let text = include_str!("../tests/fixtures/countersink-013.step");
    let expected = Countersink {
        bounds: [[0., 0., 0.], [20., 16., 4.]],
        center: [10., 8.],
        z: [2., 4.],
        radii: [2., 1.],
    };
    let certificate = certify_countersink(text, expected).unwrap();
    assert_eq!(certificate.bounds, expected.bounds);
    let volume = 1265.3392342832476;
    assert!(certificate.volume[0] <= volume && certificate.volume[1] >= volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.topology, [16, 20, 8]);
    for (point, material) in [
        ([10., 8., 1.], true),
        ([10., 8., 3.], false),
        ([11.7, 8., 2.5], false),
        ([11.7, 8., 3.5], true),
        ([21., 8., 2.], false),
    ] {
        assert_eq!(certificate.contains(point), Some(material));
    }
}

#[test]
fn countersink_certificate_refuses_changed_tensor_trim_material_and_source_graph() {
    use crate::step_accuracy::{Countersink, certify_countersink};
    let source = include_str!("../tests/fixtures/countersink-013.step");
    let shape = Countersink {
        bounds: [[0., 0., 0.], [20., 16., 4.]],
        center: [10., 8.],
        z: [2., 4.],
        radii: [2., 1.],
    };
    for (old, new) in [
        (
            "#157 = CARTESIAN_POINT('', (12.0, 10.0, 2.0));",
            "#157 = CARTESIAN_POINT('', (12.0, 10.01, 2.0));",
        ),
        (
            "#278 = CARTESIAN_POINT('', (8.0, 8.666666666666666, 2.0));",
            "#278 = CARTESIAN_POINT('', (8.0, 8.676666666666666, 2.0));",
        ),
        (
            "#305 = CARTESIAN_POINT('', (8.0, 8.0, 2.0));",
            "#305 = CARTESIAN_POINT('', (8.0, 8.00000005, 2.0));",
        ),
        (
            "#231 = CARTESIAN_POINT('', (20.0, 0.0, 2.0));",
            "#231 = CARTESIAN_POINT('', (20.00000005, 0.0, 2.0));",
        ),
        (
            "#75 = ADVANCED_FACE('', (#76, #80), #155, .T.);",
            "#75 = ADVANCED_FACE('', (#76, #80), #155, .F.);",
        ),
        (
            "#86 = EDGE_LOOP('', (#87, #88));",
            "#86 = EDGE_LOOP('', (#87));",
        ),
        (
            "#275 = SURFACE_CURVE('', #276, (#155, #174), .CURVE_3D.);",
            "#275 = SURFACE_CURVE('', #276, (#155, #150), .CURVE_3D.);",
        ),
        (
            "#18 = SHAPE_REPRESENTATION('', (#7, #19), #2);",
            "#18 = SHAPE_REPRESENTATION('', (#7, #19), #999999);",
        ),
        (
            "#297 = CARTESIAN_POINT('', (0.0, 0.0, 4.0));",
            "#297 = CARTESIAN_POINT('', (0.01, 0.0, 4.0));",
        ),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        assert!(
            certify_countersink(&source.replacen(old, new, 1), shape).is_err(),
            "accepted {new}"
        );
    }
    // Equal row weights alone cannot certify the full conical tensor interior.
    let old = "0.7071067811865476";
    assert_eq!(source.matches(old).count(), 8);
    assert!(certify_countersink(&source.replace(old, "0.71"), shape).is_err());
    assert!(
        certify_countersink(
            source,
            Countersink {
                radii: [1., 2.],
                ..shape
            }
        )
        .is_err()
    );
    let valid = certify_countersink(source, shape).unwrap();
    assert_eq!(valid.contains([11.5, 8., 3.]), None);
    assert_eq!(valid.contains([10., 8., 2.]), None);
}

#[test]
fn countersink_raw_certificate_preserves_radii_ordered_by_authored_height() {
    use crate::step_accuracy::{Countersink, certify_countersink};
    // Raw geometry control only:013's input policy still requires a decreasing radius.
    let source = include_str!("../tests/fixtures/countersink-013.step");
    let mut changed = 0;
    let reversed = source
        .lines()
        .map(|line| {
            let Some((id, _)) = line
                .strip_prefix('#')
                .and_then(|line| line.split_once(" = "))
            else {
                return line.to_owned();
            };
            let id: u64 = id.parse().unwrap();
            if !(156..=173).contains(&id)
                && !(261..=274).contains(&id)
                && !(277..=290).contains(&id)
                && !(303..=306).contains(&id)
            {
                return line.to_owned();
            }
            if !line.contains("CARTESIAN_POINT") {
                return line.to_owned();
            }
            let values = line
                .split_once("('', (")
                .unwrap()
                .1
                .strip_suffix("));")
                .unwrap();
            let mut p: Vec<f64> = values.split(", ").map(|v| v.parse().unwrap()).collect();
            assert!(p[2] == 2. || p[2] == 4.);
            let scale = if p[2] == 2. { 0.5 } else { 2. };
            p[0] = 10. + scale * (p[0] - 10.);
            p[1] = 8. + scale * (p[1] - 8.);
            changed += 1;
            format!(
                "#{id} = CARTESIAN_POINT('', ({:?}, {:?}, {:?}));",
                p[0], p[1], p[2]
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert_eq!(changed, 46);
    let shape = Countersink {
        bounds: [[0., 0., 0.], [20., 16., 4.]],
        center: [10., 8.],
        z: [2., 4.],
        radii: [1., 2.],
    };
    let certificate = certify_countersink(&reversed, shape).unwrap();
    assert!(
        certificate.volume[0] <= 1265.3392342832476 && certificate.volume[1] >= 1265.3392342832476
    );
    assert_eq!(certificate.contains([11.7, 8., 2.5]), Some(true));
    assert_eq!(certificate.contains([11.7, 8., 3.5]), Some(false));
    assert!(
        certify_countersink(
            &reversed,
            Countersink {
                radii: [2., 1.],
                ..shape
            }
        )
        .is_err()
    );
}

#[test]
fn frozen_source_circle_periodic_cylinder_has_independent_8pi_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    // Authored source(center10,8,radius2,thickness2), not its diamond preview or built solid.
    let source = include_str!("../tests/fixtures/source-circle-006.step");
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    let certificate = certify_circular_extrusion(source, shape).unwrap();
    let expected = 25.132741228718345;
    assert!(certificate.volume[0] <= expected && certificate.volume[1] >= expected);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.bounds, [[8., 6., 0.], [12., 10., 2.]]);
    assert_eq!(certificate.topology, [4, 4, 3]);
    for (point, material) in [
        ([10., 8., 1.], true),
        ([11.3, 9.3, 1.], true),
        ([12.1, 8., 1.], false),
        ([10., 8., 2.1], false),
    ] {
        assert_eq!(certificate.contains(point), Some(material));
    }
    assert_eq!(certificate.contains([12., 8., 1.]), None);
}

#[test]
fn periodic_source_circle_refuses_geometry_and_directed_source_loss() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../tests/fixtures/source-circle-006.step");
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    for (old, new) in [
        (
            "#66 = CARTESIAN_POINT('', (8.0, 8.666666666666666, 0.0));",
            "#66 = CARTESIAN_POINT('', (8.0, 8.676666666666666, 0.0));",
        ),
        (
            "#53 = CYLINDRICAL_SURFACE('', #54, 2.0);",
            "#53 = CYLINDRICAL_SURFACE('', #54, 2.01);",
        ),
        (
            "#56 = DIRECTION('', (0.0, 0.0, 1.0));",
            "#56 = DIRECTION('', (0.0, 0.001, 1.0));",
        ),
        (
            "#55 = CARTESIAN_POINT('', (10.0, 8.0, 2.0));",
            "#55 = CARTESIAN_POINT('', (10.01, 8.0, 2.0));",
        ),
        (
            "#26 = ADVANCED_FACE('', (#27, #31), #53, .T.);",
            "#26 = ADVANCED_FACE('', (#27), #53, .T.);",
        ),
        (
            "#26 = ADVANCED_FACE('', (#27, #31), #53, .T.);",
            "#26 = ADVANCED_FACE('', (#27, #31), #53, .F.);",
        ),
        (
            "#21 = ADVANCED_FACE('', (#22), #48, .F.);",
            "#21 = ADVANCED_FACE('', (#22), #48, .T.);",
        ),
        (
            "#27 = FACE_BOUND('', #28, .T.);",
            "#27 = FACE_BOUND('', #28, .F.);",
        ),
        (
            "#28 = EDGE_LOOP('', (#29, #30));",
            "#28 = EDGE_LOOP('', (#29, #29));",
        ),
        (
            "#63 = SURFACE_CURVE('', #64, (#48, #53), .CURVE_3D.);",
            "#63 = SURFACE_CURVE('', #64, (#48, #58), .CURVE_3D.);",
        ),
        (
            "#63 = SURFACE_CURVE('', #64, (#48, #53), .CURVE_3D.);",
            "#63 = SURFACE_CURVE('', #64, (#48, #53), .PCURVE_S1.);",
        ),
        (
            "#60 = CARTESIAN_POINT('', (12.0, 10.0, 2.0));",
            "#60 = CARTESIAN_POINT('', (12.0, 10.0, 2.00000005));",
        ),
        (
            "#18 = SHAPE_REPRESENTATION('', (#7, #19), #2);",
            "#18 = SHAPE_REPRESENTATION('', (#7, #19), #999999);",
        ),
        ("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)"),
        ("SI_UNIT($,.RADIAN.)", "SI_UNIT($,.DEGREE.)"),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        assert!(
            certify_circular_extrusion(&source.replacen(old, new, 1), shape).is_err(),
            "accepted {new}"
        );
    }
    let weights = "RATIONAL_B_SPLINE_CURVE((1.0, 0.75, 0.5, 0.5, 0.75, 1.0))";
    assert_eq!(source.matches(weights).count(), 4);
    let drift = source.replace(
        weights,
        "RATIONAL_B_SPLINE_CURVE((1.0, 0.75, 0.51, 0.5, 0.75, 1.0))",
    );
    assert!(certify_circular_extrusion(&drift, shape).is_err());
    for expected in [
        CircularExtrusion {
            radius: 2.01,
            ..shape
        },
        CircularExtrusion {
            center: [10.01, 8.],
            ..shape
        },
        CircularExtrusion {
            z: [0., 2.01],
            ..shape
        },
    ] {
        assert!(certify_circular_extrusion(source, expected).is_err());
    }
    let scalar_drift = source.replace(
        "CYLINDRICAL_SURFACE('', #54, 2.0)",
        "CYLINDRICAL_SURFACE('', #54, 2.01)",
    );
    assert!(
        certify_circular_extrusion(
            &scalar_drift,
            CircularExtrusion {
                radius: 2.01,
                ..shape
            }
        )
        .is_err(),
        "a changed scalar cannot authorize old source arcs"
    );
    let end = "ENDSEC;\nEND-ISO-10303-21;";
    assert_eq!(source.matches(end).count(), 1);
    let extra_face = source.replace(
        end,
        &format!("#1000 = ADVANCED_FACE('', (#22), #48, .F.);\n{end}"),
    );
    assert!(certify_circular_extrusion(&extra_face, shape).is_err());
    let carrier = source
        .split("#64 = ")
        .nth(1)
        .unwrap()
        .split("#65 = ")
        .next()
        .unwrap();
    let extra_curve = source.replace(end, &format!("#1000 = {carrier}{end}"));
    assert!(
        certify_circular_extrusion(&extra_curve, shape).is_err(),
        "unused physical carrier cannot disappear"
    );
}

#[test]
fn periodic_source_circle_binds_both_endpoints_and_preserves_axial_parameter_origin() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../tests/fixtures/source-circle-006.step");
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    for (old, new) in [
        (
            "#95 = CARTESIAN_POINT('', (8.0, 8.0, 0.0));",
            "#95 = CARTESIAN_POINT('', (8.0, 8.00000005, 0.0));",
        ),
        (
            "#96 = CARTESIAN_POINT('', (12.0, 8.0, 0.0));",
            "#96 = CARTESIAN_POINT('', (12.0, 8.00000005, 0.0));",
        ),
        (
            "#97 = CARTESIAN_POINT('', (8.0, 8.0, 2.0));",
            "#97 = CARTESIAN_POINT('', (8.0, 8.00000005, 2.0));",
        ),
        (
            "#98 = CARTESIAN_POINT('', (12.0, 8.0, 2.0));",
            "#98 = CARTESIAN_POINT('', (12.0, 8.00000005, 2.0));",
        ),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        assert!(
            certify_circular_extrusion(&source.replacen(old, new, 1), shape).is_err(),
            "accepted {new}"
        );
    }
    let old = "#98 = CARTESIAN_POINT('', (12.0, 8.0, 2.0));";
    let within = source.replacen(
        old,
        "#98 = CARTESIAN_POINT('', (12.0, 8.00000000005, 2.0));",
        1,
    );
    let bounded = certify_circular_extrusion(&within, shape).unwrap();
    assert!(bounded.bound_error >= 5e-11 && bounded.bound_error < 1e-8);
    let outside = source.replacen(
        old,
        "#98 = CARTESIAN_POINT('', (12.0, 8.0000000002, 2.0));",
        1,
    );
    assert!(certify_circular_extrusion(&outside, shape).is_err());
    let rebased = source.replacen(
        "#55 = CARTESIAN_POINT('', (10.0, 8.0, 2.0));",
        "#55 = CARTESIAN_POINT('', (10.0, 8.0, 12345.0));",
        1,
    );
    let certificate = certify_circular_extrusion(&rebased, shape).unwrap();
    assert_eq!(certificate.bounds, [[8., 6., 0.], [12., 10., 2.]]);
    assert!(
        certificate.volume[0] <= 25.132741228718345 && certificate.volume[1] >= 25.132741228718345
    );
}

fn source_circle_trim_variant(changes: &[(&str, &str)], records: &str) -> String {
    let mut source = include_str!("../tests/fixtures/source-circle-006.step").to_owned();
    for &(old, new) in changes {
        assert_eq!(source.matches(old).count(), 1);
        source = source.replacen(old, new, 1);
    }
    append_step_records(&source, records)
}

#[test]
fn periodic_source_circle_refuses_retracing_and_revisited_vertices() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    let changed = source_circle_trim_variant(
        &[
            (
                r#"#23 = EDGE_LOOP('', (#24, #25));"#,
                r#"#23 = EDGE_LOOP('', (#24, #25, #1002, #1003));"#,
            ),
            (
                r#"#32 = EDGE_LOOP('', (#33, #34));"#,
                r#"#32 = EDGE_LOOP('', (#33, #34, #1004, #1005));"#,
            ),
        ],
        r#"#1000 = EDGE_CURVE('', #44, #45, #63, .T.);
#1001 = EDGE_CURVE('', #44, #45, #63, .T.);
#1002 = ORIENTED_EDGE('', *, *, #1000, .F.);
#1003 = ORIENTED_EDGE('', *, *, #1001, .T.);
#1004 = ORIENTED_EDGE('', *, *, #1000, .F.);
#1005 = ORIENTED_EDGE('', *, *, #1001, .T.);"#,
    );
    assert!(
        certify_circular_extrusion(&changed, shape).is_err(),
        "accepted reviewer-retraced-spur"
    );
    let changed = source_circle_trim_variant(
        &[
            (
                r#"#23 = EDGE_LOOP('', (#24, #25));"#,
                r#"#23 = EDGE_LOOP('', (#24, #25, #1002, #1003));"#,
            ),
            (
                r#"#32 = EDGE_LOOP('', (#33, #34));"#,
                r#"#32 = EDGE_LOOP('', (#33, #34, #1004, #1005));"#,
            ),
            (
                r#"#41 = EDGE_CURVE('', #44, #45, #71, .T.);"#,
                r#"#41 = EDGE_CURVE('', #44, #1007, #71, .T.);"#,
            ),
        ],
        r#"#1000 = EDGE_CURVE('', #1006, #1007, #63, .T.);
#1001 = EDGE_CURVE('', #1006, #45, #63, .T.);
#1002 = ORIENTED_EDGE('', *, *, #1000, .F.);
#1003 = ORIENTED_EDGE('', *, *, #1001, .T.);
#1004 = ORIENTED_EDGE('', *, *, #1000, .F.);
#1005 = ORIENTED_EDGE('', *, *, #1001, .T.);
#1006 = VERTEX_POINT('', #95);
#1007 = VERTEX_POINT('', #96);"#,
    );
    assert!(
        certify_circular_extrusion(&changed, shape).is_err(),
        "accepted aliased-retraced-spur"
    );
    let changed = source_circle_trim_variant(
        &[
            (
                r#"#23 = EDGE_LOOP('', (#24, #25));"#,
                r#"#23 = EDGE_LOOP('', (#24, #25, #1003));"#,
            ),
            (
                r#"#32 = EDGE_LOOP('', (#33, #34));"#,
                r#"#32 = EDGE_LOOP('', (#33, #34, #1004));"#,
            ),
        ],
        r#"#1000 = EDGE_CURVE('', #45, #45, #1001, .T.);
#1001 = SURFACE_CURVE('', #1002, (#48, #53), .CURVE_3D.);
#1003 = ORIENTED_EDGE('', *, *, #1000, .T.);
#1004 = ORIENTED_EDGE('', *, *, #1000, .T.);
#1005 = CARTESIAN_POINT('', (12.0, 8.0, 0.0));
#1006 = CARTESIAN_POINT('', (12.0, 8.000000000025, 0.0));
#1007 = CARTESIAN_POINT('', (12.0, 8.00000000005, 0.0));
#1002 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE(2, (#1005, #1006, #1007), .UNSPECIFIED., .U., .U.)
    B_SPLINE_CURVE_WITH_KNOTS((3, 3), (0.0, 1.0), .UNSPECIFIED.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE((1.0, 1.0, 1.0))
    REPRESENTATION_ITEM('')
);"#,
    );
    assert!(
        certify_circular_extrusion(&changed, shape).is_err(),
        "accepted same-direction-revisited-vertex"
    );
}

#[test]
fn periodic_source_circle_accepts_monotone_quarter_arc_subdivision() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    let changed = source_circle_trim_variant(
        &[
            (
                r#"#40 = EDGE_CURVE('', #44, #45, #63, .T.);"#,
                r#"#40 = EDGE_CURVE('', #44, #1000, #63, .T.);"#,
            ),
            (
                r#"#64 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE(2, (#65, #66, #67, #68, #69, #70), .UNSPECIFIED., .U., .U.)
    B_SPLINE_CURVE_WITH_KNOTS((3, 1, 1, 1, 3), (0.0, 0.25, 0.5, 0.75, 1.0), .UNSPECIFIED.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE((1.0, 0.75, 0.5, 0.5, 0.75, 1.0))
    REPRESENTATION_ITEM('')
);
"#,
                r#"#64 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE(2, (#1010, #1011, #1012), .UNSPECIFIED., .U., .U.)
    B_SPLINE_CURVE_WITH_KNOTS((3, 3), (0.0, 1.0), .UNSPECIFIED.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE((1.0, 0.7071067811865476, 1.0))
    REPRESENTATION_ITEM('')
);
"#,
            ),
            (
                r#"#23 = EDGE_LOOP('', (#24, #25));"#,
                r#"#23 = EDGE_LOOP('', (#24, #25, #1004));"#,
            ),
            (
                r#"#32 = EDGE_LOOP('', (#33, #34));"#,
                r#"#32 = EDGE_LOOP('', (#33, #34, #1005));"#,
            ),
        ],
        r#"#1000 = VERTEX_POINT('', #1012);
#1001 = EDGE_CURVE('', #1000, #45, #1002, .T.);
#1002 = SURFACE_CURVE('', #1003, (#48, #53), .CURVE_3D.);
#1004 = ORIENTED_EDGE('', *, *, #1001, .F.);
#1005 = ORIENTED_EDGE('', *, *, #1001, .F.);
#1010 = CARTESIAN_POINT('', (8.0, 8.0, 0.0));
#1011 = CARTESIAN_POINT('', (8.0, 10.0, 0.0));
#1012 = CARTESIAN_POINT('', (10.0, 10.0, 0.0));
#1013 = CARTESIAN_POINT('', (12.0, 10.0, 0.0));
#1014 = CARTESIAN_POINT('', (12.0, 8.0, 0.0));
#1003 = (
    BOUNDED_CURVE()
    B_SPLINE_CURVE(2, (#1012, #1013, #1014), .UNSPECIFIED., .U., .U.)
    B_SPLINE_CURVE_WITH_KNOTS((3, 3), (0.0, 1.0), .UNSPECIFIED.)
    CURVE()
    GEOMETRIC_REPRESENTATION_ITEM()
    RATIONAL_B_SPLINE_CURVE((1.0, 0.7071067811865476, 1.0))
    REPRESENTATION_ITEM('')
);"#,
    );

    let certificate = certify_circular_extrusion(&changed, shape).unwrap();
    assert_eq!(certificate.topology, [5, 5, 3]);
    assert_eq!(certificate.bounds, [[8., 6., 0.], [12., 10., 2.]]);
    assert!(
        certificate.volume[0] <= 25.132741228718345 && certificate.volume[1] >= 25.132741228718345
    );
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.contains([11.3, 9.3, 1.]), Some(true));
    assert_eq!(certificate.contains([12.1, 8., 1.]), Some(false));
}

// Actual Core-authored PCB exports, measured from the emitted Part21 bytes by the raw prism
// certificate. Expected values come from the source oracle or the authored slot, never from the
// kernel mesh, and the certificate shares no code with the Monstertruck exporter.
mod pcb_prism {
    use crate::step_accuracy::{Error, certify_prisms};
    use crate::{StepBody, build_case, export_step, export_step_bodies};
    use boardstudio_contracts::PreparedCaseIR;

    const VIK_ORIGIN: [f64; 2] = [81.584194, 110.727271];

    fn decimal(value: &serde_json::Value) -> f64 {
        value.as_str().unwrap().parse().unwrap()
    }

    fn prepared(json: &str) -> PreparedCaseIR {
        serde_json::from_str(json).unwrap()
    }

    fn vik() -> PreparedCaseIR {
        prepared(include_str!(
            "../tests/fixtures/pcb-source/vik-splitter-prepared.json"
        ))
    }

    fn slot() -> PreparedCaseIR {
        prepared(include_str!(
            "../tests/fixtures/pcb-source/back-offset-slot-prepared.json"
        ))
    }

    fn step(input: &PreparedCaseIR) -> String {
        let model = build_case(input).expect("actual Core PrepareCase fixture");
        String::from_utf8(export_step(&model).unwrap()).unwrap()
    }

    fn oracle() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../tests/fixtures/pcb-source/vik-splitter-expectations.json"
        ))
        .unwrap()
    }

    fn within(interval: [f64; 2], value: f64) {
        assert!(
            interval[0] <= value && value <= interval[1],
            "{value} outside {interval:?}"
        );
    }

    fn vik_cylinders(oracle: &serde_json::Value) -> Vec<([f64; 2], f64)> {
        let place = |center: &serde_json::Value| {
            [
                decimal(&center[0]) - VIK_ORIGIN[0],
                VIK_ORIGIN[1] - decimal(&center[1]),
            ]
        };
        oracle["arc_supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arc| (place(&arc["center_kicad"]), decimal(&arc["radius_mm"])))
            .chain(
                oracle["round_drills"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|drill| {
                        (
                            place(&drill["center_kicad_mm"]),
                            decimal(&drill["radius_mm"]),
                        )
                    }),
            )
            .collect()
    }

    #[test]
    fn vik_splitter_bytes_match_the_source_oracle_volume_bounds_material_and_trims() {
        let oracle = oracle();
        let text = step(&vik());
        let certificates = certify_prisms(&text).expect("independent raw prism certificate");
        assert_eq!(certificates.len(), 1);
        let c = &certificates[0];
        let thickness = decimal(&oracle["thickness_mm"]);
        within(c.volume, decimal(&oracle["volume_mm3"]));
        assert!(c.volume[1] - c.volume[0] < 2.0e-6, "{:?}", c.volume);
        within(
            c.area,
            decimal(&oracle["outline_area_mm2"]) - decimal(&oracle["drill_area_mm2"]),
        );
        assert_eq!(c.holes, 38);
        assert_eq!(c.planar_walls, 4);
        assert_eq!(c.topology[2], 48);
        // Every authored arc support and drill is exactly one declared cylinder, no others.
        let mut actual = c.cylinders.clone();
        for (center, radius) in vik_cylinders(&oracle) {
            let at = actual
                .iter()
                .position(|(c, r)| {
                    (r - radius).abs() < 5e-12
                        && (c[0] - center[0]).abs() < 5e-12
                        && (c[1] - center[1]).abs() < 5e-12
                })
                .unwrap_or_else(|| panic!("missing cylinder {center:?} r={radius}"));
            actual.swap_remove(at);
        }
        assert!(actual.is_empty(), "unexpected cylinders {actual:?}");
        let source = oracle["source_bounds_kicad_mm"].as_array().unwrap();
        let expected = [
            [
                decimal(&source[0][0]) - VIK_ORIGIN[0],
                VIK_ORIGIN[1] - decimal(&source[1][1]),
                0.0,
            ],
            [
                decimal(&source[1][0]) - VIK_ORIGIN[0],
                VIK_ORIGIN[1] - decimal(&source[0][1]),
                thickness,
            ],
        ];
        for (actual, expected) in c.bounds.iter().flatten().zip(expected.iter().flatten()) {
            assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
        }
        for drill in oracle["round_drills"].as_array().unwrap() {
            let x = decimal(&drill["center_kicad_mm"][0]) - VIK_ORIGIN[0];
            let y = VIK_ORIGIN[1] - decimal(&drill["center_kicad_mm"][1]);
            assert_eq!(c.contains([x, y, 0.8]), Some(false));
        }
        for (point, material) in [
            ([0.0, 0.0, 0.8], true),
            ([13.0, 0.0, 0.8], false),
            ([0.0, 0.0, 1.7], false),
            ([0.0, 0.0, -0.1], false),
        ] {
            assert_eq!(c.contains(point), Some(material), "{point:?}");
        }
        assert_eq!(c.contains([0.0, 0.0, 0.0]), None);
        assert_eq!(c.contains([0.0, 0.0, thickness]), None);
        assert!(
            crate::step_accuracy::inspect(&text).is_err(),
            "the planar-only checker must still refuse curved walls"
        );
    }

    #[test]
    fn reflected_offset_slot_bytes_match_the_authored_stadium_exactly() {
        let input = slot();
        let c = &certify_prisms(&step(&input)).unwrap()[0];
        let stadium = 0.32 + 0.16 * std::f64::consts::PI;
        within(c.area, 400.0 - stadium);
        within(c.volume, input.body.thickness * (400.0 - stadium));
        assert_eq!((c.holes, c.planar_walls, c.topology[2]), (1, 6, 10));
        let center = [2.2207122724188775, -5.961787043169324];
        let (sin, cos) = 14_f64.to_radians().sin_cos();
        assert_eq!(c.cylinders.len(), 2);
        for sign in [-1.0, 1.0] {
            let expected = [center[0] + sign * cos * 0.2, center[1] + sign * sin * 0.2];
            assert_eq!(
                c.cylinders
                    .iter()
                    .filter(|(at, radius)| (radius - 0.4).abs() < 2e-14
                        && (at[0] - expected[0]).abs() < 2e-14
                        && (at[1] - expected[1]).abs() < 2e-14)
                    .count(),
                1
            );
        }
        for (point, material) in [
            ([center[0], center[1], 1.0], false),
            ([center[0] + sin * 0.38, center[1] - cos * 0.38, 1.0], false),
            ([center[0] - sin * 0.43, center[1] + cos * 0.43, 1.0], true),
            ([9.0, 9.0, 1.0], true),
        ] {
            assert_eq!(c.contains(point), Some(material), "{point:?}");
        }
        for (actual, expected) in c
            .bounds
            .iter()
            .flatten()
            .zip([-10., -10., 0., 10., 10., 1.6])
        {
            assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
        }
    }

    #[test]
    fn multi_body_export_certifies_each_product_in_body_order() {
        let (a, b) = (build_case(&vik()).unwrap(), build_case(&slot()).unwrap());
        let text = String::from_utf8(
            export_step_bodies(&[
                StepBody {
                    id: "vik",
                    name: "Vik splitter",
                    model: &a,
                },
                StepBody {
                    id: "slot",
                    name: "Slot",
                    model: &b,
                },
            ])
            .unwrap(),
        )
        .unwrap();
        let products = crate::step_accuracy::product_solid_counts(&text);
        let certificates = certify_prisms(&text).unwrap();
        assert_eq!(certificates.len(), 2);
        within(certificates[0].volume, decimal(&oracle()["volume_mm3"]));
        within(
            certificates[1].volume,
            1.6 * (400.0 - 0.32 - 0.16 * std::f64::consts::PI),
        );
        assert_eq!(
            products.unwrap(),
            [
                ("vik".to_string(), "Vik splitter".to_string(), 1),
                ("slot".to_string(), "Slot".to_string(), 1)
            ]
        );
    }

    #[test]
    fn source_circle_and_half_disc_keep_periodic_and_line_arc_trims() {
        let circle = prepared(include_str!(
            "../tests/fixtures/pcb-source/circle-tracer.json"
        ));
        let c = &certify_prisms(&step(&circle)).unwrap()[0];
        within(c.volume, 8.0 * std::f64::consts::PI);
        assert_eq!(c.cylinders, [([10.0, 8.0], 2.0)]);
        let half = prepared(include_str!(
            "../tests/fixtures/pcb-source/line-arc-tracer.json"
        ));
        let c = &certify_prisms(&step(&half)).unwrap()[0];
        within(c.volume, 4.0 * std::f64::consts::PI);
        assert_eq!((c.cylinders.len(), c.planar_walls), (1, 1));
    }

    fn broken(text: &str, from: &str, to: &str) -> Result<usize, Error> {
        assert_eq!(text.matches(from).count(), 1, "{from}");
        certify_prisms(&text.replacen(from, to, 1)).map(|c| c.len())
    }

    #[test]
    fn prism_certificate_refuses_wrong_surface_sense_and_unsupported_geometry() {
        let text = step(&slot());
        let cylinder = text
            .lines()
            .find(|line| line.contains("CYLINDRICAL_SURFACE"))
            .unwrap();
        // Same arcs, but the declared cylinder radius no longer carries them.
        assert!(matches!(
            broken(&text, cylinder, &cylinder.replace("0.4)", "0.41)")),
            Err(Error::Geometry)
        ));
        // A flipped cylinder sense puts the slot wall's material on the wrong side.
        let id = &cylinder[..cylinder.find(' ').unwrap()];
        let faces = text
            .lines()
            .filter(|line| line.contains("ADVANCED_FACE") && line.contains(&format!(", {id}, ")))
            .collect::<Vec<_>>();
        assert_eq!(faces.len(), 1);
        let toggled = if faces[0].ends_with(".T.);") {
            faces[0].replace(".T.);", ".F.);")
        } else {
            faces[0].replace(".F.);", ".T.);")
        };
        assert!(matches!(
            broken(&text, faces[0], &toggled),
            Err(Error::Geometry)
        ));
        // Faceted countersink and non-prism fixtures are never measured as prisms.
        assert!(certify_prisms(include_str!("../tests/fixtures/countersink-013.step")).is_err());
    }

    #[test]
    fn prism_certificate_refuses_disconnected_wall_trim_order() {
        let text = step(&slot());
        assert!(certify_prisms(&text).is_ok(), "unmodified source export");
        // A wall loop has four edges. Swapping adjacent uses leaves all edge counts, senses,
        // surfaces and coordinates unchanged, but disconnects the actual face boundary.
        let wall = text
            .lines()
            .filter(|line| line.contains("EDGE_LOOP"))
            .nth(2)
            .unwrap();
        let (prefix, edges) = wall.split_once("('', (").unwrap();
        let edges = edges.strip_suffix("));").unwrap();
        let mut edges: Vec<_> = edges.split(", ").collect();
        assert_eq!(edges.len(), 4);
        edges.swap(0, 1);
        let disconnected = format!("{prefix}('', ({}));", edges.join(", "));
        let result = broken(&text, wall, &disconnected);
        assert!(matches!(result, Err(Error::Topology)), "{result:?}");
    }

    #[test]
    fn prism_certificate_refuses_a_slot_outside_the_outer_cap() {
        let text = step(&slot());
        assert!(certify_prisms(&text).is_ok(), "unmodified source export");
        let mut moved = 0;
        let displaced = text
            .lines()
            .map(|line| {
                let Some((prefix, coordinates)) = line.split_once("CARTESIAN_POINT('', (") else {
                    return line.to_string();
                };
                let coordinates = coordinates.strip_suffix("));").unwrap();
                let values: Vec<f64> = coordinates
                    .split(", ")
                    .map(|v| v.parse().unwrap())
                    .collect();
                assert_eq!(values.len(), 3);
                // Move every slot point (vertices, curve controls and surface origins), leaving
                // the square at [-10,10] unchanged. The alleged hole now lies outside its cap.
                if (1.0..3.0).contains(&values[0]) && (-7.0..-5.0).contains(&values[1]) {
                    moved += 1;
                    format!(
                        "{prefix}CARTESIAN_POINT('', ({}, {}, {}));",
                        values[0] + 30.0,
                        values[1],
                        values[2]
                    )
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            moved > 20,
            "all slot curve and surface points must be moved"
        );
        let result = certify_prisms(&displaced).map(|c| c.len());
        assert!(matches!(result, Err(Error::Geometry)), "{result:?}");
    }
}
