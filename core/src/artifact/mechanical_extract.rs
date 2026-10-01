//! Exact mechanical source geometry extraction for KiCad footprints.
//!
//! This module deliberately does not use the footprint preview projection:
//! dimensions and curves remain in their source form, and no geometry is
//! assigned a mechanical purpose unless a caller explicitly maps it.

use super::sexpr;
use crate::model::{ArtifactError, ArtifactErrorCode, MechanicalDrillShape as DrillShape, Vec2};
pub use crate::model::{
    MechanicalGeometry, MechanicalGeometryKind as GeometryKind, MechanicalPrimitive,
    MechanicalPurpose, MechanicalPurposeMapping as PurposeMapping, MechanicalShape,
};
use kiutils_sexpr::Node;
use sha2::{Digest, Sha256};

const CONTOUR_ENDPOINT_TOLERANCE_MM: f64 = 0.01;

/// Explicit source mappings can select a stable source id, or every primitive
/// of one kind on a selected layer for a custom import.
///
/// Extract exact footprint-local graphics and drill dimensions. Drill holes
/// remain unclassified by default, including all NPTH holes.
pub fn extract(
    source: &str,
    mappings: &[PurposeMapping],
) -> Result<MechanicalGeometry, ArtifactError> {
    let doc = kiutils_sexpr::parse_one(source).map_err(|error| parse_error(error.to_string()))?;
    let root = doc
        .nodes
        .first()
        .ok_or_else(|| parse_error("Expected a KiCad footprint"))?;
    if !matches!(sexpr::head(root), Some("footprint" | "module")) {
        return Err(parse_error("Expected a KiCad footprint"));
    }

    for mapping in mappings {
        let by_id =
            mapping.source_id.is_some() && mapping.kind.is_none() && mapping.layer.is_none();
        let by_layer_and_kind =
            mapping.source_id.is_none() && mapping.layer.is_some() && mapping.kind.is_some();
        if !by_id && !by_layer_and_kind {
            return Err(validation(
                "A mechanical purpose mapping needs either a source id or both kind and layer",
            ));
        }
        if mapping.source_id.as_deref() == Some("") {
            return Err(validation("Mechanical source ids must not be empty"));
        }
    }

    let source_name = sexpr::items(root)
        .and_then(|items| items.get(1))
        .and_then(|node| sexpr::kicad_quoted(source, sexpr::span(node)))
        .or_else(|| {
            sexpr::items(root)
                .and_then(|items| items.get(1))
                .and_then(sexpr::atom)
                .map(str::to_owned)
        });

    let mut primitives = Vec::new();
    let mut geometry_index = 0usize;
    for node in sexpr::items(root).into_iter().flatten().skip(1) {
        let Some(kind) = graphic_kind(sexpr::head(node)) else {
            continue;
        };
        let id = format!("geometry-{geometry_index}");
        geometry_index += 1;
        let layer = layer(node);
        let geometry = parse_graphic(node, kind)?;
        let purpose = resolve_purpose(&id, kind, layer.as_deref(), mappings)?;
        primitives.push(MechanicalPrimitive {
            id: id.clone(),
            source_group_id: id,
            kind,
            layers: layer.clone().into_iter().collect(),
            layer,
            purpose,
            geometry,
        });
    }

    for (pad_index, pad) in sexpr::children(root, "pad").enumerate() {
        let Some(drill_node) = sexpr::child(pad, "drill") else {
            continue;
        };
        let id = format!("pad-{pad_index}-drill");
        let group_id = format!("pad-{pad_index}");
        let kind = GeometryKind::Drill;
        let layer = layer(pad);
        let purpose = resolve_purpose(&id, kind, layer.as_deref(), mappings)?;
        primitives.push(MechanicalPrimitive {
            id,
            source_group_id: group_id,
            kind,
            layer,
            layers: declared_layers(pad),
            purpose,
            geometry: parse_drill(pad, drill_node)?,
        });
    }

    for mapping in mappings {
        let Some(source_id) = mapping.source_id.as_deref() else {
            continue;
        };
        let matched = primitives.iter().any(|primitive| source_id == primitive.id);
        if !matched {
            return Err(validation(
                "Mechanical purpose mapping matched no source geometry",
            ));
        }
    }

    Ok(MechanicalGeometry {
        source_name,
        primitives,
    })
}

