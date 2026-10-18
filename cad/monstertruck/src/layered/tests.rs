use super::*;
use boardstudio_contracts::*;
use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
use monstertruck_modeling::*;

fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<Vec2> {
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
        .map(|(x, y)| Vec2 { x, y })
        .into()
}
fn plate() -> PreparedCaseIR {
    PreparedCaseIR {
        revision: 1,
        body: CaseBody {
            id: "plate".into(),
            name: "Plate".into(),
            board_id: "board".into(),
            kind: CaseKind::Plate,
            thickness: 4.,
            clearance: 0.,
            material_id: None,
            z: None,
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
            features: None,
            openings: None,
        },
        regions: vec![PreparedCaseRegion {
            outer: rectangle(0., 0., 20., 16.),
            holes: vec![],
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![Mount {
                id: "through-hole".into(),
                at: Vec2 { x: 10., y: 8. },
                kind: MountKind::Hole,
                hole_diameter: 2.,
                boss_diameter: None,
                height: None,
            }],
        }],
    }
}
#[test]
fn one_periodic_cylindrical_hole_face_is_closed_meshable_and_exact_step() {
    let model = build_layered_case(&plate()).expect("layered through-hole");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).expect("closed consistently oriented shell");
    assert!(solid.is_geometric_consistent());
    assert_eq!(
        solid
            .boundaries()
            .iter()
            .flatten()
            .filter(|face| matches!(face.surface(), Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4)
        .expect("complete periodic face tessellation");
    let expected = 1280. - 4. * std::f64::consts::PI;
    assert!(
        (mesh.volume() - expected).abs() < 0.002,
        "{} != {expected}",
        mesh.volume()
    );
    crate::mesh_profile(&model).expect("production mesh path");
    let step =
        String::from_utf8(crate::export_step(&model).expect("production exact STEP")).unwrap();
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1, "{step}");
    assert!(!step.contains("B_SPLINE_SURFACE"));
}

#[test]
fn original_round_seat_chain_matches_independent_volume_and_material() {
    let mut input = plate();
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].mounts.clear();
    input.regions[0].cavities = vec![rectangle(2., 2., 16., 12.)];
    input.regions[0].holes = vec![rectangle(8., 6., 4., 4.)];
    input.body.features = Some(vec![
        CaseFeature::SupportPrism {
            id: "support".into(),
            points: rectangle(3., 3., 3., 3.),
            z: 1.,
            height: 4.,
        },
        CaseFeature::RoundSeat {
            id: "round".into(),
            at: Vec2 { x: 4.5, y: 4.5 },
            z: 3.,
            height: 4.,
            diameter: 1.4,
        },
    ]);
    verify(
        &input,
        1147. - 0.98 * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 6.]],
        &[
            ([10., 8., 1.], false),
            ([1., 8., 4.], true),
            ([4., 4., 4.], true),
            ([4.5, 4.5, 4.], false),
            ([4.5, 4.5, 2.], true),
            ([7., 7., 4.], false),
        ],
        1,
        0,
    );
}

