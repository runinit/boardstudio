use super::*;
use crate::export_tests::{StepRecords, assert_step_round_trip, id, list, num, step_text};
use boardstudio_contracts::*;
use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
use monstertruck_modeling::*;

pub(super) fn material(
    mesh: &monstertruck_meshing::prelude::PolygonMesh,
    point: [f64; 3],
    inside: bool,
) {
    let target = if inside { 1. } else { 0. };
    let winding = mesh_winding([mesh], Point3::from(point));
    assert!(
        (winding - target).abs() < 1e-8,
        "material {point:?}: winding {winding} != {target}"
    );
}

pub(super) fn mesh_winding<'a>(
    meshes: impl IntoIterator<Item = &'a PolygonMesh>,
    point: Point3,
) -> f64 {
    let angle: f64 = meshes
        .into_iter()
        .flat_map(|mesh| {
            mesh.faces().triangle_iter().map(move |face| {
                let [a, b, c] = face.map(|v| mesh.positions()[v.pos] - point);
                let denominator = a.magnitude() * b.magnitude() * c.magnitude()
                    + a.dot(b) * c.magnitude()
                    + b.dot(c) * a.magnitude()
                    + c.dot(a) * b.magnitude();
                2. * a.dot(b.cross(c)).atan2(denominator)
            })
        })
        .sum();
    angle / (4. * std::f64::consts::PI)
}

pub(super) fn point_bounds(points: impl IntoIterator<Item = Point3>) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in points {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(p[axis]);
            bounds[1][axis] = bounds[1][axis].max(p[axis]);
        }
    }
    bounds
}

pub(super) fn check_solid(solid: &Solid) {
    Solid::try_new(solid.boundaries().clone()).expect("closed consistently oriented topology");
    assert!(solid.is_geometric_consistent());
}

