//! Foam layer geometry: fallback contours, clearances and exclusion subtraction.

use super::{FloatPath, FloatShapes, convex_envelope, expanded_outline, transform_part_points};
use crate::geometry::{self, BooleanOverlayRule};
use crate::model::*;

pub(super) fn fallback_foam_geometry(
    part: &Part,
    definition: &PartDefinition,
) -> Option<(Vec<Vec2>, &'static str)> {
    if definition.courtyard.len() >= 3 {
        return Some((definition.courtyard.clone(), "footprint courtyard"));
    }

    let mut pad_corners = Vec::new();
    for pad in &definition.pads {
        if !pad.at.x.is_finite()
            || !pad.at.y.is_finite()
            || !pad.size.x.is_finite()
            || !pad.size.y.is_finite()
            || pad.size.x <= 0.0
            || pad.size.y <= 0.0
        {
            continue;
        }
        let (sin, cos) = pad.rotation.unwrap_or(0.0).to_radians().sin_cos();
        for (x, y) in [
            (-pad.size.x / 2.0, -pad.size.y / 2.0),
            (pad.size.x / 2.0, -pad.size.y / 2.0),
            (pad.size.x / 2.0, pad.size.y / 2.0),
            (-pad.size.x / 2.0, pad.size.y / 2.0),
        ] {
            pad_corners.push(Vec2 {
                x: pad.at.x + x * cos - y * sin,
                y: pad.at.y + x * sin + y * cos,
            });
        }
    }
    if pad_corners.len() >= 3 {
        return Some((convex_envelope(pad_corners), "conservative pad bounds"));
    }

    let keycap = part.keycap.or(definition.keycap)?;
    if !keycap.x.is_finite() || !keycap.y.is_finite() || keycap.x <= 0.0 || keycap.y <= 0.0 {
        return None;
    }
    Some((
        vec![
            Vec2 {
                x: -keycap.x / 2.0,
                y: -keycap.y / 2.0,
            },
            Vec2 {
                x: keycap.x / 2.0,
                y: -keycap.y / 2.0,
            },
            Vec2 {
                x: keycap.x / 2.0,
                y: keycap.y / 2.0,
            },
            Vec2 {
                x: -keycap.x / 2.0,
                y: keycap.y / 2.0,
            },
        ],
        "keycap bounds",
    ))
}

pub(super) fn expanded_clearance(
    points: &[Vec2],
    amount: f64,
    revision: u64,
) -> Result<Vec<Vec2>, String> {
    if points.len() < 3
        || points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err("Foam clearance needs a finite closed polygon".into());
    }
    if amount == 0.0 {
        return Ok(points.to_vec());
    }
    let expanded = expanded_outline(
        &[Contour {
            hole: false,
            points: points.to_vec(),
        }],
        amount,
        revision,
    )?;
    expanded
        .into_iter()
        .find(|contour| !contour.hole)
        .map(|contour| contour.points)
        .ok_or_else(|| "Foam clearance collapsed while applying its margin".into())
}

pub(super) fn add_foam_clearance(
    target: &mut Vec<Contour>,
    part: &Part,
    points: &[Vec2],
    margin: f64,
    revision: u64,
) -> Result<(), String> {
    let expanded = expanded_clearance(points, margin, revision)?;
    target.push(Contour {
        hole: true,
        points: transform_part_points(part, &expanded),
    });
    Ok(())
}

pub(super) fn subtract_foam_exclusions(
    base: &[Contour],
    exclusions: &[Contour],
) -> Result<Vec<Contour>, String> {
    let mut solids: FloatShapes = Vec::new();
    let mut cuts: FloatShapes = Vec::new();
    let path = |contour: &Contour| -> Result<FloatPath, String> {
        if contour.points.len() < 3
            || contour.points.iter().any(|point| {
                !point.x.is_finite()
                    || !point.y.is_finite()
                    || point.x.abs() > boardstudio_contracts::MAX_COORDINATE_MM
                    || point.y.abs() > boardstudio_contracts::MAX_COORDINATE_MM
            })
        {
            return Err("Foam contours must contain finite, in-range polygons".into());
        }
        Ok(contour
            .points
            .iter()
            .map(|point| [point.x, point.y])
            .collect())
    };
    let add_union = |shapes: &mut FloatShapes, polygon: FloatPath| {
        let next = vec![vec![polygon]];
        if shapes.is_empty() {
            *shapes = next;
        } else {
            *shapes = geometry::overlay_shapes(shapes, &next, BooleanOverlayRule::Union);
        }
    };

    for contour in base {
        if !contour.hole {
            add_union(&mut solids, path(contour)?);
        }
    }
    if solids.is_empty() {
        return Err("Foam geometry requires at least one outer contour".into());
    }
    for contour in base.iter().filter(|contour| contour.hole).chain(exclusions) {
        add_union(&mut cuts, path(contour)?);
    }
    if cuts.is_empty() {
        return Ok(base.to_vec());
    }
    let output = geometry::overlay_shapes(&solids, &cuts, BooleanOverlayRule::Difference);
    if output.is_empty() {
        return Err("Foam clearances remove the complete sheet".into());
    }
    Ok(output
        .into_iter()
        .flat_map(|shape| {
            shape
                .into_iter()
                .enumerate()
                .map(|(index, points)| Contour {
                    hole: index > 0,
                    points: points
                        .into_iter()
                        .map(|point| Vec2 {
                            x: point[0],
                            y: point[1],
                        })
                        .collect(),
                })
        })
        .collect())
}
