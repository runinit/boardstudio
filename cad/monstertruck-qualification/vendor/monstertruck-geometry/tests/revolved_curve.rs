use monstertruck_geometry::prelude::*;
use std::f64::consts::PI;

#[test]
fn revolve_test() {
    let pt0 = Point3::new(0.0, 2.0, 1.0);
    let pt1 = Point3::new(1.0, 0.0, 0.0);
    let curve = BsplineCurve::new(KnotVector::bezier_knot(1), vec![pt0, pt1]);
    let surface = RevolutionSurface::by_revolution(curve, Point3::origin(), Vector3::unit_y());
    const N: usize = 100;
    for i in 0..=N {
        for j in 0..=N {
            let u = i as f64 / N as f64;
            let v = 2.0 * PI * j as f64 / N as f64;
            let res = surface.subs(u, v);
            let ans = Point3::new(
                u * f64::cos(v) + (1.0 - u) * f64::sin(v),
                2.0 * (1.0 - u),
                -u * f64::sin(v) + (1.0 - u) * f64::cos(v),
            );
            assert_near!(res, ans);
            let res_uder = surface.uder(u, v);
            let ans_uder =
                Vector3::new(f64::cos(v) - f64::sin(v), -2.0, -f64::sin(v) - f64::cos(v));
            assert_near!(res_uder, ans_uder);
            let res_vder = surface.vder(u, v);
            let ans_vder = Vector3::new(
                -u * f64::sin(v) + (1.0 - u) * f64::cos(v),
                0.0,
                -u * f64::cos(v) - (1.0 - u) * f64::sin(v),
            );
            assert_near!(res_vder, ans_vder);
            let res_uuder = surface.uuder(u, v);
            let ans_uuder = Vector3::zero();
            assert_near!(res_uuder, ans_uuder);
            let res_uvder = surface.uvder(u, v);
            let ans_uvder =
                Vector3::new(-f64::sin(v) - f64::cos(v), 0.0, -f64::cos(v) + f64::sin(v));
            assert_near!(res_uvder, ans_uvder);
            let res_vvder = surface.vvder(u, v);
            let ans_vvder = Vector3::new(
                -u * f64::cos(v) - (1.0 - u) * f64::sin(v),
                0.0,
                u * f64::sin(v) - (1.0 - u) * f64::cos(v),
            );
            assert_near!(res_vvder, ans_vvder);
            let normal = surface.normal(u, v);
            assert!(normal.dot(res_uder).so_small());
            assert!(normal.dot(res_vder).so_small());
        }
    }
}

#[test]
fn search_parameter() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(1),
        vec![Point3::new(0.0, 2.0, 1.0), Point3::new(1.0, 0.0, 0.0)],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());
    let pt = Point3::new(-0.5, 1.0, 0.5);
    let (u, v) = surface.search_parameter(pt, Some((0.4, 1.2)), 100).unwrap();
    assert_near!(surface.subs(u, v), pt);
}

#[test]
fn search_parameter_with_fixed_points() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(2),
        vec![
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, -1.0, 0.0),
        ],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());

    let (u, v) = surface
        .search_parameter(Point3::new(0.0, 1.0, 0.0), Some((0.5, 0.3)), 10)
        .unwrap();
    assert_near!(Vector2::new(u, v), Vector2::new(0.0, 0.3));

    let (u, v) = surface
        .search_parameter(Point3::new(0.0, -1.0, 0.0), Some((0.5, 0.3)), 10)
        .unwrap();
    assert_near!(Vector2::new(u, v), Vector2::new(1.0, 0.3));
}

#[test]
fn search_nearest_parameter() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(1),
        vec![Point3::new(0.0, 2.0, 1.0), Point3::new(1.0, 0.0, 0.0)],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());
    let pt = surface.subs(0.4, 1.2) + 0.1 * surface.normal(0.4, 1.2);
    let (u, v) = surface
        .search_nearest_parameter(pt, Some((0.4, 1.2)), 100)
        .unwrap();
    assert_near!(Vector2::new(u, v), Vector2::new(0.4, 1.2));
}

