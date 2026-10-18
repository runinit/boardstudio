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
        // Legacy eligibility heuristic, not an authenticated allocation bound:
        // nom/serde and holder growth are not precharged by this multiplier.
        // Keep its conservative refusal policy for arbitrary caller tables.
        // The sealed same-parse route below adds no second parser/table owner.
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
    report: RepairReport,
    _original: Arc<Original>,
}
#[derive(Debug)]
struct RepairReport {
    source: SourceBoundStepSolid,
    traces: Vec<ShellTrace>,
    failure: Option<&'static str>,
}

/// Immutable bytes and the actual preview parse; the table is built once on demand.
///
/// Parser/holder allocations are existing preview input, not hard-capped report
/// storage. Captures borrow this input and cannot extend its lifetime.
///
/// ```compile_fail
/// use monstertruck_io::step::load::convert::StepRepairInput;
/// let input = StepRepairInput::parse(Vec::new()).unwrap();
/// let report = input.convert_solid(1, None, None).unwrap();
/// let parts = input.into_parts();
/// let _ = report.source(); // authority cannot outlive its immutable input
/// ```
#[derive(Debug)]
pub struct StepRepairInput {
    bytes: Vec<u8>,
    exchange: step_p21::ast::Exchange,
    table: std::sync::OnceLock<Table>,
    // Partial existing-preview diagnostics, not a total allocation bound. All
    // actual AST/typed map/holder capacities remain owned by exchange/table;
    // these scalars specifically retain the source/decode peak and DATA roots.
    // Nested parser/serde allocations and allocator overhead remain unqualified.
    _input_capacities: InputCapacities,
}
#[derive(Debug)]
struct InputCapacities {
    _source_bytes: usize,
    _temporary_decoded_bytes: usize,
    _data_sections: usize,
}
impl StepRepairInput {
    /// Parses the canonical preview input under the existing source byte limit.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, LoadError> {
        if bytes.len() > SOURCE_CAP {
            return Err("STEP input exceeds source byte limit".into());
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => std::borrow::Cow::Borrowed(text),
            Err(_) => {
                std::borrow::Cow::Owned(bytes.iter().map(|&byte| byte as char).collect::<String>())
            }
        };
        let decoded_capacity = match &text {
            std::borrow::Cow::Borrowed(_) => 0,
            std::borrow::Cow::Owned(text) => text.capacity(),
        };
        let exchange = step_p21::parser::parse(&text)?;
        if exchange.data.len() != 1 {
            return Err("STEP input requires exactly one DATA section".into());
        }
        let capacities = InputCapacities {
            _source_bytes: bytes.capacity(),
            _temporary_decoded_bytes: decoded_capacity,
            _data_sections: exchange.data.capacity(),
        };
        Ok(Self {
            bytes,
            exchange,
            table: std::sync::OnceLock::new(),
            _input_capacities: capacities,
        })
    }
    /// Canonical original bytes; no report copy or decoding round trip.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// The same immutable parse used to construct the actual table.
    pub fn exchange(&self) -> &step_p21::ast::Exchange {
        &self.exchange
    }
    /// Actual immutable conversion table, cached from the original parse.
    pub fn table(&self) -> &Table {
        self.table
            .get_or_init(|| Table::from_data_section(&self.exchange.data[0]))
    }
    /// Executes the conversion once with this call's tolerance, borrowing authority.
    pub fn convert_solid(
        &self,
        solid_id: u64,
        representation: Option<u64>,
        tolerance: Option<PreviewTolerance>,
    ) -> Result<StepRepairBorrowedConversion<'_>, LoadError> {
        let table = self.table();
        Ok(StepRepairBorrowedConversion {
            _input: self,
            report: table.capture_repair(
                solid_id,
                representation,
                Some(table),
                0,
                None,
                tolerance,
            )?,
        })
    }
    /// Moves preview inputs out after all borrowed reports have been consumed.
    pub fn into_parts(self) -> (Vec<u8>, step_p21::ast::Exchange) {
        (self.bytes, self.exchange)
    }
}

