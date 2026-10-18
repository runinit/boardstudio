//! Internal operation accounting. A support translation is not whole-asset movement.
mod cuboid;
mod finite_curve;
mod finite_line;
use super::*;
pub(super) use cuboid::Cuboid;
use monstertruck_io::step::load::step_geometry::Curve3D;
use monstertruck_io::step::load::step_p21::tables::{IntoOwned, PlaceHolder};

// Preserve conversion/sqrt/outward-comparison order; whole-curve extreme and
// finite-LINE equality/nextdown controls pin the saved numerical behavior.
fn sqrt_upper(
    squared: &num_rational::BigRational,
    range_error: &'static str,
) -> Result<f64, &'static str> {
    use num_traits::{Signed, ToPrimitive};
    let mut bound = squared.to_f64().ok_or(range_error)?.sqrt();
    if squared.is_positive() && bound == 0.0 {
        bound = f64::MIN_POSITIVE.sqrt();
    }
    if !bound.is_finite() {
        return Err(range_error);
    }
    while num_rational::BigRational::from_float(bound)
        .map(|value| &value * &value)
        .as_ref()
        < Some(squared)
    {
        bound = bound.next_up();
    }
    if !bound.is_finite() {
        return Err(range_error);
    }
    Ok(bound)
}

pub(super) enum Conversion<'input> {
    Captured(StepRepairBorrowedConversion<'input>),
    Normalized(SourceBoundStepSolid),
}
impl Conversion<'_> {
    pub(super) fn source(&self) -> &SourceBoundStepSolid {
        match self {
            Self::Captured(report) => report.source(),
            Self::Normalized(source) => source,
        }
    }
    pub(super) fn operations(
        &self,
        shell: usize,
    ) -> impl Iterator<Item = monstertruck_io::step::load::convert::StepRepairOperation<'_>> {
        let report = match self {
            Self::Captured(report) => Some(report),
            Self::Normalized(_) => None,
        };
        report
            .into_iter()
            .flat_map(move |report| report.operations(shell))
    }
    pub(super) fn complete_bound(&self, shell: usize) -> Result<f64, &str> {
        match self {
            Self::Captured(report) => report.complete_bound(shell),
            Self::Normalized(_) => Err("untraced original/normalized exchange mismatch"),
        }
    }
    pub(super) fn into_source(self) -> SourceBoundStepSolid {
        match self {
            Self::Captured(report) => report.into_source(),
            Self::Normalized(source) => source,
        }
    }
}

struct NativeLineSource {
    points: [Point3; 2],
    origin: Point3,
    direction: Vec<f64>,
    magnitude: f64,
    input: Option<LineInput>,
}

#[derive(PartialEq)]
struct LineInput {
    ids: [u64; 8],
    coordinates: [[f64; 3]; 4],
    magnitude: f64,
    same_sense: bool,
}
impl LineInput {
    fn read(table: &Table, id: u64) -> Option<Self> {
        let edge = table.edge_curve.get(&id)?;
        let curve_id = entity_id(&edge.edge_geometry)?;
        let line = table.line.get(&curve_id)?;
        let origin_id = entity_id(&line.pnt)?;
        let vector_id = entity_id(&line.dir)?;
        let vector = table.vector.get(&vector_id)?;
        let direction_id = entity_id(&vector.orientation)?;
        let vertex_ids = [entity_id(&edge.edge_start)?, entity_id(&edge.edge_end)?];
        let point_ids =
            vertex_ids.map(|id| entity_id(&table.vertex_point.get(&id)?.vertex_geometry));
        let point_ids = [point_ids[0]?, point_ids[1]?];
        Some(Self {
            ids: [
                curve_id,
                origin_id,
                vector_id,
                direction_id,
                vertex_ids[0],
                vertex_ids[1],
                point_ids[0],
                point_ids[1],
            ],
            coordinates: [
                table
                    .cartesian_point
                    .get(&origin_id)?
                    .coordinates
                    .as_slice()
                    .try_into()
                    .ok()?,
                table
                    .direction
                    .get(&direction_id)?
                    .direction_ratios
                    .as_slice()
                    .try_into()
                    .ok()?,
                table
                    .cartesian_point
                    .get(&point_ids[0])?
                    .coordinates
                    .as_slice()
                    .try_into()
                    .ok()?,
                table
                    .cartesian_point
                    .get(&point_ids[1])?
                    .coordinates
                    .as_slice()
                    .try_into()
                    .ok()?,
            ],
            magnitude: vector.magnitude,
            same_sense: edge.same_sense,
        })
    }

    fn projected_branch(&self, source: &SourceBoundStepShell, index: usize, id: u64) -> bool {
        use monstertruck_io::step::load::report::ConnectivityIncidence;
        let Some(edge) = source.shell().edges.get(index) else {
            return false;
        };
        if !matches!(edge.curve, Curve3D::Line(_)) {
            return false;
        }
        let endpoints = [
            (false, edge.vertices.0, self.ids[4], self.coordinates[2]),
            (true, edge.vertices.1, self.ids[5], self.coordinates[3]),
        ];
        let mut attachments = source.connectivity_attachments().iter().filter(|a| {
            a.incidence() == ConnectivityIncidence::VertexOnEdge && a.edge_index() == index
        });
        let (Some(a), Some(b), None) = (attachments.next(), attachments.next(), attachments.next())
        else {
            return false;
        };
        endpoints
            .into_iter()
            .all(|(back, vertex_index, vertex_id, point)| {
                source.shell().vertices.get(vertex_index)
                    == Some(Point3::new(point[0], point[1], point[2]))
                    && [a, b].into_iter().any(|a| {
                        a.edge_id() == id
                            && a.vertex_id() == vertex_id
                            && a.vertex_index() == vertex_index
                            && a.is_back_endpoint() == back
                            && a.face_index().is_none()
                            && a.face_id().is_none()
                            && a.converted_curve_interval() == edge.curve.range_tuple()
                    })
            })
    }
}

pub(super) struct ProjectionFrame {
    pub occurrence: usize,
    pub factor: f64,
    pub placement: Matrix4,
    pub allowance: f64,
}

/// Captured before checked conversion, from a table bound to the original AST.
pub(super) struct OriginalGeometry {
    lines: HashMap<u64, NativeLineSource>,
    boundaries: HashMap<u64, Vec<SourceBoundary>>,
    finite_lines: HashMap<u64, finite_line::Source>,
}

struct SourceBoundary {
    orientation: bool,
    edges: Vec<(u64, bool)>,
}

fn entity_id<T>(holder: &PlaceHolder<T>) -> Option<u64> {
    match holder {
        PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
        _ => None,
    }
}

