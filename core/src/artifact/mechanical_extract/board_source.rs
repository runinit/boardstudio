//! Exact authored Edge.Cuts curves and drilled feature identities for PCB cases.
use super::*;
use crate::matrix::splay::rotate;
use crate::model::{
    CasePcbCircularSupport, CasePcbSlot, CasePcbSource, CasePcbSourceContour,
    CasePcbSourcePrimitive, Mount, MountKind,
};
use std::collections::BTreeSet;

const TAU: f64 = std::f64::consts::TAU;
mod ownership;

#[derive(Clone)]
struct SourceEdge {
    start: Vec2,
    end: Vec2,
    primitive: CasePcbSourcePrimitive,
}

pub(in crate::artifact) fn extract_board_source(
    raw: &str,
    center: Vec2,
) -> Result<(CasePcbSource, Vec<Mount>), ArtifactError> {
    let document = kiutils_sexpr::parse_one(raw).map_err(|error| parse_error(error.to_string()))?;
    let root = document
        .nodes
        .first()
        .filter(|node| sexpr::head(node) == Some("kicad_pcb"))
        .ok_or_else(|| parse_error("Expected a KiCad PCB source"))?;

    let mut source_ids = BTreeSet::new();
    let mut edges = Vec::new();
    let mut circles = Vec::new();
    validate_outline_inventory(root)?;
    for node in sexpr::items(root).into_iter().flatten().filter(|node| {
        layer(node).as_deref() == Some("Edge.Cuts")
            && matches!(
                sexpr::head(node),
                Some("gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly")
            )
    }) {
        let source_id = stable_source_id(node, "Edge.Cuts primitive")?;
        if !source_ids.insert(source_id.clone()) {
            return Err(validation("PCB source has duplicate Edge.Cuts source IDs"));
        }
        let kind = match sexpr::head(node) {
            Some("gr_line") => GeometryKind::Line,
            Some("gr_arc") => GeometryKind::Arc,
            Some("gr_circle") => GeometryKind::Circle,
            Some("gr_rect") => GeometryKind::Rectangle,
            Some("gr_poly") => GeometryKind::Polygon,
            _ => unreachable!("filtered to supported Edge.Cuts geometry"),
        };
        let shape = parse_graphic(node, kind)?;
        let mut normalized = normalize_outline_shape(&source_id, shape)?;
        for primitive in &mut normalized {
            normalize_source_curve(primitive)?;
        }
        for primitive in normalized {
            match &primitive.geometry {
                MechanicalShape::Circle { .. } => circles.push(primitive),
                _ => {
                    let (start, end) = primitive_endpoints(&primitive)?;
                    edges.push(SourceEdge {
                        start,
                        end,
                        primitive,
                    });
                }
            }
        }
    }
    if edges.is_empty() && circles.is_empty() {
        return Err(validation("PCB source has no authored Edge.Cuts curves"));
    }

    let mut contours = chain_outline_edges(edges)?;
    for circle in circles {
        contours.push(CasePcbSourceContour {
            id: format!("outline/{}", circle.source_id),
            outer_id: None,
            primitives: vec![circle],
        });
    }
    // Validate the exact curve bounds, but retain the established centering
    // translation derived from the compatibility contour projection. Every
    // source and compatibility view must share that one module frame.
    let _ = source_bounds(&contours)?;
    for contour in &mut contours {
        for primitive in &mut contour.primitives {
            canonicalize_legacy_arc(primitive)?;
        }
    }
    ownership::assign(&mut contours)?;
    for contour in &mut contours {
        for primitive in &mut contour.primitives {
            translate_primitive(primitive, center);
        }
    }
    let mut slots = Vec::new();
    let round_drills = extract_drills(root, center, &mut slots)?;
    Ok((CasePcbSource { contours, slots }, round_drills))
}

fn validate_outline_inventory(root: &Node) -> Result<(), ArtifactError> {
    let mut pending = sexpr::items(root)
        .into_iter()
        .flatten()
        .map(|node| (node, true))
        .collect::<Vec<_>>();
    while let Some((node, top_level)) = pending.pop() {
        if layer(node).as_deref() == Some("Edge.Cuts")
            && (!top_level
                || !matches!(
                    sexpr::head(node),
                    Some("gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly")
                ))
        {
            let id = sexpr::child(node, "tstamp")
                .and_then(scalar)
                .unwrap_or("unidentified");
            return Err(validation(format!(
                "Unsupported PCB Edge.Cuts primitive '{}' ({id}){}",
                sexpr::head(node).unwrap_or("unknown"),
                if top_level {
                    ""
                } else {
                    " in a nested source frame"
                },
            )));
        }
        pending.extend(
            sexpr::items(node)
                .into_iter()
                .flatten()
                .map(|child| (child, false)),
        );
    }
    Ok(())
}

