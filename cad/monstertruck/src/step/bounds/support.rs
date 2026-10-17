//! Whole active-curve support incidence for native quadrics.
use super::*;
use monstertruck_modeling::{SquareMatrix, Vector3};

struct Quadric {
    origin: Point3,
    axis: Vector3,
    slope: f64,
    intercept: f64,
}

impl Quadric {
    // Homogeneous bilinear form: radial² - (intercept + slope*height)².
    fn bilinear(&self, p: Vector4, q: Vector4) -> f64 {
        let project = |p: Vector4| {
            let offset = p.truncate() - self.origin.to_vec() * p.w;
            let h = offset.dot(self.axis);
            (
                offset - self.axis * h,
                self.intercept * p.w + self.slope * h,
            )
        };
        let (p, r) = project(p);
        let (q, s) = project(q);
        p.dot(q) - r * s
    }
}

pub(super) fn certify(
    curve: &Curve3D,
    surface: &Surface,
    factor: f64,
    placement: Matrix4,
) -> Result<(), String> {
    certify_with_allowance(curve, surface, factor, placement, TOLERANCE)
}

struct Prepared {
    inverse: Matrix4,
    quadric: Quadric,
    factor: f64,
    /// Maps source points to the expected radius in the `z` coordinate.
    radius_map: Matrix4,
}

/// None for supports that are not native cylinders or cones.
fn prepare(
    surface: &Surface,
    factor: f64,
    placement: Matrix4,
) -> Result<Option<Prepared>, String> {
    let Surface::ElementarySurface(surface) = surface else {
        return Ok(None);
    };
    let (processor, conical) = match surface {
        ElementarySurface::CylindricalSurface(s) => (s, false),
        ElementarySurface::ConicalSurface(s) => (s, true),
        _ => return Ok(None),
    };
    let matrix = *processor.transform();
    let scale = similarity_scale(matrix)?;
    let factor = factor * scale * similarity_scale(placement)?;
    if !factor.is_finite() || factor <= 0. {
        return Err("invalid source support incidence scale".into());
    }
    let inverse = matrix.invert().ok_or("singular source quadric support")?;
    let revolution = processor.entity();
    let origin = revolution.origin();
    let axis = revolution.axis();
    if (axis.magnitude2() - 1.).abs() > 1e-12
        || ![axis.x, axis.y, axis.z, origin.x, origin.y, origin.z]
            .iter()
            .all(|x| x.is_finite())
    {
        return Err("invalid source quadric axis".into());
    }
    let line = revolution.entity_curve();
    let offset = line.0 - origin;
    let height = offset.dot(axis);
    let radius = (offset - axis * height).magnitude();
    let slope = if conical {
        let delta = line.1 - line.0;
        let axial = delta.dot(axis);
        if axial == 0. || radius == 0. {
            return Err("unsupported degenerate cone support".into());
        }
        delta.dot((offset - axis * height) / radius) / axial
    } else {
        0.
    };
    let quadric = Quadric {
        origin,
        axis,
        slope,
        intercept: radius - slope * height,
    };
    if ![radius, slope, quadric.intercept]
        .iter()
        .all(|x| x.is_finite())
    {
        return Err("nonfinite source quadric support".into());
    }
    // A lower bound on the absolute expected radius strengthens the radial-distance
    // residual criterion away from an apex. The square-root criterion remains valid there.
    let a = axis * slope;
    let radius_map = Matrix4::from_cols(
        Vector4::new(0., 0., a.x, 0.),
        Vector4::new(0., 0., a.y, 0.),
        Vector4::new(0., 0., a.z, 0.),
        Vector4::new(0., 0., quadric.intercept - a.dot(origin.to_vec()), 1.),
    ) * inverse;
    Ok(Some(Prepared {
        inverse,
        quadric,
        factor,
        radius_map,
    }))
}

pub(super) fn certify_with_allowance(
    curve: &Curve3D,
    surface: &Surface,
    factor: f64,
    placement: Matrix4,
    allowance_mm: f64,
) -> Result<(), String> {
    let Some(Prepared {
        inverse,
        quadric,
        factor,
        radius_map,
    }) = prepare(surface, factor, placement)?
    else {
        return Ok(());
    };
    let mut range = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    super::curve_bounds(curve, radius_map, &mut range)?;
    let minimum = if range[0][2] <= 0. && range[1][2] >= 0. {
        0.
    } else {
        range[0][2].abs().min(range[1][2].abs())
    };
    let tolerance = allowance_mm / factor;
    let threshold = if minimum > tolerance {
        tolerance * (2. * minimum - tolerance)
    } else {
        tolerance * tolerance
    };
    if !threshold.is_finite() || threshold <= 0. {
        return Err("unsupported source support incidence precision".into());
    }
    certify_curve(curve, inverse, &quadric, threshold)
        .map_err(|_| "source boundary has uncertified quadric support incidence".into())
}

