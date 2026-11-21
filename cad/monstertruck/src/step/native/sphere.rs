//! A convex spherical triangle with three short great-circle arcs.
use super::{
    geometry::Curve,
    sphere_bounds::{ArcExtent, enclosing_planes},
    trim::{Samples, Triangles},
    *,
};
use std::f64::consts::PI;

pub(super) struct SphericalTrim {
    center: Point3,
    radius: f64,
    interior: Point3,
    bounds_planes: [Vector3<f64>; 3],
    projection_margin: f64,
    boundary: [(u64, bool); 3],
    same_sense: bool,
}

impl Face {
    pub(super) fn spherical_trim(
        &self,
        radius: f64,
        edges: &HashMap<u64, Edge>,
        epsilon: f64,
    ) -> Result<SphericalTrim, String> {
        let shape = "native sphere requires one triangular great-circle boundary";
        let [ring] = self.loops.as_slice() else {
            return Err(shape.into());
        };
        let [a, b, c] = ring.edges.as_slice() else {
            return Err(shape.into());
        };
        let boundary = [*a, *b, *c];
        let center = self
            .surface
            .frame
            .transform_point(Point3::new(0.0, 0.0, 0.0));
        let sense = if self.same_sense { 1.0 } else { -1.0 };
        let mut inward = [Vector3::new(0.0, 0.0, 0.0); 3];
        let mut corners = inward;
        let mut arcs = Vec::with_capacity(3);
        let mut radial_error = 0.0_f64;
        let mut join_error = 0.0_f64;
        for (i, (id, forward)) in boundary.into_iter().enumerate() {
            let edge = &edges[&id];
            let Curve::Circle {
                frame,
                radius: r,
                forward: curve_forward,
            } = edge.curve
            else {
                return Err(shape.into());
            };
            let arc_center = frame.transform_point(Point3::new(0.0, 0.0, 0.0));
            // Agreement of center and radius bounds the support residual
            // throughout the great circle, including between the samples.
            let residual = (arc_center - center).magnitude() + (radius - r).abs();
            if !residual.is_finite() || residual > epsilon {
                return Err("source spherical boundary is off its great circle".into());
            }
            let (start, sweep) = edge.interval(frame, r, curve_forward, epsilon)?;
            if !sweep.is_finite() || sweep.abs() >= PI {
                return Err("native spherical boundary is not a short arc".into());
            }
            let direction = if forward {
                sweep.signum()
            } else {
                -sweep.signum()
            };
            inward[i] = sense * direction * frame.z.truncate();
            corners[i] = edge.points[usize::from(!forward)] - center;
            let (x, y) = (frame.x.truncate() * r, frame.y.truncate() * r);
            let end = start + sweep;
            let offset = arc_center - center;
            radial_error = radial_error.max(residual);
            for (point, t) in edge.points.into_iter().zip([start, end]) {
                let arc_point = offset + x * t.cos() + y * t.sin();
                join_error = join_error.max(((point - center) - arc_point).magnitude());
            }
            arcs.push(ArcExtent {
                offset,
                x,
                y,
                start: start.min(end),
                end: start.max(end),
            });
        }
        let middle = (corners[0] + corners[1] + corners[2]).normalize();
        for i in 0..3 {
            // The third corner must be on the interior side selected by the
            // authored traversal. The three minor arcs then bound the convex
            // intersection of these hemispheres in a single chart.
            if !(inward[i].dot(corners[(i + 2) % 3]) > 4.0 * epsilon
                && middle.dot(corners[i]) > 4.0 * epsilon)
            {
                return Err(
                    "native spherical trim contradicts surface sense or leaves its hemisphere"
                        .into(),
                );
            }
        }
        Ok(SphericalTrim {
            center,
            radius,
            interior: center + middle * radius,
            bounds_planes: enclosing_planes(middle, inward, &arcs, corners, epsilon)?,
            projection_margin: radial_error + join_error,
            boundary,
            same_sense: self.same_sense,
        })
    }
}

