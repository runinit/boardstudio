//! Whole-domain control algebra for captured concrete polynomial/rational stages.
//! This proves only the supplied immediate mutation, never its earlier selection.
use super::*;
use monstertruck_io::step::load::convert::{StepRepairCurveChange, StepRepairRawSplineChange};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
type Polynomial = Vec<Q>;
type Homogeneous = [Polynomial; 4];
mod planar_seam;
pub(super) use planar_seam::bound as planar_bound;

fn scalar(value: f64) -> Result<Q, &'static str> {
    Q::from_float(value).ok_or("nonfinite finite-curve coefficient")
}
fn add(a: &[Q], b: &[Q]) -> Polynomial {
    (0..a.len().max(b.len()))
        .map(|i| a.get(i).cloned().unwrap_or_default() + b.get(i).cloned().unwrap_or_default())
        .collect()
}
fn scale(a: &[Q], b: &Q) -> Polynomial {
    a.iter().map(|a| a * b).collect()
}
fn product(a: &[Q], b: &[Q]) -> Polynomial {
    let mut result = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, a) in a.iter().enumerate() {
        for (j, b) in b.iter().enumerate() {
            result[i + j] += a * b;
        }
    }
    result
}
fn binomial(n: usize, k: usize) -> u64 {
    (0..k.min(n - k)).fold(1, |value, i| value * (n - i) as u64 / (i + 1) as u64)
}
fn bernstein(power: &[Q], degree: usize) -> Polynomial {
    (0..=degree)
        .map(|k| {
            (0..=k.min(power.len() - 1))
                .map(|i| &power[i] * Q::new(binomial(k, i).into(), binomial(degree, i).into()))
                .sum()
        })
        .collect()
}
struct Spline {
    degree: usize,
    knots: Vec<Q>,
    controls: Vec<[Q; 4]>,
}
impl Spline {
    fn from_curve(curve: &Curve3D) -> Result<Self, &'static str> {
        let (degree, count) = match curve {
            Curve3D::BsplineCurve(c) => (c.degree(), c.control_points().len()),
            Curve3D::NurbsCurve(c) => (c.degree(), c.control_points().len()),
            _ => return Err("captured finite carrier family has no concrete control proof"),
        };
        if degree == 0 || degree > 16 || count > 8192 || count <= degree {
            return Err("finite-curve control proof resource/domain limit");
        }
        let (degree, knots, controls) = match curve {
            Curve3D::BsplineCurve(c) => (
                c.degree(),
                c.knot_vector(),
                c.control_points()
                    .iter()
                    .map(|p| Ok([scalar(p.x)?, scalar(p.y)?, scalar(p.z)?, Q::one()]))
                    .collect::<Result<Vec<_>, &'static str>>()?,
            ),
            Curve3D::NurbsCurve(c) => (
                c.degree(),
                c.knot_vector(),
                c.control_points()
                    .iter()
                    .map(|p| Ok([scalar(p.x)?, scalar(p.y)?, scalar(p.z)?, scalar(p.w)?]))
                    .collect::<Result<Vec<_>, &'static str>>()?,
            ),
            _ => return Err("captured finite carrier family has no concrete control proof"),
        };
        if degree == 0
            || degree > 16
            || controls.len() > 8192
            || knots.len() != controls.len() + degree + 1
            || controls.len() <= degree
        {
            return Err("finite-curve control proof resource/domain limit");
        }
        let knots = knots
            .iter()
            .map(|k| scalar(*k))
            .collect::<Result<Vec<_>, _>>()?;
        if knots.windows(2).any(|k| k[0] > k[1]) || controls.iter().any(|p| !p[3].is_positive()) {
            return Err("invalid finite-curve knots or rational weights");
        }
        // A directed change at a discontinuity could select opposite one-sided
        // values at the same label. Do not manufacture endpoint correspondence.
        if knots.windows(degree + 1).any(|run| {
            run[0] == run[degree] && run[0] > knots[degree] && run[0] < knots[controls.len()]
        }) {
            return Err("discontinuous finite spline needs one-sided domain authority");
        }
        Ok(Self {
            degree,
            knots,
            controls,
        })
    }
    fn domain(&self, domain: (f64, f64)) -> Result<(Q, Q), &'static str> {
        let (a, b) = (scalar(domain.0)?, scalar(domain.1)?);
        if a == b
            || a.clone().min(b.clone()) < self.knots[self.degree]
            || a.clone().max(b.clone()) > self.knots[self.controls.len()]
        {
            return Err("finite carrier interval is unresolved or outside active support");
        }
        Ok((a.clone(), b - a))
    }
    fn piece(&self, domain: &(Q, Q), lo: &Q, hi: &Q) -> Result<Homogeneous, &'static str> {
        let origin = &domain.0 + &domain.1 * lo;
        let advance = &domain.1 * (hi - lo);
        let midpoint = &origin + &advance / Q::from_integer(2.into());
        let span = self
            .knots
            .partition_point(|k| k <= &midpoint)
            .checked_sub(1)
            .ok_or("missing finite spline span")?;
        if span < self.degree
            || span >= self.controls.len()
            || self.knots[span] == self.knots[span + 1]
        {
            return Err("invalid finite spline span");
        }
        let mut net: Vec<Homogeneous> = self.controls[span - self.degree..=span]
            .iter()
            .map(|p| std::array::from_fn(|axis| vec![p[axis].clone()]))
            .collect();
        // Symbolic de Boor on this complete knot span. No sampled evaluation.
        for level in 1..=self.degree {
            for j in (level..=self.degree).rev() {
                let index = span - self.degree + j;
                let denominator = &self.knots[index + self.degree - level + 1] - &self.knots[index];
                if denominator.is_zero() {
                    return Err("ambiguous finite spline span");
                }
                let alpha = vec![
                    (&origin - &self.knots[index]) / &denominator,
                    &advance / &denominator,
                ];
                let complement = vec![Q::one() - &alpha[0], -&alpha[1]];
                net[j] = std::array::from_fn(|axis| {
                    add(
                        &product(&net[j - 1][axis], &complement),
                        &product(&net[j][axis], &alpha),
                    )
                });
            }
        }
        Ok(net.pop().unwrap())
    }
}

