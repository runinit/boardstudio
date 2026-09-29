//! Local process screening; these checks are not supplier approval.
use crate::model::{
    Contour, Finding, MechanicalAssembly, MechanicalConfiguration, Mount, MountKind, PlateMethod,
    Scope, Severity, Vec2,
};

use i_overlay::i_float::int::{angle::Angle, point::IntPoint};
use i_overlay::mesh::{
    int::{
        outline::offset::IntOutlineOffset,
        style::{IntLineJoin, IntOutlineStyle},
    },
    math::MathMode,
};

const JLC_NPTH_MIN: f64 = 0.5;
const JLC_SLOT_MIN: f64 = 1.0;
const RECOMMENDED_WEB: f64 = 1.0;
const EPSILON: f64 = 1e-7;

pub(crate) fn propose_mounts(config: &MechanicalConfiguration, contours: &[Contour]) -> Vec<Mount> {
    let mut proposals = vec![];
    let radius = 3.0;
    let margin = radius + config.clearance.max(0.2);
    for contour in contours.iter().filter(|contour| !contour.hole) {
        let (min, max) = bounds(&contour.points);
        let columns = (((max.x - min.x) / 2.0).ceil() as usize).clamp(1, 80);
        let rows = (((max.y - min.y) / 2.0).ceil() as usize).clamp(1, 80);
        for at in (0..=rows).flat_map(|row| (0..=columns).map(move |column| Vec2 {
            x: min.x + margin + (max.x - min.x - 2.0 * margin) * column as f64 / columns as f64,
            y: min.y + margin + (max.y - min.y - 2.0 * margin) * row as f64 / rows as f64,
        })) {
            let contained = contains(&contour.points, at)
                && !contours
                    .iter()
                    .filter(|c| c.hole)
                    .any(|c| contains(&c.points, at));
            let edge_clear = contours.iter().all(|c| {
                (0..c.points.len()).all(|i| {
                    point_segment(at, c.points[i], c.points[(i + 1) % c.points.len()])
                        >= radius + config.clearance.max(0.2)
                })
            });
            let occupied = config
                .mounts
                .iter()
                .chain(config.closure_mounts.iter().flatten())
                .any(|mount| {
                    (mount.at.x - at.x).hypot(mount.at.y - at.y)
                        < radius
                            + mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0
                            + config.clearance
                });
            let battery = config.battery.as_ref().is_some_and(|battery| {
                (at.x - battery.at.x).abs() < battery.size.x / 2.0 + margin
                    && (at.y - battery.at.y).abs() < battery.size.y / 2.0 + margin
            });
            if contained
                && edge_clear
                && !occupied
                && !battery
                && !proposals
                    .iter()
                    .any(|m: &Mount| (m.at.x - at.x).hypot(m.at.y - at.y) < 2.0 * radius)
            {
                proposals.push(Mount {
                    id: format!("mount-proposal-{}", proposals.len() + 1),
                    at,
                    kind: MountKind::Hole,
                    hole_diameter: 3.0,
                    boss_diameter: Some(6.0),
                    height: None,
                });
            }
        }
    }
    // Pick well-separated supports rather than clustering at the first clear corner.
    let mut selected: Vec<Mount> = vec![];
    while !proposals.is_empty() && selected.len() < 4 {
        let index = if selected.is_empty() { 0 } else {
            proposals.iter().enumerate().max_by(|(_, a), (_, b)| {
                let distance = |mount: &Mount| selected.iter().map(|other| (mount.at.x - other.at.x).hypot(mount.at.y - other.at.y)).fold(f64::INFINITY, f64::min);
                distance(a).total_cmp(&distance(b))
            }).unwrap().0
        };
        selected.push(proposals.remove(index));
    }
    selected
}

fn contains(points: &[Vec2], p: Vec2) -> bool {
    let mut result = false;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            result = !result;
        }
    }
    result
}

