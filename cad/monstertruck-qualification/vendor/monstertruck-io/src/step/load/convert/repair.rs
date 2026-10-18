//! Immutable source/result ownership for complete repair accounting.
use super::*;
use std::sync::Arc;

const SOURCE_CAP: usize = 32 * 1024 * 1024;
const STORAGE_CAP: usize = 256 * 1024 * 1024;
const OPERATION_CAP: usize = 1_000_000;

/// Original source authority captured before any conversion or normalization.
#[derive(Debug)]
pub struct StepRepairSource {
    inner: Arc<Original>,
}
#[derive(Debug)]
struct Original {
    _bytes: Vec<u8>,
    table: Option<Table>,
    charged_bytes: usize,
    failure: Option<&'static str>,
}
impl StepRepairSource {
    /// A resource-limited capture is incomplete; it is not a preview-input refusal.
    pub fn from_step_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        // Before parsing/copying, charge a deliberately conservative bound for
        // parsed records/holders, strings, hash-table spare capacity and the one
        // temporary comparison clone. This authority never resolves references
        // recursively into per-operation geometry copies.
        let charged = bytes.len().checked_mul(1024);
        let failure = if bytes.len() > SOURCE_CAP {
            Some("repair source byte resource cap")
        } else if charged.is_none_or(|size| size > STORAGE_CAP) {
            Some("repair source storage resource cap")
        } else {
            None
        };
        let table = if failure.is_none() {
            Some(Table::from_step_bytes(bytes)?)
        } else {
            None
        };
        Ok(Self {
            inner: Arc::new(Original {
                _bytes: if failure.is_none() {
                    bytes.to_vec()
                } else {
                    Vec::new()
                },
                table,
                charged_bytes: charged.unwrap_or(STORAGE_CAP),
                failure,
            }),
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    SourceBinding,
    NativeInterval,
    CurveMutation(&'static str),
    Master,
    Surface,
    Numerical,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::SourceBinding => "source-binding-v1",
            Self::NativeInterval => "native-finite-interval-v1",
            Self::CurveMutation(stage) => stage,
            Self::Master => "selected-master-v1",
            Self::Surface => "finite-surface-support-v1",
            Self::Numerical => "numeric-conversion-v1",
        }
    }
}
#[derive(Debug)]
struct Operation {
    stage: Stage,
    ids: (u64, Option<u64>, Option<u64>),
    bound: Result<f64, &'static str>,
    // The actual emitted carrier/domain belongs to source.shells()[shell].
    // A locator cannot be rebound to another mesh or caller-owned geometry.
    _edge_index: Option<usize>,
    _emitted_interval: Option<(f64, f64)>,
    curve: Option<FiniteCurveRepair>,
}
#[derive(Debug)]
struct ShellTrace {
    operations: Vec<Operation>,
    failure: Option<&'static str>,
}

/// Owns the actual checked conversion result and its immutable source authority.
#[derive(Debug)]
pub struct StepRepairConversion {
    source: SourceBoundStepSolid,
    _original: Arc<Original>,
    traces: Vec<ShellTrace>,
    failure: Option<&'static str>,
}
/// Borrowed diagnostic view; callers cannot create a movement witness.
#[derive(Debug)]
pub struct StepRepairOperation<'a> {
    operation: &'a Operation,
}
/// Immutable captured pair from the actual conversion stage, not a certificate.
#[derive(Debug)]
pub struct StepRepairCurveChange<'a> {
    change: &'a FiniteCurveRepair,
}
impl StepRepairCurveChange<'_> {
    /// Actual immediate carriers, including all polynomial/rational controls.
    pub fn curves(&self) -> (&Curve3D, &Curve3D) {
        (&self.change.before, &self.change.after)
    }
    /// Directed finite intervals on the corresponding carriers.
    pub fn domains(&self) -> ((f64, f64), (f64, f64)) {
        (self.change.before_domain, self.change.after_domain)
    }
}
impl StepRepairOperation<'_> {
    /// Captured data only: unresolved original selection remains unsupported.
    pub fn curve_change(&self) -> Option<StepRepairCurveChange<'_>> {
        self.operation
            .curve
            .as_ref()
            .map(|change| StepRepairCurveChange { change })
    }

    /// Fixed identity of the owner stage.
    pub fn stage(&self) -> &'static str {
        self.operation.stage.name()
    }
    /// Solid, optional source face, optional shared source edge.
    pub fn source_ids(&self) -> (u64, Option<u64>, Option<u64>) {
        self.operation.ids
    }
    /// In source units; numerical conversion is a separate diagnostic stage.
    pub fn bound(&self) -> Result<f64, &str> {
        self.operation.bound
    }
}
impl StepRepairConversion {
    /// The immutable result emitted by the same checked conversion invocation.
    pub fn source(&self) -> &SourceBoundStepSolid {
        &self.source
    }
    /// Borrowed source-associated stage diagnostics for one shell.
    pub fn operations(&self, shell: usize) -> impl Iterator<Item = StepRepairOperation<'_>> {
        self.traces
            .get(shell)
            .into_iter()
            .flat_map(|trace| &trace.operations)
            .map(|operation| StepRepairOperation { operation })
    }
    /// Complete geometric repair only. Unknown numerical error is not zero repair.
    pub fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
        if let Some(reason) = self.failure {
            return Err(reason);
        }
        let source = self
            .source
            .shells()
            .get(shell)
            .ok_or("missing repair shell source")?;
        if source.shell().faces.is_empty()
            || source.source_face_ids().len() != source.shell().faces.len()
        {
            return Err("empty or incomplete finite source face coverage");
        }
        let trace = self
            .traces
            .get(shell)
            .ok_or("missing repair shell coverage")?;
        if let Some(reason) = trace.failure {
            return Err(reason);
        }
        if trace.operations.is_empty() {
            return Err("empty repair stage coverage");
        }
        let mut paths = std::collections::HashMap::<(Option<u64>, Option<u64>), f64>::new();
        for operation in &trace.operations {
            if matches!(operation.stage, Stage::Numerical) {
                continue;
            }
            let value = operation.bound?;
            if !value.is_finite() || value < 0.0 {
                return Err("invalid repair movement bound");
            }
            let path = paths.entry((operation.ids.1, operation.ids.2)).or_default();
            *path = if *path == 0.0 {
                value
            } else if value == 0.0 {
                *path
            } else {
                (*path + value).next_up()
            };
            if !path.is_finite() {
                return Err("repair movement accumulation overflow");
            }
        }
        Ok(paths.values().copied().fold(0.0, f64::max))
    }
}

