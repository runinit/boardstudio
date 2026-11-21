//! Host-compatible matrix-inspector operation predicates.
//!
//! The production inspector controller (wasm) delegates to this module so the
//! same operation logic runs under `cargo test` without a browser.

use boardstudio_model::model::{
    DiodeDirection, EditOperation, Matrix, MatrixCell, ProjectDoc, Vec2,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixNameTarget {
    Matrix,
    Layout { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixEditField {
    Name,
    Rows,
    Columns,
    PitchX,
    PitchY,
    SwitchDefinition,
    ApplyPreset,
    DiodeDirection,
    EdgeGapX,
    EdgeGapY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixPreset {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

impl MatrixPreset {
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.as_str() == value)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MxSolder => "mx-solder",
            Self::MxHotswap => "mx-hotswap",
            Self::ChocSolder => "choc-solder",
            Self::ChocHotswap => "choc-hotswap",
            Self::MxRgb => "mx-rgb",
            Self::ChocRgb => "choc-rgb",
            Self::MxHotswapRgb => "mx-hotswap-rgb",
            Self::ChocHotswapRgb => "choc-hotswap-rgb",
        }
    }

    pub(super) const ALL: [Self; 8] = [
        Self::MxSolder,
        Self::MxHotswap,
        Self::ChocSolder,
        Self::ChocHotswap,
        Self::MxRgb,
        Self::ChocRgb,
        Self::MxHotswapRgb,
        Self::ChocHotswapRgb,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchOrientation {
    South,
    North,
}

impl SwitchOrientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::South => "south",
            Self::North => "north",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixEditValue {
    Name(Option<String>),
    Rows(u32),
    Columns(u32),
    PitchX(f64),
    PitchY(f64),
    SwitchDefinition(String),
    DiodeDirection(DiodeDirection),
    EdgeGapX(f64),
    EdgeGapY(f64),
}

pub(super) fn variant_session_matches(
    expected_session: boardstudio_application::SessionEpoch,
    expected_document: &str,
    actual_session: boardstudio_application::SessionEpoch,
    actual_document: &str,
) -> bool {
    actual_session == expected_session && actual_document == expected_document
}

pub(super) fn field_value(
    document: &ProjectDoc,
    matrix_id: &str,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
) -> Option<MatrixEditValue> {
    match field {
        MatrixEditField::Name => match name_target {
            MatrixNameTarget::Matrix => document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .map(|matrix| MatrixEditValue::Name(matrix.name.clone())),
            MatrixNameTarget::Layout { id } => document
                .layouts
                .iter()
                .find(|layout| {
                    layout.id == *id && layout.matrix_ids.iter().any(|id| id == matrix_id)
                })
                .map(|layout| MatrixEditValue::Name(Some(layout.name.clone()))),
        },
        MatrixEditField::Rows => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Rows(matrix.rows)),
        MatrixEditField::Columns => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::Columns(matrix.columns)),
        MatrixEditField::PitchX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchX(matrix.pitch.x)),
        MatrixEditField::PitchY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::PitchY(matrix.pitch.y)),
        MatrixEditField::SwitchDefinition => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| MatrixEditValue::SwitchDefinition(matrix.definition_id.clone())),
        MatrixEditField::DiodeDirection => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::DiodeDirection(
                    matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
                )
            }),
        MatrixEditField::EdgeGapX => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapX(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.x))
            }),
        MatrixEditField::EdgeGapY => document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .map(|matrix| {
                MatrixEditValue::EdgeGapY(matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.y))
            }),
        MatrixEditField::ApplyPreset => None,
    }
}

