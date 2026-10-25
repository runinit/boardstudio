//! Pure mirrored-value derivation. Callers plan correspondence identities and collision checks;
//! these helpers only project existing values into the supplied identity space.

use crate::mechanical::pair;
use crate::model::{
    MirroredCompanionMode, MirroredCompanionTweak, Part, PartDefinition, PartKind, Pose2, Vec2,
};

/// Whether a part definition makes its parts controllers, which keep the reflected heading.
pub(super) fn is_controller(definitions: &[PartDefinition], definition_id: &str) -> bool {
    definitions
        .iter()
        .any(|definition| definition.id == definition_id && definition.kind == PartKind::Controller)
}

/// Reflect an independent part's pose through the pair map.
///
/// Matrix-owned parts take the switch rule elsewhere; here every part takes ADR-0010's full
/// heading rule `forward_angle + 180`, except controllers, which keep the reflected heading
/// without the half turn so a mirrored MCU faces the same way as its source.
pub(super) fn pose(mapping: Pose2, pose: Pose2, is_controller: bool) -> Pose2 {
    let reflected = pair::forward_angle(mapping, pose.rotation);
    Pose2 {
        at: pair::forward(mapping, pose.at),
        rotation: if is_controller {
            reflected.rem_euclid(360.0)
        } else {
            (reflected + 180.0).rem_euclid(360.0)
        },
    }
}

/// Derive a part using identities already selected by the caller.
pub(super) fn part(
    source: &Part,
    mirror_id: &str,
    definition_id: &str,
    reference: &str,
    mapping: Pose2,
    is_controller: bool,
) -> Part {
    let mut mirrored = source.clone();
    mirrored.id = mirror_id.to_owned();
    mirrored.definition_id = definition_id.to_owned();
    mirrored.reference = reference.to_owned();
    mirrored.pose = pose(mapping, source.pose, is_controller);
    mirrored
}

/// Inputs for projecting a matrix assembly's existing placement under its effective item rule.
/// Identity planning stays with the caller; only the caller's already selected values are used.
pub(super) struct CompanionAssemblyInput<'a> {
    pub(super) offset: Vec2,
    pub(super) rotation: Option<f64>,
    pub(super) key: Option<&'a Part>,
    pub(super) companion: Option<&'a Part>,
    pub(super) mapping: Pose2,
    pub(super) mode: MirroredCompanionMode,
    pub(super) tweak: Option<MirroredCompanionTweak>,
    pub(super) stabiliser: bool,
    pub(super) fallback: bool,
}