fn source_boundaries(table: &Table, face: &FaceSurfaceHolder) -> Option<Vec<SourceBoundary>> {
    face.bounds
        .iter()
        .map(|holder| {
            let bound = table.face_bound.get(&entity_id(holder)?)?;
            let edge_loop = table.edge_loop.get(&entity_id(&bound.bound)?)?;
            let edges = edge_loop
                .edge_list
                .iter()
                .map(|holder| {
                    let id = entity_id(holder)?;
                    match table.oriented_edge.get(&id) {
                        Some(edge) => Some((entity_id(&edge.edge_element)?, edge.orientation)),
                        None if table.edge_curve.contains_key(&id) => Some((id, true)),
                        _ => None,
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            Some(SourceBoundary {
                orientation: bound.orientation,
                edges,
            })
        })
        .collect()
}

impl OriginalGeometry {
    pub(super) fn capture(table: &Table, bytes: &[u8]) -> Self {
        let mut lines = HashMap::new();
        for (&id, holder) in &table.edge_curve {
            let PlaceHolder::Ref(Name::Entity(curve_id)) = &holder.edge_geometry else {
                continue;
            };
            if !table.line.contains_key(curve_id) {
                continue;
            }
            let (PlaceHolder::Ref(Name::Entity(_)), PlaceHolder::Ref(Name::Entity(_))) =
                (&holder.edge_start, &holder.edge_end)
            else {
                continue;
            };
            let Ok(edge) = holder.clone().into_owned(table) else {
                continue;
            };
            let CurveAny::Line(line) = &edge.edge_geometry else {
                continue;
            };
            if line.pnt.coordinates.len() != 3
                || edge.edge_start.vertex_geometry.coordinates.len() != 3
                || edge.edge_end.vertex_geometry.coordinates.len() != 3
            {
                continue;
            }
            lines.insert(
                id,
                NativeLineSource {
                    points: [
                        Point3::from(&edge.edge_start.vertex_geometry),
                        Point3::from(&edge.edge_end.vertex_geometry),
                    ],
                    origin: Point3::from(&line.pnt),
                    direction: line.dir.orientation.direction_ratios.clone(),
                    magnitude: line.dir.magnitude,
                    input: LineInput::read(table, id),
                },
            );
        }
        let boundaries = table
            .face_surface
            .iter()
            .filter_map(|(&id, face)| Some((id, source_boundaries(table, face)?)))
            .collect();
        Self {
            lines,
            boundaries,
            finite_lines: finite_line::capture(table, bytes),
        }
    }

    pub(super) fn convert_input_with_refusal<'input>(
        &self,
        input: &'input StepRepairInput,
        selection: (u64, Option<u64>),
        tolerance: PreviewTolerance,
        frames: &[ProjectionFrame],
    ) -> Result<Conversion<'input>, String> {
        let converted = input
            .convert_solid(selection.0, selection.1, Some(tolerance))
            .map_err(|error| error.to_string())?;
        self.check_projected_refusal(input.table(), converted.source(), frames)?;
        Ok(Conversion::Captured(converted))
    }

    pub(super) fn convert_normalized_with_refusal(
        &self,
        table: &Table,
        selection: (u64, Option<u64>),
        frames: &[ProjectionFrame],
    ) -> Result<Conversion<'static>, String> {
        let solid = table
            .manifold_solid_brep
            .get(&selection.0)
            .ok_or("normalized solid identity missing")?;
        let converted = match selection.1 {
            Some(owner) => table.to_source_bound_solid_in_representation_checked(solid, owner),
            None => table.to_source_bound_solid_without_connectivity_checked(solid),
        }
        .map_err(|error| error.to_string())?;
        self.check_projected_refusal(table, &converted, frames)?;
        Ok(Conversion::Normalized(converted))
    }

    // The result is created HERE from the same immutable input table: callers
    // cannot attach this refusal guard to an alternate caller-owned output.
    #[cfg(test)]
    pub(super) fn convert_with_projected_refusal(
        &self,
        table: &Table,
        original: &StepRepairSource,
        selection: (u64, Option<u64>),
        frames: &[ProjectionFrame],
    ) -> Result<StepRepairConversion, String> {
        let converted = table
            .to_source_bound_solid_with_repair_checked(selection.0, selection.1, original)
            .map_err(|error| error.to_string())?;
        self.check_projected_refusal(table, converted.source(), frames)?;
        Ok(converted)
    }

    fn check_projected_refusal(
        &self,
        table: &Table,
        converted: &SourceBoundStepSolid,
        frames: &[ProjectionFrame],
    ) -> Result<(), String> {
        for shell in converted.shells() {
            let Some(ids) = self.bind_edges(shell) else {
                continue;
            };
            for (index, id) in ids.into_iter().enumerate() {
                let (Some(parsed), Some(raw)) = (self.lines.get(&id), self.finite_lines.get(&id))
                else {
                    continue;
                };
                let Some(input) = &parsed.input else {
                    continue;
                };
                if LineInput::read(table, id).as_ref() != Some(input)
                    || !input.projected_branch(shell, index, id)
                {
                    continue;
                }
                for frame in frames {
                    if raw.projected_exceeds(frame.factor, frame.placement, frame.allowance)
                        == Ok(true)
                    {
                        return Err(format!(
                            "finite LINE repair edge #{id} occurrence {}: exact projected attachment lower bound exceeds selected {} mm",
                            frame.occurrence, frame.allowance
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn bind_edges(&self, source: &SourceBoundStepShell) -> Option<Vec<u64>> {
        let shell = source.shell();
        let mut ids = vec![None; shell.edges.len()];
        if shell.faces.len() != source.source_face_ids().len() {
            return None;
        }
        for (&id, face) in source.source_face_ids().iter().zip(&shell.faces) {
            let boundaries = self.boundaries.get(&id)?;
            if boundaries.len() != face.boundaries.len() {
                return None;
            }
            for (raw, emitted) in boundaries.iter().zip(&face.boundaries) {
                if raw.edges.len() != emitted.len() {
                    return None;
                }
                let forward = raw.orientation == face.orientation;
                for (index, edge_use) in emitted.iter().enumerate() {
                    let index = if forward {
                        index
                    } else {
                        raw.edges.len() - 1 - index
                    };
                    let (id, orientation) = raw.edges[index];
                    if edge_use.orientation != (orientation == forward) {
                        return None;
                    }
                    let slot = ids.get_mut(edge_use.index)?;
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

    // Styling may reference finite retained topology, never its infinite support.
    pub(super) fn retained_edge_ids(&self, source: &SourceBoundStepShell) -> Vec<u64> {
        self.bind_edges(source).unwrap_or_default()
    }
}

#[derive(Debug)]
pub(super) struct Operation {
    pub occurrence: usize,
    shell: Option<usize>,
    pub source_edge: Option<u64>,
    pub stage: &'static str,
    pub bound_mm: Result<f64, &'static str>,
    diagnostic_only: bool,
}

#[derive(Default)]
pub(super) struct Ledger(
    Vec<Operation>,
    Vec<Result<finite_line::Movement, &'static str>>,
    usize,
);

impl Ledger {
    pub(super) fn conversion(
        &mut self,
        original: &OriginalGeometry,
        owner: &Conversion<'_>,
        factor: f64,
        placement: Matrix4,
        occurrence: usize,
        shell: usize,
    ) {
        let source = &owner.source().shells()[shell];
        let ids = original.bind_edges(source);
        for (index, edge) in source.shell().edges.iter().enumerate() {
            let id = ids
                .as_ref()
                .and_then(|ids| ids.as_slice().get(index))
                .copied();
            self.0.push(Operation {
                occurrence,
                shell: None,
                source_edge: id,
                stage: "axis-aligned native LINE support substitution",
                diagnostic_only: true,
                bound_mm: id
                    .ok_or("original face/loop/edge binding is unproved")
                    .and_then(|id| {
                        line_support_bound(original, id, source, edge, factor, placement)
                    }),
            });
            self.1.push(
                id.ok_or("original finite LINE binding is unproved")
                    .and_then(|id| {
                        let raw = original.finite_lines.get(&id).ok_or(
                            "finite native LINE original numeric/domain authority is unavailable",
                        )?;
                        let Curve3D::Line(line) = &edge.curve else {
                            return Err("native LINE was replaced by another curve family");
                        };
                        let parsed = original
                            .lines
                            .get(&id)
                            .ok_or("original parsed LINE attachments are unavailable")?;
                        if parsed
                            .input
                            .as_ref()
                            .is_some_and(|input| input.projected_branch(source, index, id))
                        {
                            let operations = owner
                                .operations(shell)
                                .filter(|op| {
                                    op.stage() == "native-line-attachment-projection-v1"
                                        && op.source_ids().2 == Some(id)
                                })
                                .collect::<Vec<_>>();
                            let [operation] = operations.as_slice() else {
                                return Err(
                                    "projected attachment operation is missing or ambiguous",
                                );
                            };
                            return raw.projected_movement(
                                line,
                                parsed.points,
                                operation
                                    .curve_change()
                                    .ok_or("projected attachment domain capture is incomplete")?,
                                factor,
                                placement,
                                (occurrence, shell, id),
                            );
                        }
                        raw.movement(
                            line,
                            parsed.points,
                            factor,
                            placement,
                            (occurrence, shell, id),
                        )
                    }),
            );
        }
        self.unknown(
            occurrence,
            "curve trims and surface conversion",
            "source domains and whole supports are not traced",
        );
    }

    pub(super) fn conversion_operations<'a>(
        &mut self,
        operations: impl Iterator<Item = monstertruck_io::step::load::convert::StepRepairOperation<'a>>,
        complete: bool,
        shell: usize,
        factor: f64,
        placement: Matrix4,
        occurrence: usize,
    ) {
        for operation in operations {
            self.2 = self
                .2
                .saturating_add(usize::from(operation.curve_change().is_some()));
            if operation.stage().starts_with("same-master-planar-pcurve-") {
                let measured = operation
                    .curve_change()
                    .ok_or("planar same-call capture is incomplete")
                    .and_then(|change| {
                        finite_curve::planar_bound(operation.stage(), change, factor, placement)
                    });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: operation.stage(),
                    bound_mm: measured.map(|value| value.0),
                    diagnostic_only: false,
                });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: "planar-pcurve-numeric-conversion-v1",
                    bound_mm: measured.map(|value| value.1),
                    diagnostic_only: true,
                });
                continue;
            }
            if let Some(raw) = operation.raw_spline_change() {
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: operation.stage(),
                    bound_mm: finite_curve::raw_spline_bound(raw),
                    diagnostic_only: false,
                });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: "raw-spline-normalization-numeric-v1",
                    bound_mm: Err("normalized knot rounding is not bounded"),
                    diagnostic_only: true,
                });
                continue;
            }
            let numerical = operation.stage() == "numeric-conversion-v1";
            let attachment = operation.stage() == "native-line-attachment-projection-v1";
            let bound = operation.curve_change().map_or_else(
                || {
                    operation
                        .bound()
                        .map_err(|_| "owner stage remains unsupported")
                },
                |change| {
                    if attachment {
                        // Accounted exactly ONCE by the raw-decimal finite ledger.
                        Err("attachment pair is not original native-curve authority")
                    } else if operation.stage() == "native-sense-inversion-v1"
                        && let (Curve3D::Line(before), Curve3D::Line(after)) = change.curves()
                        && change.domains() == ((0., 1.), (1., 0.))
                        && before.0 == after.1
                        && before.1 == after.0
                    {
                        Ok(0.)
                    } else {
                        finite_curve::bound(change, factor, placement)
                    }
                },
            );
            self.0.push(Operation {
                occurrence,
                shell: Some(shell),
                source_edge: operation.source_ids().2,
                stage: operation.stage(),
                bound_mm: bound,
                diagnostic_only: numerical || attachment,
            });
        }
        // Even an empty/capped report must terminate in explicit incomplete coverage.
        if !complete {
            self.unknown(
                occurrence,
                "original conversion route coverage",
                "source/master/domain conversion coverage remains incomplete",
            );
        }
    }

    pub(super) fn meshing_operations(
        &mut self,
        owner: &monstertruck_meshing::tessellation::ShellRepairMesh<
            '_,
            Curve3D,
            monstertruck_io::step::load::step_geometry::Surface,
        >,
        _placement: Matrix4,
        occurrence: usize,
    ) {
        for operation in owner.operations() {
            let diagnostic_only = matches!(
                operation.stage(),
                "ordinary-mesh-approximation-v1" | "mesh-numeric-conversion-v1"
            );
            self.0.push(Operation {
                occurrence,
                shell: None,
                source_edge: None,
                stage: operation.stage(),
                bound_mm: operation
                    .bound()
                    .map_err(|_| "native finite mesh domain coverage remains unsupported"),
                diagnostic_only,
            });
        }
        // These views retain whole pieces. They are not certificates for source
        // curve approximation, finite face regions or inter-stage correspondence.
        for change in owner.domain_changes() {
            self.unknown(
                occurrence,
                change.stage(),
                "captured finite mesh domain still requires concrete surface/region proof",
            );
        }
        if owner.complete_bound().is_err() {
            self.unknown(
                occurrence,
                "complete meshing route coverage",
                "source/trim/finite-domain meshing route remains incomplete",
            );
        }
    }

    pub(super) fn numerical_unknown(
        &mut self,
        occurrence: usize,
        stage: &'static str,
        reason: &'static str,
    ) {
        self.0.push(Operation {
            occurrence,
            shell: None,
            source_edge: None,
            stage,
            bound_mm: Err(reason),
            diagnostic_only: true,
        });
    }

    /// A proved finite operation must respect its allowance even while the
    /// complete asset route is unknown. Partial evidence cannot certify the asset.
    pub(super) fn enforce_finite_lines(
        &self,
        occurrence: usize,
        allowance: f64,
    ) -> Result<(), String> {
        for movement in self.1.iter().filter_map(|entry| entry.as_ref().ok()) {
            if movement.occurrence == occurrence && !movement.within(allowance) {
                return Err(format!(
                    "finite LINE repair edge #{} occurrence {} movement bound {} mm ({}) exceeds repair budget {} mm",
                    movement.edge,
                    occurrence,
                    movement.bound_mm,
                    if movement.projected_attachment {
                        "attachment projection"
                    } else {
                        "native support substitution"
                    },
                    allowance,
                ));
            }
        }
        let mut paths = HashMap::<(Option<usize>, Option<u64>), num_rational::BigRational>::new();
        for operation in self
            .0
            .iter()
            .filter(|op| op.occurrence == occurrence && !op.diagnostic_only)
        {
            if let Ok(value) = operation.bound_mm {
                let value = num_rational::BigRational::from_float(value)
                    .filter(|v| *v >= num_rational::BigRational::from_integer(0.into()))
                    .ok_or("invalid known repair movement bound")?;
                *paths
                    .entry((operation.shell, operation.source_edge))
                    .or_default() += value;
            }
        }
        let budget =
            num_rational::BigRational::from_float(allowance).ok_or("invalid repair allowance")?;
        if paths.values().any(|path| path > &budget) {
            return Err(format!(
                "accumulated finite carrier repair occurrence {occurrence} exceeds repair budget {allowance} mm"
            ));
        }
        Ok(())
    }

    pub(super) fn unknown(&mut self, occurrence: usize, stage: &'static str, reason: &'static str) {
        self.0.push(Operation {
            occurrence,
            shell: None,
            source_edge: None,
            stage,
            bound_mm: Err(reason),
            diagnostic_only: false,
        });
    }

    /// Sum each operation once. Any missing proof poisons the complete bound.
    pub(super) fn complete_bound_mm(&self) -> Option<f64> {
        if self.0.is_empty() {
            return None;
        }
        let mut occurrences = HashMap::<usize, HashMap<(Option<usize>, Option<u64>), f64>>::new();
        for op in self.0.iter().filter(|op| !op.diagnostic_only) {
            let value = *op.bound_mm.as_ref().ok()?;
            if !value.is_finite() || value < 0.0 {
                return None;
            }
            let path = occurrences
                .entry(op.occurrence)
                .or_default()
                .entry((op.shell, op.source_edge))
                .or_default();
            *path = add_upper(*path, value);
            if !path.is_finite() {
                return None;
            }
        }
        for movement in &self.1 {
            let movement = movement.as_ref().ok()?;
            let path = occurrences
                .entry(movement.occurrence)
                .or_default()
                .entry((Some(movement.shell), Some(movement.edge)))
                .or_default();
            *path = add_upper(*path, movement.bound_mm);
            if !path.is_finite() {
                return None;
            }
        }
        Some(
            occurrences
                .values()
                .flat_map(|paths| paths.values())
                .copied()
                .fold(0.0, f64::max),
        )
    }

    pub(super) fn trace(&self) {
        if std::env::var_os("MT_DEBUG_REPAIR_MOVEMENT").is_some() {
            for op in &self.0 {
                eprintln!(
                    "STEP repair occurrence={} edge={:?} stage={} bound_mm={:?}",
                    op.occurrence, op.source_edge, op.stage, op.bound_mm
                );
            }
            for movement in &self.1 {
                match movement {
                    Ok(movement) => eprintln!(
                        "STEP finite LINE repair occurrence={} edge={} whole_domain_bound_mm={} numeric_bound_mm={}",
                        movement.occurrence,
                        movement.edge,
                        movement.bound_mm,
                        movement.numeric_bound_mm
                    ),
                    Err(reason) => eprintln!("STEP finite LINE repair unsupported={reason}"),
                }
            }
        }
    }
}

/// This proves only the whole parsed LINE support translation. Decimal decoding and
/// finite trim selection stay unknown, even when all diagnostics happen to be zero.
fn line_support_bound(
    original: &OriginalGeometry,
    id: u64,
    source: &SourceBoundStepShell,
    edge: &monstertruck_topology::compress::CompressedEdge<Curve3D>,
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    let raw = original
        .lines
        .get(&id)
        .ok_or("non-native LINE: master selection, seams and nonlinear changes are untraced")?;
    let Curve3D::Line(line) = &edge.curve else {
        return Err("native LINE was replaced by another curve family");
    };
    if source.shell().vertices.as_slice().get(edge.vertices.0) != Some(&raw.points[0])
        || source.shell().vertices.as_slice().get(edge.vertices.1) != Some(&raw.points[1])
    {
        return Err("original EDGE_CURVE vertex attachments changed");
    }
    // Restrict the proof to exact coordinate-axis supports; no tolerance tests,
    // inverse searches or endpoint samples can establish parallelism here.
    let direction = &raw.direction;
    if direction.len() != 3
        || !direction.iter().all(|v| v.is_finite())
        || !raw.magnitude.is_finite()
        || raw.magnitude == 0.0
    {
        return Err("invalid original LINE direction");
    }
    let axes: Vec<_> = (0..3).filter(|&axis| direction[axis] != 0.0).collect();
    let [axis] = axes.as_slice() else {
        return Err("native LINE support is not axis-aligned");
    };
    if line.0[*axis] == line.1[*axis] || (0..3).any(|i| i != *axis && line.0[i] != line.1[i]) {
        return Err("emitted LINE support does not preserve the native axis");
    }
    // Both entire supports are parallel. Their transverse coordinate difference is
    // one uniform translation of EVERY point; longitudinal trim changes stay unknown.
    let origin = raw.origin;
    if [origin, line.0, line.1]
        .iter()
        .any(|p| !(0..3).all(|i| p[i].is_finite()))
    {
        return Err("nonfinite LINE support");
    }
    let mut delta = [0.0; 3];
    for (i, d) in delta.iter_mut().enumerate() {
        if i != *axis {
            *d = upper((line.0[i] - origin[i]).abs());
        }
    }
    placed_translation_bound(delta, factor, placement)
}

fn upper(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value.next_up() }
}

fn add_upper(a: f64, b: f64) -> f64 {
    if a == 0.0 {
        b
    } else if b == 0.0 {
        a
    } else {
        (a + b).next_up()
    }
}

fn mul_upper(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        0.0
    } else {
        (a * b).next_up()
    }
}

