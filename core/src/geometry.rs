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
    if let OutlineFeature::Polygon { points, .. } = feature {
        automatic::check_persisted_path_size(points.len())?;
    }
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
            // An authored bridge across an axis opts this feature into one connected
            // envelope there. Automatic bridges still respect every other split.
            for connection in connections {
                let mut minimum_x = f64::INFINITY;
                let mut maximum_x = f64::NEG_INFINITY;
                for control in &connection.points {
                    let point = crate::outline_controls::point(doc, control)?;
                    minimum_x = minimum_x.min(point.x);
                    maximum_x = maximum_x.max(point.x);
                }
                axes.retain(|axis| !(minimum_x < *axis && *axis < maximum_x));
            }
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
                let x = split_x(doc, part)?;
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
                // Ticket 12: anchored paths compose at the evaluated board pose.
                let pose = crate::matrix::layout::evaluated_part_pose(doc, part);
                point(crate::outline_controls::world(
                    Vec2 { x: p[0], y: p[1] },
                    &pose,
                    part.side.clone(),
                ))
            })
            .collect();
    }
    let simple = if matches!(feature, OutlineFeature::Polygon { .. }) {
        automatic::persisted_simple(&path)?
    } else {
        automatic::simple(&path)
    };
    if !simple {
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

/// A linked refinement whose current projection does not resolve.
///
/// The link stays active: the refinement keeps its settings and choices, the board
/// shows its last valid projection, and a located finding names the board, version and
/// cause. It is not manufactured into a valid current result, so it blocks the
/// affected exports while saving and further editing remain available.
pub(crate) struct LinkedProblem {
    pub version_id: String,
    pub version_name: String,
    pub message: String,
}

impl LinkedProblem {
    pub fn finding_id(&self, board_id: &str) -> String {
        format!("board:{board_id}:outline:linked:{}", self.version_id)
    }

    pub fn finding(&self, board_id: &str) -> Finding {
        Finding {
            id: self.finding_id(board_id),
            severity: Severity::Error,
            scope: Scope::Pcb,
            message: format!(
                "Linked refinement {} cannot resolve from the board's current components: {}. Showing its last valid shape; the refinement stays linked and exports stay blocked until it resolves.",
                self.version_name, self.message
            ),
            target_ids: vec![board_id.to_owned(), self.version_id.clone()],
        }
    }
}

/// Compose one board, keeping the link and falling back to the last valid projection
/// when a linked refinement cannot resolve from the current components.
fn board_composition(
    doc: &ProjectDoc,
    cache: &OutlineCache,
    board: &crate::model::Board,
) -> (ComposedOutline, Option<LinkedProblem>) {
    let result = compose(
        crate::outline_versions::features(doc, board).iter(),
        cache,
        crate::outline_versions::settings(doc, &board.id),
    );
    let Some(version) = crate::outline_versions::active_version(doc, &board.id) else {
        return (result, None);
    };
    if version.ownership.is_fixed() {
        return (result, None);
    }
    // Only an outline-composition failure is a resolution failure. Clearance,
    // support and overhang findings come from the separate fabrication checks and
    // must report the *current* projection instead of pinning a stale one.
    let failure = result
        .findings
        .iter()
        .find(|finding| {
            finding.severity == Severity::Error && matches!(finding.scope, Scope::Outline)
        })
        .map(|finding| finding.message.clone())
        .or_else(|| {
            result
                .contours
                .is_empty()
                .then(|| "the current components resolve to no board area".to_owned())
        });
    let Some(message) = failure else {
        return (result, None);
    };
    let problem = LinkedProblem {
        version_id: version.id.clone(),
        version_name: version.name.clone(),
        message,
    };
    let mut last_valid = cache.clone();
    for feature in &version.geometry.features {
        let geometry = feature_geometry(doc, feature);
        last_valid
            .paths
            .insert(feature.id().into(), (feature.clone(), geometry));
    }
    let fallback = compose(
        version.geometry.features.iter(),
        &last_valid,
        Some(&version.geometry.settings),
    );
    (fallback, Some(problem))
}

/// Recompute every linked alternative and report its failures without letting them
/// block the selected outline or another board (ADR-0001/0002).
fn alternative_findings(doc: &ProjectDoc) -> Vec<Finding> {
    let mut findings = vec![];
    for board in &doc.boards {
        let Some(state) = doc
            .board_outlines
            .iter()
            .find(|state| state.board_id == board.id)
        else {
            continue;
        };
        for version in &state.versions {
            if version.ownership.is_fixed()
                || state.active_version_id.as_deref() == Some(version.id.as_str())
            {
                continue;
            }
            let features = crate::outline_versions::resolved_features(doc, &board.id, version);
            let failure = features
                .iter()
                .find_map(|feature| feature_geometry(doc, feature).err())
                .or_else(|| {
                    features
                        .is_empty()
                        .then(|| "it has no boundary features".to_owned())
                });
            let Some(message) = failure else {
                continue;
            };
            findings.push(Finding {
                id: format!("board:{}:outline:linked:{}", board.id, version.id),
                severity: Severity::Error,
                scope: Scope::Pcb,
                message: format!(
                    "Linked refinement {} cannot resolve from the board's current components: {}. It is not selected, so it does not block current output.",
                    version.name, message
                ),
                target_ids: vec![board.id.clone(), version.id.clone()],
            });
        }
    }
    findings
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
    let mut features: Vec<OutlineFeature> = doc.outline.clone();
    for board in &doc.boards {
        if crate::outline_versions::active_snapshot(doc, &board.id).is_some() {
            features.extend(crate::outline_versions::features(doc, board));
        }
    }
    for feature in features {
        let paths = previous
            .filter(|old| old.layouts == doc.layouts)
            .and_then(|old| old.paths.get(feature.id()))
            .filter(|(old_feature, _)| old_feature == &feature && !depends_on(&feature, moved))
            .map(|(_, paths)| paths.clone())
            .unwrap_or_else(|| feature_geometry(doc, &feature));
        cache
            .paths
            .insert(feature.id().into(), (feature.clone(), paths));
    }
    let (contours, mut findings) = if doc
        .boards
        .iter()
        .any(|board| crate::outline_versions::active_snapshot(doc, &board.id).is_some())
    {
        let mut contours = vec![];
        let mut findings = vec![];
        for board in &doc.boards {
            let (result, problem) = board_composition(doc, &cache, board);
            if let Some(problem) = &problem {
                findings.push(problem.finding(&board.id));
            }
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
    findings.extend(alternative_findings(doc));
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
            // Ticket 12: anchored islands center on the evaluated board pose.
            let pose = crate::matrix::layout::evaluated_part_pose(doc, part);
            crate::outline_controls::world(*center, &pose, part.side.clone())
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

/// Source contours of an arbitrary feature set, resolving geometry the accepted cache
/// does not hold (an inactive version's current projection).
///
/// Used by settlement to compare a reviewed projection with the current one: the
/// composition is the same one the board path uses, so a reviewed shape and its later
/// recomputation are comparable without exposing the private cache.
pub(crate) fn feature_source_contours(
    doc: &ProjectDoc,
    cache: &OutlineCache,
    features: &[OutlineFeature],
) -> Vec<Contour> {
    let mut local = cache.clone();
    for feature in features {
        local.paths.insert(
            feature.id().into(),
            (feature.clone(), feature_geometry(doc, feature)),
        );
    }
    compose(features.iter(), &local, None).source
}
pub(crate) fn valid_polygon(points: &[Vec2]) -> bool {
    if automatic::check_persisted_path_size(points.len()).is_err() {
        return false;
    }
    automatic::persisted_simple(&points.iter().copied().map(point).collect()).unwrap_or(false)
}

pub(crate) fn board_outline_scenes(
    doc: &ProjectDoc,
    cache: &OutlineCache,
) -> Vec<BoardOutlineScene> {
    doc.boards
        .iter()
        .map(|board| {
            let (result, _) = board_composition(doc, cache, board);
            let bridges = match crate::outline_versions::active_version(doc, &board.id) {
                // A linked refinement's material bridges follow its current projection.
                Some(version) if !version.ownership.is_fixed() => result.bridges.clone(),
                Some(_) => crate::outline_versions::active_snapshot(doc, &board.id)
                    .map(|snapshot| snapshot.bridges.clone())
                    .unwrap_or_else(|| result.bridges.clone()),
                None => result.bridges.clone(),
            };
            BoardOutlineScene {
                board_id: board.id.clone(),
                source_contours: result.source,
                bridges,
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
        let (result, problem) = board_composition(doc, cache, board);
        markers.extend(corner_markers_for(&board.id, &result.corner_locations));
        if let Some(problem) = &problem {
            // The last valid shape is what the board displays; the finding identifies
            // the board and version it belongs to and locates it on the workbench.
            markers.push(crate::model::FindingMarker {
                finding_id: problem.finding_id(&board.id),
                board_id: board.id.clone(),
                contours: result
                    .contours
                    .iter()
                    .filter(|contour| !contour.hole)
                    .cloned()
                    .collect(),
            });
        }
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

/// The split coordinate of a part: the origin of the unanchored root of the matrix that owns
/// it. A part outside any matrix has only its own pose.
fn split_x(doc: &ProjectDoc, part: &Part) -> Result<f64, String> {
    let Some(owner) = doc
        .matrices
        .iter()
        .find(|matrix| matrix.part_ids.contains(&part.id))
    else {
        // Ticket 12: independent parts group by their evaluated board pose,
        // so a half-frame part stays on its own half's island.
        return Ok(crate::matrix::layout::evaluated_part_pose(doc, part).at.x);
    };
    let mut root = crate::matrix::evaluation::logical_matrix(&owner.id, doc)?;
    for _ in 0..=doc.matrices.len() {
        let Some(anchor) = root.anchor.as_ref() else {
            // Half-frame origins are relative to the half offset; compare in
            // board space so each half still groups on its own side.
            return Ok(match crate::matrix::layout::half_offset(doc, &root.id) {
                Some(offset) => root.origin.x + offset.x,
                None => root.origin.x,
            });
        };
        root = crate::matrix::evaluation::logical_matrix(&anchor.matrix_id, doc)?;
    }
    Err(format!("Matrix anchors of {} form a cycle", owner.id))
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
            {"id":"left","name":"Left","boardId":"board","matrixIds":["left-matrix"],"partIds":["key-0"]},
            {"id":"right","name":"Right","boardId":"board","matrixIds":["right-matrix"],"partIds":["key-1"],"mirrorLink":{"sourceId":"left","axisX":20.0}}
        ])).unwrap();
        let shapes = feature_geometry(&doc, &doc.outline[0]).unwrap().shapes;
        assert_eq!(
            shapes.len(),
            2,
            "split halves must retain separate outlines"
        );
    }

    #[test]
    fn oversized_authored_outline_stays_saved_and_reports_a_finding() {
        let mut doc = fixture(1);
        let points = (0..=automatic::MAX_PERSISTED_OUTLINE_POINTS)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64
                    / (automatic::MAX_PERSISTED_OUTLINE_POINTS + 1) as f64;
                Vec2 {
                    x: angle.cos() * 20.0,
                    y: angle.sin() * 20.0,
                }
            })
            .collect();
        doc.outline[0] = OutlineFeature::Polygon {
            anchor_part_id: None,
            id: "large-outline".into(),
            points,
            operation: crate::model::Operation::Add,
        };

        let (cache, _, findings) = outlines(&doc, None, &[]);

        assert!(matches!(
            &doc.outline[0],
            OutlineFeature::Polygon { points, .. }
                if points.len() == automatic::MAX_PERSISTED_OUTLINE_POINTS + 1
        ));
        assert!(findings.iter().any(|finding| {
            finding.id == "outline:large-outline:invalid" && finding.message.contains("1024 points")
        }));
        assert!(cache.paths["large-outline"].1.is_err());
    }

    const SAMPLES: usize = 100;
    const WARMUP: usize = 10;
    const PITCH: f64 = 19.05;

    fn fixture(keys: usize) -> ProjectDoc {
        let mut doc = ProjectDoc::empty("bench", "Benchmark");
        doc.definitions.push(PartDefinition {
            hardware_profile: None,
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
