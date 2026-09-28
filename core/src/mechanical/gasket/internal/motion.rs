//! Exact planar intersections over all stationary Z bands and vertically swept moving prisms.
use super::*;

fn combine(a: &[FloatPath], b: &[FloatPath], rule: OverlayRule) -> Vec<FloatPath> {
    a.overlay(&b, rule, FillRule::EvenOdd)
        .into_iter()
        .flatten()
        .collect()
}

pub(super) fn validate(
    config: &MechanicalConfiguration,
    settings: &InternalGasketConfiguration,
    result: &MechanicalAssembly,
    components: &[(String, CaseOpening)],
) -> Result<(), String> {
    if settings.support_clearance.unwrap_or(config.clearance) <= 0. {
        return Err("PCB-to-support clearance permits rigid contact at the tolerance limit; choose a positive gap.".into());
    }
    let travel = config.gasket_travel.unwrap_or(0.3) + settings.tolerance;
    let mut moving: Vec<(String, Vec<FloatPath>, f64, f64)> = vec![];
    for body in result
        .case
        .bodies
        .iter()
        .filter(|b| matches!(b.body.id.as_str(), "plate" | "plate-foam" | "bottom-foam"))
        .chain(result.pcb_reference.iter())
    {
        let mut padded = body.clone();
        padded.body.clearance = settings.tolerance;
        padded.body.features = None;
        padded.body.mounts = None;
        padded.body.openings = None;
        let prepared = crate::case::prepare(&CaseAssemblyIR {
            revision: result.revision,
            bodies: vec![padded],
        })?;
        let contours: Vec<_> = prepared.bodies[0]
            .regions
            .iter()
            .flat_map(|r| std::iter::once(r.outer.clone()).chain(r.holes.clone()))
            .collect();
        let footprints = contours
            .iter()
            .map(|p| p.iter().map(|v| [v.x, v.y]).collect())
            .collect();
        let z = body.body.z.unwrap_or(0.);
        moving.push((
            body.body.id.clone(),
            footprints,
            z - travel,
            z + body.body.thickness + travel,
        ));
    }
    for (id, volume) in components {
        let expanded = expanded_outline(
            &[Contour {
                points: volume.points.clone(),
                hole: false,
            }],
            settings.tolerance,
            result.revision,
        )?;
        moving.push((
            id.clone(),
            paths(&expanded),
            volume.z - travel,
            volume.z + volume.height + travel,
        ));
    }
    let stationary: Vec<_> = result
        .case
        .bodies
        .iter()
        .filter(|b| matches!(b.body.id.as_str(), "bottom" | "retainer"))
        .cloned()
        .collect();
    let prepared = crate::case::prepare(&CaseAssemblyIR {
        revision: result.revision,
        bodies: stationary,
    })?;
    for body in prepared.bodies {
        let base = body.body.z.unwrap_or(0.);
        let floor = base + body.body.thickness;
        let top = floor + body.body.wall_height.unwrap_or(0.);
        let mut levels = vec![base, floor, top];
        for feature in body.body.features.iter().flatten() {
            if let CaseFeature::SupportPrism { z, height, .. } = feature {
                levels.extend([*z, *z + *height]);
            }
        }
        for opening in body.body.openings.iter().flatten() {
            levels.extend([opening.z, opening.z + opening.height]);
        }
        levels.sort_by(f64::total_cmp);
        levels.dedup();
        for band in levels.windows(2) {
            let middle = (band[0] + band[1]) / 2.;
            let mut material = vec![];
            if middle >= base && middle < top {
                for region in &body.regions {
                    let mut region_paths: Vec<FloatPath> = std::iter::once(&region.outer)
                        .chain(region.holes.iter())
                        .map(|p| p.iter().map(|v| [v.x, v.y]).collect())
                        .collect();
                    if body.body.kind == CaseKind::Tray && middle >= floor {
                        let cavities: Vec<FloatPath> = region
                            .cavities
                            .iter()
                            .map(|p| p.iter().map(|v| [v.x, v.y]).collect())
                            .collect();
                        region_paths = combine(&region_paths, &cavities, OverlayRule::Difference);
                    }
                    material = combine(&material, &region_paths, OverlayRule::Union);
                }
            }
            for feature in body.body.features.iter().flatten() {
                if let CaseFeature::SupportPrism {
                    points, z, height, ..
                } = feature
                {
                    if middle >= *z && middle < z + height {
                        material = combine(
                            &material,
                            &[points.iter().map(|p| [p.x, p.y]).collect()],
                            OverlayRule::Union,
                        );
                    }
                }
            }
            for opening in body.body.openings.iter().flatten() {
                if middle >= opening.z && middle < opening.z + opening.height {
                    material = combine(
                        &material,
                        &[opening.points.iter().map(|p| [p.x, p.y]).collect()],
                        OverlayRule::Difference,
                    );
                }
            }
            // Seats remove stationary material outside the moving assembly. Ignoring them
            // here is conservative and prevents treating a screw passage as motion clearance.
            for (id, footprint, low, high) in &moving {
                if *high > band[0] + 1e-8
                    && *low < band[1] - 1e-8
                    && !combine(&material, footprint, OverlayRule::Intersect).is_empty()
                {
                    return Err(format!("Moving {id} contacts rigid {} within its vertical travel/tolerance envelope.",body.body.id));
                }
            }
        }
    }
    Ok(())
}
