//! Pure matrix preset, switch-choice, and sparse-resize helpers plus native unit tests.
use boardstudio_model::model::{Matrix, MatrixCell, PartDefinition, ProjectDoc, Side, Vec2};
use boardstudio_web_catalogue::matrix_input::matrix_input_available;
use boardstudio_web_catalogue::matrix_setup_operation::MatrixSetupPreset;

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
    pub const ALL: [Self; 8] = [
        Self::MxSolder,
        Self::MxHotswap,
        Self::ChocSolder,
        Self::ChocHotswap,
        Self::MxRgb,
        Self::ChocRgb,
        Self::MxHotswapRgb,
        Self::ChocHotswapRgb,
    ];

    pub fn as_str(self) -> &'static str {
        self.setup().as_str()
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MxSolder => "MX Solder",
            Self::MxHotswap => "MX Hotswap",
            Self::ChocSolder => "Choc V1 Solder",
            Self::ChocHotswap => "Choc V1 Hotswap",
            Self::MxRgb => "MX RGB",
            Self::ChocRgb => "Choc V1 RGB",
            Self::MxHotswapRgb => "MX Hotswap RGB",
            Self::ChocHotswapRgb => "Choc V1 Hotswap RGB",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.as_str() == value)
    }

    pub(crate) fn setup(self) -> MatrixSetupPreset {
        match self {
            Self::MxSolder => MatrixSetupPreset::MxSolder,
            Self::MxHotswap => MatrixSetupPreset::MxHotswap,
            Self::ChocSolder => MatrixSetupPreset::ChocSolder,
            Self::ChocHotswap => MatrixSetupPreset::ChocHotswap,
            Self::MxRgb => MatrixSetupPreset::MxRgb,
            Self::ChocRgb => MatrixSetupPreset::ChocRgb,
            Self::MxHotswapRgb => MatrixSetupPreset::MxHotswapRgb,
            Self::ChocHotswapRgb => MatrixSetupPreset::ChocHotswapRgb,
        }
    }
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

/// The preset and orientation a matrix carries, read from its switch cells' variant tag.
pub fn matrix_preset(matrix: &Matrix) -> (Option<MatrixPreset>, Option<SwitchOrientation>) {
    let variant = matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.as_deref())
        .and_then(|value| value.strip_prefix("preset/"));
    let Some(variant) = variant else {
        return (None, None);
    };
    let preset = variant.split('/').next().and_then(MatrixPreset::parse);
    let orientation = match variant.rsplit('/').next() {
        Some("north") => Some(SwitchOrientation::North),
        Some("south") => Some(SwitchOrientation::South),
        _ => None,
    };
    (preset, orientation)
}

pub fn is_reversible(document: &ProjectDoc) -> bool {
    document
        .parameters
        .get("reversibleLayout")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn preset_variant(
    preset: MatrixPreset,
    orientation: SwitchOrientation,
    reversible: bool,
) -> String {
    let reversible = if reversible { "reversible/" } else { "" };
    format!(
        "preset/{}/{reversible}{}",
        preset.as_str(),
        orientation.as_str()
    )
}

/// Mirrors the pinned `matrixWithPreset` recipe while retaining matrix-only fields and each
/// pre-existing cell's enabled state, local pose and non-diode/non-LED assembly membership.
pub fn matrix_with_preset(
    matrix: &Matrix,
    prepared: &Matrix,
    variant: &str,
    orientation: SwitchOrientation,
) -> Matrix {
    let old_cells: std::collections::HashMap<_, _> = matrix
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let orientation_rotation = if orientation == SwitchOrientation::North {
        180.0
    } else {
        0.0
    };
    let cells = prepared
        .cells
        .iter()
        .map(|template| {
            let old = old_cells.get(&(template.row, template.column)).copied();
            let old_rotation = old.and_then(|cell| cell.rotation).unwrap_or(0.0);
            let old_orientation = old
                .and_then(|cell| cell.variant.as_deref())
                .filter(|variant| variant.starts_with("preset/") && variant.ends_with("/north"))
                .map_or(0.0, |_| 180.0);
            let next_rotation = old_rotation - old_orientation + orientation_rotation;
            let delta = (next_rotation - old_rotation).to_radians();
            let (sin, cos) = delta.sin_cos();
            let mut cell = template.clone();
            cell.variant = Some(variant.to_owned());
            cell.enabled = old.is_none_or(|cell| cell.enabled);
            cell.offset = old.and_then(|cell| cell.offset);
            cell.rotation = Some(next_rotation);
            cell.assemblies_local = Some(true);
            if let Some(old) = old {
                cell.assemblies.extend(
                    old.assemblies
                        .iter()
                        .filter(|a| a.id != "diode" && a.id != "led")
                        .map(|assembly| {
                            let mut assembly = assembly.clone();
                            assembly.offset = Vec2 {
                                x: assembly.offset.x * cos + assembly.offset.y * sin,
                                y: -assembly.offset.x * sin + assembly.offset.y * cos,
                            };
                            assembly.rotation =
                                Some(assembly.rotation.unwrap_or(0.0) - delta.to_degrees());
                            assembly.side.get_or_insert(Side::Front);
                            assembly
                        }),
                );
            }
            cell
        })
        .collect();
    let mut replacement = matrix.clone();
    replacement.definition_id = prepared.definition_id.clone();
    replacement.cells = cells;
    replacement
}

fn is_assembly_snapshot(id: &str, has_kicad_source: bool) -> bool {
    if has_kicad_source {
        return false;
    }
    let Some((prefix, _)) = id.split_once("/definition/") else {
        return false;
    };
    prefix.starts_with("assembly-")
        || (prefix.len() == 36
            && prefix.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            }))
}

