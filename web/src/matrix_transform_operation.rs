//! Private, pure mapping from accepted matrix transform fields to existing edit operations.
use boardstudio_core::model::{
    EditOperation, Matrix, MatrixCell, MatrixSplayAffect, MatrixSplayChange, Mirror,
    PartDefinition, Vec2,
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
    KeyAttached,
    KeyAssembliesLocal,
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
    Attached(Vec<(String, String)>),
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
        assemblies: Vec<(String, String)>,
        component_choices: Vec<PartDefinition>,
        mirror_target: bool,
        assemblies_local: bool,
        offset: Vec2,
        rotation: f64,
    },
}

/// Produce the choices for one attached-component selector while preserving
/// only its own assembly snapshot from the accepted document.
pub(crate) fn attachment_component_choices(
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
pub(crate) fn component_edit_admitted_by_catalogue(
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
pub(crate) fn catalogue_retry_due(
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
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
            assemblies: Vec::new(),
            component_choices: Vec::new(),
            mirror_target: false,
            assemblies_local: false,
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
