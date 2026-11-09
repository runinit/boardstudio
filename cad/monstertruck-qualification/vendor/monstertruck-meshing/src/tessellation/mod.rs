use crate::*;
use monstertruck_topology::{compress::*, *};
mod repair;
pub use repair::{
    MeshRepairDomainChange, MeshRepairOperation, ShellRepairMesh,
    robust_cshell_triangulation_with_repair_report,
};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use spade::{iterators::*, *};

/// Tessellation output primitive preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TessellationPrimitiveMode {
    /// Keep triangle output.
    #[default]
    Triangles,
    /// Convert triangles to quads when possible.
    PreferQuads,
    /// Force all triangles into quads.
    AllQuads,
    /// Prefer UV-grid quads and keep non-quad elements near trims.
    IsoQuads,
}

/// Tessellation primitive generation options.
#[derive(Clone, Copy, Debug)]
pub struct TessellationPrimitiveOptions {
    /// Primitive generation mode.
    pub mode: TessellationPrimitiveMode,
    /// Coplanarity tolerance used for triangle pairing.
    pub plane_tolerance: f64,
    /// Shape score tolerance used for triangle pairing.
    pub score_tolerance: f64,
    /// Maximum normal blend angle in radians.
    pub normal_blend_angle: f64,
    /// Minimum quad area accepted by the `AllQuads` fallback.
    pub minimum_area: f64,
    /// Maximum corner angle accepted by the `AllQuads` fallback.
    pub maximum_corner_angle: f64,
}

impl Default for TessellationPrimitiveOptions {
    fn default() -> Self {
        Self {
            mode: TessellationPrimitiveMode::Triangles,
            plane_tolerance: 0.01,
            score_tolerance: 1.0,
            normal_blend_angle: std::f64::consts::PI / 4.0,
            minimum_area: TOLERANCE * TOLERANCE,
            maximum_corner_angle: 175.0 * std::f64::consts::PI / 180.0,
        }
    }
}

/// Options for tessellation.
#[derive(Clone, Copy, Debug)]
pub struct TessellationOptions {
    /// Geometric tolerance for curve and surface approximation.
    pub tolerance: f64,
    /// Optional sampled topology-edge angular bound in radians; `None` keeps linear-only sampling.
    /// A value must be finite and positive. Invalid values refuse every lenient face;
    /// an empty lenient shell has no failure channel. Strict meshing returns a typed error.
    /// Finite curve samples do not certify continuous error or bound surface-normal deviation.
    pub edge_angular_tolerance: Option<f64>,
    /// Maximum number of Newton iterations per parameter search.
    pub search_trials: usize,
    /// Primitive generation policy.
    pub primitive: TessellationPrimitiveOptions,
    /// Preview policy for the robust compressed-shell route: a face that would otherwise
    /// come out without triangles is meshed from its shared boundary samples, with interior
    /// vertices on the surface within `tolerance`. Boundary vertices stay the shared samples
    /// bit for bit. Off by default; such faces are reported by
    /// [`robust_cshell_triangulation_with_closed_axis_metadata_report`].
    pub preview_fallback: bool,
}

impl Default for TessellationOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.01,
            edge_angular_tolerance: None,
            search_trials: 100,
            primitive: TessellationPrimitiveOptions::default(),
            preview_fallback: false,
        }
    }
}

fn invalid_edge_angular_tolerance(angle: Option<f64>) -> bool {
    angle.is_some_and(|angle| !angle.is_finite() || angle <= 0.0)
}

#[cfg(not(target_arch = "wasm32"))]
mod parallelizable {
    /// Parallelizable by `rayon`.
    pub trait Parallelizable: Send + Sync {}
    impl<T: Send + Sync> Parallelizable for T {}
}

#[cfg(target_arch = "wasm32")]
mod parallelizable {
    /// No parallelization in the case of wasm.
    pub trait Parallelizable {}
    impl<T> Parallelizable for T {}
}

pub use parallelizable::*;

/// Gathered the traits used in tessellation.
pub trait PolylineableCurve:
    ParametricCurve3D + BoundedCurve + ParameterDivision1D<Point = Point3> + Parallelizable
{
}
impl<C: ParametricCurve3D + BoundedCurve + ParameterDivision1D<Point = Point3> + Parallelizable>
    PolylineableCurve for C
{
}