/// An outward-rounded L1 bound on A*(factor*delta) covers rotations, reflections,
/// uniform scale and shear. Common occurrence translation is not a repair.
fn placed_translation_bound(
    delta: [f64; 3],
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    if delta.iter().any(|v| !v.is_finite() || *v < 0.0)
        || !factor.is_finite()
        || factor <= 0.0
        || !(0..4).all(|col| (0..4).all(|row| placement[col][row].is_finite()))
        || placement.x.w != 0.0
        || placement.y.w != 0.0
        || placement.z.w != 0.0
        || placement.w.w != 1.0
    {
        return Err("nonfinite or non-affine world transform");
    }
    let mut bound = 0.0;
    for row in 0..3 {
        for col in 0..3 {
            let term = mul_upper(mul_upper(delta[col], factor), placement[col][row].abs());
            bound = add_upper(bound, term);
        }
    }
    if bound.is_finite() && bound >= 0.0 {
        Ok(bound)
    } else {
        Err("nonfinite world movement bound")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_legacy_preview_keeps_default_explicit_and_scaled_settings() {
        let cylinder = include_str!("../../tests/fixtures/source-closed-cylinder-mm.step");
        let surface = cylinder
            .lines()
            .find(|line| line.starts_with("#30="))
            .unwrap();
        let radians = cylinder
            .replace(surface, "#30=CONICAL_SURFACE('',#44,1,0.7853981633974483);")
            .replace(
                "#63=ADVANCED_FACE('',(#62),#30,.F.);",
                "#63=ADVANCED_FACE('',(#62),#30,.T.);",
            )
            .replace(
                "#19=CARTESIAN_POINT('',(1,0,1));",
                "#19=CARTESIAN_POINT('',(2,0,1));",
            )
            .replace("#47=CIRCLE('',#45,1);", "#47=CIRCLE('',#45,2);")
            .replace(
                "#50=VECTOR('',#42,1);",
                "#74=DIRECTION('',(1,0,1));#50=VECTOR('',#74,1.4142135623730951);",
            );
        let degrees = radians
            .replace("0.7853981633974483", "45.")
            .replace("#83=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#80)));",
                "#90=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));\
                 #91=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#90);\
                 #92=(CONVERSION_BASED_UNIT('DEGREE',#91)NAMED_UNIT(#93)PLANE_ANGLE_UNIT());\
                 #93=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\
                 #94=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.000001),#80,'distance_accuracy_value','');\
                 #83=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#94))REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#80,#92)));");
        let original = StepRepairInput::parse(degrees.as_bytes().to_vec()).unwrap();
        let normalized = units::normalize_conical_exchange(original.exchange()).unwrap();
        assert_ne!(normalized.as_ref(), original.exchange());
        let mapped = |text: &str| {
            text.replace(
                "#84=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#73),#83);",
                "#84=SHAPE_REPRESENTATION('',(#73,#185),#83);\
             #181=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#73),#83);\
             #182=REPRESENTATION_MAP(#44,#181);\
             #184=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#40,2.,$);\
             #185=MAPPED_ITEM('',#182,#184);",
            )
        };
        for (text, reference) in [
            (&degrees, radians.clone()),
            (&mapped(&degrees), mapped(&radians)),
        ] {
            let reference = import_step(reference.into_bytes()).unwrap();
            for budget in [None, Some(0.02)] {
                let model = import_step_with_options(
                    text.as_bytes().to_vec(),
                    StepImportOptions {
                        preview_repair_budget_mm: budget,
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(model.original_bytes(), text.as_bytes());
                assert_eq!(model.bounds(), reference.bounds());
                assert_eq!(
                    model.face_occurrences().len(),
                    reference.face_occurrences().len()
                );
                assert_eq!(model.accuracy().max_repair_displacement_mm, None);
                assert_eq!(
                    model._repair_ledger.2, 0,
                    "normalized preview cannot lend sealed original pairs"
                );
            }
        }
        let mut table = Table::from_data_section(&normalized.data[0]);
        table.set_preview_floor_mm(0.02);
        let original_geometry = OriginalGeometry::capture(&table, degrees.as_bytes());
        let frame = ProjectionFrame {
            occurrence: 0,
            factor: 1.0,
            placement: Matrix4::from_scale(2.0),
            allowance: 0.02,
        };
        // This is the retained legacy scaled-settings branch. No original Table
        // is constructed or used to lend captures to the normalized conversion.
        let mut scaled = table.clone();
        scaled.set_preview_tolerance(PreviewTolerance {
            floor_mm: 0.01,
            declared_multiplier: 0.0,
        });
        let converted = original_geometry
            .convert_normalized_with_refusal(&scaled, (73, Some(84)), &[frame])
            .unwrap();
        assert!(
            converted
                .complete_bound(0)
                .unwrap_err()
                .contains("normalized")
        );
        assert_eq!(converted.operations(0).count(), 0);
        assert_eq!(converted.source().shells()[0].shell().faces.len(), 3);
        let over_budget = degrees.replace(
            "#51=LINE('',#10,#50);",
            "#190=CARTESIAN_POINT('',(1,0.020000000000000001,0));#51=LINE('',#190,#50);",
        );
        let error = import_step(over_budget.into_bytes())
            .err()
            .expect("normalization cannot erase the raw LINE refusal");
        assert!(error.contains("finite LINE repair edge #54"), "{error}");
    }

    #[test]
    #[ignore = "requires the existing authoritative34 manifest and original corpus bytes"]
    fn approved_corpus_operation_census() {
        use monstertruck_io::step::load::step_geometry::{ElementarySurface, Surface};
        let manifest = std::env::var("BOARDSTUDIO_CAD_CORPUS_MANIFEST").unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
        let rows = manifest["authoritative_imports"].as_array().unwrap();
        assert_eq!(rows.len(), 34);
        for row in rows {
            let path = row["source_path"].as_str().unwrap();
            let bytes = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path),
            )
            .unwrap();
            assert_eq!(bytes.len() as u64, row["source_bytes"].as_u64().unwrap());
            let model =
                import_step(bytes.clone()).unwrap_or_else(|error| panic!("{path}: {error}"));
            assert_eq!(model.original_bytes(), bytes);
            let expected = &row["expected"]["bounds_mm"];
            let bounds = model.bounds();
            if let Some(tolerance) = expected["tolerance_mm"]
                .as_f64()
                .or_else(|| expected["tolerance"].as_f64())
            {
                for (side, name) in ["min", "max"].iter().enumerate() {
                    for axis in 0..3 {
                        assert!(
                            (bounds[side][axis] - expected[name][axis].as_f64().unwrap()).abs()
                                <= tolerance,
                            "{path}: unchanged independent bounds oracle {name}[{axis}]"
                        );
                    }
                }
            } else {
                assert!(
                    expected.is_null(),
                    "retain explicit missing independent oracle"
                );
            }
            let raw = Table::from_data_section(&model._original_source.data[0]);
            let original = OriginalGeometry::capture(&raw, &bytes);
            let mut counts = std::collections::BTreeMap::<&str, usize>::new();
            let mut note = |name| {
                *counts.entry(name).or_default() += 1;
            };
            for (solid, _, representation) in &model.occurrences {
                for source in model.solids[solid].source[representation].shells() {
                    let ids = original
                        .bind_edges(source)
                        .expect("original edge identities remain complete");
                    for (edge, id) in source.shell().edges.iter().zip(ids) {
                        let input = raw.edge_curve[&id].clone().into_owned(&raw).unwrap();
                        match &input.edge_geometry {
                            CurveAny::Line(_) => {
                                let p = Point3::from(&input.edge_start.vertex_geometry);
                                let q = Point3::from(&input.edge_end.vertex_geometry);
                                match &edge.curve {
                                    Curve3D::Line(line) if [line.0, line.1] == [p, q] => {
                                        note("native_LINE_vertex_chord")
                                    }
                                    Curve3D::Line(_) => note("native_LINE_projected_interval"),
                                    _ => note("native_LINE_changed_family"),
                                }
                            }
                            CurveAny::BoundedCurve(_) => {
                                match Curve3D::try_from(&input.edge_geometry) {
                                    Ok(curve) if curve == edge.curve => {
                                        note("bounded_full_carrier_and_domain_identical")
                                    }
                                    Ok(_) => note("bounded_trim_or_relabel_requires_witness"),
                                    Err(_) => {
                                        note("bounded_normalization_requires_raw_basis_witness")
                                    }
                                }
                            }
                            CurveAny::Conic(_) => note("native_conic_finite_interval"),
                            CurveAny::Pcurve(_) => note("native_PCurve_lifted_interval"),
                            CurveAny::SurfaceCurve(_) => {
                                if let Curve3D::SurfaceCurve(actual) = &edge.curve {
                                    match Curve3D::try_from(&input.edge_geometry) {
                                        Ok(Curve3D::SurfaceCurve(raw))
                                            if raw.master_representation()
                                                == actual.master_representation() =>
                                        {
                                            note("surface_curve_selected_master_retained")
                                        }
                                        Ok(Curve3D::SurfaceCurve(_)) => {
                                            note("surface_curve_alternate_master_SELECTED")
                                        }
                                        _ => note(
                                            "surface_curve_original_master_construction_UNSUPPORTED",
                                        ),
                                    }
                                } else {
                                    note("surface_curve_changed_family");
                                }
                            }
                        }
                    }
                    for face in &source.shell().faces {
                        note(match &face.surface {
                            Surface::ElementarySurface(ElementarySurface::Plane(_)) => {
                                "plane_support"
                            }
                            Surface::ElementarySurface(ElementarySurface::CylindricalSurface(
                                _,
                            )) => "cylinder_support",
                            Surface::ElementarySurface(ElementarySurface::ConicalSurface(_)) => {
                                "cone_support"
                            }
                            Surface::ElementarySurface(ElementarySurface::Sphere(_)) => {
                                "sphere_support"
                            }
                            Surface::ElementarySurface(ElementarySurface::ToroidalSurface(_)) => {
                                "torus_support"
                            }
                            Surface::BsplineSurface(_) => "polynomial_spline_support",
                            Surface::NurbsSurface(_) => "rational_spline_support",
                            Surface::SweepSurface(_) => "sweep_support",
                        });
                    }
                }
            }
            for operation in &model._repair_ledger.0 {
                note(operation.stage);
            }
            for _ in 0..model._repair_ledger.2 {
                note("captured_actual_finite_curve_pair");
            }
            let movements: Vec<_> = model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().ok())
                .collect();
            let mut unsupported = std::collections::BTreeMap::<&str, usize>::new();
            for reason in model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().err())
            {
                *unsupported.entry(reason).or_default() += 1;
            }
            let covered = model
                .triangle_faces()
                .iter()
                .copied()
                .collect::<HashSet<_>>();
            assert_eq!(covered.len(), model.face_occurrences().len());
            println!(
                "REPAIR_CENSUS {}",
                serde_json::json!({"path":path,"routes":counts,
                "sourceFaces":model.source_faces().len(),"faceOccurrences":model.face_occurrences().len(),
                "lineOccurrences":model.line_presentation().occurrences().len(),
                "retainedOriginalFiniteLineAuthorities":original.finite_lines.len(),
                "projectedAttachmentOperations":movements.iter().filter(|op|op.projected_attachment).count(),
                "finiteLineUnsupportedReasons":unsupported,
                "finiteLineOperations":movements.len(),"nonzeroFiniteLineOperations":movements.iter().filter(|op| op.bound_mm>0.0).count(),
                "unknownFiniteLineOperations":model._repair_ledger.1.iter().filter(|op|op.is_err()).count(),
                "fallbackFaces":model.accuracy().fallback_meshed_faces,"bounds":model.bounds()})
            );
        }
    }

    fn shifted_line_tetra() -> String {
        include_str!("../../tests/fixtures/repair-finite-domain.step")
            .replace("0.03125", "0.001953125")
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn projected_native_line_has_actual_nonzero_attachment_movement() {
        let text = projected_line_tetra("0.015625");
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        let source = &model.solids[&101].source[&104];
        let shell = &source.shells()[0];
        let attachment = shell
            .connectivity_attachments()
            .iter()
            .find(|a| a.edge_id() == 23)
            .unwrap();
        let Curve3D::Line(line) = &shell.shell().edges[attachment.edge_index()].curve else {
            panic!("actual native LINE route");
        };
        assert_eq!([line.0.y, line.1.y], [0.015625; 2]);
        let movement = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .find(|op| op.edge == 23)
            .expect("actual same-call finite projected attachment must be accounted");
        assert_eq!(movement.bound_mm, 0.015625);
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.face_occurrences().len(), 4);
        assert_eq!(model._repair_ledger.complete_bound_mm(), None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn raw_spline_normalization_is_a_certified_identity_stage_and_keeps_asset_unknown() {
        let single = include_str!("../../tests/fixtures/repair-raw-spline.step");
        let spline = "#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#2),.POLYLINE_FORM.,.F.,.F.,(2,2),(3.,3.00000008),.UNSPECIFIED.);";
        assert_eq!(single.matches(spline).count(), 1);
        // A real multispan carrier: the interior label must follow the same recipe.
        let multispan = single.replace(
            spline,
            "#320=CARTESIAN_POINT('',(1.,0.,0.));#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#320,#2),.POLYLINE_FORM.,.F.,.F.,(2,1,2),(3.,3.00000004,3.00000008),.UNSPECIFIED.);",
        );
        for (text, reversed) in [single, multispan.as_str()]
            .into_iter()
            .flat_map(|text| [(text, false), (text, true)])
        {
            let text = if reversed {
                text.replace(
                    "#23=EDGE_CURVE('',#11,#12,#22,.T.);",
                    "#23=EDGE_CURVE('',#12,#11,#22,.F.);",
                )
                .replace(
                    "#52=ORIENTED_EDGE('',*,*,#23,.F.);",
                    "#52=ORIENTED_EDGE('',*,*,#23,.T.);",
                )
                .replace(
                    "#60=ORIENTED_EDGE('',*,*,#23,.T.);",
                    "#60=ORIENTED_EDGE('',*,*,#23,.F.);",
                )
            } else {
                text.to_string()
            };
            let model = import_step(text.as_bytes().to_vec()).unwrap();
            assert_eq!(model.original_bytes(), text.as_bytes());
            let stages = model
                ._repair_ledger
                .0
                .iter()
                .filter(|op| op.stage == "raw-spline-normalization-v1")
                .collect::<Vec<_>>();
            let [stage] = stages.as_slice() else {
                panic!("exactly one raw normalization stage, reversed {reversed}");
            };
            assert_eq!(stage.source_edge, Some(23));
            assert!(!stage.diagnostic_only);
            assert_eq!(stage.bound_mm, Ok(0.0), "ideal positive affine knot map");
            assert!(
                model
                    ._repair_ledger
                    .0
                    .iter()
                    .any(|op| op.stage == "raw-spline-normalization-numeric-v1"
                        && op.diagnostic_only
                        && op.bound_mm.is_err()),
                "knot rounding stays a separate unproved numeric term"
            );
            assert_eq!(model._repair_ledger.complete_bound_mm(), None);
        }
    }

    fn projected_line_tetra(offset: &str) -> String {
        let source = include_str!("../../tests/fixtures/repair-projected-line.step");
        assert_eq!(source.matches("0.015625").count(), 1);
        source.replace("0.015625", offset)
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn projected_native_line_exact_default_boundary_refuses_over_budget() {
        let text = projected_line_tetra("0.020000000000000001");
        let result = import_step(text.into_bytes());
        assert!(
            result.is_err(),
            "actual projected attachment above exact default .02 must refuse"
        );
        let error = result.err().unwrap();
        assert!(error.contains("finite LINE repair"), "{error}");
    }

    #[test]
    fn projected_native_line_attachment_does_not_borrow_native_selection_sense() {
        let source = projected_line_tetra("0.015625");
        let source = source.replace(
            "#23=EDGE_CURVE('',#11,#12,#22,.T.);",
            "#23=EDGE_CURVE('',#11,#12,#22,.F.);",
        );
        let model = import_step(source.as_bytes().to_vec()).unwrap();
        let movement = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .find(|op| op.edge == 23)
            .expect("native interval sense is not attachment-carrier authority");
        assert_eq!(movement.bound_mm, 0.015625);
        assert!(movement.projected_attachment);
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        let error = import_step(
            source
                .replace("0.015625", "0.020000000000000001")
                .into_bytes(),
        )
        .err()
        .unwrap();
        assert!(error.contains("finite LINE repair edge #23"), "{error}");
    }

    #[test]
    fn projected_native_line_budget_numeric_decomposition_and_sense_are_separate() {
        for reversed in [false, true] {
            let mut text = projected_line_tetra("0.020000000000000001");
            if reversed {
                for (a, b) in [
                    (
                        "#23=EDGE_CURVE('',#11,#12,#22,.T.);",
                        "#23=EDGE_CURVE('',#12,#11,#22,.F.);",
                    ),
                    (
                        "#52=ORIENTED_EDGE('',*,*,#23,.F.);",
                        "#52=ORIENTED_EDGE('',*,*,#23,.T.);",
                    ),
                    (
                        "#60=ORIENTED_EDGE('',*,*,#23,.T.);",
                        "#60=ORIENTED_EDGE('',*,*,#23,.F.);",
                    ),
                ] {
                    assert_eq!(text.matches(a).count(), 1);
                    text = text.replace(a, b);
                }
            }
            let model = import_step_with_options(
                text.as_bytes().to_vec(),
                StepImportOptions {
                    preview_repair_budget_mm: Some(0.02_f64.next_up()),
                    ..Default::default()
                },
            )
            .unwrap();
            let movements = model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().ok())
                .filter(|op| op.edge == 23)
                .collect::<Vec<_>>();
            assert_eq!(
                movements.len(),
                1,
                "one actual attachment correction, not both views"
            );
            let movement = movements[0];
            assert!(movement.projected_attachment);
            assert_eq!(movement.bound_mm, 0.02_f64.next_up());
            assert!(movement.numeric_bound_mm > 0. && movement.numeric_bound_mm < 1e-16);
            assert!(!movement.within(0.02));
            assert!(movement.within(0.02_f64.next_up()));
            assert_eq!(model.original_bytes(), text.as_bytes());
        }
        let text = projected_line_tetra("0.03125");
        let model = import_step_with_options(
            text.into_bytes(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.03125),
                ..Default::default()
            },
        )
        .unwrap();
        let movement = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .find(|op| op.edge == 23)
            .unwrap();
        assert_eq!(
            (movement.bound_mm, movement.numeric_bound_mm),
            (0.03125, 0.)
        );
        assert!(movement.within(0.03125));
        assert!(movement.within(0.03125_f64.next_up()));
        assert!(!movement.within(0.03125_f64.next_down()));
    }

    #[test]
    fn projected_native_line_caps_and_unrelated_authority_cannot_erase_refusal() {
        let unused = "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);";
        for padded in [false, true] {
            for before in [false, true] {
                let mutate = |source: String| {
                    let mut text = if before {
                        source.replace("DATA;", &format!("DATA;{unused}"))
                    } else {
                        source.replace(
                            "ENDSEC;END-ISO-10303-21;",
                            &format!("{unused}ENDSEC;END-ISO-10303-21;"),
                        )
                    };
                    if padded {
                        text.push_str(&" ".repeat(300_000));
                    }
                    text
                };
                let text = mutate(projected_line_tetra("0.020000000000000001"));
                let error = import_step(text.into_bytes())
                    .err()
                    .expect("known exact over-budget projection refuses even if trace capped");
                assert!(error.contains("finite LINE repair edge #23"), "{error}");
                let text = mutate(projected_line_tetra("0.015625"));
                let model = import_step(text.as_bytes().to_vec()).unwrap();
                assert_eq!(model.original_bytes(), text.as_bytes());
                assert_eq!(model.face_occurrences().len(), 4);
                assert_eq!(model.accuracy().max_repair_displacement_mm, None);
                assert!(
                    model
                        ._repair_ledger
                        .1
                        .iter()
                        .filter_map(|op| op.as_ref().ok())
                        .any(|op| op.edge == 23),
                    "padding/order/unknown scalar must preserve the sealed actual capture"
                );
            }
        }
    }

    #[test]
    fn projected_native_line_occurrences_apply_metre_units_and_rotation_once() {
        let assembly = include_str!("../../tests/fixtures/two-occurrences-mm.step");
        let tail = &assembly
            [assembly.find("#200=").unwrap()..assembly.find("ENDSEC;END-ISO-10303-21;").unwrap()];
        for metre in [false, true] {
            let text = projected_line_tetra(if metre { "0.000015625" } else { "0.015625" });
            let mut text = text.replace(
                "ENDSEC;END-ISO-10303-21;",
                &format!("{tail}ENDSEC;END-ISO-10303-21;"),
            );
            if metre {
                text = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
            }
            let model = import_step(text.as_bytes().to_vec()).unwrap();
            let movements = model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().ok())
                .filter(|op| op.edge == 23)
                .collect::<Vec<_>>();
            assert_eq!(movements.len(), 2);
            assert!(movements.iter().all(|op| op.projected_attachment
                && op.bound_mm == 0.015625
                && op.within(0.015625)));
            assert_ne!(movements[0].occurrence, movements[1].occurrence);
            for occurrence in 0..2 {
                let operations = model
                    ._repair_ledger
                    .0
                    .iter()
                    .filter(|op| op.occurrence == occurrence)
                    .collect::<Vec<_>>();
                let stage = |name| operations.iter().position(|op| op.stage == name).unwrap();
                for name in ["source-binding-v1", "original conversion route coverage"] {
                    assert_eq!(
                        operations.iter().filter(|op| op.stage == name).count(),
                        1,
                        "{name}: same-call conversion recorded once per occurrence"
                    );
                }
                assert_eq!(
                    operations
                        .iter()
                        .filter(|op| op.stage == "native-line-attachment-projection-v1"
                            && op.source_edge == Some(23))
                        .count(),
                    1,
                    "the changed shared edge is recorded once per occurrence"
                );
                assert!(stage("source-binding-v1") < stage("native-line-attachment-projection-v1"));
                assert!(
                    stage("native-line-attachment-projection-v1")
                        < stage("original conversion route coverage")
                );
                assert!(
                    stage("original conversion route coverage")
                        < stage("complete meshing route coverage"),
                    "conversion stays before meshing on each occurrence path"
                );
            }
            assert_eq!(model.original_bytes(), text.as_bytes());
            assert_eq!(model.face_occurrences().len(), 8);
            let equality = import_step_with_options(
                text.as_bytes().to_vec(),
                StepImportOptions {
                    preview_repair_budget_mm: Some(0.015625),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(equality.face_occurrences().len(), 8);
            let error = import_step_with_options(
                text.into_bytes(),
                StepImportOptions {
                    preview_repair_budget_mm: Some(0.015625_f64.next_down()),
                    ..Default::default()
                },
            )
            .err()
            .unwrap();
            assert!(
                error.contains("finite LINE repair")
                    || error
                        .contains("EDGE_CURVE attachment exceeds its source connectivity bound"),
                "{error}"
            );
        }
    }

    #[test]
    fn projected_native_line_refusal_does_not_borrow_mismatched_input_or_attachments() {
        let original = projected_line_tetra("0.020000000000000001");
        let original_table = Table::from_step_bytes(original.as_bytes()).unwrap();
        let geometry = OriginalGeometry::capture(&original_table, original.as_bytes());
        let authority = StepRepairSource::from_step_bytes(original.as_bytes()).unwrap();
        let changed = projected_line_tetra("0.015625");
        let mut table = Table::from_step_bytes(changed.as_bytes()).unwrap();
        table.set_preview_floor_mm(0.02);
        let frames = [ProjectionFrame {
            occurrence: 0,
            factor: 1.,
            placement: Matrix4::identity(),
            allowance: 0.02,
        }];
        let result = geometry
            .convert_with_projected_refusal(&table, &authority, (101, Some(104)), &frames)
            .unwrap();
        assert!(result.complete_bound(0).unwrap_err().contains("mismatch"));
        assert!(result.operations(0).all(|op| op.curve_change().is_none()));
        let shell = &result.source().shells()[0];
        let input = geometry.lines[&23].input.as_ref().unwrap();
        let attachment = shell
            .connectivity_attachments()
            .iter()
            .find(|a| a.edge_id() == 23)
            .unwrap();
        assert!(input.projected_branch(shell, attachment.edge_index(), 23));
        assert!(!input.projected_branch(shell, attachment.edge_index(), 27));
        let other = shell
            .connectivity_attachments()
            .iter()
            .find(|a| a.edge_id() == 27)
            .unwrap();
        assert!(!input.projected_branch(shell, other.edge_index(), 23));
        let ids_changed = original.replace(
            "#22=LINE('',#310,#21);",
            "#311=CARTESIAN_POINT('',(0.,0.020000000000000001,0.));#22=LINE('',#311,#21);",
        );
        let mut table = Table::from_step_bytes(ids_changed.as_bytes()).unwrap();
        table.set_preview_floor_mm(0.02);
        let result = geometry
            .convert_with_projected_refusal(&table, &authority, (101, Some(104)), &frames)
            .unwrap();
        assert!(
            result.complete_bound(0).is_err(),
            "different support identity cannot borrow original authority"
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn finite_line_repair_exceeding_preview_budget_refuses_without_attachment_metadata() {
        let text = include_str!("../../tests/fixtures/repair-finite-domain.step");
        let result = import_step(text.as_bytes().to_vec());
        assert!(
            result.is_err(),
            "a complete finite LINE moved 0.03125 mm even though emitted endpoint attachments are exact"
        );
        assert!(result.err().unwrap().contains("finite LINE repair"));
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn unrelated_unsupported_numeric_entity_cannot_erase_an_over_budget_operation() {
        let original = include_str!("../../tests/fixtures/repair-finite-domain.step");
        let source = original.replace("ENDSEC;END-ISO-10303-21;",
            "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);ENDSEC;END-ISO-10303-21;");
        assert_ne!(source, original);
        let error = import_step(source.as_bytes().to_vec()).err().unwrap();
        assert!(error.contains("finite LINE repair edge #23"), "{error}");
        let model = import_step_with_options(
            source.as_bytes().to_vec(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.03125),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(model.original_bytes(), source.as_bytes());
        assert_eq!(model.source_faces().len(), 4);
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn whole_finite_line_movement_honors_exact_budget_boundary_and_retains_unknown_asset() {
        let text = include_str!("../../tests/fixtures/repair-finite-domain.step");
        let budget = 0.03125_f64;
        for allowance in [budget, budget.next_up()] {
            let model = import_step_with_options(
                text.as_bytes().to_vec(),
                StepImportOptions {
                    preview_repair_budget_mm: Some(allowance),
                    ..Default::default()
                },
            )
            .unwrap();
            let operation = model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().ok())
                .find(|op| op.edge == 23)
                .unwrap();
            assert_eq!(operation.bound_mm, budget);
            assert_eq!(operation.occurrence, 0);
            assert_eq!(model.original_bytes(), text.as_bytes());
            assert_eq!(model.source_faces().len(), 4);
            assert_eq!(model.face_occurrences().len(), 4);
            assert_eq!(
                model
                    .triangle_faces()
                    .iter()
                    .copied()
                    .collect::<HashSet<_>>()
                    .len(),
                4
            );
            assert_eq!(model.accuracy().selected_repair_budget_mm, Some(allowance));
            assert_eq!(model.accuracy().max_repair_displacement_mm, None);
            assert_eq!(model.accuracy().analysis_error_bound_mm, None);
        }
        let error = import_step_with_options(
            text.as_bytes().to_vec(),
            StepImportOptions {
                preview_repair_budget_mm: Some(budget.next_down()),
                ..Default::default()
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("finite LINE repair"), "{error}");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn finite_line_default_floor_and_metre_occurrence_budget_are_world_mm() {
        let text = shifted_line_tetra().replace("0.001953125", "0.015625");
        let model = import_step(text.into_bytes()).unwrap();
        assert_eq!(
            model
                ._repair_ledger
                .1
                .iter()
                .filter_map(|op| op.as_ref().ok())
                .find(|op| op.edge == 23)
                .unwrap()
                .bound_mm,
            0.015625
        );
        let text = include_str!("../../tests/fixtures/two-occurrences-mm.step")
            .replace(
                "#22=LINE('',#1,#21);",
                "#310=CARTESIAN_POINT('',(0.,0.000015625,0.));#22=LINE('',#310,#21);",
            )
            .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        let model = import_step(text.clone().into_bytes()).unwrap();
        let bounds: Vec<_> = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .filter(|op| op.edge == 23)
            .map(|op| (op.occurrence, op.bound_mm))
            .collect();
        assert_eq!(bounds, [(0, 0.015625), (1, 0.015625)]);
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.face_occurrences().len(), 8);
        let error = import_step_with_options(
            text.into_bytes(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.015625_f64.next_down()),
                ..Default::default()
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("finite LINE repair"), "{error}");
    }

    #[test]
    fn ledger_sums_successive_stages_and_maxes_disjoint_regions_and_occurrences() {
        let op = |occurrence, source_edge, value| Operation {
            occurrence,
            shell: Some(0),
            source_edge,
            stage: "finite-domain test stage",
            bound_mm: Ok(value),
            diagnostic_only: false,
        };
        let mut ledger = Ledger(
            vec![
                op(0, Some(1), 0.125),
                op(0, Some(1), 0.125),
                op(0, Some(2), 0.125),
                op(1, Some(1), 0.125),
            ],
            Vec::new(),
            0,
        );
        assert_eq!(ledger.complete_bound_mm(), Some(0.25_f64.next_up()));
        assert!(
            ledger.enforce_finite_lines(0, 0.25).is_ok(),
            "exact accumulated budget equality passes"
        );
        assert!(
            ledger
                .enforce_finite_lines(0, 0.25_f64.next_down())
                .is_err()
        );
        let mut separate = Ledger::default();
        let mut first = op(0, Some(1), 0.015625);
        first.shell = Some(0);
        let mut second = op(0, Some(1), 0.015625);
        second.shell = Some(1);
        separate.0.extend([first, second]);
        assert_eq!(separate.complete_bound_mm(), Some(0.015625));
        assert!(
            separate.enforce_finite_lines(0, 0.02).is_ok(),
            "disjoint shell occurrences are maxed"
        );
        ledger.unknown(1, "omitted-stage control", "whole domain was not captured");
        assert_eq!(ledger.complete_bound_mm(), None);
        assert_eq!(Ledger::default().complete_bound_mm(), None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn native_line_support_translation_is_measured_on_the_real_import_route() {
        let text = shifted_line_tetra();
        let model = import_step(text.clone().into_bytes()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        let bound = operation
            .bound_mm
            .expect("the whole native LINE support translation is bounded");
        assert!((0.001953125..0.001953126).contains(&bound), "{bound}");
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn support_measurement_uses_world_units_and_keeps_occurrences_separate() {
        let assembly = include_str!("../../tests/fixtures/two-occurrences-mm.step");
        let text = assembly
            .replace(
                "#22=LINE('',#1,#21);",
                "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);",
            )
            .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        assert!(import_step(text.clone().into_bytes()).is_err());
        let model = import_step_with_options(
            text.into_bytes(),
            StepImportOptions {
                preview_repair_budget_mm: Some(2.0),
                ..Default::default()
            },
        )
        .unwrap();
        let operations: Vec<_> = model
            ._repair_ledger
            .0
            .iter()
            .filter(|op| {
                op.source_edge == Some(23)
                    && op.stage == "axis-aligned native LINE support substitution"
            })
            .collect();
        assert_eq!(
            operations.len(),
            2,
            "one correction per occurrence, not per face use"
        );
        assert_ne!(operations[0].occurrence, operations[1].occurrence);
        for op in operations {
            let bound = op.bound_mm.unwrap();
            assert!((1.953125..1.953126).contains(&bound), "{op:?}");
        }
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn tilted_native_support_and_selected_pcurves_cannot_borrow_the_line_proof() {
        let tilted = shifted_line_tetra().replace(
            "#20=DIRECTION('',(2.,0.,0.));",
            "#20=DIRECTION('',(2.,0.000001,0.));",
        );
        let model = import_step(tilted.into_bytes()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        assert_eq!(
            operation.bound_mm,
            Err("native LINE support is not axis-aligned")
        );
        let model =
            import_step(include_bytes!("../../tests/fixtures/source-pcurve-cube.step").to_vec())
                .unwrap();
        assert!(
            model
                ._repair_ledger
                .0
                .iter()
                .filter(|op| op.stage == "axis-aligned native LINE support substitution")
                .all(|op| op.bound_mm.is_err())
        );
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn unchanged_parsed_line_support_does_not_certify_the_asset_or_source_trim() {
        let model =
            import_step(include_bytes!("../../tests/fixtures/tetra-mm.step").to_vec()).unwrap();
        let operation = model
            ._repair_ledger
            .0
            .iter()
            .find(|op| op.source_edge == Some(23))
            .unwrap();
        assert_eq!(
            operation.bound_mm,
            Ok(0.0),
            "only this support is unchanged"
        );
        assert_eq!(model.accuracy().sampled_support_checks, 0);
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert!(
            model
                ._repair_ledger
                .0
                .iter()
                .any(|op| op.stage == "curve trims and surface conversion" && op.bound_mm.is_err())
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn original_topology_identity_mismatch_refuses_a_support_proof() {
        let text = shifted_line_tetra();
        let model = import_step(text.clone().into_bytes()).unwrap();
        let altered = text.replace(
            "#50=ORIENTED_EDGE('',*,*,#27,.T.);",
            "#50=ORIENTED_EDGE('',*,*,#23,.T.);",
        );
        let exchange = parser::parse(&altered).unwrap();
        let raw = OriginalGeometry::capture(
            &Table::from_data_section(&exchange.data[0]),
            altered.as_bytes(),
        );
        assert!(
            raw.bind_edges(&model.solids[&101].source[&104].shells()[0])
                .is_none()
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn world_translation_bound_covers_reflection_scale_and_underflow() {
        let placement =
            Matrix4::from_translation(monstertruck_modeling::Vector3::new(1e10, 0., 0.))
                * Matrix4::from_scale(-3.0);
        let bound = placed_translation_bound([0., 0.125, 0.], 2.0, placement).unwrap();
        assert!((0.75..0.750001).contains(&bound));
        assert!(
            placed_translation_bound([f64::from_bits(1), 0., 0.], 0.5, Matrix4::identity())
                .unwrap()
                > 0.0
        );
        assert!(placed_translation_bound([f64::MAX, 0., 0.], 2.0, Matrix4::identity()).is_err());
    }
}
