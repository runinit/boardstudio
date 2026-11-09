use super::*;
use std::f64::consts::PI;
use std::ops::Bound;
impl ParametricSurface for Sphere {
    type Point = Point3;
    type Vector = Vector3;
    #[inline]
    fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        self.0.derivative_mn(n, m, PI / 2.0 - v, u) * (-1f64).powi(n as i32)
    }
    #[inline]
    fn evaluate(&self, u: f64, v: f64) -> Point3 { self.0.evaluate(PI / 2.0 - v, u) }
    #[inline]
    fn derivative_u(&self, u: f64, v: f64) -> Vector3 { self.0.derivative_v(PI / 2.0 - v, u) }
    #[inline]
    fn derivative_v(&self, u: f64, v: f64) -> Vector3 { -self.0.derivative_u(PI / 2.0 - v, u) }
    #[inline]
    fn derivative_uu(&self, u: f64, v: f64) -> Vector3 { self.0.derivative_vv(PI / 2.0 - v, u) }
    #[inline]
    fn derivative_uv(&self, u: f64, v: f64) -> Vector3 { -self.0.derivative_uv(PI / 2.0 - v, u) }
    #[inline]
    fn derivative_vv(&self, u: f64, v: f64) -> Vector3 { self.0.derivative_uu(PI / 2.0 - v, u) }
    #[inline]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        (
            (Bound::Included(0.0), Bound::Excluded(2.0 * PI)),
            (Bound::Included(-PI / 2.0), Bound::Excluded(PI / 2.0)),
        )
    }
    #[inline]
    fn analytic_singularities(&self) -> Vec<AnalyticParameterSingularity> {
        vec![
            AnalyticParameterSingularity { axis: ParameterAxis::V, parameter: -PI / 2.0 },
            AnalyticParameterSingularity { axis: ParameterAxis::V, parameter: PI / 2.0 },
        ]
    }
    #[inline]
    fn period_u(&self) -> Option<f64> { Some(2.0 * PI) }
}
impl ParametricSurface3D for Sphere {
    #[inline]
    fn normal(&self, u: f64, v: f64) -> Vector3 { self.0.normal(PI / 2.0 - v, u) }
}

#[inline]
fn step_sphere_search_hint<H: Into<SearchParameterHint2D>>(hint: H) -> SearchParameterHint2D {
    match hint.into() {
        SearchParameterHint2D::Parameter(u, v) => {
            SearchParameterHint2D::Parameter(PI / 2.0 - v, u)
        }
        SearchParameterHint2D::Range((u0, u1), (v0, v1)) => {
            SearchParameterHint2D::Range((PI / 2.0 - v1, PI / 2.0 - v0), (u0, u1))
        }
        SearchParameterHint2D::None => SearchParameterHint2D::None,
    }
}

impl SearchNearestParameter<SurfaceParameter> for Sphere {
    type Point = Point3;
    #[inline]
    fn search_nearest_parameter<H: Into<SearchParameterHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.0
            .search_nearest_parameter(point, step_sphere_search_hint(hint), trials)
            .map(|(u, v)| (v, PI / 2.0 - u))
    }
}
impl SearchParameter<SurfaceParameter> for Sphere {
    type Point = Point3;
    #[inline]
    fn search_parameter<H: Into<SearchParameterHint2D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.0
            .search_parameter(point, step_sphere_search_hint(hint), trials)
            .map(|(u, v)| (v, PI / 2.0 - u))
    }
}
fn sphere_division_step(tolerance: f64, radius: f64) -> f64 {
    // Each parameter axis receives half the requested radial sag budget. Let
    // q=tolerance/radius; the per-axis chord angle is
    // 2*acos(1-q/2) = 4*asin(sqrt(q/4)). This form avoids cancellation in
    // `1-q/2` for small positive q and avoids underflow from halving tolerance
    // before dividing by the radius.
    let q = f64::min(tolerance / radius, 2.0);
    4.0 * f64::asin(0.5 * f64::sqrt(q))
}

fn sphere_divisions(range: (f64, f64), delta: f64) -> Vec<f64> {
    let divisions = 1 + ((range.1 - range.0) / delta).floor() as usize;
    (0..=divisions)
        .map(|index| range.0 + (range.1 - range.0) * index as f64 / divisions as f64)
        .collect()
}