impl Table {
    /// Additive traced facade; the existing preview conversion executes exactly once.
    /// `None` selects the existing route without connectivity authority.
    pub fn to_source_bound_solid_with_repair_checked(
        &self,
        solid_id: u64,
        representation: Option<u64>,
        original: &StepRepairSource,
    ) -> Result<StepRepairConversion, LoadError> {
        let mut failure = original.inner.failure;
        if let Some(source) = &original.inner.table {
            let mut expected = source.clone();
            expected.preview_tolerance = self.preview_tolerance;
            if expected != *self {
                // In particular, unauthenticated angle normalization must not
                // borrow unchanged-carrier evidence from the original table.
                failure = Some("original/conversion source mismatch or untraced normalization");
            }
        }
        // Capture original raw definitions BEFORE the actual conversion. Unknown
        // native trim/master choices remain unsupported, not reconstructed from
        // final endpoints or from a second invocation of the converter.
        let source_table = original.inner.table.as_ref();
        let count = source_table.map_or(0, |table| table.edge_curve.len());
        let storage = count
            .checked_mul(std::mem::size_of::<u64>())
            .and_then(|size| size.checked_add(original.inner.charged_bytes));
        let mut captured = Vec::new();
        if count > OPERATION_CAP || storage.is_none_or(|size| size > STORAGE_CAP) {
            failure = Some("repair capture storage resource cap");
        } else if failure.is_none() {
            if captured.try_reserve_exact(count).is_err() {
                failure = Some("repair capture allocation failed");
            } else {
                captured.extend(source_table.unwrap().edge_curve.keys().copied());
                captured.sort_unstable();
            }
        }
        let solid = self
            .manifold_solid_brep
            .get(&solid_id)
            .ok_or("repair solid source identity missing")?;
        self.entity_report.require_empty()?;
        let selection = representation.map_or(ConnectivitySelection::Disabled, |owner| {
            ConnectivitySelection::FromSource(Some(owner))
        });
        let capture_budget = failure.is_none().then(|| {
            std::rc::Rc::new(std::cell::Cell::new(
                STORAGE_CAP.saturating_sub(storage.unwrap_or(STORAGE_CAP)),
            ))
        });
        let converted =
            self.compressed_solid_with_repair_reported(solid, selection, capture_budget.clone())?;
        let mut source = self.bind_source_solid(converted)?;
        let mut traces = Vec::new();
        let mut charged = capture_budget
            .as_ref()
            .map_or(STORAGE_CAP, |budget| STORAGE_CAP - budget.get());
        let mut operations = 0usize;
        let shell_storage = source
            .shells()
            .len()
            .checked_mul(2 * std::mem::size_of::<ShellTrace>());
        if failure.is_none() {
            match shell_storage
                .and_then(|size| charged.checked_add(size))
                .filter(|size| *size <= STORAGE_CAP)
            {
                Some(size) if traces.try_reserve_exact(source.shells().len()).is_ok() => {
                    charged = size
                }
                _ => failure = Some("repair shell trace storage resource cap"),
            }
        }
        if failure.is_some() {
            return Ok(StepRepairConversion {
                source,
                _original: Arc::clone(&original.inner),
                traces,
                failure,
            });
        }
        // The trace accounts for every real source face and shared emitted edge.
        // Unsupported correspondence is retained explicitly; it does not change
        // the successful preview's face inventory or emitted carrier/triangles.
        for shell in &mut source.shells {
            let count = shell
                .shell()
                .edges
                .len()
                .checked_mul(2)
                .and_then(|count| count.checked_add(shell.shell().faces.len()))
                .and_then(|count| count.checked_add(2))
                .and_then(|count| count.checked_add(shell.repair_curves.len()));
            let needed = count
                .and_then(|n| n.checked_mul(2 * std::mem::size_of::<Operation>()))
                .and_then(|n| {
                    shell
                        .shell()
                        .edges
                        .len()
                        .checked_mul(8 * std::mem::size_of::<u64>())
                        .and_then(|ids| n.checked_add(ids))
                });
            let resource_failure = failure.or_else(|| {
                (count
                    .and_then(|n| operations.checked_add(n))
                    .is_none_or(|n| n > OPERATION_CAP)
                    || needed
                        .and_then(|n| charged.checked_add(n))
                        .is_none_or(|n| n > STORAGE_CAP))
                .then_some("repair operation storage resource cap")
            });
            if let Some(reason) = resource_failure {
                failure = Some(reason);
                break;
            }
            charged += needed.unwrap();
            operations += count.unwrap();
            let mut trace = ShellTrace {
                operations: Vec::new(),
                failure: resource_failure.or(shell
                    .repair_failed
                    .then_some("finite carrier capture resource failure")),
            };
            if trace.failure.is_none() {
                if trace.operations.try_reserve_exact(count.unwrap()).is_err() {
                    trace.failure = Some("repair operation allocation failed");
                } else {
                    trace.operations.push(Operation {
                        stage: Stage::SourceBinding,
                        ids: (solid_id, None, None),
                        bound: Ok(0.0),
                        _edge_index: None,
                        _emitted_interval: None,
                        curve: None,
                    });
                    // Until a unique original edge/use domain is captured at
                    // the inverse/master branch, no final edge may fabricate it.
                    let ids = source_table.and_then(|table| bind_edges(table, shell));
                    for (index, edge) in shell.shell().edges.iter().enumerate() {
                        let id = ids
                            .as_ref()
                            .and_then(|ids| ids.as_slice().get(index))
                            .copied()
                            .filter(|id| captured.binary_search(id).is_ok());
                        for stage in [Stage::NativeInterval, Stage::Master] {
                            trace.operations.push(Operation {
                                stage,
                                ids: (solid_id, None, id),
                                bound: Err(
                                    "conversion-time finite-domain correspondence is unsupported",
                                ),
                                _edge_index: Some(index),
                                _emitted_interval: Some(edge.curve.range_tuple()),
                                curve: None,
                            });
                        }
                    }
                    // Move only the same invocation's retained mutation pairs.
                    // Exact structural chaining authenticates the actual output,
                    // independently of endpoints or sampled evaluations.
                    let captured_curves = std::mem::take(&mut shell.repair_curves);
                    let mut previous = HashMap::<u64, (&Curve3D, (f64, f64))>::new();
                    let mut valid = ids.is_some();
                    for (id, change) in &captured_curves {
                        valid &= change.surface_stage_valid();
                        if change.stage == "native-line-attachment-projection-v1" {
                            valid &= source_table
                                .and_then(|table| {
                                    table.edge_curve.get(id).map(|edge| (table, edge))
                                })
                                .and_then(|(table, edge)| edge.clone().into_owned(table).ok())
                                .is_some_and(|edge| {
                                    change
                                        .native_line
                                        .as_ref()
                                        .is_some_and(|native| native.matches_original(&edge))
                                });
                        }
                        if let Some((curve, domain)) =
                            previous.insert(*id, (&change.after, change.after_domain))
                        {
                            valid &= *curve == change.before
                                && (domain == change.before_domain
                                    || change.surface_restricts(domain));
                        }
                    }
                    for (id, (curve, _)) in previous {
                        valid &= ids
                            .as_ref()
                            .and_then(|ids| ids.iter().position(|candidate| *candidate == id))
                            .is_some_and(|index| shell.shell().edges[index].curve == *curve);
                    }
                    if !valid {
                        trace.failure = Some("finite carrier source/output chain mismatch");
                    }
                    for (id, change) in captured_curves {
                        let index = ids
                            .as_ref()
                            .and_then(|ids| ids.iter().position(|candidate| *candidate == id));
                        trace.operations.push(Operation {
                            stage: Stage::CurveMutation(change.stage),
                            ids: (solid_id, None, Some(id)),
                            bound: Err(
                                "captured finite carrier pair requires concrete whole-domain proof",
                            ),
                            _edge_index: index,
                            _emitted_interval: Some(change.after_domain),
                            curve: valid.then_some(change),
                        });
                    }
                    for &face in shell.source_face_ids() {
                        trace.operations.push(Operation {
                            stage: Stage::Surface,
                            ids: (solid_id, Some(face), None),
                            bound: Err("original finite surface support domain is unsupported"),
                            _edge_index: None,
                            _emitted_interval: None,
                            curve: None,
                        });
                    }
                    trace.operations.push(Operation {
                        stage: Stage::Numerical,
                        ids: (solid_id, None, None),
                        bound: Err("original numeric conversion error is unsupported"),
                        _edge_index: None,
                        _emitted_interval: None,
                        curve: None,
                    });
                }
            }
            traces.push(trace);
        }
        Ok(StepRepairConversion {
            source,
            _original: Arc::clone(&original.inner),
            traces,
            failure,
        })
    }
}

