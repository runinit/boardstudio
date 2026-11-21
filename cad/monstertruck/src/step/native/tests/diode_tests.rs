use super::*;
use cgmath::{EuclideanSpace, Zero};

const DIODE: &str = include_str!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/Diode_1N4148W.step"
);

fn options(deflection: f64) -> StepImportOptions {
    StepImportOptions {
        target_deflection_mm: Some(deflection),
        ..Default::default()
    }
}

fn replace(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "{old}");
    source.replace(old, new)
}

fn check_shared_trims(model: &ImportedStep, asset: &Asset) {
    let exchange = parser::parse(DIODE).unwrap();
    let records = source_records(&exchange);
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let solid_ids: Vec<_> = asset.occurrences.iter().map(|(id, _, _)| *id).collect();
    let unit_by_solid = units::solid_representation_units(&solid_ids, &items, &factors).unwrap();
    let scene = scene::Scene::new(&exchange, &items, &factors, &solid_ids);
    let face_tag = |face_id| {
        model
            .face_occurrences()
            .iter()
            .position(|occ| model.source_faces()[occ.source_face].face_id == face_id)
            .unwrap()
    };
    for (edge_id, face_a, face_b) in [(653, 110, 352), (1092, 131, 1507)] {
        let a = face_tag(face_a);
        let b = face_tag(face_b);
        let index = model.face_occurrences()[a].occurrence;
        assert_eq!(model.face_occurrences()[b].occurrence, index);
        let (id, placement, _) = asset.occurrences[index];
        let world = placement * Matrix4::from_scale(unit_by_solid[&id]);
        let scale = world.transform_vector(Vector3::unit_x()).magnitude();
        let edge = Edge::read(edge_id, &records, &scene, &HashMap::new()).unwrap();
        let positions = edge
            .sample(
                model.accuracy().preview_deflection_mm / (2.0 * scale),
                1e-8 / scale,
            )
            .unwrap();
        let emitted = |tag| {
            model
                .preview_positions_f64()
                .as_chunks::<3>()
                .0
                .iter()
                .zip(model.triangle_faces())
                .filter(move |(_, target)| **target == tag)
                .flat_map(|(tri, _)| {
                    [(0, 1), (1, 2), (2, 0)]
                        .map(|(x, y)| (tri[x].map(f64::to_bits), tri[y].map(f64::to_bits)))
                })
                .collect::<HashSet<_>>()
        };
        let (face_a, face_b) = (emitted(a), emitted(b));
        for span in positions.windows(2) {
            let [p, q] = [span[0], span[1]].map(|p| {
                let p = world.transform_point(p);
                [p.x, p.y, p.z].map(f64::to_bits)
            });
            assert_ne!(face_a.contains(&(p, q)), face_a.contains(&(q, p)));
            assert_eq!(face_a.contains(&(p, q)), face_b.contains(&(q, p)));
            assert_eq!(face_a.contains(&(q, p)), face_b.contains(&(p, q)));
        }
    }
}