/// Preview fallback: maximum sampled radial distance (mm) of the curve from a native
/// cylinder or cone support. None for other supports. Not a certificate.
pub(super) fn sampled_residual_mm(
    curve: &Curve3D,
    surface: &Surface,
    factor: f64,
    placement: Matrix4,
) -> Result<Option<f64>, String> {
    let Some(prepared) = prepare(surface, factor, placement)? else {
        return Ok(None);
    };
    let q = &prepared.quadric;
    super::sampled_max(curve, |point| {
        let p = prepared.inverse.transform_point(point);
        let offset = p - q.origin;
        let height = offset.dot(q.axis);
        let radial = (offset - q.axis * height).magnitude();
        (radial - (q.intercept + q.slope * height).abs()).abs() * prepared.factor
    })
    .map(Some)
}

fn similarity_scale(matrix: Matrix4) -> Result<f64, String> {
    let axes = [
        matrix.x.truncate(),
        matrix.y.truncate(),
        matrix.z.truncate(),
    ];
    let scale = axes[0].magnitude();
    if !scale.is_finite()
        || scale <= 0.
        || matrix.x.w != 0.
        || matrix.y.w != 0.
        || matrix.z.w != 0.
        || matrix.w.w != 1.
        || axes
            .iter()
            .any(|a| (a.magnitude() / scale - 1.).abs() > 1e-12)
        || [(0, 1), (0, 2), (1, 2)]
            .iter()
            .any(|&(a, b)| (axes[a] / scale).dot(axes[b] / scale).abs() > 1e-12)
    {
        return Err("unsupported nonsimilarity source support placement".into());
    }
    Ok(scale)
}

fn certify_curve(curve: &Curve3D, map: Matrix4, q: &Quadric, limit: f64) -> Result<(), ()> {
    let visit = |p| certify_bezier(p, q, limit, 0);
    match curve {
        Curve3D::Line(line) => visit(vec![
            map * line.0.to_homogeneous(),
            map * line.1.to_homogeneous(),
        ]),
        Curve3D::Polyline(line) => {
            for pair in line.windows(2) {
                visit(pair.iter().map(|p| map * p.to_homogeneous()).collect())?;
            }
            Ok(())
        }
        Curve3D::BsplineCurve(c) => super::spline_beziers(
            c.knot_vector().to_vec(),
            c.control_points()
                .iter()
                .map(|p| map * p.to_homogeneous())
                .collect(),
            c.degree(),
            &mut |p| visit(p).map_err(|_| "quadric residual".into()),
        )
        .map_err(|_| ()),
        Curve3D::NurbsCurve(c) => super::spline_beziers(
            c.knot_vector().to_vec(),
            c.control_points().iter().map(|p| map * p).collect(),
            c.degree(),
            &mut |p| visit(p).map_err(|_| "quadric residual".into()),
        )
        .map_err(|_| ()),
        Curve3D::SurfaceCurve(c) => {
            if c.active_trim().is_none()
                && c.master_representation() == SurfaceCurveRepresentation::Curve3D
            {
                return certify_curve(c.leader(), map, q, limit);
            }
            visit(
                super::selected_affine_endpoints(c)
                    .map_err(|_| ())?
                    .into_iter()
                    .map(|p| map * p.to_homogeneous())
                    .collect(),
            )
        }
        Curve3D::Conic(c) => {
            let (matrix, range) = match c {
                Conic3D::Ellipse(c) => (map * c.transform(), c.entity().range()),
                Conic3D::Hyperbola(c) => (map * c.transform(), c.entity().range()),
                Conic3D::Parabola(c) => (map * c.transform(), c.entity().range()),
            };
            if matrix.x.w != 0. || matrix.y.w != 0. || matrix.z.w != 0. || matrix.w.w != 1. {
                return Err(());
            }
            let (a, b, c0) = (matrix.x, matrix.y, matrix.w);
            let aa = q.bilinear(a, a);
            let bb = q.bilinear(b, b);
            let cc = q.bilinear(c0, c0);
            let ca = 2. * q.bilinear(c0, a);
            let cb = 2. * q.bilinear(c0, b);
            let ab = q.bilinear(a, b);
            let residual = match c {
                Conic3D::Ellipse(_) => {
                    (cc + (aa + bb) / 2.).abs() + ca.hypot(cb) + ((aa - bb) / 2.).hypot(ab)
                }
                Conic3D::Hyperbola(_) => {
                    let t = range.0.abs().max(range.1.abs());
                    (cc + (aa - bb) / 2.).abs()
                        + (ca.abs() + cb.abs()) * t.cosh()
                        + ((aa + bb).abs() / 2. + ab.abs()) * (2. * t).cosh()
                }
                Conic3D::Parabola(_) => {
                    let (lo, hi) = (range.0.min(range.1), range.0.max(range.1));
                    let p = c0 + b * (2. * lo) + a * (lo * lo);
                    return visit(vec![
                        p,
                        p + (b + a * lo) * (hi - lo),
                        c0 + b * (2. * hi) + a * (hi * hi),
                    ]);
                }
            };
            if residual.is_finite() && residual <= limit {
                Ok(())
            } else {
                Err(())
            }
        }
        _ => Err(()),
    }
}

fn choose(n: usize, k: usize) -> f64 {
    (0..k.min(n - k)).fold(1., |v, i| v * (n - i) as f64 / (i + 1) as f64)
}

