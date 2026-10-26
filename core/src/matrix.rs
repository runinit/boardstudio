//! Matrix projection: generated keys and their companions (diodes, LEDs).
//!
//! Where things live (search the names; line numbers drift):
//! - Edits: the `SetMatrix`, `SetMatrixSplay`, `SetMatrixPlacement` and `MoveParts` arms of
//!   `apply` in lib.rs.
//! - Key geometry: `location`, `column_rotation`, `column_basis` here; `splay::update` and
//!   `placement::update` compute the changed matrix.
//! - Companion modes, in the order the solver applies them (`layout::companion_mode`
//!   classifies, `layout::assembly_placements` feeds the solver):
//!   1. constrained: an explicit `Constraint` targets the companion (`constraint_pose` in
//!      constraints.rs);
//!   2. authored: `layoutOverride == 1` (`OVERRIDE`, `mark_override`) keeps the pose;
//!      `placement::carrying_authored_companions` carries it with its key on splay and
//!      placement edits (`SetMatrix` edits: ticket 144);
//!   3. following: generated from the key by `project_matrix` here; the solver places it with
//!      `follow_pose` in constraints.rs.
//! - Key moves and linked halves: `layout::move_keys`, `layout::sync`, `set_component_pose`.
//! - Tests: `core/tests/matrix_companions.rs`, `matrix_placement.rs`, `splay_companions.rs`.

pub(crate) mod anchors;
pub(crate) mod deletion;
pub(crate) mod evaluation;
pub mod kernel;
use crate::model::{
    Matrix, MatrixCell, MatrixColumnBasis, MatrixScene, MatrixSceneCell, OutlineFeature, Part, Pin,
    Pose2, ProjectDoc, Side, Vec2,
};
use serde_json::json;
pub(crate) mod layout;
pub(crate) mod part_anchors;
pub(crate) mod placement;
pub(crate) mod splay;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAX_MATRIX_PARTS: u32 = 4096;
pub(crate) const OVERRIDE: &str = "layoutOverride";
const DEFAULT_EDGE_GAP_MM: f64 = 1.0;
const SWITCH_ROW_PAD: &str = "one";
const SWITCH_COLUMN_PAD: &str = "two";

