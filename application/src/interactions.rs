//! Application-owned part interaction geometry copied from the accepted editor policy.
use boardstudio_core::model::{Part, PartDefinition, PartKind, ProjectDoc, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct SnapGuide {
    pub at: Vec2,
    pub from: Vec2,
    pub to: Vec2,
    pub label: String,
}

const SNAP_PULL_RADIUS_PX: f64 = 8.0;
const SNAP_CAPTURE_RADIUS_PX: f64 = 3.0;
const SNAP_RELEASE_RADIUS_PX: f64 = 14.0;

#[derive(Clone, Debug, PartialEq)]
pub struct MagneticSnapTarget {
    pub at: Vec2,
    pub guide: Option<SnapGuide>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SnapSource {
    Geometry,
    Grid,
}

#[derive(Clone, Debug, PartialEq)]
struct LatchedSnap {
    source: SnapSource,
    target: MagneticSnapTarget,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MagneticSnapState {
    latched: Option<LatchedSnap>,
}

impl MagneticSnapState {
    pub fn clear(&mut self) {
        self.latched = None;
    }

    pub fn clear_geometry(&mut self) {
        if self
            .latched
            .as_ref()
            .is_some_and(|latched| latched.source == SnapSource::Geometry)
        {
            self.clear();
        }
    }

    pub fn clear_grid(&mut self) {
        if self
            .latched
            .as_ref()
            .is_some_and(|latched| latched.source == SnapSource::Grid)
        {
            self.clear();
        }
    }
}

/// Pull a point toward the nearest eligible target, capturing exactly near the target and
/// retaining an exact capture until the pointer leaves its wider release radius.
pub fn magnetic_snap(
    raw: Vec2,
    geometry: Option<MagneticSnapTarget>,
    grid: Option<Vec2>,
    pixels_per_world: f64,
    state: &mut MagneticSnapState,
) -> (Vec2, Option<SnapGuide>) {
    if !pixels_per_world.is_finite()
        || pixels_per_world <= 0.0
        || !raw.x.is_finite()
        || !raw.y.is_finite()
    {
        state.clear();
        return (raw, None);
    }

    let geometry = geometry
        .filter(|target| screen_distance(raw, target.at, pixels_per_world) <= SNAP_PULL_RADIUS_PX);
    if let Some(latched) = state.latched.take() {
        let still_latched =
            screen_distance(raw, latched.target.at, pixels_per_world) <= SNAP_RELEASE_RADIUS_PX;
        let geometry_preempts_grid = latched.source == SnapSource::Grid && geometry.is_some();
        if still_latched && !geometry_preempts_grid {
            let result = (latched.target.at, latched.target.guide.clone());
            state.latched = Some(latched);
            return result;
        }
    }

    if let Some(target) = geometry {
        return attract(raw, target, SnapSource::Geometry, pixels_per_world, state);
    }
    if let Some(at) =
        grid.filter(|at| screen_distance(raw, *at, pixels_per_world) <= SNAP_PULL_RADIUS_PX)
    {
        return attract(
            raw,
            MagneticSnapTarget { at, guide: None },
            SnapSource::Grid,
            pixels_per_world,
            state,
        );
    }
    (raw, None)
}

fn attract(
    raw: Vec2,
    target: MagneticSnapTarget,
    source: SnapSource,
    pixels_per_world: f64,
    state: &mut MagneticSnapState,
) -> (Vec2, Option<SnapGuide>) {
    let distance = screen_distance(raw, target.at, pixels_per_world);
    if distance <= SNAP_CAPTURE_RADIUS_PX {
        state.latched = Some(LatchedSnap {
            source,
            target: target.clone(),
        });
        return (target.at, target.guide);
    }

    let linear = ((SNAP_PULL_RADIUS_PX - distance)
        / (SNAP_PULL_RADIUS_PX - SNAP_CAPTURE_RADIUS_PX))
        .clamp(0.0, 1.0);
    let weight = linear * linear * (3.0 - 2.0 * linear);
    (
        Vec2 {
            x: raw.x + (target.at.x - raw.x) * weight,
            y: raw.y + (target.at.y - raw.y) * weight,
        },
        target.guide,
    )
}

fn screen_distance(a: Vec2, b: Vec2, pixels_per_world: f64) -> f64 {
    (a.x - b.x).hypot(a.y - b.y) * pixels_per_world
}
#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

fn selection_outline(parts: &[Part], definitions: &[PartDefinition]) -> Vec<Vec2> {
    let mut points = parts
        .iter()
        .flat_map(|part| {
            let definition = definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let cap = definition.and_then(|definition| {
                if definition.kind == PartKind::Switch {
                    part.keycap.or(definition.keycap)
                } else {
                    None
                }
            });
            let corners = if let Some(cap) = cap {
                vec![
                    Vec2 {
                        x: -cap.x / 2.0,
                        y: -cap.y / 2.0,
                    },
                    Vec2 {
                        x: cap.x / 2.0,
                        y: -cap.y / 2.0,
                    },
                    Vec2 {
                        x: cap.x / 2.0,
                        y: cap.y / 2.0,
                    },
                    Vec2 {
                        x: -cap.x / 2.0,
                        y: cap.y / 2.0,
                    },
                ]
            } else {
                definition.map_or_else(Vec::new, |definition| definition.courtyard.clone())
            };
            let angle = part.pose.rotation.to_radians();
            corners.into_iter().map(move |point| Vec2 {
                x: part.pose.at.x + point.x * angle.cos() - point.y * angle.sin(),
                y: part.pose.at.y + point.x * angle.sin() + point.y * angle.cos(),
            })
        })
        .collect::<Vec<_>>();
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let cross = |a: Vec2, b: Vec2, c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let half = |ordered: Vec<Vec2>| {
        let mut result: Vec<Vec2> = Vec::new();
        for point in ordered {
            while result.len() >= 2
                && cross(result[result.len() - 2], result[result.len() - 1], point) <= 0.0
            {
                result.pop();
            }
            result.push(point);
        }
        result.truncate(result.len().saturating_sub(1));
        result
    };
    let upper = half(points.iter().copied().rev().collect());
    let mut result = half(points);
    result.extend(upper);
    result
}
fn bounds(points: &[Vec2]) -> Option<Bounds> {
    Some(Bounds {
        min_x: points.iter().map(|p| p.x).min_by(f64::total_cmp)?,
        max_x: points.iter().map(|p| p.x).max_by(f64::total_cmp)?,
        min_y: points.iter().map(|p| p.y).min_by(f64::total_cmp)?,
        max_y: points.iter().map(|p| p.y).max_by(f64::total_cmp)?,
    })
}
fn axis_aligned(points: &[Vec2]) -> bool {
    points.len() == 4
        && points.iter().enumerate().all(|(i, p)| {
            let q = points[(i + 1) % points.len()];
            (p.x - q.x).abs() < 0.0001 || (p.y - q.y).abs() < 0.0001
        })
}
fn landmarks(points: &[Vec2], center: Vec2) -> Vec<Vec2> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut result = vec![center];
    result.extend_from_slice(points);
    result.extend(points.iter().enumerate().map(|(i, p)| {
        let q = points[(i + 1) % points.len()];
        Vec2 {
            x: (p.x + q.x) / 2.0,
            y: (p.y + q.y) / 2.0,
        }
    }));
    result
}

/// Match the editor's physical envelope landmarks and optional rectangular gap snap.
pub fn snap_part(
    document: &ProjectDoc,
    moving: &Part,
    tolerance: f64,
    gap: Option<f64>,
) -> Option<SnapGuide> {
    snap_part_candidates(document, moving, tolerance, gap, document.parts.iter())
}

/// Snap against only parts owned by the selected board.
pub fn snap_part_in_board(
    document: &ProjectDoc,
    board_id: &str,
    moving: &Part,
    tolerance: f64,
    gap: Option<f64>,
) -> Option<SnapGuide> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return document
            .boards
            .is_empty()
            .then(|| snap_part_candidates(document, moving, tolerance, gap, document.parts.iter()))
            .flatten();
    };
    if document.boards.len() == 1 && board.part_ids.is_empty() {
        // Legacy single-board documents may omit the part index entirely.
        return snap_part_candidates(document, moving, tolerance, gap, document.parts.iter());
    }
    let part_ids: std::collections::HashSet<_> =
        board.part_ids.iter().map(String::as_str).collect();
    let candidates = document
        .parts
        .iter()
        .filter(|part| part_ids.contains(part.id.as_str()));
    snap_part_candidates(document, moving, tolerance, gap, candidates)
}

