//! Automatic perimeter construction in board coordinates. Authored cutouts are composed later.
use super::{
    FeatureShape, MAX_ARC_SEGMENTS, MAX_CHORD_ERROR_MM, Path, SCALE, Shapes, cleanup, rect, snap,
};
use crate::model::{CornerStyle, OutlineSettings, PartKind, Pose2, ProjectDoc, Vec2};
use boardstudio_contracts::{path_contains, path_signed_area};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    mesh::float::{
        outline::offset::OutlineOffset,
        style::{LineJoin, OutlineStyle},
    },
};
use std::collections::{BTreeMap, BTreeSet};

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn length(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}
fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    cross(sub(b, a), sub(p, a)).abs() < 1e-8
        && (0..2).all(|i| p[i] >= a[i].min(b[i]) - 1e-8 && p[i] <= a[i].max(b[i]) + 1e-8)
}

pub(super) const MAX_PERSISTED_OUTLINE_POINTS: usize = 1024;
type GridPoint = [i64; 2];

fn grid_point(point: [f64; 2]) -> Option<GridPoint> {
    let [x, y] = point.map(|value| value * SCALE);
    let limit = (1_i64 << 61) as f64;
    (x.is_finite() && y.is_finite() && x.abs() <= limit && y.abs() <= limit)
        .then(|| [x.round() as i64, y.round() as i64])
}

fn orient(a: GridPoint, b: GridPoint, c: GridPoint) -> Option<i128> {
    let ab_x = b[0] as i128 - a[0] as i128;
    let ab_y = b[1] as i128 - a[1] as i128;
    let ac_x = c[0] as i128 - a[0] as i128;
    let ac_y = c[1] as i128 - a[1] as i128;
    ab_x.checked_mul(ac_y)?.checked_sub(ab_y.checked_mul(ac_x)?)
}

fn grid_on_segment(a: GridPoint, b: GridPoint, p: GridPoint) -> bool {
    orient(a, b, p) == Some(0)
        && (0..2).all(|axis| p[axis] >= a[axis].min(b[axis]) && p[axis] <= a[axis].max(b[axis]))
}

fn segments_intersect(a: GridPoint, b: GridPoint, c: GridPoint, d: GridPoint) -> bool {
    let (Some(ab_c), Some(ab_d), Some(cd_a), Some(cd_b)) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    ) else {
        return true;
    };
    ((ab_c < 0 && ab_d > 0 || ab_c > 0 && ab_d < 0)
        && (cd_a < 0 && cd_b > 0 || cd_a > 0 && cd_b < 0))
        || grid_on_segment(a, b, c)
        || grid_on_segment(a, b, d)
        || grid_on_segment(c, d, a)
        || grid_on_segment(c, d, b)
}

fn adjacent_edges_meet_only_at(
    a: GridPoint,
    b: GridPoint,
    c: GridPoint,
    d: GridPoint,
    shared: GridPoint,
) -> bool {
    let other_a = if a == shared { b } else { a };
    let other_b = if c == shared { d } else { c };
    !grid_on_segment(a, b, other_b) && !grid_on_segment(c, d, other_a)
}

// shortcut: x-overlapping boxes can still produce quadratic candidate work; persisted rings stop at 1024 points, upgrade if generated outlines profile too slowly.
pub(super) fn simple(p: &Path) -> bool {
    if p.len() < 3 || p.iter().flatten().any(|x| !x.is_finite()) || path_signed_area(p).abs() < 1e-6
    {
        return false;
    }
    let Some(points): Option<Vec<_>> = p.iter().copied().map(grid_point).collect() else {
        return false;
    };
    let mut edges: Vec<_> = (0..points.len())
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            (
                i,
                a,
                b,
                a[0].min(b[0]),
                a[0].max(b[0]),
                a[1].min(b[1]),
                a[1].max(b[1]),
            )
        })
        .collect();
    edges.sort_unstable_by_key(|edge| (edge.3, edge.5, edge.0));
    for i in 0..edges.len() {
        let (edge_i, a, b, _, max_x, min_y, max_y) = edges[i];
        if a == b {
            return false;
        }
        for &(edge_j, c, d, min_x, _, other_min_y, other_max_y) in edges.iter().skip(i + 1) {
            if min_x > max_x {
                break;
            }
            if other_min_y > max_y || other_max_y < min_y {
                continue;
            }
            if !segments_intersect(a, b, c, d) {
                continue;
            }
            let adjacent =
                (edge_i + 1) % points.len() == edge_j || (edge_j + 1) % points.len() == edge_i;
            if !adjacent {
                return false;
            }
            let shared = if (edge_i + 1) % points.len() == edge_j {
                b
            } else {
                a
            };
            if !adjacent_edges_meet_only_at(a, b, c, d, shared) {
                return false;
            }
        }
    }
    true
}

