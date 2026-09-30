//! Versioned internal plate-tab construction. Legacy projects never enter this path.
use super::*;
mod motion;
mod placement;

const MAX_SUPPORTS: usize = 64;
const CLOSURE_SPACING_MM: f64 = 120.;
const MAX_AUTOMATIC_CLOSURES: usize = 64;
const CLOSURE_END_PREFERENCE_MM: f64 = 20.;
const TAB_MARGIN: f64 = 0.5;
const ROOT_OVERLAP: f64 = 0.6;
// The preparer quantizes both boundaries to 0.001 mm; reserve both roundings.
const OFFSET_GRID_ALLOWANCE: f64 = 0.002;

fn contained(points: &[Vec2], outline: &[Contour]) -> bool {
    let polygon: FloatPath = points.iter().map(|p| [p.x, p.y]).collect();
    polygon
        .overlay(&paths(outline), OverlayRule::Difference, FillRule::EvenOdd)
        .is_empty()
}

fn remaining_wall(pocket: &[Vec2], outer: &[Contour]) -> f64 {
    let point_segment = |p: Vec2, a: Vec2, b: Vec2| {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
        distance(
            p,
            Vec2 {
                x: a.x + t * dx,
                y: a.y + t * dy,
            },
        )
    };
    let mut minimum = f64::INFINITY;
    for (a, b) in pocket
        .iter()
        .zip(pocket.iter().cycle().skip(1))
        .take(pocket.len())
    {
        for contour in outer {
            for (c, d) in contour
                .points
                .iter()
                .zip(contour.points.iter().cycle().skip(1))
                .take(contour.points.len())
            {
                minimum = minimum
                    .min(point_segment(*a, *c, *d))
                    .min(point_segment(*b, *c, *d))
                    .min(point_segment(*c, *a, *b))
                    .min(point_segment(*d, *a, *b));
            }
        }
    }
    minimum
}

fn validate(
    config: &MechanicalConfiguration,
    settings: &InternalGasketConfiguration,
    foam: &MechanicalGasketLayout,
) -> Result<(), String> {
    let hardware = &settings.hardware;
    let positive = [
        settings.minimum_wall,
        foam.length,
        foam.width,
        foam.thickness,
        hardware.head_diameter,
        hardware.head_height,
        hardware.hole_diameter,
        hardware.insert_diameter,
        hardware.seat_diameter,
        hardware.seat_depth,
        hardware.engagement,
        hardware.bottoming_clearance,
        hardware.roof,
        hardware.surround,
        hardware.thread_diameter,
        hardware.pitch,
        hardware.insert_length,
        hardware.seat_lead_depth,
        hardware.seat_lead_diameter,
        hardware.bearing_thickness,
    ];
    if positive.iter().any(|v| !v.is_finite() || *v <= 0.)
        || [
            settings.support_clearance.unwrap_or(config.clearance),
            settings.tolerance,
            hardware.thread_start,
            hardware.tip_allowance,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        || !(2..=64).contains(&settings.support_count)
        || !foam.compression.is_finite()
        || !(0.0..0.5).contains(&foam.compression)
        || hardware.screw_lengths.is_empty()
        || hardware
            .screw_lengths
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err(
            "Internal gasket dimensions, hardware and support count must be finite and feasible."
                .into(),
        );
    }
    if hardware.id.is_empty()
        || hardware.thread.is_empty()
        || hardware.hole_diameter <= hardware.thread_diameter
        || hardware.thread_start + hardware.engagement > hardware.insert_length
        || hardware.seat_depth < hardware.insert_length + hardware.bottoming_clearance
        || hardware.seat_lead_depth >= hardware.seat_depth
        || hardware.seat_lead_diameter <= hardware.seat_diameter
        || hardware.seat_lead_diameter > hardware.insert_diameter
        || hardware
            .fixed_length
            .is_some_and(|v| !v.is_finite() || v <= 0. || !hardware.screw_lengths.contains(&v))
    {
        return Err("Custom closure requires compatible thread, insert length, lead-in, bearing material and a listed fixed screw length.".into());
    }
    let process = |id: &str| {
        config
            .part_processes
            .iter()
            .flatten()
            .find(|p| p.part_id == id)
            .map_or(&config.method, |p| &p.method)
    };
    if !matches!(process("bottom"), PlateMethod::Printed | PlateMethod::Cnc)
        || !matches!(
            (process("retainer"), &hardware.installation),
            (PlateMethod::Printed, InsertInstallation::HeatSet)
                | (PlateMethod::Cnc, InsertInstallation::Tapped)
        )
    {
        return Err("Selected insert installation is incompatible with the top-case process; supply a matching printed or machined installation specification.".into());
    }
    if config.wall_thickness < settings.minimum_wall {
        return Err("Nominal wall is smaller than the minimum wall behind gasket pockets.".into());
    }
    if hardware.head_diameter <= hardware.hole_diameter
        || hardware.seat_diameter >= hardware.insert_diameter
        || hardware.engagement + hardware.bottoming_clearance > hardware.seat_depth
    {
        return Err(
            "Closure head, insert seat, engagement and bottoming clearance are incompatible."
                .into(),
        );
    }
    let travel = config.gasket_travel.unwrap_or(0.3);
    if foam
        .adhesive_thickness
        .is_some_and(|v| !v.is_finite() || v < 0.)
        || foam.minimum_foam_thickness.is_some_and(|v| {
            !v.is_finite()
                || v < 0.
                || v > foam.thickness
                || foam.thickness * (1. - foam.compression) - travel - settings.tolerance < v
        })
    {
        return Err("Adhesive thickness or the minimum compressed foam thickness is incompatible with the requested travel.".into());
    }
    if travel + settings.tolerance >= foam.thickness * foam.compression {
        return Err("Upper or lower pad loses contact before the requested vertical travel; reduce travel or choose a different preload.".into());
    }
    if travel + settings.tolerance >= foam.thickness * (1. - foam.compression) {
        return Err(
            "The compressing gasket has no remaining thickness at the vertical travel limit."
                .into(),
        );
    }
    Ok(())
}