fn matrix_terminal_pad_ids(definition: &crate::model::PartDefinition, row: bool) -> Vec<String> {
    if let Some(press) = definition
        .input_profile
        .as_ref()
        .and_then(|p| p.press.as_ref())
    {
        let role = if row { &press.row } else { &press.column };
        return definition.terminals.get(role).cloned().unwrap_or_else(|| {
            definition
                .pads
                .iter()
                .filter(|p| &p.id == role || &p.number == role)
                .map(|p| p.id.clone())
                .collect()
        });
    }
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

pub(crate) fn member_id(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

/// The generated part ID of one assembly on a matrix key.
pub(crate) fn companion_id(key_id: &str, assembly_id: &str) -> String {
    format!("{key_id}/{assembly_id}")
}

fn companion_pose(key: Pose2, assembly: &crate::model::MatrixAssembly) -> Pose2 {
    let (sin, cos) = key.rotation.to_radians().sin_cos();
    // Preserve materialization's arithmetic order: regrouping changes persisted pose bits.
    Pose2 {
        at: Vec2 {
            x: key.at.x + assembly.offset.x * cos - assembly.offset.y * sin,
            y: key.at.y + assembly.offset.x * sin + assembly.offset.y * cos,
        },
        rotation: key.rotation + assembly.rotation.unwrap_or(0.0),
    }
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
            if cells
                .get(&(row, column))
                .is_some_and(|cell| !cell.enabled || cell.deleted)
            {
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
                if requested.contains(&id) {
                    cell.enabled = false;
                    cell.deleted = true;
                    removed.extend(
                        matrix
                            .part_ids
                            .iter()
                            .filter(|part| part.starts_with(&prefix))
                            .cloned(),
                    );
                } else {
                    cell.assemblies
                        .retain(|assembly| !requested.contains(&companion_id(&id, &assembly.id)));
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
    valid_dimensions(matrix)?;
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
    // Keep legacy definitions compatible; explicitly described inputs must be suitable keys.
    for id in std::iter::once(&matrix.definition_id).chain(
        matrix
            .cells
            .iter()
            .filter(|cell| cell.enabled && !cell.deleted)
            .filter_map(|cell| cell.definition_id.as_ref()),
    ) {
        if let Some(def) = doc.definitions.iter().find(|def| &def.id == id)
            && (def.input_profile.is_some() || crate::inputs::profile(def).rotary.is_some())
        {
            crate::inputs::validate_matrix_input(def)
                .map_err(|error| format!("{}: {error}", def.name))?;
        }
    }
    if matrix.cells.len() > (matrix.rows * matrix.columns) as usize {
        return Err("Matrix cells exceed dimensions".into());
    }
    valid_projection_matrix(matrix)?;
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

pub(crate) fn evaluated(matrix: &Matrix, doc: &ProjectDoc) -> Result<kernel::MatrixLayout, String> {
    evaluation::evaluate(matrix, doc)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProjectionMode {
    Actual,
    Draft,
}

/// Must run before any work proportional to rows or columns.
pub(crate) fn valid_dimensions(matrix: &Matrix) -> Result<(), String> {
    if matrix.rows == 0
        || matrix.columns == 0
        || matrix
            .rows
            .checked_mul(matrix.columns)
            .is_none_or(|count| count > MAX_MATRIX_PARTS)
    {
        return Err("Matrix dimensions are invalid or exceed the part limit".into());
    }
    Ok(())
}

pub(crate) fn valid_projection_matrix(matrix: &Matrix) -> Result<(), String> {
    valid_dimensions(matrix)?;
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
    kernel::evaluate(&evaluation::input(matrix), kernel::RawAnchorContext::World)
        .map(|_| ())
        .map_err(str::to_owned)
}

pub(crate) fn project_matrix(
    matrix: &Matrix,
    doc: &ProjectDoc,
    mode: ProjectionMode,
) -> Result<MatrixScene, String> {
    let evaluated = evaluated(matrix, doc)?;
    let members = if mode == ProjectionMode::Actual {
        cell_members(matrix)
    } else {
        BTreeMap::new()
    };
    let by_id: BTreeMap<_, _> = if mode == ProjectionMode::Actual {
        doc.parts
            .iter()
            .map(|part| (part.id.as_str(), part))
            .collect()
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
            if cell.is_some_and(|item| item.deleted) {
                continue;
            }
            let enabled = cell.is_none_or(|item| item.enabled);
            let member_id = if enabled {
                members
                    .get(&(row, column))
                    .filter(|id| by_id.contains_key(id.as_str()))
                    .cloned()
            } else {
                None
            };
            let mut pose = evaluation::cell_pose(&evaluated, row, column)?;
            if mode == ProjectionMode::Actual
                && let Some(id) = member_id.as_deref().and_then(|id| by_id.get(id))
            {
                pose = id.pose;
            }
            if !pose.at.x.is_finite() || !pose.at.y.is_finite() || !pose.rotation.is_finite() {
                return Err("Matrix projection exceeds finite geometry".into());
            }
            let geometry = evaluated
                .cells
                .iter()
                .find(|cell| cell.row == row as usize && cell.column == column as usize)
                .ok_or("Key geometry is missing")?;
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
                shift_frame: geometry.shift_frame,
                offset_frame: geometry.offset_frame,
                row_shift: geometry.contributions.row_shift,
                key_shift: geometry.contributions.key_shift,
                row_bend: geometry.contributions.row_bend,
                key_bend: geometry.contributions.key_bend,
                cumulative_shift: geometry.contributions.cumulative_shift,
                cumulative_bend: geometry.contributions.cumulative_bend,
            });
        }
    }
    Ok(MatrixScene {
        matrix_id: matrix.id.clone(),
        cells: projected,
        columns: evaluated
            .columns
            .iter()
            .enumerate()
            .map(|(column, frame)| MatrixColumnBasis {
                column: column as u32,
                base: frame.base,
                rotation: frame.rotation,
                relative_origin: frame.relative_origin,
                spread_axis: frame.spread_axis,
                splay_origin: frame.splay_origin,
                splay_angle: frame.splay,
                custom_origin: frame.custom_origin,
                axis_x: frame.frame.axis_x,
                axis_y: frame.frame.axis_y,
                stagger_axis: frame.stagger_axis,
            })
            .collect(),
        companions: vec![],
    })
}

/// The companions this matrix generates, with the placement mode Core uses for
/// each. Only enabled, non-deleted cells whose companion part exists are listed.
pub(crate) fn scene_companions(
    matrix: &Matrix,
    doc: &ProjectDoc,
) -> Vec<crate::model::MatrixCompanionPlacement> {
    let mut companions = Vec::new();
    for cell in &matrix.cells {
        if !cell.enabled || cell.deleted {
            continue;
        }
        let key_id = member_id(&matrix.id, cell.row, cell.column);
        for assembly in &cell.assemblies {
            let part_id = companion_id(&key_id, &assembly.id);
            if !doc.parts.iter().any(|part| part.id == part_id) {
                continue;
            }
            companions.push(crate::model::MatrixCompanionPlacement {
                part_id: part_id.clone(),
                key_id: key_id.clone(),
                mode: layout::companion_mode(doc, &part_id),
            });
        }
    }
    companions
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
            if resized.row_layout.len() <= before.rows as usize {
                resized.row_layout.truncate(resized.rows as usize);
            }
            resized
                .cells
                .retain(|cell| cell.row < resized.rows || cell.row >= before.rows);
        }
        if resized.columns < before.columns {
            if resized.column_layout.len() <= before.columns as usize {
                resized.column_layout.truncate(resized.columns as usize);
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
    let evaluated = evaluated(incoming, doc)?;
    for row in 0..incoming.rows {
        for column in 0..incoming.columns {
            let cell = cells.get(&(row, column)).copied();
            if cell.is_some_and(|cell| !cell.enabled || cell.deleted) {
                continue;
            }
            let id = member_id(&incoming.id, row, column);
            if layout::omitted(doc, &incoming.id, &id) {
                continue;
            }
            let pose = evaluation::cell_pose(&evaluated, row, column)?;
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
                    let id = companion_id(&members[0].0, &assembly.id);
                    members.push((
                        id,
                        assembly.definition_id.clone(),
                        companion_pose(pose, assembly),
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
                    let new_source = doc
                        .definitions
                        .iter()
                        .find(|definition| definition.id == definition_id)
                        .and_then(|definition| definition.generator.as_ref())
                        .map(|generator| generator.source.clone());
                    let part = &mut doc.parts[index];
                    let new_definition = part.definition_id != definition_id;
                    if new_definition && part.keycap.is_none() {
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
                    if let (true, Some(source), Some(parameters)) = (
                        new_definition,
                        new_source.as_deref(),
                        part.generator_parameters.as_mut(),
                    ) {
                        // Parameters of the previous generator (its terminal nets, for
                        // example) mean nothing to the new one and must not be saved on it.
                        crate::generators::drop_foreign_parameters(source, parameters);
                    }
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
                        anchor: None,
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
    use crate::model::Mirror;

    fn project_matrix(
        matrix: &Matrix,
        parts: &[Part],
        mode: ProjectionMode,
    ) -> Result<MatrixScene, String> {
        let mut doc = ProjectDoc::empty("test", "Test");
        doc.parts = parts.to_vec();
        super::project_matrix(matrix, &doc, mode)
    }

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
            row_layout: vec![],
            column_layout: vec![
                crate::model::ColumnLayout::default(),
                crate::model::ColumnLayout {
                    stagger: 3.0,
                    splay: 15.0,
                    ..Default::default()
                },
            ],
            anchor: None,
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
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
            anchor: None,
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
    fn following_companion_preserves_materialized_pose_exactly_on_refresh_and_reopen() {
        let mut doc = ProjectDoc::empty("exact-companion", "Exact companion");
        doc.definitions = serde_json::from_value(json!([
            {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
            {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]}
        ]))
        .unwrap();
        let matrix = serde_json::from_value(json!({
            "id":"m","rows":1,"columns":1,"pitch":{"x":19,"y":19},
            "origin":{"x":1,"y":-1},"definitionId":"switch","partIds":[],
            "cells":[{"row":0,"column":0,"enabled":true,"rotation":90,
                "assemblies":[{"id":"diode","definitionId":"diode",
                    "offset":{"x":1,"y":1},"rotation":17}]}]
        }))
        .unwrap();
        set_matrix(&mut doc, &matrix).unwrap();
        let companion = |doc: &ProjectDoc| {
            doc.parts
                .iter()
                .find(|part| part.id == "matrix/m/r0c0/diode")
                .unwrap()
                .pose
        };
        let key = doc
            .parts
            .iter()
            .find(|part| part.id == "matrix/m/r0c0")
            .unwrap();
        assert_eq!(
            key.pose,
            Pose2 {
                at: Vec2 { x: 1.0, y: -1.0 },
                rotation: 90.0
            }
        );
        let (sin, cos) = 90_f64.to_radians().sin_cos();
        // Pin the original materialization grouping: persisted addition is not associative.
        let expected = Pose2 {
            at: Vec2 {
                x: (1.0 + cos) - sin,
                y: (-1.0 + sin) + cos,
            },
            rotation: 90.0 + 17.0,
        };
        assert_ne!(
            expected.at,
            Vec2 {
                x: 1.0 + (cos - sin),
                y: -1.0 + (sin + cos)
            }
        );
        let materialized = companion(&doc);
        assert_eq!(materialized, expected, "original materialization order");
        assert_eq!(
            layout::companion_mode(&doc, "matrix/m/r0c0/diode"),
            crate::model::MatrixCompanionMode::Follows
        );
        layout::refresh_existing(&mut doc).unwrap();
        assert_eq!(
            companion(&doc),
            materialized,
            "refresh must preserve persisted pose exactly"
        );
        let saved = serde_json::to_string(&doc).unwrap();
        let mut engine = crate::CoreEngine::new();
        let reply = engine.handle(crate::model::CoreRequest::Open {
            id: "reopen".into(),
            document: serde_json::from_str(&saved).unwrap(),
        });
        let crate::model::CoreReply::Scene { document, .. } = reply else {
            panic!("expected reopened scene: {reply:?}");
        };
        assert_eq!(
            companion(&document),
            materialized,
            "Open must preserve persisted pose exactly"
        );
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

    /// The captured oracle predates grouped layout records, so the fixture keeps its
    /// legacy per-axis arrays and this adapter rewrites them as column and row records.
    fn grouped_from_legacy(mut matrix: serde_json::Value) -> Matrix {
        use serde_json::{Map, Value, json};
        let object = matrix.as_object_mut().unwrap();
        let mut take = |key: &str| match object.remove(key) {
            Some(Value::Array(items)) => items,
            _ => vec![],
        };
        let (offsets, origins, splays, staggers) = (
            take("columnOffsets"),
            take("columnOrigins"),
            take("columnSplays"),
            take("columnStaggers"),
        );
        let row_offsets = take("rowOffsets");
        let columns = [&offsets, &origins, &splays, &staggers]
            .iter()
            .map(|items| items.len())
            .max()
            .unwrap();
        let record = |fields: Vec<(&str, Option<&Value>)>| {
            let mut out = Map::new();
            for (key, value) in fields {
                if let Some(value) = value.filter(|value| !value.is_null()) {
                    out.insert(key.into(), value.clone());
                }
            }
            Value::Object(out)
        };
        let column_layout: Vec<_> = (0..columns)
            .map(|i| {
                record(vec![
                    ("offset", offsets.get(i)),
                    ("splayOrigin", origins.get(i)),
                    ("splay", splays.get(i)),
                    ("stagger", staggers.get(i)),
                ])
            })
            .collect();
        let row_layout: Vec<_> = row_offsets
            .iter()
            .map(|offset| record(vec![("offset", Some(offset))]))
            .collect();
        object.insert("columnLayout".into(), json!(column_layout));
        object.insert("rowLayout".into(), json!(row_layout));
        serde_json::from_value(matrix).unwrap()
    }

    /// The captured rows predate the shift and offset frames, so only the oracle's fields.
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedCell {
        row: u32,
        column: u32,
        enabled: bool,
        member_id: Option<String>,
        pose: Pose2,
    }

    // Bypass Open intentionally: it refreshes ordinary parts before projection. This
    // fixture pins saved-pose reads without Local exceptions or layoutOverride.
    #[test]
    fn ordinary_parts_match_saved_pose_and_member_id_fixtures_in_every_mirror_mode() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/matrix-projections.json"))
                .unwrap();
        let cases = fixtures["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 9);
        let mut cell_count = 0;
        let mut saved_modes = BTreeSet::new();
        for fixture in cases {
            let case = fixture["name"].as_str().unwrap();
            let matrix = grouped_from_legacy(fixture["matrix"].clone());
            let parts: Vec<Part> = serde_json::from_value(fixture["parts"].clone()).unwrap();
            for part in &parts {
                assert!(
                    part.properties
                        .as_ref()
                        .is_none_or(|properties| !properties.contains_key(OVERRIDE))
                );
            }
            if !parts.is_empty() {
                assert!(saved_modes.insert(case));
            }
            let expected: Vec<ExpectedCell> =
                serde_json::from_value(fixture["expected"].clone()).unwrap();
            let scene = project_matrix(&matrix, &parts, ProjectionMode::Actual).unwrap();
            assert_eq!(scene.cells.len(), expected.len(), "{case}: cell count");
            cell_count += scene.cells.len();
            for (actual, expected) in scene.cells.iter().zip(expected) {
                let at = format!("{case} r{}c{}", expected.row, expected.column);
                assert_eq!(
                    (actual.row, actual.column),
                    (expected.row, expected.column),
                    "{at} row,column"
                );
                assert_eq!(actual.enabled, expected.enabled, "{at} enabled");
                assert_eq!(actual.member_id, expected.member_id, "{at} memberId");
                for (name, actual, expected) in [
                    ("x", actual.pose.at.x, expected.pose.at.x),
                    ("y", actual.pose.at.y, expected.pose.at.y),
                    ("rotation", actual.pose.rotation, expected.pose.rotation),
                ] {
                    assert!(
                        (actual - expected).abs() < 1e-9,
                        "{at} pose {name}: expected {expected}, got {actual}"
                    );
                }
            }
        }
        assert_eq!(cell_count, 81);
        assert_eq!(
            saved_modes,
            BTreeSet::from(["none-saved-poses", "x-saved-poses", "y-saved-poses"])
        );
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
    fn projection_rejects_invalid_draft_geometry() {
        let mut m = matrix();
        m.rows = 0;
        assert!(valid_projection_matrix(&m).is_err());
        m.rows = 2;
        m.pitch.x = f64::NAN;
        assert!(valid_projection_matrix(&m).is_err());
        m.pitch.x = 19.0;
        {
            let values = (vec![None; m.columns as usize + 1])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
            m.column_layout
                .resize_with(values.len().max(m.column_layout.len()), Default::default);
            for record in &mut m.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in m.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        assert!(valid_projection_matrix(&m).is_err());
        {
            let values = (vec![Some(Vec2 {
                x: f64::NAN,
                y: 0.0,
            })])
            .into_iter()
            .map(|value| value.unwrap_or_default())
            .collect::<Vec<_>>();
            m.column_layout
                .resize_with(values.len().max(m.column_layout.len()), Default::default);
            for record in &mut m.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in m.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        assert!(valid_projection_matrix(&m).is_err());
    }

    #[test]
    fn column_basis_applies_splay_mirror_and_matrix_rotation() {
        let mut m = matrix();
        {
            let values = vec![0.0];
            m.column_layout
                .resize_with(values.len().max(m.column_layout.len()), Default::default);
            for record in &mut m.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in m.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
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
