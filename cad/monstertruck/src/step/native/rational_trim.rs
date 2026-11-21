//! Recover a finite elliptic arc from a degree-elevated rational Bézier span.
use super::{geometry::Curve, *};

pub(super) struct FiniteEllipse {
    pub(super) interval: (f64, f64),
    pub(super) residual: f64,
}

pub(super) fn read(
    records: &SourceRecords<'_>,
    scene: &scene::Scene<'_>,
    id: u64,
    forward: bool,
) -> Result<(Curve, FiniteEllipse), String> {
    let unsupported = || format!("unsupported complex representation item #{id}");
    let parts = &records[&id];
    if parts.len() != 7 {
        return Err(unsupported());
    }
    let mut parts_by_name = HashMap::new();
    for &part in parts {
        if parts_by_name.insert(part.name.as_str(), part).is_some() {
            return Err(unsupported());
        }
    }
    let read = |name, count| parameters(parts_by_name.get(name).ok_or_else(unsupported)?, count);
    for name in ["BOUNDED_CURVE", "CURVE", "GEOMETRIC_REPRESENTATION_ITEM"] {
        read(name, 0)?;
    }
    let name = read("REPRESENTATION_ITEM", 1)?;
    let spline = read("B_SPLINE_CURVE", 5)?;
    let knots = read("B_SPLINE_CURVE_WITH_KNOTS", 3)?;
    let weight_data = read("RATIONAL_B_SPLINE_CURVE", 1)?;
    // The curve form, closed and self-intersect flags are informational in
    // ISO 10303-42. The control data below establish the actual finite span.
    let logical = |param: &Parameter| matches!(param, Parameter::Enumeration(v) if matches!(v.as_str(), "T" | "F" | "U"));
    if !matches!(name[0], Parameter::String(_))
        || !matches!(spline[0], Parameter::Integer(3))
        || !matches!(&spline[2], Parameter::Enumeration(v) if v == "UNSPECIFIED" || v == "ELLIPTIC_ARC")
        || !logical(&spline[3])
        || !logical(&spline[4])
        || !matches!(&knots[0], Parameter::List(v) if matches!(v.as_slice(), [Parameter::Integer(4), Parameter::Integer(4)]))
        || !matches!(&knots[2], Parameter::Enumeration(v) if v == "UNSPECIFIED" || v == "PIECEWISE_BEZIER_KNOTS")
    {
        return Err(unsupported());
    }
    let Parameter::List(knot_values) = &knots[1] else {
        return Err(unsupported());
    };
    let [lo, hi] = knot_values.as_slice() else {
        return Err(unsupported());
    };
    let span = scene::real(hi)? - scene::real(lo)?;
    if !span.is_finite() || span <= 0.0 {
        return Err(unsupported());
    }
    // With only the two clamped knots, increasing normalized parameter
    // traverses the original curve in Bernstein form from pole 0 to pole 3.
    let Parameter::List(weights) = &weight_data[0] else {
        return Err(unsupported());
    };
    let [a, b, c, d] = weights.as_slice() else {
        return Err(unsupported());
    };
    let w = scene::real(b)?;
    let cos_half = (3.0 * w - 1.0) / 2.0;
    if scene::real(a)? != 1.0
        || scene::real(d)? != 1.0
        || scene::real(c)? != w
        || !(0.0 < cos_half && cos_half < 1.0)
    {
        return Err(unsupported());
    }
    let [a, b, c, d] = references(&spline[1])?[..] else {
        return Err(unsupported());
    };
    let point = |id| scene.vector(id, "CARTESIAN_POINT");
    let origin = point(a)?;
    let [p1, p2, end] = [point(b)?, point(c)?, point(d)?].map(|p| p - origin);
    let direction = end.normalize();
    // Rational Bernstein weights are positive. Ordered pole projections
    // make the source curve strictly increasing along its endpoint chord.
    let [s1, s2, length] = [p1, p2, end].map(|p| p.dot(direction));
    if !(0.0 < s1 && s1 < s2 && s2 < length) {
        return Err(unsupported());
    }

    // Invert cubic degree elevation in homogeneous coordinates. Both
    // interior poles independently recover the quadratic middle pole.
    let h1 = p1 * w;
    let h2 = p2 * w;
    let q = (3.0 * h1 + (3.0 * h2 - end)) * 0.25;
    let elevated = [q * (2.0 / 3.0), (2.0 * q + end) / 3.0];
    let pole_error = (h1 - elevated[0])
        .magnitude()
        .max((h2 - elevated[1]).magnitude());
    let elevated_weight = (1.0 + 2.0 * cos_half) / 3.0;
    let max_point = p1.magnitude().max(p2.magnitude()).max(length);
    // Bernstein basis functions are nonnegative and sum to one. Bound the
    // rational quotient error over the entire knot interval using the least
    // denominator and the worst numerator/weight coefficient error.
    let residual = (pole_error + (elevated_weight - w).abs() * max_point) / w.min(elevated_weight);

    let half_angle = cos_half.acos();
    let sin_half = ((1.0 - cos_half) * (1.0 + cos_half)).sqrt();
    // The unit-circle quadratic has endpoints (cos α, ±sin α),
    // middle pole (1/cos α, 0), and middle weight cos α. Its affine
    // image is center + u cos(t) + v sin(t), for t in [-α, α].
    let u = (q - end * (0.5 * cos_half)) / (sin_half * sin_half);
    let v = end / (2.0 * sin_half);
    let center_offset = end * 0.5 - u * cos_half;
    let center = origin + center_offset;
    // Rotate the two harmonic coefficients into orthogonal major/minor axes
    // while preserving the increasing angle of the rational quadratic.
    let phase = 0.5 * (2.0 * u.dot(v)).atan2(u.magnitude2() - v.magnitude2());
    let (sin, cos) = phase.sin_cos();
    let major_axis = u * cos + v * sin;
    let minor_axis = v * cos - u * sin;
    let major = major_axis.magnitude();
    let x = major_axis / major;
    let y_axis = minor_axis - x * x.dot(minor_axis);
    let minor = y_axis.magnitude();
    let y = y_axis / minor;
    let z = x.cross(y);
    let (fitted_major, fitted_minor) = (x * major, y * minor);
    let residual = residual
        + ((center - origin) - center_offset).magnitude()
        + (u - (fitted_major * cos - fitted_minor * sin)).magnitude()
        + (v - (fitted_major * sin + fitted_minor * cos)).magnitude();
    if !residual.is_finite() || !(minor > 0.0 && major >= minor && major.is_finite()) {
        return Err(unsupported());
    }
    let frame = Matrix4::from_cols(
        x.extend(0.0),
        y.extend(0.0),
        z.extend(0.0),
        center.extend(1.0),
    );
    let (start, sweep) = if forward {
        (-half_angle - phase, 2.0 * half_angle)
    } else {
        (half_angle - phase, -2.0 * half_angle)
    };
    Ok((
        Curve::Ellipse {
            frame,
            major,
            minor,
            forward,
        },
        FiniteEllipse {
            interval: (start, sweep),
            residual,
        },
    ))
}
