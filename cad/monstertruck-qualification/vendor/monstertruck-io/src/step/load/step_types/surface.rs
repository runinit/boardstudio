//! Surface entities: elementary surfaces, B-spline/NURBS surfaces and
//! swept surfaces.

use super::*;

fn finite_positive_control_net<
    P: monstertruck_core::cgmath_extend_traits::control_point::ControlPoint<f64>,
>(
    controls: &[Vec<P>],
    weight: impl Fn(&P) -> f64,
) -> bool {
    !controls.is_empty()
        && !controls[0].is_empty()
        && controls.iter().all(|row| row.len() == controls[0].len())
        && controls
            .iter()
            .flatten()
            .all(|p| (0..P::DIM).all(|axis| p[axis].is_finite()) && weight(p) > 0.0)
}

/// `surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum SurfaceAny {
    #[holder(use_place_holder)]
    ElementarySurface(Box<ElementarySurfaceAny>),
    #[holder(use_place_holder)]
    BsplineSurface(Box<BsplineSurfaceAny>),
    #[holder(use_place_holder)]
    SweptSurface(Box<SweptSurfaceAny>),
}

impl SurfaceAny {
    /// Source-declared closure flags for a B-spline surface, preserving STEP
    /// metadata that is otherwise lost when converting to the geometric type.
    /// `None` means this surface family does not carry these flags.
    pub fn declared_closed_axes(&self) -> Option<[bool; 2]> {
        let Self::BsplineSurface(surface) = self else {
            return None;
        };
        Some(match surface.as_ref() {
            BsplineSurfaceAny::NonRationalBsplineSurface(surface) => surface.closed_axes(),
            BsplineSurfaceAny::RationalBsplineSurface(surface) => {
                surface.non_rational_b_spline_surface.closed_axes()
            }
        })
    }
}

impl SurfaceAny {
    /// Certifies equality of a declared closed axis's finite boundary curves.
    /// This does not imply periodic evaluation or matching seam derivatives.
    /// Unsupported representations and nonidentical homogeneous rows are refused.
    pub fn certified_closed_axes_for(&self, converted: &Surface) -> Option<[bool; 2]> {
        let declared = self.declared_closed_axes()?;
        if Surface::try_from(self).ok()?.ne(converted) {
            return None;
        }
        Self::certified_boundary_axes_for(converted, declared)
    }

    pub(crate) fn finite_positive_spline_controls_for(converted: &Surface) -> bool {
        match converted {
            Surface::BsplineSurface(surface) => {
                finite_positive_control_net(surface.control_points(), |_| 1.0)
            }
            Surface::NurbsSurface(surface) => {
                finite_positive_control_net(surface.control_points(), |p| p.w)
            }
            _ => true,
        }
    }

    fn certified_boundary_axes_for(converted: &Surface, declared: [bool; 2]) -> Option<[bool; 2]> {
        let proved = converted.finite_boundary_equal_axes()?;
        Some([declared[0] && proved[0], declared[1] && proved[1]])
    }

    /// Certifies the actual uniformly scaled surface against its exact source
    /// conversion and the same uniform transform. Nonfinite results are refused.
    pub fn certified_closed_axes_after_uniform_scale(
        &self,
        converted_source: &Surface,
        converted_scaled: &Surface,
        factor: f64,
    ) -> Option<[bool; 2]> {
        if !factor.is_finite() || factor <= 0.0 {
            return None;
        }
        let declared = self.declared_closed_axes()?;
        if Surface::try_from(self).ok()?.ne(converted_source) {
            return None;
        }
        let mut expected = converted_source.clone();
        expected.transform_by(Matrix4::from_scale(factor));
        if expected.ne(converted_scaled) {
            return None;
        }
        Self::certified_boundary_axes_for(converted_scaled, declared)
    }
}

impl TryFrom<&SurfaceAny> for Surface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(x: &SurfaceAny) -> Result<Self, Self::Error> {
        use SurfaceAny::*;
        Ok(match x {
            ElementarySurface(x) => Self::ElementarySurface(x.as_ref().try_into()?),
            BsplineSurface(x) => x.as_ref().try_into()?,
            SweptSurface(x) => Self::SweepSurface(x.as_ref().try_into()?),
        })
    }
}

/// `elementary_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum ElementarySurfaceAny {
    #[holder(use_place_holder)]
    Plane(Box<Plane>),
    #[holder(use_place_holder)]
    SphericalSurface(Box<SphericalSurface>),
    #[holder(use_place_holder)]
    CylindricalSurface(Box<CylindricalSurface>),
    #[holder(use_place_holder)]
    ToroidalSurface(Box<ToroidalSurface>),
    /// The self-intersecting torus subtype. Present so the record deserializes
    /// into a NAMED class instead of `Table::dummy`; its geometry is refused with
    /// the class named (see [`refuse_degenerate_torus`]).
    #[holder(use_place_holder)]
    DegenerateToroidalSurface(Box<DegenerateToroidalSurface>),
    #[holder(use_place_holder)]
    ConicalSurface(Box<ConicalSurface>),
}

/// Fallible since spec 011 T1: two of the five elementary surfaces carry radii a
/// STEP file can put outside the representable domain, and the torus arm used to
/// hand them to a constructor that PANICS (`Torus::new`, on
/// `major_radius <= 0.0`). 207 corpus faces reached that panic and took 11 solids
/// -- 15,191 faces -- down with them. The refusal now happens here, typed.
impl TryFrom<&ElementarySurfaceAny> for ElementarySurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &ElementarySurfaceAny) -> Result<Self, Self::Error> {
        use ElementarySurfaceAny::*;
        Ok(match value {
            Plane(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::Plane(x.as_ref().into())
            }
            SphericalSurface(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::Sphere(x.as_ref().into())
            }
            CylindricalSurface(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::CylindricalSurface(x.as_ref().into())
            }
            ToroidalSurface(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::ToroidalSurface(x.as_ref().try_into()?)
            }
            DegenerateToroidalSurface(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::ToroidalSurface(x.as_ref().try_into()?)
            }
            ConicalSurface(x) => {
                super::placement::validate_axis2_placement3d(&x.position)?;
                Self::ConicalSurface(x.as_ref().into())
            }
        })
    }
}

/// `plane`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = plane)]
#[holder(generate_deserialize)]
pub struct Plane {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
}

impl From<&Plane> for geom::Plane {
    #[inline(always)]
    fn from(plane: &Plane) -> Self {
        // Retain axes separately: adding them to a large origin can erase tilt.
        Self::xy().transformed(Matrix4::from(&plane.position))
    }
}

/// `spherical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = spherical_surface)]
#[holder(generate_deserialize)]
pub struct SphericalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
}

