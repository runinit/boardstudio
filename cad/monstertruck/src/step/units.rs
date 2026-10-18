pub(super) mod precision;
use monstertruck_io::step::load::{
    convert::*,
    step_geometry::{
        Curve3D, ElementarySurface, StepParameterCurve, Surface, SurfaceCurve3D,
        SurfaceCurveAssociatedGeometry,
    },
};
use monstertruck_modeling::{Matrix3, Matrix4, Plane, SquareMatrix, Transformed};

use std::collections::{HashMap, HashSet};

use monstertruck_io::step::load::step_p21::ast::{
    EntityInstance, Exchange, Name, Parameter, Record,
};
use monstertruck_io::step::load::step_p21::parser;

type Records = HashMap<u64, Vec<Record>>;
type Shell = StepCompressedShell;

/// Decode bytes using the same policy as `Table::from_step_bytes`: preserve
/// valid UTF-8 unchanged, otherwise map each byte to its same-valued Unicode
/// code point so raw ISO-8859-1 metadata is not replaced or lost.
pub fn decode_step_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.iter().map(|&byte| char::from(byte)).collect(),
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

pub fn resolve_mm_per_unit_by_representation(step: &str) -> Result<HashMap<u64, f64>, String> {
    let exchange = parser::parse(step).map_err(|error| format!("STEP AST parse: {error}"))?;
    if exchange.data.len() != 1 {
        return Err(format!(
            "unit resolver requires exactly one DATA section, got {}",
            exchange.data.len()
        ));
    }
    let mut records = Records::new();
    for instance in &exchange.data[0].entities {
        let (id, entity_records) = match instance {
            EntityInstance::Simple { id, record } => (*id, vec![record.clone()]),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.clone()),
        };
        if records.insert(id, entity_records).is_some() {
            return Err(format!("duplicate entity id #{id}"));
        }
    }

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

/// Convert each supported conical semi-angle from its owning representation's
/// declared plane-angle unit into radians before Monstertruck converts it.
/// An absent angle unit keeps the importer’s existing radians interpretation.
#[cfg(test)]
pub fn normalize_conical_angles(step: &str) -> Result<Exchange, String> {
    let exchange = parser::parse(step).map_err(|error| format!("STEP AST parse: {error}"))?;
    normalize_conical_exchange(&exchange).map(std::borrow::Cow::into_owned)
}

pub(super) fn normalize_conical_exchange(original: &Exchange) -> Result<std::borrow::Cow<'_, Exchange>, String> {
    let exchange = original;
    if exchange.data.len() != 1 {
        return Err(format!(
            "angle resolver requires exactly one DATA section, got {}",
            exchange.data.len()
        ));
    }
    let representation_factors = radians_per_unit_by_representation(&exchange)?;
    let cone_factors = conical_surface_factors(&exchange, &representation_factors)?;
    let changed = exchange.data[0].entities.iter().any(|entity| {
        let EntityInstance::Simple { id, record } = entity else { return false };
        if record.name != "CONICAL_SURFACE" { return false; }
        let Some(factor) = cone_factors.get(id) else { return false };
        let Parameter::List(params) = &record.parameter else { return true };
        match params.get(3) {
            Some(Parameter::Real(value)) => { let result = *value * factor; !result.is_finite() || !result.tan().is_finite() || result != *value },
            _ => true,
        }
    });
    // Preserve complex-encoding refusal even when no scalar value would change.
    if !changed && !exchange.data[0].entities.iter().any(|entity| matches!(entity,
        EntityInstance::Complex { id, subsuper } if cone_factors.contains_key(id) && subsuper.0.iter().any(|r| r.name == "CONICAL_SURFACE"))) {
        return Ok(std::borrow::Cow::Borrowed(original));
    }
    let mut exchange = original.clone();
    for entity in &mut exchange.data[0].entities {
        let (id, records, is_complex) = match entity {
            EntityInstance::Simple { id, record } => (*id, std::slice::from_mut(record), false),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.as_mut_slice(), true),
        };
        for record in records {
            if record.name != "CONICAL_SURFACE" {
                continue;
            }
            let Some(factor) = cone_factors.get(&id).copied() else {
                // Orphan geometry has no representation context and is not
                // imported into solids by the supported conversion path.
                continue;
            };
            if is_complex {
                return Err(format!(
                    "complex CONICAL_SURFACE #{id} encoding is unsupported for angle normalization"
                ));
            }
            let Parameter::List(params) = &mut record.parameter else {
                return Err(format!("CONICAL_SURFACE #{id} has malformed parameters"));
            };
            let angle = params
                .get_mut(3)
                .ok_or_else(|| format!("CONICAL_SURFACE #{id} has no semi-angle"))?;
            let value = match angle {
                Parameter::Real(value) => value,
                Parameter::Integer(value) => {
                    *angle = Parameter::Real(*value as f64);
                    match angle {
                        Parameter::Real(value) => value,
                        _ => unreachable!(),
                    }
                }
                other => {
                    return Err(format!(
                        "CONICAL_SURFACE #{id} has nonnumeric semi-angle {other:?}"
                    ));
                }
            };
            *value *= factor;
            if !value.is_finite() || !value.tan().is_finite() {
                return Err(format!(
                    "CONICAL_SURFACE #{id} has invalid normalized semi-angle {value}"
                ));
            }
        }
    }
    Ok(std::borrow::Cow::Owned(exchange))
}

