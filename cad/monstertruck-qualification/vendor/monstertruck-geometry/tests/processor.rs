use monstertruck_geometry::prelude::*;
use proptest::prelude::*;
#[test]
fn processed_circle_reversed_hint_stays_in_output_period() {
    use std::f64::consts::TAU;
    let curve =
        Processor::<_, Matrix3>::new(TrimmedCurve::new(UnitCircle::<Point2>::new(), (0.0, TAU)))
            .inverse();
    let expected = 0.2;
    let inner = TAU - expected;
    let point = Point2::new(inner.cos(), inner.sin());
    let actual = [
        curve.search_parameter(point, expected, 100).unwrap(),
        curve
            .search_nearest_parameter(point, expected, 100)
            .unwrap(),
    ];
    assert!(
        actual.into_iter().all(|t| (t - expected).abs() < 1e-12),
        "parameter/nearest={actual:?} expected={expected}"
    );
}

#[test]
fn processed_circle_v2_preserves_output_hint() {
    use monstertruck_traits::v2;
    use std::f64::consts::TAU;
    let curve = Processor::<_, Matrix3>::new(TrimmedCurve::new(
        UnitCircle::<Point2>::new(),
        (0.0, 4.0 * TAU),
    ));
    let expected = TAU + 0.2;
    let point = Point2::new(0.2_f64.cos(), 0.2_f64.sin());
    let hint = v2::SearchParameterHint1D::Parameter(expected);
    let actual = [
        v2::SearchParameter::<v2::D1<f64>>::search_parameter(&curve, point, hint, 100).unwrap(),
        v2::SearchNearestParameter::<v2::D1<f64>>::search_nearest_parameter(
            &curve, point, hint, 100,
        )
        .unwrap(),
    ];
    assert!(
        actual.into_iter().all(|t| (t - expected).abs() < 1e-12),
        "parameter/nearest={actual:?} expected={expected}"
    );
}

#[test]
fn processed_circle_hints_follow_3d_orientation_scale_and_range() {
    use monstertruck_traits::v2;
    use std::f64::consts::{PI, TAU};
    // All requested periods remain inside these genuine finite trims.
    for range in [(-4.0 * TAU, 4.0 * TAU), (-4.0 * TAU - 0.7, 4.0 * TAU + 1.3)] {
        for reverse in [false, true] {
            for scale in [0.125, 1.0, 8.0] {
                let transform = Matrix4::from_translation(Vector3::new(17.0, -29.0, 41.0))
                    * Matrix4::from_angle_x(Rad(0.7))
                    * Matrix4::from_scale(scale);
                let mut curve = Processor::with_transform(
                    TrimmedCurve::new(UnitCircle::<Point3>::new(), range),
                    transform,
                );
                if reverse {
                    curve.invert();
                }
                for angle in [0.2, 1.554367822901914e-5, PI - 1e-9, PI + 1e-9] {
                    for winding in -3..=3 {
                        let expected = angle + winding as f64 * TAU;
                        let inner = if reverse {
                            range.0 + range.1 - expected
                        } else {
                            expected
                        };
                        let point =
                            transform.transform_point(Point3::new(inner.cos(), inner.sin(), 0.0));
                        for hint in [
                            SearchParameterHint1D::Parameter(expected),
                            SearchParameterHint1D::Range(expected - 0.1, expected + 0.1),
                        ] {
                            let v2hint = match hint {
                                SearchParameterHint1D::Parameter(t) => {
                                    v2::SearchParameterHint1D::Parameter(t)
                                }
                                SearchParameterHint1D::Range(a, b) => {
                                    v2::SearchParameterHint1D::Range(a, b)
                                }
                                SearchParameterHint1D::None => unreachable!(),
                            };
                            for actual in [
                                curve.search_parameter(point, hint, 100).unwrap(),
                                curve.search_nearest_parameter(point, hint, 100).unwrap(),
                                v2::SearchParameter::<v2::D1<f64>>::search_parameter(&curve, point, v2hint, 100).unwrap(),
                                v2::SearchNearestParameter::<v2::D1<f64>>::search_nearest_parameter(&curve, point, v2hint, 100).unwrap(),
                            ] {
                                assert_near2!(actual, expected);
                                assert_near2!(curve.evaluate(actual), point);
                            }
                        }
                    }
                }
                let inner = 0.2_f64;
                let expected = if reverse {
                    range.0 + range.1 - inner
                } else {
                    inner
                };
                let point = transform.transform_point(Point3::new(inner.cos(), inner.sin(), 0.0));
                assert_near2!(curve.search_parameter(point, None, 100).unwrap(), expected);
                assert_near2!(
                    curve.search_nearest_parameter(point, None, 100).unwrap(),
                    expected
                );
                assert_near2!(
                    v2::SearchParameter::<v2::D1<f64>>::search_parameter(
                        &curve,
                        point,
                        v2::SearchParameterHint1D::None,
                        100
                    )
                    .unwrap(),
                    expected
                );
                assert_near2!(
                    v2::SearchNearestParameter::<v2::D1<f64>>::search_nearest_parameter(
                        &curve,
                        point,
                        v2::SearchParameterHint1D::None,
                        100
                    )
                    .unwrap(),
                    expected
                );
                assert_eq!(curve.range_tuple(), range);
            }
        }
    }
}

