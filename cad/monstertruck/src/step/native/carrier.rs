//! Check the two authored pcurves against the oriented 3D carrier. The supported
//! pcurves are unbounded lines and full circles, so the shared vertices and edge
//! sense select the same finite interval after lifting.
use super::{
    geometry::{Curve, Edge, Surface, positive},
    *,
};
use cgmath::Vector2;
use std::f64::consts::TAU;

pub(super) struct Carrier {
    pub(super) supports: [u64; 2],
    curves: [Curve; 2],
    seam_gap: Option<f64>,
}

impl Carrier {
    pub(super) fn read(
        records: &SourceRecords<'_>,
        scene: &scene::Scene<'_>,
        id: u64,
        kind: &str,
    ) -> Result<(u64, Self), String> {
        let params = named_parameters(records, id, kind, 4)?;
        if !matches!(&params[3], Parameter::Enumeration(master) if master == "PCURVE_S1") {
            return Err("unsupported native surface curve master".into());
        }
        let [a, b] = references(&params[2])?[..] else {
            return Err("native surface curve requires two distinct pcurves".into());
        };
        let (s1, c1, u1) = lift_pcurve(records, scene, a)?;
        let (s2, c2, u2) = lift_pcurve(records, scene, b)?;
        let seam_gap = match (kind, s1 == s2, u1.zip(u2)) {
            ("SURFACE_CURVE", false, _) => None,
            ("SEAM_CURVE", true, Some(((angle1, radius), (angle2, _)))) => {
                Some(((angle1 - angle2).abs() - TAU).abs() * radius)
            }
            _ => return Err("invalid native surface curve associations".into()),
        };
        Ok((
            reference(&params[1])?,
            Self {
                supports: [s1, s2],
                curves: [c1, c2],
                seam_gap,
            },
        ))
    }

    pub(super) fn validate(&self, edge: &Edge, epsilon: f64) -> Result<(), String> {
        if !self.seam_gap.is_none_or(|gap| gap <= epsilon) {
            return Err("native seam pcurves are not one period apart".into());
        }
        for pcurve in &self.curves {
            let residual = match (&edge.curve, pcurve) {
                (
                    Curve::Line { direction, .. },
                    Curve::Line {
                        origin,
                        direction: lifted,
                        ..
                    },
                ) if direction.dot(*lifted) > 0.0 => {
                    let [a, b] = edge.points;
                    // Distance to a line is convex along the finite trim.
                    (a - origin).cross(*lifted).magnitude()
                        + (b - origin).cross(*lifted).magnitude()
                        + (b - a).magnitude() * direction.cross(*lifted).magnitude()
                }
                (
                    Curve::Circle {
                        frame,
                        radius,
                        forward,
                    },
                    Curve::Circle {
                        frame: lifted,
                        radius: lifted_radius,
                        ..
                    },
                ) if frame.z.dot(lifted.z) > 0.0 => {
                    let (_, sweep) = edge.interval(*frame, *radius, *forward, epsilon)?;
                    let (_, lifted_sweep) = edge
                        .interval(*lifted, *lifted_radius, *forward, epsilon)
                        .map_err(|_| "native pcurve disagrees with its 3D carrier")?;
                    (frame.w - lifted.w).magnitude()
                        + (radius - lifted_radius).abs()
                        + radius * (frame.z - lifted.z).magnitude()
                        + radius * (sweep - lifted_sweep).abs()
                }
                _ => return Err("native pcurve and 3D carrier have incompatible geometry".into()),
            };
            if !residual.is_finite() || residual > epsilon {
                return Err("native pcurve disagrees with its 3D carrier".into());
            }
        }
        Ok(())
    }
}

// The restricted cylindrical chart returns the angle and radius of a generator
// for checking the two copies of a seam.
type Generator = (f64, f64);