/// Check the public analysis rolls up exactly the direct shell emission in
/// native face order, so a vendor fallback cannot pass an analytic fixture.
pub(super) fn direct_analysis(model: &ExactModel, target: Option<f64>) -> Vec<Vec<PolygonMesh>> {
    let analysis = crate::mesh_profile_for_analysis(model, target).unwrap();
    assert!(analysis.mesh_closed_oriented);
    assert_eq!(analysis.analysis_error_bound_mm, None);
    assert_eq!(
        analysis.mesh.positions,
        analysis
            .positions_mm_f64
            .iter()
            .map(|&x| x as f32)
            .collect::<Vec<_>>()
    );
    if target.is_none() {
        assert_eq!(crate::mesh_profile(model).unwrap(), analysis.mesh);
    }

    let mut triangles = analysis
        .positions_mm_f64
        .chunks_exact(9)
        .zip(&analysis.triangle_face_occurrence_indices);
    let mut occurrences = analysis.source_faces.iter().enumerate();
    let mut solids = Vec::new();
    for (solid_index, (topology, shell)) in model.shells().enumerate() {
        let faces = tessellation::tessellate(
            topology,
            shell,
            analysis.deflection_mm,
            analysis.angular_rad,
        )
        .expect("analytic fixture must use direct tessellation")
        .into_iter()
        .map(crate::qualification::proof_geometry::polygon)
        .collect::<Vec<_>>();
        for (face_index, (poly, face)) in faces.iter().zip(shell).enumerate() {
            let (occurrence, source) = occurrences.next().unwrap();
            assert_eq!(
                (source.solid_index, source.shell_index, source.face_index),
                (solid_index, 0, face_index)
            );
            assert_eq!(
                source.support_kind,
                match face.support {
                    brep::Support::Plane { .. } => "PLANE",
                    brep::Support::Round { .. } => "REVOLUTION_SURFACE",
                }
            );
            assert!(poly.faces().triangle_iter().count() > 0);
            for tri in poly.faces().triangle_iter() {
                let (points, &tag) = triangles.next().unwrap();
                assert_eq!(tag, occurrence);
                assert_eq!(
                    points,
                    tri.map(|v| {
                        let p = poly.positions()[v.pos];
                        [p.x, p.y, p.z]
                    })
                    .as_flattened()
                );
            }
        }
        solids.push(faces);
    }
    assert!(triangles.next().is_none());
    assert!(occurrences.next().is_none());
    solids
}

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
    assert_eq!(model.test_solids().len(), 1);
    let solid = &model.test_solids()[0];
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
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-3)
        .expect("complete periodic face tessellation");
    let expected = 1280. - 4. * std::f64::consts::PI;
    assert!(
        (mesh.volume() - expected).abs() < 0.065,
        "{} != {expected}",
        mesh.volume()
    );
    crate::mesh_profile(&model).expect("production mesh path");
    let step = step_text(&model);
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1, "{step}");
    assert!(!step.contains("B_SPLINE_SURFACE"));
    assert_eq!(StepRecords::parse(&step).named("CIRCLE").count(), 4);
    let imported = assert_step_round_trip(&step, [[0., 0., 0.], [20., 16., 4.]], 7);
    assert!(imported.accuracy().mesh_closed_oriented);
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
) -> (crate::ExactModel, String) {
    if std::env::var_os("LAYERED_013_EVIDENCE_OUTPUT").is_some() {
        retain_evidence(
            &format!("workloads/verified-{}.json", expected.to_bits()),
            &serde_json::to_vec_pretty(input).unwrap(),
        );
    }
    let model = build_layered_case(input).expect("layered exact case");
    assert_eq!(model.test_solids().len(), 1, "one connected material solid");
    let solid = &model.test_solids()[0];
    check_solid(solid);
    let mesh =
        shell_to_polygon_strict(&solid.boundaries()[0], 1e-3).expect("every exact face meshes");
    assert!(
        (mesh.volume() - expected).abs() < 0.065,
        "volume {} != analytic {expected}",
        mesh.volume()
    );
    let vertices = solid
        .boundaries()
        .iter()
        .flat_map(|shell| shell.vertex_iter())
        .map(|vertex| vertex.point());
    assert_eq!(point_bounds(vertices), bounds, "authored outer bounds");
    for (actual, expected) in point_bounds(mesh.positions().iter().copied())
        .into_iter()
        .flatten()
        .zip(bounds.into_iter().flatten())
    {
        assert!(
            (actual - expected).abs() <= 1e-12,
            "reference mesh bound {actual} != {expected}"
        );
    }
    for &(point, inside) in samples {
        material(&mesh, point, inside);
    }
    crate::mesh_profile(&model).expect("existing production mesh path");
    let step = step_text(&model);
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
        step.matches("CONICAL_SURFACE(").count(),
        cones,
        "analytic conical support"
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
    (model, step)
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
fn direct_band_walls_follow_independent_boss_hole_and_cone_sections() {
    let mut boss = plate();
    boss.body.kind = CaseKind::Tray;
    boss.body.thickness = 2.;
    boss.body.wall_height = Some(4.);
    boss.regions[0].cavities = vec![rectangle(2., 2., 16., 12.)];
    let mount = &mut boss.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    let mut cone = plate();
    cone.regions[0].mounts.clear();
    cone.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "countersink".into(),
        at: Vec2 { x: 10., y: 8. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    let is_cone = |input: &PreparedCaseIR| input.body.kind == CaseKind::Plate;
    for (input, exact_volume, test_points) in [
        (
            boss,
            1152. + 10.75 * std::f64::consts::PI,
            [
                ([10., 8., 1.], false),
                ([10., 8., 4.], false),
                ([11., 8., 4.], true),
                ([13., 8., 4.], false),
            ],
        ),
        (
            cone,
            1280. - 14. * std::f64::consts::PI / 3.,
            [
                ([10., 8., 1.], true),
                ([10., 8., 3.], false),
                ([11.7, 8., 2.5], false),
                ([11.7, 8., 3.5], true),
            ],
        ),
    ] {
        let model = crate::build_case(&input).unwrap();
        let coarse = direct_analysis(&model, None);
        let tolerance = 0.0005;
        let fine = direct_analysis(&model, Some(tolerance));
        let [coarse] = coarse.as_slice() else {
            panic!("expected one material solid")
        };
        let [fine] = fine.as_slice() else {
            panic!("expected one material solid")
        };
        let volume = |faces: &[PolygonMesh]| faces.iter().map(PolygonMesh::volume).sum::<f64>();
        let fine_error = (volume(fine) - exact_volume).abs();
        assert!(fine_error < 0.025, "{fine_error}");
        assert!(fine_error < (volume(coarse) - exact_volume).abs());
        for (p, inside) in test_points {
            let expected = if inside { 1. } else { 0. };
            assert!((mesh_winding(fine, Point3::from(p)) - expected).abs() < 1e-8);
        }
        let shell = &model.test_solids()[0].boundaries()[0];
        for (mesh, deflection) in [(coarse, 0.1), (fine, tolerance)] {
            for (native, face) in shell.face_iter().zip(mesh) {
                if !matches!(native.surface(), Surface::RevolutionSurface(_)) {
                    continue;
                }
                let radial = |p: Point3| Vector2::new(p.x - 10., p.y - 8.);
                let r = if radial(face.positions()[0]).magnitude() < 1. {
                    0.5
                } else {
                    2.
                };
                let radius = |z| if is_cone(&input) { 3. - z / 2. } else { r };
                for triangle in face.faces().triangle_iter() {
                    let points = triangle.map(|v| face.positions()[v.pos]);
                    for (a, b) in points.into_iter().zip(points.into_iter().cycle().skip(1)) {
                        let (u, v) = (radial(a), radial(b));
                        assert!((u.magnitude() - radius(a.z)).abs() < 1e-12);
                        assert!(u.perp_dot(v).atan2(u.dot(v)).abs() <= 0.5);
                        let midpoint_radius = ((u + v) / 2.).magnitude();
                        assert!((radius((a.z + b.z) / 2.) - midpoint_radius).abs() <= deflection);
                    }
                }
            }
        }
        if is_cone(&input) {
            use crate::qualification::{SourceQualification, prepare_generated_analysis};
            assert!(matches!(
                prepare_generated_analysis(&model, Some(0.01))
                    .unwrap()
                    .source_qualification(),
                SourceQualification::Unknown { .. }
            ));
        }
    }
}

#[test]
fn opening_subdivides_a_boss_arc_and_its_horizontal_caps() {
    let mut input = plate();
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(6.);
    mount.height = Some(3.);
    input.body.openings = Some(vec![CaseOpening {
        points: rectangle(11., 4., 4., 8.),
        z: 5.,
        height: 1.,
    }]);

    let model = crate::build_case(&input).expect("opening clips the circular wall in one band");
    let solids = direct_analysis(&model, Some(0.0005));
    let [faces] = solids.as_slice() else {
        panic!("expected one material solid");
    };
    // Integrate the right-hand circular segment removed by the one-mm-high opening.
    let removed = 9. * (1_f64 / 3.).acos() - 8_f64.sqrt();
    let expected = 1280. + (27. - 7. * 0.25) * std::f64::consts::PI - removed;
    let actual: f64 = faces.iter().map(|face| face.volume()).sum();
    assert!((actual - expected).abs() < 0.065, "{actual} != {expected}");
    for (probe, expected) in [
        ([12., 8., 4.5], 1.),
        ([12., 8., 5.5], 0.),
        ([12., 8., 6.5], 1.),
        ([10., 8., 5.5], 0.),
        ([9., 8., 5.5], 1.),
    ] {
        assert!((mesh_winding(faces, Point3::from(probe)) - expected).abs() < 1e-8);
    }

    // One sub-angular arc and its chord bound each cap in this thin cut.
    input.body.openings.as_mut().unwrap()[0].points = rectangle(6., 10.95, 8., 4.);
    let thin = crate::build_case(&input).unwrap();
    let direct = direct_analysis(&thin, None);
    assert_eq!(direct.len(), 1);
    assert!(mesh_winding(&direct[0], Point3::new(10., 10.975, 4.5)) > 0.999);
    assert!(mesh_winding(&direct[0], Point3::new(10., 10.975, 5.5)).abs() < 1e-8);
}

#[test]
fn boss_no_wider_than_its_hole_is_rejected() {
    let mut input = plate();
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.boss_diameter = Some(mount.hole_diameter);
    mount.height = Some(3.);
    let Err(error) = build_layered_case(&input) else {
        panic!("boss without a wall was built");
    };
    assert!(error.contains("wider than its hole"), "{error}");
}
#[test]
fn far_near_collinear_hole_is_an_error_not_a_panic() {
    let dx = 11_463_935_597_161.09_f64;
    let dy = 15_139_984_008_548.072_f64;
    let inside_y = f64::from_bits(dy.to_bits() + 1);
    let hole_x = f64::from_bits(dx.to_bits() - 5);
    let lower_y = f64::from_bits(inside_y.to_bits() - 5);
    let upper_y = f64::from_bits(lower_y.to_bits() + 1);
    let v = |x, y| Vec2 { x, y };
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.regions[0].outer = vec![v(0., 0.), v(2. * dx, 2. * dy), v(-2. * dy, 2. * dx)];
    input.regions[0].holes = vec![vec![
        v(dx, inside_y),
        v(hole_x, lower_y),
        v(hole_x, upper_y),
    ]];
    let Err(error) = build_layered_case(&input) else {
        panic!("out-of-range ring was built");
    };
    assert!(error.contains("'plate'"), "{error}");
}
#[test]
fn non_finite_ring_point_is_rejected() {
    let mut input = plate();
    input.regions[0].outer[0].x = f64::NAN;
    assert!(build_layered_case(&input).is_err());
}
#[test]
fn out_of_range_hole_point_is_rejected() {
    let mut input = plate();
    input.regions[0].holes = vec![vec![
        Vec2 { x: 1., y: 1. },
        Vec2 {
            x: 2_000_000.,
            y: 1.,
        },
        Vec2 { x: 1., y: 2. },
    ]];
    let Err(error) = build_layered_case(&input) else {
        panic!("out-of-range hole was built");
    };
    assert!(error.contains("'plate'"), "{error}");
}
#[test]
fn invalid_mount_positions_are_rejected() {
    for (x, y) in [(f64::NAN, 8.), (10., 2_000_000.)] {
        let mut input = plate();
        input.regions[0].mounts[0].at = Vec2 { x, y };
        let Err(error) = build_layered_case(&input) else {
            panic!("invalid mount was built: {x:?}, {y:?}");
        };
        assert!(error.contains("'plate'"), "{error}");
    }
}
#[test]
fn mount_exactly_at_the_limit_passes_the_coordinate_check() {
    let mut input = plate();
    input.regions[0].outer = rectangle(0., 0., 1_000_000., 16.);
    input.regions[0].mounts[0].at.x = 1_000_000.;
    // Geometry at the edge may fail for other reasons; only the coordinate check is under test.
    if let Err(error) = build_layered_case(&input) {
        assert!(!error.contains("not finite or lies beyond"), "{error}");
    }
}
#[test]
fn ring_vertex_exactly_at_the_limit_builds() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.regions[0].outer = rectangle(0., 0., 1_000_000., 16.);
    assert!(build_layered_case(&input).is_ok());
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
    let (_, step) = verify(
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
    use crate::step_accuracy::{Countersink, certify_countersink};
    let bounds = [[0., 0., 0.], [20., 16., 4.]];
    let shape = Countersink {
        bounds,
        center: [10., 8.],
        z: [2., 4.],
        radii: [2., 1.],
    };
    let certificate = certify_countersink(&step, shape).expect("native circular trim certificate");
    assert_eq!(certificate.topology, [16, 20, 8]);
    let volume = 1280. - 14. * std::f64::consts::PI / 3.;
    assert!(certificate.volume[0] <= volume && certificate.volume[1] >= volume);
    let imported = assert_step_round_trip(&step, bounds, 8);
    assert!(imported.accuracy().mesh_closed_oriented);
    let records = StepRecords::parse(&step);
    let (cone, args) = records.named("CONICAL_SURFACE").next().unwrap();
    let radius = format!("#{cone} = CONICAL_SURFACE('', #{}, 1.0,", id(&args[1]));
    let wrong_radius = radius.replace(", 1.0,", ", 1.01,");
    assert_eq!(step.matches(&radius).count(), 1);
    assert!(certify_countersink(&step.replacen(&radius, &wrong_radius, 1), shape).is_err());
    let wrong_units = step.replacen(".RADIAN.", ".STERADIAN.", 1);
    assert!(certify_countersink(&wrong_units, shape).is_err());
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
    assert_eq!(model.test_solids().len(), 1);
    let solid = &model.test_solids()[0];
    assert_eq!(solid.boundaries().len(), 1);
    Solid::try_new(solid.boundaries().clone())
        .expect("closed consistently oriented native topology");
    assert!(solid.is_geometric_consistent());
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-3).unwrap();
    assert!((mesh.volume() - (1280. - 112. * std::f64::consts::PI / 3.)).abs() < 0.065);
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

#[test]
fn adoption_ring_island_product_removes_only_the_authored_blind_union() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../core/tests/fixtures/adoption-reachability.json"
    ))
    .unwrap();
    let baseline: PreparedCaseIR =
        serde_json::from_value(fixture["baselinePreparedBottom"].clone()).unwrap();
    let product: PreparedCaseIR = serde_json::from_value(
        fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["prototypeId"] == "opening-ring-island")
            .unwrap()["preparedBody"]
            .clone(),
    )
    .unwrap();
    let baseline = crate::build_case(&baseline).unwrap();
    assert_eq!(baseline.test_solids().len(), 1);
    let baseline =
        shell_to_polygon_strict(&baseline.test_solids()[0].boundaries()[0], 1e-3).unwrap();
    verify(
        &product,
        baseline.volume() - 164., // Unchanged independent cutter union: 82 mm² * 2 mm.
        [[-7.502, -7.502, -4.25], [107.502, 77.502, 8.2]],
        &[
            ([10., 8., -2.75], true),
            ([5., 8., -2.75], false),
            ([5., 8., -4.], true),
            ([1., 1., -2.75], true),
        ],
        8,
        0,
    );
}