fn snap_part_candidates<'a>(
    document: &ProjectDoc,
    moving: &Part,
    tolerance: f64,
    gap: Option<f64>,
    candidates: impl Iterator<Item = &'a Part> + Clone,
) -> Option<SnapGuide> {
    let moving_poly = selection_outline(std::slice::from_ref(moving), &document.definitions);
    if moving_poly.is_empty() || tolerance < 0.0 {
        return None;
    }
    let moving_landmarks = landmarks(&moving_poly, moving.pose.at);
    let moving_bounds = if gap.is_some() && axis_aligned(&moving_poly) {
        bounds(&moving_poly)
    } else {
        None
    };
    let mut best: Option<SnapGuide> = None;
    let mut distance_squared = tolerance * tolerance;
    let mut consider = |from: Vec2, to: Vec2, reference: &str, gap: Option<f64>| {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let d = dx * dx + dy * dy;
        if d <= distance_squared {
            distance_squared = d;
            best = Some(SnapGuide {
                at: Vec2 {
                    x: moving.pose.at.x + dx,
                    y: moving.pose.at.y + dy,
                },
                from,
                to,
                label: match gap {
                    Some(gap) => format!("{reference} · envelope gap {gap:.2} mm"),
                    None => format!("{reference} · corner / midpoint / center"),
                },
            });
        }
    };
    for target in candidates.clone() {
        if target.id == moving.id {
            continue;
        }
        let polygon = selection_outline(std::slice::from_ref(target), &document.definitions);
        if polygon.is_empty() {
            continue;
        }
        let target_landmarks = landmarks(&polygon, target.pose.at);
        for from in &moving_landmarks {
            for to in &target_landmarks {
                consider(*from, *to, &target.reference, None);
            }
        }
        let Some(gap) = gap else {
            continue;
        };
        let (Some(a), Some(b)) = (moving_bounds, bounds(&polygon)) else {
            continue;
        };
        if !axis_aligned(&polygon) {
            continue;
        }
        if a.max_y.min(b.max_y) > a.min_y.max(b.min_y) {
            let y = (a.min_y.max(b.min_y) + a.max_y.min(b.max_y)) / 2.0;
            consider(
                Vec2 { x: a.min_x, y },
                Vec2 {
                    x: b.max_x + gap,
                    y,
                },
                &target.reference,
                Some(gap),
            );
            consider(
                Vec2 { x: a.max_x, y },
                Vec2 {
                    x: b.min_x - gap,
                    y,
                },
                &target.reference,
                Some(gap),
            );
        }
        if a.max_x.min(b.max_x) > a.min_x.max(b.min_x) {
            let x = (a.min_x.max(b.min_x) + a.max_x.min(b.max_x)) / 2.0;
            consider(
                Vec2 { x, y: a.min_y },
                Vec2 {
                    x,
                    y: b.max_y + gap,
                },
                &target.reference,
                Some(gap),
            );
            consider(
                Vec2 { x, y: a.max_y },
                Vec2 {
                    x,
                    y: b.min_y - gap,
                },
                &target.reference,
                Some(gap),
            );
        }
    }
    if let (Some(best_snap), Some(moving_bounds), Some(gap)) = (best.as_mut(), moving_bounds, gap) {
        if best_snap.label.contains("envelope gap") {
            let moved = Part {
                pose: boardstudio_core::model::Pose2 {
                    at: best_snap.at,
                    ..moving.pose
                },
                ..moving.clone()
            };
            let Some(snapped) = bounds(&selection_outline(
                std::slice::from_ref(&moved),
                &document.definitions,
            )) else {
                return best;
            };
            let mut alignment: Option<(bool, f64)> = None;
            for target in candidates {
                if target.id == moving.id {
                    continue;
                }
                let polygon =
                    selection_outline(std::slice::from_ref(target), &document.definitions);
                if !axis_aligned(&polygon) {
                    continue;
                }
                let Some(b) = bounds(&polygon) else {
                    continue;
                };
                let horizontal = (snapped.min_x - b.max_x - gap).abs() < 0.0001
                    || (b.min_x - snapped.max_x - gap).abs() < 0.0001;
                let vertical = (snapped.min_y - b.max_y - gap).abs() < 0.0001
                    || (b.min_y - snapped.max_y - gap).abs() < 0.0001;
                let candidates: Vec<f64> = if horizontal {
                    vec![b.min_y - snapped.min_y, b.max_y - snapped.max_y]
                } else if vertical {
                    vec![b.min_x - snapped.min_x, b.max_x - snapped.max_x]
                } else {
                    vec![]
                };
                for delta in candidates {
                    if delta.abs() <= tolerance
                        && alignment.is_none_or(|(_, current)| delta.abs() < current.abs())
                    {
                        alignment = Some((horizontal, delta));
                    }
                }
            }
            if let Some((horizontal, delta)) = alignment {
                if horizontal {
                    best_snap.at.y += delta;
                    best_snap.to.y += delta;
                } else {
                    best_snap.at.x += delta;
                    best_snap.to.x += delta;
                }
                best_snap.label.push_str(" · edges aligned");
            }
        }
        let _ = moving_bounds;
    }
    best
}

