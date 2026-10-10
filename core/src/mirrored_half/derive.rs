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
