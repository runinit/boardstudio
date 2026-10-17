//! Isolated qualification module for the reviewed bounded dish cutter.
//!
//! Geometry and refusal behavior are lifted from the reviewed v3 constructor.
//! Profile dimensions are in millimeters and tilt is in degrees, matching the
//! current numeric caller. This module preserves that caller's input scope; it
//! does not qualify arbitrary scales or act as a general modeling API.

use monstertruck_geometry::prelude::Sphere;
use monstertruck_modeling::Processor;
use monstertruck_modeling::{
    Curve, Face, Point3, Rad, Shell, Solid, Surface, Vector3, Wire, builder,
};
use std::collections::BTreeMap;

mod certification;

const BOOLEAN_TOLERANCE: f64 = 1.0e-6;

/// Profile data consumed by the bounded cutter constructor and support proof.
/// `label` and `wall` are carried from the current qualification caller; cutter
/// geometry depends on the other profile fields exactly as in the reviewed
/// source. Linear values use millimeters and `tilt` uses degrees.
#[derive(Clone, Copy)]
pub struct Profile {
    /// Current-caller diagnostic label.
    pub label: &'static str,
    /// Base width in millimeters.
    pub width: f64,
    /// Base depth in millimeters.
    pub depth: f64,
    /// Top width in millimeters.
    pub top_w: f64,
    /// Top depth in millimeters.
    pub top_d: f64,
    /// Cap height in millimeters.
    pub height: f64,
    /// Cap tilt about X in degrees.
    pub tilt: f64,
    /// Dish depth in millimeters; zero requests no cutter.
    pub dish_depth: f64,
    /// Wall thickness in millimeters, carried but not used by cutter geometry.
    pub wall: f64,
    /// Select the spherical (`true`) or cylindrical (`false`) cutter primitive.
    pub spherical: bool,
}

#[cfg(test)]
fn profile(case: &str) -> Result<Profile, String> {
    match case {
        "dsa" => Ok(Profile {
            label: "1u-dsa-spherical",
            width: 18.2,
            depth: 18.2,
            top_w: 13.2,
            top_d: 13.2,
            height: 7.5,
            tilt: 0.0,
            dish_depth: 0.65,
            wall: 1.2,
            spherical: true,
        }),
        "oem" => Ok(Profile {
            label: "1.5u-oem-row5-cylindrical-tilted",
            width: 27.725,
            depth: 18.2,
            top_w: 22.725,
            top_d: 13.2,
            height: 11.6,
            tilt: 8.0,
            dish_depth: 0.65,
            wall: 1.2,
            spherical: false,
        }),
        other => Err(format!("unknown case {other:?}; use dsa or oem")),
    }
}

fn sampled_wire(width: f64, depth: f64, z: f64, tilt_deg: f64) -> Wire {
    let r = 0.8_f64.min(width / 5.0).min(depth / 5.0);
    let mut points = Vec::with_capacity(20);
    for (x, y, start) in [
        (width / 2.0 - r, depth / 2.0 - r, 0.0_f64),
        (-width / 2.0 + r, depth / 2.0 - r, 90.0),
        (-width / 2.0 + r, -depth / 2.0 + r, 180.0),
        (width / 2.0 - r, -depth / 2.0 + r, 270.0),
    ] {
        for i in 0..=4 {
            let angle = (start + f64::from(i) * 22.5).to_radians();
            let px = x + r * angle.cos();
            let py = y + r * angle.sin();
            points.push(Point3::new(px, py, z + py * tilt_deg.to_radians().tan()));
        }
    }
    let vertices: Vec<_> = points.iter().copied().map(builder::vertex).collect();
    (0..vertices.len())
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % vertices.len()]))
        .collect::<Vec<_>>()
        .into()
}

fn dish_radius(p: Profile) -> f64 {
    let half = p.top_d / 2.0;
    (half * half + p.dish_depth * p.dish_depth) / (2.0 * p.dish_depth)
}

fn x_cylinder_fallible(radius: f64, length: f64) -> Result<Solid, String> {
    let point = builder::vertex(Point3::new(0.0, 0.0, radius));
    let circle = builder::revolve(
        &point,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_x(),
        builder::SweepAngle::Closed,
        2,
    );
    let disk: Face =
        builder::try_attach_plane(vec![circle]).map_err(|e| format!("cylinder cap: {e:?}"))?;
    let solid: Solid = builder::extrude(&disk, Vector3::unit_x() * length);
    Solid::try_new(solid.boundaries().to_vec()).map_err(|e| format!("cylinder solid: {e:?}"))
}

