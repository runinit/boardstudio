/// Imported MX stabilisers: ADR-0010 keeps their world heading in every companion mode. Their
/// position takes the X reflection, and they never raise an uncertain-heading finding.
pub const SYMMETRIC_DEFINITIONS: [&str; 2] = [
    "kicad:marbastlib/STAB_MX_2u",
    "kicad:marbastlib/STAB_MX_6.25u",
];

use crate::model::{Matrix, MirroredExceptionState, MirroredItemKind, Part, Pose2, ProjectDoc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirroredPair {
    pub id: String,
    pub source: MirrorDestination,
    pub destination: MirrorDestination,
    pub mapping: Pose2,
    pub overrides: Vec<PartOverride>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum MirrorDestination {
    SecondBoard(String),
    SameBoardLayout(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartOverride {
    pub source_id: String,
    pub state: MirrorOverride,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum MirrorOverride {
    Local(Box<Part>),
    Omitted,
    CompanionPose(Pose2),
}

pub fn mirrored_matrix_id_prefix(pair_id: &str) -> String {
    use std::fmt::Write;
    let mut prefix = String::with_capacity(8 + pair_id.len() * 2);
    prefix.push_str("mirror_");
    for byte in pair_id.bytes() {
        let _ = write!(prefix, "{byte:02x}");
    }
    prefix.push('_');
    prefix
}

pub fn mirrored_matrix_id(pair_id: &str, source_id: &str) -> String {
    let mut id = mirrored_matrix_id_prefix(pair_id);
    id.push_str(source_id);
    id
}

/// The deterministic derived id of one mirrored entity. Matrix ids use the
/// hex-prefixed form; every other kind folds the pair id and kind into the id.
/// This is the single authority for derived identities: the projector, the
/// admission guard and every consumer resolve pairs through it, never through
/// stored correspondence lists.
pub fn mirrored_entity_id(pair: &MirroredPair, kind: &str, source_id: &str) -> String {
    if kind == "matrix" {
        return mirrored_matrix_id(&pair.id, source_id);
    }
    format!("{}/{}/{kind}/{source_id}", pair.id.len(), pair.id)
}

/// The deterministic derived id of one mirrored part. Parts that belong to a
/// mirrored matrix keep their matrix-relative member name under the derived
/// matrix id; every other part folds the pair id into its id.
pub fn mirrored_part_id(doc: &ProjectDoc, pair: &MirroredPair, source_id: &str) -> String {
    for matrix in &doc.matrices {
        if owns_matrix(doc, &pair.source, matrix)
            && matrix.part_ids.iter().any(|id| id == source_id)
            && let Some(member) = source_id.strip_prefix(&format!("matrix/{}/", matrix.id))
        {
            return format!(
                "matrix/{}/{member}",
                mirrored_entity_id(pair, "matrix", &matrix.id)
            );
        }
    }
    mirrored_entity_id(pair, "part", source_id)
}

/// The authored members a mirror location projects: board parts for a second
/// board, layout parts for a same-board layout.
pub fn members<'a>(doc: &'a ProjectDoc, location: &MirrorDestination) -> Option<&'a [String]> {
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

/// Whether `matrix` is authored at `location` and therefore projected.
pub fn owns_matrix(doc: &ProjectDoc, location: &MirrorDestination, matrix: &Matrix) -> bool {
    match location {
        MirrorDestination::SecondBoard(id) => matrix.board_id.as_ref() == Some(id),
        MirrorDestination::SameBoardLayout(id) => doc
            .layouts
            .iter()
            .any(|layout| &layout.id == id && layout.matrix_ids.contains(&matrix.id)),
    }
}

/// Whether `id` is authored at `location`: a direct member or a key of an
/// owned matrix.
pub fn owns_part(doc: &ProjectDoc, location: &MirrorDestination, id: &str) -> bool {
    members(doc, location).is_some_and(|ids| ids.iter().any(|part_id| part_id == id))
        || doc.matrices.iter().any(|matrix| {
            owns_matrix(doc, location, matrix)
                && matrix.part_ids.iter().any(|part_id| part_id == id)
        })
}

/// The sparse override a pair stores for one authored source id, if any.
pub fn override_of<'a>(pair: &'a MirroredPair, source_id: &str) -> Option<&'a MirrorOverride> {
    pair.overrides
        .iter()
        .find(|item| item.source_id == source_id)
        .map(|item| &item.state)
}

/// The authored source deriving `part_id` through one pair, if any, plus the
/// source's sparse override (`None` while fully linked). A `Local` part keeps
/// its stored id, so it resolves through the override's payload rather than
/// the source scan.
pub fn pair_derivation<'a>(
    doc: &'a ProjectDoc,
    pair: &'a MirroredPair,
    part_id: &str,
) -> Option<(&'a str, Option<&'a MirrorOverride>)> {
    for item in &pair.overrides {
        if let MirrorOverride::Local(part) = &item.state
            && part.id == part_id
        {
            return Some((item.source_id.as_str(), Some(&item.state)));
        }
    }
    let mut sources: Vec<&str> = members(doc, &pair.source)
        .map(|ids| ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    sources.extend(
        doc.matrices
            .iter()
            .filter(|matrix| owns_matrix(doc, &pair.source, matrix))
            .flat_map(|matrix| matrix.part_ids.iter().map(String::as_str)),
    );
    for source_id in sources {
        if mirrored_part_id(doc, pair, source_id) == part_id {
            return Some((source_id, override_of(pair, source_id)));
        }
    }
    None
}

/// The effective state of one derived part id: the pair and authored source
/// that produce it, plus the source's sparse override (`None` while fully
/// linked).
pub fn derived_part<'a>(
    doc: &'a ProjectDoc,
    part_id: &str,
) -> Option<(&'a MirroredPair, &'a str, Option<&'a MirrorOverride>)> {
    doc.mirrored_pairs.iter().find_map(|pair| {
        pair_derivation(doc, pair, part_id).map(|(source, state)| (pair, source, state))
    })
}

/// The pair and authored source deriving `part_id` while linked: a live
/// derived mirror with no `Local` or `Omitted` override (a `CompanionPose`
/// override stays linked). Such a part is not authored.
pub fn linked_source<'a>(
    doc: &'a ProjectDoc,
    part_id: &str,
) -> Option<(&'a MirroredPair, &'a str)> {
    derived_part(doc, part_id).and_then(|(pair, source_id, state)| match state {
        None | Some(MirrorOverride::CompanionPose(_)) => Some((pair, source_id)),
        Some(MirrorOverride::Local(_) | MirrorOverride::Omitted) => None,
    })
}