pub(crate) fn apply_allowance(config: &MechanicalConfiguration, assembly: &mut MechanicalAssembly) {
    assembly.nominal_plate_contours = assembly.plate_contours.clone();
    let allowance = config.opening_allowance.unwrap_or(0.0);
    if allowance == 0.0 {
        return;
    }
    let failure = |message: String| Finding {
        id: "mechanical:opening-allowance".into(),
        severity: Severity::Error,
        scope: Scope::Case,
        message,
        target_ids: vec!["plate".into()],
    };
    if !allowance.is_finite() || allowance.abs() > 1.0 {
        assembly.diagnostics.push(failure("Opening allowance must be finite and within ±1 mm. Confirm fit against the profile's functional engagement dimensions.".into()));
        assembly.generation_blocked = true;
        return;
    }
    let mut adjusted = vec![];
    for contour in &assembly.nominal_plate_contours {
        if !contour.hole {
            adjusted.push(contour.clone());
            continue;
        }
        match offset_opening(&contour.points, allowance) {
            Some(points) => adjusted.push(Contour { hole: true, points }),
            None => {
                assembly.diagnostics.push(failure("Opening allowance collapses or splits a functional opening. Reduce the allowance or choose a compatible profile.".into()));
                assembly.generation_blocked = true;
                return;
            }
        }
    }
    assembly.plate_contours = adjusted.clone();
    if let Some(plate) = assembly
        .case
        .bodies
        .iter_mut()
        .find(|entry| entry.body.id == "plate")
    {
        plate.contours = adjusted;
    }
    assembly.diagnostics.push(Finding { id: "mechanical:fit-review".into(), severity: Severity::Warning, scope: Scope::Case, message: format!("An explicit {allowance:+.3} mm radial opening allowance changes nominal functional fit. Verify engagement with a sample; foam retains its nominal exclusions."), target_ids: vec!["plate".into()] });
}

fn offset_opening(points: &[Vec2], distance: f64) -> Option<Vec<Vec2>> {
    const SCALE: f64 = 1000.0;
    if points.len() < 3
        || points.iter().any(|p| {
            !p.x.is_finite()
                || !p.y.is_finite()
                || p.x.abs() > 1_000_000.0
                || p.y.abs() > 1_000_000.0
        })
    {
        return None;
    }
    let mut path: Vec<IntPoint<i64>> = points
        .iter()
        .map(|p| IntPoint::new((p.x * SCALE).round() as i64, (p.y * SCALE).round() as i64))
        .collect();
    let area: i128 = (0..path.len())
        .map(|i| {
            let a = path[i];
            let b = path[(i + 1) % path.len()];
            a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128
        })
        .sum();
    if area == 0 {
        return None;
    }
    if area < 0 {
        path.reverse();
    }
    // Same 0.001 mm grid and bounded miter policy as case preparation, but signed:
    // positive enlarges a hole and negative reduces it without changing its nominal source.
    let style = IntOutlineStyle::new((distance * SCALE).round() as i64)
        .math(MathMode::Float)
        .line_join(IntLineJoin::Miter(Angle::from_radians(
            0.505_360_510_284_157_3,
        )?))
        .miter_min_turn(Angle::from_radians(5.0_f64.to_radians())?);
    let shapes = path.outline(&style).ok()?;
    if shapes.len() != 1 || shapes[0].len() != 1 || shapes[0][0].len() < 3 {
        return None;
    }
    Some(
        shapes[0][0]
            .iter()
            .map(|p| Vec2 {
                x: p.x as f64 / SCALE,
                y: p.y as f64 / SCALE,
            })
            .collect(),
    )
}

