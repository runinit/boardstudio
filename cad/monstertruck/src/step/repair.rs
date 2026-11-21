//! Internal operation accounting. A support translation is not whole-asset movement.
mod conic;
mod finite_curve;
mod finite_line;
pub(in crate::step) mod spelling;
use super::*;
use monstertruck_io::step::load::step_geometry::{Conic3D, Curve3D};
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
    direction: [f64; 3],
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
    /// Native-frame vertex points `(start, end)` after edge sense, per EDGE_CURVE.
    native_ends: HashMap<u64, [Point3; 2]>,
    /// The constructor result for each EDGE_CURVE whose support is a circle/ellipse.
    conic_frames: HashMap<u64, Matrix4>,
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
        for &id in table.edge_curve.keys() {
            let Some(input) = LineInput::read(table, id) else {
                continue;
            };
            let point = |[x, y, z]: [f64; 3]| Point3::new(x, y, z);
            lines.insert(
                id,
                NativeLineSource {
                    points: [point(input.coordinates[2]), point(input.coordinates[3])],
                    origin: point(input.coordinates[0]),
                    direction: input.coordinates[1],
                    magnitude: input.magnitude,
                    input: Some(input),
                },
            );
        }
        let boundaries = table
            .face_surface
            .iter()
            .filter_map(|(&id, face)| Some((id, source_boundaries(table, face)?)))
            .collect();
        let point = |holder: &PlaceHolder<_>| -> Option<Point3> {
            let PlaceHolder::Ref(Name::Entity(vertex)) = holder else {
                return None;
            };
            let PlaceHolder::Ref(Name::Entity(point)) =
                &table.vertex_point.get(vertex)?.vertex_geometry
            else {
                return None;
            };
            let [x, y, z] = table.cartesian_point.get(point)?.coordinates.as_slice() else {
                return None;
            };
            Some(Point3::new(*x, *y, *z))
        };
        let native_ends = table
            .edge_curve
            .iter()
            .filter_map(|(&id, edge)| {
                let (start, end) = (point(&edge.edge_start)?, point(&edge.edge_end)?);
                Some((
                    id,
                    if edge.same_sense {
                        [start, end]
                    } else {
                        [end, start]
                    },
                ))
            })
            .collect();
        let conic_frames = table
            .edge_curve
            .iter()
            .filter_map(|(&id, edge)| {
                let PlaceHolder::Ref(Name::Entity(curve)) = &edge.edge_geometry else {
                    return None;
                };
                let frame = if let Some(circle) = table.circle.get(curve) {
                    let circle = circle.clone().into_owned(table).ok()?;
                    Matrix4::try_from(&circle.position).ok()? * Matrix4::from_scale(circle.radius)
                } else {
                    let ellipse = table.ellipse.get(curve)?.clone().into_owned(table).ok()?;
                    Matrix4::try_from(&ellipse.position).ok()?
                        * Matrix4::from_nonuniform_scale(
                            ellipse.semi_axis_1,
                            ellipse.semi_axis_2,
                            f64::min(ellipse.semi_axis_1, ellipse.semi_axis_2),
                        )
                };
                Some((id, frame))
            })
            .collect();
        Self {
            lines,
            boundaries,
            finite_lines: finite_line::capture(table, bytes),
            native_ends,
            conic_frames,
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
        original: Option<(&OriginalGeometry, &SourceBoundStepShell)>,
        operations: impl Iterator<Item = monstertruck_io::step::load::convert::StepRepairOperation<'a>>,
        complete: bool,
        (shell, factor, placement, occurrence): (usize, f64, Matrix4, usize),
    ) {
        let operations = operations.collect::<Vec<_>>();
        // A bounded curve whose authored full domain was kept resolves its own
        // finite selection and (nonexistent) master choice. The residual between
        // the emitted curve ends and the original vertices is the only movement.
        let mut resolved = std::collections::HashSet::new();
        for operation in &operations {
            if operation.stage() == "native-full-interval-v1"
                && let (Some(id), Some(change)) =
                    (operation.source_ids().2, operation.curve_change())
            {
                let residual = original
                    .and_then(|(original, _)| original.native_ends.get(&id))
                    .ok_or("original EDGE_CURVE vertex attachments are unavailable")
                    .and_then(|ends| finite_curve::end_residual(change, ends, factor, placement));
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: Some(id),
                    stage: "native-full-interval-attachment-v1",
                    bound_mm: residual,
                    diagnostic_only: false,
                });
                resolved.insert(id);
            }
        }
        // A circle/ellipse edge keeps its source support; its selection is the
        // counter-clockwise arc between the vertex parameters, so only the exact
        // end residual against the original vertices is repair movement.
        if let Some((original, source)) = original
            && let Some(ids) = original.bind_edges(source)
        {
            for (edge, id) in source.shell().edges.iter().zip(ids) {
                let Curve3D::Conic(Conic3D::Ellipse(ellipse)) = &edge.curve else {
                    continue;
                };
                let residual = original
                    .conic_frames
                    .get(&id)
                    .zip(original.native_ends.get(&id))
                    .ok_or("original conic record or vertex attachments are unavailable")
                    .and_then(|(frame, ends)| {
                        conic::end_residual(ellipse, frame, ends, factor, placement)
                    });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: Some(id),
                    stage: "native-conic-attachment-v1",
                    bound_mm: residual,
                    diagnostic_only: false,
                });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: Some(id),
                    stage: "native-conic-numeric-v1",
                    bound_mm: Err("binary64 conic constructor rounding is not bounded"),
                    diagnostic_only: true,
                });
                resolved.insert(id);
            }
        }
        for operation in &operations {
            if operation.stage() == "same-master-planar-uvline-trim-v1"
                && let (Some(id), Some(change)) =
                    (operation.source_ids().2, operation.curve_change())
            {
                let residual = original
                    .and_then(|(original, _)| original.native_ends.get(&id))
                    .ok_or("original EDGE_CURVE vertex attachments are unavailable")
                    .and_then(|ends| {
                        finite_curve::planar_line_end_residual(change, ends, factor, placement)
                    });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: Some(id),
                    stage: "planar-uvline-attachment-v1",
                    bound_mm: residual,
                    diagnostic_only: false,
                });
                resolved.insert(id);
            }
        }
        for operation in operations {
            if matches!(
                operation.stage(),
                "native-finite-interval-v1" | "selected-master-v1"
            ) && operation
                .source_ids()
                .2
                .is_some_and(|id| resolved.contains(&id))
            {
                continue;
            }
            self.2 = self
                .2
                .saturating_add(usize::from(operation.curve_change().is_some()));
            if operation.stage().starts_with("same-master-planar-uvline-") {
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: operation.stage(),
                    bound_mm: operation
                        .curve_change()
                        .ok_or("UV line same-call capture is incomplete")
                        .and_then(|change| {
                            finite_curve::planar_bound(operation.stage(), change, factor, placement)
                        })
                        .map(|value| value.0),
                    diagnostic_only: false,
                });
                self.0.push(Operation {
                    occurrence,
                    shell: Some(shell),
                    source_edge: operation.source_ids().2,
                    stage: "planar-uvline-numeric-conversion-v1",
                    bound_mm: Err("binary64 UV line retention is not bounded"),
                    diagnostic_only: true,
                });
                continue;
            }
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
    if !direction.iter().all(|v| v.is_finite())
        || !raw.magnitude.is_finite()
        || raw.magnitude == 0.0
    {
        return Err("invalid original LINE direction");
    }
    let mut axes = (0..3).filter(|&axis| direction[axis] != 0.0);
    let (Some(axis), None) = (axes.next(), axes.next()) else {
        return Err("native LINE support is not axis-aligned");
    };
    if line.0[axis] == line.1[axis] || (0..3).any(|i| i != axis && line.0[i] != line.1[i]) {
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
        if i != axis {
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
mod tests;
