use monstertruck_io::step::load::Table;
use monstertruck_meshing::{
    prelude::*,
    tessellation::{
        TessellationOptions, robust_cshell_triangulation_with, robust_triangulation_with,
    },
};

fn no_collapsed_facets(mesh: &PolygonMesh) {
    let key = |point: monstertruck_modeling::Point3| {
        [point.x, point.y, point.z].map(|value| (value * 1e7).round() as i64)
    };
    assert!(!mesh.faces().is_empty());
    for triangle in mesh.faces().triangle_iter() {
        let points = triangle.map(|vertex| mesh.positions()[vertex.pos]);
        assert!(
            !(0..3).any(|i| key(points[i]) == key(points[(i + 1) % 3])),
            "collapsed source-endpoint facet: {points:?}"
        );
    }
}

#[test]
fn crenshaw_source_sphere_endpoint_keeps_a_valid_native_and_compressed_mesh() {
    let table = Table::from_step(include_str!("fixtures/crenshaw-sphere-corner.step")).unwrap();
    table.entity_report.require_empty().unwrap();
    let (source, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    let options = TessellationOptions {
        tolerance: 0.0005,
        ..Default::default()
    };
    let compressed = robust_cshell_triangulation_with(&source, options);
    no_collapsed_facets(compressed.faces[0].surface.as_ref().unwrap());
    let native = monstertruck_topology::Shell::extract(source).unwrap();
    let native = robust_triangulation_with(&native, options);
    for face in native.face_iter() {
        no_collapsed_facets(face.surface().as_ref().unwrap());
    }
}

#[test]
fn angular_source_cylinder_endpoint_does_not_add_a_collapsed_facet_or_move_shared_samples() {
    let table = Table::from_step(include_str!("fixtures/angular-cylinder-corner.step")).unwrap();
    table.entity_report.require_empty().unwrap();
    let (source, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    let mut options = TessellationOptions {
        tolerance: 0.0005,
        ..Default::default()
    };
    let linear = robust_cshell_triangulation_with(&source, options);
    options.edge_angular_tolerance = Some(0.5);
    let angular = robust_cshell_triangulation_with(&source, options);
    assert_eq!(linear.edges.len(), angular.edges.len());
    for (linear, angular) in linear.edges.iter().zip(&angular.edges) {
        assert_eq!(linear.curve, angular.curve);
    }
    let mesh = angular.faces[0].surface.as_ref().unwrap();
    no_collapsed_facets(mesh);
    assert_eq!(
        mesh.faces().triangle_iter().count(),
        linear.faces[0]
            .surface
            .as_ref()
            .unwrap()
            .faces()
            .triangle_iter()
            .count()
    );
}

#[test]
fn stable_public_sphere_inverse_retains_distinct_near_pole_direction() {
    use monstertruck_geometry::prelude::{
        ParametricSurface, SearchNearestParameter, SearchParameter, Sphere, TOLERANCE,
    };
    use monstertruck_modeling::{Point3, Vector3};
    let radius = 1000.0;
    let colatitude = 5e-7_f64;
    for center in [Point3::origin(), Point3::new(17.0, -29.0, 41.0)] {
        let sphere = Sphere::new(center, radius);
        let point =
            center + Vector3::new(-radius * colatitude.sin(), 0.0, radius * colatitude.cos());
        for nearest in [false, true] {
            let (u, v) = if nearest {
                sphere.search_nearest_parameter(point, None::<(f64, f64)>, 100)
            } else {
                sphere.search_parameter(point, None::<(f64, f64)>, 100)
            }
            .unwrap();
            let delta = sphere.evaluate(u, v) - point;
            assert!(delta.x.hypot(delta.y).hypot(delta.z) <= TOLERANCE);
        }
    }
}
