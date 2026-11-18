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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MechanicalOpeningVertex;

    fn opening(vertices: &[(&str, f64, f64)]) -> MechanicalOpening {
        MechanicalOpening {
            id: "opening-1".into(),
            vertices: vertices
                .iter()
                .map(|(id, x, y)| MechanicalOpeningVertex {
                    id: (*id).into(),
                    point: Vec2 { x: *x, y: *y },
                })
                .collect(),
            z: 1.25,
            height: 3.5,
            next_vertex_identity: 0,
        }
    }

    fn vertex_ids(opening: &MechanicalOpening) -> Vec<&str> {
        opening
            .vertices
            .iter()
            .map(|vertex| vertex.id.as_str())
            .collect()
    }

    fn assert_close(actual: Vec2, expected: Vec2, label: &str) {
        assert!(
            (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }

    fn assert_opening_close(actual: &MechanicalOpening, expected: &MechanicalOpening, label: &str) {
        assert_eq!(actual.id, expected.id, "{label}: opening id");
        assert_eq!(actual.z, expected.z, "{label}: z");
        assert_eq!(actual.height, expected.height, "{label}: height");
        assert_eq!(
            actual.vertices.len(),
            expected.vertices.len(),
            "{label}: vertices"
        );
        for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
            assert_eq!(actual.id, expected.id, "{label}: vertex id");
            assert_close(actual.point, expected.point, label);
        }
    }

    fn direction(from: Vec2, to: Vec2) -> Vec2 {
        Vec2 {
            x: to.x - from.x,
            y: to.y - from.y,
        }
    }

    #[test]
    fn forward_and_inverse_round_trip_across_translated_and_rotated_mappings() {
        let points = [
            Vec2 { x: 0.5, y: -1.25 },
            Vec2 { x: 3.0, y: 2.75 },
            Vec2 { x: -2.25, y: 4.5 },
            Vec2 {
                x: 7.125,
                y: -3.875,
            },
        ];
        let mappings = [
            // Translation only: forward is `(-p.x + 12.5, p.y - 4.25)`, not identity.
            Pose2 {
                at: Vec2 { x: 12.5, y: -4.25 },
                rotation: 0.0,
            },
            // Rotation and translation together.
            Pose2 {
                at: Vec2 { x: -3.25, y: 7.5 },
                rotation: 37.5,
            },
            // A right angle keeps a second, exactly comparable fixture.
            Pose2 {
                at: Vec2 { x: 4.5, y: -2.0 },
                rotation: 90.0,
            },
        ];
        for mapping in mappings {
            for point in points {
                let mapped = forward(mapping, point);
                assert_close(inverse(mapping, mapped), point, "inverse after forward");
                let reversed = inverse(mapping, point);
                assert_close(forward(mapping, reversed), point, "forward after inverse");
            }
            assert_ne!(
                forward(mapping, points[1]),
                points[1],
                "the pair map must actually move an asymmetric point"
            );
        }
    }

    #[test]
    fn known_worked_example_matches_hand_computed_coordinates() {
        // rotation 90, at (10, 5) collapses forward to `(-p.y + 10, -p.x + 5)`.
        let right_angle = Pose2 {
            at: Vec2 { x: 10.0, y: 5.0 },
            rotation: 90.0,
        };
        assert_close(
            forward(right_angle, Vec2 { x: 2.0, y: 3.0 }),
            Vec2 { x: 7.0, y: 3.0 },
            "hand-computed forward (2, 3)",
        );
        assert_close(
            forward(right_angle, Vec2 { x: -1.0, y: 4.0 }),
            Vec2 { x: 6.0, y: 6.0 },
            "hand-computed forward (-1, 4)",
        );
        assert_close(
            inverse(right_angle, Vec2 { x: 7.0, y: 3.0 }),
            Vec2 { x: 2.0, y: 3.0 },
            "hand-computed inverse (7, 3)",
        );
        assert_close(
            inverse(right_angle, Vec2 { x: 6.0, y: 6.0 }),
            Vec2 { x: -1.0, y: 4.0 },
            "hand-computed inverse (6, 6)",
        );

        // 30 degrees with cos(30) = 0.8660254037844386, sin(30) = 0.5.
        let oblique = Pose2 {
            at: Vec2 { x: 4.5, y: -2.0 },
            rotation: 30.0,
        };
        assert_close(
            forward(oblique, Vec2 { x: 1.0, y: 0.0 }),
            Vec2 {
                x: 3.633_974_596_215_561,
                y: -2.5,
            },
            "hand-computed forward (1, 0) at 30 degrees",
        );
        assert_close(
            forward(oblique, Vec2 { x: 0.0, y: 2.0 }),
            Vec2 {
                x: 3.5,
                y: -0.2679491924311228,
            },
            "hand-computed forward (0, 2) at 30 degrees",
        );
        assert_close(
            inverse(
                oblique,
                Vec2 {
                    x: 3.5,
                    y: -0.2679491924311228,
                },
            ),
            Vec2 { x: 0.0, y: 2.0 },
            "hand-computed inverse at 30 degrees",
        );
    }

    #[test]
    fn angle_maps_carry_the_design_convention_and_round_trip_through_the_pair_map() {
        let mapping = Pose2 {
            at: Vec2 { x: 12.5, y: -4.0 },
            rotation: 30.0,
        };
        // Known values: a 100 degree feature becomes 30 - 100 = -70, and the
        // reflected orientation recovers its source by the same rule, 30 - (-70) = 100.
        assert!((forward_angle(mapping, 100.0) - (-70.0)).abs() < 1e-12);
        assert!((inverse_angle(mapping, -70.0) - 100.0).abs() < 1e-12);
        // Orientation ignores the mapping translation, so the reflection/rotation
        // composition is its own inverse and the forward pass round-trips.
        for angle in [0.0, 45.0, -12.5, 200.0, 137.5] {
            assert!(
                (forward_angle(mapping, forward_angle(mapping, angle)) - angle).abs() < 1e-12,
                "forward angle {angle} must round-trip"
            );
        }
        // Recovering a source orientation from its reflection uses the same rule.
        assert!((inverse_angle(mapping, forward_angle(mapping, 42.0)) - 42.0).abs() < 1e-12);
    }

    #[test]
    fn mirrored_part_heading_keeps_controller_heading_without_half_turn() {
        // An axis map: `forward_angle` is `-r`. Controllers keep it; every other
        // independent part turns it by 180 degrees.
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 0.0 },
            rotation: 0.0,
        };
        for heading in [0.0f64, 90.0, 180.0, 270.0] {
            assert!(
                (mirrored_part_heading(true, mapping, heading) - (0.0 - heading).rem_euclid(360.0))
                    .abs()
                    < 1e-9,
                "controller heading {heading} mirrors to rotation - r"
            );
            assert!(
                (mirrored_part_heading(false, mapping, heading)
                    - (180.0 - heading).rem_euclid(360.0))
                .abs()
                    < 1e-9,
                "independent heading {heading} keeps the half turn"
            );
        }
    }

    #[test]
    fn forward_vector_uses_only_the_reflection_and_rotation() {
        let translated = Pose2 {
            at: Vec2 {
                x: 1000.0,
                y: -500.0,
            },
            rotation: 90.0,
        };
        assert_close(
            forward_vector(translated, Vec2 { x: 2.0, y: 3.0 }),
            Vec2 { x: -3.0, y: -2.0 },
            "right-angle linear part",
        );
        // Moving the translation must not change any mapped vector.
        let origin = Pose2 {
            at: Vec2::default(),
            rotation: 90.0,
        };
        for vector in [
            Vec2 { x: 2.0, y: 3.0 },
            Vec2 { x: -1.5, y: 0.25 },
            Vec2 { x: 0.0, y: 9.0 },
        ] {
            assert_close(
                forward_vector(translated, vector),
                forward_vector(origin, vector),
                "translation is excluded from vectors",
            );
        }
        // The linear part is exactly the difference of two mapped points.
        let a = Vec2 {
            x: 7.125,
            y: -3.875,
        };
        let b = Vec2 { x: -2.25, y: 4.5 };
        assert_close(
            forward_vector(translated, direction(a, b)),
            direction(forward(translated, a), forward(translated, b)),
            "linear part matches the point difference",
        );
        // A pure translation pose still reflects across X; it does not translate vectors.
        let pure_translation = Pose2 {
            at: Vec2 { x: 5.0, y: -7.0 },
            rotation: 0.0,
        };
        assert_close(
            forward_vector(pure_translation, Vec2 { x: 1.5, y: -2.25 }),
            Vec2 { x: -1.5, y: -2.25 },
            "translation-only pose applies the reflection to vectors",
        );
    }

    #[test]
    fn reflect_opening_reverses_vertices_and_preserves_identity_z_and_height() {
        let source = opening(&[("a", 0.0, 0.0), ("b", 3.0, 0.0), ("c", 1.0, 1.0)]);
        let reflection = Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        };
        let mirrored = reflect_opening(reflection, &source);
        assert_eq!(mirrored.id, "opening-1");
        assert_eq!(mirrored.z, source.z);
        assert_eq!(mirrored.height, source.height);
        assert_eq!(vertex_ids(&mirrored), ["c", "b", "a"]);
        assert_close(mirrored.vertices[0].point, Vec2 { x: -1.0, y: 1.0 }, "c");
        assert_close(mirrored.vertices[1].point, Vec2 { x: -3.0, y: 0.0 }, "b");
        assert_close(mirrored.vertices[2].point, Vec2 { x: 0.0, y: 0.0 }, "a");
        assert_ne!(
            mirrored.points(),
            source.points(),
            "an asymmetric opening must not appear unchanged"
        );

        // A rotated pair map keeps the same identity and reversal rules.
        let rotated = Pose2 {
            at: Vec2 { x: 10.0, y: 5.0 },
            rotation: 90.0,
        };
        let rotated_mirror = reflect_opening(rotated, &source);
        assert_eq!(rotated_mirror.id, "opening-1");
        assert_eq!(vertex_ids(&rotated_mirror), ["c", "b", "a"]);
        assert_close(
            rotated_mirror.vertices[0].point,
            Vec2 { x: 9.0, y: 4.0 },
            "c",
        );
        assert_close(
            rotated_mirror.vertices[1].point,
            Vec2 { x: 10.0, y: 2.0 },
            "b",
        );
        assert_close(
            rotated_mirror.vertices[2].point,
            Vec2 { x: 10.0, y: 5.0 },
            "a",
        );
    }

    #[test]
    fn reflecting_twice_with_the_pair_and_its_inverse_restores_the_original_opening() {
        let source = opening(&[("a", 0.0, 0.0), ("b", 3.0, 0.0), ("c", 1.0, 1.0)]);
        // The inverse direction is the same reflection/rotation anchored at the
        // image of the mapping translation, derived only from the linear part.
        let inverse_mapping = |mapping: Pose2| {
            let fixed = forward_vector(mapping, mapping.at);
            Pose2 {
                at: Vec2 {
                    x: -fixed.x,
                    y: -fixed.y,
                },
                rotation: mapping.rotation,
            }
        };
        for mapping in [
            Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            Pose2 {
                at: Vec2 { x: -4.5, y: 2.25 },
                rotation: 37.5,
            },
            Pose2 {
                at: Vec2 { x: 10.0, y: 5.0 },
                rotation: 90.0,
            },
        ] {
            let there = reflect_opening(mapping, &source);
            let back = reflect_opening(inverse_mapping(mapping), &there);
            assert_opening_close(
                &back,
                &source,
                "forward then the exact inverse restores order and points",
            );
            // An axis-centered map is its own inverse, so the same pose suffices.
            let centered = Pose2 {
                at: Vec2 { x: 6.0, y: 0.0 },
                rotation: 0.0,
            };
            assert_opening_close(
                &reflect_opening(centered, &reflect_opening(centered, &source)),
                &source,
                "a reflection about a vertical axis is an involution",
            );
        }
    }

    #[test]
    fn two_distinct_reflections_compose_to_a_non_identity_map_for_an_asymmetric_opening() {
        let source = opening(&[("a", 0.0, 0.0), ("b", 3.0, 0.0), ("c", 1.0, 1.0)]);
        let plain = Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        };
        let quarter = Pose2 {
            at: Vec2::default(),
            rotation: 90.0,
        };
        // Two distinct reflections compose to a 90 degree rotation, never the
        // identity a hidden symmetric double reflection would produce.
        let composed = reflect_opening(quarter, &reflect_opening(plain, &source));
        assert_eq!(vertex_ids(&composed), ["a", "b", "c"]);
        assert_close(
            composed.vertices[1].point,
            Vec2 { x: 0.0, y: 3.0 },
            "hand-computed composition of the two reflections",
        );
        assert_ne!(
            composed.points(),
            source.points(),
            "two distinct reflections must not compose to identity"
        );
        // The plain reflection across X twice is the degenerate identity the guard
        // above must not be confused with.
        let double_plain = reflect_opening(plain, &reflect_opening(plain, &source));
        assert_eq!(double_plain, source);
    }
}