fn radians_per_unit_by_representation(exchange: &Exchange) -> Result<HashMap<u64, f64>, String> {
    let mut records = Records::new();
    for instance in &exchange.data[0].entities {
        let (id, entity_records) = match instance {
            EntityInstance::Simple { id, record } => (*id, vec![record.clone()]),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.clone()),
        };
        if records.insert(id, entity_records).is_some() {
            return Err(format!("duplicate entity id #{id}"));
        }
    }
    let mut assignments = HashMap::<u64, Vec<u64>>::new();
    let mut unit_factors = HashMap::<u64, f64>::new();
    for (&context_id, entity_records) in &records {
        let assignment_records = entity_records
            .iter()
            .filter(|r| r.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            .collect::<Vec<_>>();
        if assignment_records.len() > 1 {
            return Err(format!(
                "context entity #{context_id} has {} GLOBAL_UNIT_ASSIGNED_CONTEXT records; exactly one is supported",
                assignment_records.len()
            ));
        }
        for record in assignment_records {
            let params = list_params(record, 1)?;
            let Parameter::List(refs) = &params[0] else {
                return Err(format!(
                    "GLOBAL_UNIT_ASSIGNED_CONTEXT #{context_id} has no unit reference list"
                ));
            };
            if refs.is_empty() {
                return Err(format!(
                    "GLOBAL_UNIT_ASSIGNED_CONTEXT #{context_id} has an empty unit assignment"
                ));
            }
            let mut unit_ids = Vec::new();
            let mut seen = HashSet::new();
            let mut angle_count = 0;
            for reference in refs {
                let unit_id = entity_ref(reference).ok_or_else(|| {
                    format!("GLOBAL_UNIT_ASSIGNED_CONTEXT #{context_id} contains a non-entity unit reference")
                })?;
                if !seen.insert(unit_id) {
                    return Err(format!(
                        "GLOBAL_UNIT_ASSIGNED_CONTEXT #{context_id} repeats unit reference #{unit_id}"
                    ));
                }
                let unit_records = records.get(&unit_id).ok_or_else(|| {
                    format!("GLOBAL_UNIT_ASSIGNED_CONTEXT #{context_id} references missing entity #{unit_id}")
                })?;
                let kind = unit_kind(unit_id, unit_records)?;
                if kind == UnitKind::PlaneAngle {
                    angle_count += 1;
                    let mut stack = HashSet::new();
                    let factor = plane_angle_factor_radians(unit_id, &records, &mut stack)?;
                    unit_factors.insert(unit_id, factor);
                }
                unit_ids.push(unit_id);
            }
            if angle_count > 1 {
                return Err(format!(
                    "context #{context_id} assigns {angle_count} plane-angle units; at most one is supported"
                ));
            }
            assignments.insert(context_id, unit_ids);
        }
    }
    let mut factors = HashMap::new();
    for (&representation_id, entity_records) in &records {
        let Some(record) = entity_records
            .iter()
            .find(|record| is_shape_representation_kind(&record.name))
        else {
            continue;
        };
        let params = list_params(record, 3)?;
        let context_id = params.get(2).and_then(entity_ref).ok_or_else(|| {
            format!("shape representation #{representation_id} has no typed context reference")
        })?;
        let context_records = records.get(&context_id).ok_or_else(|| {
            format!("shape representation #{representation_id} references missing context entity #{context_id}")
        })?;
        let context_record_count = context_records
            .iter()
            .filter(|r| r.name == "REPRESENTATION_CONTEXT")
            .count();
        if context_record_count != 1 {
            return Err(format!(
                "shape representation #{representation_id} context #{context_id} has {context_record_count} REPRESENTATION_CONTEXT records; exactly one is required"
            ));
        }
        let angles = assignments
            .get(&context_id)
            .into_iter()
            .flatten()
            .filter(|id| {
                records
                    .get(id)
                    .is_some_and(|unit| unit_kind(**id, unit).ok() == Some(UnitKind::PlaneAngle))
            })
            .copied()
            .collect::<Vec<_>>();
        let factor = match angles.as_slice() {
            [] => 1.0,
            [id] => *unit_factors
                .get(id)
                .ok_or_else(|| format!("plane-angle unit #{id} was not resolved"))?,
            _ => {
                return Err(format!(
                    "shape representation #{representation_id} has multiple plane-angle units"
                ));
            }
        };
        factors.insert(representation_id, factor);
    }
    if factors.is_empty() {
        return Err("no typed shape representation contexts found for angle resolution".into());
    }
    Ok(factors)
}

