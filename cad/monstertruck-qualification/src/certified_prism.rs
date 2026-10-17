//! Scratch-only prism subtraction: exact source identity/extrusion checks,
//! rationally bounded cap planarity at the existing kernel tolerance.
mod clip;
mod planar_cells;
use monstertruck_modeling::*;
use std::collections::HashSet;
use std::result::Result;

#[derive(Debug, PartialEq)]
pub enum Unsupported {
    Shells,
    Caps,
    Geometry,
    Topology,
    Orientation,
    Containment,
}

fn same_curve(a: &Curve, b: &Curve) -> bool {
    match (a, b) {
        (Curve::Line(a), Curve::Line(b)) => a.0 == b.0 && a.1 == b.1,
        (Curve::BsplineCurve(a), Curve::BsplineCurve(b)) => {
            a.knot_vector() == b.knot_vector() && a.control_points() == b.control_points()
        }
        (Curve::NurbsCurve(a), Curve::NurbsCurve(b)) => {
            a.knot_vector() == b.knot_vector() && a.control_points() == b.control_points()
        }
        _ => false,
    }
}
fn same_surface(a: &Surface, b: &Surface) -> bool {
    match (a, b) {
        (Surface::Plane(a), Surface::Plane(b)) => {
            a.origin() == b.origin() && a.axis_u() == b.axis_u() && a.axis_v() == b.axis_v()
        }
        (Surface::BsplineSurface(a), Surface::BsplineSurface(b)) => {
            a.knot_vector_u() == b.knot_vector_u()
                && a.knot_vector_v() == b.knot_vector_v()
                && a.control_points() == b.control_points()
        }
        (Surface::NurbsSurface(a), Surface::NurbsSurface(b)) => {
            a.knot_vector_u() == b.knot_vector_u()
                && a.knot_vector_v() == b.knot_vector_v()
                && a.control_points() == b.control_points()
        }
        _ => false,
    }
}
fn controls(c: &Curve) -> Option<Vec<Point3>> {
    match c {
        Curve::Line(c) => Some(vec![c.0, c.1]),
        Curve::BsplineCurve(c) => Some(c.control_points().clone()),
        Curve::NurbsCurve(c) => c
            .control_points()
            .iter()
            .map(|p| {
                (p.w.is_finite() && p.w > 0.0).then(|| Point3::new(p.x / p.w, p.y / p.w, p.z / p.w))
            })
            .collect(),
        _ => None,
    }
}
struct Prism {
    caps: [usize; 2],
    z: [f64; 2],
}
fn certify(s: &Solid) -> Result<Prism, Unsupported> {
    if s.boundaries().len() != 1 {
        return Err(Unsupported::Shells);
    }
    let shell = &s.boundaries()[0];
    Solid::try_new(vec![shell.clone()]).map_err(|_| Unsupported::Topology)?;
    let mut caps = Vec::new();
    for (i, f) in shell.iter().enumerate() {
        if let Surface::Plane(p) = f.surface() {
            if p.axis_u().z == 0.0
                && p.axis_v().z == 0.0
                && p.normal().z.is_finite()
                && p.normal().z != 0.0
            {
                caps.push((i, p.origin().z));
            }
        }
    }
    if caps.len() != 2 {
        return Err(Unsupported::Caps);
    }
    caps.sort_by(|a, b| a.1.total_cmp(&b.1));
    let [(lo, z0), (hi, z1)] = [caps[0], caps[1]];
    if !z0.is_finite() || !z1.is_finite() || z0 >= z1 {
        return Err(Unsupported::Caps);
    }
    let v = Vector3::new(0.0, 0.0, z1 - z0);
    let tr = Matrix4::from_translation(v);
    let bottom = &shell[lo];
    let top = &shell[hi];
    for (f, z, sign) in [(bottom, z0, -1.0), (top, z1, 1.0)] {
        let Surface::Plane(p) = f.surface() else {
            unreachable!()
        };
        let n = p.normal().z * if f.orientation() { 1.0 } else { -1.0 };
        if n * sign <= 0.0 {
            return Err(Unsupported::Orientation);
        }
        for e in f.absolute_boundaries().iter().flat_map(|w| w.edge_iter()) {
            if !e.is_geometric_consistent() {
                return Err(Unsupported::Geometry);
            }
            let pts = controls(&e.curve()).ok_or(Unsupported::Geometry)?;
            if pts.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
                || !cap_within_existing_tolerance(&e.curve(), z)
            {
                return Err(Unsupported::Geometry);
            }
        }
    }
    let lower: Vec<_> = bottom
        .absolute_boundaries()
        .iter()
        .flat_map(|w| w.edge_iter())
        .cloned()
        .collect();
    let upper: Vec<_> = top
        .absolute_boundaries()
        .iter()
        .flat_map(|w| w.edge_iter())
        .cloned()
        .collect();
    if lower.len() != upper.len() || shell.len() != lower.len() + 2 {
        return Err(Unsupported::Topology);
    }
    let mut used_top = HashSet::new();
    let mut used_walls = HashSet::new();
    for e in &lower {
        let translated = e.curve().transformed(tr);
        let matches: Vec<_> = upper
            .iter()
            .filter(|u| {
                same_curve(&translated, &u.curve())
                    && u.absolute_front().point() == tr.transform_point(e.absolute_front().point())
                    && u.absolute_back().point() == tr.transform_point(e.absolute_back().point())
            })
            .collect();
        if matches.len() != 1 || !used_top.insert(matches[0].id()) {
            return Err(Unsupported::Geometry);
        }
        let u = matches[0];
        let walls: Vec<_> = shell
            .iter()
            .enumerate()
            .filter(|(i, f)| {
                *i != lo
                    && *i != hi
                    && f.absolute_boundaries()
                        .iter()
                        .flat_map(|w| w.edge_iter())
                        .any(|q| q.id() == e.id())
            })
            .collect();
        if walls.len() != 1 || !used_walls.insert(walls[0].0) {
            return Err(Unsupported::Topology);
        }
        let wall = walls[0].1;
        if wall.absolute_boundaries().len() != 1 || wall.absolute_boundaries()[0].len() != 4 {
            return Err(Unsupported::Topology);
        }
        let edges: Vec<_> = wall.absolute_boundaries()[0].edge_iter().collect();
        if edges.iter().filter(|q| q.id() == u.id()).count() != 1 {
            return Err(Unsupported::Topology);
        }
        for (a, b) in [
            (e.absolute_front(), u.absolute_front()),
            (e.absolute_back(), u.absolute_back()),
        ] {
            let connectors: Vec<_> = edges
                .iter()
                .filter(|q| {
                    q.id() != e.id()
                        && q.id() != u.id()
                        && ((q.front().id() == a.id() && q.back().id() == b.id())
                            || (q.front().id() == b.id() && q.back().id() == a.id()))
                })
                .collect();
            if connectors.len() != 1 {
                return Err(Unsupported::Topology);
            }
            let q = connectors[0];
            let Curve::Line(line) = q.curve() else {
                return Err(Unsupported::Geometry);
            };
            if line.0 != q.absolute_front().point() || line.1 != q.absolute_back().point() {
                return Err(Unsupported::Geometry);
            }
        }
        let expected: Surface = ExtrusionSurface::by_extrusion(e.curve(), v).to_same_geometry();
        if !same_surface(&expected, &wall.surface()) {
            return Err(Unsupported::Geometry);
        }
    }
    Ok(Prism {
        caps: [lo, hi],
        z: [z0, z1],
    })
}

