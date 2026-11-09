use super::*;
use monstertruck_core::DeterministicContentHash;

pub(super) const fn identity_transform() -> Matrix4 {
    Matrix4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
                 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0)
}
fn is_affine(matrix: Matrix4) -> bool {
    matrix.x.w == 0.0 && matrix.y.w == 0.0 && matrix.z.w == 0.0 && matrix.w.w == 1.0
}
pub(super) fn deserialize_transform<'de, D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Matrix4, D::Error> {
    let matrix = Matrix4::deserialize(deserializer)?;
    if !is_affine(matrix) || !(0..4).all(|i| (0..4).all(|j| matrix[i][j].is_finite())) {
        return Err(serde::de::Error::custom("Plane transform must be finite and affine"));
    }
    Ok(matrix)
}

impl Plane {
    const fn has_identity_transform(&self) -> bool {
        let m = self.transform;
        m.x.x == 1.0 && m.x.y == 0.0 && m.x.z == 0.0 && m.x.w == 0.0
            && m.y.x == 0.0 && m.y.y == 1.0 && m.y.z == 0.0 && m.y.w == 0.0
            && m.z.x == 0.0 && m.z.y == 0.0 && m.z.z == 1.0 && m.z.w == 0.0
            && m.w.x == 0.0 && m.w.y == 0.0 && m.w.z == 0.0 && m.w.w == 1.0
    }
    /// Creates a new plane from three points.
    #[inline(always)]
    pub const fn new(origin: Point3, one: Point3, another: Point3) -> Plane {
        Plane {
            o: origin,
            p: one,
            q: another,
            transform: identity_transform(),
        }
    }
    /// Returns the origin
    #[inline(always)]
    pub const fn origin(&self) -> Point3 {
        if self.has_identity_transform() { return self.o; }
        let m = self.transform;
        Point3::new(
            m.x.x*self.o.x + m.y.x*self.o.y + m.z.x*self.o.z + m.w.x,
            m.x.y*self.o.x + m.y.y*self.o.y + m.z.y*self.o.z + m.w.y,
            m.x.z*self.o.x + m.y.z*self.o.y + m.z.z*self.o.z + m.w.z,
        )
    }
    /// Returns the u-axis
    #[inline(always)]
    pub fn axis_u(&self) -> Vector3 {
        let axis = self.p - self.o;
        if self.has_identity_transform() { axis } else { self.transform.transform_vector(axis) }
    }
    /// Returns the v-axis
    #[inline(always)]
    pub fn axis_v(&self) -> Vector3 {
        let axis = self.q - self.o;
        if self.has_identity_transform() { axis } else { self.transform.transform_vector(axis) }
    }
    /// Deprecated alias for [`axis_u`](Plane::axis_u).
    ///
    /// Renamed for `<property>_<direction>` consistency with `derivative_u`,
    /// `knot_vector_u` and `cut_u`, which is the convention across this crate.
    #[deprecated(since = "0.3.4", note = "renamed to `axis_u`")]
    #[inline(always)]
    pub fn u_axis(&self) -> Vector3 { self.axis_u() }
    /// Deprecated alias for [`axis_v`](Plane::axis_v).
    ///
    /// Renamed for `<property>_<direction>` consistency with `derivative_v`,
    /// `knot_vector_v` and `cut_v`, which is the convention across this crate.
    #[deprecated(since = "0.3.4", note = "renamed to `axis_v`")]
    #[inline(always)]
    pub fn v_axis(&self) -> Vector3 { self.axis_v() }
    /// Returns the normal
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let plane = Plane::new(
    ///     Point3::new(0.0, 0.0, 0.0),
    ///     Point3::new(1.0, 0.0, 0.0),
    ///     Point3::new(0.0, 1.0, 0.0),
    /// );
    /// assert_near!(plane.normal(), Vector3::unit_z());
    /// ```
    #[inline(always)]
    pub fn normal(&self) -> Vector3 { self.axis_u().cross(self.axis_v()).normalize() }
    /// Gets the parameter of `pt` in plane's matrix.
    /// # Examples
    /// ```
    /// use monstertruck_geometry::prelude::*;
    /// let plane = Plane::new(
    ///     Point3::new(1.0, 2.0, 3.0),
    ///     Point3::new(2.0, 1.0, 3.0),
    ///     Point3::new(3.0, 4.0, -1.0),
    /// );
    ///
    /// let pt = Point3::new(2.1, -6.5, 4.7);
    /// let prm = plane.parameter(pt);
    /// let rev = plane.origin()
    ///     + prm[0] * plane.axis_u()
    ///     + prm[1] * plane.axis_v()
    ///     + prm[2] * plane.normal();
    /// assert_near!(pt, rev);
    /// ```
    #[inline(always)]
    pub fn parameter(&self, pt: Point3) -> Vector3 {
        let a = self.axis_u();
        let b = self.axis_v();
        let c = self.normal();
        // SAFETY: `u_axis`, `v_axis`, and `normal` are linearly independent by
        // the `Plane` construction invariant.
        let mat = Matrix3::from_cols(a, b, c).invert().unwrap();
        mat * (pt - self.origin())
    }
    /// xy-plane
    #[inline(always)]
    pub const fn xy() -> Self {
        Self {
            o: Point3::new(0.0, 0.0, 0.0),
            p: Point3::new(1.0, 0.0, 0.0),
            q: Point3::new(0.0, 1.0, 0.0),
            transform: identity_transform(),
        }
    }
    /// yz-plane
    #[inline(always)]
    pub const fn yz() -> Self {
        Self {
            o: Point3::new(0.0, 0.0, 0.0),
            p: Point3::new(0.0, 1.0, 0.0),
            q: Point3::new(0.0, 0.0, 1.0),
            transform: identity_transform(),
        }
    }
    /// zx-plane
    #[inline(always)]
    pub const fn zx() -> Self {
        Self {
            o: Point3::new(0.0, 0.0, 0.0),
            p: Point3::new(0.0, 0.0, 1.0),
            q: Point3::new(1.0, 0.0, 0.0),
            transform: identity_transform(),
        }
    }
}

impl ParametricSurface for Plane {
    type Point = Point3;
    type Vector = Vector3;
    #[inline(always)]
    fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Self::Vector {
        match (m, n) {
            (0, 0) => self.evaluate(u, v).to_vec(),
            (1, 0) => self.axis_u(),
            (0, 1) => self.axis_v(),
            _ => Vector3::zero(),
        }
    }
    #[inline(always)]
    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        let point = self.o + u * (self.p - self.o) + v * (self.q - self.o);
        // Legacy identity records retain saved arithmetic, including signed zero.
        if self.has_identity_transform() { point } else { self.transform.transform_point(point) }
    }
    #[inline(always)]
    fn derivative_u(&self, _: f64, _: f64) -> Vector3 { self.axis_u() }
    #[inline(always)]
    fn derivative_v(&self, _: f64, _: f64) -> Vector3 { self.axis_v() }
    #[inline(always)]
    fn derivative_uu(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn derivative_uv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn derivative_vv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    /// The unit square `[0, 1] x [0, 1]` -- a FICTION, and a load-bearing one.
    ///
    /// A plane is unbounded in both parameters: `evaluate(u, v)` is defined and
    /// meaningful for every finite `u`, `v`, and planar-cap trims routinely
    /// project far outside this square (the STEP bracket's planar faces carry
    /// world-scaled trim rectangles reaching u ~ 3.9 and v ~ 22.0). The square
    /// is the `o`/`p`/`q` frame's own cell, not a domain.
    ///
    /// AIDEV-NOTE: do NOT "fix" this to `Bound::Unbounded`. It was measured
    /// (spec 010) and the honest range is the one thing that cannot be reported
    /// here, for two independent reasons.
    ///
    /// First, [`BoundedSurface`] is implemented for `Plane` just below, and its
    /// `range_tuple` is `try_range_tuple().expect(UNBOUNDED_ERROR)`
    /// (`monstertruck-traits/src/traits/surface.rs`). An unbounded plane would
    /// turn every `plane.range_tuple()` into a panic. The v2 mirror further
    /// down hardcodes the same square for the same reason.
    ///
    /// Second, several consumers treat this square as an AUTHORITATIVE domain
    /// and would change their answers, not merely their diagnostics:
    ///
    /// - `monstertruck-healing/src/lib.rs` (`reattach_preserved_face_trims`
    ///   and `regenerate_linear_trim_segment`) HARD-clamps every projected trim
    ///   sample into the reported box -- a plane trim at u = 3.9 becomes
    ///   u = 1.0. It is the square that keeps those two paths self-consistent.
    /// - `monstertruck-meshing/src/tessellation/triangulation/` meshes an
    ///   untrimmed plane face as exactly this rectangle, and `boundary.rs`
    ///   synthesizes a missing loop side from it. An unbounded plane there
    ///   becomes `FaceDropReason::UnboundedDomain`, which
    ///   an external SSI boolean backend escalates.
    /// - a backend AABB pass over `trimmed_surface_range_aabb`
    ///   rejects a trim rectangle that leaves the surface frame, and the frame
    ///   for a plane IS this unit cell (via the homogeneous B-spline conversion
    ///   below, which emits a bezier-1 net over the same cell).
    /// - the analytic plane-vs-plane SSI in
    ///   an external analytic SSI backend
    ///   rejects intersections falling outside the reported ranges.
    ///
    /// The modeling projector
    /// (`monstertruck-modeling/src/geometry/::project_onto_surface_domain`) is
    /// the one place that sees the square and is provably inert against it: a
    /// plane's projection is linear, so all four solver attempts return the same
    /// answer and the out-of-domain rejection falls through to the unchanged
    /// fallback. That is asserted by
    /// `a_plane_whose_reported_square_is_not_a_real_bound_is_unaffected`.
    ///
    /// So: a new rule may NOT treat `try_range_tuple()` as a plane's real
    /// domain. Clamp, cull, and containment tests against it are wrong for most
    /// planar trims even though the four consumers above currently survive by
    /// agreeing with each other about the same fiction.
    #[inline(always)]
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        let range = (Bound::Included(0.0), Bound::Included(1.0));
        (range, range)
    }
}

