//! Private, pure mapping from accepted matrix transform fields to existing edit operations.
use boardstudio_core::model::{
    EditOperation, Follow, Matrix, MatrixAnchor, MatrixCell, MatrixColumnBasis,
    MatrixPlacementChange, MatrixScene, MatrixSplayAffect, MatrixSplayChange, Mirror,
    MirroredItemKind, PartDefinition, Pose2, ProjectDoc, Vec2, convert_offset_available,
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
    RowShiftX,
    RowShiftY,
    RowBend,
    RowPadding,
    ColumnOffsetX,
    ColumnOffsetY,
    ColumnOffsetReset,
    ColumnOffsetConvert,
    ColumnStagger,
    ColumnSpread,
    ColumnSplay,
    SplayOriginMode,
    SplayOriginX,
    SplayOriginY,
    SplayOriginPoint,
    SplayOriginSwingPoint,
    KeyOffsetX,
    KeyOffsetY,
    KeyShiftX,
    KeyShiftY,
    KeyBend,
    KeyRotation,
    KeyTransformReset,
    KeyEnabled,
    KeyAssembly,
    KeyAttached,
    KeyAssembliesLocal,
    KeyAddAnchoredMatrix,
    DeleteSelection,
    MatrixDetach,
    MatrixDetachConfirm,
    MatrixAnchorPick,
    MatrixFollowX,
    MatrixFollowY,
    MatrixFollowRotation,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformValue {
    Number(f64),
    Mirror(Option<Mirror>),
    Offset(Vec2),
    CellTransform {
        offset: Vec2,
        rotation: f64,
    },
    OriginMode(bool),
    Point(Vec2),
    AnchorKey {
        matrix_id: String,
        row: u32,
        column: u32,
    },
    Bool(bool),
    Text(String),
    Attached(Vec<(String, String)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixTransformFields {
    Matrix {
        origin: Vec2,
        rotation: f64,
        mirror: Option<Mirror>,
        mirror_y_locked: bool,
        /// False when the matrix is anchored without rotation Follow: Core
        /// would refuse the no-op edit, so the Inspector disables the field.
        rotation_followed: bool,
        anchor: Option<MatrixAnchor>,
    },
    Row {
        row: u32,
        offset: Vec2,
        shift: Vec2,
        bend: f64,
        padding: Option<f64>,
        pitch: f64,
    },
    Column {
        column: u32,
        offset: Vec2,
        stagger: f64,
        spread: Option<f64>,
        pitch: f64,
        splay_angle: f64,
        rotation: f64,
        splay_origin: Vec2,
        custom_origin: bool,
        /// Core's verdict on `ColumnOffsetToStagger` for this column, from the accepted
        /// matrix, so the Inspector shows Core's reason instead of re-deriving the rule.
        convert: Result<(), String>,
    },
    Key {
        row: u32,
        column: u32,
        enabled: bool,
        definition_id: String,
        choices: Vec<(String, String)>,
        assemblies: Vec<(String, String)>,
        component_choices: Vec<PartDefinition>,
        mirror_target: bool,
        assemblies_local: bool,
        offset: Vec2,
        rotation: f64,
        shift: Vec2,
        bend: f64,
    },
}

pub(crate) fn column_angle(splay: f64, rotation: f64, affect: &MatrixSplayAffect) -> f64 {
    match affect {
        MatrixSplayAffect::Column => rotation,
        MatrixSplayAffect::Following => splay,
    }
}

/// Produce the choices for one attached-component selector while preserving
/// only its own assembly snapshot from the accepted document.
pub fn attachment_component_choices(
    definitions: &[PartDefinition],
    current_id: &str,
) -> Vec<(String, String)> {
    definitions
        .iter()
        .filter(|definition| {
            definition
                .generator
                .as_ref()
                .is_none_or(|generator| generator.source != "infused-kim/nice_nano_pretty")
                && (definition.id == current_id || !is_assembly_snapshot(definition))
        })
        .map(|definition| (definition.id.clone(), definition.name.clone()))
        .collect()
}

fn is_assembly_snapshot(definition: &PartDefinition) -> bool {
    if definition.kicad_source.is_some() {
        return false;
    }
    const PREFIX: &str = "assembly-";
    const MARKER: &[u8] = b"/definition/";
    let id = definition.id.as_str();
    let bytes = id.as_bytes();
    let has_assembly_prefix = id
        .get(..PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX));
    if has_assembly_prefix
        && bytes
            .windows(MARKER.len())
            .enumerate()
            .any(|(position, candidate)| {
                position > PREFIX.len()
                    && candidate.eq_ignore_ascii_case(MARKER)
                    && id
                        .get(PREFIX.len()..position)
                        .is_some_and(js_regex_dot_matches)
            })
    {
        return true;
    }
    bytes
        .windows(MARKER.len())
        .enumerate()
        .any(|(position, candidate)| {
            position == 36
                && candidate.eq_ignore_ascii_case(MARKER)
                && id.get(..position).is_some_and(is_uuid)
        })
}

fn js_regex_dot_matches(text: &str) -> bool {
    !text
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

/// Whether an assembly edit may proceed with the accepted document's available definitions.
/// This policy is shared by the Inspector controller and native owner-level regressions.
pub fn component_edit_admitted_by_catalogue(
    catalog_built_for: Option<bool>,
    reversible: bool,
    field: MatrixTransformField,
    baseline: &MatrixTransformValue,
    value: &MatrixTransformValue,
    document_definition_ids: &[String],
) -> bool {
    if !matches!(
        field,
        MatrixTransformField::KeyAssembly | MatrixTransformField::KeyAttached
    ) || catalog_built_for == Some(reversible)
    {
        return true;
    }
    let requires_catalogue = match (field, baseline, value) {
        (
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text(current),
            MatrixTransformValue::Text(next),
        ) => current != next && !document_definition_ids.iter().any(|id| id == next),
        (
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(current),
            MatrixTransformValue::Attached(next),
        ) => next.iter().any(|(assembly_id, definition_id)| {
            current
                .iter()
                .find(|(current_id, _)| current_id == assembly_id)
                .is_none_or(|(_, current_definition)| current_definition != definition_id)
                && !document_definition_ids.iter().any(|id| id == definition_id)
        }),
        _ => false,
    };
    !requires_catalogue
}

/// Whether another user-triggered attempt can start a failed catalogue load for this construction.
pub fn catalogue_retry_due(
    requested: Option<bool>,
    loading: bool,
    catalog_built_for: Option<bool>,
    reversible: bool,
) -> bool {
    !loading && catalog_built_for != Some(reversible) && requested == Some(reversible)
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

/// The matrix parameter a placement edit names. Core owns the rules for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementParameter {
    /// Relative Y step of a column, inherited by every later column.
    Stagger { column: u32 },
    /// Independent X/Y adjustment of one column, in that column's frame.
    ColumnOffset { column: u32 },
    /// Independent X/Y adjustment of one row, in column 0's frame.
    RowOffset { row: u32 },
    /// Carried X/Y move of one row, in that row's running frame; later rows
    /// inherit it through Core's running frame.
    RowShift { row: u32 },
    /// Carried X/Y move of one key, in that key's running frame.
    KeyShift { row: u32, column: u32 },
    /// Carried turn of one row, in degrees; later rows inherit it.
    RowBend { row: u32 },
    /// Carried turn of one key, in degrees; later rows in its column inherit it.
    KeyBend { row: u32, column: u32 },
    /// Inherited X step of one column; absent means the matrix pitch.
    ColumnSpread { column: u32 },
    /// Y step of one row; absent means the matrix pitch.
    RowPadding { row: u32 },
}

/// An absolute parameter value in matrix coordinates. Spread and Padding are
/// resolved: the pitch fills an absent stored step, so a drag that starts from
/// a default writes an explicit step and never a stored copy of the pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlacementValue {
    Stagger(f64),
    Offset(Vec2),
    Shift(Vec2),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

/// The arrow keys a nudge can consume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowKey {
    Left,
    Right,
    Up,
    Down,
}

/// The frame a parameter is edited in: the column's stagger axis, its offset
/// axes, and the running-frame axes a carried Shift uses. A row Shift uses
/// column 0's running frame; a row offset uses column 0's offset frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneBasis {
    pub stagger_axis: Vec2,
    pub axis_x: Vec2,
    pub axis_y: Vec2,
    pub shift_axis_x: Vec2,
    pub shift_axis_y: Vec2,
    pub spread_axis: Option<Vec2>,
}

impl SceneBasis {
    pub fn from_column(basis: &MatrixColumnBasis) -> Self {
        Self {
            stagger_axis: basis.stagger_axis,
            axis_x: basis.axis_x,
            axis_y: basis.axis_y,
            shift_axis_x: basis.axis_x,
            shift_axis_y: basis.axis_y,
            spread_axis: basis.spread_axis,
        }
    }

    /// The basis for a parameter, or `None` when the scene has no such row or
    /// column. Row parameters read the column-0 key's frames; a key Shift reads
    /// its own key's running frame. Bend and Padding have no drag gesture, so
    /// their basis only feeds the exhaustiveness of the lookup.
    pub fn for_parameter(scene: &MatrixScene, parameter: PlacementParameter) -> Option<Self> {
        if let PlacementParameter::ColumnOffset { column }
        | PlacementParameter::RowOffset { row: column }
        | PlacementParameter::RowShift { row: column }
        | PlacementParameter::RowBend { row: column }
        | PlacementParameter::KeyShift { column, .. }
        | PlacementParameter::KeyBend { column, .. } = parameter
        {
            let (row, column) = match parameter {
                PlacementParameter::ColumnOffset { .. } => (0, column),
                PlacementParameter::KeyShift { row, column }
                | PlacementParameter::KeyBend { row, column } => (row, column),
                _ => (column, 0),
            };
            let cell = scene
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)?;
            return Some(Self {
                stagger_axis: cell.shift_frame.axis_y,
                axis_x: cell.offset_frame.axis_x,
                axis_y: cell.offset_frame.axis_y,
                shift_axis_x: cell.shift_frame.axis_x,
                shift_axis_y: cell.shift_frame.axis_y,
                spread_axis: None,
            });
        }
        let column = match parameter {
            PlacementParameter::Stagger { column }
            | PlacementParameter::ColumnOffset { column }
            | PlacementParameter::ColumnSpread { column } => column,
            PlacementParameter::RowOffset { .. }
            | PlacementParameter::RowShift { .. }
            | PlacementParameter::RowBend { .. }
            | PlacementParameter::RowPadding { .. } => 0,
            PlacementParameter::KeyShift { column, .. }
            | PlacementParameter::KeyBend { column, .. } => column,
        };
        scene
            .columns
            .iter()
            .find(|basis| basis.column == column)
            .map(Self::from_column)
    }
}

