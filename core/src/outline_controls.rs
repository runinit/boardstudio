//! Persistent outline controls use component frames, never generated contour indices.
use crate::model::*;
use std::collections::BTreeSet;

pub(crate) fn world(at: Vec2, part: &Part) -> Vec2 {
    let x = if part.side == Side::Back { -at.x } else { at.x };
    let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
    Vec2 {
        x: part.pose.at.x + x * cos - at.y * sin,
        y: part.pose.at.y + x * sin + at.y * cos,
    }
}

pub(crate) fn point(doc: &ProjectDoc, point: &OutlineControlPoint) -> Result<Vec2, String> {
    if !point.at.x.is_finite() || !point.at.y.is_finite() {
        return Err("Outline control coordinates must be finite".into());
    }
    match &point.part_id {
        None => Ok(point.at),
        Some(id) => doc
            .parts
            .iter()
            .find(|part| &part.id == id)
            .map(|part| world(point.at, part))
            .ok_or_else(|| {
                format!("Outline attachment {id} is missing; reattach it or keep the point fixed")
            }),
    }
}

pub(crate) fn dependencies(feature: &OutlineFeature) -> Vec<&str> {
    match feature {
        OutlineFeature::PartEnvelope {
            part_ids,
            connections,
            ..
        } => part_ids
            .iter()
            .map(String::as_str)
            .chain(connections.iter().flat_map(|connection| {
                connection
                    .points
                    .iter()
                    .filter_map(|point| point.part_id.as_deref())
            }))
            .collect(),
        OutlineFeature::Polygon { anchor_part_id, .. }
        | OutlineFeature::Rect { anchor_part_id, .. } => {
            anchor_part_id.iter().map(String::as_str).collect()
        }
    }
}

pub(crate) fn validate(doc: &ProjectDoc, feature: &OutlineFeature) -> Result<(), String> {
    if let OutlineFeature::PartEnvelope { connections, .. } = feature {
        let mut ids = BTreeSet::new();
        for connection in connections {
            if connection.id.is_empty() || !ids.insert(&connection.id) {
                return Err("Manual bridges need distinct nonempty IDs".into());
            }
            if !connection.width.is_finite()
                || connection.width <= 0.0
                || connection.points.len() < 2
            {
                return Err(
                    "A manual bridge needs a positive width and at least two points".into(),
                );
            }
        }
    }
    let owners: Vec<_> = doc
        .boards
        .iter()
        .filter(|board| board.outline_ids.iter().any(|id| id == feature.id()))
        .collect();
    let anchors: Vec<_> = match feature {
        OutlineFeature::PartEnvelope { connections, .. } => connections
            .iter()
            .flat_map(|connection| {
                connection
                    .points
                    .iter()
                    .filter_map(|point| point.part_id.as_deref())
            })
            .collect(),
        OutlineFeature::Polygon { anchor_part_id, .. }
        | OutlineFeature::Rect { anchor_part_id, .. } => {
            anchor_part_id.iter().map(String::as_str).collect()
        }
    };
    for id in anchors {
        if !doc.parts.iter().any(|part| part.id == id) {
            return Err(format!(
                "Outline attachment {id} is missing; reattach it or keep it fixed"
            ));
        }
        if owners
            .iter()
            .any(|board| !board.part_ids.iter().any(|part| part == id))
        {
            return Err("Outline attachments cannot cross separate PCBs".into());
        }
    }
    Ok(())
}

pub(crate) fn attached_parts(doc: &ProjectDoc) -> Vec<Part> {
    let ids: BTreeSet<_> = doc
        .outline
        .iter()
        .flat_map(|feature| match feature {
            OutlineFeature::PartEnvelope { connections, .. } => connections
                .iter()
                .flat_map(|connection| {
                    connection
                        .points
                        .iter()
                        .filter_map(|point| point.part_id.as_deref())
                })
                .collect::<Vec<_>>(),
            OutlineFeature::Polygon { anchor_part_id, .. }
            | OutlineFeature::Rect { anchor_part_id, .. } => {
                anchor_part_id.iter().map(String::as_str).collect()
            }
        })
        .collect();
    doc.parts
        .iter()
        .filter(|part| ids.contains(part.id.as_str()))
        .cloned()
        .collect()
}

/// Deleting a component preserves the authored shape at its last world pose.
/// Rectangles retain their dimensions, corner radius and world rotation.
pub(crate) fn detach_removed(before: &[Part], after: &mut ProjectDoc) {
    let present: BTreeSet<_> = after.parts.iter().map(|part| part.id.as_str()).collect();
    for feature in &mut after.outline {
        let removed = |id: &str| !present.contains(id);
        match feature {
            OutlineFeature::PartEnvelope { connections, .. } => {
                for point in connections
                    .iter_mut()
                    .flat_map(|connection| &mut connection.points)
                {
                    if point.part_id.as_deref().is_some_and(removed) {
                        if let Some(part) = before
                            .iter()
                            .find(|part| Some(&part.id) == point.part_id.as_ref())
                        {
                            point.at = world(point.at, part);
                            point.part_id = None;
                        }
                    }
                }
            }
            OutlineFeature::Polygon {
                anchor_part_id: Some(anchor),
                points,
                ..
            } if removed(anchor) => {
                if let Some(part) = before.iter().find(|part| part.id == *anchor) {
                    for point in points {
                        *point = world(*point, part);
                    }
                    if let OutlineFeature::Polygon { anchor_part_id, .. } = feature {
                        *anchor_part_id = None;
                    }
                }
            }
            OutlineFeature::Rect {
                anchor_part_id,
                center,
                rotation,
                ..
            } if anchor_part_id.as_deref().is_some_and(removed) => {
                if let Some(part) = before
                    .iter()
                    .find(|part| Some(&part.id) == anchor_part_id.as_ref())
                {
                    *center = world(*center, part);
                    *rotation = Some(
                        part.pose.rotation
                            + rotation.unwrap_or(0.0)
                                * if part.side == Side::Back { -1.0 } else { 1.0 },
                    );
                    *anchor_part_id = None;
                }
            }
            _ => {}
        }
    }
}
