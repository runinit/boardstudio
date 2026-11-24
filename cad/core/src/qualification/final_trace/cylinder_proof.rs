//! Private whole-surface proof for axis-aligned cylindrical extrusions and caps.
//!
//! See PROOF.md beside this file. All predicates and error accounting use exact
//! rationals of the retained f64 geometry. No mesher tolerance is a proof input.
use super::super::generated_qualification::GeneratedQualification;
use crate::layered::brep::Point3;
use crate::vec3::q::{cross, sub};
use num_rational::BigRational as Q;
use num_traits::{Signed, ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet};

type P = [Q; 3];
type Edge = (P, P);
type Proof<T> = Result<T, &'static str>;
fn n(v: i64) -> Q {
    Q::from_integer(v.into())
}
fn q(v: f64) -> Proof<Q> {
    if !v.is_finite() || v.abs() > 1e12 || (v != 0. && v.abs() < 1e-100) {
        return Err("numeric domain limit");
    }
    Q::from_float(v).ok_or("nonfinite source/final coordinate")
}
fn point(p: Point3) -> Proof<P> {
    Ok([q(p.x)?, q(p.y)?, q(p.z)?])
}
fn det(a: &P, b: &P) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}
fn dot2(a: &P, b: &P) -> Q {
    &a[0] * &b[0] + &a[1] * &b[1]
}
fn centered(p: &P, c: &[Q; 2]) -> P {
    [&p[0] - &c[0], &p[1] - &c[1], n(0)]
}
fn edges(ring: &[P]) -> impl Iterator<Item = (&P, &P)> {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
}
fn edge_key(a: &P, b: &P) -> Edge {
    if a < b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}
fn edge_set(ring: &[P]) -> BTreeSet<Edge> {
    edges(ring).map(|(a, b)| edge_key(a, b)).collect()
}