fn candidates(ring: &Contour, margin: f64) -> Vec<Candidate> {
    let lengths: Vec<_> = ring
        .points
        .iter()
        .enumerate()
        .map(|(i, a)| distance(*a, ring.points[(i + 1) % ring.points.len()]))
        .collect();
    let perimeter: f64 = lengths.iter().sum();
    let mut travelled = 0.;
    let mut candidates = vec![];
    for (edge, length) in lengths.into_iter().enumerate() {
        let a = ring.points[edge];
        let b = ring.points[(edge + 1) % ring.points.len()];
        let tangent = Vec2 {
            x: (b.x - a.x) / length,
            y: (b.y - a.y) / length,
        };
        let mut t = margin;
        while t <= length - margin {
            candidates.push(Candidate {
                at: add(a, scale(tangent, t)),
                tangent,
                normal: Vec2 {
                    x: tangent.y,
                    y: -tangent.x,
                },
                anchor: (travelled + t) / perimeter,
            });
            t += 1.;
        }
        travelled += length;
    }
    candidates
}

fn reflect(candidate: &Candidate, axis: f64) -> Candidate {
    Candidate {
        at: Vec2 {
            x: 2. * axis - candidate.at.x,
            y: candidate.at.y,
        },
        tangent: Vec2 {
            x: -candidate.tangent.x,
            y: candidate.tangent.y,
        },
        normal: Vec2 {
            x: -candidate.normal.x,
            y: candidate.normal.y,
        },
        anchor: candidate.anchor,
    }
}

fn nearest(candidates: &[Candidate], ideal: f64) -> Option<Candidate> {
    let difference = |anchor: f64| {
        let d = (anchor - ideal).abs();
        d.min(1. - d)
    };
    candidates
        .iter()
        .min_by(|a, b| difference(a.anchor).total_cmp(&difference(b.anchor)))
        .cloned()
}

fn keycap_envelopes(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
) -> Result<Vec<Vec<Vec2>>, String> {
    let board = document
        .boards
        .iter()
        .find(|b| b.id == config.board_id)
        .ok_or("Board is missing")?;
    let mut caps = vec![];
    for part in document
        .parts
        .iter()
        .filter(|p| board.part_ids.contains(&p.id))
    {
        let definition = document
            .definitions
            .iter()
            .find(|d| d.id == part.definition_id);
        let Some(size) = part.keycap.or_else(|| definition.and_then(|d| d.keycap)) else {
            continue;
        };
        if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0. || size.y <= 0. {
            return Err(format!("Keycap {} needs positive finite bounds", part.id));
        }
        let local = vec![
            Vec2 {
                x: -size.x / 2.,
                y: -size.y / 2.,
            },
            Vec2 {
                x: size.x / 2.,
                y: -size.y / 2.,
            },
            Vec2 {
                x: size.x / 2.,
                y: size.y / 2.,
            },
            Vec2 {
                x: -size.x / 2.,
                y: size.y / 2.,
            },
        ];
        let outline = Contour {
            points: transform_part_points(part, &local),
            hole: false,
        };
        caps.extend(
            expanded_outline(&[outline], config.clearance, document.revision)?
                .into_iter()
                .map(|c| c.points),
        );
    }
    Ok(caps)
}