/// A symmetric bound for the SAME captured directed finite pieces. Earlier
/// source/master/selection coverage remains an independent Unsupported stage.
pub(super) fn bound(
    change: StepRepairCurveChange<'_>,
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    let (before, after) = change.curves();
    let domains = change.domains();
    controls_bound(before, domains.0, after, domains.1, factor, placement)
}
fn controls_bound(
    before: &Curve3D,
    before_domain: (f64, f64),
    after: &Curve3D,
    after_domain: (f64, f64),
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    if placement.x.w != 0.0 || placement.y.w != 0.0 || placement.z.w != 0.0 || placement.w.w != 1.0
    {
        return Err("non-affine finite-curve occurrence");
    }
    let factor = scalar(factor)?;
    if !factor.is_positive() {
        return Err("invalid finite-curve unit transform");
    }
    let a = Spline::from_curve(before)?;
    let da = a.domain(before_domain)?;
    // Full retained carrier/control/knot equality plus the complete finite
    // domain is authority for this one mutation, never endpoint equality.
    if before == after && before_domain == after_domain {
        return Ok(0.0);
    }
    let b = Spline::from_curve(after)?;
    let db = b.domain(after_domain)?;
    polynomial_bound(&a, &da, &b, &db, &factor, placement)
}
fn polynomial_bound(
    a: &Spline,
    da: &(Q, Q),
    b: &Spline,
    db: &(Q, Q),
    factor: &Q,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    let mut breaks = vec![Q::zero(), Q::one()];
    for (spline, domain) in [(a, da), (b, db)] {
        breaks.extend(
            spline
                .knots
                .iter()
                .map(|k| (k - &domain.0) / &domain.1)
                .filter(|k| k > &Q::zero() && k < &Q::one()),
        );
    }
    breaks.sort();
    breaks.dedup();
    if breaks.len() > 257 {
        return Err("finite-curve span proof resource cap");
    }
    let degree = a.degree + b.degree;
    let mut squared = Q::zero();
    for interval in breaks.windows(2) {
        let a = a.piece(da, &interval[0], &interval[1])?;
        let b = b.piece(db, &interval[0], &interval[1])?;
        let denominator = bernstein(&product(&a[3], &b[3]), degree);
        let lower = denominator.into_iter().min().unwrap();
        if !lower.is_positive() {
            return Err("finite rational denominator is not positively bounded");
        }
        let delta: [Polynomial; 3] = std::array::from_fn(|axis| {
            add(
                &product(&b[axis], &a[3]),
                &scale(&product(&a[axis], &b[3]), &-Q::one()),
            )
        });
        let mut world: [Polynomial; 3] = std::array::from_fn(|_| vec![Q::zero()]);
        for row in 0..3 {
            for col in 0..3 {
                world[row] = add(
                    &world[row],
                    &scale(&delta[col], &(scalar(placement[col][row])? * factor)),
                );
            }
        }
        let nets = world.each_ref().map(|p| bernstein(p, degree));
        for i in 0..=degree {
            let norm: Q = nets.iter().map(|p| &p[i] * &p[i]).sum();
            squared = squared.max(norm / (&lower * &lower));
        }
    }
    // Convex hulls bound the full vector numerator and positive denominator on
    // every piece; the directed affine parameter map covers both finite domains.
    sqrt_upper(&squared, "finite-curve movement range")
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_modeling::{BsplineCurve, KnotVector};
    fn curve(y: f64) -> Curve3D {
        Curve3D::BsplineCurve(BsplineCurve::new_unchecked(
            KnotVector::from(vec![0., 0., 0., 1., 1., 1.]),
            vec![
                Point3::new(0., 0., 0.),
                Point3::new(1., y, 0.),
                Point3::new(2., 0., 0.),
            ],
        ))
    }
    #[test]
    fn same_endpoints_do_not_hide_interior_movement() {
        let a = curve(0.);
        let b = curve(0.0625);
        assert_eq!(a.front(), b.front());
        assert_eq!(a.back(), b.back());
        let upper = controls_bound(&a, (0., 1.), &b, (0., 1.), 1., Matrix4::identity()).unwrap();
        assert!((0.03125..=0.0625).contains(&upper));
        assert_eq!(
            upper,
            controls_bound(&b, (0., 1.), &a, (0., 1.), 1., Matrix4::identity()).unwrap()
        );
    }
    #[test]
    fn whole_curve_bound_keeps_outward_rounding_at_numeric_extremes() {
        let segment = |end| {
            Curve3D::BsplineCurve(BsplineCurve::new_unchecked(
                KnotVector::bezier_knot(1),
                vec![Point3::origin(), end],
            ))
        };
        let original = segment(Point3::new(1., 0., 0.));
        let bound = |repaired: &Curve3D| {
            controls_bound(
                &original,
                (0., 1.),
                repaired,
                (0., 1.),
                1.,
                Matrix4::identity(),
            )
        };
        let diagonal = segment(Point3::new(2., 1., 1.));
        // The full difference net has squared norm 3; nearest sqrt rounds down.
        assert_eq!(bound(&diagonal), Ok(3.0_f64.sqrt().next_up()));
        assert_eq!(
            bound(&segment(Point3::new(1., 2.0_f64.powi(-530), 0.))),
            Ok(2.0_f64.powi(-530))
        );
        assert_eq!(
            bound(&segment(Point3::new(1., 2.0_f64.powi(-600), 0.))),
            Ok(f64::MIN_POSITIVE.sqrt())
        );
        assert_eq!(
            bound(&segment(Point3::new(1., f64::MAX, 0.))),
            Err("finite-curve movement range")
        );
        assert_eq!(
            controls_bound(
                &original,
                (0., 1.),
                &original.inverse(),
                (1., 0.),
                1.,
                Matrix4::identity()
            ),
            Ok(0.)
        );
    }

    #[test]
    fn discontinuous_direction_changes_require_one_sided_domain_authority() {
        let curve = Curve3D::BsplineCurve(BsplineCurve::new_unchecked(
            KnotVector::from(vec![0., 0., 0.5, 0.5, 1., 1.]),
            vec![
                Point3::new(0., 0., 0.),
                Point3::new(1., 0., 0.),
                Point3::new(1., 1., 0.),
                Point3::new(2., 1., 0.),
            ],
        ));
        assert_eq!(
            controls_bound(
                &curve,
                (0., 1.),
                &curve.inverse(),
                (1., 0.),
                1.,
                Matrix4::identity()
            ),
            Err("discontinuous finite spline needs one-sided domain authority")
        );
    }

    #[test]
    fn actual_partial_spline_mutations_have_exact_whole_domain_control_proofs() {
        let bytes = include_bytes!(
            "../../../../monstertruck-qualification/vendor/monstertruck-io/tests/input/partial_closed_spline/solid32377-source.step"
        );
        let table = Table::from_step_bytes(bytes).unwrap();
        let original = StepRepairSource::from_step_bytes(bytes).unwrap();
        let owner = table
            .to_source_bound_solid_with_repair_checked(32377, Some(17), &original)
            .unwrap();
        let mut measured = 0;
        for operation in owner.operations(0) {
            if let Some(change) = operation.curve_change() {
                if !matches!(
                    change.curves().0,
                    Curve3D::BsplineCurve(_) | Curve3D::NurbsCurve(_)
                ) {
                    assert!(
                        bound(change, 1., Matrix4::identity()).is_err(),
                        "an attachment view is not spline/source authority"
                    );
                    continue;
                }
                let movement = bound(change, 1., Matrix4::identity()).unwrap();
                assert_eq!(
                    movement, 0.,
                    "exact source-knot restriction must preserve its selected finite support"
                );
                measured += 1;
            }
        }
        assert!(measured >= 2);
        assert!(
            owner.complete_bound(0).is_err(),
            "selected-domain proof does not authenticate earlier original selection/master coverage"
        );
    }

    #[test]
    fn directed_domain_and_world_units_are_part_of_whole_curve_proof() {
        let a = curve(0.);
        let b = curve(0.0625);
        let identity = controls_bound(&a, (0., 1.), &b, (0., 1.), 1., Matrix4::identity()).unwrap();
        assert_eq!(
            controls_bound(
                &a,
                (0., 1.),
                &b.inverse(),
                (1., 0.),
                1000.,
                Matrix4::identity()
            )
            .unwrap(),
            identity * 1000.
        );
        assert_eq!(
            controls_bound(&a, (0., 0.), &b, (0., 1.), 1., Matrix4::identity()),
            Err("finite carrier interval is unresolved or outside active support")
        );
        assert!(controls_bound(&a, (-1., 1.), &b, (0., 1.), 1., Matrix4::identity()).is_err());
    }
}

