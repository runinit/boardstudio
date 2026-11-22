use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Part, ProjectDoc};
use crate::{MirrorDestination, MirrorOverride, MirroredPair};

/// Geometry for consumers. This type has no archive encoder.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedDoc {
    geometry: ProjectDoc,
    targets_by_source: BTreeMap<String, Vec<String>>,
}

impl ResolvedDoc {
    pub fn geometry(&self) -> &ProjectDoc {
        &self.geometry
    }

    pub fn affected_targets<'a>(
        &'a self,
        sources: &'a [String],
    ) -> impl Iterator<Item = &'a String> {
        sources
            .iter()
            .filter_map(|source| self.targets_by_source.get(source))
            .flatten()
    }
}

pub fn project_document(canonical: &ProjectDoc) -> ResolvedDoc {
    let mut geometry = canonical.clone();
    let mut targets_by_source = BTreeMap::<String, Vec<String>>::new();
    let parts: BTreeMap<_, _> = canonical
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();

    for mirror in &canonical.mirrored_pairs {
        let Some(sources) = members(canonical, &mirror.source) else {
            continue;
        };
        let matrices: Vec<_> = canonical
            .matrices
            .iter()
            .filter(|matrix| owns_matrix(canonical, &mirror.source, matrix))
            .collect();
        let sources: BTreeSet<_> = sources
            .iter()
            .chain(matrices.iter().flat_map(|matrix| &matrix.part_ids))
            .collect();
        let overrides: BTreeMap<_, _> = mirror
            .overrides
            .iter()
            .map(|item| (item.source_id.as_str(), &item.state))
            .collect();
        let mut projected = Vec::new();
        let mut ids = BTreeMap::new();
        for source_id in &sources {
            let source = parts.get(source_id.as_str()).copied();
            if let Some(part) = project_part(
                canonical,
                mirror,
                source,
                overrides.get(source_id.as_str()).copied(),
            ) {
                ids.insert(source_id.as_str(), part.id.clone());
                targets_by_source
                    .entry((*source_id).clone())
                    .or_default()
                    .push(part.id.clone());
                projected.push(part);
            }
        }
        for (source_id, state) in overrides {
            if !ids.contains_key(source_id)
                && !sources.iter().any(|id| id.as_str() == source_id)
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
                layout.part_ids.extend(
                    projected
                        .iter()
                        .filter(|part| {
                            !matrices.iter().any(|matrix| {
                                matrix
                                    .part_ids
                                    .iter()
                                    .any(|id| ids.get(id.as_str()) == Some(&part.id))
                            })
                        })
                        .map(|part| part.id.clone()),
                );
                layout.matrix_ids.extend(
                    matrices
                        .iter()
                        .map(|matrix| entity_id(mirror, "matrix", &matrix.id)),
                );
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
            if let Some(keycaps) = &mut geometry.keycaps {
                for (source_id, target_id) in &ids {
                    if let Some(settings) = keycaps.keys.get(*source_id).cloned() {
                        keycaps.keys.entry(target_id.clone()).or_insert(settings);
                    }
                }
                for matrix in &matrices {
                    if let Some(settings) = keycaps.matrices.get(&matrix.id).cloned() {
                        keycaps
                            .matrices
                            .entry(entity_id(mirror, "matrix", &matrix.id))
                            .or_insert(settings);
                    }
                }
            }
            for source_matrix in matrices {
                let mut matrix = crate::reflect::matrix(mirror, source_matrix, board_id);
                matrix.part_ids = source_matrix
                    .part_ids
                    .iter()
                    .filter_map(|id| ids.get(id.as_str()).cloned())
                    .collect();
                targets_by_source
                    .entry(source_matrix.id.clone())
                    .or_default()
                    .push(matrix.id.clone());
                geometry.matrices.push(matrix);
            }
            for source_net in &canonical.nets {
                let pins: Vec<_> = source_net
                    .pins
                    .iter()
                    .filter_map(|pin| {
                        let mut target = pin.clone();
                        target.part_id.clone_from(ids.get(pin.part_id.as_str())?);
                        Some(target)
                    })
                    .collect();
                if pins.is_empty() {
                    continue;
                }
                match &mirror.destination {
                    MirrorDestination::SecondBoard(_) => {
                        let mut net = source_net.clone();
                        net.id = entity_id(mirror, "net", &source_net.id);
                        net.pins = pins;
                        board.net_ids.push(net.id.clone());
                        geometry.nets.push(net);
                    }
                    MirrorDestination::SameBoardLayout(_) => {
                        if let Some(net) =
                            geometry.nets.iter_mut().find(|net| net.id == source_net.id)
                        {
                            net.pins.extend(pins);
                        }
                    }
                }
            }
            if let MirrorDestination::SecondBoard(source_board) = &mirror.source
                && let Some(source_board) = canonical
                    .boards
                    .iter()
                    .find(|board| &board.id == source_board)
            {
                if let (Some(authored), Some(resolved)) =
                    (&canonical.hardware, &mut geometry.hardware)
                {
                    let source_instance = authored
                        .instances
                        .iter()
                        .find(|item| item.board_id == source_board.id);
                    let authored_config = source_instance
                        .and_then(|item| item.mechanical.as_ref())
                        .or_else(|| {
                            canonical
                                .mechanical
                                .as_ref()
                                .filter(|item| item.board_id == source_board.id)
                        });
                    for instance in resolved
                        .instances
                        .iter_mut()
                        .filter(|item| item.board_id == *board_id)
                    {
                        if let Some(config) = authored_config {
                            instance.mechanical =
                                Some(crate::reflect::mechanical(mirror, config, board_id));
                        }
                        if let Some(source) = source_instance {
                            instance.controller_part_id = source
                                .controller_part_id
                                .as_ref()
                                .and_then(|id| ids.get(id.as_str()).cloned());
                        }
                    }
                    if !authored
                        .boards
                        .iter()
                        .any(|item| item.board_id == *board_id)
                        && let Some(source) = authored
                            .boards
                            .iter()
                            .find(|item| item.board_id == source_board.id)
                    {
                        let mut target = source.clone();
                        target.board_id.clone_from(board_id);
                        target.controller_part_id = source
                            .controller_part_id
                            .as_ref()
                            .and_then(|id| ids.get(id.as_str()).cloned());
                        target.key_bindings = source
                            .key_bindings
                            .iter()
                            .filter_map(|(id, value)| {
                                Some((ids.get(id.as_str())?.clone(), value.clone()))
                            })
                            .collect();
                        target.protected_handoff = None;
                        resolved.boards.push(target);
                    }
                }
                if let Some(keycaps) = &mut geometry.keycaps
                    && let Some(settings) = keycaps.boards.get(&source_board.id).cloned()
                {
                    keycaps.boards.entry(board_id.clone()).or_insert(settings);
                }
                for module in canonical
                    .modules
                    .iter()
                    .filter(|module| module.host_board_id == source_board.id)
                {
                    let mut projected = module.clone();
                    projected.id = entity_id(mirror, "module", &module.id);
                    projected.host_board_id.clone_from(board_id);
                    projected.host_instance_id = None;
                    let pose = crate::reflect::pose(
                        mirror.mapping,
                        crate::model::Pose2 {
                            at: module.at,
                            rotation: module.rotation,
                        },
                    );
                    projected.at = pose.at;
                    projected.rotation = pose.rotation;
                    if let Some(connection) = &mut projected.connection {
                        if let Some(id) = ids.get(connection.host_connector_part_id.as_str()) {
                            connection.host_connector_part_id.clone_from(id);
                            if let Some(upstream) = &mut connection.upstream_module_id {
                                *upstream = entity_id(mirror, "module", upstream);
                            }
                        } else {
                            projected.connection = None;
                        }
                    }
                    geometry.modules.push(projected);
                }
                for feature in canonical
                    .outline
                    .iter()
                    .filter(|feature| source_board.outline_ids.iter().any(|id| id == feature.id()))
                {
                    if let Some(feature) = crate::reflect::outline(mirror, feature, &ids) {
                        board.outline_ids.push(feature.id().into());
                        geometry.outline.push(feature);
                    }
                }
            }
            geometry.parts.extend(projected);
        }
    }
    ResolvedDoc {
        geometry,
        targets_by_source,
    }
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
    doc: &ProjectDoc,
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
    part.id = part_id(doc, mirror, &part.id);
    if let Some(anchor) = &mut part.anchor {
        if doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == anchor.matrix_id && owns_matrix(doc, &mirror.source, matrix))
        {
            anchor.matrix_id = entity_id(mirror, "matrix", &anchor.matrix_id);
            anchor.spread = -anchor.spread;
            anchor.splay = -anchor.splay;
            anchor.offset.x = -anchor.offset.x;
            anchor.rotation = -anchor.rotation;
        } else {
            part.anchor = None;
        }
    }
    part.pose = match state {
        Some(MirrorOverride::CompanionPose(pose)) => *pose,
        _ => crate::reflect::pose(mirror.mapping, part.pose),
    };
    Some(part)
}

