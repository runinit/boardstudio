//! Shared STEP record and named-unit decoding used by import and normalization.
#![allow(missing_docs)]
use std::collections::{HashMap, HashSet};
use step_p21::ast::{Name, Parameter, Record};
pub type Records = HashMap<u64, Vec<Record>>;

pub fn list_params(record: &Record, count: usize) -> Result<&[Parameter], String> {
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

pub fn entity_ref(parameter: &Parameter) -> Option<u64> {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Length,
    PlaneAngle,
    SolidAngle,
}

pub fn unit_kind(id: u64, entity_records: &[Record]) -> Result<UnitKind, String> {
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

pub fn unit_factor_mm(id: u64, records: &Records, stack: &mut HashSet<u64>) -> Result<f64, String> {
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

pub fn length_measure(parameter: &Parameter) -> Result<f64, String> {
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

pub fn si_prefix_factor(prefix: Option<&str>) -> Option<f64> {
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

pub fn is_shape_representation_kind(name: &str) -> bool {
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

/// Selects the inherited representation fields for simple or complex shapes.
/// A complex subtype marker has no repeated inherited parameters.
pub fn shape_representation_parameters(records: &[Record]) -> Result<Option<&[Parameter]>, String> {
    if !records
        .iter()
        .any(|r| is_shape_representation_kind(&r.name))
    {
        return Ok(None);
    }
    let mut definitions = records.iter().filter(|r| {
        r.name == "REPRESENTATION"
            || (is_shape_representation_kind(&r.name)
                && !matches!(&r.parameter, Parameter::List(p) if p.is_empty()))
    });
    let record = definitions
        .next()
        .ok_or("shape representation has no inherited definition")?;
    if definitions.next().is_some() {
        return Err("duplicate shape representation definitions".into());
    }
    let p = list_params(record, 3)?;
    if p.len() != 3 {
        return Err("shape representation must have exactly three inherited parameters".into());
    }
    Ok(Some(p))
}
