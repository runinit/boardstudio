//! `CreateMirroredHalf`: Core creates the second half of a mechanical pair from its source
//! (ADR-0010). A new board receives a reflection of the source board through the pair's
//! `Pose2` map; a reused board gets a flipped instance and copies nothing.
//!
//! Everything is built on a clone of the document and committed only when the whole record
//! validates, so a refusal changes nothing. The changed ids are the new pair, both instances,
//! the new board, and the source and mirror id of every correspondence.

use crate::matrix::layout;
use crate::mechanical::MechanicalTarget;
use crate::mechanical::{features, halves, inputs::effective_inputs, pair};
use crate::model::{
    Board, BoardOutline, ElectricalBoardConfiguration, Layout, MechanicalConfiguration,
    MechanicalConstructionOwnership, MechanicalMirrorPair, MirroredCorrespondence,
    MirroredHalfMode, MirroredHalves, MirroredIdRule, MirroredItemKind, Net, OutlineBridge,
    OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineProvenance, OutlineSnapshot,
    OutlineVersion, Part, PhysicalBoardInstance, Pin, Pose2, ProjectDoc,
};
use std::collections::{BTreeMap, BTreeSet};

/// Creates the second half and returns the changed ids, sorted.
pub(crate) fn create(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_instance_id: &str,
    mode: &MirroredHalfMode,
    mapping: Pose2,
    id_rule: MirroredIdRule,
) -> Result<Vec<String>, String> {
    let MirroredIdRule::BoardPrefix = id_rule;
    if !(mapping.at.x.is_finite() && mapping.at.y.is_finite() && mapping.rotation.is_finite()) {
        return Err("The mirrored half's mapping must be finite.".into());
    }
    let mut next = doc.clone();
    let changed = match mode {
        MirroredHalfMode::ReuseBoard { instance_id } => {
            reuse_board(&mut next, pair_id, source_instance_id, instance_id, mapping)?
        }
        MirroredHalfMode::NewBoard {
            board_id,
            instance_id,
        } => new_board(
            &mut next,
            pair_id,
            source_instance_id,
            board_id,
            instance_id,
            mapping,
        )?,
    };
    layout::validate(&next)?;
    halves::validate_document(&next)?;
    *doc = next;
    Ok(changed)
}

/// The mirror id of a source id under the board-prefix rule: every whole `from` token becomes
/// `to`, and an id with no such token is prefixed with `to/`.
pub(crate) fn mirror_id(from: &str, to: &str, id: &str) -> String {
    substitute_token(id, from, to).unwrap_or_else(|| format!("{to}/{id}"))
}

/// Replaces each whole token equal to `from` (tokens are split at `/` and `-`), or `None` when
/// no token matched.
fn substitute_token(text: &str, from: &str, to: &str) -> Option<String> {
    let mut out = String::new();
    let mut matched = false;
    for piece in text.split_inclusive(['/', '-']) {
        let (token, delimiter) = match piece.chars().last() {
            Some('/' | '-') => (&piece[..piece.len() - 1], &piece[piece.len() - 1..]),
            _ => (piece, ""),
        };
        if token == from {
            out.push_str(to);
            matched = true;
        } else {
            out.push_str(token);
        }
        out.push_str(delimiter);
    }
    matched.then_some(out)
}

/// A reflected pose: the point through the pair map, the heading by the pair's rule.
pub(crate) fn mirror_pose(mapping: Pose2, pose: Pose2) -> Pose2 {
    Pose2 {
        at: pair::forward(mapping, pose.at),
        rotation: pair::forward_angle(mapping, pose.rotation),
    }
}

/// The mirror's stored configuration: the source's settings, with its openings and mounts
/// removed. Linked features reach the mirror through the pair (reconcile, ticket 19), and closure
/// mounts are regenerated from the pair, never copied.
fn mirror_configuration(source: &MechanicalConfiguration) -> MechanicalConfiguration {
    let mut configuration = source.clone();
    configuration.openings = Some(vec![]);
    configuration.mounts.clear();
    configuration
}

