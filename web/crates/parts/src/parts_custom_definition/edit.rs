use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DefinitionEdit {
    Kind(PartKind),
    CourtyardWidth(String),
    CourtyardHeight(String),
    AddPad,
    PadId {
        pad_id: String,
        value: String,
    },
    PadNumber {
        pad_id: String,
        value: String,
    },
    PadCoordinate {
        pad_id: String,
        axis: Axis,
        value: String,
    },
    PadSize {
        pad_id: String,
        axis: Axis,
        value: String,
    },
    PadDrill {
        pad_id: String,
        value: String,
    },
    PadShape {
        pad_id: String,
        shape: PadShape,
    },
    RemovePad {
        pad_id: String,
    },
}

/// Value-only validation the panel runs at dispatch, so an invalid draft is explained
/// inline without submitting anything. Checks that depend on the document (target
/// existence, eligibility, uniqueness) belong to the resolver.
pub fn validate_definition_edit(edit: &DefinitionEdit) -> Result<(), String> {
    match edit {
        DefinitionEdit::Kind(_)
        | DefinitionEdit::AddPad
        | DefinitionEdit::PadShape { .. }
        | DefinitionEdit::RemovePad { .. } => Ok(()),
        DefinitionEdit::CourtyardWidth(value) | DefinitionEdit::CourtyardHeight(value) => {
            parse_number(value, true, COURTYARD_NUMBERS).map(|_| ())
        }
        DefinitionEdit::PadId { value, .. } | DefinitionEdit::PadNumber { value, .. } => {
            if value.trim().is_empty() {
                Err(PAD_IDS_REQUIRED.into())
            } else {
                Ok(())
            }
        }
        DefinitionEdit::PadCoordinate { value, .. } => {
            parse_number(value, false, PAD_NUMBERS).map(|_| ())
        }
        DefinitionEdit::PadSize { value, .. } => parse_number(value, true, PAD_NUMBERS).map(|_| ()),
        DefinitionEdit::PadDrill { value, .. } => {
            if value.trim().is_empty() {
                Ok(())
            } else {
                parse_number(value, true, PAD_NUMBERS).map(|_| ())
            }
        }
    }
}

/// One resolved edit: Session fills the base revision and transaction identity when
/// the edit runs (ticket 03), so the resolver submits only the operation and targets.
pub(crate) fn replacement_commit(operation: EditOperation, target_ids: Vec<String>) -> Resolution {
    Resolution::submit(target_ids, operation)
}

/// Resolve one definition-field edit against the accepted document at execution: find
/// the definition, apply only this field to a clone of the accepted document, and
/// submit the replacement from there, so an edit queued behind another never reverts
/// it. Vanished or ineligible targets retire with a reason the user understands.
pub fn definition_field_resolver(
    definition_id: String,
    edit: DefinitionEdit,
    pad_id_seed: u64,
) -> EditResolver {
    EditResolver::new(
        "parts-definition-field",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if !document
                .definitions
                .iter()
                .any(|item| item.id == definition_id)
            {
                return Resolution::Retire(DEFINITION_GONE.into());
            }
            if document
                .definitions
                .iter()
                .any(|item| item.id == definition_id && item.generator.is_some())
            {
                return Resolution::Retire(GENERATOR_LOCKED.into());
            }
            let mut replacement = document.as_ref().clone();
            let mut target_ids = vec![definition_id.clone()];
            if let Err(reason) = apply_definition_edit(
                &mut replacement,
                &definition_id,
                &mut target_ids,
                &edit,
                pad_id_seed,
            ) {
                return Resolution::Retire(reason);
            }
            // The clone is built from the accepted document, so equality means this
            // field already holds the committed value: resolve Unchanged, not a
            // landing heuristic (ADR-0005).
            if replacement == **document {
                return Resolution::Unchanged;
            }
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                target_ids,
            )
        },
    )
}

