use crate::{matrix, mechanical::pair, model::*};
use std::collections::{BTreeMap, BTreeSet};

const MIRRORED_COMPONENT_SOURCE: &str = "boardstudio.mirroredComponentSource";
const MIRRORED_COMPONENT_SUBSTITUTE: &str = "boardstudio.mirroredComponentSubstitute";

pub(crate) fn validate(doc: &ProjectDoc) -> Result<(), String> {
    if doc.layouts.is_empty() {
        return Ok(());
    }
    let mut ids = BTreeSet::new();
    let mut matrices = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut linked = BTreeSet::new();
    for layout in &doc.layouts {
        if layout.id.trim().is_empty() || layout.name.trim().is_empty() || !ids.insert(&layout.id) {
            return Err("Layout names and identities must be present and unique".into());
        }
        let board = doc
            .boards
            .iter()
            .find(|board| board.id == layout.board_id)
            .ok_or("Layout board is missing")?;
        if layout.matrix_ids.is_empty() {
            return Err("Layout matrix membership must be nonempty".into());
        }
        for id in &layout.matrix_ids {
            let matrix = matrix::evaluation::logical_matrix(id, doc)?;
            if !matrices.insert(matrix.id.clone()) || matrix.board_id.as_ref() != Some(&board.id) {
                return Err("Each layout must own distinct matrices on its board".into());
            }
        }
        for id in &layout.part_ids {
            if !parts.insert(id)
                || !board.part_ids.contains(id)
                || !doc.parts.iter().any(|part| &part.id == id)
                || doc
                    .matrices
                    .iter()
                    .any(|matrix| matrix.part_ids.contains(id))
            {
                return Err("Layout components must be independent parts on the same board".into());
            }
        }
        if let Some(link) = &layout.mirror_link {
            let source = doc
                .layouts
                .iter()
                .find(|other| other.id == link.source_id)
                .ok_or("Linked layout is missing")?;
            if !link.axis_x.is_finite()
                || source.id == layout.id
                || source.board_id != layout.board_id
                || source.mirror_link.is_some()
                || !linked.insert(&source.id)
                || !linked.insert(&layout.id)
            {
                return Err("A mirrored pair must link two distinct layouts on one board".into());
            }
        }
    }
    for part in &doc.parts {
        if primary_member(doc, &part.id)
            .is_some_and(|(matrix, _, _)| partner(doc, &matrix.id).is_some())
            && doc.constraint_driving_part(&part.id)?.is_some()
        {
            return Err("Unlink the halves before constraining a linked key's position".into());
        }
    }
    Ok(())
}