/// The only place a web intent becomes a `SetMatrixPlacement`.
pub fn placement_operation(
    matrix_id: &str,
    parameter: PlacementParameter,
    value: PlacementValue,
) -> EditOperation {
    let change = match (parameter, value) {
        (PlacementParameter::Stagger { column }, PlacementValue::Stagger(value)) => {
            MatrixPlacementChange::ColumnStagger { column, value }
        }
        (PlacementParameter::ColumnOffset { column }, PlacementValue::Offset(value)) => {
            MatrixPlacementChange::ColumnOffset { column, value }
        }
        (PlacementParameter::RowOffset { row }, PlacementValue::Offset(value)) => {
            MatrixPlacementChange::RowOffset { row, value }
        }
        (PlacementParameter::RowShift { row }, PlacementValue::Shift(value)) => {
            MatrixPlacementChange::RowShift { row, value }
        }
        (PlacementParameter::KeyShift { row, column }, PlacementValue::Shift(value)) => {
            MatrixPlacementChange::KeyShift { row, column, value }
        }
        (PlacementParameter::RowBend { row }, PlacementValue::Bend(value)) => {
            MatrixPlacementChange::RowBend { row, value }
        }
        (PlacementParameter::KeyBend { row, column }, PlacementValue::Bend(value)) => {
            MatrixPlacementChange::KeyBend { row, column, value }
        }
        (PlacementParameter::ColumnSpread { column }, PlacementValue::Spread(value)) => {
            MatrixPlacementChange::ColumnSpread {
                column,
                value: Some(value),
            }
        }
        (PlacementParameter::RowPadding { row }, PlacementValue::Padding(value)) => {
            MatrixPlacementChange::RowPadding {
                row,
                value: Some(value),
            }
        }
        (parameter, value) => panic!("{value:?} is not a value of {parameter:?}"),
    };
    EditOperation::SetMatrixPlacement {
        matrix_id: matrix_id.to_owned(),
        change,
    }
}

/// The one-shot conversion of a column's offset Y into stagger. Core refuses it when the
/// geometry would move; the Inspector asks `convert_offset_available` first.
pub fn convert_offset_operation(matrix_id: &str, column: u32) -> EditOperation {
    EditOperation::SetMatrixPlacement {
        matrix_id: matrix_id.to_owned(),
        change: MatrixPlacementChange::ColumnOffsetToStagger { column },
    }
}

/// An Inspector field value: the typed number replaces one component of the current value.
pub fn placement_value(
    parameter: PlacementParameter,
    current: PlacementValue,
    set: PlacementSet,
) -> PlacementValue {
    match (parameter, current, set) {
        (PlacementParameter::Stagger { .. }, _, PlacementSet::Stagger(value)) => {
            PlacementValue::Stagger(value)
        }
        (_, PlacementValue::Offset(offset), PlacementSet::OffsetX(x)) => {
            PlacementValue::Offset(Vec2 { x, y: offset.y })
        }
        (_, PlacementValue::Offset(offset), PlacementSet::OffsetY(y)) => {
            PlacementValue::Offset(Vec2 { x: offset.x, y })
        }
        (_, PlacementValue::Shift(shift), PlacementSet::ShiftX(x)) => {
            PlacementValue::Shift(Vec2 { x, y: shift.y })
        }
        (_, PlacementValue::Shift(shift), PlacementSet::ShiftY(y)) => {
            PlacementValue::Shift(Vec2 { x: shift.x, y })
        }
        (_, _, PlacementSet::Bend(value)) => PlacementValue::Bend(value),
        (_, _, PlacementSet::Spread(value)) => PlacementValue::Spread(value),
        (_, _, PlacementSet::Padding(value)) => PlacementValue::Padding(value),
        (_, _, set) => panic!("{set:?} does not fit {parameter:?}"),
    }
}

/// What an Inspector field edit sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlacementSet {
    Stagger(f64),
    OffsetX(f64),
    OffsetY(f64),
    ShiftX(f64),
    ShiftY(f64),
    Bend(f64),
    Spread(f64),
    Padding(f64),
}

/// A drag sample: the world delta since pointer-down, projected onto the scene axes and
/// added to the start value, then snapped to the grid step (`None` skips snapping, as Alt
/// does). Returns the absolute value, so it equals the Inspector's value for the same place.
/// A carried Shift projects onto the running frame Core applies it in, not the offset
/// frame an independent offset uses.
pub fn gesture_value(
    basis: &SceneBasis,
    parameter: PlacementParameter,
    start: PlacementValue,
    world_delta: Vec2,
    snap: Option<Vec2>,
) -> PlacementValue {
    match (parameter, start) {
        (PlacementParameter::Stagger { .. }, PlacementValue::Stagger(start)) => {
            let axis = basis.stagger_axis;
            let along = dot(world_delta, axis) / dot(axis, axis);
            PlacementValue::Stagger(snapped(start + along, snap.map(|step| step.y)))
        }
        (_, PlacementValue::Offset(start)) => {
            let local = solve(basis.axis_x, basis.axis_y, world_delta);
            PlacementValue::Offset(Vec2 {
                x: snapped(start.x + local.x, snap.map(|step| step.x)),
                y: snapped(start.y + local.y, snap.map(|step| step.y)),
            })
        }
        (_, PlacementValue::Shift(start)) => {
            let local = solve(basis.shift_axis_x, basis.shift_axis_y, world_delta);
            PlacementValue::Shift(Vec2 {
                x: snapped(start.x + local.x, snap.map(|step| step.x)),
                y: snapped(start.y + local.y, snap.map(|step| step.y)),
            })
        }
        (_, PlacementValue::Spread(start)) => {
            let axis = basis.spread_axis.unwrap_or(basis.axis_x);
            let along = dot(world_delta, axis) / dot(axis, axis);
            PlacementValue::Spread(snapped(start + along, snap.map(|step| step.x)))
        }
        // Bend and Padding have no drag gesture: Bend turns the running frame
        // rather than translating along an axis, and Padding steps along the
        // pre-row frame no handle exposes. Both are Inspector-only until a
        // gesture names their frame.
        (_, PlacementValue::Bend(_)) => {
            panic!("a carried Bend has no drag gesture; use the Inspector field")
        }
        (_, PlacementValue::Padding(_)) => {
            panic!("row Padding has no drag gesture; use the Inspector field")
        }
        (parameter, start) => panic!("{start:?} is not a value of {parameter:?}"),
    }
}

