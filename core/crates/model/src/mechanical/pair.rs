//! Geometry helpers for explicit mechanical mirror pairs.
//!
//! A pair map is a `Pose2` composed with a fixed X reflection:
//! `q = R(rotation) * (-p.x, p.y) + at`, with an exact inverse. Planar orientation
//! transforms by `angle_B = rotation - angle_A`; reflection reverses polygon
//! winding, so projected opening vertex sequences reverse while keeping their
//! identities.
use crate::model::{MechanicalOpening, MechanicalOpeningVertex, Pose2, Vec2};

/// The linear part of the pair map: a fixed X reflection followed by the mapping
/// rotation. Translation is deliberately excluded so vectors stay directions.
fn linear(rotation: f64, vector: Vec2) -> Vec2 {
    let (sin, cos) = rotation.to_radians().sin_cos();
    Vec2 {
        x: -vector.x * cos - vector.y * sin,
        y: -vector.x * sin + vector.y * cos,
    }
}

/// Maps a point from the first member's Case frame to the second's:
/// `q = R(rotation) * (-p.x, p.y) + at`.
pub fn forward(mapping: Pose2, point: Vec2) -> Vec2 {
    let mapped = linear(mapping.rotation, point);
    Vec2 {
        x: mapped.x + mapping.at.x,
        y: mapped.y + mapping.at.y,
    }
}

/// The exact inverse of [`forward`]: it removes the mapping translation and then
/// applies the same reflection/rotation, so `forward(inverse(p)) == p` exactly.
pub fn inverse(mapping: Pose2, point: Vec2) -> Vec2 {
    linear(
        mapping.rotation,
        Vec2 {
            x: point.x - mapping.at.x,
            y: point.y - mapping.at.y,
        },
    )
}

/// Planar orientation from the first member to the second: `rotation - angle`.
/// The fixed reflection flips the sign of the source orientation, so a vector
/// projected by [`forward_vector`] lies on the line at this angle: the angle is only
/// fixed modulo 180 degrees, and the vector's own heading is this angle plus 180.
pub fn forward_angle(mapping: Pose2, angle: f64) -> f64 {
    mapping.rotation - angle
}

/// The second member's orientation expressed back in the reflected frame.
///
/// Reflection makes the orientation map an involution modulo 180 degrees, so
/// recovering the source orientation uses the same rule as [`forward_angle`].
pub fn inverse_angle(mapping: Pose2, angle: f64) -> f64 {
    mapping.rotation - angle
}

/// One rule for every mirrored independent-part heading.
///
/// Controllers (the MCU) keep the plain reflected heading `forward_angle`, so both halves
/// share the same world rotation. Every other independent part takes ADR-0010's full rule
/// `forward_angle + 180`. The result is normalized to `[0, 360)`.
///
/// The inverse direction uses the same function: `inverse_angle` is the same arithmetic as
/// `forward_angle` (each heading rule is its own inverse over the full circle), so callers
/// projecting back from the mirror pass the same mapping through this helper.
pub fn mirrored_part_heading(is_controller: bool, mapping: Pose2, rotation: f64) -> f64 {
    let reflected = forward_angle(mapping, rotation);
    if is_controller {
        reflected.rem_euclid(360.0)
    } else {
        (reflected + 180.0).rem_euclid(360.0)
    }
}

/// Maps a direction vector with the reflection and rotation only, ignoring the
/// mapping translation.
pub fn forward_vector(mapping: Pose2, vector: Vec2) -> Vec2 {
    linear(mapping.rotation, vector)
}

/// Projects an authored opening through the pair map.
///
/// Every vertex point and stable id survives; the sequence is reversed because
/// the reflection reverses the polygon's winding. The opening id, `z` and
/// `height` are passed through unchanged.
pub fn reflect_opening(mapping: Pose2, opening: &MechanicalOpening) -> MechanicalOpening {
    let mut vertices: Vec<MechanicalOpeningVertex> = opening
        .vertices
        .iter()
        .map(|vertex| MechanicalOpeningVertex {
            id: vertex.id.clone(),
            point: forward(mapping, vertex.point),
        })
        .collect();
    vertices.reverse();
    MechanicalOpening {
        id: opening.id.clone(),
        vertices,
        z: opening.z,
        height: opening.height,
        next_vertex_identity: opening.next_vertex_identity,
    }
}