/// The relationship is stored once, but geometry can be edited from either half.
pub(crate) fn has_link(doc: &ProjectDoc, matrix_id: &str) -> bool {
    doc.layouts.iter().filter(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .any(|owner| owner.mirror_link.is_some() || doc.layouts.iter().any(|layout| layout.mirror_link.as_ref().is_some_and(|link| link.source_id == owner.id)))
}

pub(crate) fn partner(doc: &ProjectDoc, matrix_id: &str) -> Option<(String, f64)> {
    doc.layouts.iter().filter_map(|layout| layout.mirror_link.as_ref()).find_map(|link| {
        link.correspondences.iter().find_map(|record| {
            if record.kind != MirroredItemKind::Matrix { return None; }
            if record.source_id == matrix_id { Some((record.mirror_id.clone(), link.axis_x)) }
            else if record.mirror_id == matrix_id { Some((record.source_id.clone(), link.axis_x)) }
            else { None }
        })
    })
}

/// The pair map of a vertical mirror line at `axis_x`: the reflection `x' = 2 * axis_x - x`.
pub(crate) fn axis_mapping(axis_x: f64) -> Pose2 {
    Pose2 {
        at: Vec2 {
            x: 2.0 * axis_x,
            y: 0.0,
        },
        rotation: 0.0,
    }
}

/// A companion's offset and turn are relative to its part, so the reflection only flips their
/// handedness. The pair rotation is carried once, by the part's own heading.
fn reflected_companion(mut assembly: MatrixAssembly) -> MatrixAssembly {
    assembly.offset.x = -assembly.offset.x;
    assembly.rotation = assembly.rotation.map(|rotation| -rotation);
    assembly
}

/// Reflects `source` through a pair's `mapping` onto `target`, which keeps its identity, board and
/// part ids. The local frame flips (`mirror` toggles) so the mirror keeps the source's numbering:
/// column 0 is the same physical column on both halves. The origin and the matrix heading take
/// the pair's rule ([`crate::mechanical::pair::forward`] and `forward_angle`); everything the
/// matrix places relative to that frame (cell turns, companions) only flips handedness.
///
/// Key switch rule (ADR-0010, amendment on switches and companions). A switch takes no half turn.
/// Its world heading is the pair's `forward_angle` of its heading: the mirrored matrix frame carries
/// `forward_angle(map, matrix rotation)` and the column splays are negated by the mirror, and the
/// cell turn is `-r`, so the sum reproduces `forward_angle(map, world heading)` exactly for any map
/// (`-r` on an axis map, `rotation - r` under a rotated map). A companion (diode, LED) is an
/// assembly of the cell: it keeps its side (local x flips, local y does not) and takes the negated
/// member heading. No generator parameter is changed to suit the mirror.
pub(crate) fn reflected(
    source: &Matrix,
    target: &Matrix,
    mapping: Pose2,
    local_companions: bool,
) -> Result<Matrix, String> {
    // The pair uses a vertical reflection of the local None/X frames.
    // Switching to a horizontal local frame requires an independent layout.
    if source.mirror == Some(Mirror::Y) {
        return Err("Unlink the halves before changing to a horizontal matrix mirror".into());
    }
    let mut result = target.clone();
    result.rows = source.rows;
    result.columns = source.columns;
    result.pitch = source.pitch;
    result.edge_gap = source.edge_gap;
    result.origin = pair::forward(mapping, source.origin);
    result.rotation = if source.rotation.is_some() || mapping.rotation != 0.0 {
        Some(pair::forward_angle(mapping, source.rotation.unwrap_or(0.0)))
    } else {
        None
    };
    result.mirror = Some(if source.mirror == Some(Mirror::X) {
        Mirror::None
    } else {
        Mirror::X
    });
    result.diode_direction = source.diode_direction;
    result.row_layout = source.row_layout.clone();
    result.column_layout = source.column_layout.clone();
    result.anchor = source.anchor.clone();
    if let Some(anchor) = &mut result.anchor {
        anchor.matrix_id = target.anchor.as_ref().ok_or("Linked counterpart anchor is missing")?.matrix_id.clone();
        result.origin = Vec2 { x: -source.origin.x, y: source.origin.y };
        result.rotation = source.rotation.map(|r| -r);
    }
    let source_cells: BTreeMap<_, _> = source
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut cells: BTreeMap<_, _> = target
        .cells
        .iter()
        .filter(|cell| cell.row < source.rows && cell.column < source.columns)
        .map(|cell| ((cell.row, cell.column), cell.clone()))
        .collect();
    let local_preset = target.cells.iter().find(|cell| {
        cell.assemblies_local == Some(true)
            && cell.definition_id.as_deref() == Some(target.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    for (coordinate, source_cell) in &source_cells {
        cells.entry(*coordinate).or_insert_with(|| {
            // New keys inherit this half's saved hardware, not the other half's
            // local companion assignments. Existing per-key overrides stay intact.
            if (source_cell.row >= target.rows || source_cell.column >= target.columns)
                && let Some(template) = local_preset
            {
                return MatrixCell {
                    row: source_cell.row,
                    column: source_cell.column,
                    enabled: true,
                    offset: None,
                    rotation: None,
                    ..template.clone()
                };
            }
            MatrixCell {
                shift: None,
    bend: None,
    deleted: false,
                row: source_cell.row,
                column: source_cell.column,
                enabled: true,

                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }
        });
    }
    for (coordinate, cell) in &mut cells {
        let source_cell = source_cells.get(coordinate);
        cell.enabled = source_cell.is_none_or(|cell| cell.enabled);
        cell.deleted = source_cell.is_some_and(|cell| cell.deleted);
        cell.offset = source_cell.and_then(|cell| cell.offset);
        // Cell turns and companion offsets are relative to the matrix frame and to the part, which
        // already carry the pair rotation, so only their handedness flips here.
        cell.rotation = source_cell.and_then(|cell| cell.rotation);
        cell.shift = source_cell.and_then(|cell| cell.shift);
        cell.bend = source_cell.and_then(|cell| cell.bend);
        // A sync keeps the other half's local companions; a new half copies them, since it has none.
        if cell.assemblies_local != Some(true)
            && (local_companions
                || !source_cell.is_some_and(|cell| cell.assemblies_local == Some(true)))
        {
            cell.definition_id = source_cell.and_then(|cell| cell.definition_id.clone());
            cell.variant = source_cell.and_then(|cell| cell.variant.clone());
            cell.assemblies = source_cell
                .map(|cell| {
                    cell.assemblies
                        .iter()
                        .cloned()
                        .map(reflected_companion)
                        .collect()
                })
                .unwrap_or_default();
            cell.assemblies_local = if local_companions {
                source_cell.and_then(|cell| cell.assemblies_local)
            } else {
                None
            };
        }
    }
    result.cells = cells.into_values().collect();
    Ok(result)
}

pub(crate) fn sync(doc: &mut ProjectDoc, matrix_id: &str) -> Result<Vec<String>, String> {
    let Some((target_id, axis_x)) = partner(doc, matrix_id) else {
        return Ok(vec![]);
    };
    if exception_state(doc, MirroredItemKind::Matrix, &target_id) == Some(MirroredExceptionState::Omitted) {
        return Ok(vec![]);
    }
    let source = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Linked matrix is missing")?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    let source_is_target = doc
        .layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .is_some_and(|layout| layout.mirror_link.is_some());
    let sizes: BTreeMap<_, _> = matrix::cell_members(source)
        .into_iter()
        .filter_map(|(cell, id)| {
            doc.parts
                .iter()
                .find(|part| part.id == id)
                .and_then(|part| part.keycap)
                .map(|size| (cell, size))
        })
        .collect();
    let mapping = axis_mapping(axis_x);
    let mut reflected_matrix = reflected(source, target, mapping, false)?;
    if source_is_target {
        let canonical_cells: BTreeMap<_, _> = target
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut reflected_matrix.cells {
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column))
                && cell.assemblies_local != Some(true)
            {
                cell.definition_id = canonical.definition_id.clone();
                cell.variant = canonical.variant.clone();
                cell.assemblies = canonical.assemblies.clone();
                cell.assemblies_local = canonical.assemblies_local;
            }
        }
    }
    let mut changed = matrix::set_matrix(doc, &reflected_matrix)?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    for (cell, id) in matrix::cell_members(target) {
        if let Some(size) = sizes.get(&cell)
            && let Some(part) = doc.parts.iter_mut().find(|part| part.id == id)
        {
            part.keycap = Some(*size);
        }
    }
    if source_is_target {
        let canonical = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == target_id)
            .ok_or("Linked matrix is missing")?;
        let mut local = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .ok_or("Linked matrix is missing")?
            .clone();
        let canonical_cells: BTreeMap<_, _> = canonical
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut local.cells {
            if cell.assemblies_local == Some(true) {
                continue;
            }
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column)) {
                cell.assemblies = canonical
                    .assemblies
                    .iter()
                    .cloned()
                    .map(reflected_companion)
                    .collect();
            } else {
                cell.assemblies.clear();
            }
            cell.assemblies_local = None;
        }
        changed.extend(matrix::set_matrix(doc, &local)?);
    }
    Ok(changed)
}