pub(super) fn check_persisted_path_size(point_count: usize) -> Result<(), String> {
    if point_count > MAX_PERSISTED_OUTLINE_POINTS {
        return Err(format!(
            "Outline paths are limited to {MAX_PERSISTED_OUTLINE_POINTS} points for validation"
        ));
    }
    Ok(())
}

pub(super) fn persisted_simple(p: &Path) -> Result<bool, String> {
    check_persisted_path_size(p.len())?;
    Ok(simple(p))
}

fn nearest(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let d = sub(b, a);
    let v = sub(p, a);
    let t = ((v[0] * d[0] + v[1] * d[1]) / (d[0] * d[0] + d[1] * d[1])).clamp(0.0, 1.0);
    [a[0] + t * d[0], a[1] + t * d[1]]
}
fn bridge(shapes: &Shapes, width: f64) -> (Path, [[f64; 2]; 2]) {
    let mut best = (f64::INFINITY, [0.0; 2], [0.0; 2]);
    for i in 0..shapes.len() {
        for j in i + 1..shapes.len() {
            for (a, b) in [
                (&shapes[i][0], &shapes[j][0]),
                (&shapes[j][0], &shapes[i][0]),
            ] {
                let samples = a
                    .iter()
                    .zip(a.iter().cycle().skip(1))
                    .take(a.len())
                    .map(|(p, q)| [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0])
                    .chain(a.iter().copied());
                for p in samples {
                    for (q, r) in b.iter().zip(b.iter().cycle().skip(1)).take(b.len()) {
                        let n = nearest(p, *q, *r);
                        let distance = length(sub(n, p));
                        if distance < best.0 {
                            best = (distance, p, n);
                        }
                    }
                }
            }
        }
    }
    let (distance, a, b) = best;
    if distance < 1e-9 {
        // A point contact still needs a web with positive area.
        return (
            rect(Vec2 { x: a[0], y: a[1] }, Vec2 { x: width, y: width }, 0.0),
            [a, b],
        );
    }
    let u = [(b[0] - a[0]) / distance, (b[1] - a[1]) / distance];
    let v = [-u[1] * width / 2.0, u[0] * width / 2.0];
    // Extend into both islands so quantization cannot leave a point-only connection.
    let a = [a[0] - u[0] * width / 2.0, a[1] - u[1] * width / 2.0];
    let b = [b[0] + u[0] * width / 2.0, b[1] + u[1] * width / 2.0];
    (
        vec![
            [a[0] - v[0], a[1] - v[1]],
            [b[0] - v[0], b[1] - v[1]],
            [b[0] + v[0], b[1] + v[1]],
            [a[0] + v[0], a[1] + v[1]],
        ],
        [best.1, best.2],
    )
}
pub(super) fn envelope(
    doc: &ProjectDoc,
    feature_id: &str,
    ids: &[String],
    margin: f64,
    settings: &OutlineSettings,
    connections: &[crate::model::OutlineConnection],
) -> Result<FeatureShape, String> {
    let repair = settings.repair.clone().unwrap_or_default();
    cleanup::validate(&repair)?;
    if !margin.is_finite()
        || margin < 0.0
        || !settings.bridge_width.is_finite()
        || settings.bridge_width < 0.001
    {
        return Err(
            "Margins must be nonnegative and bridge width must be at least 0.001 mm".into(),
        );
    }
    let members: BTreeSet<_> = ids.iter().collect();
    let defs: BTreeMap<_, _> = doc.definitions.iter().map(|d| (&d.id, d)).collect();
    let selected: Vec<_> = doc
        .parts
        .iter()
        .filter(|p| {
            members.contains(&p.id)
                && !p.outline.as_ref().is_some_and(|o| o.excluded)
                && defs
                    .get(&p.definition_id)
                    .is_some_and(|definition| definition.kind != PartKind::Utility)
        })
        .collect();
    let boards: BTreeSet<_> = doc
        .boards
        .iter()
        .filter(|b| selected.iter().any(|p| b.part_ids.contains(&p.id)))
        .map(|b| &b.id)
        .collect();
    if boards.len() > 1 {
        return Err("An automatic outline cannot bridge separate boards".into());
    }
    let mut shapes: Shapes = vec![];
    let mut notices = vec![];
    let mut fallback_definitions = BTreeSet::new();
    let mut envelopes = vec![];
    // Ticket 12: envelopes read the evaluated board pose, so half-frame
    // parts outline where the canvas draws them.
    let poses: BTreeMap<&str, Pose2> = selected
        .iter()
        .map(|p| {
            (
                p.id.as_str(),
                crate::matrix::layout::evaluated_part_pose(doc, p),
            )
        })
        .collect();
    for p in &selected {
        let d = defs
            .get(&p.definition_id)
            .ok_or("Outline part definition is missing")?;
        let keycap_size = if d.kind == PartKind::Switch {
            p.keycap.or(d.keycap)
        } else {
            None
        };
        let mut path = if let Some(size) = keycap_size {
            if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                return Err("Keycap dimensions must be positive".into());
            }
            rect(Vec2::default(), size, 0.0)
        } else {
            if d.kind == PartKind::Switch && fallback_definitions.insert(&d.id) {
                notices.push(format!(
                    "{}: keycap dimensions are missing; using the courtyard",
                    d.name
                ));
            }
            check_persisted_path_size(d.courtyard.len())
                .map_err(|error| format!("{}: {error}", p.reference))?;
            d.courtyard.iter().map(|p| [p.x, p.y]).collect()
        };
        for point in &mut path {
            point[0] = snap(point[0]);
            point[1] = snap(point[1]);
        }
        if matches!(p.side, crate::model::Side::Back) {
            for point in &mut path {
                point[0] = -point[0];
            }
        }
        if !persisted_simple(&path).map_err(|error| format!("{}: {error}", p.reference))? {
            return Err(format!(
                "{}: outline envelope must be a simple polygon",
                p.reference
            ));
        }
        let pose = poses.get(p.id.as_str()).copied().unwrap_or(p.pose);
        let (sin, cos) = pose.rotation.to_radians().sin_cos();
        for q in &mut path {
            let [x, y] = *q;
            *q = [
                snap(pose.at.x + x * cos - y * sin),
                snap(pose.at.y + x * sin + y * cos),
            ];
        }
        if path_signed_area(&path) < 0.0 {
            path.reverse();
        }
        let m = p.outline.as_ref().and_then(|o| o.margin).unwrap_or(margin);
        if !m.is_finite() || m < 0.0 {
            return Err("Component margin must be nonnegative".into());
        }
        let expanded = if m == 0.0 {
            vec![vec![path]]
        } else {
            path.outline_fixed_scale(&OutlineStyle::new(m).line_join(LineJoin::Miter(0.1)), SCALE)
                .map_err(|_| "Outline offset is outside supported coordinate range")?
        };
        shapes = shapes.overlay(&expanded, OverlayRule::Union, FillRule::NonZero);
        envelopes.push((p.id.as_str(), expanded));
    }
    if shapes.is_empty() {
        return Err("Outline has no included physical envelopes".into());
    }
    // Authored paths join islands before the nearest automatic bridges are chosen.
    let mut bridges = vec![];
    for connection in connections {
        let points = connection
            .points
            .iter()
            .map(|point| crate::outline_controls::point(doc, point))
            .collect::<Result<Vec<_>, _>>()?;
        let mut corridors: Shapes = vec![];
        for pair in points.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let distance = (b.x - a.x).hypot(b.y - a.y);
            if distance < 0.001 {
                return Err("Bridge points must be distinct at board precision".into());
            }
            let ux = (b.x - a.x) / distance;
            let uy = (b.y - a.y) / distance;
            let h = connection.width / 2.0;
            let path = vec![
                [a.x - ux * h - uy * h, a.y - uy * h + ux * h],
                [b.x + ux * h - uy * h, b.y + uy * h + ux * h],
                [b.x + ux * h + uy * h, b.y + uy * h - ux * h],
                [a.x - ux * h + uy * h, a.y - uy * h - ux * h],
            ];
            shapes = shapes.overlay(&vec![vec![path]], OverlayRule::Union, FillRule::NonZero);
            corridors = corridors.overlay(
                &vec![vec![vec![
                    [a.x - ux * h - uy * h, a.y - uy * h + ux * h],
                    [b.x + ux * h - uy * h, b.y + uy * h + ux * h],
                    [b.x + ux * h + uy * h, b.y + uy * h - ux * h],
                    [a.x - ux * h + uy * h, a.y - uy * h - ux * h],
                ]]],
                OverlayRule::Union,
                FillRule::NonZero,
            );
        }
        if let Some(path) = corridors.first().and_then(|shape| shape.first()) {
            bridges.push(cleanup::bridge_metadata(
                doc,
                feature_id,
                Some(&connection.id),
                connection.width,
                path,
                &points,
                &envelopes,
            ));
        }
    }
    while shapes.len() > 1 {
        let width = if repair.enabled {
            settings.bridge_width.max(repair.minimum_connection_width)
        } else {
            settings.bridge_width
        };
        let (web, ends) = bridge(&shapes, width);
        bridges.push(cleanup::bridge_metadata(
            doc,
            feature_id,
            None,
            width,
            &web,
            &ends.map(|p| Vec2 { x: p[0], y: p[1] }),
            &envelopes,
        ));
        let count = shapes.len();
        shapes = shapes.overlay(&web, OverlayRule::Union, FillRule::NonZero);
        if shapes.len() >= count {
            return Err(
                "Unable to connect outline groups; adjust bridge width or placement".into(),
            );
        }
    }
    // Only automatic voids are filled; later authored differences remain holes.
    for shape in &mut shapes {
        shape.truncate(1);
    }
    // Repair an existing narrow neck before protections/cutouts are applied.
    // A protected conflict remains visible and is reported by active-outline validation.
    if repair.enabled && repair.minimum_connection_width > 0.0 {
        for _ in 0..selected.len() {
            let style = OutlineStyle::new(-repair.minimum_connection_width / 2.0)
                .line_join(LineJoin::Miter(0.1));
            let cores = shapes
                .outline_fixed_scale(&style, SCALE)
                .map_err(|_| "Connection measurement is outside supported coordinates")?;
            if cores.len() <= shapes.len() {
                break;
            }
            let width = repair.minimum_connection_width + 0.01;
            let (web, ends) = bridge(&cores, width);
            let widened = shapes.overlay(&web, OverlayRule::Union, FillRule::NonZero);
            let after = widened
                .outline_fixed_scale(&style, SCALE)
                .map_err(|_| "Connection repair is outside supported coordinates")?;
            if after.len() >= cores.len() {
                break;
            }
            bridges.push(cleanup::bridge_metadata(
                doc,
                feature_id,
                None,
                width,
                &web,
                &ends.map(|p| Vec2 { x: p[0], y: p[1] }),
                &envelopes,
            ));
            shapes = widened;
        }
    }
    let gaps = cleanup::repair(doc, feature_id, &mut shapes, &repair, &envelopes)?;
    Ok(FeatureShape {
        shapes,
        notices,
        bridges,
        gaps,
    })
}
// Finishing must not make a cutout cross an exterior or another cutout.
fn rings_intersect(a: &Path, b: &Path) -> bool {
    for (p, q) in a.iter().zip(a.iter().cycle().skip(1)).take(a.len()) {
        for (r, s) in b.iter().zip(b.iter().cycle().skip(1)).take(b.len()) {
            let c1 = cross(sub(*q, *p), sub(*r, *p));
            let c2 = cross(sub(*q, *p), sub(*s, *p));
            let c3 = cross(sub(*s, *r), sub(*p, *r));
            let c4 = cross(sub(*s, *r), sub(*q, *r));
            if (c1 * c2 < 0.0 && c3 * c4 < 0.0)
                || on_segment(*p, *q, *r)
                || on_segment(*p, *q, *s)
                || on_segment(*r, *s, *p)
                || on_segment(*r, *s, *q)
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn finish(
    shapes: Shapes,
    settings: &OutlineSettings,
) -> Result<(Shapes, Option<f64>, Vec<Vec2>), String> {
    if !settings.size.is_finite() || settings.size < 0.0 {
        return Err("Corner size must be nonnegative".into());
    }
    if settings.corners == CornerStyle::Sharp || settings.size == 0.0 {
        return Ok((shapes, None, vec![]));
    }
    let mut reduced: Option<f64> = None;
    let mut locations = vec![];
    let mut result = vec![];
    for shape in shapes {
        let mut finished = vec![];
        for path in shape {
            let mut out = vec![];
            for i in 0..path.len() {
                let p = path[i];
                let a = sub(path[(i + path.len() - 1) % path.len()], p);
                let b = sub(path[(i + 1) % path.len()], p);
                let la = length(a);
                let lb = length(b);
                let u = [a[0] / la, a[1] / la];
                let v = [b[0] / lb, b[1] / lb];
                let theta = (u[0] * v[0] + u[1] * v[1]).clamp(-1.0, 1.0).acos();
                if (std::f64::consts::PI - theta).abs() < 1e-6 {
                    out.push(p);
                    continue;
                }
                let requested = if settings.corners == CornerStyle::Fillet {
                    settings.size / (theta / 2.0).tan()
                } else {
                    settings.size
                };
                let tangent = requested.min(la * 0.45).min(lb * 0.45);
                if tangent + 0.001 < requested {
                    let actual = if settings.corners == CornerStyle::Fillet {
                        tangent * (theta / 2.0).tan()
                    } else {
                        tangent
                    };
                    reduced = Some(reduced.map_or(actual, |old| old.min(actual)));
                    locations.push(Vec2 { x: p[0], y: p[1] });
                }
                let start = [p[0] + u[0] * tangent, p[1] + u[1] * tangent];
                let end = [p[0] + v[0] * tangent, p[1] + v[1] * tangent];
                if settings.corners == CornerStyle::Chamfer {
                    out.extend([start, end]);
                    continue;
                }
                let r = tangent * (theta / 2.0).tan();
                let bisector = [u[0] + v[0], u[1] + v[1]];
                let l = length(bisector);
                let center = [
                    p[0] + bisector[0] / l * r / (theta / 2.0).sin(),
                    p[1] + bisector[1] / l * r / (theta / 2.0).sin(),
                ];
                let a0 = (start[1] - center[1]).atan2(start[0] - center[0]);
                let a1 = (end[1] - center[1]).atan2(end[0] - center[0]);
                let mut sweep = a1 - a0;
                while sweep > std::f64::consts::PI {
                    sweep -= std::f64::consts::TAU;
                }
                while sweep < -std::f64::consts::PI {
                    sweep += std::f64::consts::TAU;
                }
                let step = 2.0 * (1.0 - MAX_CHORD_ERROR_MM / r).clamp(-1.0, 1.0).acos();
                let segments = ((sweep.abs() / step).ceil() as usize).clamp(1, MAX_ARC_SEGMENTS);
                for j in 0..=segments {
                    let angle = a0 + sweep * j as f64 / segments as f64;
                    out.push([center[0] + r * angle.cos(), center[1] + r * angle.sin()]);
                }
            }
            for p in &mut out {
                p[0] = snap(p[0]);
                p[1] = snap(p[1]);
            }
            out.dedup();
            if out.len() > 1 && out.first() == out.last() {
                out.pop();
            }
            if !simple(&out) {
                return Err("Corner finishing creates invalid geometry; reduce its size".into());
            }
            finished.push(out);
        }
        result.push(finished);
    }
    for shape in &result {
        if shape
            .iter()
            .skip(1)
            .any(|hole| !path_contains(&shape[0], hole[0]))
        {
            return Err(
                "Corner finishing moves a cutout outside its outline; reduce its size".into(),
            );
        }
    }
    let rings: Vec<_> = result.iter().flatten().collect();
    for (i, a) in rings.iter().enumerate() {
        if rings.iter().skip(i + 1).any(|b| rings_intersect(a, b)) {
            return Err("Corner finishing intersects another contour; reduce its size".into());
        }
    }
    Ok((result, reduced, locations))
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn sweep_rejects_crossings_and_non_adjacent_contacts() {
        assert!(!simple(&vec![
            [0.0, 0.0],
            [2.0, 2.0],
            [0.0, 2.0],
            [2.0, 0.0]
        ]));
        assert!(!simple(&vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [1.0, 0.0],
            [0.0, 2.0]
        ]));
    }

    #[test]
    fn adjacent_edges_may_continue_but_cannot_overlap() {
        assert!(simple(&vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [0.0, 2.0]
        ]));
        assert!(!simple(&vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [1.0, 0.0],
            [1.0, 2.0],
            [0.0, 2.0]
        ]));
    }

    #[test]
    fn persisted_paths_use_grid_contacts_and_a_per_path_budget() {
        assert!(!simple(&vec![
            [0.0, 0.0],
            [0.0004, 0.0],
            [1.0, 1.0],
            [0.0, 1.0]
        ]));
        assert!(!simple(&vec![
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 2.0],
            [1.0004, 0.0],
            [0.0, 2.0]
        ]));
        assert!(persisted_simple(&vec![[0.0, 0.0]; MAX_PERSISTED_OUTLINE_POINTS + 1]).is_err());
    }
}