/// Options captured at gesture start and updated only for the live Alt modifier.
pub struct DragSnapOptions<'a> {
    pub board_id: &'a str,
    pub pitch: Vec2,
    pub fraction: f64,
    pub geometry_snap: bool,
    pub gap: Option<f64>,
    pub targets: &'a [String],
    pub alt: bool,
    pub pixels_per_world: Option<f64>,
    pub state: &'a mut MagneticSnapState,
}

/// Apply Layout's drag-relative grid and magnetic geometry snapping to a pointer sample.
pub fn normalize_drag(
    document: &ProjectDoc,
    start: &[boardstudio_core::model::Position],
    raw: Vec<boardstudio_core::model::Position>,
    options: DragSnapOptions<'_>,
) -> (Vec<boardstudio_core::model::Position>, Option<SnapGuide>) {
    if options.alt {
        options.state.clear();
        return (raw, None);
    }

    let Some(pixels_per_world) = options.pixels_per_world else {
        options.state.clear();
        return normalize_drag_stepped(document, start, raw, options);
    };
    if !pixels_per_world.is_finite() || pixels_per_world <= 0.0 {
        options.state.clear();
        return (raw, None);
    }

    let DragSnapOptions {
        board_id,
        pitch,
        fraction,
        geometry_snap,
        gap,
        targets,
        state,
        ..
    } = options;

    let selected_id = targets.first();
    let selected_position = selected_id
        .and_then(|id| raw.iter().find(|position| &position.id == id))
        .or_else(|| raw.first());
    let Some(selected_position) = selected_position else {
        state.clear();
        return (raw, None);
    };
    let Some(origin) = start
        .iter()
        .find(|origin| origin.id == selected_position.id)
    else {
        state.clear();
        return (raw, None);
    };
    let raw_delta = Vec2 {
        x: selected_position.at.x - origin.at.x,
        y: selected_position.at.y - origin.at.y,
    };
    let grid = (fraction != 0.0).then(|| {
        let sx = (if fraction < 0.0 {
            -fraction
        } else {
            pitch.x * fraction
        })
        .max(0.001);
        let sy = (if fraction < 0.0 {
            -fraction
        } else {
            pitch.y * fraction
        })
        .max(0.001);
        Vec2 {
            x: (raw_delta.x / sx).round() * sx,
            y: (raw_delta.y / sy).round() * sy,
        }
    });
    let geometry = if geometry_snap && targets.len() == 1 {
        document
            .parts
            .iter()
            .find(|part| part.id == selected_position.id)
            .and_then(|original| {
                let moving = Part {
                    pose: boardstudio_core::model::Pose2 {
                        at: selected_position.at,
                        ..original.pose
                    },
                    ..original.clone()
                };
                snap_part_in_board(
                    document,
                    board_id,
                    &moving,
                    SNAP_PULL_RADIUS_PX / pixels_per_world,
                    gap,
                )
            })
            .map(|guide| MagneticSnapTarget {
                at: Vec2 {
                    x: guide.at.x - origin.at.x,
                    y: guide.at.y - origin.at.y,
                },
                guide: Some(guide),
            })
    } else {
        None
    };
    let (delta, guide) = magnetic_snap(raw_delta, geometry, grid, pixels_per_world, state);
    let positions = raw
        .into_iter()
        .map(|mut position| {
            if let Some(origin) = start.iter().find(|origin| origin.id == position.id) {
                position.at.x = origin.at.x + delta.x;
                position.at.y = origin.at.y + delta.y;
            }
            position
        })
        .collect::<Vec<_>>();
    (positions, guide)
}