pub(crate) fn resolve(doc: &mut ProjectDoc) -> Result<(), String> {
    validate(doc)?;
    for matrix in &doc.matrices {
        matrix::valid_projection_matrix(matrix)?;
        matrix::evaluated(matrix, doc)?;
    }
    let sources: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|layout| {
            let link = layout.mirror_link.as_ref()?;
            doc.layouts
                .iter()
                .find(|source| source.id == link.source_id)
                .map(|source| source.matrix_ids.clone())
        })
        .collect();
    for source in sources.into_iter().flatten() {
        sync(doc, &source)?;
    }
    matrix::placement::carrying_authored_companions(doc, matrix::placement::Reported::PreserveEditReport, |doc| {
        for index in matrix::evaluation::order(doc)? {
            let matrix = &doc.matrices[index];
            let evaluated = matrix::evaluated(matrix, doc)?;
            let mut poses = BTreeMap::new();
            for geometry in &evaluated.cells {
                let id = matrix::member_id(&matrix.id, geometry.row as u32, geometry.column as u32);
                if exception_state(doc, MirroredItemKind::Part, &id) == Some(MirroredExceptionState::Local) {
                    continue;
                }
                poses.insert(id.clone(), geometry.pose);
                if let Some(cell) = matrix.cells.iter().find(|cell| cell.row == geometry.row as u32 && cell.column == geometry.column as u32) {
                    for assembly in &cell.assemblies {
                        let id = matrix::companion_id(&id, &assembly.id);
                        if matches!(companion_mode(doc, &id), MatrixCompanionMode::Follows) {
                            let at = matrix::splay::rotate(assembly.offset, geometry.pose.rotation, Vec2::default());
                            poses.insert(id, Pose2 { at: Vec2 { x: geometry.pose.at.x + at.x, y: geometry.pose.at.y + at.y }, rotation: geometry.pose.rotation + assembly.rotation.unwrap_or_default() });
                        }
                    }
                }
            }
            for part in &mut doc.parts {
                if let Some(pose) = poses.get(&part.id)
                    && !part.properties.as_ref().is_some_and(|properties| properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))) {
                    part.pose = *pose;
                }
            }
        }
        Ok(vec![])
    })?;
    sync_components(doc)?;
    Ok(())
}

pub(crate) fn exception_state(doc: &ProjectDoc, kind: MirroredItemKind, id: &str) -> Option<MirroredExceptionState> {
    doc.layouts.iter().filter_map(|layout| layout.mirror_link.as_ref()).flat_map(|link| &link.exceptions)
        .chain(doc.hardware.iter().flat_map(|hardware| &hardware.pairs).filter_map(|pair| pair.halves.as_ref()).flat_map(|halves| &halves.exceptions))
        .find(|exception| exception.kind == kind && exception.mirror_id == id).map(|exception| exception.state)
}

pub(crate) fn omitted(doc: &ProjectDoc, matrix_id: &str, key_id: &str) -> bool {
    // Native reconciliation owns its staged membership plan; filter same-board overlays here.
    let state = |kind, id: &str| doc.layouts.iter().filter_map(|layout| layout.mirror_link.as_ref()).flat_map(|link| &link.exceptions)
        .find(|exception| exception.kind == kind && exception.mirror_id == id).map(|exception| exception.state);
    state(MirroredItemKind::Part, key_id)
        .or_else(|| state(MirroredItemKind::Matrix, matrix_id)) == Some(MirroredExceptionState::Omitted)
}

