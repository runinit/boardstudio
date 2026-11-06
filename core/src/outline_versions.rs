//! Board-owned fixed copies. Generated source features keep their legacy live behavior.
use crate::{geometry, model::*};

pub(crate) fn active_snapshot<'a>(
    doc: &'a ProjectDoc,
    board_id: &str,
) -> Option<&'a OutlineSnapshot> {
    let state = doc
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id)?;
    if let Some(id) = &state.active_version_id {
        state
            .versions
            .iter()
            .find(|version| &version.id == id)
            .map(|version| &version.geometry)
    } else if protection_problem(doc, board_id).is_some() {
        state.generated_last_valid.as_ref()
    } else {
        None
    }
}

pub(crate) fn protection_problem(doc: &ProjectDoc, board_id: &str) -> Option<(String, String)> {
    let board = doc.boards.iter().find(|board| board.id == board_id)?;
    for feature in &doc.outline {
        if !board.outline_ids.iter().any(|id| id == feature.id()) {
            continue;
        }
        let OutlineFeature::PartEnvelope { settings, .. } = feature else {
            continue;
        };
        for gap in settings.repair.iter().flat_map(|repair| &repair.keep_gaps) {
            match gap
                .points
                .iter()
                .map(|control| crate::outline_controls::point(doc, control))
                .collect::<Result<Vec<_>, _>>()
            {
                Err(message) => return Some((gap.id.clone(), message)),
                Ok(points) if !geometry::valid_polygon(&points) => {
                    return Some((
                        gap.id.clone(),
                        "Protected gap no longer has a valid source region".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    None
}

pub(crate) fn active_version<'a>(
    doc: &'a ProjectDoc,
    board_id: &str,
) -> Option<&'a OutlineVersion> {
    let state = doc
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id)?;
    let id = state.active_version_id.as_ref()?;
    state.versions.iter().find(|version| &version.id == id)
}

/// The board's current included components, in document order.
///
/// This is the same population the generated boundary uses: components the board owns
/// that are not excluded from the outline and are not utility parts. A linked
/// refinement resolves from this live set, so additions, removals and inclusion
/// changes follow without rewriting the version's persisted choices.
pub(crate) fn included_parts(doc: &ProjectDoc, board_id: &str) -> Vec<String> {
    let Some(board) = doc.boards.iter().find(|board| board.id == board_id) else {
        return vec![];
    };
    board
        .part_ids
        .iter()
        .filter(|id| {
            doc.parts.iter().any(|part| {
                &part.id == *id
                    && !part
                        .outline
                        .as_ref()
                        .is_some_and(|outline| outline.excluded)
                    && doc.definitions.iter().any(|definition| {
                        definition.id == part.definition_id && definition.kind != PartKind::Utility
                    })
            })
        })
        .cloned()
        .collect()
}

/// Current projection of one version's geometry.
///
/// A fixed edited outline is its own geometry. A linked refinement recomputes its
/// envelope input from the board's current included components while keeping the
/// version's settings, authored connections and authored cutouts; its persisted
/// feature list is the last resolved projection, not another recipe Core follows.
pub(crate) fn resolved_features(
    doc: &ProjectDoc,
    board_id: &str,
    version: &OutlineVersion,
) -> Vec<OutlineFeature> {
    if version.ownership.is_fixed() {
        return version.geometry.features.clone();
    }
    let included = included_parts(doc, board_id);
    version
        .geometry
        .features
        .iter()
        .map(|feature| match feature {
            OutlineFeature::PartEnvelope {
                connections,
                id,
                margin,
                operation,
                ..
            } => OutlineFeature::PartEnvelope {
                connections: connections.clone(),
                settings: version.geometry.settings.clone(),
                id: id.clone(),
                part_ids: included.clone(),
                margin: *margin,
                operation: *operation,
            },
            authored => authored.clone(),
        })
        .collect()
}

/// Number of separate board regions the board's current outline owner expects.
///
/// A fixed edited outline is authored, so its persisted count stays authoritative; a
/// generated outline keeps its current behavior, including the retained last valid
/// projection while a protected gap has no source region. A linked refinement follows
/// the board's current components, so its count is derived from the current structure
/// exactly as the permanent Generated version derives it: a mirrored pair resolves
/// each half independently and therefore expects two regions. A refinement accepted
/// before its board split must not inherit the older count and block its own valid
/// geometry, and the fallback display of a failed resolution uses the same derived
/// count so it never adds a second, false blocker.
pub(crate) fn expected_regions(doc: &ProjectDoc, board_id: &str) -> usize {
    if let Some(version) = active_version(doc, board_id) {
        if version.ownership.is_fixed() {
            return version.geometry.expected_regions as usize;
        }
    } else if let Some(snapshot) = active_snapshot(doc, board_id) {
        return snapshot.expected_regions as usize;
    }
    if doc
        .layouts
        .iter()
        .any(|layout| layout.board_id == board_id && layout.mirror_link.is_some())
    {
        2
    } else {
        1
    }
}

pub(crate) fn features(doc: &ProjectDoc, board: &Board) -> Vec<OutlineFeature> {
    if let Some(version) = active_version(doc, &board.id) {
        return resolved_features(doc, &board.id, version);
    }
    // While a protected gap has no valid source region the generated source keeps
    // displaying its retained last valid projection.
    if let Some(snapshot) = active_snapshot(doc, &board.id) {
        return snapshot.features.clone();
    }
    board
        .outline_ids
        .iter()
        .filter_map(|id| {
            doc.outline
                .iter()
                .find(|feature| feature.id() == id)
                .cloned()
        })
        .collect()
}

pub(crate) fn settings<'a>(doc: &'a ProjectDoc, board_id: &str) -> Option<&'a OutlineSettings> {
    active_snapshot(doc, board_id).map(|snapshot| &snapshot.settings)
}

fn state_mut<'a>(doc: &'a mut ProjectDoc, board_id: &str) -> Result<&'a mut BoardOutline, String> {
    if !doc.boards.iter().any(|board| board.id == board_id) {
        return Err(format!("Unknown board {board_id}"));
    }
    let index = if let Some(index) = doc
        .board_outlines
        .iter()
        .position(|state| state.board_id == board_id)
    {
        index
    } else {
        doc.board_outlines.push(BoardOutline {
            board_id: board_id.into(),
            active_version_id: None,
            versions: vec![],
            generated_last_valid: None,
            retired_version_ids: vec![],
        });
        doc.board_outlines.len() - 1
    };
    Ok(&mut doc.board_outlines[index])
}

