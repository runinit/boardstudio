//! Fabrication checks consume only the active perimeter, independently per board.
use crate::{geometry, model::*, outline_versions};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
    mesh::float::{
        outline::offset::OutlineOffset,
        style::{LineJoin, OutlineStyle},
    },
};

type Path = Vec<[f64; 2]>;

// withClosureClearance projects case bosses as owned NPTH cutters. They remove
// material where a closure crosses the PCB, including intentional edge notches;
// their courtyard is a clearance region, not a component body requiring support.
fn closure_clearance(part: &Part, definition: &PartDefinition) -> bool {
    part.id.starts_with("case-closure/")
        && definition.id.starts_with("assembly-closure/definition/")
        && part
            .outline
            .as_ref()
            .is_some_and(|outline| outline.excluded)
        && definition
            .generator
            .as_ref()
            .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
        && !definition.pads.is_empty()
        && definition.terminals.is_empty()
        && definition
            .pads
            .iter()
            .all(|pad| pad.plated == Some(false) && pad.number.is_empty() && pad.drill.is_some())
}

struct Material {
    paths: Vec<Path>,
    edge_bounds: Vec<[f64; 4]>,
}
impl Material {
    fn new(contours: &[Contour]) -> Self {
        let paths = paths(contours);
        let edge_bounds = paths
            .iter()
            .flat_map(|path| {
                path.iter()
                    .zip(path.iter().cycle().skip(1))
                    .take(path.len())
                    .map(|(a, b)| {
                        [
                            a[0].min(b[0]),
                            a[1].min(b[1]),
                            a[0].max(b[0]),
                            a[1].max(b[1]),
                        ]
                    })
            })
            .collect();
        Self { paths, edge_bounds }
    }
    fn contains(&self, p: [f64; 2]) -> bool {
        self.paths
            .iter()
            .flat_map(|path| {
                path.iter()
                    .zip(path.iter().cycle().skip(1))
                    .take(path.len())
            })
            .fold(false, |inside, (a, b)| {
                inside
                    ^ ((a[1] > p[1]) != (b[1] > p[1])
                        && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0])
            })
    }
}
fn paths(contours: &[Contour]) -> Vec<Path> {
    contours
        .iter()
        .map(|contour| contour.points.iter().map(|p| [p.x, p.y]).collect())
        .collect()
}
fn contour(path: Path) -> Contour {
    Contour {
        points: path
            .into_iter()
            .map(|p| Vec2 { x: p[0], y: p[1] })
            .collect(),
        hole: false,
    }
}
fn pad_path(at: Vec2, size: Vec2, shape: &PadShape, rotation: f64) -> Vec<Vec2> {
    let hx = size.x / 2.0;
    let hy = size.y / 2.0;
    let mut points = match shape {
        PadShape::Circle => (0..64)
            .map(|i| {
                let angle = i as f64 * std::f64::consts::TAU / 64.0;
                Vec2 {
                    x: hx * angle.cos(),
                    y: hy * angle.sin(),
                }
            })
            .collect(),
        PadShape::Oval => {
            let r = hx.min(hy);
            (0..64)
                .map(|i| {
                    let angle = i as f64 * std::f64::consts::TAU / 64.0;
                    let x = angle.cos();
                    let y = angle.sin();
                    Vec2 {
                        x: x * r + x.signum() * (hx - r),
                        y: y * r + y.signum() * (hy - r),
                    }
                })
                .collect()
        }
        // Persisted pad projections do not carry a roundrect ratio; use its conservative envelope.
        _ => vec![
            Vec2 { x: -hx, y: -hy },
            Vec2 { x: hx, y: -hy },
            Vec2 { x: hx, y: hy },
            Vec2 { x: -hx, y: hy },
        ],
    };
    let (sin, cos) = rotation.to_radians().sin_cos();
    for p in &mut points {
        *p = Vec2 {
            x: at.x + p.x * cos - p.y * sin,
            y: at.y + p.x * sin + p.y * cos,
        };
    }
    points
}
fn world(part: &Part, points: &[Vec2]) -> Path {
    let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
    points
        .iter()
        .map(|p| {
            let x = if part.side == Side::Back { -p.x } else { p.x };
            [
                part.pose.at.x + x * cos - p.y * sin,
                part.pose.at.y + x * sin + p.y * cos,
            ]
        })
        .collect()
}