/// The switch footprints a matrix can use: catalogue templates overlaid by the document's own
/// definitions, always including the current one.
pub fn switch_choices(
    document: &ProjectDoc,
    templates: &[PartDefinition],
    current_id: &str,
) -> Vec<(String, String)> {
    let mut definitions = Vec::<PartDefinition>::new();
    for definition in templates.iter().chain(&document.definitions) {
        match definitions
            .iter_mut()
            .find(|existing| existing.id == definition.id)
        {
            Some(existing) => *existing = definition.clone(),
            None => definitions.push(definition.clone()),
        }
    }
    definitions
        .into_iter()
        .filter(|definition| {
            definition.id == current_id
                || (!is_assembly_snapshot(&definition.id, definition.kicad_source.is_some())
                    && matrix_input_available(definition))
        })
        .map(|definition| {
            let label = match definition.id.as_str() {
                "generator:ceoloide/switch_mx" => "MX switch".into(),
                "generator:ceoloide/switch_choc_v1_v2" => "Choc V1 / V2 switch".into(),
                "generator:ceoloide/switch_gateron_ks27_ks33" => {
                    "Gateron KS27 / KS33 switch".into()
                }
                _ => definition.name.clone(),
            };
            (definition.id, label)
        })
        .chain(
            (!document
                .definitions
                .iter()
                .chain(templates)
                .any(|d| d.id == current_id))
            .then(|| (current_id.to_owned(), current_id.to_owned())),
        )
        .collect()
}

/// The switch definitions an edit may need: only those the accepted document lacks.
pub fn missing_definitions(
    document: &ProjectDoc,
    catalogue: &[PartDefinition],
    ids: &[&str],
) -> Vec<PartDefinition> {
    ids.iter()
        .filter(|id| {
            !document
                .definitions
                .iter()
                .any(|definition| definition.id == **id)
        })
        .filter_map(|id| {
            catalogue
                .iter()
                .find(|definition| definition.id == **id)
                .cloned()
        })
        .collect()
}