/// Retain the exact imported text and mapping decisions with a stable content
/// hash so a profile can be audited against its source later.
pub fn profile_source(
    source: &str,
    mappings: &[PurposeMapping],
) -> Result<crate::model::MechanicalProfileSource, ArtifactError> {
    let geometry = extract(source, mappings)?;
    Ok(crate::model::MechanicalProfileSource {
        text: source.to_owned(),
        sha256: hex_digest(&Sha256::digest(source.as_bytes())),
        mappings: mappings.to_vec(),
        source_ids: geometry
            .primitives
            .iter()
            .filter(|primitive| primitive.purpose.is_some())
            .map(|primitive| primitive.id.clone())
            .collect(),
    })
}

/// Resolve explicitly selected electrical PCB mounting drills for storing in
/// a profile. NPTH and plated holes are both usable only after explicit mapping.
pub fn pcb_mounting_holes(
    geometry: &MechanicalGeometry,
) -> Result<Vec<crate::model::MechanicalPcbHole>, ArtifactError> {
    geometry
        .primitives
        .iter()
        .filter(|primitive| primitive.purpose == Some(MechanicalPurpose::ElectricalPcbMountingHole))
        .map(|primitive| {
            let MechanicalShape::Drill {
                at,
                size,
                offset,
                rotation_degrees,
                shape,
                ..
            } = &primitive.geometry
            else {
                return Err(validation(format!(
                    "Mapped PCB mounting feature {} is not a drill",
                    primitive.id
                )));
            };
            if *shape != DrillShape::Circle || (size.x - size.y).abs() > 1e-9 {
                return Err(validation(format!(
                    "Mapped PCB mounting drill {} must be circular",
                    primitive.id
                )));
            }
            let angle = rotation_degrees.to_radians();
            let (sin, cos) = angle.sin_cos();
            Ok(crate::model::MechanicalPcbHole {
                source_id: primitive.id.clone(),
                at: Vec2 {
                    x: at.x + offset.x * cos - offset.y * sin,
                    y: at.y + offset.x * sin + offset.y * cos,
                },
                diameter: size.x,
            })
        })
        .collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Convert only explicitly classified plate-cutout geometry to closed rings.
/// Source lines and polylines stay exact; arcs and circles are tessellated to
/// a maximum chord deviation. Open or ambiguous contours return a diagnostic
/// instead of being silently closed.
pub fn plate_cutout_contours(
    geometry: &MechanicalGeometry,
    max_deviation_mm: f64,
) -> Result<Vec<Vec<Vec2>>, ArtifactError> {
    purpose_contours(geometry, MechanicalPurpose::PlateCutout, max_deviation_mm)
}

/// Tessellate only closed geometry explicitly selected as one clearance
/// purpose. Unclassified lines and drawing guides never create envelopes.
pub fn clearance_envelopes(
    geometry: &MechanicalGeometry,
    max_deviation_mm: f64,
) -> Result<Vec<Vec<Vec2>>, ArtifactError> {
    purpose_contours(
        geometry,
        MechanicalPurpose::ClearanceEnvelope,
        max_deviation_mm,
    )
}

fn purpose_contours(
    geometry: &MechanicalGeometry,
    purpose: MechanicalPurpose,
    max_deviation_mm: f64,
) -> Result<Vec<Vec<Vec2>>, ArtifactError> {
    if !max_deviation_mm.is_finite() || max_deviation_mm <= 0.0 {
        return Err(validation(
            "Tessellation deviation must be positive and finite",
        ));
    }
    let mut contours = Vec::new();
    let mut edges = Vec::new();
    for primitive in geometry
        .primitives
        .iter()
        .filter(|primitive| primitive.purpose == Some(purpose))
    {
        match &primitive.geometry {
            MechanicalShape::Line { start, end, .. } => edges.push(vec![*start, *end]),
            MechanicalShape::Arc {
                start,
                mid,
                end,
                angle_degrees,
                ..
            } => {
                edges.push(tessellate_arc(
                    *start,
                    *mid,
                    *end,
                    *angle_degrees,
                    max_deviation_mm,
                )?);
            }
            MechanicalShape::Circle { center, end, .. } => {
                contours.push(tessellate_circle(*center, *end, max_deviation_mm)?);
            }
            MechanicalShape::Rectangle { start, end, .. } => contours.push(vec![
                *start,
                Vec2 {
                    x: end.x,
                    y: start.y,
                },
                *end,
                Vec2 {
                    x: start.x,
                    y: end.y,
                },
            ]),
            MechanicalShape::Polygon { points, .. } => contours.push(points.clone()),
            MechanicalShape::Drill {
                at,
                size,
                offset,
                rotation_degrees,
                shape,
                ..
            } => contours.push(tessellate_drill(
                *at,
                *size,
                *offset,
                *rotation_degrees,
                shape,
                max_deviation_mm,
            )?),
        }
    }

    while !edges.is_empty() {
        let mut chain = edges.remove(0);
        while !near(*chain.last().expect("edge has endpoints"), chain[0]) {
            let current = *chain.last().expect("edge has endpoints");
            let Some(index) = edges.iter().position(|edge| {
                near(edge[0], current) || near(*edge.last().expect("edge has endpoints"), current)
            }) else {
                return Err(validation("Plate cutout geometry contains an open contour"));
            };
            let mut edge = edges.remove(index);
            if near(*edge.last().expect("edge has endpoints"), current) {
                edge.reverse();
            }
            chain.extend(edge.into_iter().skip(1));
        }
        chain.pop();
        if chain.len() < 3 {
            return Err(validation(
                "Plate cutout contour needs at least three points",
            ));
        }
        contours.push(chain);
    }
    Ok(contours)
}

fn tessellate_circle(center: Vec2, edge: Vec2, deviation: f64) -> Result<Vec<Vec2>, ArtifactError> {
    let radius = (edge.x - center.x).hypot(edge.y - center.y);
    if radius <= 0.0 {
        return Err(validation("Degenerate plate-cutout circle"));
    }
    let max_step = 2.0
        * (1.0 - deviation.min(radius) / radius)
            .clamp(-1.0, 1.0)
            .acos();
    let count = (std::f64::consts::TAU / max_step.max(0.001))
        .ceil()
        .max(12.0) as usize;
    Ok((0..count)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / count as f64;
            Vec2 {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            }
        })
        .collect())
}

