//! Approximate geometry projection for source-preserving KiCad footprints.
//!
//! The returned geometry is for preview and conservative layout bounds only;
//! callers must preserve the original source as the export authority.

use super::sexpr;
use crate::model::{
    ArtifactDiagnostic, ArtifactDiagnosticKind, ArtifactError, ArtifactErrorCode, Pad, PadShape,
    Side, Vec2,
};
use kiutils_sexpr::Node;

pub(super) struct SourceGeometryProjection {
    pub pads: Vec<Pad>,
    pub courtyard: Vec<Vec2>,
    pub conservative_bounds: Vec<Vec2>,
    pub diagnostics: Vec<ArtifactDiagnostic>,
}

pub(super) fn project_source_geometry(
    source: &str,
    root: &Node,
) -> Result<SourceGeometryProjection, ArtifactError> {
    let mut diagnostics = Vec::new();
    let pads = sexpr::children(root, "pad")
        .enumerate()
        .map(|(index, node)| project_pad(source, node, index, &mut diagnostics, root))
        .collect::<Result<Vec<_>, _>>()?;
    let courtyard = project_courtyard(root, &mut diagnostics)?;
    let unknown_custom_geometry = diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("unrecognized custom pad primitive")
    });
    let conservative_bounds = if unknown_custom_geometry {
        Vec::new()
    } else {
        bounds_from_pads_and_graphics(&pads, root)?.unwrap_or_default()
    };
    if courtyard.is_empty() {
        diagnostics.push(notice(if conservative_bounds.is_empty() {
            "No closed courtyard or reliable physical bounds were found; add an authored layout envelope"
        } else {
            "No closed courtyard was found; preview uses conservative physical bounds"
        }));
    }
    Ok(SourceGeometryProjection {
        pads,
        courtyard,
        conservative_bounds,
        diagnostics,
    })
}

fn error(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::ParseError, message)
}
fn notice(message: &str) -> ArtifactDiagnostic {
    ArtifactDiagnostic {
        kind: ArtifactDiagnosticKind::Approximation,
        message: message.into(),
        target_id: None,
        source_start: None,
        source_end: None,
    }
}

fn project_pad(
    source: &str,
    node: &Node,
    index: usize,
    notices: &mut Vec<ArtifactDiagnostic>,
    root: &Node,
) -> Result<Pad, ArtifactError> {
    let root_back = root_is_back(root);
    let parts = sexpr::items(node).ok_or_else(|| error("Invalid pad"))?;
    let number = parts
        .get(1)
        .and_then(|node| sexpr::kicad_quoted(source, sexpr::span(node)))
        .or_else(|| parts.get(1).and_then(sexpr::atom).map(str::to_owned))
        .unwrap_or_default();
    let kind = parts.get(2).and_then(sexpr::atom).unwrap_or("");
    let source_shape = parts.get(3).and_then(sexpr::atom).unwrap_or("");
    let at_node =
        sexpr::child(node, "at").ok_or_else(|| error(format!("Pad {number} has no position")))?;
    let at = root_local_point(point(at_node, "pad position")?, root_back);
    let mut size = positive_pair(
        sexpr::child(node, "size").ok_or_else(|| error(format!("Pad {number} has no size")))?,
        "pad size",
    )?;
    let rotation = Some(
        sexpr::items(at_node)
            .and_then(|v| v.get(3))
            .and_then(sexpr::atom)
            .map(parse_finite)
            .transpose()?
            .unwrap_or(0.0),
    );
    // Back-side footprints reflect local Y. Normalize that frame and layer
    // assignment so the projected definition can be placed on either side.
    let back = root_back;
    let pad_back = sexpr::child(node, "layers")
        .and_then(sexpr::items)
        .map(|v| {
            v.iter()
                .filter_map(sexpr::atom)
                .any(|s| s.starts_with("B."))
        })
        .unwrap_or(false);
    let mut approximations = Vec::new();
    let drill_node = sexpr::child(node, "drill");
    let drill_values = drill_node.and_then(sexpr::items).unwrap_or(&[]);
    let drill_numbers = drill_values
        .iter()
        .skip(1)
        .filter_map(sexpr::atom)
        .filter(|v| v.parse::<f64>().is_ok())
        .collect::<Vec<_>>();
    let drill = drill_numbers.first().map(|v| parse_finite(v)).transpose()?;
    if let Some(offset_node) = drill_node.and_then(|node| sexpr::child(node, "offset")) {
        let mut offset = point(offset_node, "drill offset")?;
        if root_back {
            offset.y = -offset.y;
        }
        let hole_size = if drill_numbers.len() >= 2 {
            Vec2 {
                x: parse_finite(drill_numbers[0])?,
                y: parse_finite(drill_numbers[1])?,
            }
        } else {
            let diameter = drill.unwrap_or(0.0);
            Vec2 {
                x: diameter,
                y: diameter,
            }
        };
        size.x = size.x.max(offset.x.abs() * 2.0 + hole_size.x);
        size.y = size.y.max(offset.y.abs() * 2.0 + hole_size.y);
        approximations.push("offset drill bounds");
    }
    if let Some(drill_node) = drill_node {
        let values = sexpr::items(drill_node).unwrap_or(&[]);
        let slotted = values.iter().any(|v| sexpr::atom(v) == Some("oval")) || values.len() > 3;
        if slotted {
            approximations.push("slotted drill");
        }
        if kind == "thru_hole" {
            if slotted && drill_numbers.len() >= 2 {
                size.x = size.x.max(parse_finite(drill_numbers[0])?);
                size.y = size.y.max(parse_finite(drill_numbers[1])?);
            } else if let Some(diameter) = drill {
                size.x = size.x.max(diameter);
                size.y = size.y.max(diameter);
            }
        }
    }
    if source_shape == "custom" {
        approximations.push("custom pad shape");
        if let Some(primitives) = sexpr::child(node, "primitives") {
            if sexpr::items(primitives)
                .into_iter()
                .flatten()
                .skip(1)
                .any(|primitive| {
                    !matches!(
                        sexpr::head(primitive),
                        Some("gr_line" | "gr_rect" | "gr_poly" | "gr_arc" | "gr_circle")
                    )
                })
            {
                approximations.push("unrecognized custom pad primitive");
            }
            if let Some(extent) = primitive_extent(primitives)? {
                size.x = size.x.max(extent.x * 2.0);
                size.y = size.y.max(extent.y * 2.0);
            } else {
                approximations.push("custom pad primitives without measurable extents");
            }
        }
    }
    let shape = match source_shape {
        "circle" => PadShape::Circle,
        "oval" => PadShape::Oval,
        "roundrect" => PadShape::Roundrect,
        _ => PadShape::Rect,
    };
    if source_shape == "roundrect"
        || source_shape == "custom"
        || (kind == "thru_hole" && drill_node.is_some())
    {
        approximations.push("pad/drill rendering");
    }
    if !approximations.is_empty() {
        approximations.sort_unstable();
        approximations.dedup();
        notices.push(ArtifactDiagnostic {
            kind: ArtifactDiagnosticKind::Approximation,
            message: format!(
                "{} preview is approximate ({})",
                if number.is_empty() {
                    "Pad".to_owned()
                } else {
                    format!("Pad {number}")
                },
                approximations.join(", ")
            ),
            target_id: Some(format!("pad-{index}")),
            source_start: None,
            source_end: None,
        });
    }
    Ok(Pad {
        id: format!("pad-{index}"),
        number,
        at,
        size,
        shape,
        drill,
        plated: Some(kind == "thru_hole"),
        side: Some(if pad_back != back {
            Side::Back
        } else {
            Side::Front
        }),
        rotation: rotation.map(|r| {
            let root_rotation = root_rotation(root);
            if back {
                root_rotation - r
            } else {
                r - root_rotation
            }
        }),
        net_id: None,
    })
}