pub(crate) fn name(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 {
        return Err("Outline name must contain 1 to 120 characters".into());
    }
    Ok(value.into())
}

/// The persisted outline entity that already owns `identity`, if any.
///
/// Outline identities share one document namespace: `SetOutline` addresses a feature by
/// ID across every version and the top-level outline, and a version's protected gaps,
/// authored connections and bridges are addressed by ID inside their version. A new
/// version identity that reused one of those names would address the wrong entity, so
/// Core refuses it and names the namespace it found.
pub(crate) fn identity_namespace(doc: &ProjectDoc, identity: &str) -> Option<String> {
    if doc.outline.iter().any(|feature| feature.id() == identity) {
        return Some("a top-level outline feature".into());
    }
    for feature in &doc.outline {
        if let OutlineFeature::PartEnvelope { connections, .. } = feature
            && connections
                .iter()
                .any(|connection| connection.id == identity)
        {
            return Some("a generated outline connection".into());
        }
    }
    for state in &doc.board_outlines {
        for version in &state.versions {
            if version
                .geometry
                .features
                .iter()
                .any(|feature| feature.id() == identity)
            {
                return Some(format!("a feature of outline version {}", version.id));
            }
            let authored_connection = version.geometry.features.iter().any(|feature| {
                matches!(feature, OutlineFeature::PartEnvelope { connections, .. }
                    if connections.iter().any(|connection| connection.id == identity))
            });
            if authored_connection {
                return Some(format!("a connection of outline version {}", version.id));
            }
            if version
                .geometry
                .protected_gaps
                .iter()
                .any(|gap| gap.id == identity)
            {
                return Some(format!("a protected gap of outline version {}", version.id));
            }
            if version
                .geometry
                .bridges
                .iter()
                .any(|bridge| bridge.id == identity)
            {
                return Some(format!("a bridge of outline version {}", version.id));
            }
        }
    }
    None
}