impl ParameterDivision2D for Sphere {
    #[inline]
    fn parameter_division(
        &self,
        ((u0, u1), (v0, v1)): ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        monstertruck_traits::nonpositive_tolerance!(tol);
        let range = ((PI / 2.0 - v1, PI / 2.0 - v0), (u0, u1));
        let radius = self.0.radius();
        let (udiv0, vdiv0) = if radius <= 0.0 {
            // Preserve the wrapped implementation's validation and edge-case
            // behavior, then retain the STEP-to-geometry axis remapping below.
            self.0.parameter_division(range, tol)
        } else {
            let delta = sphere_division_step(tol, radius);
            (
                sphere_divisions(range.0, delta),
                sphere_divisions(range.1, delta),
            )
        };
        let vdiv = udiv0.into_iter().map(|u| PI / 2.0 - u).collect();
        (vdiv0, vdiv)
    }
}

#[cfg(test)]
proptest::proptest! {
    #[test]
    fn surface(
        center in proptest::array::uniform3(-100.0f64..100.0f64),
        radius in 0.1f64..100.0f64,
        (u, v) in (0.0..=2.0 * PI, -PI / 2.0..=PI / 2.0),
    ) {
        const EPS: f64 = 1.0e-3;
        let sphere = Sphere(monstertruck_geometry::prelude::Sphere::new(center.into(), radius));

        let uder0 = sphere.uder(u, v);
        let uder1 = (sphere.subs(u + EPS, v) - sphere.subs(u - EPS, v)) / (2.0 * EPS);
        assert!(
            (uder0 - uder1).magnitude2() < EPS,
            "uder failed: {uder0:?}, {uder1:?}"
        );

        let vder0 = sphere.vder(u, v);
        let vder1 = (sphere.subs(u, v + EPS) - sphere.subs(u, v - EPS)) / (2.0 * EPS);
        assert!(
            (vder0 - vder1).magnitude2() < EPS,
            "vder failed: {vder0:?}, {vder1:?}"
        );

        let uuder0 = sphere.uuder(u, v);
        let uuder1 = (sphere.uder(u + EPS, v) - sphere.uder(u - EPS, v)) / (2.0 * EPS);
        assert!(
            (uuder0 - uuder1).magnitude2() < EPS,
            "uuder failed: {uuder0:?}, {uuder1:?}"
        );

        let uvder0 = sphere.uvder(u, v);
        let uvder1 = (sphere.uder(u, v + EPS) - sphere.uder(u, v - EPS)) / (2.0 * EPS);
        assert!(
            (uvder0 - uvder1).magnitude2() < EPS,
            "uvder failed: {uvder0:?}, {uvder1:?}"
        );

        let vvder0 = sphere.vvder(u, v);
        let vvder1 = (sphere.vder(u, v + EPS) - sphere.vder(u, v - EPS)) / (2.0 * EPS);
        assert!(
            (vvder0 - vvder1).magnitude2() < EPS,
            "vvder failed: {vvder0:?}, {vvder1:?}"
        );

        let n0 = sphere.normal(u, v);
        let n1 = sphere.uder(u, v).cross(sphere.vder(u, v)).normalize();
        assert!(
            (n0 - n1).magnitude2() < EPS,
            "normal failed: {n0:?}, {n1:?}"
        );
    }
}

#[cfg(test)]
mod derivative_mn_tests {
    use super::*;
    use monstertruck_geometry::prelude::Sphere as GeometrySphere;

    fn test_sphere() -> Sphere {
        Sphere(GeometrySphere::new(Point3::new(2.0, -3.0, 0.5), 2.75))
    }