#[test]
fn search_nearest_parameter_with_fixed_points() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(2),
        vec![
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, -1.0, 0.0),
        ],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());

    let (u, v) = surface
        .search_nearest_parameter(Point3::new(0.0, 2.0, 0.0), Some((0.5, 0.3)), 10)
        .unwrap();
    assert_near!(Vector2::new(u, v), Vector2::new(0.0, 0.3));

    let (u, v) = surface
        .search_nearest_parameter(Point3::new(0.0, -2.0, 0.0), Some((0.5, 0.3)), 10)
        .unwrap();
    assert_near!(Vector2::new(u, v), Vector2::new(1.0, 0.3));
}

#[test]
fn include_curve_normal() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(1),
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 2.0)],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());
    let parabola = BsplineCurve::new(
        KnotVector::bezier_knot(2),
        vec![
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(-1.0, 1.0, 0.0),
        ],
    );
    assert!(surface.include(&parabola));
}

#[test]
fn include_curve_abnormal0() {
    let line = BsplineCurve::new(
        KnotVector::bezier_knot(1),
        vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 2.0)],
    );
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_y());
    let parabola = BsplineCurve::new(
        KnotVector::bezier_knot(2),
        vec![
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 2.0),
            Point3::new(-1.0, 1.0, 0.0),
        ],
    );
    assert!(!surface.include(&parabola));
}

#[test]
fn include_curve_abnormal1() {
    let curve = NurbsCurve::new(BsplineCurve::new(
        KnotVector::bezier_knot(3),
        vec![
            Vector4::new(0.0, 3.0, 0.0, 1.0),
            Vector4::new(0.0, 3.0, 3.0, 0.5),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, 0.0, 3.0, 1.0),
        ],
    ));
    let pt0 = curve.subs(0.2);
    let pt1 = curve.subs(0.6);
    let surface = RevolutionSurface::by_revolution(curve, Point3::origin(), Vector3::unit_y());
    let line = BsplineCurve::new(KnotVector::bezier_knot(1), vec![pt0, pt1]);
    assert!(!surface.include(&line));
}

fn point_within(actual: Point3, expected: Point3, tolerance: f64) {
    let residual = actual.distance(expected);
    assert!(
        residual.is_finite() && residual <= tolerance,
        "actual={actual:?} expected={expected:?} residual={residual}"
    );
}

#[test]
fn axis_inverse_handles_interior_and_extended_apices_without_changing_ranges() {
    for (line, expected_t, point) in [
        (
            Line(Point3::new(0.4, 0.0, 0.0), Point3::new(1.4, 0.0, 1.0)),
            -0.4,
            Point3::new(0.0, 0.0, -0.4),
        ),
        (
            Line(Point3::new(-0.4, 0.0, -0.4), Point3::new(0.6, 0.0, 0.6)),
            0.4,
            Point3::origin(),
        ),
    ] {
        let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_z());
        assert_eq!(surface.try_range_tuple().0, Some((0.0, 1.0)));
        for hint in [
            SearchParameterHint2D::None,
            SearchParameterHint2D::Parameter(expected_t + 0.1, 7.3),
            SearchParameterHint2D::Range((expected_t - 0.2, expected_t + 0.2), (7.0, 8.0)),
        ] {
            for nearest in [false, true] {
                let found = if nearest {
                    surface.search_nearest_parameter(point, hint, 100)
                } else {
                    surface.search_parameter(point, hint, 100)
                }
                .unwrap();
                assert!((found.0 - expected_t).abs() <= 1e-6);
                let angle = match hint {
                    SearchParameterHint2D::Parameter(_, v) => v,
                    SearchParameterHint2D::Range(_, (v, _)) => v,
                    _ => 0.0,
                };
                assert_eq!(found.1, angle);
                point_within(surface.evaluate(found.0, found.1), point, 1e-6);
            }
        }
    }
}