fn retain_evidence(name: &str, bytes: &[u8]) {
    if let Some(directory) = std::env::var_os("LAYERED_013_EVIDENCE_OUTPUT") {
        let path = std::path::Path::new(&directory).join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

fn verify(
    input: &PreparedCaseIR,
    expected: f64,
    bounds: [[f64; 3]; 2],
    samples: &[([f64; 3], bool)],
    cylinders: usize,
    cones: usize,
) {
    retain_evidence(
        &format!("workloads/verified-{}.json", expected.to_bits()),
        &serde_json::to_vec_pretty(input).unwrap(),
    );
    let model = build_layered_case(input).expect("layered exact case");
    assert_eq!(model.solids.len(), 1, "one connected material solid");
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).expect("closed consistently oriented topology");
    assert!(solid.is_geometric_consistent());
    let mesh =
        shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).expect("every exact face meshes");
    assert!(
        (mesh.volume() - expected).abs() < 0.002,
        "volume {} != analytic {expected}",
        mesh.volume()
    );
    let lo = [0, 1, 2].map(|i| {
        mesh.positions()
            .iter()
            .map(|p| p[i])
            .fold(f64::INFINITY, f64::min)
    });
    let hi = [0, 1, 2].map(|i| {
        mesh.positions()
            .iter()
            .map(|p| p[i])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let vertices: Vec<_> = solid
        .boundaries()
        .iter()
        .flat_map(|shell| shell.vertex_iter())
        .map(|vertex| vertex.point())
        .collect();
    let native_lo = [0, 1, 2].map(|axis| {
        vertices
            .iter()
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min)
    });
    let native_hi = [0, 1, 2].map(|axis| {
        vertices
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    assert_eq!([native_lo, native_hi], bounds, "authored outer bounds");
    for (actual, expected) in [lo, hi]
        .into_iter()
        .flatten()
        .zip(bounds.into_iter().flatten())
    {
        assert!(
            (actual - expected).abs() <= 1e-12,
            "reference mesh bound {actual} != {expected}"
        );
    }
    for (point, inside) in samples {
        let point = Point3::from(*point);
        let angle: f64 = mesh
            .faces()
            .triangle_iter()
            .map(|face| {
                let [a, b, c] = face.map(|v| mesh.positions()[v.pos] - point);
                let denominator = a.magnitude() * b.magnitude() * c.magnitude()
                    + a.dot(b) * c.magnitude()
                    + b.dot(c) * a.magnitude()
                    + c.dot(a) * b.magnitude();
                2. * a.dot(b.cross(c)).atan2(denominator)
            })
            .sum();
        let expected = if *inside { 1. } else { 0. };
        assert!(
            (angle / (4. * std::f64::consts::PI) - expected).abs() < 1e-8,
            "point {point:?}: winding {} != {expected}",
            angle / (4. * std::f64::consts::PI)
        );
    }
    crate::mesh_profile(&model).expect("existing production mesh path");
    let step =
        String::from_utf8(crate::export_step(&model).expect("existing exact STEP path")).unwrap();
    retain_evidence(
        &format!("fixture-{}.step", expected.to_bits()),
        step.as_bytes(),
    );
    assert_eq!(
        step.matches("CYLINDRICAL_SURFACE(").count(),
        cylinders,
        "exact merged cylinders"
    );
    assert_eq!(
        step.matches("RATIONAL_B_SPLINE_SURFACE(").count(),
        cones,
        "exact cones encoded by a rational tensor net"
    );
    if cones > 0 {
        inspect_cone_step(&step, [10., 8.], 2., 4., 2., 1.);
    } else {
        assert!(!step.contains("B_SPLINE_SURFACE"));
    }
    if cylinders + cones == 0 {
        let facts = crate::step_accuracy::inspect(&step)
            .expect("reused independent 008 planar Part21 checker");
        assert_eq!(facts.len(), 1);
        assert!((facts[0].volume - expected).abs() < 1e-8);
        assert_eq!(facts[0].bounds, bounds);
        for (point, inside) in samples {
            assert_eq!(facts[0].contains(*point), *inside);
        }
    } else {
        assert!(matches!(
            crate::step_accuracy::inspect(&step),
            Err(crate::step_accuracy::Error::Unsupported)
        ));
    }
}
#[test]
fn boss_through_hole_merges_one_exact_cylinder_across_floor_and_boss() {
    let mut input = plate();
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].cavities = vec![rectangle(2., 2., 16., 12.)];
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    verify(
        &input,
        1152. + 10.75 * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 6.]],
        &[
            ([10., 8., 1.], false),
            ([10., 8., 4.], false),
            ([11., 8., 4.], true),
            ([13., 8., 4.], false),
            ([1., 8., 4.], true),
        ],
        2,
        0,
    );
}
#[test]
fn round_seat_touching_plate_top_preserves_material_without_boolean() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "flush-top".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 4.,
        height: 2.,
        diameter: 2.,
    }]);
    verify(
        &input,
        1280.,
        [[0., 0., 0.], [20., 16., 4.]],
        &[([10., 8., 3.9], true), ([10., 8., 4.1], false)],
        0,
        0,
    );
}
#[test]
fn countersink_uses_exact_cone_and_independent_frustum_integral() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "countersink".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    verify(
        &input,
        1280. - 14. * std::f64::consts::PI / 3.,
        [[0., 0., 0.], [20., 16., 4.]],
        &[
            ([10., 8., 1.], true),
            ([10., 8., 3.], false),
            ([11.7, 8., 2.5], false),
            ([11.7, 8., 3.5], true),
        ],
        0,
        1,
    );
}
#[test]
fn cavity_and_wall_opening_keep_exact_horizontal_profile_differences() {
    let mut input = plate();
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].mounts.clear();
    input.regions[0].cavities = vec![rectangle(2., 2., 16., 12.)];
    input.body.openings = Some(vec![CaseOpening {
        points: rectangle(-1., 6., 4., 4.),
        z: 3.,
        height: 2.,
    }]);
    verify(
        &input,
        1136.,
        [[0., 0., 0.], [20., 16., 6.]],
        &[
            ([1., 8., 4.], false),
            ([1., 8., 2.5], true),
            ([1., 8., 5.5], true),
            ([3., 8., 4.], false),
            ([19., 8., 4.], true),
            ([10., 8., 1.], true),
        ],
        0,
        0,
    );
}
#[test]
fn cone_crossing_vertical_wall_is_rejected_with_feature_identity() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "wall-crossing-seat".into(),
        at: Vec2 { x: 0.5, y: 8. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    let error = build_layered_case(&input)
        .err()
        .expect("cross-wall cone is unsupported");
    assert!(
        error.contains("wall-crossing-seat") && error.contains("vertical wall"),
        "{error}"
    );
}

