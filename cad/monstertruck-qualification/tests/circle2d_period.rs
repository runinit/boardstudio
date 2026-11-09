use monstertruck_geometry::prelude::*;
use monstertruck_modeling::{Conic2D, Curve2D};
use std::f64::consts::TAU;

#[test]
fn unit_circle_period_reaches_periodic_conic_and_plane_pcurve() {
    let circle = UnitCircle::<Point2>::new();
    assert_eq!(ParametricCurve::period(&circle), Some(TAU));

    let conic = Conic2D::Ellipse(Processor::<_, Matrix3>::new(TrimmedCurve::new(
        circle,
        (0.0, TAU),
    )));
    assert_eq!(ParametricCurve::period(&conic), Some(TAU));

    let pcurve = ParameterCurve::new(Curve2D::Conic(conic), Plane::xy());
    assert_eq!(ParametricCurve::period(&pcurve), Some(TAU));
}

#[test]
fn bounded_line_plane_pcurve_is_not_periodic() {
    let curve = Curve2D::Line(Line(Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)));
    let pcurve = ParameterCurve::new(curve, Plane::xy());

    assert_eq!(BoundedCurve::range_tuple(&pcurve), (0.0, 1.0));
    assert_eq!(ParametricCurve::period(&pcurve), None);
}