fn parse_finite(value: &str) -> Result<f64, ArtifactError> {
    let number = value
        .parse::<f64>()
        .map_err(|_| error("Invalid numeric coordinate or dimension"))?;
    if !number.is_finite() {
        return Err(error("Invalid non-finite coordinate or dimension"));
    }
    Ok(number)
}

fn root_is_back(root: &Node) -> bool {
    sexpr::child(root, "layer")
        .and_then(sexpr::items)
        .and_then(|v| v.get(1))
        .and_then(sexpr::atom)
        == Some("B.Cu")
}

fn root_rotation(root: &Node) -> f64 {
    sexpr::child(root, "at")
        .and_then(sexpr::items)
        .and_then(|values| values.get(3))
        .and_then(sexpr::atom)
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite())
        .unwrap_or(0.0)
}

fn root_local_point(point: Vec2, back: bool) -> Vec2 {
    // KiCad footprint child positions are already local. Root `at` affects
    // absolute orientations only, so applying it to points would transform
    // geometry twice.
    let mut local = point;
    if back {
        local.y = -local.y;
    }
    local
}

fn point(node: &Node, label: &str) -> Result<Vec2, ArtifactError> {
    let values = sexpr::items(node).ok_or_else(|| error(format!("Invalid {label}")))?;
    let x = values
        .get(1)
        .and_then(sexpr::atom)
        .ok_or_else(|| error(format!("Invalid {label}")))
        .and_then(parse_finite)?;
    let y = values
        .get(2)
        .and_then(sexpr::atom)
        .ok_or_else(|| error(format!("Invalid {label}")))
        .and_then(parse_finite)?;
    Ok(Vec2 {
        x,
        y: if y == 0.0 { 0.0 } else { -y },
    })
}
fn positive_pair(node: &Node, label: &str) -> Result<Vec2, ArtifactError> {
    let values = sexpr::items(node).ok_or_else(|| error(format!("Invalid {label}")))?;
    let x = values
        .get(1)
        .and_then(sexpr::atom)
        .ok_or_else(|| error(format!("Invalid {label}")))
        .and_then(parse_finite)?;
    let y = values
        .get(2)
        .and_then(sexpr::atom)
        .ok_or_else(|| error(format!("Invalid {label}")))
        .and_then(parse_finite)?;
    if x <= 0.0 || y <= 0.0 {
        return Err(error(format!("Invalid {label}")));
    }
    Ok(Vec2 { x, y })
}

