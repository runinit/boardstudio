//! Private, pure mapping from accepted matrix transform fields to existing edit operations.
use boardstudio_core::model::{
    EditOperation, Matrix, MatrixCell, MatrixSplayAffect, MatrixSplayChange, Mirror, Vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixTransformField {
    OriginX,
    OriginY,
    MatrixRotation,
    MatrixMirror,
    RowOffsetX,
    RowOffsetY,
    RowOffsetReset,
    ColumnOffsetX,
    ColumnOffsetY,
    ColumnOffsetReset,
    ColumnStagger,
    ColumnSplay,
    SplayOriginMode,
    SplayOriginX,
    SplayOriginY,
    SplayOriginPoint,
    KeyOffsetX,
    KeyOffsetY,
    KeyRotation,
    KeyTransformReset,
    KeyEnabled,
    KeyAssembly,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformValue {
    Number(f64),
    Mirror(Option<Mirror>),
    Offset(Vec2),
    CellTransform { offset: Vec2, rotation: f64 },
    OriginMode(bool),
    Point(Vec2),
    Bool(bool),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformFields {
    Matrix {
        origin: Vec2,
        rotation: f64,
        mirror: Option<Mirror>,
        mirror_y_locked: bool,
    },
    Row {
        row: u32,
        offset: Vec2,
    },
    Column {
        column: u32,
        offset: Vec2,
        stagger: f64,
        splay_angle: f64,
        splay_origin: Vec2,
        custom_origin: bool,
    },
    Key {
        row: u32,
        column: u32,
        enabled: bool,
        definition_id: String,
        choices: Vec<(String, String)>,
        offset: Vec2,
        rotation: f64,
    },
}

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
        (MatrixTransformFields::Row { row, offset }, Field::RowOffsetX, Value::Number(x)) => Ok(
            set_matrix_offset(matrix, true, *row, Vec2 { x, y: offset.y }),
        ),
        (MatrixTransformFields::Row { row, offset }, Field::RowOffsetY, Value::Number(y)) => Ok(
            set_matrix_offset(matrix, true, *row, Vec2 { x: offset.x, y }),
        ),
        (MatrixTransformFields::Row { row, .. }, Field::RowOffsetReset, Value::Offset(offset)) => {
            Ok(set_matrix_offset(matrix, true, *row, offset))
        }
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetX,
            Value::Number(x),
        ) => Ok(set_matrix_offset(
            matrix,
            false,
            *column,
            Vec2 { x, y: offset.y },
        )),
        (
            MatrixTransformFields::Column { column, offset, .. },
            Field::ColumnOffsetY,
            Value::Number(y),
        ) => Ok(set_matrix_offset(
            matrix,
            false,
            *column,
            Vec2 { x: offset.x, y },
        )),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnOffsetReset,
            Value::Offset(offset),
        ) => Ok(set_matrix_offset(matrix, false, *column, offset)),
        (
            MatrixTransformFields::Column { column, .. },
            Field::ColumnStagger,
            Value::Number(stagger),
        ) => {
            let mut next = matrix.clone();
            next.column_staggers = (0..matrix.columns)
                .map(|index| {
                    if index == *column {
                        stagger
                    } else {
                        matrix
                            .column_staggers
                            .get(index as usize)
                            .copied()
                            .unwrap_or(0.0)
                    }
                })
                .collect();
            Ok(set_matrix(next))
        }
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
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyAssembly,
            Value::Text(definition_id),
        ) => Ok(set_cell_assembly(matrix, *row, *column, definition_id)),
        (
            MatrixTransformFields::Key { row, column, .. },
            Field::KeyEnabled,
            Value::Bool(enabled),
        ) => Ok(set_cell_enabled(matrix, *row, *column, enabled)),
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