fn certify_bezier(points: Vec<Vector4>, q: &Quadric, limit: f64, depth: usize) -> Result<(), ()> {
    if points.is_empty() || points.iter().any(|p| p.w <= 0. || !p.w.is_finite()) {
        return Err(());
    }
    let weight = points.iter().map(|p| p.w).fold(f64::INFINITY, f64::min);
    let degree = points.len() - 1;
    let mut residual = 0f64;
    for k in 0..=2 * degree {
        let mut coefficient = 0.;
        for i in k.saturating_sub(degree)..=k.min(degree) {
            coefficient += choose(degree, i) * choose(degree, k - i) / choose(2 * degree, k)
                * q.bilinear(points[i], points[k - i]);
        }
        if !coefficient.is_finite() {
            return Err(());
        }
        residual = residual.max(coefficient.abs());
    }
    if residual.is_finite() && residual <= limit * weight * weight {
        return Ok(());
    }
    if depth >= 32
        || [points[0], points[degree]].iter().any(|p| {
            let value = q.bilinear(*p, *p) / (p.w * p.w);
            !value.is_finite() || value.abs() > limit
        })
    {
        return Err(());
    }
    let mut work = points;
    let mut left = vec![work[0]];
    let mut right = vec![work[degree]];
    for count in (1..work.len()).rev() {
        for i in 0..count {
            work[i] = (work[i] + work[i + 1]) * 0.5;
        }
        left.push(work[0]);
        right.push(work[count - 1]);
    }
    right.reverse();
    certify_bezier(left, q, limit, depth + 1)?;
    certify_bezier(right, q, limit, depth + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_plane_line_quadric_certificate_uses_active_curve_not_leader() {
        use monstertruck_io::step::load::step_geometry::{
            Curve2D, StepParameterCurve, SurfaceCurve3D, SurfaceCurveAssociatedGeometry,
            SurfaceCurveKind,
        };
        use monstertruck_modeling::{Line, Point2};
        let support = Surface::ElementarySurface(ElementarySurface::Plane(Plane::new(
            Point3::new(1., 0., 0.),
            Point3::new(1., 0., 1.),
            Point3::new(2., 0., 0.),
        )));
        let make = |master| {
            SurfaceCurve3D::new(
                SurfaceCurveKind::SurfaceCurve,
                Box::new(Curve3D::Line(Line(
                    Point3::new(2., 0., 0.),
                    Point3::new(2., 0., 1.),
                ))),
                vec![SurfaceCurveAssociatedGeometry::ParameterCurve(
                    StepParameterCurve::new(
                        Box::new(Curve2D::Line(Line(
                            Point2::new(0., 0.),
                            Point2::new(1., 0.),
                        ))),
                        Box::new(support.clone()),
                    ),
                )],
                master,
            )
            .try_with_active_trim(Some((2., 10.)))
            .unwrap()
        };
        let q = Quadric {
            origin: Point3::origin(),
            axis: Vector3::unit_z(),
            slope: 0.,
            intercept: 1.,
        };
        certify_curve(
            &Curve3D::SurfaceCurve(make(SurfaceCurveRepresentation::ParameterCurve0)),
            Matrix4::identity(),
            &q,
            1e-12,
        )
        .unwrap();
        assert!(
            certify_curve(
                &Curve3D::SurfaceCurve(make(SurfaceCurveRepresentation::Curve3D)),
                Matrix4::identity(),
                &q,
                1e-12
            )
            .is_err()
        );
    }
    #[test]
    fn quadric_incidence_uses_bernstein_residual_not_sampled_points() {
        let q = Quadric {
            origin: Point3::origin(),
            axis: Vector3::unit_z(),
            slope: 0.,
            intercept: 1.,
        };
        let points = vec![
            Vector4::new(1., 0., 0., 1.),
            Vector4::new(1., 0.02, 1. / 3., 1.),
            Vector4::new(1., -0.02, 2. / 3., 1.),
            Vector4::new(1., 0., 1., 1.),
        ];
        // Endpoints and midpoint all lie on the cylinder, interior points do not.
        assert!(certify_bezier(points, &q, 1e-6, 0).is_err());
        let circle = vec![
            Vector4::new(1., 0., 0., 1.),
            Vector4::new(0.5f64.sqrt(), 0.5f64.sqrt(), 0., 0.5f64.sqrt()),
            Vector4::new(0., 1., 0., 1.),
        ];
        certify_bezier(circle, &q, 1e-12, 0).unwrap();
    }
    #[test]
    fn nonfinite_quadric_residual_is_never_a_certificate() {
        let q = Quadric {
            origin: Point3::origin(),
            axis: Vector3::unit_z(),
            slope: 0.,
            intercept: 1.,
        };
        assert!(certify_bezier(vec![Vector4::new(1., 0., 0., -1.)], &q, 1e-6, 0).is_err());
        for value in [f64::NAN, f64::INFINITY] {
            assert!(certify_bezier(vec![Vector4::new(value, 0., 0., 1.)], &q, 1e-6, 0).is_err());
        }
    }
}
