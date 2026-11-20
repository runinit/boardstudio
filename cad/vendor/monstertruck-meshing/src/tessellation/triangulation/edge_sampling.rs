use super::*;

const MAX_DEPTH: usize = 20;
const MAX_SAMPLES: usize = 65_536;

// The endpoint tests bound the turn between adjacent accepted chords at a
// regular shared tangent. Interior checks are sampled, not a continuous proof.
pub(super) fn sample_edge<C: PolylineableCurve>(
    curve: &C,
    linear: f64,
    angular: Option<f64>,
) -> std::result::Result<PolylineCurve, EdgeSamplingFailure> {
    let Some(angular) = angular else {
        return Ok(PolylineCurve::from_curve(
            curve,
            curve.range_tuple(),
            linear,
        ));
    };
    if !linear.is_finite() || linear <= 0.0 || invalid_edge_angular_tolerance(Some(angular)) {
        return Err(EdgeSamplingFailure {
            seed: PolylineCurve::from(Vec::new()),
            reason: EdgeSamplingError::InvalidOptions,
        });
    }
    let range = curve.range_tuple();
    let (parameters, points) = curve.parameter_division(range, linear);
    let result = refine_seed(curve, range, &parameters, &points, linear, angular);
    result
        .map(PolylineCurve::from)
        .map_err(|reason| EdgeSamplingFailure {
            seed: PolylineCurve::from(points),
            reason,
        })
}