fn conical_surface_factors(
    exchange: &Exchange,
    representation_factors: &HashMap<u64, f64>,
) -> Result<HashMap<u64, f64>, String> {
    let records = exchange.data[0]
        .entities
        .iter()
        .map(|instance| match instance {
            EntityInstance::Simple { id, record } => (*id, vec![record.clone()]),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.clone()),
        })
        .collect::<HashMap<_, _>>();
    let mut result = HashMap::<u64, f64>::new();
    for (&representation_id, entity_records) in &records {
        let Some(record) = entity_records
            .iter()
            .find(|r| is_shape_representation_kind(&r.name))
        else {
            continue;
        };
        let params = list_params(record, 3)?;
        let items = match &params[1] {
            Parameter::List(items) => items
                .iter()
                .map(|item| {
                    entity_ref(item).ok_or_else(|| {
                        format!("shape representation #{representation_id} has a non-entity item")
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => {
                return Err(format!(
                    "shape representation #{representation_id} has no item list"
                ));
            }
        };
        let factor = *representation_factors
            .get(&representation_id)
            .ok_or_else(|| {
                format!("shape representation #{representation_id} has no resolved angle factor")
            })?;
        let mut stack = items;
        let mut visited = HashSet::new();
        while let Some(id) = stack.pop() {
            if !visited.insert(id) {
                continue;
            }
            let Some(child_records) = records.get(&id) else {
                return Err(format!(
                    "shape representation #{representation_id} reaches missing entity #{id}"
                ));
            };
            if child_records
                .iter()
                .any(|child| child.name == "CONICAL_SURFACE")
                && let Some(previous) = result.insert(id, factor)
                && !same_factor(previous, factor)
            {
                return Err(format!(
                    "CONICAL_SURFACE #{id} is shared by representation contexts with conflicting plane-angle units: {previous} vs {factor} radians/unit"
                ));
            }
            // A mapped item reaches a REPRESENTATION_MAP and then the mapped
            // representation. That representation owns its geometry and angle
            // context; do not attribute its surfaces to the referring context.
            if child_records
                .iter()
                .any(|child| is_shape_representation_kind(&child.name))
            {
                continue;
            }
            for child in child_records {
                collect_entity_refs(&child.parameter, &mut stack);
            }
        }
    }
    Ok(result)
}

fn collect_entity_refs(parameter: &Parameter, output: &mut Vec<u64>) {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => output.push(*id),
        Parameter::List(values) => values
            .iter()
            .for_each(|value| collect_entity_refs(value, output)),
        Parameter::Typed { parameter, .. } => collect_entity_refs(parameter, output),
        _ => {}
    }
}

fn plane_angle_factor_radians(
    id: u64,
    records: &Records,
    stack: &mut HashSet<u64>,
) -> Result<f64, String> {
    const MAX_CONVERSION_DEPTH: usize = 64;
    if stack.len() >= MAX_CONVERSION_DEPTH {
        return Err(format!(
            "plane-angle conversion chain at #{id} exceeds maximum depth {MAX_CONVERSION_DEPTH}"
        ));
    }
    if !stack.insert(id) {
        return Err(format!("cyclic plane-angle conversion at #{id}"));
    }
    let entity_records = records
        .get(&id)
        .ok_or_else(|| format!("missing plane-angle unit #{id}"))?;
    if unit_kind(id, entity_records)? != UnitKind::PlaneAngle {
        return Err(format!(
            "conversion base entity #{id} is not a PLANE_ANGLE_UNIT"
        ));
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
            "plane-angle unit #{id} must contain exactly one SI_UNIT or CONVERSION_BASED_UNIT"
        ));
    }
    let factor = if let Some(record) = si_units.first() {
        let params = list_params(record, 2)?;
        let prefix = match &params[0] {
            Parameter::Enumeration(prefix) => Some(prefix.as_str()),
            Parameter::NotProvided | Parameter::Omitted => None,
            other => {
                return Err(format!(
                    "unsupported plane-angle SI prefix in #{id}: {other:?}"
                ));
            }
        };
        let name = match &params[1] {
            Parameter::Enumeration(name) => name.as_str(),
            other => {
                return Err(format!(
                    "unsupported plane-angle SI unit in #{id}: {other:?}"
                ));
            }
        };
        if name != "RADIAN" {
            return Err(format!(
                "plane-angle SI unit #{id} uses unsupported unit {name}"
            ));
        }
        si_prefix_factor(prefix)
            .ok_or_else(|| format!("unsupported SI prefix {prefix:?} in plane-angle unit #{id}"))?
            / 1000.0
    } else {
        let record = conversion_units[0];
        let params = list_params(record, 2)?;
        let measure_id = params.get(1).and_then(entity_ref).ok_or_else(|| {
            format!("conversion-based plane-angle unit #{id} has no measure reference")
        })?;
        let measure_records = records
            .get(&measure_id)
            .ok_or_else(|| format!("plane-angle conversion measure #{measure_id} is missing"))?;
        let measures = measure_records
            .iter()
            .filter(|r| r.name == "PLANE_ANGLE_MEASURE_WITH_UNIT")
            .collect::<Vec<_>>();
        if measures.len() != 1 {
            return Err(format!(
                "plane-angle conversion measure #{measure_id} has {} PLANE_ANGLE_MEASURE_WITH_UNIT records; exactly one is required",
                measures.len()
            ));
        }
        let params = list_params(measures[0], 2)?;
        let value = plane_angle_measure(&params[0])?;
        let base_id = entity_ref(&params[1])
            .ok_or_else(|| format!("plane-angle measure #{measure_id} has no unit reference"))?;
        value * plane_angle_factor_radians(base_id, records, stack)?
    };
    stack.remove(&id);
    if !factor.is_finite() || factor <= 0.0 {
        return Err(format!(
            "invalid plane-angle conversion factor for unit #{id}: {factor}"
        ));
    }
    Ok(factor)
}

fn plane_angle_measure(parameter: &Parameter) -> Result<f64, String> {
    let Parameter::Typed { keyword, parameter } = parameter else {
        return Err(format!(
            "expected typed PLANE_ANGLE_MEASURE, got {parameter:?}"
        ));
    };
    if keyword != "PLANE_ANGLE_MEASURE" {
        return Err(format!("expected PLANE_ANGLE_MEASURE, got {keyword}"));
    }
    let value = match parameter.as_ref() {
        Parameter::Real(value) => *value,
        Parameter::Integer(value) => *value as f64,
        other => return Err(format!("unsupported PLANE_ANGLE_MEASURE value {other:?}")),
    };
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid PLANE_ANGLE_MEASURE value {value}"));
    }
    Ok(value)
}