fn lift_pcurve(
    records: &SourceRecords<'_>,
    scene: &scene::Scene<'_>,
    id: u64,
) -> Result<(u64, Curve, Option<Generator>), String> {
    let params = named_parameters(records, id, "PCURVE", 3)?;
    let support = reference(&params[1])?;
    let surface = Surface::read(simple_record(records, support)?, scene)?;
    let repr = named_parameters(
        records,
        reference(&params[2])?,
        "DEFINITIONAL_REPRESENTATION",
        3,
    )?;
    let [id] = references(&repr[1])?[..] else {
        return Err("native pcurve requires one definitional curve".into());
    };
    let context = records
        .get(&reference(&repr[2])?)
        .ok_or("missing native pcurve context")?;
    let mut dimension = None;
    let mut parametric = false;
    let mut representation = false;
    for record in context {
        match record.name.as_str() {
            "GEOMETRIC_REPRESENTATION_CONTEXT" if dimension.is_none() => {
                dimension = Some(&parameters(record, 1)?[0]);
            }
            "PARAMETRIC_REPRESENTATION_CONTEXT" if !parametric => {
                parameters(record, 0)?;
                parametric = true;
            }
            "REPRESENTATION_CONTEXT" if !representation => {
                if !parameters(record, 2)?
                    .iter()
                    .all(|p| matches!(p, Parameter::String(_)))
                {
                    return Err("invalid native pcurve context label".into());
                }
                representation = true;
            }
            _ => return Err("unsupported native pcurve context".into()),
        }
    }
    if !matches!(dimension, Some(Parameter::Integer(2))) || !parametric || !representation {
        return Err("native pcurve requires a two-dimensional parametric context".into());
    }
    let frame = surface.frame;
    let kind = simple_record(records, id)?.name.as_str();
    let curve = named_parameters(records, id, kind, 3)?;
    let (lifted, generator) = match (kind, surface.radius) {
        ("LINE", radius) => {
            let origin = vector_2d(records, &curve[1], "CARTESIAN_POINT")?;
            let vector = named_parameters(records, reference(&curve[2])?, "VECTOR", 3)?;
            positive(&vector[2])?;
            let direction = vector_2d(records, &vector[1], "DIRECTION")?;
            if !direction.magnitude2().is_finite() || direction.magnitude2() == 0.0 {
                return Err("invalid native pcurve direction".into());
            }
            let direction = direction.normalize();
            match radius {
                None => (
                    Curve::Line {
                        origin: frame.transform_point(Point3::new(origin.x, origin.y, 0.0)),
                        direction: frame.transform_vector(direction.extend(0.0)),
                        forward: true,
                    },
                    None,
                ),
                // The cylindrical chart is (azimuth in radians, axial length).
                Some(radius) if direction.x == 0.0 => (
                    Curve::Line {
                        origin: frame.transform_point(Point3::new(
                            radius * origin.x.cos(),
                            radius * origin.x.sin(),
                            origin.y,
                        )),
                        direction: frame.z.truncate() * direction.y,
                        forward: true,
                    },
                    Some((origin.x, radius)),
                ),
                Some(radius) if direction.y == 0.0 => {
                    let mut frame =
                        frame * Matrix4::from_translation(Vector3::new(0.0, 0.0, origin.y));
                    let sense = direction.x.signum();
                    frame.y *= sense;
                    frame.z *= sense;
                    (
                        Curve::Circle {
                            frame,
                            radius,
                            forward: true,
                        },
                        None,
                    )
                }
                Some(_) => return Err("native cylindrical pcurve is not a chart axis".into()),
            }
        }
        ("CIRCLE", None) => {
            let axes = named_parameters(records, reference(&curve[1])?, "AXIS2_PLACEMENT_2D", 3)?;
            let origin = vector_2d(records, &axes[1], "CARTESIAN_POINT")?;
            // A valid planar reference direction changes only the circle's parameter origin.
            if !matches!(axes[2], Parameter::NotProvided) {
                let x = vector_2d(records, &axes[2], "DIRECTION")?;
                if !x.magnitude2().is_finite() || x.magnitude2() == 0.0 {
                    return Err("invalid native planar pcurve placement".into());
                }
            }
            (
                Curve::Circle {
                    frame: frame * Matrix4::from_translation(origin.extend(0.0)),
                    radius: positive(&curve[2])?,
                    forward: true,
                },
                None,
            )
        }
        _ => return Err(format!("unsupported native pcurve {kind}")),
    };
    Ok((support, lifted, generator))
}

fn vector_2d(
    records: &SourceRecords<'_>,
    parameter: &Parameter,
    kind: &str,
) -> Result<Vector2<f64>, String> {
    let params = named_parameters(records, reference(parameter)?, kind, 2)?;
    if let Parameter::List(values) = &params[1]
        && let [x, y] = values.as_slice()
    {
        return Ok(Vector2::new(scene::real(x)?, scene::real(y)?));
    }
    Err(format!("native pcurve {kind} must be two-dimensional"))
}
