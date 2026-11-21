//! `build_operation`: one accepted Inspector field edit becomes one existing edit operation.

use super::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue, PlacementParameter,
    PlacementValue, convert_offset_operation, placement_operation,
};
use boardstudio_model::model::{
    AggregateAnchor, EditOperation, Follow, HalfExceptionState, Matrix, MatrixAnchor, MatrixCell,
    MatrixSplayAffect, MatrixSplayChange, Mirror, MirroredItemKind, Vec2, convert_offset_available,
};

pub fn build_operation(
    matrix: &Matrix,
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    value: MatrixTransformValue,
    affect: MatrixSplayAffect,
) -> Result<EditOperation, String> {
    use MatrixTransformField as Field;
    use MatrixTransformValue as Value;
    match (fields, field, value) {
        (MatrixTransformFields::Matrix { .. }, Field::DeleteSelection, Value::Bool(_)) => {
            Ok(EditOperation::RemoveMatrix {
                id: matrix.id.clone(),
            })
        }
        (MatrixTransformFields::Row { row, .. }, Field::DeleteSelection, Value::Bool(_)) => {
            Ok(EditOperation::RemoveMatrixRow {
                matrix_id: matrix.id.clone(),
                row: *row,
            })
        }
        (MatrixTransformFields::Column { column, .. }, Field::DeleteSelection, Value::Bool(_)) => {
            Ok(EditOperation::RemoveMatrixColumn {
                matrix_id: matrix.id.clone(),
                column: *column,
            })
        }
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::DeleteSelection,
            Value::Bool(_),
        ) => Ok(EditOperation::RemoveMatrixCells {
            matrix_id: matrix.id.clone(),
            cells: vec![(*row, *column)],
        }),
        (MatrixTransformFields::Matrix { .. }, Field::MatrixDetach, Value::Bool(true)) => {
            Ok(EditOperation::DetachMatrixAnchor {
                matrix_id: matrix.id.clone(),
            })
        }
        (
            MatrixTransformFields::Matrix { .. },
            Field::MatrixDetachConfirm,
            Value::Text(local_matrix_id),
        ) => Ok(EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id: matrix.id.clone(),
            local_matrix_id,
        }),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixAnchorPick,
            Value::AnchorKey {
                matrix_id,
                row,
                column,
            },
        ) => Ok(EditOperation::SetMatrixAnchor {
            matrix_id: matrix.id.clone(),
            anchor: MatrixAnchor {
                matrix_id: matrix_id.clone(),
                row,
                column,
                follow: current.follow,
                aggregate: None,
            },
        }),
        (
            MatrixTransformFields::Matrix { anchor: None, .. },
            Field::MatrixAnchorPick,
            Value::AnchorKey { .. },
        ) => Err("This matrix has no anchor to retarget. Anchor it first.".into()),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixAnchorMethod,
            Value::AnchorMethod(method),
        ) => {
            let Some(aggregate) = current.aggregate.as_ref() else {
                return Err("Anchor method editing needs an aggregate anchor.".into());
            };
            Ok(EditOperation::SetMatrixAnchor {
                matrix_id: matrix.id.clone(),
                anchor: MatrixAnchor {
                    aggregate: Some(AggregateAnchor {
                        method,
                        parts: aggregate.parts.clone(),
                    }),
                    ..current.clone()
                },
            })
        }
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixAnchorParts,
            Value::AnchorParts(parts),
        ) => {
            let Some(aggregate) = current.aggregate.as_ref() else {
                return Err("Anchor part editing needs an aggregate anchor.".into());
            };
            if parts.is_empty() {
                return Err(format!(
                    "Matrix {} aggregate anchor needs at least one part",
                    matrix.id
                ));
            }
            Ok(EditOperation::SetMatrixAnchor {
                matrix_id: matrix.id.clone(),
                anchor: MatrixAnchor {
                    aggregate: Some(AggregateAnchor {
                        method: aggregate.method,
                        parts,
                    }),
                    ..current.clone()
                },
            })
        }
        (
            MatrixTransformFields::Matrix { .. },
            Field::MatrixAnchorMethod | Field::MatrixAnchorParts,
            _,
        ) => Err("Anchor method and part editing need an aggregate anchor.".into()),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixFollowX,
            Value::Bool(x),
        ) => Ok(EditOperation::SetMatrixAnchor {
            matrix_id: matrix.id.clone(),
            anchor: MatrixAnchor {
                follow: Follow {
                    x,
                    ..current.follow
                },
                ..current.clone()
            },
        }),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixFollowY,
            Value::Bool(y),
        ) => Ok(EditOperation::SetMatrixAnchor {
            matrix_id: matrix.id.clone(),
            anchor: MatrixAnchor {
                follow: Follow {
                    y,
                    ..current.follow
                },
                ..current.clone()
            },
        }),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(current),
                ..
            },
            Field::MatrixFollowRotation,
            Value::Bool(rotation),
        ) => Ok(EditOperation::SetMatrixAnchor {
            matrix_id: matrix.id.clone(),
            anchor: MatrixAnchor {
                follow: Follow {
                    rotation,
                    ..current.follow
                },
                ..current.clone()
            },
        }),
        (MatrixTransformFields::Matrix { .. }, Field::OriginX, Value::Number(x)) => {
            let mut next = matrix.clone();
            next.origin.x = x;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Matrix { .. }, Field::OriginY, Value::Number(y)) => {
            let mut next = matrix.clone();
            next.origin.y = y;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Matrix { .. }, Field::MatrixRotation, Value::Number(rotation)) => {
            let mut next = matrix.clone();
            next.rotation = Some(rotation);
            Ok(set_matrix(next))
        }
        (
            MatrixTransformFields::Matrix {
                mirror_y_locked, ..
            },
            Field::MatrixMirror,
            Value::Mirror(mirror),
        ) => {
            if *mirror_y_locked && mirror == Some(Mirror::Y) {
                return Err("Y-axis mirroring is unavailable for a linked layout.".into());
            }
            let mut next = matrix.clone();
            next.mirror = mirror;
            Ok(set_matrix(next))
        }
        (MatrixTransformFields::Row { row, offset, .. }, Field::RowOffsetX, Value::Number(x)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowOffset { row: *row },
                PlacementValue::Offset(Vec2 { x, y: offset.y }),
            ))
        }
        (MatrixTransformFields::Row { row, offset, .. }, Field::RowOffsetY, Value::Number(y)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowOffset { row: *row },
                PlacementValue::Offset(Vec2 { x: offset.x, y }),
            ))
        }
        (MatrixTransformFields::Row { row, .. }, Field::RowOffsetReset, Value::Offset(offset)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowOffset { row: *row },
                PlacementValue::Offset(offset),
            ))
        }
        (MatrixTransformFields::Row { row, shift, .. }, Field::RowShiftX, Value::Number(x)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowShift { row: *row },
                PlacementValue::Shift(Vec2 { x, y: shift.y }),
            ))
        }
        (MatrixTransformFields::Row { row, shift, .. }, Field::RowShiftY, Value::Number(y)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowShift { row: *row },
                PlacementValue::Shift(Vec2 { x: shift.x, y }),
            ))
        }
        (MatrixTransformFields::Row { row, .. }, Field::RowBend, Value::Number(bend)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowBend { row: *row },
                PlacementValue::Bend(bend),
            ))
        }
        (MatrixTransformFields::Row { row, .. }, Field::RowPadding, Value::Number(padding)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::RowPadding { row: *row },
                PlacementValue::Padding(padding),
            ))
        }
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetX,
            Value::Number(x),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(Vec2 { x, y: offset.y }),
        )),
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetY,
            Value::Number(y),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(Vec2 { x: offset.x, y }),
        )),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnOffsetReset,
            Value::Offset(offset),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(offset),
        )),
        (MatrixTransformFields::Column { column, .. }, Field::ColumnOffsetConvert, _) => {
            convert_offset_available(matrix, *column)?;
            Ok(convert_offset_operation(&matrix.id, *column))
        }
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnStagger,
            Value::Number(stagger),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::Stagger { column: *column },
            PlacementValue::Stagger(stagger),
        )),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnSpread,
            Value::Number(spread),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::ColumnSpread { column: *column },
            PlacementValue::Spread(spread),
        )),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnSplay,
            Value::Number(angle),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Angle { angle, affect },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginMode,
            Value::OriginMode(custom),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: custom.then_some(*splay_origin),
            },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginX,
            Value::Number(x),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 {
                    x,
                    y: splay_origin.y,
                }),
            },
        }),
        (
            MatrixTransformFields::Column {
                column,
                splay_origin,
                ..
            },
            Field::SplayOriginY,
            Value::Number(y),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 {
                    x: splay_origin.x,
                    y,
                }),
            },
        }),
        (
            MatrixTransformFields::Column { column, .. },
            Field::SplayOriginPoint,
            Value::Point(point),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::Origin { world: Some(point) },
        }),
        (
            MatrixTransformFields::Column { column, .. },
            Field::SplayOriginSwingPoint,
            Value::Point(point),
        ) => Ok(EditOperation::SetMatrixSplay {
            matrix_id: matrix.id.clone(),
            column: *column,
            change: MatrixSplayChange::OriginSwing { world: Some(point) },
        }),
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyOffsetX,
            Value::Number(x),
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            Vec2 { x, y: offset.y },
            None,
        )),
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyOffsetY,
            Value::Number(y),
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            Vec2 { x: offset.x, y },
            None,
        )),
        (
            MatrixTransformFields::Key {
                row, column, shift, ..
            },
            Field::KeyShiftX,
            Value::Number(x),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::KeyShift {
                row: *row,
                column: *column,
            },
            PlacementValue::Shift(Vec2 { x, y: shift.y }),
        )),
        (
            MatrixTransformFields::Key {
                row, column, shift, ..
            },
            Field::KeyShiftY,
            Value::Number(y),
        ) => Ok(placement_operation(
            &matrix.id,
            PlacementParameter::KeyShift {
                row: *row,
                column: *column,
            },
            PlacementValue::Shift(Vec2 { x: shift.x, y }),
        )),
        (MatrixTransformFields::Key { row, column, .. }, Field::KeyBend, Value::Number(bend)) => {
            Ok(placement_operation(
                &matrix.id,
                PlacementParameter::KeyBend {
                    row: *row,
                    column: *column,
                },
                PlacementValue::Bend(bend),
            ))
        }
        (
            MatrixTransformFields::Key {
                row,
                column,
                offset,
                ..
            },
            Field::KeyRotation,
            Value::Number(rotation),
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            *offset,
            Some(rotation),
        )),
        (
            MatrixTransformFields::Key {
                row,
                column,
                mirror_target,
                ..
            },
            Field::KeyAttached,
            Value::Attached(next_list),
        ) => Ok(set_cell_attached(
            matrix,
            *row,
            *column,
            &next_list,
            *mirror_target,
        )),
        (
            MatrixTransformFields::Key {
                row,
                column,
                mirror_target,
                ..
            },
            Field::KeyAssembly,
            Value::Text(definition_id),
        ) => Ok(set_cell_assembly(
            matrix,
            *row,
            *column,
            definition_id,
            *mirror_target,
        )),
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyAssembliesLocal,
            Value::Bool(local),
        ) => Ok(set_cell_assemblies_local(matrix, *row, *column, local)),
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyEnabled,
            Value::Bool(enabled),
        ) => Ok(set_cell_enabled(matrix, *row, *column, enabled)),
        (MatrixTransformFields::Key { half_exception, .. }, Field::KeyOmit, Value::Bool(omit)) => {
            let Some(exception) = half_exception else {
                return Err("The key is not a mirrored key with a linked counterpart.".into());
            };
            Ok(EditOperation::SetHalfException {
                pair_id: exception.pair_id.clone(),
                kind: MirroredItemKind::Part,
                mirror_id: exception.mirror_id.clone(),
                state: if omit {
                    HalfExceptionState::Omitted
                } else {
                    HalfExceptionState::Linked
                },
            })
        }
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyAddAnchoredMatrix,
            Value::Text(id),
        ) => {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                return Err("Matrix ID must use letters, digits, _ or -".into());
            }
            if *row >= matrix.rows || *column >= matrix.columns {
                return Err("The selected key no longer exists.".into());
            }
            Ok(EditOperation::AddAnchoredMatrix {
                id: id.clone(),
                anchor: MatrixAnchor {
                    matrix_id: matrix.id.clone(),
                    row: *row,
                    column: *column,
                    follow: Follow::default(),
                    aggregate: None,
                },
            })
        }
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyTransformReset,
            Value::CellTransform { offset, rotation },
        ) => Ok(set_cell_transform(
            matrix,
            *row,
            *column,
            offset,
            Some(rotation),
        )),
        _ => Err("This transform field no longer matches the selected matrix context.".into()),
    }
}

