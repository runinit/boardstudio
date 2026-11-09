//! Complete representation-item traversal, including exact mapped occurrences.
mod presentation;
use super::{SourceRecords, named_record, parameters, reference, simple_record, source_records};
use monstertruck_io::step::load::step_p21::ast::{Exchange, Parameter, Record};
use monstertruck_modeling::{InnerSpace, Matrix4, SquareMatrix, Vector3};
use std::collections::{HashMap, HashSet};

// shortcut: refuse mapping nesting beyond 128 levels; use an iterative walk if real inputs need more.
const MAX_MAPPING_DEPTH: usize = 128;

pub(super) struct Scene<'a> {
    records: SourceRecords<'a>,
    items: &'a HashMap<u64, Vec<u64>>,
    units: &'a HashMap<u64, f64>,
    solids: &'a [u64],
}

impl<'a> Scene<'a> {
    pub(super) fn new(
        exchange: &'a Exchange,
        items: &'a HashMap<u64, Vec<u64>>,
        units: &'a HashMap<u64, f64>,
        solids: &'a [u64],
    ) -> Self {
        let records = source_records(exchange);
        Self {
            records,
            items,
            units,
            solids,
        }
    }
    pub(super) fn validate(&self) -> Result<(), String> {
        let mut done = HashSet::new();
        for id in self.items.keys() {
            self.validate_representation(*id, &mut HashSet::new(), &mut done)?;
        }
        Ok(())
    }
    fn validate_representation(
        &self,
        id: u64,
        path: &mut HashSet<u64>,
        done: &mut HashSet<u64>,
    ) -> Result<(), String> {
        if done.contains(&id) {
            return Ok(());
        }
        if path.len() >= MAX_MAPPING_DEPTH || !path.insert(id) {
            return Err("cyclic or excessively deep mapped representation graph".into());
        }
        let items = self
            .items
            .get(&id)
            .ok_or_else(|| format!("missing mapped shape representation #{id}"))?;
        if items.iter().copied().collect::<HashSet<_>>().len() != items.len() {
            return Err("shape representation repeats an item in its SET".into());
        }
        for item in items {
            if self.solids.contains(item) {
                continue;
            }
            let record = self.simple(*item)?;
            if record.name == "MAPPED_ITEM" {
                let params = parameters(record, 3)?;
                let map = parameters(self.named(reference(&params[1])?, "REPRESENTATION_MAP")?, 2)?;
                self.validate_representation(reference(&map[1])?, path, done)?;
            }
        }
        for child in self.related(id)? {
            self.validate_representation(child, path, done)?;
        }
        path.remove(&id);
        done.insert(id);
        Ok(())
    }
    pub(super) fn roots(&self) -> Result<Vec<u64>, String> {
        let mut referenced = HashSet::new();
        for records in self.records.values() {
            if let Some(record) = records.iter().find(|r| r.name == "REPRESENTATION_MAP") {
                let params = parameters(record, 2)?;
                referenced.insert(reference(&params[1])?);
            }
        }
        for from in self.items.keys() {
            referenced.extend(self.related(*from)?);
        }
        let mut roots: Vec<_> = self
            .items
            .keys()
            .copied()
            .filter(|id| !referenced.contains(id))
            .collect();
        roots.sort_unstable();
        if roots.is_empty() {
            return Err("shape representations have no unmapped root (possible cycle)".into());
        }
        Ok(roots)
    }
    pub(super) fn instances(
        &self,
        representation: u64,
        placement: Matrix4,
        budget: &mut super::OccurrenceBudget,
    ) -> Result<Vec<(u64, Matrix4, u64)>, String> {
        let mut result = Vec::new();
        self.walk(
            representation,
            placement,
            &mut HashSet::new(),
            &mut result,
            budget,
        )?;
        Ok(result)
    }
    fn walk(
        &self,
        representation: u64,
        placement: Matrix4,
        path: &mut HashSet<u64>,
        result: &mut Vec<(u64, Matrix4, u64)>,
        budget: &mut super::OccurrenceBudget,
    ) -> Result<(), String> {
        budget.traverse()?;
        if path.len() >= MAX_MAPPING_DEPTH || !path.insert(representation) {
            return Err("cyclic or excessively deep mapped representation graph".into());
        }
        let items = self
            .items
            .get(&representation)
            .ok_or_else(|| format!("missing shape representation #{representation}"))?;
        for id in items {
            budget.traverse()?;
            if self.solids.contains(id) {
                budget.emit()?;
                result.push((*id, placement, representation));
                continue;
            }
            let record = self.simple(*id)?;
            match record.name.as_str() {
                "GEOMETRIC_CURVE_SET" => {
                    budget.emit()?;
                    result.push((*id, placement, representation));
                }
                "AXIS2_PLACEMENT_3D" => {
                    self.frame(*id, self.factor(representation)?)?;
                }
                "MAPPED_ITEM" => {
                    budget.seen_mappings.insert(*id);
                    let mapped = parameters(record, 3)?;
                    if !matches!(mapped[0], Parameter::String(_)) {
                        return Err("malformed mapped item name".into());
                    }
                    let map = self.named(reference(&mapped[1])?, "REPRESENTATION_MAP")?;
                    let map = parameters(map, 2)?;
                    let child = reference(&map[1])?;
                    let child_factor = self.factor(child)?;
                    let parent_factor = self.factor(representation)?;
                    if !super::units::same_factor(child_factor, parent_factor) {
                        return Err(
                            "mapped representations with different length units are unsupported"
                                .into(),
                        );
                    }
                    let origin = self.frame(reference(&map[0])?, child_factor)?;
                    let target = self.operator(reference(&mapped[2])?, parent_factor)?;
                    let next = target * origin.invert().ok_or("singular mapping origin")?;
                    super::units::validate_placement_matrix(next)?;
                    self.walk(child, placement * next, path, result, budget)?;
                }
                kind => {
                    return Err(format!(
                        "unsupported shape representation item #{id}: {kind}"
                    ));
                }
            }
        }
        for child in self.related(representation)? {
            self.walk(child, placement, path, result, budget)?;
        }
        path.remove(&representation);
        Ok(())
    }
    pub(super) fn require_complete_mappings(&self, seen: &HashSet<u64>) -> Result<(), String> {
        for id in self.items.values().flatten() {
            let records = self
                .records
                .get(id)
                .ok_or("missing represented source item")?;
            if records.iter().any(|r| r.name == "MAPPED_ITEM") && !seen.contains(id) {
                return Err(format!(
                    "unreferenced mapped occurrence #{id} was not traversed"
                ));
            }
        }
        Ok(())
    }
    fn related(&self, representation: u64) -> Result<Vec<u64>, String> {
        let mut result = Vec::new();
        for records in self.records.values() {
            if records
                .iter()
                .any(|r| r.name == "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION")
            {
                continue;
            }
            let Some(shape) = records
                .iter()
                .find(|r| r.name == "SHAPE_REPRESENTATION_RELATIONSHIP")
            else {
                continue;
            };
            let relation = if let Some(base) = records
                .iter()
                .find(|r| r.name == "REPRESENTATION_RELATIONSHIP")
            {
                *base
            } else {
                *shape
            };
            let params = parameters(relation, 4)?;
            if reference(&params[2])? == representation {
                result.push(reference(&params[3])?);
            }
        }
        result.sort_unstable();
        Ok(result)
    }
    fn factor(&self, representation: u64) -> Result<f64, String> {
        self.units
            .get(&representation)
            .copied()
            .ok_or_else(|| format!("representation #{representation} has no authenticated units"))
    }
    fn simple(&self, id: u64) -> Result<&Record, String> {
        simple_record(&self.records, id)
    }
    fn named(&self, id: u64, name: &str) -> Result<&Record, String> {
        named_record(&self.records, id, name)
    }
    fn vector(&self, id: u64, kind: &str) -> Result<Vector3, String> {
        let params = parameters(self.named(id, kind)?, 2)?;
        let Parameter::List(values) = &params[1] else {
            return Err(format!("{kind} #{id} lacks coordinates"));
        };
        if values.len() != 3 {
            return Err(format!("{kind} #{id} is not three-dimensional"));
        }
        Ok(Vector3::new(
            real(&values[0])?,
            real(&values[1])?,
            real(&values[2])?,
        ))
    }
    fn direction(&self, param: &Parameter) -> Result<Option<Vector3>, String> {
        if matches!(param, Parameter::NotProvided) {
            return Ok(None);
        }
        let direction = self.vector(reference(param)?, "DIRECTION")?;
        if !direction.magnitude2().is_finite() || direction.magnitude2() == 0. {
            return Err("invalid mapped placement direction".into());
        }
        Ok(Some(direction.normalize()))
    }
    fn axes(
        &self,
        z: Option<Vector3>,
        x: Option<Vector3>,
    ) -> Result<(Vector3, Vector3, Vector3), String> {
        let z = z.unwrap_or_else(Vector3::unit_z);
        let fallback = if z == Vector3::unit_x() || z == -Vector3::unit_x() {
            Vector3::unit_y()
        } else {
            Vector3::unit_x()
        };
        let x = x.unwrap_or(fallback);
        let x = x - z * x.dot(z);
        if !x.magnitude2().is_finite() || x.magnitude2() <= 1e-24 {
            return Err("parallel mapped placement axes".into());
        }
        let x = x.normalize();
        Ok((x, z.cross(x), z))
    }
    fn frame(&self, id: u64, factor: f64) -> Result<Matrix4, String> {
        let params = parameters(self.named(id, "AXIS2_PLACEMENT_3D")?, 4)?;
        let origin = self.vector(reference(&params[1])?, "CARTESIAN_POINT")? * factor;
        let (x, y, z) = self.axes(self.direction(&params[2])?, self.direction(&params[3])?)?;
        let matrix =
            Matrix4::from_cols(x.extend(0.), y.extend(0.), z.extend(0.), origin.extend(1.));
        super::units::validate_placement_matrix(matrix)?;
        Ok(matrix)
    }
    fn operator(&self, id: u64, factor: f64) -> Result<Matrix4, String> {
        let params = parameters(self.named(id, "CARTESIAN_TRANSFORMATION_OPERATOR_3D")?, 7)?;
        if !matches!(params[0], Parameter::String(_))
            || !matches!(params[1], Parameter::String(_) | Parameter::NotProvided)
        {
            return Err("malformed mapped operator metadata".into());
        }
        let scale = if matches!(params[5], Parameter::NotProvided) {
            1.
        } else {
            real(&params[5])?
        };
        if scale <= 0. {
            return Err("mapped scale must be positive".into());
        }
        let origin = self.vector(reference(&params[4])?, "CARTESIAN_POINT")? * factor;
        let (x, mut y, z) = self.axes(self.direction(&params[6])?, self.direction(&params[2])?)?;
        if let Some(second) = self.direction(&params[3])? {
            if second.dot(y).abs() <= 1e-12 {
                return Err("invalid mapped second axis".into());
            }
            if second.dot(y) < 0. {
                y = -y;
            }
        }
        Ok(Matrix4::from_cols(
            (x * scale).extend(0.),
            (y * scale).extend(0.),
            (z * scale).extend(0.),
            origin.extend(1.),
        ))
    }
}
fn real(parameter: &Parameter) -> Result<f64, String> {
    let value = match parameter {
        Parameter::Real(v) => *v,
        Parameter::Integer(v) => *v as f64,
        _ => return Err("expected mapped numeric value".into()),
    };
    if !value.is_finite() {
        return Err("nonfinite mapped value".into());
    }
    Ok(value)
}