    fn assert_vector_near(actual: Vector3, expected: Vector3, tolerance: f64, label: &str) {
        let error = (actual - expected).magnitude();
        assert!(
            error <= tolerance,
            "{label}: error={error}, actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn derivative_mn_matches_named_first_partials() {
        let sphere = test_sphere();
        let (u, v) = (0.73, -0.21);
        for (m, n, named, label) in [
            (1, 0, sphere.derivative_u(u, v), "u"),
            (0, 1, sphere.derivative_v(u, v), "v"),
        ] {
            assert_vector_near(sphere.derivative_mn(m, n, u, v), named, 1.0e-12, label);
        }
    }

    #[test]
    fn derivative_mn_matches_named_second_partials() {
        let sphere = test_sphere();
        let (u, v) = (0.73, -0.21);
        for (m, n, named, label) in [
            (2, 0, sphere.derivative_uu(u, v), "uu"),
            (1, 1, sphere.derivative_uv(u, v), "uv"),
            (0, 2, sphere.derivative_vv(u, v), "vv"),
        ] {
            assert_vector_near(sphere.derivative_mn(m, n, u, v), named, 1.0e-12, label);
        }
    }

    #[test]
    fn derivative_mn_matches_first_and_second_finite_differences() {
        let sphere = test_sphere();
        let (u, v) = (0.73, -0.21);
        let h = 1.0e-4;
        let point_du = (sphere.evaluate(u + h, v) - sphere.evaluate(u - h, v)) / (2.0 * h);
        let point_dv = (sphere.evaluate(u, v + h) - sphere.evaluate(u, v - h)) / (2.0 * h);
        assert_vector_near(
            sphere.derivative_mn(1, 0, u, v),
            point_du,
            2.0e-7,
            "finite difference u",
        );
        assert_vector_near(
            sphere.derivative_mn(0, 1, u, v),
            point_dv,
            2.0e-7,
            "finite difference v",
        );

        let fd_uu = (sphere.derivative_u(u + h, v) - sphere.derivative_u(u - h, v)) / (2.0 * h);
        let fd_uv = (sphere.derivative_u(u, v + h) - sphere.derivative_u(u, v - h)) / (2.0 * h);
        let fd_vv = (sphere.derivative_v(u, v + h) - sphere.derivative_v(u, v - h)) / (2.0 * h);
        assert_vector_near(
            sphere.derivative_mn(2, 0, u, v),
            fd_uu,
            2.0e-7,
            "finite difference uu",
        );
        assert_vector_near(
            sphere.derivative_mn(1, 1, u, v),
            fd_uv,
            2.0e-7,
            "finite difference uv",
        );
        assert_vector_near(
            sphere.derivative_mn(0, 2, u, v),
            fd_vv,
            2.0e-7,
            "finite difference vv",
        );
    }

    #[test]
    fn derivative_mn_matches_higher_mixed_chain_rule_and_finite_differences() {
        let sphere = test_sphere();
        let (u, v) = (0.73, -0.21);
        let h = 1.0e-4;
        for m in 0..=4 {
            for n in 0..=(4 - m) {
                let expected = test_sphere().0.derivative_mn(n, m, PI / 2.0 - v, u)
                    * if n % 2 == 0 { 1.0 } else { -1.0 };
                assert_vector_near(
                    sphere.derivative_mn(m, n, u, v),
                    expected,
                    1.0e-12,
                    &format!("chain rule ({m},{n})"),
                );
            }
        }

        let fd_21 = (sphere.derivative_mn(2, 0, u, v + h) - sphere.derivative_mn(2, 0, u, v - h))
            / (2.0 * h);
        let fd_12 = (sphere.derivative_mn(1, 1, u, v + h) - sphere.derivative_mn(1, 1, u, v - h))
            / (2.0 * h);
        assert_vector_near(
            sphere.derivative_mn(2, 1, u, v),
            fd_21,
            2.0e-7,
            "finite difference 21",
        );
        assert_vector_near(
            sphere.derivative_mn(1, 2, u, v),
            fd_12,
            2.0e-7,
            "finite difference 12",
        );
    }
}

#[cfg(test)]
mod inverse_hint_tests {
    use super::*;
    use monstertruck_geometry::prelude::Sphere as GeometrySphere;

    #[test]
    fn range_hint_reorders_latitude_and_none_stays_absent() {
        // The current sphere solver ignores Range; this checks chart mapping only.
        let hint = SearchParameterHint2D::Range((0.4, 7.0), (-0.3, 0.6));
        let SearchParameterHint2D::Range(theta, phi) = step_sphere_search_hint(hint) else {
            panic!("range hint changed variant");
        };
        assert_eq!(theta, (PI / 2.0 - 0.6, PI / 2.0 + 0.3));
        assert_eq!(phi, (0.4, 7.0));
        assert!(matches!(step_sphere_search_hint(SearchParameterHint2D::None), SearchParameterHint2D::None));
    }

    fn test_sphere() -> Sphere {
        Sphere(GeometrySphere::new(Point3::new(1.5, -2.0, 0.75), 3.0))
    }