/// Resize keeping preset cells: new cells copy the preset template's assemblies.
pub(crate) fn resize(matrix: &Matrix, rows: u32, columns: u32) -> Result<Matrix, String> {
    if rows.checked_mul(columns).is_none_or(|cells| cells > 4096) {
        return Err("Matrix dimensions cannot exceed 4096 cells.".into());
    }
    let template = matrix.cells.iter().find(|cell| {
        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    let mut next = matrix.clone();
    next.rows = rows;
    next.columns = columns;
    let Some(template) = template else {
        return Ok(next);
    };
    next.cells
        .retain(|cell| cell.row < rows && cell.column < columns);
    let rotation = Some(
        if template
            .variant
            .as_deref()
            .is_some_and(|variant| variant.ends_with("/north"))
        {
            180.0
        } else {
            0.0
        },
    );
    let assemblies: Vec<_> = template
        .assemblies
        .iter()
        .filter(|a| a.id == "diode" || a.id == "led")
        .cloned()
        .collect();
    for row in 0..rows {
        for column in (0..columns).filter(|&column| row >= matrix.rows || column >= matrix.columns)
        {
            next.cells.push(MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column,
                enabled: true,
                definition_id: template.definition_id.clone(),
                variant: template.variant.clone(),
                offset: None,
                rotation,
                assemblies: assemblies.clone(),
                assemblies_local: template.assemblies_local,
            });
        }
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::{SwitchOrientation, matrix_with_preset, resize, switch_choices};
    use boardstudio_model::model::{Matrix, PartDefinition, ProjectDoc};

    #[test]
    fn matrix_with_preset_retains_state_and_transforms_extra_assemblies() {
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id": "m1", "rows": 1, "columns": 1,
            "pitch": { "x": 19.05, "y": 19.05 }, "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "old_sw", "partIds": [], "boardId": "b1",
            "cells": [{
                "row": 0, "column": 0, "enabled": false,
                "definitionId": "old_sw", "variant": "preset/mx-solder/south",
                "offset": { "x": 1.5, "y": -0.5 }, "rotation": 10.0,
                "assemblies": [
                    { "id": "diode", "definitionId": "old_d", "offset": { "x": 0.0, "y": 0.0 } },
                    { "id": "encoder_ring", "definitionId": "ring", "offset": { "x": 2.0, "y": 0.0 }, "rotation": 5.0 }
                ]
            }]
        }))
        .unwrap();
        let prepared: Matrix = serde_json::from_value(serde_json::json!({
            "id": "m1", "rows": 1, "columns": 1,
            "pitch": { "x": 19.05, "y": 19.05 }, "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "new_sw", "partIds": [], "boardId": "b1",
            "cells": [{
                "row": 0, "column": 0, "enabled": true,
                "definitionId": "new_sw",
                "assemblies": [
                    { "id": "diode", "definitionId": "new_d", "offset": { "x": 0.0, "y": 0.0 } }
                ]
            }]
        }))
        .unwrap();

        let next = matrix_with_preset(
            &matrix,
            &prepared,
            "preset/mx-hotswap/north",
            SwitchOrientation::North,
        );
        assert_eq!(next.definition_id, "new_sw");
        let cell = &next.cells[0];
        assert!(!cell.enabled);
        assert_eq!(cell.variant.as_deref(), Some("preset/mx-hotswap/north"));
        assert_eq!(cell.rotation, Some(190.0));
        assert_eq!(cell.assemblies.len(), 2);
        let extra = &cell.assemblies[1];
        assert_eq!(extra.id, "encoder_ring");
        assert!((extra.offset.x + 2.0).abs() < 1e-9 && extra.offset.y.abs() < 1e-9);
        assert!((extra.rotation.unwrap() + 175.0).abs() < 1e-9);
    }

    #[test]
    fn switch_choices_prefers_doc_definitions_excludes_snapshots_and_keeps_current() {
        use boardstudio_model::model::{Pad, PadShape, PartKind, Vec2};
        let pad = |id: &str, number: &str, x: f64| Pad {
            id: id.into(),
            number: number.into(),
            at: Vec2 { x, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Rect,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        };
        let matrix_part = |id: &str, name: &str| PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.into(),
            name: name.into(),
            kind: PartKind::Switch,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: Vec::new(),
            pads: vec![pad("one", "1", 0.0), pad("two", "2", 2.0)],
            models: None,
            generator: None,
            mechanical_profile: None,
        };
        let mut doc = ProjectDoc::empty("doc", "Test");
        doc.definitions = vec![
            matrix_part("generator:ceoloide/switch_mx", "Custom MX Override"),
            matrix_part("assembly-123/definition/snap", "Hidden Assembly Snapshot"),
            matrix_part(
                "01234567-89ab-cdef-0123-456789abcdef/definition/snap",
                "UUID Snapshot",
            ),
        ];
        let templates = vec![
            matrix_part("generator:ceoloide/switch_mx", "Template MX"),
            matrix_part("generator:ceoloide/switch_choc_v1_v2", "Template Choc"),
        ];
        let choices = switch_choices(&doc, &templates, "missing_current_id");
        assert!(choices.contains(&("generator:ceoloide/switch_mx".into(), "MX switch".into())));
        assert!(choices.contains(&(
            "generator:ceoloide/switch_choc_v1_v2".into(),
            "Choc V1 / V2 switch".into()
        )));
        assert!(
            !choices
                .iter()
                .any(|(id, _)| id.contains("/definition/snap"))
        );
        assert!(choices.contains(&("missing_current_id".into(), "missing_current_id".into())));
    }

    #[test]
    fn resize_preserves_sparse_preset_cells_and_enforces_4096_cell_limit() {
        let matrix: Matrix = serde_json::from_value(serde_json::json!({
            "id": "m1",
            "rows": 2,
            "columns": 3,
            "pitch": { "x": 19.05, "y": 19.05 },
            "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "switch_mx",
            "partIds": [],
            "boardId": "b1",
            "cells": [
                {
                    "row": 0,
                    "column": 0,
                    "enabled": true,
                    "definitionId": "switch_mx",
                    "variant": "preset/mx-hotswap/north",
                    "assemblies": [
                        { "id": "diode", "definitionId": "d1", "offset": { "x": 0.0, "y": 0.0 } },
                        { "id": "custom_mount", "definitionId": "c1", "offset": { "x": 1.0, "y": 0.0 } }
                    ],
                    "assembliesLocal": true
                },
                {
                    "row": 1,
                    "column": 2,
                    "enabled": false,
                    "definitionId": "switch_mx",
                    "variant": "preset/mx-hotswap/north",
                    "assemblies": []
                }
            ]
        }))
        .unwrap();
        let err = resize(&matrix, 65, 65).unwrap_err();
        assert!(err.contains("4096"), "{err}");

        let expanded = resize(&matrix, 3, 4).unwrap();
        assert_eq!(expanded.rows, 3);
        assert_eq!(expanded.columns, 4);
        assert!(
            expanded
                .cells
                .iter()
                .any(|c| c.row == 1 && c.column == 2 && !c.enabled)
        );
        assert!(!expanded.cells.iter().any(|c| c.row == 1 && c.column == 1));
        let added = expanded
            .cells
            .iter()
            .find(|c| c.row == 2 && c.column == 3)
            .expect("frontier cell");
        assert_eq!(added.rotation, Some(180.0));
        assert_eq!(added.assemblies.len(), 1);
        assert_eq!(added.assemblies[0].id, "diode");

        let shrunk = resize(&expanded, 2, 2).unwrap();
        assert!(shrunk.cells.iter().all(|c| c.row < 2 && c.column < 2));
    }
}