fn check_curved_facets(model: &ImportedStep, asset: &Asset) {
    let exchange = parser::parse(DIODE).unwrap();
    let records = source_records(&exchange);
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let solid_ids: Vec<_> = asset.occurrences.iter().map(|(id, _, _)| *id).collect();
    let unit_by_solid = units::solid_representation_units(&solid_ids, &items, &factors).unwrap();
    let scene = scene::Scene::new(&exchange, &items, &factors, &solid_ids);
    let mut supports = HashMap::new();
    for (tag, occurrence) in model.face_occurrences().iter().enumerate() {
        let source = &model.source_faces()[occurrence.source_face];
        // Include the eight cylinders adjoining the reduced rational trims.
        if source.support_kind != "SPHERICAL_SURFACE"
            && ![110, 352, 882, 1372, 1507, 1597, 1773, 2007].contains(&source.face_id)
        {
            continue;
        }
        let (id, world, _) = asset.occurrences[occurrence.occurrence];
        assert_eq!(id, source.solid_id);
        let world = world * Matrix4::from_scale(unit_by_solid[&id]);
        let surface = Surface::read(
            simple_record(&records, source.support_id).unwrap(),
            &scene,
            None,
        )
        .unwrap();
        let (radius, axis) = match surface.kind {
            SurfaceKind::Sphere(radius) => (radius, None),
            SurfaceKind::Cylinder(radius) => (
                radius,
                Some(
                    world
                        .transform_vector(surface.frame.z.truncate())
                        .normalize(),
                ),
            ),
            _ => unreachable!(),
        };
        let center = world.transform_point(surface.frame.transform_point(Point3::origin()));
        let radius = world.transform_vector(Vector3::unit_x()).magnitude() * radius;
        supports.insert(tag, (center, radius, axis));
    }
    assert_eq!(supports.len(), 16);
    for (triangle, tag) in model
        .preview_positions_f64()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(model.triangle_faces())
    {
        let Some(&(center, radius, axis)) = supports.get(tag) else {
            continue;
        };
        let [a, b, c] = triangle.map(|p| {
            let delta = Point3::from(p) - center;
            axis.map_or(delta, |axis| delta - axis * axis.dot(delta))
        });
        let sides = [(a, b), (b, c), (c, a)];
        let normal = (b - a).cross(c - a);
        if axis.is_none() {
            assert!(normal.dot(a) > 0.0);
        }
        // The extrema of distance to the center (or axis) over a facet
        // occur at a vertex, a projected edge point, or the planar foot.
        let foot = normal * (normal.dot(a) / normal.magnitude2());
        let closest = if normal.magnitude2() > 1e-30
            && sides
                .iter()
                .all(|&(x, y)| (y - x).cross(foot - x).dot(normal) >= 0.0)
        {
            foot.magnitude()
        } else {
            sides
                .into_iter()
                .map(|(x, y)| {
                    let d2 = (y - x).magnitude2();
                    if d2 == 0.0 {
                        return x.magnitude();
                    }
                    let t = (y - x).dot(-x) / d2;
                    (x + (y - x) * t.clamp(0.0, 1.0)).magnitude()
                })
                .fold(f64::INFINITY, f64::min)
        };
        assert!(radius - closest <= model.accuracy().preview_deflection_mm);
        for (x, y) in sides {
            assert!((x.magnitude() - radius).abs() < 1e-10);
            assert!(x.cross(y).magnitude().atan2(x.dot(y)) <= ANGULAR_TOLERANCE + 1e-12);
        }
    }
}

