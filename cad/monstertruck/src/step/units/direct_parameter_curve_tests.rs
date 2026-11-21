use super::*;
use monstertruck_io::step::load::step_geometry::{Curve2D, StepParameterCurve};
use monstertruck_modeling::{
    Line, MetricSpace, ParametricCurve, ParametricSurface, Point2, Point3, Tolerance,
};

#[test]
fn normalization_preserves_plane_frames_in_faces_and_all_associations() {
    use monstertruck_io::step::load::step_geometry::{
        SurfaceCurveKind, SurfaceCurveRepresentation,
    };
    use monstertruck_modeling::Vector3;
    use monstertruck_topology::compress::{CompressedEdge, CompressedFace};

    for factor in [1.0, 25.4, 1000.0] {
        let origin = Point3::new(1e12, 1e12, 1e12);
        let plane = Plane::xy().transformed(Matrix4::new(
            1., 0., -1e-12, 0., 0., 1., 0., 0., 1e-12, 0., 1., 0., origin.x, origin.y, origin.z, 1.,
        ));
        let surface = Surface::ElementarySurface(ElementarySurface::Plane(plane));
        let pcurve = StepParameterCurve::new(
            Box::new(Curve2D::Line(Line(
                Point2::new(0., 0.),
                Point2::new(1., 0.),
            ))),
            Box::new(surface.clone()),
        );
        let mut shell = Shell {
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
            vertices: vec![origin, origin + Vector3::unit_x()],
            edges: vec![
                CompressedEdge {
                    vertices: (0, 1),
                    curve: Curve3D::ParameterCurve(pcurve.clone()),
                },
                CompressedEdge {
                    vertices: (0, 1),
                    curve: Curve3D::SurfaceCurve(SurfaceCurve3D::new(
                        SurfaceCurveKind::SurfaceCurve,
                        Box::new(Curve3D::Line(Line(origin, origin + Vector3::unit_x()))),
                        vec![
                            SurfaceCurveAssociatedGeometry::Surface(Box::new(surface.clone())),
                            SurfaceCurveAssociatedGeometry::ParameterCurve(pcurve),
                        ],
                        SurfaceCurveRepresentation::Curve3D,
                    )),
                },
            ],
            faces: vec![CompressedFace {
                boundaries: vec![],
                orientation: true,
                surface,
            }],
        };
        normalize_shell_to_mm(&mut shell, factor).unwrap();
        let check_plane = |surface: &Surface| {
            let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = surface else {
                panic!("normalization must retain the native Plane carrier");
            };
            assert_eq!(plane.origin(), origin * factor);
            assert_eq!(plane.axis_u(), Vector3::new(1., 0., -1e-12));
            assert_eq!(plane.axis_v(), Vector3::unit_y());
        };
        check_plane(&shell.faces[0].surface);
        let Curve3D::ParameterCurve(direct) = &shell.edges[0].curve else {
            unreachable!()
        };
        check_plane(direct.surface());
        assert_eq!(direct.curve().subs(1.).x, factor);
        let Curve3D::SurfaceCurve(associated) = &shell.edges[1].curve else {
            unreachable!()
        };
        for geometry in associated.associated_geometry() {
            match geometry {
                SurfaceCurveAssociatedGeometry::Surface(surface) => check_plane(surface),
                SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => {
                    check_plane(curve.surface());
                    assert_eq!(curve.curve().subs(1.).x, factor);
                }
            }
        }
    }
}

#[test]
fn normalization_keeps_selected_pcurve_active_scalar_labels() {
    use monstertruck_io::step::load::step_geometry::{
        SurfaceCurveKind, SurfaceCurveRepresentation,
    };
    let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy().transformed(
        Matrix4::from_translation(monstertruck_modeling::Vector3::new(10., 20., 30.)),
    )));
    let source = SurfaceCurve3D::new(
        SurfaceCurveKind::SurfaceCurve,
        Box::new(Curve3D::Line(Line(
            Point3::new(100., 100., 100.),
            Point3::new(101., 100., 100.),
        ))),
        vec![
            SurfaceCurveAssociatedGeometry::Surface(Box::new(surface.clone())),
            SurfaceCurveAssociatedGeometry::ParameterCurve(StepParameterCurve::new(
                Box::new(Curve2D::Line(Line(
                    Point2::new(2., 3.),
                    Point2::new(4., 6.),
                ))),
                Box::new(surface),
            )),
        ],
        SurfaceCurveRepresentation::ParameterCurve1,
    )
    .try_with_active_trim(Some((2., 10.)))
    .unwrap();
    for factor in [1., 25.4, 1000.] {
        let normalized =
            normalize_curve_with_plane_pcurves(Curve3D::SurfaceCurve(source.clone()), factor)
                .unwrap();
        let Curve3D::SurfaceCurve(normalized) = normalized else {
            panic!("lost surface curve")
        };
        assert_eq!(normalized.active_trim(), Some((2., 10.)));
        assert!(
            normalized
                .subs(2.)
                .distance(Point3::new(16., 29., 30.) * factor)
                < 1e-8
        );
        assert!(
            normalized
                .subs(10.)
                .distance(Point3::new(32., 53., 30.) * factor)
                < 1e-8
        );
        assert_eq!(source.active_trim(), Some((2., 10.)));
    }
    assert!(normalize_curve_with_plane_pcurves(Curve3D::SurfaceCurve(source), f64::MAX).is_err());
}

#[test]
fn direct_plane_parameter_curve_scales_uv_without_scaling_frame_axes() {
    let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
    let source = StepParameterCurve::new(
        Box::new(Curve2D::Line(Line(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        ))),
        Box::new(surface.clone()),
    );
    let expected = surface.subs(1.0, 0.0) * 1000.0;
    let normalized = normalize_parameter_curve(source.clone(), 1000.0);
    let (uv, _) = normalized.decompose();
    let actual_uv = uv.subs(1.0);
    let normalized_face = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
    let actual = normalized_face.subs(actual_uv.x, actual_uv.y);
    assert!(actual.near(&expected));
    assert!(actual.near(&Point3::new(1000.0, 0.0, 0.0)));

    // The previous generic transform scaled the pcurve plane but left its UV curve unchanged.
    let mut baseline = source;
    baseline.transform_by(Matrix4::from_scale(1000.0));
    let (uv, _) = baseline.decompose();
    let baseline_uv = uv.subs(1.0);
    let old_boundary = normalized_face.subs(baseline_uv.x, baseline_uv.y);
    assert!(!old_boundary.near(&expected));
}