/// Same-call output and captures whose authority cannot outlive the sealed input.
#[derive(Debug)]
pub struct StepRepairBorrowedConversion<'input> {
    _input: &'input StepRepairInput,
    report: RepairReport,
}
impl StepRepairBorrowedConversion<'_> {
    /// Actual owned conversion result from this invocation.
    pub fn source(&self) -> &SourceBoundStepSolid {
        self.report.source()
    }
    /// Immutable stage views, borrowing only this report.
    pub fn operations(&self, shell: usize) -> impl Iterator<Item = StepRepairOperation<'_>> {
        self.report.operations(shell)
    }
    /// Mathematical completeness, independent of source capture availability.
    pub fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
        self.report.complete_bound(shell)
    }
    /// Drops captures and the authority borrow, retaining ordinary preview geometry.
    pub fn into_source(self) -> SourceBoundStepSolid {
        self.report.source
    }
}
impl StepRepairConversion {
    /// Actual owned conversion result from this invocation.
    pub fn source(&self) -> &SourceBoundStepSolid {
        self.report.source()
    }
    /// Immutable stage views, borrowing only this report.
    pub fn operations(&self, shell: usize) -> impl Iterator<Item = StepRepairOperation<'_>> {
        self.report.operations(shell)
    }
    /// Complete mathematical repair bound, when supported by every stage.
    pub fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
        self.report.complete_bound(shell)
    }
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
impl RepairReport {
    /// The immutable result emitted by the same checked conversion invocation.
    fn source(&self) -> &SourceBoundStepSolid {
        &self.source
    }
    /// Borrowed source-associated stage diagnostics for one shell.
    fn operations(&self, shell: usize) -> impl Iterator<Item = StepRepairOperation<'_>> {
        self.traces
            .get(shell)
            .into_iter()
            .flat_map(|trace| &trace.operations)
            .map(|operation| StepRepairOperation { operation })
    }
    /// Complete geometric repair only. Unknown numerical error is not zero repair.
    fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
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
        for operation in &trace.operations {
            if matches!(operation.stage, Stage::Numerical) {
                continue;
            }
            let value = operation.bound?;
            if !value.is_finite() || value < 0.0 {
                return Err("invalid repair movement bound");
            }
        }
        // Do not allocate a proof map outside the captured report's resource charge.
        let mut maximum = 0.0_f64;
        for (index, operation) in trace.operations.iter().enumerate() {
            if matches!(operation.stage, Stage::Numerical)
                || trace.operations[..index].iter().any(|prior| {
                    !matches!(prior.stage, Stage::Numerical)
                        && prior.ids.1 == operation.ids.1
                        && prior.ids.2 == operation.ids.2
                })
            {
                continue;
            }
            let mut path = 0.0_f64;
            for next in &trace.operations[index..] {
                if matches!(next.stage, Stage::Numerical)
                    || next.ids.1 != operation.ids.1
                    || next.ids.2 != operation.ids.2
                {
                    continue;
                }
                let value = next.bound?;
                path = if path == 0.0 {
                    value
                } else if value == 0.0 {
                    path
                } else {
                    (path + value).next_up()
                };
                if !path.is_finite() {
                    return Err("repair movement accumulation overflow");
                }
            }
            maximum = maximum.max(path);
        }
        Ok(maximum)
    }
}