#[test]
fn adoption_clipped_cone_retains_closed_exact_topology_and_native_geometry() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/cone-clipped-height.json"
    ))
    .unwrap();
    let model = crate::build_case(&input).expect("height-clipped exact cone");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    assert_eq!(solid.boundaries().len(), 1);
    Solid::try_new(solid.boundaries().clone())
        .expect("closed consistently oriented native topology");
    assert!(solid.is_geometric_consistent());
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).unwrap();
    assert!((mesh.volume() - (1280. - 112. * std::f64::consts::PI / 3.)).abs() < 0.002);
}

#[test]
fn adoption_overlapping_openings_preserve_collinear_authored_segments() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/overlapping-openings-lineage.json"
    ))
    .unwrap();
    verify(
        &input,
        1208.,
        [[0., 0., 0.], [20., 16., 4.]],
        &[
            ([5., 5., 3.], false),
            ([15., 8., 3.], false),
            ([10., 8., 3.], true),
            ([5., 5., 1.], true),
        ],
        0,
        0,
    );
}

#[test]
fn adoption_ring_island_blind_cut_preserves_connected_floor() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/opening-ring-island.json"
    ))
    .unwrap();
    verify(
        &input,
        1116., // Existing independent oracle: 1280 - (48 + 42 - 8) * 2.
        [[0., 0., 0.], [20., 16., 4.]],
        &[
            ([5., 5., 3.], false),
            ([10., 8., 3.], true),
            ([5., 5., 1.], true),
            ([3.5, 11., 3.], false),
        ],
        0,
        0,
    );
}

