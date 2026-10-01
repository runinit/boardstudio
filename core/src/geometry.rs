use crate::model::{
    BoardOutlineScene, Contour, Finding, Operation, OutlineFeature, OutlineSettings, Part,
    ProjectDoc, Scope, Severity, Vec2,
};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use std::collections::BTreeMap;
#[path = "outline.rs"]
mod automatic;
#[path = "outline_cleanup.rs"]
mod cleanup;

// Millimetre coordinates are quantized to a micrometre before clipping.
const SCALE: f64 = 1_000.0;
const MAX_CHORD_ERROR_MM: f64 = 0.05;
const MAX_ARC_SEGMENTS: usize = 1024;
type Path = Vec<[f64; 2]>;
type Shapes = Vec<Vec<Path>>;

fn snap(value: f64) -> f64 {
    (value * SCALE).round() / SCALE
}
fn point(p: Vec2) -> [f64; 2] {
    [snap(p.x), snap(p.y)]
}

fn rect(center: Vec2, size: Vec2, radius: f64) -> Path {
    let hx = size.x / 2.0;
    let hy = size.y / 2.0;
    let r = radius.max(0.0).min(hx).min(hy);
    if r == 0.0 {
        return vec![
            point(Vec2 {
                x: center.x - hx,
                y: center.y - hy,
            }),
            point(Vec2 {
                x: center.x + hx,
                y: center.y - hy,
            }),
            point(Vec2 {
                x: center.x + hx,
                y: center.y + hy,
            }),
            point(Vec2 {
                x: center.x - hx,
                y: center.y + hy,
            }),
        ];
    }
    let corners = [
        (center.x + hx - r, center.y + hy - r, 0.0),
        (center.x - hx + r, center.y + hy - r, 90.0),
        (center.x - hx + r, center.y - hy + r, 180.0),
        (center.x + hx - r, center.y - hy + r, 270.0),
    ];
    let max_angle = 2.0 * (1.0 - MAX_CHORD_ERROR_MM / r).clamp(-1.0, 1.0).acos();
    let segments =
        ((std::f64::consts::FRAC_PI_2 / max_angle).ceil() as usize).clamp(1, MAX_ARC_SEGMENTS);
    let mut path = Vec::with_capacity((segments + 1) * 4);
    for (cx, cy, start) in corners {
        for step in 0..=segments {
            let angle = (start + step as f64 * 90.0 / segments as f64).to_radians();
            path.push([snap(cx + r * angle.cos()), snap(cy + r * angle.sin())]);
        }
    }
    path
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FeatureShape {
    shapes: Shapes,
    notices: Vec<String>,
    bridges: Vec<crate::model::OutlineBridge>,
    gaps: Vec<crate::model::OutlineGap>,
}
type FeatureGeometry = Result<FeatureShape, String>;
fn feature_geometry(doc: &ProjectDoc, feature: &OutlineFeature) -> FeatureGeometry {
    crate::outline_controls::validate(doc, feature)?;
    let mut path = match feature {
        OutlineFeature::Polygon { points, .. } => points.iter().copied().map(point).collect(),
        OutlineFeature::Rect {
            center,
            size,
            radius,
            ..
        } => {
            if !size.x.is_finite()
                || !size.y.is_finite()
                || size.x <= 0.0
                || size.y <= 0.0
                || !radius.is_finite()
                || *radius < 0.0
            {
                return Err("Rectangle dimensions must be positive and radius nonnegative".into());
            }
            rect(*center, *size, *radius)
        }
        OutlineFeature::PartEnvelope {
            part_ids,
            connections,
            margin,
            settings,
            ..
        } => {
            let mut axes: Vec<f64> = doc
                .layouts
                .iter()
                .filter_map(|layout| {
                    let link = layout.mirror_link.as_ref()?;
                    let board = doc.boards.iter().find(|board| board.id == layout.board_id);
                    if board
                        .is_some_and(|board| !part_ids.iter().any(|id| board.part_ids.contains(id)))
                    {
                        return None;
                    }
                    Some(link.axis_x)
                })
                .collect();
            axes.sort_by(f64::total_cmp);
            axes.dedup();
            if axes.is_empty() {
                return automatic::envelope(
                    doc,
                    feature.id(),
                    part_ids,
                    *margin,
                    settings,
                    connections,
                );
            }
            // Build each physical half independently so automatic bridges never cross a split.
            let mut groups: BTreeMap<Vec<bool>, Vec<String>> = BTreeMap::new();
            for id in part_ids {
                let Some(part) = doc.parts.iter().find(|part| &part.id == id) else {
                    continue;
                };
                let owner = doc.layouts.iter().find(|layout| {
                    layout.part_ids.contains(id)
                        || doc.matrices.iter().any(|matrix| {
                            matrix.id == layout.matrix_id && matrix.part_ids.contains(id)
                        })
                });
                let x = owner
                    .and_then(|layout| {
                        doc.matrices
                            .iter()
                            .find(|matrix| matrix.id == layout.matrix_id)
                    })
                    .map_or(part.pose.at.x, |matrix| matrix.origin.x);
                groups
                    .entry(axes.iter().map(|axis| x < *axis).collect())
                    .or_default()
                    .push(id.clone());
            }
            let mut shapes = vec![];
            let mut warnings = vec![];
            let mut bridges = vec![];
            let mut gaps = vec![];
            let mut grouped_connections: BTreeMap<Vec<bool>, Vec<crate::model::OutlineConnection>> =
                BTreeMap::new();
            for connection in connections {
                let mut group = None;
                for control in &connection.points {
                    let point = crate::outline_controls::point(doc, control)?;
                    let key: Vec<_> = axes.iter().map(|axis| point.x < *axis).collect();
                    if axes
                        .iter()
                        .any(|axis| (point.x - axis).abs() <= connection.width / 2.0)
                        || group.as_ref().is_some_and(|previous| previous != &key)
                    {
                        return Err("Manual bridges cannot cross split boundaries".into());
                    }
                    group = Some(key);
                }
                if let Some(key) = group {
                    grouped_connections
                        .entry(key)
                        .or_default()
                        .push(connection.clone());
                }
            }
            if grouped_connections
                .keys()
                .any(|key| !groups.contains_key(key))
            {
                return Err("Manual bridge has no included parts on this side of the split".into());
            }
            for (group, ids) in &groups {
                let connections = grouped_connections
                    .get(group)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let half =
                    automatic::envelope(doc, feature.id(), ids, *margin, settings, connections)?;
                shapes.extend(half.shapes);
                warnings.extend(half.notices);
                bridges.extend(half.bridges);
                gaps.extend(half.gaps);
            }
            return Ok(FeatureShape {
                shapes,
                notices: warnings,
                bridges,
                gaps,
            });
        }
    };
    if let OutlineFeature::Rect {
        center,
        rotation: Some(rotation),
        ..
    } = feature
    {
        if !rotation.is_finite() {
            return Err("Rectangle rotation must be finite".into());
        }
        let (sin, cos) = rotation.to_radians().sin_cos();
        path = path
            .into_iter()
            .map(|p| {
                let x = p[0] - center.x;
                let y = p[1] - center.y;
                point(Vec2 {
                    x: center.x + x * cos - y * sin,
                    y: center.y + x * sin + y * cos,
                })
            })
            .collect();
    }
    let anchor = match feature {
        OutlineFeature::Polygon { anchor_part_id, .. }
        | OutlineFeature::Rect { anchor_part_id, .. } => anchor_part_id.as_ref(),
        _ => None,
    };
    if let Some(id) = anchor {
        let part = doc
            .parts
            .iter()
            .find(|part| &part.id == id)
            .ok_or("Outline attachment is missing")?;
        path = path
            .into_iter()
            .map(|p| {
                point(crate::outline_controls::world(
                    Vec2 { x: p[0], y: p[1] },
                    part,
                ))
            })
            .collect();
    }
    if !automatic::simple(&path) {
        return Err(
            "Outline requires a non-self-intersecting polygon with three distinct finite points"
                .into(),
        );
    }
    Ok(FeatureShape {
        shapes: vec![vec![path]],
        notices: vec![],
        bridges: vec![],
        gaps: vec![],
    })
}
#[derive(Clone, Default)]
pub struct OutlineCache {
    layouts: Vec<crate::model::Layout>,
    paths: BTreeMap<String, (OutlineFeature, FeatureGeometry)>,
}
pub fn outlines(
    doc: &ProjectDoc,
    previous: Option<&OutlineCache>,
    moved: &[String],
) -> (OutlineCache, Vec<Contour>, Vec<Finding>) {
    let mut cache = OutlineCache {
        layouts: doc.layouts.clone(),
        ..Default::default()
    };
    let mut features: Vec<_> = doc.outline.iter().collect();
    for board in &doc.boards {
        if let Some(snapshot) = crate::outline_versions::active_snapshot(doc, &board.id) {
            features.extend(&snapshot.features);
        }
    }
    for feature in features {
        let paths = previous
            .filter(|old| old.layouts == doc.layouts)
            .and_then(|old| old.paths.get(feature.id()))
            .filter(|(old_feature, _)| old_feature == feature && !depends_on(feature, moved))
            .map(|(_, paths)| paths.clone())
            .unwrap_or_else(|| feature_geometry(doc, feature));
        cache
            .paths
            .insert(feature.id().into(), (feature.clone(), paths));
    }
    let (contours, findings) = if doc
        .boards
        .iter()
        .any(|board| crate::outline_versions::active_snapshot(doc, &board.id).is_some())
    {
        let mut contours = vec![];
        let mut findings = vec![];
        for board in &doc.boards {
            let result = compose(
                crate::outline_versions::features(doc, board).into_iter(),
                &cache,
                crate::outline_versions::settings(doc, &board.id),
            );
            contours.extend(result.contours);
            findings.extend(result.findings);
        }
        // Outlines without a board owner retain their existing scripting behavior.
        let result = compose(
            doc.outline.iter().filter(|feature| {
                !doc.boards
                    .iter()
                    .any(|board| board.outline_ids.iter().any(|id| id == feature.id()))
            }),
            &cache,
            None,
        );
        contours.extend(result.contours);
        findings.extend(result.findings);
        (contours, findings)
    } else {
        let result = compose(doc.outline.iter(), &cache, None);
        (result.contours, result.findings)
    };
    (cache, contours, findings)
}

fn depends_on(feature: &OutlineFeature, moved: &[String]) -> bool {
    crate::outline_controls::dependencies(feature)
        .iter()
        .any(|id| moved.iter().any(|moved| moved == id))
}

pub(crate) struct ComposedOutline {
    pub source: Vec<Contour>,
    pub contours: Vec<Contour>,
    pub findings: Vec<Finding>,
    pub bridges: Vec<crate::model::OutlineBridge>,
    pub gaps: Vec<crate::model::OutlineGap>,
    pub corner_locations: Vec<Vec2>,
}
fn compose<'a>(
    features: impl Iterator<Item = &'a OutlineFeature>,
    cache: &OutlineCache,
    settings: Option<&'a OutlineSettings>,
) -> ComposedOutline {
    let mut shapes: Shapes = vec![];
    let mut findings = vec![];
    let mut finishing = settings;
    let mut target_ids = Vec::new();
    let mut bridges = vec![];
    let mut gaps = vec![];
    for feature in features {
        target_ids.push(feature.id().to_owned());
        if let OutlineFeature::PartEnvelope { settings, .. } = feature {
            finishing.get_or_insert(settings);
        }
        let Some((_, geometry)) = cache.paths.get(feature.id()) else {
            continue;
        };
        let result = match geometry {
            Ok(value) => value,
            Err(message) => {
                findings.push(Finding {
                    id: format!("outline:{}:invalid", feature.id()),
                    severity: Severity::Error,
                    scope: Scope::Outline,
                    message: message.clone(),
                    target_ids: vec![feature.id().into()],
                });
                continue;
            }
        };
        bridges.extend(result.bridges.clone());
        gaps.extend(result.gaps.clone());
        for (i, message) in result.notices.iter().enumerate() {
            findings.push(Finding {
                id: format!("outline:{}:notice:{i}", feature.id()),
                severity: Severity::Warning,
                scope: Scope::Outline,
                message: message.clone(),
                target_ids: vec![feature.id().into()],
            });
        }
        let rule = match feature.operation() {
            Operation::Add => OverlayRule::Union,
            Operation::Subtract => OverlayRule::Difference,
        };
        // An addition is material, never an implicit cutout. Closing a recess
        // can introduce a new enclosed ring; restore existing/authored holes.
        let addition = settings.is_some()
            && matches!(
                feature,
                OutlineFeature::Polygon {
                    operation: Operation::Add,
                    ..
                }
            );
        let retained_holes: Shapes = if addition {
            shapes
                .iter()
                .chain(result.shapes.iter())
                .flat_map(|shape| shape.iter().skip(1).map(|hole| vec![hole.clone()]))
                .collect()
        } else {
            vec![]
        };
        shapes = shapes.overlay(&result.shapes, rule, FillRule::EvenOdd);
        if addition {
            for shape in &mut shapes {
                shape.truncate(1);
            }
            if !retained_holes.is_empty() {
                shapes =
                    shapes.overlay(&retained_holes, OverlayRule::Difference, FillRule::EvenOdd);
            }
        }
    }
    // Float clipping may introduce sub-grid vertices on straight shared edges.
    // Resolve those at our document precision before measuring corner lengths.
    for path in shapes.iter_mut().flatten() {
        for p in path.iter_mut() {
            *p = [snap(p[0]), snap(p[1])];
        }
        path.dedup();
        if path.len() > 1 && path.first() == path.last() {
            path.pop();
        }
        let original = path.clone();
        let n = original.len();
        if n >= 3 {
            *path = (0..n)
                .filter_map(|i| {
                    let a = original[(i + n - 1) % n];
                    let b = original[i];
                    let c = original[(i + 1) % n];
                    let cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
                    (cross.abs() > 1e-8).then_some(b)
                })
                .collect();
        }
    }
    if shapes.iter().flatten().any(|path| !automatic::simple(path)) {
        findings.push(Finding {
            id: "outline:precision:invalid".into(),
            severity: Severity::Error,
            scope: Scope::Outline,
            message: "Outline becomes invalid at 0.001 mm precision; increase narrow features"
                .into(),
            target_ids: target_ids.clone(),
        });
    }
    let source = contours(&shapes);
    let mut corner_locations = vec![];
    if let Some(settings) = finishing {
        match automatic::finish(shapes.clone(), settings) {
            Ok((finished, reduced, locations)) => {
                corner_locations = locations;
                shapes = finished;
                if let Some(actual) = reduced {
                    findings.push(Finding {id:"outline:corners:fitted".into(),severity:Severity::Warning,scope:Scope::Outline,message:format!("Corner size reduced from {} mm to as little as {:.3} mm to fit nearby edges",settings.size,actual),target_ids:target_ids.clone()});
                }
            }
            Err(message) => findings.push(Finding {
                id: "outline:corners:invalid".into(),
                severity: Severity::Error,
                scope: Scope::Outline,
                message,
                target_ids: target_ids.clone(),
            }),
        }
    }
    ComposedOutline {
        source,
        corner_locations,
        contours: contours(&shapes),
        findings,
        bridges,
        gaps,
    }
}

