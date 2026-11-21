//! Cylindrical strips between a circular rim and an oblique planar section.
use super::{
    geometry::Curve,
    trim::{Samples, Triangles},
    *,
};
use cgmath::Vector2;
use std::f64::consts::{PI, TAU};

impl Face {
    pub(super) fn check_planar_ellipse_trim(
        &self,
        edges: &HashMap<u64, Edge>,
        epsilon: f64,
    ) -> Result<(), String> {
        let error = "native planar ellipse requires one arc and a straight return path";
        let [ring] = self.loops.as_slice() else {
            return Err(error.into());
        };
        let mut ellipses = ring
            .edges
            .iter()
            .filter_map(|(id, _)| matches!(edges[id].curve, Curve::Ellipse { .. }).then_some(*id));
        let ellipse_id = ellipses.next().ok_or(error)?;
        if ellipses.next().is_some() {
            return Err(error.into());
        }
        let ellipse = &edges[&ellipse_id];
        let (frame, _, start, sweep) = ellipse.ellipse(epsilon)?;
        if sweep.abs() >= PI {
            return Err("native planar ellipse does not fit a convex angular sector".into());
        }
        let center = Point3::new(frame.w.x, frame.w.y, frame.w.z);
        let radial =
            |angle: f64| frame.x.truncate() * angle.cos() + frame.y.truncate() * angle.sin();
        let start_ray = radial(start).normalize();
        let end_ray = radial(start + sweep).normalize();
        let axis = frame.z.truncate() * sweep.signum();
        // An arc shorter than a half turn lies inside the intersection of
        // its two radial half-planes. Keep each return edge in the exterior of
        // one half-plane so neither an arc crossing nor a tangent contact can
        // disappear between the mesh samples. Only its authored joins may touch.
        for &(id, _) in &ring.edges {
            if id == ellipse_id {
                continue;
            }
            let edge = &edges[&id];
            if !matches!(edge.curve, Curve::Line { .. }) {
                return Err(error.into());
            }
            let points = edge.vertices.into_iter().zip(edge.points);
            let outside_start = points.clone().all(|(vertex, point)| {
                vertex == ellipse.vertices[0]
                    || axis.dot(start_ray.cross(point - center)) < -epsilon
            });
            let outside_end = points.clone().all(|(vertex, point)| {
                vertex == ellipse.vertices[1]
                    || axis.dot((point - center).cross(end_ray)) < -epsilon
            });
            if !outside_start && !outside_end {
                return Err("native planar return path enters the elliptical arc sector".into());
            }
        }
        Ok(())
    }