// The loft is a union of bilinear patches, hence inside its control hull.
// The cap after cavity subtraction is a subset of that hull.
/// Return the conservative ceiling used to truncate the finite dish primitive.
/// The returned projected height is in millimeters.
pub fn cutter_ceiling(p: Profile) -> f64 {
    let (sn, cs) = p.tilt.to_radians().sin_cos();
    let mut upper = 0.0_f64;
    for wire in [
        sampled_wire(p.width, p.depth, 0.0, 0.0),
        sampled_wire(p.top_w, p.top_d, p.height, p.tilt),
    ] {
        for e in wire.edge_iter() {
            let point = e.front().point();
            let z = (point.z - p.height).next_up();
            let q = ((-sn * point.y).next_up() + (cs * z).next_up()).next_up();
            upper = upper.max(q);
        }
    }
    // The separation is geometric overrun; it does not change Boolean tolerance.
    (upper + 0.01).next_up()
}

fn spherical_chart(radius: f64, middle: Vector3, pole: Vector3) -> Surface {
    use monstertruck_modeling::{InnerSpace, Matrix4, Vector4};
    let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), radius);
    let x = -middle.normalize();
    let z = pole.normalize();
    let y = z.cross(x);
    let processor = Processor::with_transform(
        sphere,
        Matrix4::from_cols(
            x.extend(0.0),
            y.extend(0.0),
            z.extend(0.0),
            Vector4::new(0.0, 0.0, radius, 1.0),
        ),
    );
    Surface::SphericalSurface(processor)
}

fn analytic_full_sphere(radius: f64) -> Result<Solid, String> {
    use monstertruck_modeling::InnerSpace;
    let points = [
        Vector3::unit_x(),
        -Vector3::unit_x(),
        Vector3::unit_y(),
        -Vector3::unit_y(),
        Vector3::unit_z(),
        -Vector3::unit_z(),
    ];
    let vertices: Vec<_> = points
        .iter()
        .map(|p| {
            builder::vertex(Point3::new(
                p.x * radius,
                p.y * radius,
                (p.z + 1.0) * radius,
            ))
        })
        .collect();
    let mut arcs = BTreeMap::<(usize, usize), monstertruck_modeling::Edge>::new();
    let mut shell = Shell::default();
    for ix in 0..2 {
        for iy in 2..4 {
            for iz in 4..6 {
                let middle = points[ix] + points[iy] + points[iz];
                let mut ids = [ix, iy, iz];
                if points[ix].dot(points[iy].cross(points[iz])) < 0.0 {
                    ids.swap(1, 2);
                }
                let mut edges = Vec::new();
                for k in 0..3 {
                    let (a, b) = (ids[k], ids[(k + 1) % 3]);
                    let key = (a.min(b), a.max(b));
                    let edge = if let Some(edge) = arcs.get(&key) {
                        edge.clone()
                    } else {
                        let through = (points[key.0] + points[key.1]).normalize() * radius;
                        let edge = builder::try_circle_arc(
                            &vertices[key.0],
                            &vertices[key.1],
                            Point3::new(through.x, through.y, through.z + radius),
                        )
                        .map_err(|e| format!("sphere octant arc: {e:?}"))?;
                        arcs.insert(key, edge.clone());
                        edge
                    };
                    edges.push(if a < b { edge } else { edge.inverse() });
                }
                let pole = points[ix] - points[iy];
                shell.push(
                    Face::try_new(vec![edges.into()], spherical_chart(radius, middle, pole))
                        .map_err(|e| format!("sphere octant: {e:?}"))?,
                );
            }
        }
    }
    Solid::try_new(vec![shell]).map_err(|e| format!("analytic octant sphere: {e:?}"))
}

