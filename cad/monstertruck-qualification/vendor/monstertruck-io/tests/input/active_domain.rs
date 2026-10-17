use monstertruck_io::step::load::{
    BoundedCurveAny, BsplineCurveAny, BsplineCurveWithKnots, CurveAny, EdgeCurve,
    NonRationalBsplineCurve, RationalBsplineCurve, Table,
    step_geometry::{Curve3D, re_exports::*},
    step_p21::tables::IntoOwned,
};

// Unchanged records from Choc_V2_Red.step, SHA256 0542ee277eeb6988412e0780ae86b23d96fdd118f8242e0bf3224cff7dd7acc4.
const SOURCE: &str = include_str!("fixtures/choc-degree6-active.step");

fn raw_edge() -> EdgeCurve {
    let table = Table::from_step(SOURCE).unwrap();
    table.edge_curve[&27392].clone().into_owned(&table).unwrap()
}

#[test]
fn step_active_end_derivative_uses_incoming_source_span() {
    let curve = raw_edge().parse_curve3d().unwrap();
    let end = curve.range_tuple().1;
    // Independent rational Bezier endpoint formula on the source's one active
    // degree-six span: 6/span * (w5/w6) * (P6-P5).
    let expected = Vector3::new(0.0, -1.3488721329545263e-13, 0.09855342964495123);
    assert!((curve.derivative(end) - expected).magnitude() < 1e-12);
    assert!((curve.derivative_n(1, end) - expected).magnitude() < 1e-12);
}

fn rational(raw: &mut EdgeCurve) -> &mut RationalBsplineCurve {
    let CurveAny::BoundedCurve(bounded) = &mut raw.edge_geometry else {
        unreachable!()
    };
    let BoundedCurveAny::BsplineCurve(spline) = bounded.as_mut() else {
        unreachable!()
    };
    let BsplineCurveAny::RationalBsplineCurve(rational) = spline.as_mut() else {
        unreachable!()
    };
    rational
}

fn with_knots(raw: &mut EdgeCurve) -> &mut BsplineCurveWithKnots {
    let NonRationalBsplineCurve::BsplineCurveWithKnots(spline) =
        &mut rational(raw).non_rational_b_spline_curve
    else {
        unreachable!()
    };
    spline
}

#[test]
fn step_full_unclamped_rational_edge_keeps_geometry_and_active_range() {
    let table = Table::from_step(SOURCE).unwrap();
    assert_eq!(table.edge_curve.len(), 1);
    assert_eq!(table.vertex_point.len(), 1);
    let raw = table.edge_curve[&27392].clone().into_owned(&table).unwrap();
    assert_eq!(raw.edge_start, raw.edge_end);
    let source = Curve3D::try_from(&raw.edge_geometry).unwrap();
    let converted = raw.parse_curve3d().unwrap();
    assert_eq!(converted, source);
    assert_eq!(converted.range_tuple(), (0.0, 6.28318530718));
    let vertex = Point3::from(&raw.edge_start.vertex_geometry);
    assert!(converted.front().distance(vertex) < 1e-10);
    assert!(converted.back().distance(vertex) < 1e-10);
    let midpoint = Point3::new(
        1.4499999999998883,
        -1.908901187190366e-12,
        -2.2124999999974744,
    );
    assert!(converted.subs(3.14159265359).distance(midpoint) < 1e-12);
    let (parameters, points) = converted.parameter_division(converted.range_tuple(), 0.01);
    assert_eq!(
        (parameters[0], *parameters.last().unwrap()),
        (0.0, 6.28318530718)
    );
    assert!(points.len() >= 3);
    assert!(
        points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite() && point.z.is_finite())
    );
}

#[test]
fn step_active_domain_preserves_narrow_and_scaled_knot_intervals() {
    for span in [1e-9_f64, 1e-4, 1.0, 1e6] {
        let mut raw = raw_edge();
        let outside = span.max(1.0);
        with_knots(&mut raw).knots = vec![-outside, 0.0, span, 2.0 * outside];
        let source = Curve3D::try_from(&raw.edge_geometry).unwrap();
        let converted = raw.parse_curve3d().unwrap();
        assert_eq!(converted, source);
        assert_eq!(converted.range_tuple(), (0.0, span));
        let midpoint = Point3::new(
            1.4499999999998883,
            -1.908901187190366e-12,
            -2.2124999999974744,
        );
        assert!(
            converted.subs(span / 2.0).distance(midpoint) < 1e-12,
            "span={span}"
        );
        let (parameters, points) = converted.parameter_division(converted.range_tuple(), 0.01);
        assert_eq!((parameters[0], *parameters.last().unwrap()), (0.0, span));
        assert!(points.len() >= 3);
        assert!(
            points
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite() && point.z.is_finite())
        );
    }
}

#[test]
fn step_active_domain_keeps_reverse_geometry_sense() {
    let mut raw = raw_edge();
    let source = Curve3D::try_from(&raw.edge_geometry).unwrap();
    raw.same_sense = false;
    let converted = raw.parse_curve3d().unwrap();
    assert_eq!(converted.range_tuple(), (0.0, 6.28318530718));
    for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert!(
            converted
                .subs(6.28318530718 * fraction)
                .distance(source.subs(6.28318530718 * (1.0 - fraction)))
                < 1e-12
        );
    }
}

#[test]
fn step_active_domain_does_not_admit_invalid_weights_multiplicities_or_vertices() {
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut raw = raw_edge();
        rational(&mut raw).weights_data[1] = weight;
        assert!(raw.parse_curve3d().is_err(), "weight={weight}");
    }
    for multiplicities in [vec![1, 7, 7, 1], vec![1, 0, 6, 1]] {
        let mut raw = raw_edge();
        with_knots(&mut raw).knot_multiplicities = multiplicities;
        assert!(raw.parse_curve3d().is_err());
    }
    let mut raw = raw_edge();
    with_knots(&mut raw).control_points_list[1].coordinates[0] = f64::NAN;
    assert!(raw.parse_curve3d().is_err());
    let mut raw = raw_edge();
    raw.edge_start.vertex_geometry.coordinates[2] += 0.01;
    raw.edge_end.vertex_geometry.coordinates[2] += 0.01;
    assert!(raw.parse_curve3d().is_err());
}
