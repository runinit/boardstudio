use super::*;
use array_macro::array;
#[cfg(test)]
use robust::{Coord, Coord3D, orient2d, orient3d};

/// Find collisions between two polygon meshes and extract interference lines.
///
/// # Details
///
/// The algorithm for mesh extraction is as follows.
/// All the triangles are projected toward the axis to form intervals.
/// By looking at the overlap of these intervals, we can narrow down the interfering triangles.
/// The overlap of the intervals is obtained by sorting the endpoints.
///
/// # Remarks
///
/// We see that the surfaces that are in contact are not interfering with each other.
/// Therefore, polygon meshes included in one plane are not interfering.
pub trait Collision {
    /// If `self` and `other` collide, then returns only one interference line.
    /// Otherwise, returns `None`.
    fn collide_with(&self, other: &PolygonMesh) -> Option<(Point3, Point3)>;
    /// Extract all interference lines between `self` and `other`.
    /// # Remarks
    /// The results is not arranged so that included lines make continuous maximal polyline curve.
    fn extract_interference(&self, other: &PolygonMesh) -> Vec<(Point3, Point3)>;
}

impl Collision for PolygonMesh {
    #[inline(always)]
    fn collide_with(&self, other: &PolygonMesh) -> Option<(Point3, Point3)> {
        are_colliding(self, other)
    }
    #[inline(always)]
    fn extract_interference(&self, other: &PolygonMesh) -> Vec<(Point3, Point3)> {
        collision(self, other)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EndPointType {
    Front,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct EndPoint {
    entity: f64,
    r#type: EndPointType,
    segnum: usize,
    index: usize,
}

impl EndPoint {
    #[inline(always)]
    fn new(entity: f64, r#type: EndPointType, segnum: usize, index: usize) -> EndPoint {
        EndPoint {
            entity,
            r#type,
            segnum,
            index,
        }
    }
    #[inline(always)]
    fn from_seg(seg: (f64, f64), segnum: usize, index: usize) -> Vec<EndPoint> {
        vec![
            EndPoint::new(seg.0, EndPointType::Front, segnum, index),
            EndPoint::new(seg.1, EndPointType::Back, segnum, index),
        ]
    }
}

#[cfg(test)]
fn orient2(a: Point3, b: Point3, c: Point3, drop_axis: usize) -> f64 {
    let project = |p: Point3| match drop_axis {
        0 => Coord { x: p.y, y: p.z },
        1 => Coord { x: p.x, y: p.z },
        _ => Coord { x: p.x, y: p.y },
    };
    orient2d(project(a), project(b), project(c))
}

fn triangle_has_resolvable_area(tri: [Point3; 3]) -> bool {
    if !tri
        .iter()
        .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
    {
        return false;
    }
    let [a, b, c] = tri.map(interval_point);
    let normal = icross(isub(b, a), isub(c, a));
    if normal.iter().any(|v| v.sign().is_some()) {
        return true;
    }
    use num_traits::Zero;
    let [a, b, c] = tri.map(exact_rational_point);
    exact_cross(&exact_sub(&b, &a), &exact_sub(&c, &a))
        .iter()
        .any(|v| !v.is_zero())
}

fn sorted_endpoints<I, J>(iter0: I, iter1: J, _unit: Vector3) -> Vec<EndPoint>
where
    I: IntoIterator<Item = [Point3; 3]>,
    J: IntoIterator<Item = [Point3; 3]>,
{
    let mut axes: [Vec<EndPoint>; 3] = std::array::from_fn(|_| Vec::new());
    for (segnum, index, tri) in iter0
        .into_iter()
        .enumerate()
        .map(|(i, t)| (0, i, t))
        .chain(iter1.into_iter().enumerate().map(|(i, t)| (1, i, t)))
    {
        if !triangle_has_resolvable_area(tri) {
            continue;
        }
        for (axis, endpoints) in axes.iter_mut().enumerate() {
            let lo = tri.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
            let hi = tri
                .iter()
                .map(|p| p[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            endpoints.extend(EndPoint::from_seg((lo, hi), segnum, index));
        }
    }
    for endpoints in &mut axes {
        endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    // Count candidate pairs without materializing them; ties retain x/y/z order.
    axes.into_iter()
        .min_by_key(|endpoints| {
            let mut active = [0usize; 2];
            let mut pairs = 0usize;
            for e in endpoints {
                match e.r#type {
                    EndPointType::Front => {
                        pairs = pairs.saturating_add(active[1 - e.segnum]);
                        active[e.segnum] += 1;
                    }
                    EndPointType::Back => active[e.segnum] -= 1,
                }
            }
            pairs
        })
        .unwrap()
}

fn colliding_segment_pairs(sort_endpoints: Vec<EndPoint>) -> impl Iterator<Item = (usize, usize)> {
    let mut current = [Vec::<usize>::new(), Vec::<usize>::new()];
    sort_endpoints
        .into_iter()
        .filter_map(
            move |EndPoint {
                      r#type,
                      segnum,
                      index,
                      ..
                  }| match r#type {
                EndPointType::Front => {
                    current[segnum].push(index);
                    Some(
                        current[1 - segnum]
                            .clone()
                            .into_iter()
                            .map(move |i| if segnum == 0 { (index, i) } else { (i, index) }),
                    )
                }
                EndPointType::Back => {
                    // SAFETY: every Back endpoint has a matching Front that pushed this index.
                    let i = current[segnum]
                        .iter()
                        .enumerate()
                        .find(|(_, idx)| **idx == index)
                        .unwrap()
                        .0;
                    current[segnum].swap_remove(i);
                    None
                }
            },
        )
        .flatten()
}

fn disjoint_bdbs(tri0: [Point3; 3], tri1: [Point3; 3]) -> bool {
    let bdb0: BoundingBox<Point3> = tri0.iter().collect();
    let bdb1: BoundingBox<Point3> = tri1.iter().collect();
    bdb0.max()[0] < bdb1.min()[0]
        || bdb1.max()[0] < bdb0.min()[0]
        || bdb0.max()[1] < bdb1.min()[1]
        || bdb1.max()[1] < bdb0.min()[1]
        || bdb0.max()[2] < bdb1.min()[2]
        || bdb1.max()[2] < bdb0.min()[2]
}

#[cfg(test)]
fn orient3(p: Point3) -> Coord3D<f64> {
    Coord3D {
        x: p.x,
        y: p.y,
        z: p.z,
    }
}

fn exact_rational_point(p: Point3) -> [num_rational::BigRational; 3] {
    [
        num_rational::BigRational::from_float(p.x).expect("finite x coordinate"),
        num_rational::BigRational::from_float(p.y).expect("finite y coordinate"),
        num_rational::BigRational::from_float(p.z).expect("finite z coordinate"),
    ]
}

fn exact_sub(
    a: &[num_rational::BigRational; 3],
    b: &[num_rational::BigRational; 3],
) -> [num_rational::BigRational; 3] {
    [
        a[0].clone() - b[0].clone(),
        a[1].clone() - b[1].clone(),
        a[2].clone() - b[2].clone(),
    ]
}

fn exact_cross(
    a: &[num_rational::BigRational; 3],
    b: &[num_rational::BigRational; 3],
) -> [num_rational::BigRational; 3] {
    [
        a[1].clone() * b[2].clone() - a[2].clone() * b[1].clone(),
        a[2].clone() * b[0].clone() - a[0].clone() * b[2].clone(),
        a[0].clone() * b[1].clone() - a[1].clone() * b[0].clone(),
    ]
}

fn exact_dot(
    a: &[num_rational::BigRational; 3],
    b: &[num_rational::BigRational; 3],
) -> num_rational::BigRational {
    a[0].clone() * b[0].clone() + a[1].clone() * b[1].clone() + a[2].clone() * b[2].clone()
}

// Every elementary operation encloses its exact real result, including gradual
// underflow. Infinite/NaN arithmetic widens to the entire line, forcing fallback.
#[derive(Clone, Copy)]
struct Interval {
    lo: f64,
    hi: f64,
}
impl Interval {
    fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn bound(lo: f64, hi: f64) -> Self {
        if !lo.is_finite() || !hi.is_finite() {
            Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            }
        } else {
            Self {
                lo: lo.next_down(),
                hi: hi.next_up(),
            }
        }
    }
    fn sub(self, b: Self) -> Self {
        Self::bound(self.lo - b.hi, self.hi - b.lo)
    }
    fn add(self, b: Self) -> Self {
        Self::bound(self.lo + b.lo, self.hi + b.hi)
    }
    fn mul(self, b: Self) -> Self {
        let p = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if p.iter().any(|x| !x.is_finite()) {
            return Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        Self::bound(
            p.into_iter().fold(f64::INFINITY, f64::min),
            p.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn sign(self) -> Option<bool> {
        if self.lo > 0. {
            Some(true)
        } else if self.hi < 0. {
            Some(false)
        } else {
            None
        }
    }
}
fn interval_point(p: Point3) -> [Interval; 3] {
    [p.x, p.y, p.z].map(Interval::point)
}
fn isub(a: [Interval; 3], b: [Interval; 3]) -> [Interval; 3] {
    std::array::from_fn(|i| a[i].sub(b[i]))
}
fn icross(a: [Interval; 3], b: [Interval; 3]) -> [Interval; 3] {
    [
        a[1].mul(b[2]).sub(a[2].mul(b[1])),
        a[2].mul(b[0]).sub(a[0].mul(b[2])),
        a[0].mul(b[1]).sub(a[1].mul(b[0])),
    ]
}
fn idot(a: [Interval; 3], b: [Interval; 3]) -> Interval {
    a[0].mul(b[0]).add(a[1].mul(b[1])).add(a[2].mul(b[2]))
}
fn certified_miss(seg: [Point3; 2], tri: [Point3; 3]) -> bool {
    let [a, b, c] = tri.map(interval_point);
    let [p, q] = seg.map(interval_point);
    let n = icross(isub(b, a), isub(c, a));
    let dp = idot(isub(p, a), n).sign();
    let dq = idot(isub(q, a), n).sign();
    if dp.is_some() && dp == dq {
        return true;
    }
    // The line crosses the triangle iff all three line/edge determinants have
    // one weak sign. Two certified opposite signs prove exclusion without h.
    let d = isub(q, p);
    let sides =
        [(a, b), (b, c), (c, a)].map(|(u, v)| idot(icross(isub(u, p), isub(v, p)), d).sign());
    sides.contains(&Some(true)) && sides.contains(&Some(false))
}
fn collide_seg_triangle(seg: [Point3; 2], tri: [Point3; 3]) -> Option<Point3> {
    if certified_miss(seg, tri) {
        None
    } else {
        exact_collide_seg_triangle(seg, tri)
    }
}

/// Exact dyadic-rational prototype: all decisions use the exact values of the
/// input f64 coordinates. The returned point is rounded only after the hit has
/// been classified. This is deliberately a diagnostic candidate; its cost is
/// not yet suitable for the production all-pairs collision path.
fn exact_collide_seg_triangle(seg: [Point3; 2], tri: [Point3; 3]) -> Option<Point3> {
    if !seg
        .iter()
        .chain(tri.iter())
        .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
    {
        return None;
    }
    use num_traits::{One, Signed, Zero};
    let [a, b, c] = tri.map(exact_rational_point);
    let [p, q] = seg.map(exact_rational_point);
    let n = exact_cross(&exact_sub(&b, &a), &exact_sub(&c, &a));
    if n.iter().all(Zero::is_zero) {
        return None;
    }
    let dp = exact_dot(&exact_sub(&p, &a), &n);
    let dq = exact_dot(&exact_sub(&q, &a), &n);
    if dp.is_zero() && dq.is_zero()
        || dp.is_positive() && dq.is_positive()
        || dp.is_negative() && dq.is_negative()
    {
        return None;
    }
    let denom = dp.clone() - dq.clone();
    if denom.is_zero() {
        return None;
    }
    let t = dp / denom;
    if t < num_rational::BigRational::zero() || t > num_rational::BigRational::one() {
        return None;
    }
    let h = std::array::from_fn(|i| p[i].clone() + t.clone() * (q[i].clone() - p[i].clone()));
    for (u, v) in [(&a, &b), (&b, &c), (&c, &a)] {
        if exact_dot(&exact_cross(&exact_sub(v, u), &exact_sub(&h, u)), &n)
            < num_rational::BigRational::zero()
        {
            return None;
        }
    }
    use num_traits::ToPrimitive;
    let rounded = h.map(|coord| {
        coord
            .to_f64()
            .expect("finite segment intersection coordinate")
    });
    Some(Point3::new(rounded[0], rounded[1], rounded[2]))
}

fn squared_distance_cmp(a: Point3, b: Point3, c: Point3, d: Point3) -> std::cmp::Ordering {
    if (a == c && b == d) || (a == d && b == c) {
        return std::cmp::Ordering::Equal;
    }
    if a == b {
        return if c == d {
            std::cmp::Ordering::Equal
        } else {
            std::cmp::Ordering::Less
        };
    }
    if c == d {
        return std::cmp::Ordering::Greater;
    }
    let ab = isub(interval_point(a), interval_point(b));
    let cd = isub(interval_point(c), interval_point(d));
    let x = idot(ab, ab);
    let y = idot(cd, cd);
    if x.lo > y.hi {
        return std::cmp::Ordering::Greater;
    }
    if x.hi < y.lo {
        return std::cmp::Ordering::Less;
    }
    let ab = exact_sub(&exact_rational_point(a), &exact_rational_point(b));
    let cd = exact_sub(&exact_rational_point(c), &exact_rational_point(d));
    exact_dot(&ab, &ab).cmp(&exact_dot(&cd, &cd))
}

fn collide_triangles(tri0: [Point3; 3], tri1: [Point3; 3]) -> Option<(Point3, Point3)> {
    let hits = [
        collide_seg_triangle([tri0[0], tri0[1]], tri1),
        collide_seg_triangle([tri0[1], tri0[2]], tri1),
        collide_seg_triangle([tri0[2], tri0[0]], tri1),
        collide_seg_triangle([tri1[0], tri1[1]], tri0),
        collide_seg_triangle([tri1[1], tri1[2]], tri0),
        collide_seg_triangle([tri1[2], tri1[0]], tri0),
    ];
    let mut longest = None;
    // Compare squared lengths without overflowing or erasing subnormal spans.
    for (i, a) in hits.iter().enumerate() {
        for b in &hits[i + 1..] {
            if let (Some(a), Some(b)) = (*a, *b) {
                if longest.is_none_or(|(c, d)| squared_distance_cmp(a, b, c, d).is_gt()) {
                    longest = Some((a, b));
                }
            }
        }
    }
    longest
}

fn make_pos_tri(poly: &PolygonMesh, face: [StandardVertex; 3]) -> [Point3; 3] {
    array![i => poly.positions()[face[i].pos]; 3]
}

fn collision(poly0: &PolygonMesh, poly1: &PolygonMesh) -> Vec<(Point3, Point3)> {
    let unit = match poly0.positions().is_empty() {
        true => return Vec::new(),
        false => Vector3::unit_x(),
    };
    let tris0 = poly0.faces().triangle_iter().collect::<Vec<_>>();
    let tris1 = poly1.faces().triangle_iter().collect::<Vec<_>>();
    let iter0 = tris0.iter().map(|face| make_pos_tri(poly0, *face));
    let iter1 = tris1.iter().map(|face| make_pos_tri(poly1, *face));
    colliding_segment_pairs(sorted_endpoints(iter0, iter1, unit))
        .filter_map(|(idx0, idx1)| {
            let tri0 = make_pos_tri(poly0, tris0[idx0]);
            let tri1 = make_pos_tri(poly1, tris1[idx1]);
            match disjoint_bdbs(tri0, tri1) {
                true => None,
                false => collide_triangles(tri0, tri1),
            }
        })
        .collect()
}

fn are_colliding(poly0: &PolygonMesh, poly1: &PolygonMesh) -> Option<(Point3, Point3)> {
    let unit = match poly0.positions().is_empty() {
        true => return None,
        false => Vector3::unit_x(),
    };
    let tris0 = poly0.faces().triangle_iter().collect::<Vec<_>>();
    let tris1 = poly1.faces().triangle_iter().collect::<Vec<_>>();
    let iter0 = tris0.iter().map(|face| make_pos_tri(poly0, *face));
    let iter1 = tris1.iter().map(|face| make_pos_tri(poly1, *face));
    colliding_segment_pairs(sorted_endpoints(iter0, iter1, unit)).find_map(|(idx0, idx1)| {
        let tri0 = make_pos_tri(poly0, tris0[idx0]);
        let tri1 = make_pos_tri(poly1, tris1[idx1]);
        match disjoint_bdbs(tri0, tri1) {
            true => None,
            false => collide_triangles(tri0, tri1),
        }
    })
}

#[test]
fn collide_triangles_test() {
    let tri0 = [
        Point3::origin(),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let tri1 = [
        Point3::new(0.0, 0.0, -1.0),
        Point3::new(-1.0, -1.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
    ];
    assert!(collide_triangles(tri0, tri1).is_some());
    let tri0 = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ];
    let tri1 = [
        Point3::new(0.0, 0.0, 0.5),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(1.0, 1.0, 1.0),
    ];
    assert!(collide_triangles(tri0, tri1).is_none());
}

#[cfg(test)]
mod scale_diagnosis_tests {
    use super::*;

    fn crossing_pair(scale: f64) -> (PolygonMesh, PolygonMesh, [[Point3; 3]; 2]) {
        let tri0 = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(scale, 0.0, 0.0),
            Point3::new(0.0, scale, 0.0),
        ];
        let tri1 = [
            Point3::new(0.0, 0.0, -scale),
            Point3::new(-scale, -scale, scale),
            Point3::new(scale, scale, scale),
        ];
        let mesh = |tri: [Point3; 3]| {
            PolygonMesh::new(
                StandardAttributes {
                    positions: tri.to_vec(),
                    ..Default::default()
                },
                Faces::from_iter(&[[0, 1, 2]]),
            )
        };
        (mesh(tri0), mesh(tri1), [tri0, tri1])
    }

    #[test]
    fn scale_probe_confirms_sorted_endpoint_area_cutoff() {
        for scale in [
            1.0, 1.0e-2, 2.0e-3, 1.0e-3, 5.0e-4, 1.0e-4, 1.0e-100, 1.0e-150,
        ] {
            let (a, b, triangles) = crossing_pair(scale);
            let normal0 =
                (triangles[0][1] - triangles[0][0]).cross(triangles[0][2] - triangles[0][0]);
            let normal1 =
                (triangles[1][1] - triangles[1][0]).cross(triangles[1][2] - triangles[1][0]);
            let expected = (Point3::origin(), Point3::new(scale * 0.5, scale * 0.5, 0.0));
            let expected_length = scale / 2.0_f64.sqrt();
            let endpoints = sorted_endpoints([triangles[0]], [triangles[1]], Vector3::unit_z());
            let direct = collide_triangles(triangles[0], triangles[1]);
            let extracted = a.extract_interference(&b);
            eprintln!(
                "scale={scale:.9e} normal0={normal0:?} normal0_small={} normal1={normal1:?} normal1_small={} analytic_crossing={expected:?} analytic_length={expected_length:.9e} sorted_endpoints={} direct_triangle_hit={direct:?} extracted_segments={} extracted={extracted:?}",
                normal0.so_small(),
                normal1.so_small(),
                endpoints.len(),
                extracted.len(),
            );
            // Analytically, both triangles contain the segment from the origin to
            // (scale/2, scale/2, 0): tri0 lies in z=0, and tri1 lies in x=y.
            assert!(expected_length > 0.0);
            assert_eq!(extracted.len(), 1);
            let (a, b) = extracted[0];
            assert!(a.distance(expected.0) < scale * 1.0e-9);
            assert!(b.distance(expected.1) < scale * 1.0e-9);
            assert_eq!(endpoints.len(), 4);
            assert!(direct.is_some());
        }
    }

    #[test]
    fn translated_and_representability_controls() {
        let offset = Vector3::new(1.0e9, -1.0e9, 1.0e9);
        let shift = |tri: [Point3; 3]| tri.map(|p| p + offset);
        let a = shift([
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ]);
        let b = shift([
            Point3::new(0.0, 0.0, -1.0),
            Point3::new(-1.0, -1.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
        ]);
        let hit = collide_triangles(a, b).expect("translated ordinary crossing");
        assert!(hit.0.distance(Point3::origin() + offset) < 1.0e-6);
        assert!(
            hit.1
                .distance(Point3::origin() + offset + Vector3::new(0.5, 0.5, 0.0))
                < 1.0e-6
        );

        // At this translation, unit edges cannot be represented (ULP > 1), so
        // the inputs collapse before the predicate sees them. Refuse them as
        // degenerate rather than claiming a recovered crossing.
        let huge = Point3::new(1.0e16, 1.0e16, 1.0e16);
        let collapsed = [huge, huge + Vector3::unit_x(), huge + Vector3::unit_y()];
        assert_eq!(collapsed[0], collapsed[1]);
        assert!(!triangle_has_resolvable_area(collapsed));
    }

    #[test]
    fn robust_predicates_preserve_valid_skinny_triangle_crossing() {
        let crossing = [
            Point3::new(0.25, 0.0, -1.0),
            Point3::new(0.75, 0.0, 1.0),
            Point3::new(0.75, 0.0, -1.0),
        ];
        // Analytically the triangles overlap along x=[0.5,0.75], y=z=0.
        // Exercise aspect ratios down to subnormal-scale heights: no geometric
        // area cutoff is allowed to discard these representable triangles.
        for height in [1.0e-15, 1.0e-100, 1.0e-300] {
            let skinny = [
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, height, 0.0),
            ];
            assert!(triangle_has_resolvable_area(skinny), "height={height:e}");
            let hit = collide_triangles(skinny, crossing)
                .unwrap_or_else(|| panic!("height={height:e}: missing analytic crossing"));
            assert!((hit.0.distance(hit.1) - 0.25).abs() < 1.0e-12);
        }
    }

    #[test]
    fn exact_degenerate_and_nonfinite_triangles_fail_closed() {
        let collinear = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(2.0, 2.0, 2.0),
        ];
        assert!(!triangle_has_resolvable_area(collinear));
        let nonfinite = [
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, f64::NAN, 0.0),
        ];
        assert!(!triangle_has_resolvable_area(nonfinite));
    }

    #[test]
    fn disjoint_parallel_and_tangent_controls() {
        let base = [
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let parallel = base.map(|p| p + Vector3::unit_z());
        assert!(collide_triangles(base, parallel).is_none());
        let disjoint = [
            Point3::new(2.0, 0.0, -1.0),
            Point3::new(2.0, 1.0, 1.0),
            Point3::new(2.0, -1.0, 1.0),
        ];
        assert!(collide_triangles(base, disjoint).is_none());
        let tangent = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, -1.0, 1.0),
            Point3::new(2.0, 1.0, 1.0),
        ];
        let hit = collide_triangles(base, tangent);
        assert!(hit.map(|(p, q)| p.distance(q) < 1.0e-12).unwrap_or(true));
        for separation in [1.0e-9, 1.0e-14] {
            let crossing = [
                Point3::new(0.25, 0.25, separation),
                Point3::new(0.25, 0.25, -separation),
            ];
            let hit = collide_seg_triangle(crossing, base).expect("near-coplanar crossing");
            assert!(hit.distance(Point3::new(0.25, 0.25, 0.0)) <= separation);
            let parallel = [
                Point3::new(0.25, 0.25, separation),
                Point3::new(0.75, 0.25, separation),
            ];
            assert!(collide_seg_triangle(parallel, base).is_none());
        }
    }
}

#[cfg(test)]
mod exact_oracle_tests {
    use super::*;
    use num_rational::BigRational;
    use num_traits::{One, Signed, Zero};

    type Q = BigRational;
    type QVec = [Q; 3];

    fn exact_point(p: Point3) -> QVec {
        [
            Q::from_float(p.x).expect("finite x"),
            Q::from_float(p.y).expect("finite y"),
            Q::from_float(p.z).expect("finite z"),
        ]
    }

    fn qsub(a: &QVec, b: &QVec) -> QVec {
        [
            a[0].clone() - b[0].clone(),
            a[1].clone() - b[1].clone(),
            a[2].clone() - b[2].clone(),
        ]
    }

    fn qcross(a: &QVec, b: &QVec) -> QVec {
        [
            a[1].clone() * b[2].clone() - a[2].clone() * b[1].clone(),
            a[2].clone() * b[0].clone() - a[0].clone() * b[2].clone(),
            a[0].clone() * b[1].clone() - a[1].clone() * b[0].clone(),
        ]
    }

    fn qdot(a: &QVec, b: &QVec) -> Q {
        a[0].clone() * b[0].clone() + a[1].clone() * b[1].clone() + a[2].clone() * b[2].clone()
    }

    /// Exact segment/triangle point classification over the exact dyadic
    /// rationals represented by finite f64 inputs. Coplanar segments are
    /// intentionally `None`: the production API only extracts unique points.
    fn exact_segment_triangle(seg: [Point3; 2], tri: [Point3; 3]) -> Option<QVec> {
        let a = exact_point(tri[0]);
        let b = exact_point(tri[1]);
        let c = exact_point(tri[2]);
        let p = exact_point(seg[0]);
        let q = exact_point(seg[1]);
        let n = qcross(&qsub(&b, &a), &qsub(&c, &a));
        if n.iter().all(Zero::is_zero) {
            return None;
        }
        let dp = qdot(&qsub(&p, &a), &n);
        let dq = qdot(&qsub(&q, &a), &n);
        if dp.is_zero() && dq.is_zero()
            || dp.is_positive() && dq.is_positive()
            || dp.is_negative() && dq.is_negative()
        {
            return None;
        }
        let t = dp.clone() / (dp.clone() - dq.clone());
        if t < Q::zero() || t > Q::one() {
            return None;
        }
        let h = [
            p[0].clone() + t.clone() * (q[0].clone() - p[0].clone()),
            p[1].clone() + t.clone() * (q[1].clone() - p[1].clone()),
            p[2].clone() + t * (q[2].clone() - p[2].clone()),
        ];
        for (u, v) in [(&a, &b), (&b, &c), (&c, &a)] {
            let side = qdot(&qcross(&qsub(v, u), &qsub(&h, u)), &n);
            if side < Q::zero() {
                return None;
            }
        }
        Some(h)
    }

    fn raw_robust_probe(seg: [Point3; 2], tri: [Point3; 3]) -> Option<Point3> {
        let dp = orient3d(
            orient3(tri[0]),
            orient3(tri[1]),
            orient3(tri[2]),
            orient3(seg[0]),
        );
        let dq = orient3d(
            orient3(tri[0]),
            orient3(tri[1]),
            orient3(tri[2]),
            orient3(seg[1]),
        );
        if !dp.is_finite() || !dq.is_finite() || (dp > 0.0 && dq > 0.0) || (dp < 0.0 && dq < 0.0) {
            return None;
        }
        let denom = dp - dq;
        if denom == 0.0 || !denom.is_finite() {
            return None;
        }
        let t = dp / denom;
        if !(0.0..=1.0).contains(&t) {
            return None;
        }
        let h = seg[0] + t * (seg[1] - seg[0]);
        let e1 = tri[1] - tri[0];
        let e2 = tri[2] - tri[0];
        let normal = e1.cross(e2);
        let axis = if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
            0
        } else if normal.y.abs() >= normal.z.abs() {
            1
        } else {
            2
        };
        let winding = orient2(tri[0], tri[1], tri[2], axis);
        let inside = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])]
            .into_iter()
            .all(|(u, v)| {
                let side = orient2(u, v, h, axis);
                side == 0.0
                    || side.is_finite() && side.is_sign_positive() == winding.is_sign_positive()
            });
        inside.then_some(h)
    }

    fn compare_case(name: &str, seg: [Point3; 2], tri: [Point3; 3], expected_hit: bool) {
        let exact = exact_segment_triangle(seg, tri);
        assert_eq!(exact.is_some(), expected_hit, "oracle setup: {name}");
        let actual = collide_seg_triangle(seg, tri);
        assert_eq!(actual.is_some(), expected_hit, "candidate: {name}");
        let raw = raw_robust_probe(seg, tri);
        eprintln!(
            "case={name:?} exact_hit={} exact_candidate_hit={} raw_robust_hit={}",
            exact.is_some(),
            actual.is_some(),
            raw.is_some()
        );
    }

    #[test]
    fn dyadic_oracle_controls_oblique_edge_vertex_and_reverse_order() {
        let tri = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 1.0),
            Point3::new(0.0, 2.0, 1.0),
        ];
        let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        let shared_edge_hit = Point3::new(1.0, 0.0, 0.5);
        let edge_seg = [shared_edge_hit + normal, shared_edge_hit - normal];
        compare_case("oblique shared edge", edge_seg, tri, true);
        compare_case(
            "reversed oblique shared edge",
            edge_seg,
            [tri[0], tri[2], tri[1]],
            true,
        );

        let vertex_seg = [tri[0] + normal, tri[0] - normal];
        compare_case("oblique vertex touch", vertex_seg, tri, true);
        let outside = f64::from_bits(1.0_f64.to_bits());
        let outside_seg = [
            Point3::new(-outside, 0.0, -1.0),
            Point3::new(-outside, 0.0, 1.0),
        ];
        compare_case("outside parallel-edge line", outside_seg, tri, false);
    }

    #[test]
    fn dyadic_oracle_tracks_disparate_exponents_and_translation() {
        let tri = [
            Point3::new(1.0e12, -1.0e12, 0.0),
            Point3::new(1.0e12 + 1.0, -1.0e12, 0.0),
            Point3::new(1.0e12, -1.0e12 + 1.0e-3, 0.0),
        ];
        let seg = [
            Point3::new(1.0e12 + 0.125, -1.0e12 + 1.0e-4, -1.0),
            Point3::new(1.0e12 + 0.125, -1.0e12 + 1.0e-4, 1.0),
        ];
        compare_case("translated disparate-scale triangle", seg, tri, true);

        let scale = 1.0e-100;
        let tri = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(scale, 0.0, 0.0),
            Point3::new(0.0, scale, 0.0),
        ];
        let seg = [
            Point3::new(scale / 4.0, scale / 4.0, -scale),
            Point3::new(scale / 4.0, scale / 4.0, scale),
        ];
        compare_case("uniform tiny scale", seg, tri, true);
    }

    #[test]
    fn dyadic_oracle_exposes_overflow_span_false_negative() {
        let tri = [
            Point3::new(-1.0e308, 0.0, 0.0),
            Point3::new(0.0, 1.0e308, 0.0),
            Point3::new(1.0e308, 0.0, 0.0),
        ];
        let seg = [
            Point3::new(0.0, 2.5e307, -1.0e308),
            Point3::new(0.0, 2.5e307, 1.0e308),
        ];
        assert!(exact_segment_triangle(seg, tri).is_some());
        eprintln!(
            "overflow_case exact_hit=true exact_candidate_hit={} raw_robust_hit={}",
            collide_seg_triangle(seg, tri).is_some(),
            raw_robust_probe(seg, tri).is_some()
        );
        // The exact-rational prototype handles this case without overflowing
        // intermediate f64 differences. The raw adaptive predicate alone does not.
        assert!(collide_seg_triangle(seg, tri).is_some());
    }
}