impl From<&SphericalSurface> for step_geometry::SphericalSurface {
    #[inline(always)]
    fn from(ss: &SphericalSurface) -> Self {
        let mat = Matrix4::from(&ss.position);
        let sphere = Sphere(geom::Sphere::new(Point3::origin(), ss.radius));
        Processor::new(sphere).transformed(mat)
    }
}

/// `cylindrical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = cylindrical_surface)]
#[holder(generate_deserialize)]
pub struct CylindricalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
}

impl From<&CylindricalSurface> for step_geometry::CylindricalSurface {
    #[inline(always)]
    fn from(cs: &CylindricalSurface) -> Self {
        let mat = Matrix4::from(&cs.position);
        let x = mat[0].truncate();
        let z = mat[2].truncate();
        let center = Point3::from_homogeneous(mat[3]);
        let radius = cs.radius;
        let p = center + x * radius;
        let mut res = Processor::new(RevolutionSurface::by_revolution(Line(p, p + z), center, z));
        res.invert();
        res
    }
}

/// `toroidal_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = toroidal_surface)]
#[holder(generate_deserialize)]
pub struct ToroidalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    major_radius: f64,
    minor_radius: f64,
}

/// The DEGENERATE (self-intersecting) toroidal regime, refused with the class
/// named -- for BOTH spellings real exporters use for it.
///
/// # The regime, and why it is one regime with two spellings
///
/// A torus whose `minor_radius` exceeds its `major_radius` self-intersects: the
/// tube circle crosses the axis of revolution, so the swept surface passes
/// through itself and closes into an apex (the "spindle" or "lemon" torus).
/// ISO 10303-42 splits `degenerate_toroidal_surface` out of `toroidal_surface`
/// precisely for it (`WHERE major_radius < minor_radius`, plus `select_outer` to
/// pick a sheet). Measured over the 8-file corpus (spec 011 T1), it arrives
/// THREE ways:
///
/// | spelling | records | files |
/// |---|---|---|
/// | `DEGENERATE_TOROIDAL_SURFACE` | 253 | Rocky_House 156, Cruise_Assembly 63, Ai-14R 34 |
/// | `TOROIDAL_SURFACE` with NEGATIVE `major_radius` | 207 | NissanGT-R 201, ROTOR 6 |
/// | `TOROIDAL_SURFACE` with `0 < major_radius < minor_radius` | 136 | NissanGT-R 126, UMC-500 10 |
///
/// A negative major radius is out-of-schema (`positive_length_measure`), and it
/// is the same surface: `(-R, r)` at `(u, v)` equals `(R, r)` at
/// `(u + pi, pi - v)`. That reparameterisation is NOT a placement change, so it
/// cannot be normalised away without invalidating the face's trims (C2).
/// In every one of the 207 measured records `|major| < minor`, i.e. all three
/// spellings are the SAME degenerate regime.
///
/// # Why refuse rather than represent (measured, not assumed)
///
/// The rational-NURBS control net IS exact here -- with the builder's spindle
/// guard bypassed, the emitted net matches the analytic torus to a relative
/// 8e-16..9e-16 over the whole domain, with the control hull a superset of the
/// analytic bbox (the 7y standard, at the witness radii `0.633974596215563/1.0`
/// and at both negative-major samples). Exactness is not what fails.
///
/// What fails is everything that has to invert the parameterisation. On a
/// spindle, `Torus::search_parameter` returns `None` for 168 of 576 on-surface
/// grid points (29%; 144/576 = 25% at the NissanGT-R radii), and
/// `search_nearest_parameter` answers those same points with parameters that
/// evaluate to a DIFFERENT point -- silently, because the implicit form
/// `(sqrt(x^2+y^2) - R)^2 + z^2 = r^2` does not describe the sheet swept through
/// the axis. The ring control scores 576/576 on both. Converting would therefore
/// trade a typed refusal for a face whose trims can be placed wrongly on a
/// quarter of its domain -- worse than no fix, by the correct-or-typed standard.
/// `monstertruck_geometry`'s builder already encodes the same verdict by
/// returning `None` (`bspline_conversion.rs`, `torus_spindle_is_rejected`).
///
/// # What is NOT refused
///
/// HORN tori (`major_radius == minor_radius`, and the fp-near-horn fillets where
/// the two differ by a few ulps) are representable and stay so: the inner
/// equator pinches to a point on the axis, nothing self-intersects, and
/// parameter recovery measures 576/576. The rule below is deliberately the same
/// predicate the geometry builder uses, so the STEP layer refuses exactly what
/// the geometry layer cannot build -- no wider. `Ai-14R.stp` writes an exact
/// horn torus (`3., 3.`) as a `DEGENERATE_TOROIDAL_SURFACE`, so the spelling
/// cannot be trusted to imply the regime; the RADII decide.
fn refuse_degenerate_torus(
    entity: &str,
    major_radius: f64,
    minor_radius: f64,
) -> Result<(), StepConvertingError> {
    if !major_radius.is_finite() || !minor_radius.is_finite() {
        return Err(format!(
            "{entity} refused: non-finite radii (major_radius {major_radius}, \
             minor_radius {minor_radius})."
        )
        .into());
    }
    if minor_radius <= 0.0 {
        return Err(format!(
            "{entity} refused: non-positive minor_radius {minor_radius} \
             (major_radius {major_radius}); ISO 10303-42 declares it a \
             positive_length_measure."
        )
        .into());
    }
    if major_radius <= 0.0 {
        return Err(format!(
            "{entity} refused: non-positive major_radius {major_radius} with minor_radius \
             {minor_radius} -- ISO 10303-42 declares major_radius a positive_length_measure. \
             Negating it names the SAME surface as ({}, {minor_radius}) under a \
             (u + pi, pi - v) reparameterisation that no placement can absorb, and that \
             surface is the degenerate self-intersecting (spindle/lemon) torus monstertruck \
             cannot represent.",
            -major_radius
        )
        .into());
    }
    // Exactly the geometry builder's predicate (`TryIntoHomogeneousBsplineSurface
    // for Torus`): horn and fp-near-horn stay representable, true spindles do not.
    if minor_radius - major_radius > TOLERANCE * (major_radius + minor_radius) {
        return Err(format!(
            "{entity} refused: degenerate self-intersecting torus -- major_radius \
             {major_radius} < minor_radius {minor_radius} (the spindle/lemon regime \
             ISO 10303-42 splits degenerate_toroidal_surface out for). monstertruck has no \
             representation for it: the rational-NURBS builder rejects it, and analytic \
             parameter recovery is silently wrong on ~29% of its domain."
        )
        .into());
    }
    Ok(())
}