    fn assert_parameter_near(actual: (f64, f64), expected: (f64, f64), label: &str) {
        assert!(
            (actual.0 - expected.0).abs() < 1.0e-10 && (actual.1 - expected.1).abs() < 1.0e-10,
            "{label}: actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn exact_parameter_search_maps_pole_longitude_hint() {
        let sphere = test_sphere();
        let hinted_u = 2.0 * PI + 0.4;
        let point = sphere.evaluate(hinted_u, PI / 2.0);
        let found = SearchParameter::<SurfaceParameter>::search_parameter(
            &sphere,
            point,
            (hinted_u, 0.7),
            1,
        )
        .unwrap();
        assert_parameter_near(found, (hinted_u, PI / 2.0), "exact search at pole");
    }

    #[test]
    fn nearest_parameter_search_uses_canonical_pole_branch() {
        let sphere = test_sphere();
        let point = sphere.evaluate(1.4, PI / 2.0);
        let found = SearchNearestParameter::<SurfaceParameter>::search_nearest_parameter(
            &sphere,
            point,
            (1.4, 0.7),
            1,
        )
        .unwrap();
        // The underlying nearest search explicitly uses geometry longitude 0 at poles;
        // unlike exact parameter search, it does not consume this hint there.
        assert_parameter_near(found, (0.0, PI / 2.0), "nearest search at pole");
    }

    #[test]
    fn exact_parameter_search_maps_periodic_longitude_hint() {
        let sphere = test_sphere();
        let principal_u = 0.4;
        let hinted_u = 2.0 * PI + principal_u;
        let point = sphere.evaluate(principal_u, 0.2);
        let found = SearchParameter::<SurfaceParameter>::search_parameter(
            &sphere,
            point,
            (hinted_u, 0.2),
            1,
        )
        .unwrap();
        assert_parameter_near(found, (hinted_u, 0.2), "exact periodic branch");
    }

    #[test]
    fn nearest_parameter_search_maps_periodic_longitude_hint() {
        let sphere = test_sphere();
        let principal_u = 0.4;
        let hinted_u = 2.0 * PI + principal_u;
        let point = sphere.evaluate(principal_u, 0.2);
        let found = SearchNearestParameter::<SurfaceParameter>::search_nearest_parameter(
            &sphere,
            point,
            (hinted_u, 0.2),
            1,
        )
        .unwrap();
        assert_parameter_near(found, (hinted_u, 0.2), "nearest periodic branch");
    }
}


#[cfg(test)]
mod singularity_capability_tests {
    use super::*;

    #[test]
    fn processor_orientation_maps_declared_parameter_axis() {
        let sphere: SphericalSurface = Processor::new(Sphere(
            monstertruck_geometry::prelude::Sphere::new(Point3::origin(), 5.0),
        ));
        assert_eq!(
            sphere.analytic_singularities(),
            vec![
                AnalyticParameterSingularity {
                    axis: ParameterAxis::V,
                    parameter: -PI / 2.0,
                },
                AnalyticParameterSingularity {
                    axis: ParameterAxis::V,
                    parameter: PI / 2.0,
                },
            ]
        );
        let swapped = sphere.inverse();
        assert_eq!(
            swapped.analytic_singularities(),
            vec![
                AnalyticParameterSingularity {
                    axis: ParameterAxis::U,
                    parameter: -PI / 2.0,
                },
                AnalyticParameterSingularity {
                    axis: ParameterAxis::U,
                    parameter: PI / 2.0,
                },
            ]
        );
    }
}

#[cfg(test)]
mod linear_deflection_tests {
    use super::*;
    use monstertruck_geometry::prelude::Sphere as GeometrySphere;

    fn max_step(divisions: &[f64]) -> f64 {
        divisions.windows(2).map(|pair| (pair[1] - pair[0]).abs()).fold(0.0, f64::max)
    }

