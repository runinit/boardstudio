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
                    && middle >= *z
                    && middle < z + height
                {
                    material = combine(
                        &material,
                        &[points.iter().map(|p| [p.x, p.y]).collect()],
                        OverlayRule::Union,
                    );
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
                    return Err(format!(
                        "Moving {id} contacts rigid {} within its vertical travel/tolerance envelope.",
                        body.body.id
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn clear_foam(
    config: &MechanicalConfiguration,
    settings: &InternalGasketConfiguration,
    result: &mut MechanicalAssembly,
) -> Result<(), String> {
    let travel = config.gasket_travel.unwrap_or(0.3) + settings.tolerance;
    // Reserve fit clearance as well as the lateral tolerance used by validation.
    let margin =
        config.clearance.max(settings.tolerance) + settings.tolerance + OFFSET_GRID_ALLOWANCE;
    let supports: Vec<_> = result
        .case
        .bodies
        .iter()
        .filter(|body| matches!(body.body.id.as_str(), "bottom" | "retainer"))
        .flat_map(|body| body.body.features.iter().flatten())
        .filter_map(|feature| match feature {
            CaseFeature::SupportPrism {
                points, z, height, ..
            } => Some((points.clone(), *z, *z + *height)),
            _ => None,
        })
        .collect();
    for foam in result
        .case
        .bodies
        .iter_mut()
        .filter(|body| matches!(body.body.id.as_str(), "plate-foam" | "bottom-foam"))
    {
        let low = foam.body.z.unwrap_or(0.) - travel;
        let high = foam.body.z.unwrap_or(0.) + foam.body.thickness + travel;
        let mut exclusions = Vec::new();
        for (points, bottom, top) in &supports {
            if high > *bottom && low < *top {
                exclusions.extend(expanded_outline(
                    &[Contour {
                        points: points.clone(),
                        hole: false,
                    }],
                    margin,
                    result.revision,
                )?);
            }
        }
        if !exclusions.is_empty() {
            foam.contours = subtract_foam_exclusions(&foam.contours, &exclusions)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foam_clears_supports_only_within_its_swept_height() {
        let input: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../cad/bench/fixtures/internal-gasket-v1/rectangle.json"
        ))
        .unwrap();
        let mut document: ProjectDoc = serde_json::from_value(input["document"].clone()).unwrap();
        let contours: Vec<Contour> = serde_json::from_value(input["contours"].clone()).unwrap();
        document.mechanical.as_mut().unwrap().plate_foam_thickness = 3.0;
        document.mechanical.as_mut().unwrap().bottom_foam_thickness = 1.0;
        let config = document.mechanical.as_ref().unwrap();
        let settings = config.internal_gasket.as_ref().unwrap();
        let mut assembly = crate::mechanical::resolve(&document, &contours);
        assert!(!assembly.generation_blocked, "{:?}", assembly.diagnostics);
        let pcb = assembly.pcb_reference.clone();
        let plate = assembly.plate_contours.clone();
        let square = |x: f64| {
            vec![
                Vec2 { x, y: 20. },
                Vec2 { x: x + 4., y: 20. },
                Vec2 { x: x + 4., y: 24. },
                Vec2 { x, y: 24. },
            ]
        };
        assembly
            .case
            .bodies
            .iter_mut()
            .find(|b| b.body.id == "plate-foam")
            .unwrap()
            .contours
            .push(Contour {
                hole: true,
                points: square(10.),
            });
        assembly
            .case
            .bodies
            .iter_mut()
            .find(|b| b.body.id == "bottom")
            .unwrap()
            .body
            .features
            .as_mut()
            .unwrap()
            .extend([
                CaseFeature::SupportPrism {
                    id: "foam-support".into(),
                    points: square(30.),
                    z: 0.5,
                    height: 1.0,
                },
                CaseFeature::SupportPrism {
                    id: "bottom-foam-support".into(),
                    points: square(40.),
                    z: -2.5,
                    height: 0.5,
                },
                CaseFeature::SupportPrism {
                    id: "travel-only".into(),
                    points: square(60.),
                    z: 3.1,
                    height: 0.1,
                },
                CaseFeature::SupportPrism {
                    id: "above-foam".into(),
                    points: square(50.),
                    z: 4.,
                    height: 0.3,
                },
            ]);
        assert!(
            validate(config, settings, &assembly, &[])
                .unwrap_err()
                .contains("foam")
        );
        clear_foam(config, settings, &mut assembly).unwrap();
        let foam = assembly
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "plate-foam")
            .unwrap();
        assert!(
            foam.contours
                .iter()
                .any(|c| c.hole && c.points.iter().any(|p| p.x < 30. && p.x > 29.))
        );
        assert!(
            !foam
                .contours
                .iter()
                .any(|c| c.hole && c.points.iter().any(|p| p.x > 49. && p.x < 55.))
        );
        assert!(
            foam.contours
                .iter()
                .any(|c| c.hole && c.points.iter().any(|p| (p.x - 10.).abs() < 1e-5))
        );
        assert!(
            foam.contours
                .iter()
                .any(|c| c.hole && c.points.iter().any(|p| p.x < 60. && p.x > 59.))
        );
        let bottom_foam = assembly
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "bottom-foam")
            .unwrap();
        assert!(
            bottom_foam
                .contours
                .iter()
                .any(|c| c.hole && c.points.iter().any(|p| p.x < 40. && p.x > 39.))
        );
        assert_eq!(assembly.pcb_reference, pcb);
        assert_eq!(assembly.plate_contours, plate);
        validate(config, settings, &assembly, &[]).unwrap();
        assembly
            .case
            .bodies
            .iter_mut()
            .find(|b| b.body.id == "bottom")
            .unwrap()
            .body
            .features
            .as_mut()
            .unwrap()
            .push(CaseFeature::SupportPrism {
                id: "pcb-collision".into(),
                points: square(70.),
                z: -1.,
                height: 2.,
            });
        clear_foam(config, settings, &mut assembly).unwrap();
        assert!(
            validate(config, settings, &assembly, &[])
                .unwrap_err()
                .contains("pcb-reference")
        );
    }
}
