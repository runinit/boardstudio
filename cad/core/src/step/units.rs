pub(super) mod precision;
use super::Matrix4;
use cgmath::SquareMatrix;

use std::collections::{HashMap, HashSet};

use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};
#[cfg(test)]
use step_p21::parser;

type Records<'a> = super::SourceRecords<'a>;

/// Decode bytes using the same policy as `Table::from_step_bytes`: preserve
/// valid UTF-8 unchanged, otherwise map each byte to its same-valued Unicode
/// code point so raw ISO-8859-1 metadata is not replaced or lost.
pub(super) fn decode_step_bytes(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    match std::str::from_utf8(bytes) {
        Ok(text) => std::borrow::Cow::Borrowed(text),
        Err(_) => std::borrow::Cow::Owned(bytes.iter().map(|&byte| char::from(byte)).collect()),
    }
}

#[cfg(test)]
fn resolve_mm_per_unit(step: &str) -> Result<f64, String> {
    let factors = resolve_mm_per_unit_by_representation(step)?;
    let mut values = factors.values();
    let expected = *values
        .next()
        .ok_or("no shape representation factors found")?;
    if values.any(|factor| !same_factor(expected, *factor)) {
        return Err(format!(
            "mixed shape representation length contexts are unsupported: {factors:?}"
        ));
    }
    Ok(expected)
}

#[cfg(test)]
fn resolve_mm_per_unit_by_representation(step: &str) -> Result<HashMap<u64, f64>, String> {
    let exchange = parser::parse(step).map_err(|error| format!("STEP AST parse: {error}"))?;
    length_factors(&exchange)
}

fn checked_records(exchange: &Exchange) -> Result<Records<'_>, String> {
    let records = super::source_records(exchange);
    if records.len() != exchange.data[0].entities.len() {
        return Err("duplicate STEP entity identity".into());
    }
    Ok(records)
}