pub(crate) fn sync_components(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let pairs: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|target| {
            let link = target.mirror_link.as_ref()?;
            let source = doc
                .layouts
                .iter()
                .find(|layout| layout.id == link.source_id)?;
            Some((source.id.clone(), target.id.clone(), link.axis_x))
        })
        .collect();
    let mut changed = Vec::new();
    for (source_layout_id, target_layout_id, axis_x) in pairs {
        adopt_unassigned_components(doc, &source_layout_id, axis_x, &mut changed)?;
        let source_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_layout_id)
            .ok_or("Linked component source layout is missing")?
            .part_ids
            .clone();
        let target_layout_parts = doc
            .layouts
            .iter()
            .find(|layout| layout.id == target_layout_id)
            .ok_or("Linked component target layout is missing")?
            .part_ids
            .clone();

        let orphaned: Vec<_> = target_layout_parts
            .iter()
            .filter_map(|target_id| {
                let target = doc.parts.iter().find(|part| &part.id == target_id)?;
                let source_id = target
                    .properties
                    .as_ref()?
                    .get(MIRRORED_COMPONENT_SOURCE)?
                    .as_str()?;
                (!source_ids.iter().any(|id| id == source_id)).then(|| {
                    (
                        target_id.clone(),
                        source_id.to_string(),
                        doc.parts.iter().any(|part| part.id == source_id),
                    )
                })
            })
            .collect();
        for (target_id, source_id, source_exists) in orphaned {
            if source_exists {
                if let Some(part) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                    clear_component_link(part);
                    changed.push(target_id);
                }
            } else {
                remove_component(doc, &target_id);
                changed.extend([target_id, source_id]);
            }
        }

        for source_id in source_ids {
            let Some(mut source) = doc.parts.iter().find(|part| part.id == source_id).cloned()
            else {
                continue;
            };
            let target_id = doc.parts.iter().find_map(|target| {
                (target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(source_id.as_str()))
                .then(|| target.id.clone())
            });
            if source
                .properties
                .as_ref()
                .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                changed.push(source_id.clone());
                changed.extend(target_id);
                continue;
            }
            if let Some(target_id) = target_id.as_ref().filter(|id| {
                doc.parts
                    .iter()
                    .find(|part| &part.id == *id)
                    .and_then(|part| part.properties.as_ref())
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SUBSTITUTE))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
            }) {
                changed.push(target_id.clone());
                continue;
            }

            let target_id = match target_id {
                Some(id) => id,
                None => {
                    let base = format!("{target_layout_id}::mirror::{source_id}");
                    if doc.parts.iter().any(|part| part.id == base) {
                        let mut suffix = 1;
                        while doc
                            .parts
                            .iter()
                            .any(|part| part.id == format!("{base}::{suffix}"))
                        {
                            suffix += 1;
                        }
                        format!("{base}::{suffix}")
                    } else {
                        base
                    }
                }
            };
            let mut target_pose = crate::model::Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x - source.pose.at.x,
                    y: source.pose.at.y,
                },
                rotation: 180.0 - source.pose.rotation,
            };
            let target_exists = doc.parts.iter().any(|part| part.id == target_id);
            if let Some(target) = doc
                .parts
                .iter()
                .find(|part| part.id == target_id)
                .filter(|_| {
                    doc.constraint_driving_part(&target_id)
                        .ok()
                        .flatten()
                        .is_some()
                })
            {
                let source_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - target.pose.at.x,
                        y: target.pose.at.y,
                    },
                    rotation: 180.0 - target.pose.rotation,
                };
                set_component_pose(doc, &source_id, source_pose)?;
                source = doc
                    .parts
                    .iter()
                    .find(|part| part.id == source_id)
                    .cloned()
                    .ok_or("Mirrored component source is missing")?;
                changed.push(source_id.clone());
                target_pose = crate::model::Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - source.pose.at.x,
                        y: source.pose.at.y,
                    },
                    rotation: 180.0 - source.pose.rotation,
                };
            }

            let mut mirrored = source.clone();
            mirrored.id = target_id.clone();
            mirrored.reference = format!("{}_M", source.reference);
            mirrored.pose = target_pose;
            mirrored
                .properties
                .get_or_insert_with(BTreeMap::new)
                .insert(
                    MIRRORED_COMPONENT_SOURCE.into(),
                    serde_json::json!(source_id),
                );
            mirrored
                .properties
                .as_mut()
                .unwrap()
                .remove(MIRRORED_COMPONENT_SUBSTITUTE);
            if let Some(target) = doc.parts.iter_mut().find(|part| part.id == target_id) {
                *target = mirrored;
            } else {
                doc.parts.push(mirrored);
                if let Some(layout) = doc
                    .layouts
                    .iter_mut()
                    .find(|layout| layout.id == target_layout_id)
                {
                    layout.part_ids.push(target_id.clone());
                }
                let board_id = doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == target_layout_id)
                    .map(|layout| layout.board_id.clone())
                    .ok_or("Linked component target layout is missing")?;
                for board in &mut doc.boards {
                    if board.id == board_id && !board.part_ids.contains(&target_id) {
                        board.part_ids.push(target_id.clone());
                    }
                }
            }
            sync_component_nets(doc, &source_id, &target_id);
            for feature in &mut doc.outline {
                if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature
                    && part_ids.contains(&source_id)
                    && !part_ids.contains(&target_id)
                {
                    part_ids.push(target_id.clone());
                }
            }
            changed.extend([source_id, target_id]);
            if !target_exists {
                changed.push(target_layout_id.clone());
            }
        }
    }
    Ok(changed)
}