impl SphericalTrim {
    pub(super) fn include_bounds(
        &self,
        edges: &HashMap<u64, Edge>,
        world: Matrix4,
        epsilon: f64,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<(), String> {
        let mut rim = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for (id, _) in self.boundary {
            edges[&id].include_bounds(world, epsilon, &mut rim)?;
        }
        // Radial projection moves the original curves onto the spherical
        // support. Inflate their finite bounds by the projection error,
        // including the possible displacement at tolerant joins.
        for i in 0..3 {
            let row = Vector3::new(world.x[i], world.y[i], world.z[i]);
            let length = row.x.hypot(row.y).hypot(row.z);
            let margin = self.projection_margin * length;
            let lower = (rim[0][i] - margin).next_down();
            let upper = (rim[1][i] + margin).next_up();
            if length <= 0.0 || !lower.is_finite() || !upper.is_finite() {
                return Err("nonfinite native spherical bounds".into());
            }
            bounds[0][i] = bounds[0][i].min(lower);
            bounds[1][i] = bounds[1][i].max(upper);
            // A coordinate has only two stationary points away from the
            // boundary. Exclude a pole only if an enclosing plane excludes it.
            let axis = row / length;
            for (pole, sign) in [(axis, 1.0), (-axis, -1.0)] {
                if self
                    .bounds_planes
                    .iter()
                    .all(|side| side.dot(pole) >= -8.0 * f64::EPSILON * side.magnitude())
                {
                    // Place the center first. Adding the radius in source coordinates
                    // can erase it before a large translation cancels the center.
                    // Bound the signed coordinate outward at each affine operation.
                    let mut center = sign * world.w[i];
                    let mut norm2 = 0.0;
                    for (coefficient, origin) in [
                        (row.x, self.center.x),
                        (row.y, self.center.y),
                        (row.z, self.center.z),
                    ] {
                        if coefficient != 0.0 {
                            center = (sign * coefficient).mul_add(origin, center).next_up();
                            norm2 = coefficient.mul_add(coefficient, norm2).next_up();
                        }
                    }
                    let extent = self
                        .radius
                        .mul_add(norm2.sqrt().next_up(), center)
                        .next_up();
                    check_coordinate(Point3::new(extent, 0.0, 0.0))?;
                    if sign > 0.0 {
                        bounds[1][i] = bounds[1][i].max(extent);
                    } else {
                        bounds[0][i] = bounds[0][i].min(-extent);
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn mesh(
        &self,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
        world: Matrix4,
    ) -> Result<Triangles, String> {
        let error = "native spherical facet exceeds its deflection";
        let depth = deflection.min(self.radius * 0.5);
        // An authored vertex can have attachment error to its circle as well
        // as the circle's own attachment error to the sphere.
        let shell_error = 2.0 * epsilon;
        if depth <= shell_error {
            return Err(error.into());
        }
        // For barycentric weights λ, |Σ λ p|² = Σ λ |p|² -
        // Σ λ_i λ_j |p_i - p_j|². The second weight sum is at most 1/3,
        // so this chord limit bounds every point of the triangle.
        let depth_chord2 = 3.0 * (depth - shell_error) * (2.0 * self.radius - depth - shell_error);
        // Two radial directions with angle θ and radii at least r_min
        // have chord length at least 2 r_min sin(θ/2).
        let angular_chord = 2.0 * (self.radius - shell_error) * (ANGULAR_TOLERANCE * 0.5).sin();
        let chord2 = depth_chord2.min(angular_chord * angular_chord);
        if !(chord2.is_finite() && chord2 > 0.0) {
            return Err(error.into());
        }
        let distance2 = |a: Point3, b: Point3| (b - a).magnitude2();
        let mut pending = Vec::new();
        for (id, forward) in self.boundary {
            for pair in samples[&id].windows(2) {
                let (a, b) = if forward {
                    (pair[0], pair[1])
                } else {
                    (pair[1], pair[0])
                };
                if [a, b].into_iter().any(|p| {
                    let residual = ((p - self.center).magnitude() - self.radius).abs();
                    !residual.is_finite() || residual > shell_error
                }) {
                    return Err("source spherical sample is off its support".into());
                }
                if distance2(a, b) > chord2 {
                    return Err("native spherical boundary needs additional shared samples".into());
                }
                pending.push([a, b, self.interior]);
            }
        }
        let mut triangles = Vec::new();
        while let Some(triangle) = pending.pop() {
            let [a, b, c] = triangle;
            let sides = [distance2(a, b), distance2(b, c), distance2(c, a)];
            let longest = (0..3)
                .max_by(|&a, &b| sides[a].total_cmp(&sides[b]))
                .unwrap();
            if sides[longest] <= chord2 {
                let radial = triangle.map(|point| point - self.center);
                if !super::facet::radial_facet_fits(triangle, radial, self.same_sense, world) {
                    return Err("native spherical facet exceeds angular tolerance".into());
                }
                triangles.push(triangle);
                continue;
            }
            if pending.len() + triangles.len() >= 32768 {
                return Err("native spherical trim refinement limit".into());
            }
            let [a, b, c] = [0, 1, 2].map(|offset| triangle[(longest + offset) % 3]);
            // Only interior edges can exceed the limit. Adjacent triangles
            // bisect such an edge at the identical point on the sphere.
            let mid =
                self.center + ((a - self.center) + (b - self.center)).normalize() * self.radius;
            pending.extend([[a, mid, c], [mid, b, c]]);
        }
        Ok(triangles)
    }
}