#[test]
fn processed_circle_hints_follow_nested_2d_reversal() {
    use monstertruck_traits::v2;
    use std::f64::consts::TAU;
    let transform = Matrix3::from_translation(Vector2::new(7.0, -11.0))
        * Matrix3::from_angle_z(Rad(0.3))
        * Matrix3::from_nonuniform_scale(2.0, 0.5);
    let inner = Processor::with_transform(
        TrimmedCurve::new(UnitCircle::<Point2>::new(), (-4.0 * TAU, 4.0 * TAU)),
        transform,
    )
    .inverse();
    let curve = Processor::<_, Matrix3>::new(inner).inverse();
    for winding in -3..=3 {
        let expected = 0.2 + winding as f64 * TAU;
        let point = transform.transform_point(Point2::new(expected.cos(), expected.sin()));
        for hint in [
            SearchParameterHint1D::Parameter(expected),
            SearchParameterHint1D::Range(expected - 0.1, expected + 0.1),
        ] {
            assert_near2!(curve.search_parameter(point, hint, 100).unwrap(), expected);
            assert_near2!(
                curve.search_nearest_parameter(point, hint, 100).unwrap(),
                expected
            );
        }
        let hint = v2::SearchParameterHint1D::Range(expected - 0.1, expected + 0.1);
        assert_near2!(
            v2::SearchParameter::<v2::D1<f64>>::search_parameter(&curve, point, hint, 100).unwrap(),
            expected
        );
        assert_near2!(
            v2::SearchNearestParameter::<v2::D1<f64>>::search_nearest_parameter(
                &curve, point, hint, 100
            )
            .unwrap(),
            expected
        );
    }
}

type PResult = std::result::Result<(), TestCaseError>;

fn exec_compatible_with_bspcurve(ycoords: [f64; 7], mat: [f64; 9]) -> PResult {
    let knot_vec = KnotVector::uniform_knot(3, 4);
    let control_points: Vec<Point3> = ycoords
        .into_iter()
        .enumerate()
        .map(|(i, y)| Point3::new(i as f64, y, 0.0))
        .collect();
    let mut curve = BsplineCurve::new(knot_vec, control_points);
    let mut processor = Processor::new(curve.clone());
    let mat = *<&Matrix3>::from(&mat);
    prop_assume!(!mat.determinant().so_small(), "omitted: {:?}", mat);

    curve.transform_by(mat);
    processor.transform_by(mat);
    prop_assert_eq!(curve.parameter_range(), processor.parameter_range());

    const N: usize = 100;
    for i in 0..=N {
        let t = i as f64 / N as f64;
        prop_assert_near!(ParametricCurve::subs(&curve, t), processor.subs(t));
        prop_assert_near!(ParametricCurve::der(&curve, t), processor.der(t));
        prop_assert_near!(ParametricCurve::der2(&curve, t), processor.der2(t));
    }

    curve.invert();
    processor.invert();
    prop_assert_eq!(curve.parameter_range(), processor.parameter_range());
    for i in 0..=N {
        let t = i as f64 / N as f64;
        prop_assert_near!(ParametricCurve::subs(&curve, t), processor.subs(t));
        prop_assert_near!(ParametricCurve::der(&curve, t), processor.der(t));
        prop_assert_near!(ParametricCurve::der2(&curve, t), processor.der2(t));
    }
    Ok(())
}

