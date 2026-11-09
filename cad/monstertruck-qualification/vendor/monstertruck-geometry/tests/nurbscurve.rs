use monstertruck_geometry::prelude::*;
use proptest::prelude::*;

#[test]
fn active_endpoint_derivatives_match_bezier_under_scaling_and_reversal() {
    for weight in [1.0, std::f64::consts::FRAC_1_SQRT_2] {
        let controls = vec![
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(weight, weight, weight),
            Vector3::new(0.0, 1.0, 1.0),
        ];
        let reference = NurbsCurve::new(BsplineCurve::new(
            KnotVector::bezier_knot(2),
            controls.clone(),
        ));
        for span in [1e-9_f64, 1.0, 1e6] {
            let outside = span.max(1.0);
            let curve = NurbsCurve::new(BsplineCurve::new(
                // Symmetric padding keeps reversal exact even for the narrow domain.
                KnotVector::from(vec![
                    -outside,
                    -span * 0.5,
                    -span * 0.5,
                    span * 0.5,
                    span * 0.5,
                    outside,
                ]),
                controls.clone(),
            ));
            for reverse in [false, true] {
                let (reference, curve) = if reverse {
                    (reference.inverse(), curve.inverse())
                } else {
                    (reference.clone(), curve.clone())
                };
                let (a, b) = curve.range_tuple();
                let (ra, rb) = reference.range_tuple();
                for s in [0.0, 0.125, 0.5, 1.0] {
                    let t = a + (b - a) * s;
                    let rt = ra + (rb - ra) * s;
                    assert!(curve.evaluate(t).distance(reference.evaluate(rt)) < 1e-14);
                    for rank in [1, 2] {
                        let expected = reference.derivative_n(rank, rt) / span.powi(rank as i32);
                        assert!(
                            (curve.derivative_n(rank, t) - expected).magnitude()
                                < 1e-12 * (1.0 + expected.magnitude()),
                            "weight={weight} span={span} reverse={reverse} s={s} rank={rank} actual={:?} expected={expected:?}",
                            curve.derivative_n(rank, t)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn discontinuous_rational_padded_endpoint_keeps_legacy_right_span() {
    let curve = NurbsCurve::new(BsplineCurve::new(
        KnotVector::from(vec![0.0, 0.5, 1.0, 1.0, 2.0]),
        vec![
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(2.0, 0.0, 2.0),
            Vector3::new(30.0, 0.0, 3.0),
        ],
    ));
    // A discontinuity has no rational derivative. In particular, do not mix
    // this legacy right value with incoming homogeneous derivative terms.
    assert_eq!(curve.evaluate(1.0), Point2::new(10.0, 0.0));
    assert_eq!(curve.derivative(1.0), Vector2::new(0.0, 0.0));
    assert!((curve.derivative(0.75) - Vector2::new(16.0 / 9.0, 0.0)).magnitude() < 1e-12);
}

#[test]
fn rational_curve_uses_its_active_domain_without_changing_geometry() {
    let knots = KnotVector::from(vec![-1.0, 0.0, 0.0, 1.0, 1.0, 2.0]);
    let controls = vec![
        Vector3::new(0., 0., 1.),
        Vector3::new(2., 2., 2.),
        Vector3::new(0., 0., 1.),
    ];
    let curve = NurbsCurve::new(BsplineCurve::new(knots.clone(), controls.clone()));
    assert_eq!(curve.range_tuple(), (0.0, 1.0));
    assert_eq!(curve.knot_vector(), &knots);
    assert_eq!(curve.control_points(), &controls);
    assert_eq!(curve.front(), Point2::new(0., 0.));
    assert_eq!(curve.back(), Point2::new(0., 0.));
    assert!(curve.subs(0.5).distance(Point2::new(2. / 3., 2. / 3.)) < 1e-14);
}

#[test]
fn nurbs_circle() {
    let knot_vec = KnotVector::bezier_knot(2);
    let control_points = vec![
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(0.0, 2.0, 2.0),
    ];
    let curve = NurbsCurve::new(BsplineCurve::new(knot_vec, control_points));

    const N: usize = 10;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        let p = curve.subs(t).to_vec();
        assert_near!(p.magnitude(), 1.0);
        let der = curve.der(t);
        assert!(p.dot(der).so_small());
    }
}

proptest! {
    #[test]
    fn test_der_n(
        t in 0f64..=1.0,
        n in 0usize..=4,
        degree in 2usize..=6,
        div in 1usize..=10,
        pts in prop::array::uniform16(prop::array::uniform3(-10f64..=10.0)),
        weights in prop::array::uniform16(0.5f64..=10.0),
    ) {
        prop_assume!(degree > n + 1);
        let knot_vec = KnotVector::uniform_knot(degree, div);
        let control_points = pts[0..degree + div]
            .iter()
            .zip(weights)
            .map(|(&p, w)| Vector4::new(p[0], p[1], p[2], w))
            .collect::<Vec<_>>();
        let bsp = NurbsCurve::new(BsplineCurve::new(knot_vec, control_points));

        const EPS: f64 = 1.0e-4;
        let der0 = bsp.der_n(n + 1, t);
        let der1 = (bsp.der_n(n, t + EPS) - bsp.der_n(n, t - EPS)) / (2.0 * EPS);
        prop_assert!((der0 - der1).magnitude() <= 0.01 * der0.magnitude());
    }

    #[test]
    fn test_ders(
        t in 0f64..=1.0,
        n in 0usize..=6,
        degree in 2usize..=6,
        div in 1usize..=10,
        pts in prop::array::uniform16(prop::array::uniform3(-10f64..=10.0)),
        weights in prop::array::uniform16(0.5f64..=10.0),
    ) {
        prop_assume!(degree > n + 1);
        let knot_vec = KnotVector::uniform_knot(degree, div);
        let control_points = pts[0..degree + div]
            .iter()
            .zip(weights)
            .map(|(&p, w)| Vector4::new(p[0], p[1], p[2], w))
            .collect::<Vec<_>>();
        let bsp = NurbsCurve::new(BsplineCurve::new(knot_vec, control_points));

        let ders0 = (0..=n).map(|i| bsp.der_n(i, t)).collect::<Vec<_>>();

        let ders1 =  bsp.ders(n, t);

        prop_assert_eq!(ders0.len(), ders1.len());

        let mut iter = ders0.into_iter().zip(&*ders1);
        iter.try_for_each(|(v0, v1)| {
            prop_assert_near!(v0, v1);
            Ok(())
        })?;
    }
}

fn exec_concat_positive_test(
    v0: [[f64; 3]; 8],
    v1: [f64; 8],
    t: f64,
    w: f64,
) -> std::result::Result<(), TestCaseError> {
    let mut part0 = NurbsCurve::new(BsplineCurve::new(
        KnotVector::uniform_knot(4, 4),
        v0.into_iter()
            .zip(v1)
            .map(|(v0, v1)| Vector3::from(v0).extend(v1))
            .collect(),
    ));
    let mut part1 = part0.cut(t);
    part1.transform_control_points(|vec| *vec *= w);
    prop_assert_near!(part0.back(), part1.front());
    concat_random_test(&part0, &part1, 10);
    Ok(())
}

proptest! {
    #[test]
    fn concat_positive_test(
        v0 in prop::array::uniform8(prop::array::uniform3(-10f64..10f64)),
        v1 in prop::array::uniform8(0.5f64..=10f64),
        // Strictly INTERIOR. `cut` at a domain end leaves one piece empty, and
        // the test's premise -- that the two pieces still meet, `part0.back()`
        // against `part1.front()` -- has no meaning when one of them has no
        // back. proptest shrank to exactly `t = 0.0`.
        t in 1e-3f64..=1.0f64 - 1e-3f64,
        w in -10f64..=10f64,
    ) {
        exec_concat_positive_test(v0, v1, t, w)?;
    }
}

#[test]
fn test_parameter_division() {
    let knot_vec = KnotVector::uniform_knot(2, 3);
    let control_points = vec![
        Vector4::new(0.0, 0.0, 0.0, 1.0),
        Vector4::new(2.0, 0.0, 0.0, 2.0),
        Vector4::new(0.0, 3.0, 0.0, 3.0),
        Vector4::new(0.0, 0.0, 2.0, 2.0),
        Vector4::new(1.0, 1.0, 1.0, 1.0),
    ];
    let curve = NurbsCurve::new(BsplineCurve::new(knot_vec, control_points));
    let tol = 0.01;
    let (div, pts) = curve.parameter_division(curve.range_tuple(), tol * 0.5);
    let knot_vec = curve.knot_vector();
    assert_eq!(knot_vec[0], div[0]);
    assert_eq!(knot_vec.range_length(), div.last().unwrap() - div[0]);
    for i in 1..div.len() {
        let pt0 = curve.subs(div[i - 1]);
        assert_eq!(pt0, pts[i - 1]);
        let pt1 = curve.subs(div[i]);
        assert_eq!(pt1, pts[i]);
        let value_middle = pt0.midpoint(pt1);
        let param_middle = curve.subs((div[i - 1] + div[i]) / 2.0);
        let dist = value_middle.distance(param_middle);
        assert!(dist < tol, "large distance: {dist}");
    }
}
