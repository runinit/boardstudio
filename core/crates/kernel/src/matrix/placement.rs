//! Named matrix placement parameters. `SetMatrixPlacement` routes here so Core
//! owns the index and value rules; a gesture and an Inspector field reach the same
//! matrix for a given value.
use super::splay::rotate;
use super::{OVERRIDE, member_id, valid_projection_matrix};
use crate::model::{
    ColumnLayout, EditOperation, Matrix, MatrixCell, MatrixPlacementChange, Mirror,
    MirroredExceptionState, MirroredItemKind, OutlineFeature, Part, Pose2, ProjectDoc, Side, Vec2,
    convert_offset_available,
};
use std::collections::{BTreeMap, BTreeSet};

fn grow<T: Clone + Default>(values: &mut Vec<T>, index: usize) {
    if values.len() <= index {
        values.resize(index + 1, T::default());
    }
}

fn finite(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn set_stagger(
    layout: &mut Vec<ColumnLayout>,
    columns: u32,
    column: u32,
    value: f64,
) -> Result<bool, &'static str> {
    if column >= columns {
        return Err("Matrix placement column is out of bounds");
    }
    if !value.is_finite() {
        return Err("Matrix placement value is not finite");
    }
    let index = column as usize;
    // Equal signed zeros are a no-op, retaining the stored value (or absent record).
    if layout.get(index).map_or(0.0, |record| record.stagger) == value {
        return Ok(false);
    }
    grow(layout, index);
    layout[index].stagger = value;
    Ok(true)
}

/// Update a clone of a settled document whose target passed full scene validation.
/// The accepted parts, in draft order, supply the existing change-report oracle.
/// Returning None leaves the draft untouched for the general placement path.
pub fn try_settled_column_stagger(
    doc: &mut ProjectDoc,
    accepted_parts: &[Part],
    matrix_id: &str,
    column: u32,
    value: f64,
) -> Option<Vec<String>> {
    if !doc.constraints.is_empty()
        || doc
            .layouts
            .iter()
            .any(|layout| layout.mirror_link.is_some())
        || doc
            .hardware
            .iter()
            .flat_map(|hw| &hw.pairs)
            .any(|pair| pair.halves.is_some())
        || doc.parts.iter().any(|part| part.anchor.is_some())
        || doc.matrices.iter().any(|matrix| {
            matrix.anchor.is_some() || matrix.cells.iter().any(|cell| !cell.assemblies.is_empty())
        })
    {
        return None;
    }
    let matrix = doc
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == matrix_id)?;
    let mut input = super::evaluation::input(matrix);
    if !set_stagger(&mut input.column_layout, matrix.columns, column, value).ok()? {
        return Some(vec![]);
    }

    // A valid saved matrix may still need SetMatrix to repair its generated
    // parts or registrations. Only take the pose-only path if those writes
    // would leave membership, order, and attributes intact.
    let board = doc
        .boards
        .iter()
        .find(|board| Some(&board.id) == matrix.board_id.as_ref())?;
    let members: BTreeSet<_> = matrix.part_ids.iter().map(String::as_str).collect();
    let registration_matches = |ids: &[String], owns_matrix: bool| {
        let registered = ids.iter().map(String::as_str).collect();
        if owns_matrix {
            members.is_subset(&registered)
        } else {
            members.is_disjoint(&registered)
        }
    };
    if doc
        .boards
        .iter()
        .any(|candidate| !registration_matches(&candidate.part_ids, candidate.id == board.id))
        || doc.outline.iter().any(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
            if !registration_matches(part_ids, board.outline_ids.contains(id)))
        })
    {
        return None;
    }
    let parts: BTreeMap<_, _> = doc
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| (part.id.as_str(), index))
        .collect();
    if parts.contains_key(matrix_id) {
        // The general change report also compares a part that shares the matrix ID.
        return None;
    }
    let cells: BTreeMap<_, _> = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut owned = matrix.part_ids.iter();
    let mut slots = Vec::with_capacity(matrix.part_ids.len());
    for row in 0..matrix.rows {
        for col in 0..matrix.columns {
            let id = member_id(matrix_id, row, col);
            let index = parts.get(id.as_str()).copied();
            let cell = cells.get(&(row, col)).copied();
            if cell.is_some_and(|cell| !cell.enabled || cell.deleted) {
                if index.is_some() {
                    return None;
                }
                continue;
            }
            let index = index?;
            let part = &doc.parts[index];
            let definition = cell
                .and_then(|cell| cell.definition_id.as_ref())
                .unwrap_or(&matrix.definition_id);
            let variant = part
                .properties
                .as_ref()
                .and_then(|props| props.get("variant"));
            let variant_matches = match (variant, cell.and_then(|cell| cell.variant.as_ref())) {
                (None, None) => true,
                (Some(serde_json::Value::String(stored)), Some(authored)) => stored == authored,
                _ => false,
            };
            if owned.next() != Some(&id)
                || &part.definition_id != definition
                || part.side != Side::Front
                || !variant_matches
            {
                return None;
            }
            slots.push((index, row as usize, col as usize));
        }
    }
    if owned.next().is_some() {
        return None;
    }

    let first_column = column as usize;
    let evaluated = super::kernel::evaluate_column_suffix(&input, first_column).ok()?;
    matrix.column_layout = input.column_layout;
    matrix.edge_gap.get_or_insert(Vec2 {
        x: super::DEFAULT_EDGE_GAP_MM,
        y: super::DEFAULT_EDGE_GAP_MM,
    });
    let mut changed = vec![matrix.id.clone()];
    for (index, row, column) in slots {
        let part = &mut doc.parts[index];
        if column >= first_column && !super::overridden(part) {
            // The legacy receipt tests equality, but the saved pose keeps its bits.
            part.pose = evaluated[(column - first_column) * input.rows + row].pose;
        }
        if *part != accepted_parts[index] {
            changed.push(part.id.clone());
        }
    }
    Some(changed)
}

