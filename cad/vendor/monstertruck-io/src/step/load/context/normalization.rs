//! The checked STEP length normalizer, preserving STEP plane parameter units.
use super::super::{convert::StepCompressedShell, step_geometry::*};

fn normalize_plane_to_mm(plane: &mut Plane, factor: f64) {
    // UV lengths scale separately; retain frame axes without world endpoint rounding.
    let frame = Matrix4::from_cols(
        plane.axis_u().extend(0.0),
        plane.axis_v().extend(0.0),
        plane.normal().extend(0.0),
        (plane.origin() * factor).to_homogeneous(),
    );
    *plane = Plane::xy().transformed(frame);
}

/// Applies the existing STEP millimetre normalization to a private source clone.
pub fn normalize_shell_to_mm(shell: &mut StepCompressedShell, mm_per_unit: f64) {
    let scale = Matrix4::from_scale(mm_per_unit);
    for vertex in &mut shell.vertices {
        vertex.transform_by(scale);
    }
    for edge in &mut shell.edges {
        edge.curve = normalize_curve_with_plane_pcurves(edge.curve.clone(), mm_per_unit);
    }
    for face in &mut shell.faces {
        if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut face.surface {
            normalize_plane_to_mm(plane, mm_per_unit);
        } else {
            face.surface.transform_by(scale);
        }
    }
}

fn normalize_curve_with_plane_pcurves(mut curve: Curve3D, factor: f64) -> Curve3D {
    let scale = Matrix4::from_scale(factor);
    if let Curve3D::ParameterCurve(parameter_curve) = &curve {
        return Curve3D::ParameterCurve(normalize_parameter_curve(parameter_curve.clone(), factor));
    }
    let Curve3D::SurfaceCurve(surface_curve) = &curve else {
        curve.transform_by(scale);
        return curve;
    };
    let mut leader = surface_curve.leader().clone();
    leader.transform_by(scale);
    let associated = surface_curve
        .associated_geometry()
        .iter()
        .cloned()
        .map(|geometry| match geometry {
            SurfaceCurveAssociatedGeometry::ParameterCurve(parameter_curve) => {
                SurfaceCurveAssociatedGeometry::ParameterCurve(normalize_parameter_curve(
                    parameter_curve,
                    factor,
                ))
            }
            SurfaceCurveAssociatedGeometry::Surface(mut surface) => {
                if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut *surface {
                    normalize_plane_to_mm(plane, factor);
                } else {
                    surface.transform_by(scale);
                }
                SurfaceCurveAssociatedGeometry::Surface(surface)
            }
        })
        .collect();
    let normalized = SurfaceCurve3D::new(
        surface_curve.kind(),
        Box::new(leader),
        associated,
        surface_curve.master_representation(),
    );
    Curve3D::SurfaceCurve(
        normalized
            .try_with_active_trim(surface_curve.active_trim())
            .expect("length normalization preserves the selected-master scalar interval"),
    )
}

fn normalize_parameter_curve(mut curve: StepParameterCurve, factor: f64) -> StepParameterCurve {
    let scale = Matrix4::from_scale(factor);
    let (mut uv_curve, mut surface) = curve.clone().decompose();
    if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &mut *surface {
        uv_curve.transform_by(Matrix3::from_scale(factor));
        normalize_plane_to_mm(plane, factor);
        StepParameterCurve::new(uv_curve, surface)
    } else {
        curve.transform_by(scale);
        curve
    }
}
