//! Extrema of finite rational Bézier spans, in world coordinates.
use super::{Geometry, Matrix4, point3};
use cadmpeg_ir::geometry::nurbs::NurbsCurve;
use cadmpeg_ir::ids::CurveId;
use cadmpeg_ir::topology::Edge;
use cgmath::{InnerSpace, Transform, Vector4};

impl Geometry<'_> {
    pub(super) fn rational_bounds(
        &self,
        edge: &Edge,
        curve: &CurveId,
        nurbs: &NurbsCurve,
        world: Matrix4,
        include: &mut impl FnMut(usize, f64),
    ) -> Option<()> {
        let degree = usize::try_from(nurbs.degree()).ok()?;
        let knots = nurbs.knots().as_slice();
        let points = nurbs.control_points();
        let weights = nurbs.weights()?;
        let clamped_cubic = degree == 3
            && points.len() == 4
            && knots[..4].iter().all(|k| *k == knots[3])
            && knots[4..].iter().all(|k| *k == knots[4]);
        if nurbs.periodic() || !(degree == 2 || clamped_cubic) {
            return None;
        }

        let domain = [knots[degree], knots[points.len()]];
        let mut trim = if let Some(range) = edge.param_range() {
            range.get()
        } else {
            let a = self.vertex(&edge.start)?;
            let b = self.vertex(&edge.end)?;
            let p = self.point(curve, domain[0])?;
            let q = self.point(curve, domain[1])?;
            let forward = (a - p).magnitude().max((b - q).magnitude());
            let reverse = (a - q).magnitude().max((b - p).magnitude());
            if forward.min(reverse) > 1e-8 {
                return None;
            }
            domain
        };
        trim.sort_by(f64::total_cmp);
        if trim[0] < domain[0] || trim[0] >= trim[1] || trim[1] > domain[1] {
            return None;
        }
        let poles: Vec<_> = points
            .into_iter()
            .zip(weights)
            .map(|(p, w)| {
                let w = w.get();
                let p = world.transform_point(point3(p.get()));
                Vector4::new(p.x, p.y, p.z, 1.0) * w
            })
            .collect();
        if poles
            .iter()
            .any(|p| p.w <= 0.0 || (0..4).any(|i| !p[i].is_finite()))
        {
            return None;
        }
        for span in degree..poles.len() {
            let [a, b] = [knots[span], knots[span + 1]];
            let [low, high] = [a.max(trim[0]), b.min(trim[1])];
            if low >= high {
                continue;
            }
            let range = [(low - a) / (b - a), (high - a) / (b - a)];
            if clamped_cubic {
                include_span(&poles, range, include)?;
            } else {
                // A homogeneous quadratic is determined by the two limiting
                // values and the midpoint on this knot span.
                let at = |t| quadratic_at(&poles[span - 2..=span], knots, span, t);
                let [first, middle, last] = [a, f64::midpoint(a, b), b].map(at);
                include_span(
                    &[first, 2.0 * middle - (first + last) * 0.5, last],
                    range,
                    include,
                )?;
            }
        }
        Some(())
    }
}

fn quadratic_at(poles: &[Vector4<f64>], knots: &[f64], span: usize, t: f64) -> Vector4<f64> {
    let mix = |a: Vector4<f64>, b: Vector4<f64>, low, high| {
        let u = (t - low) / (high - low);
        a * (1.0 - u) + b * u
    };
    let a = mix(poles[0], poles[1], knots[span - 1], knots[span + 1]);
    let b = mix(poles[1], poles[2], knots[span], knots[span + 2]);
    mix(a, b, knots[span], knots[span + 1])
}

fn power(bernstein: &[f64]) -> Vec<f64> {
    match *bernstein {
        [a, b, c] => vec![a, 2.0 * (b - a), (c - b) - (b - a)],
        [a, b, c, d] => {
            let [x, y, z] = [b - a, c - b, d - c];
            vec![a, 3.0 * x, 3.0 * (y - x), (z - y) - (y - x)]
        }
        _ => unreachable!("only quadratic and cubic spans"),
    }
}

fn polynomial(coefficients: &[f64], t: f64) -> f64 {
    coefficients
        .iter()
        .rev()
        .fold(0.0, |value, c| value.mul_add(t, *c))
}