fn project_courtyard(
    root: &Node,
    notices: &mut Vec<ArtifactDiagnostic>,
) -> Result<Vec<Vec2>, ArtifactError> {
    let mut edges: Vec<(Vec2, Vec2, Vec<Vec2>)> = Vec::new();
    let mut explicit_shapes = Vec::<Vec<Vec2>>::new();
    let front_courtyard = sexpr::items(root).into_iter().flatten().any(|node| {
        matches!(
            sexpr::head(node),
            Some("fp_line" | "fp_arc" | "fp_circle" | "fp_poly" | "fp_rect")
        ) && sexpr::child(node, "layer")
            .and_then(sexpr::items)
            .and_then(|v| v.get(1))
            .and_then(sexpr::atom)
            == Some("F.CrtYd")
    });
    let has_back_courtyard = sexpr::items(root).into_iter().flatten().any(|node| {
        matches!(
            sexpr::head(node),
            Some("fp_line" | "fp_arc" | "fp_circle" | "fp_poly" | "fp_rect")
        ) && sexpr::child(node, "layer")
            .and_then(sexpr::items)
            .and_then(|v| v.get(1))
            .and_then(sexpr::atom)
            == Some("B.CrtYd")
    });
    let has_root_side = sexpr::child(root, "layer").is_some();
    let back = if has_root_side {
        root_is_back(root)
    } else {
        !front_courtyard && has_back_courtyard
    };
    let target_layer = if back { "B.CrtYd" } else { "F.CrtYd" };
    for node in sexpr::items(root).into_iter().flatten().filter(|node| {
        matches!(
            sexpr::head(node),
            Some("fp_line" | "fp_arc" | "fp_circle" | "fp_poly" | "fp_rect")
        )
    }) {
        let layer = sexpr::child(node, "layer")
            .and_then(|n| sexpr::items(n))
            .and_then(|v| v.get(1))
            .and_then(sexpr::atom);
        if layer != Some(target_layer) {
            continue;
        }
        let convert = |p: Vec2| Ok(root_local_point(p, back));
        match sexpr::head(node) {
            Some("fp_line") => {
                if let (Some(a), Some(b)) = (sexpr::child(node, "start"), sexpr::child(node, "end"))
                {
                    let a = convert(point(a, "courtyard")?)?;
                    let b = convert(point(b, "courtyard")?)?;
                    edges.push((a, b, vec![a, b]));
                }
            }
            Some("fp_arc") => {
                let curve = flatten_arc(node)?;
                if let (Some(a), Some(b)) = (curve.first(), curve.last()) {
                    let a = convert(*a)?;
                    let b = convert(*b)?;
                    let curve = curve
                        .into_iter()
                        .map(convert)
                        .collect::<Result<Vec<_>, _>>()?;
                    edges.push((a, b, curve));
                }
            }
            Some("fp_rect") => {
                let a = point(
                    sexpr::child(node, "start")
                        .ok_or_else(|| error("Courtyard rectangle needs start"))?,
                    "courtyard",
                )?;
                let b = point(
                    sexpr::child(node, "end")
                        .ok_or_else(|| error("Courtyard rectangle needs end"))?,
                    "courtyard",
                )?;
                let a = convert(a)?;
                let b = convert(b)?;
                explicit_shapes.push(vec![a, Vec2 { x: b.x, y: a.y }, b, Vec2 { x: a.x, y: b.y }]);
            }
            Some("fp_circle") => {
                let c = point(
                    sexpr::child(node, "center")
                        .ok_or_else(|| error("Courtyard circle needs center"))?,
                    "courtyard",
                )?;
                let e = point(
                    sexpr::child(node, "end").ok_or_else(|| error("Courtyard circle needs end"))?,
                    "courtyard",
                )?;
                let c = convert(c)?;
                let e = convert(e)?;
                let r = ((e.x - c.x).powi(2) + (e.y - c.y).powi(2)).sqrt();
                let half_step = (r / (r + 0.01)).acos();
                let count = (std::f64::consts::PI / half_step).ceil().max(12.0) as usize;
                let vertex_radius = r / (std::f64::consts::PI / count as f64).cos();
                explicit_shapes.push(
                    (0..count)
                        .map(|i| {
                            let angle = std::f64::consts::TAU * i as f64 / count as f64;
                            Vec2 {
                                x: c.x + vertex_radius * angle.cos(),
                                y: c.y + vertex_radius * angle.sin(),
                            }
                        })
                        .collect(),
                );
            }
            Some("fp_poly") => {
                if let Some(pts) = sexpr::child(node, "pts") {
                    let mut polygon = Vec::new();
                    for xy in sexpr::children(pts, "xy") {
                        polygon.push(convert(point(xy, "courtyard polygon")?)?);
                    }
                    if polygon.len() >= 3 {
                        explicit_shapes.push(polygon);
                    }
                }
            }
            _ => {}
        }
    }
    if !explicit_shapes.is_empty() {
        notices.push(notice(
            "Courtyard circles, rectangles, or polygons are projected as preview outlines",
        ));
        if explicit_shapes.len() == 1 && edges.is_empty() {
            return Ok(explicit_shapes.remove(0));
        }
        let mut points = explicit_shapes.into_iter().flatten().collect::<Vec<_>>();
        points.extend(edges.iter().flat_map(|(_, _, curve)| curve.iter().copied()));
        return Ok(convex_hull(points));
    }
    Ok(chain_courtyard_edges(edges).unwrap_or_default())
}

