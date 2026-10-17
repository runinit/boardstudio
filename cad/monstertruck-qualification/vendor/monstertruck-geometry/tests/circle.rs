use monstertruck_geometry::prelude::*;
use proptest::prelude::*;
use std::f64::consts::{PI, TAU};
use std::ops::Bound;

#[test]
fn inverse_hint_retains_recorded_small_angle() {
    let expected = 1.554367822901914e-5_f64;
    let point = Point2::new(expected.cos(), expected.sin());
    let circle = UnitCircle::<Point2>::new();
    let parameter = circle.search_parameter(point, expected, 1).unwrap();
    assert_near2!(parameter, expected);
}

#[test]
fn inverse_near_axes_keeps_angles_winding_and_ranges() {
    use monstertruck_traits::v2;

    let circle = UnitCircle::<Point2>::new();
    let circle3 = UnitCircle::<Point3>::new();
    for angle in [1e-9, 1.554367822901914e-5, PI - 1e-9, PI + 1e-9, TAU - 1e-9] {
        let point = Point2::new(angle.cos(), angle.sin());
        let point3 = Point3::new(point.x, point.y, 0.0);
        for winding in -3..=3 {
            let expected = angle + winding as f64 * TAU;
            for hint in [
                SearchParameterHint1D::Parameter(expected),
                SearchParameterHint1D::Range(expected - 1e-4, expected + 1e-4),
            ] {
                assert_near2!(circle.search_parameter(point, hint, 1).unwrap(), expected);
                assert_near2!(circle3.search_parameter(point3, hint, 1).unwrap(), expected);
                for scale in [0.125, 1.0, 8.0] {
                    let scaled = Point2::from_vec(point.to_vec() * scale);
                    assert_near2!(
                        circle.search_nearest_parameter(scaled, hint, 1).unwrap(),
                        expected
                    );
                    let scaled3 = Point3::new(scaled.x, scaled.y, 3.0);
                    assert_near2!(
                        circle3.search_nearest_parameter(scaled3, hint, 1).unwrap(),
                        expected
                    );
                }
                let hint = match hint {
                    SearchParameterHint1D::Parameter(t) => v2::SearchParameterHint1D::Parameter(t),
                    SearchParameterHint1D::Range(a, b) => v2::SearchParameterHint1D::Range(a, b),
                    SearchParameterHint1D::None => unreachable!(),
                };
                assert_near2!(
                    v2::SearchParameter::<v2::D1<f64>>::search_parameter(&circle3, point3, hint, 1)
                        .unwrap(),
                    expected
                );
                assert_near2!(
                    v2::SearchNearestParameter::<v2::D1<f64>>::search_nearest_parameter(
                        &circle, point, hint, 1
                    )
                    .unwrap(),
                    expected
                );
            }
        }
    }
}

#[test]
fn inverse_keeps_axis_representatives_and_admission() {
    let circle = UnitCircle::<Point2>::new();
    for (point, expected) in [
        (Point2::new(1.0, 0.0), TAU),
        (Point2::new(1.0, -0.0), TAU),
        (Point2::new(0.0, 1.0), PI / 2.0),
        (Point2::new(-1.0, 0.0), PI),
        (Point2::new(-1.0, -0.0), PI),
        (Point2::new(0.0, -1.0), 3.0 * PI / 2.0),
    ] {
        // The exact +X seam keeps its existing no-hint TAU representative.
        assert_eq!(circle.search_parameter(point, None, 1), Some(expected));
        assert_eq!(
            circle.search_nearest_parameter(point, None, 1),
            Some(expected)
        );
        for winding in -3..=3 {
            let hint = expected + winding as f64 * TAU;
            assert_eq!(circle.search_parameter(point, hint, 1), Some(hint));
            assert_eq!(circle.search_nearest_parameter(point, hint, 1), Some(hint));
        }
    }
    assert!(circle.search_parameter(Point2::origin(), None, 1).is_none());
    assert!(
        circle
            .search_parameter(Point2::new(1.01, 0.0), None, 1)
            .is_none()
    );
    assert!(
        circle
            .search_nearest_parameter(Point2::origin(), None, 1)
            .is_none()
    );
    let circle3 = UnitCircle::<Point3>::new();
    assert!(
        circle3
            .search_parameter(Point3::new(1.0, 0.0, 0.01), None, 1)
            .is_none()
    );
    assert_eq!(
        circle.parameter_range(),
        (Bound::Included(0.0), Bound::Excluded(TAU))
    );
    assert_eq!(
        circle3.parameter_range(),
        (Bound::Included(0.0), Bound::Excluded(TAU))
    );
}

proptest! {
    #[test]
    fn search_parameter(t in 0f64..=(2.0 * PI)) {
        let circle = UnitCircle::<Point2>::new();
        let p = circle.subs(t);
        let s = circle.search_nearest_parameter(p, None, 1).unwrap();
        prop_assert_near!(s, t);
    }
    #[test]
    fn search_nearest_parameter(t in 0f64..=(2.0 * PI), a in 0.1f64..=5f64) {
        let circle = UnitCircle::<Point2>::new();
        let p = a * circle.subs(t);
        let s = circle.search_nearest_parameter(p, None, 1).unwrap();
        let q = a * circle.subs(s);
        prop_assert_near!(p, q);
    }

    /// With a `Parameter` hint outside the canonical `[0, TAU)` period, the
    /// search must return the parameter in the period nearest the hint rather
    /// than folding back into the first period.
    #[test]
    fn search_parameter_honors_hint(
        t in 0f64..=TAU,
        winding in -3i32..=3i32,
    ) {
        let circle = UnitCircle::<Point2>::new();
        let p = circle.evaluate(t);
        let hint = t + winding as f64 * TAU;
        let s = circle.search_parameter(p, hint, 1).unwrap();
        prop_assert_near2!(s, hint);
    }

    /// The nearest-parameter search honors the hint identically, and the
    /// returned parameter still evaluates back to the queried point.
    #[test]
    fn search_nearest_parameter_honors_hint(
        t in 0f64..=TAU,
        winding in -3i32..=3i32,
    ) {
        let circle = UnitCircle::<Point2>::new();
        let p = circle.evaluate(t);
        let hint = t + winding as f64 * TAU;
        let s = circle.search_nearest_parameter(p, hint, 1).unwrap();
        prop_assert_near2!(s, hint);
        prop_assert_near!(circle.evaluate(s), p);
    }

    #[test]
    fn to_nurbs(t0 in 0f64..=PI, t1 in PI..=(2.0 * PI)) {
        let circle = UnitCircle::<Point2>::new();
        let arc = TrimmedCurve::new(circle, (t0, t1));
        let bsp: NurbsCurve<_> = arc.to_same_geometry();
        prop_assert_near!(bsp.front(), arc.front());
        prop_assert_near!(bsp.back(), arc.back());
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let p = bsp.subs(t).to_vec();
            let der = bsp.der(t);
            prop_assert_near!(p.magnitude2(), 1.0);
            prop_assert!(der.dot(p).so_small());
        }
    }
}

#[test]
fn parameter_division() {
    let c = UnitCircle::<Point2>::new();
    let (_div, pts) = c.parameter_division(c.range_tuple(), 0.05);
    for a in pts.windows(2) {
        let p = a[0].midpoint(a[1]);
        assert!(p.to_vec().magnitude() > 0.95);
    }
}