/// Largest world-space distance between the kept curve's two ends and the
/// original native-frame vertices, from exact rational end points.
///
/// Only a clamped carrier ends at its first/last control point, so anything else
/// stays Unsupported rather than borrowing an endpoint sample.
pub(super) fn end_residual(
    change: StepRepairCurveChange<'_>,
    ends: &[monstertruck_modeling::Point3; 2],
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    let (curve, _) = change.curves();
    let (degree, knots, controls): (usize, &[f64], Vec<[f64; 4]>) = match curve {
        Curve3D::BsplineCurve(c) => (
            c.degree(),
            c.knot_vector(),
            c.control_points()
                .iter()
                .map(|p| [p.x, p.y, p.z, 1.])
                .collect(),
        ),
        Curve3D::NurbsCurve(c) => (
            c.degree(),
            c.knot_vector(),
            c.control_points()
                .iter()
                .map(|p| [p.x, p.y, p.z, p.w])
                .collect(),
        ),
        _ => return Err("full-interval carrier family has no concrete end proof"),
    };
    let n = controls.len();
    let (a, b) = change.domains().0;
    if degree == 0
        || n <= degree
        || knots.len() != n + degree + 1
        || knots[..=degree].iter().any(|k| *k != a)
        || knots[n..].iter().any(|k| *k != b)
    {
        return Err("full-interval carrier is not clamped; its ends are not control points");
    }
    let mut worst = 0.0_f64;
    for (end, vertex) in [(&controls[0], &ends[0]), (&controls[n - 1], &ends[1])] {
        let w = scalar(end[3])?;
        if !w.is_positive() {
            return Err("full-interval carrier end weight is not positive");
        }
        let mut delta = [0.0; 3];
        for axis in 0..3 {
            let d = (scalar(end[axis])? / &w - scalar(vertex[axis])?).abs();
            delta[axis] = sqrt_upper(&(&d * &d), "full-interval end residual range")?;
        }
        worst = worst.max(super::placed_translation_bound(delta, factor, placement)?);
    }
    Ok(worst)
}

