//! Immutable source/result ownership for complete repair accounting.
use super::*;
use std::sync::Arc;

const SOURCE_CAP: usize = 32 * 1024 * 1024;
const STORAGE_CAP: usize = 256 * 1024 * 1024;
const OPERATION_CAP: usize = 1_000_000;

/// Original source authority captured before any conversion or normalization.
pub struct StepRepairSource {
    inner: Arc<Original>,
}
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
        let table = if failure.is_none() { Some(Table::from_step_bytes(bytes)?) } else { None };
        Ok(Self { inner: Arc::new(Original {
            _bytes: if failure.is_none() { bytes.to_vec() } else { Vec::new() },
            table, charged_bytes: charged.unwrap_or(STORAGE_CAP), failure,
        }) })
    }
}

#[derive(Clone, Copy)]
enum Stage { SourceBinding, NativeInterval, Master, Surface, Numerical }
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::SourceBinding => "source-binding-v1",
            Self::NativeInterval => "native-finite-interval-v1",
            Self::Master => "selected-master-v1",
            Self::Surface => "finite-surface-support-v1",
            Self::Numerical => "numeric-conversion-v1",
        }
    }
}
struct CapturedEdge {
    id: u64,
    // Original raw carrier and its topological finite-domain selectors are
    // retained by identity in Original, before the converter can replace them.
    original: EdgeCurveHolder,
}
struct Operation {
    stage: Stage,
    ids: (u64, Option<u64>, Option<u64>),
    bound: Result<f64, &'static str>,
    // The actual emitted carrier/domain belongs to source.shells()[shell].
    // A locator cannot be rebound to another mesh or caller-owned geometry.
    _edge_index: Option<usize>,
    _emitted_interval: Option<(f64, f64)>,
}
struct ShellTrace {
    operations: Vec<Operation>,
    failure: Option<&'static str>,
}

/// Owns the actual checked conversion result and its immutable source authority.
pub struct StepRepairConversion {
    source: SourceBoundStepSolid,
    _original: Arc<Original>,
    traces: Vec<ShellTrace>,
    failure: Option<&'static str>,
}
/// Borrowed diagnostic view; callers cannot create a movement witness.
pub struct StepRepairOperation<'a> { operation: &'a Operation }
impl StepRepairOperation<'_> {
    pub fn stage(&self) -> &'static str { self.operation.stage.name() }
    /// Solid, optional source face, optional shared source edge.
    pub fn source_ids(&self) -> (u64, Option<u64>, Option<u64>) { self.operation.ids }
    /// In source units; numerical conversion is a separate diagnostic stage.
    pub fn bound(&self) -> Result<f64, &str> { self.operation.bound }
}
impl StepRepairConversion {
    pub fn source(&self) -> &SourceBoundStepSolid { &self.source }
    pub fn operations(&self, shell: usize) -> impl Iterator<Item = StepRepairOperation<'_>> {
        self.traces.get(shell).into_iter().flat_map(|trace| &trace.operations)
            .map(|operation| StepRepairOperation { operation })
    }
    /// Complete geometric repair only. Unknown numerical error is not zero repair.
    pub fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
        if let Some(reason) = self.failure { return Err(reason); }
        let trace = self.traces.get(shell).ok_or("missing repair shell coverage")?;
        if let Some(reason) = trace.failure { return Err(reason); }
        if trace.operations.is_empty() { return Err("empty repair stage coverage"); }
        let mut paths = std::collections::HashMap::<(Option<u64>, Option<u64>), f64>::new();
        for operation in &trace.operations {
            if matches!(operation.stage, Stage::Numerical) { continue; }
            let value = operation.bound?;
            if !value.is_finite() || value < 0.0 { return Err("invalid repair movement bound"); }
            let path = paths.entry((operation.ids.1, operation.ids.2)).or_default();
            *path = if *path == 0.0 { value } else if value == 0.0 { *path } else { (*path + value).next_up() };
            if !path.is_finite() { return Err("repair movement accumulation overflow"); }
        }
        Ok(paths.values().copied().fold(0.0, f64::max))
    }
}