impl TryFrom<&ToroidalSurface> for step_geometry::ToroidalSurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(
        ToroidalSurface {
            position,
            major_radius,
            minor_radius,
            ..
        }: &ToroidalSurface,
    ) -> Result<Self, Self::Error> {
        refuse_degenerate_torus("TOROIDAL_SURFACE", *major_radius, *minor_radius)?;
        let mat = Matrix4::from(position);
        let torus = Torus::new(Point3::origin(), *major_radius, *minor_radius);
        Ok(Processor::new(torus).transformed(mat))
    }
}

/// `degenerate_toroidal_surface`
///
/// ISO 10303-42 subtype of `toroidal_surface`:
/// `(name, position, major_radius, minor_radius, select_outer)`. The fifth
/// attribute is a BOOLEAN -- which SHEET of the self-intersecting torus the face
/// lies on -- and is nothing like `toroidal_surface`'s attribute list, which
/// simply stops at `minor_radius`. The subtype's WHERE rule is
/// `major_radius < minor_radius`.
///
/// The entity exists in the schema, so it deserializes here rather than falling
/// into `Table::dummy`: a record that lands in `dummy` can only ever be refused
/// as a lookup miss (`Lookup failed for #145`), which names neither the class nor
/// the reason. See [`refuse_degenerate_torus`] for the fate of the geometry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = degenerate_toroidal_surface)]
#[holder(generate_deserialize)]
pub struct DegenerateToroidalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    major_radius: f64,
    minor_radius: f64,
    select_outer: bool,
}

impl TryFrom<&DegenerateToroidalSurface> for step_geometry::ToroidalSurface {
    type Error = StepConvertingError;
    fn try_from(
        DegenerateToroidalSurface {
            position,
            major_radius,
            minor_radius,
            select_outer,
            ..
        }: &DegenerateToroidalSurface,
    ) -> Result<Self, Self::Error> {
        refuse_degenerate_torus("DEGENERATE_TOROIDAL_SURFACE", *major_radius, *minor_radius)?;
        // Past the radii check the surface is a horn torus (or a ring one, if an
        // exporter mis-spells one as degenerate): `select_outer = .T.` is then the
        // whole surface and converts, while `.F.` selects the sheet that has
        // pinched to a single point on the axis -- which is not a surface.
        if !select_outer {
            return Err(format!(
                "DEGENERATE_TOROIDAL_SURFACE refused: select_outer = .F. selects the inner \
                 (apex) sheet, which at major_radius {major_radius} / minor_radius \
                 {minor_radius} has collapsed onto the axis of revolution and is not a \
                 representable surface."
            )
            .into());
        }
        let mat = Matrix4::from(position);
        let torus = Torus::new(Point3::origin(), *major_radius, *minor_radius);
        Ok(Processor::new(torus).transformed(mat))
    }
}

/// `conical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = conical_surface)]
#[holder(generate_deserialize)]
pub struct ConicalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
    semi_angle: f64,
}

impl From<&ConicalSurface> for step_geometry::ConicalSurface {
    fn from(
        ConicalSurface {
            position,
            radius,
            semi_angle,
            ..
        }: &ConicalSurface,
    ) -> Self {
        let mat = Matrix4::from(position);
        let p = Point3::new(*radius, 0.0, 0.0);
        // BoardStudio's STEP adapter normalizes the owning representation's declared
        // plane-angle unit before this context-free geometry conversion.
        let v = Vector3::new(f64::tan(*semi_angle), 0.0, 1.0);
        let rev =
            RevolutionSurface::by_revolution(Line(p, p + v), Point3::origin(), Vector3::unit_z());
        let mut processor = Processor::new(rev);
        processor.transform_by(mat);
        processor.invert();
        processor
    }
}

/// `b_spline_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum BsplineSurfaceAny {
    #[holder(use_place_holder)]
    NonRationalBsplineSurface(NonRationalBsplineSurface),
    #[holder(use_place_holder)]
    RationalBsplineSurface(RationalBsplineSurface),
}

impl TryFrom<&BsplineSurfaceAny> for Surface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BsplineSurfaceAny) -> Result<Self, Self::Error> {
        use BsplineSurfaceAny::*;
        Ok(match value {
            NonRationalBsplineSurface(bsp) => Surface::BsplineSurface(bsp.try_into()?),
            RationalBsplineSurface(bsp) => Surface::NurbsSurface(bsp.try_into()?),
        })
    }
}

/// `b_spline_surface_form`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BsplineSurfaceForm {
    PlaneSurf,
    CylindricalSurf,
    ConicalSurf,
    SphericalSurf,
    ToroidalSurf,
    SurfOfRevolution,
    RuledSurf,
    GeneralisedCone,
    QuadricSurf,
    SurfOfLinearExtrusion,
    Unspecified,
}

/// `b_spline_surface_with_knots`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = b_spline_surface_with_knots)]
#[holder(generate_deserialize)]
pub struct BsplineSurfaceWithKnots {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BsplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
    u_multiplicities: Vec<i64>,
    v_multiplicities: Vec<i64>,
    u_knots: Vec<f64>,
    v_knots: Vec<f64>,
    knot_spec: KnotType,
}

impl TryFrom<&BsplineSurfaceWithKnots> for BsplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &BsplineSurfaceWithKnots) -> Result<Self, StepConvertingError> {
        let uknots = surface.u_knots.to_vec();
        let umulti = surface
            .u_multiplicities
            .iter()
            .map(|n| *n as usize)
            .collect();
        let uknots = KnotVector::from_single_multi(uknots, umulti)?;
        let vknots = surface.v_knots.to_vec();
        let vmulti = surface
            .v_multiplicities
            .iter()
            .map(|n| *n as usize)
            .collect();
        let vknots = KnotVector::from_single_multi(vknots, vmulti)?;
        let ctrls = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Ok(Self::try_new((uknots, vknots), ctrls)?)
    }
}

/// `uniform_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = uniform_surface)]
#[holder(generate_deserialize)]
pub struct UniformSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BsplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl TryFrom<&UniformSurface> for BsplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &UniformSurface) -> Result<Self, StepConvertingError> {
        let uknots = uniform_knots(surface.control_points_list.len(), surface.u_degree as usize)?;
        let first = surface
            .control_points_list
            .first()
            .ok_or("control points list is empty.")?;
        let vknots = uniform_knots(first.len(), surface.v_degree as usize)?;
        let ctrls = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Ok(Self::try_new((uknots, vknots), ctrls)?)
    }
}