#[test]
fn adoption_ring_island_through_cut_reports_two_explicit_material_solids() {
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/opening-ring-island.json"
    ))
    .unwrap();
    for opening in input.body.openings.as_mut().unwrap() {
        opening.z = 0.;
        opening.height = 4.;
    }
    let model = crate::build_case(&input).expect("legitimate post-opening material split");
    assert_eq!(
        model.test_solids().len(),
        2,
        "through cut disconnects the 32 mm² island"
    );
    let mut volumes = Vec::new();
    for solid in model.test_solids() {
        assert_eq!(solid.boundaries().len(), 1);
        Solid::try_new(solid.boundaries().clone()).unwrap();
        assert!(solid.is_geometric_consistent());
        volumes.push(
            shell_to_polygon_strict(&solid.boundaries()[0], 1e-3)
                .unwrap()
                .volume(),
        );
    }
    volumes.sort_by(f64::total_cmp);
    for (actual, expected) in volumes.into_iter().zip([128., 824.]) {
        assert!((actual - expected).abs() < 1e-8);
    }
    let step = step_text(&model);
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 2);
    assert_eq!(step.matches("CLOSED_SHELL(").count(), 2);
    assert!(!step.contains("B_SPLINE_SURFACE"));
    let facts = crate::step_accuracy::inspect(&step).unwrap();
    assert_eq!(
        facts
            .iter()
            .filter(|solid| solid.contains([10., 8., 2.]))
            .count(),
        1
    );
    assert!(facts.iter().all(|solid| !solid.contains([5., 8., 2.])));
}