pub(crate) fn move_linked_component(
    doc: &mut ProjectDoc,
    id: &str,
    at: Vec2,
) -> Result<Option<Vec<String>>, String> {
    let Some(target) = doc.parts.iter().find(|part| part.id == id) else {
        return Ok(None);
    };
    let Some(properties) = target.properties.as_ref() else {
        return Ok(None);
    };
    if properties
        .get(MIRRORED_COMPONENT_SUBSTITUTE)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let Some(source_id) = properties
        .get(MIRRORED_COMPONENT_SOURCE)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        return Ok(None);
    };
    let target_id = target.id.clone();
    let Some(source_layout) = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.contains(&source_id))
    else {
        return Ok(None);
    };
    let Some(target_layout) = doc.layouts.iter().find(|layout| {
        layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| link.source_id == source_layout.id)
    }) else {
        return Ok(None);
    };
    let axis_x = target_layout
        .mirror_link
        .as_ref()
        .ok_or("Linked component target layout is missing")?
        .axis_x;
    let rotation = 180.0 - target.pose.rotation;
    set_component_pose(
        doc,
        &source_id,
        crate::model::Pose2 {
            at: Vec2 {
                x: 2.0 * axis_x - at.x,
                y: at.y,
            },
            rotation,
        },
    )?;
    Ok(Some(vec![source_id, target_id]))
}

fn adopt_unassigned_components(
    doc: &mut ProjectDoc,
    source_layout_id: &str,
    axis_x: f64,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let source_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == source_layout_id)
        .ok_or("Linked component source layout is missing")?;
    let board_id = source_layout.board_id.clone();
    let key_positions: Vec<_> = doc.matrices.iter()
        .filter(|matrix| source_layout.matrix_ids.contains(&matrix.id))
        .flat_map(matrix::cell_members).filter_map(|(_, id)| doc.parts.iter().find(|part| part.id == id))
        .map(|part| part.pose.at.x).collect();
    if key_positions.is_empty() { return Ok(()); }
    let source_center = key_positions.iter().sum::<f64>() / key_positions.len() as f64;
    if (source_center - axis_x).abs() <= f64::EPSILON {
        return Ok(());
    }
    let source_is_left = source_center < axis_x;

    let mut assigned: BTreeSet<_> = doc
        .layouts
        .iter()
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    for matrix in &doc.matrices {
        assigned.extend(matrix.part_ids.iter().cloned());
        assigned.extend(matrix::cell_members(matrix).into_values());
    }
    let board_parts = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Linked component board is missing")?
        .part_ids
        .clone();
    let unassigned: Vec<_> = board_parts
        .into_iter()
        // Case hardware is resolved in board coordinates, independently of layout mirrors.
        .filter(|id| !assigned.contains(id) && !id.starts_with("case-closure/"))
        .filter_map(|id| {
            let part = doc.parts.iter().find(|part| part.id == id)?;
            let on_source_side = if source_is_left {
                part.pose.at.x < axis_x
            } else {
                part.pose.at.x > axis_x
            };
            on_source_side.then_some(id)
        })
        .collect();
    if unassigned.is_empty() {
        return Ok(());
    }
    if let Some(layout) = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == source_layout_id)
    {
        for id in unassigned {
            if !layout.part_ids.contains(&id) {
                layout.part_ids.push(id.clone());
                changed.push(id);
            }
        }
    }
    Ok(())
}

fn set_component_pose(
    doc: &mut ProjectDoc,
    id: &str,
    pose: crate::model::Pose2,
) -> Result<(), String> {
    let current = doc
        .parts
        .iter()
        .find(|part| part.id == id)
        .ok_or("Mirrored component source is missing")?
        .pose;
    if let Some(constraint_id) = doc
        .constraint_driving_part(id)?
        .map(crate::model::Constraint::id)
        .map(str::to_owned)
    {
        let constraint = doc
            .constraints
            .iter_mut()
            .find(|constraint| constraint.id() == constraint_id)
            .ok_or_else(|| format!("Constraint {constraint_id} disappeared"))?;
        let delta = Vec2 {
            x: pose.at.x - current.at.x,
            y: pose.at.y - current.at.y,
        };
        let turn = Some(pose.rotation - current.rotation);
        if crate::constraints::retune_offset(&doc.parts, constraint, delta, turn)?
            == crate::constraints::Retune::Mirror
        {
            return Err(format!(
                "Cannot move mirrored component because source {id} is controlled by a mirror constraint"
            ));
        }
    }
    let part = doc
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .ok_or("Mirrored component source is missing")?;
    part.pose = pose;
    Ok(())
}

fn sync_component_nets(doc: &mut ProjectDoc, source_id: &str, target_id: &str) {
    for net in &mut doc.nets {
        let source_pins: Vec<_> = net
            .pins
            .iter()
            .filter(|pin| pin.part_id == source_id)
            .cloned()
            .collect();
        net.pins.retain(|pin| pin.part_id != target_id);
        net.pins.extend(source_pins.into_iter().map(|mut pin| {
            pin.part_id = target_id.to_string();
            pin
        }));
    }
}

fn clear_component_link(part: &mut Part) {
    if let Some(properties) = part.properties.as_mut() {
        properties.remove(MIRRORED_COMPONENT_SOURCE);
        properties.remove(MIRRORED_COMPONENT_SUBSTITUTE);
        if properties.is_empty() {
            part.properties = None;
        }
    }
}