#[test]
fn axis_nearest_keeps_off_surface_contract_and_rejects_exact_off_surface_query() {
    let line = Line(Point3::new(0.4, 0.0, 0.0), Point3::new(1.4, 0.0, 1.0));
    let surface = RevolutionSurface::by_revolution(line, Point3::origin(), Vector3::unit_z());
    let query = Point3::origin();
    assert!(surface.search_parameter(query, None, 100).is_none());
    let found = surface
        .search_nearest_parameter(query, (-0.1, PI / 2.0), 100)
        .unwrap();
    point_within(
        surface.evaluate(found.0, found.1),
        Point3::new(0.0, 0.2, -0.2),
        1e-6,
    );
    assert!(surface.evaluate(found.0, found.1).distance(query) > 0.28);
    for query in [Point3::new(0.1, 0.0, -0.4), Point3::new(0.0, 0.0, f64::NAN)] {
        assert!(surface.search_parameter(query, None, 100).is_none());
    }
    for nearest in [false, true] {
        let found = if nearest {
            surface.search_nearest_parameter(Point3::new(0.0, 0.0, -0.4), (f64::NAN, 0.0), 100)
        } else {
            surface.search_parameter(Point3::new(0.0, 0.0, -0.4), (f64::NAN, 0.0), 100)
        };
        assert!(found.is_none());
    }
}

#[test]
fn near_axis_points_keep_their_radial_direction_instead_of_becoming_poles() {
    let surface = RevolutionSurface::by_revolution(
        Line(Point3::new(0.4, 0.0, 0.0), Point3::new(1.4, 0.0, 1.0)),
        Point3::origin(),
        Vector3::unit_z(),
    );
    // Opposite the generator at a radius below the spatial tolerance: a
    // tolerance-based pole shortcut would introduce a1.6e-6 spatial error.
    let radius = 0.8e-6;
    let query = Point3::new(-radius, 0.0, -0.4 + radius);
    let found = surface.search_parameter(query, None, 100).unwrap();
    point_within(surface.evaluate(found.0, found.1), query, 1e-10);
    assert!((found.1 - PI).abs() < 1e-8);
}

#[test]
fn translated_axis_and_length_unit_controls_preserve_actual_cone_geometry() {
    let origin = Point3::new(3.0, -2.0, 5.0);
    for (axis, radial) in [
        (Vector3::unit_x(), Vector3::unit_y()),
        (Vector3::unit_y(), Vector3::unit_z()),
        (Vector3::unit_z(), Vector3::unit_x()),
    ] {
        for radius in [0.0004, 0.4, 400.0] {
            let start = origin + radius * radial;
            let surface =
                RevolutionSurface::by_revolution(Line(start, start + radial + axis), origin, axis);
            let query = origin - radius * axis;
            let found = surface.search_parameter(query, None, 100).unwrap();
            point_within(surface.evaluate(found.0, found.1), query, 1e-6);
        }
    }
}

#[test]
fn axis_search_preserves_hints_when_a_generator_has_multiple_roots() {
    // x=(t-1/4)(t-3/4), z=0 has two distinct generator parameters at the axis.
    let curve = BsplineCurve::new(
        KnotVector::bezier_knot(2),
        vec![
            Point3::new(0.1875, 0.0, 0.0),
            Point3::new(-0.3125, 0.0, 0.0),
            Point3::new(0.1875, 0.0, 0.0),
        ],
    );
    let surface = RevolutionSurface::by_revolution(curve, Point3::origin(), Vector3::unit_z());
    for (seed, expected) in [(0.2, 0.25), (0.8, 0.75)] {
        for hint in [
            SearchParameterHint2D::Parameter(seed, 7.3),
            SearchParameterHint2D::Range((seed - 0.1, seed + 0.1), (7.3, 8.0)),
        ] {
            let found = surface
                .search_parameter(Point3::origin(), hint, 100)
                .unwrap();
            assert!(
                (found.0 - expected).abs() < 1e-6,
                "hint={hint:?} found={found:?}"
            );
            assert_eq!(found.1, 7.3);
            point_within(surface.evaluate(found.0, found.1), Point3::origin(), 1e-6);
        }
    }
}