fn spherical_segment(radius: f64, height: f64) -> Result<Solid, String> {
    if height >= 2.0 * radius {
        return analytic_full_sphere(radius);
    }
    if !(height > 0.0 && height.is_finite()) {
        return Err("invalid sphere segment height".into());
    }
    let z = height;
    let radial = (height * (2.0 * radius - height)).sqrt();
    let angle = (radial / radius).atan2(1.0 - height / radius);
    let south = builder::vertex(Point3::new(0.0, 0.0, 0.0));
    let vertices: Vec<_> = (0..4)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::FRAC_PI_2;
            builder::vertex(Point3::new(radial * a.cos(), radial * a.sin(), z))
        })
        .collect();
    let arcs: Vec<_> = (0..4)
        .map(|i| {
            let a = (i as f64 + 0.5) * std::f64::consts::FRAC_PI_2;
            builder::try_circle_arc(
                &vertices[i],
                &vertices[(i + 1) % 4],
                Point3::new(radial * a.cos(), radial * a.sin(), z),
            )
            .map_err(|e| format!("sphere closure arc: {e:?}"))
        })
        .collect::<Result<_, _>>()?;
    let meridians: Vec<_> = (0..4)
        .map(|i| {
            let a = i as f64 * std::f64::consts::FRAC_PI_2;
            let mid_r = radius * (angle / 2.0).sin();
            builder::try_circle_arc(
                &south,
                &vertices[i],
                Point3::new(
                    mid_r * a.cos(),
                    mid_r * a.sin(),
                    2.0 * radius * (angle / 4.0).sin().powi(2),
                ),
            )
            .map_err(|e| format!("sphere meridian arc: {e:?}"))
        })
        .collect::<Result<_, _>>()?;
    let mut shell = Shell::default();
    for i in 0..4 {
        let a = (i as f64 + 0.5) * std::f64::consts::FRAC_PI_2;
        let middle = Vector3::new(
            (angle / 2.0).sin() * a.cos(),
            (angle / 2.0).sin() * a.sin(),
            -(angle / 2.0).cos(),
        );
        let pole = Vector3::new(-a.sin(), a.cos(), 0.0);
        let wire: Wire = vec![
            meridians[(i + 1) % 4].clone(),
            arcs[i].inverse(),
            meridians[i].inverse(),
        ]
        .into();
        shell.push(
            Face::try_new(vec![wire], spherical_chart(radius, middle, pole))
                .map_err(|e| format!("sphere sector: {e:?}"))?,
        );
    }
    shell.push(
        builder::try_attach_plane(vec![arcs.into()])
            .map_err(|e| format!("sphere cap closure: {e:?}"))?,
    );
    Solid::try_new(vec![shell]).map_err(|e| format!("closed analytic sphere segment: {e:?}"))
}

fn cylindrical_segment(radius: f64, height: f64, length: f64) -> Result<Solid, String> {
    if height >= 2.0 * radius {
        let cylinder = builder::translated(
            &x_cylinder_fallible(radius, length)?,
            Vector3::new(0.0, 0.0, radius),
        );
        return Ok(cylinder);
    }
    if !(height > 0.0 && height.is_finite()) {
        return Err("invalid cylinder segment height".into());
    }
    let z = height;
    let radial = (height * (2.0 * radius - height)).sqrt();
    let left = builder::vertex(Point3::new(0.0, -radial, z));
    let right = builder::vertex(Point3::new(0.0, radial, z));
    let arc = builder::try_circle_arc(&left, &right, Point3::new(0.0, 0.0, 0.0))
        .map_err(|e| format!("cylinder segment arc: {e:?}"))?;
    let wire: Wire = vec![arc, builder::line(&right, &left)].into();
    let disk = builder::try_attach_plane(vec![wire])
        .map_err(|e| format!("cylinder segment disk: {e:?}"))?;
    let solid: Solid = builder::extrude(&disk, Vector3::unit_x() * length);
    Solid::try_new(solid.boundaries().to_vec())
        .map_err(|e| format!("cylinder segment solid: {e:?}"))
}

/// Build and certify a finite dish cutter for `p`.
///
/// A zero dish depth is a no-op (`Ok(None)`). Positive depths that cannot be
/// certified at the unchanged Boolean tolerance return `Err`.
pub fn dish_cutter(p: Profile) -> Result<Option<Solid>, String> {
    if p.dish_depth == 0.0 {
        return Ok(None);
    }
    let radius = dish_radius(p);
    certification::certify_source_scale(p, radius)?;
    let ceiling = cutter_ceiling(p);
    let height = ceiling + p.dish_depth;
    if !radius.is_finite() || !(2.0 * radius).is_finite() || !height.is_finite() || height <= 0.0 {
        return Err("dish cutter cannot be represented at the requested scale".into());
    }
    let cutter = if p.spherical {
        spherical_segment(radius, height)?
    } else {
        builder::translated(
            &cylindrical_segment(radius, height, p.width * 2.0)?,
            Vector3::new(-p.width, 0.0, 0.0),
        )
    };
    // Keep the closure near the cap instead of subtracting and readding radius.
    let cutter = builder::translated(&cutter, Vector3::new(0.0, 0.0, p.height - p.dish_depth));
    let cutter = builder::rotated(
        &cutter,
        Point3::new(0.0, 0.0, p.height),
        Vector3::unit_x(),
        Rad(p.tilt.to_radians()),
    );
    certification::certify_cutter(p, &cutter, height < 2.0 * radius)?;
    Ok(Some(cutter))
}

#[cfg(test)]
mod tests;
