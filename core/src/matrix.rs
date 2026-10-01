use crate::model::{
    Matrix, MatrixCell, MatrixColumnBasis, MatrixScene, MatrixSceneCell, Mirror, OutlineFeature,
    Part, Pin, Pose2, ProjectDoc, Side, Vec2,
};
use serde_json::json;
pub(crate) mod layout;
pub(crate) mod splay;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_MATRIX_PARTS: u32 = 4096;
const OVERRIDE: &str = "layoutOverride";
const DEFAULT_EDGE_GAP_MM: f64 = 1.0;
const SWITCH_ROW_PAD: &str = "one";
const SWITCH_COLUMN_PAD: &str = "two";

fn matrix_terminal_pad_ids(definition: &crate::model::PartDefinition, row: bool) -> Vec<String> {
    let ids = if let Some(terminals) = &definition.matrix_terminals {
        let name = if row {
            &terminals.row
        } else {
            &terminals.column
        };
        definition.terminals.get(name).cloned().unwrap_or_default()
    } else {
        let default_pad = if row {
            SWITCH_ROW_PAD
        } else {
            SWITCH_COLUMN_PAD
        };
        if definition.pads.iter().any(|pad| pad.id == default_pad) {
            vec![default_pad.into()]
        } else {
            vec![]
        }
    };
    ids.into_iter()
        .filter(|id| definition.pads.iter().any(|pad| &pad.id == id))
        .collect()
}

fn member_id(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn cell_members(matrix: &Matrix) -> BTreeMap<(u32, u32), String> {
    let cells: BTreeMap<_, _> = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut members = BTreeMap::new();
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            if cells.get(&(row, column)).is_some_and(|cell| !cell.enabled) {
                continue;
            }
            let id = member_id(&matrix.id, row, column);
            members.insert((row, column), id);
        }
    }
    members
}