fn chain_courtyard_edges(mut edges: Vec<(Vec2, Vec2, Vec<Vec2>)>) -> Option<Vec<Vec2>> {
    // Chain shuffled and reversed segments by endpoint proximity. A single
    // polygon can represent only one loop, so multiple loops become a
    // conservative convex envelope rather than concatenated invalid rings.
    let tolerance = 0.01;
    let mut loops = Vec::new();
    while !edges.is_empty() {
        let (_, _, mut chain) = edges.remove(0);
        let first = *chain.first()?;
        let mut current = *chain.last()?;
        while !near(current, first, tolerance) {
            let index = edges.iter().position(|(a, b, _)| {
                near(*a, current, tolerance) || near(*b, current, tolerance)
            })?;
            let (a, b, mut curve) = edges.remove(index);
            if near(b, current, tolerance) {
                curve.reverse();
            }
            if near(*curve.first()?, current, tolerance) {
                curve.remove(0);
            }
            chain.extend(curve);
            current = if near(a, current, tolerance) { b } else { a };
        }
        if chain.len() < 4 {
            return None;
        }
        if chain
            .last()
            .is_some_and(|last| near(*last, first, tolerance))
        {
            chain.pop();
        }
        loops.push(chain);
    }
    if loops.len() == 1 {
        return loops.pop();
    }
    Some(convex_hull(loops.into_iter().flatten().collect()))
}

fn convex_hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup_by(|a, b| near(*a, *b, 1e-12));
    if points.len() <= 2 {
        return points;
    }
    let cross = |o: Vec2, a: Vec2, b: Vec2| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}
fn near(a: Vec2, b: Vec2, tol: f64) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= tol
}
fn flatten_arc(node: &Node) -> Result<Vec<Vec2>, ArtifactError> {
    let start_node = sexpr::child(node, "start").ok_or_else(|| error("Arc needs start"))?;
    let end_node = sexpr::child(node, "end").ok_or_else(|| error("Arc needs end"))?;
    if let Some(angle_node) = sexpr::child(node, "angle") {
        // KiCad 5 stored an arc as its center, an endpoint, and a signed sweep.
        let center = point(start_node, "legacy arc center")?;
        let endpoint = point(end_node, "legacy arc endpoint")?;
        let radius = (endpoint.x - center.x).hypot(endpoint.y - center.y);
        if radius <= 0.0 {
            return Err(error("Degenerate legacy courtyard arc"));
        }
        let sweep = sexpr::items(angle_node)
            .and_then(|parts| parts.get(1))
            .and_then(sexpr::atom)
            .ok_or_else(|| error("Legacy arc needs angle"))
            .and_then(parse_finite)?
            .to_radians();
        // KiCad 5's positive sweep is clockwise in its Y-down frame. `point`
        // already converted that frame to Y-up, so negate the sweep here.
        let sweep = -sweep;
        let initial = (endpoint.y - center.y).atan2(endpoint.x - center.x);
        let step = 2.0 * (1.0 - 0.01 / radius.max(0.01)).clamp(-1.0, 1.0).acos();
        let count = (sweep.abs() / step.max(0.001)).ceil().max(2.0) as usize;
        return Ok((0..=count)
            .map(|index| {
                let angle = initial + sweep * index as f64 / count as f64;
                Vec2 {
                    x: center.x + radius * angle.cos(),
                    y: center.y + radius * angle.sin(),
                }
            })
            .collect());
    }
    let start = point(start_node, "arc start")?;
    let mid = point(
        sexpr::child(node, "mid").ok_or_else(|| error("Arc needs midpoint"))?,
        "arc midpoint",
    )?;
    let end = point(end_node, "arc end")?;
    let (cx, cy, r) =
        circumcenter(start, mid, end).ok_or_else(|| error("Degenerate courtyard arc"))?;
    let angle = |p: Vec2| (p.y - cy).atan2(p.x - cx);
    let a0 = angle(start);
    let am = angle(mid);
    let a1 = angle(end);
    let ccw =
        (am - a0).rem_euclid(std::f64::consts::TAU) <= (a1 - a0).rem_euclid(std::f64::consts::TAU);
    let sweep = if ccw {
        (a1 - a0).rem_euclid(std::f64::consts::TAU)
    } else {
        -((a0 - a1).rem_euclid(std::f64::consts::TAU))
    };
    let step = 2.0 * (1.0 - 0.01 / r.max(0.01)).clamp(-1.0, 1.0).acos();
    let count = (sweep.abs() / step.max(0.001)).ceil().max(2.0) as usize;
    Ok((0..=count)
        .map(|i| {
            let a = a0 + sweep * i as f64 / count as f64;
            Vec2 {
                x: cx + r * a.cos(),
                y: cy + r * a.sin(),
            }
        })
        .collect())
}
fn circumcenter(a: Vec2, b: Vec2, c: Vec2) -> Option<(f64, f64, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let aa = a.x * a.x + a.y * a.y;
    let bb = b.x * b.x + b.y * b.y;
    let cc = c.x * c.x + c.y * c.y;
    let x = (aa * (b.y - c.y) + bb * (c.y - a.y) + cc * (a.y - b.y)) / d;
    let y = (aa * (c.x - b.x) + bb * (a.x - c.x) + cc * (b.x - a.x)) / d;
    Some((x, y, (a.x - x).hypot(a.y - y)))
}

