use std::collections::BTreeMap;

use boardstudio_model::mechanical::pair;

use crate::model::{Part, Pose2, ProjectDoc};
use crate::{MirrorDestination, MirrorOverride, MirroredPair};

/// Geometry for consumers. This type has no archive encoder.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDoc {
    geometry: ProjectDoc,
}

impl ResolvedDoc {
    pub fn geometry(&self) -> &ProjectDoc {
        &self.geometry
    }
}

pub fn project_document(canonical: &ProjectDoc) -> ResolvedDoc {
    let mut geometry = canonical.clone();
    let parts: BTreeMap<_, _> = canonical
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();

    for mirror in &canonical.mirrored_pairs {
        let Some(sources) = members(canonical, &mirror.source) else {
            continue;
        };
        let overrides: BTreeMap<_, _> = mirror
            .overrides
            .iter()
            .map(|item| (item.source_id.as_str(), &item.state))
            .collect();
        let mut projected = Vec::new();
        for source_id in sources {
            let source = parts.get(source_id.as_str()).copied();
            if let Some(part) =
                project_part(mirror, source, overrides.get(source_id.as_str()).copied())
            {
                projected.push(part);
            }
        }
        for (source_id, state) in overrides {
            if !sources.iter().any(|id| id == source_id)
                && let MirrorOverride::Local(part) = state
            {
                projected.push((**part).clone());
            }
        }

        let board_id = match &mirror.destination {
            MirrorDestination::SecondBoard(board_id) => board_id,
            MirrorDestination::SameBoardLayout(layout_id) => {
                let Some(layout) = geometry
                    .layouts
                    .iter_mut()
                    .find(|layout| &layout.id == layout_id)
                else {
                    continue;
                };
                layout
                    .part_ids
                    .extend(projected.iter().map(|part| part.id.clone()));
                &layout.board_id
            }
        };
        if let Some(board) = geometry
            .boards
            .iter_mut()
            .find(|board| &board.id == board_id)
        {
            board
                .part_ids
                .extend(projected.iter().map(|part| part.id.clone()));
            geometry.parts.extend(projected);
        }
    }
    ResolvedDoc { geometry }
}

fn members<'a>(doc: &'a ProjectDoc, location: &MirrorDestination) -> Option<&'a [String]> {
    match location {
        MirrorDestination::SecondBoard(id) => doc
            .boards
            .iter()
            .find(|board| &board.id == id)
            .map(|board| board.part_ids.as_slice()),
        MirrorDestination::SameBoardLayout(id) => doc
            .layouts
            .iter()
            .find(|layout| &layout.id == id)
            .map(|layout| layout.part_ids.as_slice()),
    }
}

fn project_part(
    mirror: &MirroredPair,
    source: Option<&Part>,
    state: Option<&MirrorOverride>,
) -> Option<Part> {
    match state {
        Some(MirrorOverride::Local(part)) => return Some((**part).clone()),
        Some(MirrorOverride::Omitted) => return None,
        _ => {}
    }

    let mut part = source?.clone();
    part.id = format!("{}/{}/{}", mirror.id.len(), mirror.id, part.id);
    part.pose = match state {
        Some(MirrorOverride::CompanionPose(pose)) => *pose,
        _ => Pose2 {
            at: pair::forward(mirror.mapping, part.pose.at),
            rotation: pair::forward_angle(mirror.mapping, part.pose.rotation),
        },
    };
    Some(part)
}