#[test]
fn cylinder_axis_controls_keep_exact_refusal_and_nearest_distance() {
    let surface = RevolutionSurface::by_revolution(
        Line(Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 0.0, 1.0)),
        Point3::origin(),
        Vector3::unit_z(),
    );
    let query = Point3::new(0.0, 0.0, -3.0);
    assert!(surface.search_parameter(query, None, 100).is_none());
    let found = surface.search_nearest_parameter(query, None, 100).unwrap();
    point_within(
        surface.evaluate(found.0, found.1),
        Point3::new(1.0, 0.0, -3.0),
        1e-6,
    );
    // The Processor's singular-refinement fallback cannot certify this farther
    // seed within1e-6, so it must retain the existing refusal.
    let processed = Processor::<_, Matrix4>::new(surface);
    assert!(
        processed
            .search_nearest_parameter(query, None, 100)
            .is_none()
    );
    let ordinary = Point3::new(1.0, 0.0, -3.0);
    let found = processed.search_parameter(ordinary, None, 100).unwrap();
    point_within(processed.evaluate(found.0, found.1), ordinary, 1e-6);
}

#[test]
fn processor_singular_refinement_seed_obeys_the_existing_world_spatial_cutoff() {
    for radius in [0.999e-6, 1.001e-6] {
        let cylinder = RevolutionSurface::by_revolution(
            Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let cylinder = Processor::<_, Matrix4>::new(cylinder);
        let query = Point3::new(0.0, 0.0, -0.3);
        // The generator seed is already stationary. Zero refinement trials
        // force the wrapper's nonzero normal-offset Newton step to exhaust its
        // budget, so this exercises the fallback rather than a successful solve.
        let found = cylinder.search_nearest_parameter(query, (-0.3, 0.0), 0);
        if radius < 1e-6 {
            let (u, v) =
                found.expect("failed-refinement seed just inside cutoff remains admissible");
            assert!(u.is_finite() && v.is_finite());
            point_within(
                cylinder.evaluate(u, v),
                Point3::new(radius, 0.0, -0.3),
                1e-12,
            );
            assert!(cylinder.evaluate(u, v).distance(query) < 1e-6);
        } else {
            assert!(
                found.is_none(),
                "farther singular seed must remain refused: {found:?}"
            );
        }
    }
}

#[test]
fn axis_exact_reconstruction_obeys_a_euclidean_spatial_cutoff() {
    for (x, y, accepted) in [(0.9e-6, 0.0, true), (0.9e-6, 0.9e-6, false)] {
        let surface = RevolutionSurface::by_revolution(
            Line(Point3::new(x, y, 0.0), Point3::new(x, y, 1.0)),
            Point3::origin(),
            Vector3::unit_z(),
        );
        let query = Point3::new(0.0, 0.0, -0.3);
        let found = surface.search_parameter(query, (-0.3, 0.0), 100);
        assert_eq!(found.is_some(), accepted, "x={x} y={y} found={found:?}");
        if let Some((u, v)) = found {
            assert!(surface.evaluate(u, v).distance(query) <= 1e-6);
        }
    }
}

#[test]
fn processor_singular_diagonal_seed_obeys_a_euclidean_spatial_cutoff() {
    let radius = 1.3e-6;
    let surface = Processor::<_, Matrix4>::new(RevolutionSurface::by_revolution(
        Line(Point3::new(radius, 0.0, 0.0), Point3::new(radius, 0.0, 1.0)),
        Point3::origin(),
        Vector3::unit_z(),
    ));
    let query = Point3::new(0.0, 0.0, -0.3);
    // A stationary generator seed and zero refinement budget isolate fallback.
    // At longitudePI/4 each XY component is below1e-6, but distance is1.3e-6.
    let found = surface.search_nearest_parameter(query, (-0.3, PI / 4.0), 0);
    assert!(
        found.is_none(),
        "diagonal farther seed must remain refused: {found:?}"
    );
}