fn primitive_extent(node: &Node) -> Result<Option<Vec2>, ArtifactError> {
    let mut max = Vec2::default();
    let mut found = false;
    let mut stroke_radius: f64 = 0.0;
    for item in sexpr::items(node).into_iter().flatten().skip(1) {
        if let Some(width) = graphic_width(item) {
            stroke_radius = stroke_radius.max(parse_finite(width)?.abs() / 2.0);
        }
        match sexpr::head(item) {
            Some("gr_arc") => {
                for p in flatten_arc(item)? {
                    max.x = max.x.max(p.x.abs());
                    max.y = max.y.max(p.y.abs());
                    found = true;
                }
            }
            Some("gr_circle") => {
                let c = point(
                    sexpr::child(item, "center")
                        .ok_or_else(|| error("Custom circle needs center"))?,
                    "custom primitive",
                )?;
                let e = point(
                    sexpr::child(item, "end").ok_or_else(|| error("Custom circle needs end"))?,
                    "custom primitive",
                )?;
                let r = (e.x - c.x).hypot(e.y - c.y);
                max.x = max.x.max(c.x.abs() + r);
                max.y = max.y.max(c.y.abs() + r);
                found = true;
            }
            Some("gr_poly") => {
                if let Some(pts) = sexpr::child(item, "pts") {
                    for xy in sexpr::children(pts, "xy") {
                        let p = point(xy, "custom polygon")?;
                        max.x = max.x.max(p.x.abs());
                        max.y = max.y.max(p.y.abs());
                        found = true;
                    }
                }
            }
            _ => {}
        }
        for key in ["start", "end", "center", "at"] {
            if let Some(p) = sexpr::child(item, key) {
                let p = point(p, "custom pad primitive")?;
                max.x = max.x.max(p.x.abs());
                max.y = max.y.max(p.y.abs());
                found = true;
            }
        }
        if let Some(size) = sexpr::child(item, "size") {
            let s = positive_pair(size, "custom primitive size")?;
            max.x = max.x.max(s.x / 2.0);
            max.y = max.y.max(s.y / 2.0);
            found = true;
        }
    }
    max.x += stroke_radius;
    max.y += stroke_radius;
    Ok(found.then_some(max))
}
fn bounds_from_pads(pads: &[Pad]) -> Option<Vec<Vec2>> {
    if pads.is_empty() {
        return None;
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in pads {
        let a = p.rotation.unwrap_or(0.0).to_radians();
        let hx = (a.cos().abs() * p.size.x + a.sin().abs() * p.size.y) / 2.0;
        let hy = (a.sin().abs() * p.size.x + a.cos().abs() * p.size.y) / 2.0;
        minx = minx.min(p.at.x - hx);
        maxx = maxx.max(p.at.x + hx);
        miny = miny.min(p.at.y - hy);
        maxy = maxy.max(p.at.y + hy);
    }
    let m = 0.5;
    Some(vec![
        Vec2 {
            x: minx - m,
            y: miny - m,
        },
        Vec2 {
            x: maxx + m,
            y: miny - m,
        },
        Vec2 {
            x: maxx + m,
            y: maxy + m,
        },
        Vec2 {
            x: minx - m,
            y: maxy + m,
        },
    ])
}

fn bounds_from_pads_and_graphics(
    pads: &[Pad],
    root: &Node,
) -> Result<Option<Vec<Vec2>>, ArtifactError> {
    let mut points = Vec::new();
    if let Some(pad_bounds) = bounds_from_pads(pads) {
        points.extend(pad_bounds);
    }
    for node in sexpr::items(root).into_iter().flatten() {
        if !matches!(
            sexpr::head(node),
            Some("fp_line" | "fp_arc" | "fp_circle" | "fp_poly" | "fp_rect" | "fp_text")
        ) {
            continue;
        }
        let layer = sexpr::child(node, "layer")
            .and_then(sexpr::items)
            .and_then(|v| v.get(1))
            .and_then(sexpr::atom)
            .unwrap_or("");
        if layer.ends_with(".CrtYd") {
            continue;
        }
        let point_start = points.len();
        let convert = |point| Ok(root_local_point(point, root_is_back(root)));
        match sexpr::head(node) {
            Some("fp_line") => {
                for key in ["start", "end"] {
                    if let Some(p) = sexpr::child(node, key) {
                        points.push(convert(point(p, "graphic")?)?);
                    }
                }
            }
            Some("fp_arc") => points.extend(
                flatten_arc(node)?
                    .into_iter()
                    .map(convert)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Some("fp_circle") => {
                let c = convert(point(
                    sexpr::child(node, "center")
                        .ok_or_else(|| error("Graphic circle needs center"))?,
                    "graphic",
                )?)?;
                let e = convert(point(
                    sexpr::child(node, "end").ok_or_else(|| error("Graphic circle needs end"))?,
                    "graphic",
                )?)?;
                let r = (e.x - c.x).hypot(e.y - c.y);
                points.extend([
                    Vec2 {
                        x: c.x - r,
                        y: c.y - r,
                    },
                    Vec2 {
                        x: c.x + r,
                        y: c.y + r,
                    },
                ]);
            }
            Some("fp_rect") => {
                let a = point(
                    sexpr::child(node, "start")
                        .ok_or_else(|| error("Graphic rectangle needs start"))?,
                    "graphic",
                )?;
                let b = point(
                    sexpr::child(node, "end")
                        .ok_or_else(|| error("Graphic rectangle needs end"))?,
                    "graphic",
                )?;
                points.extend([
                    convert(a)?,
                    convert(Vec2 { x: b.x, y: a.y })?,
                    convert(b)?,
                    convert(Vec2 { x: a.x, y: b.y })?,
                ]);
            }
            Some("fp_poly") => {
                if let Some(pts) = sexpr::child(node, "pts") {
                    for xy in sexpr::children(pts, "xy") {
                        points.push(convert(point(xy, "graphic polygon")?)?);
                    }
                }
            }
            Some("fp_text") => {
                if let Some(at) = sexpr::child(node, "at") {
                    let c = convert(point(at, "text position")?)?;
                    let size = sexpr::child(node, "effects")
                        .and_then(|e| sexpr::child(e, "font"))
                        .and_then(|f| sexpr::child(f, "size"))
                        .map(|s| positive_pair(s, "text size"))
                        .transpose()?
                        .unwrap_or(Vec2 { x: 1.0, y: 1.0 });
                    points.extend([
                        Vec2 {
                            x: c.x - size.x,
                            y: c.y - size.y,
                        },
                        Vec2 {
                            x: c.x + size.x,
                            y: c.y + size.y,
                        },
                    ]);
                }
            }
            _ => {}
        }
        let width = graphic_width(node)
            .map(parse_finite)
            .transpose()?
            .unwrap_or(0.05);
        if point_start < points.len() {
            let local = &points[point_start..];
            let min_x = local
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min)
                - width / 2.0;
            let max_x = local
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max)
                + width / 2.0;
            let min_y = local
                .iter()
                .map(|point| point.y)
                .fold(f64::INFINITY, f64::min)
                - width / 2.0;
            let max_y = local
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max)
                + width / 2.0;
            points.extend([
                Vec2 { x: min_x, y: min_y },
                Vec2 { x: max_x, y: min_y },
                Vec2 { x: max_x, y: max_y },
                Vec2 { x: min_x, y: max_y },
            ]);
        }
    }
    if points.is_empty() {
        return Ok(None);
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in points {
        minx = minx.min(p.x);
        maxx = maxx.max(p.x);
        miny = miny.min(p.y);
        maxy = maxy.max(p.y);
    }
    let m = 0.5;
    Ok(Some(vec![
        Vec2 {
            x: minx - m,
            y: miny - m,
        },
        Vec2 {
            x: maxx + m,
            y: miny - m,
        },
        Vec2 {
            x: maxx + m,
            y: maxy + m,
        },
        Vec2 {
            x: minx - m,
            y: maxy + m,
        },
    ]))
}