/// Apply one named placement change, changing only the entry it names. Existing
/// vectors are padded with zeros so an index beyond the stored length is valid.
pub fn update(
    matrix: &Matrix,
    change: &MatrixPlacementChange,
    doc: &ProjectDoc,
) -> Result<Matrix, String> {
    valid_projection_matrix(matrix)?;
    let mut after = matrix.clone();
    match change {
        MatrixPlacementChange::ColumnSpread { column, value } => {
            if *column == 0 || *column >= matrix.columns || value.is_some_and(|v| !v.is_finite()) {
                return Err("Spread requires a finite value and a nonzero column in bounds".into());
            }
            if matrix
                .column_layout
                .get(*column as usize)
                .copied()
                .unwrap_or_default()
                .spread
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.column_layout, *column as usize);
            after.column_layout[*column as usize].spread = *value;
        }
        MatrixPlacementChange::RowPadding { row, value } => {
            if *row == 0 || *row >= matrix.rows || value.is_some_and(|v| !v.is_finite()) {
                return Err("Padding requires a finite value and a nonzero row in bounds".into());
            }
            if matrix
                .row_layout
                .get(*row as usize)
                .copied()
                .unwrap_or_default()
                .padding
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.row_layout, *row as usize);
            after.row_layout[*row as usize].padding = *value;
        }
        MatrixPlacementChange::RowBend { row, value } => {
            if *row >= matrix.rows || !value.is_finite() {
                return Err("Matrix row Bend must be finite and in bounds".into());
            }
            if matrix
                .row_layout
                .get(*row as usize)
                .copied()
                .unwrap_or_default()
                .bend
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.row_layout, *row as usize);
            after.row_layout[*row as usize].bend = *value;
        }
        MatrixPlacementChange::KeyShift { row, column, .. }
        | MatrixPlacementChange::KeyBend { row, column, .. } => {
            if *row >= matrix.rows || *column >= matrix.columns {
                return Err("Matrix placement key is out of bounds".into());
            }
            let current = matrix
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            match change {
                MatrixPlacementChange::KeyShift { value, .. } => {
                    if !finite(*value) {
                        return Err("Key Shift must be finite".into());
                    }
                    if current.and_then(|cell| cell.shift).unwrap_or_default() == *value {
                        return Ok(matrix.clone());
                    }
                }
                MatrixPlacementChange::KeyBend { value, .. } => {
                    if !value.is_finite() {
                        return Err("Key Bend must be finite".into());
                    }
                    if current.and_then(|cell| cell.bend).unwrap_or_default() == *value {
                        return Ok(matrix.clone());
                    }
                }
                _ => unreachable!("only key Shift and Bend enter this arm"),
            }
            let index = after
                .cells
                .iter()
                .position(|cell| cell.row == *row && cell.column == *column)
                .unwrap_or_else(|| {
                    after.cells.push(MatrixCell {
                        row: *row,
                        column: *column,
                        enabled: true,
                        deleted: false,
                        shift: None,
                        bend: None,
                        definition_id: None,
                        variant: None,
                        offset: None,
                        rotation: None,
                        assemblies: vec![],
                        assemblies_local: None,
                    });
                    after.cells.len() - 1
                });
            let cell = &mut after.cells[index];
            match change {
                MatrixPlacementChange::KeyShift { value, .. } => cell.shift = Some(*value),
                MatrixPlacementChange::KeyBend { value, .. } => cell.bend = Some(*value),
                _ => unreachable!("only key Shift and Bend enter this arm"),
            }
        }
        MatrixPlacementChange::RowShift { row, value } => {
            if *row >= matrix.rows || !finite(*value) {
                return Err("Matrix row Shift must be finite and in bounds".into());
            }
            if matrix
                .row_layout
                .get(*row as usize)
                .copied()
                .unwrap_or_default()
                .shift
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.row_layout, *row as usize);
            after.row_layout[*row as usize].shift = *value;
        }
        MatrixPlacementChange::MatrixTranslation { delta } => {
            if *delta != Vec2::default() {
                rigid_translation_available(matrix, None, doc)?;
            }
            translate_frame(&mut after, *delta, doc)?;
        }
        MatrixPlacementChange::ColumnTranslation { column, delta } => {
            if !finite(*delta) || *column >= matrix.columns {
                return Err("Column translation must be finite and in bounds".into());
            }
            if *delta == Vec2::default() {
                return Ok(matrix.clone());
            }
            rigid_translation_available(matrix, Some(*column), doc)?;
            let evaluated = super::evaluated(matrix, doc)?;
            if *column == 0 {
                translate_frame(&mut after, *delta, doc)?;
            } else {
                translate_step(
                    &mut after,
                    *column,
                    *delta,
                    &evaluated.columns[*column as usize],
                );
            }
            if *column + 1 < matrix.columns {
                translate_step(
                    &mut after,
                    *column + 1,
                    Vec2 {
                        x: -delta.x,
                        y: -delta.y,
                    },
                    &evaluated.columns[*column as usize + 1],
                );
            }
        }
        MatrixPlacementChange::ColumnStagger { column, value } => {
            if !set_stagger(&mut after.column_layout, matrix.columns, *column, *value)? {
                return Ok(after);
            }
        }
        MatrixPlacementChange::ColumnOffset { column, value } => {
            if *column >= matrix.columns {
                return Err("Matrix placement column is out of bounds".into());
            }
            if !finite(*value) {
                return Err("Matrix placement value is not finite".into());
            }
            if after
                .column_layout
                .get(*column as usize)
                .map(|record| record.offset)
                .unwrap_or_default()
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.column_layout, *column as usize);
            after.column_layout[*column as usize].offset = *value;
        }
        MatrixPlacementChange::RowOffset { row, value } => {
            if *row >= matrix.rows {
                return Err("Matrix placement row is out of bounds".into());
            }
            if !finite(*value) {
                return Err("Matrix placement value is not finite".into());
            }
            if after
                .row_layout
                .get(*row as usize)
                .map(|record| record.offset)
                .unwrap_or_default()
                == *value
            {
                return Ok(matrix.clone());
            }
            grow(&mut after.row_layout, *row as usize);
            after.row_layout[*row as usize].offset = *value;
        }
        MatrixPlacementChange::ColumnOffsetToStagger { column } => {
            if *column >= matrix.columns {
                return Err("Matrix placement column is out of bounds".into());
            }
            let index = *column as usize;
            let amount = after
                .column_layout
                .get(index)
                .map_or(0.0, |record| record.offset.y);
            // Nothing to convert is a valid no-op, whatever the splay.
            if amount == 0.0 {
                return Ok(matrix.clone());
            }
            convert_offset_available(matrix, *column)?;
            grow(&mut after.column_layout, index);
            after.column_layout[index].stagger += amount;
            if *column + 1 < matrix.columns {
                grow(&mut after.column_layout, index + 1);
                after.column_layout[index + 1].stagger -= amount;
            }
            after.column_layout[index].offset.y = 0.0;
        }
    }
    super::evaluated(&after, doc)?;
    Ok(after)
}