/// It can be meshed, but not necessarily trimmed.
pub trait PreMeshableSurface: ParametricSurface3D + ParameterDivision2D + Parallelizable {}
impl<S: ParametricSurface3D + ParameterDivision2D + Parallelizable> PreMeshableSurface for S {}

/// The generated mesh can be trimmed only if the boundary curves ride strictly on a surface.
pub trait MeshableSurface:
    PreMeshableSurface + SearchParameter<SurfaceParameter, Point = Point3>
{
}
impl<S: PreMeshableSurface + SearchParameter<SurfaceParameter, Point = Point3>> MeshableSurface
    for S
{
}

/// The generated mesh can be trimmed if the boundary curves does not ride strictly on a surface.
pub trait RobustMeshableSurface:
    MeshableSurface + SearchNearestParameter<SurfaceParameter, Point = Point3>
{
}
impl<S: MeshableSurface + SearchNearestParameter<SurfaceParameter, Point = Point3>>
    RobustMeshableSurface for S
{
}

type PolylineCurve = monstertruck_mesh::PolylineCurve<Point3>;

/// Options for optional isoparametric curve output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoparametricCurveOptions {
    /// Number of iso curves generated in each parameter direction.
    pub samples_per_direction: usize,
    /// Number of linear segments generated per iso curve before trim clipping.
    pub segments_per_curve: usize,
}

impl Default for IsoparametricCurveOptions {
    fn default() -> Self {
        Self {
            samples_per_direction: 4,
            segments_per_curve: 24,
        }
    }
}

/// Tessellation result with optional per-face diagnostic curve output.
#[derive(Clone, Debug)]
pub struct CompressedShellTessellation {
    /// Tessellated shell geometry.
    pub shell: CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>,
    /// Isoparametric polylines grouped by face.
    pub face_isoparams: Vec<Vec<Vec<Point3>>>,
}

/// Trait for converting tessellated shape into polygon.
pub trait MeshedShape {
    /// Converts tessellated shape into polygon.
    fn to_polygon(&self) -> PolygonMesh;
}

pub use triangulation::ExactTrimBoundary2D;
pub use triangulation::{FaceDropReason, face_drop_count, reset_face_drop_count};

/// Private typed refusal produced by the bounded per-face metric refinement.
/// It is carried through the internal shell result so strict callers can report
/// the exact reason while the established lenient APIs continue to emit `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetricRefinementError {
    InvalidMetric,
    InvalidTopology,
    NonConvergent,
}

impl MetricRefinementError {
    const fn tag(self) -> &'static str {
        match self {
            Self::InvalidMetric => "invalid-metric",
            Self::InvalidTopology => "invalid-topology",
            Self::NonConvergent => "non-convergent",
        }
    }

    const fn face_drop_reason(self) -> FaceDropReason {
        match self {
            Self::InvalidMetric => FaceDropReason::InvalidMetric,
            Self::InvalidTopology => FaceDropReason::InvalidTopology,
            Self::NonConvergent => FaceDropReason::NonConvergent,
        }
    }
}

/// A typed tessellation refusal, produced ONLY by the opt-in strict meshing path
/// ([`shell_to_polygon_strict`]).
///
/// The default `triangulation` / [`MeshedShape::to_polygon`] path never yields
/// this: it keeps rendering lenient by dropping a face that fails to tessellate
/// (emitting no mesh for it) and emits diagnostics rather than a hard failure.
/// That drop is dangerous only for a caller that then
/// *trusts* the mesh for a quantity the missing face changes -- above all the
/// boolean kernel, whose volume-conservation guard reads a divergence-theorem
/// volume off the mesh (spec 006 corner-100: a revolve-pole cap dropped to
/// `None`, the volume read flat, and a wrong boolean result was accepted). Such
/// callers opt in to the strict path and get this refusal instead of a
/// quietly-wrong number -- the tessellator-side analogue of the kernel's
/// typed-refusal doctrine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TessellationError {
    /// A face produced no usable mesh (unbounded domain, boundary projection
    /// failure, or typed metric-refinement refusal) and would be dropped from
    /// the lenient result, understating the meshed volume and breaking closure.
    /// Carries the face
    /// index within its shell, the surface class ([`std::any::type_name`] of the
    /// face's surface type), and the [`FaceDropReason`].
    FaceDropped {
        /// Index of the dropped face within its shell (`-1` if unknown).
        face: i64,
        /// The dropped face's surface class (its surface type's `type_name`).
        surface: &'static str,
        /// Why the face produced no usable mesh.
        reason: FaceDropReason,
    },
}