impl Table {
    /// Additive traced facade; the existing preview conversion executes exactly once.
    /// `None` selects the existing route without connectivity authority.
    pub fn to_source_bound_solid_with_repair_checked(
        &self, solid_id: u64, representation: Option<u64>, original: &StepRepairSource,
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
        let storage = count.checked_mul(std::mem::size_of::<CapturedEdge>())
            .and_then(|size| size.checked_add(original.inner.charged_bytes));
        let mut captured = Vec::new();
        if count > OPERATION_CAP || storage.is_none_or(|size| size > STORAGE_CAP) {
            failure = Some("repair capture storage resource cap");
        } else if failure.is_none() {
            captured.try_reserve_exact(count).map_err(|_| LoadError::from("repair capture allocation failed"))?;
            for (&id, edge) in &source_table.unwrap().edge_curve {
                captured.push(CapturedEdge { id, original: edge.clone() });
            }
        }
        let solid = self.manifold_solid_brep.get(&solid_id).ok_or("repair solid source identity missing")?;
        let source = match representation {
            Some(owner) => self.to_source_bound_solid_in_representation_checked(solid, owner)?,
            None => self.to_source_bound_solid_without_connectivity_checked(solid)?,
        };
        let mut traces = Vec::new();
        // The trace accounts for every real source face and shared emitted edge.
        // Unsupported correspondence is retained explicitly; it does not change
        // the successful preview's face inventory or emitted carrier/triangles.
        for shell in source.shells() {
            let count = shell.shell().edges.len().checked_mul(2)
                .and_then(|count| count.checked_add(shell.shell().faces.len()))
                .and_then(|count| count.checked_add(2));
            let needed = count.and_then(|n| n.checked_mul(std::mem::size_of::<Operation>()));
            let resource_failure = failure.or_else(|| {
                (count.is_none_or(|n| n > OPERATION_CAP)
                    || needed.and_then(|n| storage.and_then(|s| n.checked_add(s)))
                        .is_none_or(|n| n > STORAGE_CAP))
                    .then_some("repair operation storage resource cap")
            });
            let mut trace = ShellTrace { operations: Vec::new(), failure: resource_failure };
            if trace.failure.is_none() {
                if trace.operations.try_reserve_exact(count.unwrap()).is_err() {
                    trace.failure = Some("repair operation allocation failed");
                } else {
                    trace.operations.push(Operation { stage: Stage::SourceBinding,
                        ids: (solid_id, None, None), bound: Ok(0.0), _edge_index: None, _emitted_interval: None });
                    // Until a unique original edge/use domain is captured at
                    // the inverse/master branch, no final edge may fabricate it.
                    for (index, edge) in shell.shell().edges.iter().enumerate() {
                        let candidates = captured.iter().filter(|raw| {
                            let (Some(front), Some(back)) = (Self::ref_id(&raw.original.edge_start), Self::ref_id(&raw.original.edge_end)) else { return false; };
                            front != back && source_table.is_some_and(|table| table.edge_curve.contains_key(&raw.id))
                        }).collect::<Vec<_>>();
                        let id = (candidates.len() == 1).then(|| candidates[0].id);
                        for stage in [Stage::NativeInterval, Stage::Master] {
                            trace.operations.push(Operation { stage, ids: (solid_id, None, id),
                                bound: Err("conversion-time finite-domain correspondence is unsupported"),
                                _edge_index: Some(index), _emitted_interval: Some(edge.curve.range_tuple()) });
                        }
                    }
                    for &face in shell.source_face_ids() {
                        trace.operations.push(Operation { stage: Stage::Surface,
                            ids: (solid_id, Some(face), None), bound: Err("original finite surface support domain is unsupported"),
                            _edge_index: None, _emitted_interval: None });
                    }
                    trace.operations.push(Operation { stage: Stage::Numerical,
                        ids: (solid_id, None, None), bound: Err("original numeric conversion error is unsupported"),
                        _edge_index: None, _emitted_interval: None });
                }
            }
            traces.push(trace);
        }
        Ok(StepRepairConversion { source, _original: Arc::clone(&original.inner), traces, failure })
    }
}
