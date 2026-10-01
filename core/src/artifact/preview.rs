//! Read-only board projection. Original source remains the export authority.
use super::sexpr::{atom, child, children, head, items};
use crate::model::*;
use kiutils_sexpr::Node;
use std::f64::consts::{PI, TAU};

fn error(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::ParseError, message)
}
fn value(n: &Node, i: usize) -> &str {
    items(n).and_then(|v| v.get(i)).and_then(atom).unwrap_or("")
}
fn number(n: &Node, i: usize, fallback: f64) -> Result<f64, ArtifactError> {
    let s = value(n, i);
    if s.is_empty() {
        return Ok(fallback);
    }
    let v: f64 = s
        .parse()
        .map_err(|_| error(format!("Invalid numeric board value: {s}")))?;
    if !v.is_finite() || v.abs() > 1e7 {
        return Err(error("Board coordinate is outside preview limits"));
    }
    Ok(v)
}
fn point(n: &Node) -> Result<Vec2, ArtifactError> {
    Ok(Vec2 {
        x: number(n, 1, 0.)?,
        y: -number(n, 2, 0.)?,
    })
}
fn xy(n: &Node, key: &str) -> Result<Vec2, ArtifactError> {
    child(n, key)
        .map(point)
        .unwrap_or(Ok(Vec2 { x: 0., y: 0. }))
}
fn scalar(n: &Node, key: &str, default: f64) -> Result<f64, ArtifactError> {
    child(n, key)
        .map(|n| number(n, 1, default))
        .unwrap_or(Ok(default))
}
fn pose(n: &Node) -> Result<Pose2, ArtifactError> {
    Ok(Pose2 {
        at: xy(n, "at")?,
        rotation: child(n, "at").map(|n| number(n, 3, 0.)).unwrap_or(Ok(0.))?,
    })
}
fn transform(p: Vec2, pose: &Pose2) -> Vec2 {
    let (s, c) = pose.rotation.to_radians().sin_cos();
    Vec2 {
        x: pose.at.x + p.x * c - p.y * s,
        y: pose.at.y + p.x * s + p.y * c,
    }
}
fn circle(at: Vec2, rx: f64, ry: f64) -> Vec<Vec2> {
    (0..64)
        .map(|i| {
            let t = i as f64 * TAU / 64.;
            Vec2 {
                x: at.x + rx * t.cos(),
                y: at.y + ry * t.sin(),
            }
        })
        .collect()
}
fn round_rect(size: Vec2, radius: f64) -> Vec<Vec2> {
    let r = radius.max(0.).min(size.x.min(size.y) / 2.);
    (0..4)
        .flat_map(|corner| {
            (0..17).map(move |i| {
                let t = (corner as f64 + i as f64 / 16.) * PI / 2.;
                let x = if corner == 0 || corner == 3 {
                    size.x / 2. - r
                } else {
                    -size.x / 2. + r
                };
                let y = if corner < 2 {
                    size.y / 2. - r
                } else {
                    -size.y / 2. + r
                };
                Vec2 {
                    x: x + r * t.cos(),
                    y: y + r * t.sin(),
                }
            })
        })
        .collect()
}
fn arc(n: &Node) -> Result<Vec<Vec2>, ArtifactError> {
    let a = xy(n, "start")?;
    let b = xy(n, "mid")?;
    let c = xy(n, "end")?;
    let d = 2. * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-10 {
        return Err(error("Degenerate board arc"));
    }
    let aa = a.x * a.x + a.y * a.y;
    let bb = b.x * b.x + b.y * b.y;
    let cc = c.x * c.x + c.y * c.y;
    let o = Vec2 {
        x: (aa * (b.y - c.y) + bb * (c.y - a.y) + cc * (a.y - b.y)) / d,
        y: (aa * (c.x - b.x) + bb * (a.x - c.x) + cc * (b.x - a.x)) / d,
    };
    let start = (a.y - o.y).atan2(a.x - o.x);
    let middle = ((b.y - o.y).atan2(b.x - o.x) - start).rem_euclid(TAU);
    let mut sweep = ((c.y - o.y).atan2(c.x - o.x) - start).rem_euclid(TAU);
    if middle > sweep {
        sweep -= TAU;
    }
    let radius = (a.x - o.x).hypot(a.y - o.y);
    let steps = ((radius * sweep.abs() / 0.1).ceil() as usize).clamp(8, 2048);
    Ok((0..=steps)
        .map(|i| {
            let t = start + sweep * i as f64 / steps as f64;
            Vec2 {
                x: o.x + radius * t.cos(),
                y: o.y + radius * t.sin(),
            }
        })
        .collect())
}
fn surface(layer: String, points: Vec<Vec2>, width: f64, filled: bool) -> PcbSurface {
    PcbSurface {
        layer,
        points,
        width,
        filled,
        text: String::new(),
        rotation: 0.,
        text_size: 1.,
    }
}
fn layers(n: &Node) -> Vec<String> {
    let ls = child(n, "layers").and_then(items).unwrap_or(&[]);
    let mut result = vec![];
    for l in ls.iter().skip(1).filter_map(atom) {
        match l {
            "*.Cu" => result.extend(["F.Cu".into(), "B.Cu".into()]),
            "*.Mask" => result.extend(["F.Mask".into(), "B.Mask".into()]),
            _ => result.push(l.into()),
        }
    }
    result
}
fn graphic(
    n: &Node,
    placement: &Pose2,
    out: &mut PcbPreview,
    edges: &mut Vec<Vec<Vec2>>,
    allow_fabrication: bool,
) -> Result<(), ArtifactError> {
    let layer = child(n, "layer").map(|n| value(n, 1)).unwrap_or("");
    if !matches!(
        layer,
        "Edge.Cuts" | "F.SilkS" | "B.SilkS" | "F.Cu" | "B.Cu" | "F.Mask" | "B.Mask"
    ) && !(allow_fabrication && matches!(layer, "F.Fab" | "B.Fab"))
    {
        return Ok(());
    }
    let tag = head(n).unwrap_or("");
    let width = if let Some(stroke) = child(n, "stroke") {
        scalar(stroke, "width", 0.15)?
    } else {
        scalar(n, "width", 0.15)?
    };
    let filled = child(n, "fill").is_some_and(|n| value(n, 1) == "solid");
    let mut closed = false;
    let pts = if tag.ends_with("_line") || tag == "segment" {
        vec![xy(n, "start")?, xy(n, "end")?]
    } else if tag.ends_with("_arc") || tag == "arc" {
        arc(n)?
    } else if tag.ends_with("_circle") {
        closed = true;
        let c = xy(n, "center")?;
        let e = xy(n, "end")?;
        let r = (e.x - c.x).hypot(e.y - c.y);
        circle(c, r, r)
    } else if tag.ends_with("_rect") {
        closed = true;
        let a = xy(n, "start")?;
        let b = xy(n, "end")?;
        vec![a, Vec2 { x: b.x, y: a.y }, b, Vec2 { x: a.x, y: b.y }]
    } else if tag.ends_with("_poly") {
        closed = true;
        child(n, "pts")
            .map(|n| children(n, "xy").map(point).collect())
            .unwrap_or(Ok(vec![]))?
    } else if tag.ends_with("_text") || tag == "property" {
        if items(n)
            .unwrap_or(&[])
            .iter()
            .any(|n| atom(n) == Some("hide"))
            || child(n, "hide").is_some_and(|n| value(n, 1) == "yes")
        {
            return Ok(());
        }
        // KiCad text may omit its angle before the optional unlocked flag.
        let at = if child(n, "at").is_some_and(|at| value(at, 3) == "unlocked") {
            Pose2 {
                at: xy(n, "at")?,
                rotation: 0.,
            }
        } else {
            pose(n)?
        };
        let text = if tag == "fp_text" || tag == "property" {
            value(n, 2)
        } else {
            value(n, 1)
        };
        let size = child(n, "effects")
            .and_then(|n| child(n, "font"))
            .and_then(|n| child(n, "size"))
            .map(|n| number(n, 1, 1.))
            .unwrap_or(Ok(1.))?;
        out.surfaces.push(PcbSurface {
            layer: layer.into(),
            points: vec![transform(at.at, placement)],
            width: 0.,
            filled: true,
            text: text.into(),
            rotation: at.rotation,
            text_size: size,
        });
        return Ok(());
    } else {
        out.diagnostics
            .push(format!("Unsupported preview graphic: {tag} on {layer}"));
        return Ok(());
    };
    let mut pts: Vec<_> = pts.into_iter().map(|p| transform(p, placement)).collect();
    if closed && !pts.is_empty() {
        pts.push(pts[0]);
    }
    if layer == "Edge.Cuts" {
        edges.push(pts);
    } else {
        out.surfaces.push(surface(layer.into(), pts, width, filled));
    }
    Ok(())
}
fn pad(n: &Node, fp: &Pose2, out: &mut PcbPreview) -> Result<(), ArtifactError> {
    let kind = value(n, 2);
    let shape = value(n, 3);
    let at = pose(n)?;
    let size = child(n, "size").ok_or_else(|| error("Pad has no size"))?;
    let size = Vec2 {
        x: number(size, 1, 0.)?,
        y: number(size, 2, 0.)?,
    };
    if size.x <= 0. || size.y <= 0. {
        return Err(error("Pad size must be positive"));
    }
    // KiCad pad angles are absolute; positions are footprint-local.
    let p = Pose2 {
        at: transform(at.at, fp),
        rotation: at.rotation,
    };
    let radius = match shape {
        "circle" | "oval" => size.x.min(size.y) / 2.,
        "roundrect" => size.x.min(size.y) * scalar(n, "roundrect_rratio", 0.25)?,
        _ => 0.,
    };
    let mut points = round_rect(size, radius);
    if shape == "custom" {
        if let Some(primitives) = child(n, "primitives") {
            for primitive in items(primitives).unwrap_or(&[]).iter().skip(1) {
                if head(primitive) == Some("gr_poly") {
                    points = child(primitive, "pts")
                        .map(|n| children(n, "xy").map(point).collect())
                        .unwrap_or(Ok(points))?;
                } else {
                    out.diagnostics
                        .push("Custom pad primitive approximated by its anchor".into());
                }
            }
        }
    } else if !matches!(shape, "rect" | "roundrect" | "circle" | "oval") {
        out.diagnostics
            .push(format!("Pad shape {shape} approximated by rectangle"));
    }
    let points: Vec<_> = points.into_iter().map(|v| transform(v, &p)).collect();
    if kind != "np_thru_hole" {
        for layer in layers(n) {
            if layer.ends_with(".Cu") || layer.ends_with(".Mask") {
                out.surfaces.push(surface(layer, points.clone(), 0., true));
            }
        }
    }
    if let Some(drill) = child(n, "drill") {
        let oval = value(drill, 1) == "oval";
        let i = if oval { 2 } else { 1 };
        let x = number(drill, i, 0.)?;
        let y = if oval { number(drill, i + 1, x)? } else { x };
        if x > 0. && y > 0. {
            let offset = xy(drill, "offset")?;
            out.holes.push(
                round_rect(Vec2 { x, y }, x.min(y) / 2.)
                    .into_iter()
                    .map(|v| {
                        transform(
                            Vec2 {
                                x: v.x + offset.x,
                                y: v.y + offset.y,
                            },
                            &p,
                        )
                    })
                    .collect(),
            );
        }
    }
    Ok(())
}
fn xyz(n: &Node, key: &str, default: f64) -> Result<Vec3, ArtifactError> {
    let n = child(n, key).and_then(|n| child(n, "xyz"));
    match n {
        Some(n) => Ok(Vec3 {
            x: number(n, 1, default)?,
            y: number(n, 2, default)?,
            z: number(n, 3, default)?,
        }),
        None => Ok(Vec3 {
            x: default,
            y: default,
            z: default,
        }),
    }
}
fn inside(p: Vec2, poly: &[Vec2]) -> bool {
    let mut result = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            result = !result;
        }
    }
    result
}
fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).hypot(a.y - b.y) < 0.002
}
fn contours(mut edges: Vec<Vec<Vec2>>) -> Result<Vec<Contour>, ArtifactError> {
    edges.retain(|p| p.len() > 1);
    let mut loops = vec![];
    while let Some(mut path) = edges.pop() {
        while !close(path[0], *path.last().unwrap()) {
            let last = *path.last().unwrap();
            let Some(i) = edges
                .iter()
                .position(|e| close(e[0], last) || close(*e.last().unwrap(), last))
            else {
                return Err(error("PCB outline is open; close Edge.Cuts in KiCad"));
            };
            let mut next = edges.swap_remove(i);
            if !close(next[0], last) {
                next.reverse();
            }
            path.extend(next.into_iter().skip(1));
        }
        path.pop();
        if path.len() >= 3 {
            loops.push(path);
        }
    }
    Ok(loops
        .iter()
        .enumerate()
        .map(|(i, points)| Contour {
            points: points.clone(),
            hole: loops
                .iter()
                .enumerate()
                .filter(|(j, p)| *j != i && inside(points[0], p))
                .count()
                % 2
                == 1,
        })
        .collect())
}
pub(super) fn board(source: &str, revision: u64) -> Result<PcbPreview, ArtifactError> {
    if source.len() > 32 * 1024 * 1024 {
        return Err(error("Board exceeds the 32 MiB preview limit"));
    }
    let parsed = kiutils_sexpr::parse_one(source).map_err(|e| error(e.to_string()))?;
    let root = parsed.nodes.first().ok_or_else(|| error("Empty board"))?;
    if head(root) != Some("kicad_pcb") {
        return Err(error("Expected a .kicad_pcb board"));
    }
    let thickness = child(root, "general")
        .map(|n| scalar(n, "thickness", 1.6))
        .unwrap_or(Ok(1.6))?;
    if thickness <= 0. || thickness > 100. {
        return Err(error("Invalid PCB thickness"));
    }
    let mut out = PcbPreview {
        revision,
        thickness,
        contours: vec![],
        surfaces: vec![],
        holes: vec![],
        models: vec![],
        diagnostics: vec![],
    };
    let origin = Pose2 {
        at: Vec2 { x: 0., y: 0. },
        rotation: 0.,
    };
    let mut edges = vec![];
    for n in items(root).unwrap_or(&[]).iter().skip(1) {
        match head(n).unwrap_or("") {
            "footprint" | "module" => {
                let fp = pose(n)?;
                let side = if child(n, "layer").is_some_and(|n| value(n, 1).starts_with("B.")) {
                    Side::Back
                } else {
                    Side::Front
                };
                let reference = children(n, "property")
                    .find(|n| value(n, 1) == "Reference")
                    .map(|n| value(n, 2))
                    .or_else(|| {
                        children(n, "fp_text")
                            .find(|n| value(n, 1) == "reference")
                            .map(|n| value(n, 2))
                    })
                    .unwrap_or("Component")
                    .to_owned();
                for node in items(n).unwrap_or(&[]).iter().skip(1) {
                    match head(node).unwrap_or("") {
                        "pad" => pad(node, &fp, &mut out)?,
                        "model" => {
                            if child(node, "hide").is_none() {
                                out.models.push(PcbModel {
                                    id: format!("{}:{}", reference, out.models.len()),
                                    reference: reference.clone(),
                                    path: value(node, 1).into(),
                                    pose: fp.clone(),
                                    side: side.clone(),
                                    offset: xyz(node, "offset", 0.)?,
                                    rotation: xyz(node, "rotate", 0.)?,
                                    scale: xyz(node, "scale", 1.)?,
                                });
                            }
                        }
                        tag if tag.starts_with("fp_") || tag == "property" => {
                            graphic(node, &fp, &mut out, &mut edges, false)?
                        }
                        _ => {}
                    }
                }
            }
            "via" => {
                let at = xy(n, "at")?;
                let size = scalar(n, "size", 0.6)?;
                let drill = scalar(n, "drill", 0.3)?;
                for layer in ["F.Cu", "B.Cu"] {
                    out.surfaces.push(surface(
                        layer.into(),
                        circle(at, size / 2., size / 2.),
                        0.,
                        true,
                    ));
                }
                out.holes.push(circle(at, drill / 2., drill / 2.));
            }
            "zone" => {
                let layer = child(n, "layer").map(|n| value(n, 1)).unwrap_or("F.Cu");
                let fills: Vec<_> = children(n, "filled_polygon").collect();
                if fills.is_empty() {
                    out.diagnostics.push(
                        "Zone has no saved fill; refill it in KiCad to preview copper".into(),
                    );
                }
                for fill in fills {
                    let layer = child(fill, "layer").map(|n| value(n, 1)).unwrap_or(layer);
                    let points = child(fill, "pts")
                        .map(|n| children(n, "xy").map(point).collect())
                        .unwrap_or(Ok(vec![]))?;
                    out.surfaces.push(surface(layer.into(), points, 0., true));
                }
            }
            tag if tag.starts_with("gr_") || tag == "segment" || tag == "arc" => {
                graphic(n, &origin, &mut out, &mut edges, false)?
            }
            _ => {}
        }
    }
    out.contours = contours(edges)?;
    if out.contours.is_empty() {
        return Err(error("PCB has no closed Edge.Cuts outline"));
    }
    out.diagnostics.sort();
    out.diagnostics.dedup();
    Ok(out)
}

