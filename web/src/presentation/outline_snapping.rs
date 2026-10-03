//! Private port of the pinned React outline snap policy.
use boardstudio_core::model::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Guide {
    pub(super) id: String,
    pub(super) from: Vec2,
    pub(super) direction: Vec2,
    pub(super) label: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Snap {
    pub(super) at: Vec2,
    pub(super) guides: Vec<Guide>,
}

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub(super) anchor: Option<Vec2>,
    pub(super) previous: Option<Vec2>,
    pub(super) exclude: Option<Vec2>,
    pub(super) neighbor: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Origin {
    pub(super) reference: String,
    pub(super) center: Vec2,
    pub(super) polygon: Vec<Vec2>,
}

pub(super) fn snap_origin(
    point: Vec2,
    origins: &[Origin],
    tolerance: f64,
) -> Option<(Vec2, String)> {
    let mut best: Option<(Vec2, String)> = None;
    let mut nearest = tolerance;
    for origin in origins {
        let mut landmarks = Vec::with_capacity(1 + origin.polygon.len() * 2);
        landmarks.push(origin.center);
        landmarks.extend(origin.polygon.iter().copied());
        for index in 0..origin.polygon.len() {
            let a = origin.polygon[index];
            let b = origin.polygon[(index + 1) % origin.polygon.len()];
            landmarks.push(Vec2 {
                x: (a.x + b.x) / 2.0,
                y: (a.y + b.y) / 2.0,
            });
        }
        for to in landmarks {
            let next = distance(point, to);
            if next <= nearest {
                nearest = next;
                best = Some((
                    to,
                    format!("{} · origin / corner / midpoint", origin.reference),
                ));
            }
        }
    }
    best
}

pub(super) fn convex_hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
    let cross = |a: Vec2, b: Vec2, c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let half = |ordered: &[Vec2]| {
        let mut hull = Vec::new();
        for point in ordered.iter().copied() {
            while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0
            {
                hull.pop();
            }
            hull.push(point);
        }
        hull.pop();
        hull
    };
    if points.len() < 2 {
        return points;
    }
    let lower = half(&points);
    points.reverse();
    let upper = half(&points);
    lower.into_iter().chain(upper).collect()
}

