//! Automatic perimeter construction in board coordinates. Authored cutouts are composed later.
use super::{
    FeatureShape, MAX_ARC_SEGMENTS, MAX_CHORD_ERROR_MM, Path, SCALE, Shapes, cleanup, rect, snap,
};
use crate::model::{CornerStyle, OutlineSettings, PartKind, ProjectDoc, Vec2};
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
fn area(p: &Path) -> f64 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| cross(*a, *b))
        .sum::<f64>()
        / 2.0
}
fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    cross(sub(b, a), sub(p, a)).abs() < 1e-8
        && (0..2).all(|i| p[i] >= a[i].min(b[i]) - 1e-8 && p[i] <= a[i].max(b[i]) + 1e-8)
}
pub(super) fn simple(p: &Path) -> bool {
    if p.len() < 3 || p.iter().flatten().any(|x| !x.is_finite()) || area(p).abs() < 1e-6 {
        return false;
    }
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        if length(sub(b, a)) < 1e-6 {
            return false;
        }
        for j in i + 1..p.len() {
            if j == i + 1 || (i == 0 && j == p.len() - 1) {
                continue;
            }
            let (c, d) = (p[j], p[(j + 1) % p.len()]);
            let c1 = cross(sub(b, a), sub(c, a));
            let c2 = cross(sub(b, a), sub(d, a));
            let c3 = cross(sub(d, c), sub(a, c));
            let c4 = cross(sub(d, c), sub(b, c));
            if (c1 * c2 < 0.0 && c3 * c4 < 0.0)
                || on_segment(a, b, c)
                || on_segment(a, b, d)
                || on_segment(c, d, a)
                || on_segment(c, d, b)
            {
                return false;
            }
        }
    }
    true
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
    for p in &selected {
        let d = defs
            .get(&p.definition_id)
            .ok_or("Outline part definition is missing")?;
        let mut path = if d.kind == PartKind::Switch {
            if let Some(size) = p.keycap.or(d.keycap) {
                if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                    return Err("Keycap dimensions must be positive".into());
                }
                rect(Vec2::default(), size, 0.0)
            } else {
                if fallback_definitions.insert(&d.id) {
                    notices.push(format!(
                        "{}: keycap dimensions are missing; using the courtyard",
                        d.name
                    ));
                }
                d.courtyard.iter().map(|p| [p.x, p.y]).collect()
            }
        } else {
            d.courtyard.iter().map(|p| [p.x, p.y]).collect()
        };
        if matches!(p.side, crate::model::Side::Back) {
            for point in &mut path {
                point[0] = -point[0];
            }
        }
        if !simple(&path) {
            return Err(format!(
                "{}: outline envelope must be a simple polygon",
                p.reference
            ));
        }
        let (sin, cos) = p.pose.rotation.to_radians().sin_cos();
        for q in &mut path {
            let [x, y] = *q;
            *q = [
                snap(p.pose.at.x + x * cos - y * sin),
                snap(p.pose.at.y + x * sin + y * cos),
            ];
        }
        if area(&path) < 0.0 {
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

fn inside(path: &Path, p: [f64; 2]) -> bool {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .take(path.len())
        .fold(false, |hit, (a, b)| {
            hit ^ ((a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0])
        })
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
        if shape.iter().skip(1).any(|hole| !inside(&shape[0], hole[0])) {
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
