//! Actual meshing ownership and finite-domain mutation captures.
use super::*;
use crate::Point2;
use std::result::Result;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const STORAGE_CAP: usize = 256 * 1024 * 1024;
const OPERATION_CAP: usize = 1_000_000;
type Output = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;

#[derive(Clone, Copy, Debug)]
pub(super) enum Stage {
    SourceDomain,
    Approximation,
    Numerical,
    Branch,
    BoundaryRemap,
    Loop,
    Fallback,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::SourceDomain => "source-curve-to-UV-domain-v1",
            Self::Approximation => "ordinary-mesh-approximation-v1",
            Self::Numerical => "mesh-numeric-conversion-v1",
            Self::Branch => "closed-piece-branch-v1",
            Self::BoundaryRemap => "aligned-boundary-remap-v1",
            Self::Loop => "face-loop-reconciliation-v1",
            Self::Fallback => "preview-boundary-fallback-v1",
        }
    }
}

#[derive(Debug)]
struct Change {
    stage: Stage,
    face: usize,
    before_uv: Vec<Point2>,
    after_uv: Vec<Point2>,
    before_xyz: Vec<Point3>,
    after_xyz: Vec<Point3>,
}
#[derive(Debug)]
struct Operation {
    stage: Stage,
    face: usize,
    bound: Result<f64, &'static str>,
}
#[derive(Debug, Default)]
struct FaceTrace {
    changes: Vec<Change>,
    operations: Vec<Operation>,
    failure: Option<&'static str>,
}
#[derive(Debug)]
pub(super) struct Capture {
    faces: Vec<Mutex<FaceTrace>>,
    bytes: AtomicUsize,
    operations: AtomicUsize,
    failure: Option<&'static str>,
}
#[derive(Clone, Copy)]
pub(super) struct FaceCapture<'a> {
    capture: &'a Capture,
    face: usize,
}
pub(super) struct Pending<'a> {
    owner: FaceCapture<'a>,
    change: Change,
}