fn inspect_cone_step(text: &str, center: [f64; 2], low: f64, high: f64, r0: f64, r1: f64) {
    use std::collections::BTreeMap;
    use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};
    fn list(p: &Parameter) -> &[Parameter] {
        match p {
            Parameter::List(p) => p,
            _ => panic!("expected Part21 list"),
        }
    }
    fn id(p: &Parameter) -> u64 {
        match p {
            Parameter::Ref(Name::Entity(id)) => *id,
            _ => panic!("expected Part21 reference"),
        }
    }
    fn num(p: &Parameter) -> f64 {
        match p {
            Parameter::Real(v) => *v,
            Parameter::Integer(v) => *v as f64,
            _ => panic!("expected Part21 number"),
        }
    }
    fn args<'a>(
        records: &'a BTreeMap<u64, Vec<Record>>,
        entity: u64,
        name: &str,
    ) -> &'a [Parameter] {
        list(
            &records[&entity]
                .iter()
                .find(|r| r.name.as_str() == name)
                .expect("required exact geometry record")
                .parameter,
        )
    }
    fn xyz(records: &BTreeMap<u64, Vec<Record>>, entity: u64, name: &str) -> [f64; 3] {
        let p = list(&args(records, entity, name)[1]);
        [num(&p[0]), num(&p[1]), num(&p[2])]
    }
    let exchange: Exchange = text.parse().expect("valid independent Part21 parse");
    let records: BTreeMap<_, _> = exchange
        .data
        .into_iter()
        .flat_map(|d| d.entities)
        .map(|e| match e {
            EntityInstance::Simple { id, record } => (id, vec![record]),
            EntityInstance::Complex { id, subsuper } => (id, subsuper.0),
        })
        .collect();
    let surfaces: Vec<_> = records
        .iter()
        .filter(|(_, r)| {
            r.iter()
                .any(|r| r.name.as_str() == "RATIONAL_B_SPLINE_SURFACE")
        })
        .map(|(&id, _)| id)
        .collect();
    assert_eq!(surfaces.len(), 1);
    let surface = surfaces[0];
    let net = args(&records, surface, "B_SPLINE_SURFACE");
    assert_eq!([num(&net[0]), num(&net[1])], [1., 2.]);
    let rows = list(&net[2]);
    assert_eq!(rows.len(), 2);
    let weights = list(&args(&records, surface, "RATIONAL_B_SPLINE_SURFACE")[0]);
    assert_eq!(weights.len(), 2);
    let knots = args(&records, surface, "B_SPLINE_SURFACE_WITH_KNOTS");
    assert_eq!(
        list(&knots[0]).iter().map(num).collect::<Vec<_>>(),
        vec![2., 2.]
    );
    assert_eq!(
        list(&knots[1]).iter().map(num).collect::<Vec<_>>(),
        vec![3., 2., 2., 2., 3.]
    );
    assert_eq!(
        list(&knots[2]).iter().map(num).collect::<Vec<_>>(),
        vec![0., 1.]
    );
    assert_eq!(
        list(&knots[3]).iter().map(num).collect::<Vec<_>>(),
        vec![0., 0.25, 0.5, 0.75, 1.]
    );
    for (row_index, (z, radius)) in [(low, r0), (high, r1)].into_iter().enumerate() {
        let row: Vec<_> = list(&rows[row_index])
            .iter()
            .map(|p| xyz(&records, id(p), "CARTESIAN_POINT"))
            .collect();
        let w = list(&weights[row_index]);
        assert_eq!(row.len(), 9);
        assert_eq!(w.len(), 9);
        for i in 0..9 {
            assert!((row[i][2] - z).abs() < 1e-12);
            let expected = if i % 2 == 0 {
                1.
            } else {
                std::f64::consts::FRAC_1_SQRT_2
            };
            assert!((num(&w[i]) - expected).abs() < 1e-15);
        }
        assert_eq!(row[0], row[8]);
        for i in (0..8).step_by(2) {
            let a = [row[i][0] - center[0], row[i][1] - center[1]];
            let b = [row[i + 2][0] - center[0], row[i + 2][1] - center[1]];
            assert!((a[0].hypot(a[1]) - radius).abs() < 1e-12);
            assert!((b[0].hypot(b[1]) - radius).abs() < 1e-12);
            assert!(
                (a[0] * b[0] + a[1] * b[1]).abs() < 1e-12,
                "orthogonal quarter-circle endpoints"
            );
            for axis in 0..2 {
                assert!(
                    (row[i + 1][axis] - center[axis] - a[axis] - b[axis]).abs() < 1e-12,
                    "exact rational quarter-circle middle control"
                );
            }
        }
    }
    // Equal weights and corresponding radial controls in the two degree-one rows prove linear r(z).
    for i in 0..9 {
        assert_eq!(num(&list(&weights[0])[i]), num(&list(&weights[1])[i]));
        let a = xyz(&records, id(&list(&rows[0])[i]), "CARTESIAN_POINT");
        let b = xyz(&records, id(&list(&rows[1])[i]), "CARTESIAN_POINT");
        for axis in 0..2 {
            assert!(((a[axis] - center[axis]) / r0 - (b[axis] - center[axis]) / r1).abs() < 1e-12);
        }
    }
    let radius = |z: f64| r0 + (r1 - r0) * (z - low) / (high - low);
    let face = records
        .iter()
        .find_map(|(_, r)| {
            r.iter()
                .find(|r| r.name.as_str() == "ADVANCED_FACE")
                .filter(|r| id(&list(&r.parameter)[2]) == surface)
        })
        .expect("cone face is actually used");
    let mut zs = vec![];
    for bound in list(&list(&face.parameter)[1]) {
        let bound = args(&records, id(bound), "FACE_BOUND");
        let wire = args(&records, id(&bound[1]), "EDGE_LOOP");
        for oriented in list(&wire[1]) {
            let edge = args(
                &records,
                id(&args(&records, id(oriented), "ORIENTED_EDGE")[3]),
                "EDGE_CURVE",
            );
            let curve = args(&records, id(&edge[3]), "SURFACE_CURVE");
            assert!(
                records[&id(&curve[1])]
                    .iter()
                    .any(|r| r.name.as_str() == "RATIONAL_B_SPLINE_CURVE"),
                "cone circular trims remain exact rational arcs"
            );
            for vertex in [&edge[1], &edge[2]] {
                let point = xyz(
                    &records,
                    id(&args(&records, id(vertex), "VERTEX_POINT")[1]),
                    "CARTESIAN_POINT",
                );
                assert!(
                    ((point[0] - center[0]).hypot(point[1] - center[1]) - radius(point[2])).abs()
                        < 1e-12
                );
                zs.push(point[2]);
            }
        }
    }
    assert_eq!(zs.iter().copied().fold(f64::INFINITY, f64::min), low);
    assert_eq!(zs.iter().copied().fold(f64::NEG_INFINITY, f64::max), high);
}