/// Subtract valid, outward, non-self-intersecting source solids in the supported
/// prism family. This checks the extrusion certificate, not global input validity.
/// Unsupported cases leave inputs untouched and must use a different Boolean route.
/// Approximate cap planarity is bounded over the entire curve at TOLERANCE;
/// source curves and their identities are never modified or welded.
pub fn subtract(body: &Solid, cutter: &Solid) -> Result<Solid, Unsupported> {
    let b = certify(body)?;
    let mut c = certify(cutter)?;
    let overlap = [b.z[0].max(c.z[0]), b.z[1].min(c.z[1])];
    if overlap[0] >= overlap[1] {
        return Ok(body.clone());
    }
    // Separate distinct cap supports by their summed existing curve-deviation bounds.
    if b.z != c.z
        && b.z.iter().any(|a| {
            c.z.iter()
                .any(|d| a != d && (a - d).abs() <= 2.0 * TOLERANCE)
        })
    {
        return Err(Unsupported::Caps);
    }
    let clipped = clip::trim(cutter, &c, overlap)?;
    c.z = overlap;
    let bs = &body.boundaries()[0];
    let cs = &clipped.boundaries()[0];
    for k in 0..2 {
        let cap = &cs[c.caps[k]];
        if cap.absolute_boundaries().len() != 1 {
            return Err(Unsupported::Containment);
        }
        let points: Vec<_> = cap.absolute_boundaries()[0]
            .edge_iter()
            .map(|e| exact_controls(&e.curve()))
            .collect::<Option<Vec<_>>>()
            .ok_or(Unsupported::Geometry)?
            .into_iter()
            .flatten()
            .collect();
        if !contained_hull(&points, &bs[b.caps[k]]) {
            return Err(Unsupported::Containment);
        }
    }
    let mut faces = Vec::new();
    for (i, f) in bs.iter().enumerate() {
        if let Some(k) = b
            .caps
            .iter()
            .position(|&n| n == i)
            .filter(|&k| b.z[k] == c.z[k])
        {
            let material = planar_cells::Material {
                inside: if k == 0 { [false, true] } else { [true, false] },
                outside: [false, false],
            };
            faces.extend(planar_cells::arrange(
                f,
                &cs[c.caps[k]],
                [material, material],
                planar_cells::Operation::Difference,
            )?);
        } else {
            faces.push(f.clone());
        }
    }
    faces.extend(
        cs.iter()
            .enumerate()
            .filter(|(i, _)| {
                c.caps
                    .iter()
                    .position(|j| j == i)
                    .is_none_or(|k| c.z[k] != b.z[k])
            })
            .map(|(_, f)| f.inverse()),
    );
    let shell: Shell = faces.into();
    Solid::try_new(shell.connected_components()).map_err(|_| Unsupported::Topology)
}