/// Derive the assembly placement only; callers remain responsible for inserting materialized parts.
pub(super) fn companion_assembly(
    input: CompanionAssemblyInput<'_>,
) -> Result<(Vec2, Option<f64>), &'static str> {
    let CompanionAssemblyInput {
        mut offset,
        mut rotation,
        key,
        companion,
        mapping,
        mode,
        tweak,
        stabiliser,
        fallback,
    } = input;

    if stabiliser {
        let key = key.ok_or("A companion's source switch is unavailable")?;
        let companion = companion.ok_or("A source companion is unavailable")?;
        // Assemblies store a heading relative to the mirrored switch. Cancel that switch's
        // reflected tilt so the stabiliser keeps its source world heading.
        rotation = Some(companion.pose.rotation - pair::forward_angle(mapping, key.pose.rotation));
    } else if !fallback {
        match mode {
            MirroredCompanionMode::LiteralMirror => {}
            MirroredCompanionMode::DuplicatePosition => {
                let key = key.ok_or("A companion's source switch is unavailable")?;
                let companion = companion.ok_or("A source companion is unavailable")?;
                let (sin, cos) = key.pose.rotation.to_radians().sin_cos();
                let dx = companion.pose.at.x - key.pose.at.x;
                let dy = companion.pose.at.y - key.pose.at.y;
                offset = Vec2 {
                    x: dx * cos + dy * sin,
                    y: -dx * sin + dy * cos,
                };
                rotation = Some(companion.pose.rotation - key.pose.rotation);
            }
            MirroredCompanionMode::LiteralMirrorWithTweak => {
                if let Some(tweak) = tweak {
                    offset.x += tweak.x;
                    offset.y += tweak.y;
                    rotation = Some(rotation.unwrap_or(0.0) + tweak.rotation);
                }
            }
        }
    }

    Ok((offset, rotation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vec2;
    use serde_json::json;

    #[test]
    fn controller_pose_keeps_the_reflected_heading_without_the_half_turn() {
        // An axis map: `forward_angle` is `-r`. A controller keeps it, so an MCU facing the
        // same way on the source still faces that way on the mirror.
        let mapping = Pose2 {
            at: Vec2 { x: 200.0, y: 0.0 },
            rotation: 0.0,
        };
        let at = Vec2 { x: -24.0, y: 5.0 };
        let expected_at = pair::forward(mapping, at);
        for heading in [0.0, 90.0, 180.0, 270.0] {
            let controller = pose(
                mapping,
                Pose2 {
                    at,
                    rotation: heading,
                },
                true,
            );
            assert_eq!(controller.at, expected_at, "heading {heading}");
            assert_eq!(
                controller.rotation,
                (0.0 - heading).rem_euclid(360.0),
                "controller heading {heading} mirrors to rotation - r"
            );
            // Every other independent part keeps the full rule: the reflected heading
            // turned by 180 degrees.
            let independent = pose(
                mapping,
                Pose2 {
                    at,
                    rotation: heading,
                },
                false,
            );
            assert_eq!(independent.at, expected_at, "heading {heading}");
            assert_eq!(
                independent.rotation,
                (180.0 - heading).rem_euclid(360.0),
                "independent heading {heading} keeps the half turn"
            );
        }
    }

    #[test]
    fn part_derivation_reuses_supplied_identities_without_a_board_or_pair() {
        let source: Part = serde_json::from_value(json!({
            "id": "source/part", "definitionId": "source/definition", "reference": "U1",
            "pose": {"at": {"x": 2.0, "y": 5.0}, "rotation": 35.0}, "side": "back",
            "keycap": {"x": 1.25, "y": 2.0}, "locked": true,
            "properties": {"custom": [1, "keep", false]},
            "generatorParameters": {"reversible": false, "side": "B", "custom": 9}
        }))
        .unwrap();
        let before = serde_json::to_value(&source).unwrap();
        for (mapping, at, heading) in [
            (
                Pose2 {
                    at: Vec2 { x: 10.0, y: -7.0 },
                    rotation: 0.0,
                },
                Vec2 { x: 8.0, y: -2.0 },
                145.0,
            ),
            (
                Pose2 {
                    at: Vec2 { x: 20.0, y: -3.0 },
                    rotation: 90.0,
                },
                Vec2 { x: 15.0, y: -5.0 },
                235.0,
            ),
        ] {
            let mirrored = part(
                &source,
                "existing/item-7",
                "existing/definition",
                "J42",
                mapping,
                false,
            );
            assert!((mirrored.pose.at.x - at.x).abs() < 1e-9);
            assert!((mirrored.pose.at.y - at.y).abs() < 1e-9);
            assert_eq!(mirrored.pose.rotation, heading);
            let mut expected = before.clone();
            expected["id"] = json!("existing/item-7");
            expected["definitionId"] = json!("existing/definition");
            expected["reference"] = json!("J42");
            expected["pose"] = serde_json::to_value(mirrored.pose).unwrap();
            assert_eq!(serde_json::to_value(mirrored).unwrap(), expected);
        }
        assert_eq!(serde_json::to_value(source).unwrap(), before);
    }

    #[test]
    fn companion_assembly_uses_existing_parts_and_each_items_stored_settings() {
        use crate::model::{MirroredCompanionMode, MirroredCompanionTweak};

        let key: Part = serde_json::from_value(json!({
            "id": "matrix/source/r0c0", "definitionId": "switch", "reference": "S1",
            "pose": {"at": {"x": 4.0, "y": 7.0}, "rotation": 15.0}, "side": "back"
        }))
        .unwrap();
        let companion: Part = serde_json::from_value(json!({
            "id": "matrix/source/r0c0/diode", "definitionId": "diode", "reference": "D1",
            "pose": {"at": {"x": 11.0, "y": 13.0}, "rotation": 62.0},
            "side": "back", "generatorParameters": {"side": "B", "custom": 9}
        }))
        .unwrap();
        let before = (
            serde_json::to_value(&key).unwrap(),
            serde_json::to_value(&companion).unwrap(),
        );
        let existing_key = part(
            &key,
            "existing/mirror-key",
            "switch",
            "S42",
            Pose2 {
                at: Vec2 { x: 20.0, y: -3.0 },
                rotation: 0.0,
            },
            false,
        );
        let existing_companion = part(
            &companion,
            "existing/mirror-diode",
            "diode",
            "D42",
            Pose2 {
                at: Vec2 { x: 20.0, y: -3.0 },
                rotation: 0.0,
            },
            false,
        );
        assert_eq!(existing_key.id, "existing/mirror-key");
        assert_eq!(existing_companion.id, "existing/mirror-diode");

        let mappings = [
            Pose2 {
                at: Vec2 { x: 10.0, y: -7.0 },
                rotation: 0.0,
            },
            Pose2 {
                at: Vec2 { x: 20.0, y: -3.0 },
                rotation: 27.0,
            },
        ];
        for mapping in mappings {
            let mirrored_offset = Vec2 { x: 0.75, y: -1.5 };
            let mirrored_rotation = Some(17.0);

            // The item's stored duplicate setting derives the source switch-local placement.
            let (duplicate_offset, duplicate_rotation) =
                companion_assembly(CompanionAssemblyInput {
                    offset: mirrored_offset,
                    rotation: mirrored_rotation,
                    key: Some(&key),
                    companion: Some(&companion),
                    mapping,
                    mode: MirroredCompanionMode::DuplicatePosition,
                    tweak: None,
                    stabiliser: false,
                    fallback: false,
                })
                .unwrap();
            let (sin, cos) = key.pose.rotation.to_radians().sin_cos();
            let dx = companion.pose.at.x - key.pose.at.x;
            let dy = companion.pose.at.y - key.pose.at.y;
            assert!((duplicate_offset.x - (dx * cos + dy * sin)).abs() < 1e-9);
            assert!((duplicate_offset.y - (-dx * sin + dy * cos)).abs() < 1e-9);
            assert_eq!(
                duplicate_rotation,
                Some(companion.pose.rotation - key.pose.rotation)
            );

            // A different item can retain a distinct tweak without changing its sibling's rule.
            let tweak = MirroredCompanionTweak {
                x: -2.25,
                y: 3.5,
                rotation: -11.0,
            };
            let (tweaked_offset, tweaked_rotation) = companion_assembly(CompanionAssemblyInput {
                offset: mirrored_offset,
                rotation: mirrored_rotation,
                key: Some(&key),
                companion: Some(&companion),
                mapping,
                mode: MirroredCompanionMode::LiteralMirrorWithTweak,
                tweak: Some(tweak),
                stabiliser: false,
                fallback: false,
            })
            .unwrap();
            assert_eq!(tweaked_offset, Vec2 { x: -1.5, y: 2.0 });
            assert_eq!(tweaked_rotation, Some(6.0));

            // Stabiliser headings stay in world space for both axis and rotated maps.
            let (stabiliser_offset, stabiliser_rotation) =
                companion_assembly(CompanionAssemblyInput {
                    offset: mirrored_offset,
                    rotation: mirrored_rotation,
                    key: Some(&key),
                    companion: Some(&companion),
                    mapping,
                    mode: MirroredCompanionMode::DuplicatePosition,
                    tweak: None,
                    stabiliser: true,
                    fallback: false,
                })
                .unwrap();
            assert_eq!(stabiliser_offset, mirrored_offset);
            assert_eq!(
                stabiliser_rotation,
                Some(companion.pose.rotation - (mapping.rotation - key.pose.rotation))
            );
            let mirrored_key_heading = mapping.rotation - key.pose.rotation;
            assert_eq!(
                mirrored_key_heading + stabiliser_rotation.unwrap(),
                companion.pose.rotation,
                "materializing the assembly keeps the stabiliser's world heading"
            );

            // A repaired companion uses literal reflection, ignoring the requested mode and tweak.
            let (fallback_offset, fallback_rotation) = companion_assembly(CompanionAssemblyInput {
                offset: mirrored_offset,
                rotation: mirrored_rotation,
                key: None,
                companion: None,
                mapping,
                mode: MirroredCompanionMode::LiteralMirrorWithTweak,
                tweak: Some(tweak),
                stabiliser: false,
                fallback: true,
            })
            .unwrap();
            assert_eq!(fallback_offset, mirrored_offset);
            assert_eq!(fallback_rotation, mirrored_rotation);
        }

        assert_eq!(
            (
                serde_json::to_value(key).unwrap(),
                serde_json::to_value(companion).unwrap(),
            ),
            before,
            "derivation must not insert or mutate source parts"
        );
    }
}