#[test]
fn build_has_no_general_3d_boolean_dependency_or_runtime_calls() {
    assert!(!include_str!("../../Cargo.lock").contains("name = \"monstertruck-solid\""));
    for source in [
        include_str!("../layered.rs"),
        include_str!("contours.rs"),
        include_str!("topology.rs"),
        include_str!("pcb.rs"),
    ] {
        assert!(!source.contains("monstertruck_solid"));
        assert!(!source.contains("cadrum"));
    }
}
#[test]
fn authored_close_height_boundaries_are_retained_exactly() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    let low = 4. - 0.0001;
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "thin-depth".into(),
        at: Vec2 { x: 10., y: 8. },
        z: low,
        height: 0.0001,
        diameter: 2.,
    }]);
    let model = build_layered_case(&input).expect("authored thin cut");
    let vertices: Vec<_> = model.solids[0]
        .boundaries()
        .iter()
        .flat_map(|s| s.vertex_iter())
        .map(|v| v.point().z)
        .collect();
    assert!(vertices.iter().any(|&z| z.to_bits() == low.to_bits()));
    assert!(vertices.iter().any(|&z| z.to_bits() == 4f64.to_bits()));
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1);
}
#[test]
fn enclosed_void_is_refused_instead_of_emitting_a_second_body() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "sealed-seat".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 1.,
        height: 1.,
        diameter: 2.,
    }]);
    let error = build_layered_case(&input)
        .err()
        .expect("sealed void ownership is outside this prototype");
    assert!(error.contains("disconnected"), "{error}");
}

fn cylinder_senses(text: &str) -> Vec<(f64, bool)> {
    use std::collections::BTreeMap;
    use step_p21::ast::{EntityInstance, Exchange, Name, Parameter};
    fn list(p: &Parameter) -> &[Parameter] {
        match p {
            Parameter::List(p) => p,
            _ => panic!("Part21 list"),
        }
    }
    fn id(p: &Parameter) -> u64 {
        match p {
            Parameter::Ref(Name::Entity(id)) => *id,
            _ => panic!("Part21 ref"),
        }
    }
    let exchange: Exchange = text.parse().unwrap();
    let records: BTreeMap<_, _> = exchange
        .data
        .into_iter()
        .flat_map(|d| d.entities)
        .filter_map(|e| match e {
            EntityInstance::Simple { id, record } => Some((id, record)),
            _ => None,
        })
        .collect();
    let mut result = vec![];
    for face in records
        .values()
        .filter(|r| r.name.as_str() == "ADVANCED_FACE")
    {
        let face = list(&face.parameter);
        let surface = &records[&id(&face[2])];
        if surface.name.as_str() != "CYLINDRICAL_SURFACE" {
            continue;
        }
        let radius = match list(&surface.parameter)[2] {
            Parameter::Real(v) => v,
            _ => panic!("radius"),
        };
        let sense = match &face[3] {
            Parameter::Enumeration(v) if v == "T" => true,
            Parameter::Enumeration(v) if v == "F" => false,
            _ => panic!("face same_sense"),
        };
        result.push((radius, sense));
    }
    result.sort_by(|a, b| a.0.total_cmp(&b.0));
    result
}
#[test]
fn exact_step_cylinder_face_senses_match_holes_and_boss_material() {
    let input = plate();
    let model = build_layered_case(&input).unwrap();
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        cylinder_senses(&step),
        vec![(1., false)],
        "through-hole cylinder must point into its void"
    );
    let mut input = input;
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].cavities = vec![rectangle(2., 2., 16., 12.)];
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    let model = build_layered_case(&input).unwrap();
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        cylinder_senses(&step),
        vec![(0.5, false), (2., true)],
        "hole inward, boss exterior outward"
    );
}

#[test]
fn reviewed_authored_mount_order_preserves_later_boss_hole() {
    let mut input = plate();
    input.regions[0].mounts = vec![
        Mount {
            id: "first-hole".into(),
            at: Vec2 { x: 10., y: 8. },
            kind: MountKind::Hole,
            hole_diameter: 3.,
            boss_diameter: None,
            height: None,
        },
        Mount {
            id: "later-boss".into(),
            at: Vec2 { x: 10., y: 8. },
            kind: MountKind::Boss,
            hole_diameter: 1.,
            boss_diameter: Some(4.),
            height: Some(2.),
        },
    ];
    let model = build_layered_case(&input).expect("authored mixed mount sequence");
    let solid = &model.solids[0];
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).unwrap();
    assert!(
        (mesh.volume() - (1280. - 1.5 * std::f64::consts::PI)).abs() < 0.002,
        "authored order volume {}",
        mesh.volume()
    );
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        cylinder_senses(&step),
        vec![(0.5, false), (1.5, false), (2., true)]
    );
}
#[test]
fn reviewed_sealed_opening_is_refused_with_opening_context() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.openings = Some(vec![CaseOpening {
        points: rectangle(8., 6., 4., 4.),
        z: 1.,
        height: 1.,
    }]);
    let error = build_layered_case(&input)
        .err()
        .expect("sealed void must never become a material body");
    assert!(
        error.contains("opening-0") && error.contains("sealed"),
        "{error}"
    );
}
#[test]
fn reviewed_unrepresentable_positive_span_is_refused_with_feature_context() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "collapsed-depth".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 1.,
        height: 1e-17,
        diameter: 2.,
    }]);
    let error = build_layered_case(&input)
        .err()
        .expect("positive authored height cannot disappear");
    assert!(
        error.contains("collapsed-depth") && error.contains("height"),
        "{error}"
    );
}
#[test]
fn outward_material_split_by_opening_remains_two_valid_solids() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.openings = Some(vec![CaseOpening {
        points: rectangle(8., -1., 4., 18.),
        z: 0.,
        height: 4.,
    }]);
    let model = build_layered_case(&input).expect("valid outward split bodies");
    assert_eq!(model.solids.len(), 2);
    for solid in &model.solids {
        Solid::try_new(solid.boundaries().clone()).unwrap();
        let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).unwrap();
        assert!((mesh.volume() - 512.).abs() < 1e-8);
    }
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 2);
}