fn stable_source_id(node: &Node, description: &str) -> Result<String, ArtifactError> {
    ["tstamp", "uuid"]
        .into_iter()
        .filter_map(|key| sexpr::child(node, key).and_then(scalar))
        .find(|id| !id.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            validation(format!(
                "{description} is missing its stable tstamp or uuid"
            ))
        })
}

fn normalize_outline_shape(
    source_id: &str,
    shape: MechanicalShape,
) -> Result<Vec<CasePcbSourcePrimitive>, ArtifactError> {
    let primitive = |segment_index, geometry, curve_support| CasePcbSourcePrimitive {
        source_id: source_id.to_owned(),
        segment_index,
        geometry,
        curve_support,
    };
    match shape {
        MechanicalShape::Line { .. }
        | MechanicalShape::Arc { .. }
        | MechanicalShape::Circle { .. } => Ok(vec![primitive(0, shape, None)]),
        MechanicalShape::Rectangle { start, end, width } => {
            let points = [
                start,
                Vec2 {
                    x: end.x,
                    y: start.y,
                },
                end,
                Vec2 {
                    x: start.x,
                    y: end.y,
                },
            ];
            Ok((0..4)
                .map(|index| {
                    primitive(
                        index,
                        MechanicalShape::Line {
                            start: points[index as usize],
                            end: points[(index as usize + 1) % 4],
                            width,
                        },
                        None,
                    )
                })
                .collect())
        }
        MechanicalShape::Polygon { points, width } => {
            if points.len() < 3 {
                return Err(validation(
                    "PCB Edge.Cuts polygon needs at least three points",
                ));
            }
            let open_points = if points.first() == points.last() {
                &points[..points.len() - 1]
            } else {
                &points
            };
            if open_points.len() < 3 {
                return Err(validation("PCB Edge.Cuts polygon is degenerate"));
            }
            Ok((0..open_points.len())
                .map(|index| {
                    primitive(
                        index as u32,
                        MechanicalShape::Line {
                            start: open_points[index],
                            end: open_points[(index + 1) % open_points.len()],
                            width,
                        },
                        None,
                    )
                })
                .collect())
        }
        MechanicalShape::Drill { .. } => {
            Err(validation("PCB Edge.Cuts profile cannot contain a drill"))
        }
    }
}

fn normalize_source_curve(primitive: &mut CasePcbSourcePrimitive) -> Result<(), ArtifactError> {
    match &primitive.geometry {
        MechanicalShape::Arc { .. } => {
            let (support, _, _) = arc_support(&primitive.geometry)?;
            primitive.curve_support = Some(support);
        }
        MechanicalShape::Circle { center, end, .. } => {
            let dx = end.x - center.x;
            let dy = end.y - center.y;
            let radius = dx.hypot(dy);
            if !radius.is_finite() || radius <= 0.0 {
                return Err(validation("PCB Edge.Cuts circle has invalid radius"));
            }
            primitive.curve_support = Some(CasePcbCircularSupport {
                center: *center,
                radius_mm: radius,
                start_direction: Vec2 {
                    x: dx / radius,
                    y: dy / radius,
                },
                sweep_radians: TAU,
            });
        }
        _ => {}
    }
    Ok(())
}

fn arc_support(
    shape: &MechanicalShape,
) -> Result<(CasePcbCircularSupport, Vec2, Vec2), ArtifactError> {
    let MechanicalShape::Arc {
        start,
        mid,
        end,
        angle_degrees,
        ..
    } = shape
    else {
        return Err(validation("Expected an authored PCB arc"));
    };
    let (center, _, sweep, radius) = circular_arc_parameters(*start, *mid, *end, *angle_degrees)?;
    let start_point = if mid.is_some() { *start } else { *end };
    if !radius.is_finite() || radius <= 0.0 || !sweep.is_finite() || sweep.abs() <= 0.0 {
        return Err(validation("PCB source arc has invalid circular support"));
    }
    let direction = Vec2 {
        x: (start_point.x - center.x) / radius,
        y: (start_point.y - center.y) / radius,
    };
    let support = CasePcbCircularSupport {
        center,
        radius_mm: radius,
        start_direction: direction,
        sweep_radians: sweep,
    };
    let terminal = if mid.is_some() {
        *end
    } else {
        let angle = direction.y.atan2(direction.x) + sweep;
        support_point(&support, angle)
    };
    Ok((support, start_point, terminal))
}

