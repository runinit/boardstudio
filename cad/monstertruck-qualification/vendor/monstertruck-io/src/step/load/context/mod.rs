//! Source representation ownership and declared connectivity accuracy.
//!
//! Accuracy is interpreted as a bound on direct asserted attachments. It is
//! independent of tessellation error, manufacturing variation, and parameter tolerances.
use super::*;
use std::collections::{HashMap, HashSet};
use step_p21::ast::Record;
use units::{Records, UnitKind, entity_ref, length_measure, unit_factor_mm, unit_kind};

pub mod normalization;
pub mod units;

/// Original parsed records. Fields are opaque: only parsed source can authorize
/// a connectivity bound, and conversion also checks the current geometry binding.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceRecords {
    records: Records,
    complex: HashSet<u64>,
    duplicates: HashSet<u64>,
}

/// A selected source declaration, expressed in the representation's model units.
/// This describes attachment discrepancy, never a total mesh error budget.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectivityAccuracy {
    representation: u64,
    context: u64,
    measure: u64,
    unit: u64,
    model_mm_per_unit: f64,
    /// Effective bound; equals `declared_bound` unless a preview floor raised it.
    bound: f64,
    declared_bound: f64,
}
impl ConnectivityAccuracy {
    pub fn representation_id(&self) -> u64 {
        self.representation
    }
    pub fn context_id(&self) -> u64 {
        self.context
    }
    pub fn measure_id(&self) -> u64 {
        self.measure
    }
    pub fn unit_id(&self) -> u64 {
        self.unit
    }
    pub fn model_mm_per_unit(&self) -> f64 {
        self.model_mm_per_unit
    }
    /// The effective attachment bound (floored in preview mode).
    pub fn bound(&self) -> f64 {
        self.bound
    }
    /// The bound the source declared, in model units.
    pub fn declared_bound(&self) -> f64 {
        self.declared_bound
    }
    fn with_preview_tolerance(mut self, tolerance: PreviewTolerance) -> Self {
        let scaled = self.declared_bound * tolerance.declared_multiplier;
        let floored = tolerance.floor_mm / self.model_mm_per_unit;
        let effective = scaled.max(floored);
        if effective.is_finite() && effective > self.bound {
            self.bound = effective;
        }
        self
    }
}

#[derive(Debug)]
pub(super) struct ShellConnectivity {
    pub edges: HashMap<u64, Option<ConnectivityAccuracy>>,
    pub faces: HashMap<u64, Option<ConnectivityAccuracy>>,
    model_mm_per_unit: f64,
}

struct Representation {
    id: u64,
    context: u64,
}

fn params(record: &Record, count: usize) -> Result<&[Parameter], String> {
    let Parameter::List(p) = &record.parameter else {
        return Err(format!("{} parameters are not a list", record.name));
    };
    if p.len() != count {
        return Err(format!(
            "{} has {} parameters; expected {count}",
            record.name,
            p.len()
        ));
    }
    Ok(p)
}
fn references(parameter: &Parameter) -> Result<Vec<u64>, String> {
    let Parameter::List(p) = parameter else {
        return Err("expected entity reference set".into());
    };
    if p.is_empty() {
        return Err("empty assigned entity set".into());
    }
    let ids = p
        .iter()
        .map(|p| entity_ref(p).ok_or_else(|| "non-entity assigned reference".into()))
        .collect::<Result<Vec<_>, String>>()?;
    if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err("duplicate assigned entity reference".into());
    }
    Ok(ids)
}

fn collect_refs(p: &Parameter, out: &mut Vec<u64>) {
    match p {
        Parameter::Ref(Name::Entity(id)) => out.push(*id),
        Parameter::List(p) => p.iter().for_each(|p| collect_refs(p, out)),
        Parameter::Typed { parameter, .. } => collect_refs(parameter, out),
        _ => {}
    }
}

