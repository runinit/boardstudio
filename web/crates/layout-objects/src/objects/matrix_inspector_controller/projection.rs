//! Accepted-document projection for the Matrix Inspector: identity, choices, and text.
use super::*;
use boardstudio_web_catalogue::matrix_input::matrix_input_available;

pub(super) fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    editor_instance_id: u64,
    context_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
) -> (Option<MatrixInspectorProjection>, bool) {
    if workspace != "Layout" {
        return (None, false);
    }
    let model = runtime.model();
    let scope = runtime.scope();
    project_current_for(
        runtime,
        &model,
        scope.as_ref(),
        selected,
        MatrixProjectionContext {
            editor_instance_id,
            context_generation,
            scope_generation,
            workspace,
        },
    )
    .map_or((None, false), |(projection, editable)| {
        (Some(projection), editable)
    })
}

pub(super) struct MatrixProjectionContext {
    pub editor_instance_id: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub workspace: &'static str,
}

pub fn switch_choices(
    document: &ProjectDoc,
    templates: &[PartDefinition],
    current_id: &str,
) -> Vec<(String, String)> {
    let mut definitions = Vec::<PartDefinition>::new();
    for definition in templates.iter().chain(&document.definitions) {
        if let Some(existing) = definitions
            .iter_mut()
            .find(|existing| existing.id == definition.id)
        {
            *existing = definition.clone();
        } else {
            definitions.push(definition.clone());
        }
    }
    if !definitions
        .iter()
        .any(|definition| definition.id == current_id)
    {
        definitions.push(PartDefinition {
            id: current_id.into(),
            name: current_id.into(),
            kind: boardstudio_model::model::PartKind::Switch,
            hardware_profile: None,
            input_profile: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            models: None,
            generator: None,
            mechanical_profile: None,
        });
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
        .collect()
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
            && prefix.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn parse_preset_variant(
    variant: Option<&str>,
) -> (Option<MatrixPreset>, Option<SwitchOrientation>) {
    let Some(variant) = variant.and_then(|value| value.strip_prefix("preset/")) else {
        return (None, None);
    };
    let mut segments = variant.split('/');
    let preset = segments.next().and_then(parse_matrix_preset);
    let orientation = variant.rsplit('/').next().and_then(|value| match value {
        "north" => Some(SwitchOrientation::North),
        "south" => Some(SwitchOrientation::South),
        _ => None,
    });
    (preset, orientation)
}

fn parse_matrix_preset(value: &str) -> Option<MatrixPreset> {
    Some(match value {
        "mx-solder" => MatrixPreset::MxSolder,
        "mx-hotswap" => MatrixPreset::MxHotswap,
        "choc-solder" => MatrixPreset::ChocSolder,
        "choc-hotswap" => MatrixPreset::ChocHotswap,
        "mx-rgb" => MatrixPreset::MxRgb,
        "choc-rgb" => MatrixPreset::ChocRgb,
        "mx-hotswap-rgb" => MatrixPreset::MxHotswapRgb,
        "choc-hotswap-rgb" => MatrixPreset::ChocHotswapRgb,
        _ => return None,
    })
}

#[allow(dead_code)]
pub(super) fn matrix_preset_variant(matrix: &Matrix) -> Option<String> {
    matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.clone())
}

pub(super) fn preset_variant(
    preset: MatrixPreset,
    orientation: SwitchOrientation,
    reversible: bool,
) -> String {
    if reversible {
        format!(
            "preset/{}/reversible/{}",
            preset.as_str(),
            orientation.as_str()
        )
    } else {
        format!("preset/{}/{}", preset.as_str(), orientation.as_str())
    }
}

pub(super) fn project_current_for(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    live_scope: Option<&Scope>,
    selected: Option<&ScopedTreeContext>,
    context: MatrixProjectionContext,
) -> Option<(MatrixInspectorProjection, bool)> {
    let MatrixProjectionContext {
        editor_instance_id,
        context_generation,
        scope_generation,
        workspace,
    } = context;
    if workspace != "Layout" {
        return None;
    }
    let scope = live_scope?;
    let selected = selected?;
    if &selected.scope != scope || runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let TreeContext::Matrix { matrix_id } = &selected.context else {
        return None;
    };
    let snapshot = model.accepted.as_ref()?;
    if snapshot.session_epoch != scope.session_epoch
        || snapshot.document.id != scope.document_id
        || model.active_board_id != scope.board_id
        || model.active_instance_id != scope.instance_id
        || super::super::resolve_selection(model, &selected.context).is_none()
    {
        return None;
    }
    let matrix = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)?;
    let layout = snapshot.document.layouts.iter().find(|layout| {
        layout.board_id == scope.board_id && layout.matrix_ids.iter().any(|id| id == &matrix.id)
    });
    let (name_target, name_label, name_value, name_baseline) = if let Some(layout) = layout {
        (
            MatrixNameTarget::Layout {
                id: layout.id.clone(),
            },
            "Layout name",
            layout.name.clone(),
            MatrixEditValue::Name(Some(layout.name.clone())),
        )
    } else {
        let name = matrix.name.clone();
        let fallback = super::super::context_label(model, &selected.context)
            .unwrap_or_else(|| format!("Matrix {}", matrix.id));
        (
            MatrixNameTarget::Matrix,
            "Matrix name",
            name.clone().unwrap_or(fallback),
            MatrixEditValue::Name(name),
        )
    };
    // Matrix edits queue behind each other, so the inspector stays editable while an earlier
    // edit is applying or saving; only an unopened or unrecoverable session blocks it.
    let saved = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    );
    let editable = saved && model.display_preview.is_none() && model.gesture.is_none();
    let baseline_variant = matrix
        .cells
        .iter()
        .find(|cell| cell.definition_id.as_deref() == Some(matrix.definition_id.as_str()))
        .and_then(|cell| cell.variant.clone());
    let (preset, orientation) = parse_preset_variant(baseline_variant.as_deref());
    let unlink_layout = layout.and_then(|active_layout| {
        if active_layout.mirror_link.is_some() {
            Some(active_layout)
        } else {
            snapshot.document.layouts.iter().find(|candidate| {
                candidate.board_id == scope.board_id
                    && candidate
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == active_layout.id)
            })
        }
    });
    let partner_layout = unlink_layout.and_then(|linked_layout| {
        if layout.is_some_and(|active_layout| active_layout.id == linked_layout.id) {
            linked_layout.mirror_link.as_ref().and_then(|link| {
                snapshot.document.layouts.iter().find(|candidate| {
                    candidate.board_id == scope.board_id && candidate.id == link.source_id
                })
            })
        } else {
            Some(linked_layout)
        }
    });
    let layout_relation = layout.map(|_| MatrixLayoutRelation {
        partner_name: partner_layout.map(|partner| partner.name.clone()),
        unlink_layout_id: unlink_layout
            .filter(|linked_layout| linked_layout.mirror_link.is_some())
            .map(|linked_layout| linked_layout.id.clone()),
    });
    Some((
        MatrixInspectorProjection {
            owner: MatrixInspectorOwner {
                editor_instance_id,
                context_generation,
                scope_generation,
                scope: scope.clone(),
                matrix_id: matrix.id.clone(),
                name_target,
            },
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            matrix_label: super::super::context_label(model, &selected.context)
                .unwrap_or_else(|| format!("Matrix {}", matrix.id)),
            name_label,
            name_value,
            name_baseline,
            rows: matrix.rows,
            columns: matrix.columns,
            pitch_x: matrix.pitch.x,
            pitch_y: matrix.pitch.y,
            definition_id: matrix.definition_id.clone(),
            switch_choices: switch_choices(&snapshot.document, &[], &matrix.definition_id),
            diode_direction: matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
            edge_gap_x: matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.x),
            edge_gap_y: matrix.edge_gap.as_ref().map_or(1.0, |gap| gap.y),
            preset,
            orientation,
            baseline_variant,
            layout_relation,
        },
        editable,
    ))
}

