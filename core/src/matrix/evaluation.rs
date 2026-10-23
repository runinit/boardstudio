//! Document adaptation and anchor resolution for the single layout kernel.
use super::kernel::{self, FollowBasis, KeyLayout, LayoutInput, MatrixLayout, RawAnchorContext};
use crate::model::{Matrix, Pose2, ProjectDoc, Vec2};
use std::collections::BTreeSet;
use std::borrow::Cow;

/// Omitted owners have a live logical matrix, but never recreate persisted parts.
pub(crate) fn logical_matrix<'a>(id: &str, doc: &'a ProjectDoc) -> Result<Cow<'a, Matrix>, String> {
    if let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == id) {
        return Ok(Cow::Borrowed(matrix));
    }
    use crate::model::{MirroredExceptionState, MirroredItemKind};
    let links = doc.layouts.iter().filter_map(|layout| layout.mirror_link.as_ref()).map(|link| (&link.correspondences, &link.exceptions, super::layout::axis_mapping(link.axis_x)))
        .chain(doc.hardware.iter().flat_map(|hardware| &hardware.pairs).filter_map(|pair| pair.halves.as_ref().map(|halves| (&halves.correspondences, &halves.exceptions, pair.mapping))));
    for (records, exceptions, mapping) in links {
        let Some(record) = records.iter().find(|record| record.kind == MirroredItemKind::Matrix && record.mirror_id == id) else { continue; };
        if !exceptions.iter().any(|exception| exception.kind == MirroredItemKind::Matrix && exception.mirror_id == id && exception.state == MirroredExceptionState::Omitted) { continue; }
        let source = doc.matrices.iter().find(|matrix| matrix.id == record.source_id).ok_or("Omitted matrix source is missing")?;
        let mut target = source.clone();
        target.id = id.to_owned();
        target.board_id = doc.layouts.iter().find(|layout| layout.matrix_ids.iter().any(|member| member == id)).map(|layout| layout.board_id.clone());
        if let Some(anchor) = &mut target.anchor {
            anchor.matrix_id = records.iter().find(|record| record.kind == MirroredItemKind::Matrix && record.source_id == anchor.matrix_id)
                .ok_or("Omitted matrix anchor correspondence is missing")?.mirror_id.clone();
        }
        return super::layout::reflected(source, &target, mapping, false).map(Cow::Owned);
    }
    Err(format!("Logical matrix {id} is missing"))
}

pub(crate) fn order(doc: &ProjectDoc) -> Result<Vec<usize>, String> {
    fn visit(index: usize, doc: &ProjectDoc, visiting: &mut BTreeSet<usize>, done: &mut BTreeSet<usize>, result: &mut Vec<usize>) -> Result<(), String> {
        if done.contains(&index) { return Ok(()); }
        if !visiting.insert(index) { return Err(format!("Matrix anchor cycle at {}", doc.matrices[index].id)); }
        if let Some(anchor) = &doc.matrices[index].anchor
            && let Some(target) = doc.matrices.iter().position(|matrix| matrix.id == anchor.matrix_id) {
            visit(target, doc, visiting, done, result)?;
        }
        visiting.remove(&index);
        done.insert(index);
        result.push(index);
        Ok(())
    }
    let mut result = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    for index in 0..doc.matrices.len() { visit(index, doc, &mut visiting, &mut done, &mut result)?; }
    Ok(result)
}

pub(crate) fn input(matrix: &Matrix) -> LayoutInput {
    LayoutInput {
        column_layout: matrix.column_layout.clone(),
        row_layout: matrix.row_layout.clone(),
        cells: matrix.cells.iter().map(|cell| KeyLayout {
            row: cell.row as usize,
            column: cell.column as usize,
            shift: cell.shift.unwrap_or_default(),
            bend: cell.bend.unwrap_or_default(),
            offset: cell.offset.unwrap_or_default(),
            rotation: cell.rotation.unwrap_or_default(),
        }).collect(),
        origin: matrix.origin,
        rotation: matrix.rotation.unwrap_or_default(),
        mirror: matrix.mirror.unwrap_or(crate::model::Mirror::None),
        edge_gap: matrix.edge_gap.map_or(super::DEFAULT_EDGE_GAP_MM, |gap| gap.x.min(gap.y)),
        rows: matrix.rows as usize,
        columns: matrix.columns as usize,
        pitch: matrix.pitch,
    }
}