proptest! {
    #[test]
    fn compatible_with_bspcurve(
        ycoords in prop::array::uniform7(-10f64..10f64),
        mat in prop::array::uniform9(-2f64..2f64),
    ) {
        exec_compatible_with_bspcurve(ycoords, mat)?;
    }
}

fn exec_compatible_with_bspsurface(
    ycoords: [[f64; 7]; 7],
    mat: [f64; 9],
    (u, v): (f64, f64),
) -> PResult {
    let knot_vec = KnotVector::uniform_knot(3, 4);
    let knot_vecs = (knot_vec.clone(), knot_vec);
    let control_points: Vec<Vec<Point3>> = ycoords
        .into_iter()
        .enumerate()
        .map(|(i, arr)| {
            arr.into_iter()
                .enumerate()
                .map(|(j, y)| Point3::new(i as f64, j as f64, y))
                .collect()
        })
        .collect();

    let mut surface = BsplineSurface::new(knot_vecs, control_points);
    let mut processor = Processor::new(surface.clone());
    let mat = *<&Matrix3>::from(&mat);
    prop_assume!(!mat.determinant().so_small(), "omitted: {:?}", mat);

    surface.transform_by(mat);
    processor.transform_by(mat);
    assert_eq!(surface.range_tuple(), processor.range_tuple());

    let pt0 = ParametricSurface::subs(&surface, u, v);
    let pt1 = processor.subs(u, v);
    prop_assert_near!(pt0, pt1);
    let uder0 = surface.uder(u, v);
    let uder1 = processor.uder(u, v);
    prop_assert_near!(uder0, uder1);
    let vder0 = surface.vder(u, v);
    let vder1 = processor.vder(u, v);
    prop_assert_near!(vder0, vder1);
    let uuder0 = surface.uuder(u, v);
    let uuder1 = processor.uuder(u, v);
    prop_assert_near!(uuder0, uuder1);
    let uvder0 = surface.uvder(u, v);
    let uvder1 = processor.uvder(u, v);
    prop_assert_near!(uvder0, uvder1);
    let vvder0 = surface.vvder(u, v);
    let vvder1 = processor.vvder(u, v);
    prop_assert_near!(vvder0, vvder1);
    let n0 = surface.normal(u, v);
    let n1 = processor.normal(u, v);
    prop_assert_near!(n0, n1);

    surface.swap_axes();
    processor.invert();
    prop_assert_eq!(surface.range_tuple(), processor.range_tuple());
    let pt0 = ParametricSurface::subs(&surface, u, v);
    let pt1 = processor.subs(u, v);
    prop_assert_near!(pt0, pt1);
    let uder0 = surface.uder(u, v);
    let uder1 = processor.uder(u, v);
    prop_assert_near!(uder0, uder1);
    let vder0 = surface.vder(u, v);
    let vder1 = processor.vder(u, v);
    prop_assert_near!(vder0, vder1);
    let uuder0 = surface.uuder(u, v);
    let uuder1 = processor.uuder(u, v);
    prop_assert_near!(uuder0, uuder1);
    let uvder0 = surface.uvder(u, v);
    let uvder1 = processor.uvder(u, v);
    prop_assert_near!(uvder0, uvder1);
    let vvder0 = surface.vvder(u, v);
    let vvder1 = processor.vvder(u, v);
    prop_assert_near!(vvder0, vvder1);
    let n0 = surface.normal(u, v);
    let n1 = processor.normal(u, v);
    prop_assert_near!(n0, n1);
    Ok(())
}

proptest! {
    #[test]
    fn compatible_with_bspsurface(
        ycoords in prop::array::uniform7(prop::array::uniform7(-10f64..10f64)),
        mat in prop::array::uniform9(-2f64..2f64),
        (u, v) in (0f64..=1f64, 0f64..=1f64),
    ) {
        exec_compatible_with_bspsurface(ycoords, mat, (u, v))?;
    }
}
