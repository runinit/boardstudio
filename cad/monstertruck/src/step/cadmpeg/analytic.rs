//! Exact facts read from the decoded source geometry rather than its mesh.
mod rational;

use super::*;
use cadmpeg_ir::eval::{admission::EvaluationAdmission, model_curve_point_by_id};
use cadmpeg_ir::geometry::{SolvedCurveGeometry, SolvedSurfaceGeometry};
use cadmpeg_ir::ids::{CoedgeId, CurveId, EdgeId, VertexId};
use cadmpeg_ir::index::{ModelIndex, StandardIndex};
use cadmpeg_ir::topology::Edge;
use cadmpeg_tessellate::FaceMesh;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

pub(super) struct Geometry<'ir> {
    index: ModelIndex<'ir>,
}

fn point3(p: cadmpeg_ir::math::Point3) -> Point3 {
    Point3::new(p.x, p.y, p.z)
}

/// Wraps an angle difference into (-pi, pi].
fn wrap(angle: f64) -> f64 {
    let wrapped = angle.rem_euclid(TAU);
    if wrapped > PI { wrapped - TAU } else { wrapped }
}

impl<'ir> Geometry<'ir> {
    pub(super) fn new(ir: &'ir cadmpeg_ir::CadIr) -> Self {
        Self {
            index: ModelIndex::new_model_only(ir),
        }
    }