fn strict_metric_failure(
    face: usize,
    surface: &'static str,
    reason: MetricRefinementError,
) -> TessellationError {
    TessellationError::FaceDropped {
        face: face as i64,
        surface,
        reason: reason.face_drop_reason(),
    }
}

impl std::fmt::Display for TessellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FaceDropped {
                face,
                surface,
                reason,
            } => write!(
                f,
                "tessellation dropped face {face} ({reason}): surface={surface} -- the meshed \
                 shell is missing this face and its volume is understated",
            ),
        }
    }
}

impl std::error::Error for TessellationError {}

/// Meshes `shell` at `tolerance` and merges every face into one [`PolygonMesh`],
/// but REFUSES with a typed [`TessellationError::FaceDropped`] on the first face
/// that would silently drop -- the opt-in, correctness-critical counterpart to
/// `shell.triangulation(tolerance).to_polygon()`.
///
/// It is deliberately equivalent to `triangulation(tolerance).to_polygon()` on
/// any shell whose faces all mesh: the same faces are visited in the same order
/// and merged with the same orientation handling, so the returned polygon (and
/// therefore its volume) is byte-identical -- a caller can swap it in and see an
/// identical result on a clean shell, and a typed refusal *only* when a face
/// would have vanished. The single behavioural difference is a face whose mesh
/// is `None`.
///
/// Escalation covers `None` faces and typed refinement refusals. Empty meshes
/// are still allowed for compatibility: an empty tessellation does not prove
/// the source face has zero area. Callers requiring complete geometry or a
/// trusted volume must also verify source-face coverage.
pub fn shell_to_polygon_strict<C: PolylineableCurve, S: MeshableSurface>(
    shell: &Shell<Point3, C, S>,
    tolerance: f64,
) -> std::result::Result<PolygonMesh, TessellationError> {
    shell_to_polygon_strict_with(
        shell,
        TessellationOptions {
            tolerance,
            ..Default::default()
        },
    )
}

/// Strict meshing with explicit edge sampling and primitive options.
/// Angular sampling failures refuse the shell before any face is merged.
pub fn shell_to_polygon_strict_with<C: PolylineableCurve, S: MeshableSurface>(
    shell: &Shell<Point3, C, S>,
    options: TessellationOptions,
) -> std::result::Result<PolygonMesh, TessellationError> {
    let surface = std::any::type_name::<S>();
    if invalid_edge_angular_tolerance(options.edge_angular_tolerance) {
        return Err(TessellationError::FaceDropped {
            face: -1,
            surface,
            reason: FaceDropReason::InvalidAngularOptions,
        });
    }
    let tolerance = options.tolerance;
    nonpositive_tolerance!(tolerance);
    let sp = triangulation::search_parameter_sp::<S>(options.search_trials);
    #[cfg(not(target_arch = "wasm32"))]
    let result = triangulation::shell_tessellation_with_failures(
        shell,
        tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    #[cfg(target_arch = "wasm32")]
    let result = triangulation::shell_tessellation_single_thread_with_failures(
        shell,
        tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    if let Some((index, reason)) = result.edge_failures.iter().min_by_key(|(index, _)| index) {
        return Err(TessellationError::FaceDropped {
            face: *index as i64,
            surface,
            reason: *reason,
        });
    }
    let meshed = result.shell;
    let metric_failures = result.metric_failures;
    if let Some((index, reason)) = metric_failures.iter().min_by_key(|(index, _)| index) {
        return Err(strict_metric_failure(*index, surface, *reason));
    }
    let mut polygon = PolygonMesh::default();
    for (index, face) in meshed.face_iter().enumerate() {
        match face.surface() {
            // `None` == a real face was lost. Reclassify with the shared
            // source-of-truth (untrimmed => unbounded domain, trimmed =>
            // boundary-projection failure) and refuse typed rather than trust a
            // volume mesh with a hole in it.
            None => {
                let is_untrimmed = face
                    .absolute_boundaries()
                    .iter()
                    .all(|wire| wire.is_empty());
                let reason = triangulation::classify_face_drop(None, is_untrimmed)
                    .expect("a `None` face is always a drop");
                return Err(TessellationError::FaceDropped {
                    face: index as i64,
                    surface,
                    reason,
                });
            }
            // A produced mesh (possibly empty). Merge exactly as `to_polygon`
            // does -- an empty mesh merges as a no-op, contributing no volume.
            Some(mut poly) => {
                if !face.orientation() {
                    poly.invert();
                }
                polygon.merge(poly);
            }
        }
    }
    Ok(polygon)
}

impl MeshedShape for Shell<Point3, PolylineCurve, PolygonMesh> {
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.face_iter().for_each(|face| {
            polygon.merge(face.oriented_surface());
        });
        polygon
    }
}

impl MeshedShape for Shell<Point3, PolylineCurve, Option<PolygonMesh>> {
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.face_iter().for_each(|face| {
            if let Some(mut poly) = face.surface() {
                if !face.orientation() {
                    poly.invert();
                }
                polygon.merge(poly);
            }
        });
        polygon
    }
}