// Exact rational signs on the represented f64 coordinates. The hull encloses
// the complete positive-weight curve and its filled single-wire cutter region.
use num_rational::BigRational as Q;
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct P(Q, Q);
fn xy(p: Point3) -> Option<P> {
    Some(P(Q::from_float(p.x)?, Q::from_float(p.y)?))
}
fn orient(a: &P, b: &P, c: &P) -> Q {
    (&b.0 - &a.0) * (&c.1 - &a.1) - (&b.1 - &a.1) * (&c.0 - &a.0)
}
fn on(a: &P, b: &P, p: &P) -> bool {
    orient(a, b, p) == Q::from_integer(0.into())
        && p.0 >= a.0.clone().min(b.0.clone())
        && p.0 <= a.0.clone().max(b.0.clone())
        && p.1 >= a.1.clone().min(b.1.clone())
        && p.1 <= a.1.clone().max(b.1.clone())
}
fn inside(p: &P, ring: &[P]) -> bool {
    let zero = Q::from_integer(0.into());
    let mut winding = 0;
    for (a, b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if on(a, b, p) {
            return false;
        }
        if a.1 <= p.1 && p.1 < b.1 && orient(a, b, p) > zero {
            winding += 1
        }
        if b.1 <= p.1 && p.1 < a.1 && orient(a, b, p) < zero {
            winding -= 1
        }
    }
    winding != 0
}
fn intersects(a: &P, b: &P, c: &P, d: &P) -> bool {
    let z = Q::from_integer(0.into());
    on(a, b, c)
        || on(a, b, d)
        || on(c, d, a)
        || on(c, d, b)
        || ((orient(a, b, c) > z) != (orient(a, b, d) > z)
            && (orient(c, d, a) > z) != (orient(c, d, b) > z))
}
fn cap_deviation(curve: &Curve, z: f64) -> Option<Q> {
    let plane = Q::from_float(z)?;
    let zs: Vec<Q> = match curve {
        Curve::NurbsCurve(c) => c
            .control_points()
            .iter()
            .map(|p| {
                if !p.w.is_finite() || p.w <= 0.0 {
                    return None;
                }
                Some(Q::from_float(p.z)? / Q::from_float(p.w)?)
            })
            .collect::<Option<Vec<_>>>()?,
        _ => controls(curve)?
            .into_iter()
            .map(|p| Q::from_float(p.z))
            .collect::<Option<Vec<_>>>()?,
    };
    zs.into_iter()
        .map(|z| {
            let d = z - &plane;
            if d < Q::from_integer(0.into()) { -d } else { d }
        })
        .max()
}
fn cap_within_existing_tolerance(curve: &Curve, z: f64) -> bool {
    cap_deviation(curve, z).is_some_and(|d| d <= Q::from_float(TOLERANCE).unwrap())
}
fn distance2(p: &P, a: &P, b: &P) -> Q {
    let dx = &b.0 - &a.0;
    let dy = &b.1 - &a.1;
    let l2 = &dx * &dx + &dy * &dy;
    let dot = (&p.0 - &a.0) * &dx + (&p.1 - &a.1) * &dy;
    let zero = Q::from_integer(0.into());
    let q = if l2 == zero || dot <= zero {
        a.clone()
    } else if dot >= l2 {
        b.clone()
    } else {
        P(&a.0 + &dx * &dot / &l2, &a.1 + &dy * &dot / &l2)
    };
    let x = &p.0 - &q.0;
    let y = &p.1 - &q.1;
    &x * &x + &y * &y
}
fn exact_controls(curve: &Curve) -> Option<Vec<P>> {
    match curve {
        Curve::NurbsCurve(c) => c
            .control_points()
            .iter()
            .map(|p| {
                if !p.w.is_finite() || p.w <= 0.0 {
                    return None;
                }
                let w = Q::from_float(p.w)?;
                Some(P(Q::from_float(p.x)? / &w, Q::from_float(p.y)? / &w))
            })
            .collect(),
        _ => controls(curve)?.into_iter().map(xy).collect(),
    }
}
// A supported cap is one simple outer ring with disjoint, nonnested holes.
// A topology shell count alone does not establish geometric connectedness.
fn single_region_with_holes(rings: &[Vec<P>]) -> bool {
    if rings.is_empty() {
        return false;
    }
    for ring in rings {
        for i in 0..ring.len() {
            let ni = (i + 1) % ring.len();
            if ring[i] == ring[ni] {
                return false;
            }
            let next = &ring[(i + 2) % ring.len()];
            if orient(&ring[i], &ring[ni], next) == Q::from_integer(0.into())
                && (on(&ring[i], &ring[ni], next) || on(&ring[ni], next, &ring[i]))
            {
                return false;
            }
            for j in i + 1..ring.len() {
                let nj = (j + 1) % ring.len();
                if j == ni || nj == i {
                    continue;
                }
                if intersects(&ring[i], &ring[ni], &ring[j], &ring[nj]) {
                    return false;
                }
            }
        }
    }
    for (i, a) in rings.iter().enumerate() {
        for b in rings.iter().skip(i + 1) {
            for j in 0..a.len() {
                for k in 0..b.len() {
                    if intersects(&a[j], &a[(j + 1) % a.len()], &b[k], &b[(k + 1) % b.len()]) {
                        return false;
                    }
                }
            }
        }
    }
    let outers: Vec<_> = (0..rings.len())
        .filter(|&i| (0..rings.len()).all(|j| i == j || inside(&rings[j][0], &rings[i])))
        .collect();
    if outers.len() != 1 {
        return false;
    }
    let outer = outers[0];
    (0..rings.len()).filter(|&i| i != outer).all(|i| {
        (0..rings.len())
            .filter(|&j| j != outer && j != i)
            .all(|j| !inside(&rings[i][0], &rings[j]))
    })
}