pub(crate) fn check(config: &MechanicalConfiguration, contours: &[Contour]) -> Vec<Finding> {
    let mut findings = vec![];
    let mut add = |id: String, severity, message: String| {
        findings.push(Finding {
            id: format!("mechanical:process:{id}"),
            severity,
            scope: Scope::Case,
            message,
            target_ids: vec!["plate".into()],
        })
    };
    for process in config.part_processes.iter().flatten() {
        if process.constraints_version != "2026-09-24" {
            add(
                format!("version:{}", process.part_id),
                Severity::Error,
                format!(
                    "Part {} uses unsupported constraints version {}; select 2026-09-24 or review the profile before export.",
                    process.part_id, process.constraints_version
                ),
            );
        }
        let disabled_foam = matches!(process.part_id.as_str(), "plate-foam" | "bottom-foam")
            && process.thickness == 0.0
            && expected_layer_thickness(config, &process.part_id) == Some(0.0);
        if process.material.trim().is_empty()
            || !process.thickness.is_finite()
            || (process.thickness <= 0.0 && !disabled_foam)
        {
            add(
                format!("stock:{}", process.part_id),
                Severity::Error,
                format!(
                    "Part {} requires a material and positive stock thickness.",
                    process.part_id
                ),
            );
        }
        let expected_thickness = expected_layer_thickness(config, &process.part_id);
        if expected_thickness.is_some_and(|expected| (process.thickness - expected).abs() > EPSILON)
        {
            add(
                format!("stack-process:{}", process.part_id),
                Severity::Error,
                format!(
                    "Part {} stock thickness disagrees with the resolved stack. Change its stack thickness and regenerate before export.",
                    process.part_id
                ),
            );
        }
        if process.part_id == "plate"
            && ((process.thickness - config.plate_thickness).abs() > EPSILON
                || process.method != config.method)
        {
            add("plate-process".into(), Severity::Error, "The plate process record must match the assembly plate method and thickness; change the shared plate settings to regenerate its geometry.".into());
        }
    }
    if config.method == PlateMethod::PcbFr4 {
        add("constraints".into(), Severity::Info, "JLC PCB capability snapshot 2026-09-24: documented minimum NPTH 0.5 mm, non-plated slot 1.0 mm; slot tolerance ±0.2 mm. Supplier review is required for routing shape and stock selection.".into());
        for mount in &config.mounts {
            if mount.hole_diameter < JLC_NPTH_MIN {
                add(
                    format!("drill:{}", mount.id),
                    Severity::Error,
                    format!(
                        "Mount {} has a {:.3} mm unplated hole; JLC's documented minimum is 0.5 mm.",
                        mount.id, mount.hole_diameter
                    ),
                );
            }
        }
        for (index, contour) in contours.iter().enumerate().filter(|(_, c)| c.hole) {
            let (min, max) = bounds(&contour.points);
            if (max.x - min.x).min(max.y - min.y) + EPSILON < JLC_SLOT_MIN {
                add(
                    format!("slot:{index}"),
                    Severity::Error,
                    format!(
                        "Opening {index} is narrower than JLC's documented 1.0 mm non-plated slot minimum."
                    ),
                );
            }
            if sharp_corner(&contour.points) {
                add(
                    format!("corner:{index}"),
                    Severity::Error,
                    format!(
                        "Opening {index} contains a sharp routed corner. Select a reviewed rounded profile; JLC does not support sharp rectangular routed openings."
                    ),
                );
            }
        }
    }
    if config.method == PlateMethod::Cnc {
        for (index, contour) in contours.iter().enumerate().filter(|(_, c)| c.hole) {
            if sharp_corner(&contour.points) {
                add(
                    format!("cnc-corner:{index}"),
                    Severity::Warning,
                    format!(
                        "Opening {index} has sharp internal corners. Review tool radius or a profile-approved relief without changing switch engagement."
                    ),
                );
            }
        }
    }
    if matches!(config.method, PlateMethod::Printed | PlateMethod::Cnc)
        && config.wall_thickness < 1.0
    {
        add("wall".into(), Severity::Warning, "Walls below 1.0 mm require material and supplier review; 1.0 mm is a screening recommendation, not a documented process limit.".into());
    }
    // Internal closures are validated against their case lands by the generator;
    // their independent positions intentionally lie outside the floating plate.
    let internal_closures = config.internal_gasket.is_some()
        && config.mount == crate::model::MechanicalMount::Gasket;
    let mounts: Vec<_> = config
        .mounts
        .iter()
        .chain(config.closure_mounts.iter().flatten().filter(|_| !internal_closures))
        .collect();
    for (index, mount) in mounts.iter().enumerate() {
        let radius = mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0;
        let material = contours
            .iter()
            .filter(|c| !c.hole)
            .any(|c| contains(&c.points, mount.at))
            && !contours
                .iter()
                .filter(|c| c.hole)
                .any(|c| contains(&c.points, mount.at));
        let edge_clear = contours.iter().all(|c| {
            (0..c.points.len()).all(|i| {
                point_segment(mount.at, c.points[i], c.points[(i + 1) % c.points.len()]) + EPSILON
                    >= radius + config.clearance
            })
        });
        if !material || !edge_clear {
            add(
                format!("mount-material:{}", mount.id),
                Severity::Error,
                format!(
                    "Mount {} does not leave material for its full hole/boss and clearance envelope. Move it away from the edge or opening.",
                    mount.id
                ),
            );
        }
        for other in mounts.iter().skip(index + 1) {
            let other_radius = other.boss_diameter.unwrap_or(other.hole_diameter) / 2.0;
            if (mount.at.x - other.at.x).hypot(mount.at.y - other.at.y)
                < radius + other_radius + config.clearance
            {
                add(
                    format!("mount-overlap:{}:{}", mount.id, other.id),
                    Severity::Error,
                    format!(
                        "Mounts {} and {} overlap their clearance envelopes. Keep closure screws separate from suspension hardware.",
                        mount.id, other.id
                    ),
                );
            }
        }
    }
    for (index, hole) in contours.iter().enumerate().filter(|(_, c)| c.hole) {
        if !contours
            .iter()
            .filter(|c| !c.hole)
            .any(|outer| hole.points.iter().all(|p| contains(&outer.points, *p)))
        {
            add(
                format!("outside:{index}"),
                Severity::Error,
                format!(
                    "Opening {index} lies outside the plate material. Move the component or enlarge the nominal plate outline."
                ),
            );
        }
    }
    for (index, a) in contours.iter().enumerate() {
        for (other, b) in contours.iter().enumerate().skip(index + 1) {
            if !a.hole && !b.hole {
                continue;
            }
            let distance = contour_distance(&a.points, &b.points);
            if distance <= EPSILON {
                add(
                    format!("intersection:{index}:{other}"),
                    Severity::Error,
                    format!(
                        "Contours {index} and {other} intersect; resolve overlapping openings or openings crossing the plate boundary."
                    ),
                );
            } else if distance < RECOMMENDED_WEB {
                add(
                    format!("web:{index}:{other}"),
                    Severity::Warning,
                    format!(
                        "Contours {index} and {other} leave {distance:.3} mm of material. The 1.0 mm web screening recommendation requires material and supplier review."
                    ),
                );
            }
        }
    }
    findings
}