fn support_paths(
    definition: &PartDefinition,
    native: Option<&MechanicalGeometry>,
) -> Vec<Vec<Vec2>> {
    let mut paths = vec![];
    for pad in &definition.pads {
        let rotation = pad.rotation.unwrap_or_default();
        paths.push(pad_path(pad.at, pad.size, &pad.shape, rotation));
        if let Some(drill) = pad.drill {
            paths.push(pad_path(
                pad.at,
                Vec2 { x: drill, y: drill },
                &PadShape::Circle,
                rotation,
            ));
        }
    }
    for hole in definition
        .mechanical_profile
        .iter()
        .flat_map(|profile| profile.pcb_holes.iter().flatten())
    {
        paths.push(pad_path(
            hole.at,
            Vec2 {
                x: hole.diameter,
                y: hole.diameter,
            },
            &PadShape::Circle,
            0.0,
        ));
    }
    // Native source carries slot dimensions/offsets absent from the Pad projection.
    for primitive in native.into_iter().flat_map(|geometry| &geometry.primitives) {
        if let MechanicalShape::Drill {
            at,
            size,
            offset,
            rotation_degrees,
            shape,
            ..
        } = &primitive.geometry
        {
            let (sin, cos) = rotation_degrees.to_radians().sin_cos();
            let center = Vec2 {
                x: at.x + offset.x * cos - offset.y * sin,
                y: at.y + offset.x * sin + offset.y * cos,
            };
            let shape = if *shape == MechanicalDrillShape::Oval {
                PadShape::Oval
            } else {
                PadShape::Circle
            };
            paths.push(pad_path(center, *size, &shape, *rotation_degrees));
        }
    }
    paths
}
fn area(path: &Path) -> f64 {
    path.iter()
        .zip(path.iter().cycle().skip(1))
        .take(path.len())
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        .abs()
        / 2.0
}
fn missing(path: &Path, material: &Material, clearance: f64) -> bool {
    if path.len() < 3
        || path
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        || area(path) < 1e-9
    {
        return false;
    }
    let bounds = path.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        },
    );
    // If no perimeter/hole edge reaches this expanded bounding box, membership
    // is constant over the entire required region. Only boundary cases need clipping.
    let intersects = material.edge_bounds.iter().any(|edge| {
        edge[2] >= bounds[0] - clearance
            && edge[0] <= bounds[2] + clearance
            && edge[3] >= bounds[1] - clearance
            && edge[1] <= bounds[3] + clearance
    });
    if !intersects {
        return !material.contains(path[0]);
    }
    let required = if clearance > 0.0 {
        match path.outline_fixed_scale(
            &OutlineStyle::new(clearance).line_join(LineJoin::Miter(0.1)),
            1000.0,
        ) {
            Ok(shapes) => shapes,
            Err(_) => return true,
        }
    } else {
        vec![vec![path.clone()]]
    };
    required
        .overlay(&material.paths, OverlayRule::Difference, FillRule::EvenOdd)
        .iter()
        .flatten()
        .map(area)
        .sum::<f64>()
        > 0.000_001
}