/// Ideal tiny-span knot normalization of a clamped non-rational spline.
///
/// The positive affine map `(k - first) / (last - first)` relabels the parameter
/// of every polynomial piece and moves no point of the carrier, so this repair
/// term is zero when the captured intermediate keeps the original degree, control
/// points, knot multiplicity pattern and the full `[0, 1]` extent. Binary64
/// rounding of the normalized knots is a separate numerical term.
pub(super) fn raw_spline_bound(raw: StepRepairRawSplineChange<'_>) -> Result<f64, &'static str> {
    raw_spline_identity(
        raw.degree(),
        raw.knots(),
        raw.controls(),
        raw.domains(),
        raw.after(),
    )
}

fn raw_spline_identity(
    degree: usize,
    knots: &[f64],
    controls: &[monstertruck_modeling::Point3],
    domains: ((f64, f64), (f64, f64)),
    after: &Curve3D,
) -> Result<f64, &'static str> {
    let ((first, last), normalized) = domains;
    if degree == 0
        || degree > 16
        || controls.len() > 8192
        || controls.len() <= degree
        || knots.len() != controls.len() + degree + 1
    {
        return Err("raw spline resource or shape limit");
    }
    if !knots.iter().all(|k| k.is_finite())
        || knots.windows(2).any(|pair| pair[0] > pair[1])
        || knots[0] != first
        || knots[knots.len() - 1] != last
        || !(first < last)
        || knots[..=degree].iter().any(|k| *k != first)
        || knots[controls.len()..].iter().any(|k| *k != last)
    {
        return Err("raw spline is not a clamped full-extent carrier");
    }
    let Curve3D::BsplineCurve(after) = after else {
        return Err("normalized intermediate is not a non-rational spline");
    };
    let after_knots = after.knot_vector();
    if normalized != (0., 1.)
        || after.degree() != degree
        || after.control_points().as_slice() != controls
        || after_knots.len() != knots.len()
    {
        return Err("normalized intermediate changed degree, controls or extent");
    }
    // Every label must be exactly the saved `(k - first) / (last - first)` recipe,
    // in that arithmetic order. Order or multiplicity alone would admit an interior
    // label shift, which is a different parametrisation and image-preserving only
    // by accident. The recipe is the ideal positive affine map up to binary64
    // rounding, which stays a separate numerical term.
    let span = last - first;
    if !(span.is_finite() && span > 0.0)
        || knots
            .iter()
            .zip(after_knots.iter())
            .any(|(raw, normalized)| (raw - first) / span != *normalized)
    {
        return Err("normalized intermediate is not the saved positive affine knot recipe");
    }
    Ok(0.0)
}