fn set_matrix(matrix: Matrix) -> EditOperation {
    EditOperation::SetMatrix {
        matrix,
        definitions: None,
    }
}

fn set_cell_enabled(matrix: &Matrix, row: u32, column: u32, enabled: bool) -> EditOperation {
    let mut next = matrix.clone();
    if let Some(cell) = next
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        cell.enabled = enabled;
    } else {
        next.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column,
            enabled,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
    }
    set_matrix(next)
}

/// Only a linked mirror target keeps an assembly edit local to its key, like the reference
/// Workbench; edits on the canonical half must keep propagating to the paired half.
fn set_cell_assembly(
    matrix: &Matrix,
    row: u32,
    column: u32,
    definition_id: String,
    mirror_target: bool,
) -> EditOperation {
    let mut next = matrix.clone();
    if let Some(cell) = next
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        let changed = cell.definition_id.as_deref() != Some(definition_id.as_str())
            || cell.variant.as_deref() != Some(definition_id.as_str());
        cell.definition_id = Some(definition_id.clone());
        cell.variant = Some(definition_id);
        if changed && mirror_target {
            cell.assemblies_local = Some(true);
        }
    } else {
        next.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column,
            enabled: true,
            definition_id: Some(definition_id.clone()),
            variant: Some(definition_id),
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: mirror_target.then_some(true),
        });
    }
    set_matrix(next)
}

