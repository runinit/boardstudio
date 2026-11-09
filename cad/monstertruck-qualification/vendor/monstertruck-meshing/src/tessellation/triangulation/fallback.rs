//! Preview-only fallback that triangulates a face from its shared boundary samples.
//!
//! The ordinary route triangulates in the surface's `(u, v)` chart. That needs every boundary
//! sample projected into the chart, which fails for collapsed (pole) edges, repeated seam edges
//! and sub-tolerance sliver faces. This route works from the 3D samples that neighbouring faces
//! share, so no chart is needed:
//!
//! 1. each loop is cleaned (coincident samples merged, back-and-forth slits cancelled) and the
//!    loops are joined into one weakly simple polygon by zero-width bridges,
//! 2. the polygon is triangulated over exactly those samples by minimising total triangle area
//!    (a topological disk, so seam slits and annuli need no chart),
//! 3. every triangle whose centroid sits further than half the tolerance from the
//!    surface has its longest interior edge bisected at the nearest surface point, repeatedly
//!    (edges with one triangle are shared boundary chords and never split),
//! 4. when the polygon is unusable, a fan over the shared samples is the last resort.
//!
//! Boundary vertices are the original sample points, never re-evaluated, so neighbouring faces
//! still join. The result is preview geometry: its uv attribute is a placeholder.

use super::*;
use mesh::ensure_winding_matches_normals;

const REFINEMENT_ROUNDS: usize = 24;
const MAX_VERTICES: usize = 4096;
const MAX_SURFACE_SEARCHES: usize = 32768;
/// Cubic-time area minimisation is bounded by this many polygon vertices.
const MAX_POLYGON: usize = 480;
/// Fraction of the tolerance a triangle centroid may deviate from the surface.
const DEVIATION_FRACTION: f64 = 0.5;
/// Samples closer than this fraction of the tolerance are one point.
const COINCIDENT_FRACTION: f64 = 1.0e-6;

/// Triangulates `loops` (closed implicitly, in face order) without a surface chart.
pub(super) fn preview_boundary_triangulation<S: PreMeshableSurface>(
    surface: &S,
    loops: &[Vec<Point3>],
    tolerance: f64,
    sp: &impl SP<S>,
) -> Option<PolygonMesh> {
    let eps = tolerance * COINCIDENT_FRACTION;
    let cleaned = loops
        .iter()
        .map(|raw| clean_loop(raw, eps))
        .filter(|ring| ring.len() >= 3)
        .collect::<Vec<_>>();
    area_minimising_triangulation(surface, cleaned, tolerance, sp)
        .filter(|mesh| !mesh.tri_faces().is_empty())
        .or_else(|| fan_triangulation(surface, loops, sp))
}

fn clean_loop(raw: &[Point3], eps: f64) -> Vec<Point3> {
    let mut ring = Vec::<Point3>::new();
    for &point in raw {
        if ring.last().is_some_and(|last| last.distance(point) <= eps) {
            continue;
        }
        if ring.len() >= 2 && ring[ring.len() - 2].distance(point) <= eps {
            // The loop retraced its last segment: a zero-width slit.
            ring.pop();
            continue;
        }
        ring.push(point);
    }
    loop {
        let n = ring.len();
        if n >= 2 && ring[0].distance(ring[n - 1]) <= eps {
            ring.pop();
        } else if n >= 3 && ring[0].distance(ring[n - 2]) <= eps {
            ring.pop();
        } else if n >= 3 && ring[1].distance(ring[n - 1]) <= eps {
            ring.remove(0);
        } else {
            return ring;
        }
    }
}

fn area_vector(ring: &[Point3]) -> Vector3 {
    let origin = ring[0];
    ring.windows(2)
        .map(|w| (w[0] - origin).cross(w[1] - origin))
        .fold(Vector3::zero(), |acc, v| acc + v)
        * 0.5
}

fn is_finite_vector(v: Vector3) -> bool { v.x.is_finite() && v.y.is_finite() && v.z.is_finite() }

fn surface_normal<S: PreMeshableSurface>(
    surface: &S,
    sp: &impl SP<S>,
    point: Point3,
    otherwise: Vector3,
) -> Vector3 {
    sp(surface, point, None)
        .map(|(u, v)| surface.normal(u, v))
        .filter(|n| is_finite_vector(*n) && n.magnitude2() > 0.0)
        .unwrap_or(otherwise)
}