fn contours(shapes: &Shapes) -> Vec<Contour> {
    shapes
        .iter()
        .flat_map(|shape| {
            shape.iter().enumerate().map(|(index, path)| Contour {
                points: path.iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
                hole: index > 0,
            })
        })
        .collect()
}

/// Keep a complete authored rounded opening analytic when it survives composition.
/// Intersected/combined openings retain their resolved polygon instead.
pub(crate) fn retained_cutout(
    doc: &ProjectDoc,
    cache: &OutlineCache,
    board: &crate::model::Board,
    hole: &Contour,
    id: String,
) -> Option<OutlineFeature> {
    let target = vec![vec![
        hole.points.iter().copied().map(point).collect::<Path>(),
    ]];
    for original in board
        .outline_ids
        .iter()
        .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
    {
        let OutlineFeature::Rect {
            center,
            size,
            radius,
            rotation,
            anchor_part_id,
            operation: Operation::Subtract,
            ..
        } = original
        else {
            continue;
        };
        let Ok(shape) = &cache.paths.get(original.id())?.1 else {
            continue;
        };
        let difference = shape
            .shapes
            .overlay(&target, OverlayRule::Xor, FillRule::EvenOdd);
        if !difference.is_empty() {
            continue;
        }
        let anchor = anchor_part_id
            .as_ref()
            .and_then(|id| doc.parts.iter().find(|part| &part.id == id));
        let center = anchor.map_or(*center, |part| {
            crate::outline_controls::world(*center, part)
        });
        let rotation = anchor.map_or(rotation.unwrap_or_default(), |part| {
            part.pose.rotation
                + rotation.unwrap_or_default()
                    * if part.side == crate::model::Side::Back {
                        -1.0
                    } else {
                        1.0
                    }
        });
        return Some(OutlineFeature::Rect {
            id,
            anchor_part_id: None,
            center,
            size: *size,
            radius: *radius,
            rotation: Some(rotation),
            operation: Operation::Subtract,
        });
    }
    None
}