fn normalize_drag_stepped(
    document: &ProjectDoc,
    start: &[boardstudio_core::model::Position],
    raw: Vec<boardstudio_core::model::Position>,
    options: DragSnapOptions<'_>,
) -> (Vec<boardstudio_core::model::Position>, Option<SnapGuide>) {
    let DragSnapOptions {
        board_id,
        pitch,
        fraction,
        geometry_snap,
        gap,
        targets,
        ..
    } = options;
    let mut positions = raw
        .into_iter()
        .map(|mut position| {
            if let Some(origin) = start.iter().find(|origin| origin.id == position.id)
                && fraction != 0.0
            {
                let sx = (if fraction < 0.0 {
                    -fraction
                } else {
                    pitch.x * fraction
                })
                .max(0.001);
                let sy = (if fraction < 0.0 {
                    -fraction
                } else {
                    pitch.y * fraction
                })
                .max(0.001);
                position.at.x = origin.at.x + ((position.at.x - origin.at.x) / sx).round() * sx;
                position.at.y = origin.at.y + ((position.at.y - origin.at.y) / sy).round() * sy;
            }
            position
        })
        .collect::<Vec<_>>();
    let mut guide = None;
    if geometry_snap
        && targets.len() == 1
        && let Some(position) = positions
            .iter_mut()
            .find(|position| position.id == targets[0])
        && let Some(original) = document.parts.iter().find(|part| part.id == position.id)
    {
        let moving = Part {
            pose: boardstudio_core::model::Pose2 {
                at: position.at,
                ..original.pose
            },
            ..original.clone()
        };
        guide = snap_part_in_board(document, board_id, &moving, 2.0, gap);
        if let Some(snap) = &guide {
            position.at = snap.at;
        }
    }
    (positions, guide)
}