fn expected_layer_thickness(config: &MechanicalConfiguration, part_id: &str) -> Option<f64> {
    match part_id {
        "plate" => Some(config.plate_thickness),
        "bottom" => Some(config.bottom_thickness),
        "plate-foam" => Some(config.plate_foam_thickness),
        "bottom-foam" => Some(config.bottom_foam_thickness),
        _ => None,
    }
}

fn bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    points.iter().fold(
        (
            Vec2 {
                x: f64::INFINITY,
                y: f64::INFINITY,
            },
            Vec2 {
                x: f64::NEG_INFINITY,
                y: f64::NEG_INFINITY,
            },
        ),
        |(min, max), p| {
            (
                Vec2 {
                    x: min.x.min(p.x),
                    y: min.y.min(p.y),
                },
                Vec2 {
                    x: max.x.max(p.x),
                    y: max.y.max(p.y),
                },
            )
        },
    )
}
fn sharp_corner(points: &[Vec2]) -> bool {
    if points.len() < 3 {
        return false;
    }
    (0..points.len()).any(|i| {
        let a = points[(i + points.len() - 1) % points.len()];
        let b = points[i];
        let c = points[(i + 1) % points.len()];
        let u = Vec2 {
            x: a.x - b.x,
            y: a.y - b.y,
        };
        let v = Vec2 {
            x: c.x - b.x,
            y: c.y - b.y,
        };
        let lengths = (u.x * u.x + u.y * u.y).sqrt() * (v.x * v.x + v.y * v.y).sqrt();
        lengths > EPSILON && (u.x * v.x + u.y * v.y) / lengths > -0.5
    })
}
fn cross(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn point_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let norm = dx * dx + dy * dy;
    let t = if norm < EPSILON {
        0.0
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / norm
    }
    .clamp(0.0, 1.0);
    ((p.x - a.x - t * dx).powi(2) + (p.y - a.y - t * dy).powi(2)).sqrt()
}
fn segment_distance(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> f64 {
    if cross(a, b, c) * cross(a, b, d) < 0.0 && cross(c, d, a) * cross(c, d, b) < 0.0 {
        return 0.0;
    }
    point_segment(a, c, d)
        .min(point_segment(b, c, d))
        .min(point_segment(c, a, b))
        .min(point_segment(d, a, b))
}
fn contour_distance(a: &[Vec2], b: &[Vec2]) -> f64 {
    let mut min = f64::INFINITY;
    for i in 0..a.len() {
        for j in 0..b.len() {
            min = min.min(segment_distance(
                a[i],
                a[(i + 1) % a.len()],
                b[j],
                b[(j + 1) % b.len()],
            ));
        }
    }
    min
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square(x: f64, y: f64, size: f64) -> Vec<Vec2> {
        vec![
            Vec2 { x, y },
            Vec2 { x: x + size, y },
            Vec2 {
                x: x + size,
                y: y + size,
            },
            Vec2 { x, y: y + size },
        ]
    }
    #[test]
    fn detects_crossings_and_remaining_web() {
        assert_eq!(
            contour_distance(&square(0.0, 0.0, 2.0), &square(1.0, 1.0, 2.0)),
            0.0
        );
        assert!(
            (contour_distance(&square(0.0, 0.0, 2.0), &square(2.4, 0.0, 2.0)) - 0.4).abs()
                < EPSILON
        );
    }
    #[test]
    fn sharp_rectangles_are_distinct_from_tessellated_round_openings() {
        assert!(sharp_corner(&square(0.0, 0.0, 2.0)));
        let round: Vec<_> = (0..64)
            .map(|i| {
                let t = i as f64 * std::f64::consts::TAU / 64.0;
                Vec2 {
                    x: t.cos(),
                    y: t.sin(),
                }
            })
            .collect();
        assert!(!sharp_corner(&round));
    }
}

pub(crate) fn check_specifications(
    config: &MechanicalConfiguration,
    assembly: &crate::model::CaseAssemblyIR,
) -> Vec<Finding> {
    let mut findings = vec![];
    let mut ids = std::collections::HashSet::new();
    for specification in config.hardware.iter().flatten() {
        let target_exists = assembly.bodies.iter().any(|part| {
            part.body.id == specification.part_id
                && part
                    .body
                    .mounts
                    .iter()
                    .flatten()
                    .any(|mount| mount.id == specification.feature_id)
        });
        if specification.id.trim().is_empty()
            || !ids.insert(specification.id.clone())
            || specification.designation.trim().is_empty()
            || specification.thread.trim().is_empty()
            || !specification.length.is_finite()
            || specification.length <= 0.0
            || specification.quantity == 0
            || !target_exists
        {
            findings.push(Finding {
                id: format!("mechanical:hardware:{}", specification.id),
                severity: Severity::Error, scope: Scope::Case,
                message: "Hardware requires a unique id, designation, thread, positive length and quantity, and an existing mounting feature on the selected generated part.".into(),
                target_ids: vec![specification.part_id.clone(), specification.feature_id.clone()],
            });
        }
    }
    ids.clear();
    for specification in config.critical_fits.iter().flatten() {
        let coordinates = [
            specification.from.x,
            specification.from.y,
            specification.to.x,
            specification.to.y,
        ];
        if specification.id.trim().is_empty()
            || !ids.insert(specification.id.clone())
            || specification.label.trim().is_empty()
            || specification.tolerance.trim().is_empty()
            || coordinates.iter().any(|value| !value.is_finite())
            || (specification.to.x - specification.from.x)
                .hypot(specification.to.y - specification.from.y)
                <= EPSILON
            || !assembly
                .bodies
                .iter()
                .any(|part| part.body.id == specification.part_id)
        {
            findings.push(Finding {
                id: format!("mechanical:critical-fit:{}", specification.id),
                severity: Severity::Error, scope: Scope::Case,
                message: "Critical fits require a unique id, label, tolerance, distinct finite dimension endpoints, and an existing generated part.".into(),
                target_ids: vec![specification.part_id.clone()],
            });
        }
    }
    findings
}