pub(super) fn matrix_field_text(
    key: &MatrixPendingKey,
    projection: Option<&MatrixInspectorProjection>,
) -> String {
    match key {
        MatrixPendingKey::Field(field) => matrix_field_value_text(projection, *field),
        MatrixPendingKey::Action(_) => String::new(),
    }
}

pub(super) fn matrix_field_value_text(
    projection: Option<&MatrixInspectorProjection>,
    field: MatrixEditField,
) -> String {
    let Some(projection) = projection else {
        return String::new();
    };
    match field {
        MatrixEditField::Name => projection.name_value.clone(),
        MatrixEditField::Rows => projection.rows.to_string(),
        MatrixEditField::Columns => projection.columns.to_string(),
        MatrixEditField::PitchX => projection.pitch_x.to_string(),
        MatrixEditField::PitchY => projection.pitch_y.to_string(),
        MatrixEditField::SwitchDefinition => projection.definition_id.clone(),
        MatrixEditField::DiodeDirection => match projection.diode_direction {
            DiodeDirection::Row2col => "row2col".to_owned(),
            DiodeDirection::Col2row => "col2row".to_owned(),
        },
        MatrixEditField::EdgeGapX => projection.edge_gap_x.to_string(),
        MatrixEditField::EdgeGapY => projection.edge_gap_y.to_string(),
        MatrixEditField::ApplyPreset => String::new(),
    }
}