fn bind_edges(table: &Table, source: &SourceBoundStepShell) -> Option<Vec<u64>> {
    let mut ids = vec![None; source.shell().edges.len()];
    if source.source_face_ids().len() != source.shell().faces.len() {
        return None;
    }
    for (&id, emitted) in source.source_face_ids().iter().zip(&source.shell().faces) {
        let face = table.face_surface.get(&id)?;
        if face.bounds.len() != emitted.boundaries.len() {
            return None;
        }
        for (bound, edges) in face.bounds.iter().zip(&emitted.boundaries) {
            let bound = table.face_bound.get(&Table::ref_id(bound)?)?;
            let raw = table.edge_loop.get(&Table::ref_id(&bound.bound)?)?;
            if raw.edge_list.len() != edges.len() {
                return None;
            }
            let forward = bound.orientation == emitted.orientation;
            for (index, edge) in edges.iter().enumerate() {
                let original_index = if forward {
                    index
                } else {
                    edges.len() - 1 - index
                };
                let raw_id = Table::ref_id(&raw.edge_list[original_index])?;
                let (id, orientation) = match table.oriented_edge.get(&raw_id) {
                    Some(original) => {
                        (Table::ref_id(&original.edge_element)?, original.orientation)
                    }
                    None if table.edge_curve.contains_key(&raw_id) => (raw_id, true),
                    _ => return None,
                };
                if edge.orientation != (orientation == forward) {
                    return None;
                }
                let slot = ids.get_mut(edge.index)?;
                if slot.is_some_and(|previous| previous != id) {
                    return None;
                }
                *slot = Some(id);
            }
        }
    }
    let ids = ids.into_iter().collect::<Option<Vec<_>>>()?;
    (ids.iter().copied().collect::<HashSet<_>>().len() == ids.len()).then_some(ids)
}