/// `quasi_uniform_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = quasi_uniform_surface)]
#[holder(generate_deserialize)]
pub struct QuasiUniformSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BsplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl TryFrom<&QuasiUniformSurface> for BsplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &QuasiUniformSurface) -> Result<Self, StepConvertingError> {
        let uknots =
            quasi_uniform_knots(surface.control_points_list.len(), surface.u_degree as usize);
        let first = surface
            .control_points_list
            .first()
            .ok_or("control points list is empty.")?;
        let vknots = quasi_uniform_knots(first.len(), surface.v_degree as usize);
        let ctrls = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Ok(Self::try_new((uknots, vknots), ctrls)?)
    }
}

/// `bezier_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = bezier_surface)]
#[holder(generate_deserialize)]
pub struct BezierSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BsplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl From<&BezierSurface> for BsplineSurface<Point3> {
    #[inline(always)]
    fn from(value: &BezierSurface) -> Self {
        let uknots = KnotVector::bezier_knot(value.u_degree as usize);
        let vknots = KnotVector::bezier_knot(value.v_degree as usize);
        let ctrls = value
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Self::new((uknots, vknots), ctrls)
    }
}

/// Entity that does not exist in AP042.
/// Surface before rationalization of [`RationalBsplineSurface`] defined by a complex entity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum NonRationalBsplineSurface {
    #[holder(use_place_holder)]
    BsplineSurfaceWithKnots(Box<BsplineSurfaceWithKnots>),
    #[holder(use_place_holder)]
    UniformSurface(Box<UniformSurface>),
    #[holder(use_place_holder)]
    QuasiUniformSurface(Box<QuasiUniformSurface>),
    #[holder(use_place_holder)]
    BezierSurface(Box<BezierSurface>),
}

impl NonRationalBsplineSurface {
    fn closed_axes(&self) -> [bool; 2] {
        let (u, v) = match self {
            Self::BsplineSurfaceWithKnots(surface) => {
                (surface.u_closed.clone(), surface.v_closed.clone())
            }
            Self::UniformSurface(surface) => (surface.u_closed.clone(), surface.v_closed.clone()),
            Self::QuasiUniformSurface(surface) => {
                (surface.u_closed.clone(), surface.v_closed.clone())
            }
            Self::BezierSurface(surface) => (surface.u_closed.clone(), surface.v_closed.clone()),
        };
        [matches!(u, Logical::True), matches!(v, Logical::True)]
    }
}

impl TryFrom<&NonRationalBsplineSurface> for BsplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &NonRationalBsplineSurface) -> Result<Self, Self::Error> {
        use NonRationalBsplineSurface::*;
        match value {
            BsplineSurfaceWithKnots(x) => x.as_ref().try_into(),
            UniformSurface(x) => x.as_ref().try_into(),
            QuasiUniformSurface(x) => x.as_ref().try_into(),
            BezierSurface(x) => Ok(x.as_ref().into()),
        }
    }
}

/// `rational_b_spline_surface` as complex entity
///
/// This struct is an ad hoc implementation that differs from the definition by EXPRESS:
/// in AP042, rationalized curves are defined as complex entities,
/// but here the surfaces before rationalization are held as internal variables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = rational_b_spline_surface)]
#[holder(generate_deserialize)]
pub struct RationalBsplineSurface {
    #[holder(use_place_holder)]
    non_rational_b_spline_surface: NonRationalBsplineSurface,
    weights_data: Vec<Vec<f64>>,
}

impl TryFrom<&RationalBsplineSurface> for NurbsSurface<Vector4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(
        RationalBsplineSurface {
            non_rational_b_spline_surface,
            weights_data,
        }: &RationalBsplineSurface,
    ) -> Result<Self, Self::Error> {
        let surface: BsplineSurface<Point3> = non_rational_b_spline_surface.try_into()?;
        Ok(Self::try_from_bspline_and_weights(
            surface,
            weights_data.clone(),
        )?)
    }
}

/// `swept_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum SweptSurfaceAny {
    #[holder(use_place_holder)]
    SurfaceOfLinearExtrusion(Box<SurfaceOfLinearExtrusion>),
    #[holder(use_place_holder)]
    SurfaceOfRevolution(Box<SurfaceOfRevolution>),
}

impl TryFrom<&SweptSurfaceAny> for SweepSurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &SweptSurfaceAny) -> Result<Self, Self::Error> {
        use SweptSurfaceAny::*;
        Ok(match value {
            SurfaceOfLinearExtrusion(x) => SweepSurface::ExtrusionSurface(x.as_ref().try_into()?),
            SurfaceOfRevolution(x) => SweepSurface::RevolutionSurface(x.as_ref().try_into()?),
        })
    }
}

/// `surface_of_linear_extrusion`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_linear_extrusion)]
#[holder(generate_deserialize)]
pub struct SurfaceOfLinearExtrusion {
    label: String,
    #[holder(use_place_holder)]
    swept_curve: CurveAny,
    #[holder(use_place_holder)]
    extrusion_axis: Vector,
}

impl TryFrom<&SurfaceOfLinearExtrusion> for StepExtrusionSurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(sr: &SurfaceOfLinearExtrusion) -> Result<Self, Self::Error> {
        let curve = Curve3D::try_from(&sr.swept_curve)?;
        let vector = Vector3::from(&sr.extrusion_axis);
        Ok(ExtrusionSurface::by_extrusion(curve, vector))
    }
}

/// `surface_of_revolution`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_revolution)]
#[holder(generate_deserialize)]
pub struct SurfaceOfRevolution {
    label: String,
    #[holder(use_place_holder)]
    swept_curve: CurveAny,
    #[holder(use_place_holder)]
    axis_position: Axis1Placement,
}

impl TryFrom<&SurfaceOfRevolution> for StepRevolutionSurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(sr: &SurfaceOfRevolution) -> Result<Self, Self::Error> {
        let curve = Curve3D::try_from(&sr.swept_curve)?;
        let origin = Point3::from(&sr.axis_position.location);
        let axis = sr.axis_position.direction().normalize();
        let mut rev = Processor::new(RevolutionSurface::by_revolution(curve, origin, axis));
        rev.invert();
        Ok(rev)
    }
}

#[cfg(test)]
mod placement_validation_tests {
    use super::*;

    fn position(axis: Option<[f64; 3]>, reference: Option<[f64; 3]>) -> Axis2Placement3d {
        let direction = |ratios: [f64; 3]| Direction {
            label: String::new(),
            direction_ratios: ratios.to_vec(),
        };
        Axis2Placement3d {
            label: String::new(),
            location: CartesianPoint::default(),
            axis: axis.map(direction),
            ref_direction: reference.map(direction),
        }
    }