pub(crate) fn board_source(
    doc: &ProjectDoc,
    cache: &OutlineCache,
    board: &crate::model::Board,
) -> ComposedOutline {
    compose(
        crate::outline_versions::features(doc, board).into_iter(),
        cache,
        crate::outline_versions::settings(doc, &board.id),
    )
}
pub(crate) fn generated_source(
    doc: &ProjectDoc,
    cache: &OutlineCache,
    board: &crate::model::Board,
) -> ComposedOutline {
    compose(
        board
            .outline_ids
            .iter()
            .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id)),
        cache,
        None,
    )
}
pub(crate) fn valid_polygon(points: &[Vec2]) -> bool {
    automatic::simple(&points.iter().copied().map(point).collect())
}

pub(crate) fn board_outline_scenes(
    doc: &ProjectDoc,
    cache: &OutlineCache,
) -> Vec<BoardOutlineScene> {
    doc.boards
        .iter()
        .map(|board| {
            let result = board_source(doc, cache, board);
            BoardOutlineScene {
                board_id: board.id.clone(),
                source_contours: result.source,
                bridges: crate::outline_versions::active_snapshot(doc, &board.id)
                    .map(|snapshot| snapshot.bridges.clone())
                    .unwrap_or(result.bridges),
                gaps: result.gaps,
            }
        })
        .collect()
}