impl Capture {
    fn new(faces: usize) -> Self {
        let bytes = faces.checked_mul(size_of::<Mutex<FaceTrace>>());
        let mut traces = Vec::new();
        let failure = if faces > OPERATION_CAP || bytes.is_none_or(|n| n > STORAGE_CAP) {
            Some("mesh repair trace resource cap")
        } else if traces.try_reserve_exact(faces).is_err() {
            Some("mesh repair trace allocation failed")
        } else {
            None
        };
        if failure.is_none() {
            traces.resize_with(faces, Mutex::default);
        }
        Self {
            faces: traces,
            bytes: AtomicUsize::new(bytes.unwrap_or(STORAGE_CAP)),
            operations: AtomicUsize::new(0),
            failure,
        }
    }
    pub(super) fn face(&self, face: usize) -> Option<FaceCapture<'_>> {
        self.faces.get(face).map(|_| FaceCapture {
            capture: self,
            face,
        })
    }
    fn charge(&self, bytes: usize) -> bool {
        self.bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes).filter(|n| *n <= STORAGE_CAP)
            })
            .is_ok()
    }
}
impl FaceCapture<'_> {
    pub(super) fn reserve_auxiliary(self, bytes: usize) -> bool {
        if self.capture.charge(bytes) {
            true
        } else {
            self.fail("mesh repair auxiliary storage resource cap");
            false
        }
    }
    pub(super) fn mark(self) -> (usize, usize) {
        let trace = self.capture.faces[self.face].lock().unwrap();
        (trace.operations.len(), trace.changes.len())
    }
    pub(super) fn discard_since(self, mark: (usize, usize)) {
        let mut trace = self.capture.faces[self.face].lock().unwrap();
        trace.operations.truncate(mark.0);
        trace.changes.truncate(mark.1);
    }
    fn fail(self, reason: &'static str) {
        self.capture.faces[self.face].lock().unwrap().failure = Some(reason);
    }
    pub(super) fn unsupported(self, stage: Stage, reason: &'static str) {
        if self.capture.operations.fetch_add(1, Ordering::Relaxed) >= OPERATION_CAP
            || !self.capture.charge(2 * size_of::<Operation>())
        {
            self.fail("mesh repair operation resource cap");
            return;
        }
        let mut trace = self.capture.faces[self.face].lock().unwrap();
        if trace.operations.try_reserve_exact(1).is_err() {
            trace.failure = Some("mesh repair operation allocation failed");
            return;
        }
        trace.operations.push(Operation {
            stage,
            face: self.face,
            bound: Err(reason),
        });
    }
    pub(super) fn before<'a>(
        self,
        stage: Stage,
        uv: impl Iterator<Item = Point2>,
        xyz: impl Iterator<Item = Point3>,
    ) -> Option<Pending<'a>>
    where
        Self: 'a,
    {
        let (Some(uv_len), Some(xyz_len)) = (uv.size_hint().1, xyz.size_hint().1) else {
            self.fail("mesh repair control capacity unavailable");
            return None;
        };
        let Some(bytes) = uv_len
            .checked_mul(size_of::<Point2>())
            .and_then(|n| {
                xyz_len
                    .checked_mul(size_of::<Point3>())
                    .and_then(|m| n.checked_add(m))
            })
        else {
            self.fail("mesh repair control storage overflow");
            return None;
        };
        if !self.capture.charge(bytes) {
            self.fail("mesh repair control storage resource cap");
            return None;
        }
        let (mut before_uv, mut before_xyz) = (Vec::new(), Vec::new());
        if before_uv.try_reserve_exact(uv_len).is_err()
            || before_xyz.try_reserve_exact(xyz_len).is_err()
        {
            self.fail("mesh repair control allocation failed");
            return None;
        }
        before_uv.extend(uv);
        before_xyz.extend(xyz);
        Some(Pending {
            owner: self,
            change: Change {
                stage,
                face: self.face,
                before_uv,
                before_xyz,
                after_uv: Vec::new(),
                after_xyz: Vec::new(),
            },
        })
    }
}
impl Pending<'_> {
    pub(super) fn finish(
        mut self,
        uv: impl Iterator<Item = Point2>,
        xyz: impl Iterator<Item = Point3>,
    ) {
        let (Some(uv_len), Some(xyz_len)) = (uv.size_hint().1, xyz.size_hint().1) else {
            self.owner.fail("mesh repair control capacity unavailable");
            return;
        };
        let Some(bytes) = uv_len
            .checked_mul(size_of::<Point2>())
            .and_then(|n| {
                xyz_len
                    .checked_mul(size_of::<Point3>())
                    .and_then(|m| n.checked_add(m))
            })
        else {
            self.owner.fail("mesh repair control storage overflow");
            return;
        };
        if !self
            .owner
            .capture
            .charge(bytes + 2 * size_of::<Change>())
        {
            self.owner.fail("mesh repair control storage resource cap");
            return;
        }
        if self.change.after_uv.try_reserve_exact(uv_len).is_err()
            || self.change.after_xyz.try_reserve_exact(xyz_len).is_err()
        {
            self.owner.fail("mesh repair control allocation failed");
            return;
        }
        self.change.after_uv.extend(uv);
        self.change.after_xyz.extend(xyz);
        let mut trace = self.owner.capture.faces[self.owner.face].lock().unwrap();
        if trace.changes.try_reserve_exact(1).is_err() {
            trace.failure = Some("mesh repair change allocation failed");
            return;
        }
        trace.changes.push(self.change);
    }
}

