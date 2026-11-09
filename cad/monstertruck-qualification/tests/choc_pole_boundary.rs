use monstertruck_io::step::load::Table;
use monstertruck_meshing::analyzers::WithPointCloud;
use monstertruck_meshing::{
    prelude::*,
    tessellation::{
        TessellationOptions, robust_cshell_triangulation_with, robust_triangulation_with,
    },
};
use std::collections::BTreeMap;

fn origin_triangle_distance(points: [Vector3; 3]) -> f64 {
    let mut distance2 = points
        .iter()
        .map(|p| p.magnitude2())
        .fold(f64::INFINITY, f64::min);
    for i in 0..3 {
        let a = points[i];
        let edge = points[(i + 1) % 3] - a;
        let t = (-a.dot(edge) / edge.magnitude2()).clamp(0.0, 1.0);
        distance2 = distance2.min((a + edge * t).magnitude2());
    }
    let normal = (points[1] - points[0]).cross(points[2] - points[0]);
    let projected = normal * (normal.dot(points[0]) / normal.magnitude2());
    if (0..3).all(|i| {
        (points[(i + 1) % 3] - points[i])
            .cross(projected - points[i])
            .dot(normal)
            >= 0.0
    }) {
        distance2 = distance2.min(projected.magnitude2());
    }
    distance2.sqrt()
}

