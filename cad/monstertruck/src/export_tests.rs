use crate::{ProfileInput, ProfileKind, ProfileRegion, build_profile, export_step};
use boardstudio_contracts::Vec2;
use std::collections::BTreeMap;
use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};

pub(crate) struct StepRecords(BTreeMap<u64, Vec<Record>>);

impl StepRecords {
    pub(crate) fn parse(text: &str) -> Self {
        let exchange: Exchange = text.parse().expect("valid independent Part 21 parse");
        Self(
            exchange
                .data
                .into_iter()
                .flat_map(|section| section.entities)
                .map(|entity| match entity {
                    EntityInstance::Simple { id, record } => (id, vec![record]),
                    EntityInstance::Complex { id, subsuper } => (id, subsuper.0),
                })
                .collect(),
        )
    }

    pub(crate) fn named<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = (u64, &'a [Parameter])> {
        self.0.iter().filter_map(move |(&id, records)| {
            records
                .iter()
                .find(|record| record.name.as_str() == name)
                .map(|record| (id, list(&record.parameter)))
        })
    }

    pub(crate) fn args(&self, id: u64, name: &str) -> &[Parameter] {
        let record = self.0[&id]
            .iter()
            .find(|record| record.name.as_str() == name)
            .unwrap_or_else(|| panic!("required {name} at #{id}"));
        list(&record.parameter)
    }

    pub(crate) fn xyz(&self, id: u64, name: &str) -> [f64; 3] {
        let coordinates = list(&self.args(id, name)[1]);
        assert_eq!(coordinates.len(), 3);
        std::array::from_fn(|axis| num(&coordinates[axis]))
    }
}

pub(crate) fn list(p: &Parameter) -> &[Parameter] {
    match p {
        Parameter::List(p) => p,
        _ => panic!("expected Part 21 list"),
    }
}

pub(crate) fn id(p: &Parameter) -> u64 {
    match p {
        Parameter::Ref(Name::Entity(id)) => *id,
        _ => panic!("expected Part 21 reference"),
    }
}

pub(crate) fn num(p: &Parameter) -> f64 {
    match p {
        Parameter::Real(v) => *v,
        Parameter::Integer(v) => *v as f64,
        _ => panic!("expected Part 21 number"),
    }
}

pub(crate) fn step_text(model: &crate::ExactModel) -> String {
    String::from_utf8(export_step(model).expect("exact STEP export")).unwrap()
}

pub(crate) fn assert_step_round_trip(
    text: &str,
    bounds: [[f64; 3]; 2],
    faces: usize,
) -> crate::ImportedStep {
    let imported = crate::import_step(text.as_bytes().to_vec()).expect("generated STEP round-trip");
    assert_eq!(imported.solid_count(), 1);
    assert_eq!(imported.occurrence_count(), 1);
    assert_eq!(imported.source_faces().len(), faces);
    assert_eq!(imported.face_occurrences().len(), faces);
    assert_eq!(
        imported
            .triangle_faces()
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        faces,
    );
    assert!(imported.accuracy().bounds_from_source_geometry);
    for (actual, expected) in imported
        .bounds()
        .into_iter()
        .flatten()
        .zip(bounds.into_iter().flatten())
    {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "bound {actual} != {expected}"
        );
    }
    imported
}

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

fn plate_profile(base_z: f64, thickness: f64, regions: Vec<ProfileRegion>) -> crate::ExactModel {
    build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z,
        thickness,
        wall_height: 0.0,
        regions,
    })
    .unwrap()
}

#[test]
fn exact_step_exports_one_mm_solid() {
    let model = plate_profile(4.0, 2.0, vec![rectangle(3.0, -5.0, 20.0, 10.0)]);
    let text = step_text(&model);
    assert_eq!(text, step_text(&model), "AP214 output is deterministic");
    assert!(text.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))"));
    assert_eq!(
        StepRecords::parse(&text)
            .named("MANIFOLD_SOLID_BREP")
            .count(),
        1
    );
    assert!(text.contains("SI_UNIT(.MILLI.,.METRE.)"));
}