/// Project only source-local front/back silkscreen and fabrication artwork from a footprint.
pub(super) fn footprint_surfaces(source: &str) -> Result<Vec<PcbSurface>, ArtifactError> {
    if source.len() > 4 * 1024 * 1024 {
        return Err(error("Footprint source exceeds the 4 MiB preview limit"));
    }
    let parsed = kiutils_sexpr::parse_one(source).map_err(|e| error(e.to_string()))?;
    let root = parsed
        .nodes
        .first()
        .ok_or_else(|| error("Empty footprint source"))?;
    if !matches!(head(root), Some("footprint" | "module")) {
        return Err(error("Expected a KiCad footprint source"));
    }
    let mut out = PcbPreview {
        revision: 0,
        thickness: 0.,
        contours: Vec::new(),
        surfaces: Vec::new(),
        holes: Vec::new(),
        models: Vec::new(),
        diagnostics: Vec::new(),
    };
    let identity = Pose2 {
        at: Vec2 { x: 0., y: 0. },
        rotation: 0.,
    };
    let mut edges = Vec::new();
    for node in items(root).unwrap_or(&[]).iter().skip(1) {
        let tag = head(node).unwrap_or("");
        if tag.starts_with("fp_") || tag == "property" {
            graphic(node, &identity, &mut out, &mut edges, true)?;
        }
    }
    out.surfaces.retain(|surface| {
        matches!(
            surface.layer.as_str(),
            "F.SilkS" | "B.SilkS" | "F.Fab" | "B.Fab"
        )
    });
    Ok(out.surfaces)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EDGE: &str =
        "(gr_rect (start 0 0) (end 20 10) (layer \"Edge.Cuts\") (stroke (width 0.05)))";
    #[test]
    fn projects_copper_holes_models_and_saved_fills() {
        let input = format!(
            "(kicad_pcb (general (thickness 1.2)) {EDGE} (segment (start 1 2) (end 5 2) (width 0.3) (layer \"F.Cu\")) (via (at 5 2) (size 0.7) (drill 0.3)) (footprint \"switch\" (layer \"B.Cu\") (at 4 5 30) (property \"Reference\" \"SW1\") (pad \"1\" thru_hole oval (at 2 0 30) (size 3 2) (drill oval 1.5 0.8) (layers \"*.Cu\" \"*.Mask\")) (model \"switch.step\" (offset (xyz 1 2 3)) (rotate (xyz 20 40 70)))) (zone (layer \"B.Cu\")))"
        );
        let result = board(&input, 17).unwrap();
        assert_eq!(result.revision, 17);
        assert_eq!(result.thickness, 1.2);
        assert_eq!(result.contours.len(), 1);
        assert_eq!(result.holes.len(), 2);
        assert_eq!(result.models[0].side, Side::Back);
        assert_eq!(result.models[0].offset.y, 2.);
        assert_eq!(result.models[0].pose.at.y, -5.);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|s| s.contains("no saved fill"))
        );
        assert!(result.surfaces.iter().any(|s| s.layer == "B.Mask"));
    }
    #[test]
    fn footprint_projection_includes_fab_without_changing_board_preview_layers() {
        let footprint = "(footprint \"resistor\" (layer \"B.Cu\") (at 0 0) (fp_line (start 0 0) (end 2 0) (stroke (width 0.1)) (layer \"B.SilkS\")) (fp_rect (start -1 -1) (end 1 1) (stroke (width 0.05)) (layer \"B.Fab\")))";
        let surfaces = footprint_surfaces(footprint).unwrap();
        assert!(surfaces.iter().any(|surface| surface.layer == "B.SilkS"));
        assert!(surfaces.iter().any(|surface| surface.layer == "B.Fab"));

        let board = board(&format!("(kicad_pcb {EDGE} {footprint})"), 0).unwrap();
        assert!(
            board
                .surfaces
                .iter()
                .any(|surface| surface.layer == "B.SilkS")
        );
        assert!(
            !board
                .surfaces
                .iter()
                .any(|surface| surface.layer == "B.Fab")
        );
    }
    #[test]
    fn rejects_open_edges_and_non_finite_values() {
        assert!(
            board(
                "(kicad_pcb (gr_line (start 0 0) (end 1 1) (layer \"Edge.Cuts\")))",
                0
            )
            .is_err()
        );
        assert!(
            board(
                &format!("(kicad_pcb {EDGE} (segment (start NaN 0) (end 1 1) (layer \"F.Cu\")))"),
                0
            )
            .is_err()
        );
    }
    #[test]
    fn joins_edges_and_classifies_cutouts() {
        let result = board(
            &format!("(kicad_pcb {EDGE} (gr_circle (center 5 5) (end 6 5) (layer \"Edge.Cuts\")))"),
            0,
        )
        .unwrap();
        assert_eq!(result.contours.iter().filter(|c| c.hole).count(), 1);
    }
}
