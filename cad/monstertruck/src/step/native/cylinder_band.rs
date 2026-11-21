//! A full-period cylinder bounded by two independently authored circular rims.
use super::{
    geometry::Curve,
    trim::{Samples, Triangles},
    *,
};
use cgmath::Vector2;

impl Face {
    pub(super) fn mesh_cylinder_band(
        &self,
        radius: f64,
        rings: [&Boundary; 2],
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
    ) -> Result<Triangles, String> {
        let [&[(a, fa)], &[(b, fb)]] = rings.map(|ring| ring.edges.as_slice()) else {
            return Err("native cylindrical band requires two single-edge rims".into());
        };
        if radius <= 4.0 * epsilon {
            return Err(
                "native cylindrical band radius is too small for attachment tolerance".into(),
            );
        }
        for id in [a, b] {
            let edge = &edges[&id];
            if edge.vertices[0] != edge.vertices[1] || !matches!(edge.curve, Curve::Circle { .. }) {
                return Err("native cylindrical band rims must be complete circles".into());
            }
            edge.cylinder_sweep(self.surface, radius, epsilon)?;
        }

        let a = &samples[&a];
        let b = &samples[&b];
        if a.len() != b.len() || a.len() < 4 {
            return Err("native cylindrical band rims lack matching subdivisions".into());
        }
        let last = a.len() - 1;
        // The edge sampler caps both rims at 4096 segments. Pair their
        // authored start phases and traverse the second rim in reverse so
        // each pair of triangles inherits the two boundary directions.
        let at = |i| {
            [
                a[if fa { i } else { last - i }],
                b[if fb { last - i } else { i }],
            ]
        };
        let radial = |p: Point3| {
            let p = self.surface.local(p);
            Vector2::new(p.x, p.y)
        };
        let axis = self.surface.frame.z.truncate();
        let [a0, b0] = at(0);
        let height = (b0 - a0).dot(axis);
        if height.abs() <= 4.0 * epsilon {
            return Err("native cylindrical band rims touch".into());
        }
        for i in 0..a.len() {
            let [p, q] = at(i);
            let delta = q - p;
            if delta.cross(axis).magnitude() > epsilon || (delta.dot(axis) - height).abs() > epsilon
            {
                return Err("native cylindrical band rims are not aligned".into());
            }
        }

        let mut triangles = Vec::with_capacity(2 * last);
        let sense = if self.same_sense { 1.0 } else { -1.0 };
        let margin = deflection - 2.0 * epsilon;
        let cos_limit = ANGULAR_TOLERANCE.cos();
        for i in 0..last {
            let [[a, d], [b, c]] = [at(i), at(i + 1)];
            let mid = (radial(a).normalize() + radial(b).normalize()).normalize();
            for triangle in [[a, b, d], [b, c, d]] {
                let [a, b, c] = triangle;
                let n = self
                    .surface
                    .local_direction((b - a).cross(c - a).normalize())
                    * sense;
                let n = Vector2::new(n.x, n.y);
                // Projection onto the mid-span radial is affine. Together with
                // convexity of the radial norm this bounds the whole facet's
                // distance to the cylinder, including its interior.
                // The normal test also extends over the facet: for a convex
                // combination of the vertex radials, n·r >= cos(limit) * |r|.
                for r in triangle.map(radial) {
                    if !(mid.dot(r) >= radius - margin
                        && r.magnitude() <= radius + margin
                        && n.dot(r) >= cos_limit * r.magnitude())
                    {
                        return Err("native cylindrical band exceeds facet tolerance".into());
                    }
                }
                triangles.push(triangle);
            }
        }
        Ok(triangles)
    }
}