impl<P, C, S> MeshedShape for Solid<P, C, S>
where
    Shell<P, C, S>: MeshedShape,
{
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.boundaries().iter().for_each(|shell| {
            polygon.merge(shell.to_polygon());
        });
        polygon
    }
}

impl MeshedShape for CompressedShell<Point3, PolylineCurve, PolygonMesh> {
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.faces.iter().for_each(|face| match face.orientation {
            true => polygon.merge(face.surface.clone()),
            false => polygon.merge(face.surface.inverse()),
        });
        polygon
    }
}

impl MeshedShape for CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>> {
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.faces.iter().for_each(|face| {
            if let Some(surface) = &face.surface {
                match face.orientation {
                    true => polygon.merge(surface.clone()),
                    false => polygon.merge(surface.inverse()),
                }
            }
        });
        polygon
    }
}

impl<P, C, S> MeshedShape for CompressedSolid<P, C, S>
where
    CompressedShell<P, C, S>: MeshedShape,
{
    fn to_polygon(&self) -> PolygonMesh {
        let mut polygon = PolygonMesh::default();
        self.boundaries.iter().for_each(|shell| {
            polygon.merge(shell.to_polygon());
        });
        polygon
    }
}

/// Trait for tessellating `Shell` and `Solid`.
pub trait MeshableShape {
    /// Shape whose edges are made polylines and faces polygon surface.
    type MeshedShape: MeshedShape;
    /// Tessellates shapes. The division of curves and surfaces are by `ParameterDivision1D` and `ParameterDivision2D`,
    /// and the constrained Delauney triangulation is based on the crate [`spade`](https://crates.io/crates/spade).
    ///
    /// # Panics
    ///
    /// `tolerance` must be greater than or equal to `TOLERANCE`.
    ///
    /// # Remarks
    ///
    /// - The tessellated mesh is not necessarily closed even if `self` is `Solid`.
    ///   If you want to get closed mesh, use [`OptimizingFilter::put_together_same_attrs`].
    /// - This method requires that the curve ride strictly on a surface. If not, try [`RobustMeshableShape`].
    ///
    /// [`OptimizingFilter::put_together_same_attrs`]: crate::filters::OptimizingFilter::put_together_same_attrs
    ///
    /// # Examples
    /// ```
    /// use monstertruck_meshing::prelude::*;
    /// use monstertruck_modeling::*;
    /// use monstertruck_topology::shell::ShellCondition;
    ///
    /// // modeling a unit cube
    /// let v = builder::vertex(Point3::origin());
    /// let e = builder::extrude(&v, Vector3::unit_x());
    /// let f = builder::extrude(&e, Vector3::unit_y());
    /// let cube: Solid = builder::extrude(&f, Vector3::unit_z());
    ///
    /// // cube is Solid, however, the tessellated mesh is not closed.
    /// let mut mesh = cube.triangulation(0.01).to_polygon();
    /// assert_ne!(mesh.shell_condition(), ShellCondition::Closed);
    ///
    /// // use optimization filters!
    /// mesh.put_together_same_attrs(TOLERANCE);
    /// assert_eq!(mesh.shell_condition(), ShellCondition::Closed);
    /// ```
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape;
}

