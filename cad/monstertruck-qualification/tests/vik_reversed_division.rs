use monstertruck_io::step::load::Table;
use monstertruck_meshing::tessellation::{TessellationOptions, robust_cshell_triangulation_with};

#[test]
fn vik_source_faces_survive_angular_sampling_without_moving_shared_points() {
    for fixture in [
        include_str!("fixtures/vik-source-face6711.step"),
        include_str!("fixtures/vik-source-face6720.step"),
    ] {
        let table = Table::from_step(fixture).unwrap();
        table.entity_report.require_empty().unwrap();
        let (source, report) = table
            .to_compressed_shell_reported(table.shell.values().next().unwrap())
            .unwrap();
        report.require_lossless().unwrap();
        let options = TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        };
        let linear = robust_cshell_triangulation_with(&source, options);
        assert!(linear.faces[0].surface.is_some());
        let angular = robust_cshell_triangulation_with(
            &source,
            TessellationOptions {
                edge_angular_tolerance: Some(0.5),
                ..options
            },
        );
        let face = angular.faces[0]
            .surface
            .as_ref()
            .expect("valid source face must retain its complete boundary");
        assert!(!face.faces().is_empty());
        for (seed, refined) in linear.edges.iter().zip(&angular.edges) {
            assert_eq!(seed.vertices, refined.vertices);
            assert_eq!(seed.curve, refined.curve);
        }
    }
}

#[test]
fn reversed_processor_division_preserves_tiny_endpoint_labels() {
    use monstertruck_geometry::prelude::*;
    let range = (1.3322676295501878e-15, 6.283185307179588);
    let curve = Processor::<_, Matrix4>::new(TrimmedCurve::new(UnitCircle::<Point3>::new(), range))
        .inverse();
    for (parameters, _) in [
        curve.parameter_division(range, 0.0005),
        curve.try_parameter_division(range, 0.0005).unwrap(),
    ] {
        assert_eq!(parameters.first(), Some(&range.0));
        assert_eq!(parameters.last(), Some(&range.1));
    }
}