#[test]
fn adoption_ring_island_retains_disjoint_through_holes_in_either_opening_order() {
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/opening-ring-island.json"
    ))
    .unwrap();
    input.regions[0].mounts.push(Mount {
        id: "disjoint-round-through-hole".into(),
        at: Vec2 { x: 18., y: 8. },
        kind: MountKind::Hole,
        hole_diameter: 1.,
        boss_diameter: None,
        height: None,
    });
    let rectangle = CaseOpening {
        points: rectangle(1., 1., 1., 1.),
        z: 0.,
        height: 4.,
    };
    let ring_openings = input.body.openings.clone().unwrap();
    for &(reverse_ring, rectangle_first) in &[(false, true), (true, false)] {
        let mut openings = ring_openings.clone();
        if reverse_ring {
            openings.reverse();
        }
        openings.insert(if rectangle_first { 0 } else { 2 }, rectangle.clone());
        input.body.openings = Some(openings);
        verify(
            &input,
            1116. - std::f64::consts::PI - 4., // Independent Ø1 × 4 mm bore and 1 × 1 × 4 mm cut.
            [[0., 0., 0.], [20., 16., 4.]],
            &[
                ([18., 8., 3.], false),
                ([18., 8., 1.], false),
                ([19., 8., 3.], true),
                ([1.5, 1.5, 3.], false),
                ([1.5, 1.5, 1.], false),
                ([2.5, 1.5, 3.], true),
                ([10., 8., 3.], true),
                ([5., 8., 3.], false),
                ([5., 8., 1.], true),
            ],
            1,
            0,
        );
    }
}