/// Apply one field edit in place. `Err` is the retire reason.
fn apply_definition_edit(
    doc: &mut ProjectDoc,
    definition_id: &str,
    target_ids: &mut Vec<String>,
    edit: &DefinitionEdit,
    pad_id_seed: u64,
) -> Result<(), String> {
    let Some(definition) = doc
        .definitions
        .iter_mut()
        .find(|item| item.id == definition_id)
    else {
        return Err(DEFINITION_GONE.into());
    };
    match edit {
        DefinitionEdit::Kind(kind) => definition.kind = kind.clone(),
        DefinitionEdit::CourtyardWidth(value) => {
            let width = parse_number(value, true, COURTYARD_NUMBERS)?;
            let (_, height, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::CourtyardHeight(value) => {
            let height = parse_number(value, true, COURTYARD_NUMBERS)?;
            let (width, _, center) = courtyard_bounds(&definition.courtyard);
            authored_rectangle(definition, width, height, center);
        }
        DefinitionEdit::AddPad => {
            imported_pads_retire(definition)?;
            let number = next_pad_number(&definition.pads);
            let id = unique_pad_id(&definition.pads, pad_id_seed);
            definition.pads.push(Pad {
                id,
                number,
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 2.0, y: 2.0 },
                shape: PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            });
        }
        DefinitionEdit::PadId { pad_id, value } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            if value.trim().is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.id == *value)
            {
                return Err(PAD_IDS_UNIQUE.into());
            }
            if let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) {
                pad.id = value.clone();
            }
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                for pin in &mut net.pins {
                    if part_ids.contains(&pin.part_id) && pin.pad_id == *pad_id {
                        pin.pad_id = value.clone();
                    }
                }
            }
            target_ids.extend([pad_id.clone(), value.clone()]);
        }
        DefinitionEdit::PadNumber { pad_id, value } => {
            imported_pads_retire(definition)?;
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(PAD_IDS_REQUIRED.into());
            }
            if definition
                .pads
                .iter()
                .any(|other| other.id != *pad_id && other.number == value)
            {
                return Err(PAD_NUMBERS_UNIQUE.into());
            }
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.number = value;
        }
        DefinitionEdit::PadCoordinate {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = parse_number(value, false, PAD_NUMBERS)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            match axis {
                Axis::X => pad.at.x = number,
                Axis::Y => pad.at.y = number,
            }
        }
        DefinitionEdit::PadSize {
            pad_id,
            axis,
            value,
        } => {
            imported_pads_retire(definition)?;
            let number = parse_number(value, true, PAD_NUMBERS)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            match axis {
                Axis::X => pad.size.x = number,
                Axis::Y => pad.size.y = number,
            }
        }
        DefinitionEdit::PadDrill { pad_id, value } => {
            imported_pads_retire(definition)?;
            let drill = if value.trim().is_empty() {
                None
            } else {
                Some(parse_number(value, true, PAD_NUMBERS)?)
            };
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.drill = drill;
        }
        DefinitionEdit::PadShape { pad_id, shape } => {
            imported_pads_retire(definition)?;
            let Some(pad) = definition.pads.iter_mut().find(|pad| pad.id == *pad_id) else {
                return Err(PAD_GONE.into());
            };
            pad.shape = shape.clone();
        }
        DefinitionEdit::RemovePad { pad_id } => {
            imported_pads_retire(definition)?;
            if !definition.pads.iter().any(|pad| pad.id == *pad_id) {
                return Err(PAD_GONE.into());
            }
            definition.pads.retain(|pad| pad.id != *pad_id);
            let part_ids = doc
                .parts
                .iter()
                .filter(|part| part.definition_id == definition.id)
                .map(|part| part.id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            for net in &mut doc.nets {
                net.pins
                    .retain(|pin| !part_ids.contains(&pin.part_id) || pin.pad_id != *pad_id);
            }
            target_ids.push(pad_id.clone());
        }
    }
    Ok(())
}

/// Rewrite the courtyard as an authored rectangle of the given size around `center`.
fn authored_rectangle(definition: &mut PartDefinition, width: f64, height: f64, center: Vec2) {
    let half_x = width / 2.0;
    let half_y = height / 2.0;
    definition.courtyard = vec![
        Vec2 {
            x: center.x - half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y - half_y,
        },
        Vec2 {
            x: center.x + half_x,
            y: center.y + half_y,
        },
        Vec2 {
            x: center.x - half_x,
            y: center.y + half_y,
        },
    ];
    let envelope = definition
        .envelope_source
        .get_or_insert_with(Default::default);
    envelope.courtyard = Some(boardstudio_model::model::EnvelopeOrigin::Authored);
}

fn imported_pads_retire(definition: &PartDefinition) -> Result<(), String> {
    if definition.kicad_source.is_some() {
        Err(IMPORTED_PADS.into())
    } else {
        Ok(())
    }
}
pub(super) fn parse_number(value: &str, positive: bool, message: &str) -> Result<f64, String> {
    let number = if value.trim().is_empty() && !positive {
        0.0
    } else {
        value.parse::<f64>().unwrap_or(f64::NAN)
    };
    if !number.is_finite() || (positive && number <= 0.0) {
        Err(message.into())
    } else {
        Ok(number)
    }
}
pub(crate) fn courtyard_bounds(points: &[Vec2]) -> (f64, f64, Vec2) {
    if points.is_empty() {
        return (10.0, 6.0, Vec2 { x: 0.0, y: 0.0 });
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let min_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    (
        (max_x - min_x),
        (max_y - min_y),
        Vec2 {
            x: (min_x + max_x) / 2.0,
            y: (min_y + max_y) / 2.0,
        },
    )
}
pub(super) fn next_pad_number(pads: &[Pad]) -> String {
    let mut candidate = pads.len() + 1;
    while pads.iter().any(|pad| pad.number == candidate.to_string()) {
        candidate += 1;
    }
    candidate.to_string()
}
fn unique_pad_id(pads: &[Pad], operation: u64) -> String {
    let base = format!("pad-{operation}");
    if !pads.iter().any(|pad| pad.id == base) {
        return base;
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !pads.iter().any(|pad| pad.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}
