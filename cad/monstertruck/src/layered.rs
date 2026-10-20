//! Private 013 prototype. Every operation before topology assembly is planar.
use crate::ExactModel;
use boardstudio_contracts::{CaseFeature, CaseKind, MountKind, PreparedCaseIR, Vec2};
mod contours;
mod pcb;
mod topology;
use contours::{Profile, Ring, difference, from_rings, polygon, union};

#[derive(Clone)]
struct Feature {
    id: String,
    ring: Ring,
    low: f64,
    high: f64,
    add: bool,
    cone: Option<([f64; 2], f64, f64)>,
    cutter: Option<Profile>,
}
impl Feature {
    fn at(&self, z: f64) -> Ring {
        match self.cone {
            Some((center, r0, r1)) => circle(
                center,
                r0 + (r1 - r0) * (z - self.low) / (self.high - self.low),
                &self.id,
            ),
            None => self.ring.clone(),
        }
    }
}
fn circle(center: [f64; 2], radius: f64, id: &str) -> Ring {
    let mut ring = Ring::new_closed();
    ring.add(center[0] + radius, center[1], 1.0);
    ring.add(center[0] - radius, center[1], 1.0);
    ring.sources
        .fill(Some(contours::Source::Circle(contours::CircleSource {
            id: id.to_owned(),
            center,
            radius,
        })));
    ring
}
fn feature(id: String, ring: Ring, low: f64, height: f64, add: bool) -> Result<Feature, String> {
    if !low.is_finite()
        || !height.is_finite()
        || height <= 0.
        || !(low + height).is_finite()
        || low + height <= low
    {
        return Err(format!("Feature '{id}' has invalid height or elevation"));
    }
    contours::validate(&ring).map_err(|e| format!("Feature '{id}': {e}"))?;
    Ok(Feature {
        id,
        ring,
        low,
        high: low + height,
        add,
        cone: None,
        cutter: None,
    })
}
fn circular_feature(
    id: String,
    center: [f64; 2],
    radius: f64,
    low: f64,
    height: f64,
    add: bool,
) -> Result<Feature, String> {
    let ring = circle(center, radius, &id);
    feature(id, ring, low, height, add)
}
/// Builds prepared case geometry without general 3D Booleans.
/// Rejects non-finite or beyond-limit ring points before any kernel call, which can panic on them.
fn check_ring_coordinates(input: &PreparedCaseIR) -> Result<(), String> {
    let limit = boardstudio_contracts::MAX_COORDINATE_MM;
    for region in &input.regions {
        let gaskets = region
            .gaskets
            .iter()
            .flat_map(|g| g.holes.iter().chain([&g.outer]));
        let rings = [&region.outer]
            .into_iter()
            .chain(&region.holes)
            .chain(&region.cavities)
            .chain(gaskets);
        for p in rings.flatten() {
            if !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > limit || p.y.abs() > limit {
                return Err(format!(
                    "Case body '{}' has a point ({}, {}) that is not finite or lies beyond {limit} mm",
                    input.body.id, p.x, p.y
                ));
            }
        }
    }
    Ok(())
}
pub(crate) fn build_layered_case(input: &PreparedCaseIR) -> Result<ExactModel, String> {
    check_ring_coordinates(input)?;
    if input.body.pcb_source.is_some() {
        return pcb::build(input);
    }
    if input.regions.is_empty() {
        return Err("Case requires at least one prepared region".into());
    }
    let body = &input.body;
    let base = body.z.unwrap_or(0.);
    let wall = if body.kind == CaseKind::Plate {
        0.
    } else {
        body.wall_height.unwrap_or(0.)
    };
    let total = body.thickness + wall;
    if !base.is_finite()
        || !body.thickness.is_finite()
        || body.thickness <= 0.
        || !wall.is_finite()
        || wall < 0.
        || !(base + total).is_finite()
        || base + total <= base
    {
        return Err(format!(
            "Case body '{}' has invalid height or elevation",
            body.id
        ));
    }
    for item in body.features.iter().flatten() {
        if let CaseFeature::SupportPrism { id, points, .. } = item {
            let points: Vec<_> = points.iter().map(|p| [p.x, p.y]).collect();
            if boardstudio_contracts::support_polygon_has_self_intersection(&points) {
                return Err(format!(
                    "Support '{id}' polygon must not self-intersect or self-touch"
                ));
            }
            let owners = input
                .regions
                .iter()
                .map(|r| {
                    let exterior: Vec<_> = r.outer.iter().map(|p| [p.x, p.y]).collect();
                    boardstudio_contracts::support_polygon_outside_area_below_owner_cutoff(
                        &points, &exterior,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Support '{id}': {e}"))?;
            if owners.iter().filter(|&&owned| owned).count() != 1 {
                return Err(format!(
                    "Support '{id}' must remain inside exactly one case exterior"
                ));
            }
        }
    }
    let mut solids = Vec::new();
    for (region_index, region) in input.regions.iter().enumerate() {
        let exterior: Vec<_> = region.outer.iter().map(|p| [p.x, p.y]).collect();
        let holes: Vec<Vec<_>> = region
            .holes
            .iter()
            .map(|h| h.iter().map(|p| [p.x, p.y]).collect())
            .collect();
        let (outer, holes) =
            boardstudio_contracts::normalize_case_profile_region(&exterior, &holes)
                .map_err(|e| format!("Case profile {e}"))?;
        let convert = |p: Vec<[f64; 2]>| {
            p.into_iter()
                .map(|p| Vec2 { x: p[0], y: p[1] })
                .collect::<Vec<_>>()
        };
        let mut rings = vec![(polygon(&convert(outer))?, true)];
        for h in holes {
            rings.push((polygon(&convert(h))?, false));
        }
        let initial = from_rings(rings)?;
        let mut features = Vec::new();
        if body.kind != CaseKind::Plate {
            let cavity_z = if body.kind == CaseKind::Tray {
                base + body.thickness
            } else {
                base
            };
            for (i, c) in region.cavities.iter().enumerate() {
                features.push(feature(
                    format!("cavity-{i}"),
                    polygon(c)?,
                    cavity_z,
                    wall,
                    false,
                )?);
            }
            for (i, g) in region.gaskets.iter().enumerate() {
                let depth = body
                    .gasket
                    .as_ref()
                    .ok_or("Prepared gasket has no body gasket dimensions")?
                    .depth;
                let z = if body.kind == CaseKind::Tray {
                    base + total - depth
                } else {
                    base
                };
                let mut gasket =
                    feature(format!("gasket-{i}"), polygon(&g.outer)?, z, depth, false)?;
                let mut cutter = from_rings(vec![(gasket.ring.clone(), true)])?;
                for hole in &g.holes {
                    cutter = difference(&cutter, &polygon(hole)?)
                        .map_err(|error| format!("Gasket 'gasket-{i}': {error}"))?;
                }
                gasket.cutter = Some(cutter);
                features.push(gasket);
            }
        }
        let mut hole_low = base;
        let mut hole_high = base + total;
        let mut bosses = Vec::with_capacity(region.mounts.len());
        for m in &region.mounts {
            let boss = if m.kind == MountKind::Boss {
                let height = m
                    .height
                    .ok_or_else(|| format!("Boss '{}' has no height", m.id))?;
                let diameter = m
                    .boss_diameter
                    .ok_or_else(|| format!("Boss '{}' has no diameter", m.id))?;
                // A case boss must be wider than its hole, matching core's case preparation.
                if !diameter.is_finite() || diameter <= m.hole_diameter {
                    return Err(format!("Boss '{}' must be wider than its hole", m.id));
                }
                let z = if body.kind == CaseKind::Lid {
                    base + wall - height
                } else {
                    base + body.thickness
                };
                hole_low = hole_low.min(z);
                hole_high = hole_high.max(z + height);
                Some(circular_feature(
                    m.id.clone(),
                    [m.at.x, m.at.y],
                    diameter / 2.,
                    z,
                    height,
                    true,
                )?)
            } else {
                None
            };
            bosses.push(boss);
        }
        // Preserve authored interleaving; cuts may otherwise widen a later boss hole.
        for (m, boss) in region.mounts.iter().zip(bosses) {
            if let Some(boss) = boss {
                features.push(boss);
            }
            if !m.hole_diameter.is_finite() || m.hole_diameter <= 0. {
                return Err(format!("Mount '{}' has invalid diameter", m.id));
            }
            features.push(circular_feature(
                m.id.clone(),
                [m.at.x, m.at.y],
                m.hole_diameter / 2.,
                hole_low,
                hole_high - hole_low,
                false,
            )?);
        }
        for item in body.features.iter().flatten() {
            if let CaseFeature::SupportPrism {
                id,
                points,
                z,
                height,
            } = item
            {
                let coordinates: Vec<_> = points.iter().map(|p| [p.x, p.y]).collect();
                if boardstudio_contracts::support_polygon_outside_area_below_owner_cutoff(
                    &coordinates,
                    &exterior,
                )
                .map_err(|e| format!("Support '{id}': {e}"))?
                {
                    features.push(feature(id.clone(), polygon(points)?, *z, *height, true)?);
                }
            }
        }
        for item in body.features.iter().flatten() {
            match item {
                CaseFeature::RoundSeat {
                    id,
                    at,
                    z,
                    height,
                    diameter,
                } => {
                    if !diameter.is_finite() || *diameter <= 0. {
                        return Err(format!("Seat '{id}' has invalid diameter"));
                    }
                    features.push(circular_feature(
                        id.clone(),
                        [at.x, at.y],
                        diameter / 2.,
                        *z,
                        *height,
                        false,
                    )?);
                }
                CaseFeature::ConicalSeat {
                    id,
                    at,
                    z,
                    height,
                    diameter,
                    end_diameter,
                } => {
                    if !diameter.is_finite()
                        || !end_diameter.is_finite()
                        || *end_diameter <= 0.
                        || diameter <= end_diameter
                    {
                        return Err(format!("Conical seat '{id}' has invalid dimensions"));
                    }
                    let mut f = circular_feature(
                        id.clone(),
                        [at.x, at.y],
                        diameter / 2.,
                        *z,
                        *height,
                        false,
                    )?;
                    f.cone = Some(([at.x, at.y], diameter / 2., end_diameter / 2.));
                    features.push(f);
                }
                CaseFeature::SupportPrism { .. } => {}
            }
        }
        let before_openings = features.len();
        for (i, o) in body.openings.iter().flatten().enumerate() {
            features.push(feature(
                format!("opening-{i}"),
                polygon(&o.points)?,
                o.z,
                o.height,
                false,
            )?);
        }
        let mut heights = vec![base, base + total];
        for f in &features {
            heights.extend([f.low, f.high]);
        }
        heights.sort_by(f64::total_cmp);
        heights.dedup_by(|a, b| *a == *b);
        let frame = contours::frame(
            initial
                .iter()
                .flat_map(|r| std::iter::once(&r.outer).chain(r.holes.iter()))
                .chain(features.iter().map(|f| &f.ring)),
        )?;
        let profile = |band: usize, z: f64, count: usize| -> Result<Profile, String> {
            let low = heights[band];
            let high = heights[band + 1];
            let mut p = if low >= base && high <= base + total {
                initial.clone()
            } else {
                vec![]
            };
            for f in &features[..count] {
                if !(low >= f.low && high <= f.high) {
                    continue;
                }
                if f.cone.is_some() {
                    contours::check_contained_cone(&p, f, low, high)
                        .map_err(|e| format!("Conical seat '{}': {e}", f.id))?;
                }
                p = if let Some(cutter) = &f.cutter {
                    contours::subtract_profile(&p, cutter, &frame)
                } else if f.add {
                    union(&p, &f.at(z), &frame)
                } else {
                    difference(&p, &f.at(z))
                }
                .map_err(|e| format!("Feature '{}': {e}", f.id))?;
            }
            Ok(p)
        };
        let mut bands = Vec::new();
        let mut before = Vec::new();
        for i in 0..heights.len() - 1 {
            let low = heights[i];
            let high = heights[i + 1];
            bands.push(topology::Band {
                low,
                high,
                initial: low >= base && high <= base + total,
                active: features
                    .iter()
                    .map(|f| low >= f.low && high <= f.high)
                    .collect(),
                lower: profile(i, low, features.len())?,
                upper: profile(i, high, features.len())?,
            });
            if before_openings != features.len() {
                before.push(topology::Band {
                    low,
                    high,
                    initial: low >= base && high <= base + total,
                    active: features
                        .iter()
                        .enumerate()
                        .map(|(n, f)| n < before_openings && low >= f.low && high <= f.high)
                        .collect(),
                    lower: profile(i, low, before_openings)?,
                    upper: profile(i, high, before_openings)?,
                });
            }
        }
        if !before.is_empty() && topology::assemble(&before, &features, &frame)?.len() != 1 {
            return Err(format!(
                "Region {region_index}: features leave disconnected material before openings"
            ));
        }
        let built = topology::assemble(&bands, &features, &frame).map_err(|error| {
            let ids = features
                .iter()
                .map(|feature| feature.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Region {region_index}, features [{ids}]: {error}")
        })?;
        if before_openings == features.len() && !features.is_empty() && built.len() != 1 {
            return Err(format!(
                "Region {region_index}: features leave disconnected material"
            ));
        }
        solids.extend(built);
    }
    if solids.is_empty() {
        return Err("Layered case has no material".into());
    }
    Ok(ExactModel { solids })
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod pcb_tests;
