//! Source-only line/arc containment; classification never changes an authored curve.
use super::*;
use cavalier_contours::polyline::{
    PlineContainsOptions, PlineContainsResult, PlineSelfIntersectOptions, PlineSource,
    PlineSourceMut, Polyline,
};

// Match the exact PCB consumer's XY equality budget; this permits refusal, not repair.
const POSITION_EPS_MM: f64 = 1e-10;

pub(super) fn assign(contours: &mut [CasePcbSourceContour]) -> Result<(), ArtifactError> {
    // Keep the existing ModuleBoard contour budget at the source boundary too.
    if contours.len() > 1000 {
        return Err(validation("PCB source exceeds the 1000-contour limit"));
    }
    let rings = contours.iter().map(view).collect::<Result<Vec<_>, _>>()?;
    let indices: Vec<_> = rings
        .iter()
        .map(|ring| ring.create_approx_aabb_index())
        .collect();
    let mut parents = vec![Vec::new(); rings.len()];
    for i in 0..rings.len() {
        for j in i + 1..rings.len() {
            match rings[i].contains_opt(
                &rings[j],
                &PlineContainsOptions {
                    pline1_aabb_index: Some(&indices[i]),
                    pos_equal_eps: POSITION_EPS_MM,
                },
            ) {
                PlineContainsResult::Pline1InsidePline2 => parents[i].push(j),
                PlineContainsResult::Pline2InsidePline1 => parents[j].push(i),
                PlineContainsResult::Disjoint => {}
                _ => {
                    return Err(validation(format!(
                        "PCB source contours '{}' and '{}' intersect, touch or have ambiguous ownership",
                        contours[i].id, contours[j].id
                    )));
                }
            }
        }
    }
    for i in 0..contours.len() {
        if parents[i].len() % 2 == 0 {
            contours[i].outer_id = None;
            continue;
        }
        let owners: Vec<_> = parents[i]
            .iter()
            .filter(|&&parent| parents[parent].len() + 1 == parents[i].len())
            .copied()
            .collect();
        if owners.len() != 1 {
            return Err(validation(format!(
                "PCB source contour '{}' has ambiguous material ownership",
                contours[i].id
            )));
        }
        contours[i].outer_id = Some(contours[owners[0]].id.clone());
    }
    Ok(())
}

fn view(contour: &CasePcbSourceContour) -> Result<Polyline<f64>, ArtifactError> {
    let mut ring = Polyline::new_closed();
    for primitive in &contour.primitives {
        match &primitive.geometry {
            MechanicalShape::Line { start, .. } => ring.add(start.x, start.y, 0.),
            MechanicalShape::Circle { center, end, .. } if contour.primitives.len() == 1 => {
                ring.add(end.x, end.y, 1.);
                ring.add(
                    center.x - (end.x - center.x),
                    center.y - (end.y - center.y),
                    1.,
                );
            }
            MechanicalShape::Arc { start, .. } => {
                let support = primitive.curve_support.ok_or_else(|| {
                    validation(format!(
                        "PCB arc '{}' has no authored support",
                        primitive.source_id
                    ))
                })?;
                let count = (support.sweep_radians.abs() / std::f64::consts::PI).ceil() as usize;
                let span = support.sweep_radians / count as f64;
                let bulge = if span.abs() == std::f64::consts::PI {
                    span.signum()
                } else {
                    (span / 4.).tan()
                };
                for index in 0..count {
                    let point = if index == 0 {
                        *start
                    } else {
                        support_point(
                            &support,
                            support.start_direction.y.atan2(support.start_direction.x)
                                + span * index as f64,
                        )
                    };
                    ring.add(point.x, point.y, bulge);
                }
            }
            _ => {
                return Err(validation(format!(
                    "PCB contour '{}' is not normalized",
                    contour.id
                )));
            }
        }
    }
    if ring.vertex_count() < 2
        || !ring.area().is_finite()
        || ring.area() == 0.
        || ring.scan_for_self_intersect_opt(&PlineSelfIntersectOptions {
            pos_equal_eps: POSITION_EPS_MM,
            ..Default::default()
        })
    {
        return Err(validation(format!(
            "PCB source contour '{}' is degenerate, self-crossing or self-touching",
            contour.id
        )));
    }
    Ok(ring)
}
