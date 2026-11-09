//! Authenticated length uncertainty attached to the owning representation context.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct StepUncertainty {
    pub entity_id: u64,
    pub length_unit_id: u64,
    pub millimetres: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepPrecision {
    pub representation_id: u64,
    pub context_id: u64,
    pub geometric_dimension: Option<u32>,
    pub declarations: Vec<StepUncertainty>,
}

impl StepPrecision {
    /// One context-owned declaration; repair still requires carrier authority.
    /// Multiple declarations are retained without choosing an allowance.
    pub fn uncertainty_mm(&self) -> Option<f64> {
        if self.geometric_dimension != Some(3) {
            return None;
        }
        match self.declarations.as_slice() {
            [declaration] => Some(declaration.millimetres),
            _ => None,
        }
    }
}

pub(in crate::step) fn resolve(exchange: &Exchange) -> Result<Vec<StepPrecision>, String> {
    if exchange.data.len() != 1 {
        return Err("precision requires one DATA section".into());
    }
    let mut records = Records::new();
    for instance in &exchange.data[0].entities {
        let (id, parts) = match instance {
            EntityInstance::Simple { id, record } => (*id, vec![record.clone()]),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.clone()),
        };
        if records.insert(id, parts).is_some() {
            return Err("duplicate precision entity identity".into());
        }
    }
    let mut result = Vec::new();
    for (id, parts) in &records {
        let Some(representation) = parts.iter().find(|r| is_shape_representation_kind(&r.name))
        else {
            continue;
        };
        let params = list_params(representation, 3)?;
        let context_id = entity_ref(&params[2]).ok_or("representation precision lacks context")?;
        let context = records
            .get(&context_id)
            .ok_or("representation precision context is missing")?;
        if context
            .iter()
            .filter(|r| r.name == "REPRESENTATION_CONTEXT")
            .count()
            != 1
        {
            return Err("representation precision context is not authenticated".into());
        }
        let geometric: Vec<_> = context
            .iter()
            .filter(|r| r.name == "GEOMETRIC_REPRESENTATION_CONTEXT")
            .collect();
        if geometric.len() > 1 {
            return Err("duplicate geometric source context".into());
        }
        let geometric_dimension = if let Some(record) = geometric.first() {
            match list_params(record, 1)? {
                [Parameter::Integer(3)] => Some(3),
                _ => return Err("unsupported source geometric context dimension".into()),
            }
        } else {
            None
        };
        let assignments: Vec<_> = context
            .iter()
            .filter(|r| r.name == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT")
            .collect();
        if assignments.len() > 1 {
            return Err("duplicate precision assignments".into());
        }
        let mut declarations = Vec::new();
        if let Some(assignment) = assignments.first() {
            let params = list_params(assignment, 1)?;
            if params.len() != 1 {
                return Err("malformed precision assignment".into());
            }
            let Parameter::List(values) = &params[0] else {
                return Err("precision assignment has no SET".into());
            };
            if values.is_empty() {
                return Err("empty precision assignment".into());
            }
            let mut seen = HashSet::new();
            for value in values {
                let uncertainty_id = entity_ref(value).ok_or("non-entity precision reference")?;
                if !seen.insert(uncertainty_id) {
                    return Err("duplicate precision reference".into());
                }
                let parts = records
                    .get(&uncertainty_id)
                    .ok_or("missing precision declaration")?;
                let uncertainty: Vec<_> = parts
                    .iter()
                    .filter(|r| r.name == "UNCERTAINTY_MEASURE_WITH_UNIT")
                    .collect();
                if uncertainty.len() != 1 {
                    return Err("precision reference is not one uncertainty declaration".into());
                }
                let params = list_params(uncertainty[0], 2)?;
                let measure = if parts.len() == 1 {
                    if params.len() != 4
                        || !matches!(params[2], Parameter::String(_))
                        || !matches!(params[3], Parameter::String(_) | Parameter::NotProvided)
                    {
                        return Err("malformed uncertainty metadata".into());
                    }
                    params
                } else {
                    let base: Vec<_> = parts
                        .iter()
                        .filter(|r| r.name == "MEASURE_WITH_UNIT")
                        .collect();
                    if base.len() != 1
                        || params.len() != 2
                        || !matches!(params[0], Parameter::String(_))
                        || !matches!(params[1], Parameter::String(_) | Parameter::NotProvided)
                    {
                        return Err("malformed complex uncertainty".into());
                    }
                    let measure = list_params(base[0], 2)?;
                    if measure.len() != 2 {
                        return Err("malformed uncertainty measure".into());
                    }
                    measure
                };
                let unit_id = entity_ref(&measure[1]).ok_or("uncertainty has no unit identity")?;
                let unit = records.get(&unit_id).ok_or("uncertainty unit is missing")?;
                if unit_kind(unit_id, unit)? != UnitKind::Length {
                    continue;
                }
                let value = length_measure(&measure[0])?;
                let factor = unit_factor_mm(unit_id, &records, &mut HashSet::new())?;
                let millimetres = value * factor;
                if !millimetres.is_finite() || millimetres <= 0. {
                    return Err("invalid physical uncertainty".into());
                }
                declarations.push(StepUncertainty {
                    entity_id: uncertainty_id,
                    length_unit_id: unit_id,
                    millimetres,
                });
            }
        }
        result.push(StepPrecision {
            representation_id: *id,
            context_id,
            geometric_dimension,
            declarations,
        });
    }
    result.sort_by_key(|p| p.representation_id);
    Ok(result)
}