pub fn shape_representation_items(step: &str) -> Result<HashMap<u64, Vec<u64>>, String> {
    let exchange = parser::parse(step).map_err(|error| format!("STEP AST parse: {error}"))?;
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

fn unit_kind(id: u64, entity_records: &[Record]) -> Result<UnitKind, String> {
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

fn unit_factor_mm(id: u64, records: &Records, stack: &mut HashSet<u64>) -> Result<f64, String> {
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
    let metres_per_unit = match prefix {
        None => 1.0,
        Some("EXA") => 1e18,
        Some("PETA") => 1e15,
        Some("TERA") => 1e12,
        Some("GIGA") => 1e9,
        Some("MEGA") => 1e6,
        Some("KILO") => 1e3,
        Some("HECTO") => 1e2,
        Some("DECA") => 1e1,
        Some("DECI") => 1e-1,
        Some("CENTI") => 1e-2,
        Some("MILLI") => 1e-3,
        Some("MICRO") => 1e-6,
        Some("NANO") => 1e-9,
        Some("PICO") => 1e-12,
        Some("FEMTO") => 1e-15,
        Some("ATTO") => 1e-18,
        Some(_) => return None,
    };
    Some(metres_per_unit * 1000.0)
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

pub fn normalized_placement_mm(
    placement: &NodeMatrix,
    child_mm_per_unit: f64,
    parent_mm_per_unit: f64,
) -> Result<Matrix4, String> {
    let matrix = match placement {
        NodeMatrix::Identity if same_factor(child_mm_per_unit, parent_mm_per_unit) => {
            Matrix4::identity()
        }
        NodeMatrix::Identity => {
            return Err(
                "identity assembly placement cannot bridge different representation units".into(),
            );
        }
        NodeMatrix::Transform(transformation) => transformation
            .normalized_matrix_mm(child_mm_per_unit, parent_mm_per_unit)
            .map_err(|error| format!("normalized placement: {error}"))?,
    };
    validate_placement_matrix(matrix).map_err(|error| format!("normalized placement: {error}"))?;
    Ok(matrix)
}

fn normalize_plane_to_mm(plane: &mut Plane, factor: f64) {
    // UV lengths scale separately; retain frame axes without world endpoint rounding.
    let frame = Matrix4::from_cols(
        plane.axis_u().extend(0.0),
        plane.axis_v().extend(0.0),
        plane.normal().extend(0.0),
        (plane.origin() * factor).to_homogeneous(),
    );
    *plane = Plane::xy().transformed(frame);
}

pub fn normalize_shell_to_mm(shell: &mut Shell, mm_per_unit: f64) -> Result<(), String> {
    let scale = Matrix4::from_scale(mm_per_unit);
    for vertex in &mut shell.vertices {
        vertex.transform_by(scale);
    }
    for edge in &mut shell.edges {
        edge.curve = normalize_curve_with_plane_pcurves(edge.curve.clone(), mm_per_unit)?;
    }
    for face in &mut shell.faces {
        if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut face.surface {
            normalize_plane_to_mm(plane, mm_per_unit);
        } else {
            face.surface.transform_by(scale);
        }
    }
    Ok(())
}

fn normalize_curve_with_plane_pcurves(mut curve: Curve3D, factor: f64) -> Result<Curve3D, String> {
    let scale = Matrix4::from_scale(factor);
    if let Curve3D::ParameterCurve(parameter_curve) = &curve {
        return Ok(Curve3D::ParameterCurve(normalize_parameter_curve(
            parameter_curve.clone(),
            factor,
        )));
    }
    let Curve3D::SurfaceCurve(surface_curve) = &curve else {
        curve.transform_by(scale);
        return Ok(curve);
    };
    let active_trim = surface_curve.active_trim();
    let mut leader = surface_curve.leader().clone();
    leader.transform_by(scale);
    let associated = surface_curve
        .associated_geometry()
        .iter()
        .cloned()
        .map(|geometry| match geometry {
            SurfaceCurveAssociatedGeometry::ParameterCurve(parameter_curve) => {
                SurfaceCurveAssociatedGeometry::ParameterCurve(normalize_parameter_curve(
                    parameter_curve,
                    factor,
                ))
            }
            SurfaceCurveAssociatedGeometry::Surface(mut surface) => {
                if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut *surface {
                    normalize_plane_to_mm(plane, factor);
                } else {
                    surface.transform_by(scale);
                }
                SurfaceCurveAssociatedGeometry::Surface(surface)
            }
        })
        .collect();
    let normalized = SurfaceCurve3D::new(
        surface_curve.kind(),
        Box::new(leader),
        associated,
        surface_curve.master_representation(),
    )
    .try_with_active_trim(active_trim)
    .ok_or("unit normalization could not preserve the selected source active interval")?;
    Ok(Curve3D::SurfaceCurve(normalized))
}

fn normalize_parameter_curve(mut curve: StepParameterCurve, factor: f64) -> StepParameterCurve {
    let scale = Matrix4::from_scale(factor);
    let (mut uv_curve, mut surface) = curve.clone().decompose();
    if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut *surface {
        uv_curve.transform_by(Matrix3::from_scale(factor));
        normalize_plane_to_mm(plane, factor);
        StepParameterCurve::new(uv_curve, surface)
    } else {
        curve.transform_by(scale);
        curve
    }
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
mod tests {
    use super::*;

    fn exchange(data: &str) -> String {
        format!(
            "ISO-10303-21;HEADER;FILE_DESCRIPTION(('unit resolver test'),'2;1');FILE_NAME('units.step','','',(),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;{data}ENDSEC;END-ISO-10303-21;"
        )
    }

    #[test]
    fn recognizes_shape_representations_but_not_relationship_records() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('plain',(),#2);\
             #4=ADVANCED_BREP_SHAPE_REPRESENTATION('brep',(),#2);\
             #5=GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION('wire',(),#2);\
             #6=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#7,#8);\
             #7=REPRESENTATION_RELATIONSHIP('placement','',#3,#4);\
             #8=REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#7,#9);\
             #9=ITEM_DEFINED_TRANSFORMATION('','',#10,#11);\
             #10=AXIS2_PLACEMENT_3D('',#12,#13,#14);\
             #11=AXIS2_PLACEMENT_3D('',#12,#13,#14);\
             #12=CARTESIAN_POINT('',(0.,0.,0.));\
             #13=DIRECTION('',(0.,0.,1.));\
             #14=DIRECTION('',(1.,0.,0.));\
             ",
        );
        assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
        assert!(is_shape_representation_kind("SHAPE_REPRESENTATION"));
        assert!(is_shape_representation_kind(
            "ADVANCED_BREP_SHAPE_REPRESENTATION"
        ));
        assert!(is_shape_representation_kind(
            "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION"
        ));
        assert!(!is_shape_representation_kind(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
        ));
    }

    #[test]
    fn resolves_only_context_linked_typed_units() {
        let step = exchange(
            "/* #99 = (LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.KILO.,.METRE.)); */\
             #1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('unused',#404));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('decoy GLOBAL_UNIT_ASSIGNED_CONTEXT SI_UNIT',(),#3);\
             #5=PRODUCT('GLOBAL_UNIT_ASSIGNED_CONTEXT((#2))', 'quoted #99 = SI_UNIT', #4, #4);\
             ",
        );
        assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
    }

    #[test]
    fn resolves_conversion_based_chains_and_si_prefixes() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#1);\
             #3=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('INCH',#2));\
             #4=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(12.),#3);\
             #5=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('FOOT',#4));\
             #6=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#5)));\
             #7=ADVANCED_BREP_SHAPE_REPRESENTATION('shape',(),#6);\
             ",
        );
        let factor = resolve_mm_per_unit(&step).unwrap();
        assert!((factor - 304.8).abs() < 1e-10, "factor={factor}");
    }

    #[test]
    fn accepts_equivalent_si_and_conversion_context_factors() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#2);\
             #4=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('MILLIMETRE',#3));\
             #5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #6=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#4)));\
             #7=SHAPE_REPRESENTATION('si',(),#5);\
             #8=SHAPE_REPRESENTATION('conversion',(),#6);\
             ",
        );
        assert!((resolve_mm_per_unit(&step).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn validates_length_angle_and_solid_angle_assigned_units() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #3=(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT());\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3)));\
             #5=SHAPE_REPRESENTATION('shape',(),#4);\
             ",
        );
        assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
    }

    #[test]
    fn defaults_absent_length_assignment_to_mm_but_validates_context() {
        let missing = exchange(
            "#1=REPRESENTATION_CONTEXT('3D','MODEL');\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
        );
        assert_eq!(resolve_mm_per_unit(&missing).unwrap(), 1.0);

        let non_length_units = exchange(
            "#1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT());\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
        );
        assert_eq!(resolve_mm_per_unit(&non_length_units).unwrap(), 1.0);

        let malformed_empty = exchange(
            "#1=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(()));\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&malformed_empty)
                .unwrap_err()
                .contains("empty GLOBAL_UNIT_ASSIGNED_CONTEXT")
        );

        let dangling_context = exchange(
            "#1=SHAPE_REPRESENTATION('shape',(),#404);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&dangling_context)
                .unwrap_err()
                .contains("references missing context entity #404")
        );

        let wrong_context_type = exchange(
            "#1=PRODUCT('not a representation context','','',());\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&wrong_context_type)
                .unwrap_err()
                .contains("0 REPRESENTATION_CONTEXT records")
        );
    }

    #[test]
    fn rejects_conflicting_and_mixed_context_units() {
        let conflicting = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&conflicting)
                .unwrap_err()
                .contains("assigns 2 length units")
        );

        let mixed = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#2)));\
             #5=SHAPE_REPRESENTATION('shape-mm',(),#3);\
             #6=SHAPE_REPRESENTATION('shape-m',(),#4);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&mixed)
                .unwrap_err()
                .contains("mixed shape representation")
        );
    }

    #[test]
    fn rejects_tiny_but_materially_different_context_factors() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.ATTO.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.FEMTO.,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#2)));\
             #5=SHAPE_REPRESENTATION('atto',(),#3);\
             #6=SHAPE_REPRESENTATION('femto',(),#4);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&step)
                .unwrap_err()
                .contains("mixed shape representation")
        );
    }

    #[test]
    fn rejects_overdeep_conversion_chains() {
        let mut data = String::from("#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\n");
        let mut base = 1_u64;
        for index in 0..70_u64 {
            let measure = 1000 + index;
            let unit = 2 + index;
            data.push_str(&format!(
                "#{measure}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#{base});\n\
                 #{unit}=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('CHAIN',#{measure}));\n"
            ));
            base = unit;
        }
        data.push_str(&format!(
            "#2000=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#{base})));\n\
             #2001=SHAPE_REPRESENTATION('deep',(),#2000);\n"
        ));
        assert!(
            resolve_mm_per_unit(&exchange(&data))
                .unwrap_err()
                .contains("exceeds maximum depth")
        );
    }

    #[test]
    fn conversion_chain_depth_boundary_is_explicit() {
        fn chain(conversion_units: u64) -> String {
            let mut data = String::from("#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\n");
            let mut base = 1_u64;
            for index in 0..conversion_units {
                let measure = 1000 + index;
                let unit = 2 + index;
                data.push_str(&format!(
                    "#{measure}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#{base});\n\
                     #{unit}=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('CHAIN',#{measure}));\n"
                ));
                base = unit;
            }
            data.push_str(&format!(
                "#2000=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#{base})));\n\
                 #2001=SHAPE_REPRESENTATION('depth',(),#2000);\n"
            ));
            exchange(&data)
        }
        assert!(resolve_mm_per_unit(&chain(63)).is_ok());
        assert!(
            resolve_mm_per_unit(&chain(64))
                .unwrap_err()
                .contains("exceeds maximum depth 64")
        );
    }

    #[test]
    fn rejects_cyclic_conversion_chains() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('LOOP',#2));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('cycle',(),#3);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&step)
                .unwrap_err()
                .contains("cyclic length unit conversion")
        );
    }

    #[test]
    fn rejects_unit_with_conflicting_si_and_conversion_records() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) CONVERSION_BASED_UNIT('conflict',#2));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&step)
                .unwrap_err()
                .contains("has 2 SI_UNIT, CONVERSION_BASED_UNIT, or DERIVED_UNIT records")
        );
    }

    #[test]
    fn rejects_dangling_and_non_unit_assignment_references() {
        let dangling = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#404)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&dangling)
                .unwrap_err()
                .contains("references missing entity #404")
        );

        let wrong_type = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=PRODUCT('not a unit','','',());\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&wrong_type)
                .unwrap_err()
                .contains("not exactly one supported named")
        );
    }

    #[test]
    fn rejects_assignment_on_non_representation_context_entity() {
        let step = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&step)
                .unwrap_err()
                .contains("has 0 REPRESENTATION_CONTEXT records")
        );
    }

    #[test]
    fn rejects_nonlength_conversion_base() {
        let nonlength_base = exchange(
            "#1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('BAD_LENGTH',#2));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#3)));\
             #5=SHAPE_REPRESENTATION('shape',(),#4);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&nonlength_base)
                .unwrap_err()
                .contains("conversion base entity #1 is not a LENGTH_UNIT")
        );
    }

    #[test]
    fn rejects_duplicate_si_definition_records() {
        let duplicate_si = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) SI_UNIT(.CENTI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&duplicate_si)
                .unwrap_err()
                .contains("has 2 SI_UNIT, CONVERSION_BASED_UNIT, or DERIVED_UNIT records")
        );
    }

    #[test]
    fn rejects_duplicate_context_assignment_records() {
        let duplicate_context_assignment = exchange(
            "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)) GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
        );
        assert!(
            resolve_mm_per_unit(&duplicate_context_assignment)
                .unwrap_err()
                .contains("has 2 GLOBAL_UNIT_ASSIGNED_CONTEXT records")
        );
    }
}