#[test]
fn pcb_polygon_outline_and_authored_round_drill_are_exact() {
    let input: PreparedCaseIR =
        serde_json::from_str(include_str!("../../tests/fixtures/layered/pcb-simple.json")).unwrap();
    verify(
        &input,
        512. - 1.6 * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 1.6]],
        &[
            ([10., 8., 0.8], false),
            ([11.2, 8., 0.8], true),
            ([1., 1., 0.8], true),
            ([10., 8., 1.7], false),
        ],
        1,
        0,
    );
    let model = build_layered_case(&input).unwrap();
    assert_eq!(
        model.solids[0]
            .boundaries()
            .iter()
            .flatten()
            .filter(|f| matches!(f.surface(), Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(cylinder_senses(&step), vec![(1., false)]);
}
#[test]
fn pcb_nonconvex_polygon_with_diagonal_and_multiple_drills_is_exact() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/pcb-nonconvex.json"
    ))
    .unwrap();
    // Rectangle30x10 plus trapezoid(15+10)x10/2 gives425mm2; drills remove1.8125*pi.
    verify(
        &input,
        680. - 2.9 * std::f64::consts::PI,
        [[0., 0., 0.], [30., 20., 1.6]],
        &[
            ([5., 5., 0.8], false),
            ([25., 5., 0.8], false),
            ([5., 15., 0.8], false),
            ([6.2, 5., 0.8], true),
            ([11., 11., 0.8], true),
            ([25., 15., 0.8], false),
        ],
        3,
        0,
    );
    let model = build_layered_case(&input).unwrap();
    let shell = &model.solids[0].boundaries()[0];
    assert_eq!(
        shell
            .iter()
            .filter(|f| matches!(f.surface(), Surface::Plane(_)))
            .count(),
        8
    );
    assert_eq!(
        shell
            .iter()
            .filter(|f| matches!(f.surface(), Surface::RevolutionSurface(_)))
            .count(),
        3
    );
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        cylinder_senses(&step),
        vec![(0.5, false), (0.75, false), (1., false)]
    );
}
#[test]
fn comparable_build_workload() {
    let text = match std::env::var("LAYERED_013_WORKLOAD") {
        Ok(path) => std::fs::read_to_string(path).expect("workload JSON"),
        Err(_) => include_str!("../../tests/fixtures/layered/pcb-simple.json").to_owned(),
    };
    let input: PreparedCaseIR =
        serde_json::from_str(&text).expect("unchanged PreparedCaseIR workload");
    eprintln!("LAYERED_013_BUILD_BEGIN id={}", input.body.id);
    let started = std::time::Instant::now();
    let model = build_layered_case(&input).expect("layered build");
    let elapsed = started.elapsed();
    eprintln!(
        "LAYERED_013_BUILD_END id={} elapsed_ns={}",
        input.body.id,
        elapsed.as_nanos()
    );
    assert!(!model.solids.is_empty());
}

#[test]
fn unrepresentable_body_interval_is_rejected_before_layer_assembly() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.id = "collapsed-pcb-elevation".into();
    input.body.z = Some(1e17);
    input.body.thickness = 1.;
    let error = build_layered_case(&input)
        .err()
        .expect("body height cannot collapse at its elevation");
    assert!(
        error.contains("collapsed-pcb-elevation") && error.contains("height"),
        "{error}"
    );
}

#[test]
fn prepared_gasket_pair_retains_inner_material_for_tray_and_lid() {
    let assembly: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/gasketed-pair.prepared.json"
    ))
    .unwrap();
    for input in assembly.bodies {
        let z = input.body.z.unwrap();
        let groove_z = if input.body.kind == CaseKind::Tray {
            7.75
        } else {
            12.25
        };
        verify(
            &input,
            9122.,
            [[0., 0., z], [60., 40., z + 8.]],
            &[
                ([1.5, 20., groove_z], false),
                ([0.5, 20., groove_z], true),
                ([3., 20., groove_z], true),
                ([30., 20., z + 3.], false),
            ],
            0,
            0,
        );
    }
}