#[cfg(test)]
mod raw_spline_tests {
    use super::*;
    use monstertruck_modeling::{BsplineCurve, KnotVector, Point3};

    fn controls() -> Vec<Point3> {
        (0..3)
            .map(|i| Point3::new(f64::from(i), f64::from(i * i), 0.))
            .collect()
    }
    fn raw_knots() -> Vec<f64> {
        vec![3., 3., 3. + 3e-8, 3. + 8e-8, 3. + 8e-8]
    }
    fn recipe(knots: &[f64]) -> Vec<f64> {
        let span = knots[knots.len() - 1] - knots[0];
        knots.iter().map(|k| (k - knots[0]) / span).collect()
    }
    fn after(knots: Vec<f64>) -> Curve3D {
        Curve3D::BsplineCurve(BsplineCurve::new_unchecked(
            KnotVector::from(knots),
            controls(),
        ))
    }
    fn check(after: &Curve3D) -> Result<f64, &'static str> {
        raw_spline_identity(
            1,
            &raw_knots(),
            &controls(),
            ((3., 3. + 8e-8), (0., 1.)),
            after,
        )
    }

    #[test]
    fn interior_knot_must_match_the_saved_affine_recipe() {
        assert_eq!(check(&after(recipe(&raw_knots()))), Ok(0.0));
        // Same order, multiplicity, ends, degree and controls: only the interior
        // label moved, which changes the parametrisation of both spans.
        let mut shifted = recipe(&raw_knots());
        shifted[2] += 0.125;
        assert!(shifted[1] < shifted[2] && shifted[2] < shifted[3]);
        assert!(check(&after(shifted)).is_err());
    }
}