pub(crate) fn part_id(doc: &ProjectDoc, mirror: &MirroredPair, source_id: &str) -> String {
    for matrix in &doc.matrices {
        if owns_matrix(doc, &mirror.source, matrix)
            && matrix.part_ids.iter().any(|id| id == source_id)
            && let Some(member) = source_id.strip_prefix(&format!("matrix/{}/", matrix.id))
        {
            return format!(
                "matrix/{}/{member}",
                entity_id(mirror, "matrix", &matrix.id)
            );
        }
    }
    entity_id(mirror, "part", source_id)
}

pub(crate) fn entity_id(mirror: &MirroredPair, kind: &str, source_id: &str) -> String {
    if kind == "matrix" {
        use std::fmt::Write;
        let mut id = String::from("mirror_");
        for byte in mirror.id.bytes() {
            write!(id, "{byte:02x}").unwrap();
        }
        id.push('_');
        id.push_str(source_id);
        return id;
    }
    format!("{}/{}/{kind}/{source_id}", mirror.id.len(), mirror.id)
}

pub(crate) fn owns_matrix(
    doc: &ProjectDoc,
    location: &MirrorDestination,
    matrix: &crate::model::Matrix,
) -> bool {
    match location {
        MirrorDestination::SecondBoard(id) => matrix.board_id.as_ref() == Some(id),
        MirrorDestination::SameBoardLayout(id) => doc
            .layouts
            .iter()
            .any(|layout| &layout.id == id && layout.matrix_ids.contains(&matrix.id)),
    }
}

pub(crate) fn owns_part(doc: &ProjectDoc, location: &MirrorDestination, id: &str) -> bool {
    members(doc, location).is_some_and(|ids| ids.iter().any(|part_id| part_id == id))
        || doc.matrices.iter().any(|matrix| {
            owns_matrix(doc, location, matrix)
                && matrix.part_ids.iter().any(|part_id| part_id == id)
        })
}