/// A keyboard step (0.1 mm, Shift 1 mm), exact and not grid-snapped. Stagger consumes the
/// arrow pair closest to the stagger axis on screen and ignores the perpendicular pair;
/// offsets step in screen directions converted through the frame.
pub fn nudge_value(
    basis: &SceneBasis,
    parameter: PlacementParameter,
    current: PlacementValue,
    key: ArrowKey,
    big: bool,
) -> Option<PlacementValue> {
    let step = if big { 1.0 } else { 0.1 };
    let delta = match key {
        ArrowKey::Left => Vec2 { x: -step, y: 0.0 },
        ArrowKey::Right => Vec2 { x: step, y: 0.0 },
        ArrowKey::Up => Vec2 { x: 0.0, y: step },
        ArrowKey::Down => Vec2 { x: 0.0, y: -step },
    };
    if matches!(parameter, PlacementParameter::Stagger { .. }) {
        let axis = basis.stagger_axis;
        let vertical = axis.y.abs() >= axis.x.abs();
        if vertical != matches!(key, ArrowKey::Up | ArrowKey::Down) {
            return None;
        }
    }
    if matches!(parameter, PlacementParameter::ColumnSpread { .. }) {
        // A Spread nudge consumes only the arrow pair along the step: a
        // perpendicular press is not consumed, so it never submits a no-op.
        let axis = basis.spread_axis.unwrap_or(basis.axis_x);
        let horizontal = axis.x.abs() >= axis.y.abs();
        if horizontal != matches!(key, ArrowKey::Left | ArrowKey::Right) {
            return None;
        }
    }
    Some(gesture_value(basis, parameter, current, delta, None))
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// Coordinates of `delta` in the frame `(x_axis, y_axis)`: `delta = a * x_axis + b * y_axis`.
fn solve(x_axis: Vec2, y_axis: Vec2, delta: Vec2) -> Vec2 {
    let determinant = x_axis.x * y_axis.y - x_axis.y * y_axis.x;
    if determinant.abs() < f64::EPSILON {
        return Vec2 { x: 0.0, y: 0.0 };
    }
    Vec2 {
        x: (delta.x * y_axis.y - delta.y * y_axis.x) / determinant,
        y: (x_axis.x * delta.y - x_axis.y * delta.x) / determinant,
    }
}

/// Rounds to the nearest multiple of `step`; no step, or a non-positive one, leaves the value.
fn snapped(value: f64, step: Option<f64>) -> f64 {
    match step {
        Some(step) if step > 0.0 => (value / step).round() * step,
        _ => value,
    }
}

fn set_matrix(matrix: Matrix) -> EditOperation {
    EditOperation::SetMatrix {
        matrix,
        definitions: None,
    }
}

/// Where a matrix drag's Follow permission is evaluated: the matrix's own
/// world axes, or the source-world axes transported through the pair map for
/// a linked partner. Mirrors the kernel's anchor evaluation basis from the
/// accepted document; Core remains the placement authority and re-validates
/// the solved Origin on commit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatrixDragTransport {
    World,
    Reflected { translation: Vec2, rotation: f64 },
}

/// The Follow basis for dragging a matrix, derived from the accepted document.
pub fn matrix_drag_transport(document: &ProjectDoc, matrix_id: &str) -> MatrixDragTransport {
    for layout in &document.layouts {
        if let Some(link) = &layout.mirror_link
            && link.correspondences.iter().any(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == matrix_id
            })
        {
            return MatrixDragTransport::Reflected {
                translation: Vec2 {
                    x: 2.0 * link.axis_x,
                    y: 0.0,
                },
                rotation: 0.0,
            };
        }
    }
    if let Some(hardware) = &document.hardware {
        for pair in &hardware.pairs {
            if pair.halves.as_ref().is_some_and(|halves| {
                halves.correspondences.iter().any(|record| {
                    record.kind == MirroredItemKind::Matrix && record.mirror_id == matrix_id
                })
            }) {
                return MatrixDragTransport::Reflected {
                    translation: pair.mapping.at,
                    rotation: pair.mapping.rotation,
                };
            }
        }
    }
    MatrixDragTransport::World
}

/// The anchor context a matrix drag inverse-solves through: the anchor key's
/// final world pose, the Follow subset and the transported basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MatrixDragAnchor {
    pub key_pose: Pose2,
    pub follow: Follow,
    pub transport: MatrixDragTransport,
}

/// The anchor key's final world pose from the accepted scene, or `None` when
/// the anchor target is not laid out and the drag cannot start.
pub fn matrix_drag_anchor_pose(scenes: &[MatrixScene], anchor: &MatrixAnchor) -> Option<Pose2> {
    scenes
        .iter()
        .find(|scene| scene.matrix_id == anchor.matrix_id)
        .and_then(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| cell.row == anchor.row && cell.column == anchor.column)
        })
        .map(|cell| cell.pose)
}

fn transport_delta_to_source(transport: MatrixDragTransport, delta: Vec2) -> Vec2 {
    match transport {
        MatrixDragTransport::World => delta,
        MatrixDragTransport::Reflected { rotation, .. } => {
            let (sin, cos) = rotation.to_radians().sin_cos();
            Vec2 {
                x: -(delta.x * cos + delta.y * sin),
                y: -delta.x * sin + delta.y * cos,
            }
        }
    }
}

fn transport_delta_from_source(transport: MatrixDragTransport, delta: Vec2) -> Vec2 {
    match transport {
        MatrixDragTransport::World => delta,
        MatrixDragTransport::Reflected { rotation, .. } => {
            let (sin, cos) = rotation.to_radians().sin_cos();
            Vec2 {
                x: -delta.x * cos - delta.y * sin,
                y: -delta.x * sin + delta.y * cos,
            }
        }
    }
}

fn zero_unfollowed(follow: &Follow, point: Vec2) -> Vec2 {
    Vec2 {
        x: if follow.x { point.x } else { 0.0 },
        y: if follow.y { point.y } else { 0.0 },
    }
}

/// Solve the dragged matrix's new Origin from a world drag delta. Unanchored
/// matrices move 1:1; anchored matrices inverse-solve through the anchor
/// frame with the delta constrained to the Follow directions, so an
/// unfollowed component never moves the frame. Components Follow discards
/// keep their authored values, so a drag outside every Follow direction is a
/// no-op that commits nothing.
pub fn solve_matrix_origin(
    matrix: &Matrix,
    anchor: Option<MatrixDragAnchor>,
    world_delta: Vec2,
) -> Vec2 {
    let constrained = match anchor {
        None => world_delta,
        Some(anchor) => transport_delta_from_source(
            anchor.transport,
            zero_unfollowed(
                &anchor.follow,
                transport_delta_to_source(anchor.transport, world_delta),
            ),
        ),
    };
    // Through the inverse of the anchor frame for anchored matrices; the
    // identity for unanchored ones, whose Origin is already in world axes.
    let (sin, cos) = anchor
        .map(|anchor| anchor.key_pose.rotation.to_radians().sin_cos())
        .unwrap_or((0.0, 1.0));
    Vec2 {
        x: matrix.origin.x + constrained.x * cos + constrained.y * sin,
        y: matrix.origin.y - constrained.x * sin + constrained.y * cos,
    }
}