fn set_matrix_offset(matrix: &Matrix, row_axis: bool, index: u32, value: Vec2) -> EditOperation {
    let mut next = matrix.clone();
    let offsets = if row_axis {
        &mut next.row_offsets
    } else {
        &mut next.column_offsets
    };
    while offsets.len() <= index as usize {
        offsets.push(Vec2 { x: 0.0, y: 0.0 });
    }
    offsets[index as usize] = value;
    set_matrix(next)
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

fn set_cell_assembly(
    matrix: &Matrix,
    row: u32,
    column: u32,
    definition_id: String,
) -> EditOperation {
    let mut next = matrix.clone();
    if let Some(cell) = next
        .cells
        .iter_mut()
        .find(|cell| cell.row == row && cell.column == column)
    {
        cell.definition_id = Some(definition_id.clone());
        cell.variant = Some(definition_id);
    } else {
        next.cells.push(MatrixCell {
            row,
            column,
            enabled: true,
            definition_id: Some(definition_id.clone()),
            variant: Some(definition_id),
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{EditOperation, MatrixAssembly};

    fn matrix() -> Matrix {
        Matrix {
            id: "matrix-1".into(),
            name: Some("Fixture".into()),
            rows: 2,
            columns: 3,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 12.0, y: -4.0 },
            definition_id: "switch".into(),
            part_ids: vec!["p0".into()],
            board_id: Some("board-1".into()),
            mirror: None,
            rotation: Some(0.0),
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![Vec2 { x: 2.0, y: 3.0 }, Vec2 { x: 4.0, y: 5.0 }],
            column_offsets: vec![Vec2 { x: 1.0, y: 2.0 }],
            column_staggers: vec![0.25, 0.5, 0.75],
            column_splays: vec![0.0, 4.0, 8.0],
            column_origins: Vec::new(),
            cells: vec![MatrixCell {
                row: 1,
                column: 2,
                enabled: true,
                definition_id: Some("switch".into()),
                variant: Some("north".into()),
                offset: Some(Vec2 { x: 6.0, y: 7.0 }),
                rotation: Some(9.0),
                assemblies: vec![MatrixAssembly {
                    id: "led".into(),
                    definition_id: "led-def".into(),
                    offset: Vec2 { x: 1.0, y: 0.0 },
                    rotation: Some(90.0),
                    side: None,
                }],
                assemblies_local: Some(true),
            }],
        }
    }

    #[test]
    fn row_offset_changes_only_the_selected_row_and_preserves_other_matrix_data() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Row {
            row: 1,
            offset: Vec2 { x: 4.0, y: 5.0 },
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::RowOffsetX,
            MatrixTransformValue::Number(-2.5),
            MatrixSplayAffect::Following,
        )
        .expect("row offset is a supported accepted edit");
        let EditOperation::SetMatrix {
            matrix: next,
            definitions,
        } = operation
        else {
            panic!("row edits use SetMatrix");
        };
        assert_eq!(
            next.row_offsets,
            vec![Vec2 { x: 2.0, y: 3.0 }, Vec2 { x: -2.5, y: 5.0 }]
        );
        assert_eq!(next.cells, matrix.cells);
        assert_eq!(next.column_staggers, matrix.column_staggers);
        assert_eq!(next.origin, matrix.origin);
        assert!(definitions.is_none());
    }

    #[test]
    fn picking_column_splay_origin_is_one_atomic_origin_edit() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Column {
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            stagger: 0.5,
            splay_angle: 4.0,
            splay_origin: Vec2 { x: 10.0, y: 20.0 },
            custom_origin: true,
        };
        let point = Vec2 { x: 33.5, y: -12.0 };
        assert!(matches!(
            build_operation(
                &matrix,
                &fields,
                MatrixTransformField::SplayOriginPoint,
                MatrixTransformValue::Point(point),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: 1,
                change: MatrixSplayChange::Origin { world: Some(origin) },
            } if matrix_id == "matrix-1" && origin == point
        ));
    }

    #[test]
    fn key_reset_preserves_assembly_and_cell_identity() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyTransformReset,
            MatrixTransformValue::CellTransform {
                offset: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            MatrixSplayAffect::Following,
        )
        .expect("cell-local reset is a supported accepted edit");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell edits use SetMatrix");
        };
        assert_eq!(next.cells.len(), 1);
        let cell = &next.cells[0];
        assert_eq!((cell.row, cell.column), (1, 2));
        assert_eq!(cell.offset, Some(Vec2 { x: 0.0, y: 0.0 }));
        assert_eq!(cell.rotation, Some(0.0));
        assert_eq!(cell.definition_id.as_deref(), Some("switch"));
        assert_eq!(cell.variant.as_deref(), Some("north"));
        assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
        assert_eq!(cell.assemblies_local, Some(true));
    }

    #[test]
    fn key_enabled_toggle_preserves_cell_and_creates_missing_cell() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyEnabled,
            MatrixTransformValue::Bool(false),
            MatrixSplayAffect::Following,
        )
        .expect("enabled is a supported key edit");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell edits use SetMatrix");
        };
        assert_eq!(next.cells.len(), 1);
        assert!(!next.cells[0].enabled);
        assert_eq!(next.cells[0].offset, matrix.cells[0].offset);
        assert_eq!(next.cells[0].assemblies, matrix.cells[0].assemblies);

        let mut empty = matrix.clone();
        empty.cells.clear();
        let EditOperation::SetMatrix { matrix: next, .. } = build_operation(
            &empty,
            &fields,
            MatrixTransformField::KeyEnabled,
            MatrixTransformValue::Bool(false),
            MatrixSplayAffect::Following,
        )
        .unwrap() else {
            panic!("cell edits use SetMatrix");
        };
        assert_eq!(
            (
                next.cells[0].row,
                next.cells[0].column,
                next.cells[0].enabled
            ),
            (1, 2, false)
        );
    }

    #[test]
    fn key_assembly_sets_definition_and_variant_without_losing_cell_state() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let EditOperation::SetMatrix { matrix: next, .. } = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text("alt-switch".into()),
            MatrixSplayAffect::Following,
        )
        .unwrap() else {
            panic!("cell edits use SetMatrix");
        };
        let cell = &next.cells[0];
        assert_eq!(cell.definition_id.as_deref(), Some("alt-switch"));
        assert_eq!(cell.variant.as_deref(), Some("alt-switch"));
        assert_eq!(cell.offset, matrix.cells[0].offset);
        assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
    }

    #[test]
    fn empty_key_cell_edit_adds_a_semantic_cell_without_part_ids() {
        let mut matrix = matrix();
        matrix.cells.clear();
        matrix.part_ids = vec!["existing-primary".into()];
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 0,
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyOffsetX,
            MatrixTransformValue::Number(-1.25),
            MatrixSplayAffect::Following,
        )
        .expect("a semantic empty cell can receive a local transform");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell-local fields use SetMatrix");
        };
        assert_eq!(next.part_ids, vec!["existing-primary"]);
        assert_eq!(next.cells.len(), 1);
        assert_eq!((next.cells[0].row, next.cells[0].column), (0, 1));
        assert_eq!(next.cells[0].offset, Some(Vec2 { x: -1.25, y: 0.0 }));
        assert!(next.cells[0].enabled);
        assert!(next.cells[0].definition_id.is_none());
        assert!(next.cells[0].assemblies.is_empty());
    }

    #[test]
    fn cell_edit_preserves_order_and_untouched_cell_fields() {
        let mut matrix = matrix();
        matrix.cells.push(MatrixCell {
            row: 0,
            column: 1,
            enabled: false,
            definition_id: Some("alternate".into()),
            variant: Some("south".into()),
            offset: Some(Vec2 { x: -3.0, y: 2.0 }),
            rotation: Some(-4.0),
            assemblies: Vec::new(),
            assemblies_local: None,
        });
        let untouched = matrix.cells[1].clone();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyOffsetX,
            MatrixTransformValue::Number(-1.0),
            MatrixSplayAffect::Following,
        )
        .expect("a cell-local edit is supported");
        let EditOperation::SetMatrix { matrix: next, .. } = operation else {
            panic!("cell-local fields use SetMatrix");
        };
        assert_eq!(next.cells.len(), 2);
        assert_eq!(next.cells[0].offset, Some(Vec2 { x: -1.0, y: 7.0 }));
        assert_eq!(next.cells[0].assemblies, matrix.cells[0].assemblies);
        assert_eq!(next.cells[1], untouched);
    }

    #[test]
    fn column_splay_uses_existing_affect_and_origin_operations() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Column {
            column: 2,
            offset: Vec2 { x: 0.0, y: 0.0 },
            stagger: 0.0,
            splay_angle: 8.0,
            splay_origin: Vec2 { x: 11.0, y: 12.0 },
            custom_origin: false,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::ColumnSplay,
            MatrixTransformValue::Number(12.5),
            MatrixSplayAffect::Column,
        )
        .expect("splay uses the existing typed operation");
        assert!(matches!(
            operation,
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: 2,
                change: MatrixSplayChange::Angle {
                    angle: 12.5,
                    affect: MatrixSplayAffect::Column
                }
            } if matrix_id == "matrix-1"
        ));
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::SplayOriginMode,
            MatrixTransformValue::OriginMode(true),
            MatrixSplayAffect::Following,
        )
        .expect("custom origin uses the existing operation");
        assert!(matches!(
            operation,
            EditOperation::SetMatrixSplay {
                column: 2,
                change: MatrixSplayChange::Origin {
                    world: Some(Vec2 { x: 11.0, y: 12.0 })
                },
                ..
            }
        ));
    }

    #[test]
    fn linked_layout_rejects_y_mirror_at_the_operation_boundary() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: true,
        };
        let error = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::MatrixMirror,
            MatrixTransformValue::Mirror(Some(Mirror::Y)),
            MatrixSplayAffect::Following,
        )
        .expect_err("linked layout mirrors must retain the restriction");
        assert!(error.contains("linked layout"));
    }

    #[test]
    fn remaining_numeric_fields_map_to_their_existing_matrix_properties() {
        let matrix = matrix();
        let matrix_fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: false,
        };
        for (field, value, expected) in [
            (
                MatrixTransformField::OriginX,
                20.0,
                Vec2 { x: 20.0, y: -4.0 },
            ),
            (
                MatrixTransformField::OriginY,
                -8.0,
                Vec2 { x: 12.0, y: -8.0 },
            ),
        ] {
            let next = set_matrix_result(
                build_operation(
                    &matrix,
                    &matrix_fields,
                    field,
                    MatrixTransformValue::Number(value),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            );
            assert_eq!(next.origin, expected);
            assert_eq!(next.cells, matrix.cells);
        }
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &matrix_fields,
                MatrixTransformField::MatrixRotation,
                MatrixTransformValue::Number(33.0),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.rotation, Some(33.0));

        let row_fields = MatrixTransformFields::Row {
            row: 0,
            offset: Vec2 { x: 2.0, y: 3.0 },
        };
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &row_fields,
                MatrixTransformField::RowOffsetY,
                MatrixTransformValue::Number(-6.0),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.row_offsets[0], Vec2 { x: 2.0, y: -6.0 });
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &row_fields,
                MatrixTransformField::RowOffsetReset,
                MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.row_offsets[0], Vec2 { x: 0.0, y: 0.0 });

        let column_fields = MatrixTransformFields::Column {
            column: 2,
            offset: Vec2 { x: 1.0, y: 2.0 },
            stagger: 0.75,
            splay_angle: 8.0,
            splay_origin: Vec2 { x: 11.0, y: 12.0 },
            custom_origin: true,
        };
        for (field, value, expected) in [
            (
                MatrixTransformField::ColumnOffsetX,
                5.0,
                Vec2 { x: 5.0, y: 2.0 },
            ),
            (
                MatrixTransformField::ColumnOffsetY,
                -3.0,
                Vec2 { x: 1.0, y: -3.0 },
            ),
        ] {
            let next = set_matrix_result(
                build_operation(
                    &matrix,
                    &column_fields,
                    field,
                    MatrixTransformValue::Number(value),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            );
            assert_eq!(next.column_offsets[2], expected);
        }
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &column_fields,
                MatrixTransformField::ColumnOffsetReset,
                MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.column_offsets[2], Vec2 { x: 0.0, y: 0.0 });
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &column_fields,
                MatrixTransformField::ColumnStagger,
                MatrixTransformValue::Number(-1.5),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.column_staggers, vec![0.25, 0.5, -1.5]);

        for (field, value, expected) in [
            (
                MatrixTransformField::SplayOriginX,
                13.0,
                Vec2 { x: 13.0, y: 12.0 },
            ),
            (
                MatrixTransformField::SplayOriginY,
                -2.0,
                Vec2 { x: 11.0, y: -2.0 },
            ),
        ] {
            assert!(matches!(
                build_operation(
                    &matrix,
                    &column_fields,
                    field,
                    MatrixTransformValue::Number(value),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
                EditOperation::SetMatrixSplay {
                    change: MatrixSplayChange::Origin { world: Some(point) },
                    ..
                } if point == expected
            ));
        }

        let key_fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
        };
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &key_fields,
                MatrixTransformField::KeyOffsetY,
                MatrixTransformValue::Number(-9.0),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.cells[0].offset, Some(Vec2 { x: 6.0, y: -9.0 }));
        let next = set_matrix_result(
            build_operation(
                &matrix,
                &key_fields,
                MatrixTransformField::KeyRotation,
                MatrixTransformValue::Number(45.0),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(next.cells[0].rotation, Some(45.0));
    }

    fn set_matrix_result(operation: EditOperation) -> Matrix {
        let EditOperation::SetMatrix { matrix, .. } = operation else {
            panic!("this field maps to SetMatrix");
        };
        matrix
    }
}
