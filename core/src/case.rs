//! Pure case contour preparation for the CAD adapter.
//!
//! Coordinates are quantized to the native i_overlay grid at 1000 units/mm.

use crate::model::{
    CaseAssemblyIR, CaseIR, CaseKind, Contour, PreparedCaseAssemblyIR, PreparedCaseIR,
    PreparedCaseRegion, PreparedGasketRegion, Vec2,
};
use i_overlay::i_float::int::{angle::Angle, point::IntPoint};
use i_overlay::mesh::{
    int::{
        outline::offset::IntOutlineOffset,
        style::{IntLineJoin, IntOutlineStyle},
    },
    math::MathMode,
};
use std::f64::consts::PI;

const SCALE: f64 = 1_000.0;
const MAX_SCALED_COORD: f64 = 1_000_000_000.0;
const MITER_LIMIT: f64 = 0.505_360_510_284_157_3;
const MITER_MIN_TURN: f64 = 5.0 * PI / 180.0;

/// Prepare contours and derived shell cuts without mutating engine state.
pub fn prepare(ir: &CaseAssemblyIR) -> Result<PreparedCaseAssemblyIR, String> {
    if ir.bodies.is_empty() {
        return Err("Case assembly requires at least one body".into());
    }

    let mut bodies = Vec::with_capacity(ir.bodies.len());
    for (index, body) in ir.bodies.iter().enumerate() {
        if body.revision != ir.revision {
            return Err(format!("Case assembly body {index} has a stale revision"));
        }
        bodies.push(prepare_body(body)?);
    }

    Ok(PreparedCaseAssemblyIR {
        revision: ir.revision,
        bodies,
    })
}

fn prepare_body(ir: &CaseIR) -> Result<PreparedCaseIR, String> {
    validate_body(ir)?;
    let regions = if ir.body.clearance == 0.0 {
        group_unoffset_contours(clean_unoffset_contours(&ir.contours, &ir.body.id)?)
    } else {
        let input = normalize_contours(&ir.contours, &ir.body.id)?;
        offset_shapes(&input, ir.body.clearance, &ir.body.id)?
            .into_iter()
            .filter_map(|shape| {
                let outer = shape.first()?;
                Some((
                    from_path(outer),
                    shape
                        .iter()
                        .skip(1)
                        .filter(|path| signed_area_i128(path) < 0)
                        .map(|path| from_path(path))
                        .collect(),
                ))
            })
            .collect()
    };
    if regions.is_empty() {
        return Err(format!(
            "Case body '{}' requires at least one outer contour after clearance",
            ir.body.id
        ));
    }

    for feature in ir.body.features.iter().flatten() {
        if let crate::model::CaseFeature::SupportPrism { id, points, .. } = feature {
            use i_overlay::{
                core::{fill_rule::FillRule, overlay_rule::OverlayRule},
                float::single::SingleFloatOverlay,
            };
            let footprint: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
            let owners = regions
                .iter()
                .filter(|(outer, _)| {
                    let boundary: Vec<[f64; 2]> = outer.iter().map(|p| [p.x, p.y]).collect();
                    footprint
                        .overlay(&boundary, OverlayRule::Difference, FillRule::EvenOdd)
                        .is_empty()
                })
                .count();
            if owners != 1 {
                return Err(format!(
                    "Support feature '{id}' must remain inside one case exterior"
                ));
            }
        }
    }

    let mut prepared = Vec::with_capacity(regions.len());
    for (outer, holes) in regions {
        let mut cavities = Vec::new();
        let mut gaskets = Vec::new();
        if !matches!(&ir.body.kind, CaseKind::Plate) {
            let wall = ir.body.wall_thickness.expect("validated wall thickness");
            cavities = offset_single(&outer, -wall, &ir.body.id)?;
            if let Some(gasket) = &ir.body.gasket {
                let groove_outer = offset_single(&outer, -gasket.inset, &ir.body.id)?;
                let groove_inner =
                    offset_single(&outer, -(gasket.inset + gasket.width), &ir.body.id)?;
                for groove in groove_outer {
                    let holes = groove_inner
                        .iter()
                        .filter(|inner| {
                            inner.first().is_some_and(|point| contains(&groove, *point))
                        })
                        .cloned()
                        .collect();
                    gaskets.push(PreparedGasketRegion {
                        outer: groove,
                        holes,
                    });
                }
            }
        }

        let mounts = ir
            .body
            .mounts
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|mount| contains(&outer, mount.at))
            .cloned()
            .collect::<Vec<_>>();
        prepared.push(PreparedCaseRegion {
            outer,
            holes,
            cavities,
            gaskets,
            mounts,
        });
    }

    Ok(PreparedCaseIR {
        revision: ir.revision,
        body: ir.body.clone(),
        regions: prepared,
    })
}