fn into_mesh(positions: Vec<Point3>, normals: Vec<Vector3>, tris: Vec<[usize; 3]>) -> PolygonMesh {
    let uv_coords = vec![Vector2::new(0.0, 0.0); positions.len()];
    let tri_faces = tris
        .into_iter()
        .map(|t| array![i => [t[i], t[i], t[i]].into(); 3])
        .collect::<Vec<[StandardVertex; 3]>>();
    let mut mesh = PolygonMesh::debug_new(
        StandardAttributes {
            positions,
            uv_coords,
            normals,
        },
        Faces::from_tri_and_quad_faces(tri_faces, Vec::new()),
    );
    ensure_winding_matches_normals(&mut mesh);
    mesh
}

/// Joins every other loop to the largest one through a zero-width bridge between their
/// closest vertices, giving one weakly simple polygon.
fn join_loops(mut rings: Vec<Vec<Point3>>) -> Vec<Point3> {
    let outer = (0..rings.len())
        .max_by(|&a, &b| {
            area_vector(&rings[a])
                .magnitude2()
                .total_cmp(&area_vector(&rings[b]).magnitude2())
        })
        .unwrap_or(0);
    let mut polygon = rings.swap_remove(outer);
    for hole in rings {
        let (a, b) = (0..polygon.len())
            .flat_map(|a| (0..hole.len()).map(move |b| (a, b)))
            .min_by(|&(a0, b0), &(a1, b1)| {
                polygon[a0]
                    .distance2(hole[b0])
                    .total_cmp(&polygon[a1].distance2(hole[b1]))
            })
            .unwrap_or((0, 0));
        let bridge = (0..=hole.len()).map(|i| hole[(b + i) % hole.len()]);
        let tail = polygon.split_off(a + 1);
        polygon.extend(bridge);
        polygon.push(polygon[a]);
        polygon.extend(tail);
    }
    polygon
}

/// Triangle indices of the minimum-total-area triangulation of the polygon `0..n`.
fn minimum_area_triangles(points: &[Point3]) -> Option<Vec<[usize; 3]>> {
    let n = points.len();
    if !(3..=MAX_POLYGON).contains(&n) {
        return None;
    }
    // Area first; the edge-length term only breaks ties towards compact triangles.
    let weight = |i: usize, k: usize, j: usize| {
        let (a, b, c) = (points[i], points[k], points[j]);
        let longest = a.distance2(b).max(b.distance2(c)).max(c.distance2(a));
        (b - a).cross(c - a).magnitude() * 0.5 + 0.02 * longest
    };
    let mut cost = vec![0.0f64; n * n];
    let mut split = vec![0usize; n * n];
    for span in 2..n {
        for i in 0..n - span {
            let j = i + span;
            let mut best = (f64::INFINITY, i + 1);
            for k in i + 1..j {
                let total = cost[i * n + k] + cost[k * n + j] + weight(i, k, j);
                if total < best.0 {
                    best = (total, k);
                }
            }
            if !best.0.is_finite() {
                return None;
            }
            cost[i * n + j] = best.0;
            split[i * n + j] = best.1;
        }
    }
    let mut tris = Vec::with_capacity(n - 2);
    let mut stack = vec![(0, n - 1)];
    while let Some((i, j)) = stack.pop() {
        if j - i < 2 {
            continue;
        }
        let k = split[i * n + j];
        tris.push([i, k, j]);
        stack.push((i, k));
        stack.push((k, j));
    }
    Some(tris)
}