impl Table {
    fn same_repair_input(&self, other: &Self) -> bool {
        let Self {
            source_records,
            representation,
            representation_item,
            representation_context,
            cartesian_point,
            direction,
            vector,
            placement,
            axis1_placement,
            axis2_placement_2d,
            axis2_placement_3d,
            line,
            polyline,
            b_spline_curve_with_knots,
            bezier_curve,
            quasi_uniform_curve,
            uniform_curve,
            rational_b_spline_curve,
            circle,
            ellipse,
            hyperbola,
            parabola,
            pcurve,
            surface_curve,
            plane,
            spherical_surface,
            cylindrical_surface,
            toroidal_surface,
            degenerate_toroidal_surface,
            conical_surface,
            b_spline_surface_with_knots,
            uniform_surface,
            quasi_uniform_surface,
            bezier_surface,
            rational_b_spline_surface,
            surface_of_linear_extrusion,
            surface_of_revolution,
            vertex_point,
            edge_curve,
            oriented_edge,
            edge_loop,
            vertex_loop,
            face_bound,
            face_surface,
            oriented_face,
            shell,
            oriented_shell,
            shell_based_surface_model,
            manifold_solid_brep,
            application_context,
            product_context,
            product,
            product_definition_formation,
            product_definition_context,
            product_definition,
            product_definition_shape,
            shape_definition_representation,
            shape_representation,
            context_dependent_shape_representation,
            shape_representation_relationship,
            shape_representation_relationship_with_transformation,
            next_assembly_usage_occurrence,
            item_defined_transformation,
            geometric_curve_set,
            definitional_representation,
            dummy,
            entity_report,
            preview_tolerance: _,
        } = self;
        true && source_records == &other.source_records
            && representation == &other.representation
            && representation_item == &other.representation_item
            && representation_context == &other.representation_context
            && cartesian_point == &other.cartesian_point
            && direction == &other.direction
            && vector == &other.vector
            && placement == &other.placement
            && axis1_placement == &other.axis1_placement
            && axis2_placement_2d == &other.axis2_placement_2d
            && axis2_placement_3d == &other.axis2_placement_3d
            && line == &other.line
            && polyline == &other.polyline
            && b_spline_curve_with_knots == &other.b_spline_curve_with_knots
            && bezier_curve == &other.bezier_curve
            && quasi_uniform_curve == &other.quasi_uniform_curve
            && uniform_curve == &other.uniform_curve
            && rational_b_spline_curve == &other.rational_b_spline_curve
            && circle == &other.circle
            && ellipse == &other.ellipse
            && hyperbola == &other.hyperbola
            && parabola == &other.parabola
            && pcurve == &other.pcurve
            && surface_curve == &other.surface_curve
            && plane == &other.plane
            && spherical_surface == &other.spherical_surface
            && cylindrical_surface == &other.cylindrical_surface
            && toroidal_surface == &other.toroidal_surface
            && degenerate_toroidal_surface == &other.degenerate_toroidal_surface
            && conical_surface == &other.conical_surface
            && b_spline_surface_with_knots == &other.b_spline_surface_with_knots
            && uniform_surface == &other.uniform_surface
            && quasi_uniform_surface == &other.quasi_uniform_surface
            && bezier_surface == &other.bezier_surface
            && rational_b_spline_surface == &other.rational_b_spline_surface
            && surface_of_linear_extrusion == &other.surface_of_linear_extrusion
            && surface_of_revolution == &other.surface_of_revolution
            && vertex_point == &other.vertex_point
            && edge_curve == &other.edge_curve
            && oriented_edge == &other.oriented_edge
            && edge_loop == &other.edge_loop
            && vertex_loop == &other.vertex_loop
            && face_bound == &other.face_bound
            && face_surface == &other.face_surface
            && oriented_face == &other.oriented_face
            && shell == &other.shell
            && oriented_shell == &other.oriented_shell
            && shell_based_surface_model == &other.shell_based_surface_model
            && manifold_solid_brep == &other.manifold_solid_brep
            && application_context == &other.application_context
            && product_context == &other.product_context
            && product == &other.product
            && product_definition_formation == &other.product_definition_formation
            && product_definition_context == &other.product_definition_context
            && product_definition == &other.product_definition
            && product_definition_shape == &other.product_definition_shape
            && shape_definition_representation == &other.shape_definition_representation
            && shape_representation == &other.shape_representation
            && context_dependent_shape_representation
                == &other.context_dependent_shape_representation
            && shape_representation_relationship == &other.shape_representation_relationship
            && shape_representation_relationship_with_transformation
                == &other.shape_representation_relationship_with_transformation
            && next_assembly_usage_occurrence == &other.next_assembly_usage_occurrence
            && item_defined_transformation == &other.item_defined_transformation
            && geometric_curve_set == &other.geometric_curve_set
            && definitional_representation == &other.definitional_representation
            && dummy == &other.dummy
            && entity_report == &other.entity_report
    }
    /// Additive traced facade; the existing preview conversion executes exactly once.
    /// `None` selects the existing route without connectivity authority.
    pub fn to_source_bound_solid_with_repair_checked(
        &self,
        solid_id: u64,
        representation: Option<u64>,
        original: &StepRepairSource,
    ) -> Result<StepRepairConversion, LoadError> {
        let mut failure = original.inner.failure;
        if let Some(source) = &original.inner.table
            && !source.same_repair_input(self)
        {
            // In particular, unauthenticated angle normalization must not
            // borrow unchanged-carrier evidence from the original table.
            failure = Some("original/conversion source mismatch or untraced normalization");
        }
        Ok(StepRepairConversion {
            report: self.capture_repair(
                solid_id,
                representation,
                original.inner.table.as_ref(),
                original.inner.charged_bytes,
                failure,
                self.preview_tolerance,
            )?,
            _original: Arc::clone(&original.inner),
        })
    }

