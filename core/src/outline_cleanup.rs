//! Bounded exterior pockets are repaired once, before authored differences.
//! The span is the mouth chord, not the recess depth or a global hull radius.
use super::{Path, Shapes, automatic, point, snap};
use crate::{model::*, outline_controls};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use std::collections::BTreeSet;

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn area(path: &Path) -> f64 {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .take(path.len())
        .map(|(a, b)| cross(*a, *b))
        .sum::<f64>()
        / 2.0
}
fn contains(path: &Path, p: [f64; 2]) -> bool {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .take(path.len())
        .fold(false, |inside, (a, b)| {
            inside
                ^ ((a[1] > p[1]) != (b[1] > p[1])
                    && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0])
        })
}
fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = sub(b, a);
    let t = (dot(sub(p, a), d) / dot(d, d)).clamp(0.0, 1.0);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

pub(super) fn validate(settings: &OutlineRepairSettings) -> Result<(), String> {
    if !settings.maximum_gap_span.is_finite()
        || settings.maximum_gap_span < 0.0
        || !settings.minimum_connection_width.is_finite()
        || settings.minimum_connection_width < 0.0
        || !settings.edge_clearance.is_finite()
        || settings.edge_clearance < 0.0
    {
        return Err(
            "Gap span, minimum connection width and edge clearance must be finite and nonnegative"
                .into(),
        );
    }
    Ok(())
}

fn owner<'a>(p: [f64; 2], envelopes: &'a [(&str, Shapes)]) -> Option<&'a str> {
    envelopes
        .iter()
        .map(|(id, shapes)| {
            let d = shapes
                .iter()
                .map(|shape| {
                    if contains(&shape[0], p) {
                        return 0.0;
                    }
                    shape[0]
                        .iter()
                        .zip(shape[0].iter().cycle().skip(1))
                        .take(shape[0].len())
                        .map(|(a, b)| distance(p, *a, *b))
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min);
            (*id, d)
        })
        .min_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.cmp(b)))
        .map(|(id, _)| id)
}

fn control(doc: &ProjectDoc, p: [f64; 2], envelopes: &[(&str, Shapes)]) -> OutlineControlPoint {
    let part = owner(p, envelopes).and_then(|id| doc.parts.iter().find(|part| part.id == id));
    let at = Vec2 { x: p[0], y: p[1] };
    if let Some(part) = part {
        let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
        let dx = at.x - part.pose.at.x;
        let dy = at.y - part.pose.at.y;
        let x = dx * cos + dy * sin;
        OutlineControlPoint {
            at: Vec2 {
                x: snap(if part.side == Side::Back { -x } else { x }),
                y: snap(-dx * sin + dy * cos),
            },
            part_id: Some(part.id.clone()),
        }
    } else {
        OutlineControlPoint { at, part_id: None }
    }
}

// Deterministic identity in component frames, independent of contour start/order.
fn identity(prefix: &str, points: &[OutlineControlPoint]) -> String {
    let mut frames: Vec<_> = points
        .iter()
        .map(|p| {
            format!(
                "{}:{:.3},{:.3}",
                p.part_id.as_deref().unwrap_or("world"),
                p.at.x,
                p.at.y
            )
        })
        .collect();
    frames.sort();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in frames.join("|").bytes() {
        hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
    }
    format!("{prefix}:{hash:016x}")
}