#[test]
fn prepared_wall_support_and_round_seat_preserve_clipped_circular_wall() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/wall-support-seat.prepared.json"
    ))
    .unwrap();
    let strip_area = 2. * ((1.6_f64 * 1.6 - 1.).sqrt() + 1.6 * 1.6 * (1. / 1.6_f64).asin());
    verify(
        &input,
        2832. - 3. * strip_area,
        [[0., 0., 0.], [30., 20., 10.]],
        &[
            ([4., 10., 4.], true),
            ([7., 10., 4.], false),
            ([10., 1., 5.], false),
            ([10., 1., 3.], true),
            ([12., 1., 5.], true),
        ],
        2,
        0,
    ); // Two disconnected cylinder patches on opposite sides of the clipped circle.
    let model = build_layered_case(&input).unwrap();
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        cylinder_senses(&step),
        vec![(1.6, false), (1.6, false)],
        "authored radius must set the exact native cylinder, not a reconstructed endpoint radius"
    );
}

#[test]
fn prepared_concave_gasket_pair_keeps_nested_material_and_notch() {
    let assembly: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/concave-gasket.prepared.json"
    ))
    .unwrap();
    // L-shaped area3900; cavity72*52-30*30=2844; groove3624-3356=268.
    for input in assembly.bodies {
        let z = input.body.z.unwrap();
        let groove_z = if input.body.kind == CaseKind::Tray {
            7.75
        } else {
            12.25
        };
        verify(
            &input,
            14002.,
            [[0., 0., z], [80., 60., z + 8.]],
            &[
                ([1.5, 20., groove_z], false),
                ([0.5, 20., groove_z], true),
                ([3., 20., groove_z], true),
                ([20., 20., z + 3.], false),
                ([60., 40., z + 1.], false),
            ],
            0,
            0,
        );
    }
}