    fn capture_repair(
        &self,
        solid_id: u64,
        representation: Option<u64>,
        source_table: Option<&Table>,
        charged_bytes: usize,
        mut failure: Option<&'static str>,
        tolerance: Option<PreviewTolerance>,
    ) -> Result<RepairReport, LoadError> {
        let storage = charged_bytes.checked_add(3 * std::mem::size_of::<usize>());
        if storage.is_none_or(|bytes| bytes > STORAGE_CAP) {
            failure = Some("repair capture storage resource cap");
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
        let converted = self.compressed_solid_with_repair_reported(
            solid,
            selection,
            capture_budget.clone(),
            tolerance,
        )?;
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
                Some(size)
                    if traces.try_reserve_exact(source.shells().len()).is_ok()
                        && traces.capacity() <= source.shells().len() =>
                {
                    charged = size
                }
                _ => failure = Some("repair shell trace storage resource cap"),
            }
        }
        if failure.is_some() {
            return Ok(RepairReport {
                source,
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
                        .checked_mul(
                            2 * (std::mem::size_of::<Option<u64>>()
                                + 2 * std::mem::size_of::<u64>()
                                + std::mem::size_of::<Option<usize>>()),
                        )
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
                if trace.operations.try_reserve_exact(count.unwrap()).is_err()
                    || trace.operations.capacity() > count.unwrap()
                {
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
                    // vec![value; n] requests precisely n elements (Vec's public
                    // guarantee). Charge two layouts above for each workspace:
                    // construction plus a possible into_boxed_slice shrink.
                    // No growth policy or hash capacity is assumed. Allocator
                    // bookkeeping is outside this requested-storage accounting.
                    let mut id_storage = vec![0_u64; shell.shell().edges.len()].into_boxed_slice();
                    let id_values = &mut *id_storage;
                    let ids = if source_table
                        .is_some_and(|table| bind_edges(table, shell, id_values).is_some())
                    {
                        Some(&*id_values)
                    } else {
                        None
                    };
                    for (index, edge) in shell.shell().edges.iter().enumerate() {
                        let id =
                            ids.as_ref()
                                .and_then(|ids| ids.get(index))
                                .copied()
                                .filter(|id| {
                                    source_table
                                        .is_some_and(|table| table.edge_curve.contains_key(id))
                                });
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
                    let mut previous_storage =
                        vec![None::<usize>; shell.shell().edges.len()].into_boxed_slice();
                    let previous = &mut *previous_storage;
                    let mut valid = ids.is_some();
                    for (ordinal, (id, change)) in captured_curves.iter().enumerate() {
                        valid &= change.surface_stage_valid();
                        if change.stage == "native-line-attachment-projection-v1" {
                            valid &= source_table
                                .and_then(|table| {
                                    table.edge_curve.get(id).map(|edge| (table, edge))
                                })
                                .is_some_and(|(table, edge)| {
                                    change
                                        .native_line
                                        .as_ref()
                                        .is_some_and(|native| native.matches_original(table, edge))
                                });
                        }
                        let index = ids
                            .as_ref()
                            .and_then(|ids| ids.iter().position(|candidate| candidate == id));
                        if let Some(index) = index {
                            if let Some(prior) = previous[index].replace(ordinal) {
                                let prior = &captured_curves[prior].1;
                                valid &= prior.after == change.before
                                    && (prior.after_domain == change.before_domain
                                        || change.surface_restricts(prior.after_domain));
                            }
                        } else {
                            valid = false;
                        }
                    }
                    for (index, ordinal) in previous.iter().enumerate() {
                        if let Some(ordinal) = ordinal {
                            valid &= shell.shell().edges[index].curve
                                == captured_curves[*ordinal].1.after;
                        }
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
        Ok(RepairReport {
            source,
            traces,
            failure,
        })
    }
}

// The caller precharges all simultaneous fixed-layout index workspaces.
fn bind_edges(table: &Table, source: &SourceBoundStepShell, result: &mut [u64]) -> Option<()> {
    let count = source.shell().edges.len();
    if result.len() != count {
        return None;
    }
    let mut id_storage = vec![None::<u64>; count].into_boxed_slice();
    let ids = &mut *id_storage;
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
    let mut sorted_storage = vec![0_u64; count].into_boxed_slice();
    let sorted = &mut *sorted_storage;
    for ((result, sorted), id) in result.iter_mut().zip(sorted.iter_mut()).zip(ids.iter()) {
        *result = (*id)?;
        *sorted = *result;
    }
    sorted.sort_unstable();
    (!sorted.windows(2).any(|pair| pair[0] == pair[1])).then_some(())
}
