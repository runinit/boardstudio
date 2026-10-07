//! Pads and the courtyard envelope measured from generated forms.
use serde::Serialize;

use crate::error::{GeneratorError, Result};
use crate::number::{cos, hypot, js_to_number, radians, sin};
use crate::sexpr::{Expr, child, children, scalar_error};
use crate::types::{Pad, PadShape, Side, Vec2};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Geometry {
    pub pads: Vec<Pad>,
    pub courtyard: Vec<Vec2>,
    #[serde(rename = "courtyardFallback")]
    pub courtyard_fallback: bool,
}

fn number(node: Option<&Expr>) -> Result<f64> {
    let value = match node {
        Some(Expr::Atom(text)) => js_to_number(text),
        _ => f64::NAN,
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(GeneratorError::Geometry(
            "Invalid numeric Ergogen geometry".into(),
        ))
    }
}

fn value(node: Option<&Expr>) -> Result<&str> {
    node.ok_or_else(scalar_error)?.value()
}

fn is_atom(node: Option<&Expr>, text: &str) -> bool {
    matches!(node, Some(Expr::Atom(atom)) if atom == text)
}

/// A point in KiCad's downward Y, flipped to the document's upward Y.
fn point(node: &[Expr]) -> Result<Vec2> {
    Ok(Vec2 {
        x: number(node.get(1))?,
        y: -number(node.get(2))?,
    })
}

fn is_footprint(form: &Expr) -> Option<&[Expr]> {
    let items = form.as_list()?;
    matches!(items.first(), Some(Expr::Atom(head)) if head == "footprint" || head == "module")
        .then_some(items)
}

const GRAPHICS: [&str; 5] = ["fp_line", "fp_rect", "fp_poly", "fp_circle", "fp_arc"];

fn layer_matches(layer: &str) -> bool {
    let Some((side, rest)) = layer.split_once('.') else {
        return false;
    };
    matches!(side, "F" | "B") && matches!(rest, "CrtYd" | "Fab" | "SilkS")
}