    fn surface(&self, face: &FaceId) -> Option<&'ir SolvedSurfaceGeometry> {
        let face = self.index.faces(face.as_str(), StandardIndex)?;
        self.index
            .surfaces(face.surface.as_str(), StandardIndex)?
            .geometry
            .solved()
    }

    fn edge(&self, coedge: &CoedgeId) -> Option<&'ir Edge> {
        let coedge = self.index.coedges(coedge.as_str(), StandardIndex)?;
        self.index.edges(coedge.edge.as_str(), StandardIndex)
    }

    fn curve(&self, curve: &CurveId) -> Option<&'ir SolvedCurveGeometry> {
        self.index
            .curves(curve.as_str(), StandardIndex)?
            .geometry
            .solved()
    }

    fn vertex(&self, vertex: &VertexId) -> Option<Point3> {
        let vertex = self.index.vertices(vertex.as_str(), StandardIndex)?;
        let point = self.index.points(vertex.point.as_str(), StandardIndex)?;
        Some(point3(point.position().get()))
    }

    fn point(&self, curve: &CurveId, t: f64) -> Option<Point3> {
        model_curve_point_by_id(EvaluationAdmission::Standard, &self.index, curve, t)
            .ok()
            .map(|p| point3(p.get()))
    }

    /// A plane bounded only by lines lies on its support exactly when every
    /// boundary vertex does, so its incidence is certified, not sampled.
    pub(super) fn lines_on_plane(&self, face: &FaceId, mesh: &FaceMesh) -> bool {
        matches!(self.surface(face), Some(SolvedSurfaceGeometry::Plane(_)))
            && mesh.coedges.iter().all(|trace| {
                self.edge(&trace.coedge)
                    .and_then(|edge| edge.curve())
                    .and_then(|curve| self.curve(curve))
                    .is_some_and(|curve| matches!(curve, SolvedCurveGeometry::Line(_)))
            })
    }

    fn nearest_curve_distance(
        &self,
        curve: &CurveId,
        target: Point3,
        domain: [f64; 2],
    ) -> Option<f64> {
        const SAMPLES: u8 = 64;
        const STEPS: usize = 60;
        let span = domain[1] - domain[0];
        let step = span / f64::from(SAMPLES);
        let gap = |t: f64| self.point(curve, t).map(|p| (p - target).magnitude());
        let mut best: Option<(f64, f64)> = None;
        for sample in 0..=SAMPLES {
            let t = domain[0] + step * f64::from(sample);
            if let Some(d) = gap(t)
                && best.is_none_or(|(_, nearest)| d < nearest)
            {
                best = Some((t, d));
            }
        }
        let (t, mut best_gap) = best?;
        let (mut low, mut high) = ((t - step).max(domain[0]), (t + step).min(domain[1]));
        for _ in 0..STEPS {
            let first = low + (high - low) / 3.0;
            let second = high - (high - low) / 3.0;
            match (gap(first), gap(second)) {
                (Some(a), Some(b)) => {
                    best_gap = best_gap.min(a).min(b);
                    if a <= b {
                        high = second;
                    } else {
                        low = first;
                    }
                }
                _ => break,
            }
        }
        if let Some(d) = gap(f64::midpoint(low, high)) {
            best_gap = best_gap.min(d);
        }
        Some(best_gap)
    }

    /// The largest distance, in model units, between a B-spline edge curve's
    /// ends and its authored vertices. A line or conic edge is trimmed by its
    /// vertices and has no independent endpoint, so it reports nothing.
    pub(super) fn bspline_endpoint_gap(
        &self,
        coedge: &CoedgeId,
        checked: &mut HashSet<&'ir EdgeId>,
    ) -> Option<(&'ir EdgeId, f64)> {
        let edge = self.edge(coedge)?;
        if !checked.insert(&edge.id) {
            return None;
        }
        let curve = edge.curve()?;
        let SolvedCurveGeometry::Nurbs(nurbs) = self.curve(curve)? else {
            return None;
        };
        let [low, high] = match edge.param_range() {
            Some(range) => range.get(),
            None => cadmpeg_ir::eval::nurbs_curve_parameter_domain(nurbs)?.endpoints(),
        };
        let (Some(start), Some(end)) = (self.vertex(&edge.start), self.vertex(&edge.end)) else {
            return Some((&edge.id, f64::INFINITY));
        };
        let (Some(first), Some(last)) = (self.point(curve, low), self.point(curve, high)) else {
            return Some((&edge.id, f64::INFINITY));
        };
        let forward = (first - start).magnitude().max((last - end).magnitude());
        let reverse = (first - end).magnitude().max((last - start).magnitude());
        let endpoint_gap = forward.min(reverse);
        if edge.param_range().is_some() || edge.start == edge.end || endpoint_gap <= 1e-9 {
            return Some((&edge.id, endpoint_gap));
        }
        let start_gap = self
            .nearest_curve_distance(curve, start, [low, high])
            .unwrap_or(f64::INFINITY);
        let end_gap = self
            .nearest_curve_distance(curve, end, [low, high])
            .unwrap_or(f64::INFINITY);
        Some((&edge.id, endpoint_gap.min(start_gap.max(end_gap))))
    }

    /// World bounds from trim extrema and any interior stationary points.
    pub(super) fn face_bounds(
        &self,
        face: &FaceId,
        mesh: &FaceMesh,
        matrix: Matrix4,
    ) -> Option<[[f64; 3]; 2]> {
        let support = self.surface(face)?;
        let sphere = match support {
            SolvedSurfaceGeometry::Plane(_)
            | SolvedSurfaceGeometry::Cylinder(_)
            | SolvedSurfaceGeometry::Cone(_) => None,
            SolvedSurfaceGeometry::Sphere(sphere) => {
                let face = self.index.faces(face.as_str(), StandardIndex)?;
                if sphere.radius().get() <= 0.0 || face.loops.len() != 1 || mesh.coedges.len() != 3
                {
                    return None;
                }
                let sense = if face.sense == cadmpeg_ir::topology::Sense::Reversed {
                    -1.0
                } else {
                    1.0
                };
                Some((point3(sphere.center().get()), sphere.radius().get(), sense))
            }
            _ => return None,
        };
        let mut inward = Vec::new();
        let mut corners = Vec::new();
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        let mut include = |axis: usize, value: f64| {
            bounds[0][axis] = bounds[0][axis].min(value);
            bounds[1][axis] = bounds[1][axis].max(value);
        };
        for trace in &mesh.coedges {
            let include_point = |p, include: &mut dyn FnMut(usize, f64)| {
                let p = matrix.transform_point(point3(p));
                for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
                    include(axis, value);
                }
            };
            include_point(*trace.points.first()?, &mut include);
            include_point(*trace.points.last()?, &mut include);
            let edge = self.edge(&trace.coedge)?;
            let Some(curve) = edge.curve() else {
                // A degenerate edge is the pole point already traced.
                continue;
            };
            let shape = self.curve(curve)?;
            let center = match shape {
                SolvedCurveGeometry::Line(_) if sphere.is_none() => {
                    for p in &trace.points {
                        include_point(*p, &mut include);
                    }
                    continue;
                }
                SolvedCurveGeometry::Circle(circle) => point3(circle.center().get()),
                SolvedCurveGeometry::Ellipse(ellipse) if sphere.is_none() => {
                    point3(ellipse.center().get())
                }
                SolvedCurveGeometry::Nurbs(nurbs) if sphere.is_none() => {
                    self.rational_bounds(edge, curve, nurbs, matrix, &mut include)?;
                    continue;
                }
                _ => return None,
            };
            // p(t) = c + cos(t) X + sin(t) Y, read from the carrier itself.
            let x = self.point(curve, 0.0)? - center;
            let y = self.point(curve, FRAC_PI_2)? - center;
            let (xx, yy) = (x.magnitude2(), y.magnitude2());
            if xx == 0.0 || yy == 0.0 {
                return None;
            }
            let parameters: Vec<f64> = trace
                .points
                .iter()
                .map(|p| {
                    let d = point3(*p) - center;
                    (d.dot(y) / yy).atan2(d.dot(x) / xx)
                })
                .collect();
            if let Some((origin, radius, sense)) = sphere {
                // Authenticate the whole circle as a great circle using its
                // authored parameters. The samples only select its directed arc.
                let error = (origin - center).magnitude() + (radius - xx.sqrt()).abs();
                let roundoff = 64.0
                    * f64::EPSILON
                    * radius
                        .max(origin.x.abs())
                        .max(origin.y.abs())
                        .max(origin.z.abs());
                let sweep: f64 = parameters.windows(2).map(|p| wrap(p[1] - p[0])).sum();
                if error > roundoff || !(sweep.abs() > 0.0 && sweep.abs() < PI) {
                    return None;
                }
                inward.push(x.cross(y).normalize() * (sense * sweep.signum()));
                corners.push((point3(*trace.points.first()?) - origin) / radius);
            }
            let world_center = matrix.transform_point(center);
            let world_x = matrix.transform_vector(x);
            let world_y = matrix.transform_vector(y);
            for axis in 0..3 {
                let amplitude = world_x[axis].hypot(world_y[axis]);
                if amplitude == 0.0 {
                    continue;
                }
                let phase = world_y[axis].atan2(world_x[axis]);
                for (extreme, value) in [
                    (phase, world_center[axis] + amplitude),
                    (phase + PI, world_center[axis] - amplitude),
                ] {
                    // Boundary samples are at most a quarter turn apart, so each
                    // traced segment covers the short arc between its ends.
                    let traversed = parameters.windows(2).any(|pair| {
                        let span = wrap(pair[1] - pair[0]);
                        let offset = wrap(extreme - pair[0]);
                        if span >= 0.0 {
                            (0.0..=span).contains(&offset)
                        } else {
                            (span..=0.0).contains(&offset)
                        }
                    });
                    if traversed {
                        include(axis, value);
                    }
                }
            }
        }
        if let Some((center, radius, _)) = sphere {
            // Three minor great-circle arcs with the third corner on the
            // interior side bound the convex spherical triangle. Refuse a
            // complementary region or a degenerate triangle.
            if inward.len() != 3 || (0..3).any(|i| inward[i].dot(corners[(i + 2) % 3]) <= 1e-12) {
                return None;
            }
            let world_center = matrix.transform_point(center);
            for axis in 0..3 {
                let row = Vector3::new(matrix.x[axis], matrix.y[axis], matrix.z[axis]);
                let length = row.magnitude();
                for sign in [-1.0, 1.0] {
                    let pole = row * (sign / length);
                    if inward.iter().all(|side| side.dot(pole) >= -1e-12) {
                        include(axis, (sign * radius).mul_add(length, world_center[axis]));
                    }
                }
            }
        }
        bounds
            .iter()
            .flatten()
            .all(|v| v.is_finite())
            .then_some(bounds)
    }
}