// Record deletions in the parametric source before cleaning up generated parts.
pub(crate) fn removed_members(doc: &mut ProjectDoc, requested: &[String]) -> Vec<String> {
    let mut removed: BTreeSet<String> = requested.iter().cloned().collect();
    for matrix in &mut doc.matrices {
        let members = cell_members(matrix);
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                let id = members
                    .get(&(row, column))
                    .cloned()
                    .unwrap_or_else(|| member_id(&matrix.id, row, column));
                let prefix = format!("{id}/");
                if !requested
                    .iter()
                    .any(|part| part == &id || part.starts_with(&prefix))
                {
                    continue;
                }
                let index = matrix
                    .cells
                    .iter()
                    .position(|cell| cell.row == row && cell.column == column)
                    .unwrap_or_else(|| {
                        matrix.cells.push(MatrixCell {
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
                if requested.contains(&id) {
                    cell.enabled = false;
                    removed.extend(
                        matrix
                            .part_ids
                            .iter()
                            .filter(|part| part.starts_with(&prefix))
                            .cloned(),
                    );
                } else {
                    cell.assemblies
                        .retain(|assembly| !requested.contains(&format!("{id}/{}", assembly.id)));
                }
            }
        }
    }
    removed.into_iter().collect()
}

pub(crate) fn valid_matrix(matrix: &Matrix, doc: &ProjectDoc) -> Result<(), String> {
    if matrix.id.is_empty()
        || !matrix
            .id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("Matrix ID must use letters, digits, _ or -".into());
    }
    if matrix.rows == 0
        || matrix.columns == 0
        || matrix
            .rows
            .checked_mul(matrix.columns)
            .is_none_or(|count| count > MAX_MATRIX_PARTS)
    {
        return Err("Matrix dimensions are invalid or exceed the part limit".into());
    }
    if !matrix.pitch.x.is_finite()
        || !matrix.pitch.y.is_finite()
        || matrix.pitch.x <= 0.0
        || matrix.pitch.y <= 0.0
        || !matrix.origin.x.is_finite()
        || !matrix.origin.y.is_finite()
        || matrix.rotation.is_some_and(|angle| !angle.is_finite())
    {
        return Err("Matrix pitch, origin or rotation is invalid".into());
    }
    if !doc
        .definitions
        .iter()
        .any(|def| def.id == matrix.definition_id)
    {
        return Err(format!("Unknown definition {}", matrix.definition_id));
    }
    if matrix.cells.len() > (matrix.rows * matrix.columns) as usize {
        return Err("Matrix cells exceed dimensions".into());
    }
    if matrix.row_offsets.len() > matrix.rows as usize
        || matrix.column_offsets.len() > matrix.columns as usize
        || matrix.column_staggers.len() > matrix.columns as usize
        || matrix.column_splays.len() > matrix.columns as usize
        || matrix.column_origins.len() > matrix.columns as usize
    {
        return Err("Matrix offsets exceed dimensions".into());
    }
    let gap = matrix.edge_gap.unwrap_or(Vec2 {
        x: DEFAULT_EDGE_GAP_MM,
        y: DEFAULT_EDGE_GAP_MM,
    });
    if !gap.x.is_finite()
        || !gap.y.is_finite()
        || gap.x < 0.0
        || gap.y < 0.0
        || gap.x >= matrix.pitch.x
        || gap.y >= matrix.pitch.y
    {
        return Err("Matrix edge gap must be nonnegative and smaller than pitch".into());
    }
    let finite = |point: &Vec2| point.x.is_finite() && point.y.is_finite();
    if matrix
        .row_offsets
        .iter()
        .chain(&matrix.column_offsets)
        .chain(matrix.column_origins.iter().flatten())
        .any(|point| !finite(point))
    {
        return Err("Matrix offsets must be finite".into());
    }
    if matrix
        .column_staggers
        .iter()
        .chain(&matrix.column_splays)
        .any(|value| !value.is_finite())
    {
        return Err("Matrix stagger and splay must be finite".into());
    }
    let mut cells = BTreeSet::new();
    for cell in &matrix.cells {
        if cell.row >= matrix.rows
            || cell.column >= matrix.columns
            || !cells.insert((cell.row, cell.column))
        {
            return Err("Matrix cell coordinate is invalid or duplicated".into());
        }
        if cell.offset.as_ref().is_some_and(|point| !finite(point))
            || cell.rotation.is_some_and(|rotation| !rotation.is_finite())
        {
            return Err("Matrix cell transform is invalid".into());
        }
        if cell
            .definition_id
            .as_ref()
            .is_some_and(|id| !doc.definitions.iter().any(|def| &def.id == id))
        {
            return Err("Matrix cell definition is missing".into());
        }
        let mut assemblies = BTreeSet::new();
        for assembly in &cell.assemblies {
            if !assembly
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                || assembly.id.is_empty()
                || !assemblies.insert(&assembly.id)
                || !finite(&assembly.offset)
                || assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
                || !doc
                    .definitions
                    .iter()
                    .any(|def| def.id == assembly.definition_id)
            {
                return Err("Matrix assembly is invalid".into());
            }
        }
    }
    Ok(())
}

fn location(matrix: &Matrix, row: u32, column: u32, cell: Option<&MatrixCell>) -> Vec2 {
    let row_offset = matrix
        .row_offsets
        .get(row as usize)
        .copied()
        .unwrap_or_default();
    let column_offset = matrix
        .column_offsets
        .get(column as usize)
        .copied()
        .unwrap_or_default();
    let cell_offset = cell.and_then(|cell| cell.offset).unwrap_or_default();
    let mut x = column as f64 * matrix.pitch.x + row_offset.x + column_offset.x + cell_offset.x;
    let mut y = row as f64 * matrix.pitch.y + row_offset.y + column_offset.y + cell_offset.y;
    y += matrix
        .column_staggers
        .iter()
        .take(column as usize + 1)
        .sum::<f64>();
    // Earlier splays transform the pivots of every later column.
    for index in (0..(column as usize + 1).min(matrix.column_splays.len())).rev() {
        let angle = matrix
            .column_splays
            .get(index)
            .copied()
            .unwrap_or(0.0)
            .to_radians();
        if angle == 0.0 {
            continue;
        }
        let pivot = matrix
            .column_origins
            .get(index)
            .copied()
            .flatten()
            .unwrap_or(Vec2 {
                x: index as f64 * matrix.pitch.x,
                y: matrix.column_staggers.iter().take(index + 1).sum::<f64>(),
            });
        let pivot_x = pivot.x;
        let pivot_y = pivot.y;
        let (sin, cos) = angle.sin_cos();
        let dx = x - pivot_x;
        let dy = y - pivot_y;
        x = pivot_x + dx * cos - dy * sin;
        y = pivot_y + dx * sin + dy * cos;
    }
    match matrix.mirror.unwrap_or(Mirror::None) {
        Mirror::X => x = -x,
        Mirror::Y => y = -y,
        Mirror::None => {}
    }
    let angle = matrix.rotation.unwrap_or(0.0).to_radians();
    let (sin, cos) = angle.sin_cos();
    Vec2 {
        x: matrix.origin.x + x * cos - y * sin,
        y: matrix.origin.y + x * sin + y * cos,
    }
}

fn column_rotation(matrix: &Matrix, column: u32) -> f64 {
    let angle = matrix
        .column_splays
        .iter()
        .take(column as usize + 1)
        .sum::<f64>();
    if matches!(matrix.mirror, Some(Mirror::X | Mirror::Y)) {
        -angle
    } else {
        angle
    }
}

fn column_basis(matrix: &Matrix, column: u32) -> MatrixColumnBasis {
    let angle = matrix
        .column_splays
        .iter()
        .take(column as usize + 1)
        .sum::<f64>()
        .to_radians();
    let (sin, cos) = angle.sin_cos();
    let reflect = |v: Vec2| match matrix.mirror {
        Some(Mirror::X) => Vec2 { x: -v.x, y: v.y },
        Some(Mirror::Y) => Vec2 { x: v.x, y: -v.y },
        None | Some(Mirror::None) => v,
    };
    let rotate = |v: Vec2| {
        let matrix_angle = matrix.rotation.unwrap_or(0.0).to_radians();
        let (matrix_sin, matrix_cos) = matrix_angle.sin_cos();
        Vec2 {
            x: v.x * matrix_cos - v.y * matrix_sin,
            y: v.x * matrix_sin + v.y * matrix_cos,
        }
    };
    let axis_x = rotate(reflect(Vec2 { x: cos, y: sin }));
    let axis_y = rotate(reflect(Vec2 { x: -sin, y: cos }));
    MatrixColumnBasis {
        column,
        splay_origin: splay::origin_world(matrix, column),
        splay_angle: matrix
            .column_splays
            .get(column as usize)
            .copied()
            .unwrap_or(0.0),
        custom_origin: matrix
            .column_origins
            .get(column as usize)
            .is_some_and(Option::is_some),
        axis_x,
        axis_y,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectionMode {
    Actual,
    Draft,
}

pub(crate) fn valid_projection_matrix(matrix: &Matrix) -> Result<(), String> {
    if matrix.id.is_empty() || matrix.rows == 0 || matrix.columns == 0 {
        return Err("Matrix dimensions are invalid".into());
    }
    if matrix
        .rows
        .checked_mul(matrix.columns)
        .is_none_or(|n| n > MAX_MATRIX_PARTS)
    {
        return Err("Matrix dimensions exceed the part limit".into());
    }
    if matrix.cells.len() > (matrix.rows * matrix.columns) as usize {
        return Err("Matrix cells exceed dimensions".into());
    }
    if matrix.row_offsets.len() > matrix.rows as usize
        || matrix.column_offsets.len() > matrix.columns as usize
        || matrix.column_staggers.len() > matrix.columns as usize
        || matrix.column_splays.len() > matrix.columns as usize
        || matrix.column_origins.len() > matrix.columns as usize
    {
        return Err("Matrix offsets exceed dimensions".into());
    }
    if !matrix.pitch.x.is_finite()
        || !matrix.pitch.y.is_finite()
        || matrix.pitch.x <= 0.0
        || matrix.pitch.y <= 0.0
        || !matrix.origin.x.is_finite()
        || !matrix.origin.y.is_finite()
        || matrix.rotation.is_some_and(|value| !value.is_finite())
        || matrix
            .row_offsets
            .iter()
            .chain(&matrix.column_offsets)
            .chain(matrix.column_origins.iter().flatten())
            .any(|v| !v.x.is_finite() || !v.y.is_finite())
        || matrix
            .column_staggers
            .iter()
            .chain(&matrix.column_splays)
            .any(|v| !v.is_finite())
        || matrix.cells.iter().any(|cell| {
            cell.row >= matrix.rows
                || cell.column >= matrix.columns
                || cell
                    .offset
                    .is_some_and(|v| !v.x.is_finite() || !v.y.is_finite())
                || cell.rotation.is_some_and(|value| !value.is_finite())
        })
    {
        return Err("Matrix contains non-finite geometry".into());
    }
    let mut coordinates = BTreeSet::new();
    if matrix
        .cells
        .iter()
        .any(|cell| !coordinates.insert((cell.row, cell.column)))
    {
        return Err("Matrix cell coordinate is invalid or duplicated".into());
    }
    Ok(())
}

pub(crate) fn project_matrix(
    matrix: &Matrix,
    parts: &[Part],
    mode: ProjectionMode,
) -> Result<MatrixScene, String> {
    valid_projection_matrix(matrix)?;
    let members = if mode == ProjectionMode::Actual {
        cell_members(matrix)
    } else {
        BTreeMap::new()
    };
    let by_id: BTreeMap<_, _> = if mode == ProjectionMode::Actual {
        parts.iter().map(|part| (part.id.as_str(), part)).collect()
    } else {
        BTreeMap::new()
    };
    let cells: BTreeMap<_, _> = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut projected = Vec::with_capacity((matrix.rows * matrix.columns) as usize);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            let cell = cells.get(&(row, column)).copied();
            let enabled = cell.is_none_or(|item| item.enabled);
            let member_id = if enabled {
                members.get(&(row, column)).cloned()
            } else {
                None
            };
            let mut pose = Pose2 {
                at: location(matrix, row, column, cell),
                rotation: matrix.rotation.unwrap_or(0.0)
                    + column_rotation(matrix, column)
                    + cell.and_then(|item| item.rotation).unwrap_or(0.0),
            };
            if mode == ProjectionMode::Actual
                && let Some(id) = member_id.as_deref().and_then(|id| by_id.get(id))
            {
                pose = id.pose;
            }
            if !pose.at.x.is_finite() || !pose.at.y.is_finite() || !pose.rotation.is_finite() {
                return Err("Matrix projection exceeds finite geometry".into());
            }
            projected.push(MatrixSceneCell {
                row,
                column,
                enabled,
                member_id: if mode == ProjectionMode::Draft {
                    None
                } else {
                    member_id
                },
                pose,
            });
        }
    }
    Ok(MatrixScene {
        matrix_id: matrix.id.clone(),
        cells: projected,
        columns: (0..matrix.columns)
            .map(|column| column_basis(matrix, column))
            .collect(),
    })
}

pub fn mark_override(part: &mut Part) {
    part.properties
        .get_or_insert_with(BTreeMap::new)
        .insert(OVERRIDE.into(), json!(1));
}

// A matrix definition replacement keeps electrical identities by logical terminal,
// rather than carrying old physical pad IDs into the new footprint.
fn remap_member_pins(
    doc: &mut ProjectDoc,
    index: usize,
    definition_id: &str,
) -> Result<Vec<String>, String> {
    let part = &doc.parts[index];
    if part.definition_id == definition_id
        || !doc
            .nets
            .iter()
            .any(|net| net.pins.iter().any(|pin| pin.part_id == part.id))
    {
        return Ok(vec![]);
    }
    let before = doc
        .definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .ok_or("Previous matrix definition is missing")?;
    let after = doc
        .definitions
        .iter()
        .find(|d| d.id == definition_id)
        .ok_or("Replacement matrix definition is missing")?;
    let old_row = matrix_terminal_pad_ids(before, true);
    let old_column = matrix_terminal_pad_ids(before, false);
    let mut mapping = BTreeMap::new();
    for pad in &before.pads {
        let mut targets = if old_row.contains(&pad.id) {
            matrix_terminal_pad_ids(after, true)
        } else if old_column.contains(&pad.id) {
            matrix_terminal_pad_ids(after, false)
        } else {
            vec![]
        };
        if targets.is_empty() {
            for (name, pads) in &before.terminals {
                if pads.contains(&pad.id) {
                    targets.extend(after.terminals.get(name).into_iter().flatten().cloned());
                }
            }
        }
        if targets.is_empty() && !pad.number.is_empty() {
            targets.extend(
                after
                    .pads
                    .iter()
                    .filter(|new| new.number == pad.number)
                    .map(|new| new.id.clone()),
            );
        }
        if targets.is_empty() && after.pads.iter().any(|new| new.id == pad.id) {
            targets.push(pad.id.clone());
        }
        targets.sort();
        targets.dedup();
        mapping.insert(pad.id.clone(), targets);
    }
    let part_id = part.id.clone();
    let mut assigned = BTreeMap::new();
    let mut changed = vec![];
    for net in &mut doc.nets {
        if !net.pins.iter().any(|pin| pin.part_id == part_id) {
            continue;
        }
        let mut pins = vec![];
        for pin in &net.pins {
            if pin.part_id != part_id {
                pins.push(pin.clone());
                continue;
            }
            let targets = mapping
                .get(&pin.pad_id)
                .filter(|targets| !targets.is_empty())
                .ok_or_else(|| {
                    format!(
                        "Cannot map connected pad {}/{} to the replacement footprint",
                        part_id, pin.pad_id
                    )
                })?;
            for target in targets {
                if assigned
                    .insert(target.clone(), net.id.clone())
                    .is_some_and(|previous| previous != net.id)
                {
                    return Err(format!(
                        "Replacement would merge different nets on pad {part_id}/{target}"
                    ));
                }
                let mapped = Pin {
                    part_id: part_id.clone(),
                    pad_id: target.clone(),
                };
                if !pins.contains(&mapped) {
                    pins.push(mapped);
                }
            }
        }
        if pins != net.pins {
            net.pins = pins;
            changed.push(net.id.clone());
        }
    }
    Ok(changed)
}

pub fn set_matrix(doc: &mut ProjectDoc, incoming: &Matrix) -> Result<Vec<String>, String> {
    let previous = doc
        .matrices
        .iter()
        .find(|item| item.id == incoming.id)
        .cloned();
    // Only a shrinking axis removes edits outside its former bounds.
    let mut resized = incoming.clone();
    if let Some(before) = &previous {
        if resized.rows < before.rows {
            if resized.row_offsets.len() <= before.rows as usize {
                resized.row_offsets.truncate(resized.rows as usize);
            }
            resized
                .cells
                .retain(|cell| cell.row < resized.rows || cell.row >= before.rows);
        }
        if resized.columns < before.columns {
            if resized.column_staggers.len() <= before.columns as usize {
                resized.column_staggers.truncate(resized.columns as usize);
            }
            if resized.column_splays.len() <= before.columns as usize {
                resized.column_splays.truncate(resized.columns as usize);
            }
            if resized.column_origins.len() <= before.columns as usize {
                resized.column_origins.truncate(resized.columns as usize);
            }
            if resized.column_offsets.len() <= before.columns as usize {
                resized.column_offsets.truncate(resized.columns as usize);
            }
            resized
                .cells
                .retain(|cell| cell.column < resized.columns || cell.column >= before.columns);
        }
    }
    let incoming = &resized;
    valid_matrix(incoming, doc)?;
    let old_ids: BTreeSet<_> = previous
        .as_ref()
        .map(|item| item.part_ids.iter().cloned().collect())
        .unwrap_or_default();
    let target_id = incoming.board_id.as_ref().or_else(|| {
        previous
            .as_ref()
            .and_then(|matrix| matrix.board_id.as_ref())
    });
    if let Some(id) = target_id
        && !doc.boards.iter().any(|board| &board.id == id)
    {
        return Err(format!("Unknown board {id}"));
    }
    let board_index = target_id
        .and_then(|id| doc.boards.iter().position(|board| &board.id == id))
        .or_else(|| {
            doc.boards
                .iter()
                .position(|board| board.part_ids.iter().any(|id| old_ids.contains(id)))
        })
        .unwrap_or(0);
    let board_ids = doc
        .boards
        .get(board_index)
        .map(|board| board.outline_ids.clone())
        .unwrap_or_default();
    let mut next = incoming.clone();
    next.edge_gap = Some(incoming.edge_gap.unwrap_or(Vec2 {
        x: DEFAULT_EDGE_GAP_MM,
        y: DEFAULT_EDGE_GAP_MM,
    }));
    next.board_id = doc.boards.get(board_index).map(|board| board.id.clone());
    next.part_ids.clear();
    let mut changed = vec![incoming.id.clone()];
    let cells: BTreeMap<_, _> = incoming
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut part_index: BTreeMap<String, usize> = doc
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| (part.id.clone(), index))
        .collect();
    let mut references: BTreeSet<String> = doc
        .parts
        .iter()
        .map(|part| part.reference.clone())
        .collect();
    let mut next_reference: BTreeMap<&str, usize> = BTreeMap::new();
    for row in 0..incoming.rows {
        for column in 0..incoming.columns {
            let cell = cells.get(&(row, column)).copied();
            if cell.is_some_and(|cell| !cell.enabled) {
                continue;
            }
            let id = member_id(&incoming.id, row, column);
            let pose = Pose2 {
                at: location(incoming, row, column, cell),
                rotation: incoming.rotation.unwrap_or(0.0)
                    + column_rotation(incoming, column)
                    + cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
            };
            let mut members = vec![(
                id,
                incoming.definition_id.clone(),
                pose,
                Side::Front,
                cell.and_then(|cell| cell.variant.as_ref()),
            )];
            if let Some(cell) = cell {
                members[0].1 = cell
                    .definition_id
                    .as_ref()
                    .unwrap_or(&incoming.definition_id)
                    .clone();
                for assembly in &cell.assemblies {
                    let id = format!("{}/{}", members[0].0, assembly.id);
                    let rotation = pose.rotation.to_radians();
                    let (sin, cos) = rotation.sin_cos();
                    let at = Vec2 {
                        x: pose.at.x + assembly.offset.x * cos - assembly.offset.y * sin,
                        y: pose.at.y + assembly.offset.x * sin + assembly.offset.y * cos,
                    };
                    members.push((
                        id,
                        assembly.definition_id.clone(),
                        Pose2 {
                            at,
                            rotation: pose.rotation + assembly.rotation.unwrap_or(0.0),
                        },
                        assembly.side.clone().unwrap_or(Side::Front),
                        None,
                    ));
                }
            }
            for (id, definition_id, pose, side, variant) in members {
                if !old_ids.contains(&id) && part_index.contains_key(&id) {
                    return Err(format!("Matrix member ID {id} is already owned"));
                }
                if let Some(&index) = part_index.get(&id) {
                    changed.extend(remap_member_pins(doc, index, &definition_id)?);
                    let previous = doc
                        .definitions
                        .iter()
                        .find(|definition| definition.id == doc.parts[index].definition_id);
                    let authored_keycap = previous
                        .filter(|definition| {
                            definition.envelope_source.as_ref().is_some_and(|source| {
                                source.keycap == Some(crate::model::EnvelopeOrigin::Authored)
                            })
                        })
                        .and_then(|definition| definition.keycap);
                    let part = &mut doc.parts[index];
                    if part.definition_id != definition_id && part.keycap.is_none() {
                        part.keycap = authored_keycap;
                    }
                    let overridden = part
                        .properties
                        .as_ref()
                        .is_some_and(|props| props.get(OVERRIDE) == Some(&json!(1)));
                    if !overridden {
                        part.pose = pose;
                    }
                    part.definition_id = definition_id;
                    part.side = side;
                    if let Some(variant) = variant {
                        part.properties
                            .get_or_insert_with(BTreeMap::new)
                            .insert("variant".into(), json!(variant));
                    } else if let Some(properties) = &mut part.properties {
                        properties.remove("variant");
                    }
                } else {
                    let prefix = "SW";
                    let number = next_reference.entry(prefix).or_insert(1);
                    let reference = loop {
                        let candidate = format!("{prefix}{number}");
                        *number += 1;
                        if references.insert(candidate.clone()) {
                            break candidate;
                        }
                    };
                    let mut properties = BTreeMap::new();
                    if let Some(variant) = variant {
                        properties.insert("variant".into(), json!(variant));
                    }
                    part_index.insert(id.clone(), doc.parts.len());
                    doc.parts.push(Part {
                        id: id.clone(),
                        definition_id,
                        reference,
                        pose,
                        side,
                        locked: None,
                        keycap: None,
                        outline: None,
                        properties: (!properties.is_empty()).then_some(properties),
                        generator_parameters: None,
                    });
                }
                next.part_ids.push(id.clone());
                changed.push(id);
            }
        }
    }
    let new_ids: BTreeSet<_> = next.part_ids.iter().cloned().collect();
    let removed: Vec<_> = old_ids.difference(&new_ids).cloned().collect();
    let removed_set: BTreeSet<_> = removed.iter().cloned().collect();
    doc.parts.retain(|part| !removed_set.contains(&part.id));
    for board in &mut doc.boards {
        board.part_ids.retain(|id| !removed_set.contains(id));
        if board.id != next.board_id.as_deref().unwrap_or("") {
            board.part_ids.retain(|id| !old_ids.contains(id));
        }
    }
    for net in &mut doc.nets {
        net.pins.retain(|pin| !removed_set.contains(&pin.part_id));
    }
    doc.constraints.retain(|constraint| {
        !removed_set.contains(constraint.source()) && !removed_set.contains(constraint.target())
    });
    for feature in &mut doc.outline {
        if let OutlineFeature::PartEnvelope { part_ids, .. } = feature {
            part_ids.retain(|id| !removed_set.contains(id));
        }
    }
    if let Some(board) = doc.boards.get_mut(board_index) {
        let mut present: BTreeSet<_> = board.part_ids.iter().cloned().collect();
        for id in &next.part_ids {
            if present.insert(id.clone()) {
                board.part_ids.push(id.clone());
            }
        }
    }
    for feature in &mut doc.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature {
            if !board_ids.contains(id) {
                part_ids.retain(|member| !old_ids.contains(member));
            }
            if board_ids.contains(id) || part_ids.iter().any(|id| old_ids.contains(id)) {
                let mut present: BTreeSet<_> = part_ids.iter().cloned().collect();
                for member in &next.part_ids {
                    if present.insert(member.clone()) {
                        part_ids.push(member.clone());
                    }
                }
            }
        }
    }
    if doc.boards.is_empty()
        && let Some(OutlineFeature::PartEnvelope { part_ids, .. }) = doc
            .outline
            .iter_mut()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
    {
        let mut present: BTreeSet<_> = part_ids.iter().cloned().collect();
        for member in &next.part_ids {
            if present.insert(member.clone()) {
                part_ids.push(member.clone());
            }
        }
    }
    if let Some(current) = doc.matrices.iter_mut().find(|item| item.id == incoming.id) {
        *current = next;
    } else {
        doc.matrices.push(next);
    }
    changed.extend(removed);
    Ok(changed)
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn matrix() -> Matrix {
        Matrix {
            id: "m".into(),
            name: None,
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2 { x: 0.0, y: 0.0 },
            definition_id: "missing".into(),
            part_ids: vec![
                "matrix/m/r0c0".into(),
                "matrix/m/r0c1".into(),
                "matrix/m/r0c2".into(),
            ],
            board_id: None,
            mirror: Some(Mirror::X),
            rotation: Some(10.0),
            edge_gap: None,

            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![0.0, 3.0],
            column_splays: vec![0.0, 15.0],
            column_origins: vec![],
            cells: vec![MatrixCell {
                row: 1,
                column: 1,
                enabled: false,

                definition_id: None,
                variant: None,
                offset: Some(Vec2 { x: 2.0, y: -1.0 }),
                rotation: Some(7.0),
                assemblies: vec![],
                assemblies_local: None,
            }],
        }
    }

    fn part(id: &str, x: f64, y: f64) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: "missing".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y },
                rotation: 33.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    #[test]
    fn draft_is_geometry_only_and_contains_row_major_disabled_cells() {
        let scene = project_matrix(&matrix(), &[], ProjectionMode::Draft).unwrap();
        assert_eq!(scene.cells.len(), 6);
        assert!(scene.cells.iter().all(|cell| cell.member_id.is_none()));
        assert!(!scene.cells[4].enabled);
        assert_eq!(scene.cells[0].row, 0);
        assert_eq!(scene.cells[5].column, 2);
    }

    #[test]
    fn actual_projection_uses_saved_poses_and_parametric_empty_cells() {
        let m = matrix();
        let parts = vec![
            part("matrix/m/r0c0", 100.0, 10.0),
            part("matrix/m/r0c2", 100.0, 30.0),
            part("matrix/m/r0c1", 55.0, 66.0),
        ];
        let scene = project_matrix(&m, &parts, ProjectionMode::Actual).unwrap();
        assert_eq!(scene.cells[1].pose, parts[2].pose);
        assert_eq!(scene.cells[0].member_id.as_deref(), Some("matrix/m/r0c0"));
        assert_eq!(scene.cells[2].member_id.as_deref(), Some("matrix/m/r0c2"));
        assert!(scene.cells[4].member_id.is_none());
        assert!(scene.cells[4].pose.at.x.is_finite());
    }

    #[test]
    fn projection_enforces_geometry_validation_at_its_boundary() {
        let mut m = matrix();
        m.cells.push(m.cells[0].clone());
        assert!(project_matrix(&m, &[], ProjectionMode::Draft).is_err());
        m.cells.clear();
        m.pitch.x = f64::NAN;
        assert!(project_matrix(&m, &[], ProjectionMode::Actual).is_err());
    }

    #[test]
    fn projections_match_captured_typescript_behavior() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/matrix-projections.json"))
                .unwrap();
        for fixture in fixtures["cases"].as_array().unwrap() {
            let matrix = serde_json::from_value(fixture["matrix"].clone()).unwrap();
            let parts: Vec<Part> = serde_json::from_value(fixture["parts"].clone()).unwrap();
            let expected: Vec<MatrixSceneCell> =
                serde_json::from_value(fixture["expected"].clone()).unwrap();
            let scene = project_matrix(&matrix, &parts, ProjectionMode::Actual).unwrap();
            assert_eq!(scene.cells.len(), expected.len());
            for (actual, expected) in scene.cells.iter().zip(expected) {
                assert_eq!(
                    (actual.row, actual.column, actual.enabled, &actual.member_id),
                    (
                        expected.row,
                        expected.column,
                        expected.enabled,
                        &expected.member_id
                    )
                );
                assert!(
                    (actual.pose.at.x - expected.pose.at.x).abs() < 1e-9,
                    "{} x",
                    fixture["name"]
                );
                assert!(
                    (actual.pose.at.y - expected.pose.at.y).abs() < 1e-9,
                    "{} y",
                    fixture["name"]
                );
                assert!(
                    (actual.pose.rotation - expected.pose.rotation).abs() < 1e-9,
                    "{} rotation",
                    fixture["name"]
                );
            }
        }
    }

    #[test]
    fn projection_rejects_invalid_draft_geometry() {
        let mut m = matrix();
        m.rows = 0;
        assert!(valid_projection_matrix(&m).is_err());
        m.rows = 2;
        m.pitch.x = f64::NAN;
        assert!(valid_projection_matrix(&m).is_err());
        m.pitch.x = 19.0;
        m.column_origins = vec![None; m.columns as usize + 1];
        assert!(valid_projection_matrix(&m).is_err());
        m.column_origins = vec![Some(Vec2 {
            x: f64::NAN,
            y: 0.0,
        })];
        assert!(valid_projection_matrix(&m).is_err());
    }

    #[test]
    fn column_basis_applies_splay_mirror_and_matrix_rotation() {
        let mut m = matrix();
        m.column_splays = vec![0.0];
        m.rotation = Some(90.0);
        m.mirror = None;
        let scene = project_matrix(&m, &[], ProjectionMode::Draft).unwrap();
        assert!((scene.columns[0].axis_x.x).abs() < 1e-9);
        assert!((scene.columns[0].axis_x.y - 1.0).abs() < 1e-9);
        m.mirror = Some(Mirror::X);
        let scene = project_matrix(&m, &[], ProjectionMode::Draft).unwrap();
        assert!((scene.columns[0].axis_x.y + 1.0).abs() < 1e-9);
        let determinant = scene.columns[0].axis_x.x * scene.columns[0].axis_y.y
            - scene.columns[0].axis_x.y * scene.columns[0].axis_y.x;
        assert!((determinant + 1.0).abs() < 1e-9);
    }
}