pub(super) fn bridge_metadata(
    doc: &ProjectDoc,
    feature: &str,
    authored: Option<&str>,
    width: f64,
    path: &Path,
    ends: &[Vec2],
    envelopes: &[(&str, Shapes)],
) -> OutlineBridge {
    let controls: Vec<_> = ends
        .iter()
        .map(|p| control(doc, [p.x, p.y], envelopes))
        .collect();
    let part_ids: Vec<_> = controls
        .iter()
        .filter_map(|p| p.part_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let matrix_ids =
        doc.matrices
            .iter()
            .filter(|matrix| {
                part_ids.iter().any(|id| {
                    matrix.part_ids.contains(id)
                        || doc.layouts.iter().any(|layout| {
                            layout.matrix_id == matrix.id && layout.part_ids.contains(id)
                        })
                })
            })
            .map(|matrix| matrix.id.clone())
            .collect();
    OutlineBridge {
        id: authored
            .map(|id| format!("{feature}:bridge:{id}"))
            .unwrap_or_else(|| identity(&format!("{feature}:bridge"), &controls)),
        width,
        points: path
            .iter()
            .map(|p| Vec2 {
                x: snap(p[0]),
                y: snap(p[1]),
            })
            .collect(),
        part_ids,
        matrix_ids,
        authored: authored.is_some(),
    }
}

fn pockets(path: &Path, maximum: f64) -> Vec<Path> {
    let mut path = path.clone();
    if area(&path) < 0.0 {
        path.reverse();
    }
    let n = path.len();
    let mut result = vec![];
    // Evaluate original mouths once: repairs never grow across a wider valley.
    for start in 0..n {
        let a = path[start];
        let incoming = sub(a, path[(start + n - 1) % n]);
        let first = sub(path[(start + 1) % n], a);
        if cross(incoming, first) <= 1e-8 {
            continue;
        }
        for count in 2..n - 1 {
            let end = (start + count) % n;
            let b = path[end];
            let chord = sub(b, a);
            let span = chord[0].hypot(chord[1]);
            if span < 0.001 || span > maximum + 1e-8 || dot(incoming, chord) <= 1e-8 {
                continue;
            }
            let outgoing = sub(path[(end + 1) % n], b);
            let last = sub(b, path[(end + n - 1) % n]);
            if dot(outgoing, chord) <= 1e-8 || cross(last, outgoing) <= 1e-8 {
                continue;
            }
            let pocket: Path = (0..=count).map(|i| path[(start + i) % n]).collect();
            // Shallow stagger steps are design alternatives, not structural slots.
            // The retained red examples have depth/span >= 0.47; blue steps <= 0.23.
            let depth = pocket
                .iter()
                .map(|p| cross(chord, sub(*p, a)).abs() / span)
                .fold(0.0, f64::max);
            if depth < span * 0.25 {
                continue;
            }
            if area(&pocket) >= -0.001
                || pocket[1..count]
                    .iter()
                    .any(|p| cross(chord, sub(*p, a)) <= 1e-8)
                || contains(&path, [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0])
                || !automatic::simple(&pocket)
            {
                continue;
            }
            // Reject chords crossing another boundary or enclosing existing material.
            let intersection = vec![vec![pocket.clone()]].overlay(
                &vec![vec![path.clone()]],
                OverlayRule::Intersect,
                FillRule::EvenOdd,
            );
            if intersection
                .iter()
                .flatten()
                .map(area)
                .map(f64::abs)
                .sum::<f64>()
                > 0.001
            {
                continue;
            }
            result.push(pocket);
        }
    }
    result.sort_by(|a, b| area(b).abs().total_cmp(&area(a).abs()));
    result
}

pub(super) fn repair(
    doc: &ProjectDoc,
    feature: &str,
    shapes: &mut Shapes,
    settings: &OutlineRepairSettings,
    envelopes: &[(&str, Shapes)],
) -> Result<Vec<OutlineGap>, String> {
    let protected: Vec<Path> = settings
        .keep_gaps
        .iter()
        .map(|gap| {
            let path = gap
                .points
                .iter()
                .map(|p| outline_controls::point(doc, p).map(point))
                .collect::<Result<Path, _>>()?;
            if !automatic::simple(&path) {
                return Err(format!(
                    "Protected gap {} no longer has a valid source region",
                    gap.id
                ));
            }
            Ok(path)
        })
        .collect::<Result<_, String>>()?;
    let candidates: Vec<_> = shapes
        .iter()
        .flat_map(|shape| pockets(&shape[0], settings.maximum_gap_span))
        .collect();
    let mut gaps = vec![];
    let mut accepted: Shapes = vec![];
    for path in candidates {
        let region = vec![vec![path.clone()]];
        if !accepted
            .overlay(&region, OverlayRule::Intersect, FillRule::EvenOdd)
            .is_empty()
        {
            continue;
        }
        let points: Vec<_> = path.iter().map(|p| control(doc, *p, envelopes)).collect();
        let id = identity(&format!("{feature}:gap"), &points);
        let protected_ids: Vec<_> = settings
            .keep_gaps
            .iter()
            .zip(&protected)
            .filter(|(gap, p)| {
                gap.id == id
                    || !region
                        .overlay(
                            &vec![vec![(*p).clone()]],
                            OverlayRule::Intersect,
                            FillRule::EvenOdd,
                        )
                        .is_empty()
            })
            .map(|(gap, _)| gap.id.clone())
            .collect();
        let keep = !protected_ids.is_empty();
        let a = path[0];
        let b = *path.last().unwrap();
        gaps.push(OutlineGap {
            id,
            feature_id: feature.into(),
            span: snap((a[0] - b[0]).hypot(a[1] - b[1])),
            points,
            protected: keep,
            protected_ids,
        });
        accepted = accepted.overlay(&region, OverlayRule::Union, FillRule::EvenOdd);
        if settings.enabled && !keep {
            *shapes = shapes.overlay(&region, OverlayRule::Union, FillRule::EvenOdd);
        }
    }
    for path in protected {
        *shapes = shapes.overlay(
            &vec![vec![path]],
            OverlayRule::Difference,
            FillRule::EvenOdd,
        );
    }
    Ok(gaps)
}