#[cfg(test)]
mod filtered_tests {
    use super::*;
    #[test]
    fn broadphase_preserves_finite_extreme_triangles() {
        for tri in [
            [
                Point3::new(-1e308, 0., 0.),
                Point3::new(1e308, 0., 0.),
                Point3::new(0., 1e-300, 0.),
            ],
            [
                Point3::origin(),
                Point3::new(1e300, 0., 0.),
                Point3::new(1e300, 1e-300, 0.),
            ],
        ] {
            assert!(triangle_has_resolvable_area(tri));
            let endpoints = sorted_endpoints([tri], [tri], Vector3::new(1., 1., 1.));
            assert_eq!(endpoints.len(), 4);
            assert!(endpoints.iter().all(|e| e.entity.is_finite()));
        }
    }
}

#[cfg(test)]
mod qualification_tests {
    use super::*;
    fn measure(name: &str, cases: &[([Point3; 2], [Point3; 3])]) {
        let start = std::time::Instant::now();
        let expected: Vec<_> = cases
            .iter()
            .map(|&(s, t)| exact_collide_seg_triangle(s, t))
            .collect();
        let exact = start.elapsed();
        let start = std::time::Instant::now();
        let actual: Vec<_> = cases
            .iter()
            .map(|&(s, t)| collide_seg_triangle(s, t))
            .collect();
        let filtered = start.elapsed();
        assert_eq!(actual, expected, "{name}");
        let fallbacks = cases
            .iter()
            .filter(|&&(s, t)| !certified_miss(s, t))
            .count();
        let hits = actual.iter().filter(|h| h.is_some()).count();
        eprintln!(
            "MEASURE {name}: cases={} hits={hits} fallbacks={fallbacks} exact_ms={:.3} filtered_ms={:.3} speedup={:.3}",
            cases.len(),
            exact.as_secs_f64() * 1000.,
            filtered.as_secs_f64() * 1000.,
            exact.as_secs_f64() / filtered.as_secs_f64()
        );
    }
    #[test]
    fn differential_mesh_and_extreme_corpus() {
        let text = include_str!("../../tests/fixtures/collision-board-sample.txt");
        let tris: Vec<[Point3; 3]> = text
            .lines()
            .map(|l| {
                let v: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
                std::array::from_fn(|i| Point3::new(v[3 * i], v[3 * i + 1], v[3 * i + 2]))
            })
            .collect();
        let mut pairs = Vec::new();
        for (i, t) in tris.iter().enumerate() {
            for j in 0..16 {
                let u = tris[(i * 17 + j * 7) % tris.len()]
                    .map(|p| p + Vector3::new(0.125, -0.25, 0.5));
                pairs.push(([u[0], u[1]], *t));
            }
        }
        measure("sampled-board-mesh", &pairs);
        let mut state = 0x9e3779b97f4a7c15u64;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64 * 4. - 2.
        };
        let mut ordinary = Vec::new();
        for _ in 0..2048 {
            let points: [Point3; 5] =
                std::array::from_fn(|_| Point3::new(random(), random(), random()));
            ordinary.push(([points[3], points[4]], [points[0], points[1], points[2]]));
        }
        measure("random-local-cells", &ordinary);
        let tri = [
            Point3::origin(),
            Point3::new(2., 0., 1.),
            Point3::new(0., 2., 1.),
        ];
        let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
        let mut contact = Vec::new();
        for i in 0..512 {
            let h = Point3::new((i % 128) as f64 / 128., 0., (i % 128) as f64 / 256.);
            contact.push(([h + n, h - n], tri));
        }
        measure("oblique-boundary-contact", &contact);
        let mut extreme = Vec::new();
        for exp in [-1070, -1000, -600, -300, 0, 300, 600, 1000] {
            let scale = 2f64.powi(exp);
            for &(s, t) in ordinary.iter().take(128) {
                extreme.push((
                    s.map(|p| Point3::from_vec(p.to_vec() * scale)),
                    t.map(|p| Point3::from_vec(p.to_vec() * scale)),
                ));
            }
        }
        measure("wide-exponents", &extreme);
    }
}