fn tessellate_drill(
    at: Vec2,
    size: Vec2,
    offset: Vec2,
    rotation_degrees: f64,
    shape: &DrillShape,
    deviation: f64,
) -> Result<Vec<Vec2>, ArtifactError> {
    let angle = rotation_degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let center = Vec2 {
        x: at.x + offset.x * cos - offset.y * sin,
        y: at.y + offset.x * sin + offset.y * cos,
    };
    if *shape == DrillShape::Circle || (size.x - size.y).abs() <= 1e-9 {
        return tessellate_circle(
            center,
            Vec2 {
                x: center.x + size.x / 2.0,
                y: center.y,
            },
            deviation,
        );
    }
    let (major, minor, horizontal) = if size.x > size.y {
        (size.x, size.y, true)
    } else {
        (size.y, size.x, false)
    };
    let radius = minor / 2.0;
    let half_center = (major - minor) / 2.0;
    let max_step = 2.0
        * (1.0 - deviation.min(radius) / radius)
            .clamp(-1.0, 1.0)
            .acos();
    let segments = (std::f64::consts::PI / max_step.max(0.001)).ceil().max(4.0) as usize;
    let mut points = Vec::with_capacity(2 * (segments + 1));
    for index in 0..=segments {
        let theta =
            -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * index as f64 / segments as f64;
        let mut local = Vec2 {
            x: half_center + radius * theta.cos(),
            y: radius * theta.sin(),
        };
        if !horizontal {
            local = Vec2 {
                x: -local.y,
                y: local.x,
            };
        }
        points.push(Vec2 {
            x: center.x + local.x * cos - local.y * sin,
            y: center.y + local.x * sin + local.y * cos,
        });
    }
    for index in 0..=segments {
        let theta =
            std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * index as f64 / segments as f64;
        let mut local = Vec2 {
            x: -half_center + radius * theta.cos(),
            y: radius * theta.sin(),
        };
        if !horizontal {
            local = Vec2 {
                x: -local.y,
                y: local.x,
            };
        }
        points.push(Vec2 {
            x: center.x + local.x * cos - local.y * sin,
            y: center.y + local.x * sin + local.y * cos,
        });
    }
    Ok(points)
}

