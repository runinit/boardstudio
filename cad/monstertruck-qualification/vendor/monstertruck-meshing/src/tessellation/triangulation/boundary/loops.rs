//! `PolyBoundary`: assembling projected pieces into closed, consistently
//! wound parameter-space loops, testing point inclusion, cutting isoparametric
//! curves, and feeding the constrained Delaunay triangulation.
//!
//! Split out of the module file so the source stays readable; the module path
//! is unchanged.

use super::*;
use mesh::spade_round;
use std::{f64::consts::TAU, iter, mem};

const ISOPARAM_BOUNDARY_SEARCH_ITERATIONS: usize = 32;

fn same_roundoff_point(lhs: Point3, rhs: Point3) -> bool {
    let scale = [lhs.x, lhs.y, lhs.z, rhs.x, rhs.y, rhs.z]
        .into_iter()
        .map(f64::abs)
        .fold(1.0, f64::max);
    lhs.distance(rhs) <= 64.0 * f64::EPSILON * scale
}

#[cfg(test)]
mod strict_declared_pole_tests {
    use super::*;
    use monstertruck_geometry::prelude::{
        AnalyticParameterSingularity, ParameterAxis, ParameterDivision2D, ParametricSurface,
        ParametricSurface3D,
    };
    use std::f64::consts::{PI, TAU};

    #[derive(Clone, Copy, Debug)]
    struct TestSphere {
        center: Point3,
        radius: f64,
        x_axis: Vector3,
        y_axis: Vector3,
        z_axis: Vector3,
    }

    impl TestSphere {
        fn point(&self, u: f64, v: f64) -> Point3 {
            Point3::from_vec(
                self.center.to_vec()
                    + self.radius
                        * (self.x_axis * (v.cos() * u.cos())
                            + self.y_axis * (v.cos() * u.sin())
                            + self.z_axis * v.sin()),
            )
        }

        fn local() -> Self {
            Self {
                center: Point3::new(0.0, 0.0, 0.0),
                radius: 1.0,
                x_axis: Vector3::unit_x(),
                y_axis: Vector3::unit_y(),
                z_axis: Vector3::unit_z(),
            }
        }

        fn translated_small() -> Self {
            Self {
                center: Point3::new(1.0e12, -1.0e12, 1.0e12),
                radius: 1.0e-3,
                x_axis: Vector3::unit_x(),
                y_axis: Vector3::unit_y(),
                z_axis: Vector3::unit_z(),
            }
        }
    }