fn graphic_width(node: &Node) -> Option<&str> {
    sexpr::child(node, "stroke")
        .and_then(|stroke| sexpr::child(stroke, "width"))
        .or_else(|| sexpr::child(node, "width"))
        .and_then(sexpr::items)
        .and_then(|values| values.get(1))
        .and_then(sexpr::atom)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project(body: &str) -> Result<SourceGeometryProjection, ArtifactError> {
        let source = format!("(footprint \"test\" {body})");
        let doc = kiutils_sexpr::parse_one(&source).unwrap();
        project_source_geometry(&source, &doc.nodes[0])
    }
    #[test]
    fn pad_projection_keeps_duplicate_numbers_and_unique_ids_with_y_up_rotation_and_sizes() {
        let p=project("(pad \"1\" smd rect (at 2 3 30) (size 4 2) (layers \"F.Cu\")) (pad \"1\" thru_hole oval (at -1 -2) (size 3 1) (drill oval 1 2) (layers \"*.Cu\"))").unwrap();
        assert_eq!(p.pads[0].id, "pad-0");
        assert_eq!(p.pads[1].id, "pad-1");
        assert_eq!(p.pads[0].number, "1");
        assert_eq!(p.pads[1].number, "1");
        assert_eq!(p.pads[0].at, Vec2 { x: 2.0, y: -3.0 });
        assert_eq!(p.pads[0].rotation, Some(30.0));
        assert_eq!(p.pads[0].size, Vec2 { x: 4.0, y: 2.0 });
        assert_eq!(p.pads[1].plated, Some(true));
        assert!(!p.diagnostics.is_empty());
    }
    #[test]
    fn courtyard_chains_shuffled_reversed_segments_and_supports_both_sides() {
        let p=project("(fp_line (start 10 10) (end 0 10) (layer \"F.CrtYd\")) (fp_line (start 0 0) (end 0 10) (layer \"F.CrtYd\")) (fp_line (start 10 0) (end 0 0) (layer \"F.CrtYd\")) (fp_line (start 10 0) (end 10 10) (layer \"F.CrtYd\"))").unwrap();
        assert_eq!(p.courtyard.len(), 4);
        assert_eq!(
            p.courtyard
                .iter()
                .map(|p| p.x)
                .fold(f64::INFINITY, f64::min),
            0.0
        );
        let b = project("(fp_rect (start 1 2) (end 5 8) (layer \"B.CrtYd\"))").unwrap();
        assert_eq!(b.courtyard.len(), 4);
        assert!(b.courtyard.iter().any(|p| p.x == 5.0 && p.y == 8.0));
    }
    #[test]
    fn courtyard_arc_flattening_respects_tolerance_and_open_edges_fall_back() {
        let arc=project("(fp_arc (start 1 0) (mid 0 1) (end -1 0) (layer \"F.CrtYd\")) (fp_line (start -1 0) (end 1 0) (layer \"F.CrtYd\"))").unwrap();
        assert!(arc.courtyard.len() > 10);
        assert!(arc.courtyard.iter().any(|p| p.y < -0.99));
        for pair in arc.courtyard.windows(2) {
            let a = pair[0].y.atan2(pair[0].x);
            let b = pair[1].y.atan2(pair[1].x);
            let delta = (b - a).abs().min(std::f64::consts::TAU - (b - a).abs());
            assert!(1.0 - (delta / 2.0).cos() <= 0.010001);
        }
        let open=project("(pad \"A\" smd rect (at 0 0 45) (size 2 4) (layers \"F.Cu\")) (fp_line (start 0 0) (end 2 0) (layer \"F.CrtYd\"))").unwrap();
        assert!(open.courtyard.is_empty());
        assert_eq!(open.conservative_bounds.len(), 4);
    }
    #[test]
    fn malformed_or_negative_sizes_fail_but_missing_bounds_are_a_notice() {
        assert!(project("(pad \"1\" smd rect (at 0 0) (size -1 2) (layers \"F.Cu\"))").is_err());
        let empty = project("").unwrap();
        assert!(empty.courtyard.is_empty());
        assert!(empty.conservative_bounds.is_empty());
        assert!(
            empty
                .diagnostics
                .iter()
                .any(|d| d.message.contains("authored layout envelope"))
        );
    }

    #[test]
    fn slotted_drill_and_mixed_layers_keep_shared_local_coordinates() {
        let p=project("(pad \"1\" thru_hole oval (at 2 3) (size 3 1) (drill oval 0.9 1.4) (layers \"*.Cu\")) (pad \"2\" smd rect (at 2 3 15) (size 1 1) (layers \"F.Cu\" \"B.Cu\"))").unwrap();
        assert_eq!(p.pads[0].drill, Some(0.9));
        assert_eq!(p.pads[0].at, Vec2 { x: 2.0, y: -3.0 });
        assert_eq!(p.pads[0].size.y, 1.4);
        assert_eq!(p.pads[1].at, Vec2 { x: 2.0, y: -3.0 });
        assert_eq!(p.pads[1].rotation, Some(15.0));
        assert!(
            p.diagnostics
                .iter()
                .any(|d| d.message.contains("slotted drill"))
        );
        let back =
            project("(layer \"B.Cu\") (pad \"1\" smd rect (at 2 3) (size 1 1) (layers \"B.Cu\"))")
                .unwrap();
        assert_eq!(back.pads[0].at, Vec2 { x: 2.0, y: 3.0 });
    }

    #[test]
    fn missing_pad_angle_is_absolute_zero_before_root_normalization() {
        let p = project("(at 0 0 37) (pad \"1\" smd rect (at 2 3) (size 1 1) (layers \"F.Cu\"))")
            .unwrap();
        assert_eq!(p.pads[0].rotation, Some(-37.0));
    }

    #[test]
    fn legacy_arc_center_end_angle_projects_as_a_curve() {
        let p =
            project("(fp_arc (start 0 0) (end 2 0) (angle 90) (layer \"F.SilkS\") (width 0.15))")
                .unwrap();
        assert!(!p.conservative_bounds.is_empty());
        let doc = kiutils_sexpr::parse_one(
            "(fp_arc (start 0 0) (end 2 0) (angle 90) (layer \"F.SilkS\") (width 0.15))",
        )
        .unwrap();
        let flattened = flatten_arc(&doc.nodes[0]).unwrap();
        let midpoint = flattened[flattened.len() / 2];
        assert!(midpoint.y < -1.4, "{midpoint:?}");
    }

    #[test]
    fn nested_modern_stroke_width_expands_fallback_bounds() {
        let p = project(
            "(fp_line (start 0 0) (end 4 0) (stroke (width 2) (type solid)) (layer \"F.SilkS\"))",
        )
        .unwrap();
        assert!(p.conservative_bounds.iter().any(|point| point.y >= 1.0));
        assert!(p.conservative_bounds.iter().any(|point| point.y <= -1.0));
    }

    #[test]
    fn graphics_supply_fallback_bounds_without_pads() {
        let p = project("(fp_rect (start -4 -2) (end 4 2) (layer \"F.SilkS\"))").unwrap();
        assert!(p.courtyard.is_empty());
        assert_eq!(p.conservative_bounds.len(), 4);
        assert!(p.conservative_bounds.iter().any(|point| point.x <= -4.5));
    }

    #[test]
    fn custom_primitive_extents_expand_rotated_pad_bounds() {
        let p=project("(pad \"A\" smd custom (at 0 0 45) (size 1 1) (layers \"F.Cu\") (options (clearance outline) (anchor rect)) (primitives (gr_line (start -3 0) (end 3 0) (width 0.2))))").unwrap();
        assert!(p.pads[0].size.x >= 6.0);
        assert!(p.conservative_bounds.iter().any(|point| point.x < -3.0));
        let arc=project("(pad \"A\" smd custom (at 0 0 90) (size 0.2 0.2) (layers \"F.Cu\") (primitives (gr_arc (start 1 0) (mid 0 2) (end -1 0) (width 0.2))))").unwrap();
        assert!(arc.pads[0].size.y >= 4.0);
    }

    #[test]
    fn large_courtyard_circles_stay_within_point_one_millimeter_of_the_curve() {
        let p = project("(fp_circle (center 0 0) (end 100 0) (layer \"F.CrtYd\"))").unwrap();
        let minimum_radius = p
            .courtyard
            .iter()
            .zip(p.courtyard.iter().cycle().skip(1))
            .map(|(a, b)| {
                let midpoint = Vec2 {
                    x: (a.x + b.x) / 2.0,
                    y: (a.y + b.y) / 2.0,
                };
                midpoint.x.hypot(midpoint.y)
            })
            .fold(f64::INFINITY, f64::min);
        assert!(
            minimum_radius >= 99.99,
            "minimum radius was {minimum_radius}"
        );
    }

    #[test]
    fn multiple_courtyard_islands_form_a_conservative_valid_envelope() {
        let p = project("(fp_rect (start 0 0) (end 2 2) (layer \"F.CrtYd\")) (fp_rect (start 8 0) (end 10 2) (layer \"F.CrtYd\"))").unwrap();
        assert_eq!(p.courtyard.len(), 4);
        assert!(p.courtyard.iter().any(|point| point.x <= 0.0));
        assert!(p.courtyard.iter().any(|point| point.x >= 10.0));
        assert!(
            p.courtyard
                .iter()
                .all(|point| point.y == 0.0 || point.y == -2.0)
        );
    }

    #[test]
    fn custom_bounds_include_stroke_width_drill_offsets_and_unknown_primitive_notices() {
        let stroked = project("(pad \"A\" smd custom (at 0 0) (size 1 1) (layers \"F.Cu\") (primitives (gr_line (start -3 0) (end 3 0) (width 0.4))))").unwrap();
        assert!(stroked.pads[0].size.x >= 6.4);
        let offset = project("(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.8 (offset 2 0)) (layers \"*.Cu\"))").unwrap();
        assert!(offset.pads[0].size.x >= 4.8);
        let unknown = project("(pad \"A\" smd custom (at 0 0) (size 1 1) (layers \"F.Cu\") (primitives (gr_text \"unknown\" (at 5 0) (effects (font (size 1 1) (thickness 0.15))))))").unwrap();
        assert!(unknown.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("unrecognized custom pad primitive")
        }));
    }

    #[test]
    fn courtyard_coordinates_follow_root_side_even_when_layer_is_back() {
        let front = project("(layer \"F.Cu\") (fp_rect (start 1 2) (end 5 8) (layer \"B.CrtYd\"))")
            .unwrap();
        assert!(front.courtyard.is_empty());
        let back = project("(layer \"B.Cu\") (fp_rect (start 1 2) (end 5 8) (layer \"B.CrtYd\"))")
            .unwrap();
        assert!(
            back.courtyard
                .iter()
                .any(|point| point.x == 1.0 && point.y == 2.0)
        );
    }

    #[test]
    fn root_rotation_normalizes_angles_and_keeps_child_positions_local() {
        let p = project("(at 10 20 90) (pad \"A\" smd rect (at 2 3 120) (size 1 1) (layers \"F.Cu\")) (fp_rect (start 0 0) (end 2 2) (layer \"F.CrtYd\"))").unwrap();
        assert_eq!(p.pads[0].at, Vec2 { x: 2.0, y: -3.0 });
        assert_eq!(p.pads[0].rotation, Some(30.0));
        assert!(
            p.courtyard
                .iter()
                .all(|point| point.x.abs() < 1e-9 || (point.x - 2.0).abs() < 1e-9)
        );
    }
}