impl ParametricSurface3D for Plane {
    fn affine_plane_certificate(&self) -> Option<AffinePlaneCertificate> {
        AffinePlaneCertificate::from_points(self.o, self.p, self.q)?.transformed(self.transform.into())
    }
    fn is_affine_plane(&self) -> bool {
        let area = self.axis_u().cross(self.axis_v()).magnitude2();
        [self.origin(), self.evaluate(1.0, 0.0), self.evaluate(0.0, 1.0)].into_iter()
            .all(|p| [p.x, p.y, p.z].into_iter().all(f64::is_finite))
            && area.is_finite() && area > 0.0
    }
    #[inline(always)]
    fn normal(&self, _: f64, _: f64) -> Vector3 { self.normal() }
    #[inline(always)]
    fn normal_uder(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn normal_vder(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
}

impl BoundedSurface for Plane {}

impl Invertible for Plane {
    #[inline(always)]
    fn inverse(&self) -> Self {
        Plane {
            o: self.o,
            p: self.q,
            q: self.p,
            transform: self.transform,
        }
    }
    #[inline(always)]
    fn invert(&mut self) { *self = self.inverse(); }
}

impl IncludeCurve<Line<Point3>> for Plane {
    #[inline(always)]
    fn include(&self, line: &Line<Point3>) -> bool {
        self.search_parameter(line.0, None, 1).is_some()
            && self.search_parameter(line.1, None, 1).is_some()
    }
}

impl IncludeCurve<BsplineCurve<Point3>> for Plane {
    #[inline(always)]
    fn include(&self, curve: &BsplineCurve<Point3>) -> bool {
        let origin = self.origin();
        let normal = self.normal();
        curve
            .control_points()
            .iter()
            .all(|pt| (pt - origin).dot(normal).so_small())
    }
}

impl IncludeCurve<NurbsCurve<Vector4>> for Plane {
    fn include(&self, curve: &NurbsCurve<Vector4>) -> bool {
        let origin = self.origin();
        let normal = self.normal();
        let (s, e) = (curve.front(), curve.back());
        if !(s - origin).dot(normal).so_small() || !(e - origin).dot(normal).so_small() {
            return false;
        }
        curve.non_rationalized().control_points().iter().all(|pt| {
            if pt[3].so_small() {
                true
            } else {
                let pt = Point3::from_homogeneous(*pt);
                (pt - origin).dot(normal).so_small()
            }
        })
    }
}

impl ParameterDivision2D for Plane {
    #[inline(always)]
    fn parameter_division(&self, range: ((f64, f64), (f64, f64)), _: f64) -> (Vec<f64>, Vec<f64>) {
        (vec![range.0.0, range.0.1], vec![range.1.0, range.1.1])
    }
}

impl<T: Transform3<Scalar = f64>> Transformed<T> for Plane {
    #[inline(always)]
    fn transform_by(&mut self, trans: T) {
        let matrix: Matrix4 = trans.into();
        if is_affine(matrix) {
            // Keep authored points: independently rounding their images loses exact plane relations.
            self.transform = matrix * self.transform;
        } else {
            // Preserve the historical three-point image for projective transforms.
            *self = Self::new(
                matrix.transform_point(self.origin()),
                matrix.transform_point(self.transform.transform_point(self.p)),
                matrix.transform_point(self.transform.transform_point(self.q)),
            );
        }
    }
    #[inline(always)]
    fn transformed(&self, trans: T) -> Self {
        let mut result = *self;
        result.transform_by(trans);
        result
    }
}

impl SearchParameter<SurfaceParameter> for Plane {
    type Point = Point3;
    #[inline(always)]
    fn search_parameter<H: Into<SearchParameterHint2D>>(
        &self,
        point: Point3,
        _: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let v = self.parameter(point);
        match v[2].so_small() {
            true => Some((v[0], v[1])),
            false => None,
        }
    }
}

impl SearchNearestParameter<SurfaceParameter> for Plane {
    type Point = Point3;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<SearchParameterHint2D>>(
        &self,
        point: Point3,
        _: H,
        _: usize,
    ) -> Option<(f64, f64)> {
        let v = self.parameter(point);
        Some((v[0], v[1]))
    }
}

impl From<Plane> for BsplineSurface<Point3> {
    fn from(plane: Plane) -> Self {
        if plane.has_identity_transform() {
            let Plane { o, p, q, .. } = plane;
            return BsplineSurface::debug_new(
                (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
                vec![vec![o, q], vec![p, p + (q - o)]],
            );
        }
        let o = plane.evaluate(0.0, 0.0);
        let p = plane.evaluate(1.0, 0.0);
        let q = plane.evaluate(0.0, 1.0);
        BsplineSurface::debug_new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            vec![vec![o, q], vec![p, plane.evaluate(1.0, 1.0)]],
        )
    }
}

impl ToSameGeometry<BsplineSurface<Point3>> for Plane {
    fn to_same_geometry(&self) -> BsplineSurface<Point3> { (*self).into() }
}

// -- v2 scalar-generic impls ------------------------------------------------

use monstertruck_traits::v2;

impl v2::ParametricSurface for Plane {
    type Scalar = f64;
    type Point = Point3;
    type Vector = Vector3;

    #[inline(always)]
    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        ParametricSurface::evaluate(self, u, v)
    }
    #[inline(always)]
    fn derivative_u(&self, _: f64, _: f64) -> Vector3 { self.axis_u() }
    #[inline(always)]
    fn derivative_v(&self, _: f64, _: f64) -> Vector3 { self.axis_v() }
    #[inline(always)]
    fn derivative_uu(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn derivative_uv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn derivative_vv(&self, _: f64, _: f64) -> Vector3 { Vector3::zero() }
    #[inline(always)]
    fn period_u(&self) -> Option<f64> { None }
    #[inline(always)]
    fn period_v(&self) -> Option<f64> { None }
}

impl v2::BoundedSurface for Plane {
    #[inline(always)]
    fn range_tuple(&self) -> ((f64, f64), (f64, f64)) { ((0.0, 1.0), (0.0, 1.0)) }
}

impl v2::ParametricSurface3D for Plane {}

impl v2::SearchNearestParameter<v2::D2<f64>> for Plane {
    type Point = Point3;
    #[inline(always)]
    fn search_nearest_parameter<H: Into<v2::SearchParameterHint2D<f64>>>(
        &self,
        pt: Point3,
        _: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        SearchNearestParameter::<D2>::search_nearest_parameter(self, pt, None, trials)
    }
}

impl v2::SearchParameter<v2::D2<f64>> for Plane {
    type Point = Point3;
    #[inline(always)]
    fn search_parameter<H: Into<v2::SearchParameterHint2D<f64>>>(
        &self,
        pt: Point3,
        _: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        SearchParameter::<D2>::search_parameter(self, pt, None, trials)
    }
}

impl DeterministicContentHash for Plane {
    fn content_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.o.content_hash(state);
        self.p.content_hash(state);
        self.q.content_hash(state);
        self.transform.content_hash(state);
    }
}

#[cfg(test)]
mod source_hash_tests {
    use super::*;
    use std::hash::Hasher;
    fn hash(plane: Plane) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        plane.content_hash(&mut state);
        state.finish()
    }
    #[test]
    fn hash_covers_authored_points_and_transform() {
        let source = Plane::xy();
        let moved = source.transformed(Matrix4::from_translation(Vector3::new(1.0,2.0,3.0)));
        let flattened = Plane::new(moved.origin(), moved.evaluate(1.0,0.0), moved.evaluate(0.0,1.0));
        assert_eq!(moved.evaluate(0.2,0.4), flattened.evaluate(0.2,0.4));
        assert_ne!(hash(source), hash(moved));
        assert_ne!(hash(moved), hash(flattened));
        assert_eq!(hash(moved), hash(moved));
    }
}