/// Trait for tessellating `Shell` and `Solid` in `monstertruck-modeling`.
pub trait RobustMeshableShape {
    /// Shape whose edges are made polylines and faces polygon surface.
    type MeshedShape: MeshedShape;
    /// Tessellates shapes. The division of curves and surfaces are by `ParameterDivision1D` and `ParameterDivision2D`,
    /// and the constrained Delauney triangulation is based on the crate [`spade`](https://crates.io/crates/spade).
    ///
    /// # Panics
    ///
    /// `tolerance` must be greater than or equal to `TOLERANCE`.
    ///
    /// # Remarks
    ///
    /// Since polyline vertices are projected onto the surface, processing speed is often slower than with [`MeshableShape::triangulation`].
    /// It also does not close the mesh of a solid even if one uses [`OptimizingFilter::put_together_same_attrs`].
    ///
    /// [`OptimizingFilter::put_together_same_attrs`]: crate::filters::OptimizingFilter::put_together_same_attrs
    ///
    /// # Examples
    /// ```
    /// use monstertruck_meshing::prelude::*;
    /// use monstertruck_modeling::*;
    /// use monstertruck_topology::shell::ShellCondition;
    ///
    /// // manual modeling an open half cylinder
    ///
    /// // points
    /// let p = [
    ///     Point3::new(1.0, 0.0, 0.0),
    ///     Point3::new(-1.0, 0.0, 0.0),
    ///     Point3::new(1.0, 0.0, 1.0),
    ///     Point3::new(-1.0, 0.0, 1.0),
    /// ];
    /// // vertices
    /// let v = Vertex::from_points(&p);
    /// // Curves that do not ride on a cylinder
    /// let bsp0: Curve = BsplineCurve::new(
    ///     KnotVector::bezier_knot(3),
    ///     vec![
    ///         p[0],
    ///         Point3::new(1.0, 4.0 / 3.0, 0.0),
    ///         Point3::new(-1.0, 4.0 / 3.0, 0.0),
    ///         p[1],
    ///     ],
    /// )
    /// .into();
    /// let bsp1: Curve = BsplineCurve::new(
    ///     KnotVector::bezier_knot(3),
    ///     vec![
    ///         p[3],
    ///         Point3::new(-1.0, 4.0 / 3.0, 1.0),
    ///         Point3::new(1.0, 4.0 / 3.0, 1.0),
    ///         p[2],
    ///     ],
    /// )
    /// .into();
    /// // wire
    /// let w: Wire = vec![
    ///     builder::line(&v[2], &v[0]),
    ///     Edge::new(&v[0], &v[1], bsp0),
    ///     builder::line(&v[1], &v[3]),
    ///     Edge::new(&v[3], &v[2], bsp1),
    /// ]
    /// .into();
    /// // revoluted curve
    /// let surface_raw = RevolutionSurface::by_revolution(
    ///     Curve::Line(Line(p[2], p[0])),
    ///     Point3::origin(),
    ///     Vector3::unit_z(),
    /// );
    /// let surface: Surface = Processor::new(surface_raw).into();
    /// // shell
    /// let shell: Shell = vec![Face::new(vec![w], surface)].into();
    ///
    /// // Simple triangulation fails since some edges do not ride on a cylinder
    /// let poly_shell = shell.triangulation(0.01);
    /// assert!(poly_shell[0].surface().is_none());
    ///
    /// // Robust triangulation!
    /// let poly_shell = shell.robust_triangulation(0.01);
    /// let poly = poly_shell[0].surface().unwrap();
    /// assert!(!poly.positions().is_empty());
    /// ```
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape;
}

/// Tessellates a [`Shell`] with a [`TessellationOptions`].
pub fn triangulation_with<C: PolylineableCurve, S: MeshableSurface>(
    shell: &Shell<Point3, C, S>,
    options: TessellationOptions,
) -> Shell<Point3, PolylineCurve, Option<PolygonMesh>> {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_parameter_sp::<S>(options.search_trials);
    #[cfg(not(target_arch = "wasm32"))]
    let res = triangulation::shell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    #[cfg(target_arch = "wasm32")]
    let res = triangulation::shell_tessellation_single_thread(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    res
}

/// Tessellates a [`Shell`] with robust parameter search and a [`TessellationOptions`].
pub fn robust_triangulation_with<C: PolylineableCurve, S: RobustMeshableSurface>(
    shell: &Shell<Point3, C, S>,
    options: TessellationOptions,
) -> Shell<Point3, PolylineCurve, Option<PolygonMesh>> {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    #[cfg(not(target_arch = "wasm32"))]
    let res = triangulation::shell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    #[cfg(target_arch = "wasm32")]
    let res = triangulation::shell_tessellation_single_thread(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    );
    res
}

/// Tessellates a [`CompressedShell`] with a [`TessellationOptions`].
pub fn cshell_triangulation_with<
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: MeshableSurface,
>(
    shell: &CompressedShell<Point3, C, S>,
    options: TessellationOptions,
) -> CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>> {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_parameter_sp::<S>(options.search_trials);
    triangulation::cshell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    )
}