fn source_half(
    doc: &ProjectDoc,
    pair_id: &str,
    source_id: &str,
) -> Result<(PhysicalBoardInstance, MechanicalConfiguration), String> {
    let hardware = doc
        .hardware
        .as_ref()
        .ok_or("Physical instances are unavailable")?;
    if pair_id.trim().is_empty() || hardware.pairs.iter().any(|pair| pair.id == pair_id) {
        return Err(format!(
            "A mechanical pair named {pair_id:?} already exists."
        ));
    }
    let instance = hardware
        .instances
        .iter()
        .find(|instance| instance.id == source_id)
        .ok_or_else(|| format!("Physical instance {source_id:?} is unavailable."))?
        .clone();
    if hardware
        .pairs
        .iter()
        .any(|pair| pair.members.iter().any(|member| member == source_id))
    {
        return Err(format!(
            "Physical instance {source_id:?} already belongs to a mechanical pair."
        ));
    }
    if !doc.boards.iter().any(|board| board.id == instance.board_id) {
        return Err(format!(
            "The board of physical instance {source_id:?} is unavailable."
        ));
    }
    let configuration = effective_inputs(
        doc,
        &[],
        &MechanicalTarget::Instance {
            instance_id: source_id.into(),
        },
    )?
    .configuration;
    Ok((instance, configuration))
}

/// Refuses a board that already names a half of a mirrored-half record.
fn ensure_board_unclaimed(doc: &ProjectDoc, board_id: &str) -> Result<(), String> {
    let Some(hardware) = doc.hardware.as_ref() else {
        return Ok(());
    };
    for pair in &hardware.pairs {
        if let Some(halves) = &pair.halves
            && (halves.source_board_id == board_id || halves.mirror_board_id == board_id)
        {
            return Err(format!(
                "Board {board_id:?} is already a half of mechanical pair {:?}.",
                pair.id
            ));
        }
    }
    Ok(())
}

fn ensure_instance_fresh(doc: &ProjectDoc, instance_id: &str) -> Result<(), String> {
    let exists = doc
        .hardware
        .as_ref()
        .is_some_and(|hardware| hardware.instances.iter().any(|i| i.id == instance_id));
    if instance_id.trim().is_empty() || exists {
        return Err(format!("Physical instance {instance_id:?} already exists."));
    }
    Ok(())
}

/// The mirror's half label and role: the opposite half of a left/right pair, with the recipes'
/// central/peripheral roles.
fn mirror_half_label(source_half: &str) -> &str {
    match source_half {
        "left" => "right",
        "right" => "left",
        other => other,
    }
}

fn push_pair(doc: &mut ProjectDoc, pair: MechanicalMirrorPair) {
    if let Some(hardware) = doc.hardware.as_mut() {
        hardware.pairs.push(pair);
    }
}

fn source_owned_by_pair(
    doc: &mut ProjectDoc,
    source_id: &str,
    configuration: &MechanicalConfiguration,
) {
    if let Some(hardware) = doc.hardware.as_mut()
        && let Some(instance) = hardware
            .instances
            .iter_mut()
            .find(|instance| instance.id == source_id)
    {
        if instance.mechanical.is_none() {
            instance.mechanical = Some(configuration.clone());
        }
        instance.construction_ownership = MechanicalConstructionOwnership::Pair;
    }
}