fn support_point(support: &CasePcbCircularSupport, angle: f64) -> Vec2 {
    Vec2 {
        x: support.center.x + support.radius_mm * angle.cos(),
        y: support.center.y + support.radius_mm * angle.sin(),
    }
}

fn canonicalize_legacy_arc(primitive: &mut CasePcbSourcePrimitive) -> Result<(), ArtifactError> {
    let MechanicalShape::Arc {
        mid: None,
        angle_degrees: Some(_),
        width,
        ..
    } = primitive.geometry
    else {
        return Ok(());
    };
    let support = primitive
        .curve_support
        .ok_or_else(|| validation("PCB source arc is missing curve support"))?;
    let (_, start, end) = arc_support(&primitive.geometry)?;
    let middle =
        support.start_direction.y.atan2(support.start_direction.x) + support.sweep_radians / 2.0;
    // These are consistency witnesses; the retained source support still selects the circle.
    primitive.geometry = MechanicalShape::Arc {
        start,
        mid: Some(support_point(&support, middle)),
        end,
        angle_degrees: None,
        width,
    };
    Ok(())
}

fn primitive_endpoints(primitive: &CasePcbSourcePrimitive) -> Result<(Vec2, Vec2), ArtifactError> {
    match &primitive.geometry {
        MechanicalShape::Line { start, end, .. } => Ok((*start, *end)),
        MechanicalShape::Arc { .. } => {
            let (_, start, end) = arc_support(&primitive.geometry)?;
            Ok((start, end))
        }
        _ => Err(validation(
            "Only line and arc pieces can join an outline loop",
        )),
    }
}

