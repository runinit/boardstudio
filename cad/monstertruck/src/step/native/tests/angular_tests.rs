use super::*;
use cgmath::Point3;

const EPSILON: f64 = 1e-8;

fn frame(origin: Point3<f64>, x: Vector3<f64>, y: Vector3<f64>) -> Matrix4 {
    Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        x.cross(y).extend(0.0),
        Vector3::new(origin.x, origin.y, origin.z).extend(1.0),
    )
}

fn edge(vertices: [u64; 2], points: [Point3<f64>; 2], curve: geometry::Curve) -> Edge {
    Edge {
        vertices,
        points,
        curve,
        carrier: None,
        finite_ellipse: None,
    }
}

fn face(kind: SurfaceKind, ring: &[(u64, bool)]) -> Face {
    Face {
        source: 0,
        surface: Surface {
            frame: Matrix4::identity(),
            kind,
        },
        same_sense: true,
        loops: vec![Boundary {
            explicit_outer: true,
            edges: ring.to_vec(),
        }],
    }
}

fn samples(edges: &HashMap<u64, Edge>, deflection: f64) -> trim::Samples {
    edges
        .iter()
        .map(|(&id, edge)| (id, edge.sample(deflection, EPSILON).unwrap()))
        .collect()
}

fn spherical_triangle(latitude: f64, trim_radius: f64) -> (Face, HashMap<u64, Edge>) {
    let points = [(-0.2_f64, 0.0_f64), (0.2, 0.0), (0.0, latitude)].map(|(lon, lat)| {
        Point3::new(lon.cos() * lat.cos(), lon.sin() * lat.cos(), lat.sin()) * trim_radius
    });
    let mut edges = HashMap::new();
    for (index, (a, b)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
        let [p, q] = [points[a], points[b]];
        let x = Vector3::new(p.x, p.y, p.z).normalize();
        let z = x.cross(Vector3::new(q.x, q.y, q.z)).normalize();
        edges.insert(
            index as u64,
            edge(
                [a as u64, b as u64],
                [p, q],
                geometry::Curve::Circle {
                    frame: frame(Point3::new(0.0, 0.0, 0.0), x, z.cross(x)),
                    radius: trim_radius,
                    forward: true,
                },
            ),
        );
    }
    (
        face(SurfaceKind::Sphere(1.0), &[(0, true), (1, true), (2, true)]),
        edges,
    )
}

#[test]
fn thin_sphere_refuses_facets_with_excess_normal_error() {
    let (face, edges) = spherical_triangle(0.001, 1.0);
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let result = face.mesh(
        &edges,
        &samples(&edges, 0.05),
        EPSILON,
        0.1,
        Matrix4::identity(),
        &mut bounds,
    );
    assert_eq!(
        result.err().as_deref(),
        Some("native spherical facet exceeds angular tolerance")
    );
}

#[test]
fn narrow_oblique_strip_refuses_facets_with_excess_normal_error() {
    use geometry::Curve;
    let heights = |t: f64| 0.001 + 2.0 * (1.0 - t.cos());
    let [a, b] = [-0.2_f64, 0.2].map(|t| Point3::new(t.cos(), t.sin(), 0.0));
    let [c, d] = [
        Point3::new(b.x, b.y, heights(0.2)),
        Point3::new(a.x, a.y, heights(-0.2)),
    ];
    let line = |origin| Curve::Line {
        origin,
        direction: Vector3::unit_z(),
        forward: true,
    };
    let edges = HashMap::from([
        (
            0,
            edge(
                [0, 1],
                [a, b],
                Curve::Circle {
                    frame: Matrix4::identity(),
                    radius: 1.0,
                    forward: true,
                },
            ),
        ),
        (1, edge([1, 2], [b, c], line(b))),
        (
            2,
            edge(
                [3, 2],
                [d, c],
                Curve::Ellipse {
                    frame: frame(
                        Point3::new(0.0, 0.0, 2.001),
                        Vector3::new(1.0, 0.0, -2.0).normalize(),
                        Vector3::unit_y(),
                    ),
                    major: 5.0_f64.sqrt(),
                    minor: 1.0,
                    forward: true,
                },
            ),
        ),
        (3, edge([0, 3], [a, d], line(a))),
    ]);
    let face = face(
        SurfaceKind::Cylinder(1.0),
        &[(0, true), (1, true), (2, false), (3, false)],
    );
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let result = face.mesh(
        &edges,
        &samples(&edges, 0.02),
        EPSILON,
        0.02,
        Matrix4::identity(),
        &mut bounds,
    );
    assert_eq!(
        result.err().as_deref(),
        Some("native oblique cylinder facet exceeds angular tolerance")
    );
}

