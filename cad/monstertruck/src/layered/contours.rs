use super::{Feature, Vec2, circle};
use cavalier_contours::{core::math::Vector2, polyline::*};
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CircleSource {
    pub(super) id: String,
    pub(super) center: [f64; 2],
    pub(super) radius: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Source {
    Line([f64; 2], [f64; 2]),
    Circle(CircleSource),
}
// Native Cavalier geometry plus segment lineage; no second curve representation.
#[derive(Clone)]
pub(super) struct Ring {
    pub(super) pline: Polyline<f64>,
    pub(super) sources: Vec<Option<Source>>,
}
impl std::ops::Deref for Ring {
    type Target = Polyline<f64>;
    fn deref(&self) -> &Self::Target {
        &self.pline
    }
}
impl Ring {
    pub(super) fn new_closed() -> Self {
        Self {
            pline: Polyline::new_closed(),
            sources: vec![],
        }
    }
    pub(super) fn add(&mut self, x: f64, y: f64, bulge: f64) {
        self.pline.add(x, y, bulge);
        self.sources.push(None);
    }
    pub(super) fn invert_direction_mut(&mut self) {
        self.pline.invert_direction_mut();
        self.sources.reverse();
        if !self.sources.is_empty() {
            self.sources.rotate_left(1);
        }
    }
}
const POSITION_EPS: f64 = 1e-10;
fn same_vertices(a: &Polyline<f64>, b: &Polyline<f64>) -> bool {
    a.is_closed() == b.is_closed()
        && a.vertex_count() == b.vertex_count()
        && a.iter_vertexes()
            .zip(b.iter_vertexes())
            .all(|(a, b)| a.x == b.x && a.y == b.y && a.bulge == b.bulge)
}
fn canonical_line_junctions(ring: &mut Ring) -> Result<(), String> {
    for i in 0..ring.vertex_count() {
        let previous = (i + ring.vertex_count() - 1) % ring.vertex_count();
        if let (Some(Source::Line(a, b)), Some(Source::Line(c, d))) =
            (&ring.sources[previous], &ring.sources[i])
        {
            // Canonical endpoint/source order makes the same two authored lines evaluate identically.
            let order = |a: [f64; 2], b: [f64; 2]| {
                if a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])).is_lt() {
                    (a, b)
                } else {
                    (b, a)
                }
            };
            let (a, b) = order(*a, *b);
            let (c, d) = order(*c, *d);
            let (a, b, c, d) = if a[0]
                .total_cmp(&c[0])
                .then(a[1].total_cmp(&c[1]))
                .then(b[0].total_cmp(&d[0]))
                .then(b[1].total_cmp(&d[1]))
                .is_lt()
            {
                (a, b, c, d)
            } else {
                (c, d, a, b)
            };
            // A shared authored corner is already the exact intersection; preserve its bits.
            let shared = match (a == c || a == d, b == c || b == d) {
                (true, false) => Some(a),
                (false, true) => Some(b),
                _ => None,
            };
            let point = if let Some(shared) = shared {
                shared
            } else {
                let u = [b[0] - a[0], b[1] - a[1]];
                let v = [d[0] - c[0], d[1] - c[1]];
                let determinant = u[0] * v[1] - u[1] * v[0];
                if determinant == 0. {
                    continue;
                }
                let t = ((c[0] - a[0]) * v[1] - (c[1] - a[1]) * v[0]) / determinant;
                let mut point = [a[0] + t * u[0], a[1] + t * u[1]];
                for coordinate in 0..2 {
                    if u[coordinate] == 0. {
                        point[coordinate] = a[coordinate];
                    } else if v[coordinate] == 0. {
                        point[coordinate] = c[coordinate];
                    }
                }
                point
            };
            let old = ring.at(i);
            if !point.iter().all(|value| value.is_finite())
                || (point[0] - old.x).abs() > POSITION_EPS
                || (point[1] - old.y).abs() > POSITION_EPS
            {
                return Err(
                    "authored line intersection exceeds the 2D source residual budget".into(),
                );
            }
            ring.pline
                .set_vertex(i, PlineVertex::new(point[0], point[1], old.bulge));
        }
    }
    Ok(())
}
fn retain_lineage(
    output: BooleanResultPline<Polyline<f64>>,
    a: &Ring,
    b: &Ring,
) -> Result<Ring, String> {
    if output.subslices.is_empty() {
        // Non-intersected Boolean results are unchanged whole input polylines.
        for input in [a, b] {
            if same_vertices(&output.pline, &input.pline) {
                return Ok(input.clone());
            }
        }
        return Err("2D Boolean result has no provable unchanged input lineage".into());
    }
    let mut rebuilt = Polyline::new();
    let mut sources = vec![];
    for slice in output.subslices {
        let input = if slice.source_is_pline1 { a } else { b };
        let view = slice.view(&input.pline);
        for (i, vertex) in view.iter_vertexes().enumerate() {
            let source = if i + 1 == view.vertex_count() {
                None
            } else {
                let offset = if slice.view_data.inverted_direction {
                    slice.view_data.end_index_offset - i
                } else {
                    i
                };
                input.sources[(slice.view_data.start_index + offset) % input.vertex_count()].clone()
            };
            let count = rebuilt.vertex_count();
            rebuilt.add_or_replace_vertex(vertex, POSITION_EPS);
            if rebuilt.vertex_count() == count {
                if sources.last().is_some_and(Option::is_some) {
                    return Err("2D Boolean collapsed a sourced segment during stitching".into());
                }
                *sources.last_mut().ok_or("empty lineage stitch")? = source;
            } else {
                sources.push(source);
            }
        }
    }
    rebuilt.remove_last();
    sources.pop();
    rebuilt.set_is_closed(true);
    if !same_vertices(&rebuilt, &output.pline) {
        return Err("2D Boolean slice lineage does not reproduce returned contour".into());
    }
    let mut ring = Ring {
        pline: output.pline,
        sources,
    };
    canonical_line_junctions(&mut ring)?;
    Ok(ring)
}
#[derive(Clone)]
pub(super) struct Region {
    pub outer: Ring,
    pub holes: Vec<Ring>,
}
pub(super) type Profile = Vec<Region>;
pub(super) fn validate(r: &Ring) -> Result<(), String> {
    if r.sources.len() != r.vertex_count()
        || r.iter_vertexes()
            .zip(&r.sources)
            .any(|(vertex, source)| vertex.bulge != 0. && source.is_none())
        || !r.is_closed()
        || r.vertex_count() < 2
        || r.iter_vertexes().any(|p| {
            !p.x.is_finite() || !p.y.is_finite() || !p.bulge.is_finite() || p.bulge.abs() > 1.
        })
        || r.area() == 0.
        || r.scan_for_self_intersect()
    {
        return Err("invalid or self-touching line/arc contour".into());
    }
    Ok(())
}
pub(super) fn polygon(points: &[Vec2]) -> Result<Ring, String> {
    let mut r = Ring::new_closed();
    for p in points {
        r.add(p.x, p.y, 0.);
    }
    r.sources = r
        .iter_segments()
        .map(|(a, b)| Some(Source::Line([a.x, a.y], [b.x, b.y])))
        .collect();
    validate(&r)?;
    Ok(ccw(r))
}
fn ccw(mut r: Ring) -> Ring {
    if r.area() < 0. {
        r.invert_direction_mut();
    }
    r
}
fn proven_coincident(a: &Ring, b: &Ring) -> bool {
    let circle = |ring: &Ring| -> Option<([f64; 2], f64)> {
        let Some(Some(Source::Circle(first))) = ring.sources.first() else {
            return None;
        };
        ring.sources.iter().all(|source| matches!(source,Some(Source::Circle(next)) if first.center==next.center && first.radius==next.radius)).then_some((first.center,first.radius))
    };
    if let (Some(a), Some(b)) = (circle(a), circle(b)) {
        return a == b;
    }
    a.vertex_count() == b.vertex_count()
        && (0..a.vertex_count()).any(|offset| {
            (0..a.vertex_count()).all(|i| {
                let p = a.at(i);
                let q = b.at((i + offset) % b.vertex_count());
                p.x == q.x && p.y == q.y && p.bulge == q.bulge
            })
        })
}
fn pair(
    a: &Ring,
    b: &Ring,
    op: BooleanOp,
) -> Result<(Vec<Ring>, Vec<Ring>, BooleanResultInfo), String> {
    // No collapsed-area filter; the private XY position equality budget is 1e-10 mm.
    let r: BooleanResult<Polyline<f64>> = a.pline.boolean_opt(
        &b.pline,
        op,
        &PlineBooleanOptions {
            pos_equal_eps: POSITION_EPS,
            collapsed_area_eps: None,
            ..Default::default()
        },
    );
    if matches!(r.result_info, BooleanResultInfo::Overlapping) && !proven_coincident(a, b) {
        return Err("2D Boolean reported unproved coincidence of distinct authored sources".into());
    }
    if matches!(r.result_info, BooleanResultInfo::InvalidInput) {
        return Err("2D Boolean rejected contour input".into());
    }
    let pos: Vec<_> = r
        .pos_plines
        .into_iter()
        .map(|r| retain_lineage(r, a, b).map(ccw))
        .collect::<Result<_, _>>()?;
    let neg: Vec<_> = r
        .neg_plines
        .into_iter()
        .map(|r| retain_lineage(r, a, b).map(ccw))
        .collect::<Result<_, _>>()?;
    for ring in pos.iter().chain(&neg) {
        validate(ring).map_err(|e| format!("{e}: op={op:?} info={:?}", r.result_info))?;
    }
    Ok((pos, neg, r.result_info))
}
fn contained(outer: &Ring, inner: &Ring) -> Result<bool, String> {
    let (_, _, info) = pair(outer, inner, BooleanOp::And)?;
    Ok(matches!(info, BooleanResultInfo::Pline2InsidePline1))
}
pub(super) fn from_rings(rings: Vec<(Ring, bool)>) -> Result<Profile, String> {
    for (ring, _) in &rings {
        validate(ring)?;
    }
    let mut p: Profile = rings
        .iter()
        .filter(|(_, positive)| *positive)
        .map(|(r, _)| Region {
            outer: ccw(r.clone()),
            holes: vec![],
        })
        .collect();
    for (h, positive) in &rings {
        if *positive {
            continue;
        }
        let mut owners = vec![];
        for (i, r) in p.iter().enumerate() {
            if contained(&r.outer, h)? {
                owners.push(i);
            }
        }
        let owner = owners
            .into_iter()
            .min_by(|&a, &b| p[a].outer.area().total_cmp(&p[b].outer.area()))
            .ok_or("2D hole has no fully containing material owner")?;
        for existing in &p[owner].holes {
            let (overlap, _, _) = pair(existing, h, BooleanOp::And)?;
            if !overlap.is_empty() {
                return Err("2D hole interiors overlap".into());
            }
        }
        p[owner].holes.push(ccw(h.clone()));
    }
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            let (overlap, _, info) = pair(&p[i].outer, &p[j].outer, BooleanOp::And)?;
            if overlap.is_empty() {
                continue;
            }
            let (large, small) = match info {
                BooleanResultInfo::Pline2InsidePline1 => (i, j),
                BooleanResultInfo::Pline1InsidePline2 => (j, i),
                _ => return Err("2D material exteriors overlap or touch".into()),
            };
            let mut island = false;
            for h in &p[large].holes {
                if contained(h, &p[small].outer)? {
                    island = true;
                }
            }
            if !island {
                return Err("nested material has no separating owned hole".into());
            }
        }
    }
    Ok(p)
}
pub(super) fn frame<'a>(rings: impl Iterator<Item = &'a Ring>) -> Result<Ring, String> {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for r in rings {
        let b = r.extents().ok_or("empty bounds")?;
        lo[0] = lo[0].min(b.min_x);
        lo[1] = lo[1].min(b.min_y);
        hi[0] = hi[0].max(b.max_x);
        hi[1] = hi[1].max(b.max_y);
    }
    let m = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1.);
    polygon(&[
        Vec2 {
            x: lo[0] - m,
            y: lo[1] - m,
        },
        Vec2 {
            x: hi[0] + m,
            y: lo[1] - m,
        },
        Vec2 {
            x: hi[0] + m,
            y: hi[1] + m,
        },
        Vec2 {
            x: lo[0] - m,
            y: hi[1] + m,
        },
    ])
}
fn complement(p: &Profile, frame: &Ring) -> Result<Profile, String> {
    let mut rings = vec![];
    let mut has_frame = false;
    for r in p {
        let (_, _, info) = pair(frame, &r.outer, BooleanOp::And)?;
        if matches!(info, BooleanResultInfo::Overlapping) {
            has_frame = true;
        } else {
            rings.push((r.outer.clone(), false));
        }
        for h in &r.holes {
            rings.push((h.clone(), true));
        }
    }
    if !has_frame {
        rings.push((frame.clone(), true));
    }
    from_rings(rings)
}
pub(super) fn difference(p: &Profile, cutter: &Ring) -> Result<Profile, String> {
    let mut result = Vec::new();
    for r in p {
        let mut voids = r.holes.clone();
        voids.push(ccw(cutter.clone()));
        let mut merged: Vec<Ring> = vec![];
        for mut pending in voids {
            let mut i = 0;
            while i < merged.len() {
                let (pos, neg, info) = pair(&pending, &merged[i], BooleanOp::Or)?;
                // shortcut: void unions enclosing new material need a proved multi-ring adapter.
                if !neg.is_empty() {
                    return Err("unsupported void union creates a material island".into());
                }
                if matches!(info, BooleanResultInfo::Disjoint) {
                    i += 1;
                    continue;
                }
                if pos.len() != 1 {
                    return Err("unsupported touching void union".into());
                }
                pending = pos.into_iter().next().unwrap();
                merged.remove(i);
                i = 0;
            }
            merged.push(pending);
        }
        let mut outers = vec![r.outer.clone()];
        let mut holes = vec![];
        for void in merged {
            let mut next = vec![];
            for outer in outers {
                let (pos, neg, _) = pair(&outer, &void, BooleanOp::Not)?;
                next.extend(pos);
                holes.extend(neg);
            }
            outers = next;
        }
        result.extend(from_rings(
            outers
                .into_iter()
                .map(|r| (r, true))
                .chain(holes.into_iter().map(|r| (r, false)))
                .collect(),
        )?);
    }
    Ok(result)
}
pub(super) fn union(p: &Profile, add: &Ring, frame: &Ring) -> Result<Profile, String> {
    complement(&difference(&complement(p, frame)?, add)?, frame)
}
fn intersection(a: &Profile, b: &Profile) -> Result<Profile, String> {
    let mut result = vec![];
    for ar in a {
        for br in b {
            let (pos, neg, _) = pair(&ar.outer, &br.outer, BooleanOp::And)?;
            let mut p = from_rings(
                pos.into_iter()
                    .map(|r| (r, true))
                    .chain(neg.into_iter().map(|r| (r, false)))
                    .collect(),
            )?;
            for h in ar.holes.iter().chain(&br.holes) {
                p = difference(&p, h)?;
            }
            result.extend(p);
        }
    }
    Ok(result)
}
pub(super) fn subtract_profile(a: &Profile, b: &Profile, _frame: &Ring) -> Result<Profile, String> {
    let mut result = a.clone();
    for r in b {
        let previous = result;
        result = vec![];
        for ar in &previous {
            let (pos, neg, _) = pair(&ar.outer, &r.outer, BooleanOp::Not)?;
            let mut remaining = from_rings(
                pos.into_iter()
                    .map(|r| (r, true))
                    .chain(neg.into_iter().map(|r| (r, false)))
                    .collect(),
            )?;
            for h in &ar.holes {
                remaining = difference(&remaining, h)?;
            }
            result.extend(remaining);
        }
        for h in &r.holes {
            result.extend(intersection(
                &previous,
                &vec![Region {
                    outer: h.clone(),
                    holes: vec![],
                }],
            )?);
        }
    }
    Ok(result)
}
pub(super) fn check_contained_cone(p: &Profile, f: &Feature) -> Result<(), String> {
    let (center, r0, r1) = f.cone.unwrap();
    let max = circle(center, r0.max(r1), &f.id);
    let min = circle(center, r0.min(r1), &f.id);
    let mut overlaps_exterior = false;
    for r in p {
        let (overlap, _, info) = pair(&r.outer, &max, BooleanOp::And)?;
        overlaps_exterior |= !overlap.is_empty();
        if !matches!(info, BooleanResultInfo::Pline2InsidePline1) {
            continue;
        }
        let mut valid = true;
        for h in &r.holes {
            let (overlap, _, _) = pair(h, &max, BooleanOp::And)?;
            if overlap.is_empty() {
                continue;
            }
            // A coaxial authored bore at the cone's minimum radius is contained exactly,
            // including equality; do not infer that equality from a tolerant Boolean result.
            let coaxial_bore = h.sources.iter().all(|source| source.as_ref().is_some_and(|source| matches!(source,Source::Circle(source) if source.center == center && source.radius <= r0.min(r1))));
            if !coaxial_bore {
                let (_, _, contained) = pair(&min, h, BooleanOp::And)?;
                if !matches!(contained, BooleanResultInfo::Pline2InsidePline1) {
                    valid = false;
                }
            }
        }
        if valid && r.outer.winding_number(Vector2::new(center[0], center[1])) != 0 {
            return Ok(());
        }
    }
    if overlaps_exterior {
        Err("cone crosses a vertical wall or has unproved containment".into())
    } else {
        Ok(())
    }
}