fn tessellate_arc(
    start: Vec2,
    mid: Option<Vec2>,
    end: Vec2,
    legacy_sweep_degrees: Option<f64>,
    deviation: f64,
) -> Result<Vec<Vec2>, ArtifactError> {
    let (center, first, sweep, radius) = if let (Some(mid), None) = (mid, legacy_sweep_degrees) {
        let (center, radius) = circumcenter(start, mid, end)
            .ok_or_else(|| validation("Degenerate plate-cutout arc"))?;
        let angle = |point: Vec2| (point.y - center.y).atan2(point.x - center.x);
        let a0 = angle(start);
        let am = angle(mid);
        let a1 = angle(end);
        let ccw = (am - a0).rem_euclid(std::f64::consts::TAU)
            <= (a1 - a0).rem_euclid(std::f64::consts::TAU);
        let sweep = if ccw {
            (a1 - a0).rem_euclid(std::f64::consts::TAU)
        } else {
            -((a0 - a1).rem_euclid(std::f64::consts::TAU))
        };
        (center, a0, sweep, radius)
    } else if let (None, Some(degrees)) = (mid, legacy_sweep_degrees) {
        // In legacy KiCad syntax `start` is the center and `end` lies on it.
        let radius = (end.x - start.x).hypot(end.y - start.y);
        if radius <= 0.0 {
            return Err(validation("Degenerate legacy plate-cutout arc"));
        }
        let first = (end.y - start.y).atan2(end.x - start.x);
        (start, first, -degrees.to_radians(), radius)
    } else {
        return Err(validation(
            "Arc must have either a midpoint or legacy sweep",
        ));
    };
    let max_step = 2.0
        * (1.0 - deviation.min(radius) / radius)
            .clamp(-1.0, 1.0)
            .acos();
    let count = (sweep.abs() / max_step.max(0.001)).ceil().max(2.0) as usize;
    Ok((0..=count)
        .map(|index| {
            let angle = first + sweep * index as f64 / count as f64;
            Vec2 {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            }
        })
        .collect())
}

fn circumcenter(a: Vec2, b: Vec2, c: Vec2) -> Option<(Vec2, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let aa = a.x * a.x + a.y * a.y;
    let bb = b.x * b.x + b.y * b.y;
    let cc = c.x * c.x + c.y * c.y;
    let center = Vec2 {
        x: (aa * (b.y - c.y) + bb * (c.y - a.y) + cc * (a.y - b.y)) / d,
        y: (aa * (c.x - b.x) + bb * (a.x - c.x) + cc * (b.x - a.x)) / d,
    };
    Some((center, (a.x - center.x).hypot(a.y - center.y)))
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= CONTOUR_ENDPOINT_TOLERANCE_MM
}

fn resolve_purpose(
    id: &str,
    kind: GeometryKind,
    layer: Option<&str>,
    mappings: &[PurposeMapping],
) -> Result<Option<MechanicalPurpose>, ArtifactError> {
    let matches = mappings
        .iter()
        .filter(|mapping| {
            mapping.source_id.as_deref() == Some(id)
                || (mapping.source_id.is_none()
                    && mapping.kind == Some(kind)
                    && mapping.layer.as_deref() == layer)
        })
        .map(|mapping| mapping.purpose)
        .collect::<Vec<_>>();
    if matches.len() > 1 && matches.iter().any(|purpose| *purpose != matches[0]) {
        return Err(validation(format!("Conflicting purpose mappings for {id}")));
    }
    Ok(matches.first().copied())
}

fn graphic_kind(head: Option<&str>) -> Option<GeometryKind> {
    match head {
        Some("fp_line") => Some(GeometryKind::Line),
        Some("fp_arc") => Some(GeometryKind::Arc),
        Some("fp_circle") => Some(GeometryKind::Circle),
        Some("fp_rect") => Some(GeometryKind::Rectangle),
        Some("fp_poly") => Some(GeometryKind::Polygon),
        _ => None,
    }
}

fn parse_graphic(node: &Node, kind: GeometryKind) -> Result<MechanicalShape, ArtifactError> {
    let width = graphic_width(node).map(parse_finite).transpose()?;
    match kind {
        GeometryKind::Line => Ok(MechanicalShape::Line {
            start: point(required_child(node, "start", "Line needs start")?)?,
            end: point(required_child(node, "end", "Line needs end")?)?,
            width,
        }),
        GeometryKind::Arc => {
            let start = point(required_child(node, "start", "Arc needs start")?)?;
            let end = point(required_child(node, "end", "Arc needs end")?)?;
            let angle_degrees = sexpr::child(node, "angle")
                .and_then(scalar)
                .map(parse_finite)
                .transpose()?;
            let mid = sexpr::child(node, "mid").map(point).transpose()?;
            if angle_degrees.is_some() == mid.is_some() {
                return Err(validation("Arc needs either a midpoint or a legacy angle"));
            }
            Ok(MechanicalShape::Arc {
                start,
                mid,
                end,
                angle_degrees,
                width,
            })
        }
        GeometryKind::Circle => Ok(MechanicalShape::Circle {
            center: point(required_child(node, "center", "Circle needs center")?)?,
            end: point(required_child(node, "end", "Circle needs end")?)?,
            width,
        }),
        GeometryKind::Rectangle => Ok(MechanicalShape::Rectangle {
            start: point(required_child(node, "start", "Rectangle needs start")?)?,
            end: point(required_child(node, "end", "Rectangle needs end")?)?,
            width,
        }),
        GeometryKind::Polygon => {
            let points_node = required_child(node, "pts", "Polygon needs points")?;
            let points = sexpr::children(points_node, "xy")
                .map(point)
                .collect::<Result<Vec<_>, _>>()?;
            if points.len() < 3 {
                return Err(validation("Polygon needs at least three points"));
            }
            Ok(MechanicalShape::Polygon { points, width })
        }
        GeometryKind::Drill => unreachable!("graphics cannot be drills"),
    }
}