pub(crate) fn unlink_components(doc: &mut ProjectDoc, layout_id: &str) -> Vec<String> {
    let Some((part_ids, matrix_ids)) = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .map(|layout| (layout.part_ids.clone(), layout.matrix_ids.clone()))
    else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for id in part_ids {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
            changed.push(id);
        }
    }
    for matrix in doc.matrices.iter_mut().filter(|matrix| matrix_ids.contains(&matrix.id)) {
        for cell in &mut matrix.cells {
            if cell.assemblies_local == Some(true) {
                cell.assemblies_local = None;
            }
        }
        changed.push(matrix.id.clone());
    }
    changed
}

fn remove_component(doc: &mut ProjectDoc, id: &str) {
    doc.parts.retain(|part| part.id != id);
    doc.constraints
        .retain(|constraint| constraint.source() != id && constraint.target() != id);
    for board in &mut doc.boards {
        board.part_ids.retain(|part_id| part_id != id);
    }
    for layout in &mut doc.layouts {
        layout.part_ids.retain(|part_id| part_id != id);
    }
    for matrix in &mut doc.matrices {
        matrix.part_ids.retain(|part_id| part_id != id);
    }
    for net in &mut doc.nets {
        net.pins.retain(|pin| pin.part_id != id);
    }
    for feature in &mut doc.outline {
        if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature {
            part_ids.retain(|part_id| part_id != id);
        }
    }
}

pub(crate) fn mirrored_component_ids(doc: &ProjectDoc, requested: &[String]) -> Vec<String> {
    let mut ids: BTreeSet<_> = requested.iter().cloned().collect();
    for target in &doc.parts {
        let Some(source_id) = target
            .properties
            .as_ref()
            .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if ids.contains(&target.id) || ids.contains(source_id) {
            ids.insert(target.id.clone());
            ids.insert(source_id.to_string());
        }
    }
    ids.into_iter().collect()
}

pub(crate) fn remove_matrix(doc: &mut ProjectDoc, matrix_id: &str) {
    let removed: BTreeSet<_> = doc
        .layouts
        .iter()
        .filter(|layout| layout.matrix_ids == [matrix_id])
        .map(|layout| layout.id.clone())
        .collect();
    let detached_parts: Vec<_> = doc
        .layouts
        .iter()
        .filter(|layout| {
            removed.contains(&layout.id)
                || layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| removed.contains(&link.source_id))
        })
        .flat_map(|layout| layout.part_ids.iter().cloned())
        .collect();
    doc.layouts.retain(|layout| !removed.contains(&layout.id));
    for layout in &mut doc.layouts { layout.matrix_ids.retain(|id| id != matrix_id); }
    for layout in &mut doc.layouts {
        if layout
            .mirror_link
            .as_ref()
            .is_some_and(|link| removed.contains(&link.source_id))
        {
            layout.mirror_link = None;
        }
    }
    for id in detached_parts {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            clear_component_link(part);
        }
    }
}

/// Whether `id` is a matrix key: the primary member of a cell, never a companion of one.
pub fn is_matrix_key(doc: &ProjectDoc, id: &str) -> bool {
    primary_member(doc, id).is_some()
}

fn primary_member<'a>(doc: &'a ProjectDoc, id: &str) -> Option<(&'a Matrix, u32, u32)> {
    for matrix in &doc.matrices {
        if !matrix.part_ids.iter().any(|part| part == id) {
            continue;
        }
        if let Some(((row, column), _)) = matrix::cell_members(matrix)
            .into_iter()
            .find(|(_, member)| member == id)
        {
            return Some((matrix, row, column));
        }
    }
    None
}

pub(crate) fn has_linked_positions(doc: &ProjectDoc, positions: &[Position]) -> bool {
    let mut affected: BTreeSet<_> = positions
        .iter()
        .map(|position| position.id.as_str())
        .collect();
    loop {
        let previous = affected.len();
        for id in affected.clone() {
            if let Some(constraint) = doc.constraint_driving_part(id).ok().flatten() {
                affected.insert(constraint.target());
            }
        }
        if affected.len() == previous {
            break;
        }
    }
    if affected.iter().any(|id| primary_member(doc, id).is_some()) {
        return true;
    }
    positions.iter().any(|position| {
        primary_member(doc, &position.id).is_some()
            || doc.parts.iter().any(|part| {
                part.id == position.id
                    && part.properties.as_ref().is_some_and(|properties| {
                        properties.contains_key(MIRRORED_COMPONENT_SOURCE)
                    })
            })
            || doc.parts.iter().any(|target| {
                target
                    .properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some(position.id.as_str())
            })
            || doc
                .constraint_driving_part(&position.id)
                .ok()
                .flatten()
                .is_some_and(|constraint| constraint.offset_axes().is_some())
    })
}