fn rigid_translation_available(
    matrix: &Matrix,
    column: Option<u32>,
    doc: &ProjectDoc,
) -> Result<(), String> {
    let links = doc
        .layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .map(|link| &link.correspondences);
    let derived = boardstudio_model::mirrored_half::matrix_counterparts(doc, &matrix.id);
    let mut matrices = vec![matrix.id.as_str()];
    for record in links
        .flatten()
        .filter(|record| record.kind == MirroredItemKind::Matrix)
    {
        let counterpart = if record.source_id == matrix.id {
            record.mirror_id.as_str()
        } else if record.mirror_id == matrix.id {
            record.source_id.as_str()
        } else {
            continue;
        };
        if super::layout::exception_state(doc, MirroredItemKind::Matrix, counterpart).is_none() {
            matrices.push(counterpart);
        }
    }
    // A derived two-board counterpart follows the source unless the pair
    // stores an overlay for it.
    for counterpart in &derived {
        if super::layout::exception_state(doc, MirroredItemKind::Matrix, counterpart).is_none()
            && !matrices.contains(&counterpart.as_str())
        {
            matrices.push(counterpart);
        }
    }
    for owner in doc
        .matrices
        .iter()
        .filter(|owner| matrices.contains(&owner.id.as_str()))
    {
        for row in 0..owner.rows {
            for col in 0..owner.columns {
                if column.is_some_and(|column| column != col) {
                    continue;
                }
                let id = member_id(&owner.id, row, col);
                if !owner.part_ids.contains(&id) {
                    continue;
                }
                if super::layout::exception_state(doc, MirroredItemKind::Part, &id)
                    == Some(MirroredExceptionState::Local)
                {
                    return Err("Exact translation cannot move a Local key; explicitly Rejoin or edit its independent pose".into());
                }
            }
        }
    }
    Ok(())
}