    #[test]
    fn plane_surface_conversion_is_invariant_to_direction_ratio_scale() {
        let unit = Plane {
            label: String::new(),
            position: position(Some([0.0, 0.0, 1.0]), Some([1.0, 0.0, 1.0])),
        };
        let scaled = Plane {
            label: String::new(),
            position: position(Some([0.0, 0.0, 2.0]), Some([2.0, 0.0, 2.0])),
        };
        let unit_any = ElementarySurfaceAny::Plane(Box::new(unit));
        let scaled_any = ElementarySurfaceAny::Plane(Box::new(scaled));
        let unit_geom = ElementarySurface::try_from(&unit_any).unwrap();
        let scaled_geom = ElementarySurface::try_from(&scaled_any).unwrap();
        assert_eq!(unit_geom, scaled_geom);
    }

    #[test]
    fn cylinder_surface_conversion_is_invariant_to_direction_ratio_scale() {
        let unit = CylindricalSurface {
            label: String::new(),
            position: position(Some([0.0, 0.0, 1.0]), Some([1.0, 0.0, 1.0])),
            radius: 2.0,
        };
        let scaled = CylindricalSurface {
            label: String::new(),
            position: position(Some([0.0, 0.0, 2.0]), Some([2.0, 0.0, 2.0])),
            radius: 2.0,
        };
        let unit_any = ElementarySurfaceAny::CylindricalSurface(Box::new(unit));
        let scaled_any = ElementarySurfaceAny::CylindricalSurface(Box::new(scaled));
        let unit_geom = ElementarySurface::try_from(&unit_any).unwrap();
        let scaled_geom = ElementarySurface::try_from(&scaled_any).unwrap();
        assert_eq!(unit_geom, scaled_geom);
    }

    #[test]
    fn omitted_reference_uses_step_default_for_positive_and_negative_x_axes() {
        for sign in [1.0, -1.0] {
            for scale in [1e-300, 1.0, 1e300] {
                let plane = Plane {
                    label: String::new(),
                    position: position(Some([sign * scale, 0.0, 0.0]), None),
                };
                let any = ElementarySurfaceAny::Plane(Box::new(plane));
                let ElementarySurface::Plane(converted) =
                    ElementarySurface::try_from(&any).unwrap()
                else {
                    panic!("plane conversion returned a different surface type");
                };
                assert_eq!(converted.axis_u(), Vector3::unit_y());
                assert_eq!(converted.axis_v(), Vector3::unit_z() * sign);
            }
        }
    }

    #[test]
    fn analytic_surface_conversion_rejects_invalid_explicit_directions() {
        let invalid = Plane {
            label: String::new(),
            position: position(Some([0.0, 0.0, 0.0]), Some([1.0, 0.0, 0.0])),
        };
        let any = ElementarySurfaceAny::Plane(Box::new(invalid));
        assert!(ElementarySurface::try_from(&any).is_err());
    }
}

#[cfg(test)]
mod closed_chart_certificate_tests {
    use super::*;
    use step_p21::tables::IntoOwned;