fn contained_hull(points: &[P], face: &Face) -> bool {
    let mut points = points.to_vec();
    points.sort();
    points.dedup();
    if points.len() < 3 {
        return false;
    }
    let z = Q::from_integer(0.into());
    let mut hull = Vec::new();
    for p in &points {
        while hull.len() >= 2 && orient(&hull[hull.len() - 2], &hull[hull.len() - 1], p) <= z {
            hull.pop();
        }
        hull.push(p.clone());
    }
    let n = hull.len();
    for p in points.iter().rev().skip(1) {
        while hull.len() > n && orient(&hull[hull.len() - 2], &hull[hull.len() - 1], p) <= z {
            hull.pop();
        }
        hull.push(p.clone());
    }
    hull.pop();
    if hull.len() < 3 {
        return false;
    }
    let mut rings = Vec::new();
    for wire in face.boundaries() {
        let mut ring = Vec::new();
        for e in wire.edge_iter() {
            let Curve::Line(line) = e.curve() else {
                return false;
            };
            if line.0 != e.absolute_front().point() || line.1 != e.absolute_back().point() {
                return false;
            }
            let Some(p) = xy(e.front().point()) else {
                return false;
            };
            ring.push(p);
        }
        if ring.len() < 3 {
            return false;
        }
        rings.push(ring);
    }
    if !single_region_with_holes(&rings) {
        return false;
    }
    if hull
        .iter()
        .any(|p| rings.iter().filter(|r| inside(p, r)).count() % 2 != 1)
    {
        return false;
    }
    for (a, b) in hull
        .iter()
        .zip(hull.iter().cycle().skip(1))
        .take(hull.len())
    {
        for ring in &rings {
            for (c, d) in ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
            {
                let tol = Q::from_float(TOLERANCE).unwrap();
                let tol2 = &tol * &tol;
                if intersects(a, b, c, d)
                    || [
                        distance2(a, c, d),
                        distance2(b, c, d),
                        distance2(c, a, b),
                        distance2(d, a, b),
                    ]
                    .into_iter()
                    .any(|d| d <= tol2)
                {
                    return false;
                }
            }
        }
    }
    !rings.iter().flatten().any(|p| inside(p, &hull))
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
    fn base_fixture() -> Solid {
        let points = [
            (-15.599999880790705, 48.00000007692723),
            (-15.599999880790705, 34.000000076927236),
            (-4.599999880790705, 34.000000076927236),
            (-4.599999880790705, -4.599999827705332),
            (35.933547997474676, -4.599999827705332),
            (35.933547997474676, -15.599999827705332),
            (49.93354799747467, -15.599999827705332),
            (49.93354799747467, -4.599999827705332),
            (120.00000002384186, -4.599999827705332),
            (120.00000002384186, -15.599999827705332),
            (134.00000002384186, -15.599999827705332),
            (134.00000002384186, -4.599999827705332),
            (149.59999992847443, -4.599999827705332),
            (149.59999992847443, 109.5999999815598),
            (146.00000002384186, 109.5999999815598),
            (146.00000002384186, 120.5999999815598),
            (132.00000002384186, 120.5999999815598),
            (132.00000002384186, 109.5999999815598),
            (111.00000002384186, 109.5999999815598),
            (111.00000002384186, 120.5999999815598),
            (97.00000002384186, 120.5999999815598),
            (97.00000002384186, 109.5999999815598),
            (25.000000023841864, 109.5999999815598),
            (25.000000023841864, 120.5999999815598),
            (11.000000023841864, 120.5999999815598),
            (11.000000023841864, 109.5999999815598),
            (-4.599999880790705, 109.5999999815598),
            (-4.599999880790705, 48.00000007692723),
        ];
        let vertices = builder::vertices(points.iter().map(|&(x, y)| Point3::new(x, y, -6.6)));
        let wire: Wire = (0..vertices.len())
            .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % vertices.len()]))
            .collect();
        let face: Face = builder::try_attach_plane(vec![wire]).unwrap();
        builder::extrude(&face, Vector3::unit_z() * 13.3)
    }

    fn cutter_fixture() -> Solid {
        let origin = Point3::new(42.93354797363281, -11.1, -6.6);
        let radius = 1.1;
        let right = builder::vertex(Point3::new(origin.x + radius, origin.y, origin.z));
        let left = builder::vertex(Point3::new(origin.x - radius, origin.y, origin.z));
        let wire: Wire = vec![
            builder::circle_arc(
                &right,
                &left,
                Point3::new(origin.x, origin.y + radius, origin.z),
            ),
            builder::circle_arc(
                &left,
                &right,
                Point3::new(origin.x, origin.y - radius, origin.z),
            ),
        ]
        .into();
        let face: Face = builder::try_attach_plane(vec![wire]).unwrap();
        builder::extrude(&face, Vector3::unit_z() * 13.3)
    }

    #[test]
    fn exact_fixture_is_closed_outward_and_reuses_every_edge() {
        let b = base_fixture();
        let c = cutter_fixture();
        let r = subtract(&b, &c).expect("certified source prism");
        let mesh = shell_to_polygon_strict(&r.boundaries()[0], 0.1).unwrap();
        let bm = shell_to_polygon_strict(&b.boundaries()[0], 0.1).unwrap();
        let expected = bm.volume() - std::f64::consts::PI * 1.1 * 1.1 * 13.3;
        let bound = std::f64::consts::PI * (2.0 * 1.1 * 0.1 - 0.1 * 0.1) * 13.3;
        assert!(
            (mesh.volume() - expected).abs() < bound,
            "volume {} expected {}",
            mesh.volume(),
            expected
        );
        let ray = Vector3::new(0.123, 0.456, 1.0).normalize();
        assert_eq!(
            mesh.signed_crossing_faces(Point3::new(42.93354797363281, -11.1, 0.0), ray),
            0
        );
        assert_eq!(
            mesh.signed_crossing_faces(Point3::new(42.93354797363281, -14.0, 0.0), ray),
            1
        );
        let mut nonzero = 0;
        for face in c.boundaries()[0].iter() {
            if let Surface::Plane(p) = face.surface() {
                for edge in face
                    .absolute_boundaries()
                    .iter()
                    .flat_map(|w| w.edge_iter())
                {
                    if let Curve::NurbsCurve(n) = edge.curve() {
                        for q in n.control_points() {
                            if Q::from_float(q.z).unwrap()
                                != Q::from_float(p.origin().z).unwrap()
                                    * Q::from_float(q.w).unwrap()
                            {
                                nonzero += 1;
                            }
                        }
                    }
                }
            }
        }
        let max_dev = c.boundaries()[0]
            .iter()
            .filter_map(|f| match f.surface() {
                Surface::Plane(p) => Some((f, p.origin().z)),
                _ => None,
            })
            .flat_map(|(f, z)| {
                f.absolute_boundaries()
                    .iter()
                    .flat_map(|w| w.edge_iter())
                    .map(move |e| cap_deviation(&e.curve(), z).unwrap())
            })
            .max()
            .unwrap();
        let approximate = max_dev.numer().to_string().parse::<f64>().unwrap()
            / max_dev.denom().to_string().parse::<f64>().unwrap();
        println!(
            "homogeneous controls with mathematically nonzero cap-plane residual={nonzero}; maximum whole-curve bound={approximate:e}mm; existing tolerance={TOLERANCE:e}mm"
        );
        let original: HashSet<_> = b.boundaries()[0]
            .edge_iter()
            .chain(c.boundaries()[0].edge_iter())
            .map(|e| e.id())
            .collect();
        let result: HashSet<_> = r.boundaries()[0].edge_iter().map(|e| e.id()).collect();
        assert_eq!(original, result);
        let mut incidence = std::collections::HashMap::new();
        for edge in r.boundaries()[0]
            .iter()
            .flat_map(|f| f.boundaries())
            .flat_map(|w| w.into_iter())
        {
            let entry = incidence.entry(edge.id()).or_insert((0usize, 0i32));
            entry.0 += 1;
            entry.1 += if edge.orientation() { 1 } else { -1 };
        }
        assert!(
            incidence
                .values()
                .all(|&(count, orientation)| count == 2 && orientation == 0)
        );
        println!(
            "closed source solid; faces={}; volume={}; analytic={}; original edge identities={}",
            r.boundaries()[0].len(),
            mesh.volume(),
            expected,
            result.len()
        );
    }

    fn rectangle(x: f64, y: f64, w: f64, h: f64, z: f64) -> Wire {
        let vs = builder::vertices(
            [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
                .into_iter()
                .map(|(x, y)| Point3::new(x, y, z)),
        );
        (0..4)
            .map(|i| builder::line(&vs[i], &vs[(i + 1) % 4]))
            .collect()
    }
    fn prism(x: f64, y: f64, w: f64, h: f64, z: f64, depth: f64) -> Solid {
        let f = builder::try_attach_plane(vec![rectangle(x, y, w, h, z)]).unwrap();
        builder::extrude(&f, Vector3::new(0.0, 0.0, depth))
    }
    fn round_prism(z: f64, depth: f64) -> Solid {
        let right = builder::vertex(Point3::new(5.0, 4.0, z));
        let left = builder::vertex(Point3::new(3.0, 4.0, z));
        let wire: Wire = vec![
            builder::circle_arc(&right, &left, Point3::new(4.0, 5.0, z)),
            builder::circle_arc(&left, &right, Point3::new(4.0, 3.0, z)),
        ]
        .into();
        let face = builder::try_attach_plane(vec![wire]).unwrap();
        builder::extrude(&face, Vector3::unit_z() * depth)
    }
    #[test]
    fn contained_blind_through_one_flush_and_cavity_cuts_preserve_material() {
        let body = prism(0.0, 0.0, 8.0, 8.0, 0.0, 4.0);
        for (label, z, depth, removed_height, shells) in [
            ("blind", 2.0, 3.0, 2.0, 1),
            ("nonflush through", -1.0, 6.0, 4.0, 1),
            ("one flush", 0.0, 2.0, 2.0, 1),
            ("cavity", 1.0, 2.0, 2.0, 2),
        ] {
            let cutter = round_prism(z, depth);
            let result = subtract(&body, &cutter).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert_eq!(result.boundaries().len(), shells, "{label}");
            let meshes: Vec<_> = result
                .boundaries()
                .iter()
                .map(|s| shell_to_polygon_strict(s, 0.05).unwrap())
                .collect();
            let volume: f64 = meshes.iter().map(|m| m.volume()).sum();
            let expected = 256.0 - std::f64::consts::PI * removed_height;
            let bound = std::f64::consts::PI * (2.0 * 0.05 - 0.05 * 0.05) * removed_height;
            assert!(
                (volume - expected).abs() < bound,
                "{label}: {volume} expected {expected}"
            );
            if shells == 2 {
                assert!(meshes[1].volume() < 0.0, "cavity shell must be inward");
            }
            let dir = Vector3::new(0.123, 0.456, 1.0).normalize();
            for height in [0.5, 1.5, 2.5, 3.5] {
                let expected = if height > z && height < z + depth {
                    0
                } else {
                    1
                };
                let crossings: isize = meshes
                    .iter()
                    .map(|m| m.signed_crossing_faces(Point3::new(4.0, 4.0, height), dir))
                    .sum();
                assert_eq!(crossings, expected, "{label} at z={height}");
            }
            let mut incidence = std::collections::HashMap::new();
            for edge in result
                .boundaries()
                .iter()
                .flat_map(|s| s.iter())
                .flat_map(|f| f.boundaries())
                .flat_map(|w| w.into_iter())
            {
                let entry = incidence.entry(edge.id()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += if edge.orientation() { 1 } else { -1 };
            }
            assert!(incidence.values().all(|&v| v == (2, 0)), "{label}");
            assert!(
                body.boundaries()[0]
                    .edge_iter()
                    .all(|e| incidence.contains_key(&e.id())),
                "body edge identities must survive"
            );
            for wall in cutter.boundaries()[0]
                .iter()
                .filter(|f| !matches!(f.surface(), Surface::Plane(_)))
            {
                assert!(
                    result
                        .boundaries()
                        .iter()
                        .flat_map(|s| s.iter())
                        .any(|f| same_surface(&f.surface(), &wall.surface())),
                    "original cutter wall support must survive {label}"
                );
            }
            println!(
                "{label}: shells={shells}; volume={volume}; expected={expected}; material=pass"
            );
        }
    }

    #[test]
    fn contained_box_preserves_material_and_volume() {
        let b = prism(0.0, 0.0, 4.0, 4.0, 0.0, 2.0);
        let c = prism(1.0, 1.0, 1.0, 1.0, 0.0, 2.0);
        let r = subtract(&b, &c).unwrap();
        let m = shell_to_polygon_strict(&r.boundaries()[0], 0.1).unwrap();
        assert!((m.volume() - 30.0).abs() < 1e-10);
    }
    #[test]
    fn supported_depths_and_unsupported_trim_contacts_are_distinguished() {
        let b = prism(0.0, 0.0, 2.0, 2.0, 0.0, 1.0);
        for (name, c, expected) in [
            (
                "partially overlapping coplanar trim",
                prism(1.0, 0.5, 2.0, 1.0, 0.0, 1.0),
                Some(Unsupported::Containment),
            ),
            ("blind", prism(0.5, 0.5, 1.0, 1.0, 0.5, 0.5), None),
            (
                "nonflush through",
                prism(0.5, 0.5, 1.0, 1.0, -1.0, 3.0),
                None,
            ),
            ("one flush end", prism(0.5, 0.5, 1.0, 1.0, 0.0, 2.0), None),
            (
                "touching trim",
                prism(0.0, 0.5, 1.0, 1.0, 0.0, 1.0),
                Some(Unsupported::Containment),
            ),
            (
                "within existing tolerance of trim",
                prism(0.5e-6, 0.5, 1.0, 1.0, 0.0, 1.0),
                Some(Unsupported::Containment),
            ),
        ] {
            match expected {
                Some(expected) => assert_eq!(subtract(&b, &c).unwrap_err(), expected, "{name}"),
                None => assert!(subtract(&b, &c).is_ok(), "{name} is now supported"),
            }
        }
    }
    #[test]
    fn authored_gasket_coordinates_support_blind_circular_floor() {
        let body = base_fixture();
        let (x, y, z, r) = (42.93354797363281, -11.1, 2.0, 1.1);
        let right = builder::vertex(Point3::new(x + r, y, z));
        let left = builder::vertex(Point3::new(x - r, y, z));
        let wire: Wire = vec![
            builder::circle_arc(&right, &left, Point3::new(x, y + r, z)),
            builder::circle_arc(&left, &right, Point3::new(x, y - r, z)),
        ]
        .into();
        let face = builder::try_attach_plane(vec![wire]).unwrap();
        let cutter = builder::extrude(&face, Vector3::unit_z() * 13.3);
        let result = subtract(&body, &cutter).expect("authored coordinates blind cut");
        assert_eq!(result.boundaries().len(), 1);
        let mesh = shell_to_polygon_strict(&result.boundaries()[0], 0.1).unwrap();
        let before = shell_to_polygon_strict(&body.boundaries()[0], 0.1)
            .unwrap()
            .volume();
        let height = certify(&body).unwrap().z[1] - z;
        let expected = before - std::f64::consts::PI * r * r * height;
        let bound = std::f64::consts::PI * (2.0 * r * 0.1 - 0.1 * 0.1) * height;
        assert!((mesh.volume() - expected).abs() < bound);
        let dir = Vector3::new(0.123, 0.456, 1.0).normalize();
        assert_eq!(mesh.signed_crossing_faces(Point3::new(x, y, 0.0), dir), 1);
        assert_eq!(mesh.signed_crossing_faces(Point3::new(x, y, 4.0), dir), 0);
        assert_eq!(
            mesh.signed_crossing_faces(Point3::new(x, -14.0, 4.0), dir),
            1
        );
        println!(
            "authored blind: volume={}; expected={expected}; bound={bound}; material=pass",
            mesh.volume()
        );
    }
    #[test]
    fn separated_depth_returns_source_and_uncertain_depth_contact_refuses() {
        let body = prism(0.0, 0.0, 8.0, 8.0, 0.0, 4.0);
        for (z, depth) in [(5.0, 1.0), (-2.0, 1.0), (4.0, 1.0)] {
            let result = subtract(&body, &round_prism(z, depth)).unwrap();
            assert_eq!(
                result.boundaries()[0]
                    .iter()
                    .map(|f| f.id())
                    .collect::<Vec<_>>(),
                body.boundaries()[0]
                    .iter()
                    .map(|f| f.id())
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(
            subtract(&body, &round_prism(TOLERANCE, 5.0)).unwrap_err(),
            Unsupported::Caps
        );
    }

    #[test]
    fn body_hole_is_preserved_and_enclosed_or_crossed_holes_rejected() {
        let f = builder::try_attach_plane(vec![
            rectangle(0.0, 0.0, 6.0, 6.0, 0.0),
            rectangle(1.0, 1.0, 1.0, 1.0, 0.0).inverse(),
        ])
        .unwrap();
        let b: Solid = builder::extrude(&f, Vector3::new(0.0, 0.0, 2.0));
        let c = prism(3.0, 3.0, 1.0, 1.0, 0.0, 2.0);
        let r = subtract(&b, &c).unwrap();
        let m = shell_to_polygon_strict(&r.boundaries()[0], 0.1).unwrap();
        assert!((m.volume() - 68.0).abs() < 1e-10);
        let blind = subtract(&b, &prism(3.0, 3.0, 1.0, 1.0, 1.0, 2.0)).unwrap();
        let blind_mesh = shell_to_polygon_strict(&blind.boundaries()[0], 0.1).unwrap();
        assert!((blind_mesh.volume() - 69.0).abs() < 1e-10);
        let dir = Vector3::new(0.123, 0.456, 1.0).normalize();
        for p in [Point3::new(1.5, 1.5, 1.0), Point3::new(3.5, 3.5, 1.0)] {
            assert_eq!(m.signed_crossing_faces(p, dir), 0);
        }
        assert_eq!(
            subtract(&b, &prism(0.5, 0.5, 2.0, 2.0, 0.0, 2.0)).unwrap_err(),
            Unsupported::Containment
        );
        assert_eq!(
            subtract(&b, &prism(1.5, 1.5, 2.0, 2.0, 0.0, 2.0)).unwrap_err(),
            Unsupported::Containment
        );
    }
    #[test]
    fn inward_orientation_and_disconnected_input_are_rejected() {
        let b = prism(0.0, 0.0, 4.0, 4.0, 0.0, 2.0);
        let mut c = prism(1.0, 1.0, 1.0, 1.0, 0.0, 2.0);
        c.not();
        assert_eq!(subtract(&b, &c).unwrap_err(), Unsupported::Orientation);
        let remote = prism(10.0, 10.0, 1.0, 1.0, 0.0, 2.0);
        let disconnected = Solid::try_new(vec![
            b.boundaries()[0].clone(),
            remote.boundaries()[0].clone(),
        ])
        .unwrap();
        assert_eq!(
            subtract(&disconnected, &c).unwrap_err(),
            Unsupported::Shells
        );
    }

    #[test]
    fn whole_curve_plane_bound_uses_existing_tolerance_without_widening() {
        let curve = |z| Curve::Line(Line(Point3::new(0.0, 0.0, z), Point3::new(1.0, 0.0, z)));
        assert!(cap_within_existing_tolerance(&curve(TOLERANCE), 0.0));
        assert!(!cap_within_existing_tolerance(
            &curve(f64::from_bits(TOLERANCE.to_bits() + 1)),
            0.0
        ));
        assert!(!cap_within_existing_tolerance(&curve(f64::NAN), 0.0));
    }
    #[test]
    #[ignore = "deliberately failing generic-route baseline"]
    fn generic_route_actual_fixture_baseline() {
        let b = base_fixture();
        let c = cutter_fixture();
        let r = monstertruck_solid::difference(&b, &c, TOLERANCE)
            .expect("generic route must return a closed solid");
        shell_to_polygon_strict(&r.boundaries()[0], 0.1).unwrap();
    }
    #[test]
    fn altered_wall_geometry_and_cutter_holes_are_unsupported() {
        let body = prism(0.0, 0.0, 6.0, 6.0, 0.0, 2.0);
        let cutter = prism(1.0, 1.0, 1.0, 1.0, 0.0, 2.0);
        let cp = certify(&cutter).unwrap();
        let mut faces = cutter.boundaries()[0].iter().cloned().collect::<Vec<_>>();
        let wall = (0..faces.len()).find(|i| !cp.caps.contains(i)).unwrap();
        let old = &faces[wall];
        let mut replacement = Face::new(
            old.absolute_boundaries().clone(),
            Surface::Plane(Plane::new(
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            )),
        );
        if !old.orientation() {
            replacement.invert();
        }
        faces[wall] = replacement;
        let invalid = Solid::try_new(vec![faces.into()]).unwrap();
        assert!(subtract(&body, &invalid).is_err());
        let f = builder::try_attach_plane(vec![
            rectangle(1.0, 1.0, 3.0, 3.0, 0.0),
            rectangle(2.0, 2.0, 1.0, 1.0, 0.0).inverse(),
        ])
        .unwrap();
        let holed: Solid = builder::extrude(&f, Vector3::new(0.0, 0.0, 2.0));
        assert_eq!(
            subtract(&body, &holed).unwrap_err(),
            Unsupported::Containment
        );
    }
    #[test]
    fn disconnected_regions_in_one_cap_face_are_explicitly_unsupported() {
        let f = builder::try_attach_plane(vec![
            rectangle(0.0, 0.0, 4.0, 4.0, 0.0),
            rectangle(10.0, 10.0, 4.0, 4.0, 0.0),
        ])
        .unwrap();
        let body: Solid = builder::extrude(&f, Vector3::new(0.0, 0.0, 2.0));
        assert_eq!(body.boundaries().len(), 1);
        let cutter = prism(1.0, 1.0, 1.0, 1.0, 0.0, 2.0);
        assert_eq!(
            subtract(&body, &cutter).unwrap_err(),
            Unsupported::Containment
        );
    }
    #[test]
    fn exact_ring_rejects_adjacent_backtracking_but_allows_forward_collinearity() {
        let ring = |coords: &[(f64, f64)]| {
            coords
                .iter()
                .map(|&(x, y)| xy(Point3::new(x, y, 0.0)).unwrap())
                .collect::<Vec<_>>()
        };
        assert!(!single_region_with_holes(&[ring(&[
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.0)
        ])]));
        assert!(single_region_with_holes(&[ring(&[
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0)
        ])]));
        assert!(!single_region_with_holes(&[ring(&[
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.0),
            (1.0, 2.0),
            (0.0, 2.0)
        ])]));
    }
}