#[test]
fn adoption_ring_island_retains_another_owned_cut_in_either_authoring_order() {
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/adoption/opening-ring-island.json"
    ))
    .unwrap();
    input.body.openings.as_mut().unwrap().push(CaseOpening {
        points: rectangle(8., 7., 4., 2.),
        z: 2.,
        height: 2.,
    });
    for _ in 0..2 {
        verify(
            &input,
            1100., // Original 164 mm³ cut plus an independent 8 mm² * 2 mm island pocket.
            [[0., 0., 0.], [20., 16., 4.]],
            &[
                ([10., 8., 3.], false),
                ([7., 8., 3.], true),
                ([5., 8., 3.], false),
                ([10., 8., 1.], true),
            ],
            0,
            0,
        );
        input.body.openings.as_mut().unwrap().reverse();
    }
}

fn inspect_cone_step(text: &str, center: [f64; 2], low: f64, high: f64, r0: f64, r1: f64) {
    let records = StepRecords::parse(text);
    let [(surface, support)] = records.named("CONICAL_SURFACE").collect::<Vec<_>>()[..] else {
        panic!("expected one conical support");
    };
    let placement = records.args(id(&support[1]), "AXIS2_PLACEMENT_3D");
    let origin = records.xyz(id(&placement[1]), "CARTESIAN_POINT");
    let axis = records.xyz(id(&placement[2]), "DIRECTION");
    assert_eq!(origin, [center[0], center[1], high]);
    assert_eq!(axis, [0., 0., -1.]);
    assert_eq!(num(&support[2]), r1);
    assert!((num(&support[3]).tan() * (high - low) - (r0 - r1)).abs() < 1e-12);
    let radius = |z: f64| r0 + (r1 - r0) * (z - low) / (high - low);
    let (_, face) = records
        .named("ADVANCED_FACE")
        .find(|(_, p)| id(&p[2]) == surface)
        .expect("cone face is actually used");
    assert_eq!(face[3], step_p21::ast::Parameter::Enumeration("F".into()));
    let mut zs = vec![];
    for bound in list(&face[1]) {
        let bound = records.args(id(bound), "FACE_BOUND");
        let wire = records.args(id(&bound[1]), "EDGE_LOOP");
        for oriented in list(&wire[1]) {
            let edge = records.args(
                id(&records.args(id(oriented), "ORIENTED_EDGE")[3]),
                "EDGE_CURVE",
            );
            let circle = records.args(id(&edge[3]), "CIRCLE");
            let placement = records.args(id(&circle[1]), "AXIS2_PLACEMENT_3D");
            let origin = records.xyz(id(&placement[1]), "CARTESIAN_POINT");
            assert_eq!(origin[..2], center);
            assert_eq!(num(&circle[2]), radius(origin[2]));
            for vertex in [&edge[1], &edge[2]] {
                let point = records.xyz(
                    id(&records.args(id(vertex), "VERTEX_POINT")[1]),
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
    let lock = include_str!("../../../Cargo.lock");
    let mut deps_by_pkg: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for block in lock.split("[[package]]").skip(1) {
        let mut name = None;
        let mut in_deps = false;
        let mut deps = Vec::new();
        for line in block.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("name = \"") {
                name = rest.strip_suffix('"');
            } else if line == "dependencies = [" {
                in_deps = true;
            } else if in_deps {
                if line == "]" {
                    in_deps = false;
                } else if let Some(dep) = line
                    .trim_end_matches(',')
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                {
                    deps.push(dep.split_whitespace().next().unwrap_or(dep));
                }
            }
        }
        if let Some(pkg) = name {
            deps_by_pkg.entry(pkg).or_default().extend(deps);
        }
    }
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec!["boardstudio-cad", "boardstudio-cad-wasm"];
    while let Some(pkg) = stack.pop() {
        if visited.insert(pkg)
            && let Some(deps) = deps_by_pkg.get(pkg)
        {
            stack.extend(deps.iter().copied());
        }
    }
    assert!(visited.contains("boardstudio-cad"));
    assert!(visited.contains("boardstudio-cad-wasm"));
    assert!(!visited.contains("monstertruck-solid"));
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
    let vertices: Vec<_> = model.test_solids()[0]
        .boundaries()
        .iter()
        .flat_map(|s| s.vertex_iter())
        .map(|v| v.point().z)
        .collect();
    assert!(vertices.iter().any(|&z| z.to_bits() == low.to_bits()));
    assert!(vertices.iter().any(|&z| z.to_bits() == 4f64.to_bits()));
    let step = step_text(&model);
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
    use step_p21::ast::Parameter;
    let records = StepRecords::parse(text);
    let mut result = Vec::new();
    for (_, face) in records.named("ADVANCED_FACE") {
        let Some((_, cylinder)) = records
            .named("CYLINDRICAL_SURFACE")
            .find(|(surface, _)| *surface == id(&face[2]))
        else {
            continue;
        };
        let sense = match &face[3] {
            Parameter::Enumeration(v) if v == "T" => true,
            Parameter::Enumeration(v) if v == "F" => false,
            _ => panic!("face same_sense"),
        };
        result.push((num(&cylinder[2]), sense));
    }
    result.sort_by(|a, b| a.0.total_cmp(&b.0));
    result
}
#[test]
fn exact_step_cylinder_face_senses_match_holes_and_boss_material() {
    let input = plate();
    let model = build_layered_case(&input).unwrap();
    let step = step_text(&model);
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
    let step = step_text(&model);
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
    let solid = &model.test_solids()[0];
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-3).unwrap();
    assert!(
        (mesh.volume() - (1280. - 1.5 * std::f64::consts::PI)).abs() < 0.065,
        "authored order volume {}",
        mesh.volume()
    );
    let step = step_text(&model);
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
    let direct = direct_analysis(&model, None);
    assert_eq!(direct.len(), 2);
    for faces in &direct {
        let volume: f64 = faces.iter().map(|face| face.volume()).sum();
        assert!((volume - 512.).abs() < 1e-8);
        assert!(mesh_winding(faces, Point3::new(10., 8., 2.)).abs() < 1e-8);
    }
    assert_eq!(model.test_solids().len(), 2);
    for solid in model.test_solids() {
        Solid::try_new(solid.boundaries().clone()).unwrap();
        let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-3).unwrap();
        assert!((mesh.volume() - 512.).abs() < 1e-8);
    }
    let step = step_text(&model);
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 2);
}

mod prepared_geometry;
