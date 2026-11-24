use boardstudio_model::mechanical::pair;

use crate::MirroredPair;
use crate::model::{Matrix, Mirror, Pose2};
use boardstudio_model::mirrored_half::mirrored_entity_id;

pub(crate) fn matrix(mirror: &MirroredPair, source: &Matrix, board_id: &str) -> Matrix {
    let mut target = source.clone();
    target.id = mirrored_entity_id(mirror, "matrix", &source.id);
    target.board_id = Some(board_id.into());
    target.mirror = Some(match source.mirror {
        Some(Mirror::X | Mirror::Y) => Mirror::None,
        _ => Mirror::X,
    });
    if let Some(anchor) = &mut target.anchor {
        anchor.matrix_id = mirrored_entity_id(mirror, "matrix", &anchor.matrix_id);
        target.origin.x = -target.origin.x;
        target.rotation = source.rotation.map(|angle| -angle);
    } else {
        target.origin = pair::forward(mirror.mapping, source.origin);
        target.rotation = Some(pair::forward_angle(
            mirror.mapping,
            source.rotation.unwrap_or_default(),
        ));
    }
    if source.mirror == Some(Mirror::Y) {
        target.rotation = Some((target.rotation.unwrap_or_default() + 180.0).rem_euclid(360.0));
    }
    for cell in &mut target.cells {
        for assembly in &mut cell.assemblies {
            assembly.offset.x = -assembly.offset.x;
            assembly.rotation = assembly.rotation.map(|angle| -angle);
        }
    }
    target
}

pub(crate) fn pose(mapping: Pose2, pose: Pose2) -> Pose2 {
    Pose2 {
        at: pair::forward(mapping, pose.at),
        rotation: pair::forward_angle(mapping, pose.rotation),
    }
}

pub(crate) fn outline(
    mirror: &MirroredPair,
    feature: &crate::model::OutlineFeature,
    parts: &std::collections::BTreeMap<&str, String>,
) -> Option<crate::model::OutlineFeature> {
    use crate::model::OutlineFeature;

    let mut projected = feature.clone();
    match &mut projected {
        OutlineFeature::Polygon {
            id,
            anchor_part_id,
            points,
            ..
        } => {
            *id = mirrored_entity_id(mirror, "outline", id);
            if let Some(anchor) = anchor_part_id {
                *anchor = parts.get(anchor.as_str())?.clone();
                for point in points.iter_mut() {
                    point.x = -point.x;
                }
            } else {
                for point in points.iter_mut() {
                    *point = pair::forward(mirror.mapping, *point);
                }
            }
            points.reverse();
        }
        OutlineFeature::Rect {
            id,
            anchor_part_id,
            center,
            rotation,
            ..
        } => {
            *id = mirrored_entity_id(mirror, "outline", id);
            if let Some(anchor) = anchor_part_id {
                *anchor = parts.get(anchor.as_str())?.clone();
                center.x = -center.x;
                *rotation = rotation.map(|angle| -angle);
            } else {
                *center = pair::forward(mirror.mapping, *center);
                *rotation = Some(pair::forward_angle(
                    mirror.mapping,
                    rotation.unwrap_or_default(),
                ));
            }
        }
        OutlineFeature::PartEnvelope {
            id,
            part_ids,
            connections,
            ..
        } => {
            *id = mirrored_entity_id(mirror, "outline", id);
            *part_ids = part_ids
                .iter()
                .filter_map(|id| parts.get(id.as_str()).cloned())
                .collect();
            for connection in connections {
                connection.id = mirrored_entity_id(mirror, "connection", &connection.id);
                for point in &mut connection.points {
                    point.at = pair::forward(mirror.mapping, point.at);
                    point.part_id = point
                        .part_id
                        .as_ref()
                        .and_then(|id| parts.get(id.as_str()).cloned());
                }
            }
        }
    }
    Some(projected)
}

pub(crate) fn mechanical(
    mirror: &MirroredPair,
    source: &crate::model::MechanicalConfiguration,
    board_id: &str,
) -> crate::model::MechanicalConfiguration {
    let mut target = source.clone();
    board_id.clone_into(&mut target.board_id);
    if let Some(openings) = &mut target.openings {
        for opening in openings {
            *opening = pair::reflect_opening(mirror.mapping, opening);
        }
    }
    for mount in target
        .mounts
        .iter_mut()
        .chain(target.closure_mounts.iter_mut().flatten())
    {
        mount.at = pair::forward(mirror.mapping, mount.at);
    }
    if let Some(battery) = &mut target.battery {
        battery.at = pair::forward(mirror.mapping, battery.at);
        battery.cable_exit = pair::forward(mirror.mapping, battery.cable_exit);
    }
    target
}