fn area_minimising_triangulation<S: PreMeshableSurface>(
    surface: &S,
    rings: Vec<Vec<Point3>>,
    tolerance: f64,
    sp: &impl SP<S>,
) -> Option<PolygonMesh> {
    if rings.is_empty() {
        return None;
    }
    let polygon = join_loops(rings);
    let eps = tolerance * COINCIDENT_FRACTION;
    // One vertex per coincident group, so folded slits collapse to zero-area triangles.
    let mut positions = Vec::<Point3>::new();
    let ids = polygon
        .iter()
        .map(|&p| {
            positions.iter().position(|q| q.distance(p) <= eps).unwrap_or_else(|| {
                positions.push(p);
                positions.len() - 1
            })
        })
        .collect::<Vec<_>>();
    let mut tris = minimum_area_triangles(&polygon)?
        .into_iter()
        .map(|t| t.map(|i| ids[i]))
        .filter(|&[a, b, c]| {
            a != b && b != c && c != a && (positions[b] - positions[a]).cross(positions[c] - positions[a]).magnitude2() > 0.0
        })
        .collect::<Vec<_>>();
    if tris.is_empty() {
        return None;
    }
    let reference = tris
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| positions[i]);
            (b - a).cross(c - a)
        })
        .fold(Vector3::zero(), |acc, v| acc + v);
    let otherwise = if reference.magnitude2() > 0.0 { reference.normalize() } else { Vector3::unit_z() };
    let located = positions
        .iter()
        .map(|&p| {
            sp(surface, p, None)
                .map(|(u, v)| ((u, v), surface.normal(u, v)))
                .filter(|(_, n)| is_finite_vector(*n) && n.magnitude2() > 0.0)
        })
        .collect::<Vec<_>>();
    let mut uvs = located.iter().map(|l| l.map(|(uv, _)| uv)).collect::<Vec<_>>();
    let mut normals = located
        .iter()
        .map(|l| l.map_or(otherwise, |(_, n)| n))
        .collect::<Vec<_>>();

    // Bisect the longest interior edge of every triangle whose centroid is off the surface.
    // An edge with a single triangle is a shared boundary sample chord and never moves.
    let threshold = tolerance * DEVIATION_FRACTION;
    let mut searches = 0usize;
    let nearest = |searches: &mut usize, point: Point3, hint: Option<(f64, f64)>, reach: f64| {
        *searches += 1;
        let (u, v) = sp(surface, point, hint)?;
        let on_surface = surface.subs(u, v);
        let normal = surface.normal(u, v);
        (on_surface.distance(point) <= reach && is_finite_vector(normal) && normal.magnitude2() > 0.0)
            .then_some((on_surface, normal, (u, v)))
    };
    // A triangle's check depends only on its corners, which never move, so a triangle that
    // already passed (or was ruled to belong to another sheet) is not searched again.
    let mut settled = HashMap::<[usize; 3], ()>::default();
    let key = |ids: [usize; 3]| {
        let mut sorted = ids;
        sorted.sort_unstable();
        sorted
    };
    for _ in 0..REFINEMENT_ROUNDS {
        let mut edge_uses = HashMap::<(usize, usize), usize>::default();
        for t in &tris {
            for i in 0..3 {
                let (a, b) = (t[i], t[(i + 1) % 3]);
                *edge_uses.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        let mut consumed = vec![false; tris.len()];
        let mut edge_splits = Vec::<((usize, usize), Point3, Vector3, (f64, f64))>::new();
        let mut centroid_splits = Vec::<(usize, Point3, Vector3, (f64, f64))>::new();
        let triangles_of_edge = |edge: (usize, usize), tris: &[[usize; 3]]| {
            (0..tris.len())
                .filter(|&t| tris[t].contains(&edge.0) && tris[t].contains(&edge.1))
                .collect::<Vec<_>>()
        };
        for t in 0..tris.len() {
            if positions.len() + edge_splits.len() + centroid_splits.len() >= MAX_VERTICES
                || searches >= MAX_SURFACE_SEARCHES
            {
                break;
            }
            let ids = tris[t];
            if settled.contains_key(&key(ids)) {
                continue;
            }
            let corners = ids.map(|i| positions[i]);
            let centroid =
                Point3::from_vec((corners[0].to_vec() + corners[1].to_vec() + corners[2].to_vec()) / 3.0);
            let lengths = [0, 1, 2].map(|i| corners[i].distance(corners[(i + 1) % 3]));
            let scale = lengths.into_iter().fold(0.0, f64::max);
            let hint = (0..3)
                .filter(|&i| uvs[ids[i]].is_some())
                .min_by(|&a, &b| corners[a].distance2(centroid).total_cmp(&corners[b].distance2(centroid)))
                .and_then(|i| uvs[ids[i]]);
            // A nearest point far beyond the triangle belongs to another sheet.
            let Some((on_surface, normal, uv)) = nearest(&mut searches, centroid, hint, scale) else {
                settled.insert(key(ids), ());
                continue;
            };
            if on_surface.distance(centroid) <= threshold {
                settled.insert(key(ids), ());
                continue;
            }
            let mut order = [0usize, 1, 2];
            order.sort_by(|&a, &b| lengths[b].total_cmp(&lengths[a]));
            let interior = order.into_iter().find(|&i| {
                let (a, b) = (ids[i], ids[(i + 1) % 3]);
                edge_uses[&(a.min(b), a.max(b))] == 2
            });
            match interior {
                Some(i) => {
                    let (a, b) = (ids[i], ids[(i + 1) % 3]);
                    let edge = (a.min(b), a.max(b));
                    if edge_splits.iter().any(|(e, ..)| *e == edge) {
                        continue;
                    }
                    let adjacent = triangles_of_edge(edge, &tris);
                    if adjacent.iter().any(|&u| consumed[u]) {
                        continue;
                    }
                    let midpoint = Point3::from_vec((corners[i].to_vec() + corners[(i + 1) % 3].to_vec()) * 0.5);
                    let hint = uvs[a].or(uvs[b]);
                    if let Some((point, normal, uv)) = nearest(&mut searches, midpoint, hint, lengths[i]) {
                        adjacent.into_iter().for_each(|u| consumed[u] = true);
                        edge_splits.push((edge, point, normal, uv));
                    }
                }
                None => {
                    consumed[t] = true;
                    centroid_splits.push((t, on_surface, normal, uv));
                }
            }
        }
        if edge_splits.is_empty() && centroid_splits.is_empty() {
            break;
        }
        let mut next = Vec::with_capacity(tris.len() + 2 * edge_splits.len());
        let mut replaced = vec![false; tris.len()];
        for (edge, point, normal, uv) in edge_splits {
            let m = positions.len();
            positions.push(point);
            normals.push(normal);
            uvs.push(Some(uv));
            for t in triangles_of_edge(edge, &tris) {
                replaced[t] = true;
                let ids = tris[t];
                let i = (0..3)
                    .find(|&i| {
                        let (a, b) = (ids[i], ids[(i + 1) % 3]);
                        (a.min(b), a.max(b)) == edge
                    })
                    .unwrap_or(0);
                let (x, y, w) = (ids[i], ids[(i + 1) % 3], ids[(i + 2) % 3]);
                next.extend([[x, m, w], [m, y, w]]);
            }
        }
        for (t, point, normal, uv) in centroid_splits {
            let p = positions.len();
            positions.push(point);
            normals.push(normal);
            uvs.push(Some(uv));
            replaced[t] = true;
            let [a, b, c] = tris[t];
            next.extend([[a, b, p], [b, c, p], [c, a, p]]);
        }
        next.extend((0..tris.len()).filter(|&t| !replaced[t]).map(|t| tris[t]));
        tris = next;
    }
    Some(into_mesh(positions, normals, tris))
}

/// Last resort when no usable polygon exists: a fan over the shared samples.
fn fan_triangulation<S: PreMeshableSurface>(
    surface: &S,
    loops: &[Vec<Point3>],
    sp: &impl SP<S>,
) -> Option<PolygonMesh> {
    let mut best = loops
        .iter()
        .map(|raw| {
            let mut ring = raw.clone();
            ring.dedup();
            while ring.len() > 1 && ring.first() == ring.last() {
                ring.pop();
            }
            ring
        })
        .filter(|ring| ring.len() >= 3);
    let first = best.next()?;
    let ring = best.fold(first, |acc, ring| if ring.len() > acc.len() { ring } else { acc });
    let area = area_vector(&ring);
    let otherwise = if area.magnitude2() > 0.0 { area.normalize() } else { Vector3::unit_z() };
    let normals = ring
        .iter()
        .map(|&p| surface_normal(surface, sp, p, otherwise))
        .collect::<Vec<_>>();
    let tris = (1..ring.len() - 1).map(|i| [0, i, i + 1]).collect();
    Some(into_mesh(ring, normals, tris))
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_geometry::prelude::Plane;

    fn sp(surface: &Plane, point: Point3, _: Option<(f64, f64)>) -> Option<(f64, f64)> {
        surface.search_parameter(point, None, 100)
    }

    fn pts(raw: &[[f64; 2]]) -> Vec<Point3> {
        raw.iter().map(|p| Point3::new(p[0], p[1], 0.0)).collect()
    }

    fn has_every_point(mesh: &PolygonMesh, ring: &[Point3]) -> bool {
        ring.iter().all(|p| mesh.positions().iter().any(|q| q == p))
    }

    #[test]
    fn square_with_a_pole_style_slit_is_covered_with_exact_boundary_points() {
        // Out-and-back slit from the corner, as a collapsed pole edge leaves behind.
        let mut ring = pts(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0], [0.0, 0.0]]);
        ring.extend(pts(&[[1.0, 1.0], [2.0, 2.0], [2.0, 2.0], [1.0, 1.0]]));
        let mesh = preview_boundary_triangulation(&Plane::xy(), &[ring.clone()], 0.1, &sp).unwrap();
        assert!(has_every_point(&mesh, &ring[..4]));
        let area: f64 = mesh
            .faces()
            .triangle_iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| mesh.positions()[v.pos]);
                (b - a).cross(c - a).magnitude() * 0.5
            })
            .sum();
        assert!((area - 16.0).abs() < 1e-9, "area {area}");
    }

    #[test]
    fn sub_tolerance_lens_still_gets_triangles_on_its_shared_samples() {
        // Two chords whose endpoints agree to the last ulps: a zero-width digon.
        let ring = vec![
            Point3::new(0.3, 0.1, 0.0),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::EPSILON * 1e3, 0.0, 0.0),
            Point3::new(0.3 + 1e-15, 0.1, 0.0),
        ];
        let mesh = preview_boundary_triangulation(&Plane::xy(), &[ring.clone()], 0.1, &sp).unwrap();
        assert!(!mesh.tri_faces().is_empty());
        assert!(has_every_point(&mesh, &ring));
    }
}