/// Tessellates a [`CompressedShell`] with robust parameter search and a [`TessellationOptions`].
pub fn robust_cshell_triangulation_with<
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
>(
    shell: &CompressedShell<Point3, C, S>,
    options: TessellationOptions,
) -> CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>> {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    triangulation::cshell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    )
}

/// Scratch-only face-local seam route requiring source-certified closed-axis metadata.
/// The caller must establish equality of the actual surface's entire finite boundary
/// curves (for example, clamped homogeneous B-spline coefficient equality). A STEP
/// closure flag or finite samples alone do not satisfy this precondition. The axis
/// span is a finite chart boundary, never an evaluator period.
#[doc(hidden)]
pub fn robust_cshell_triangulation_with_closed_axis_metadata<
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
>(
    shell: &CompressedShell<Point3, C, S>,
    options: TessellationOptions,
    closed_axes: &[(usize, usize, (f64, f64), Vec<usize>)],
) -> CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>> {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    triangulation::cshell_tessellation_with_closed_axis_metadata(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
        closed_axes,
        false,
    )
}

/// As [`robust_cshell_triangulation_with_closed_axis_metadata`], also returning the indices of
/// faces meshed by the `options.preview_fallback` route.
#[doc(hidden)]
pub fn robust_cshell_triangulation_with_closed_axis_metadata_report<
    C: PolylineableCurve + ParameterBoundary2D<S>,
    S: RobustMeshableSurface,
>(
    shell: &CompressedShell<Point3, C, S>,
    options: TessellationOptions,
    closed_axes: &[(usize, usize, (f64, f64), Vec<usize>)],
) -> (
    CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>,
    Vec<usize>,
) {
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    triangulation::cshell_tessellation_with_closed_axis_metadata_report(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
        closed_axes,
        false,
        options.preview_fallback,
    )
}

/// Tessellates a [`CompressedTrimmedShell`] with a [`TessellationOptions`].
pub fn trimmed_cshell_triangulation_with<
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
>(
    shell: &CompressedTrimmedShell<Point3, C, S, T>,
    options: TessellationOptions,
) -> CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>
where
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_parameter_sp::<S>(options.search_trials);
    triangulation::trimmed_cshell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    )
}

/// Tessellates a [`CompressedTrimmedShell`] and emits trim-aware isoparametric curves.
pub fn compressed_trimmed_shell_triangulation_with_isoparams<
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
>(
    shell: &CompressedTrimmedShell<Point3, C, S, T>,
    options: TessellationOptions,
    isoparametric_options: IsoparametricCurveOptions,
) -> CompressedShellTessellation
where
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_parameter_sp::<S>(options.search_trials);
    triangulation::compressed_trimmed_shell_tessellation_with_isoparams(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        Some(isoparametric_options),
        options.edge_angular_tolerance,
    )
}

/// Tessellates a [`CompressedTrimmedShell`] with robust parameter search and a [`TessellationOptions`].
pub fn robust_trimmed_cshell_triangulation_with<
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
>(
    shell: &CompressedTrimmedShell<Point3, C, S, T>,
    options: TessellationOptions,
) -> CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>
where
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    triangulation::trimmed_cshell_tessellation(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        options.edge_angular_tolerance,
    )
}

/// Robustly tessellates a [`CompressedTrimmedShell`] and emits trim-aware isoparametric curves.
pub fn robust_compressed_trimmed_shell_triangulation_with_isoparams<
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
>(
    shell: &CompressedTrimmedShell<Point3, C, S, T>,
    options: TessellationOptions,
    isoparametric_options: IsoparametricCurveOptions,
) -> CompressedShellTessellation
where
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    nonpositive_tolerance!(options.tolerance);
    let sp = triangulation::search_nearest_parameter_sp::<S>(options.search_trials);
    triangulation::compressed_trimmed_shell_tessellation_with_isoparams(
        shell,
        options.tolerance,
        sp,
        options.primitive,
        Some(isoparametric_options),
        options.edge_angular_tolerance,
    )
}