    impl ParametricSurface for TestSphere {
        type Point = Point3;
        type Vector = Vector3;

        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            self.point(u, v)
        }
        fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
            self.radius * v.cos() * (-self.x_axis * u.sin() + self.y_axis * u.cos())
        }
        fn derivative_v(&self, u: f64, v: f64) -> Vector3 {
            self.radius
                * (-self.x_axis * (v.sin() * u.cos()) - self.y_axis * (v.sin() * u.sin())
                    + self.z_axis * v.cos())
        }
        fn derivative_uu(&self, u: f64, v: f64) -> Vector3 {
            -self.radius * v.cos() * (self.x_axis * u.cos() + self.y_axis * u.sin())
        }
        fn derivative_uv(&self, u: f64, v: f64) -> Vector3 {
            self.radius * (self.x_axis * (v.sin() * u.sin()) - self.y_axis * (v.sin() * u.cos()))
        }
        fn derivative_vv(&self, u: f64, v: f64) -> Vector3 {
            -self.radius
                * (self.x_axis * (v.cos() * u.cos())
                    + self.y_axis * (v.cos() * u.sin())
                    + self.z_axis * v.sin())
        }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            match (m, n) {
                (1, 0) => self.derivative_u(u, v),
                (0, 1) => self.derivative_v(u, v),
                (2, 0) => self.derivative_uu(u, v),
                (1, 1) => self.derivative_uv(u, v),
                (0, 2) => self.derivative_vv(u, v),
                _ => Vector3::zero(),
            }
        }
        fn analytic_singularities(&self) -> Vec<AnalyticParameterSingularity> {
            vec![AnalyticParameterSingularity {
                axis: ParameterAxis::V,
                parameter: -PI / 2.0,
            }]
        }
    }

    impl ParametricSurface3D for TestSphere {
        fn normal(&self, u: f64, v: f64) -> Vector3 {
            (self.point(u, v) - self.center).normalize()
        }
    }

    impl ParameterDivision2D for TestSphere {
        fn parameter_division(&self, _: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
            (vec![0.0, PI, TAU], vec![-PI / 2.0, 0.0, PI / 2.0])
        }
    }

    fn boundary_loop(
        surface: &TestSphere,
        reverse: bool,
        near_pole_offset: f64,
    ) -> Vec<SurfacePoint> {
        let near_v = -PI / 2.0 + near_pole_offset;
        let mut points = vec![
            (Point2::new(0.0, -PI / 2.0), surface.point(0.0, -PI / 2.0)).into(),
            (Point2::new(0.4, near_v), surface.point(0.4, near_v)).into(),
            (Point2::new(2.0, -PI / 2.0), surface.point(2.0, -PI / 2.0)).into(),
            (Point2::new(2.8, -1.1), surface.point(2.8, -1.1)).into(),
            (Point2::new(4.4, -1.2), surface.point(4.4, -1.2)).into(),
            (Point2::new(TAU, -PI / 2.0), surface.point(TAU, -PI / 2.0)).into(),
        ];
        if reverse {
            points.reverse();
        }
        let boundary = PolyBoundary::new(vec![PolyBoundaryPiece(points)], surface, 0.01);
        assert!(boundary.is_valid(), "test boundary must remain valid");
        boundary.loops.into_iter().next().expect("one loop")
    }

    #[test]
    fn exact_declared_pole_identity_collapses_representatives_but_preserves_nearby_rows() {
        for (surface, offset) in [
            (TestSphere::local(), 1.0e-14),
            (TestSphere::translated_small(), 1.0e-14),
        ] {
            for reverse in [false, true] {
                let points = boundary_loop(&surface, reverse, offset);
                let poles: Vec<_> = points.iter().filter(|point| point.y == -PI / 2.0).collect();
                assert!(poles.len() >= 2);
                assert!(poles.iter().all(|point| point.x == poles[0].x));

                let near_pole = points
                    .iter()
                    .find(|point| point.x == 0.4)
                    .expect("near-pole sample must be retained");
                assert_eq!(near_pole.y, -PI / 2.0 + offset);
                if surface.radius == 1.0e-3 {
                    assert_eq!(near_pole.point, poles[0].point);
                }
            }
        }
    }

    #[test]
    fn source_pole_lift_preserves_distinct_near_pole_source_vertices_at_huge_origin() {
        for (center, radius, offset) in [
            (Point3::origin(), 1.0, 5e-7),
            (Point3::new(1e12, 1e12, 1e12), 1000.0, 1e-7),
        ] {
            let surface = TestSphere {
                center,
                radius,
                ..TestSphere::local()
            };
            let uv = [
                Point2::new(0.0, -PI / 2.0 + offset),
                Point2::new(PI / 2.0, -PI / 2.0 + offset),
                Point2::new(PI / 4.0, -PI / 2.0 + 2.0 * offset),
            ];
            let positions = uv.map(|p| surface.point(p.x, p.y));
            assert!(
                positions[0] != positions[1]
                    && positions[1] != positions[2]
                    && positions[2] != positions[0]
            );
            assert!(
                (positions[1] - positions[0])
                    .cross(positions[2] - positions[0])
                    .magnitude2()
                    > 0.0
            );
            let mut points = uv
                .into_iter()
                .zip(positions)
                .map(SurfacePoint::from)
                .collect::<Vec<_>>();
            points.push(points[0]);
            let boundary = PolyBoundary::new_with_source_vertices(
                vec![PolyBoundaryPiece(points)],
                &surface,
                0.0005,
                positions.into_iter().enumerate().map(|(id, p)| (p, id, p)),
            );
            assert!(boundary.is_valid());
            for (id, (uv, position)) in uv.into_iter().zip(positions).enumerate() {
                assert!(
                    boundary
                        .loops
                        .iter()
                        .flatten()
                        .any(|p| p.uv == uv && p.point == position && p.source_vertex == Some(id)),
                    "distinct source vertex must retain its UV and XYZ"
                );
            }
        }
    }

    #[derive(Clone)]
    struct ActiveSphere {
        sphere: TestSphere,
        outside: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl ParametricSurface for ActiveSphere {
        type Point = Point3;
        type Vector = Vector3;
        fn evaluate(&self, u: f64, v: f64) -> Point3 {
            if !(0.0..=PI / 2.0).contains(&v) {
                self.outside
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            self.sphere.evaluate(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_u(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_v(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_v(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_uu(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_uu(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_uv(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_uv(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_vv(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_vv(u, v.clamp(0.0, PI / 2.0))
        }
        fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
            self.sphere.derivative_mn(m, n, u, v.clamp(0.0, PI / 2.0))
        }
        fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
            use std::ops::Bound::Included;
            (
                (Included(0.0), Included(TAU)),
                (Included(0.0), Included(PI / 2.0)),
            )
        }
        fn analytic_singularities(&self) -> Vec<AnalyticParameterSingularity> {
            [-PI / 2.0, PI / 2.0]
                .into_iter()
                .map(|parameter| AnalyticParameterSingularity {
                    axis: ParameterAxis::V,
                    parameter,
                })
                .collect()
        }
    }
    impl ParametricSurface3D for ActiveSphere {
        fn normal(&self, u: f64, v: f64) -> Vector3 {
            self.sphere.normal(u, v.clamp(0.0, PI / 2.0))
        }
    }
    impl ParameterDivision2D for ActiveSphere {
        fn parameter_division(
            &self,
            range: ((f64, f64), (f64, f64)),
            tolerance: f64,
        ) -> (Vec<f64>, Vec<f64>) {
            self.sphere.parameter_division(range, tolerance)
        }
    }

    #[test]
    fn source_pole_lift_never_probes_a_declared_pole_outside_the_active_range() {
        let surface = ActiveSphere {
            sphere: TestSphere::local(),
            outside: std::sync::Arc::default(),
        };
        let uv = [
            Point2::new(0.2, 0.0),
            Point2::new(0.4, 0.0),
            Point2::new(0.3, 0.2),
        ];
        let positions = uv.map(|p| surface.evaluate(p.x, p.y));
        let mut points = uv
            .into_iter()
            .zip(positions)
            .map(SurfacePoint::from)
            .collect::<Vec<_>>();
        points.push(points[0]);
        let boundary = PolyBoundary::new_with_source_vertices(
            vec![PolyBoundaryPiece(points)],
            &surface,
            0.0005,
            positions.into_iter().enumerate().map(|(id, p)| (p, id, p)),
        );
        assert_eq!(
            surface.outside.load(std::sync::atomic::Ordering::Relaxed),
            0,
            "global declared poles do not authorize out-of-domain evaluation"
        );
        for (uv, position) in uv.into_iter().zip(positions) {
            assert!(
                boundary
                    .loops
                    .iter()
                    .flatten()
                    .any(|p| p.uv == uv && p.point == position)
            );
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::tessellation::triangulation) struct PolyBoundary {
    pub(in crate::tessellation::triangulation) loops: Vec<Vec<SurfacePoint>>,
    /// UV-space axis-aligned bounding box for cheap rejection in `include()`.
    pub(in crate::tessellation::triangulation) uv_min: Point2,
    pub(in crate::tessellation::triangulation) uv_max: Point2,
    pub(in crate::tessellation::triangulation) valid: bool,
}

impl Default for PolyBoundary {
    fn default() -> Self {
        Self {
            loops: Vec::new(),
            uv_min: Point2::new(f64::INFINITY, f64::INFINITY),
            uv_max: Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            valid: true,
        }
    }
}

fn loop_signed_area(curve: &[SurfacePoint]) -> f64 {
    curve
        .iter()
        .circular_tuple_windows()
        .fold(0.0, |sum, (p, q)| sum + (q.x + p.x) * (q.y - p.y))
}

fn loop_orientation(curve: &[SurfacePoint]) -> bool {
    loop_signed_area(curve) > 0.0
}

fn normalize_closed_loop_winding(closed: &mut [Vec<SurfacePoint>]) {
    let outer_index = closed
        .iter()
        .enumerate()
        .max_by(|(_, lhs), (_, rhs)| {
            loop_signed_area(lhs)
                .abs()
                .total_cmp(&loop_signed_area(rhs).abs())
        })
        .map(|(index, _)| index);
    if let Some(outer_index) = outer_index {
        closed.iter_mut().enumerate().for_each(|(index, curve)| {
            let should_be_positive = index == outer_index;
            if loop_orientation(curve) != should_be_positive {
                curve.reverse();
            }
        });
    }
}

fn open_pair_connector_score(
    p0: SurfacePoint,
    p1: SurfacePoint,
    q0: SurfacePoint,
    q1: SurfacePoint,
) -> f64 {
    let p1q0 = p1.uv - q0.uv;
    let q1p0 = q1.uv - p0.uv;
    p1q0.x * p1q0.x + p1q0.y * p1q0.y + q1p0.x * q1p0.x + q1p0.y * q1p0.y
}

pub(in crate::tessellation::triangulation) fn boundary_segment_parameter(
    point: Point2,
    front: Point2,
    back: Point2,
) -> Option<f64> {
    let segment = back - front;
    let denom = segment.dot(segment);
    if denom.so_small() {
        None
    } else {
        let offset = point - front;
        let parameter = offset.dot(segment) / denom;
        let projected = front + segment * parameter;
        (parameter > 0.0 && parameter < 1.0 && projected.distance(point) <= 1.0e-9)
            .then_some(parameter)
    }
}

fn push_isoparam_uv(curve: &mut Vec<Point2>, uv: Point2) {
    if curve.last().is_none_or(|previous| !previous.near(&uv)) {
        curve.push(uv);
    }
}

fn push_finished_isoparam_curve(curves: &mut Vec<Vec<Point2>>, curve: &mut Vec<Point2>) {
    if curve.len() >= 2 {
        curves.push(mem::take(curve));
    } else {
        curve.clear();
    }
}

fn add_direct_cdt_constraint(
    triangulation: &mut Cdt,
    added_constraints: &mut usize,
    skipped_constraints: &mut usize,
    front: FixedVertexHandle,
    back: FixedVertexHandle,
) -> bool {
    if front == back || !triangulation.can_add_constraint(front, back) {
        *skipped_constraints += 1;
        false
    } else {
        let constraints = triangulation.add_constraint_and_split(front, back, |point| point);
        if constraints.is_empty() {
            *skipped_constraints += 1;
            false
        } else {
            *added_constraints += constraints.len();
            true
        }
    }
}

impl PolyBoundary {
    /// Maps very small parameter-space spans to an order-one range for CDT
    /// predicates. Original parameters remain on boundary points; this is only
    /// an injective working-coordinate transform.
    fn cdt_uv_axis(value: f64, min: f64, max: f64) -> f64 {
        let span = max - min;
        if span > 0.0 && span < 1.0e-6 {
            (value - min) / span
        } else {
            value
        }
    }

    pub(in crate::tessellation::triangulation) fn uv_to_cdt(&self, uv: Point2) -> Point2 {
        Point2::new(
            Self::cdt_uv_axis(uv.x, self.uv_min.x, self.uv_max.x),
            Self::cdt_uv_axis(uv.y, self.uv_min.y, self.uv_max.y),
        )
    }

    pub(in crate::tessellation::triangulation) fn cdt_to_uv(&self, uv: Point2) -> Point2 {
        let unmap = |value: f64, min: f64, max: f64| {
            let span = max - min;
            if span > 0.0 && span < 1.0e-6 {
                min + value * span
            } else {
                value
            }
        };
        Point2::new(
            unmap(uv.x, self.uv_min.x, self.uv_max.x),
            unmap(uv.y, self.uv_min.y, self.uv_max.y),
        )
    }

    /// Lifts a proven source vertex at an active analytic pole to its incoming
    /// and outgoing chart limits without changing any original XYZ sample.
    pub(in crate::tessellation::triangulation) fn new_with_source_vertices(
        pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tolerance: f64,
        endpoints: impl IntoIterator<Item = (Point3, usize, Point3)>,
    ) -> Self {
        Self::new_with_source_vertices_and_repair(pieces, surface, tolerance, endpoints, None)
    }

    pub(in crate::tessellation::triangulation) fn new_with_source_vertices_and_repair(
        mut pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tolerance: f64,
        endpoints: impl IntoIterator<Item = (Point3, usize, Point3)>,
        repair: Option<repair::FaceCapture<'_>>,
    ) -> Self {
        let mut pending = Vec::new();
        if let Some(trace) = repair {
            if let Some(slots) = trace.pending(pieces.len()) {
                pending = slots;
                for piece in &pieces {
                    pending.push(trace.before(
                        repair::Stage::Loop,
                        piece.0.iter().map(|p| p.uv),
                        piece.0.iter().map(|p| p.point),
                    ));
                }
            } else {
                trace.unsupported(
                    repair::Stage::Loop,
                    "original finite loop capture unavailable",
                );
            }
        }
        let key = |point: Point3| {
            [point.x, point.y, point.z].map(|value| if value == 0.0 { 0 } else { value.to_bits() })
        };
        let mut sources = HashMap::<_, Option<(usize, Point3)>>::default();
        let endpoints = endpoints.into_iter().collect::<Vec<_>>();
        for &(point, source, canonical) in &endpoints {
            sources
                .entry(key(point))
                .and_modify(|prior| {
                    if *prior != Some((source, canonical)) {
                        *prior = None;
                    }
                })
                .or_insert(Some((source, canonical)));
        }
        let ranges = surface.try_range_tuple();
        let singularities = surface
            .analytic_singularities()
            .into_iter()
            .filter(|singularity| {
                let range = match singularity.axis {
                    ParameterAxis::U => ranges.0,
                    ParameterAxis::V => ranges.1,
                };
                range.is_none_or(|(min, max)| {
                    singularity.parameter >= min && singularity.parameter <= max
                })
            })
            .collect::<Vec<_>>();
        for PolyBoundaryPiece(points) in &mut pieces {
            let poles = points
                .iter_mut()
                .map(|point| {
                    let (source, canonical) = sources.get(&key(point.point)).copied().flatten()?;
                    point.source_vertex = Some(source);
                    singularities.iter().copied().find_map(|singularity| {
                        let uv = match singularity.axis {
                            ParameterAxis::U => Point2::new(singularity.parameter, point.y),
                            ParameterAxis::V => Point2::new(point.x, singularity.parameter),
                        };
                        (surface.evaluate(uv.x, uv.y) == canonical).then_some((
                            source,
                            singularity,
                            canonical,
                        ))
                    })
                })
                .collect::<Vec<_>>();
            if poles.iter().all(Option::is_none) || points.len() < 3 {
                continue;
            }
            let first = points[0];
            let last = *points.last().unwrap();
            if first.point != last.point
                && !(first.source_vertex.is_some() && first.source_vertex == last.source_vertex)
            {
                continue;
            }
            let Some(start) = poles.iter().position(Option::is_none) else {
                continue;
            };
            let mut repaired = Vec::new();
            let count = points.len();
            let mut offset = 0;
            while offset < count {
                let index = (start + offset) % count;
                let Some((source, singularity, canonical)) = poles[index] else {
                    repaired.push(points[index]);
                    offset += 1;
                    continue;
                };
                let run_start = offset;
                while offset < count
                    && poles[(start + offset) % count].is_some_and(|(id, axis, _)| {
                        id == source
                            && axis.axis == singularity.axis
                            && axis.parameter == singularity.parameter
                    })
                {
                    offset += 1;
                }
                let before = points[(start + run_start + count - 1) % count].uv;
                let after = points[(start + offset) % count].uv;
                let lift = |uv: Point2| match singularity.axis {
                    ParameterAxis::U => Point2::new(singularity.parameter, uv.y),
                    ParameterAxis::V => Point2::new(uv.x, singularity.parameter),
                };
                let incoming = lift(before);
                let outgoing = lift(after);
                if surface.evaluate(incoming.x, incoming.y) != canonical
                    || surface.evaluate(outgoing.x, outgoing.y) != canonical
                {
                    repaired
                        .extend((run_start..offset).map(|offset| points[(start + offset) % count]));
                    continue;
                }
                let mut point = points[index];
                point.uv = incoming;
                repaired.push(point);
                if offset - run_start == 1 && incoming != outgoing {
                    point.uv = outgoing;
                    repaired.push(point);
                } else {
                    repaired.extend((run_start + 1..offset).map(|offset| {
                        let mut point = points[(start + offset) % count];
                        point.uv = outgoing;
                        point
                    }));
                }
            }
            repaired.push(repaired[0]);
            *points = repaired;
        }
        let mut boundary = Self::new(pieces, surface, tolerance);
        boundary.bind_source_vertices(
            endpoints
                .into_iter()
                .flat_map(|(sample, source, canonical)| [(sample, source), (canonical, source)]),
        );
        if let Some(trace) = repair {
            if pending.len() == 1 && boundary.loops.len() == 1 {
                if let Some(before) = pending.pop().flatten() {
                    let after = &boundary.loops[0];
                    // PolyBoundary loops include their implicit last->first
                    // segment. Expose that actual finite closure explicitly.
                    before.finish(
                        after.iter().chain(after.first()).map(|p| p.uv),
                        after.iter().chain(after.first()).map(|p| p.point),
                    );
                }
            } else {
                trace.unsupported(
                    repair::Stage::Loop,
                    "multiple finite pieces/loops have no authenticated correspondence",
                );
                for before in pending.into_iter().flatten() {
                    before.finish(std::iter::empty(), std::iter::empty());
                }
                for after in &boundary.loops {
                    if let Some(before) =
                        trace.before(repair::Stage::Loop, std::iter::empty(), std::iter::empty())
                    {
                        before.finish(
                            after.iter().chain(after.first()).map(|p| p.uv),
                            after.iter().chain(after.first()).map(|p| p.point),
                        );
                    }
                }
            }
        }
        boundary
    }

    pub(in crate::tessellation::triangulation) fn new(
        pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tolerance: f64,
    ) -> Self {
        if !pieces.is_empty()
            && surface.period_u().is_none()
            && surface.period_v().is_none()
            && pieces
                .iter()
                .any(|PolyBoundaryPiece(points)| points.len() < 3)
        {
            return Self {
                valid: false,
                ..Self::default()
            };
        }
        let (mut closed, mut open) = (Vec::new(), Vec::new());
        pieces.into_iter().for_each(|PolyBoundaryPiece(mut vec)| {
            let singularities = surface.analytic_singularities();
            let mut pole_representatives =
                Vec::<(AnalyticParameterSingularity, SurfacePoint)>::new();
            for point in &mut vec {
                if point.source_vertex.is_some() {
                    continue;
                }
                for singularity in &singularities {
                    let on_singularity = match singularity.axis {
                        ParameterAxis::U => point.x == singularity.parameter,
                        ParameterAxis::V => point.y == singularity.parameter,
                    };
                    if !on_singularity {
                        continue;
                    }
                    let representative = pole_representatives.iter().find(|(axis, prior)| {
                        axis.axis == singularity.axis
                            && axis.parameter == singularity.parameter
                            && same_roundoff_point(prior.point, point.point)
                    });
                    if let Some((_, prior)) = representative {
                        match singularity.axis {
                            ParameterAxis::U => point.y = prior.y,
                            ParameterAxis::V => point.x = prior.x,
                        }
                    } else {
                        pole_representatives.push((*singularity, *point));
                    }
                }
            }
            compact_periodic_closed_loop(&mut vec, surface);
            if let Some((axis, period)) = periodic_axis_full_span(&vec, surface) {
                open.push(open_periodic_closed_loop(vec, surface, axis, period));
            } else {
                match vec[0].uv.distance(vec[vec.len() - 1].uv) < 1.0e-3 {
                    true => {
                        vec.pop();
                        closed.push(vec)
                    }
                    false => open.push(vec),
                }
            }
        });
        open.retain(|curve| curve.len() >= 2);
        closed.retain(|curve| curve.len() >= 2);
        let mut point_cache = HashMap::<UvKey, Point3>::default();
        match open.len() {
            1 => {
                // SAFETY: open.len() == 1 was matched above.
                let mut curve = open.pop().unwrap();
                if let Some((axis, _)) = periodic_open_axis_full_span(&curve, surface)
                    && let Some(singular_curve) = close_open_periodic_curve_to_singular(
                        curve.clone(),
                        surface,
                        axis,
                        tolerance,
                        &mut point_cache,
                    )
                {
                    closed.push(singular_curve);
                } else if let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple() {
                    let p = curve[0];
                    let q = curve[curve.len() - 1];
                    if p.x < q.x - TOLERANCE {
                        normalize_range(&mut curve, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v1),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v1),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if q.x < p.x - TOLERANCE {
                        normalize_range(&mut curve, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v0),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v0),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if p.y < q.y - TOLERANCE {
                        normalize_range(&mut curve, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v0),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v1),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if q.y < p.y - TOLERANCE {
                        normalize_range(&mut curve, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v1),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v0),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    }
                }
            }
            2 => {
                // SAFETY: open.len() == 2 was matched above.
                let mut curve1 = open.pop().unwrap();
                let mut curve0 = open.pop().unwrap();
                fn end_pts<T: Copy>(vec: &[T]) -> (T, T) {
                    (vec[0], vec[vec.len() - 1])
                }
                let full_period_pair_aligned =
                    align_full_period_open_pair(&mut curve0, &mut curve1, surface);
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                if !full_period_pair_aligned && !p0.x.near(&p1.x) && !q0.x.near(&q1.x) {
                    if let Some(period) = surface.period_u() {
                        align_periodic_open_pair(&curve0, &mut curve1, 0, period);
                    } else if let (Some(urange), _) = surface.try_range_tuple() {
                        normalize_range(&mut curve0, 0, urange);
                        normalize_range(&mut curve1, 0, urange);
                    }
                } else if !full_period_pair_aligned && !p0.y.near(&p1.y) && !q0.y.near(&q1.y) {
                    if let Some(period) = surface.period_v() {
                        align_periodic_open_pair(&curve0, &mut curve1, 1, period);
                    } else if let (_, Some(vrange)) = surface.try_range_tuple() {
                        normalize_range(&mut curve0, 1, vrange);
                        normalize_range(&mut curve1, 1, vrange);
                    }
                }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                let current_score = open_pair_connector_score(p0, p1, q0, q1);
                let reversed_score = open_pair_connector_score(p0, p1, q1, q0);
                if reversed_score + TOLERANCE < current_score {
                    curve1.reverse();
                }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                let vec0 = polyline_on_surface(surface, p1, q0, tolerance, &mut point_cache);
                let vec1 = polyline_on_surface(surface, q1, p0, tolerance, &mut point_cache);
                closed.push(connect_edges([curve0, vec0, curve1, vec1]));
            }
            _ => {}
        }
        if closed.len() == 1 && !loop_orientation(&closed[0]) {
            closed[0].reverse();
        }
        if !closed.iter().any(|curve| loop_orientation(curve))
            && let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple()
        {
            let p = [
                surface_point_with_cache(surface, Point2::new(u0, v0), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u1, v0), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u1, v1), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u0, v1), &mut point_cache),
            ];
            let vec0 = polyline_on_surface(surface, p[0], p[1], tolerance, &mut point_cache);
            let vec1 = polyline_on_surface(surface, p[1], p[2], tolerance, &mut point_cache);
            let vec2 = polyline_on_surface(surface, p[2], p[3], tolerance, &mut point_cache);
            let vec3 = polyline_on_surface(surface, p[3], p[0], tolerance, &mut point_cache);
            closed.push(connect_edges([vec0, vec1, vec2, vec3]));
        }
        normalize_closed_loop_winding(&mut closed);
        let (mut uv_min, mut uv_max) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for pt in closed.iter().flatten() {
            uv_min.x = f64::min(uv_min.x, pt.x);
            uv_min.y = f64::min(uv_min.y, pt.y);
            uv_max.x = f64::max(uv_max.x, pt.x);
            uv_max.y = f64::max(uv_max.y, pt.y);
        }
        Self {
            loops: closed,
            uv_min,
            uv_max,
            valid: true,
        }
    }

    pub(in crate::tessellation::triangulation) fn is_valid(&self) -> bool {
        self.valid
    }

    /// Whether `c` is included in the domain with boundary = `self`.
    pub(in crate::tessellation::triangulation) fn include(&self, c: Point2) -> bool {
        self.include_impl(c, false)
    }

    // Only resolved constrained triangles use this exact boundary classification;
    // generic grid samples retain the established near-boundary exclusion band.
    pub(in crate::tessellation::triangulation) fn include_cdt_centroid(&self, c: Point2) -> bool {
        self.include_impl(c, true)
    }

    fn include_impl(&self, c: Point2, strict: bool) -> bool {
        // AABB early reject.
        if c.x < self.uv_min.x || c.x > self.uv_max.x || c.y < self.uv_min.y || c.y > self.uv_max.y
        {
            return false;
        }
        let c = self.uv_to_cdt(c);
        let t = TAU * HashGen::hash1(c);
        let r = Vector2::new(f64::cos(t), f64::sin(t));
        self.loops
            .iter()
            .flat_map(|vec| vec.iter().circular_tuple_windows())
            .try_fold(0_i32, move |counter, (p0, p1)| {
                let a = self.uv_to_cdt(**p0) - c;
                let b = self.uv_to_cdt(**p1) - c;
                let s0 = r.x * a.y - r.y * a.x; // v times a.
                let s1 = r.x * b.y - r.y * b.x; // v times b.
                let s2 = a.x * b.y - a.y * b.x; // a times b.
                let x = s2 / (s1 - s0);
                if (x == 0.0 || (!strict && x.so_small())) && s0 * s1 < 0.0 {
                    None
                } else if x > 0.0 && s0 <= 0.0 && s1 > 0.0 {
                    Some(counter + 1)
                } else if x > 0.0 && s0 >= 0.0 && s1 < 0.0 {
                    Some(counter - 1)
                } else {
                    Some(counter)
                }
            })
            .map(|counter| counter > 0)
            .unwrap_or(false)
    }

    pub(in crate::tessellation::triangulation) fn isoparametric_curves<S: PreMeshableSurface>(
        &self,
        surface: &S,
        options: IsoparametricCurveOptions,
    ) -> Vec<Vec<Point3>> {
        if self.loops.is_empty()
            || options.samples_per_direction == 0
            || options.segments_per_curve == 0
            || self.uv_min.x >= self.uv_max.x
            || self.uv_min.y >= self.uv_max.y
        {
            Vec::new()
        } else {
            (0..=1)
                .flat_map(|axis| {
                    self.isoparametric_axis_values(axis, options.samples_per_direction)
                        .into_iter()
                        .flat_map(move |axis_value| {
                            self.isoparametric_uv_curves(
                                axis,
                                axis_value,
                                options.segments_per_curve,
                            )
                            .into_iter()
                            .map(|curve| {
                                curve
                                    .into_iter()
                                    .map(|uv| surface.evaluate(uv.x, uv.y))
                                    .collect::<Vec<_>>()
                            })
                        })
                })
                .collect()
        }
    }

    fn isoparametric_axis_values(&self, axis: usize, count: usize) -> Vec<f64> {
        let (min, max) = match axis {
            0 => (self.uv_min.x, self.uv_max.x),
            _ => (self.uv_min.y, self.uv_max.y),
        };
        (0..count)
            .map(|index| {
                let parameter = (index + 1) as f64 / (count + 1) as f64;
                min + (max - min) * parameter
            })
            .collect()
    }

    fn isoparametric_uv_curves(
        &self,
        constant_axis: usize,
        constant_value: f64,
        segments: usize,
    ) -> Vec<Vec<Point2>> {
        let variable_axis = 1 - constant_axis;
        let (min, max) = match variable_axis {
            0 => (self.uv_min.x, self.uv_max.x),
            _ => (self.uv_min.y, self.uv_max.y),
        };
        let samples = (0..=segments)
            .map(|index| {
                let parameter = index as f64 / segments as f64;
                let variable_value = min + (max - min) * parameter;
                match constant_axis {
                    0 => Point2::new(constant_value, variable_value),
                    _ => Point2::new(variable_value, constant_value),
                }
            })
            .collect::<Vec<_>>();
        self.trim_uv_samples(samples)
    }

    fn trim_uv_samples(&self, samples: Vec<Point2>) -> Vec<Vec<Point2>> {
        let (_, mut curves, mut current) = samples.into_iter().fold(
            (
                None::<(Point2, bool)>,
                Vec::<Vec<Point2>>::new(),
                Vec::<Point2>::new(),
            ),
            |(previous, mut curves, mut current), uv| {
                let included = self.include(uv);
                if let Some((previous_uv, previous_included)) = previous
                    && previous_included != included
                {
                    let crossing =
                        self.isoparametric_boundary_crossing(previous_uv, uv, previous_included);
                    push_isoparam_uv(&mut current, crossing);
                    if previous_included {
                        push_finished_isoparam_curve(&mut curves, &mut current);
                    }
                    if included {
                        push_isoparam_uv(&mut current, crossing);
                    }
                }
                if included {
                    push_isoparam_uv(&mut current, uv);
                }
                (Some((uv, included)), curves, current)
            },
        );
        push_finished_isoparam_curve(&mut curves, &mut current);
        curves
    }

    fn isoparametric_boundary_crossing(
        &self,
        front: Point2,
        back: Point2,
        front_included: bool,
    ) -> Point2 {
        let (mut included, mut excluded) = match front_included {
            true => (front, back),
            false => (back, front),
        };
        (0..ISOPARAM_BOUNDARY_SEARCH_ITERATIONS).for_each(|_| {
            let middle = Point2::new(
                (included.x + excluded.x) * 0.5,
                (included.y + excluded.y) * 0.5,
            );
            if self.include(middle) {
                included = middle;
            } else {
                excluded = middle;
            }
        });
        Point2::new(
            (included.x + excluded.x) * 0.5,
            (included.y + excluded.y) * 0.5,
        )
    }

    /// Binds exact sampled endpoints; conflicting source identities remain unknown.
    pub(in crate::tessellation::triangulation) fn bind_source_vertices(
        &mut self,
        endpoints: impl IntoIterator<Item = (Point3, usize)>,
    ) {
        let key = |point: Point3| {
            [point.x, point.y, point.z].map(|value| if value == 0.0 { 0 } else { value.to_bits() })
        };
        let mut sources = HashMap::<_, Option<usize>>::default();
        for (point, source) in endpoints {
            sources
                .entry(key(point))
                .and_modify(|prior| {
                    if *prior != Some(source) {
                        *prior = None;
                    }
                })
                .or_insert(Some(source));
        }
        for point in self.loops.iter_mut().flatten() {
            point.source_vertex = sources.get(&key(point.point)).copied().flatten();
        }
    }

    /// Inserts points and adds constraint into triangulation.
    pub(in crate::tessellation::triangulation) fn insert_to(
        &self,
        triangulation: &mut Cdt,
        boundary_map: &mut HashMap<FixedVertexHandle, SurfacePoint>,
    ) -> (usize, usize, usize) {
        let poly2tri: Vec<_> = self
            .loops
            .iter()
            .flatten()
            .map(|pt| {
                let uv = self.uv_to_cdt(pt.uv);
                let p = [spade_round(uv.x), spade_round(uv.y)];
                match triangulation.insert(SPoint2::from(p)) {
                    Err(_) => None,
                    Ok(idx) => {
                        boundary_map
                            .entry(idx)
                            .and_modify(|prior| {
                                let same_source = prior.source_vertex == pt.source_vertex;
                                *prior = *pt;
                                if !same_source {
                                    prior.source_vertex = None;
                                }
                            })
                            .or_insert(*pt);
                        Some(idx)
                    }
                }
            })
            .collect();
        let split_vertices = triangulation
            .vertices()
            .map(|vertex| {
                let point = vertex.position();
                (vertex.fix(), Point2::new(point.x, point.y))
            })
            .collect::<Vec<_>>();
        let mut counter = 0;
        let mut added_constraints = 0usize;
        let mut skipped_constraints = 0usize;
        let mut add_constraint = |front: FixedVertexHandle, back: FixedVertexHandle| {
            if front == back {
                false
            } else {
                let front_point = triangulation.vertex(front).position();
                let back_point = triangulation.vertex(back).position();
                let front_point = Point2::new(front_point.x, front_point.y);
                let back_point = Point2::new(back_point.x, back_point.y);
                let mut chain = split_vertices
                    .iter()
                    .filter_map(|(handle, point)| {
                        if *handle == front || *handle == back {
                            None
                        } else {
                            boundary_segment_parameter(*point, front_point, back_point)
                                .map(|parameter| (parameter, *handle))
                        }
                    })
                    .collect::<Vec<_>>();
                chain.sort_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));
                let handles = iter::once(front)
                    .chain(chain.into_iter().map(|(_, handle)| handle))
                    .chain(iter::once(back))
                    .collect::<Vec<_>>();
                let mut handled = false;
                handles.windows(2).for_each(|window| {
                    if window[0] != window[1] {
                        handled = true;
                        add_direct_cdt_constraint(
                            triangulation,
                            &mut added_constraints,
                            &mut skipped_constraints,
                            window[0],
                            window[1],
                        );
                    }
                });
                handled
            }
        };
        self.loops.iter().map(Vec::len).for_each(|len| {
            let range = counter..counter + len;
            counter += len;
            let mut prev: Option<usize> = None;
            range.circular_tuple_windows().for_each(|(i, j)| {
                let Some(vj) = poly2tri[j] else { return };
                if let Some(p) = prev {
                    let Some(v) = poly2tri[p] else { return };
                    if add_constraint(v, vj) {
                        prev = None;
                    }
                } else {
                    let Some(vi) = poly2tri[i] else { return };
                    if !add_constraint(vi, vj) {
                        prev = Some(i);
                    }
                }
            });
        });
        (boundary_map.len(), added_constraints, skipped_constraints)
    }
}