fn set_cell_attached(
    matrix: &Matrix,
    row: u32,
    column: u32,
    attached: &[(String, String)],
    mirror_target: bool,
) -> EditOperation {
    let mut next = matrix.clone();
    if let Some(cell) = next
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        let assemblies = cell
            .assemblies
            .iter()
            .filter_map(|assembly| {
                attached
                    .iter()
                    .find(|(id, _)| *id == assembly.id)
                    .map(|(_, definition_id)| {
                        let mut assembly = assembly.clone();
                        assembly.definition_id = definition_id.clone();
                        assembly
                    })
            })
            .collect::<Vec<_>>();
        if mirror_target && assemblies != cell.assemblies {
            cell.assemblies_local = Some(true);
        }
        cell.assemblies = assemblies;
    }
    set_matrix(next)
}

fn set_cell_assemblies_local(matrix: &Matrix, row: u32, column: u32, local: bool) -> EditOperation {
    let mut next = matrix.clone();
    if let Some(cell) = next
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        cell.assemblies_local = Some(local);
    }
    set_matrix(next)
}

fn set_cell_transform(
    matrix: &Matrix,
    row: u32,
    column: u32,
    offset: Vec2,
    rotation: Option<f64>,
) -> EditOperation {
    let mut next = matrix.clone();
    let mut cell = next
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .cloned()
        .unwrap_or(MatrixCell {
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
            assemblies: Vec::new(),
            assemblies_local: None,
        });
    cell.offset = Some(offset);
    if let Some(rotation) = rotation {
        cell.rotation = Some(rotation);
    }
    if let Some(existing) = next
        .cells
        .iter_mut()
        .find(|existing| existing.row == row && existing.column == column)
    {
        *existing = cell;
    } else {
        next.cells.push(cell);
    }
    set_matrix(next)
}