/// Constraints remain solver-owned; check the exact promise after that solver settles.
pub fn validate_translation(
    before: &[Part],
    after: &ProjectDoc,
    operation: &EditOperation,
) -> Result<(), String> {
    let (matrix_id, column, delta) = match operation {
        EditOperation::SetMatrixPlacement {
            matrix_id,
            change: MatrixPlacementChange::MatrixTranslation { delta },
        } => (matrix_id, None, delta),
        EditOperation::SetMatrixPlacement {
            matrix_id,
            change: MatrixPlacementChange::ColumnTranslation { column, delta },
        } => (matrix_id, Some(*column), delta),
        _ => return Ok(()),
    };
    let matrix = after
        .matrices
        .iter()
        .find(|matrix| &matrix.id == matrix_id)
        .ok_or("Translated matrix is missing")?;
    for row in 0..matrix.rows {
        for col in 0..matrix.columns {
            let id = member_id(matrix_id, row, col);
            let Some(previous) = before.iter().find(|part| part.id == id) else {
                continue;
            };
            let part = after
                .parts
                .iter()
                .find(|part| part.id == id)
                .ok_or("Translated key is missing")?;
            let movement = if column.is_none_or(|column| column == col) {
                *delta
            } else {
                Vec2::default()
            };
            let wanted = Vec2 {
                x: previous.pose.at.x + movement.x,
                y: previous.pose.at.y + movement.y,
            };
            // Translation changes no turn parameters, so the resolved heading must stay exact.
            if !finite(wanted)
                || !finite(part.pose.at)
                || (part.pose.at.x - wanted.x).abs() > 1e-9
                || (part.pose.at.y - wanted.y).abs() > 1e-9
                || part.pose.rotation != previous.pose.rotation
            {
                return Err("Exact translation is not representable after constraint resolution; edit the constraint or its selected scope first".into());
            }
        }
    }
    Ok(())
}