pub fn board_contours(
    doc: &ProjectDoc,
    cache: &OutlineCache,
) -> (
    Vec<crate::model::BoardContours>,
    Vec<Finding>,
    Vec<crate::model::FindingMarker>,
) {
    let mut boards = Vec::with_capacity(doc.boards.len());
    let mut findings = vec![];
    let mut markers = vec![];
    for board in &doc.boards {
        let result = board_source(doc, cache, board);
        markers.extend(corner_markers_for(&board.id, &result.corner_locations));
        for problem in result.findings {
            findings.push(Finding {
                id: format!("board:{}:feature:{}", board.id, problem.id),
                severity: problem.severity,
                scope: Scope::Pcb,
                message: problem.message,
                target_ids: vec![board.id.clone()]
                    .into_iter()
                    .chain(problem.target_ids)
                    .collect(),
            });
        }
        boards.push(crate::model::BoardContours {
            board_id: board.id.clone(),
            contours: result.contours,
        });
    }
    (boards, findings, markers)
}

pub fn part_valid(part: &Part, doc: &ProjectDoc) -> bool {
    doc.definitions
        .iter()
        .any(|def| def.id == part.definition_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PartDefinition, PartKind, Pose2, Side};
    use std::time::Instant;

    #[test]
    fn linked_halves_do_not_bridge_automatic_envelopes() {
        let mut doc = fixture(2);
        doc.parts[1].pose.at.x = 40.0;
        doc.outline.truncate(1);
        if let OutlineFeature::PartEnvelope {
            part_ids, settings, ..
        } = &mut doc.outline[0]
        {
            *part_ids = vec!["key-0".into(), "key-1".into()];
            settings.bridge_width = 10.0;
        }
        doc.layouts = serde_json::from_value(serde_json::json!([
            {"id":"left","name":"Left","boardId":"board","matrixId":"left-matrix","partIds":["key-0"]},
            {"id":"right","name":"Right","boardId":"board","matrixId":"right-matrix","partIds":["key-1"],"mirrorLink":{"sourceId":"left","axisX":20.0}}
        ])).unwrap();
        let shapes = feature_geometry(&doc, &doc.outline[0]).unwrap().shapes;
        assert_eq!(
            shapes.len(),
            2,
            "split halves must retain separate outlines"
        );
    }

    const SAMPLES: usize = 100;
    const WARMUP: usize = 10;
    const PITCH: f64 = 19.05;

    fn fixture(keys: usize) -> ProjectDoc {
        let mut doc = ProjectDoc::empty("bench", "Benchmark");
        doc.definitions.push(PartDefinition {
            input_profile: None,
            mechanical_profile: None,
            id: "switch".into(),
            name: "Switch".into(),
            kind: PartKind::Switch,
            courtyard: vec![
                Vec2 { x: -7.0, y: -7.0 },
                Vec2 { x: 7.0, y: -7.0 },
                Vec2 { x: 7.0, y: 7.0 },
                Vec2 { x: -7.0, y: 7.0 },
            ],
            pads: vec![],
            models: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
        });
        for index in 0..keys {
            let id = format!("key-{index}");
            doc.parts.push(Part {
                id: id.clone(),
                definition_id: "switch".into(),
                reference: format!("SW{}", index + 1),
                pose: Pose2 {
                    at: Vec2 {
                        x: (index % 20) as f64 * PITCH,
                        y: (index / 20) as f64 * PITCH,
                    },
                    rotation: 0.0,
                },
                side: Side::Front,
                locked: None,
                keycap: None,
                outline: None,
                properties: None,
                generator_parameters: None,
            });
            doc.outline.push(OutlineFeature::PartEnvelope {
                connections: vec![],
                id: format!("edge-{index}"),
                part_ids: vec![id],
                settings: Default::default(),
                margin: 1.0,
                operation: Operation::Add,
            });
        }
        doc
    }

    #[test]
    fn finishing_findings_retain_outline_targets() {
        let mut doc = fixture(1);
        if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
            settings.size = 100.0;
            settings.corners = crate::model::CornerStyle::Fillet;
        }
        let (cache, _, findings) = outlines(&doc, None, &[]);
        let fitted = findings
            .iter()
            .find(|finding| finding.id == "outline:corners:fitted")
            .expect("oversized corners should be fitted");
        assert_eq!(fitted.target_ids, vec!["edge-0"]);
        let (_, _, cached) = outlines(&doc, Some(&cache), &[]);
        assert_eq!(
            serde_json::to_value(findings).unwrap(),
            serde_json::to_value(cached).unwrap()
        );
    }

    #[test]
    fn rounded_rect_uses_physical_chord_tolerance() {
        let small = rect(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 2.0, y: 2.0 }, 0.5);
        let large = rect(Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 200.0, y: 200.0 }, 50.0);
        assert!(large.len() > small.len());
        assert!(large.len() <= (MAX_ARC_SEGMENTS + 1) * 4);
        for (radius, path) in [(0.5, small), (50.0, large)] {
            let segments = path.len() / 4 - 1;
            let sagitta =
                radius * (1.0 - (std::f64::consts::FRAC_PI_2 / segments as f64 / 2.0).cos());
            assert!(sagitta <= MAX_CHORD_ERROR_MM);
        }
    }

    #[test]
    fn cached_move_matches_full_and_reuses_paths() {
        let mut doc = fixture(4);
        let (old, _, _) = outlines(&doc, None, &[]);
        doc.parts[0].pose.at.x += 3.0;
        let (cached, contours, findings) = outlines(&doc, Some(&old), &["key-0".into()]);
        let (full, expected, expected_findings) = outlines(&doc, None, &[]);
        assert_eq!(contours, expected);
        assert_eq!(findings, expected_findings);
        assert_ne!(cached.paths["edge-0"].1, old.paths["edge-0"].1);
        assert_eq!(cached.paths["edge-1"].1, old.paths["edge-1"].1);
        assert_eq!(cached.paths["edge-0"].1, full.paths["edge-0"].1);
    }

    fn percentile(samples: &mut [u128], percent: usize) -> u128 {
        samples.sort_unstable();
        samples[samples.len() * percent / 100]
    }

    fn compare(keys: usize) {
        let mut doc = fixture(keys);
        let (mut cache, _, _) = outlines(&doc, None, &[]);
        let moved = vec!["key-0".into()];
        let mut full = Vec::with_capacity(SAMPLES);
        let mut cached = Vec::with_capacity(SAMPLES);
        for index in 0..SAMPLES + WARMUP {
            doc.parts[0].pose.at.x = index as f64 * 0.01;
            let start = Instant::now();
            let (_, full_contours, _) = outlines(&doc, None, &[]);
            let full_time = start.elapsed().as_micros();
            let start = Instant::now();
            let (next, cached_contours, _) = outlines(&doc, Some(&cache), &moved);
            let cached_time = start.elapsed().as_micros();
            assert_eq!(full_contours, cached_contours);
            cache = next;
            if index >= WARMUP {
                full.push(full_time);
                cached.push(cached_time);
            }
        }
        println!(
            "{keys} features: full p50={}µs p95={}µs; cached p50={}µs p95={}µs",
            percentile(&mut full, 50),
            percentile(&mut full, 95),
            percentile(&mut cached, 50),
            percentile(&mut cached, 95)
        );
    }

    #[test]
    #[ignore = "Run with cargo test --release --lib -- --ignored --nocapture"]
    fn compare_100() {
        compare(100);
    }

    #[test]
    #[ignore = "Run with cargo test --release --lib -- --ignored --nocapture"]
    fn compare_200() {
        compare(200);
    }
}

/// Mark only the vertices where finishing actually reduced the requested size.
fn corner_markers_for(board_id: &str, locations: &[Vec2]) -> Vec<crate::model::FindingMarker> {
    if locations.is_empty() {
        return vec![];
    }
    let contours: Vec<_> = locations
        .iter()
        .map(|p| Contour {
            hole: false,
            points: vec![
                Vec2 {
                    x: p.x - 0.8,
                    y: p.y - 0.8,
                },
                Vec2 {
                    x: p.x + 0.8,
                    y: p.y - 0.8,
                },
                Vec2 {
                    x: p.x + 0.8,
                    y: p.y + 0.8,
                },
                Vec2 {
                    x: p.x - 0.8,
                    y: p.y + 0.8,
                },
            ],
        })
        .collect();
    [
        "outline:corners:fitted".to_owned(),
        format!("board:{board_id}:feature:outline:corners:fitted"),
    ]
    .into_iter()
    .map(|finding_id| crate::model::FindingMarker {
        finding_id,
        board_id: board_id.to_owned(),
        contours: contours.clone(),
    })
    .collect()
}
