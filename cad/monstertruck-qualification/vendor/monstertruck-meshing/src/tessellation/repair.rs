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
pub(super) type ClosedAxis = (usize, usize, (f64, f64), Vec<usize>);
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
    #[cfg(test)]
    pub(super) fn for_test(faces: usize) -> Self {
        Self::new(faces)
    }

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
        let failure = failure.or_else(|| {
            (traces.capacity() > faces).then_some("mesh repair trace capacity unavailable")
        });
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
    fn record(&self) -> bool {
        self.operations
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(1).filter(|n| *n <= OPERATION_CAP)
            })
            .is_ok()
    }
    fn into_traces(self) -> (Vec<FaceTrace>, Option<&'static str>) {
        let mut traces = Vec::new();
        // The destination coexists with every face lock and its retained records.
        let bytes = self.faces.len().checked_mul(size_of::<FaceTrace>());
        let failure = self.failure.or_else(|| {
            if bytes.is_none_or(|bytes| !self.charge(bytes)) {
                Some("mesh repair final trace storage resource cap")
            } else if traces.try_reserve_exact(self.faces.len()).is_err() {
                Some("mesh repair final trace allocation failed")
            } else if traces.capacity() > self.faces.len() {
                Some("mesh repair final trace capacity unavailable")
            } else {
                None
            }
        });
        if failure.is_none() {
            for trace in self.faces {
                traces.push(trace.into_inner().unwrap());
            }
        }
        (traces, failure)
    }
}
impl<'capture> FaceCapture<'capture> {
    pub(super) fn reserve_auxiliary(self, bytes: usize) -> bool {
        if self.capture.charge(bytes) {
            true
        } else {
            self.fail("mesh repair auxiliary storage resource cap");
            false
        }
    }
    pub(super) fn pending(self, count: usize) -> Option<Vec<Option<Pending<'capture>>>> {
        let Some(bytes) = count.checked_mul(size_of::<Option<Pending<'capture>>>()) else {
            self.fail("mesh repair pending storage overflow");
            return None;
        };
        if !self.reserve_auxiliary(bytes) {
            return None;
        }
        let mut pending = Vec::new();
        if pending.try_reserve_exact(count).is_err() {
            self.fail("mesh repair pending allocation failed");
            return None;
        }
        if pending.capacity() > count {
            self.fail("mesh repair pending capacity unavailable");
            return None;
        }
        Some(pending)
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
        // Two slots per append bound the old/new Vec buffers during exact growth.
        if !self.capture.record() || !self.capture.charge(2 * size_of::<Operation>()) {
            self.fail("mesh repair operation resource cap");
            return;
        }
        let mut trace = self.capture.faces[self.face].lock().unwrap();
        let capacity = trace.operations.capacity().max(trace.operations.len() + 1);
        if trace.operations.try_reserve_exact(1).is_err() {
            trace.failure = Some("mesh repair operation allocation failed");
            return;
        }
        if trace.operations.capacity() > capacity {
            trace.failure = Some("mesh repair operation capacity unavailable");
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
        if !self.capture.record() {
            self.fail("mesh repair control storage resource cap");
            return None;
        }
        let before_uv = self.controls(uv)?;
        let before_xyz = self.controls(xyz)?;
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
    fn controls<P>(self, controls: impl Iterator<Item = P>) -> Option<Vec<P>> {
        let Some(count) = controls.size_hint().1 else {
            self.fail("mesh repair control capacity unavailable");
            return None;
        };
        let Some(bytes) = count.checked_mul(size_of::<P>()) else {
            self.fail("mesh repair control storage overflow");
            return None;
        };
        if !self.capture.charge(bytes) {
            self.fail("mesh repair control storage resource cap");
            return None;
        }
        let mut result = Vec::new();
        if result.try_reserve_exact(count).is_err() {
            self.fail("mesh repair control allocation failed");
            return None;
        }
        if result.capacity() > count {
            self.fail("mesh repair control capacity unavailable");
            return None;
        }
        for point in controls {
            // Iterator hints are advisory; never let extend/push grow this buffer.
            if result.len() == count {
                self.fail("mesh repair control count exceeded capacity");
                return None;
            }
            result.push(point);
        }
        Some(result)
    }
}
impl Pending<'_> {
    pub(super) fn finish(
        mut self,
        uv: impl Iterator<Item = Point2>,
        xyz: impl Iterator<Item = Point3>,
    ) {
        let Some(after_uv) = self.owner.controls(uv) else {
            return;
        };
        let Some(after_xyz) = self.owner.controls(xyz) else {
            return;
        };
        self.change.after_uv = after_uv;
        self.change.after_xyz = after_xyz;
        if !self.owner.capture.charge(2 * size_of::<Change>()) {
            self.owner.fail("mesh repair control storage resource cap");
            return;
        }
        let mut trace = self.owner.capture.faces[self.owner.face].lock().unwrap();
        let capacity = trace.changes.capacity().max(trace.changes.len() + 1);
        if trace.changes.try_reserve_exact(1).is_err() {
            trace.failure = Some("mesh repair change allocation failed");
            return;
        }
        if trace.changes.capacity() > capacity {
            trace.failure = Some("mesh repair change capacity unavailable");
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
#[expect(
    clippy::type_complexity,
    reason = "exact additive metadata signature approved in MOVEMENT_SEAM.md"
)]
pub fn robust_cshell_triangulation_with_repair_report<'a, C, S>(
    shell: &'a CompressedShell<Point3, C, S>,
    options: TessellationOptions,
    closed_axes: &[(usize, usize, (f64, f64), Vec<usize>)],
) -> ShellRepairMesh<'a, C, S>
where
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
{
    nonpositive_tolerance!(options.tolerance);
    let capture = Capture::new(shell.faces.len());
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    let (mesh, fallback) = triangulation::cshell_tessellation_with_closed_axis_metadata_and_repair(
        shell,
        options,
        sp,
        closed_axes,
        false,
        Some(&capture),
    );
    let (traces, failure) = capture.into_traces();
    ShellRepairMesh {
        input: shell,
        mesh,
        fallback,
        traces,
        failure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_geometry::prelude::{Line, Plane};
    use std::panic::catch_unwind;

    #[test]
    fn repair_control_capture_never_trusts_an_iterator_capacity_hint() {
        struct Extra(usize);
        impl Iterator for Extra {
            type Item = Point2;
            fn next(&mut self) -> Option<Point2> {
                self.0 = self.0.checked_sub(1)?;
                Some(Point2::origin())
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (0, Some(1))
            }
        }
        let capture = Capture::new(1);
        let face = capture.face(0).unwrap();
        assert!(
            face.before(Stage::Loop, Extra(2), std::iter::empty())
                .is_none()
        );
        assert!(capture.faces[0].lock().unwrap().failure.is_some());
        assert_eq!(face.mark(), (0, 0));
    }

    #[test]
    fn repair_record_counter_cannot_wrap_and_reopen_capture() {
        let capture = Capture::new(2);
        capture.operations.store(usize::MAX, Ordering::Relaxed);
        capture.face(0).unwrap().unsupported(Stage::Loop, "first");
        capture.face(1).unwrap().unsupported(Stage::Loop, "second");
        assert_eq!(capture.face(1).unwrap().mark(), (0, 0));
        assert!(
            capture
                .faces
                .iter()
                .all(|trace| trace.lock().unwrap().failure.is_some())
        );
    }

    #[test]
    fn repair_finalization_charges_destination_before_releasing_face_locks() {
        let capture = Capture::new(2);
        let bytes = capture.bytes.load(Ordering::Relaxed);
        assert_eq!(bytes, 2 * size_of::<Mutex<FaceTrace>>());
        let final_bytes = 2 * size_of::<FaceTrace>();
        capture
            .bytes
            .store(STORAGE_CAP - final_bytes + 1, Ordering::Relaxed);
        let (traces, failure) = capture.into_traces();
        assert!(traces.is_empty(), "refuse before destination allocation");
        assert_eq!(
            failure,
            Some("mesh repair final trace storage resource cap")
        );

        let capture = Capture::new(2);
        capture
            .face(0)
            .unwrap()
            .unsupported(Stage::SourceDomain, "original domain");
        let used = capture.bytes.load(Ordering::Relaxed);
        capture
            .bytes
            .store(STORAGE_CAP - final_bytes, Ordering::Relaxed);
        let (traces, failure) = capture.into_traces();
        assert!(failure.is_none(), "the exact existing cap fits");
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].operations.len(), 1);
        assert!(used > bytes);
    }

    #[test]
    fn repair_pending_and_rejected_controls_keep_their_coexistence_charge() {
        let capture = Capture::new(1);
        let face = capture.face(0).unwrap();
        let initial = capture.bytes.load(Ordering::Relaxed);
        let point = [Point2::origin()];
        let pending = face
            .before(Stage::Loop, point.into_iter(), std::iter::empty())
            .unwrap();
        let before = capture.bytes.load(Ordering::Relaxed);
        assert_eq!(before - initial, size_of::<Point2>());
        capture
            .bytes
            .store(STORAGE_CAP - size_of::<Point2>() + 1, Ordering::Relaxed);
        pending.finish(point.into_iter(), std::iter::empty());
        assert_eq!(face.mark(), (0, 0));
        assert!(capture.faces[0].lock().unwrap().failure.is_some());
        face.discard_since((0, 0));
        assert!(capture.faces[0].lock().unwrap().failure.is_some());
        assert_eq!(
            capture.bytes.load(Ordering::Relaxed),
            STORAGE_CAP - size_of::<Point2>() + 1
        );
    }

    #[test]
    fn repair_reuses_rollback_capacity_without_reallocating_or_erasing_failures() {
        let capture = Capture::new(1);
        let face = capture.face(0).unwrap();
        for _ in 0..4 {
            face.unsupported(Stage::Loop, "candidate");
        }
        let capacity = capture.faces[0].lock().unwrap().operations.capacity();
        let charged = capture.bytes.load(Ordering::Relaxed);
        face.discard_since((0, 0));
        face.unsupported(Stage::Branch, "accepted candidate");
        let trace = capture.faces[0].lock().unwrap();
        assert_eq!(trace.operations.capacity(), capacity);
        assert_eq!(trace.operations.len(), 1);
        assert!(trace.failure.is_none());
        assert!(capture.bytes.load(Ordering::Relaxed) >= charged);
    }

    #[test]
    fn repair_pending_vector_overflow_and_cap_refuse_before_reserve() {
        let capture = Capture::new(1);
        let face = capture.face(0).unwrap();
        assert!(face.pending(usize::MAX).is_none());
        let used = capture.bytes.load(Ordering::Relaxed);
        assert!(capture.faces[0].lock().unwrap().failure.is_some());
        face.discard_since((0, 0));
        assert_eq!(capture.bytes.load(Ordering::Relaxed), used);
        assert!(capture.faces[0].lock().unwrap().failure.is_some());

        let capture = Capture::new(1);
        let face = capture.face(0).unwrap();
        capture.bytes.store(STORAGE_CAP, Ordering::Relaxed);
        assert!(face.pending(1).is_none());
        assert!(capture.faces[0].lock().unwrap().failure.is_some());
    }

    #[test]
    fn repair_report_preserves_legacy_tolerance_precondition_on_empty_shell() {
        let shell: CompressedShell<Point3, Line<Point3>, Plane> = CompressedShell {
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        };
        for tolerance in [0.0, -0.01, f64::NAN, TOLERANCE.next_down(), TOLERANCE, 0.01] {
            let options = TessellationOptions {
                tolerance,
                ..Default::default()
            };
            let legacy = catch_unwind(|| {
                robust_cshell_triangulation_with_closed_axis_metadata_report(&shell, options, &[])
            });
            let reported = catch_unwind(|| {
                robust_cshell_triangulation_with_repair_report(&shell, options, &[])
            });
            assert_eq!(legacy.is_ok(), tolerance >= TOLERANCE);
            assert_eq!(
                reported.is_err(),
                legacy.is_err(),
                "same precondition for tolerance {tolerance:?}"
            );
            if let (Ok((mesh, fallback)), Ok(report)) = (legacy, reported) {
                assert_eq!(&mesh, report.mesh());
                assert_eq!(&fallback, report.fallback_faces());
                assert!(report.complete_bound().is_err());
            }
        }
    }

    #[test]
    fn repair_report_keeps_positive_tolerance_cube_output_identical_to_legacy() {
        use monstertruck_modeling::{Edge, Face, Solid, builder};
        let vertex = builder::vertex(Point3::origin());
        let edge: Edge = builder::extrude(&vertex, Vector3::unit_x());
        let face: Face = builder::extrude(&edge, Vector3::unit_y());
        let solid: Solid = builder::extrude(&face, Vector3::unit_z());
        let shell = solid.boundaries()[0].compress();
        let options = TessellationOptions::default();
        let (mesh, fallback) =
            robust_cshell_triangulation_with_closed_axis_metadata_report(&shell, options, &[]);
        let reported = robust_cshell_triangulation_with_repair_report(&shell, options, &[]);
        assert_eq!(&mesh, reported.mesh());
        assert_eq!(&fallback, reported.fallback_faces());
        assert!(
            reported
                .mesh()
                .faces
                .iter()
                .all(|face| face.surface.is_some())
        );
        assert!(std::ptr::eq(reported.input(), &shell));
    }

    #[test]
    fn repair_trace_rollback_keeps_failures_and_other_faces() {
        let capture = Capture::new(2);
        let first = capture.face(0).unwrap();
        let other = capture.face(1).unwrap();
        other.unsupported(Stage::SourceDomain, "other face");
        let mark = first.mark();
        first.unsupported(Stage::BoundaryRemap, "rejected candidate");
        assert!(!first.reserve_auxiliary(STORAGE_CAP + 1));
        first.discard_since(mark);
        assert_eq!(first.mark(), mark);
        assert_eq!(other.mark(), (1, 0));
        assert_eq!(
            capture.faces[0].lock().unwrap().failure,
            Some("mesh repair auxiliary storage resource cap")
        );
    }
}