fn finite_vector(v: Vector3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

fn unit(v: Vector3) -> std::result::Result<Vector3, EdgeSamplingError> {
    if !finite_vector(v) {
        return Err(EdgeSamplingError::NonFinite);
    }
    let scale = v.x.abs().max(v.y.abs()).max(v.z.abs());
    if scale == 0.0 {
        return Err(EdgeSamplingError::SingularTangent);
    }
    let scaled = v / scale;
    Ok(scaled / scaled.magnitude())
}

#[derive(Clone, Copy)]
struct Sample {
    parameter: f64,
    point: Point3,
    tangent: Vector3,
}

fn sample<C: PolylineableCurve>(
    curve: &C,
    parameter: f64,
    point: Point3,
    direction: f64,
) -> std::result::Result<Sample, EdgeSamplingError> {
    if !finite_vector(point.to_vec()) {
        return Err(EdgeSamplingError::NonFinite);
    }
    Ok(Sample {
        parameter,
        point,
        tangent: unit(curve.derivative(parameter))? * direction,
    })
}

fn refine_seed<C: PolylineableCurve>(
    curve: &C,
    range: (f64, f64),
    parameters: &[f64],
    points: &[Point3],
    linear: f64,
    angular: f64,
) -> std::result::Result<Vec<Point3>, EdgeSamplingError> {
    if !range.0.is_finite()
        || !range.1.is_finite()
        || range.0 == range.1
        || parameters.len() != points.len()
        || points.len() < 2
        || parameters.first() != Some(&range.0)
        || parameters.last() != Some(&range.1)
    {
        return Err(EdgeSamplingError::InvalidSeed);
    }
    if points.len() > MAX_SAMPLES {
        return Err(EdgeSamplingError::BudgetExceeded);
    }
    let direction = if range.1 > range.0 { 1.0 } else { -1.0 };
    if parameters.iter().any(|t| !t.is_finite())
        || parameters
            .windows(2)
            .any(|w| (w[1] - w[0]) * direction <= 0.0)
    {
        return Err(EdgeSamplingError::InvalidSeed);
    }
    let mut output = vec![points[0]];
    let mut budget = MAX_SAMPLES - points.len();
    let mut left = sample(curve, parameters[0], points[0], direction)?;
    for index in 1..points.len() {
        let right = sample(curve, parameters[index], points[index], direction)?;
        refine_interval(
            curve,
            left,
            right,
            direction,
            linear,
            angular,
            0,
            &mut budget,
            &mut output,
        )?;
        left = right;
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn refine_interval<C: PolylineableCurve>(
    curve: &C,
    left: Sample,
    right: Sample,
    direction: f64,
    linear: f64,
    angular: f64,
    depth: usize,
    budget: &mut usize,
    output: &mut Vec<Point3>,
) -> std::result::Result<(), EdgeSamplingError> {
    let midpoint = left.parameter * 0.5 + right.parameter * 0.5;
    if midpoint == left.parameter || midpoint == right.parameter {
        return Err(EdgeSamplingError::NoProgress);
    }
    if *budget == 0 {
        return Err(EdgeSamplingError::BudgetExceeded);
    }
    *budget -= 1;
    let middle = sample(curve, midpoint, curve.evaluate(midpoint), direction)?;
    let chord = right.point - left.point;
    if !finite_vector(chord) {
        return Err(EdgeSamplingError::NonFinite);
    }
    let length = chord.magnitude();
    if !length.is_finite() {
        return Err(EdgeSamplingError::NonFinite);
    }
    let acceptable = if length > 0.0 {
        let chord_unit = chord / length;
        let cosine = (angular.min(std::f64::consts::TAU) * 0.5).cos();
        let tangents_ok = [left.tangent, middle.tangent, right.tangent]
            .into_iter()
            .all(|t| chord_unit.dot(t).clamp(-1.0, 1.0) >= cosine);
        let offset = middle.point - left.point;
        let foot = left.point + chord_unit * offset.dot(chord_unit).clamp(0.0, length);
        let distance = (middle.point - foot).magnitude();
        if !distance.is_finite() {
            return Err(EdgeSamplingError::NonFinite);
        }
        tangents_ok && distance <= linear
    } else {
        false
    };
    if acceptable {
        output.push(right.point);
        return Ok(());
    }
    if depth == MAX_DEPTH {
        return Err(EdgeSamplingError::BudgetExceeded);
    }
    refine_interval(
        curve,
        left,
        middle,
        direction,
        linear,
        angular,
        depth + 1,
        budget,
        output,
    )?;
    refine_interval(
        curve,
        middle,
        right,
        direction,
        linear,
        angular,
        depth + 1,
        budget,
        output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Bound;

    #[derive(Clone)]
    struct Arc;
    impl ParametricCurve for Arc {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, t: f64) -> Point3 {
            Point3::new(t.cos(), t.sin(), 0.0)
        }
        fn derivative(&self, t: f64) -> Vector3 {
            Vector3::new(-t.sin(), t.cos(), 0.0)
        }
        fn derivative_2(&self, t: f64) -> Vector3 {
            Vector3::new(-t.cos(), -t.sin(), 0.0)
        }
        fn derivative_n(&self, n: usize, t: f64) -> Vector3 {
            let t = t + n as f64 * std::f64::consts::FRAC_PI_2;
            Vector3::new(t.cos(), t.sin(), 0.0)
        }
        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(0.0), Bound::Included(1.1))
        }
    }
    impl BoundedCurve for Arc {}
    impl ParameterDivision1D for Arc {
        type Point = Point3;
        fn parameter_division(&self, range: (f64, f64), _: f64) -> (Vec<f64>, Vec<Point3>) {
            (
                vec![range.0, range.1],
                vec![self.evaluate(range.0), self.evaluate(range.1)],
            )
        }
    }

    #[test]
    fn angular_edge_refines_linear_seed() {
        let points = sample_edge(&Arc, 0.2, Some(0.5)).ok().unwrap();
        assert!(points.len() > 2, "linear-only seed ignores angular budget");
        for triple in points.windows(3) {
            let a = triple[1] - triple[0];
            let b = triple[2] - triple[1];
            assert!(a.angle(b).0 <= 0.5);
        }
        assert_eq!(points[0], Arc.evaluate(0.0));
        assert_eq!(*points.last().unwrap(), Arc.evaluate(1.1));
    }

    #[test]
    fn no_angular_option_preserves_linear_seed() {
        let points = sample_edge(&Arc, 0.2, None).ok().unwrap();
        assert_eq!(&points[..], &[Arc.evaluate(0.0), Arc.evaluate(1.1)]);
    }
    #[derive(Clone)]
    struct SeededCurve {
        kind: u8,
        range: (f64, f64),
        parameters: Vec<f64>,
    }
    impl ParametricCurve for SeededCurve {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, t: f64) -> Point3 {
            match self.kind {
                0 => Arc.evaluate(t),
                1 => Point3::new(t, 2.0 * t * t * t, 0.0),
                _ => Point3::new(t * t, 0.0, 0.0),
            }
        }
        fn derivative(&self, t: f64) -> Vector3 {
            match self.kind {
                0 => Arc.derivative(t),
                1 => Vector3::new(1.0, 6.0 * t * t, 0.0),
                _ => Vector3::new(2.0 * t, 0.0, 0.0),
            }
        }
        fn derivative_2(&self, t: f64) -> Vector3 {
            match self.kind {
                0 => Arc.derivative_2(t),
                1 => Vector3::new(0.0, 12.0 * t, 0.0),
                _ => Vector3::new(2.0, 0.0, 0.0),
            }
        }
        fn derivative_n(&self, n: usize, t: f64) -> Vector3 {
            match n {
                0 => self.evaluate(t).to_vec(),
                1 => self.derivative(t),
                2 => self.derivative_2(t),
                3 if self.kind == 1 => Vector3::new(0.0, 12.0, 0.0),
                _ if self.kind == 0 => Arc.derivative_n(n, t),
                _ => Vector3::zero(),
            }
        }
        fn parameter_range(&self) -> ParameterRange {
            (Bound::Included(self.range.0), Bound::Included(self.range.1))
        }
    }
    impl BoundedCurve for SeededCurve {}
    impl ParameterDivision1D for SeededCurve {
        type Point = Point3;
        fn parameter_division(&self, _: (f64, f64), _: f64) -> (Vec<f64>, Vec<Point3>) {
            (
                self.parameters.clone(),
                self.parameters.iter().map(|t| self.evaluate(*t)).collect(),
            )
        }
    }

    #[test]
    fn angular_sampling_keeps_each_seed_and_reverse_order() {
        let curve = SeededCurve {
            kind: 0,
            range: (1.1, 0.0),
            parameters: vec![1.1, 0.71, 0.3, 0.0],
        };
        let points = sample_edge(&curve, 0.1, Some(0.2)).ok().unwrap();
        let mut cursor = 0;
        for t in &curve.parameters {
            cursor += points[cursor..]
                .iter()
                .position(|p| *p == curve.evaluate(*t))
                .unwrap();
        }
        for p in points.windows(3) {
            assert!((p[1] - p[0]).angle(p[2] - p[1]).0 <= 0.2);
        }
    }

    #[test]
    fn angular_sampling_handles_closed_curve_without_welding_endpoints() {
        let curve = SeededCurve {
            kind: 0,
            range: (0.0, std::f64::consts::TAU),
            parameters: vec![0.0, std::f64::consts::TAU],
        };
        let points = sample_edge(&curve, 0.1, Some(0.5)).ok().unwrap();
        assert!(points.len() > 8);
        assert_eq!(points[0], curve.evaluate(0.0));
        assert_eq!(
            *points.last().unwrap(),
            curve.evaluate(std::f64::consts::TAU)
        );
        for p in points.windows(3) {
            assert!((p[1] - p[0]).angle(p[2] - p[1]).0 <= 0.5);
        }
    }

    #[test]
    fn angular_sampling_detects_inflection_hidden_from_midpoint_distance() {
        let curve = SeededCurve {
            kind: 1,
            range: (-1.0, 1.0),
            parameters: vec![-1.0, 1.0],
        };
        let points = sample_edge(&curve, 0.1, Some(0.5)).ok().unwrap();
        assert!(points.len() > 2);
        for p in points.windows(3) {
            assert!((p[1] - p[0]).angle(p[2] - p[1]).0 <= 0.5);
        }
    }

    #[test]
    fn angular_sampling_refuses_stationary_touch_and_preserves_failure_seed() {
        let curve = SeededCurve {
            kind: 2,
            range: (-1.0, 1.0),
            parameters: vec![-1.0, 1.0],
        };
        let error = sample_edge(&curve, 0.1, Some(0.5)).err().unwrap();
        assert_eq!(error.reason, EdgeSamplingError::SingularTangent);
        assert_eq!(
            &error.seed[..],
            &[curve.evaluate(-1.0), curve.evaluate(1.0)]
        );
    }

    #[test]
    fn angular_sampling_refuses_duplicate_parameter_seed() {
        let curve = SeededCurve {
            kind: 0,
            range: (0.0, 1.0),
            parameters: vec![0.0, 0.5, 0.5, 1.0],
        };
        let error = sample_edge(&curve, 0.1, Some(0.5)).err().unwrap();
        assert_eq!(error.reason, EdgeSamplingError::InvalidSeed);
        assert_eq!(error.seed.len(), 4);
    }

    #[test]
    fn angular_sampling_preserves_repeated_knot_rational_curve_seeds() {
        use monstertruck_geometry::prelude::{BsplineCurve, KnotVector, NurbsCurve, Vector4};
        let knots = KnotVector::from(vec![0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0]);
        let control: Vec<_> = (0..6)
            .map(|index| {
                let x = index as f64 / 5.0;
                let weight = 1.0 + 0.1 * index as f64;
                Vector4::new(x * weight, 0.2 * x * x * weight, 0.0, weight)
            })
            .collect();
        let curve = NurbsCurve::new(BsplineCurve::new(knots, control));
        let (_, seed) = curve.parameter_division(curve.range_tuple(), 0.01);
        let output = sample_edge(&curve, 0.01, Some(0.2)).ok().unwrap();
        let mut cursor = 0;
        for point in seed {
            cursor += output[cursor..].iter().position(|p| *p == point).unwrap();
        }
        for p in output.windows(3) {
            assert!((p[1] - p[0]).angle(p[2] - p[1]).0 <= 0.2);
        }
    }

    #[test]
    fn angular_sampling_invalid_options_and_nonprogress_are_typed() {
        for angular in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            let failure = sample_edge(&Arc, 0.1, Some(angular)).err().unwrap();
            assert_eq!(failure.reason, EdgeSamplingError::InvalidOptions);
        }
        let next = f64::from_bits(1.0_f64.to_bits() + 1);
        let curve = SeededCurve {
            kind: 0,
            range: (1.0, next),
            parameters: vec![1.0, next],
        };
        let failure = sample_edge(&curve, 0.1, Some(0.5)).err().unwrap();
        assert_eq!(failure.reason, EdgeSamplingError::NoProgress);
    }

    #[test]
    fn angular_sampling_refuses_c0_knot_corner_without_coarse_fallback() {
        use monstertruck_geometry::prelude::{BsplineCurve, KnotVector};
        let curve = BsplineCurve::new(
            KnotVector::from(vec![0., 0., 0.5, 1., 1.]),
            vec![
                Point3::new(0., 0., 0.),
                Point3::new(1., 0., 0.),
                Point3::new(1., 1., 0.),
            ],
        );
        let failure = sample_edge(&curve, 0.1, Some(0.5)).unwrap_err();
        assert_eq!(failure.reason, EdgeSamplingError::BudgetExceeded);
        assert_eq!(failure.seed[0], curve.evaluate(0.));
        assert_eq!(*failure.seed.last().unwrap(), curve.evaluate(1.));
    }

    #[test]
    fn angular_sampling_budget_and_nonfinite_seed_refuse() {
        let failure = sample_edge(&Arc, 0.1, Some(1.0e-10)).unwrap_err();
        assert_eq!(failure.reason, EdgeSamplingError::BudgetExceeded);
        assert_eq!(failure.seed.len(), 2);
        let failure = refine_seed(
            &Arc,
            (0., 1.),
            &[0., 1.],
            &[Point3::new(f64::NAN, 0., 0.), Arc.evaluate(1.)],
            0.1,
            0.5,
        )
        .unwrap_err();
        assert_eq!(failure, EdgeSamplingError::NonFinite);
        assert_eq!(
            unit(Vector3::new(f64::INFINITY, 0., 0.)).unwrap_err(),
            EdgeSamplingError::NonFinite
        );
        assert_eq!(
            unit(Vector3::new(1.0e-300, 0., 0.)).unwrap(),
            Vector3::unit_x()
        );
        assert_eq!(
            unit(Vector3::new(1.0e300, 0., 0.)).unwrap(),
            Vector3::unit_x()
        );
    }
}