    pub(super) fn mesh_oblique_cylinder(
        &self,
        radius: f64,
        ring: &Boundary,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        world: Matrix4,
    ) -> Result<Triangles, String> {
        let shape_error =
            "native oblique cylinder requires one circle, one ellipse and two generators";
        let start = ring
            .edges
            .iter()
            .position(|(id, _)| matches!(edges[id].curve, Curve::Circle { .. }))
            .ok_or(shape_error)?;
        if ring.edges.len() != 4 {
            return Err(shape_error.into());
        }
        let [
            (circle, circle_forward),
            (generator_a, _),
            (ellipse, ellipse_forward),
            (generator_b, _),
        ] = [0, 1, 2, 3].map(|i| ring.edges[(start + i) % 4]);
        let (Curve::Line { .. }, Curve::Ellipse { .. }, Curve::Line { .. }) = (
            &edges[&generator_a].curve,
            &edges[&ellipse].curve,
            &edges[&generator_b].curve,
        ) else {
            return Err(shape_error.into());
        };
        let radial = |p: Vector3<f64>| Vector2::new(p.x, p.y);
        for generator in [&edges[&generator_a], &edges[&generator_b]] {
            let [a, b] = generator.points.map(|point| self.surface.local(point));
            if radial(b - a).magnitude() > epsilon || (b.z - a.z).abs() <= epsilon {
                return Err("native oblique cylinder line is not a generator".into());
            }
        }
        let sweep = edges[&circle].cylinder_sweep(self.surface, radius, epsilon)?;
        let sweep = if circle_forward { sweep } else { -sweep };
        let circle = &samples[&circle];
        let circular_start = self
            .surface
            .local(circle[if circle_forward { 0 } else { circle.len() - 1 }]);
        let (frame, _, start, extent) = edges[&ellipse].ellipse(epsilon)?;
        // Compare the two trims in the circle's traversal direction.
        let (start, extent) = if ellipse_forward {
            (start + extent, -extent)
        } else {
            (start, extent)
        };
        let angular_margin = 4.0 * epsilon / radius;
        if (sweep.abs() - extent.abs()).abs() * radius > epsilon
            || sweep.abs() <= angular_margin
            || sweep.abs() >= TAU - angular_margin
        {
            return Err(
                "native oblique cylinder trims do not have a common single-chart sweep".into(),
            );
        }

        let center = self
            .surface
            .local(Point3::new(frame.w.x, frame.w.y, frame.w.z));
        let x = self.surface.local_direction(frame.x.truncate());
        let y = self.surface.local_direction(frame.y.truncate());
        let (sin, cos) = start.sin_cos();
        let first = x * cos + y * sin;
        let quarter = (y * cos - x * sin) * extent.signum();
        let u = radial(circular_start).normalize() * radius;
        let v = Vector2::new(-u.y, u.x) * sweep.signum();
        // The entire projected ellipse is c + a cos(t) + b sin(t). Matching
        // its coefficients to a circle bounds the residual over the full arc,
        // including the portions between the tessellated vertices. The same
        // coefficient error bounds the derivative. At less than one quarter
        // radius it cannot reverse the sign of the azimuthal derivative.
        let residual = radial(center).magnitude()
            + (radial(first) - u).magnitude()
            + (radial(quarter) - v).magnitude();
        let reduction = edges[&ellipse]
            .finite_ellipse
            .as_ref()
            .map_or(0.0, |trim| trim.residual);
        if !residual.is_finite() || residual + reduction > epsilon || 4.0 * residual >= radius {
            return Err("source elliptical boundary is off its CYLINDRICAL_SURFACE".into());
        }

        // The height difference to the horizontal rim is sinusoidal. Check
        // both endpoints and every interior extremum before accepting the strip.
        let end = extent.abs();
        let gap = |t: f64| center.z - circular_start.z + first.z * t.cos() + quarter.z * t.sin();
        let sign = gap(0.0).signum();
        if [0.0, end]
            .into_iter()
            .chain(super::super::bounds::ellipse_extrema(
                first.z, quarter.z, 0.0, end,
            ))
            .any(|t| !(gap(t) * sign > 4.0 * epsilon))
        {
            return Err("native oblique cylinder trims touch or cross".into());
        }

        let ellipse = &samples[&ellipse];
        let (m, n) = (circle.len() - 1, ellipse.len() - 1);
        let c = |i| circle[if circle_forward { i } else { m - i }];
        let e = |j| ellipse[if ellipse_forward { n - j } else { j }];
        let axis = self.surface.frame.z.truncate();
        let origin = self
            .surface
            .frame
            .transform_point(Point3::new(0.0, 0.0, 0.0));
        let mut triangles = Vec::with_capacity(m + n);
        let (mut i, mut j) = (0, 0);
        // Merge the two angular subdivisions. Each triangle's angular width
        // is bounded by the larger rail step, with no new boundary samples.
        while i < m || j < n {
            let triangle = if i < m && (j == n || (i + 1) * n <= (j + 1) * m) {
                i += 1;
                [c(i - 1), c(i), e(j)]
            } else {
                j += 1;
                [c(i), e(j), e(j - 1)]
            };
            let directions = triangle.map(|point| {
                let p = point - origin;
                p - axis * p.dot(axis)
            });
            if !super::facet::radial_facet_fits(triangle, directions, self.same_sense, world) {
                return Err("native oblique cylinder facet exceeds angular tolerance".into());
            }
            triangles.push(triangle);
        }
        Ok(triangles)
    }
}
