//! A convex spherical triangle with three short great-circle arcs.
use super::{
    geometry::Curve,
    trim::{Samples, Triangles},
    *,
};
use std::f64::consts::PI;

pub(super) struct SphericalTrim {
    center: Point3,
    radius: f64,
    interior: Point3,
    inward: [Vector3<f64>; 3],
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
            let (_, sweep) = edge.interval(frame, r, curve_forward, epsilon)?;
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
            inward,
            boundary,
            same_sense: self.same_sense,
        })
    }
}

impl SphericalTrim {
    pub(super) fn include_bounds(
        &self,
        world: Matrix4,
        epsilon: f64,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<(), String> {
        // Boundary extrema are already included via the authored arcs. Each
        // world coordinate has just two stationary points on the full sphere.
        for i in 0..3 {
            let axis = Vector3::new(world.x[i], world.y[i], world.z[i]).normalize();
            for pole in [axis, -axis] {
                if self
                    .inward
                    .iter()
                    .all(|side| side.dot(pole) * self.radius >= -2.0 * epsilon)
                {
                    let point = world.transform_point(self.center + pole * self.radius);
                    check_coordinate(point)?;
                    super::super::bounds::push(bounds, point)?;
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
                let orientation = (b - a).cross(c - a).dot(a - self.center);
                if !orientation.is_finite()
                    || orientation == 0.0
                    || (orientation > 0.0) != self.same_sense
                {
                    return Err("native spherical facet folds under radial projection".into());
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
