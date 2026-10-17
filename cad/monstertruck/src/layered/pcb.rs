//! Exact authored PCB source; polygon fields are only a compatibility projection.
use super::{circle, contours, feature, topology};
use crate::ExactModel;
use boardstudio_contracts::{
    CaseKind, CasePcbCircularSupport, CasePcbSourceContour, CasePcbSourcePrimitive,
    MechanicalShape, PreparedCaseIR, Vec2,
};
use cavalier_contours::polyline::{PlineSource, PlineVertex};
use std::collections::{BTreeMap, BTreeSet};

// Same source residual budget as the existing line/arc lineage validator; never selects geometry.
const SOURCE_RESIDUAL: f64 = 1e-10;

pub(super) fn build(input: &PreparedCaseIR) -> Result<ExactModel, String> {
    let body = &input.body;
    let source = body.pcb_source.as_ref().ok_or("PCB source is missing")?;
    if body.kind != CaseKind::Plate
        || body.clearance != 0.
        || body.wall_height.is_some_and(|value| value != 0.)
        || body.wall_thickness.is_some_and(|value| value != 0.)
        || body.gasket.is_some()
        || body
            .features
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || body
            .openings
            .as_ref()
            .is_some_and(|values| !values.is_empty())
    {
        return Err(format!("PCB '{}' has unsupported body modifiers", body.id));
    }
    if source.contours.is_empty() {
        return Err(format!(
            "PCB '{}' source composition is not supported yet",
            body.id
        ));
    }
    if input
        .regions
        .iter()
        .any(|region| !region.cavities.is_empty() || !region.gaskets.is_empty())
    {
        return Err(format!(
            "PCB '{}' has unsupported prepared modifiers",
            body.id
        ));
    }
    let mut contours_by_id = BTreeMap::new();
    let mut primitive_ids = BTreeSet::new();
    let mut exterior_indices = BTreeMap::new();
    let mut rings = Vec::new();
    let mut declared_holes = Vec::new();
    for contour in &source.contours {
        if contour.id.is_empty()
            || contour.primitives.is_empty()
            || contours_by_id
                .insert(contour.id.as_str(), contour)
                .is_some()
        {
            return Err(format!(
                "PCB contour '{}' has missing or duplicate identity/geometry",
                contour.id
            ));
        }
        for primitive in &contour.primitives {
            if !primitive_ids.insert((primitive.source_id.as_str(), primitive.segment_index)) {
                return Err(fail(
                    primitive,
                    "duplicate source/segment identity across contours",
                ));
            }
        }
        if contour.outer_id.is_none() {
            exterior_indices.insert(contour.id.as_str(), exterior_indices.len());
        }
        let ring = contour_ring(contour)?;
        if let Some(owner) = &contour.outer_id {
            declared_holes.push((contour.id.as_str(), owner.as_str(), ring.clone()));
        }
        rings.push((ring, contour.outer_id.is_none()));
    }
    for contour in &source.contours {
        if let Some(owner) = &contour.outer_id
            && !exterior_indices.contains_key(owner.as_str())
        {
            return Err(format!(
                "PCB contour '{}' has missing or non-exterior owner '{}'",
                contour.id, owner
            ));
        }
    }
    let exterior = rings
        .iter()
        .find(|(_, positive)| *positive)
        .ok_or_else(|| format!("PCB '{}' has no source exterior", body.id))?;
    let span = feature(
        body.id.clone(),
        exterior.0.clone(),
        body.z.unwrap_or(0.),
        body.thickness,
        true,
    )?;
    let frame = contours::frame(rings.iter().map(|(ring, _)| ring))?;
    let mut cut_ids = BTreeSet::new();
    for mount in body.mounts.as_deref().unwrap_or_default() {
        if mount.id.is_empty() || !cut_ids.insert(mount.id.clone()) {
            return Err(format!(
                "PCB drill '{}' has a missing or duplicate identity",
                mount.id
            ));
        }
        if mount.kind != boardstudio_contracts::MountKind::Hole {
            return Err(format!("PCB mount '{}' is not a round drill", mount.id));
        }
        if !mount.hole_diameter.is_finite() || mount.hole_diameter <= 0. {
            return Err(format!("PCB drill '{}' has an invalid diameter", mount.id));
        }
        let cut = circle(
            [mount.at.x, mount.at.y],
            mount.hole_diameter / 2.,
            &mount.id,
        );
        contours::validate(&cut).map_err(|error| format!("PCB drill '{}': {error}", mount.id))?;
        rings.push((cut, false));
    }
    for slot in &source.slots {
        if slot.id.is_empty() || !cut_ids.insert(slot.id.clone()) {
            return Err(format!(
                "PCB slot '{}' has missing or duplicate identity",
                slot.id
            ));
        }
        let cut = slot_ring(slot)?;
        rings.push((cut, false));
    }
    // The existing validator proves full source containment and disjoint void interiors.
    // Boundary-crossing or overlapping cuts refuse with their source IDs, never a center filter.
    let profile = contours::from_rings(rings).map_err(|error| {
        format!(
            "PCB '{}' contours [{}] cuts [{}]: {error}",
            body.id,
            source
                .contours
                .iter()
                .map(|contour| contour.id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            cut_ids.into_iter().collect::<Vec<_>>().join(", ")
        )
    })?;
    // from_rings preserves each normalized input ring. Check its actual smallest material
    // owner against the explicit source declaration; no point-only containment or reassignment.
    for (id, owner, mut expected) in declared_holes {
        if expected.area() < 0. {
            expected.invert_direction_mut();
        }
        let region = &profile[exterior_indices[owner]];
        if !region
            .holes
            .iter()
            .any(|hole| hole.iter_vertexes().eq(expected.iter_vertexes()))
        {
            return Err(format!(
                "PCB contour '{id}' contradicts declared owner '{owner}'"
            ));
        }
    }
    let bands = [topology::Band {
        low: span.low,
        high: span.high,
        initial: true,
        active: vec![],
        lower: profile.clone(),
        upper: profile,
    }];
    let solids = topology::assemble(&bands, &[], &frame)?;
    Ok(ExactModel { solids })
}

fn fail(primitive: &CasePcbSourcePrimitive, message: &str) -> String {
    format!(
        "PCB primitive '{}'/{}: {message}",
        primitive.source_id, primitive.segment_index
    )
}
fn xy(point: &Vec2) -> [f64; 2] {
    [point.x, point.y]
}
fn witness(
    primitive: &CasePcbSourcePrimitive,
    actual: [f64; 2],
    expected: [f64; 2],
) -> Result<(), String> {
    if actual
        .into_iter()
        .zip(expected)
        .any(|(a, b)| !a.is_finite() || (a - b).abs() > SOURCE_RESIDUAL)
    {
        Err(fail(
            primitive,
            "point witness disagrees with authored support",
        ))
    } else {
        Ok(())
    }
}
fn circular_support(primitive: &CasePcbSourcePrimitive) -> Result<&CasePcbCircularSupport, String> {
    let support = primitive
        .curve_support
        .as_ref()
        .ok_or_else(|| fail(primitive, "missing authored circular support"))?;
    if ![
        support.center.x,
        support.center.y,
        support.radius_mm,
        support.start_direction.x,
        support.start_direction.y,
        support.sweep_radians,
    ]
    .into_iter()
    .all(f64::is_finite)
        || support.radius_mm <= 0.
        || support.sweep_radians == 0.
        || support.sweep_radians.abs() > std::f64::consts::TAU
        || (support.start_direction.x.hypot(support.start_direction.y) - 1.).abs() > SOURCE_RESIDUAL
    {
        return Err(fail(primitive, "invalid authored circular support"));
    }
    Ok(support)
}
fn support_point(support: &CasePcbCircularSupport, angle: f64) -> [f64; 2] {
    let (s, c) = angle.sin_cos();
    [
        support.center.x
            + support.radius_mm * (c * support.start_direction.x - s * support.start_direction.y),
        support.center.y
            + support.radius_mm * (s * support.start_direction.x + c * support.start_direction.y),
    ]
}
fn source_id(primitive: &CasePcbSourcePrimitive) -> String {
    format!("{}:{}", primitive.source_id, primitive.segment_index)
}
fn contour_ring(contour: &CasePcbSourceContour) -> Result<contours::Ring, String> {
    let mut identities = BTreeSet::new();
    for primitive in &contour.primitives {
        if primitive.source_id.is_empty()
            || !identities.insert((&primitive.source_id, primitive.segment_index))
        {
            return Err(fail(
                primitive,
                "missing or duplicate source/segment identity",
            ));
        }
    }
    if contour.primitives.len() == 1 {
        let primitive = &contour.primitives[0];
        if let MechanicalShape::Circle { center, end, .. } = &primitive.geometry {
            let support = circular_support(primitive)?;
            if support.sweep_radians.abs() != std::f64::consts::TAU {
                return Err(fail(primitive, "circle requires one full signed turn"));
            }
            witness(primitive, xy(center), xy(&support.center))?;
            witness(primitive, xy(end), support_point(support, 0.))?;
            let ring = circle(
                xy(&support.center),
                support.radius_mm,
                &source_id(primitive),
            );
            contours::validate(&ring).map_err(|error| fail(primitive, &error))?;
            return Ok(ring);
        }
    }
    let mut ring = contours::Ring::new_closed();
    let mut first = None;
    let mut previous = None;
    for primitive in &contour.primitives {
        let (start, end, vertices) = match &primitive.geometry {
            MechanicalShape::Line { start, end, .. } => {
                let (start, end) = (xy(start), xy(end));
                if primitive.curve_support.is_some()
                    || start == end
                    || !start.into_iter().chain(end).all(f64::is_finite)
                {
                    return Err(fail(primitive, "invalid normalized line"));
                }
                (
                    start,
                    end,
                    vec![(
                        PlineVertex::new(start[0], start[1], 0.),
                        contours::Source::Line(start, end),
                    )],
                )
            }
            MechanicalShape::Arc {
                start,
                mid: Some(mid),
                end,
                angle_degrees: None,
                ..
            } => {
                let support = circular_support(primitive)?;
                let (start, end) = (xy(start), xy(end));
                if start == end || support.sweep_radians.abs() == std::f64::consts::TAU {
                    return Err(fail(primitive, "midpoint arc has degenerate endpoints"));
                }
                witness(primitive, start, support_point(support, 0.))?;
                witness(
                    primitive,
                    end,
                    support_point(support, support.sweep_radians),
                )?;
                let radial = [mid.x - support.center.x, mid.y - support.center.y];
                if !radial.into_iter().all(f64::is_finite)
                    || (radial[0].hypot(radial[1]) - support.radius_mm).abs() > SOURCE_RESIDUAL
                {
                    return Err(fail(primitive, "midpoint witness is off authored circle"));
                }
                let direction = &support.start_direction;
                let turn = (direction.x * radial[1] - direction.y * radial[0])
                    .atan2(direction.x * radial[0] + direction.y * radial[1]);
                let position =
                    (turn * support.sweep_radians.signum()).rem_euclid(std::f64::consts::TAU);
                if position <= 0. || position >= support.sweep_radians.abs() {
                    return Err(fail(primitive, "midpoint witness is outside authored arc"));
                }
                let count = (support.sweep_radians.abs() / std::f64::consts::PI)
                    .ceil()
                    .max(1.) as usize;
                let span = support.sweep_radians / count as f64;
                let bulge = if span.abs() == std::f64::consts::PI {
                    span.signum()
                } else {
                    (span / 4.).tan()
                };
                if bulge == 0. || !bulge.is_finite() || bulge.abs() > 1. {
                    return Err(fail(primitive, "arc span is not representable"));
                }
                let source = contours::Source::Circle(contours::CircleSource {
                    id: source_id(primitive),
                    center: xy(&support.center),
                    radius: support.radius_mm,
                });
                let vertices = (0..count)
                    .map(|index| {
                        let point = if index == 0 {
                            start
                        } else {
                            support_point(support, span * index as f64)
                        };
                        (PlineVertex::new(point[0], point[1], bulge), source.clone())
                    })
                    .collect();
                (start, end, vertices)
            }
            _ => {
                return Err(fail(
                    primitive,
                    "requires normalized line, midpoint arc, or standalone circle",
                ));
            }
        };
        if previous.is_some_and(|point| point != start) {
            return Err(fail(
                primitive,
                "open loop: adjacent authored endpoints differ",
            ));
        }
        first.get_or_insert(start);
        previous = Some(end);
        for (vertex, source) in vertices {
            ring.add(vertex.x, vertex.y, vertex.bulge);
            *ring.sources.last_mut().unwrap() = Some(source);
        }
    }
    if previous != first || first.is_none() {
        return Err(format!("PCB contour '{}' is open", contour.id));
    }
    contours::validate(&ring).map_err(|error| format!("PCB contour '{}': {error}", contour.id))?;
    Ok(ring)
}

fn slot_ring(slot: &boardstudio_contracts::CasePcbSlot) -> Result<contours::Ring, String> {
    let error = |message: &str| format!("PCB slot '{}': {message}", slot.id);
    let MechanicalShape::Drill {
        at,
        size,
        offset,
        rotation_degrees,
        shape: boardstudio_contracts::MechanicalDrillShape::Oval,
        ..
    } = &slot.geometry
    else {
        return Err(error("requires authored oval Drill geometry"));
    };
    if ![
        at.x,
        at.y,
        size.x,
        size.y,
        offset.x,
        offset.y,
        *rotation_degrees,
    ]
    .into_iter()
    .all(f64::is_finite)
        || size.x <= 0.
        || size.y <= 0.
    {
        return Err(error("invalid authored oval dimensions or placement"));
    }
    let (sin, cos) = rotation_degrees.to_radians().sin_cos();
    // Prepared placement has already reflected the local offset. Resolve it exactly once.
    let center = [
        at.x + cos * offset.x - sin * offset.y,
        at.y + sin * offset.x + cos * offset.y,
    ];
    let (axis, major, minor) = if size.x >= size.y {
        ([cos, sin], size.x, size.y)
    } else {
        ([-sin, cos], size.y, size.x)
    };
    let radius = minor / 2.;
    let half_line = (major - minor) / 2.;
    if radius == 0. || !radius.is_finite() || !center.into_iter().all(f64::is_finite) {
        return Err(error("oval support is not representable"));
    }
    if half_line == 0. {
        let ring = circle(center, radius, &slot.id);
        contours::validate(&ring).map_err(|message| error(&message))?;
        return Ok(ring);
    }
    let normal = [-axis[1], axis[0]];
    let ends: [[f64; 2]; 2] = [-1., 1.].map(|sign| {
        [
            center[0] + sign * axis[0] * half_line,
            center[1] + sign * axis[1] * half_line,
        ]
    });
    let point = |end: usize, sign: f64| {
        [
            ends[end][0] + sign * normal[0] * radius,
            ends[end][1] + sign * normal[1] * radius,
        ]
    };
    let vertices = [point(0, -1.), point(1, -1.), point(1, 1.), point(0, 1.)];
    let mut ring = contours::Ring::new_closed();
    for index in 0..4 {
        let vertex = vertices[index];
        let bulge = if index % 2 == 0 { 0. } else { 1. };
        ring.add(vertex[0], vertex[1], bulge);
        *ring.sources.last_mut().unwrap() = Some(if bulge == 0. {
            contours::Source::Line(vertex, vertices[(index + 1) % 4])
        } else {
            let end = usize::from(index == 1);
            contours::Source::Circle(contours::CircleSource {
                id: format!("{}:end{end}", slot.id),
                center: ends[end],
                radius,
            })
        });
    }
    contours::validate(&ring).map_err(|message| error(&message))?;
    Ok(ring)
}