fn required_offset(
    rings: &[Contour],
    envelopes: &[Vec<Vec2>],
    revision: u64,
) -> Result<f64, String> {
    if envelopes.iter().all(|p| contained(p, rings)) {
        return Ok(0.);
    }
    let mut upper = 1.;
    let mut lower = 0.;
    loop {
        let offset = expanded_outline(rings, upper, revision)?;
        if envelopes.iter().all(|p| contained(p, &offset)) {
            break;
        }
        lower = upper;
        upper *= 2.;
        if upper > 128. {
            return Err("Moving envelope requires more than the bounded 128 mm case-offset search; review the board and component positions.".into());
        }
    }
    while upper - lower > 0.001 {
        let middle = (upper + lower) / 2.;
        let offset = expanded_outline(rings, middle, revision)?;
        if envelopes.iter().all(|p| contained(p, &offset)) {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    Ok((upper * 1000.).ceil() / 1000.)
}

pub(super) fn generate(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    settings: &InternalGasketConfiguration,
    result: &mut MechanicalAssembly,
    component_volumes: &[(String, CaseOpening)],
) -> Result<(), String> {
    let defaults = MechanicalGasketLayout {
        auto_size: None,
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
    let foam = config.gasket_layout.as_ref().unwrap_or(&defaults);
    validate(config, settings, foam)?;
    let hardware = &settings.hardware;
    let compressed =
        foam.thickness * (1. - foam.compression) + foam.adhesive_thickness.unwrap_or(0.);
    let travel = config.gasket_travel.unwrap_or(0.3);
    let clearance = config.clearance.max(settings.tolerance);
    let front = settings.support_clearance.unwrap_or(config.clearance) + settings.tolerance;
    let nominal = result.plate_contours.clone();
    let rings: Vec<_> = nominal
        .iter()
        .filter(|c| !c.hole)
        .map(simple_ring)
        .collect();
    let seam = config.plate_to_pcb + config.plate_thickness + compressed;
    let mut caps = keycap_envelopes(document, config)?;
    let mut component_envelopes = vec![];
    for (_, volume) in component_volumes {
        let expanded = expanded_outline(
            &[Contour {
                points: volume.points.clone(),
                hole: false,
            }],
            clearance,
            document.revision,
        )?;
        if volume.z + volume.height + travel + settings.tolerance > seam {
            caps.extend(expanded.iter().map(|c| c.points.clone()));
        }
        component_envelopes.extend(expanded.into_iter().map(|c| c.points));
    }
    let component_offset = required_offset(&rings, &component_envelopes, document.revision)?;
    let cap_offset = required_offset(&rings, &caps, document.revision)?;
    let pad_inner = front.max(cap_offset) + TAB_MARGIN;
    let maximum_width = foam
        .supports
        .iter()
        .filter_map(|s| s.width)
        .fold(foam.width, f64::max);
    let tab_outer = pad_inner + maximum_width + TAB_MARGIN;
    let pocket_outer = tab_outer + clearance;
    let contact = config.plate_to_pcb - compressed;
    let old_bottom = result
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "bottom")
        .ok_or("Bottom case is missing")?;
    // Lower the floor, not the floating assembly, to preserve the downward motion reserve.
    let component_bottom = component_volumes
        .iter()
        .map(|(_, v)| v.z)
        .fold(-config.pcb_thickness, f64::min);
    let floor_z = (old_bottom.body.z.unwrap_or(0.) + config.bottom_thickness).min(component_bottom)
        - travel
        - settings.tolerance;
    let bottom_z = floor_z
        - config
            .bottom_thickness
            .max(hardware.head_height + hardware.bearing_thickness);
    if contact <= floor_z {
        return Err("Lower pad leaves no positive support tower above the floor.".into());
    }
    let bearing = bottom_z + hardware.head_height;
    let grip = seam - bearing;
    let length_correction=match hardware.length_datum {
        ScrewLengthDatum::UnderHead=>0.,
        ScrewLengthDatum::Overall=>hardware.head_height,
        ScrewLengthDatum::Unresolved=>return Err("Selected screw length datum is unresolved; supply a product drawing or explicit Custom datum.".into()),
    };
    if hardware.head_profile == ScrewHeadProfile::Countersunk
        && hardware.length_datum != ScrewLengthDatum::Overall
    {
        return Err("Countersunk screw length must use the overall-length datum.".into());
    }
    let available: Vec<_> = hardware
        .screw_lengths
        .iter()
        .copied()
        .filter(|l| hardware.fixed_length.is_none_or(|fixed| fixed == *l))
        .collect();
    let fits = |length: f64| {
        let reach = length - length_correction;
        reach - hardware.tip_allowance >= grip + hardware.thread_start + hardware.engagement
            && reach <= grip + hardware.seat_depth - hardware.bottoming_clearance
    };
    let (length, boss_depth) = if let Some(length) = available
        .iter()
        .copied()
        .filter(|l| fits(*l))
        .min_by(f64::total_cmp)
    {
        (length, 0.)
    } else {
        available.iter().copied().filter_map(|length| {
            let depth=grip+hardware.thread_start+hardware.engagement+hardware.tip_allowance-(length-length_correction);
            (depth>0. && depth<seam-floor_z-clearance && length>length_correction).then_some((length,depth))
        }).min_by(|a,b|a.1.total_cmp(&b.1)).ok_or("No listed screw reaches the insert with a supported boss and receiving clearance; choose a compatible length or insert.")?
    };
    let top_height = (hardware.seat_depth - boss_depth + hardware.roof).max(hardware.roof);
    let boss_radius = hardware.insert_diameter / 2. + hardware.surround;
    let closure_radius = (hardware.head_diameter.max(hardware.insert_diameter) / 2.
        + hardware.surround)
        .max(if boss_depth > 0. {
            boss_radius + clearance + hardware.surround
        } else {
            0.
        });
    let cavity_offset = config
        .clearance
        .max(component_offset)
        .max(
            pocket_outer + settings.tolerance + OFFSET_GRID_ALLOWANCE
                - (config.wall_thickness - settings.minimum_wall),
        )
        .max(front + 2. * closure_radius + settings.tolerance - config.wall_thickness);
    let outer_offset = cavity_offset + config.wall_thickness;
    let outer = expanded_outline(&rings, outer_offset, document.revision)?;
    if outer.len() != rings.len() {
        return Err(
            "Internal case offsets merge disconnected regions; increase the split clearance."
                .into(),
        );
    }
    let cavity = expanded_outline(&rings, cavity_offset, document.revision)?;
    let top_opening = union(&expanded_outline(&rings, front, document.revision)?, &caps);
    for (id, volume) in component_volumes {
        if volume.z - travel - settings.tolerance < floor_z || !contained(&volume.points, &cavity) {
            return Err(format!(
                "Component {id} exceeds the resolved floating clearance envelope; supply a compatible component profile or case dimensions."
            ));
        }
    }
    let mut openings = old_bottom.body.openings.clone().unwrap_or_default();
    let obstacles: Vec<_> = result
        .case
        .bodies
        .iter()
        .flat_map(|b| b.body.openings.iter().flatten())
        .chain(component_volumes.iter().map(|(_, v)| v))
        .map(|o| o.points.clone())
        .collect();
    let mut closures: Vec<(String, Candidate)> = vec![];
    for (index, ring) in rings.iter().enumerate() {
        let id = region_id(document, &config.board_id, ring, index);
        let available: Vec<_> = candidates(ring, closure_radius + 1.)
            .into_iter()
            .filter(|c| {
                let land = rectangle(
                    c,
                    2. * closure_radius,
                    -front,
                    outer_offset - settings.tolerance,
                );
                contained(&land, &outer)
                    && !overlaps(&land, &ring.points)
                    && !obstacles
                        .iter()
                        .chain(caps.iter())
                        .any(|o| overlaps(&land, o))
            })
            .collect();
        if let Some(saved) = config.closure_mounts.as_ref().filter(|mounts| {
            mounts
                .iter()
                .any(|m| m.id.starts_with(&format!("closure:{id}:")))
        }) {
            for mount in saved
                .iter()
                .filter(|m| m.id.starts_with(&format!("closure:{id}:")))
            {
                let candidate=available.iter().filter_map(|c| {
                    let center=add(c.at,scale(c.normal,front+closure_radius));
                    let delta=Vec2 { x:mount.at.x-center.x,y:mount.at.y-center.y };
                    let along=delta.x*c.tangent.x+delta.y*c.tangent.y;
                    if (delta.x*c.normal.x+delta.y*c.normal.y).abs()>1e-6 || along.abs()>0.51 { return None; }
                    let mut placed=c.clone(); placed.at=add(c.at,scale(c.tangent,along));
                    let land=rectangle(&placed,2.*closure_radius,-front,outer_offset-settings.tolerance);
                    (contained(&land,&outer) && !obstacles.iter().any(|o|overlaps(&land,o))).then_some(placed)
                }).next().ok_or_else(||format!("Closure {} no longer fits its fixed position; move it explicitly or change the hardware/clearance.",mount.id))?;
                closures.push((mount.id.clone(), candidate));
            }
            continue;
        }
        let perimeter: f64 = ring
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| distance(*p, ring.points[(i + 1) % ring.points.len()]))
            .sum();
        // Add screws in pairs to keep opposite perimeter targets balanced.
        let closure_count = ((perimeter / (2. * CLOSURE_SPACING_MM)).ceil() * 2.).max(4.) as usize;
        if closure_count > MAX_AUTOMATIC_CLOSURES {
            return Err(format!(
                "Case region {id} needs more than {MAX_AUTOMATIC_CLOSURES} automatic closure screws; split the case into smaller regions."
            ));
        }
        // Compact four-screw cases reserve long straight runs for pads.
        // Larger layouts also need screws along those runs to limit gaps.
        let end_search_radius = if closure_count == 4 {
            f64::INFINITY
        } else {
            CLOSURE_END_PREFERENCE_MM
        };
        let minimum_anchor_gap = 0.5 / closure_count as f64;
        for slot in 0..closure_count {
            let clear: Vec<_> = available
                .iter()
                .filter(|c| {
                    !closures.iter().any(|(other_id, other)| {
                        let delta = (c.anchor - other.anchor).abs();
                        (other_id.starts_with(&format!("closure:{id}:"))
                            && delta.min(1. - delta) < minimum_anchor_gap)
                            || overlaps(
                                &rectangle(c, 2. * closure_radius + 1., -front, outer_offset),
                                &rectangle(other, 2. * closure_radius + 1., -front, outer_offset),
                            )
                    })
                })
                .cloned()
                .collect();
            let candidate = placement::closure(ring,&clear,closure_radius+1.,slot as f64/closure_count as f64,end_search_radius).ok_or_else(||format!("No independent closure layout fits {id}; increase perimeter clearance or move openings."))?;
            closures.push((format!("closure:{id}:{slot}"), candidate));
        }
    }
    if let Some(saved) = config.closure_mounts.as_ref().filter(|m| !m.is_empty()) {
        if saved
            .iter()
            .any(|m| !closures.iter().any(|(id, _)| *id == m.id))
        {
            return Err("An established closure references an unresolved case region; explicitly convert or reset closures.".into());
        }
    }
    for (index, (id, c)) in closures.iter().enumerate() {
        if closures[..index].iter().any(|(other_id, other)| {
            id == other_id
                || overlaps(
                    &rectangle(c, 2. * closure_radius + clearance, -front, outer_offset),
                    &rectangle(other, 2. * closure_radius + clearance, -front, outer_offset),
                )
        }) {
            return Err(format!("Closure {id} conflicts with another closure land."));
        }
    }
    result.suggested_mounts = closures
        .iter()
        .map(|(id, c)| Mount {
            id: id.clone(),
            at: add(c.at, scale(c.normal, front + closure_radius)),
            kind: MountKind::Hole,
            hole_diameter: hardware.hole_diameter,
            boss_diameter: Some(2. * closure_radius),
            height: None,
        })
        .collect();
    let valid_support = |candidate: &Candidate, length: f64, width: f64| {
        let tab_outer = pad_inner + width + TAB_MARGIN;
        let pocket_outer = tab_outer + clearance;
        let pocket = rectangle(
            candidate,
            length + 2. * TAB_MARGIN + 2. * clearance + 1.,
            ROOT_OVERLAP,
            pocket_outer,
        );
        // The tower extends beyond the pad pocket to meet the case wall.
        // Check its full reach so concave outlines cannot put an opposite
        // edge of the floating assembly inside the tower.
        let outward = rectangle(
            candidate,
            length + 2. * TAB_MARGIN + 2. * clearance + 1.,
            -front,
            pocket_outer.max(outer_offset),
        );
        let root = rectangle(candidate, length + 2. * TAB_MARGIN, ROOT_OVERLAP, tab_outer);
        let buffered = expanded_outline(
            &[Contour {
                points: pocket.clone(),
                hole: false,
            }],
            settings.minimum_wall + settings.tolerance,
            document.revision,
        );
        let pad = rectangle(candidate, length, -pad_inner, pad_inner + width);
        contained(&root, &outer)
            && remaining_wall(&pocket, &outer) + 1e-9 >= settings.minimum_wall + settings.tolerance
            && !top_opening
                .iter()
                .any(|opening| overlaps(&pad, &opening.points))
            && buffered.is_ok_and(|contours| contours.iter().all(|c| contained(&c.points, &outer)))
            && !rings.iter().any(|r| overlaps(&outward, &r.points))
            && !nominal
                .iter()
                .filter(|c| c.hole)
                .any(|hole| overlaps(&root, &hole.points))
            && !obstacles.iter().any(|o| overlaps(&pocket, o))
            && !closures.iter().any(|(_, c)| {
                overlaps(
                    &pocket,
                    &rectangle(c, 2. * closure_radius + clearance, -front, outer_offset),
                )
            })
    };
    let automatic = foam.auto_size.unwrap_or(true);
    let automatic_count = settings.auto_count.unwrap_or(settings.support_count == 4);
    let mut chosen: Vec<(String, Candidate, f64, f64)> = vec![];
    let side = |c: &Candidate| {
        if c.normal.x.abs() > c.normal.y.abs() {
            if c.normal.x > 0. { 0 } else { 1 }
        } else if c.normal.y > 0. {
            2
        } else {
            3
        }
    };
    for (index, ring) in rings.iter().enumerate() {
        let id = region_id(document, &config.board_id, ring, index);
        let key = outline_key(ring);
        // Editing tracks cover the entire perimeter. Fit is a separate result.
        let tracks = placement::tracks(ring, &id);
        let region = Region {
            id: id.clone(),
            key: key.clone(),
            candidates: vec![],
            tracks,
        };
        let pinned: Vec<_> = foam
            .supports
            .iter()
            .filter(|s| s.region_id == id && s.placement != Some(GasketPlacement::Generated))
            .filter_map(|s| on_track(&region, s.anchor).map(|c| (s, c)))
            .collect();
        let lengths: Vec<f64> = if automatic {
            (1..=8)
                .rev()
                .map(|n| n as f64 * 10.)
                .chain(std::iter::once(5.))
                .collect()
        } else {
            vec![foam.length]
        };
        let collides =
            |c: &Candidate, length: f64, other: &Candidate, other_length: f64, other_width: f64| {
                overlaps(
                    &rectangle(
                        c,
                        length + 2. * TAB_MARGIN + 2. * clearance,
                        ROOT_OVERLAP,
                        pad_inner + foam.width + TAB_MARGIN + clearance,
                    ),
                    &rectangle(
                        other,
                        other_length + 2. * TAB_MARGIN + 2. * clearance,
                        ROOT_OVERLAP,
                        pad_inner + other_width + TAB_MARGIN + clearance,
                    ),
                )
            };
        let plan = placement::plan(
            &region,
            (!automatic_count)
                .then_some(settings.support_count.max(4).saturating_sub(pinned.len())),
            &lengths,
            TAB_MARGIN + clearance + 1.,
            &pinned.iter().map(|(_, c)| c.clone()).collect::<Vec<_>>(),
            |c, length, planned| {
                valid_support(c, length, foam.width)
                    && !pinned.iter().any(|(s, other)| {
                        collides(
                            c,
                            length,
                            other,
                            s.length.unwrap_or(foam.length),
                            s.width.unwrap_or(foam.width),
                        )
                    })
                    && !chosen.iter().any(|(_, other, other_length, other_width)| {
                        collides(c, length, other, *other_length, *other_width)
                    })
                    && !planned.iter().any(|(other, other_length)| {
                        collides(c, length, other, *other_length, foam.width)
                    })
                    && document
                        .layouts
                        .iter()
                        .filter_map(|l| l.mirror_link.as_ref().map(|link| (l, link)))
                        .filter(|(l, link)| l.id == id || link.source_id == id)
                        .all(|(_, link)| {
                            valid_support(&reflect(c, link.axis_x), length, foam.width)
                        })
            },
        );
        let mut slots: Vec<usize> = if automatic_count {
            pinned
                .iter()
                .filter_map(|(s, _)| s.id.strip_prefix(&format!("{id}:"))?.parse().ok())
                .filter(|slot| *slot < MAX_SUPPORTS)
                .collect()
        } else {
            (0..settings.support_count.max(4)).collect()
        };
        if automatic_count {
            let mut slot = 0;
            for _ in &plan {
                while slots.contains(&slot) {
                    slot += 1;
                }
                slots.push(slot);
                slot += 1;
            }
        }
        let mut planned = plan.into_iter();
        slots.sort_by_key(|slot| {
            !foam.supports.iter().any(|s| {
                s.id == format!("{id}:{slot}") && s.placement != Some(GasketPlacement::Generated)
            })
        });
        for slot in slots {
            let support_id = format!("{id}:{slot}");
            let saved = foam
                .supports
                .iter()
                .find(|s| s.id == support_id && s.placement != Some(GasketPlacement::Generated));
            let width = saved.and_then(|s| s.width).unwrap_or(foam.width);
            let clear = |c: &Candidate, length: f64| {
                !chosen.iter().any(|(_, other, other_length, other_width)| {
                    overlaps(
                        &rectangle(
                            c,
                            length + 2. * TAB_MARGIN + 2. * clearance,
                            ROOT_OVERLAP,
                            pad_inner + width + TAB_MARGIN + clearance,
                        ),
                        &rectangle(
                            other,
                            other_length + 2. * TAB_MARGIN + 2. * clearance,
                            ROOT_OVERLAP,
                            pad_inner + other_width + TAB_MARGIN + clearance,
                        ),
                    )
                })
            };
            let (candidate, length, mut fit_error) = if let Some(saved) = saved {
                let length = saved.length.unwrap_or(foam.length);
                let c = on_track(&region, saved.anchor).ok_or_else(|| {
                    format!("Support {support_id} has an invalid perimeter anchor.")
                })?;
                let error = if saved.region_id != id || saved.outline_key != key {
                    Some("Outline changed; move this gasket to confirm its position.".to_string())
                } else if !length.is_finite()
                    || length <= 0.
                    || !width.is_finite()
                    || width <= 0.
                    || !valid_support(&c, length, width)
                    || !clear(&c, length)
                {
                    Some("Gasket does not fit here; shorten it or move it clear of corners, closures and other supports.".to_string())
                } else {
                    None
                };
                (c, length, error)
            } else {
                planned.next().map(|(c,length)|(c,length,None)).unwrap_or_else(|| {
                    let occupied: Vec<_> = chosen.iter().filter(|(name,_,_,_)|name.starts_with(&format!("{id}:"))).map(|(_,c,_,_)|side(c)).collect();
                    let target_side=(0..4).min_by_key(|s|occupied.iter().filter(|other|*other==s).count()).unwrap();
                    let candidate=region.tracks.iter().filter_map(|track|on_track(&region,(track.start_anchor+track.end_anchor)/2.))
                        .find(|c|side(c)==target_side).unwrap_or_else(||on_track(&region,0.5).unwrap());
                    (candidate,if automatic {5.} else {foam.length},Some("No clear support fits this count; reduce the count or adjust the gasket, openings or closures.".into()))
                })
            };
            if !length.is_finite() || !width.is_finite() {
                fit_error = Some("Gasket dimensions must be finite.".into());
            }
            result.gasket_supports.push(MechanicalGasketSupport {
                fit_error,
                placement: Some(if saved.is_some() {
                    GasketPlacement::User
                } else {
                    GasketPlacement::Generated
                }),
                id: support_id.clone(),
                region_id: id.clone(),
                outline_key: key.clone(),
                anchor: candidate.anchor,
                at: candidate.at,
                tangent: candidate.tangent,
                normal: candidate.normal,
                length,
                width,
                z: contact,
                thickness: compressed,
                pair_id: None,
                mirror_axis: None,
                unlinked: saved.is_some_and(|s| s.unlinked),
            });
            chosen.push((support_id, candidate, length, width));
        }
        result.gasket_tracks.extend(region.tracks);
    }
    if let Some(saved) = foam.supports.iter().find(|s| {
        s.placement != Some(GasketPlacement::Generated)
            && !chosen.iter().any(|(id, _, _, _)| *id == s.id)
    }) {
        return Err(format!(
            "Saved support {} cannot be discarded by a count or region change; remove it explicitly.",
            saved.id
        ));
    }
    // Placement prioritizes pinned supports; presentation keeps stable slot order.
    result.gasket_supports.sort_by(|a, b| {
        let slot = |id: &str| {
            id.rsplit(':')
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0)
        };
        a.region_id
            .cmp(&b.region_id)
            .then_with(|| slot(&a.id).cmp(&slot(&b.id)))
    });
    for layout in &document.layouts {
        let Some(link) = &layout.mirror_link else {
            continue;
        };
        for slot in 0..result
            .gasket_supports
            .iter()
            .filter_map(|s| s.id.rsplit(':').next()?.parse::<usize>().ok())
            .max()
            .map_or(0, |slot| slot + 1)
        {
            let source_id = format!("{}:{slot}", link.source_id);
            let target_id = format!("{}:{slot}", layout.id);
            let Some(source) = result
                .gasket_supports
                .iter()
                .position(|s| s.id == source_id)
            else {
                continue;
            };
            let Some(target) = result
                .gasket_supports
                .iter()
                .position(|s| s.id == target_id)
            else {
                continue;
            };
            if result.gasket_supports[source].unlinked || result.gasket_supports[target].unlinked {
                continue;
            }
            let (master, follower) =
                if result.gasket_supports[target].placement == Some(GasketPlacement::User) {
                    (target, source)
                } else {
                    (source, target)
                };
            let master_support = result.gasket_supports[master].clone();
            let follower_support = result.gasket_supports[follower].clone();
            let reflected = Vec2 {
                x: 2. * link.axis_x - master_support.at.x,
                y: master_support.at.y,
            };
            if follower_support.placement == Some(GasketPlacement::User)
                && distance(reflected, follower_support.at) > 0.001
            {
                return Err(format!(
                    "Pinned linked supports {source_id} and {target_id} disagree; explicitly move or unlink them."
                ));
            }
            let tracks: Vec<_> = result
                .gasket_tracks
                .iter()
                .filter(|t| t.region_id == follower_support.region_id)
                .cloned()
                .collect();
            let region = Region {
                id: follower_support.region_id.clone(),
                key: follower_support.outline_key.clone(),
                candidates: vec![],
                tracks,
            };
            let candidate=region.tracks.iter().filter_map(|track|{
                let dx=track.end.x-track.start.x;let dy=track.end.y-track.start.y;
                let fraction=((reflected.x-track.start.x)*dx+(reflected.y-track.start.y)*dy)/(dx*dx+dy*dy);
                if !(0. ..=1.).contains(&fraction) {return None;}
                on_track(&region,track.start_anchor+fraction*(track.end_anchor-track.start_anchor))
                    .filter(|c|distance(c.at,reflected)<0.001)
            }).next().ok_or_else(||format!("Linked support {} does not fit its reflected capture track; move or unlink it.",follower_support.id))?;
            let conflict = !valid_support(&candidate, master_support.length, master_support.width);
            let support = &mut result.gasket_supports[follower];
            support.length = master_support.length;
            support.width = master_support.width;
            support.fit_error = if conflict {
                Some("Linked gasket does not fit; resize, move or unlink it.".into())
            } else {
                master_support.fit_error.clone()
            };
            support.at = candidate.at;
            support.anchor = candidate.anchor;
            support.tangent = candidate.tangent;
            support.normal = candidate.normal;
            support.placement = master_support.placement.clone();
            support.pair_id = Some(master_support.id.clone());
            support.mirror_axis = Some(link.axis_x);
            result.gasket_supports[master].pair_id = Some(follower_support.id.clone());
            result.gasket_supports[master].mirror_axis = Some(link.axis_x);
            *chosen
                .iter_mut()
                .find(|(id, _, _, _)| *id == follower_support.id)
                .unwrap() = (
                follower_support.id.clone(),
                candidate,
                master_support.length,
                master_support.width,
            );
        }
    }
    // Validate final reflected positions together, not against a follower's temporary placement.
    for (index, (id, candidate, length, width)) in chosen.iter().enumerate() {
        if chosen[..index]
            .iter()
            .any(|(_, other, other_length, other_width)| {
                overlaps(
                    &rectangle(
                        candidate,
                        length + 2. * TAB_MARGIN + 2. * clearance,
                        ROOT_OVERLAP,
                        pad_inner + width + TAB_MARGIN + clearance,
                    ),
                    &rectangle(
                        other,
                        other_length + 2. * TAB_MARGIN + 2. * clearance,
                        ROOT_OVERLAP,
                        pad_inner + other_width + TAB_MARGIN + clearance,
                    ),
                )
            })
        {
            if let Some(support) = result.gasket_supports.iter_mut().find(|s| s.id == *id) {
                support.fit_error =
                    Some("Gasket overlaps another support; shorten or move it.".into());
            }
        }
    }
    for (index, ring) in rings.iter().enumerate() {
        let id = region_id(document, &config.board_id, ring, index);
        let sides: std::collections::HashSet<_> = chosen
            .iter()
            .filter(|(name, _, _, _)| name.starts_with(&format!("{id}:")))
            .map(|(_, c, _, _)| side(c))
            .collect();
        if sides.len() < 4 {
            result.diagnostics.push(Finding { id:format!("gasket-coverage:{id}"), severity:Severity::Error, scope:Scope::Case,
                message:"Gaskets must support all four sides; move or reset the supports to restore coverage.".into(), target_ids:vec![] });
            result.generation_blocked = true;
        }
    }
    for support in &result.gasket_supports {
        if let Some(message) = &support.fit_error {
            result.diagnostics.push(Finding {
                id: format!("gasket-fit:{}", support.id),
                severity: Severity::Error,
                scope: Scope::Case,
                message: message.clone(),
                target_ids: vec![format!("gasket:{}:lower", support.id)],
            });
            result.generation_blocked = true;
        }
    }
    if result.generation_blocked {
        return Ok(());
    }
    let tabs: Vec<_> = chosen
        .iter()
        .map(|(_, c, length, width)| {
            rectangle(
                c,
                length + 2. * TAB_MARGIN,
                ROOT_OVERLAP,
                pad_inner + width + TAB_MARGIN,
            )
        })
        .collect();
    result.plate_contours = union(&nominal, &tabs);
    result
        .case
        .bodies
        .iter_mut()
        .find(|b| b.body.id == "plate")
        .ok_or("Plate is missing")?
        .contours = result.plate_contours.clone();
    let mut bottom_features = vec![];
    let mut top_features = vec![];
    for (id, c, length, width) in &chosen {
        bottom_features.push(CaseFeature::SupportPrism {
            id: format!("tower:{id}"),
            points: rectangle(
                c,
                *length + 2. * TAB_MARGIN,
                -front,
                outer_offset - settings.tolerance,
            ),
            z: floor_z,
            height: contact - floor_z,
        });
        openings.push(CaseOpening {
            points: rectangle(
                c,
                *length + 2. * TAB_MARGIN + 2. * clearance,
                ROOT_OVERLAP,
                pad_inner + width + TAB_MARGIN + clearance,
            ),
            z: contact,
            height: seam - contact,
        });
    }
    for (id, c) in &closures {
        let at = add(c.at, scale(c.normal, front + closure_radius));
        bottom_features.push(CaseFeature::SupportPrism {
            id: format!("land:{id}"),
            points: rectangle(
                c,
                2. * closure_radius,
                -front,
                outer_offset - settings.tolerance,
            ),
            z: floor_z,
            height: seam - floor_z,
        });
        bottom_features.push(CaseFeature::RoundSeat {
            id: format!("bore:{id}"),
            at,
            z: bottom_z,
            height: seam - bottom_z,
            diameter: hardware.hole_diameter,
        });
        bottom_features.push(match hardware.head_profile {
            ScrewHeadProfile::Flat => CaseFeature::RoundSeat {
                id: format!("head:{id}"),
                at,
                z: bottom_z,
                height: hardware.head_height,
                diameter: hardware.head_diameter,
            },
            ScrewHeadProfile::Countersunk => CaseFeature::ConicalSeat {
                id: format!("head:{id}"),
                at,
                z: bottom_z,
                height: hardware.head_height,
                diameter: hardware.head_diameter,
                end_diameter: hardware.hole_diameter,
            },
        });
        if boss_depth > 0. {
            let boss_front = front + closure_radius - boss_radius;
            let boss_outer = front + closure_radius + boss_radius;
            top_features.push(CaseFeature::SupportPrism {
                id: format!("closure-boss:{id}"),
                points: rectangle(c, 2. * boss_radius, -boss_front, boss_outer),
                z: seam - boss_depth,
                height: boss_depth + top_height / 2.,
            });
            openings.push(CaseOpening {
                points: rectangle(
                    c,
                    2. * (boss_radius + clearance),
                    -(boss_front - clearance),
                    boss_outer + clearance,
                ),
                z: seam - boss_depth - clearance,
                height: boss_depth + clearance,
            });
        }
        top_features.push(CaseFeature::RoundSeat {
            id: format!("insert:{id}"),
            at,
            z: seam - boss_depth,
            height: hardware.seat_depth,
            diameter: hardware.seat_diameter,
        });
        top_features.push(CaseFeature::ConicalSeat {
            id: format!("insert-lead:{id}"),
            at,
            z: seam - boss_depth,
            height: hardware.seat_lead_depth,
            diameter: hardware.seat_lead_diameter,
            end_diameter: hardware.seat_diameter,
        });
        for (prefix, part_id, designation, dimension) in [
            ("screw", "bottom", "Custom screw", length),
            (
                "insert",
                "retainer",
                "Custom insert",
                hardware.insert_length,
            ),
        ] {
            result
                .generated_hardware
                .push(MechanicalHardwareSpecification {
                    id: format!("{prefix}:{id}"),
                    part_id: part_id.into(),
                    feature_id: id.clone(),
                    designation: designation.into(),
                    thread: hardware.thread.clone(),
                    length: dimension,
                    quantity: 1,
                    notes: Some(format!(
                        "Custom geometry {}: process and fit require review",
                        hardware.id
                    )),
                    tolerance: None,
                });
        }
    }
    let bottom = result
        .case
        .bodies
        .iter_mut()
        .find(|b| b.body.id == "bottom")
        .unwrap();
    bottom.contours = outer.clone();
    bottom.body.kind = CaseKind::Tray;
    bottom.body.z = Some(bottom_z);
    bottom.body.thickness = floor_z - bottom_z;
    bottom.body.wall_height = Some(seam - floor_z);
    bottom.body.wall_thickness = Some(config.wall_thickness);
    bottom.body.mounts = None;
    bottom.body.gasket = None;
    bottom.body.openings = Some(openings);
    bottom.body.features = Some(bottom_features);
    let mut top_contours = outer;
    top_contours.extend(top_opening.into_iter().map(|c| Contour {
        hole: true,
        points: c.points,
    }));
    // Retain the old ID so per-part visibility and process references remain addressable.
    let mut top = make_body(
        config,
        document.revision,
        "retainer",
        seam,
        top_height,
        top_contours,
    );
    top.body.name = "Top case".into();
    top.body.features = Some(top_features);
    result.case.bodies.push(top);
    result.stack.push(MechanicalStackLayer {
        id: "retainer".into(),
        z: seam,
        thickness: top_height,
    });
    for (id, c, length, width) in &chosen {
        result.generated_materials.push(MechanicalMaterialSpecification {
            adhesive_thickness:foam.adhesive_thickness.unwrap_or(0.),
            id:format!("foam:{id}"),feature_id:id.clone(),quantity:2,
            size:Vec3{x:*length,y:*width,z:foam.thickness},
            preset_id:if *length == foam.length && *width == foam.width { foam.preset_id.clone() } else { None },material:foam.material.clone(),
            notes:"Free cutting dimensions; preview pads use assembled thickness. Material limits and adhesive inclusion need review.".into(),
        });
        for (side, z) in [
            ("lower", contact),
            ("upper", config.plate_to_pcb + config.plate_thickness),
        ] {
            let pad_id = format!("gasket:{id}:{side}");
            result.case.bodies.push(make_body(
                config,
                document.revision,
                &pad_id,
                z,
                compressed,
                vec![Contour {
                    hole: false,
                    points: rectangle(c, *length, -pad_inner, pad_inner + width),
                }],
            ));
            result.stack.push(MechanicalStackLayer {
                id: pad_id,
                z,
                thickness: compressed,
            });
        }
    }
    if let Some(layer) = result.stack.iter_mut().find(|l| l.id == "bottom") {
        layer.z = bottom_z;
        layer.thickness = floor_z - bottom_z;
    }
    result.diagnostics.push(Finding { id:"mechanical:internal-foam-material".into(),severity:Severity::Warning,scope:Scope::Case,
        message:"Foam compression limits and material tolerances are unverified; vertical geometry uses the supplied preload and tolerance.".into(),target_ids:vec![] });
    motion::clear_foam(config, settings, result)?;
    motion::validate(config, settings, result, component_volumes)?;
    Ok(())
}
