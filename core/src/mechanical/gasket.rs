//! Discrete plate supports and their matching case/retainer geometry.
use super::*;
mod internal;

const SUPPORT_COUNT: usize = 6;
const RETAINER_THICKNESS: f64 = 3.0;

#[derive(Clone)]
struct Candidate {
    at: Vec2,
    tangent: Vec2,
    normal: Vec2,
    anchor: f64,
}
struct Region {
    id: String,
    key: String,
    candidates: Vec<Candidate>,
    tracks: Vec<MechanicalGasketTrack>,
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}
fn scale(a: Vec2, s: f64) -> Vec2 {
    Vec2 {
        x: a.x * s,
        y: a.y * s,
    }
}
fn distance(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn rectangle(c: &Candidate, length: f64, inside: f64, outside: f64) -> Vec<Vec2> {
    [
        (-length / 2., -inside),
        (length / 2., -inside),
        (length / 2., outside),
        (-length / 2., outside),
    ]
    .map(|(t, n)| add(c.at, add(scale(c.tangent, t), scale(c.normal, n))))
    .to_vec()
}
fn paths(contours: &[Contour]) -> Vec<FloatPath> {
    contours
        .iter()
        .map(|c| c.points.iter().map(|p| [p.x, p.y]).collect())
        .collect()
}
fn overlaps(a: &[Vec2], b: &[Vec2]) -> bool {
    let bounds = |points: &[Vec2]| {
        points.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
        )
    };
    let x = bounds(a);
    let y = bounds(b);
    if x[0] >= y[2] || x[2] <= y[0] || x[1] >= y[3] || x[3] <= y[1] {
        return false;
    }
    let a = a.iter().map(|p| [p.x, p.y]).collect::<FloatPath>();
    let b = b.iter().map(|p| [p.x, p.y]).collect::<FloatPath>();
    !a.overlay(&b, OverlayRule::Intersect, FillRule::EvenOdd)
        .is_empty()
}
fn union(contours: &[Contour], extras: &[Vec<Vec2>]) -> Vec<Contour> {
    let mut shapes = paths(
        &contours
            .iter()
            .filter(|c| !c.hole)
            .cloned()
            .collect::<Vec<_>>(),
    );
    for extra in extras {
        let p = extra.iter().map(|p| [p.x, p.y]).collect::<FloatPath>();
        shapes = shapes
            .overlay(&p, OverlayRule::Union, FillRule::EvenOdd)
            .into_iter()
            .flatten()
            .collect();
    }
    shapes
        .into_iter()
        .map(|p| Contour {
            hole: false,
            points: p.into_iter().map(|p| Vec2 { x: p[0], y: p[1] }).collect(),
        })
        .chain(contours.iter().filter(|c| c.hole).cloned())
        .collect()
}
fn simple_ring(contour: &Contour) -> Contour {
    let mut points = contour.points.clone();
    if points.first() == points.last() {
        points.pop();
    }
    loop {
        let before = points.len();
        points = (0..before)
            .filter_map(|i| {
                let a = points[(i + before - 1) % before];
                let b = points[i];
                let c = points[(i + 1) % before];
                (((b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x)).abs() > 0.00001)
                    .then_some(b)
            })
            .collect();
        if points.len() == before || points.len() < 3 {
            break;
        }
    }
    let area = points
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = points[(i + 1) % points.len()];
            a.x * b.y - a.y * b.x
        })
        .sum::<f64>();
    if area < 0. {
        points.reverse();
    }
    if let Some((index, _)) = points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)))
    {
        points.rotate_left(index);
    }
    Contour {
        hole: false,
        points,
    }
}
fn make_body(
    config: &MechanicalConfiguration,
    revision: u64,
    id: &str,
    z: f64,
    thickness: f64,
    contours: Vec<Contour>,
) -> CaseIR {
    CaseIR {
        revision,
        contours,
        body: CaseBody {
            features: None,
            id: id.into(),
            name: id.replace(':', " "),
            board_id: config.board_id.clone(),
            kind: CaseKind::Plate,
            thickness,
            clearance: 0.,
            material_id: None,
            z: Some(z),
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            gasket: None,
            openings: None,
        },
    }
}
fn region_id(document: &ProjectDoc, board_id: &str, ring: &Contour, index: usize) -> String {
    document
        .layouts
        .iter()
        .filter(|layout| layout.board_id == board_id)
        .find(|layout| {
            document.parts.iter().any(|part| {
                (layout.part_ids.contains(&part.id)
                    || part
                        .id
                        .starts_with(&format!("matrix/{}/", layout.matrix_id)))
                    && inside_outline(part.pose.at, std::slice::from_ref(ring))
            })
        })
        .map_or_else(|| format!("outline-{index}"), |layout| layout.id.clone())
}
fn outline_key(ring: &Contour) -> String {
    let origin = ring.points[0];
    ring.points
        .iter()
        .map(|p| format!("{:.3},{:.3}", p.x - origin.x, p.y - origin.y))
        .collect::<Vec<_>>()
        .join(";")
}