/// Solve the dragged matrix's new stored rotation, or `None` when rotation is
/// not followed and the Rotate gesture must stay disabled.
pub fn solve_matrix_rotation(
    rotation: f64,
    follow_rotation: bool,
    delta_degrees: f64,
) -> Option<f64> {
    follow_rotation.then_some(rotation + delta_degrees)
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        EditOperation, KicadSource, MatrixAssembly, PartGenerator, PartKind,
    };

    fn component_definition(
        id: String,
        generator_source: Option<&str>,
        kicad_source: bool,
    ) -> PartDefinition {
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.clone(),
            name: format!("Label {id}"),
            kind: PartKind::Passive,
            keycap: None,
            envelope_source: None,
            kicad_source: kicad_source.then(|| KicadSource {
                format_version: 1,
                source: "fixture".into(),
            }),
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            models: None,
            generator: generator_source.map(|source| PartGenerator {
                source: source.into(),
                version: "1".into(),
                parameters: Default::default(),
            }),
            mechanical_profile: None,
        }
    }

    #[test]
    fn attached_component_choices_keep_only_this_members_snapshot_and_preserve_order() {
        let current = "assembly-preset-mx-hotswap-south-left-keys-0/definition/diode".to_owned();
        let mut definitions = (0..30)
            .map(|index| {
                let id = if index == 12 {
                    "assembly-imported-board/definition/diode".into()
                } else {
                    format!("catalogue-{index:02}")
                };
                component_definition(id, None, index == 12)
            })
            .collect::<Vec<_>>();
        let unrelated = (0..7)
            .map(|index| {
                component_definition(
                    format!("assembly-placement-{index}/definition/switch"),
                    None,
                    false,
                )
            })
            .collect::<Vec<_>>();
        definitions.extend(unrelated);
        definitions.push(component_definition(current.clone(), None, false));
        definitions.extend(
            (30..51)
                .map(|index| component_definition(format!("catalogue-{index:02}"), None, false)),
        );

        let choices = attachment_component_choices(&definitions, &current);

        let mut expected = definitions[..30]
            .iter()
            .map(|definition| (definition.id.clone(), definition.name.clone()))
            .collect::<Vec<_>>();
        let current_definition = definitions
            .iter()
            .find(|definition| definition.id == current)
            .unwrap();
        expected.push((current.clone(), current_definition.name.clone()));
        expected.extend(
            definitions[38..]
                .iter()
                .map(|definition| (definition.id.clone(), definition.name.clone())),
        );
        assert_eq!(choices, expected);
        assert_eq!(choices.len(), 52);
        assert!(choices.iter().any(|(id, _)| id == &current));
        assert!(
            choices
                .iter()
                .any(|(id, _)| { id == "assembly-imported-board/definition/diode" })
        );
        assert!(
            !choices
                .iter()
                .any(|(id, _)| id.starts_with("assembly-placement-"))
        );

        let retired = component_definition(
            "retired-nice-nano".into(),
            Some("infused-kim/nice_nano_pretty"),
            false,
        );
        assert!(attachment_component_choices(&[retired], "retired-nice-nano").is_empty());
    }

    #[test]
    fn failed_catalogue_does_not_block_removing_a_document_owned_attachment() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            assemblies: vec![("led".into(), "led-def".into())],
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
        };
        assert!(component_edit_admitted_by_catalogue(
            None,
            false,
            MatrixTransformField::KeyAttached,
            &MatrixTransformValue::Attached(vec![("led".into(), "led-def".into())]),
            &MatrixTransformValue::Attached(Vec::new()),
            &["led-def".into()],
        ));
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(Vec::new()),
            MatrixSplayAffect::Following,
        )
        .expect("removal uses the existing accepted edit operation");
        assert!(matches!(
            operation,
            EditOperation::SetMatrix { matrix, .. } if matrix.cells[0].assemblies.is_empty()
        ));
    }

    #[test]
    fn failed_catalogue_can_be_retried_for_the_same_construction() {
        let reversible = false;
        let mut requested = None;
        let mut loading = false;
        let mut built_for = None;
        assert!(!catalogue_retry_due(
            requested, loading, built_for, reversible
        ));

        // Initial effect request, followed by a failed load.
        requested = Some(reversible);
        loading = true;
        assert!(!catalogue_retry_due(
            requested, loading, built_for, reversible
        ));
        loading = false;
        assert!(catalogue_retry_due(
            requested, loading, built_for, reversible
        ));

        // A user assembly action clears the failed request; the effect starts it again.
        if catalogue_retry_due(requested, loading, built_for, reversible) {
            requested = None;
        }
        assert_eq!(requested, None);
        assert!(!catalogue_retry_due(
            requested, loading, built_for, reversible
        ));
        requested = Some(reversible);
        loading = true;
        assert!(!catalogue_retry_due(
            requested, loading, built_for, reversible
        ));

        // A successful retry tags the construction as ready and prevents another retry.
        loading = false;
        built_for = Some(reversible);
        assert!(!catalogue_retry_due(
            requested, loading, built_for, reversible
        ));
    }

    #[test]
    fn assembly_edits_requiring_a_catalogue_wait_for_the_matching_construction() {
        let baseline = MatrixTransformValue::Text("switch".into());
        let next = MatrixTransformValue::Text("bundled-switch".into());
        assert!(!component_edit_admitted_by_catalogue(
            None,
            false,
            MatrixTransformField::KeyAssembly,
            &baseline,
            &next,
            &["switch".into()],
        ));
        assert!(component_edit_admitted_by_catalogue(
            Some(false),
            false,
            MatrixTransformField::KeyAssembly,
            &baseline,
            &next,
            &["switch".into()],
        ));
    }

    #[test]
    fn document_owned_assembly_changes_and_replacements_do_not_need_catalogue() {
        assert!(component_edit_admitted_by_catalogue(
            None,
            true,
            MatrixTransformField::KeyAssembly,
            &MatrixTransformValue::Text("switch".into()),
            &MatrixTransformValue::Text("alternate-switch".into()),
            &["switch".into(), "alternate-switch".into()],
        ));
        assert!(component_edit_admitted_by_catalogue(
            None,
            true,
            MatrixTransformField::KeyAttached,
            &MatrixTransformValue::Attached(vec![("led".into(), "led-original".into())]),
            &MatrixTransformValue::Attached(vec![("led".into(), "led-replacement".into())]),
            &["led-original".into(), "led-replacement".into()],
        ));
    }

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
            row_layout: vec![
                boardstudio_core::model::RowLayout {
                    offset: Vec2 { x: 2.0, y: 3.0 },
                    ..Default::default()
                },
                boardstudio_core::model::RowLayout {
                    offset: Vec2 { x: 4.0, y: 5.0 },
                    ..Default::default()
                },
            ],
            column_layout: vec![
                boardstudio_core::model::ColumnLayout {
                    offset: Vec2 { x: 1.0, y: 2.0 },
                    stagger: 0.25,
                    ..Default::default()
                },
                boardstudio_core::model::ColumnLayout {
                    stagger: 0.5,
                    splay: 4.0,
                    ..Default::default()
                },
                boardstudio_core::model::ColumnLayout {
                    stagger: 0.75,
                    splay: 8.0,
                    ..Default::default()
                },
            ],
            anchor: None,
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
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
    fn convert_offset_builds_the_one_shot_placement_and_follows_core_availability() {
        let mut matrix = matrix();
        let fields = |column| MatrixTransformFields::Column {
            column,
            offset: Vec2 { x: 0.0, y: -9.525 },
            stagger: 0.0,
            splay_angle: 0.0,
            rotation: 0.0,
            splay_origin: Vec2::default(),
            custom_origin: false,
            convert: Ok(()),
            spread: None,
            pitch: 19.05,
        };
        let build = |matrix: &Matrix| {
            build_operation(
                matrix,
                &fields(0),
                MatrixTransformField::ColumnOffsetConvert,
                MatrixTransformValue::Number(0.0),
                MatrixSplayAffect::Following,
            )
        };
        assert_eq!(
            placement_result(build(&matrix).unwrap()),
            MatrixPlacementChange::ColumnOffsetToStagger { column: 0 }
        );
        // A splayed column with a default origin is refused with Core's own reason.
        {
            let values = vec![12.0];
            let count = values.len().max(matrix.column_layout.len());
            matrix.column_layout.resize_with(count, Default::default);
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        assert_eq!(
            build(&matrix),
            Err(convert_offset_available(&matrix, 0).unwrap_err())
        );
    }

    #[test]
    fn bend_spread_and_padding_map_to_named_placement_edits() {
        let matrix = matrix();
        let row_fields = MatrixTransformFields::Row {
            row: 1,
            offset: Vec2 { x: 4.0, y: 5.0 },
            shift: Vec2::default(),
            bend: 2.0,
            padding: None,
            pitch: 19.05,
        };
        assert_eq!(
            placement_result(
                build_operation(
                    &matrix,
                    &row_fields,
                    MatrixTransformField::RowBend,
                    MatrixTransformValue::Number(7.5),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            ),
            MatrixPlacementChange::RowBend { row: 1, value: 7.5 }
        );
        assert_eq!(
            placement_result(
                build_operation(
                    &matrix,
                    &row_fields,
                    MatrixTransformField::RowPadding,
                    MatrixTransformValue::Number(24.0),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            ),
            MatrixPlacementChange::RowPadding {
                row: 1,
                value: Some(24.0),
            }
        );
        let key_fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
        };
        assert_eq!(
            placement_result(
                build_operation(
                    &matrix,
                    &key_fields,
                    MatrixTransformField::KeyBend,
                    MatrixTransformValue::Number(-3.0),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            ),
            MatrixPlacementChange::KeyBend {
                row: 1,
                column: 2,
                value: -3.0,
            }
        );
        let column_fields = MatrixTransformFields::Column {
            column: 1,
            offset: Vec2::default(),
            stagger: 0.5,
            spread: None,
            pitch: 19.05,
            splay_angle: 0.0,
            rotation: 0.0,
            splay_origin: Vec2::default(),
            custom_origin: false,
            convert: Ok(()),
        };
        assert_eq!(
            placement_result(
                build_operation(
                    &matrix,
                    &column_fields,
                    MatrixTransformField::ColumnSpread,
                    MatrixTransformValue::Number(24.0),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            ),
            MatrixPlacementChange::ColumnSpread {
                column: 1,
                value: Some(24.0),
            }
        );
        // Inspector sets compose per component against the resolved current value.
        assert_eq!(
            placement_value(
                PlacementParameter::RowBend { row: 1 },
                PlacementValue::Bend(2.0),
                PlacementSet::Bend(7.5),
            ),
            PlacementValue::Bend(7.5)
        );
        assert_eq!(
            placement_value(
                PlacementParameter::ColumnSpread { column: 1 },
                PlacementValue::Spread(19.05),
                PlacementSet::Spread(24.0),
            ),
            PlacementValue::Spread(24.0)
        );
        assert_eq!(
            placement_value(
                PlacementParameter::RowPadding { row: 1 },
                PlacementValue::Padding(19.05),
                PlacementSet::Padding(24.0),
            ),
            PlacementValue::Padding(24.0)
        );
    }

    #[test]
    fn row_offset_changes_only_the_selected_row_and_preserves_other_matrix_data() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Row {
            row: 1,
            offset: Vec2 { x: 4.0, y: 5.0 },
            shift: Vec2::default(),
            bend: 0.0,
            padding: None,
            pitch: 19.05,
        };
        let operation = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::RowOffsetX,
            MatrixTransformValue::Number(-2.5),
            MatrixSplayAffect::Following,
        )
        .expect("row offset is a supported accepted edit");
        assert_eq!(
            operation,
            EditOperation::SetMatrixPlacement {
                matrix_id: matrix.id.clone(),
                change: MatrixPlacementChange::RowOffset {
                    row: 1,
                    value: Vec2 { x: -2.5, y: 5.0 },
                },
            },
            "a row edit sends the absolute row offset and nothing else"
        );
    }

    #[test]
    fn picking_column_splay_origin_is_one_atomic_origin_edit() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Column {
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            stagger: 0.5,
            splay_angle: 4.0,
            rotation: 0.0,
            splay_origin: Vec2 { x: 10.0, y: 20.0 },
            custom_origin: true,
            convert: Ok(()),
            spread: None,
            pitch: 19.05,
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
    fn ctrl_click_pick_writes_an_uncompensated_swing() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Column {
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            stagger: 0.5,
            splay_angle: 4.0,
            rotation: 0.0,
            splay_origin: Vec2 { x: 10.0, y: 20.0 },
            custom_origin: true,
            convert: Ok(()),
            spread: None,
            pitch: 19.05,
        };
        let point = Vec2 { x: 33.5, y: -12.0 };
        assert!(matches!(
            build_operation(
                &matrix,
                &fields,
                MatrixTransformField::SplayOriginSwingPoint,
                MatrixTransformValue::Point(point),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: 1,
                change: MatrixSplayChange::OriginSwing { world: Some(origin) },
            } if matrix_id == "matrix-1" && origin == point
        ));
    }

    #[test]
    fn detach_maps_to_one_atomic_core_intent() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: false,
            rotation_followed: true,
            anchor: None,
        };
        assert!(matches!(
            build_operation(
                &matrix,
                &fields,
                MatrixTransformField::MatrixDetach,
                MatrixTransformValue::Bool(true),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
            EditOperation::DetachMatrixAnchor { matrix_id }
                if matrix_id == "matrix-1"
        ));
    }

    #[test]
    fn confirmed_detach_carries_the_named_local_exception() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: false,
            rotation_followed: true,
            anchor: None,
        };
        assert!(matches!(
            build_operation(
                &matrix,
                &fields,
                MatrixTransformField::MatrixDetachConfirm,
                MatrixTransformValue::Text("partner-mirror".into()),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
            EditOperation::DetachMatrixAnchorConfirmed {
                matrix_id,
                local_matrix_id,
            } if matrix_id == "matrix-1" && local_matrix_id == "partner-mirror"
        ));
    }

    #[test]
    fn follow_toggles_rewrite_only_their_own_component() {
        let matrix = matrix();
        let anchor = MatrixAnchor {
            matrix_id: "other".into(),
            row: 0,
            column: 1,
            follow: Follow {
                x: true,
                y: true,
                rotation: true,
            },
            aggregate: None,
        };
        let fields = MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: 0.0,
            mirror: None,
            mirror_y_locked: false,
            rotation_followed: true,
            anchor: Some(anchor),
        };
        for (field, expected) in [
            (MatrixTransformField::MatrixFollowX, (false, true, true)),
            (MatrixTransformField::MatrixFollowY, (true, false, true)),
            (
                MatrixTransformField::MatrixFollowRotation,
                (true, true, false),
            ),
        ] {
            let EditOperation::SetMatrixAnchor { matrix_id, anchor } = build_operation(
                &matrix,
                &fields,
                field,
                MatrixTransformValue::Bool(false),
                MatrixSplayAffect::Following,
            )
            .unwrap() else {
                panic!("a Follow toggle reaches SetMatrixAnchor")
            };
            assert_eq!(matrix_id, "matrix-1");
            assert_eq!(anchor.matrix_id, "other");
            assert_eq!((anchor.row, anchor.column), (0, 1));
            assert_eq!(
                (anchor.follow.x, anchor.follow.y, anchor.follow.rotation),
                expected,
                "only the toggled component changes"
            );
        }
    }

    #[test]
    fn key_reset_preserves_assembly_and_cell_identity() {
        let matrix = matrix();
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
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
    fn attached_components_replace_and_remove_by_id() {
        let mut matrix = matrix();
        matrix.cells[0].assemblies = vec![
            MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode-a".into(),
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: None,
                side: None,
            },
            MatrixAssembly {
                id: "led".into(),
                definition_id: "led-a".into(),
                offset: Vec2 { x: 3.0, y: 4.0 },
                rotation: None,
                side: None,
            },
        ];
        let fields = MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
        };
        let EditOperation::SetMatrix { matrix: next, .. } = build_operation(
            &matrix,
            &fields,
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(vec![("diode".into(), "diode-b".into())]),
            MatrixSplayAffect::Following,
        )
        .unwrap() else {
            panic!("cell edits use SetMatrix");
        };
        assert_eq!(next.cells[0].assemblies.len(), 1);
        assert_eq!(next.cells[0].assemblies[0].id, "diode");
        assert_eq!(next.cells[0].assemblies[0].definition_id, "diode-b");
        assert_eq!(next.cells[0].assemblies[0].offset, Vec2 { x: 1.0, y: 2.0 });
    }

    fn paired_key_fields(mirror_target: bool, assemblies_local: bool) -> MatrixTransformFields {
        MatrixTransformFields::Key {
            enabled: true,
            definition_id: "switch".into(),
            choices: Vec::new(),
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target,
            assemblies_local,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
        }
    }

    fn edited_cell(
        matrix: &Matrix,
        fields: &MatrixTransformFields,
        field: MatrixTransformField,
        value: MatrixTransformValue,
    ) -> MatrixCell {
        set_matrix_result(
            build_operation(matrix, fields, field, value, MatrixSplayAffect::Following).unwrap(),
        )
        .cells
        .remove(0)
    }

    #[test]
    fn assembly_edits_stay_shared_on_the_canonical_half() {
        let mut matrix = matrix();
        matrix.cells[0].assemblies_local = None;
        let fields = paired_key_fields(false, false);
        let attached = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(vec![("led".into(), "led-b".into())]),
        );
        assert_eq!(attached.assemblies[0].definition_id, "led-b");
        assert_eq!(attached.assemblies_local, None);
        let assembly = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text("alt-switch".into()),
        );
        assert_eq!(assembly.assemblies_local, None);
    }

    #[test]
    fn assembly_edits_make_a_linked_target_key_local() {
        let mut matrix = matrix();
        matrix.cells[0].assemblies_local = None;
        let fields = paired_key_fields(true, false);
        let attached = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(vec![("led".into(), "led-b".into())]),
        );
        assert_eq!(attached.assemblies_local, Some(true));
        let assembly = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text("alt-switch".into()),
        );
        assert_eq!(assembly.assemblies_local, Some(true));

        matrix.cells.clear();
        let created = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text("alt-switch".into()),
        );
        assert_eq!(created.assemblies_local, Some(true));
    }

    #[test]
    fn unchanged_assembly_does_not_localize_a_linked_target_key() {
        let mut matrix = matrix();
        matrix.cells[0].variant = Some("switch".into());
        matrix.cells[0].assemblies_local = None;
        let fields = paired_key_fields(true, false);
        let assembly = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAssembly,
            MatrixTransformValue::Text("switch".into()),
        );
        assert_eq!(assembly.assemblies_local, None);
        let attached = edited_cell(
            &matrix,
            &fields,
            MatrixTransformField::KeyAttached,
            MatrixTransformValue::Attached(vec![("led".into(), "led-def".into())]),
        );
        assert_eq!(attached.assemblies_local, None);
    }

    #[test]
    fn use_mirrored_components_clears_the_local_flag_only() {
        let matrix = matrix();
        let cell = edited_cell(
            &matrix,
            &paired_key_fields(true, true),
            MatrixTransformField::KeyAssembliesLocal,
            MatrixTransformValue::Bool(false),
        );
        assert_eq!(cell.assemblies_local, Some(false));
        assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
        assert_eq!(cell.definition_id, matrix.cells[0].definition_id);
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 0,
            column: 1,
            offset: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            shift: Vec2::default(),
            bend: 0.0,
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
            shift: None,
            bend: None,
            deleted: false,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
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
            rotation: 0.0,
            splay_origin: Vec2 { x: 11.0, y: 12.0 },
            custom_origin: false,
            convert: Ok(()),
            spread: None,
            pitch: 19.05,
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
            rotation_followed: true,
            anchor: None,
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
            rotation_followed: true,
            anchor: None,
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
            shift: Vec2::default(),
            bend: 0.0,
            padding: None,
            pitch: 19.05,
        };
        let change = placement_result(
            build_operation(
                &matrix,
                &row_fields,
                MatrixTransformField::RowOffsetY,
                MatrixTransformValue::Number(-6.0),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(
            change,
            MatrixPlacementChange::RowOffset {
                row: 0,
                value: Vec2 { x: 2.0, y: -6.0 }
            }
        );
        let change = placement_result(
            build_operation(
                &matrix,
                &row_fields,
                MatrixTransformField::RowOffsetReset,
                MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(
            change,
            MatrixPlacementChange::RowOffset {
                row: 0,
                value: Vec2 { x: 0.0, y: 0.0 }
            }
        );

        let column_fields = MatrixTransformFields::Column {
            column: 2,
            offset: Vec2 { x: 1.0, y: 2.0 },
            stagger: 0.75,
            splay_angle: 8.0,
            rotation: 0.0,
            splay_origin: Vec2 { x: 11.0, y: 12.0 },
            custom_origin: true,
            convert: Ok(()),
            spread: None,
            pitch: 19.05,
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
            let change = placement_result(
                build_operation(
                    &matrix,
                    &column_fields,
                    field,
                    MatrixTransformValue::Number(value),
                    MatrixSplayAffect::Following,
                )
                .unwrap(),
            );
            assert_eq!(
                change,
                MatrixPlacementChange::ColumnOffset {
                    column: 2,
                    value: expected
                }
            );
        }
        let change = placement_result(
            build_operation(
                &matrix,
                &column_fields,
                MatrixTransformField::ColumnOffsetReset,
                MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(
            change,
            MatrixPlacementChange::ColumnOffset {
                column: 2,
                value: Vec2 { x: 0.0, y: 0.0 }
            }
        );
        let change = placement_result(
            build_operation(
                &matrix,
                &column_fields,
                MatrixTransformField::ColumnStagger,
                MatrixTransformValue::Number(-1.5),
                MatrixSplayAffect::Following,
            )
            .unwrap(),
        );
        assert_eq!(
            change,
            MatrixPlacementChange::ColumnStagger {
                column: 2,
                value: -1.5
            }
        );

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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
            row: 1,
            column: 2,
            offset: Vec2 { x: 6.0, y: 7.0 },
            rotation: 9.0,
            shift: Vec2::default(),
            bend: 0.0,
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

    fn placement_result(operation: EditOperation) -> MatrixPlacementChange {
        let EditOperation::SetMatrixPlacement { change, .. } = operation else {
            panic!("this field maps to SetMatrixPlacement");
        };
        change
    }

    fn set_matrix_result(operation: EditOperation) -> Matrix {
        let EditOperation::SetMatrix { matrix, .. } = operation else {
            panic!("this field maps to SetMatrix");
        };
        matrix
    }

    // --- Placement: one module builds every placement edit -----------------------------

    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditPhase, SceneDelta};

    /// A rotated, mirrored, splayed (custom origin) matrix, opened in a real `CoreEngine`.
    fn transformed_scene() -> (MatrixScene, Matrix) {
        let mut document =
            serde_json::to_value(boardstudio_core::model::ProjectDoc::empty("p", "P")).unwrap();
        document["definitions"] = serde_json::json!([{
            "id": "switch", "name": "switch", "kind": "switch",
            "courtyard": [{ "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
                          { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }],
            "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                       "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
        }]);
        document["boards"] = serde_json::json!([{ "id": "board", "name": "Board",
            "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6 }]);
        let mut engine = CoreEngine::new();
        let CoreReply::Scene { scene: opened, .. } = engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: serde_json::from_value(document).unwrap(),
        }) else {
            panic!("the fixture opens");
        };
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id": "m", "rows": 3, "columns": 4,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 10.0, "y": 5.0 },
            "definitionId": "switch", "boardId": "board", "partIds": [],
            "rotation": 20.0, "mirror": "x",
            "columnLayout": [{"stagger": 0.0, "splay": 0.0}, {"stagger": 3.0, "splay": 8.0, "splayOrigin": {"x": 30.0, "y": 12.0}}, {"stagger": -2.0, "splay": 0.0}, {"stagger": 1.0, "splay": -6.0}]
}))
        .unwrap();
        let CoreReply::Scene { scene, .. } = engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command: EditCommand {
                base_revision: opened.revision,
                transaction_id: "edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrix {
                    matrix: matrix.clone(),
                    definitions: None,
                },
            },
        }) else {
            panic!("the matrix lands");
        };
        let scene: SceneDelta = scene;
        (scene.matrix_scenes.into_iter().next().unwrap(), matrix)
    }

    #[test]
    fn inspector_and_gesture_send_the_same_operation() {
        let (scene, _) = transformed_scene();
        for (parameter, start, target) in [
            (
                PlacementParameter::Stagger { column: 1 },
                PlacementValue::Stagger(3.0),
                PlacementValue::Stagger(7.5),
            ),
            (
                PlacementParameter::ColumnOffset { column: 3 },
                PlacementValue::Offset(Vec2 { x: 1.0, y: -2.0 }),
                PlacementValue::Offset(Vec2 { x: -4.0, y: 6.5 }),
            ),
            (
                PlacementParameter::RowOffset { row: 2 },
                PlacementValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                PlacementValue::Offset(Vec2 { x: 2.5, y: -3.0 }),
            ),
            (
                PlacementParameter::ColumnSpread { column: 1 },
                PlacementValue::Spread(19.0),
                PlacementValue::Spread(24.0),
            ),
            (
                PlacementParameter::RowShift { row: 2 },
                PlacementValue::Shift(Vec2 { x: 1.0, y: -2.0 }),
                PlacementValue::Shift(Vec2 { x: -4.0, y: 6.5 }),
            ),
            (
                PlacementParameter::KeyShift { row: 1, column: 2 },
                PlacementValue::Shift(Vec2 { x: 0.5, y: 1.5 }),
                PlacementValue::Shift(Vec2 { x: -2.0, y: 3.0 }),
            ),
        ] {
            let basis = SceneBasis::for_parameter(&scene, parameter).unwrap();
            // The world delta that moves the parameter from `start` to `target` is worked
            // from the scene axes, not from the function under test.
            let world_delta = match (start, target) {
                (PlacementValue::Stagger(from), PlacementValue::Stagger(to)) => Vec2 {
                    x: (to - from) * basis.stagger_axis.x,
                    y: (to - from) * basis.stagger_axis.y,
                },
                (PlacementValue::Offset(from), PlacementValue::Offset(to)) => Vec2 {
                    x: (to.x - from.x) * basis.axis_x.x + (to.y - from.y) * basis.axis_y.x,
                    y: (to.x - from.x) * basis.axis_x.y + (to.y - from.y) * basis.axis_y.y,
                },
                (PlacementValue::Shift(from), PlacementValue::Shift(to)) => Vec2 {
                    x: (to.x - from.x) * basis.shift_axis_x.x
                        + (to.y - from.y) * basis.shift_axis_y.x,
                    y: (to.x - from.x) * basis.shift_axis_x.y
                        + (to.y - from.y) * basis.shift_axis_y.y,
                },
                (PlacementValue::Spread(from), PlacementValue::Spread(to)) => {
                    let axis = basis.spread_axis.unwrap_or(basis.axis_x);
                    Vec2 {
                        x: (to - from) * axis.x,
                        y: (to - from) * axis.y,
                    }
                }
                _ => unreachable!(),
            };
            let gesture = gesture_value(&basis, parameter, start, world_delta, None);
            let from_inspector = match target {
                PlacementValue::Stagger(value) => {
                    placement_value(parameter, start, PlacementSet::Stagger(value))
                }
                PlacementValue::Offset(value) => {
                    let x = placement_value(parameter, start, PlacementSet::OffsetX(value.x));
                    placement_value(parameter, x, PlacementSet::OffsetY(value.y))
                }
                PlacementValue::Shift(value) => {
                    let x = placement_value(parameter, start, PlacementSet::ShiftX(value.x));
                    placement_value(parameter, x, PlacementSet::ShiftY(value.y))
                }
                PlacementValue::Spread(value) => {
                    placement_value(parameter, start, PlacementSet::Spread(value))
                }
                PlacementValue::Bend(value) => {
                    placement_value(parameter, start, PlacementSet::Bend(value))
                }
                PlacementValue::Padding(value) => {
                    placement_value(parameter, start, PlacementSet::Padding(value))
                }
            };
            let (gesture_op, inspector_op) = (
                placement_operation("m", parameter, gesture),
                placement_operation("m", parameter, from_inspector),
            );
            match (gesture_op, inspector_op) {
                (
                    EditOperation::SetMatrixPlacement { change: a, .. },
                    EditOperation::SetMatrixPlacement { change: b, .. },
                ) => assert_placement_close(&a, &b),
                _ => panic!("both build SetMatrixPlacement"),
            }
        }
    }

    fn assert_placement_close(a: &MatrixPlacementChange, b: &MatrixPlacementChange) {
        let close = |x: f64, y: f64| (x - y).abs() < 1e-9;
        match (a, b) {
            (
                MatrixPlacementChange::ColumnStagger {
                    column: ca,
                    value: va,
                },
                MatrixPlacementChange::ColumnStagger {
                    column: cb,
                    value: vb,
                },
            ) => assert!(ca == cb && close(*va, *vb), "{a:?} vs {b:?}"),
            (
                MatrixPlacementChange::ColumnOffset {
                    column: ca,
                    value: va,
                },
                MatrixPlacementChange::ColumnOffset {
                    column: cb,
                    value: vb,
                },
            ) => assert!(
                ca == cb && close(va.x, vb.x) && close(va.y, vb.y),
                "{a:?} vs {b:?}"
            ),
            (
                MatrixPlacementChange::RowOffset { row: ra, value: va },
                MatrixPlacementChange::RowOffset { row: rb, value: vb },
            ) => assert!(
                ra == rb && close(va.x, vb.x) && close(va.y, vb.y),
                "{a:?} vs {b:?}"
            ),
            (
                MatrixPlacementChange::ColumnSpread {
                    column: ca,
                    value: va,
                },
                MatrixPlacementChange::ColumnSpread {
                    column: cb,
                    value: vb,
                },
            ) => assert!(ca == cb && va == vb, "{a:?} vs {b:?}"),
            (
                MatrixPlacementChange::RowShift { row: ra, value: va },
                MatrixPlacementChange::RowShift { row: rb, value: vb },
            ) => assert!(
                ra == rb && close(va.x, vb.x) && close(va.y, vb.y),
                "{a:?} vs {b:?}"
            ),
            (
                MatrixPlacementChange::KeyShift {
                    row: ra,
                    column: ca,
                    value: va,
                },
                MatrixPlacementChange::KeyShift {
                    row: rb,
                    column: cb,
                    value: vb,
                },
            ) => assert!(
                ra == rb && ca == cb && close(va.x, vb.x) && close(va.y, vb.y),
                "{a:?} vs {b:?}"
            ),
            _ => panic!("{a:?} vs {b:?}"),
        }
    }

    #[test]
    fn gesture_value_snaps_the_resulting_value() {
        let basis = SceneBasis {
            stagger_axis: Vec2 { x: 0.0, y: 1.0 },
            axis_x: Vec2 { x: 1.0, y: 0.0 },
            axis_y: Vec2 { x: 0.0, y: 1.0 },
            shift_axis_x: Vec2 { x: 1.0, y: 0.0 },
            shift_axis_y: Vec2 { x: 0.0, y: 1.0 },
            spread_axis: None,
        };
        let parameter = PlacementParameter::Stagger { column: 0 };
        let step = Some(Vec2 {
            x: 4.7625,
            y: 4.7625,
        });
        // Off the grid: the first snap brings the value onto it.
        let value = gesture_value(
            &basis,
            parameter,
            PlacementValue::Stagger(0.3),
            Vec2 { x: 0.0, y: 4.5 },
            step,
        );
        assert_eq!(value, PlacementValue::Stagger(4.7625));
        // On the grid: the change is a whole number of steps.
        let value = gesture_value(
            &basis,
            parameter,
            PlacementValue::Stagger(4.7625),
            Vec2 { x: 0.0, y: 4.7 },
            step,
        );
        assert_eq!(value, PlacementValue::Stagger(2.0 * 4.7625));
        // Alt skips the snap and returns the raw value.
        let value = gesture_value(
            &basis,
            parameter,
            PlacementValue::Stagger(0.3),
            Vec2 { x: 0.0, y: 4.5 },
            None,
        );
        let PlacementValue::Stagger(raw) = value else {
            panic!("a stagger stays a stagger");
        };
        assert!((raw - 4.8).abs() < 1e-12);
        // Motion across the stagger axis is ignored.
        let value = gesture_value(
            &basis,
            parameter,
            PlacementValue::Stagger(0.0),
            Vec2 { x: 50.0, y: 0.0 },
            step,
        );
        assert_eq!(value, PlacementValue::Stagger(0.0));
    }

    #[test]
    fn nudge_pairs_follow_the_axis() {
        // An axis rotated 90 degrees on screen consumes Left/Right, not Up/Down.
        let rotated = SceneBasis {
            stagger_axis: Vec2 { x: 1.0, y: 0.0 },
            axis_x: Vec2 { x: 0.0, y: 1.0 },
            axis_y: Vec2 { x: -1.0, y: 0.0 },
            shift_axis_x: Vec2 { x: 0.0, y: 1.0 },
            shift_axis_y: Vec2 { x: -1.0, y: 0.0 },
            spread_axis: None,
        };
        let stagger = PlacementParameter::Stagger { column: 0 };
        let start = PlacementValue::Stagger(0.0);
        assert_eq!(
            nudge_value(&rotated, stagger, start, ArrowKey::Up, false),
            None
        );
        assert_eq!(
            nudge_value(&rotated, stagger, start, ArrowKey::Down, false),
            None
        );
        let right = nudge_value(&rotated, stagger, start, ArrowKey::Right, false).unwrap();
        let PlacementValue::Stagger(right) = right else {
            panic!()
        };
        assert!(
            (right - 0.1).abs() < 1e-12,
            "Right moves the column along its axis"
        );
        let left = nudge_value(&rotated, stagger, start, ArrowKey::Left, true).unwrap();
        let PlacementValue::Stagger(left) = left else {
            panic!()
        };
        assert!((left + 1.0).abs() < 1e-12, "Shift steps 1 mm");

        // A mirror-Y frame's ArrowUp increases the on-screen Y, which lowers its local Y.
        let mirrored = SceneBasis {
            stagger_axis: Vec2 { x: 0.0, y: -1.0 },
            axis_x: Vec2 { x: 1.0, y: 0.0 },
            axis_y: Vec2 { x: 0.0, y: -1.0 },
            shift_axis_x: Vec2 { x: 1.0, y: 0.0 },
            shift_axis_y: Vec2 { x: 0.0, y: -1.0 },
            spread_axis: None,
        };
        let offset = PlacementParameter::ColumnOffset { column: 0 };
        let zero = PlacementValue::Offset(Vec2 { x: 0.0, y: 0.0 });
        let up = nudge_value(&mirrored, offset, zero, ArrowKey::Up, false).unwrap();
        let PlacementValue::Offset(up) = up else {
            panic!()
        };
        assert!((up.y + 0.1).abs() < 1e-12 && up.x.abs() < 1e-12, "{up:?}");
        let right = nudge_value(&mirrored, offset, zero, ArrowKey::Right, false).unwrap();
        let PlacementValue::Offset(right) = right else {
            panic!()
        };
        assert!(
            (right.x - 0.1).abs() < 1e-12 && right.y.abs() < 1e-12,
            "{right:?}"
        );
    }

    fn move_matrix(origin: Vec2, anchor: Option<MatrixAnchor>) -> Matrix {
        let mut matrix = matrix();
        matrix.origin = origin;
        matrix.anchor = anchor;
        matrix
    }

    fn drag_anchor(
        at: Vec2,
        rotation: f64,
        follow: Follow,
        transport: MatrixDragTransport,
    ) -> MatrixDragAnchor {
        MatrixDragAnchor {
            key_pose: Pose2 { at, rotation },
            follow,
            transport,
        }
    }

    #[test]
    fn unanchored_matrix_drag_moves_one_to_one() {
        let matrix = move_matrix(Vec2 { x: 5.0, y: 7.0 }, None);
        let solved = solve_matrix_origin(&matrix, None, Vec2 { x: 10.0, y: -3.0 });
        assert!((solved.x - 15.0).abs() < 1e-9 && (solved.y - 4.0).abs() < 1e-9);
    }

    #[test]
    fn anchored_drag_inverses_through_the_anchor_frame() {
        let matrix = move_matrix(
            Vec2 { x: 3.0, y: 4.0 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow: Follow::default(),
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(
            Vec2::default(),
            0.0,
            Follow::default(),
            MatrixDragTransport::World,
        );
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 10.0, y: 5.0 });
        assert!((solved.x - 13.0).abs() < 1e-9 && (solved.y - 9.0).abs() < 1e-9);
    }

    #[test]
    fn anchored_drag_solves_through_a_rotated_anchor_frame() {
        // Anchor key at (100, 200) turned 90 degrees, matrix origin (0, -19.05):
        // the frame sits at (119.05, 200). Dragging the frame +10 in world Y
        // must land the origin at (10, -19.05).
        let matrix = move_matrix(
            Vec2 { x: 0.0, y: -19.05 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow: Follow::default(),
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(
            Vec2 { x: 100.0, y: 200.0 },
            90.0,
            Follow::default(),
            MatrixDragTransport::World,
        );
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 0.0, y: 10.0 });
        assert!((solved.x - 10.0).abs() < 1e-9 && (solved.y + 19.05).abs() < 1e-9);
    }

    #[test]
    fn anchored_drag_ignores_unfollowed_directions() {
        let matrix = move_matrix(
            Vec2 { x: 3.0, y: 4.0 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow: Follow {
                    x: false,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(
            Vec2::default(),
            0.0,
            Follow {
                x: false,
                y: true,
                rotation: true,
            },
            MatrixDragTransport::World,
        );
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 10.0, y: 5.0 });
        assert!((solved.x - 3.0).abs() < 1e-9 && (solved.y - 9.0).abs() < 1e-9);
    }

    #[test]
    fn transported_drag_round_trips_through_the_pair_map() {
        // Linked partner mirrored about x = 50 in the source layout: with
        // full Follow the world drag applies exactly, round-tripping through
        // the transported source-world axes.
        let matrix = move_matrix(
            Vec2 { x: 0.0, y: 0.0 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow: Follow::default(),
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(
            Vec2 { x: 100.0, y: 0.0 },
            0.0,
            Follow::default(),
            MatrixDragTransport::Reflected {
                translation: Vec2 { x: 100.0, y: 0.0 },
                rotation: 0.0,
            },
        );
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 10.0, y: 5.0 });
        assert!((solved.x - 10.0).abs() < 1e-9 && (solved.y - 5.0).abs() < 1e-9);
    }

    #[test]
    fn transported_drag_ignores_source_unfollowed_directions() {
        let follow = Follow {
            x: false,
            y: true,
            rotation: true,
        };
        let matrix = move_matrix(
            Vec2 { x: 0.0, y: 0.0 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow,
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(
            Vec2 { x: 100.0, y: 0.0 },
            0.0,
            follow,
            MatrixDragTransport::Reflected {
                translation: Vec2 { x: 100.0, y: 0.0 },
                rotation: 0.0,
            },
        );
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 10.0, y: 5.0 });
        assert!((solved.x - 0.0).abs() < 1e-9 && (solved.y - 5.0).abs() < 1e-9);
    }

    #[test]
    fn drag_outside_every_follow_direction_solves_to_the_same_origin() {
        let follow = Follow {
            x: false,
            y: false,
            rotation: true,
        };
        let matrix = move_matrix(
            Vec2 { x: 3.0, y: 4.0 },
            Some(MatrixAnchor {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
                follow,
                aggregate: None,
            }),
        );
        let anchor = drag_anchor(Vec2::default(), 0.0, follow, MatrixDragTransport::World);
        let solved = solve_matrix_origin(&matrix, Some(anchor), Vec2 { x: 10.0, y: 5.0 });
        assert!((solved.x - 3.0).abs() < 1e-9 && (solved.y - 4.0).abs() < 1e-9);
    }

    #[test]
    fn rotate_solves_only_when_rotation_is_followed() {
        assert_eq!(solve_matrix_rotation(12.0, true, 30.0), Some(42.0));
        assert_eq!(solve_matrix_rotation(12.0, false, 30.0), None);
    }
}