#[cfg(test)]
mod direct_parameter_curve_tests {
    use super::*;
    use monstertruck_io::step::load::step_geometry::{Curve2D, StepParameterCurve};
    use monstertruck_modeling::{
        Line, MetricSpace, ParametricCurve, ParametricSurface, Point2, Point3, Tolerance,
    };

    #[test]
    fn normalization_preserves_plane_frames_in_faces_and_all_associations() {
        use monstertruck_io::step::load::step_geometry::{
            SurfaceCurveKind, SurfaceCurveRepresentation,
        };
        use monstertruck_modeling::Vector3;
        use monstertruck_topology::compress::{CompressedEdge, CompressedFace};

        for factor in [1.0, 25.4, 1000.0] {
            let origin = Point3::new(1e12, 1e12, 1e12);
            let plane = Plane::xy().transformed(Matrix4::new(
                1., 0., -1e-12, 0., 0., 1., 0., 0., 1e-12, 0., 1., 0., origin.x, origin.y,
                origin.z, 1.,
            ));
            let surface = Surface::ElementarySurface(ElementarySurface::Plane(plane));
            let pcurve = StepParameterCurve::new(
                Box::new(Curve2D::Line(Line(
                    Point2::new(0., 0.),
                    Point2::new(1., 0.),
                ))),
                Box::new(surface.clone()),
            );
            let mut shell = Shell {
                vertex_stable_ids: None,
                edge_stable_ids: None,
                face_stable_ids: None,
                vertices: vec![origin, origin + Vector3::unit_x()],
                edges: vec![
                    CompressedEdge {
                        vertices: (0, 1),
                        curve: Curve3D::ParameterCurve(pcurve.clone()),
                    },
                    CompressedEdge {
                        vertices: (0, 1),
                        curve: Curve3D::SurfaceCurve(SurfaceCurve3D::new(
                            SurfaceCurveKind::SurfaceCurve,
                            Box::new(Curve3D::Line(Line(origin, origin + Vector3::unit_x()))),
                            vec![
                                SurfaceCurveAssociatedGeometry::Surface(Box::new(surface.clone())),
                                SurfaceCurveAssociatedGeometry::ParameterCurve(pcurve),
                            ],
                            SurfaceCurveRepresentation::Curve3D,
                        )),
                    },
                ],
                faces: vec![CompressedFace {
                    boundaries: vec![],
                    orientation: true,
                    surface,
                }],
            };
            normalize_shell_to_mm(&mut shell, factor).unwrap();
            let check_plane = |surface: &Surface| {
                let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = surface else {
                    panic!("normalization must retain the native Plane carrier");
                };
                assert_eq!(plane.origin(), origin * factor);
                assert_eq!(plane.axis_u(), Vector3::new(1., 0., -1e-12));
                assert_eq!(plane.axis_v(), Vector3::unit_y());
            };
            check_plane(&shell.faces[0].surface);
            let Curve3D::ParameterCurve(direct) = &shell.edges[0].curve else {
                unreachable!()
            };
            check_plane(direct.surface());
            assert_eq!(direct.curve().subs(1.).x, factor);
            let Curve3D::SurfaceCurve(associated) = &shell.edges[1].curve else {
                unreachable!()
            };
            for geometry in associated.associated_geometry() {
                match geometry {
                    SurfaceCurveAssociatedGeometry::Surface(surface) => check_plane(surface),
                    SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => {
                        check_plane(curve.surface());
                        assert_eq!(curve.curve().subs(1.).x, factor);
                    }
                }
            }
        }
    }