#[cfg(test)]
mod mesh_bench {
    use super::*;
    fn exact_triangles(tri0: [Point3; 3], tri1: [Point3; 3]) -> Option<(Point3, Point3)> {
        let mut tuple = (None, None);
        [
            exact_collide_seg_triangle([tri0[0], tri0[1]], tri1),
            exact_collide_seg_triangle([tri0[1], tri0[2]], tri1),
            exact_collide_seg_triangle([tri0[2], tri0[0]], tri1),
            exact_collide_seg_triangle([tri1[0], tri1[1]], tri0),
            exact_collide_seg_triangle([tri1[1], tri1[2]], tri0),
            exact_collide_seg_triangle([tri1[2], tri1[0]], tri0),
        ]
        .into_iter()
        .for_each(|pt| match tuple {
            (None, _) => tuple.0 = pt,
            (Some(_), None) => tuple.1 = pt,
            (Some(ref mut p), Some(ref mut q)) => {
                if let Some(pt) = pt {
                    let dist0 = pt.distance2(*p);
                    let dist1 = pt.distance2(*q);
                    let dist2 = p.distance2(*q);
                    if dist2 < dist0 {
                        *q = pt;
                    } else if dist2 < dist1 {
                        *p = pt;
                    }
                }
            }
        });
        match tuple {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    #[test]
    fn small_mesh_end_to_end() {
        let text = include_str!("../../tests/fixtures/collision-board-sample.txt");
        let tris: Vec<[Point3; 3]> = text
            .lines()
            .map(|l| {
                let v: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
                std::array::from_fn(|i| Point3::new(v[3 * i], v[3 * i + 1], v[3 * i + 2]))
            })
            .collect();
        let mesh = |ts: &[[Point3; 3]]| {
            PolygonMesh::new(
                StandardAttributes {
                    positions: ts.iter().flatten().copied().collect(),
                    ..Default::default()
                },
                Faces::from_iter((0..ts.len()).map(|i| [3 * i, 3 * i + 1, 3 * i + 2])),
            )
        };
        for shift in [Vector3::new(0.125, -0.25, 0.5), Vector3::new(0., 0., 0.)] {
            let moved: Vec<_> = tris.iter().map(|t| t.map(|p| p + shift)).collect();
            let a = mesh(&tris);
            let b = mesh(&moved);
            let start = std::time::Instant::now();
            let pairs: Vec<_> = colliding_segment_pairs(sorted_endpoints(
                tris.iter().copied(),
                moved.iter().copied(),
                Vector3::unit_x(),
            ))
            .filter(|&(i, j)| !disjoint_bdbs(tris[i], moved[j]))
            .collect();
            let expected: Vec<_> = pairs
                .iter()
                .filter_map(|&(i, j)| exact_triangles(tris[i], moved[j]))
                .collect();
            let exact = start.elapsed();
            let start = std::time::Instant::now();
            let actual = a.extract_interference(&b);
            let filtered = start.elapsed();
            assert_eq!(actual, expected);
            eprintln!(
                "MESH shift={shift:?} triangles=64+64 candidate_pairs={} segments={} exact_ms={:.3} filtered_ms={:.3} speedup={:.3}",
                pairs.len(),
                actual.len(),
                exact.as_secs_f64() * 1000.,
                filtered.as_secs_f64() * 1000.,
                exact.as_secs_f64() / filtered.as_secs_f64()
            );
        }
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    #[test]
    fn extreme_crossing_endpoints_are_order_independent() {
        for scale in [1., 1e-200, 1e-300, 1e200, 1e308] {
            let a = [
                Point3::new(scale, 0., 0.),
                Point3::origin(),
                Point3::new(0., scale, 0.),
            ];
            let b = [
                Point3::new(0., 0., -scale),
                Point3::new(-scale, -scale, scale),
                Point3::new(scale, scale, scale),
            ];
            let expected = [Point3::origin(), Point3::new(scale * 0.5, scale * 0.5, 0.)];
            for order in [
                [0, 1, 2],
                [0, 2, 1],
                [1, 0, 2],
                [1, 2, 0],
                [2, 0, 1],
                [2, 1, 0],
            ] {
                for swapped in [false, true] {
                    let aa = order.map(|i| a[i]);
                    let (p, q) = if swapped {
                        collide_triangles(b, aa)
                    } else {
                        collide_triangles(aa, b)
                    }
                    .expect("crossing");
                    let mesh = |tri: [Point3; 3]| {
                        PolygonMesh::new(
                            StandardAttributes {
                                positions: tri.to_vec(),
                                ..Default::default()
                            },
                            Faces::from_iter(&[[0, 1, 2]]),
                        )
                    };
                    let (m0, m1) = if swapped {
                        (mesh(b), mesh(aa))
                    } else {
                        (mesh(aa), mesh(b))
                    };
                    assert_eq!(m0.collide_with(&m1), Some((p, q)));
                    assert_eq!(m0.extract_interference(&m1), vec![(p, q)]);
                    assert!(
                        (p == expected[0] && q == expected[1])
                            || (q == expected[0] && p == expected[1]),
                        "scale={scale} order={order:?} swapped={swapped} got {p:?},{q:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn squared_distance_order_survives_extreme_nonzero_lengths() {
    use std::cmp::Ordering;
    let p = |x| Point3::new(x, 0., 0.);
    for scale in [1e-300, 1., 1e300] {
        assert_eq!(
            squared_distance_cmp(p(0.), p(scale), p(0.), p(2. * scale)),
            Ordering::Less
        );
        assert_eq!(
            squared_distance_cmp(p(0.), p(2. * scale), p(0.), p(scale)),
            Ordering::Greater
        );
    }
    assert_eq!(
        squared_distance_cmp(p(0.), p(1e308), p(-1e308), p(1e308)),
        Ordering::Less
    );
}

#[cfg(test)]
mod sweep_axis_tests {
    use super::*;

    fn triangle(x: f64, y: f64, z: f64) -> [Point3; 3] {
        [
            Point3::new(x, y, z),
            Point3::new(x + 1.0, y, z),
            Point3::new(x, y + 1.0, z),
        ]
    }

    #[test]
    fn sparse_meshes_choose_a_separating_coordinate_axis() {
        let a = [triangle(0.0, 0.0, 0.0), triangle(0.0, 2.0, 0.0)];
        for offset in [Vector3::new(0.0, 10.0, 0.0), Vector3::new(0.0, 0.0, 10.0)] {
            let b = a.map(|tri| tri.map(|point| point + offset));
            // Both x projections overlap, but a different coordinate separates the meshes.
            let pairs = colliding_segment_pairs(sorted_endpoints(a, b, Vector3::unit_x())).count();
            assert_eq!(
                pairs, 0,
                "separated meshes should need no triangle-pair checks"
            );
        }
    }

    #[test]
    fn selected_sweep_keeps_endpoint_contacts_and_zero_width_intervals() {
        let a = [triangle(0.0, 0.0, 0.0)];
        let b = [triangle(1.0, 0.0, 0.0)];
        for (left, right) in [(a, b), (b, a)] {
            let pairs: Vec<_> =
                colliding_segment_pairs(sorted_endpoints(left, right, Vector3::unit_x())).collect();
            assert_eq!(
                pairs,
                vec![(0, 0)],
                "closed intervals touching at x=1 must overlap"
            );
        }
        let yz = [
            Point3::origin(),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let mut pairs: Vec<_> =
            colliding_segment_pairs(sorted_endpoints([yz, yz], [yz, yz], Vector3::unit_x()))
                .collect();
        pairs.sort_unstable();
        assert_eq!(
            pairs,
            vec![(0, 0), (0, 1), (1, 0), (1, 1)],
            "zero-width projections and equal endpoints preserve every pair"
        );
    }
}