fn clean_unoffset_contours(
    contours: &[Contour],
    id: &str,
) -> Result<Vec<(Vec<Vec2>, bool)>, String> {
    if contours.is_empty() {
        return Err(format!(
            "Case body '{id}' requires at least one outer contour"
        ));
    }
    let mut cleaned = Vec::with_capacity(contours.len());
    for contour in contours {
        let mut points = Vec::with_capacity(contour.points.len());
        for point in &contour.points {
            if !valid_point(*point) {
                return Err(format!(
                    "Case body '{id}' contour contains a non-finite or out-of-range coordinate"
                ));
            }
            if points
                .last()
                .is_none_or(|last: &Vec2| last.x != point.x || last.y != point.y)
            {
                points.push(*point);
            }
        }
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if points.len() < 3 || signed_area_float(&points) == 0.0 {
            return Err(format!(
                "Case body '{id}' contour has fewer than three non-collinear points"
            ));
        }
        let wants_positive = !contour.hole;
        if (signed_area_float(&points) > 0.0) != wants_positive {
            points.reverse();
        }
        cleaned.push((points, contour.hole));
    }
    if !cleaned.iter().any(|(_, hole)| !hole) {
        return Err(format!(
            "Case body '{id}' requires at least one outer contour"
        ));
    }
    Ok(cleaned)
}

fn group_unoffset_contours(contours: Vec<(Vec<Vec2>, bool)>) -> Vec<(Vec<Vec2>, Vec<Vec<Vec2>>)> {
    let outers = contours
        .iter()
        .filter(|(_, hole)| !hole)
        .map(|(points, _)| points.clone())
        .collect::<Vec<_>>();
    let mut regions = outers
        .iter()
        .cloned()
        .map(|outer| (outer, Vec::new()))
        .collect::<Vec<_>>();
    for (hole, is_hole) in contours {
        if !is_hole {
            continue;
        }
        let Some(point) = hole.first().copied() else {
            continue;
        };
        let parent = regions
            .iter()
            .enumerate()
            .filter(|(_, (outer, _))| contains(outer, point))
            .min_by(|(_, (a, _)), (_, (b, _))| area_float(a).total_cmp(&area_float(b)))
            .map(|(index, _)| index);
        if let Some(index) = parent {
            regions[index].1.push(hole);
        }
    }
    regions
}