/// Why a proposed outline version identity cannot be created, if it cannot.
///
/// One rule for every creator: the identity must be usable, must not duplicate a live
/// version, must not reuse a retired one (never reissued, so a pending edit that names
/// a removed version can never land on its replacement), and must not collide with any
/// other persisted outline namespace.
pub(crate) fn version_identity_problem(doc: &ProjectDoc, version_id: &str) -> Option<String> {
    if version_id.trim().is_empty() || version_id == "generated" {
        return Some("A fixed outline needs a unique version ID".into());
    }
    if doc.board_outlines.iter().any(|state| {
        state
            .versions
            .iter()
            .any(|version| version.id == *version_id)
    }) {
        return Some(format!("Duplicate outline version {version_id}"));
    }
    if doc.board_outlines.iter().any(|state| {
        state
            .retired_version_ids
            .iter()
            .any(|retired| retired == version_id)
    }) {
        return Some(format!(
            "Outline version {version_id} was removed and its identity stays reserved; create the replacement under a new identity"
        ));
    }
    identity_namespace(doc, version_id)
        .map(|namespace| format!("Outline version {version_id} would collide with {namespace}"))
}

pub(crate) fn snapshot(
    doc: &ProjectDoc,
    board_id: &str,
    prefix: &str,
) -> Result<OutlineSnapshot, String> {
    let board = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| format!("Unknown board {board_id}"))?;
    if let Some(original) = active_snapshot(doc, board_id) {
        let mut copy = original.clone();
        for (index, feature) in copy.features.iter_mut().enumerate() {
            match feature {
                OutlineFeature::Polygon { id, .. }
                | OutlineFeature::Rect { id, .. }
                | OutlineFeature::PartEnvelope { id, .. } => {
                    *id = format!("{prefix}:contour:{index}")
                }
            }
        }
        return Ok(copy);
    }
    let (cache, _, _) = geometry::outlines(doc, None, &[]);
    generated_snapshot(doc, &cache, board, prefix)
}