impl<C: PolylineableCurve, S: MeshableSurface> MeshableShape for Shell<Point3, C, S> {
    type MeshedShape = Shell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C: PolylineableCurve, S: RobustMeshableSurface> RobustMeshableShape for Shell<Point3, C, S> {
    type MeshedShape = Shell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        robust_triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C: PolylineableCurve, S: MeshableSurface> MeshableShape for Solid<Point3, C, S> {
    type MeshedShape = Solid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries()
            .par_iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries()
            .iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        Solid::new_unchecked(boundaries)
    }
}

impl<C: PolylineableCurve, S: RobustMeshableSurface> RobustMeshableShape for Solid<Point3, C, S> {
    type MeshedShape = Solid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries()
            .par_iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries()
            .iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        Solid::new_unchecked(boundaries)
    }
}

impl<C: PolylineableCurve + ParameterBoundary2D<S>, S: MeshableSurface> MeshableShape
    for CompressedShell<Point3, C, S>
{
    type MeshedShape = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        cshell_triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C: PolylineableCurve + ParameterBoundary2D<S>, S: RobustMeshableSurface> RobustMeshableShape
    for CompressedShell<Point3, C, S>
{
    type MeshedShape = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        robust_cshell_triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C, S, T> MeshableShape for CompressedTrimmedShell<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    type MeshedShape = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        trimmed_cshell_triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C, S, T> RobustMeshableShape for CompressedTrimmedShell<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    type MeshedShape = CompressedShell<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        robust_trimmed_cshell_triangulation_with(
            self,
            TessellationOptions {
                tolerance,
                ..Default::default()
            },
        )
    }
}

impl<C: PolylineableCurve + ParameterBoundary2D<S>, S: MeshableSurface> MeshableShape
    for CompressedSolid<Point3, C, S>
{
    type MeshedShape = CompressedSolid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries
            .par_iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries
            .iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        CompressedSolid {
            boundaries,
            id_allocator: None,
            attributes: None,
        }
    }
}

impl<C: PolylineableCurve + ParameterBoundary2D<S>, S: RobustMeshableSurface> RobustMeshableShape
    for CompressedSolid<Point3, C, S>
{
    type MeshedShape = CompressedSolid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries
            .par_iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries
            .iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        CompressedSolid {
            boundaries,
            id_allocator: None,
            attributes: None,
        }
    }
}

impl<C, S, T> MeshableShape for CompressedTrimmedSolid<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: MeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    type MeshedShape = CompressedSolid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries
            .par_iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries
            .iter()
            .map(|shell| shell.triangulation(tolerance))
            .collect::<Vec<_>>();
        CompressedSolid {
            boundaries,
            id_allocator: None,
            attributes: None,
        }
    }
}

impl<C, S, T> RobustMeshableShape for CompressedTrimmedSolid<Point3, C, S, T>
where
    C: PolylineableCurve + ParameterBoundary2D<S> + ExactParameterBoundary2D<S>,
    S: RobustMeshableSurface,
    T: ExactTrimBoundary2D + Parallelizable,
    <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
{
    type MeshedShape = CompressedSolid<Point3, PolylineCurve, Option<PolygonMesh>>;
    fn robust_triangulation(&self, tolerance: f64) -> Self::MeshedShape {
        #[cfg(not(target_arch = "wasm32"))]
        let boundaries = self
            .boundaries
            .par_iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        #[cfg(target_arch = "wasm32")]
        let boundaries = self
            .boundaries
            .iter()
            .map(|shell| shell.robust_triangulation(tolerance))
            .collect::<Vec<_>>();
        CompressedSolid {
            boundaries,
            id_allocator: None,
            attributes: None,
        }
    }
}

mod triangulation;

#[cfg(test)]
mod metric_error_tests {
    use super::*;

    #[test]
    fn strict_metric_refusals_preserve_face_and_reason() {
        for (failure, expected) in [
            (
                MetricRefinementError::InvalidMetric,
                FaceDropReason::InvalidMetric,
            ),
            (
                MetricRefinementError::InvalidTopology,
                FaceDropReason::InvalidTopology,
            ),
            (
                MetricRefinementError::NonConvergent,
                FaceDropReason::NonConvergent,
            ),
        ] {
            assert_eq!(
                strict_metric_failure(7, "test-surface", failure),
                TessellationError::FaceDropped {
                    face: 7,
                    surface: "test-surface",
                    reason: expected,
                },
            );
        }
    }
}