    #[test]
    fn closed_chart_certificate_accepts_source_unclamped_active_boundary() {
        // Exact surface/control records from Choc_V2_Red.step, SHA256
        // 0542ee277eeb6988412e0780ae86b23d96fdd118f8242e0bf3224cff7dd7acc4.
        let table = Table::from_step(include_str!(
            "../../../../tests/input/fixtures/choc-unclamped-surface.step"
        ))
        .unwrap();
        let rational = table.rational_b_spline_surface[&27734]
            .clone()
            .into_owned(&table)
            .unwrap();
        let raw = SurfaceAny::BsplineSurface(Box::new(BsplineSurfaceAny::RationalBsplineSurface(
            rational,
        )));
        let converted = Surface::try_from(&raw).unwrap();
        let unchanged = converted.clone();
        let Surface::NurbsSurface(surface) = &converted else {
            unreachable!()
        };
        assert_eq!(surface.degrees(), (6, 10));
        assert_eq!(surface.control_points().len(), 7);
        assert_eq!(surface.control_points()[0].len(), 38);
        assert_eq!(
            surface.knot_vectors().0.as_ref(),
            &[
                -6.28318530718,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                6.28318530718,
                6.28318530718,
                6.28318530718,
                6.28318530718,
                6.28318530718,
                6.28318530718,
                12.56637061436,
            ]
        );
        assert_eq!(
            converted.try_range_tuple(),
            (Some((0.0, 6.28318530718)), Some((0.0, 6.303048278758)))
        );
        assert_eq!(raw.declared_closed_axes(), Some([true, false]));
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([true, false])
        );
        assert_eq!(converted, unchanged);
        assert_eq!(converted.period_u(), None);
        assert_eq!(converted.period_v(), None);
    }

    fn source(points: Vec<Vec<[f64; 3]>>, weights: Option<Vec<Vec<f64>>>) -> SurfaceAny {
        let nu = points.len() as i64;
        let nv = points[0].len() as i64;
        let nonrational =
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(Box::new(BsplineSurfaceWithKnots {
                label: String::new(),
                u_degree: nu - 1,
                v_degree: nv - 1,
                control_points_list: points
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|p| CartesianPoint {
                                label: String::new(),
                                coordinates: p.to_vec(),
                            })
                            .collect()
                    })
                    .collect(),
                surface_form: BsplineSurfaceForm::Unspecified,
                u_closed: Logical::True,
                v_closed: Logical::False,
                self_intersect: Logical::Unknown,
                u_multiplicities: vec![nu, nu],
                v_multiplicities: vec![nv, nv],
                u_knots: vec![0.0, 1.0],
                v_knots: vec![0.0, 1.0],
                knot_spec: KnotType::Unspecified,
            }));
        SurfaceAny::BsplineSurface(Box::new(match weights {
            Some(weights_data) => {
                BsplineSurfaceAny::RationalBsplineSurface(RationalBsplineSurface {
                    non_rational_b_spline_surface: nonrational,
                    weights_data,
                })
            }
            None => BsplineSurfaceAny::NonRationalBsplineSurface(nonrational),
        }))
    }

    fn closed_points() -> Vec<Vec<[f64; 3]>> {
        let boundary = vec![[0.0, 0.0, 0.0], [0.0, 1.0, 1.0], [0.0, 2.0, 0.0]];
        vec![
            boundary.clone(),
            boundary.iter().map(|p| [1.0, p[1], p[2]]).collect(),
            boundary,
        ]
    }

    fn with_knots(raw: &mut SurfaceAny) -> &mut BsplineSurfaceWithKnots {
        let SurfaceAny::BsplineSurface(any) = raw else {
            unreachable!()
        };
        let nonrational = match any.as_mut() {
            BsplineSurfaceAny::NonRationalBsplineSurface(surface) => surface,
            BsplineSurfaceAny::RationalBsplineSurface(surface) => {
                &mut surface.non_rational_b_spline_surface
            }
        };
        let NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface) = nonrational else {
            unreachable!()
        };
        surface
    }

    fn arithmetic_source(knots: Vec<f64>, transpose: bool) -> SurfaceAny {
        let table = Table::from_step(include_str!(
            "../../../../tests/input/fixtures/choc-unclamped-surface.step"
        ))
        .unwrap();
        let mut rational = table.rational_b_spline_surface[&27734]
            .clone()
            .into_owned(&table)
            .unwrap();
        let NonRationalBsplineSurface::BsplineSurfaceWithKnots(spline) =
            &mut rational.non_rational_b_spline_surface
        else {
            unreachable!()
        };
        spline.u_knots = knots;
        if transpose {
            let nv = spline.control_points_list[0].len();
            let nu = spline.control_points_list.len();
            spline.control_points_list = (0..nv)
                .map(|v| {
                    (0..nu)
                        .map(|u| spline.control_points_list[u][v].clone())
                        .collect()
                })
                .collect();
            rational.weights_data = (0..nv)
                .map(|v| (0..nu).map(|u| rational.weights_data[u][v]).collect())
                .collect();
            std::mem::swap(&mut spline.u_degree, &mut spline.v_degree);
            std::mem::swap(&mut spline.u_closed, &mut spline.v_closed);
            std::mem::swap(&mut spline.u_knots, &mut spline.v_knots);
            std::mem::swap(&mut spline.u_multiplicities, &mut spline.v_multiplicities);
        }
        SurfaceAny::BsplineSurface(Box::new(BsplineSurfaceAny::RationalBsplineSurface(
            rational,
        )))
    }

    #[test]
    fn closed_chart_certificate_refuses_subnormal_active_span() {
        for transpose in [false, true] {
            let raw = arithmetic_source(vec![-1.0, 0.0, 1e-320, 2.0], transpose);
            let converted = Surface::try_from(&raw).unwrap();
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([false, false])
            );
        }
    }

    #[test]
    fn closed_chart_certificate_refuses_overflowed_outer_extent() {
        for transpose in [false, true] {
            let raw = arithmetic_source(vec![-f64::MAX, 0.0, 1.0, f64::MAX], transpose);
            let converted = Surface::try_from(&raw).unwrap();
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([false, false])
            );
        }
    }

    #[test]
    fn closed_chart_certificate_accepts_representable_source_active_spans() {
        for transpose in [false, true] {
            for span in [1e-9_f64, 1e-4, 1.0, 1e6] {
                let outside = span.max(1.0);
                let raw = arithmetic_source(vec![-outside, 0.0, span, 2.0 * outside], transpose);
                let converted = Surface::try_from(&raw).unwrap();
                assert_eq!(
                    raw.certified_closed_axes_for(&converted),
                    Some([!transpose, transpose])
                );
                let v = 3.151524139379;
                let endpoints = if transpose {
                    [converted.evaluate(v, 0.0), converted.evaluate(v, span)]
                } else {
                    [converted.evaluate(0.0, v), converted.evaluate(span, v)]
                };
                assert!(
                    endpoints
                        .iter()
                        .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite())
                );
                assert!(endpoints[0].distance(endpoints[1]) < 1e-12);
            }
        }
    }

    #[test]
    fn closed_chart_certificate_accepts_exact_active_endpoint_rows_in_either_axis() {
        for transpose in [false, true] {
            for weighted in [false, true] {
                for (knots, multiplicities) in [
                    (vec![0.0, 1.0], vec![3, 3]),
                    (vec![-1.0, 0.0, 1.0, 2.0], vec![1, 2, 2, 1]),
                    (vec![0.0, 1.0, 2.0], vec![3, 2, 1]),
                    (vec![-1.0, 0.0, 1.0], vec![1, 2, 3]),
                ] {
                    let points = closed_points();
                    let points = if transpose {
                        (0..3)
                            .map(|v| (0..3).map(|u| points[u][v]).collect())
                            .collect()
                    } else {
                        points
                    };
                    let weights = weighted.then(|| {
                        (0..3)
                            .map(|u| {
                                (0..3)
                                    .map(|v| {
                                        if (if transpose { u } else { v }) == 1 {
                                            2.0
                                        } else {
                                            1.0
                                        }
                                    })
                                    .collect()
                            })
                            .collect()
                    });
                    let mut raw = source(points, weights);
                    let spline = with_knots(&mut raw);
                    if transpose {
                        spline.u_closed = Logical::False;
                        spline.v_closed = Logical::True;
                        spline.v_knots = knots;
                        spline.v_multiplicities = multiplicities;
                    } else {
                        spline.u_knots = knots;
                        spline.u_multiplicities = multiplicities;
                    }
                    let converted = Surface::try_from(&raw).unwrap();
                    assert_eq!(
                        raw.certified_closed_axes_for(&converted),
                        Some([!transpose, transpose])
                    );
                    let expected = Point3::new(0.0, 1.0, if weighted { 2.0 / 3.0 } else { 0.5 });
                    let endpoints = if transpose {
                        [converted.evaluate(0.5, 0.0), converted.evaluate(0.5, 1.0)]
                    } else {
                        [converted.evaluate(0.0, 0.5), converted.evaluate(1.0, 0.5)]
                    };
                    assert!(
                        endpoints
                            .iter()
                            .all(|point| point.distance(expected) < 1e-14)
                    );
                    assert_eq!(converted.period_u(), None);
                    assert_eq!(converted.period_v(), None);
                }
            }
        }
    }

    #[test]
    fn closed_chart_certificate_preserves_narrow_scaled_and_joined_domains() {
        for span in [1e-9_f64, 1.0, 1e6] {
            let boundary = (0..5)
                .map(|i| [0.0, i as f64, (i % 2) as f64])
                .collect::<Vec<_>>();
            let interior = boundary.iter().map(|p| [1.0, p[1], p[2]]).collect();
            let mut raw = source(
                vec![boundary.clone(), interior, boundary],
                Some(vec![vec![1.0; 5]; 3]),
            );
            let spline = with_knots(&mut raw);
            let outside = span.max(1.0);
            spline.u_knots = vec![-outside, 0.0, span, 2.0 * outside];
            spline.u_multiplicities = vec![1, 2, 2, 1];
            spline.v_degree = 2;
            spline.v_knots = vec![0.0, 0.5, 1.0];
            spline.v_multiplicities = vec![3, 2, 3];
            let converted = Surface::try_from(&raw).unwrap();
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([true, false])
            );
            for v in [0.0, 0.125, 0.5 - 1e-9, 0.5, 0.5 + 1e-9, 0.875, 1.0] {
                assert!(
                    converted
                        .evaluate(0.0, v)
                        .distance(converted.evaluate(span, v))
                        < 1e-14
                );
            }
        }
    }

    #[test]
    fn closed_chart_certificate_rejects_shifted_active_endpoint_blocks() {
        let points = closed_points();
        let mut second_interior = points[1].clone();
        for point in &mut second_interior {
            point[0] = 2.0;
        }
        let rows = vec![
            points[0].clone(),
            points[1].clone(),
            second_interior,
            points[0].clone(),
        ];
        for (knots, multiplicities) in [
            (vec![-1.0, 0.0, 1.0, 2.0], vec![1, 3, 2, 1]),
            (vec![-1.0, 0.0, 1.0, 2.0], vec![1, 2, 3, 1]),
            (vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0], vec![1; 7]),
        ] {
            let mut raw = source(rows.clone(), None);
            let spline = with_knots(&mut raw);
            spline.u_degree = 2;
            spline.u_knots = knots;
            spline.u_multiplicities = multiplicities;
            let converted = Surface::try_from(&raw).unwrap();
            let (Some((a, b)), _) = converted.try_range_tuple() else {
                unreachable!()
            };
            // A shifted right block can match at the endpoint while jumping
            // to a different row immediately inside the active interval.
            let epsilon = (b - a) * 1e-8;
            assert!(
                converted
                    .evaluate(a + epsilon, 0.5)
                    .distance(converted.evaluate(b - epsilon, 0.5))
                    > 0.49
            );
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([false, false])
            );
        }
    }

    #[test]
    fn closed_chart_certificate_rejects_unclamped_mismatch_and_invalid_weights() {
        let mut raw = source(closed_points(), Some(vec![vec![1.0; 3]; 3]));
        let spline = with_knots(&mut raw);
        spline.u_knots = vec![-1.0, 0.0, 1.0, 2.0];
        spline.u_multiplicities = vec![1, 2, 2, 1];
        let converted = Surface::try_from(&raw).unwrap();
        let mut different = raw.clone();
        with_knots(&mut different).control_points_list[1][1].coordinates[0] += 1.0;
        assert_eq!(
            raw.certified_closed_axes_for(&Surface::try_from(&different).unwrap()),
            None
        );
        with_knots(&mut different).control_points_list[2][1].coordinates[2] += 1.0;
        let other = Surface::try_from(&different).unwrap();
        assert!(other.evaluate(0.0, 0.5).distance(other.evaluate(1.0, 0.5)) > 0.49);
        assert_eq!(
            different.certified_closed_axes_for(&other),
            Some([false, false])
        );
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([true, false])
        );
        for weight in [0.0, -1.0, f64::INFINITY, f64::NAN, 2.0] {
            let mut different = raw.clone();
            let SurfaceAny::BsplineSurface(any) = &mut different else {
                unreachable!()
            };
            let BsplineSurfaceAny::RationalBsplineSurface(rational) = any.as_mut() else {
                unreachable!()
            };
            rational.weights_data[2][1] = weight;
            let converted = Surface::try_from(&different).unwrap();
            let certificate = different.certified_closed_axes_for(&converted);
            if weight == 2.0 {
                assert_eq!(certificate, Some([false, false]));
            } else {
                assert_eq!(certificate, None);
            }
        }
    }

    #[test]
    fn closed_chart_certificate_accepts_equal_rows_without_a_period() {
        for weights in [None, Some(vec![vec![1.0, 2.0, 1.0]; 3])] {
            let raw = source(closed_points(), weights);
            let converted = Surface::try_from(&raw).unwrap();
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([true, false])
            );
            assert_eq!(converted.period_u(), None);
            assert_eq!(converted.period_v(), None);
            assert!(
                converted
                    .derivative_u(0.0, 0.5)
                    .distance(converted.derivative_u(1.0, 0.5))
                    > 1.0
            );
        }
    }

    #[test]
    fn closed_chart_certificate_rejects_sample_aliased_open_seam() {
        // Bernstein coefficients for 100*v*(v-.25)*(v-.5)*(v-.75)*(v-1).
        let gap = [0.0, 15.0 / 8.0, -65.0 / 16.0, 65.0 / 16.0, -15.0 / 8.0, 0.0];
        let lower = (0..6)
            .map(|i| [0.0, i as f64 / 5.0, 0.0])
            .collect::<Vec<_>>();
        let upper = (0..6).map(|i| [0.0, i as f64 / 5.0, gap[i]]).collect();
        let raw = source(vec![lower, upper], None);
        let converted = Surface::try_from(&raw).unwrap();
        assert_eq!(raw.declared_closed_axes(), Some([true, false]));
        for v in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!(
                converted
                    .evaluate(0.0, v)
                    .distance(converted.evaluate(1.0, v))
                    < 1e-12
            );
        }
        assert!(
            converted
                .evaluate(0.0, 0.125)
                .distance(converted.evaluate(1.0, 0.125))
                > 0.32
        );
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }

    #[test]
    fn closed_chart_certificate_rejects_unequal_rational_weights() {
        let mut weights = vec![vec![1.0; 3]; 3];
        weights[2][1] = 2.0;
        let raw = source(closed_points(), Some(weights));
        let converted = Surface::try_from(&raw).unwrap();
        assert!(
            converted
                .evaluate(0.0, 0.5)
                .distance(converted.evaluate(1.0, 0.5))
                > 0.1
        );
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }

    #[test]
    fn closed_chart_certificate_is_bound_to_the_converted_surface() {
        let raw = source(closed_points(), None);
        let mut other = closed_points();
        other[1][1][0] += 1.0;
        let different = Surface::try_from(&source(other, None)).unwrap();
        assert_eq!(raw.certified_closed_axes_for(&different), None);
    }

    #[test]
    fn closed_chart_certificate_rejects_unclamped_endpoint_rows() {
        let mut raw = source(closed_points(), None);
        let SurfaceAny::BsplineSurface(ref mut any) = raw else {
            unreachable!()
        };
        let BsplineSurfaceAny::NonRationalBsplineSurface(
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface),
        ) = any.as_mut()
        else {
            unreachable!()
        };
        surface.u_knots = vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
        surface.u_multiplicities = vec![1; 6];
        let converted = Surface::try_from(&raw).unwrap();
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }

    #[test]
    fn closed_chart_certificate_rejects_excess_endpoint_multiplicity() {
        let points = closed_points();
        let mut rows = vec![
            points[0].clone(),
            points[1].clone(),
            points[1].clone(),
            points[0].clone(),
        ];
        for point in &mut rows[2] {
            point[0] = 2.0;
        }
        let mut raw = source(rows, None);
        let SurfaceAny::BsplineSurface(ref mut any) = raw else {
            unreachable!()
        };
        let BsplineSurfaceAny::NonRationalBsplineSurface(
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface),
        ) = any.as_mut()
        else {
            unreachable!()
        };
        surface.u_degree = 1;
        surface.u_multiplicities = vec![3, 3];
        let converted = Surface::try_from(&raw).unwrap();
        assert!(
            converted
                .evaluate(1e-8, 0.5)
                .distance(converted.evaluate(1.0 - 1e-8, 0.5))
                > 0.9
        );
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }

    #[test]
    fn closed_chart_certificate_rejects_approximately_clamped_knots() {
        let mut raw = source(closed_points(), None);
        let SurfaceAny::BsplineSurface(ref mut any) = raw else {
            unreachable!()
        };
        let BsplineSurfaceAny::NonRationalBsplineSurface(
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface),
        ) = any.as_mut()
        else {
            unreachable!()
        };
        surface.u_knots = vec![0.0, 1e-8, 2e-8, 1.0];
        surface.u_multiplicities = vec![1, 1, 1, 3];
        let converted = Surface::try_from(&raw).unwrap();
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }

    #[test]
    fn closed_chart_certificate_checks_v_axis_and_positive_weights() {
        let points = closed_points();
        let transposed = (0..3)
            .map(|v| (0..3).map(|u| points[u][v]).collect())
            .collect();
        let mut raw = source(transposed, None);
        let SurfaceAny::BsplineSurface(ref mut any) = raw else {
            unreachable!()
        };
        let BsplineSurfaceAny::NonRationalBsplineSurface(
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface),
        ) = any.as_mut()
        else {
            unreachable!()
        };
        surface.u_closed = Logical::False;
        surface.v_closed = Logical::True;
        let converted = Surface::try_from(&raw).unwrap();
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, true])
        );
        for weight in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            let raw = source(closed_points(), Some(vec![vec![weight; 3]; 3]));
            let converted = Surface::try_from(&raw).unwrap();
            assert!(raw.certified_closed_axes_for(&converted).is_none());
        }
    }

    #[test]
    fn closed_chart_certificate_requires_declared_closure() {
        let mut raw = source(closed_points(), None);
        let SurfaceAny::BsplineSurface(ref mut any) = raw else {
            unreachable!()
        };
        let BsplineSurfaceAny::NonRationalBsplineSurface(
            NonRationalBsplineSurface::BsplineSurfaceWithKnots(surface),
        ) = any.as_mut()
        else {
            unreachable!()
        };
        surface.u_closed = Logical::False;
        let converted = Surface::try_from(&raw).unwrap();
        assert_eq!(
            raw.certified_closed_axes_for(&converted),
            Some([false, false])
        );
    }
    #[test]
    fn closed_chart_certificate_reproves_uniformly_scaled_geometry() {
        for weights in [None, Some(vec![vec![1.0, 2.0, 1.0]; 3])] {
            let raw = source(closed_points(), weights);
            let converted = Surface::try_from(&raw).unwrap();
            for factor in [0.001, 1.0, 25.4, 1000.0] {
                let mut scaled = converted.clone();
                scaled.transform_by(Matrix4::from_scale(factor));
                assert_eq!(
                    raw.certified_closed_axes_after_uniform_scale(&converted, &scaled, factor),
                    Some([true, false])
                );
            }
        }
    }

    #[test]
    fn closed_chart_certificate_scale_requires_both_exact_surface_bindings() {
        let raw = source(closed_points(), None);
        let converted = Surface::try_from(&raw).unwrap();
        let mut scaled = converted.clone();
        scaled.transform_by(Matrix4::from_scale(25.4));
        for factor in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0] {
            assert_eq!(
                raw.certified_closed_axes_after_uniform_scale(&converted, &scaled, factor),
                None
            );
        }
        for factor in [0.0, -1.0] {
            let mut invalid = converted.clone();
            invalid.transform_by(Matrix4::from_scale(factor));
            assert_eq!(
                raw.certified_closed_axes_after_uniform_scale(&converted, &invalid, factor),
                None
            );
        }
        let mut other_points = closed_points();
        other_points[1][1][0] = 2.0;
        let other = Surface::try_from(&source(other_points, None)).unwrap();
        let mut other_scaled = other.clone();
        other_scaled.transform_by(Matrix4::from_scale(25.4));
        assert_eq!(
            raw.certified_closed_axes_after_uniform_scale(&other, &other_scaled, 25.4),
            None
        );
        assert_eq!(
            raw.certified_closed_axes_after_uniform_scale(&converted, &other_scaled, 25.4),
            None
        );
        assert_eq!(
            raw.certified_closed_axes_after_uniform_scale(&converted, &scaled, 25.4),
            Some([true, false])
        );
    }

    #[test]
    fn closed_chart_certificate_scale_refuses_overflow_after_exact_transform() {
        for weighted in [false, true] {
            let mut points = closed_points();
            for point in points.iter_mut().flatten() {
                point[0] = 1e308;
            }
            let raw = source(points, weighted.then(|| vec![vec![1.0; 3]; 3]));
            let converted = Surface::try_from(&raw).unwrap();
            assert_eq!(
                raw.certified_closed_axes_for(&converted),
                Some([true, false])
            );
            let mut scaled = converted.clone();
            scaled.transform_by(Matrix4::from_scale(2.0));
            assert_eq!(
                raw.certified_closed_axes_after_uniform_scale(&converted, &scaled, 2.0),
                None
            );
        }
    }

    #[test]
    fn closed_chart_certificate_scale_preserves_equivalent_mm_and_inch_boundaries() {
        for mm_per_unit in [1.0, 25.4] {
            let mut points = closed_points();
            for coordinate in points.iter_mut().flatten().flatten() {
                *coordinate /= mm_per_unit;
            }
            let raw = source(points, Some(vec![vec![1.0, 2.0, 1.0]; 3]));
            let converted = Surface::try_from(&raw).unwrap();
            let mut normalized = converted.clone();
            normalized.transform_by(Matrix4::from_scale(mm_per_unit));
            assert_eq!(
                raw.certified_closed_axes_after_uniform_scale(&converted, &normalized, mm_per_unit),
                Some([true, false])
            );
            let expected = Point3::new(0.0, 1.0, 2.0 / 3.0);
            assert!(normalized.evaluate(0.0, 0.5).distance(expected) < 1e-14);
            assert!(normalized.evaluate(1.0, 0.5).distance(expected) < 1e-14);
        }
    }
}