fn distance(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn rounded(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn add(candidates: &mut Vec<Guide>, from: Vec2, direction: Vec2, label: &'static str) {
    let length = direction.x.hypot(direction.y);
    if length < 1.0e-8 {
        return;
    }
    let direction = Vec2 {
        x: direction.x / length,
        y: direction.y / length,
    };
    candidates.push(Guide {
        id: format!(
            "{label}/{}/{}/{}/{}",
            from.x, from.y, direction.x, direction.y
        ),
        from,
        direction,
        label,
    });
}

fn landmarks(candidates: &mut Vec<Guide>, point: Vec2, excluded: Option<Vec2>) {
    if excluded.is_some_and(|excluded| distance(point, excluded) < 1.0e-6) {
        return;
    }
    add(
        candidates,
        point,
        Vec2 { x: 0.0, y: 1.0 },
        "Vertical alignment",
    );
    add(
        candidates,
        point,
        Vec2 { x: 1.0, y: 0.0 },
        "Horizontal alignment",
    );
}

/// Screen-distance acquisition/release avoids zoom-dependent magnetic strength.
/// Kept in step with `app/src/ui/outlineSnapping.ts` at the pinned reference.
#[allow(clippy::too_many_arguments)]
pub(super) fn snap_outline_point(
    point: Vec2,
    context: Context,
    paths: &[Vec<Vec2>],
    grid: Vec2,
    tolerance: f64,
    enabled: bool,
    free: bool,
    previous: Option<&Snap>,
) -> Snap {
    if free {
        return Snap {
            at: point,
            guides: Vec::new(),
        };
    }
    let mut at = Vec2 {
        x: if grid.x > 0.0 {
            rounded((point.x / grid.x).round() * grid.x)
        } else {
            point.x
        },
        y: if grid.y > 0.0 {
            rounded((point.y / grid.y).round() * grid.y)
        } else {
            point.y
        },
    };
    if !enabled {
        return Snap {
            at,
            guides: Vec::new(),
        };
    }
    let mut candidates = Vec::<Guide>::new();
    for path in paths {
        if path.len() < 2 {
            continue;
        }
        for index in 0..path.len() {
            let a = path[index];
            let b = path[(index + 1) % path.len()];
            landmarks(&mut candidates, a, context.exclude);
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let length = dx * dx + dy * dy;
            if length == 0.0 {
                continue;
            }
            if context.exclude.is_some_and(|excluded| {
                distance(a, excluded) < 1.0e-6 || distance(b, excluded) < 1.0e-6
            }) {
                continue;
            }
            let t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / length;
            if (0.0..=1.0).contains(&t) && distance(a, b) > tolerance * 2.0 {
                add(&mut candidates, a, Vec2 { x: dx, y: dy }, "On edge");
            }
            if let Some(anchor) = context.anchor {
                let k = ((anchor.x - a.x) * dx + (anchor.y - a.y) * dy) / length;
                if (-1.0e-6..=1.0 + 1.0e-6).contains(&k)
                    && distance(
                        anchor,
                        Vec2 {
                            x: a.x + k * dx,
                            y: a.y + k * dy,
                        },
                    ) < 0.002
                {
                    add(&mut candidates, anchor, Vec2 { x: dx, y: dy }, "Collinear");
                    add(
                        &mut candidates,
                        anchor,
                        Vec2 { x: -dy, y: dx },
                        "Perpendicular",
                    );
                }
            }
        }
    }
    if let Some(anchor) = context.anchor {
        landmarks(&mut candidates, anchor, context.exclude);
    }
    if let Some(neighbor) = context.neighbor {
        landmarks(&mut candidates, neighbor, context.exclude);
        if let Some(excluded) = context.exclude {
            let d = Vec2 {
                x: excluded.x - neighbor.x,
                y: excluded.y - neighbor.y,
            };
            add(&mut candidates, neighbor, d, "Collinear");
            add(
                &mut candidates,
                neighbor,
                Vec2 { x: -d.y, y: d.x },
                "Perpendicular",
            );
        }
    }
    if let (Some(anchor), Some(previous_point)) = (context.anchor, context.previous) {
        let d = Vec2 {
            x: anchor.x - previous_point.x,
            y: anchor.y - previous_point.y,
        };
        add(&mut candidates, anchor, d, "Collinear");
        add(
            &mut candidates,
            anchor,
            Vec2 { x: -d.y, y: d.x },
            "Perpendicular",
        );
    }
    let projection = |guide: &Guide| {
        let t = (point.x - guide.from.x) * guide.direction.x
            + (point.y - guide.from.y) * guide.direction.y;
        Vec2 {
            x: guide.from.x + t * guide.direction.x,
            y: guide.from.y + t * guide.direction.y,
        }
    };
    let mut ranked = candidates
        .into_iter()
        .filter_map(|guide| {
            let projected = projection(&guide);
            let d = distance(point, projected);
            let held = previous
                .is_some_and(|last| last.guides.iter().any(|candidate| candidate.id == guide.id));
            (d <= tolerance * if held { 1.8 } else { 1.0 }).then_some((guide, projected, d, held))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| a.2.total_cmp(&b.2)));
    let Some((first, projected, _, _)) = ranked.first() else {
        return Snap {
            at,
            guides: Vec::new(),
        };
    };
    let first = first.clone();
    let grid_at = at;
    let second = ranked.iter().find(|(guide, _, _, _)| {
        (first.direction.x * guide.direction.y - first.direction.y * guide.direction.x).abs() > 0.1
    });
    let mut guides = vec![first.clone()];
    at = *projected;
    if first.direction.x.abs() < 1.0e-6 {
        at.y = grid_at.y;
    } else if first.direction.y.abs() < 1.0e-6 {
        at.x = grid_at.x;
    }
    if let Some((second, _, _, _)) = second {
        let denominator =
            first.direction.x * second.direction.y - first.direction.y * second.direction.x;
        let t = ((second.from.x - first.from.x) * second.direction.y
            - (second.from.y - first.from.y) * second.direction.x)
            / denominator;
        let intersection = Vec2 {
            x: first.from.x + t * first.direction.x,
            y: first.from.y + t * first.direction.y,
        };
        if distance(intersection, point) <= tolerance * 2.6 {
            at = intersection;
            guides.push(second.clone());
        }
    }
    Snap {
        at: Vec2 {
            x: rounded(at.x),
            y: rounded(at.y),
        },
        guides,
    }
}
