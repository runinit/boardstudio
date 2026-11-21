//! Attached-component choices and the catalogue admission policy for assembly edits.

use super::{MatrixTransformField, MatrixTransformValue};
use boardstudio_model::model::PartDefinition;

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