    #[test]
    fn normalization_keeps_selected_pcurve_active_scalar_labels() {
        use monstertruck_io::step::load::step_geometry::{
            SurfaceCurveKind, SurfaceCurveRepresentation,
        };
        let surface =
            Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy().transformed(
                Matrix4::from_translation(monstertruck_modeling::Vector3::new(10., 20., 30.)),
            )));
        let source = SurfaceCurve3D::new(
            SurfaceCurveKind::SurfaceCurve,
            Box::new(Curve3D::Line(Line(
                Point3::new(100., 100., 100.),
                Point3::new(101., 100., 100.),
            ))),
            vec![
                SurfaceCurveAssociatedGeometry::Surface(Box::new(surface.clone())),
                SurfaceCurveAssociatedGeometry::ParameterCurve(StepParameterCurve::new(
                    Box::new(Curve2D::Line(Line(
                        Point2::new(2., 3.),
                        Point2::new(4., 6.),
                    ))),
                    Box::new(surface),
                )),
            ],
            SurfaceCurveRepresentation::ParameterCurve1,
        )
        .try_with_active_trim(Some((2., 10.)))
        .unwrap();
        for factor in [1., 25.4, 1000.] {
            let normalized =
                normalize_curve_with_plane_pcurves(Curve3D::SurfaceCurve(source.clone()), factor)
                    .unwrap();
            let Curve3D::SurfaceCurve(normalized) = normalized else {
                panic!("lost surface curve")
            };
            assert_eq!(normalized.active_trim(), Some((2., 10.)));
            assert!(
                normalized
                    .subs(2.)
                    .distance(Point3::new(16., 29., 30.) * factor)
                    < 1e-8
            );
            assert!(
                normalized
                    .subs(10.)
                    .distance(Point3::new(32., 53., 30.) * factor)
                    < 1e-8
            );
            assert_eq!(source.active_trim(), Some((2., 10.)));
        }
        assert!(
            normalize_curve_with_plane_pcurves(Curve3D::SurfaceCurve(source), f64::MAX).is_err()
        );
    }

    #[test]
    fn direct_plane_parameter_curve_scales_uv_without_scaling_frame_axes() {
        let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
        let source = StepParameterCurve::new(
            Box::new(Curve2D::Line(Line(
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
            ))),
            Box::new(surface.clone()),
        );
        let expected = surface.subs(1.0, 0.0) * 1000.0;
        let normalized = normalize_parameter_curve(source.clone(), 1000.0);
        let (uv, _) = normalized.decompose();
        let actual_uv = uv.subs(1.0);
        let normalized_face = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
        let actual = normalized_face.subs(actual_uv.x, actual_uv.y);
        assert!(actual.near(&expected));
        assert!(actual.near(&Point3::new(1000.0, 0.0, 0.0)));

        // The previous generic transform scaled the pcurve plane but left its UV curve unchanged.
        let mut baseline = source;
        baseline.transform_by(Matrix4::from_scale(1000.0));
        let (uv, _) = baseline.decompose();
        let baseline_uv = uv.subs(1.0);
        let old_boundary = normalized_face.subs(baseline_uv.x, baseline_uv.y);
        assert!(!old_boundary.near(&expected));
    }
}

