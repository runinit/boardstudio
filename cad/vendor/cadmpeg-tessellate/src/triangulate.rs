// SPDX-License-Identifier: Apache-2.0
//! Triangulation of a polygon with holes by ear clipping.
//!
//! Each hole is joined to the outer ring by a bridge to a visible vertex, so
//! the region becomes one ring that repeats the bridge ends. Ears are then
//! clipped until three vertices remain.

use crate::face::Refusal;
use crate::plane::{cross2, signed_area, Uv};

/// The rings of one region by vertex index: the outer ring and its holes.
pub(crate) struct Rings<'a> {
    pub(crate) points: &'a [Uv],
    pub(crate) outer: Vec<usize>,
    pub(crate) holes: Vec<Vec<usize>>,
}

fn ring_area(points: &[Uv], ring: &[usize]) -> f64 {
    signed_area(&ring.iter().map(|at| points[*at]).collect::<Vec<_>>())
}

fn two_diff(a: f64, b: f64) -> (f64, f64) {
    let diff = a - b;
    let virtual_b = diff - a;
    let err = (a - (diff - virtual_b)) - (b + virtual_b);
    (diff, err)
}

/// Kahan's FMA 2x2 determinant `a * d - b * c` combined with Knuth `two_diff`
/// shifts, avoiding catastrophic cancellation on thin diagonal rings.
fn accurate_shifted_det2(p: Uv, q: Uv, origin: Uv) -> f64 {
    let (px, px_err) = two_diff(p[0], origin[0]);
    let (py, py_err) = two_diff(p[1], origin[1]);
    let (qx, qx_err) = two_diff(q[0], origin[0]);
    let (qy, qy_err) = two_diff(q[1], origin[1]);
    let bc = py * qx;
    let err = (-py).mul_add(qx, bc);
    (px.mul_add(qy, -bc) + err) + px * qy_err + px_err * qy - py * qx_err - py_err * qx
}

fn accurate_cross2(a: Uv, b: Uv, c: Uv) -> f64 {
    accurate_shifted_det2(b, c, a)
}

fn shifted_ring_area(points: &[Uv], ring: &[usize], origin: Uv) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut twice = 0.0;
    for at in 0..ring.len() {
        twice += accurate_shifted_det2(
            points[ring[at]],
            points[ring[(at + 1) % ring.len()]],
            origin,
        );
    }
    twice
}

enum EarStep {
    Clip(usize, [usize; 3]),
    DropCollinear(usize, f64),
}

fn has_inside_vertex(
    points: &[Uv],
    ring: &[usize],
    a: usize,
    b: usize,
    c: usize,
    roundoff: f64,
) -> bool {
    ring.iter()
        .any(|p| *p != a && *p != b && *p != c && inside(points, a, b, c, *p, roundoff))
}

fn select_ear_step(points: &[Uv], ring: &[usize], roundoff: f64) -> Result<EarStep, Refusal> {
    let count = ring.len();
    let corners = |at: usize| {
        (
            ring[(at + count - 1) % count],
            ring[at],
            ring[(at + 1) % count],
        )
    };
    let mut best_exact: Option<(usize, [usize; 3], f64)> = None;
    let mut collinear: Option<(usize, f64)> = None;

    for at in 0..count {
        let (a, b, c) = corners(at);
        let turn = cross2(points[a], points[b], points[c]);
        if turn > roundoff && !has_inside_vertex(points, ring, a, b, c, roundoff) {
            return Ok(EarStep::Clip(at, [a, b, c]));
        }
        let exact_turn = accurate_cross2(points[a], points[b], points[c]);
        if exact_turn > 0.0
            && !has_inside_vertex(points, ring, a, b, c, 0.0)
            && best_exact.is_none_or(|(_, _, largest)| exact_turn > largest)
        {
            best_exact = Some((at, [a, b, c], exact_turn));
        }
        let abs_turn = turn.abs();
        if abs_turn <= roundoff && collinear.is_none_or(|(_, smallest)| abs_turn < smallest) {
            collinear = Some((at, abs_turn));
        }
    }

    if let Some((at, tri, _)) = best_exact {
        return Ok(EarStep::Clip(at, tri));
    }
    if let Some((at, area_error)) = collinear {
        return Ok(EarStep::DropCollinear(at, area_error));
    }
    Err(Refusal::new("polygon ring has no valid ear"))
}