/// Numeric moves and keyboard nudges must update the same parametric geometry
/// as pointer edits; absolute part overrides would break the next linked edit.
pub(crate) fn move_keys(
    doc: &mut ProjectDoc,
    positions: &[Position],
) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let mut matrices = BTreeMap::new();
    let mut handled = BTreeSet::new();
    let mut carried = Vec::new();
    let mut constrained = Vec::new();
    for position in positions {
        let Some((source, row, column)) = primary_member(doc, &position.id) else {
            continue;
        };
        let part = doc
            .parts
            .iter()
            .find(|part| part.id == position.id)
            .ok_or("Key is missing")?;
        if part.locked == Some(true) {
            return Err(format!("Part {} is locked", part.id));
        }
        if !position.at.x.is_finite() || !position.at.y.is_finite() {
            return Err("Position must be finite".into());
        }
        if let Some(constraint) = doc.constraint_driving_part(&position.id)? {
            if constraint.is_mirror() {
                return Err(crate::constraints::mirror_drag_refusal(&position.id));
            }
            let mut pose = part.pose;
            pose.at = position.at;
            constrained.push((position.id.clone(), pose));
        }
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let displacement = Vec2 {
            x: position.at.x - part.pose.at.x,
            y: position.at.y - part.pose.at.y,
        };
        let cell = source
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column);
        if let Some(cell) = cell {
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&position.id, &assembly.id);
                if positions.iter().any(|position| position.id == id)
                    || doc.constraint_driving_part(&id).ok().flatten().is_some()
                {
                    continue;
                }
                if let Some(companion) = doc.parts.iter().find(|part| part.id == id)
                    && companion.properties.as_ref().is_some_and(|properties| {
                        properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
                    })
                {
                    let mut pose = companion.pose;
                    pose.at.x += displacement.x;
                    pose.at.y += displacement.y;
                    carried.push((id, pose));
                }
            }
        }
        // Older absolute key moves are absorbed into the cell's parametric offset.
        let evaluated = matrix::evaluated(source, doc)?;
        let generated = matrix::evaluation::cell_pose(&evaluated, row, column)?.at;
        let delta = Vec2 {
            x: position.at.x - generated.x,
            y: position.at.y - generated.y,
        };
        let frame = &evaluated.cells.iter().find(|c| c.row == row as usize && c.column == column as usize).ok_or("Key frame is missing")?.offset_frame;
        let local = matrix::evaluation::inverse_offset(frame, delta);
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,

                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        handled.insert(position.id.clone());
    }
    // Editing both sides at once would give two competing values for one link.
    if matrices
        .keys()
        .any(|id| partner(doc, id).is_some_and(|(other, _)| matrices.contains_key(&other)))
    {
        return Err("Move one linked half at a time".into());
    }
    let mut changed = vec![];
    for part in &mut doc.parts {
        if handled.contains(&part.id)
            && let Some(properties) = &mut part.properties
        {
            properties.remove(matrix::OVERRIDE);
        }
    }
    for (id, pose) in constrained.into_iter().chain(carried) {
        set_component_pose(doc, &id, pose)?;
        changed.push(id);
    }
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    Ok((changed, handled))
}

pub(crate) struct AssemblyPlacement {
    pub(crate) parent_id: String,
    pub(crate) local_pose: Pose2,
    pub(crate) authored_pose: Option<Pose2>,
    pub(crate) mode: MatrixCompanionMode,
}

/// Classify a generated companion the same way the solver places it: an explicit
/// constraint wins, then an authored pose, otherwise the companion follows its
/// key. The placement solver and the scene projection both read this mode, so the
/// precedence rule has one source.
pub(crate) fn companion_mode(doc: &ProjectDoc, companion_id: &str) -> MatrixCompanionMode {
    if let Some(constraint) = doc.constraint_driving_part(companion_id).ok().flatten() {
        return MatrixCompanionMode::Constrained {
            constraint_id: constraint.id().to_string(),
            source_part_id: constraint.source().to_string(),
        };
    }
    let authored = doc
        .parts
        .iter()
        .find(|part| part.id == companion_id)
        .is_some_and(|part| {
            part.properties.as_ref().is_some_and(|properties| {
                properties.get(matrix::OVERRIDE) == Some(&serde_json::json!(1))
            })
        });
    if authored {
        MatrixCompanionMode::Authored
    } else {
        MatrixCompanionMode::Follows
    }
}

pub(crate) fn assembly_placements(
    doc: &ProjectDoc,
    explicit: &[Position],
) -> BTreeMap<String, AssemblyPlacement> {
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let mut placements = BTreeMap::new();
    for matrix in &doc.matrices {
        for cell in &matrix.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            let parent_id = matrix::member_id(&matrix.id, cell.row, cell.column);
            let Some(parent) = parts.get(parent_id.as_str()) else {
                continue;
            };
            for assembly in &cell.assemblies {
                let id = matrix::companion_id(&parent_id, &assembly.id);
                if explicit.iter().any(|position| position.id == id) {
                    continue;
                }
                let Some(companion) = parts.get(id.as_str()) else {
                    continue;
                };
                let mode = companion_mode(doc, &id);
                let authored = matches!(mode, MatrixCompanionMode::Authored);
                let local_pose = if authored {
                    let (sin, cos) = parent.pose.rotation.to_radians().sin_cos();
                    let dx = companion.pose.at.x - parent.pose.at.x;
                    let dy = companion.pose.at.y - parent.pose.at.y;
                    Pose2 {
                        at: Vec2 {
                            x: dx * cos + dy * sin,
                            y: -dx * sin + dy * cos,
                        },
                        rotation: companion.pose.rotation - parent.pose.rotation,
                    }
                } else {
                    Pose2 {
                        at: assembly.offset,
                        rotation: assembly.rotation.unwrap_or(0.0),
                    }
                };
                placements.insert(
                    id,
                    AssemblyPlacement {
                        parent_id: parent_id.clone(),
                        local_pose,
                        authored_pose: authored.then_some(companion.pose),
                        mode,
                    },
                );
            }
        }
    }
    placements
}