fn validate_body(ir: &CaseIR) -> Result<(), String> {
    let body = &ir.body;
    let id = &body.id;
    if !body.thickness.is_finite() || body.thickness <= 0.0 {
        return Err(format!("Case body '{id}' thickness must be positive"));
    }
    if !body.clearance.is_finite() || body.clearance < 0.0 {
        return Err(format!("Case body '{id}' clearance must be non-negative"));
    }
    if body.z.is_some_and(|z| !z.is_finite()) {
        return Err(format!("Case body '{id}' z must be finite"));
    }
    if !matches!(&body.kind, CaseKind::Plate) {
        let (Some(height), Some(thickness)) = (body.wall_height, body.wall_thickness) else {
            return Err(format!(
                "Case body '{id}' tray and lid require wall height and thickness"
            ));
        };
        if !height.is_finite() || !thickness.is_finite() || height <= 0.0 || thickness <= 0.0 {
            return Err(format!(
                "Case body '{id}' wall height and thickness must be positive"
            ));
        }
        if let Some(gasket) = &body.gasket
            && (!gasket.inset.is_finite()
                || !gasket.width.is_finite()
                || !gasket.depth.is_finite()
                || gasket.inset < 0.0
                || gasket.width <= 0.0
                || gasket.depth <= 0.0
                || gasket.inset + gasket.width >= thickness
                || gasket.depth >= height)
        {
            return Err(format!(
                "Case body '{id}' gasket groove must fit within the wall rim"
            ));
        }
    }
    let mut feature_ids = std::collections::HashSet::new();
    for feature in body.features.iter().flatten() {
        use crate::model::CaseFeature;
        let (feature_id, z, height, valid) = match feature {
            CaseFeature::SupportPrism {
                id,
                points,
                z,
                height,
            } => (
                id,
                *z,
                *height,
                points.len() >= 3
                    && points.iter().all(|p| valid_point(*p))
                    && area_float(points) > 0.000001,
            ),
            CaseFeature::ConicalSeat {
                id,
                at,
                z,
                height,
                diameter,
                end_diameter,
            } => (
                id,
                *z,
                *height,
                valid_point(*at)
                    && diameter.is_finite()
                    && *diameter > 0.
                    && end_diameter.is_finite()
                    && *end_diameter > 0.
                    && *end_diameter < *diameter,
            ),
            CaseFeature::RoundSeat {
                id,
                at,
                z,
                height,
                diameter,
            } => (
                id,
                *z,
                *height,
                valid_point(*at) && diameter.is_finite() && *diameter > 0.,
            ),
        };
        if feature_id.is_empty()
            || !feature_ids.insert(feature_id)
            || !valid
            || !z.is_finite()
            || !height.is_finite()
            || height <= 0.
        {
            return Err(format!(
                "Case body '{id}' feature '{feature_id}' requires a unique ID and finite positive geometry"
            ));
        }
    }
    for opening in body.openings.as_deref().unwrap_or_default() {
        if !opening.z.is_finite()
            || !opening.height.is_finite()
            || opening.height <= 0.0
            || opening.points.len() < 3
            || opening.points.iter().any(|point| !valid_point(*point))
            || area_float(&opening.points).abs() < 0.000001
        {
            return Err(format!(
                "Case body '{id}' opening must have a finite polygon, Z and positive height"
            ));
        }
    }
    for mount in body.mounts.as_deref().unwrap_or_default() {
        if !valid_point(mount.at) {
            return Err(format!(
                "Case body '{id}' mount '{}' has an invalid coordinate",
                mount.id
            ));
        }
        if !mount.hole_diameter.is_finite() || mount.hole_diameter <= 0.0 {
            return Err(format!(
                "Case body '{id}' mount '{}' hole diameter must be positive",
                mount.id
            ));
        }
        if let crate::model::MountKind::Boss = &mount.kind {
            let (Some(diameter), Some(height)) = (mount.boss_diameter, mount.height) else {
                return Err(format!(
                    "Case body '{id}' boss '{}' requires diameter and height",
                    mount.id
                ));
            };
            if !diameter.is_finite()
                || !height.is_finite()
                || diameter <= mount.hole_diameter
                || height <= 0.0
            {
                return Err(format!(
                    "Case body '{id}' boss '{}' diameter and height must exceed its hole and zero",
                    mount.id
                ));
            }
        }
    }
    Ok(())
}

fn normalize_contours(contours: &[Contour], id: &str) -> Result<Vec<Vec<IntPoint<i64>>>, String> {
    if contours.is_empty() {
        return Err(format!(
            "Case body '{id}' requires at least one outer contour"
        ));
    }
    let mut paths = Vec::with_capacity(contours.len());
    for contour in contours {
        let mut points = Vec::with_capacity(contour.points.len());
        for point in &contour.points {
            if !valid_point(*point) {
                return Err(format!(
                    "Case body '{id}' contour contains a non-finite or out-of-range coordinate"
                ));
            }
            let point = IntPoint::new(
                (point.x * SCALE).round() as i64,
                (point.y * SCALE).round() as i64,
            );
            if points
                .last()
                .is_none_or(|last: &IntPoint<i64>| *last != point)
            {
                points.push(point);
            }
        }
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if points.len() < 3 || signed_area_i128(&points) == 0 {
            return Err(format!(
                "Case body '{id}' contour has fewer than three non-collinear points"
            ));
        }
        let wants_positive = !contour.hole;
        if (signed_area_i128(&points) > 0) != wants_positive {
            points.reverse();
        }
        paths.push(points);
    }
    if !contours.iter().any(|contour| !contour.hole) {
        return Err(format!(
            "Case body '{id}' requires at least one outer contour"
        ));
    }
    Ok(paths)
}