/// Whether the segments `a`–`b` and `c`–`d` cross at a point interior to
/// both.
fn segments_cross(a: Uv, b: Uv, c: Uv, d: Uv) -> bool {
    let d1 = cross2(a, b, c);
    let d2 = cross2(a, b, d);
    let d3 = cross2(c, d, a);
    let d4 = cross2(c, d, b);
    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

fn visible(points: &[Uv], from: usize, to: usize, rings: &[&[usize]]) -> bool {
    let (a, b) = (points[from], points[to]);
    for ring in rings {
        for at in 0..ring.len() {
            let (c, d) = (ring[at], ring[(at + 1) % ring.len()]);
            // A bridge through a third vertex creates an unrepresented junction
            // in the joined ring, even when neither incident edge crosses it.
            let vertex = points[c];
            if vertex != a
                && vertex != b
                && (0..2).all(|axis| {
                    a[axis].min(b[axis]) <= vertex[axis] && vertex[axis] <= a[axis].max(b[axis])
                })
                && cross2(a, b, vertex) == 0.0
            {
                return false;
            }
            if [c, d].contains(&from) || [c, d].contains(&to) {
                continue;
            }
            if segments_cross(a, b, points[c], points[d]) {
                return false;
            }
        }
    }
    true
}

/// A joined boundary can visit a bridge endpoint several times. Select the
/// occurrence whose interior wedge contains the next bridge.
fn matches_corner(points: &[Uv], ring: &[usize], at: usize, from: usize) -> bool {
    let prev = points[ring[(at + ring.len() - 1) % ring.len()]];
    let target = points[ring[at]];
    let next = points[ring[(at + 1) % ring.len()]];
    let from = points[from];
    let left_of_incoming = cross2(prev, target, from) > 0.0;
    let left_of_outgoing = cross2(target, next, from) > 0.0;
    let enters_wedge = if cross2(prev, target, next) >= 0.0 {
        left_of_incoming && left_of_outgoing
    } else {
        left_of_incoming || left_of_outgoing
    };
    enters_wedge
        || (!ring[..at].iter().any(|&i| points[i] == target)
            && !ring[at + 1..].iter().any(|&i| points[i] == target))
}

/// Joins each hole to `outer` with a bridge.
fn bridge(
    points: &[Uv],
    mut outer: Vec<usize>,
    mut holes: Vec<Vec<usize>>,
) -> Result<Vec<usize>, Refusal> {
    // Holes farthest to the right join first, so later bridges see them.
    let rightmost = |ring: &[usize]| -> usize {
        ring.iter()
            .enumerate()
            .max_by(|a, b| {
                points[*a.1][0]
                    .total_cmp(&points[*b.1][0])
                    .then(a.1.cmp(b.1))
            })
            .map_or(0, |(position, _)| position)
    };
    holes.sort_by(|a, b| {
        let x = |ring: &Vec<usize>| points[ring[rightmost(ring)]][0];
        x(b).total_cmp(&x(a)).then(a[0].cmp(&b[0]))
    });
    for at in 0..holes.len() {
        let hole = holes[at].clone();
        let from_position = rightmost(&hole);
        let from = hole[from_position];
        let mut candidates: Vec<usize> = (0..outer.len()).collect();
        candidates.sort_by(|a, b| {
            let gap = |position: &usize| {
                let p = points[outer[*position]];
                let q = points[from];
                (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)
            };
            gap(a).total_cmp(&gap(b)).then(a.cmp(b))
        });
        let others: Vec<&[usize]> = holes[at + 1..].iter().map(Vec::as_slice).collect();
        let mut rings: Vec<&[usize]> = vec![outer.as_slice(), hole.as_slice()];
        rings.extend(others);
        let target = candidates
            .into_iter()
            .find(|&position| {
                matches_corner(points, &outer, position, from)
                    && visible(points, from, outer[position], &rings)
            })
            .ok_or_else(|| Refusal::new("an inner loop has no bridge to the outer loop"))?;
        let mut joined = outer[..=target].to_vec();
        joined.extend(hole[from_position..].iter().copied());
        joined.extend(hole[..=from_position].iter().copied());
        joined.extend(outer[target..].iter().copied());
        outer = joined;
    }
    Ok(outer)
}

fn inside(points: &[Uv], a: usize, b: usize, c: usize, p: usize, roundoff: f64) -> bool {
    let (pa, pb, pc, pp) = (points[a], points[b], points[c], points[p]);
    if pp == pa || pp == pb || pp == pc {
        return false;
    }
    let turn = |u, v, w| {
        if roundoff == 0.0 {
            accurate_cross2(u, v, w)
        } else {
            cross2(u, v, w)
        }
    };
    [turn(pa, pb, pp), turn(pb, pc, pp), turn(pc, pa, pp)]
        .into_iter()
        .all(|t| t >= -roundoff)
}

fn has_crossing_edges(points: &[Uv], ring: &[usize]) -> bool {
    let count = ring.len();
    if count < 4 {
        return false;
    }
    for i in 0..count {
        let a = points[ring[i]];
        let b = points[ring[(i + 1) % count]];
        for j in (i + 2)..count {
            if i == 0 && j + 1 == count {
                continue;
            }
            let c = points[ring[j]];
            let d = points[ring[(j + 1) % count]];
            if segments_cross(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

fn triangulate_simple(
    points: &[Uv],
    mut ring: Vec<usize>,
    expected_area: f64,
    roundoff: f64,
) -> Result<Vec<[usize; 3]>, Refusal> {
    let mut triangles = Vec::new();
    let mut dropped_area = 0.0;
    while ring.len() > 3 {
        match select_ear_step(points, &ring, roundoff)? {
            EarStep::Clip(at, tri) => {
                triangles.push(tri);
                ring.remove(at);
            }
            EarStep::DropCollinear(at, area_error) => {
                dropped_area += area_error;
                ring.remove(at);
            }
        }
    }
    if ring.len() == 3 && cross2(points[ring[0]], points[ring[1]], points[ring[2]]) > 0.0 {
        triangles.push([ring[0], ring[1], ring[2]]);
    }
    if triangles.is_empty() {
        return Err(Refusal::new("polygon ring produced no triangles"));
    }
    if expected_area > 0.0 {
        let emitted_area: f64 = triangles
            .iter()
            .map(|[a, b, c]| accurate_cross2(points[*a], points[*b], points[*c]))
            .sum();
        let tolerance = 1e-4 * expected_area + dropped_area;
        if (emitted_area - expected_area).abs() > tolerance {
            return Err(Refusal::new("triangulation failed to cover polygon area"));
        }
    }
    Ok(triangles)
}

fn triangulate_crossing_ring(
    points: &[Uv],
    mut ring: Vec<usize>,
    roundoff: f64,
) -> Result<Vec<[usize; 3]>, Refusal> {
    let mut triangles = Vec::new();
    while ring.len() > 3 {
        let count = ring.len();
        let mut best: Option<(usize, f64)> = None;
        let mut clipped = None;
        for at in 0..count {
            let (a, b, c) = (
                ring[(at + count - 1) % count],
                ring[at],
                ring[(at + 1) % count],
            );
            let turn = cross2(points[a], points[b], points[c]);
            if turn <= roundoff {
                continue;
            }
            if best.is_none_or(|(_, largest)| turn > largest) {
                best = Some((at, turn));
            }
            if !has_inside_vertex(points, &ring, a, b, c, roundoff) {
                clipped = Some(at);
                break;
            }
        }
        if clipped.is_none() && best.is_none() {
            for at in 0..count {
                let (a, b, c) = (
                    ring[(at + count - 1) % count],
                    ring[at],
                    ring[(at + 1) % count],
                );
                let turn = cross2(points[a], points[b], points[c]);
                if turn > 0.0 && best.is_none_or(|(_, largest)| turn > largest) {
                    best = Some((at, turn));
                }
            }
        }
        let at = clipped.or(best.map(|(at, _)| at)).unwrap_or(0);
        let (a, b, c) = (
            ring[(at + count - 1) % count],
            ring[at],
            ring[(at + 1) % count],
        );
        if cross2(points[a], points[b], points[c]) > 0.0 {
            triangles.push([a, b, c]);
        }
        ring.remove(at);
    }
    if ring.len() == 3 && cross2(points[ring[0]], points[ring[1]], points[ring[2]]) > 0.0 {
        triangles.push([ring[0], ring[1], ring[2]]);
    }
    if triangles.is_empty() {
        return Err(Refusal::new(
            "degenerate crossing ring produced no triangles",
        ));
    }
    Ok(triangles)
}

/// The counterclockwise triangles of a region.
pub(crate) fn triangulate(rings: Rings<'_>) -> Result<Vec<[usize; 3]>, Refusal> {
    let points = rings.points;
    let mut outer = rings.outer;
    if ring_area(points, &outer) < 0.0 {
        outer.reverse();
    }
    let holes: Vec<Vec<usize>> = rings
        .holes
        .into_iter()
        .filter(|hole| hole.len() >= 3)
        .map(|mut hole| {
            if ring_area(points, &hole) > 0.0 {
                hole.reverse();
            }
            hole
        })
        .collect();
    let expected_area = outer.first().map_or(0.0, |&first| {
        let origin = points[first];
        shifted_ring_area(points, &outer, origin)
            + holes
                .iter()
                .map(|hole| shifted_ring_area(points, hole, origin))
                .sum::<f64>()
    });
    let mut ring = bridge(points, outer, holes)?;
    ring.dedup_by(|a, b| points[*a] == points[*b]);
    while ring.len() > 3 && points[ring[0]] == points[ring[ring.len() - 1]] {
        ring.pop();
    }
    // Parameter inversion can move a collinear trim sample by a few ulps.
    // Scale the roundoff area with the region so changing length units
    // does not turn such a sample into an ear.
    let span = |axis| {
        let (low, high) = ring
            .iter()
            .map(|&i| points[i][axis])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), x| {
                (low.min(x), high.max(x))
            });
        high - low
    };
    let roundoff = 64.0 * f64::EPSILON * span(0) * span(1);
    match triangulate_simple(points, ring.clone(), expected_area, roundoff) {
        Ok(triangles) => Ok(triangles),
        Err(refusal) => {
            if has_crossing_edges(points, &ring) {
                triangulate_crossing_ring(points, ring, roundoff)
            } else {
                Err(refusal)
            }
        }
    }
}

#[cfg(test)]
mod tests;