pub(super) fn build_operation(
    document: &ProjectDoc,
    matrix: &Matrix,
    name_target: &MatrixNameTarget,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<EditOperation, String> {
    if field == MatrixEditField::Name {
        let MatrixEditValue::Name(Some(name)) = value else {
            return Err("Enter a non-empty matrix or layout name.".into());
        };
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a non-empty matrix or layout name.".into());
        }
        return match name_target {
            MatrixNameTarget::Matrix => {
                let mut replacement = matrix.clone();
                replacement.name = Some(name.to_owned());
                Ok(EditOperation::SetMatrix {
                    matrix: replacement,
                    definitions: None,
                })
            }
            MatrixNameTarget::Layout { id } => {
                let mut layout = document
                    .layouts
                    .iter()
                    .find(|layout| {
                        layout.id == *id && layout.matrix_ids.iter().any(|id| id == &matrix.id)
                    })
                    .cloned()
                    .ok_or_else(|| "The selected layout no longer owns this matrix.".to_owned())?;
                layout.name = name.to_owned();
                Ok(EditOperation::SetLayout { layout })
            }
        };
    }
    if matches!(name_target, MatrixNameTarget::Layout { .. }) && field == MatrixEditField::Name {
        return Err("The layout name target is unavailable.".into());
    }
    let replacement = matrix_edit(matrix, field, value)?;
    Ok(EditOperation::SetMatrix {
        matrix: replacement,
        definitions: None,
    })
}

pub(super) fn matrix_edit(
    matrix: &Matrix,
    field: MatrixEditField,
    value: MatrixEditValue,
) -> Result<Matrix, String> {
    match (field, value) {
        (MatrixEditField::Rows, MatrixEditValue::Rows(rows)) => {
            resize_matrix(matrix, rows, matrix.columns)
        }
        (MatrixEditField::Columns, MatrixEditValue::Columns(columns)) => {
            resize_matrix(matrix, matrix.rows, columns)
        }
        (MatrixEditField::PitchX, MatrixEditValue::PitchX(x)) if x.is_finite() && x > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.x = x;
            Ok(next)
        }
        (MatrixEditField::PitchY, MatrixEditValue::PitchY(y)) if y.is_finite() && y > 0.0 => {
            let mut next = matrix.clone();
            next.pitch.y = y;
            Ok(next)
        }
        (MatrixEditField::SwitchDefinition, MatrixEditValue::SwitchDefinition(id))
            if !id.trim().is_empty() =>
        {
            let mut next = matrix.clone();
            next.definition_id = id;
            Ok(next)
        }
        (MatrixEditField::DiodeDirection, MatrixEditValue::DiodeDirection(direction)) => {
            let mut next = matrix.clone();
            next.diode_direction = Some(direction);
            Ok(next)
        }
        (MatrixEditField::EdgeGapX, MatrixEditValue::EdgeGapX(x)) if x.is_finite() && x >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.x = x;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        (MatrixEditField::EdgeGapY, MatrixEditValue::EdgeGapY(y)) if y.is_finite() && y >= 0.0 => {
            let mut next = matrix.clone();
            let mut gap = next.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            gap.y = y;
            next.edge_gap = Some(gap);
            Ok(next)
        }
        _ => Err("The matrix field value is invalid.".into()),
    }
}