impl SourceRecords {
    pub(super) fn record(&mut self, instance: &EntityInstance) {
        let (id, records) = match instance {
            EntityInstance::Simple { id, record } => (*id, vec![record.clone()]),
            EntityInstance::Complex { id, subsuper } => {
                self.complex.insert(*id);
                (*id, subsuper.0.clone())
            }
        };
        if self.records.insert(id, records).is_some() {
            self.duplicates.insert(id);
        }
    }
    fn records(&self, id: u64) -> Result<&[Record], String> {
        if self.duplicates.contains(&id) {
            return Err(format!("duplicate source entity #{id}"));
        }
        self.records
            .get(&id)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("missing source entity #{id}"))
    }
    fn component(&self, id: u64, name: &str) -> Result<Option<&Record>, String> {
        let mut found = self.records(id)?.iter().filter(|r| r.name == name);
        let first = found.next();
        if found.next().is_some() {
            return Err(format!("duplicate {name} in #{id}"));
        }
        Ok(first)
    }
    fn representation(&self, id: u64) -> Result<Option<Representation>, String> {
        let records = self.records(id)?;
        if !records.iter().any(|r| {
            units::is_shape_representation_kind(&r.name)
                || r.name == "UNCERTAINTY_ASSIGNED_REPRESENTATION"
        }) {
            return Ok(None);
        }
        let mut bases = records.iter().filter(|r| {
            r.name == "REPRESENTATION"
                || (units::is_shape_representation_kind(&r.name)
                    && !matches!(&r.parameter, Parameter::List(p) if p.is_empty()))
                || (r.name == "UNCERTAINTY_ASSIGNED_REPRESENTATION" && !self.complex.contains(&id))
        });
        let base = bases
            .next()
            .ok_or_else(|| format!("representation #{id} has no definition"))?;
        if bases.next().is_some() {
            return Err(format!("duplicate representation definition in #{id}"));
        }
        let p = params(
            base,
            if base.name == "UNCERTAINTY_ASSIGNED_REPRESENTATION" {
                4
            } else {
                3
            },
        )?;
        references(&p[1])?;
        let context = entity_ref(&p[2])
            .ok_or_else(|| format!("representation #{id} has no context reference"))?;
        Ok(Some(Representation { id, context }))
    }
    fn owning_shell_paths(table: &Table, items: &Parameter, shells: &HashSet<u64>) -> HashSet<u64> {
        let Parameter::List(items) = items else {
            return HashSet::new();
        };
        let shell_path = |id: u64| -> Option<Vec<u64>> {
            if shells.contains(&id) {
                return Some(vec![id]);
            }
            let shell = Table::ref_id(&table.oriented_shell.get(&id)?.shell_element)?;
            shells.contains(&shell).then_some(vec![id, shell])
        };
        let mut paths = HashSet::new();
        // Only typed geometric containment establishes ownership. In particular,
        // a placement/style/Boolean reference to a shell is not containment.
        for item in items.iter().filter_map(entity_ref) {
            if let Some(path) = shell_path(item) {
                paths.extend(path);
            }
            if let Some(model) = table.shell_based_surface_model.get(&item) {
                for shell in model.sbsm_boundary.iter().filter_map(Table::ref_id) {
                    if let Some(path) = shell_path(shell) {
                        paths.insert(item);
                        paths.extend(path);
                    }
                }
            }
            if let Some(solid) = table.manifold_solid_brep.get(&item) {
                if let Some(path) = Table::ref_id(&solid.outer).and_then(shell_path) {
                    paths.insert(item);
                    paths.extend(path);
                }
                for shell in solid.voids.iter().filter_map(Table::ref_id) {
                    if table.oriented_shell.contains_key(&shell) {
                        if let Some(path) = shell_path(shell) {
                            paths.insert(item);
                            paths.extend(path);
                        }
                    }
                }
            }
        }
        paths
    }
    fn definition_ids(&self, roots: impl IntoIterator<Item = u64>) -> Result<HashSet<u64>, String> {
        let mut result = HashSet::new();
        let mut stack = roots.into_iter().collect::<Vec<_>>();
        while let Some(id) = stack.pop() {
            if !result.insert(id) {
                continue;
            }
            let records = self.records(id)?;
            if records.iter().any(|r| {
                matches!(r.name.as_str(), "MAPPED_ITEM" | "REPRESENTATION_MAP")
                    || units::is_shape_representation_kind(&r.name)
                    || matches!(
                        r.name.as_str(),
                        "REPRESENTATION" | "UNCERTAINTY_ASSIGNED_REPRESENTATION"
                    )
                    || r.name.contains("REPRESENTATION_RELATIONSHIP")
            }) {
                continue;
            }
            for record in records {
                collect_refs(&record.parameter, &mut stack);
            }
        }
        Ok(result)
    }
    fn model_unit_mm(&self, context: u64) -> Result<f64, String> {
        self.component(context, "REPRESENTATION_CONTEXT")?
            .ok_or_else(|| format!("#{context} is not a representation context"))?;
        let Some(assignment) = self.component(context, "GLOBAL_UNIT_ASSIGNED_CONTEXT")? else {
            return Ok(1.0);
        };
        let p = params(assignment, 1)?;
        let ids = references(&p[0])?;
        let mut length = None;
        for id in ids {
            let kind = unit_kind(id, self.records(id)?)?;
            if kind == UnitKind::Length {
                if length.replace(id).is_some() {
                    return Err(format!("context #{context} assigns multiple length units"));
                }
            }
        }
        length.map_or(Ok(1.0), |id| {
            unit_factor_mm(id, &self.records, &mut HashSet::new())
        })
    }
    fn assigned_bound(
        &self,
        owner: u64,
        component: &str,
        rep: &Representation,
        model_mm: f64,
    ) -> Result<Option<ConnectivityAccuracy>, String> {
        let Some(record) = self.component(owner, component)? else {
            return Ok(None);
        };
        let p = params(
            record,
            if component == "UNCERTAINTY_ASSIGNED_REPRESENTATION" && !self.complex.contains(&owner)
            {
                4
            } else {
                1
            },
        )?;
        let ids = references(p.last().unwrap())?;
        let mut length = None;
        for id in ids {
            let record = self
                .component(id, "UNCERTAINTY_MEASURE_WITH_UNIT")?
                .ok_or_else(|| format!("assignment references no uncertainty measure #{id}"))?;
            let p = params(record, 4)?;
            let Parameter::Typed { keyword, parameter } = &p[0] else {
                return Err(format!("uncertainty #{id} has no typed value"));
            };
            let unit =
                entity_ref(&p[1]).ok_or_else(|| format!("uncertainty #{id} has no own unit"))?;
            let kind = unit_kind(unit, self.records(unit)?)?;
            if keyword != "LENGTH_MEASURE" {
                let expected = match keyword.as_str() {
                    "PLANE_ANGLE_MEASURE" => UnitKind::PlaneAngle,
                    "SOLID_ANGLE_MEASURE" => UnitKind::SolidAngle,
                    _ => {
                        return Err(format!(
                            "unsupported uncertainty quantity {keyword} in #{id}"
                        ));
                    }
                };
                let value = match parameter.as_ref() {
                    Parameter::Real(v) => *v,
                    Parameter::Integer(v) => *v as f64,
                    _ => f64::NAN,
                };
                if kind != expected || !value.is_finite() || value <= 0.0 {
                    return Err(format!("invalid nonlength uncertainty #{id}"));
                }
                continue;
            }
            if kind != UnitKind::Length {
                return Err(format!("length uncertainty #{id} has nonlength unit"));
            }
            if length.is_some() {
                return Err(format!(
                    "conflicting length uncertainty declarations in #{owner}"
                ));
            }
            let bound = length_measure(&p[0])?
                * unit_factor_mm(unit, &self.records, &mut HashSet::new())?
                / model_mm;
            if !bound.is_finite()
                || bound <= 0.0
                || !bound.powi(2).is_finite()
                || bound.powi(2) == 0.0
            {
                return Err(format!("unrepresentable uncertainty bound in #{id}"));
            }
            length = Some(ConnectivityAccuracy {
                representation: rep.id,
                context: rep.context,
                measure: id,
                unit,
                model_mm_per_unit: model_mm,
                bound,
                declared_bound: bound,
            });
        }
        Ok(length)
    }
    fn item_bound(
        &self,
        item: u64,
        rep: &Representation,
        model_mm: f64,
    ) -> Result<Option<ConnectivityAccuracy>, String> {
        // Statistical/qualitative item qualifiers are not maximum connectivity bounds.
        if self
            .component(item, "QUALIFIED_REPRESENTATION_ITEM")?
            .is_some()
        {
            return Err(format!(
                "item #{item} uncertainty qualifiers do not establish a supported connectivity bound"
            ));
        }
        self.assigned_bound(rep.id, "UNCERTAINTY_ASSIGNED_REPRESENTATION", rep, model_mm)?
            .map_or_else(
                || {
                    self.assigned_bound(
                        rep.context,
                        "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
                        rep,
                        model_mm,
                    )
                },
                |bound| Ok(Some(bound)),
            )
    }
    pub(super) fn shell_connectivity(
        &self,
        table: &Table,
        shell: &ShellHolder,
        selected: Option<u64>,
    ) -> Result<Option<ShellConnectivity>, String> {
        let ids = table
            .shell
            .iter()
            .filter_map(|(id, candidate)| (candidate == shell).then_some(*id))
            .collect::<HashSet<_>>();
        if ids.is_empty() {
            return selected.map_or(Ok(None), |_| {
                Err("selected source shell has no entity identity".into())
            });
        }
        let mut owners = Vec::new();
        for id in self.records.keys().copied() {
            if selected.is_some_and(|selected| selected != id) {
                continue;
            }
            let records = &self.records[&id];
            if !records.iter().any(|r| {
                units::is_shape_representation_kind(&r.name)
                    || r.name == "UNCERTAINTY_ASSIGNED_REPRESENTATION"
            }) {
                continue;
            }
            let mut ownership_path = HashSet::new();
            for record in records {
                if units::is_shape_representation_kind(&record.name)
                    || record.name == "REPRESENTATION"
                    || record.name == "UNCERTAINTY_ASSIGNED_REPRESENTATION"
                {
                    if let Parameter::List(p) = &record.parameter {
                        if let Some(items) = p.get(1) {
                            ownership_path.extend(Self::owning_shell_paths(table, items, &ids));
                        }
                    }
                }
            }
            if ownership_path.is_empty() {
                continue;
            }
            let rep = self
                .representation(id)?
                .ok_or("source owner is not a representation")?;
            let Some(geometric) =
                self.component(rep.context, "GEOMETRIC_REPRESENTATION_CONTEXT")?
            else {
                continue;
            };
            if params(geometric, 1)? != [Parameter::Integer(3)] {
                return Err(format!(
                    "representation #{} is not a supported three-dimensional geometric context",
                    rep.id
                ));
            }
            owners.push((rep, ownership_path));
        }
        owners.sort_by_key(|(rep, _)| rep.id);
        if owners.is_empty() {
            return selected.map_or(Ok(None), |_| {
                Err("representation does not own this source shell".into())
            });
        }
        let source_id = *ids.iter().min().unwrap();
        let mut scope = self.definition_ids([source_id])?;
        for (_, path) in &owners {
            scope.extend(path);
        }
        let mut binding_scope = scope.clone();
        let mut resolved = None;
        for (rep, _) in owners {
            binding_scope.insert(rep.id);
            binding_scope.extend(self.definition_ids([rep.context])?);
            if let Some(assignment) =
                self.component(rep.id, "UNCERTAINTY_ASSIGNED_REPRESENTATION")?
            {
                let mut declarations = Vec::new();
                collect_refs(&assignment.parameter, &mut declarations);
                binding_scope.extend(self.definition_ids(declarations)?);
            }
            let model_mm = self.model_unit_mm(rep.context)?;
            let mut edges = HashMap::new();
            let mut faces = HashMap::new();
            // Validate qualifiers on all geometry defining the selected shell, so
            // an unsupported higher-precedence declaration cannot fall through.
            for id in &scope {
                let bound = self.item_bound(*id, &rep, model_mm)?;
                let bound = match table.preview_tolerance {
                    Some(tolerance) => bound.map(|b| b.with_preview_tolerance(tolerance)),
                    None => bound,
                };
                if table.edge_curve.contains_key(id) {
                    edges.insert(*id, bound.clone());
                }
                if table.face_surface.contains_key(id) {
                    faces.insert(*id, bound);
                }
            }
            let current = ShellConnectivity {
                edges,
                faces,
                model_mm_per_unit: model_mm,
            };
            if let Some(previous) = &resolved {
                let same =
                    |a: &HashMap<u64, Option<ConnectivityAccuracy>>,
                     b: &HashMap<u64, Option<ConnectivityAccuracy>>| {
                        a.len() == b.len()
                            && a.iter().all(|(id, bound)| {
                                b.get(id).is_some_and(|other| {
                                    bound.as_ref().map(|b| (b.bound, b.model_mm_per_unit))
                                        == other.as_ref().map(|b| (b.bound, b.model_mm_per_unit))
                                })
                            })
                    };
                let previous: &ShellConnectivity = previous;
                if previous.model_mm_per_unit != current.model_mm_per_unit
                    || !same(&previous.edges, &current.edges)
                    || !same(&previous.faces, &current.faces)
                {
                    return Err("source shell has conflicting geometric representation contexts; select a representation explicitly".into());
                }
            } else {
                resolved = Some(current);
            }
        }
        if resolved.as_ref().is_some_and(|policy| {
            policy.edges.values().any(Option::is_some) || policy.faces.values().any(Option::is_some)
        }) {
            let instances = binding_scope
                .iter()
                .map(|id| {
                    let records = self.records(*id)?.to_vec();
                    Ok(if self.complex.contains(id) {
                        EntityInstance::Complex {
                            id: *id,
                            subsuper: SubSuperRecord(records),
                        }
                    } else {
                        EntityInstance::Simple {
                            id: *id,
                            record: records.into_iter().next().ok_or("empty source entity")?,
                        }
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let original = Table::from_iter(instances.iter());
            if !table.same_source_definitions(&original, &binding_scope) {
                return Err("source connectivity declaration no longer matches the converted geometry/topology".into());
            }
        }
        Ok(resolved)
    }
}