/// Immutable view of a whole finite piece before and after the actual mutation.
#[derive(Debug)]
pub struct MeshRepairDomainChange<'a> {
    change: &'a Change,
}
impl MeshRepairDomainChange<'_> {
    /// Fixed identity of the owner stage.
    pub fn stage(&self) -> &'static str {
        self.change.stage.name()
    }
    /// Actual input face and optional shared edge (unknown for a whole loop).
    pub fn indices(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.change.face), None)
    }
    /// Complete finite UV-linear piece controls, preserving the piece boundary.
    pub fn uv_controls(&self) -> (&[Point2], &[Point2]) {
        (&self.change.before_uv, &self.change.after_uv)
    }
    /// Actual carried XYZ controls; absent association remains explicitly empty.
    pub fn spatial_controls(&self) -> (&[Point3], &[Point3]) {
        (&self.change.before_xyz, &self.change.after_xyz)
    }
}
/// Borrowed diagnostic stage, never a caller-constructible certificate.
#[derive(Debug)]
pub struct MeshRepairOperation<'a> {
    operation: &'a Operation,
}
impl MeshRepairOperation<'_> {
    /// Fixed owner-stage identity.
    pub fn stage(&self) -> &'static str {
        self.operation.stage.name()
    }
    /// Actual input face and optional shared edge.
    pub fn indices(&self) -> (Option<usize>, Option<usize>) {
        (Some(self.operation.face), None)
    }
    /// Geometric repair in the actual supplied shell's coordinate frame.
    pub fn bound(&self) -> Result<f64, &str> {
        self.operation.bound
    }
}
/// Borrows only the actual input and owns the same invocation's final mesh/trace.
#[derive(Debug)]
pub struct ShellRepairMesh<'a, C, S> {
    input: &'a CompressedShell<Point3, C, S>,
    mesh: Output,
    fallback: Vec<usize>,
    traces: Vec<FaceTrace>,
    failure: Option<&'static str>,
}
impl<C, S> ShellRepairMesh<'_, C, S> {
    /// The exact input borrowed by this meshing invocation.
    pub fn input(&self) -> &CompressedShell<Point3, C, S> {
        self.input
    }
    /// The actual emitted result owned by this invocation.
    pub fn mesh(&self) -> &Output {
        &self.mesh
    }
    /// Faces meshed through the existing preview fallback.
    pub fn fallback_faces(&self) -> &[usize] {
        &self.fallback
    }
    /// Complete per-stage diagnostics; a missing route is unsupported.
    pub fn operations(&self) -> impl Iterator<Item = MeshRepairOperation<'_>> {
        self.traces
            .iter()
            .flat_map(|trace| &trace.operations)
            .map(|operation| MeshRepairOperation { operation })
    }
    /// Immutable full finite-piece mutation data for a concrete private certifier.
    pub fn domain_changes(&self) -> impl Iterator<Item = MeshRepairDomainChange<'_>> {
        self.traces
            .iter()
            .flat_map(|trace| &trace.changes)
            .map(|change| MeshRepairDomainChange { change })
    }
    /// Complete repair only; generic sampling never supplies a whole-domain proof.
    pub fn complete_bound(&self) -> Result<f64, &str> {
        if let Some(reason) = self.failure {
            return Err(reason);
        }
        if self.traces.len() != self.input.faces.len() || self.traces.is_empty() {
            return Err("missing mesh repair face coverage");
        }
        for trace in &self.traces {
            if let Some(reason) = trace.failure {
                return Err(reason);
            }
        }
        Err("generic source/trim and finite-piece movement coverage is unsupported")
    }
}

/// Existing mesher plus immutable captures; options/chart metadata are not borrowed.
pub fn robust_cshell_triangulation_with_repair_report<'a, C, S>(
    shell: &'a CompressedShell<Point3, C, S>,
    options: TessellationOptions,
    closed_axes: &[(usize, usize, (f64, f64), Vec<usize>)],
) -> ShellRepairMesh<'a, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
{
    let capture = Capture::new(shell.faces.len());
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    let (mesh, fallback) = triangulation::cshell_tessellation_with_closed_axis_metadata_and_repair(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
        closed_axes,
        false,
        options.preview_fallback,
        Some(&capture),
    );
    let failure = capture.failure;
    let traces = capture
        .faces
        .into_iter()
        .map(|trace| trace.into_inner().unwrap())
        .collect();
    ShellRepairMesh {
        input: shell,
        mesh,
        fallback,
        traces,
        failure,
    }
}