fn resize_matrix(matrix: &Matrix, rows: u32, columns: u32) -> Result<Matrix, String> {
    if rows == 0 || columns == 0 {
        return Err("Matrix dimensions must be positive whole numbers.".into());
    }
    if rows
        .checked_mul(columns)
        .is_none_or(|cell_count| cell_count > 4096)
    {
        return Err("Matrix dimensions cannot exceed 4096 cells.".into());
    }
    let template = matrix.cells.iter().find(|cell| {
        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    let mut replacement = matrix.clone();
    replacement.rows = rows;
    replacement.columns = columns;
    let Some(template) = template else {
        return Ok(replacement);
    };

    replacement
        .cells
        .retain(|cell| cell.row < rows && cell.column < columns);
    let assemblies: Vec<boardstudio_model::model::MatrixAssembly> = template
        .assemblies
        .iter()
        .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
        .cloned()
        .collect();
    let definition_id = template.definition_id.clone();
    let variant = template.variant.clone();
    let assemblies_local = template.assemblies_local;
    let rotation = Some(
        if variant
            .as_deref()
            .is_some_and(|variant| variant.ends_with("/north"))
        {
            180.0
        } else {
            0.0
        },
    );
    for row in 0..rows {
        for column in 0..columns {
            if row < matrix.rows && column < matrix.columns {
                continue;
            }
            replacement.cells.push(MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column,
                enabled: true,
                definition_id: definition_id.clone(),
                variant: variant.clone(),
                offset: None,
                rotation,
                assemblies: assemblies.clone(),
                assemblies_local,
            });
        }
    }
    Ok(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::MatrixAssembly;

    fn synthetic_outline_document() -> ProjectDoc {
        serde_json::from_str(include_str!(
            "../../../../../core/tests/fixtures/synthetic-outline.json"
        ))
        .expect("synthetic outline fixture should deserialize")
    }

    fn synthetic_matrix() -> Matrix {
        let document = synthetic_outline_document();
        document
            .matrices
            .into_iter()
            .find(|matrix| matrix.id == "matrix")
            .expect("synthetic fixture has its authored matrix")
    }

    fn request_matrix(matrix: &Matrix, field: MatrixEditField, value: MatrixEditValue) -> Matrix {
        let document = synthetic_outline_document();
        match build_operation(&document, matrix, &MatrixNameTarget::Matrix, field, value)
            .expect("the selected matrix field should produce a SetMatrix request")
        {
            EditOperation::SetMatrix { matrix, .. } => matrix,
            _ => panic!("matrix fields should use SetMatrix"),
        }
    }

    #[test]
    fn variant_owner_does_not_claim_a_later_open_even_with_the_same_project_id() {
        let original_operation = boardstudio_application::SessionEpoch(12);
        let reopened_variant = boardstudio_application::SessionEpoch(13);
        assert!(variant_session_matches(
            original_operation,
            "variant-id",
            original_operation,
            "variant-id",
        ));
        assert!(!variant_session_matches(
            original_operation,
            "variant-id",
            reopened_variant,
            "variant-id",
        ));
        assert!(!variant_session_matches(
            original_operation,
            "variant-id",
            original_operation,
            "user-opened-project",
        ));
    }

    #[test]
    fn resizing_synthetic_matrix_preserves_cells_and_inherits_only_electrical_members() {
        let mut matrix = synthetic_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        let old_rows = matrix.rows;
        let old_columns = matrix.columns;
        matrix
            .cells
            .retain(|cell| cell.row < old_rows && cell.column < old_columns);
        let original = matrix.cells[0].clone();
        matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "diode".into(),
            definition_id: "fixture-diode".into(),
            offset: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
            rotation: None,
            side: None,
        });
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "led".into(),
            definition_id: "fixture-led".into(),
            offset: boardstudio_model::model::Vec2 { x: 2.0, y: -3.0 },
            rotation: Some(27.0),
            side: Some(boardstudio_model::model::Side::Back),
        });
        matrix.cells[0].assemblies.push(MatrixAssembly {
            id: "local-extra".into(),
            definition_id: "fixture-extra".into(),
            offset: boardstudio_model::model::Vec2 { x: 9.0, y: 8.0 },
            rotation: None,
            side: None,
        });
        let template = matrix.cells[0].clone();

        let resized = request_matrix(
            &matrix,
            MatrixEditField::Columns,
            MatrixEditValue::Columns(3),
        );
        assert_eq!(resized.rows, 2);
        assert_eq!(resized.columns, 3);
        assert_eq!(resized.cells[0].row, original.row);
        assert_eq!(resized.cells[0].offset, original.offset);
        assert_eq!(resized.cells[0].enabled, original.enabled);
        let added = resized
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == 2)
            .unwrap();
        assert!(added.enabled);
        assert_eq!(
            added.definition_id.as_deref(),
            Some(matrix.definition_id.as_str())
        );
        assert_eq!(added.variant.as_deref(), Some("preset/mx-hotswap/north"));
        assert_eq!(added.rotation, Some(180.0));
        assert_eq!(added.offset, None);
        assert_eq!(added.assemblies_local, template.assemblies_local);
        assert_eq!(
            added.assemblies,
            template
                .assemblies
                .iter()
                .filter(|assembly| assembly.id == "diode" || assembly.id == "led")
                .cloned()
                .collect::<Vec<_>>()
        );
        assert_eq!(added.assemblies.len(), 2);
        assert!(
            !added
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "local-extra")
        );
        assert_eq!(matrix.cells[0].assemblies.len(), template.assemblies.len());
    }

    fn sparse_two_by_two() -> Matrix {
        let mut matrix = synthetic_matrix();
        matrix.rows = 2;
        matrix.columns = 2;
        matrix.cells.retain(|cell| cell.row < 2 && cell.column < 2);
        matrix.cells[0].variant = Some("preset/mx-hotswap/north".into());
        matrix
            .cells
            .retain(|cell| !(cell.row == 1 && cell.column == 1));
        matrix.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 2,
            column: 5,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: Vec::new(),
            assemblies_local: None,
        });
        matrix
    }

    #[test]
    fn resize_preserves_sparse_slots_and_trims_only_cells_outside_new_bounds() {
        let matrix = sparse_two_by_two();
        let before = matrix
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(3));
        let retained = resized
            .cells
            .iter()
            .filter(|cell| cell.row < 2 && cell.column < 2)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(retained, before);
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 1 && cell.column == 1)
        );
        assert!(
            !resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 5)
        );
        assert!(
            resized
                .cells
                .iter()
                .any(|cell| cell.row == 2 && cell.column == 1)
        );
    }

    #[test]
    fn custom_matrix_resize_keeps_cells_and_rejects_oversized_dimensions() {
        let mut matrix = synthetic_matrix();
        matrix.cells.iter_mut().for_each(|cell| cell.variant = None);
        let cells = matrix.cells.clone();
        let resized = request_matrix(&matrix, MatrixEditField::Rows, MatrixEditValue::Rows(4));
        assert_eq!(resized.cells, cells);
        for (field, value) in [
            (MatrixEditField::Rows, u32::MAX),
            (MatrixEditField::Rows, 0),
            (MatrixEditField::Columns, u32::MAX),
            (MatrixEditField::Columns, 0),
        ] {
            let edit = match field {
                MatrixEditField::Rows => MatrixEditValue::Rows(value),
                MatrixEditField::Columns => MatrixEditValue::Columns(value),
                _ => unreachable!("rejection table only covers dimensions"),
            };
            assert!(
                matrix_edit(&matrix, field, edit,).is_err(),
                "{field:?} should reject {value}"
            );
        }
    }

    #[test]
    fn matrix_pitch_request_preserves_every_unrelated_accepted_matrix_field() {
        let matrix = synthetic_matrix();
        let next = request_matrix(
            &matrix,
            MatrixEditField::PitchX,
            MatrixEditValue::PitchX(20.0),
        );
        assert_eq!(next.pitch.x, 20.0);
        assert_eq!(next.pitch.y, matrix.pitch.y);
        assert_eq!(next.rows, matrix.rows);
        assert_eq!(next.columns, matrix.columns);
        assert_eq!(next.cells, matrix.cells);
        assert_eq!(next.part_ids, matrix.part_ids);
        assert_eq!(next.board_id, matrix.board_id);
    }

    #[test]
    fn owning_layout_name_request_uses_set_layout_without_renaming_the_matrix() {
        let document = synthetic_outline_document();
        let mut matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "matrix")
            .unwrap()
            .clone();
        matrix.name = Some("Authored matrix name".into());
        let layout = document
            .layouts
            .iter()
            .find(|layout| {
                layout.matrix_ids.iter().any(|id| id == &matrix.id) && layout.board_id == "main"
            })
            .unwrap();
        let operation = build_operation(
            &document,
            &matrix,
            &MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            MatrixEditField::Name,
            MatrixEditValue::Name(Some("Renamed layout".into())),
        )
        .unwrap();
        let EditOperation::SetLayout { layout: renamed } = operation else {
            panic!("layout name changes should use SetLayout");
        };
        assert_eq!(renamed.id, layout.id);
        assert_eq!(renamed.matrix_ids, [matrix.id.clone()]);
        assert_eq!(renamed.name, "Renamed layout");
        assert_eq!(matrix.name.as_deref(), Some("Authored matrix name"));
    }
}