pub(crate) fn evaluate(matrix: &Matrix, doc: &ProjectDoc) -> Result<MatrixLayout, String> {
    resolve(matrix, doc, &mut BTreeSet::new())
}

fn resolve(matrix: &Matrix, doc: &ProjectDoc, visiting: &mut BTreeSet<String>) -> Result<MatrixLayout, String> {
    if !visiting.insert(matrix.id.clone()) {
        return Err(format!("Matrix anchor cycle at {}", matrix.id));
    }
    let context = if let Some(anchor) = &matrix.anchor {
        let target = logical_matrix(&anchor.matrix_id, doc)
            .map_err(|_| format!("Matrix {} anchor target {} is missing", matrix.id, anchor.matrix_id))?;
        let owner = doc.layouts.iter().find(|l| l.matrix_ids.contains(&matrix.id))
            .ok_or_else(|| format!("Anchored matrix {} must belong to a Layout", matrix.id))?;
        if !owner.matrix_ids.contains(&target.id) || target.board_id != matrix.board_id {
            return Err(format!("Matrix {} anchor must stay on the same board and Layout", matrix.id));
        }
        let key_id = super::member_id(&target.id, anchor.row, anchor.column);
        if doc.constraint_driving_part(&key_id)?.is_some() {
            return Err(format!("Matrix {} anchor key {} is constraint-driven", matrix.id, key_id));
        }
        let target_layout = resolve(&target, doc, visiting)?;
        let logical = target_layout.cells.iter().find(|cell| cell.row == anchor.row as usize && cell.column == anchor.column as usize)
            .ok_or_else(|| format!("Matrix {} anchor key {} is missing", matrix.id, key_id))?;
        let pose = doc.parts.iter().find(|part| part.id == key_id).map_or(logical.pose, |part| part.pose);
        RawAnchorContext::Key { pose, follow: anchor.follow, basis: follow_basis(matrix, doc) }
    } else {
        RawAnchorContext::World
    };
    let result = kernel::evaluate(&input(matrix), context).map_err(str::to_owned);
    visiting.remove(&matrix.id);
    result
}

fn follow_basis(matrix: &Matrix, doc: &ProjectDoc) -> FollowBasis {
    use crate::model::MirroredItemKind;
    for layout in &doc.layouts {
        if let Some(link) = &layout.mirror_link
            && link.correspondences.iter().any(|c| c.kind == MirroredItemKind::Matrix && c.mirror_id == matrix.id)
        {
            return FollowBasis::Reflected { translation: Vec2 { x: 2.0 * link.axis_x, y: 0.0 }, rotation: 0.0 };
        }
    }
    if let Some(hardware) = &doc.hardware {
        for pair in &hardware.pairs {
            if pair.halves.as_ref().is_some_and(|halves| halves.correspondences.iter().any(|c| c.kind == MirroredItemKind::Matrix && c.mirror_id == matrix.id)) {
                return FollowBasis::Reflected { translation: pair.mapping.at, rotation: pair.mapping.rotation };
            }
        }
    }
    FollowBasis::World
}

pub(crate) fn inverse_offset(frame: &kernel::ComposedFrame, delta: Vec2) -> Vec2 {
    Vec2 { x: delta.x * frame.axis_x.x + delta.y * frame.axis_x.y, y: delta.x * frame.axis_y.x + delta.y * frame.axis_y.y }
}

pub(crate) fn cell_pose(layout: &MatrixLayout, row: u32, column: u32) -> Result<Pose2, String> {
    layout.cells.iter().find(|c| c.row == row as usize && c.column == column as usize)
        .map(|c| c.pose).ok_or_else(|| "Matrix cell is out of bounds".into())
}