#[test]
fn named_step_keeps_body_identity_and_all_disconnected_solids() {
    use crate::{StepBody, export_step_bodies};
    let build = |regions| plate_profile(4.0, 2.0, regions);
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
    let text = std::str::from_utf8(&bytes).unwrap();
    let records = StepRecords::parse(text);
    let products: Vec<_> = records
        .named("PRODUCT")
        .map(|(_, p)| (&p[0], &p[1]))
        .collect();
    assert_eq!(
        products,
        [("first", "Plate's first body"), ("second", "two islands")]
            .map(|(id, name)| (Parameter::String(id.into()), Parameter::String(name.into())))
            .iter()
            .map(|(id, name)| (id, name))
            .collect::<Vec<_>>()
    );
    assert_eq!(records.named("MANIFOLD_SOLID_BREP").count(), 3);
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
    let model = plate_profile(4.0, 2.0, vec![region]);
    let step = step_text(&model);
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
    let model = plate_profile(
        0.0,
        2.0,
        vec![ProfileRegion {
            outer,
            holes: vec![rectangle(15.0, 15.0, 5.0, 5.0).outer],
        }],
    );
    step_text(&model)
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
    let model = plate_profile(0.0, 2.0, vec![region]);
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
    let model = |z| plate_profile(z, 2.0, vec![rectangle(0.0, 0.0, 20.0, 20.0)]);
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
    let empty = ExactModel::from_test_solids(vec![]);
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
    let model = plate_profile(0.0, 2.0, vec![rectangle(0.0, 0.0, 2.0, 1.0)]);
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
        "../tests/fixtures/profile-topology/rotated-concave.json"
    ))
    .unwrap();
    let holes: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/profile-topology/multi-hole-region.json"
    ))
    .unwrap();
    let split: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/profile-topology/split-regions.json"
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
    let bytes = export_step(&crate::ExactModel::from_test_solids(vec![solid])).unwrap();
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
            x: 999990.1 + x * angle.cos() - y * angle.sin(),
            y: 999990.2 + x * angle.sin() + y * angle.cos(),
        })
        .into();
    let model = plate_profile(
        1000000.3,
        4.0,
        vec![ProfileRegion {
            outer,
            holes: vec![],
        }],
    );
    let text = step_text(&model);
    let solids = crate::step_accuracy::inspect(&text).expect("valid rotated placed box");
    assert!((solids[0].volume - 24.0).abs() <= 0.1);
}

fn edit_record(step: &str, entity: u64, from: &str, to: &str) -> String {
    let prefix = format!("#{entity} = ");
    let record = step.lines().find(|line| line.starts_with(&prefix)).unwrap();
    assert!(record.contains(from), "control must edit {record}");
    step.replacen(record, &record.replacen(from, to, 1), 1)
}

fn replace_record(step: &str, entity: u64, contents: &str) -> String {
    let prefix = format!("#{entity} = ");
    let old = step
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap();
    assert_ne!(old, contents);
    edit_record(step, entity, old, contents)
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
    let model = plate_profile(0.0, 1.0, vec![rectangle(0.0, 0.0, 1.0, 1.0)]);
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
    let records = StepRecords::parse(&text);
    let (representation, params) = records.named("SHAPE_REPRESENTATION").nth(1).unwrap();
    let old_context = id(&params[2]);
    let rewritten = edit_record(
        &text,
        representation,
        &format!(", #{old_context});"),
        ", #9000);",
    );
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
        plate_profile(
            0.0,
            3.0,
            vec![ProfileRegion {
                outer: polygon(3.0),
                holes: vec![polygon(drill)],
            }],
        )
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
        let model = plate_profile(
            0.0,
            body["thickness"].as_f64().unwrap(),
            vec![ProfileRegion {
                outer: points(outers[0]),
                holes,
            }],
        );
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

mod curved_solids;

// Source-derived complete trim expectations replace the rejected general prism WIP.
mod fixture_exports;
