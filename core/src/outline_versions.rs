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

pub(crate) fn features<'a>(doc: &'a ProjectDoc, board: &Board) -> Vec<&'a OutlineFeature> {
    if let Some(snapshot) = active_snapshot(doc, &board.id) {
        snapshot.features.iter().collect()
    } else {
        board
            .outline_ids
            .iter()
            .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
            .collect()
    }
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
        });
        doc.board_outlines.len() - 1
    };
    Ok(&mut doc.board_outlines[index])
}

fn name(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 {
        return Err("Outline name must contain 1 to 120 characters".into());
    }
    Ok(value.into())
}

fn snapshot(doc: &ProjectDoc, board_id: &str, prefix: &str) -> Result<OutlineSnapshot, String> {
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

fn generated_snapshot(
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

pub(crate) fn apply(
    doc: &mut ProjectDoc,
    operation: &EditOperation,
) -> Result<Vec<String>, String> {
    let board_id = match operation {
        EditOperation::CopyOutline {
            board_id,
            version_id,
            name: label,
            edit,
            feature,
        } => {
            let label = name(label)?;
            if version_id.trim().is_empty() || version_id == "generated" {
                return Err("A fixed outline needs a unique version ID".into());
            }
            if doc.board_outlines.iter().any(|state| {
                state
                    .versions
                    .iter()
                    .any(|version| &version.id == version_id)
            }) {
                return Err(format!("Duplicate outline version {version_id}"));
            }
            let prefix = format!("board:{board_id}:version:{version_id}");
            let mut geometry = snapshot(doc, board_id, &prefix)?;
            if let Some(edit) = edit {
                let Some(feature) = geometry.features.get_mut(edit.contour as usize) else {
                    return Err("Outline contour is missing".into());
                };
                *feature = OutlineFeature::Polygon {
                    id: feature.id().into(),
                    anchor_part_id: None,
                    points: edit.points.clone(),
                    operation: feature.operation(),
                };
            }
            if let Some(feature) = feature {
                geometry.features.push(feature.clone());
            }
            let source = OutlineProvenance {
                revision: doc.revision,
                version_id: doc
                    .board_outlines
                    .iter()
                    .find(|state| &state.board_id == board_id)
                    .and_then(|state| state.active_version_id.clone()),
            };
            let state = state_mut(doc, board_id)?;
            state.versions.push(OutlineVersion {
                id: version_id.clone(),
                name: label,
                source,
                geometry,
            });
            state.active_version_id = Some(version_id.clone());
            board_id
        }
        EditOperation::SelectOutline {
            board_id,
            version_id,
        } => {
            let state = state_mut(doc, board_id)?;
            if let Some(id) = version_id
                && !state.versions.iter().any(|version| &version.id == id)
            {
                return Err(format!("Unknown outline version {id}"));
            }
            state.active_version_id = version_id.clone();
            board_id
        }
        EditOperation::RenameOutline {
            board_id,
            version_id,
            name: label,
        } => {
            let label = name(label)?;
            let version = state_mut(doc, board_id)?
                .versions
                .iter_mut()
                .find(|version| &version.id == version_id)
                .ok_or("Outline version is missing")?;
            version.name = label;
            board_id
        }
        EditOperation::RemoveOutline {
            board_id,
            version_id,
        } => {
            let state = state_mut(doc, board_id)?;
            if !state
                .versions
                .iter()
                .any(|version| &version.id == version_id)
            {
                return Err("Outline version is missing".into());
            }
            state.versions.retain(|version| &version.id != version_id);
            if state.active_version_id.as_ref() == Some(version_id) {
                state.active_version_id = None;
            }
            board_id
        }
        _ => return Err("Unsupported outline version command".into()),
    };
    Ok(vec![board_id.clone()])
}