fn reuse_board(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_id: &str,
    instance_id: &str,
    mapping: Pose2,
) -> Result<Vec<String>, String> {
    let (source, configuration) = source_half(doc, pair_id, source_id)?;
    if source.flipped {
        return Err(format!(
            "Physical instance {source_id:?} is already flipped; a reused board needs an unflipped source."
        ));
    }
    ensure_board_unclaimed(doc, &source.board_id)?;
    ensure_instance_fresh(doc, instance_id)?;
    let board = source.board_id.clone();
    source_owned_by_pair(doc, source_id, &configuration);
    if let Some(hardware) = doc.hardware.as_mut() {
        hardware.instances.push(PhysicalBoardInstance {
            id: instance_id.into(),
            name: "Right half".into(),
            board_id: board.clone(),
            half: "right".into(),
            role: "peripheral".into(),
            flipped: true,
            controller_part_id: None,
            mechanical: Some(mirror_configuration(&configuration)),
            construction_ownership: MechanicalConstructionOwnership::Pair,
            mechanical_identity_reservation: None,
        });
    }
    push_pair(
        doc,
        MechanicalMirrorPair {
            id: pair_id.into(),
            members: [source_id.into(), instance_id.into()],
            mapping,
            construction: features::shared_construction(&configuration),
            construction_source: Some(source_id.into()),
            halves: Some(MirroredHalves {
                source_board_id: board.clone(),
                mirror_board_id: board,
                correspondences: vec![],
                exceptions: vec![],
            }),
            features: vec![],
        },
    );
    let mut changed = vec![
        pair_id.to_owned(),
        source_id.to_owned(),
        instance_id.to_owned(),
    ];
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Every id a document already uses in the namespaces a mirror writes into.
fn taken_ids(doc: &ProjectDoc) -> BTreeSet<String> {
    let mut taken = BTreeSet::new();
    taken.extend(doc.boards.iter().map(|b| b.id.clone()));
    taken.extend(doc.parts.iter().map(|p| p.id.clone()));
    taken.extend(doc.matrices.iter().map(|m| m.id.clone()));
    taken.extend(doc.layouts.iter().map(|l| l.id.clone()));
    taken.extend(doc.outline.iter().map(|o| o.id().to_owned()));
    taken.extend(doc.nets.iter().map(|n| n.id.clone()));
    taken.extend(
        doc.board_outlines
            .iter()
            .flat_map(|outline| outline.versions.iter().map(|v| v.id.clone())),
    );
    taken
}

fn mint(taken: &mut BTreeSet<String>, id: String) -> Result<String, String> {
    if !taken.insert(id.clone()) {
        return Err(format!("The mirrored id {id:?} is already in use."));
    }
    Ok(id)
}

/// Maps the points of a feature through the pair; a reflected polygon keeps its orientation by
/// reversing its point order.
fn mirror_feature(
    feature: &OutlineFeature,
    mapping: Pose2,
    map: &dyn Fn(&str) -> String,
) -> OutlineFeature {
    match feature {
        OutlineFeature::Polygon {
            anchor_part_id,
            id,
            points,
            operation,
        } => OutlineFeature::Polygon {
            anchor_part_id: anchor_part_id.as_deref().map(map),
            id: map(id),
            points: points
                .iter()
                .rev()
                .map(|point| pair::forward(mapping, *point))
                .collect(),
            operation: *operation,
        },
        OutlineFeature::Rect {
            rotation,
            anchor_part_id,
            id,
            center,
            size,
            radius,
            operation,
        } => OutlineFeature::Rect {
            rotation: (rotation.is_some() || mapping.rotation != 0.0)
                .then(|| pair::forward_angle(mapping, rotation.unwrap_or(0.0))),
            anchor_part_id: anchor_part_id.as_deref().map(map),
            id: map(id),
            center: pair::forward(mapping, *center),
            size: *size,
            radius: *radius,
            operation: *operation,
        },
        OutlineFeature::PartEnvelope {
            connections,
            settings,
            id,
            part_ids,
            margin,
            operation,
        } => OutlineFeature::PartEnvelope {
            connections: connections
                .iter()
                .map(|connection| OutlineConnection {
                    id: map(&connection.id),
                    width: connection.width,
                    points: connection
                        .points
                        .iter()
                        .map(|point| OutlineControlPoint {
                            at: pair::forward(mapping, point.at),
                            part_id: point.part_id.as_deref().map(map),
                        })
                        .collect(),
                })
                .collect(),
            settings: settings.clone(),
            id: map(id),
            part_ids: part_ids.iter().map(|part| map(part)).collect(),
            margin: *margin,
            operation: *operation,
        },
    }
}

/// A fixed outline version's snapshot, reflected. Protected gaps are not mirrored yet, so a
/// snapshot that has any is refused rather than silently losing them.
fn mirror_snapshot(
    snapshot: &OutlineSnapshot,
    mapping: Pose2,
    map: &dyn Fn(&str) -> String,
    matrix_map: &BTreeMap<String, String>,
) -> Result<OutlineSnapshot, String> {
    if !snapshot.protected_gaps.is_empty() {
        return Err("Protected outline gaps cannot be mirrored yet; remove them first.".into());
    }
    Ok(OutlineSnapshot {
        features: snapshot
            .features
            .iter()
            .map(|feature| mirror_feature(feature, mapping, map))
            .collect(),
        settings: snapshot.settings.clone(),
        expected_regions: snapshot.expected_regions,
        bridges: snapshot
            .bridges
            .iter()
            .map(|bridge| OutlineBridge {
                id: map(&bridge.id),
                width: bridge.width,
                points: bridge
                    .points
                    .iter()
                    .map(|point| pair::forward(mapping, *point))
                    .collect(),
                part_ids: bridge.part_ids.iter().map(|part| map(part)).collect(),
                matrix_ids: bridge
                    .matrix_ids
                    .iter()
                    .map(|matrix| {
                        matrix_map
                            .get(matrix)
                            .cloned()
                            .unwrap_or_else(|| matrix.clone())
                    })
                    .collect(),
                authored: bridge.authored,
            })
            .collect(),
        protected_gaps: vec![],
    })
}

/// A mirror's reference: the board token substituted, or `to-reference` when none matches.
fn mirror_reference(from: &str, to: &str, reference: &str) -> String {
    substitute_token(reference, from, to).unwrap_or_else(|| format!("{to}-{reference}"))
}

fn corresponds(kind: MirroredItemKind, source: &str, mirror: &str) -> MirroredCorrespondence {
    MirroredCorrespondence {
        kind,
        source_id: source.to_owned(),
        mirror_id: mirror.to_owned(),
    }
}

fn new_board(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_id: &str,
    board_id: &str,
    instance_id: &str,
    mapping: Pose2,
) -> Result<Vec<String>, String> {
    let (source, configuration) = source_half(doc, pair_id, source_id)?;
    let from = source.board_id.clone();
    let to = board_id.to_owned();
    ensure_board_unclaimed(doc, &from)?;
    if to.trim().is_empty() || doc.boards.iter().any(|board| board.id == to) {
        return Err(format!("Board {to:?} already exists."));
    }
    ensure_instance_fresh(doc, instance_id)?;
    let source_board = doc
        .boards
        .iter()
        .find(|board| board.id == from)
        .cloned()
        .ok_or("The source board is unavailable")?;
    if doc
        .modules
        .iter()
        .any(|module| module.host_board_id == from)
        || doc
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.host_board_id == from)
    {
        return Err(format!(
            "Board {from:?} hosts modules or embedded circuits, which cannot be mirrored yet."
        ));
    }

    let map = |id: &str| mirror_id(&from, &to, id);

    // Every new id is minted before anything is written, so a collision refuses the record.
    let mut taken = taken_ids(doc);
    mint(&mut taken, to.clone())?;
    let mut part_map = BTreeMap::new();
    for part in &source_board.part_ids {
        let mirrored = mint(&mut taken, map(part))?;
        part_map.insert(part.clone(), mirrored);
    }
    let matrices: Vec<_> = doc
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref() == Some(from.as_str()))
        .cloned()
        .collect();
    let mut matrix_map = BTreeMap::new();
    for matrix in &matrices {
        let mirrored = mint(&mut taken, map(&matrix.id))?;
        matrix_map.insert(matrix.id.clone(), mirrored);
    }
    let matrix_owned: BTreeSet<&str> = matrices
        .iter()
        .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str))
        .collect();
    let layouts: Vec<_> = doc
        .layouts
        .iter()
        .filter(|layout| layout.board_id == from)
        .cloned()
        .collect();
    let mut layout_map = BTreeMap::new();
    for layout in &layouts {
        let mirrored = mint(&mut taken, map(&layout.id))?;
        layout_map.insert(layout.id.clone(), mirrored);
    }
    let mut outline_map = BTreeMap::new();
    for outline in &source_board.outline_ids {
        if !doc.outline.iter().any(|feature| feature.id() == outline) {
            return Err(format!("Board outline {outline:?} is unavailable."));
        }
        let mirrored = mint(&mut taken, map(outline))?;
        outline_map.insert(outline.clone(), mirrored);
    }
    let mut net_map = BTreeMap::new();
    for net in &source_board.net_ids {
        if !doc.nets.iter().any(|item| item.id == *net) {
            return Err(format!("Net {net:?} is unavailable."));
        }
        let mirrored = mint(&mut taken, map(net))?;
        net_map.insert(net.clone(), mirrored);
    }
    let board_outline = doc
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == from)
        .cloned();
    let mut version_map = BTreeMap::new();
    if let Some(outline) = &board_outline {
        for version in &outline.versions {
            let mirrored = mint(&mut taken, map(&version.id))?;
            version_map.insert(version.id.clone(), mirrored);
        }
    }
    let electrical = doc.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|entry| entry.board_id == from)
            .cloned()
    });
    let part_set: BTreeSet<&str> = source_board.part_ids.iter().map(String::as_str).collect();
    let net_set: BTreeSet<&str> = source_board.net_ids.iter().map(String::as_str).collect();
    let mapped_reference = |id: &str| -> String {
        if let Some(part) = part_map.get(id) {
            part.clone()
        } else if let Some(net) = net_map.get(id) {
            net.clone()
        } else {
            id.to_owned()
        }
    };

    let mut correspondences = Vec::new();

    // The board. Its part list is set once every part exists.
    doc.boards.push(Board {
        id: to.clone(),
        name: format!("{} (mirror)", source_board.name),
        outline_ids: source_board
            .outline_ids
            .iter()
            .map(|id| outline_map[id].clone())
            .collect(),
        part_ids: vec![],
        net_ids: source_board
            .net_ids
            .iter()
            .map(|id| net_map[id].clone())
            .collect(),
        thickness: source_board.thickness,
        traces: vec![],
        vias: vec![],
    });

    // Fixed outline features the board references.
    for outline in &source_board.outline_ids {
        let feature = doc
            .outline
            .iter()
            .find(|feature| feature.id() == outline)
            .cloned()
            .ok_or("Board outline is unavailable")?;
        doc.outline.push(mirror_feature(&feature, mapping, &map));
        correspondences.push(corresponds(
            MirroredItemKind::Outline,
            outline,
            &outline_map[outline],
        ));
    }

    // Matrices reflect through the pair map, keeping the source numbering. `set_matrix` places
    // each matrix's members and gives them their ids; their attributes are copied afterwards.
    for matrix in &matrices {
        let mut template = matrix.clone();
        template.id = matrix_map[&matrix.id].clone();
        template.board_id = Some(to.clone());
        template.cells.clear();
        template.part_ids.clear();
        let reflected = layout::reflected(matrix, &template, mapping, true)?;
        crate::matrix::set_matrix(doc, &reflected)?;
        correspondences.push(corresponds(
            MirroredItemKind::Matrix,
            &matrix.id,
            &matrix_map[&matrix.id],
        ));
        for member in &matrix.part_ids {
            let source_part = doc
                .parts
                .iter()
                .find(|part| part.id == *member)
                .cloned()
                .ok_or("A matrix member is unavailable")?;
            let mirrored = part_map
                .get(member)
                .ok_or("A matrix member has no mirror id")?
                .clone();
            let placed = doc
                .parts
                .iter_mut()
                .find(|part| part.id == mirrored)
                .ok_or("A mirrored matrix member was not placed")?;
            placed.keycap = source_part.keycap;
            placed.outline = source_part.outline.clone();
            placed.locked = source_part.locked;
            placed.properties = source_part.properties.clone();
            placed.generator_parameters = source_part.generator_parameters.clone();
        }
    }

    // Parts. Matrix members are already placed; other parts copy and reflect here.
    let mut references: BTreeSet<String> = doc
        .parts
        .iter()
        .map(|part| part.reference.clone())
        .collect();
    for id in &source_board.part_ids {
        correspondences.push(corresponds(MirroredItemKind::Part, id, &part_map[id]));
        if matrix_owned.contains(id.as_str()) {
            continue;
        }
        let part = doc
            .parts
            .iter()
            .find(|part| part.id == *id)
            .cloned()
            .ok_or("A board part is unavailable")?;
        let reference = mirror_reference(&from, &to, &part.reference);
        if !references.insert(reference.clone()) {
            return Err(format!(
                "The mirrored reference {reference:?} is already in use."
            ));
        }
        // A footprint with a reversible variant takes it on the mirror.
        let mut generator_parameters = part.generator_parameters.clone();
        if let Some(parameters) = generator_parameters.as_mut()
            && parameters.contains_key("reversible")
        {
            parameters.insert("reversible".into(), serde_json::json!(true));
        }
        doc.parts.push(Part {
            keycap: part.keycap,
            outline: part.outline.clone(),
            id: part_map[id].clone(),
            definition_id: part.definition_id.clone(),
            reference,
            pose: mirror_pose(mapping, part.pose),
            side: part.side.clone(),
            locked: part.locked,
            properties: part.properties.clone(),
            generator_parameters,
        });
    }

    // Layouts: independent copies of the source's layouts on the new board.
    for layout in &layouts {
        let matrix_id = matrix_map
            .get(&layout.matrix_id)
            .ok_or("A layout's matrix is not on its board")?
            .clone();
        let part_ids = layout
            .part_ids
            .iter()
            .map(|id| {
                part_map
                    .get(id)
                    .cloned()
                    .ok_or_else(|| format!("Layout part {id:?} is not on board {from:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        doc.layouts.push(Layout {
            id: layout_map[&layout.id].clone(),
            name: layout.name.clone(),
            board_id: to.clone(),
            matrix_id,
            part_ids,
            mirror_link: None,
        });
        correspondences.push(corresponds(
            MirroredItemKind::Layout,
            &layout.id,
            &layout_map[&layout.id],
        ));
    }

    // Nets keep their names; a pin on a part of the source board follows the part.
    for net in &source_board.net_ids {
        let source_net = doc
            .nets
            .iter()
            .find(|item| item.id == *net)
            .cloned()
            .ok_or("A board net is unavailable")?;
        let pins = source_net
            .pins
            .iter()
            .filter(|pin| part_set.contains(pin.part_id.as_str()))
            .map(|pin| Pin {
                part_id: part_map[&pin.part_id].clone(),
                pad_id: pin.pad_id.clone(),
            })
            .collect();
        doc.nets.push(Net {
            id: net_map[net].clone(),
            name: source_net.name.clone(),
            pins,
        });
        correspondences.push(corresponds(MirroredItemKind::Net, net, &net_map[net]));
    }

    // Keycap settings keyed by the source board, its matrices and its keys. A setting that
    // already names a mirror id is stale data, so it refuses rather than being overwritten.
    if let Some(source_keycaps) = doc.keycaps.clone()
        && let Some(target) = doc.keycaps.as_mut()
    {
        let copy = |existing: bool, key: &str| -> Result<(), String> {
            if existing {
                return Err(format!("A keycap setting already names {key:?}."));
            }
            Ok(())
        };
        if let Some(settings) = source_keycaps.boards.get(&from) {
            copy(target.boards.contains_key(&to), &to)?;
            target.boards.insert(to.clone(), settings.clone());
        }
        for (matrix, settings) in &source_keycaps.matrices {
            if let Some(mirrored) = matrix_map.get(matrix) {
                copy(target.matrices.contains_key(mirrored), mirrored)?;
                target.matrices.insert(mirrored.clone(), settings.clone());
            }
        }
        for (part, settings) in &source_keycaps.keys {
            if let Some(mirrored) = part_map.get(part) {
                copy(target.keys.contains_key(mirrored), mirrored)?;
                target.keys.insert(mirrored.clone(), settings.clone());
            }
        }
    }

    // The electrical configuration of the source board, with the ids it names reflected.
    if let Some(entry) = electrical {
        let controller_part_id = entry
            .controller_part_id
            .as_deref()
            .filter(|id| part_set.contains(id))
            .map(mapped_reference);
        let remap = |map: &BTreeMap<String, String>| -> BTreeMap<String, String> {
            map.iter()
                .map(|(key, value)| {
                    let key = if part_set.contains(key.as_str()) {
                        mapped_reference(key)
                    } else {
                        key.clone()
                    };
                    let value =
                        if part_set.contains(value.as_str()) || net_set.contains(value.as_str()) {
                            mapped_reference(value)
                        } else {
                            value.clone()
                        };
                    (key, value)
                })
                .collect()
        };
        let mirrored = ElectricalBoardConfiguration {
            board_id: to.clone(),
            controller_part_id,
            mode: entry.mode,
            locks: remap(&entry.locks),
            assignments: remap(&entry.assignments),
            key_bindings: remap(&entry.key_bindings),
            jumper_states: entry.jumper_states.clone(),
            protected_handoff: None,
        };
        let hardware = doc
            .hardware
            .as_mut()
            .ok_or("Physical instances are unavailable")?;
        if hardware.boards.iter().any(|entry| entry.board_id == to) {
            return Err(format!(
                "Board {to:?} already has an electrical configuration."
            ));
        }
        hardware.boards.push(mirrored);
        correspondences.push(corresponds(MirroredItemKind::ElectricalBoard, &from, &to));
    }

    // Fixed outline versions: the geometry reflects, and the stored record starts without its
    // generated snapshot or retired identities.
    if let Some(outline) = board_outline {
        if doc.board_outlines.iter().any(|entry| entry.board_id == to) {
            return Err(format!("Board {to:?} already has a board outline."));
        }
        let mut versions = Vec::new();
        for version in &outline.versions {
            let geometry = mirror_snapshot(&version.geometry, mapping, &map, &matrix_map)?;
            let provenance = OutlineProvenance {
                revision: version.source.revision,
                version_id: version
                    .source
                    .version_id
                    .as_ref()
                    .and_then(|id| version_map.get(id).cloned()),
            };
            versions.push(OutlineVersion {
                id: version_map[&version.id].clone(),
                name: version.name.clone(),
                source: provenance,
                geometry,
                ownership: version.ownership,
            });
            correspondences.push(corresponds(
                MirroredItemKind::OutlineVersion,
                &version.id,
                &version_map[&version.id],
            ));
        }
        doc.board_outlines.push(BoardOutline {
            board_id: to.clone(),
            active_version_id: outline
                .active_version_id
                .as_ref()
                .and_then(|id| version_map.get(id).cloned()),
            versions,
            generated_last_valid: None,
            retired_version_ids: vec![],
        });
    }

    // The board lists its parts in the source's order.
    let board_parts: Vec<String> = source_board
        .part_ids
        .iter()
        .map(|id| part_map[id].clone())
        .collect();
    if let Some(board) = doc.boards.iter_mut().find(|board| board.id == to) {
        board.part_ids = board_parts;
    }

    // The mirror instance, and the source instance that the pair now owns.
    source_owned_by_pair(doc, source_id, &configuration);
    let half = mirror_half_label(&source.half).to_owned();
    let mirror_config = mirror_configuration(&configuration);
    if let Some(hardware) = doc.hardware.as_mut() {
        hardware.instances.push(PhysicalBoardInstance {
            id: instance_id.into(),
            name: match half.as_str() {
                "left" => "Left half".into(),
                "right" => "Right half".into(),
                _ => format!("{} mirror", source.name),
            },
            board_id: to.clone(),
            role: if half == "right" {
                "peripheral"
            } else {
                "central"
            }
            .into(),
            half,
            flipped: false,
            controller_part_id: source
                .controller_part_id
                .as_deref()
                .filter(|id| part_set.contains(id))
                .map(mapped_reference),
            mechanical: Some(mirror_config),
            construction_ownership: MechanicalConstructionOwnership::Pair,
            mechanical_identity_reservation: None,
        });
    }
    push_pair(
        doc,
        MechanicalMirrorPair {
            id: pair_id.into(),
            members: [source_id.into(), instance_id.into()],
            mapping,
            construction: features::shared_construction(&configuration),
            construction_source: Some(source_id.into()),
            halves: Some(MirroredHalves {
                source_board_id: from,
                mirror_board_id: to.clone(),
                correspondences: correspondences.clone(),
                exceptions: vec![],
            }),
            features: vec![],
        },
    );

    let mut changed: BTreeSet<String> = BTreeSet::new();
    changed.extend([
        pair_id.to_owned(),
        source_id.to_owned(),
        instance_id.to_owned(),
        to,
    ]);
    for correspondence in &correspondences {
        changed.insert(correspondence.source_id.clone());
        changed.insert(correspondence.mirror_id.clone());
    }
    Ok(changed.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Vec2;

    #[test]
    fn mirror_ids_substitute_whole_board_tokens_only() {
        let mirror = |id: &str| mirror_id("left", "right", id);
        assert_eq!(mirror("left/U1"), "right/U1");
        assert_eq!(mirror("left-U1"), "right-U1");
        assert_eq!(mirror("left-outline"), "right-outline");
        assert_eq!(mirror("matrix/left-keys"), "matrix/right-keys");
        assert_eq!(
            mirror("matrix/left-keys/r0c0/diode"),
            "matrix/right-keys/r0c0/diode"
        );
        // A token inside a word is not the board.
        assert_eq!(mirror("leftover/U1"), "right/leftover/U1");
        // An id that names no board is prefixed with the mirror board.
        assert_eq!(mirror("outline-1"), "right/outline-1");
    }

    #[test]
    fn a_mirrored_pose_takes_the_pair_heading_rule() {
        let mapping = Pose2 {
            at: Vec2 { x: 10.0, y: 0.0 },
            rotation: 30.0,
        };
        let pose = Pose2 {
            at: Vec2 { x: 0.0, y: 5.0 },
            rotation: 20.0,
        };
        let mirrored = mirror_pose(mapping, pose);
        // The heading is the pair rotation minus the source heading.
        assert!((mirrored.rotation - 10.0).abs() < 1e-12);
        let expected = pair::forward(mapping, pose.at);
        assert_eq!(mirrored.at, expected);
    }

    #[test]
    fn a_non_finite_mapping_is_refused_before_anything_is_written() {
        let mut doc = ProjectDoc::empty("doc", "doc");
        let before = doc.clone();
        let mapping = Pose2 {
            at: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
            rotation: 0.0,
        };
        let error = create(
            &mut doc,
            "pair-right",
            "left",
            &MirroredHalfMode::ReuseBoard {
                instance_id: "right".into(),
            },
            mapping,
            MirroredIdRule::BoardPrefix,
        )
        .unwrap_err();
        assert!(error.contains("finite"), "{error}");
        assert_eq!(doc, before);
    }
}