fn valid_point(point: Vec2) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() * SCALE <= MAX_SCALED_COORD
        && point.y.abs() * SCALE <= MAX_SCALED_COORD
}

fn offset_shapes(
    paths: &[Vec<IntPoint<i64>>],
    distance: f64,
    id: &str,
) -> Result<Vec<Vec<Vec<IntPoint<i64>>>>, String> {
    let distance = scaled_distance(distance, id, "offset")?;
    let style = outline_style(distance);
    let result = paths
        .outline(&style)
        .map_err(|error| format!("Case body '{id}' contour offset failed: {error}"))?;
    if result.iter().flatten().flatten().any(|point| {
        (point.x as f64).abs() > MAX_SCALED_COORD || (point.y as f64).abs() > MAX_SCALED_COORD
    }) {
        return Err(format!(
            "Case body '{id}' offset exceeds the supported coordinate range"
        ));
    }
    Ok(result
        .into_iter()
        .filter(|shape| shape.first().is_some_and(|path| path.len() >= 3))
        .collect())
}

fn offset_single(path: &[Vec2], distance: f64, id: &str) -> Result<Vec<Vec<Vec2>>, String> {
    if distance == 0.0 {
        return Ok(vec![path.to_vec()]);
    }
    let scaled = path
        .iter()
        .map(|point| {
            IntPoint::new(
                (point.x * SCALE).round() as i64,
                (point.y * SCALE).round() as i64,
            )
        })
        .collect::<Vec<_>>();
    let distance = scaled_distance(distance, id, "derived offset")?;
    let style = outline_style(distance);
    let shapes = scaled
        .outline(&style)
        .map_err(|error| format!("Case body '{id}' derived contour offset failed: {error}"))?;
    if shapes.iter().flatten().flatten().any(|point| {
        (point.x as f64).abs() > MAX_SCALED_COORD || (point.y as f64).abs() > MAX_SCALED_COORD
    }) {
        return Err(format!(
            "Case body '{id}' derived offset exceeds the supported coordinate range"
        ));
    }
    Ok(shapes
        .into_iter()
        .filter_map(|shape| {
            shape
                .first()
                .filter(|contour| contour.len() >= 3)
                .map(|contour| from_path(contour))
        })
        .collect())
}

fn scaled_distance(distance: f64, id: &str, label: &str) -> Result<i64, String> {
    if !distance.is_finite() || distance.abs() * SCALE > MAX_SCALED_COORD {
        return Err(format!(
            "Case body '{id}' {label} exceeds the supported coordinate range"
        ));
    }
    Ok((distance * SCALE).round() as i64)
}

fn outline_style(offset: i64) -> IntOutlineStyle<i64> {
    let miter = Angle::from_radians(MITER_LIMIT).expect("miter limit is a valid angle");
    let min_turn = Angle::from_radians(MITER_MIN_TURN).expect("minimum turn is a valid angle");
    IntOutlineStyle::new(offset)
        .math(MathMode::Float)
        .line_join(IntLineJoin::Miter(miter))
        .miter_min_turn(min_turn)
}

fn from_path(path: &[IntPoint<i64>]) -> Vec<Vec2> {
    path.iter()
        .map(|point| Vec2 {
            x: point.x as f64 / SCALE,
            y: point.y as f64 / SCALE,
        })
        .collect()
}

fn signed_area_i128(path: &[IntPoint<i64>]) -> i128 {
    let Some(origin) = path.first() else { return 0 };
    let (ox, oy) = (origin.x as i128, origin.y as i128);
    let mut area = 0_i128;
    for index in 0..path.len() {
        let a = path[index];
        let b = path[(index + 1) % path.len()];
        area += (a.x as i128 - ox) * (b.y as i128 - oy) - (b.x as i128 - ox) * (a.y as i128 - oy);
    }
    area
}