/// Derivative roots partition the interval into monotone pieces. On each
/// piece with a sign change the bracket encloses one root to binary64 precision.
fn roots(coefficients: &[f64], [low, high]: [f64; 2]) -> Vec<f64> {
    let Some(degree) = coefficients.iter().rposition(|c| *c != 0.0) else {
        return Vec::new();
    };
    let interior = |t: &f64| low < *t && *t < high;
    match coefficients[..=degree] {
        [_] => return Vec::new(),
        [c, b] => return [-c / b].into_iter().filter(interior).collect(),
        [c, b, a] => {
            let scale = a.abs().max(b.abs()).max(c.abs());
            let [c, b, a] = [c, b, a].map(|x| x / scale);
            let discriminant = b.mul_add(b, -4.0 * a * c);
            if discriminant < 0.0 {
                return Vec::new();
            }
            // Use the noncancelling form for one root and Vieta for the other.
            let q = -0.5 * (b + discriminant.sqrt().copysign(b));
            if q == 0.0 {
                return [0.0].into_iter().filter(interior).collect();
            }
            let mut result: Vec<_> = [q / a, c / q].into_iter().filter(interior).collect();
            result.sort_by(f64::total_cmp);
            result.dedup();
            return result;
        }
        _ => {}
    }
    let derivative: Vec<_> = (1..=degree).map(|i| i as f64 * coefficients[i]).collect();
    let mut cuts = vec![low];
    cuts.extend(roots(&derivative, [low, high]));
    cuts.push(high);
    let mut result = Vec::new();
    for pair in cuts.windows(2) {
        let (mut a, mut b) = (pair[0], pair[1]);
        let (fa, fb) = (polynomial(coefficients, a), polynomial(coefficients, b));
        if fa == 0.0 && a > low {
            result.push(a);
        }
        if fa == 0.0 || fb == 0.0 || fa.is_sign_negative() == fb.is_sign_negative() {
            continue;
        }
        for _ in 0..64 {
            let mid = f64::midpoint(a, b);
            let fm = polynomial(coefficients, mid);
            if fm == 0.0 || mid == a || mid == b {
                a = mid;
                b = mid;
                break;
            }
            if fm.is_sign_negative() == fa.is_sign_negative() {
                a = mid;
            } else {
                b = mid;
            }
        }
        result.push(f64::midpoint(a, b));
    }
    result
}

fn include_span(
    poles: &[Vector4<f64>],
    range: [f64; 2],
    include: &mut impl FnMut(usize, f64),
) -> Option<()> {
    let weights: Vec<_> = poles.iter().map(|p| p.w).collect();
    let denominator = power(&weights);
    for axis in 0..3 {
        let numerator = power(&poles.iter().map(|p| p[axis]).collect::<Vec<_>>());
        // N'W - NW'. Pair the opposite terms to cancel the highest power
        // before root isolation. Positive weights keep W away from zero.
        let mut derivative = [0.0; 5];
        for i in 0..numerator.len() {
            for j in i + 1..numerator.len() {
                derivative[i + j - 1] += (j - i) as f64
                    * (numerator[j] * denominator[i] - numerator[i] * denominator[j]);
            }
        }
        if derivative.iter().any(|v| !v.is_finite()) {
            return None;
        }
        for t in range.into_iter().chain(roots(&derivative, range)) {
            let mut work = [Vector4::new(0.0, 0.0, 0.0, 0.0); 4];
            work[..poles.len()].copy_from_slice(poles);
            for n in (1..poles.len()).rev() {
                for i in 0..n {
                    work[i] = work[i] * (1.0 - t) + work[i + 1] * t;
                }
            }
            let value = work[0][axis] / work[0].w;
            if !value.is_finite() {
                return None;
            }
            include(axis, value);
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_extremum_is_included_only_on_the_trimmed_span() {
        let poles = [
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, 1.0, 0.0, 0.5),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        for (range, expected) in [([0.0, 1.0], 2.0 / 3.0), ([0.0, 0.25], 6.0 / 13.0)] {
            let mut ymax: f64 = 0.0;
            include_span(&poles, range, &mut |axis, value| {
                if axis == 1 {
                    ymax = ymax.max(value);
                }
            })
            .unwrap();
            assert!((ymax - expected).abs() < 1e-14, "{range:?}: {ymax}");
        }
    }
}