fn chain_outline_edges(
    mut edges: Vec<SourceEdge>,
) -> Result<Vec<CasePcbSourceContour>, ArtifactError> {
    let mut contours = Vec::new();
    while !edges.is_empty() {
        let first = edges.remove(0);
        let loop_start = first.start;
        let mut current = first.end;
        let mut primitives = vec![first.primitive];
        let mut steps = 0;
        while !same_point(current, loop_start) {
            let matches = edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    if same_point(edge.start, current) || same_point(edge.end, current) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if matches.is_empty() {
                return Err(validation("PCB Edge.Cuts curves contain an open contour"));
            }
            if matches.len() != 1 {
                let ids = matches
                    .iter()
                    .map(|&index| edges[index].primitive.source_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(validation(format!(
                    "PCB Edge.Cuts sources [{ids}] have an ambiguous junction"
                )));
            }
            let mut edge = edges.remove(matches[0]);
            if same_point(edge.end, current) {
                edge.primitive = reverse_primitive(edge.primitive)?;
                std::mem::swap(&mut edge.start, &mut edge.end);
            }
            current = edge.end;
            primitives.push(edge.primitive);
            steps += 1;
            if steps > 100_000 {
                return Err(validation("PCB Edge.Cuts contour exceeds the source limit"));
            }
        }
        if primitives.len() < 2 {
            return Err(validation("PCB Edge.Cuts loop has too few curve pieces"));
        }
        let id = format!("outline/{}", primitives[0].source_id);
        contours.push(CasePcbSourceContour {
            id,
            outer_id: None,
            primitives,
        });
    }
    Ok(contours)
}

fn reverse_primitive(
    mut primitive: CasePcbSourcePrimitive,
) -> Result<CasePcbSourcePrimitive, ArtifactError> {
    primitive.geometry = match primitive.geometry {
        MechanicalShape::Line { start, end, width } => MechanicalShape::Line {
            start: end,
            end: start,
            width,
        },
        MechanicalShape::Arc {
            start,
            mid: Some(mid),
            end,
            angle_degrees,
            width,
        } => MechanicalShape::Arc {
            start: end,
            mid: Some(mid),
            end: start,
            angle_degrees,
            width,
        },
        MechanicalShape::Arc {
            start: center,
            mid: None,
            end: radius_point,
            angle_degrees: Some(degrees),
            width,
        } => {
            let (_, _, terminal) = arc_support(&MechanicalShape::Arc {
                start: center,
                mid: None,
                end: radius_point,
                angle_degrees: Some(degrees),
                width,
            })?;
            MechanicalShape::Arc {
                start: center,
                mid: None,
                end: terminal,
                angle_degrees: Some(-degrees),
                width,
            }
        }
        _ => {
            return Err(validation(
                "Only line and arc pieces can reverse in an outline",
            ));
        }
    };
    if let Some(support) = &mut primitive.curve_support {
        let angle =
            support.start_direction.y.atan2(support.start_direction.x) + support.sweep_radians;
        support.start_direction = Vec2 {
            x: angle.cos(),
            y: angle.sin(),
        };
        support.sweep_radians = -support.sweep_radians;
    }
    Ok(primitive)
}

fn source_bounds(contours: &[CasePcbSourceContour]) -> Result<(Vec2, Vec2), ArtifactError> {
    let mut minimum = Vec2 {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut maximum = Vec2 {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    let mut include = |point: Vec2| -> Result<(), ArtifactError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(validation("PCB source outline coordinates must be finite"));
        }
        minimum.x = minimum.x.min(point.x);
        minimum.y = minimum.y.min(point.y);
        maximum.x = maximum.x.max(point.x);
        maximum.y = maximum.y.max(point.y);
        Ok(())
    };
    for primitive in contours.iter().flat_map(|contour| &contour.primitives) {
        match &primitive.geometry {
            MechanicalShape::Line { start, end, .. } => {
                include(*start)?;
                include(*end)?;
            }
            MechanicalShape::Arc { .. } => {
                let support = primitive
                    .curve_support
                    .ok_or_else(|| validation("PCB source arc is missing curve support"))?;
                let (_, start, end) = arc_support(&primitive.geometry)?;
                include(start)?;
                include(end)?;
                for angle in [
                    0.0,
                    std::f64::consts::FRAC_PI_2,
                    std::f64::consts::PI,
                    3.0 * std::f64::consts::FRAC_PI_2,
                ] {
                    if angle_in_sweep(
                        angle,
                        support.start_direction.y.atan2(support.start_direction.x),
                        support.sweep_radians,
                    ) {
                        include(support_point(&support, angle))?;
                    }
                }
            }
            MechanicalShape::Circle { .. } => {
                let support = primitive
                    .curve_support
                    .ok_or_else(|| validation("PCB source circle is missing curve support"))?;
                include(Vec2 {
                    x: support.center.x - support.radius_mm,
                    y: support.center.y - support.radius_mm,
                })?;
                include(Vec2 {
                    x: support.center.x + support.radius_mm,
                    y: support.center.y + support.radius_mm,
                })?;
            }
            _ => {
                return Err(validation(
                    "PCB source profile was not normalized to lines and curves",
                ));
            }
        }
    }
    if !minimum.x.is_finite() {
        return Err(validation("PCB source outline bounds are empty"));
    }
    Ok((minimum, maximum))
}

fn angle_in_sweep(candidate: f64, start: f64, sweep: f64) -> bool {
    let distance = if sweep > 0.0 {
        (candidate - start).rem_euclid(TAU)
    } else {
        (start - candidate).rem_euclid(TAU)
    };
    distance <= sweep.abs() + f64::EPSILON * 16.0
}

fn translate_primitive(primitive: &mut CasePcbSourcePrimitive, by: Vec2) {
    let translate = |point: &mut Vec2| {
        point.x -= by.x;
        point.y -= by.y;
    };
    match &mut primitive.geometry {
        MechanicalShape::Line { start, end, .. }
        | MechanicalShape::Rectangle { start, end, .. } => {
            translate(start);
            translate(end);
        }
        MechanicalShape::Arc {
            start, mid, end, ..
        } => {
            translate(start);
            if let Some(mid) = mid {
                translate(mid);
            }
            translate(end);
        }
        MechanicalShape::Circle { center, end, .. } => {
            translate(center);
            translate(end);
        }
        MechanicalShape::Polygon { points, .. } => points.iter_mut().for_each(translate),
        MechanicalShape::Drill { at, .. } => translate(at),
    }
    if let Some(support) = &mut primitive.curve_support {
        translate(&mut support.center);
    }
}

fn same_point(left: Vec2, right: Vec2) -> bool {
    left.x == right.x && left.y == right.y
}

fn extract_drills(
    root: &Node,
    center: Vec2,
    slots: &mut Vec<CasePcbSlot>,
) -> Result<Vec<Mount>, ArtifactError> {
    let mut ids = BTreeSet::new();
    let mut round_drills = Vec::new();
    for (footprint_index, footprint) in sexpr::items(root)
        .into_iter()
        .flatten()
        .filter(|node| matches!(sexpr::head(node), Some("footprint" | "module")))
        .enumerate()
    {
        let footprint_id = sexpr::child(footprint, "tstamp")
            .and_then(scalar)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("footprint-{footprint_index}"));
        let footprint_at = sexpr::child(footprint, "at")
            .map(point)
            .transpose()?
            .unwrap_or_default();
        let footprint_rotation = sexpr::child(footprint, "at")
            .and_then(|at| sexpr::items(at))
            .and_then(|items| items.get(3))
            .and_then(sexpr::atom)
            .map(parse_finite)
            .transpose()?
            .unwrap_or(0.0);
        for (pad_index, pad) in sexpr::children(footprint, "pad").enumerate() {
            let Some(drill_node) = sexpr::child(pad, "drill") else {
                continue;
            };
            let pad_stamp = sexpr::child(pad, "tstamp")
                .and_then(scalar)
                .filter(|id| !id.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("pad-{pad_index}"));
            let id = format!("{footprint_id}/{pad_stamp}");
            if !ids.insert(id.clone()) {
                return Err(validation("PCB source has duplicate drilled feature IDs"));
            }
            let MechanicalShape::Drill {
                at,
                size,
                offset,
                rotation_degrees,
                shape,
                plated,
            } = parse_drill(pad, drill_node)?
            else {
                unreachable!("pad drill parser returns drill geometry")
            };
            let anchor = rotate(at, footprint_rotation, Vec2::default());
            let board_anchor = Vec2 {
                x: footprint_at.x + anchor.x,
                y: footprint_at.y + anchor.y,
            };
            // KiCad stores pad angles in the board frame, including on B.Cu.
            let rotated_offset = rotate(offset, rotation_degrees, Vec2::default());
            let drill_center = Vec2 {
                x: board_anchor.x + rotated_offset.x,
                y: board_anchor.y + rotated_offset.y,
            };
            if matches!(shape, DrillShape::Circle) && (size.x - size.y).abs() <= 1e-9 {
                round_drills.push(Mount {
                    id,
                    at: Vec2 {
                        x: drill_center.x - center.x,
                        y: drill_center.y - center.y,
                    },
                    kind: MountKind::Hole,
                    hole_diameter: size.x,
                    boss_diameter: None,
                    height: None,
                });
            } else {
                let resolved = MechanicalShape::Drill {
                    at: Vec2 {
                        x: board_anchor.x - center.x,
                        y: board_anchor.y - center.y,
                    },
                    size,
                    offset,
                    rotation_degrees,
                    shape,
                    plated,
                };
                slots.push(CasePcbSlot {
                    id,
                    geometry: resolved,
                });
            }
        }
    }
    for (via_index, via) in sexpr::children(root, "via").enumerate() {
        let id = ["tstamp", "uuid"]
            .into_iter()
            .filter_map(|key| sexpr::child(via, key).and_then(scalar))
            .find(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("via-{via_index}"));
        if !ids.insert(id.clone()) {
            return Err(validation("PCB source has duplicate drilled feature IDs"));
        }
        // KiCad omits the through-via type; locked is an independent legacy flag.
        if let Some(kind) = sexpr::items(via)
            .into_iter()
            .flatten()
            .skip(1)
            .filter_map(sexpr::atom)
            .find(|kind| *kind != "locked")
        {
            return Err(validation(format!(
                "Unsupported PCB via '{id}' type '{kind}': partial or unknown depth cannot be represented as a through-hole"
            )));
        }
        let mut spans = sexpr::children(via, "layers");
        let through_span = spans.next().and_then(sexpr::items).is_some_and(|items| {
            items.len() == 3
                && matches!(
                    (sexpr::atom(&items[1]), sexpr::atom(&items[2])),
                    (Some("F.Cu"), Some("B.Cu")) | (Some("B.Cu"), Some("F.Cu"))
                )
        });
        if !through_span || spans.next().is_some() {
            return Err(validation(format!(
                "Unsupported PCB via '{id}' layer span: only a resolved F.Cu/B.Cu through-hole can be represented"
            )));
        }
        let drill = sexpr::child(via, "drill")
            .and_then(scalar)
            .ok_or_else(|| validation("PCB via is missing its drill diameter"))
            .and_then(parse_finite)?;
        if drill <= 0.0 {
            return Err(validation("PCB via drill diameter must be positive"));
        }
        let at = sexpr::child(via, "at")
            .map(point)
            .transpose()?
            .ok_or_else(|| validation("PCB via is missing its authored centre"))?;
        round_drills.push(Mount {
            id,
            at: Vec2 {
                x: at.x - center.x,
                y: at.y - center.y,
            },
            kind: MountKind::Hole,
            hole_diameter: drill,
            boss_diameter: None,
            height: None,
        });
    }
    round_drills.sort_by(|left, right| left.id.cmp(&right.id));
    slots.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(round_drills)
}
