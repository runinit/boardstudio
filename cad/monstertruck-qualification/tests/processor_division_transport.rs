use monstertruck_geometry::prelude::*;
use std::ops::Bound;

// A bounded public producer with deliberately non-evaluated sample positions:
// Processor must carry those positions, rather than replace them by evaluation.
#[derive(Clone)]
struct DivisionWitness<P> {
    domain: (f64, f64),
    parameters: Vec<f64>,
    points: Vec<P>,
    strict: bool,
}
impl<P: EuclideanSpace<Scalar = f64>> ParametricCurve for DivisionWitness<P> {
    type Point = P;
    type Vector = P::Diff;
    fn evaluate(&self, _: f64) -> P {
        self.points[0]
    }
    fn derivative(&self, _: f64) -> P::Diff {
        P::Diff::zero()
    }
    fn derivative_2(&self, _: f64) -> P::Diff {
        P::Diff::zero()
    }
    fn derivative_n(&self, _: usize, _: f64) -> P::Diff {
        P::Diff::zero()
    }
    fn parameter_range(&self) -> ParameterRange {
        (
            Bound::Included(self.domain.0),
            Bound::Included(self.domain.1),
        )
    }
}
impl<P: EuclideanSpace<Scalar = f64>> BoundedCurve for DivisionWitness<P> {}
impl<P: EuclideanSpace<Scalar = f64>> ParameterDivision1D for DivisionWitness<P> {
    type Point = P;
    fn try_parameter_division(&self, range: (f64, f64), _: f64) -> Option<(Vec<f64>, Vec<P>)> {
        (!self.strict || range == self.domain)
            .then(|| (self.parameters.clone(), self.points.clone()))
    }
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<P>) {
        self.try_parameter_division(range, tol)
            .expect("exact child domain required")
    }
}

macro_rules! transport_controls {
    ($name:ident, $point:ty, $matrix:ty, $points:expr) => {
        #[test]
        fn $name() {
            let points = $points;
            // Tiny/off-zero full domains exercise input transport; a partial
            // request exercises output labels independently of domain endpoints.
            for (domain, requested, child_range) in [
                (
                    (1.3322676295501878e-15, 6.283185307179588),
                    (1.3322676295501878e-15, 6.283185307179588),
                    (1.3322676295501878e-15, 6.283185307179588),
                ),
                ((0.2, 4.3), (0.2, 4.3), (0.2, 4.3)),
                ((0.0, 4.0), (0.1, 3.8), (0.20000000000000018, 3.9)),
            ] {
                let child = DivisionWitness {
                    domain,
                    parameters: vec![child_range.0, 1.25, child_range.1],
                    points: points.clone(),
                    strict: requested == domain,
                };
                let mut curve = Processor::<_, $matrix>::new(child.clone());
                let transform = <$matrix>::from_scale(2.0);
                curve.transform_by(transform);
                // Forward processing leaves the producer's parameter labels intact.
                let forward = curve.parameter_division(child_range, 0.01);
                assert_eq!(forward.0, child.parameters);
                assert_eq!(
                    forward.1,
                    points
                        .iter()
                        .map(|p| transform.transform_point(*p))
                        .collect::<Vec<_>>()
                );
                curve.invert();
                for (parameters, actual_points) in [
                    curve.parameter_division(requested, 0.01),
                    curve.try_parameter_division(requested, 0.01).unwrap(),
                ] {
                    assert_eq!(parameters.first(), Some(&requested.0));
                    assert_eq!(parameters.last(), Some(&requested.1));
                    assert_eq!(parameters[1], domain.0 + domain.1 - 1.25);
                    assert_eq!(
                        actual_points,
                        forward.1.iter().rev().copied().collect::<Vec<_>>()
                    );
                }
                if requested == domain {
                    // Two nested reversals preserve exact full-domain identity.
                    let nested = Processor::<_, $matrix>::new(curve).inverse();
                    for (parameters, actual_points) in [
                        nested.parameter_division(domain, 0.01),
                        nested.try_parameter_division(domain, 0.01).unwrap(),
                    ] {
                        assert_eq!(parameters.first(), Some(&domain.0));
                        assert_eq!(parameters.last(), Some(&domain.1));
                        assert_eq!(actual_points, forward.1);
                    }
                }
            }
        }
    };
}
transport_controls!(
    division_transport_2d,
    Point2,
    Matrix3,
    vec![
        Point2::new(11.0, 12.0),
        Point2::new(21.0, 22.0),
        Point2::new(31.0, 32.0)
    ]
);
transport_controls!(
    division_transport_3d,
    Point3,
    Matrix4,
    vec![
        Point3::new(11.0, 12.0, 13.0),
        Point3::new(21.0, 22.0, 23.0),
        Point3::new(31.0, 32.0, 33.0)
    ]
);

#[test]
fn malformed_child_endpoints_are_not_replaced_by_requested_labels() {
    for parameters in [vec![1.125, 2.0, 2.875], vec![], vec![2.0]] {
        let points: Vec<_> = parameters
            .iter()
            .map(|t| Point3::new(*t, 7.0, 8.0))
            .collect();
        let child = DivisionWitness {
            domain: (0.0, 4.0),
            parameters: parameters.clone(),
            points: points.clone(),
            strict: false,
        };
        let curve = Processor::<_, Matrix4>::new(child).inverse();
        for (actual, xyz) in [
            curve.parameter_division((1.0, 3.0), 0.01),
            curve.try_parameter_division((1.0, 3.0), 0.01).unwrap(),
        ] {
            assert_eq!(
                actual,
                parameters.iter().rev().map(|t| 4.0 - t).collect::<Vec<_>>()
            );
            assert_eq!(xyz, points.iter().rev().copied().collect::<Vec<_>>());
        }
    }
}

#[test]
fn only_matching_child_endpoint_is_restored() {
    let a = 0.1;
    let b = 3.8;
    let child = DivisionWitness {
        domain: (0.0, 4.0),
        parameters: vec![0.25, 1.25, 3.9],
        points: vec![Point2::new(11.0, 12.0); 3],
        strict: false,
    };
    let curve = Processor::<_, Matrix3>::new(child).inverse();
    let (parameters, _) = curve.parameter_division((a, b), 0.01);
    assert_eq!(parameters, vec![a, 2.75, 3.75]);
    assert_ne!(parameters[2], b);
}

#[test]
fn strict_child_division_refusal_is_preserved() {
    let domain = (0.2, 4.3);
    let child = DivisionWitness {
        domain,
        parameters: vec![domain.0, domain.1],
        points: vec![Point3::new(1.0, 2.0, 3.0); 2],
        strict: true,
    };
    for curve in [
        Processor::<_, Matrix4>::new(child.clone()),
        Processor::<_, Matrix4>::new(child).inverse(),
    ] {
        assert!(
            curve
                .try_parameter_division((0.1, domain.1), 0.01)
                .is_none()
        );
        assert!(
            curve
                .try_parameter_division((domain.0, 4.4), 0.01)
                .is_none()
        );
    }
}