#[cfg(test)]
mod plane_angle_normalization_tests {
    use super::*;

    const DEGREE_RAD: f64 = 0.0174532925;

    fn angle_exchange(angle: &str, assignment: &str, extra: &str) -> String {
        format!(
            "ISO-10303-21;HEADER;FILE_DESCRIPTION(('angle units'),'2;1');FILE_NAME('angle.step','','',(),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
             #1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE({DEGREE_RAD}),#1);\
             #3=(CONVERSION_BASED_UNIT('DEGREE',#2) NAMED_UNIT(#4) PLANE_ANGLE_UNIT());\
             #4=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\
             #5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(({assignment})));\
             #6=CONICAL_SURFACE('',#9,1.,{angle});\
             #7=SHAPE_REPRESENTATION('cone',(#6),#5);\
             #9=AXIS2_PLACEMENT_3D('',#10,#11,#12);\
             #10=CARTESIAN_POINT('',(0.,0.,0.));\
             #11=DIRECTION('',(0.,0.,1.));\
             #12=DIRECTION('',(1.,0.,0.));\
             {extra}ENDSEC;END-ISO-10303-21;"
        )
    }

    fn normalized_angle(step: &str) -> f64 {
        let exchange = normalize_conical_angles(step).unwrap();
        let EntityInstance::Simple { record, .. } = exchange.data[0]
            .entities
            .iter()
            .find(|entity| match entity {
                EntityInstance::Simple { record, .. } => record.name == "CONICAL_SURFACE",
                EntityInstance::Complex { subsuper, .. } => {
                    subsuper.0.iter().any(|r| r.name == "CONICAL_SURFACE")
                }
            })
            .unwrap()
        else {
            panic!("expected simple conical surface record")
        };
        let Parameter::List(params) = &record.parameter else {
            panic!("expected conical parameters")
        };
        match params[3] {
            Parameter::Real(value) => value,
            _ => panic!("expected real angle"),
        }
    }

    #[test]
    fn degrees_and_radians_are_normalized_by_declared_context_for_small_and_large_values() {
        for degrees in [2.0_f64, 45.0] {
            let degrees_text = angle_exchange(&degrees.to_string(), "#3", "");
            let radians = degrees * DEGREE_RAD;
            let radians_text = angle_exchange(&radians.to_string(), "#1", "");
            let actual = normalized_angle(&degrees_text);
            let expected = normalized_angle(&radians_text);
            assert!(
                (actual - expected).abs() < 1e-14,
                "{degrees} degrees -> {actual}, expected {expected}"
            );
            assert!((actual - radians).abs() < 1e-14);
        }
    }