pub(crate) fn validate(
    doc: &ProjectDoc,
    boards: &[BoardContours],
    sources: &[BoardOutlineScene],
) -> (Vec<Finding>, Vec<FindingMarker>) {
    let mut findings = vec![];
    let mut markers = vec![];
    let native = doc
        .definitions
        .iter()
        .map(|definition| {
            let source = definition
                .mechanical_profile
                .as_ref()
                .and_then(|profile| profile.source_geometry.as_ref());
            let text = source.map(|source| source.text.as_str()).or_else(|| {
                definition
                    .kicad_source
                    .as_ref()
                    .map(|source| source.source.as_str())
            });
            let geometry = text.map(|text| {
                crate::artifact::mechanical_extract::extract(
                    text,
                    source
                        .map(|source| source.mappings.as_slice())
                        .unwrap_or_default(),
                )
            });
            (definition.id.as_str(), geometry)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    // Repeated components share their local pad/drill geometry. Build it once
    // per definition in this validation, then transform for each placement.
    let support = doc
        .definitions
        .iter()
        .map(|definition| {
            let geometry = native
                .get(definition.id.as_str())
                .and_then(|value| value.as_ref())
                .and_then(|value| value.as_ref().ok());
            (definition.id.as_str(), support_paths(definition, geometry))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for board in &doc.boards {
        let Some(resolved) = boards.iter().find(|item| item.board_id == board.id) else {
            continue;
        };
        let source = sources.iter().find(|item| item.board_id == board.id);
        let features = outline_versions::features(doc, board);
        let settings = outline_versions::settings(doc, &board.id)
            .cloned()
            .or_else(|| {
                features.iter().find_map(|feature| {
                    if let OutlineFeature::PartEnvelope { settings, .. } = feature {
                        Some(settings.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default();
        let repair = settings.repair.unwrap_or_default();
        let mut report = |suffix: String,
                          message: String,
                          target: Option<&str>,
                          regions: Vec<Contour>,
                          severity: Severity| {
            let id = format!("board:{}:outline:{suffix}", board.id);
            findings.push(Finding {
                id: id.clone(),
                severity,
                scope: Scope::Pcb,
                message,
                target_ids: std::iter::once(board.id.clone())
                    .chain(target.map(str::to_owned))
                    .collect(),
            });
            if !regions.is_empty() {
                markers.push(FindingMarker {
                    finding_id: id,
                    board_id: board.id.clone(),
                    contours: regions,
                });
            }
        };
        if [
            repair.maximum_gap_span,
            repair.minimum_connection_width,
            repair.edge_clearance,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            report("settings".into(), "Gap span, minimum connection width and edge clearance must be finite and nonnegative".into(), None, resolved.contours.clone(), Severity::Error);
            continue;
        }
        let generated = doc
            .board_outlines
            .iter()
            .find(|state| state.board_id == board.id)
            .is_none_or(|state| state.active_version_id.is_none());
        if generated
            && let Some((gap_id, message)) = outline_versions::protection_problem(doc, &board.id)
        {
            let regions = outline_versions::active_snapshot(doc, &board.id)
                .into_iter()
                .flat_map(|snapshot| &snapshot.protected_gaps)
                .filter(|gap| gap.id == gap_id)
                .map(|gap| Contour {
                    points: gap.points.iter().map(|p| p.at).collect(),
                    hole: false,
                })
                .collect();
            report(
                format!("keep-gap:{gap_id}"),
                format!(
                    "{message}. Last valid outline retained; remove Keep gap or restore its sources."
                ),
                None,
                regions,
                Severity::Error,
            );
        }
        let regions = resolved
            .contours
            .iter()
            .filter(|contour| !contour.hole)
            .count();
        let expected = outline_versions::active_snapshot(doc, &board.id)
            .map(|snapshot| snapshot.expected_regions as usize)
            .unwrap_or_else(|| {
                if doc
                    .layouts
                    .iter()
                    .any(|layout| layout.board_id == board.id && layout.mirror_link.is_some())
                {
                    2
                } else {
                    1
                }
            });
        if regions > expected {
            report(
                "disconnected".into(),
                "Outline contains an unintended disconnected region; reconnect or remove it".into(),
                None,
                resolved.contours.clone(),
                Severity::Error,
            );
        }
        let material = Material::new(&resolved.contours);
        if material.paths.is_empty() {
            continue;
        }
        for part in doc
            .parts
            .iter()
            .filter(|part| board.part_ids.contains(&part.id))
        {
            let Some(definition) = doc
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
            else {
                continue;
            };
            if definition.kind == PartKind::Utility || closure_clearance(part, definition) {
                continue;
            }
            let required: Vec<_> = support[definition.id.as_str()]
                .iter()
                .map(|path| world(part, path))
                .collect();
            if let Some(Some(Err(error))) = native.get(definition.id.as_str()) {
                report(
                    format!("support-source:{}", part.id),
                    format!(
                        "{}: native support geometry could not be read: {}",
                        part.reference, error.message
                    ),
                    Some(&part.id),
                    vec![contour(world(part, &definition.courtyard))],
                    Severity::Error,
                );
            }
            let outside: Vec<_> = required
                .iter()
                .filter(|path| missing(path, &material, repair.edge_clearance))
                .collect();
            if !outside.is_empty() {
                report(
                    format!("support:{}", part.id),
                    format!(
                        "{}: required pad or drill support lies outside the active outline or its {:.3} mm edge clearance",
                        part.reference, repair.edge_clearance
                    ),
                    Some(&part.id),
                    outside
                        .iter()
                        .map(|path| contour((*path).clone()))
                        .collect(),
                    Severity::Error,
                );
            }
            let body = world(part, &definition.courtyard);
            if definition.courtyard.len() >= 3 && !geometry::valid_polygon(&definition.courtyard) {
                report(
                    format!("body-source:{}", part.id),
                    format!("{}: courtyard support geometry is invalid", part.reference),
                    Some(&part.id),
                    vec![contour(body.clone())],
                    Severity::Error,
                );
            }
            if missing(&body, &material, 0.0) {
                let permitted = part
                    .outline
                    .as_ref()
                    .is_some_and(|outline| outline.allow_body_overhang);
                // Without pad/drill sources, the courtyard is the conservative required-support fallback.
                let blocker = !permitted;
                report(
                    format!("body:{}", part.id),
                    if required.is_empty() && blocker {
                        format!(
                            "{}: no pad/drill support geometry is available; the courtyard extends outside the outline",
                            part.reference
                        )
                    } else {
                        format!(
                            "{}: component body overhangs the outline{}",
                            part.reference,
                            if permitted {
                                " (allowed)"
                            } else {
                                "; verify mechanical fit"
                            }
                        )
                    },
                    Some(&part.id),
                    vec![contour(body)],
                    if blocker {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                );
            }
            if definition.kind == PartKind::Switch
                && let Some(size) = part.keycap.or(definition.keycap)
            {
                let keycap = world(part, &pad_path(Vec2::default(), size, &PadShape::Rect, 0.0));
                if missing(&keycap, &material, 0.0) {
                    report(
                        format!("keycap:{}", part.id),
                        format!("{}: keycap overhangs the outline", part.reference),
                        Some(&part.id),
                        vec![contour(keycap)],
                        Severity::Warning,
                    );
                }
            }
        }
        if repair.minimum_connection_width > 0.0 {
            if let Some(source) = source {
                for bridge in &source.bridges {
                    if bridge.width + 0.001 < repair.minimum_connection_width {
                        report(
                            format!("bridge:{}", bridge.id),
                            format!(
                                "Connection width {:.3} mm is below the {:.3} mm minimum",
                                bridge.width, repair.minimum_connection_width
                            ),
                            Some(&bridge.id),
                            vec![Contour {
                                points: bridge.points.clone(),
                                hole: false,
                            }],
                            Severity::Error,
                        );
                    }
                }
            }
            // Erosion detects physical bottlenecks, including manually edited necks.
            for (index, outer) in resolved
                .contours
                .iter()
                .filter(|contour| !contour.hole)
                .enumerate()
            {
                let outer_path = paths(std::slice::from_ref(outer));
                let local_holes: Vec<_> = resolved
                    .contours
                    .iter()
                    .filter(|contour| contour.hole)
                    .flat_map(|hole| paths(std::slice::from_ref(hole)))
                    .collect();
                let region =
                    outer_path.overlay(&local_holes, OverlayRule::Difference, FillRule::EvenOdd);
                if let Ok(cores) = region.outline_fixed_scale(
                    &OutlineStyle::new(-repair.minimum_connection_width / 2.0)
                        .line_join(LineJoin::Miter(0.1)),
                    1000.0,
                ) {
                    let substantial: Vec<_> = cores
                        .iter()
                        .filter(|shape| area(&shape[0]) > repair.minimum_connection_width.powi(2))
                        .collect();
                    if substantial.len() > 1 {
                        let mut regions = vec![];
                        for pair in substantial.windows(2) {
                            let (a, b) = pair[0][0]
                                .iter()
                                .flat_map(|a| pair[1][0].iter().map(move |b| (*a, *b)))
                                .min_by(|(a, b), (c, d)| {
                                    (a[0] - b[0])
                                        .hypot(a[1] - b[1])
                                        .total_cmp(&(c[0] - d[0]).hypot(c[1] - d[1]))
                                })
                                .unwrap();
                            let length = (b[0] - a[0]).hypot(b[1] - a[1]).max(0.001);
                            let normal = [
                                -(b[1] - a[1]) / length * repair.minimum_connection_width,
                                (b[0] - a[0]) / length * repair.minimum_connection_width,
                            ];
                            regions.push(contour(vec![
                                [a[0] - normal[0], a[1] - normal[1]],
                                [b[0] - normal[0], b[1] - normal[1]],
                                [b[0] + normal[0], b[1] + normal[1]],
                                [a[0] + normal[0], a[1] + normal[1]],
                            ]));
                        }
                        report(
                            format!("connection:{index}"),
                            format!(
                                "A connection is narrower than the configured {:.3} mm minimum",
                                repair.minimum_connection_width
                            ),
                            None,
                            regions,
                            Severity::Error,
                        );
                    }
                }
            }
        }
    }
    (findings, markers)
}

pub(crate) fn feature_markers(doc: &ProjectDoc, findings: &[Finding]) -> Vec<FindingMarker> {
    let mut markers = vec![];
    for board in &doc.boards {
        for feature in outline_versions::features(doc, board) {
            let points = match feature {
                OutlineFeature::Polygon {
                    points,
                    anchor_part_id,
                    ..
                } => {
                    if let Some(part) = anchor_part_id
                        .as_ref()
                        .and_then(|id| doc.parts.iter().find(|part| &part.id == id))
                    {
                        contour(world(part, points)).points
                    } else {
                        points.clone()
                    }
                }
                OutlineFeature::Rect {
                    center,
                    size,
                    rotation,
                    anchor_part_id,
                    ..
                } => {
                    let points = pad_path(
                        *center,
                        *size,
                        &PadShape::Rect,
                        rotation.unwrap_or_default(),
                    );
                    if let Some(part) = anchor_part_id
                        .as_ref()
                        .and_then(|id| doc.parts.iter().find(|part| &part.id == id))
                    {
                        contour(world(part, &points)).points
                    } else {
                        points
                    }
                }
                _ => vec![],
            };
            if points.is_empty() {
                continue;
            }
            for finding in findings.iter().filter(|finding| {
                finding.severity == Severity::Error
                    && finding.target_ids.iter().any(|id| id == feature.id())
            }) {
                markers.push(FindingMarker {
                    finding_id: finding.id.clone(),
                    board_id: board.id.clone(),
                    contours: vec![Contour {
                        points: points.clone(),
                        hole: false,
                    }],
                });
            }
        }
    }
    for board in &doc.boards {
        for finding in findings.iter().filter(|finding| {
            !finding.id.starts_with("keycaps/") && !finding.id.ends_with("outline:corners:fitted")
        }) {
            let contours: Vec<_> = doc
                .parts
                .iter()
                .filter(|part| {
                    board.part_ids.contains(&part.id) && finding.target_ids.contains(&part.id)
                })
                .filter_map(|part| {
                    doc.definitions
                        .iter()
                        .find(|definition| definition.id == part.definition_id)
                        .filter(|definition| !definition.courtyard.is_empty())
                        .map(|definition| contour(world(part, &definition.courtyard)))
                })
                .collect();
            if !contours.is_empty() {
                markers.push(FindingMarker {
                    finding_id: finding.id.clone(),
                    board_id: board.id.clone(),
                    contours,
                });
            }
        }
    }
    markers
}