pub(crate) fn generated_snapshot(
    doc: &ProjectDoc,
    cache: &geometry::OutlineCache,
    board: &Board,
    prefix: &str,
) -> Result<OutlineSnapshot, String> {
    let composed = geometry::generated_source(doc, cache, board);
    if composed.source.is_empty()
        || composed
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    {
        return Err("Resolve the generated outline before making a fixed copy".into());
    }
    let mut settings = board
        .outline_ids
        .iter()
        .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { settings, .. } => Some(settings.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let mut protected_gaps = vec![];
    for feature in board
        .outline_ids
        .iter()
        .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
    {
        if let OutlineFeature::PartEnvelope { settings, .. } = feature {
            for gap in settings.repair.iter().flat_map(|repair| &repair.keep_gaps) {
                protected_gaps.push(ProtectedOutlineGap {
                    id: gap.id.clone(),
                    points: gap
                        .points
                        .iter()
                        .map(|control| {
                            crate::outline_controls::point(doc, control)
                                .map(|at| OutlineControlPoint { at, part_id: None })
                        })
                        .collect::<Result<_, _>>()?,
                });
            }
        }
    }
    if let Some(repair) = &mut settings.repair {
        repair.keep_gaps.clear();
    }
    Ok(OutlineSnapshot {
        expected_regions: composed
            .source
            .iter()
            .filter(|contour| !contour.hole)
            .count() as u32,
        features: composed
            .source
            .into_iter()
            .enumerate()
            .map(|(index, contour)| {
                let id = format!("{prefix}:contour:{index}");
                if contour.hole
                    && let Some(feature) =
                        geometry::retained_cutout(doc, cache, board, &contour, id.clone())
                {
                    return feature;
                }
                OutlineFeature::Polygon {
                    anchor_part_id: None,
                    id,
                    points: contour.points,
                    operation: if contour.hole {
                        Operation::Subtract
                    } else {
                        Operation::Add
                    },
                }
            })
            .collect(),
        settings,
        bridges: composed.bridges,
        protected_gaps,
    })
}

/// Drop authored choices that require removed source components.
///
/// This runs inside the accepted edit that removes the components, so the cleanup and
/// the removal share one Undo step. Only choices that name a removed component are
/// dropped: an authored connection, protected gap or anchored cutout that requires it.
/// Independent choices survive, and a surviving choice is never retargeted onto
/// another component. Fixed edited outlines keep their geometry; their clearance
/// findings report the changed fit instead (ADR-0001).
///
/// Returns the identities of the choices it removed.
pub(crate) fn prune_removed_sources(doc: &mut ProjectDoc, removed: &[String]) -> Vec<String> {
    if removed.is_empty() {
        return vec![];
    }
    let requires = |point: &OutlineControlPoint| {
        point
            .part_id
            .as_ref()
            .is_some_and(|id| removed.iter().any(|removed| removed == id))
    };
    let gap_requires = |gap: &ProtectedOutlineGap| gap.points.iter().any(requires);
    let mut changed = vec![];
    for state in &mut doc.board_outlines {
        for version in &mut state.versions {
            if version.ownership.is_fixed() {
                continue;
            }
            if let Some(repair) = &mut version.geometry.settings.repair {
                repair.keep_gaps.retain(|gap| {
                    let keep = !gap_requires(gap);
                    if !keep {
                        changed.push(gap.id.clone());
                    }
                    keep
                });
            }
            version.geometry.protected_gaps.retain(|gap| {
                let keep = !gap_requires(gap);
                if !keep {
                    changed.push(gap.id.clone());
                }
                keep
            });
            version
                .geometry
                .features
                .retain_mut(|feature| match feature {
                    OutlineFeature::PartEnvelope {
                        part_ids,
                        connections,
                        settings,
                        ..
                    } => {
                        part_ids.retain(|id| !removed.iter().any(|removed| removed == id));
                        connections.retain(|connection| {
                            let keep = !connection.points.iter().any(requires);
                            if !keep {
                                changed.push(connection.id.clone());
                            }
                            keep
                        });
                        if let Some(repair) = &mut settings.repair {
                            repair.keep_gaps.retain(|gap| {
                                let keep = !gap_requires(gap);
                                if !keep {
                                    changed.push(gap.id.clone());
                                }
                                keep
                            });
                        }
                        true
                    }
                    OutlineFeature::Polygon {
                        anchor_part_id: Some(anchor),
                        id,
                        ..
                    }
                    | OutlineFeature::Rect {
                        anchor_part_id: Some(anchor),
                        id,
                        ..
                    } => {
                        let keep = !removed.iter().any(|removed| removed == anchor);
                        if !keep {
                            changed.push(id.clone());
                        }
                        keep
                    }
                    _ => true,
                });
        }
    }
    changed.sort();
    changed.dedup();
    changed
}

/// The outline version a located finding is attributed to, when its ID names one.
///
/// Blocking is decided per owner: a failure of an inactive alternative is visible but
/// cannot block output from the selected outline or another board (ADR-0001/0002).
pub(crate) fn finding_version(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix("board:")?;
    let (board_id, rest) = rest.split_once(":outline:linked:")?;
    Some((board_id.to_owned(), rest.to_owned()))
}

/// Whether a finding belongs to an outline version the board has not selected.
pub(crate) fn is_inactive_version_finding(doc: &ProjectDoc, id: &str) -> bool {
    let Some((board_id, version_id)) = finding_version(id) else {
        return false;
    };
    doc.board_outlines
        .iter()
        .find(|state| state.board_id == board_id)
        .is_none_or(|state| state.active_version_id.as_deref() != Some(version_id.as_str()))
}

pub(crate) fn refresh_recovery(doc: &mut ProjectDoc, cache: &geometry::OutlineCache) {
    for board in doc.boards.clone() {
        let has_protection = board.outline_ids.iter().filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
            .any(|feature| matches!(feature, OutlineFeature::PartEnvelope { settings, .. } if settings.repair.as_ref().is_some_and(|repair| !repair.keep_gaps.is_empty())));
        if !has_protection || protection_problem(doc, &board.id).is_some() {
            continue;
        }
        if let Ok(snapshot) = generated_snapshot(
            doc,
            cache,
            &board,
            &format!("board:{}:generated:last-valid", board.id),
        ) && let Ok(state) = state_mut(doc, &board.id)
        {
            state.generated_last_valid = Some(snapshot);
        }
    }
}