// Oriented triangle chains, not an area comparison: internal edges cancel once;
// remaining degree-two cycles are the complete boundary, including every hole.
fn boundary(triangles: &[[P; 3]]) -> Proof<Vec<Vec<P>>> {
    let mut uses: BTreeMap<Edge, (usize, i32)> = BTreeMap::new();
    for tri in triangles {
        for (a, b) in edges(tri) {
            if a == b {
                return Err("zero edge");
            }
            let entry = uses.entry(edge_key(a, b)).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for ((a, b), (count, sign)) in uses {
        match (count, sign) {
            (2, 0) => (),
            (1, 1) | (1, -1) => {
                let (a, b) = if sign == 1 { (a, b) } else { (b, a) };
                if outgoing.insert(a, b.clone()).is_some() || !incoming.insert(b) {
                    return Err("branched boundary");
                }
            }
            _ => return Err("duplicate or inconsistent triangle chain"),
        }
    }
    if outgoing.keys().cloned().collect::<BTreeSet<_>>() != incoming {
        return Err("open boundary");
    }
    let mut rings = Vec::new();
    while let Some(start) = outgoing.keys().next().cloned() {
        let mut ring = vec![start.clone()];
        let mut at = outgoing.remove(&start).ok_or("boundary start")?;
        while at != start {
            let next = outgoing.remove(&at).ok_or("boundary cycle")?;
            ring.push(at);
            at = next;
        }
        if ring.len() < 3 {
            return Err("sliver boundary");
        }
        rings.push(ring);
    }
    if rings.is_empty() {
        return Err("missing face boundary");
    }
    Ok(rings)
}

// A single angular turn is authenticated without atan2, pi, normalization or UV
// modulo arithmetic. Positive minor steps and winding one prohibit retracing.
fn turn(ring: &[P], center: &[Q; 2]) -> Proof<i32> {
    let radial = ring.iter().map(|p| centered(p, center)).collect::<Vec<_>>();
    let mut winding = 0;
    let mut sign = 0;
    for (a, b) in edges(&radial) {
        let d = det(a, b);
        if d.is_zero() || dot2(a, b) < n(0) {
            return Err("unproven angular seam/span");
        }
        let s = if d > n(0) { 1 } else { -1 };
        if sign != 0 && sign != s {
            return Err("angular retrace");
        }
        sign = s;
        if a[1] <= n(0) && b[1] > n(0) && d > n(0) {
            winding += 1;
        }
        if b[1] <= n(0) && a[1] > n(0) && d < n(0) {
            winding -= 1;
        }
    }
    if winding != sign {
        return Err("incomplete angular turn");
    }
    Ok(sign)
}
fn segment_radius2(a: &P, b: &P) -> Q {
    let d = sub(b, a);
    let length = dot2(&d, &d);
    if length.is_zero() {
        return dot2(a, a);
    }
    let t = (-dot2(a, &d) / length).max(n(0)).min(n(1));
    let p = std::array::from_fn(|i| &a[i] + &t * &d[i]);
    dot2(&p, &p)
}
fn triangle_radial_error(tri: &[P; 3], center: &[Q; 2], radius: &Q) -> Proof<Q> {
    let p = tri.each_ref().map(|v| centered(v, center));
    let signs = edges(&p).map(|(a, b)| det(a, b)).collect::<Vec<_>>();
    if signs.iter().all(|v| v >= &n(0)) || signs.iter().all(|v| v <= &n(0)) {
        return Err("triangle reaches cylinder axis");
    }
    let lower = edges(&p)
        .map(|(a, b)| segment_radius2(a, b))
        .min()
        .ok_or("no radial edges")?;
    let upper = p
        .iter()
        .map(|v| dot2(v, v))
        .max()
        .ok_or("no radial vertices")?;
    let r2 = radius * radius;
    // |sqrt(s)-r| = |s-r^2|/(sqrt(s)+r) <= |s-r^2|/r.
    Ok((&lower - &r2).abs().max((&upper - &r2).abs()) / radius)
}

#[derive(Clone)]
struct Circle<Id> {
    center: [Q; 2],
    radius: Q,
    z: [Q; 2],
    source_error: Q,
    source_edges: Vec<Id>,
    source_levels: Vec<Q>,
    mesh_rings: Vec<Vec<P>>,
    mesh_levels: Vec<Q>,
    mesh_height: Q,
}

fn cylinder_mesh<Id>(
    circle: &mut Circle<Id>,
    tris: &[[P; 3]],
    expected_orientation: Option<i32>,
) -> Proof<Q> {
    circle.mesh_rings = boundary(tris)?;
    if circle.mesh_rings.len() != 2 {
        return Err("cylinder seam or missing patch");
    }
    let gap = &circle.z[1] - &circle.z[0];
    let mut levels = BTreeSet::new();
    for ring in &circle.mesh_rings {
        let midpoint = (&circle.z[0] + &circle.z[1]) / n(2);
        let level = if ring.iter().all(|p| p[2] < midpoint) {
            &circle.z[0]
        } else if ring.iter().all(|p| p[2] > midpoint) {
            &circle.z[1]
        } else {
            return Err("final cylinder height graphs are not separated");
        };
        if !levels.insert(level.clone()) {
            return Err("incomplete cylinder end rings");
        }
        let height = ring.iter().map(|p| (&p[2] - level).abs()).max().unwrap();
        if n(2) * &height >= gap {
            return Err("final cylinder height enclosure exceeds half interval");
        }
        circle.mesh_height = circle.mesh_height.clone().max(height);
        circle.mesh_levels.push(level.clone());
        let sign = turn(ring, &circle.center)?;
        if let Some(outward) = expected_orientation
            && sign
                != if level == &circle.z[0] {
                    outward
                } else {
                    -outward
                }
        {
            return Err("final cylinder end orientation");
        }
    }
    let mut error = n(0);
    let mut orientation = 0;
    for tri in tris {
        if tri.iter().any(|p| {
            p[2] < &circle.z[0] - &circle.mesh_height || p[2] > &circle.z[1] + &circle.mesh_height
        }) {
            return Err("cylinder height overshoot");
        }
        let normal = cross(&sub(&tri[1], &tri[0]), &sub(&tri[2], &tri[0]));
        for p in tri {
            let j =
                &normal[0] * (&p[0] - &circle.center[0]) + &normal[1] * (&p[1] - &circle.center[1]);
            if j.is_zero() {
                return Err("singular radial projection");
            }
            let sign = if j > n(0) { 1 } else { -1 };
            if orientation != 0 && orientation != sign {
                return Err("folded radial projection");
            }
            orientation = sign;
        }
        error = error.max(triangle_radial_error(tri, &circle.center, &circle.radius)?);
    }
    // The two boundary cycles must induce opposite cylinder orientations.
    if turn(&circle.mesh_rings[0], &circle.center)? == turn(&circle.mesh_rings[1], &circle.center)?
    {
        return Err("cylinder boundary orientation");
    }
    if expected_orientation.is_some_and(|expected| expected != orientation) {
        return Err("final cylinder orientation");
    }
    Ok(error)
}

fn axis_frame(origin: &P, normal: P) -> Proof<(usize, Q, P)> {
    let nonzero = (0..3).filter(|&i| !normal[i].is_zero()).collect::<Vec<_>>();
    if nonzero.len() != 1 {
        return Err("unsupported or singular plane support");
    }
    let axis = nonzero[0];
    Ok((axis, origin[axis].clone(), normal))
}

fn rectangle(points: Vec<P>, axis: usize, level: &Q) -> Proof<(Vec<P>, Q)> {
    if !(4..=128).contains(&points.len()) {
        return Err("unsupported noncircular trim");
    }
    // A retained line's normal coordinate is affine. Its endpoint maximum
    // encloses the entire source-to-plane discrepancy, including placement.
    let height = points
        .iter()
        .map(|p| (&p[axis] - level).abs())
        .max()
        .unwrap();
    let mut points = points
        .iter()
        .map(|p| project(p, axis, level))
        .collect::<Vec<_>>();
    loop {
        let count = points.len();
        let removable = (0..count).find(|&i| {
            let a = &points[(i + count - 1) % count];
            let b = &points[i];
            let c = &points[(i + 1) % count];
            cross(&sub(b, a), &sub(c, b)).iter().all(Q::is_zero)
                && (0..3).all(|k| {
                    b[k] >= a[k].clone().min(c[k].clone()) && b[k] <= a[k].clone().max(c[k].clone())
                })
        });
        if count <= 4 || removable.is_none() {
            break;
        }
        points.remove(removable.unwrap());
    }
    if points.len() != 4 {
        return Err("unsupported subdivided rectangle trim");
    }
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    let coords = axes
        .iter()
        .map(|&i| points.iter().map(|p| p[i].clone()).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();
    if coords.iter().any(|s| s.len() != 2) || points.iter().collect::<BTreeSet<_>>().len() != 4 {
        return Err("nonrectangular source trim");
    }
    for (a, b) in edges(&points) {
        if (0..3).filter(|&i| a[i] != b[i]).count() != 1 {
            return Err("diagonal rectangle trim");
        }
    }
    Ok((points, height))
}
fn rectangle_matches(source: &[P], actual: &[P]) -> Proof<()> {
    let mut intervals = vec![Vec::<(Q, Q)>::new(); 4];
    for (a, b) in edges(actual) {
        let mut matched = false;
        for (i, (s, e)) in edges(source).enumerate() {
            let axis = (0..3)
                .find(|&k| s[k] != e[k])
                .ok_or("rectangle zero side")?;
            if (0..3).any(|k| k != axis && (a[k] != s[k] || b[k] != s[k])) {
                continue;
            }
            let ta = (&a[axis] - &s[axis]) / (&e[axis] - &s[axis]);
            let tb = (&b[axis] - &s[axis]) / (&e[axis] - &s[axis]);
            if ta == tb || ta < n(0) || tb < n(0) || ta > n(1) || tb > n(1) {
                continue;
            }
            intervals[i].push((ta.clone().min(tb.clone()), ta.max(tb)));
            matched = true;
            break;
        }
        if !matched {
            return Err("rectangle boundary mismatch");
        }
    }
    for mut list in intervals {
        list.sort();
        let mut at = n(0);
        for (lo, hi) in list {
            if lo != at {
                return Err("rectangle gap/duplicate");
            }
            at = hi;
        }
        if at != n(1) {
            return Err("missing rectangle side");
        }
    }
    Ok(())
}
fn ring_area(ring: &[P], axis: usize) -> Q {
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    edges(ring)
        .map(|(a, b)| &a[axes[0]] * &b[axes[1]] - &a[axes[1]] * &b[axes[0]])
        .sum()
}

enum CapRing {
    Rectangle(Vec<P>),
    Circular(usize),
}
fn project(p: &P, axis: usize, level: &Q) -> P {
    let mut out = p.clone();
    out[axis] = level.clone();
    out
}

fn circle_end<Id: PartialEq>(circles: &[Circle<Id>], ids: &Id) -> Option<(usize, usize)> {
    circles.iter().enumerate().find_map(|(circle, c)| {
        c.source_edges
            .iter()
            .position(|edges| edges == ids)
            .map(|end| (circle, end))
    })
}

enum PlanarTrim {
    Rectangle {
        points: Vec<P>,
        error: Q,
    },
    Circular {
        circle: usize,
        end: usize,
        sign: i32,
        error: Q,
    },
}

fn planar_domain<Id>(
    (axis, level, normal): (usize, Q, P),
    trims: Vec<PlanarTrim>,
    tris: &[[P; 3]],
    circles: &[Circle<Id>],
    mesh_error: &Q,
    cylinder_source_error: &Q,
) -> Proof<Q> {
    let mut tri_sign = 0;
    let mut mesh_height = n(0);
    for tri in tris {
        mesh_height = mesh_height.max(tri.iter().map(|p| (&p[axis] - &level).abs()).max().unwrap());
        let nn = cross(&sub(&tri[1], &tri[0]), &sub(&tri[2], &tri[0]));
        if &nn[axis] * &normal[axis] <= n(0) {
            return Err("plane triangle orientation/degeneracy");
        }
        let area = ring_area(tri, axis);
        let sign = if area > n(0) { 1 } else { -1 };
        if tri_sign != 0 && sign != tri_sign {
            return Err("inconsistent cap orientation");
        }
        tri_sign = sign;
    }
    let mut rings = boundary(tris)?;
    if rings.len() != trims.len() {
        return Err("incomplete cap holes/domain");
    }
    // The same delivered triangles project affinely; positivity above excludes
    // collapsed/folded projections. Authenticate their complete projected chain.
    let projected = tris
        .iter()
        .map(|tri| tri.each_ref().map(|p| project(p, axis, &level)))
        .collect::<Vec<_>>();
    let projected_rings = boundary(&projected)?;
    let expected_rings = rings
        .iter()
        .map(|ring| {
            edge_set(
                &ring
                    .iter()
                    .map(|p| project(p, axis, &level))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeSet<_>>();
    if projected_rings
        .iter()
        .map(|r| edge_set(r))
        .collect::<BTreeSet<_>>()
        != expected_rings
    {
        return Err("projection changed cap boundary census");
    }
    let mut specs = Vec::new();
    let mut source_error = cylinder_source_error.clone();
    for (index, trim) in trims.into_iter().enumerate() {
        let sense = if index == 0 { tri_sign } else { -tri_sign };
        let (position, spec) = match trim {
            PlanarTrim::Circular {
                circle,
                end,
                sign,
                error,
            } => {
                let c = &circles[circle];
                source_error = source_error.max(error);
                if sign != sense {
                    return Err("source cap circular material orientation");
                }
                let expected = c
                    .mesh_rings
                    .iter()
                    .zip(&c.mesh_levels)
                    .find(|(_, z)| **z == c.source_levels[end])
                    .map(|(ring, _)| ring)
                    .ok_or("missing cap/cylinder join")?;
                let pos = rings
                    .iter()
                    .position(|r| edge_set(r) == edge_set(expected))
                    .ok_or("cap/cylinder boundary mismatch")?;
                (pos, CapRing::Circular(circle))
            }
            PlanarTrim::Rectangle { points, error } => {
                source_error = source_error.max(error);
                let source_sign = if ring_area(&points, axis) > n(0) {
                    1
                } else {
                    -1
                };
                if source_sign != sense {
                    return Err("source cap rectangular material orientation");
                }
                let pos = rings
                    .iter()
                    .position(|r| {
                        rectangle_matches(
                            &points,
                            &r.iter()
                                .map(|p| project(p, axis, &level))
                                .collect::<Vec<_>>(),
                        )
                        .is_ok()
                    })
                    .ok_or("uncovered rectangular cap")?;
                (pos, CapRing::Rectangle(points))
            }
        };
        let ring = rings.remove(position);
        let sign = if ring_area(&ring, axis) > n(0) { 1 } else { -1 };
        if sign != if index == 0 { tri_sign } else { -tri_sign } {
            return Err("cap material orientation");
        }
        specs.push(spec);
    }
    if !rings.is_empty() {
        return Err("extra cap boundary");
    }
    // Every cap authenticates its own raw trims. Carrier identity does not
    // authorize discarding their error, even if the cylinder was checked first.
    let band = mesh_error + source_error + mesh_height;
    for spec in &specs {
        if let CapRing::Circular(ci) = spec
            && band >= circles[*ci].radius
        {
            return Err("cap radial band reaches axis");
        }
    }
    // Disjoint enclosing bands keep each radial boundary deformation local.
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    let boxes = specs
        .iter()
        .map(|spec| -> Proof<[[Q; 2]; 2]> {
            match spec {
                CapRing::Rectangle(points) => Ok(std::array::from_fn(|a| {
                    [
                        points.iter().map(|p| p[axes[a]].clone()).min().unwrap(),
                        points.iter().map(|p| p[axes[a]].clone()).max().unwrap(),
                    ]
                })),
                CapRing::Circular(ci) => {
                    let c = &circles[*ci];
                    let r = &c.radius + &band;
                    Ok(std::array::from_fn(|a| {
                        [&c.center[a] - &r, &c.center[a] + &r]
                    }))
                }
            }
        })
        .collect::<Proof<Vec<_>>>()?;
    for i in 1..specs.len() {
        match &specs[0] {
            CapRing::Rectangle(_) => {
                if (0..2)
                    .any(|a| boxes[i][a][0] <= boxes[0][a][0] || boxes[i][a][1] >= boxes[0][a][1])
                {
                    return Err("trim band reaches exterior");
                }
            }
            CapRing::Circular(ci) => {
                let c = &circles[*ci];
                let inside = &c.radius - &band;
                if inside <= n(0) {
                    return Err("outer trim band reaches axis");
                }
                for x in &boxes[i][0] {
                    for y in &boxes[i][1] {
                        let d = [x - &c.center[0], y - &c.center[1], n(0)];
                        if dot2(&d, &d) >= &inside * &inside {
                            return Err("nested trim bands overlap");
                        }
                    }
                }
            }
        }
        for j in 1..i {
            if !(0..2).any(|a| boxes[i][a][1] < boxes[j][a][0] || boxes[j][a][1] < boxes[i][a][0]) {
                return Err("hole trim bands overlap");
            }
        }
    }
    Ok(band)
}

fn qualification(result: Proof<Q>) -> GeneratedQualification {
    let result = result.and_then(|bound| {
        let mut out = bound.to_f64().ok_or("bound conversion overflow")?;
        while q(out)? < bound {
            out = out.next_up();
        }
        Ok(out)
    });
    match result {
        Ok(bound) => GeneratedQualification::Bounded {
            version: 1,
            class: "analytic-generated",
            direct_bound_mm: bound,
            source_precision_mm: 0.,
            total_bound_mm: bound,
        },
        Err(reason) => GeneratedQualification::Unknown {
            reasons: vec![reason.into()],
        },
    }
}
pub(super) mod native;