fn on_track(region: &Region, anchor: f64) -> Option<Candidate> {
    let track = region
        .tracks
        .iter()
        .find(|track| anchor >= track.start_anchor && anchor <= track.end_anchor)?;
    let t = (anchor - track.start_anchor) / (track.end_anchor - track.start_anchor);
    let delta = Vec2 {
        x: track.end.x - track.start.x,
        y: track.end.y - track.start.y,
    };
    let length = distance(track.start, track.end);
    let tangent = scale(delta, 1. / length);
    Some(Candidate {
        at: add(track.start, scale(delta, t)),
        tangent,
        normal: Vec2 {
            x: tangent.y,
            y: -tangent.x,
        },
        anchor,
    })
}

pub(super) fn generate(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    result: &mut MechanicalAssembly,
    component_volumes: &[(String, CaseOpening)],
) -> Result<(), String> {
    if let Some(settings) = &config.internal_gasket {
        return internal::generate(document, config, settings, result, component_volumes);
    }
    let defaults = MechanicalGasketLayout {
        adhesive_thickness: None,
        minimum_foam_thickness: None,
        preset_id: None,
        material: None,
        length: 12.,
        width: 3.,
        thickness: 2.,
        compression: 0.15,
        supports: vec![],
    };
    let settings = config.gasket_layout.as_ref().unwrap_or(&defaults);
    if [settings.length, settings.width, settings.thickness]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
        || !settings.compression.is_finite()
        || !(0.0..0.5).contains(&settings.compression)
    {
        return Err(
            "Gasket dimensions must be positive; nominal compression must be between 0% and 50%."
                .into(),
        );
    }
    let compressed = settings.thickness * (1. - settings.compression);
    let travel = config.gasket_travel.unwrap_or(0.3);
    if travel <= 0. || travel >= compressed || travel >= config.plate_to_pcb {
        return Err(
            "Gasket travel must fit within the compressed strip and switch clearance.".into(),
        );
    }
    let original = result.plate_contours.clone();
    let mut rings = original
        .iter()
        .filter(|c| !c.hole)
        .map(simple_ring)
        .collect::<Vec<_>>();
    rings.sort_by(|a, b| {
        a.points[0]
            .x
            .total_cmp(&b.points[0].x)
            .then(a.points[0].y.total_cmp(&b.points[0].y))
    });
    let tab_depth = settings.width + 1.;
    let bolt_distance = tab_depth + config.clearance + config.wall_thickness + 2.5;
    let lobe_depth = bolt_distance + 2.5 + config.wall_thickness;
    let lobe_length = (settings.length + 2.).max(5. + 2. * config.wall_thickness);
    let obstacle_rings = expanded_outline(
        &rings,
        config.clearance + config.wall_thickness,
        document.revision,
    )?;
    let mut obstacles = config
        .openings
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|opening| opening.points)
        .collect::<Vec<_>>();
    for body in &result.case.bodies {
        for opening in body.body.openings.iter().flatten() {
            obstacles.push(opening.points.clone());
        }
    }
    for mount in config
        .mounts
        .iter()
        .chain(config.closure_mounts.iter().flatten())
    {
        let r = mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.
            + config.clearance
            + config.wall_thickness;
        obstacles.push(
            (0..16)
                .map(|i| {
                    let angle = i as f64 * std::f64::consts::TAU / 16.;
                    Vec2 {
                        x: mount.at.x + r * angle.cos(),
                        y: mount.at.y + r * angle.sin(),
                    }
                })
                .collect(),
        );
    }
    let mut regions = Vec::new();
    for (index, ring) in rings.iter().enumerate() {
        let id = region_id(document, &config.board_id, ring, index);
        let key = outline_key(ring);
        let lengths = ring
            .points
            .iter()
            .enumerate()
            .map(|(i, a)| distance(*a, ring.points[(i + 1) % ring.points.len()]))
            .collect::<Vec<_>>();
        let perimeter = lengths.iter().sum::<f64>();
        let mut travelled = 0.;
        let mut candidates = Vec::new();
        let mut tracks = Vec::new();
        for (edge, &length) in lengths.iter().enumerate() {
            let a = ring.points[edge];
            let b = ring.points[(edge + 1) % ring.points.len()];
            let tangent = Vec2 {
                x: (b.x - a.x) / length,
                y: (b.y - a.y) / length,
            };
            let normal = Vec2 {
                x: tangent.y,
                y: -tangent.x,
            };
            let margin = lobe_length / 2. + config.clearance + 1.;
            let mut run: Option<Candidate> = None;
            let mut previous: Option<Candidate> = None;
            let steps = ((length - 2. * margin).max(0.) / 1.).floor() as usize;
            for step in 0..=steps + 1 {
                let t = margin + step as f64;
                let candidate = Candidate {
                    at: add(a, scale(tangent, t)),
                    tangent,
                    normal,
                    anchor: (travelled + t) / perimeter,
                };
                let footprint = rectangle(&candidate, lobe_length, 0.5, lobe_depth);
                let valid = t <= length - margin
                    && !overlaps(
                        &rectangle(&candidate, lobe_length, -0.1, lobe_depth),
                        &ring.points,
                    )
                    && !obstacle_rings
                        .iter()
                        .filter(|other| {
                            !inside_outline(ring.points[0], std::slice::from_ref(other))
                        })
                        .any(|other| overlaps(&footprint, &other.points))
                    && !original
                        .iter()
                        .filter(|c| c.hole)
                        .any(|hole| overlaps(&footprint, &hole.points))
                    && !obstacles
                        .iter()
                        .any(|opening| overlaps(&footprint, opening));
                if valid {
                    if run.is_none() {
                        run = Some(candidate.clone());
                    }
                    previous = Some(candidate.clone());
                    candidates.push(candidate);
                } else if let (Some(start), Some(end)) = (run.take(), previous.take()) {
                    if distance(start.at, end.at) > 0.1 {
                        tracks.push(MechanicalGasketTrack {
                            region_id: id.clone(),
                            start: start.at,
                            end: end.at,
                            start_anchor: start.anchor,
                            end_anchor: end.anchor,
                        });
                    }
                }
            }
            travelled += length;
        }
        if candidates.is_empty() {
            return Err(format!(
                "No clear gasket/closure positions on {id}; increase perimeter space or move conflicting openings."
            ));
        }
        regions.push(Region {
            id,
            key,
            candidates,
            tracks,
        });
    }
    let mut chosen: Vec<(String, Candidate)> = Vec::new();
    for region in &regions {
        let mut supports = Vec::new();
        for slot in 0..SUPPORT_COUNT {
            let id = format!("{}:{slot}", region.id);
            let saved = settings.supports.iter().find(|a| a.id == id);
            if let Some(saved) = saved {
                if saved.outline_key != region.key || saved.region_id != region.id {
                    return Err(format!(
                        "Gasket {id} was moved manually and its outline changed. Reset its positions or restore the outline."
                    ));
                }
            }
            let ideal = saved.map_or((slot as f64 + 0.5) / SUPPORT_COUNT as f64, |a| a.anchor);
            let saved_candidate = saved.and_then(|saved| on_track(region, saved.anchor));
            if saved.is_some() && saved_candidate.is_none() {
                return Err(format!(
                    "Saved gasket {id} is outside its valid perimeter track. Reset its position."
                ));
            }
            let candidates = saved_candidate
                .as_ref()
                .map_or(region.candidates.as_slice(), std::slice::from_ref);
            let candidate = candidates
                .iter()
                .filter(|c| {
                    !chosen.iter().any(|(_, other)| {
                        overlaps(
                            &rectangle(c, lobe_length + 1., 0.5, lobe_depth + 0.5),
                            &rectangle(other, lobe_length + 1., 0.5, lobe_depth + 0.5),
                        )
                    })
                })
                .min_by(|a, b| {
                    let diff = |x: f64| {
                        let d = (x - ideal).abs();
                        d.min(1. - d)
                    };
                    diff(a.anchor).total_cmp(&diff(b.anchor))
                })
                .cloned()
                .ok_or_else(|| {
                    format!("Cannot fit six separated gasket supports on {}.", region.id)
                })?;
            if saved.is_some() && (candidate.anchor - ideal).abs() > 0.005 {
                return Err(format!(
                    "Saved gasket {id} collides with another support or opening. Move it or reset positions."
                ));
            }
            chosen.push((id.clone(), candidate.clone()));
            supports.push(MechanicalGasketSupport {
                placement: None,
                id,
                region_id: region.id.clone(),
                outline_key: region.key.clone(),
                anchor: candidate.anchor,
                at: candidate.at,
                tangent: candidate.tangent,
                normal: candidate.normal,
                length: settings.length,
                width: settings.width,
                z: config.plate_to_pcb - compressed,
                thickness: compressed,
                pair_id: None,
                mirror_axis: None,
                unlinked: saved.is_some_and(|a| a.unlinked),
            });
        }
        result.gasket_supports.extend(supports);
        result.gasket_tracks.extend(region.tracks.clone());
    }
    // Linked layouts explicitly own the reflection relationship; geometry alone never creates one.
    for layout in &document.layouts {
        let Some(link) = &layout.mirror_link else {
            continue;
        };
        for slot in 0..SUPPORT_COUNT {
            let source_id = format!("{}:{slot}", link.source_id);
            let target_id = format!("{}:{slot}", layout.id);
            let Some(source) = result
                .gasket_supports
                .iter()
                .find(|s| s.id == source_id)
                .cloned()
            else {
                continue;
            };
            let Some(target_index) = result
                .gasket_supports
                .iter()
                .position(|s| s.id == target_id)
            else {
                continue;
            };
            if source.unlinked || result.gasket_supports[target_index].unlinked {
                continue;
            }
            let reflected = Vec2 {
                x: 2. * link.axis_x - source.at.x,
                y: source.at.y,
            };
            let region = regions.iter().find(|r| r.id == layout.id).unwrap();
            let candidate = region
                .tracks
                .iter()
                .filter_map(|track| {
                    let delta = Vec2 {
                        x: track.end.x - track.start.x,
                        y: track.end.y - track.start.y,
                    };
                    let length = distance(track.start, track.end);
                    let t = (((reflected.x - track.start.x) * delta.x
                        + (reflected.y - track.start.y) * delta.y)
                        / (length * length))
                        .clamp(0., 1.);
                    on_track(
                        region,
                        track.start_anchor + t * (track.end_anchor - track.start_anchor),
                    )
                })
                .min_by(|a, b| distance(a.at, reflected).total_cmp(&distance(b.at, reflected)))
                .filter(|c| distance(c.at, reflected) < 0.001);
            let Some(candidate) = candidate else {
                return Err("Linked gasket positions do not fit the mirrored outline. Unlink the support or restore symmetric clearance.".into());
            };
            let target = &mut result.gasket_supports[target_index];
            target.at = candidate.at;
            target.anchor = candidate.anchor;
            target.tangent = candidate.tangent;
            target.normal = candidate.normal;
            target.pair_id = Some(source_id.clone());
            target.mirror_axis = Some(link.axis_x);
            let source = result
                .gasket_supports
                .iter_mut()
                .find(|s| s.id == source_id)
                .unwrap();
            source.pair_id = Some(target_id.clone());
            source.mirror_axis = Some(link.axis_x);
            if let Some((_, c)) = chosen.iter_mut().find(|(id, _)| id == &target_id) {
                *c = candidate.clone();
            }
        }
    }
    // Recheck all paired placements after reflection.
    for (index, (_, a)) in chosen.iter().enumerate() {
        for (_, b) in &chosen[..index] {
            if overlaps(
                &rectangle(a, lobe_length + 0.5, 0.5, lobe_depth),
                &rectangle(b, lobe_length + 0.5, 0.5, lobe_depth),
            ) {
                return Err(
                    "Mirrored gasket supports collide; move or unlink the conflicting pair.".into(),
                );
            }
        }
    }
    let tabs = chosen
        .iter()
        .map(|(_, c)| rectangle(c, settings.length + 1., 0.6, tab_depth))
        .collect::<Vec<_>>();
    result.plate_contours = union(&original, &tabs);
    result
        .case
        .bodies
        .iter_mut()
        .find(|b| b.body.id == "plate")
        .ok_or("Plate is missing")?
        .contours = result.plate_contours.clone();
    let base = expanded_outline(
        &rings,
        config.clearance + config.wall_thickness,
        document.revision,
    )?;
    let lobes = chosen
        .iter()
        .map(|(_, c)| rectangle(c, lobe_length, config.wall_thickness, lobe_depth))
        .collect::<Vec<_>>();
    let outer = union(&base, &lobes);
    if outer.iter().filter(|c| !c.hole).count() != rings.len() {
        return Err(
            "Gasket supports join disconnected case halves; move the supports away from the split."
                .into(),
        );
    }
    let lower_cavity = expanded_outline(&rings, config.clearance, document.revision)?;
    let upper_cavity = expanded_outline(
        &result
            .plate_contours
            .iter()
            .filter(|c| !c.hole)
            .cloned()
            .collect::<Vec<_>>(),
        config.clearance,
        document.revision,
    )?;
    let retainer_z = config.plate_to_pcb + config.plate_thickness + compressed;
    let mut retainer_contours = outer.clone();
    retainer_contours.extend(lower_cavity.iter().map(|c| Contour {
        hole: true,
        points: c.points.clone(),
    }));
    let bottom = result
        .case
        .bodies
        .iter_mut()
        .find(|b| b.body.id == "bottom")
        .ok_or("Bottom case is missing")?;
    let floor_z = bottom.body.z.unwrap_or(0.) + config.bottom_thickness;
    let bottom_z = bottom.body.z.unwrap_or(0.);
    bottom.contours = outer;
    bottom.body.kind = CaseKind::Plate;
    bottom.body.thickness = retainer_z - bottom_z;
    bottom.body.wall_height = None;
    bottom.body.gasket = None;
    let mut openings = bottom.body.openings.take().unwrap_or_default();
    openings.extend(lower_cavity.iter().map(|c| CaseOpening {
        points: c.points.clone(),
        z: floor_z,
        height: retainer_z - floor_z + 0.01,
    }));
    openings.extend(upper_cavity.iter().map(|c| CaseOpening {
        points: c.points.clone(),
        z: config.plate_to_pcb - compressed,
        height: retainer_z - (config.plate_to_pcb - compressed) + 0.01,
    }));
    let mut mounts = bottom.body.mounts.take().unwrap_or_default();
    for (id, c) in &chosen {
        let at = add(c.at, scale(c.normal, bolt_distance));
        mounts.push(Mount {
            id: format!("closure:{id}"),
            at,
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        });
        let nut = (0..6)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 6.;
                Vec2 {
                    x: at.x + 2.5 * a.cos(),
                    y: at.y + 2.5 * a.sin(),
                }
            })
            .collect();
        openings.push(CaseOpening {
            points: nut,
            z: bottom_z - 0.01,
            height: 1.81,
        });
    }
    bottom.body.openings = Some(openings);
    bottom.body.mounts = Some(mounts.clone());
    let mut retainer = make_body(
        config,
        document.revision,
        "retainer",
        retainer_z,
        RETAINER_THICKNESS,
        retainer_contours,
    );
    retainer.body.mounts = Some(mounts.clone());
    result.case.bodies.push(retainer);
    result.stack.push(MechanicalStackLayer {
        id: "retainer".into(),
        z: retainer_z,
        thickness: RETAINER_THICKNESS,
    });
    for support in &result.gasket_supports {
        let c = Candidate {
            at: support.at,
            tangent: support.tangent,
            normal: support.normal,
            anchor: support.anchor,
        };
        let pad = rectangle(&c, settings.length, -0.5, settings.width + 0.5);
        for (side, z) in [
            ("lower", config.plate_to_pcb - compressed),
            ("upper", config.plate_to_pcb + config.plate_thickness),
        ] {
            let id = format!("gasket:{}:{side}", support.id);
            result.case.bodies.push(make_body(
                config,
                document.revision,
                &id,
                z,
                compressed,
                vec![Contour {
                    hole: false,
                    points: pad.clone(),
                }],
            ));
            result.stack.push(MechanicalStackLayer {
                id,
                z,
                thickness: compressed,
            });
        }
    }
    let screw_length = ((retainer_z + RETAINER_THICKNESS - bottom_z) / 2.).ceil() * 2.;
    result.generated_hardware=mounts.iter().flat_map(|mount|[
        MechanicalHardwareSpecification{id:format!("screw:{}",mount.id),part_id:"retainer".into(),feature_id:mount.id.clone(),designation:"Socket screw".into(),thread:"M2 × 0.4".into(),length:screw_length,quantity:1,notes:Some("Through-bolt; length derived from closure stack".into()),tolerance:None},
        MechanicalHardwareSpecification{id:format!("nut:{}",mount.id),part_id:"bottom".into(),feature_id:mount.id.clone(),designation:"Captive M2 hex nut".into(),thread:"M2 × 0.4".into(),length:1.6,quantity:1,notes:Some("4 mm across flats; 5 mm pocket circumscribed diameter, 1.8 mm pocket depth".into()),tolerance:None},
    ]).collect();
    let bottom_method = config
        .part_processes
        .iter()
        .flatten()
        .find(|p| p.part_id == "bottom")
        .map_or(&config.method, |p| &p.method);
    if matches!(bottom_method, PlateMethod::CutSheet | PlateMethod::PcbFr4) {
        result.diagnostics.push(Finding{id:"mechanical:gasket-case-process".into(),severity:Severity::Error,scope:Scope::Case,message:"The gasket shell and captive-nut pockets require printing or milling; assign a compatible bottom process before manufacturing export.".into(),target_ids:vec!["bottom".into()]});
    }
    Ok(())
}