fn check_corner(input: &str, expected_azimuth_span: f64, center: Point3, radius: f64) {
    println!("checking {}", input.lines().nth(2).unwrap());
    let table = Table::from_step(input).unwrap();
    table.entity_report.require_empty().unwrap();
    let (source, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    let mesh = robust_cshell_triangulation_with(
        &source,
        TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        },
    );
    let quantize =
        |point: Point3| [point.x, point.y, point.z].map(|value| (value * 1e7).round() as i64);
    let key = |a: Point3, b: Point3| {
        let a = quantize(a);
        let b = quantize(b);
        if a < b { [a, b] } else { [b, a] }
    };
    let expected = mesh
        .edges
        .iter()
        .flat_map(|edge| edge.curve.windows(2).map(|pair| key(pair[0], pair[1])))
        .collect::<std::collections::BTreeSet<_>>();
    let face = mesh.faces[0].surface.as_ref().unwrap();
    let referenced = face
        .faces()
        .triangle_iter()
        .flat_map(|triangle| triangle.map(|vertex| vertex.pos))
        .collect::<std::collections::BTreeSet<_>>();
    for edge in &mesh.edges {
        for sample in &edge.curve[1..edge.curve.len() - 1] {
            assert!(
                referenced
                    .iter()
                    .any(|index| face.positions()[*index] == *sample),
                "original interior boundary sample must remain exactly referenced: {sample:?}"
            );
        }
    }
    let mut actual = BTreeMap::<_, usize>::new();
    let mut uv_area = 0.0;
    let surface = &source.faces[0].surface;
    let mut solid_angle = 0.0_f64;
    let mut max_sagitta = 0.0_f64;
    let mut max_radius_error = 0.0_f64;
    let mut negative_solid_angle = 0.0_f64;
    let mut meridional_facets = 0;
    for triangle in face.faces().triangle_iter() {
        let uv = triangle.map(|vertex| face.uv_coords()[vertex.uv.unwrap()]);
        let a = uv[1] - uv[0];
        let b = uv[2] - uv[0];
        let area = a.x * b.y - a.y * b.x;
        assert!(area > 0.0, "reversed UV facet");
        uv_area += area * 0.5;
        let points = triangle.map(|vertex| face.positions()[vertex.pos]);
        let radial = points.map(|point| point - center);
        for p in radial {
            max_radius_error = max_radius_error.max((p.magnitude() - radius).abs());
        }
        max_sagitta = max_sagitta.max(radius - origin_triangle_distance(radial));
        let unit = radial.map(|p| p.normalize());
        let angle = 2.0
            * unit[0]
                .dot(unit[1].cross(unit[2]))
                .atan2(1.0 + unit[0].dot(unit[1]) + unit[1].dot(unit[2]) + unit[2].dot(unit[0]));
        if angle < 0.0 {
            negative_solid_angle -= angle;
        }
        if angle.abs() < 1e-12 {
            meridional_facets += 1;
        }
        solid_angle += angle;
        let centroid = (uv[0] + uv[1] + uv[2]) / 3.0;
        assert!(
            (points[1] - points[0])
                .cross(points[2] - points[0])
                .dot(source.faces[0].surface.normal(centroid.x, centroid.y))
                > 0.0,
            "facet disagrees with source normal"
        );
        for i in 0..3 {
            *actual
                .entry(key(
                    face.positions()[triangle[i].pos],
                    face.positions()[triangle[(i + 1) % 3].pos],
                ))
                .or_default() += 1;
        }
    }
    println!("retained nondegenerate UV area={uv_area}");
    // Radial source-to-chord and chord-to-source error is bounded by R minus
    // the minimum radius on the actual triangle, including meridional facets
    // whose infinite plane passes through the center outside the triangle.
    // All finite signed spherical images must form the source-area partition;
    // tiny negative roundoff is separately bounded and never changes meshing.
    assert!(
        negative_solid_angle < 1e-12,
        "folded spherical area: {negative_solid_angle}"
    );
    assert!(
        (solid_angle - expected_azimuth_span).abs() < 1e-10,
        "spherical source area changed: {solid_angle}"
    );
    assert!(
        max_sagitta + max_radius_error + 1e-12 <= 0.0005,
        "continuous radial source bound exceeds unchanged tolerance: {max_sagitta}"
    );
    let material_bound = solid_angle * (radius.powi(3) - (radius - max_sagitta).powi(3)) / 3.0;
    println!(
        "radius={radius} solid_angle={solid_angle} negative_solid_angle={negative_solid_angle} meridional_facets={meridional_facets} max_radial_error_bound={} max_vertex_radius_error={max_radius_error} radial_shell_volume_bound={material_bound}",
        max_sagitta + max_radius_error + 1e-12
    );
    let mut equator = source
        .vertices
        .iter()
        .filter_map(|point| {
            source.faces[0]
                .surface
                .search_parameter(*point, None::<(f64, f64)>, 100)
        })
        .filter(|(_, v)| v.abs() < 1e-12)
        .map(|(u, _)| u)
        .collect::<Vec<_>>();
    assert_eq!(equator.len(), 2);
    let equatorial_edge = mesh
        .edges
        .iter()
        .find(|edge| {
            edge.curve.iter().all(|point| {
                surface
                    .search_parameter(*point, None::<(f64, f64)>, 100)
                    .is_some_and(|(_, v)| v.abs() < 1e-12)
            })
        })
        .unwrap();
    let middle = surface
        .search_parameter(
            equatorial_edge.curve[equatorial_edge.curve.len() / 2],
            None::<(f64, f64)>,
            100,
        )
        .unwrap()
        .0;
    for angle in &mut equator {
        while *angle - middle > std::f64::consts::PI {
            *angle -= std::f64::consts::TAU;
        }
        while *angle - middle < -std::f64::consts::PI {
            *angle += std::f64::consts::TAU;
        }
    }
    let u_min = equator[0].min(equator[1]);
    let u_max = equator[0].max(equator[1]);
    assert!((u_max - u_min - expected_azimuth_span).abs() < 1e-12);
    let witnesses = [0.0, 0.13, 0.37, 0.61, 0.87, 1.0]
        .into_iter()
        .flat_map(|x| {
            [0.0, 0.13, 0.37, 0.61, 0.87, 0.95, 0.99, 1.0].map(move |y| {
                surface.evaluate(u_min + x * (u_max - u_min), y * std::f64::consts::FRAC_PI_2)
            })
        })
        .collect::<Vec<_>>();
    assert!(
        face.neighborhood_include(&witnesses, 0.0005),
        "quarter sphere source witnesses must be covered at original tolerance"
    );
    let dense = (0..=64)
        .flat_map(|i| {
            (0..=128).map(move |j| {
                surface.evaluate(
                    u_min + (u_max - u_min) * i as f64 / 64.0,
                    std::f64::consts::FRAC_PI_2 * (1.0 - (j as f64 / 128.0).powi(2)),
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        face.neighborhood_include(&dense, 0.0005),
        "dense independent polar source coverage must hold at original tolerance"
    );
    for segment in &expected {
        assert_eq!(
            actual.get(segment),
            Some(&1),
            "original shared boundary segment missing: {segment:?}"
        );
    }
    for (segment, count) in actual {
        assert_eq!(
            count,
            if expected.contains(&segment) { 1 } else { 2 },
            "unexpected open/interior edge: {segment:?}"
        );
    }
    let native = monstertruck_topology::Shell::extract(source).unwrap();
    let native = robust_triangulation_with(
        &native,
        TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        },
    );
    let native = native.face_iter().next().unwrap().surface();
    assert_eq!(
        native.as_ref().unwrap().faces().triangle_iter().count(),
        face.faces().triangle_iter().count()
    );
}

#[test]
fn source_sphere_corner_reuses_every_boundary_segment_without_internal_open_edges() {
    for (input, center) in [
        (
            include_str!("fixtures/choc-sphere-8955.step"),
            Point3::new(-0.5, 1.4, 2.1),
        ),
        (
            include_str!("fixtures/choc-sphere-8972.step"),
            Point3::new(0.5, 1.4, 2.1),
        ),
        (
            include_str!("fixtures/choc-sphere-9102.step"),
            Point3::new(-0.5, 1.4, 4.0),
        ),
        (
            include_str!("fixtures/choc-sphere-9201.step"),
            Point3::new(0.5, 1.4, 4.0),
        ),
    ] {
        check_corner(input, std::f64::consts::FRAC_PI_2, center, 0.1);
    }
    check_corner(
        include_str!("fixtures/choc-sphere-14981.step"),
        std::f64::consts::PI,
        Point3::new(1.2, -0.6, 4.75),
        0.05,
    );
}