#[test]
fn prepared_internal_gasket_inventory() {
    let expectations: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/internal-independent-expectations.json"
    ))
    .unwrap();
    let mut work_items = vec![];
    for name in [
        "rectangle",
        "countersunk",
        "downward-boss",
        "rotated-concave",
        "split",
    ] {
        let path = format!(
            "{}/tests/fixtures/layered/internal-gasket-v1/{name}.prepared.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let assembly: PreparedCaseAssemblyIR =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for input in assembly.bodies {
            work_items.push((name, input));
        }
    }
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = work_items
            .into_iter()
            .map(|(name, input)| {
                let expectations = &expectations;
                scope.spawn(move || -> Option<String> {
                    match build_layered_case(&input) {
                        Ok(model) => {
                            let expected = &expectations["fixtures"][name][&input.body.id];
                            assert_eq!(model.solids.len(), input.regions.len());
                            let mut meshes = vec![];
                            for solid in &model.solids {
                                Solid::try_new(solid.boundaries().clone())
                                    .expect("closed consistently oriented internal case");
                                assert!(solid.is_geometric_consistent());
                                for shell in solid.boundaries() {
                                    meshes.push(
                                        shell_to_polygon_strict(shell, 1e-5)
                                            .expect("every internal face tessellates"),
                                    );
                                }
                            }
                            let volume: f64 = meshes.iter().map(|mesh| mesh.volume()).sum();
                            let target = expected["volume"].as_f64().unwrap();
                            assert!(
                                (volume - target).abs() < 0.002,
                                "{name}/{} volume {volume} != independent {target}",
                                input.body.id
                            );
                            let points: Vec<_> = model
                                .solids
                                .iter()
                                .flat_map(|solid| solid.boundaries())
                                .flat_map(|shell| shell.vertex_iter())
                                .map(|vertex| vertex.point())
                                .collect();
                            for axis in 0..3 {
                                let lo = points
                                    .iter()
                                    .map(|point| point[axis])
                                    .fold(f64::INFINITY, f64::min);
                                let hi = points
                                    .iter()
                                    .map(|point| point[axis])
                                    .fold(f64::NEG_INFINITY, f64::max);
                                assert!(
                                    (lo - expected["bounds"][0][axis].as_f64().unwrap()).abs()
                                        < 1e-10
                                );
                                assert!(
                                    (hi - expected["bounds"][1][axis].as_f64().unwrap()).abs()
                                        < 1e-10
                                );
                            }
                            for probe in expected["probes"].as_array().unwrap() {
                                let point = Point3::new(
                                    probe[0][0].as_f64().unwrap(),
                                    probe[0][1].as_f64().unwrap(),
                                    probe[0][2].as_f64().unwrap(),
                                );
                                let angle: f64 = meshes
                                    .iter()
                                    .flat_map(|mesh| {
                                        mesh.faces().triangle_iter().map(move |face| {
                                            let [a, b, c] = face
                                                .map(|vertex| mesh.positions()[vertex.pos] - point);
                                            let denominator =
                                                a.magnitude() * b.magnitude() * c.magnitude()
                                                    + a.dot(b) * c.magnitude()
                                                    + b.dot(c) * a.magnitude()
                                                    + c.dot(a) * b.magnitude();
                                            2. * a.dot(b.cross(c)).atan2(denominator)
                                        })
                                    })
                                    .sum();
                                let target = if probe[1].as_bool().unwrap() { 1. } else { 0. };
                                assert!(
                                    (angle / (4. * std::f64::consts::PI) - target).abs() < 1e-8,
                                    "{name}/{} probe {point:?}",
                                    input.body.id
                                );
                            }
                            crate::mesh_profile(&model).expect("existing mesh path");
                            let step =
                                crate::export_step(&model).expect("existing exact STEP path");
                            retain_evidence(
                                &format!(
                                    "internal-step/{name}-{}.step",
                                    input.body.id.replace(':', "_")
                                ),
                                &step,
                            );
                            None
                        }
                        Err(error) => Some(format!("{name}/{}: {error}", input.body.id)),
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap())
            .collect()
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn near_concentric_distinct_sources_are_not_declared_coincident() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![
        CaseFeature::RoundSeat {
            id: "first-circle".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
        CaseFeature::RoundSeat {
            id: "distinct-circle".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2. + 1e-11,
        },
    ]);
    let error = build_layered_case(&input)
        .err()
        .expect("distinct authored circles must not be silently coincident");
    assert!(error.contains("unproved coincidence"), "{error}");
    if let Some(features) = &mut input.body.features
        && let CaseFeature::RoundSeat { at, diameter, .. } = &mut features[1]
    {
        at.x += 1e-11;
        *diameter = 2.;
    }
    let error = build_layered_case(&input)
        .err()
        .expect("nearby authored axes must not be declared coincident");
    assert!(error.contains("unproved coincidence"), "{error}");
}

#[test]
fn exactly_coincident_authored_seats_keep_one_native_cylinder() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![
        CaseFeature::RoundSeat {
            id: "first".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
        CaseFeature::RoundSeat {
            id: "second".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
    ]);
    verify(
        &input,
        1280. - 4. * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 4.]],
        &[([10., 8., 2.], false), ([12., 8., 2.], true)],
        1,
        0,
    );
}

#[test]
fn rotated_notched_wall_is_one_authored_plane() {
    let angle = 0.3_f64;
    let (sine, cosine) = angle.sin_cos();
    let rotate = |point: &mut Vec2| {
        let (x, y) = (point.x, point.y);
        point.x = x * cosine - y * sine;
        point.y = x * sine + y * cosine;
    };
    let mut input = plate();
    input.regions[0].mounts.clear();
    for point in &mut input.regions[0].outer {
        rotate(point);
    }
    let mut at = Vec2 { x: 10., y: 0. };
    rotate(&mut at);
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "rotated-wall-notch".into(),
        at,
        z: 1.,
        height: 2.,
        diameter: 2.,
    }]);
    let retained: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/rotated-wall-notch.json"
    ))
    .unwrap();
    // Cover the exact typed RED and the normal JSON seam without changing decoder features.
    for (name, input) in [("typed", &input), ("json", &retained)] {
        let model = build_layered_case(input).expect("rotated wall with finite-height notch");
        let outward = Vector3::new(sine, -cosine, 0.);
        let count=model.solids[0].boundaries()[0].iter().filter(|face| matches!(face.oriented_surface(),Surface::Plane(plane) if plane.normal().normalize().dot(outward)>1.-1e-12)).count();
        assert_eq!(
            count, 1,
            "one connected authored wall plane, including the boundary around its notch"
        );
        let sample = |x: f64, y: f64, z: f64| [x * cosine - y * sine, x * sine + y * cosine, z];
        verify(
            input,
            1280. - std::f64::consts::PI,
            [
                [-16. * sine, 0., 0.],
                [20. * cosine, 20. * sine + 16. * cosine, 4.],
            ],
            &[
                (sample(10., 0.5, 2.), false),
                (sample(10., 1.5, 2.), true),
                (sample(10., 0.5, 0.5), true),
                (sample(10., -0.5, 2.), false),
            ],
            1,
            0,
        );
        let exact = crate::export_step(&model).unwrap();
        retain_evidence(&format!("rotated-{name}.step"), &exact);
    }
}