    fn check_range(radius: f64, tol: f64, range: ((f64, f64), (f64, f64))) {
        let sphere = Sphere(GeometrySphere::new(Point3::new(13.0, -7.0, 2.0), radius));
        let (u_div, v_div) = sphere.parameter_division(range, tol);
        assert!(u_div.len() >= 2 && v_div.len() >= 2);
        let endpoint_eps = 8.0 * f64::EPSILON;
        let endpoint_matches = |actual: f64, target: f64| {
            (actual - target).abs() <= endpoint_eps * target.abs().max(1.0)
        };
        assert!(endpoint_matches(u_div[0], range.0.0));
        assert!(endpoint_matches(*u_div.last().unwrap(), range.0.1));
        // STEP sphere latitude is reversed relative to the wrapped geometry sphere's polar angle.
        assert!(endpoint_matches(v_div[0], range.1.1));
        assert!(endpoint_matches(*v_div.last().unwrap(), range.1.0));
        let (du, dv) = (max_step(&u_div), max_step(&v_div));
        let axis_sag_u = radius * (1.0 - f64::cos(du / 2.0));
        let axis_sag_v = radius * (1.0 - f64::cos(dv / 2.0));
        let combined_sag = radius * (1.0 - f64::cos(du / 2.0) * f64::cos(dv / 2.0));
        let eps = 32.0 * f64::EPSILON * radius.max(tol);
        assert!(axis_sag_u <= tol / 2.0 + eps, "u sag {axis_sag_u} > {}", tol / 2.0);
        assert!(axis_sag_v <= tol / 2.0 + eps, "v sag {axis_sag_v} > {}", tol / 2.0);
        assert!(combined_sag <= tol + eps, "combined sag {combined_sag} > {tol}");
    }

    #[test]
    fn division_respects_two_axis_budget_for_full_partial_and_polar_ranges() {
        for radius in [0.5, 5.0, 50.0] {
            for tol in [0.1, 0.01] {
                check_range(radius, tol, ((0.0, 2.0 * PI), (-PI / 2.0, PI / 2.0)));
                check_range(radius, tol, ((0.3, 2.1), (-0.8, 0.4)));
                check_range(radius, tol, ((1.1, 3.5), (0.9, PI / 2.0)));
                check_range(radius, tol, ((-0.2, 1.7), (-PI / 2.0, -0.6)));
            }
        }
    }

    #[test]
    fn tiny_sag_ratio_keeps_a_finite_positive_step_without_halving_tolerance() {
        let delta = sphere_division_step(1.0e-15, 5.0);
        assert!(delta.is_finite() && delta > 0.0);
        let recovered_sag = 2.0 * 5.0 * f64::sin(delta / 4.0).powi(2);
        assert!((recovered_sag / 5.0e-16 - 1.0).abs() < 1.0e-12);

        let smallest_positive = f64::from_bits(1);
        let subnormal_delta = sphere_division_step(smallest_positive, 1.0);
        assert!(subnormal_delta.is_finite() && subnormal_delta > 0.0);
    }

    #[test]
    fn zero_radius_fallback_preserves_step_axis_mapping() {
        let sphere = Sphere(GeometrySphere::new(Point3::new(1.0, 2.0, 3.0), 0.0));
        let range = ((0.2, 1.7), (-0.4, 0.9));
        let geometry_range = ((PI / 2.0 - range.1.1, PI / 2.0 - range.1.0), range.0);
        let (geometry_u, geometry_v) = sphere.0.parameter_division(geometry_range, 0.5);
        let expected = (
            geometry_v,
            geometry_u.into_iter().map(|u| PI / 2.0 - u).collect::<Vec<_>>(),
        );
        assert_eq!(sphere.parameter_division(range, 0.5), expected);
    }

    #[test]
    fn invalid_tolerance_keeps_wrapped_assertion_policy() {
        let sphere = Sphere(GeometrySphere::new(Point3::origin(), 5.0));
        let minimum = monstertruck_geometry::prelude::TOLERANCE;
        let range = ((0.0, 1.0e-6), (-0.5, -0.5 + 1.0e-6));
        assert!(std::panic::catch_unwind(|| sphere.parameter_division(range, minimum)).is_ok());
        for tolerance in [0.0, minimum * 0.5, f64::NAN] {
            assert!(std::panic::catch_unwind(|| sphere.parameter_division(range, tolerance)).is_err());
        }
    }

    #[test]
    fn nan_tolerance_keeps_wrapped_assertion_policy() {
        let sphere = Sphere(GeometrySphere::new(Point3::origin(), 5.0));
        let range = ((0.0, 1.0e-6), (-0.5, -0.5 + 1.0e-6));
        assert!(std::panic::catch_unwind(|| sphere.parameter_division(range, f64::NAN)).is_err());
    }
}