pub(crate) fn reconcile_constraint_keys(
    doc: &mut ProjectDoc,
    moves: &BTreeMap<String, (Pose2, Pose2)>,
    solved: &BTreeMap<String, Pose2>,
) -> Result<Vec<String>, String> {
    let mut matrices = BTreeMap::new();
    for (id, (_, next)) in moves {
        let Some((source, row, column)) = primary_member(doc, id) else {
            continue;
        };
        let evaluated = matrix::evaluated(source, doc)?;
        let geometry = evaluated.cells.iter().find(|c| c.row == row as usize && c.column == column as usize).ok_or("Key frame is missing")?;
        let local = matrix::evaluation::inverse_offset(&geometry.offset_frame, Vec2 { x: next.at.x - geometry.pose.at.x, y: next.at.y - geometry.pose.at.y });
        let matrix = matrices
            .entry(source.id.clone())
            .or_insert_with(|| source.clone());
        let index = matrix
            .cells
            .iter()
            .position(|cell| cell.row == row && cell.column == column)
            .unwrap_or_else(|| {
                matrix.cells.push(MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                });
                matrix.cells.len() - 1
            });
        let cell = &mut matrix.cells[index];
        let offset = cell.offset.unwrap_or_default();
        cell.offset = Some(Vec2 {
            x: offset.x + local.x,
            y: offset.y + local.y,
        });
        let sign = if matches!(source.mirror, Some(Mirror::X | Mirror::Y)) { -1.0 } else { 1.0 };
        cell.rotation = Some(cell.rotation.unwrap_or_default() + sign * (next.rotation - geometry.pose.rotation));
    }
    let mut changed = Vec::new();
    for matrix in matrices.values() {
        changed.extend(matrix::set_matrix(doc, matrix)?);
        changed.extend(sync(doc, &matrix.id)?);
    }
    // The combined dependency solver owns final poses, including independently
    // constrained and explicitly positioned companions overwritten by generation.
    for part in &mut doc.parts {
        if let Some(pose) = solved.get(&part.id) {
            part.pose = *pose;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod case_hardware_tests {
    use super::*;

    #[test]
    fn case_clearance_holes_are_not_adopted_by_linked_layouts() {
        let mut doc = ProjectDoc::empty("case", "Case");
        doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["case-closure/board/10/10"],"netIds":[],"thickness":1.6}])).unwrap();
        doc.parts = serde_json::from_value(serde_json::json!([{"id":"case-closure/board/10/10","definitionId":"hole","reference":"MH1","pose":{"at":{"x":10,"y":10},"rotation":0},"side":"front"}])).unwrap();
        doc.matrices = serde_json::from_value(serde_json::json!([{"id":"matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}])).unwrap();
        doc.layouts = serde_json::from_value(serde_json::json!([{"id":"left","name":"Left","boardId":"board","matrixIds":["matrix"],"partIds":[]}])).unwrap();
        adopt_unassigned_components(&mut doc, "left", 100.0, &mut vec![]).unwrap();
        assert!(doc.layouts[0].part_ids.is_empty());
    }

    fn matrix_with_companion() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "source", "rows": 1, "columns": 1, "pitch": {"x": 19, "y": 19},
            "origin": {"x": 10, "y": 5}, "rotation": 20, "definitionId": "key", "partIds": [],
            "cells": [{"row": 0, "column": 0, "enabled": true, "rotation": 15,
                "assemblies": [{"id": "diode", "definitionId": "d", "offset": {"x": 3, "y": 4}, "rotation": 30}]}]
        }))
        .unwrap()
    }

    fn empty_target() -> Matrix {
        let mut target = matrix_with_companion();
        target.id = "target".into();
        target.cells.clear();
        target
    }

    #[test]
    fn a_vertical_axis_reflects_the_origin_turns_and_companions() {
        let reflected = reflected(
            &matrix_with_companion(),
            &empty_target(),
            axis_mapping(50.0),
            true,
        )
        .unwrap();
        assert_eq!(reflected.origin, Vec2 { x: 90.0, y: 5.0 });
        assert_eq!(reflected.rotation, Some(-20.0));
        assert_eq!(reflected.mirror, Some(Mirror::X));
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }

    #[test]
    fn a_rotated_map_turns_the_matrix_heading_once_and_leaves_relative_values_alone() {
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 10.0 },
            rotation: 40.0,
        };
        let reflected =
            reflected(&matrix_with_companion(), &empty_target(), mapping, true).unwrap();
        // The origin goes through the map and the heading is `rotation - angle`.
        let expected = pair::forward(mapping, Vec2 { x: 10.0, y: 5.0 });
        assert_eq!(reflected.origin, expected);
        assert_eq!(reflected.rotation, Some(20.0));
        // The cell turn and the companion are relative to that frame and to the part, which
        // already carry the pair rotation: only their handedness flips.
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }
}