    #[test]
    fn absent_angle_unit_retains_radian_interpretation() {
        let step = angle_exchange("2.0", "", "").replace(
            "#5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(()));",
            "#5=REPRESENTATION_CONTEXT('3D','MODEL');",
        );
        assert_eq!(normalized_angle(&step), 2.0);
    }

    #[test]
    fn keeps_schema_permitted_signed_angles_and_zero_without_guessing_a_range() {
        for angle in [0.0_f64, -0.25, 90.0, 270.0] {
            let step = angle_exchange(&angle.to_string(), "#3", "");
            let normalized = normalized_angle(&step);
            assert!((normalized - angle * DEGREE_RAD).abs() < 1e-14);
        }
    }

    #[test]
    fn rejects_non_finite_normalized_cone_angle() {
        let step = angle_exchange("1.E999", "#3", "");
        assert!(
            normalize_conical_angles(&step)
                .unwrap_err()
                .contains("invalid normalized semi-angle")
        );
    }

    #[test]
    fn conflicting_or_malformed_plane_angle_units_fail_closed() {
        let conflict = angle_exchange("45.", "#1,#3", "");
        assert!(
            normalize_conical_angles(&conflict)
                .unwrap_err()
                .contains("assigns 2 plane-angle units")
        );

        let malformed = angle_exchange(
            "45.",
            "#20",
            "#20=(CONVERSION_BASED_UNIT('BAD',#21) NAMED_UNIT(#4) PLANE_ANGLE_UNIT());#21=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(1.),#404);",
        );
        assert!(
            normalize_conical_angles(&malformed)
                .unwrap_err()
                .contains("missing plane-angle unit #404")
        );
    }

    #[test]
    fn shared_cone_with_conflicting_representation_angle_contexts_is_rejected() {
        let step = angle_exchange(
            "45.",
            "#3",
            "#13=REPRESENTATION_CONTEXT('3D','RADIAN');#14=SHAPE_REPRESENTATION('same',(#6),#13);",
        );
        assert!(
            normalize_conical_angles(&step)
                .unwrap_err()
                .contains("shared by representation contexts with conflicting plane-angle units")
        );
    }

    #[test]
    fn mapped_representation_keeps_its_own_angle_context() {
        let step = angle_exchange(
            "2.0",
            "#3",
            "#13=REPRESENTATION_CONTEXT('3D','MODEL');\
             #14=SHAPE_REPRESENTATION('mapped cone',(#6),#13);\
             #15=REPRESENTATION_MAP(#16,#14);\
             #16=AXIS2_PLACEMENT_3D('',#10,#11,#12);\
             #17=MAPPED_ITEM('mapped',#15,#18);\
             #18=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#11,#12,$,1.);\
             #19=SHAPE_REPRESENTATION('outer',(#17),#5);",
        )
        .replace(
            "#7=SHAPE_REPRESENTATION('cone',(#6),#5);",
            "#7=SHAPE_REPRESENTATION('cone',(),#5);",
        );
        // The mapped geometry's context has no assigned angle unit, so it
        // retains radians even though the outer mapped-item context declares
        // degrees. Traversal through #15 -> #14 must stop at #14.
        assert_eq!(normalized_angle(&step), 2.0);
    }

    #[test]
    fn complex_conical_surface_encoding_is_rejected_explicitly() {
        let step = angle_exchange("2.", "#3", "")
        .replace(
            "#6=CONICAL_SURFACE('',#9,1.,2.);",
            "#6=(CONICAL_SURFACE(2.) ELEMENTARY_SURFACE(#9) SURFACE() GEOMETRIC_REPRESENTATION_ITEM() REPRESENTATION_ITEM(''));",
        );
        assert!(
            normalize_conical_angles(&step)
                .unwrap_err()
                .contains("complex CONICAL_SURFACE #6 encoding is unsupported")
        );
    }

    #[test]
    fn ignores_unowned_orphan_conical_surface() {
        let step = angle_exchange("45.", "#3", "#13=CONICAL_SURFACE('',#9,1.,45.);");
        let exchange = normalize_conical_angles(&step).unwrap();
        let orphan = exchange.data[0]
            .entities
            .iter()
            .find(|entity| matches!(entity, EntityInstance::Simple { id: 13, .. }))
            .unwrap();
        let EntityInstance::Simple { record, .. } = orphan else {
            unreachable!()
        };
        let Parameter::List(params) = &record.parameter else {
            unreachable!()
        };
        assert!(matches!(params[3], Parameter::Real(45.0)));
    }
}

#[cfg(test)]
mod step_byte_decode_tests {
    use super::decode_step_bytes;

    #[test]
    fn ascii_and_valid_utf8_are_preserved_byte_for_byte_as_text() {
        let ascii = b"FILE_NAME('plain ASCII','date');";
        assert_eq!(
            decode_step_bytes(ascii),
            std::str::from_utf8(ascii).unwrap()
        );

        let utf8 = "FILE_NAME('Caf\u{00e9}','date');".as_bytes();
        assert!(std::str::from_utf8(utf8).is_ok());
        assert_eq!(decode_step_bytes(utf8), std::str::from_utf8(utf8).unwrap());
    }

    #[test]
    fn invalid_utf8_uses_the_same_lossless_latin1_mapping_as_from_step_bytes() {
        let mut bytes = b"FILE_NAME('Caf','date');".to_vec();
        bytes.insert(b"FILE_NAME('Caf".len(), 0xe9);
        assert!(std::str::from_utf8(&bytes).is_err());
        let expected = "FILE_NAME('Caf\u{00e9}','date');";
        assert_eq!(decode_step_bytes(&bytes), expected);
        assert_ne!(decode_step_bytes(&bytes), String::from_utf8_lossy(&bytes));
    }
}
