//! Pure mirrored-value derivation. Callers plan correspondence identities and collision checks;
//! these helpers only project existing values into the supplied identity space.

use crate::mechanical::pair;
use crate::model::{Part, Pose2};

/// Reflect an independent part's pose through the pair map using ADR-0010's full heading rule.
pub(super) fn pose(mapping: Pose2, pose: Pose2) -> Pose2 {
    Pose2 {
        at: pair::forward(mapping, pose.at),
        rotation: (pair::forward_angle(mapping, pose.rotation) + 180.0).rem_euclid(360.0),
    }
}

/// Derive a part using identities already selected by the caller.
pub(super) fn part(
    source: &Part,
    mirror_id: &str,
    definition_id: &str,
    reference: &str,
    mapping: Pose2,
) -> Part {
    let mut mirrored = source.clone();
    mirrored.id = mirror_id.to_owned();
    mirrored.definition_id = definition_id.to_owned();
    mirrored.reference = reference.to_owned();
    mirrored.pose = pose(mapping, source.pose);
    mirrored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vec2;
    use serde_json::json;

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
}