/// Measure pads and the outline envelope. Courtyard graphics win when present;
/// otherwise physical fab/silkscreen graphics and rotated pad extents bound it.
pub fn geometry(forms: &[Expr]) -> Result<Geometry> {
    let mut pads: Vec<Pad> = Vec::new();
    let mut courtyard_points: Vec<Vec2> = Vec::new();
    let mut body_points: Vec<Vec2> = Vec::new();
    let mut courtyard_segments: Vec<(Vec2, Vec2)> = Vec::new();
    let mut courtyard_polygons: Vec<Vec<Vec2>> = Vec::new();
    for footprint in forms.iter().filter_map(is_footprint) {
        for item in children(footprint, "pad") {
            let (Some(at), Some(size)) = (child(item, "at"), child(item, "size")) else {
                return Err(GeneratorError::Geometry(
                    "Ergogen pad is missing position or size".into(),
                ));
            };
            let drill = child(item, "drill");
            let layers = child(item, "layers");
            let rotation = match at.get(3) {
                None => 0.0,
                Some(_) => -number(at.get(3))?,
            };
            let angle = radians(rotation);
            let hx = number(size.get(1))? / 2.0;
            let hy = number(size.get(2))? / 2.0;
            for (x, y) in [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
                body_points.push(Vec2 {
                    x: number(at.get(1))? + x * cos(angle) - y * sin(angle),
                    y: -number(at.get(2))? - x * sin(angle) - y * cos(angle),
                });
            }
            let pad_number = value(item.get(1))?.to_owned();
            let at_point = point(at)?;
            let size_value = Vec2 {
                x: number(size.get(1))?,
                y: number(size.get(2))?,
            };
            let shape = match item.get(3) {
                Some(Expr::Atom(text)) => match text.as_str() {
                    "circle" => PadShape::Circle,
                    "oval" => PadShape::Oval,
                    "roundrect" => PadShape::Roundrect,
                    _ => PadShape::Rect,
                },
                _ => PadShape::Rect,
            };
            let drill_size = match drill {
                None => None,
                Some(drill) if is_atom(drill.get(1), "oval") => {
                    Some(number(drill.get(2))?.min(number(drill.get(3))?))
                }
                Some(drill) => Some(number(drill.get(1))?),
            };
            let has_layer = |name: &str| {
                layers.is_some_and(|items| items.iter().any(|entry| is_atom(Some(entry), name)))
            };
            pads.push(Pad {
                id: format!("pad-{}", pads.len()),
                number: pad_number,
                at: at_point,
                size: size_value,
                shape,
                drill: drill_size,
                plated: is_atom(item.get(2), "np_thru_hole").then_some(false),
                side: (has_layer("B.Cu") && !has_layer("F.Cu")).then_some(Side::Back),
                rotation: at.get(3).map(|_| rotation),
                net_id: None,
            });
        }
        for item in footprint.iter().skip(1).filter_map(Expr::as_list) {
            let graphic = match item.first() {
                Some(Expr::Atom(name)) => name.as_str(),
                _ => "",
            };
            if !GRAPHICS.contains(&graphic) {
                continue;
            }
            let layer = value(child(item, "layer").and_then(|layer| layer.get(1)))?;
            if !layer_matches(layer) {
                continue;
            }
            let courtyard_graphic = layer.ends_with("CrtYd");
            let destination = if courtyard_graphic {
                &mut courtyard_points
            } else {
                &mut body_points
            };
            for name in ["start", "end", "center", "mid", "xy"] {
                for node in children(item, name) {
                    if node.len() >= 3 && !matches!(node[1], Expr::List(_)) {
                        destination.push(Vec2 {
                            x: number(node.get(1))?,
                            y: -number(node.get(2))?,
                        });
                    }
                }
            }
            for polygon in children(item, "pts") {
                let points = children(polygon, "xy")
                    .map(point)
                    .collect::<Result<Vec<_>>>()?;
                destination.extend(points.iter().copied());
                if courtyard_graphic && points.len() >= 3 {
                    courtyard_polygons.push(points);
                }
            }
            if graphic == "fp_rect"
                && let (Some(start), Some(end)) = (child(item, "start"), child(item, "end"))
            {
                let corner = |x: Option<&Expr>, y: Option<&Expr>| -> Result<Vec2> {
                    Ok(Vec2 {
                        x: number(x)?,
                        y: -number(y)?,
                    })
                };
                let rectangle = vec![
                    corner(start.get(1), start.get(2))?,
                    corner(end.get(1), start.get(2))?,
                    corner(end.get(1), end.get(2))?,
                    corner(start.get(1), end.get(2))?,
                ];
                destination.extend(rectangle.iter().copied());
                if courtyard_graphic {
                    courtyard_polygons.push(rectangle);
                }
            }
            if graphic == "fp_line"
                && courtyard_graphic
                && let (Some(start), Some(end)) = (child(item, "start"), child(item, "end"))
            {
                courtyard_segments.push((point(start)?, point(end)?));
            }
        }
    }
    let mut closed = courtyard_polygons;
    closed.extend(join_segments(courtyard_segments));
    let has_closed = !closed.is_empty();
    let source = if has_closed {
        if closed.len() == 1 {
            closed[0].clone()
        } else {
            bounds(&closed.concat())
        }
    } else if courtyard_points.len() >= 3 {
        courtyard_points
    } else {
        body_points
    };
    let mut courtyard = if has_closed && closed.len() == 1 {
        source
    } else {
        bounds(&source)
    };
    if courtyard.len() < 3 {
        let extents: Vec<Vec2> = pads
            .iter()
            .flat_map(|pad| {
                let angle = radians(pad.rotation.unwrap_or(0.0));
                let (hx, hy) = (pad.size.x / 2.0, pad.size.y / 2.0);
                [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)].map(|(x, y)| Vec2 {
                    x: pad.at.x + x * cos(angle) - y * sin(angle),
                    y: pad.at.y + x * sin(angle) + y * cos(angle),
                })
            })
            .collect();
        courtyard = bounds(&extents);
    }
    Ok(Geometry {
        pads,
        courtyard,
        courtyard_fallback: !has_closed,
    })
}

/// Chain loose line segments into closed loops; open chains are dropped.
fn join_segments(source: Vec<(Vec2, Vec2)>) -> Vec<Vec<Vec2>> {
    let same = |a: Vec2, b: Vec2| hypot(a.x - b.x, a.y - b.y) < 0.03;
    let mut pending = source;
    let mut loops = Vec::new();
    while !pending.is_empty() {
        let first = pending.remove(0);
        let mut path = vec![first.0, first.1];
        while !pending.is_empty() && !same(path[path.len() - 1], path[0]) {
            let last = path[path.len() - 1];
            let Some(index) = pending
                .iter()
                .position(|(a, b)| same(*a, last) || same(*b, last))
            else {
                break;
            };
            let (a, b) = pending.remove(index);
            path.push(if same(a, last) { b } else { a });
        }
        if path.len() >= 4 && same(path[path.len() - 1], path[0]) {
            path.pop();
            loops.push(path);
        }
    }
    loops
}

/// The bounding rectangle, or nothing when it is thinner than 0.01 mm.
fn bounds(points: &[Vec2]) -> Vec<Vec2> {
    if points.is_empty() {
        return Vec::new();
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for point in points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    if max_x - min_x < 0.01 || max_y - min_y < 0.01 {
        return Vec::new();
    }
    vec![
        Vec2 { x: min_x, y: min_y },
        Vec2 { x: max_x, y: min_y },
        Vec2 { x: max_x, y: max_y },
        Vec2 { x: min_x, y: max_y },
    ]
}