/// The authored source matrix deriving `matrix_id` through one pair, if any,
/// plus the source's sparse override.
pub fn pair_matrix_derivation<'a>(
    doc: &'a ProjectDoc,
    pair: &'a MirroredPair,
    matrix_id: &str,
) -> Option<(&'a str, Option<&'a MirrorOverride>)> {
    for matrix in &doc.matrices {
        if owns_matrix(doc, &pair.source, matrix)
            && mirrored_matrix_id(&pair.id, &matrix.id) == matrix_id
        {
            return Some((matrix.id.as_str(), override_of(pair, matrix.id.as_str())));
        }
    }
    None
}

/// The authored source matrix deriving `matrix_id`, if any pair derives it.
pub fn derived_matrix<'a>(
    doc: &'a ProjectDoc,
    matrix_id: &str,
) -> Option<(&'a MirroredPair, &'a str, Option<&'a MirrorOverride>)> {
    doc.mirrored_pairs.iter().find_map(|pair| {
        pair_matrix_derivation(doc, pair, matrix_id).map(|(source, state)| (pair, source, state))
    })
}

/// The counterpart matrix ids of `matrix_id`: the derived id when an authored
/// matrix sits at a pair source, or the source id when `matrix_id` is itself
/// a derived mirror. Layout links carry their own records.
pub fn matrix_counterparts(doc: &ProjectDoc, matrix_id: &str) -> Vec<String> {
    let mut counterparts = Vec::new();
    for pair in &doc.mirrored_pairs {
        for matrix in &doc.matrices {
            if !owns_matrix(doc, &pair.source, matrix) {
                continue;
            }
            let derived = mirrored_matrix_id(&pair.id, &matrix.id);
            if matrix.id == matrix_id {
                counterparts.push(derived);
            } else if derived == matrix_id {
                counterparts.push(matrix.id.clone());
            }
        }
    }
    counterparts
}

/// The overlay state covering a derived part or matrix id: the sparse
/// override a pair stores for its source, or `None` while fully linked. A
/// `CompanionPose` override stays linked, so it reports no overlay.
pub fn derived_override_state(
    doc: &ProjectDoc,
    kind: MirroredItemKind,
    id: &str,
) -> Option<MirroredExceptionState> {
    let state = match kind {
        MirroredItemKind::Part => derived_part(doc, id).map(|(_, _, state)| state)?,
        MirroredItemKind::Matrix => derived_matrix(doc, id).map(|(_, _, state)| state)?,
        _ => return None,
    };
    match state {
        None | Some(MirrorOverride::CompanionPose(_)) => None,
        Some(MirrorOverride::Local(_)) => Some(MirroredExceptionState::Local),
        Some(MirrorOverride::Omitted) => Some(MirroredExceptionState::Omitted),
    }
}