#[test]
fn spherical_bounds_enclose_radial_projection_of_a_nearly_attached_trim() {
    let (face, edges) = spherical_triangle(1e-5, 1.0 - 5e-9);
    let world = Matrix4::from_angle_y(cgmath::Rad(-1e-6));
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for edge in edges.values() {
        edge.include_bounds(world, EPSILON, &mut bounds).unwrap();
    }
    let support_witness = world.transform_point(Point3::new(1.0, 0.0, 0.0));
    // The positive world pole is below the equator. The source maximum is
    // on the projected equatorial trim, beyond the authored circle's AABB.
    assert!(bounds[1][0] < support_witness.x);
    face.spherical_trim(1.0, &edges, EPSILON)
        .unwrap()
        .include_bounds(&edges, world, EPSILON, &mut bounds)
        .unwrap();
    assert!(
        bounds[1][0] >= support_witness.x,
        "{bounds:?}, {support_witness:?}"
    );
    let corner = |i| {
        let p = edges[&i].points[0];
        Vector3::new(p.x, p.y, p.z)
    };
    let center = (corner(0) + corner(1) + corner(2)).normalize();
    let fan_center = world.transform_point(Point3::new(center.x, center.y, center.z));
    assert!(bounds[1][0] >= fan_center.x, "{bounds:?}, {fan_center:?}");
}

#[test]
fn facet_angle_is_checked_after_placement_and_respects_mirrors() {
    let angle = 0.2_f64;
    let points = [
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(angle.cos(), angle.sin(), 0.0),
        Point3::new(angle.cos(), 0.0, angle.sin()),
    ];
    let radial = points.map(|p| Vector3::new(p.x, p.y, p.z));
    let fits = |world| super::super::facet::radial_facet_fits(points, radial, true, world);
    assert!(fits(Matrix4::identity()));
    assert!(fits(Matrix4::from_nonuniform_scale(-1.0, 1.0, 1.0)));
    assert!(!fits(Matrix4::from_nonuniform_scale(1.0, 0.05, 1.0)));
}

#[test]
fn translated_spherical_pole_retains_sub_ulp_source_radius() {
    let shift = 2.0_f64.powi(46);
    let center = Point3::new(shift, 0.0, 0.0);
    let radius = 1.0_f64 + 2.0_f64.powi(-9);
    let x = 63.0 / 64.0;
    let ring_radius = (radius * radius - x * x).sqrt();
    let offsets = [0.0, 1.0, 2.0].map(|step| {
        let angle = step * std::f64::consts::TAU / 3.0;
        Vector3::new(x, ring_radius * angle.cos(), ring_radius * angle.sin())
    });
    let mut edges = HashMap::new();
    for (id, (a, b)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
        let radial = offsets[a].normalize();
        let normal = radial.cross(offsets[b]).normalize();
        edges.insert(
            id as u64,
            edge(
                [a as u64, b as u64],
                [center + offsets[a], center + offsets[b]],
                geometry::Curve::Circle {
                    frame: frame(center, radial, normal.cross(radial)),
                    radius,
                    forward: true,
                },
            ),
        );
    }
    let mut face = face(
        SurfaceKind::Sphere(radius),
        &[(0, true), (1, true), (2, true)],
    );
    face.surface.frame = Matrix4::from_translation(Vector3::unit_x() * shift);
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let world = Matrix4::from_translation(Vector3::unit_x() * -shift);
    face.spherical_trim(radius, &edges, EPSILON)
        .unwrap()
        .include_bounds(&edges, world, EPSILON, &mut bounds)
        .unwrap();
    assert!(bounds[1][0] >= radius, "{bounds:?} omits pole at {radius}");
}