fn parse_drill(pad: &Node, drill: &Node) -> Result<MechanicalShape, ArtifactError> {
    let values = sexpr::items(drill).ok_or_else(|| validation("Invalid drill"))?;
    let oval = values.get(1).and_then(sexpr::atom) == Some("oval");
    let number_start = if oval { 2 } else { 1 };
    let x = values
        .get(number_start)
        .and_then(sexpr::atom)
        .ok_or_else(|| validation("Drill needs a diameter"))
        .and_then(parse_finite)?;
    let y = if oval {
        values
            .get(number_start + 1)
            .and_then(sexpr::atom)
            .ok_or_else(|| validation("Oval drill needs two dimensions"))
            .and_then(parse_finite)?
    } else {
        x
    };
    if x <= 0.0 || y <= 0.0 {
        return Err(validation("Drill dimensions must be positive"));
    }
    let at = sexpr::child(pad, "at")
        .map(point)
        .transpose()?
        .unwrap_or_default();
    let offset = sexpr::child(drill, "offset")
        .map(point)
        .transpose()?
        .unwrap_or_default();
    let rotation_degrees = sexpr::child(pad, "at")
        .and_then(|at| sexpr::items(at))
        .and_then(|items| items.get(3))
        .and_then(sexpr::atom)
        .map(parse_finite)
        .transpose()?
        .unwrap_or(0.0);
    let plated = sexpr::items(pad)
        .and_then(|items| items.get(2))
        .and_then(sexpr::atom)
        .and_then(|kind| match kind {
            "thru_hole" => Some(true),
            "np_thru_hole" => Some(false),
            _ => None,
        });
    Ok(MechanicalShape::Drill {
        at,
        size: Vec2 { x, y },
        offset,
        rotation_degrees,
        shape: if oval {
            DrillShape::Oval
        } else {
            DrillShape::Circle
        },
        plated,
    })
}

fn layer(node: &Node) -> Option<String> {
    sexpr::child(node, "layer")
        .and_then(|value| sexpr::items(value))
        .and_then(|items| items.get(1))
        .and_then(sexpr::atom)
        .map(str::to_owned)
}

fn declared_layers(node: &Node) -> Vec<String> {
    sexpr::child(node, "layers")
        .and_then(sexpr::items)
        .into_iter()
        .flatten()
        .skip(1)
        .filter_map(sexpr::atom)
        .map(str::to_owned)
        .collect()
}

fn graphic_width(node: &Node) -> Option<&str> {
    sexpr::child(node, "stroke")
        .and_then(|stroke| sexpr::child(stroke, "width"))
        .or_else(|| sexpr::child(node, "width"))
        .and_then(scalar)
}

fn scalar(node: &Node) -> Option<&str> {
    sexpr::items(node)
        .and_then(|items| items.get(1))
        .and_then(sexpr::atom)
}

fn point(node: &Node) -> Result<Vec2, ArtifactError> {
    let values = sexpr::items(node).ok_or_else(|| validation("Expected a coordinate pair"))?;
    let x = values
        .get(1)
        .and_then(sexpr::atom)
        .ok_or_else(|| validation("Coordinate needs x"))
        .and_then(parse_finite)?;
    let y = values
        .get(2)
        .and_then(sexpr::atom)
        .ok_or_else(|| validation("Coordinate needs y"))
        .and_then(parse_finite)?;
    Ok(Vec2 { x, y: -y })
}

fn required_child<'a>(
    node: &'a Node,
    name: &str,
    message: &str,
) -> Result<&'a Node, ArtifactError> {
    sexpr::child(node, name).ok_or_else(|| validation(message))
}