fn translate_step(
    matrix: &mut Matrix,
    column: u32,
    delta: Vec2,
    frame: &super::kernel::ColumnFrame,
) {
    grow(&mut matrix.column_layout, column as usize);
    let values = &mut matrix.column_layout[column as usize];
    if let Some(axis) = frame.spread_axis {
        values.spread =
            Some(values.spread.unwrap_or(matrix.pitch.x) + delta.x * axis.x + delta.y * axis.y);
    }
    values.stagger += delta.x * frame.stagger_axis.x + delta.y * frame.stagger_axis.y;
}

fn translate_frame(matrix: &mut Matrix, delta: Vec2, doc: &ProjectDoc) -> Result<(), String> {
    if !finite(delta) {
        return Err("Matrix translation must be finite".into());
    }
    let local = super::evaluation::origin_delta(matrix, doc, delta)?;
    matrix.origin.x += local.x;
    matrix.origin.y += local.y;
    super::evaluated(matrix, doc)?;
    Ok(())
}

/// Which parts an edit that carries companions reports as changed.
#[derive(Clone, Copy)]
pub enum Reported {
    /// Preserve the edit closure's ID list. `SetMatrix`'s matrix and generated-member
    /// report is pinned by `core/tests/fixtures/boundary/transcript.json`.
    PreserveEditReport,
    /// Every part the edit touched, moved or not. A splay uses this: the golden
    /// transcript pins that it reports every matrix member.
    Touched,
    /// Only parts whose projection actually moved.
    Moved,
}

/// Run a matrix edit and carry authored companions with their keys.
pub fn carrying_authored_companions(
    doc: &mut ProjectDoc,
    reported: Reported,
    edit: impl FnOnce(&mut ProjectDoc) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, String> {
    let before: BTreeMap<String, Part> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.clone()))
        .collect();
    let before_matrices: BTreeMap<String, Matrix> = doc
        .matrices
        .iter()
        .map(|matrix| (matrix.id.clone(), matrix.clone()))
        .collect();
    let mut changed = edit(doc)?;
    let carried = carry_authored_companions(doc, &before, &before_matrices);
    if !matches!(reported, Reported::PreserveEditReport) {
        changed.extend(carried);
    }
    if matches!(reported, Reported::Moved) {
        changed.retain(|id| match before.get(id) {
            Some(previous) => doc
                .parts
                .iter()
                .find(|part| &part.id == id)
                .is_none_or(|part| part != previous),
            None => true,
        });
    }
    Ok(changed)
}