#[test]
fn bundled_diode_has_complete_native_preview_and_unchanged_qualification() {
    let mut models = Vec::new();
    for deflection in [TOLERANCE_MM, 0.001] {
        let options = options(deflection);
        let bytes = DIODE.as_bytes();
        let asset = native_candidate(bytes, options).unwrap();
        let model = native_model(bytes, options);
        assert_eq!(model.solid_count(), 4);
        assert_eq!(model.occurrence_count(), 4);
        assert_eq!(model.source_faces().len(), 73);
        assert_eq!(model.face_occurrences().len(), 73);
        assert_eq!(model.source_faces(), asset.source_faces);
        assert_eq!(model.face_occurrences(), asset.face_occurrences);
        assert_eq!(model.triangle_faces(), asset.triangle_faces);
        assert_eq!(model.preview_positions_f64(), asset.positions);
        assert_eq!(model.bounds(), asset.bounds);
        assert!(model.validate_invariants().is_ok());
        for (actual, expected) in model
            .bounds()
            .into_iter()
            .flatten()
            .zip([-1.825, -0.7075, -0.775, 1.825, 0.7425, 0.775])
        {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert!(matches!(
            crate::qualification::prepare_vendor_analysis(&model)
                .unwrap()
                .source_qualification(),
            crate::qualification::SourceQualification::Unknown { .. }
        ));
        check_curved_facets(&model, &asset);
        check_shared_trims(&model, &asset);
        models.push(model);
    }
    assert_eq!(models[0].bounds(), models[1].bounds());
    assert!(models[1].triangle_faces().len() > models[0].triangle_faces().len());
}

#[test]
fn bundled_diode_preserves_trim_and_bounds_in_representation_length_units() {
    let mm = "SI_UNIT ( .MILLI., .METRE. )";
    assert_eq!(DIODE.matches(mm).count(), 6);
    let scaled = DIODE.replace(mm, "SI_UNIT ( .CENTI., .METRE. )");
    let source = native_model(DIODE.as_bytes(), options(0.001));
    let scaled = native_model(scaled.as_bytes(), options(0.01));
    assert_eq!(source.source_faces(), scaled.source_faces());
    assert_eq!(source.triangle_faces(), scaled.triangle_faces());
    assert_eq!(
        source.preview_positions_f64().len(),
        scaled.preview_positions_f64().len()
    );
    for (actual, expected) in scaled
        .preview_positions_f64()
        .iter()
        .flatten()
        .chain(scaled.bounds().iter().flatten())
        .zip(
            source
                .preview_positions_f64()
                .iter()
                .flatten()
                .chain(source.bounds().iter().flatten()),
        )
    {
        assert!((actual - 10.0 * expected).abs() < 1e-11);
    }
}

#[test]
fn spherical_bounds_include_an_interior_world_coordinate_extremum() {
    let exchange = parser::parse(DIODE).unwrap();
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let scene = scene::Scene::new(&exchange, &items, &factors, &[938]);
    let mut catalogue = Vec::new();
    let solid = Solid::read(
        938,
        &source_records(&exchange),
        &scene,
        &HashMap::new(),
        &mut catalogue,
    )
    .unwrap();
    let face = solid
        .faces
        .iter()
        .find(|face| catalogue[face.source].face_id == 131)
        .unwrap();
    let SurfaceKind::Sphere(radius) = face.surface.kind else {
        unreachable!();
    };
    let center = face.surface.frame.transform_point(Point3::origin());
    let ring = &face.loops[0].edges;
    let x = ring
        .iter()
        .map(|(id, forward)| solid.edges[id].points[usize::from(!forward)] - center)
        .fold(Vector3::zero(), |sum, corner| sum + corner)
        .normalize();
    // Face #131's mean corner direction is strictly inside its spherical
    // triangle. Rotate that direction onto the queried world axis.
    let y = x.cross(Vector3::unit_z()).normalize();
    let z = x.cross(y);
    let world = Matrix4::from_cols(
        Vector3::new(x.x, y.x, z.x).extend(0.0),
        Vector3::new(x.y, y.y, z.y).extend(0.0),
        Vector3::new(x.z, y.z, z.z).extend(0.0),
        Point3::origin().to_homogeneous(),
    );
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for &(id, _) in ring {
        solid.edges[&id]
            .include_bounds(world, 1e-8, &mut bounds)
            .unwrap();
    }
    let maximum = world.transform_point(center).x + radius;
    assert!(maximum - bounds[1][0] > 1e-4);
    face.spherical_trim(radius, &solid.edges, 1e-8)
        .unwrap()
        .include_bounds(world, 1e-8, &mut bounds)
        .unwrap();
    assert!((bounds[1][0] - maximum).abs() < 1e-12);
}

#[test]
fn source_rational_conic_retains_the_full_span_in_either_edge_direction() {
    let reversed = replace(
        DIODE,
        "#653 = EDGE_CURVE ( 'NONE', #1972, #2005, #781, .T. ) ;",
        "#653 = EDGE_CURVE ( 'NONE', #2005, #1972, #781, .F. ) ;",
    );
    let read = |source: &str| {
        let exchange = parser::parse(source).unwrap();
        let items = units::representation_items(&exchange).unwrap();
        let factors = units::length_factors(&exchange).unwrap();
        let scene = scene::Scene::new(&exchange, &items, &factors, &[]);
        Edge::read(653, &source_records(&exchange), &scene, &HashMap::new()).unwrap()
    };
    let source = read(DIODE);
    let reverse = read(&reversed);
    let (chart, _, start, sweep) = source.ellipse(1e-8).unwrap();
    let (_, _, reverse_start, reverse_sweep) = reverse.ellipse(1e-8).unwrap();
    assert!((start + sweep - reverse_start).abs() < 1e-14);
    assert_eq!(sweep, -reverse_sweep);
    assert!((0.0..std::f64::consts::PI).contains(&sweep));
    assert_eq!(source.points, [reverse.points[1], reverse.points[0]]);
    let wrong_sense = replace(
        DIODE,
        "#653 = EDGE_CURVE ( 'NONE', #1972, #2005, #781, .T. ) ;",
        "#653 = EDGE_CURVE ( 'NONE', #1972, #2005, #781, .F. ) ;",
    );
    assert!(read(&wrong_sense).ellipse(1e-8).is_err());

    // Independent Bernstein evaluation of the four original weighted poles
    // from #781, versus the reduced conic at the corresponding circle angle.
    let poles = [
        [1.3500000000000003, 0.0, -0.7251217974870086],
        [
            1.3500000000000012,
            -1.266314256145179e-15,
            -0.7543691396444112,
        ],
        [
            1.3293691396444118,
            -1.266314256145179e-15,
            -0.7750000000000004,
        ],
        [1.3001217974870092, 0.0, -0.7750000000000002],
    ]
    .map(Vector3::from);
    let w = 0.8058833813891978;
    let half_angle = sweep * 0.5;
    for i in 0..=64 {
        let t = f64::from(i) / 64.0;
        let u = 1.0 - t;
        let weights = [
            u * u * u,
            3.0 * w * u * u * t,
            3.0 * w * u * t * t,
            t * t * t,
        ];
        let s = poles
            .into_iter()
            .zip(weights)
            .fold(Vector3::zero(), |sum, (pole, weight)| sum + pole * weight)
            / weights.into_iter().sum::<f64>();
        let theta = (half_angle.sin() * (t * t - u * u))
            .atan2(half_angle.cos() * (u * u + t * t) + 2.0 * u * t);
        let angle = start + half_angle + theta;
        let p = chart.transform_point(Point3::new(angle.cos(), angle.sin(), 0.0));
        assert!((p.to_vec() - s).magnitude() < 1e-12);
    }
}

#[test]
fn diode_refuses_changed_rational_and_spherical_source_geometry() {
    let bent = replace(
        DIODE,
        "#1824 = CARTESIAN_POINT ( 'NONE',  ( 1.350000000000001200, -1.266314256145179000E-015, -0.7543691396444112100 ) ) ;",
        "#1824 = CARTESIAN_POINT ( 'NONE',  ( 1.350000000000001200, 0.000001, -0.7543691396444112100 ) ) ;",
    );
    let sphere = replace(
        DIODE,
        "#147 = SPHERICAL_SURFACE ( 'NONE', #1713, 0.05000000000000000300 ) ;",
        "#147 = SPHERICAL_SURFACE ( 'NONE', #1713, 0.05000005 ) ;",
    );
    let sphere_sense = replace(
        DIODE,
        "#131 = ADVANCED_FACE ( 'NONE', ( #292 ), #147, .T. ) ;",
        "#131 = ADVANCED_FACE ( 'NONE', ( #292 ), #147, .F. ) ;",
    );
    for (source, reason) in [
        (
            bent,
            "native rational ellipse does not match its oriented finite endpoints",
        ),
        (sphere, "source spherical boundary is off its great circle"),
        (
            sphere_sense,
            "native spherical trim contradicts surface sense",
        ),
    ] {
        let options = options(0.001);
        let err = native_candidate(source.as_bytes(), options).err().unwrap();
        assert!(err.contains(reason), "{err}");
        if let Ok(model) = import_step_with_options(source.into_bytes(), options) {
            assert!(!model.solids.is_empty());
            assert_eq!(model.accuracy().analysis_error_bound_mm, None);
        }
    }
}