fn signed_area_float(path: &[Vec2]) -> f64 {
    let Some(origin) = path.first() else {
        return 0.0;
    };
    let mut area = 0.0;
    for index in 0..path.len() {
        let a = Vec2 {
            x: path[index].x - origin.x,
            y: path[index].y - origin.y,
        };
        let next = path[(index + 1) % path.len()];
        let b = Vec2 {
            x: next.x - origin.x,
            y: next.y - origin.y,
        };
        area += a.x * b.y - b.x * a.y;
    }
    area
}

fn area_float(path: &[Vec2]) -> f64 {
    signed_area_float(path).abs()
}

fn contains(path: &[Vec2], point: Vec2) -> bool {
    let mut inside = false;
    for index in 0..path.len() {
        let a = path[index];
        let b = path[(index + path.len() - 1) % path.len()];
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CaseBody, Gasket, Mount, MountKind};

    fn rectangle(x: f64, y: f64, width: f64, height: f64, hole: bool) -> Contour {
        Contour {
            points: vec![
                Vec2 { x, y },
                Vec2 { x: x + width, y },
                Vec2 {
                    x: x + width,
                    y: y + height,
                },
                Vec2 { x, y: y + height },
            ],
            hole,
        }
    }

    fn input(kind: CaseKind, contours: Vec<Contour>) -> CaseIR {
        CaseIR {
            revision: 7,
            body: CaseBody {
                features: None,
                openings: None,
                id: "case-a".into(),
                name: "Case A".into(),
                board_id: "board-a".into(),
                kind,
                thickness: 2.0,
                clearance: 0.0,
                material_id: None,
                z: None,
                wall_height: Some(10.0),
                wall_thickness: Some(2.0),
                mounts: None,
                gasket: None,
            },
            contours,
        }
    }

    fn assembly(body: CaseIR) -> CaseAssemblyIR {
        CaseAssemblyIR {
            revision: 7,
            bodies: vec![body],
        }
    }

    #[test]
    fn zero_clearance_preserves_rectangle_geometry_and_reversed_winding() {
        let mut body = input(
            CaseKind::Plate,
            vec![rectangle(0.0, 0.0, 30.0, 20.0, false)],
        );
        body.contours[0].points.reverse();
        let prepared = prepare(&assembly(body)).unwrap();
        assert_eq!(prepared.bodies[0].regions.len(), 1);
        let outer = &prepared.bodies[0].regions[0].outer;
        assert_eq!(outer.len(), 4);
        assert!((area(outer) - 600.0).abs() < 0.01);

        let fractional = input(
            CaseKind::Plate,
            vec![Contour {
                points: vec![
                    Vec2 {
                        x: 0.0004,
                        y: 0.0007,
                    },
                    Vec2 {
                        x: 10.0004,
                        y: 0.0007,
                    },
                    Vec2 {
                        x: 10.0004,
                        y: 10.0007,
                    },
                    Vec2 {
                        x: 0.0004,
                        y: 10.0007,
                    },
                ],
                hole: false,
            }],
        );
        let output = prepare(&assembly(fractional)).unwrap();
        assert!(
            output.bodies[0].regions[0]
                .outer
                .iter()
                .any(|point| point.x == 0.0004)
        );
    }

    #[test]
    fn tiny_contour_near_coordinate_bound_keeps_its_area_and_winding() {
        let body = input(
            CaseKind::Plate,
            vec![rectangle(999_999.0, 999_999.0, 0.001, 0.001, false)],
        );
        let result = prepare(&assembly(body)).unwrap();
        let outer = &result.bodies[0].regions[0].outer;
        assert_eq!(outer.len(), 4);
        assert!(signed_area_float(outer) > 0.0);
        assert!((signed_area_float(outer) / 2.0 - 1.0e-6).abs() < 1.0e-12);
    }

    #[test]
    fn clearance_offsets_concave_and_sharp_contours_with_bounded_miters() {
        let mut body = input(
            CaseKind::Plate,
            vec![Contour {
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 20.0, y: 0.0 },
                    Vec2 { x: 20.0, y: 20.0 },
                    Vec2 { x: 10.1, y: 1.0 },
                    Vec2 { x: 0.0, y: 20.0 },
                ],
                hole: false,
            }],
        );
        body.body.clearance = 1.0;
        let region = &prepare(&assembly(body)).unwrap().bodies[0].regions[0];
        assert!(area(&region.outer) > 210.0);
        assert!(
            region
                .outer
                .iter()
                .all(|point| point.x.abs() < 30.0 && point.y.abs() < 30.0)
        );
    }

    #[test]
    fn holes_and_nested_islands_are_grouped_by_native_shape_output() {
        let mut body = input(
            CaseKind::Plate,
            vec![
                rectangle(0.0, 0.0, 40.0, 40.0, false),
                rectangle(10.0, 10.0, 20.0, 20.0, true),
                rectangle(15.0, 15.0, 10.0, 10.0, false),
            ],
        );
        body.body.clearance = 0.2;
        let prepared = prepare(&assembly(body)).unwrap();
        assert_eq!(prepared.bodies[0].regions.len(), 2);
        let mut regions = prepared.bodies[0].regions.iter().collect::<Vec<_>>();
        regions.sort_by(|a, b| area(&b.outer).total_cmp(&area(&a.outer)));
        assert_eq!(regions[0].holes.len(), 1);
        assert!((area(&regions[0].outer) - 40.4 * 40.4).abs() < 0.1);
        assert!((area(&regions[0].holes[0]) - 19.6 * 19.6).abs() < 0.1);
        assert!(regions[1].holes.is_empty());
        assert!((area(&regions[1].outer) - 10.4 * 10.4).abs() < 0.1);
    }

    #[test]
    fn disjoint_outers_and_collapsing_cavities_are_reported_as_regions() {
        let mut body = input(
            CaseKind::Tray,
            vec![
                rectangle(0.0, 0.0, 8.0, 8.0, false),
                rectangle(20.0, 0.0, 8.0, 8.0, false),
            ],
        );
        body.body.clearance = 0.1;
        body.body.wall_thickness = Some(5.0);
        let prepared = prepare(&assembly(body)).unwrap();
        assert_eq!(prepared.bodies[0].regions.len(), 2);
        assert!(
            prepared.bodies[0]
                .regions
                .iter()
                .all(|region| region.cavities.is_empty())
        );
    }

    #[test]
    fn inward_offset_splits_a_narrow_neck_into_two_cavities() {
        let mut body = input(
            CaseKind::Tray,
            vec![Contour {
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 4.0 },
                    Vec2 { x: 20.0, y: 4.0 },
                    Vec2 { x: 20.0, y: 0.0 },
                    Vec2 { x: 30.0, y: 0.0 },
                    Vec2 { x: 30.0, y: 10.0 },
                    Vec2 { x: 20.0, y: 10.0 },
                    Vec2 { x: 20.0, y: 6.0 },
                    Vec2 { x: 10.0, y: 6.0 },
                    Vec2 { x: 10.0, y: 10.0 },
                    Vec2 { x: 0.0, y: 10.0 },
                ],
                hole: false,
            }],
        );
        body.body.wall_thickness = Some(1.1);
        let prepared = prepare(&assembly(body)).unwrap();
        assert_eq!(prepared.bodies[0].regions.len(), 1);
        assert_eq!(prepared.bodies[0].regions[0].cavities.len(), 2);
    }

    #[test]
    fn gasket_ring_bounds_match_inset_and_width() {
        let mut body = input(CaseKind::Tray, vec![rectangle(0.0, 0.0, 40.0, 30.0, false)]);
        body.body.gasket = Some(Gasket {
            inset: 0.2,
            width: 0.4,
            depth: 1.0,
        });
        let prepared = prepare(&assembly(body)).unwrap();
        let gasket = &prepared.bodies[0].regions[0].gaskets[0];
        assert!((area(&gasket.outer) - 39.6 * 29.6).abs() < 0.1);
        assert_eq!(gasket.holes.len(), 1);
        assert!((area(&gasket.holes[0]) - 38.8 * 28.8).abs() < 0.1);
    }

    #[test]
    fn near_straight_vertices_use_the_configured_bevel_cutoff() {
        let mut body = input(
            CaseKind::Plate,
            vec![Contour {
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 10.0, y: 0.0 },
                    Vec2 { x: 20.0, y: 0.001 },
                    Vec2 { x: 20.0, y: 10.0 },
                    Vec2 { x: 0.0, y: 10.0 },
                ],
                hole: false,
            }],
        );
        body.body.clearance = 0.2;
        let prepared = prepare(&assembly(body)).unwrap();
        assert_eq!(prepared.bodies[0].regions.len(), 1);
        assert!(
            prepared.bodies[0].regions[0]
                .outer
                .iter()
                .all(|point| point.x.abs() < 21.0)
        );
    }

    #[test]
    fn gasket_grooves_are_nested_and_mounts_outside_are_removed() {
        let mut body = input(CaseKind::Tray, vec![rectangle(0.0, 0.0, 40.0, 30.0, false)]);
        body.body.gasket = Some(Gasket {
            inset: 0.2,
            width: 0.4,
            depth: 1.0,
        });
        body.body.mounts = Some(vec![
            Mount {
                id: "inside".into(),
                at: Vec2 { x: 5.0, y: 5.0 },
                kind: MountKind::Hole,
                hole_diameter: 2.0,
                boss_diameter: None,
                height: None,
            },
            Mount {
                id: "outside".into(),
                at: Vec2 { x: 45.0, y: 5.0 },
                kind: MountKind::Hole,
                hole_diameter: 2.0,
                boss_diameter: None,
                height: None,
            },
        ]);
        let prepared = prepare(&assembly(body)).unwrap();
        let region = &prepared.bodies[0].regions[0];
        assert_eq!(
            region
                .mounts
                .iter()
                .map(|mount| mount.id.as_str())
                .collect::<Vec<_>>(),
            vec!["inside"]
        );
        assert_eq!(region.gaskets.len(), 1);
        assert_eq!(region.gaskets[0].holes.len(), 1);
    }

    #[test]
    fn rejects_revision_mismatch_empty_assembly_invalid_geometry_and_out_of_range_coordinates() {
        assert!(
            prepare(&CaseAssemblyIR {
                revision: 1,
                bodies: vec![]
            })
            .unwrap_err()
            .contains("at least one body")
        );
        let mut stale = assembly(input(
            CaseKind::Plate,
            vec![rectangle(0.0, 0.0, 10.0, 10.0, false)],
        ));
        stale.bodies[0].revision += 1;
        assert!(prepare(&stale).unwrap_err().contains("stale revision"));
        let mut invalid = input(
            CaseKind::Plate,
            vec![rectangle(0.0, 0.0, 10.0, 10.0, false)],
        );
        invalid.body.thickness = f64::NAN;
        assert!(
            prepare(&assembly(invalid))
                .unwrap_err()
                .contains("thickness")
        );
        let mut out_of_range = input(
            CaseKind::Plate,
            vec![rectangle(2_000_000.0, 0.0, 10.0, 10.0, false)],
        );
        out_of_range.body.clearance = 0.0;
        assert!(
            prepare(&assembly(out_of_range))
                .unwrap_err()
                .contains("out-of-range")
        );
        let mut huge_wall = input(
            CaseKind::Tray,
            vec![rectangle(0.0, 0.0, 100.0, 100.0, false)],
        );
        huge_wall.body.wall_thickness = Some(1.0e300);
        assert!(
            prepare(&assembly(huge_wall))
                .unwrap_err()
                .contains("derived offset exceeds")
        );
    }

    fn area(points: &[Vec2]) -> f64 {
        points
            .iter()
            .enumerate()
            .map(|(index, point)| {
                let next = points[(index + 1) % points.len()];
                point.x * next.y - next.x * point.y
            })
            .sum::<f64>()
            .abs()
            / 2.0
    }
}