pub(super) fn length_factors(exchange: &Exchange) -> Result<HashMap<u64, f64>, String> {
    if exchange.data.len() != 1 {
        return Err(format!(
            "unit resolver requires exactly one DATA section, got {}",
            exchange.data.len()
        ));
    }
    let records = checked_records(exchange)?;

    let mut assignments = HashMap::<u64, Vec<u64>>::new();
    let mut units = HashMap::<u64, f64>::new();
    for (id, entity_records) in &records {
        let assignment_records = entity_records
            .iter()
            .filter(|record| record.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            .collect::<Vec<_>>();
        if assignment_records.len() > 1 {
            return Err(format!(
                "context entity #{id} has {} GLOBAL_UNIT_ASSIGNED_CONTEXT records; exactly one is supported",
                assignment_records.len()
            ));
        }
        if let Some(record) = assignment_records.first() {
            let representation_context_records = entity_records
                .iter()
                .filter(|record| record.name == "REPRESENTATION_CONTEXT")
                .count();
            if representation_context_records != 1 {
                return Err(format!(
                    "assigned context entity #{id} has {representation_context_records} REPRESENTATION_CONTEXT records; exactly one is required"
                ));
            }
            let params = list_params(record, 1)?;
            let Parameter::List(references) = &params[0] else {
                return Err(format!(
                    "GLOBAL_UNIT_ASSIGNED_CONTEXT #{id} has no unit reference list"
                ));
            };
            let unit_ids = references
                .iter()
                .map(|parameter| entity_ref(parameter).ok_or_else(|| {
                    format!("GLOBAL_UNIT_ASSIGNED_CONTEXT #{id} contains a non-entity unit reference")
                }))
                .collect::<Result<Vec<_>, _>>()?;
            let mut unique_unit_ids = HashSet::new();
            for unit_id in &unit_ids {
                if !unique_unit_ids.insert(*unit_id) {
                    return Err(format!(
                        "GLOBAL_UNIT_ASSIGNED_CONTEXT #{id} repeats unit reference #{unit_id}"
                    ));
                }
                let unit_records = records.get(unit_id).ok_or_else(|| {
                    format!(
                        "GLOBAL_UNIT_ASSIGNED_CONTEXT #{id} references missing entity #{unit_id}"
                    )
                })?;
                unit_kind(*unit_id, unit_records)?;
            }
            assignments.insert(*id, unit_ids);
        }
    }
    let mut representation_contexts = Vec::new();
    for (id, entity_records) in &records {
        for record in entity_records {
            if is_shape_representation_kind(&record.name) {
                let params = match &record.parameter {
                    Parameter::List(params) => params,
                    _ => {
                        return Err(format!(
                            "shape representation #{id} has malformed parameters"
                        ));
                    }
                };
                let context_id = params.get(2).and_then(entity_ref).ok_or_else(|| {
                    format!("shape representation #{id} has no typed context reference")
                })?;
                representation_contexts.push((*id, context_id));
                break;
            }
        }
    }
    if representation_contexts.is_empty() {
        return Err("no typed shape representation contexts found".into());
    }

    let mut resolved = Vec::new();
    for (representation_id, context_id) in representation_contexts {
        let context_entity = records.get(&context_id).ok_or_else(|| {
            format!(
                "shape representation #{representation_id} references missing context entity #{context_id}"
            )
        })?;
        let representation_context_records = context_entity
            .iter()
            .filter(|record| record.name == "REPRESENTATION_CONTEXT")
            .count();
        if representation_context_records != 1 {
            return Err(format!(
                "shape representation #{representation_id} context #{context_id} has {representation_context_records} REPRESENTATION_CONTEXT records; exactly one is required"
            ));
        }
        let unit_ids = assignments.get(&context_id);
        if unit_ids.is_some_and(Vec::is_empty) {
            return Err(format!(
                "shape representation #{representation_id} context #{context_id} has an empty GLOBAL_UNIT_ASSIGNED_CONTEXT"
            ));
        }
        let length_ids = unit_ids
            .into_iter()
            .flatten()
            .filter(|unit_id| {
                records
                    .get(unit_id)
                    .and_then(|unit_records| unit_kind(**unit_id, unit_records).ok())
                    == Some(UnitKind::Length)
            })
            .copied()
            .collect::<Vec<_>>();
        if length_ids.len() > 1 {
            return Err(format!(
                "shape representation #{representation_id} context #{context_id} assigns {} length units; at most one is supported",
                length_ids.len()
            ));
        }
        let Some(unit_id) = length_ids.first().copied() else {
            // A missing length assignment means coordinates are already millimetres.
            resolved.push((representation_id, context_id, 0, 1.0));
            continue;
        };
        let factor = if let Some(factor) = units.get(&unit_id) {
            *factor
        } else {
            let mut stack = HashSet::new();
            let factor = unit_factor_mm(unit_id, &records, &mut stack)?;
            units.insert(unit_id, factor);
            factor
        };
        resolved.push((representation_id, context_id, unit_id, factor));
    }

    Ok(resolved
        .into_iter()
        .map(|(representation_id, _, _, factor)| (representation_id, factor))
        .collect())
}

pub(super) fn representation_items(exchange: &Exchange) -> Result<HashMap<u64, Vec<u64>>, String> {
    if exchange.data.len() != 1 {
        return Err(format!(
            "representation item resolver requires exactly one DATA section, got {}",
            exchange.data.len()
        ));
    }
    let mut item_lists = HashMap::new();
    for instance in &exchange.data[0].entities {
        let (id, entity_records) = match instance {
            EntityInstance::Simple { id, record } => (*id, std::slice::from_ref(record)),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.as_slice()),
        };
        for record in entity_records {
            if !is_shape_representation_kind(&record.name) {
                continue;
            }
            let params = list_params(record, 3)?;
            let Parameter::List(items) = &params[1] else {
                return Err(format!("shape representation #{id} has no typed item list"));
            };
            let items = items
                .iter()
                .map(|item| {
                    entity_ref(item)
                        .ok_or_else(|| format!("shape representation #{id} has a non-entity item"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if item_lists.insert(id, items).is_some() {
                return Err(format!(
                    "entity #{id} contains duplicate shape representation records"
                ));
            }
        }
    }
    Ok(item_lists)
}

fn is_shape_representation_kind(name: &str) -> bool {
    matches!(
        name,
        "SHAPE_REPRESENTATION"
            | "ADVANCED_BREP_SHAPE_REPRESENTATION"
            | "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION"
            | "FACETED_BREP_SHAPE_REPRESENTATION"
            | "MANIFOLD_SURFACE_SHAPE_REPRESENTATION"
            | "BREP_WITH_VOIDS_SHAPE_REPRESENTATION"
    )
}

pub fn same_factor(a: f64, b: f64) -> bool {
    (a - b).abs() <= a.abs().max(b.abs()) * 1e-12
}

fn list_params(record: &Record, count: usize) -> Result<&[Parameter], String> {
    let Parameter::List(params) = &record.parameter else {
        return Err(format!("{} record is not a parameter list", record.name));
    };
    if params.len() < count {
        return Err(format!(
            "{} record has {} parameters, expected {count}",
            record.name,
            params.len()
        ));
    }
    Ok(params)
}

fn entity_ref(parameter: &Parameter) -> Option<u64> {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitKind {
    Length,
    PlaneAngle,
    SolidAngle,
}

fn unit_kind(id: u64, entity_records: &[&Record]) -> Result<UnitKind, String> {
    let length_count = entity_records
        .iter()
        .filter(|r| r.name == "LENGTH_UNIT")
        .count();
    let plane_angle_count = entity_records
        .iter()
        .filter(|r| r.name == "PLANE_ANGLE_UNIT")
        .count();
    let solid_angle_count = entity_records
        .iter()
        .filter(|r| r.name == "SOLID_ANGLE_UNIT")
        .count();
    let named_unit_count = entity_records
        .iter()
        .filter(|r| r.name == "NAMED_UNIT")
        .count();
    let kind_counts = [length_count, plane_angle_count, solid_angle_count];
    if kind_counts.iter().sum::<usize>() != 1 || named_unit_count != 1 {
        return Err(format!(
            "assigned entity #{id} is not exactly one supported named LENGTH_UNIT, PLANE_ANGLE_UNIT, or SOLID_ANGLE_UNIT"
        ));
    }
    let definition_count = entity_records
        .iter()
        .filter(|r| {
            matches!(
                r.name.as_str(),
                "SI_UNIT" | "CONVERSION_BASED_UNIT" | "DERIVED_UNIT"
            )
        })
        .count();
    if definition_count != 1 {
        return Err(format!(
            "unit entity #{id} has {definition_count} SI_UNIT, CONVERSION_BASED_UNIT, or DERIVED_UNIT records; exactly one is supported"
        ));
    }
    if length_count == 1 {
        Ok(UnitKind::Length)
    } else if plane_angle_count == 1 {
        Ok(UnitKind::PlaneAngle)
    } else {
        Ok(UnitKind::SolidAngle)
    }
}

fn unit_factor_mm(id: u64, records: &Records<'_>, stack: &mut HashSet<u64>) -> Result<f64, String> {
    const MAX_CONVERSION_DEPTH: usize = 64;
    if stack.len() >= MAX_CONVERSION_DEPTH {
        return Err(format!(
            "length unit conversion chain at #{id} exceeds maximum depth {MAX_CONVERSION_DEPTH}"
        ));
    }
    if !stack.insert(id) {
        return Err(format!("cyclic length unit conversion at #{id}"));
    }
    let entity_records = records
        .get(&id)
        .ok_or_else(|| format!("missing unit entity #{id}"))?;
    if unit_kind(id, entity_records)? != UnitKind::Length {
        return Err(format!("conversion base entity #{id} is not a LENGTH_UNIT"));
    }
    let si_units = entity_records
        .iter()
        .filter(|r| r.name == "SI_UNIT")
        .collect::<Vec<_>>();
    let conversion_units = entity_records
        .iter()
        .filter(|r| r.name == "CONVERSION_BASED_UNIT")
        .collect::<Vec<_>>();
    if si_units.len() + conversion_units.len() != 1 {
        return Err(format!(
            "length unit #{id} must contain exactly one SI_UNIT or CONVERSION_BASED_UNIT record"
        ));
    }
    let result = if let Some(record) = si_units.first() {
        let params = list_params(record, 2)?;
        let prefix = match &params[0] {
            Parameter::Enumeration(prefix) => Some(prefix.as_str()),
            Parameter::NotProvided | Parameter::Omitted => None,
            other => return Err(format!("unsupported SI prefix in #{id}: {other:?}")),
        };
        let unit_name = match &params[1] {
            Parameter::Enumeration(name) => name.as_str(),
            other => return Err(format!("unsupported SI unit in #{id}: {other:?}")),
        };
        if unit_name != "METRE" {
            return Err(format!(
                "length unit #{id} uses unsupported SI unit {unit_name}"
            ));
        }
        si_prefix_factor(prefix).ok_or_else(|| format!("unsupported SI prefix {prefix:?} in #{id}"))
    } else if let Some(record) = conversion_units.first() {
        let params = list_params(record, 2)?;
        let measure_id = params
            .get(1)
            .and_then(entity_ref)
            .ok_or_else(|| format!("conversion-based unit #{id} has no measure reference"))?;
        let measure_records = records
            .get(&measure_id)
            .ok_or_else(|| format!("conversion measure #{measure_id} is missing"))?;
        let measure_records = measure_records
            .iter()
            .filter(|r| r.name == "LENGTH_MEASURE_WITH_UNIT")
            .collect::<Vec<_>>();
        if measure_records.len() != 1 {
            return Err(format!(
                "conversion measure #{measure_id} has {} LENGTH_MEASURE_WITH_UNIT records; exactly one is required",
                measure_records.len()
            ));
        }
        let measure = measure_records[0];
        let params = list_params(measure, 2)?;
        let value = length_measure(&params[0])?;
        let base_unit_id = entity_ref(&params[1])
            .ok_or_else(|| format!("length measure #{measure_id} has no unit reference"))?;
        Ok(value * unit_factor_mm(base_unit_id, records, stack)?)
    } else {
        return Err(format!("unsupported LENGTH_UNIT entity #{id}"));
    }?;
    stack.remove(&id);
    if !result.is_finite() || result <= 0.0 {
        return Err(format!(
            "invalid length conversion factor for unit #{id}: {result}"
        ));
    }
    Ok(result)
}

fn length_measure(parameter: &Parameter) -> Result<f64, String> {
    let Parameter::Typed { keyword, parameter } = parameter else {
        return Err(format!("expected typed LENGTH_MEASURE, got {parameter:?}"));
    };
    if keyword != "LENGTH_MEASURE" {
        return Err(format!("expected LENGTH_MEASURE, got {keyword}"));
    }
    let value = match parameter.as_ref() {
        Parameter::Real(value) => *value,
        Parameter::Integer(value) => *value as f64,
        other => return Err(format!("unsupported LENGTH_MEASURE value {other:?}")),
    };
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid LENGTH_MEASURE value {value}"));
    }
    Ok(value)
}

fn si_prefix_factor(prefix: Option<&str>) -> Option<f64> {
    Some(si_prefix(prefix)?.0 * 1000.0)
}

fn si_prefix(prefix: Option<&str>) -> Option<(f64, i32)> {
    // Retain the binary constants used for mesh placement and also the
    // corresponding decimal exponents used for exact source placement.
    Some(match prefix {
        None => (1.0, 0),
        Some("EXA") => (1e18, 18),
        Some("PETA") => (1e15, 15),
        Some("TERA") => (1e12, 12),
        Some("GIGA") => (1e9, 9),
        Some("MEGA") => (1e6, 6),
        Some("KILO") => (1e3, 3),
        Some("HECTO") => (1e2, 2),
        Some("DECA") => (1e1, 1),
        Some("DECI") => (1e-1, -1),
        Some("CENTI") => (1e-2, -2),
        Some("MILLI") => (1e-3, -3),
        Some("MICRO") => (1e-6, -6),
        Some("NANO") => (1e-9, -9),
        Some("PICO") => (1e-12, -12),
        Some("FEMTO") => (1e-15, -15),
        Some("ATTO") => (1e-18, -18),
        Some(_) => return None,
    })
}

pub fn solid_representation_units(
    solid_ids: &[u64],
    representation_items: &HashMap<u64, Vec<u64>>,
    representation_units: &HashMap<u64, f64>,
) -> Result<HashMap<u64, f64>, String> {
    let mut result = HashMap::new();
    for solid_id in solid_ids {
        let mut owners = representation_items
            .iter()
            .filter(|(_, items)| items.contains(solid_id))
            .map(|(representation_id, _)| {
                representation_units
                    .get(representation_id)
                    .map(|factor| (*representation_id, *factor))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("solid #{solid_id} is directly referenced by a representation without a resolved context"))?;
        owners.sort_by_key(|(representation_id, _)| *representation_id);
        if owners.is_empty() {
            return Err(format!(
                "solid #{solid_id} has no direct shape representation owner; indirect/shared ownership is unsupported"
            ));
        }
        let expected = owners[0].1;
        if owners
            .iter()
            .any(|(_, factor)| !same_factor(*factor, expected))
        {
            return Err(format!(
                "solid #{solid_id} is shared across representation contexts with conflicting length units: {owners:?}"
            ));
        }
        result.insert(*solid_id, expected);
    }
    Ok(result)
}

pub fn validate_placement_matrix(matrix: Matrix4) -> Result<(), String> {
    let values = [
        matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w, matrix.y.x, matrix.y.y, matrix.y.z,
        matrix.y.w, matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w, matrix.w.x, matrix.w.y,
        matrix.w.z, matrix.w.w,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("non-finite placement matrix".into());
    }
    let determinant = matrix.determinant();
    let x_norm = (matrix.x.x.powi(2) + matrix.x.y.powi(2) + matrix.x.z.powi(2)).sqrt();
    let y_norm = (matrix.y.x.powi(2) + matrix.y.y.powi(2) + matrix.y.z.powi(2)).sqrt();
    let z_norm = (matrix.z.x.powi(2) + matrix.z.y.powi(2) + matrix.z.z.powi(2)).sqrt();
    let scale = x_norm * y_norm * z_norm;
    if !scale.is_finite() || scale == 0.0 || determinant.abs() <= scale * 1e-12 {
        return Err("singular placement axes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod step_byte_decode_tests;

#[cfg(test)]
mod test_fixture;
