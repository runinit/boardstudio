//! Host-compatible matrix input predicate shared by every presentation crate.
//!
//! The Parts library owns the presentation call sites; this module is the single
//! production implementation used on both targets and in native tests. Core remains
//! authoritative when SetMatrix is accepted: this predicate is advisory for
//! placement and switch-choice affordances.
use boardstudio_model::model::PartDefinition;

/// Whether `definition` can drive a matrix key: an independent press contact pair
/// whose terminals resolve to distinct, real pads outside any rotary contact set.
/// A definition without press contacts falls back to the legacy two-pad switch.
pub fn matrix_input_available(definition: &PartDefinition) -> bool {
    let press = if let Some(profile) = definition.input_profile.as_ref() {
        profile
            .press
            .as_ref()
            .map(|press| (press.row.clone(), press.column.clone(), press.independent))
    } else {
        definition
            .matrix_terminals
            .as_ref()
            .map(|press| (press.row.clone(), press.column.clone(), true))
            .or_else(|| {
                definition
                    .generator
                    .as_ref()
                    .filter(|generator| generator.source == "ceoloide/rotary_encoder_ec11_ec12")
                    .map(|_| ("S1".into(), "S2".into(), true))
            })
    };
    let Some((row_terminal, column_terminal, independent)) = press else {
        return definition.input_profile.is_none()
            && definition.pads.iter().any(|pad| pad.id == "one")
            && definition.pads.iter().any(|pad| pad.id == "two");
    };
    let pads_for = |terminal: &str| {
        definition
            .terminals
            .get(terminal)
            .cloned()
            .unwrap_or_else(|| {
                definition
                    .pads
                    .iter()
                    .filter(|pad| pad.id == terminal || pad.number == terminal)
                    .map(|pad| pad.id.clone())
                    .collect()
            })
    };
    let row = pads_for(&row_terminal);
    let column = pads_for(&column_terminal);
    let rotation = definition
        .input_profile
        .as_ref()
        .and_then(|profile| profile.rotary.as_ref())
        .into_iter()
        .flat_map(|rotation| [&rotation.a, &rotation.b, &rotation.common])
        .flat_map(|terminal| pads_for(terminal))
        .collect::<std::collections::BTreeSet<_>>();
    independent
        && !row.is_empty()
        && !column.is_empty()
        && row.iter().all(|id| !column.contains(id))
        && row
            .iter()
            .chain(&column)
            .all(|id| definition.pads.iter().any(|pad| pad.id == *id) && !rotation.contains(id))
}