fn parse_finite(value: &str) -> Result<f64, ArtifactError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| validation(format!("Invalid mechanical dimension: {value}")))?;
    if !parsed.is_finite() {
        return Err(validation("Mechanical dimensions must be finite"));
    }
    Ok(parsed)
}

fn parse_error(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::ParseError, message)
}

fn validation(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::Validation, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MX_FIXTURE: &str = r#"(footprint "SW_MX_1u" (version 20240108) (generator "pcbnew")
      (fp_line (start -7.62 0) (end 7.62 0) (stroke (width 0.05) (type default)) (layer "User.Eco2"))
      (fp_arc (start 0 0) (mid 1 1) (end 2 0) (stroke (width 0.1) (type default)) (layer "User.Drawings"))
      (fp_circle (center 0 0) (end 3.1 0) (stroke (width 0.2) (type default)) (layer "F.Fab"))
      (fp_rect (start -8 -8) (end 8 8) (stroke (width 0) (type default)) (layer "User.Eco2"))
      (fp_poly (pts (xy 0 0) (xy 4 0) (xy 4 2) (xy 0 2)) (stroke (width 0.05) (type default)) (layer "User.Eco2"))
      (pad "1" thru_hole circle (at 1 2 30) (size 2 2) (drill 1.6 (offset 0.2 0.1)) (layers "*.Cu" "*.Mask"))
      (pad "MP" np_thru_hole circle (at -2 -3) (size 1.8 1.8) (drill 1.8) (layers "*.Cu" "*.Mask")))"#;

    #[test]
    fn preserves_library_graphics_and_drills_without_preview_approximations() {
        let geometry = extract(MX_FIXTURE, &[]).unwrap();
        assert_eq!(geometry.source_name.as_deref(), Some("SW_MX_1u"));
        assert_eq!(geometry.primitives.len(), 7);
        assert_eq!(geometry.primitives[0].id, "geometry-0");
        assert_eq!(geometry.primitives[0].layer.as_deref(), Some("User.Eco2"));
        assert_eq!(geometry.primitives[0].layers, ["User.Eco2"]);
        assert_eq!(geometry.primitives[0].purpose, None);
        assert!(matches!(
            geometry.primitives[0].geometry,
            MechanicalShape::Line {
                start: Vec2 { x: -7.62, y: 0.0 },
                end: Vec2 { x: 7.62, y: 0.0 },
                width: Some(0.05)
            }
        ));
        assert!(matches!(
            geometry.primitives[1].geometry,
            MechanicalShape::Arc {
                start: Vec2 { x: 0.0, y: 0.0 },
                mid: Some(Vec2 { x: 1.0, y: -1.0 }),
                end: Vec2 { x: 2.0, y: 0.0 },
                width: Some(0.1),
                ..
            }
        ));
        assert!(matches!(
            geometry.primitives[2].geometry,
            MechanicalShape::Circle {
                center: Vec2 { x: 0.0, y: 0.0 },
                end: Vec2 { x: 3.1, y: 0.0 },
                width: Some(0.2)
            }
        ));
        assert!(matches!(
            geometry.primitives[5].geometry,
            MechanicalShape::Drill {
                at: Vec2 { x: 1.0, y: -2.0 },
                size: Vec2 { x: 1.6, y: 1.6 },
                offset: Vec2 { x: 0.2, y: -0.1 },
                plated: Some(true),
                ..
            }
        ));
        assert!(matches!(
            geometry.primitives[6].geometry,
            MechanicalShape::Drill {
                size: Vec2 { x: 1.8, y: 1.8 },
                plated: Some(false),
                ..
            }
        ));
        assert_eq!(geometry.primitives[6].purpose, None);
    }

    #[test]
    fn reviewed_marbastlib_switch_fixture_keeps_cut_layer_separate_from_npth_drills() {
        let source = include_str!("../../tests/fixtures/mechanical/SW_MX_1u.kicad_mod");
        let mapping = PurposeMapping {
            source_id: None,
            kind: Some(GeometryKind::Line),
            layer: Some("Eco2.User".into()),
            purpose: MechanicalPurpose::PlateCutout,
        };
        let geometry = extract(source, &[mapping]).unwrap();
        assert_eq!(geometry.source_name.as_deref(), Some("SW_MX_1u"));
        assert!(geometry.primitives.iter().any(|primitive| {
            primitive.layer.as_deref() == Some("Eco2.User")
                && primitive.purpose == Some(MechanicalPurpose::PlateCutout)
        }));
        let drills = geometry
            .primitives
            .iter()
            .filter(|primitive| primitive.kind == GeometryKind::Drill)
            .collect::<Vec<_>>();
        assert_eq!(drills.len(), 5);
        let npth = drills
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive.geometry,
                    MechanicalShape::Drill {
                        plated: Some(false),
                        ..
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(npth.len(), 3);
        assert!(npth.iter().all(|primitive| {
            primitive.purpose.is_none() && primitive.layers == ["*.Cu", "*.Mask"]
        }));
        assert!(drills.iter().any(|primitive| {
            matches!(
                primitive.geometry,
                MechanicalShape::Drill {
                    plated: Some(true),
                    ..
                }
            )
        }));
    }

    #[test]
    fn retains_source_hash_and_only_extracts_explicit_pcb_mounting_holes() {
        let source = include_str!("../../tests/fixtures/mechanical/SW_MX_1u.kicad_mod");
        let mappings = (0..3)
            .map(|index| PurposeMapping {
                source_id: Some(format!("pad-{index}-drill")),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::ElectricalPcbMountingHole,
            })
            .collect::<Vec<_>>();
        let geometry = extract(source, &mappings).unwrap();
        let provenance = profile_source(source, &mappings).unwrap();
        let holes = pcb_mounting_holes(&geometry).unwrap();
        assert_eq!(provenance.text, source);
        assert_eq!(
            provenance.sha256,
            hex_digest(&Sha256::digest(source.as_bytes()))
        );
        assert_eq!(provenance.sha256.len(), 64);
        assert_eq!(
            provenance.source_ids,
            ["pad-0-drill", "pad-1-drill", "pad-2-drill"]
        );
        assert_eq!(holes.len(), 3);
        assert_eq!(holes[0].diameter, 1.75);
        assert_eq!(holes[1].diameter, 3.9878);
        assert_eq!(holes[2].diameter, 1.75);
    }

    #[test]
    fn explicit_marbastlib_switch_cut_mapping_builds_one_bounded_contour() {
        let source = include_str!("../../tests/fixtures/mechanical/SW_MX_1u.kicad_mod");
        let geometry = extract(
            source,
            &[
                PurposeMapping {
                    source_id: None,
                    kind: Some(GeometryKind::Line),
                    layer: Some("Eco2.User".into()),
                    purpose: MechanicalPurpose::PlateCutout,
                },
                PurposeMapping {
                    source_id: None,
                    kind: Some(GeometryKind::Arc),
                    layer: Some("Eco2.User".into()),
                    purpose: MechanicalPurpose::PlateCutout,
                },
            ],
        )
        .unwrap();
        let contours = plate_cutout_contours(&geometry, 0.01).unwrap();
        assert_eq!(contours.len(), 1);
        assert!(contours[0].len() > 8);
        assert!(contours[0].iter().any(|point| point.x < -6.99));
        assert!(contours[0].iter().any(|point| point.x > 6.99));
    }

    #[test]
    fn artifact_request_returns_selectable_source_geometry_and_mapped_cutouts() {
        let source = include_str!("../../tests/fixtures/mechanical/SW_MX_1u.kicad_mod");
        let request = crate::model::ArtifactRequest::ExtractMechanical {
            id: "extract-test".into(),
            source: source.into(),
            mappings: vec![
                PurposeMapping {
                    source_id: None,
                    kind: Some(GeometryKind::Line),
                    layer: Some("Eco2.User".into()),
                    purpose: MechanicalPurpose::PlateCutout,
                },
                PurposeMapping {
                    source_id: None,
                    kind: Some(GeometryKind::Arc),
                    layer: Some("Eco2.User".into()),
                    purpose: MechanicalPurpose::PlateCutout,
                },
            ],
            max_deviation_mm: 0.01,
        };
        let json = serde_json::to_string(&request).unwrap();
        let response: crate::model::ArtifactReply =
            serde_json::from_str(&super::super::request(&json)).unwrap();
        let crate::model::ArtifactReply::ExtractMechanical { id, result } = response else {
            panic!("expected mechanical extraction reply");
        };
        assert_eq!(id, "extract-test");
        assert_eq!(result.geometry.source_name.as_deref(), Some("SW_MX_1u"));
        assert!(!result.geometry.primitives.is_empty());
        assert_eq!(result.plate_cutouts.len(), 1);
        assert!(result.clearance_envelopes.is_empty());
        assert_eq!(result.source_geometry.text, source);
        assert_eq!(result.source_geometry.source_ids.len(), 8);
        assert!(result.pcb_holes.is_empty());
    }

    #[test]
    fn maps_purpose_only_when_explicitly_selected_by_id_or_layer_and_kind() {
        let by_id = PurposeMapping {
            source_id: Some("geometry-0".into()),
            kind: None,
            layer: None,
            purpose: MechanicalPurpose::PlateCutout,
        };
        let mapped = extract(MX_FIXTURE, &[by_id]).unwrap();
        assert_eq!(
            mapped.primitives[0].purpose,
            Some(MechanicalPurpose::PlateCutout)
        );
        assert_eq!(mapped.primitives[3].purpose, None);

        let by_layer = PurposeMapping {
            source_id: None,
            kind: Some(GeometryKind::Line),
            layer: Some("User.Eco2".into()),
            purpose: MechanicalPurpose::DrawingGuide,
        };
        let mapped = extract(MX_FIXTURE, &[by_layer]).unwrap();
        assert_eq!(
            mapped.primitives[0].purpose,
            Some(MechanicalPurpose::DrawingGuide)
        );
        assert_eq!(mapped.primitives[5].purpose, None);
    }

    #[test]
    fn drawing_guides_do_not_become_clearance_envelopes() {
        let mappings = [
            PurposeMapping {
                source_id: Some("geometry-1".into()),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::DrawingGuide,
            },
            PurposeMapping {
                source_id: Some("geometry-2".into()),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::ClearanceEnvelope,
            },
        ];
        let geometry = extract(MX_FIXTURE, &mappings).unwrap();
        assert!(plate_cutout_contours(&geometry, 0.01).unwrap().is_empty());
        let envelopes = clearance_envelopes(&geometry, 0.01).unwrap();
        assert_eq!(envelopes.len(), 1);
        assert!(envelopes[0].len() >= 12);
    }

    #[test]
    fn preserves_legacy_arc_sweep_and_exact_oval_drill_size() {
        let source = r#"(module "legacy" (fp_arc (start 1 2) (end 4 2) (angle 45) (layer "F.SilkS") (width 0.15)) (pad "1" thru_hole oval (at 0 0 30) (size 2 1) (drill oval 0.7 1.1) (layers "*.Cu")))"#;
        let geometry = extract(source, &[]).unwrap();
        assert!(matches!(
            geometry.primitives[0].geometry,
            MechanicalShape::Arc {
                start: Vec2 { x: 1.0, y: -2.0 },
                end: Vec2 { x: 4.0, y: -2.0 },
                angle_degrees: Some(45.0),
                mid: None,
                width: Some(0.15)
            }
        ));
        assert!(matches!(
            geometry.primitives[1].geometry,
            MechanicalShape::Drill {
                size: Vec2 { x: 0.7, y: 1.1 },
                rotation_degrees: 30.0,
                shape: DrillShape::Oval,
                ..
            }
        ));
        let slot = extract(
            source,
            &[PurposeMapping {
                source_id: Some("pad-0-drill".into()),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::PlateCutout,
            }],
        )
        .unwrap();
        let contour = plate_cutout_contours(&slot, 0.01).unwrap().remove(0);
        let x_extent = contour
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            - contour.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let y_extent = contour
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max)
            - contour.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert!((x_extent - 0.9).abs() < 0.03, "x extent {x_extent}");
        assert!((y_extent - 1.0464).abs() < 0.03, "y extent {y_extent}");
    }

    #[test]
    fn rejects_ambiguous_or_conflicting_mapping_and_malformed_dimensions() {
        let ambiguous = PurposeMapping {
            source_id: Some("geometry-0".into()),
            kind: Some(GeometryKind::Line),
            layer: Some("User.Eco2".into()),
            purpose: MechanicalPurpose::PlateCutout,
        };
        assert!(extract(MX_FIXTURE, &[ambiguous]).is_err());
        let conflict = [
            PurposeMapping {
                source_id: Some("geometry-0".into()),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::PlateCutout,
            },
            PurposeMapping {
                source_id: Some("geometry-0".into()),
                kind: None,
                layer: None,
                purpose: MechanicalPurpose::ClearanceEnvelope,
            },
        ];
        assert!(extract(MX_FIXTURE, &conflict).is_err());
        assert!(
            extract(
                "(footprint \"bad\" (fp_circle (center 0 0) (end NaN 0) (layer \"F.Fab\")))",
                &[]
            )
            .is_err()
        );
    }
}