/// Move authored companions rigidly with their key, so a placement change keeps
/// the relative pose the user authored: the offset in the key's frame and the turn
/// relative to the key. A splay turns the key, so the companion turns around it.
/// Companions with an explicit constraint keep that constraint instead: the
/// constraint solver settles them.
fn carry_authored_companions(
    doc: &mut ProjectDoc,
    before: &BTreeMap<String, Part>,
    before_matrices: &BTreeMap<String, Matrix>,
) -> Vec<String> {
    let mut carries: Vec<(String, Pose2, Pose2, Option<CompanionMirror>)> = Vec::new();
    for matrix in &doc.matrices {
        let previous_matrix = before_matrices.get(&matrix.id);
        for cell in &matrix.cells {
            if !cell.enabled || cell.deleted {
                continue;
            }
            let key_id = member_id(&matrix.id, cell.row, cell.column);
            let Some(previous) = before.get(&key_id) else {
                continue;
            };
            let Some(key) = doc.parts.iter().find(|part| part.id == key_id) else {
                continue;
            };
            let mirror = previous_matrix
                .filter(|previous_matrix| {
                    previous_matrix.mirror.unwrap_or(crate::model::Mirror::None)
                        != matrix.mirror.unwrap_or(crate::model::Mirror::None)
                })
                .map(|previous_matrix| CompanionMirror {
                    from_mirror: previous_matrix.mirror.unwrap_or(Mirror::None),
                    to_mirror: matrix.mirror.unwrap_or(Mirror::None),
                    from_rotation: previous_matrix.rotation.unwrap_or(0.0),
                    to_rotation: matrix.rotation.unwrap_or(0.0),
                });
            if key.pose == previous.pose && mirror.is_none() {
                continue;
            }
            for assembly in &cell.assemblies {
                let id = format!("{key_id}/{}", assembly.id);
                if doc.constraint_driving_part(&id).ok().flatten().is_some() {
                    continue;
                }
                let authored = doc
                    .parts
                    .iter()
                    .find(|part| part.id == id)
                    .is_some_and(|part| {
                        part.properties.as_ref().is_some_and(|properties| {
                            properties.get(OVERRIDE) == Some(&serde_json::json!(1))
                        })
                    });
                if authored {
                    carries.push((id, previous.pose, key.pose, mirror));
                }
            }
        }
    }
    let mut changed = Vec::new();
    for (id, from, to, mirror) in carries {
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == id) {
            let turn = to.rotation - from.rotation;
            // Turn about the key's old position, then move by the key's delta. With no
            // turn the companion only adds the delta, so a translation stays bit-identical.
            if let Some(mirror) = mirror {
                let relative = Vec2 {
                    x: part.pose.at.x - from.at.x,
                    y: part.pose.at.y - from.at.y,
                };
                let offset = mirror.apply_world(relative);
                part.pose.at = Vec2 {
                    x: to.at.x + offset.x,
                    y: to.at.y + offset.y,
                };

                let heading = Vec2 {
                    x: part.pose.rotation.to_radians().cos(),
                    y: part.pose.rotation.to_radians().sin(),
                };
                let heading = mirror.apply_world(heading);
                part.pose.rotation = heading.y.atan2(heading.x).to_degrees();
            } else {
                let at = if turn == 0.0 {
                    part.pose.at
                } else {
                    rotate(part.pose.at, turn, from.at)
                };
                part.pose.at = Vec2 {
                    x: at.x + (to.at.x - from.at.x),
                    y: at.y + (to.at.y - from.at.y),
                };
                if turn != 0.0 {
                    part.pose.rotation += turn;
                }
            }
            changed.push(id);
        }
    }
    changed
}

#[derive(Clone, Copy)]
struct CompanionMirror {
    from_mirror: Mirror,
    to_mirror: Mirror,
    from_rotation: f64,
    to_rotation: f64,
}

impl CompanionMirror {
    fn apply(&self, point: Vec2) -> Vec2 {
        let reflect = |point: Vec2, mirror| match mirror {
            Mirror::X => Vec2 {
                x: -point.x,
                y: point.y,
            },
            Mirror::Y => Vec2 {
                x: point.x,
                y: -point.y,
            },
            Mirror::None => point,
        };
        reflect(reflect(point, self.from_mirror), self.to_mirror)
    }

    fn apply_world(&self, point: Vec2) -> Vec2 {
        let local = rotate(point, -self.from_rotation, Vec2::default());
        rotate(self.apply(local), self.to_rotation, Vec2::default())
    }
}
