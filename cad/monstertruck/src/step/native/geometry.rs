//! Analytic edge intervals and surface frames in authored length units.
use super::carrier::Carrier;
use super::rational_trim::{self, FiniteEllipse};
use super::*;
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Copy)]
pub(super) struct Surface {
    pub(super) frame: Matrix4,
    pub(super) kind: SurfaceKind,
}

#[derive(Clone, Copy)]
pub(super) enum SurfaceKind {
    Plane,
    Cylinder(f64),
    Sphere(f64),
    Cone {
        radius: f64,
        slope: f64,
        radians_per_unit: f64,
    },
}

impl Surface {
    pub(super) fn read(
        record: &Record,
        scene: &scene::Scene<'_>,
        cone_angle: Option<f64>,
    ) -> Result<Self, String> {
        let (params, kind) = match record.name.as_str() {
            "PLANE" => (parameters(record, 2)?, SurfaceKind::Plane),
            "CYLINDRICAL_SURFACE" => {
                let params = parameters(record, 3)?;
                (params, SurfaceKind::Cylinder(positive(&params[2])?))
            }
            "SPHERICAL_SURFACE" => {
                let params = parameters(record, 3)?;
                (params, SurfaceKind::Sphere(positive(&params[2])?))
            }
            "CONICAL_SURFACE" => {
                let params = parameters(record, 4)?;
                let radians_per_unit = cone_angle.ok_or("missing native cone angle context")?;
                let radius = scene::real(&params[2])?;
                let angle = scene::real(&params[3])? * radians_per_unit;
                if radius < 0.0 || !(angle > 0.0 && angle < FRAC_PI_2) {
                    return Err("invalid native conical radius or semi-angle".into());
                }
                (
                    params,
                    SurfaceKind::Cone {
                        radius,
                        slope: angle.tan(),
                        radians_per_unit,
                    },
                )
            }
            _ => return Err(format!("unsupported native surface {}", record.name)),
        };
        if !matches!(params[0], Parameter::String(_)) {
            return Err("invalid surface label".into());
        }
        Ok(Self {
            frame: scene.frame(reference(&params[1])?, 1.0)?,
            kind,
        })
    }

    pub(super) fn local(self, point: Point3) -> Vector3<f64> {
        self.local_direction(point - point_from_vector(self.frame.w.truncate()))
    }

    pub(super) fn local_direction(self, delta: Vector3<f64>) -> Vector3<f64> {
        Vector3::new(
            delta.dot(self.frame.x.truncate()),
            delta.dot(self.frame.y.truncate()),
            delta.dot(self.frame.z.truncate()),
        )
    }
}

pub(super) struct Edge {
    pub(super) vertices: [u64; 2],
    pub(super) points: [Point3; 2],
    pub(super) curve: Curve,
    pub(super) carrier: Option<Box<Carrier>>,
    pub(super) finite_ellipse: Option<FiniteEllipse>,
}

pub(super) enum Curve {
    Line {
        origin: Point3,
        direction: Vector3<f64>,
        forward: bool,
    },
    Circle {
        frame: Matrix4,
        radius: f64,
        forward: bool,
    },
    Ellipse {
        frame: Matrix4,
        major: f64,
        minor: f64,
        forward: bool,
    },
}

impl Edge {
    pub(super) fn read(
        id: u64,
        records: &SourceRecords<'_>,
        scene: &scene::Scene<'_>,
        cone_angles: &HashMap<u64, f64>,
    ) -> Result<Self, String> {
        let edge = named_parameters(records, id, "EDGE_CURVE", 5)?;
        let vertices = [reference(&edge[1])?, reference(&edge[2])?];
        let point = |vertex| -> Result<Point3, String> {
            let vertex = named_parameters(records, vertex, "VERTEX_POINT", 2)?;
            Ok(point_from_vector(
                scene.vector(reference(&vertex[1])?, "CARTESIAN_POINT")?,
            ))
        };
        let points = [point(vertices[0])?, point(vertices[1])?];
        let forward = flag(&edge[4])?;
        let id = reference(&edge[3])?;
        if records.get(&id).is_some_and(|parts| parts.len() > 1) {
            let (curve, interval) = rational_trim::read(records, scene, id, forward)?;
            return Ok(Self {
                vertices,
                points,
                curve,
                carrier: None,
                finite_ellipse: Some(interval),
            });
        }
        let mut kind = simple_record(records, id)?.name.as_str();
        let (curve_id, carrier) = if matches!(kind, "SURFACE_CURVE" | "SEAM_CURVE") {
            let (id, carrier) = Carrier::read(records, scene, cone_angles, id, kind)?;
            kind = simple_record(records, id)?.name.as_str();
            (id, Some(Box::new(carrier)))
        } else {
            (id, None)
        };
        let params = named_parameters(
            records,
            curve_id,
            kind,
            if kind == "ELLIPSE" { 4 } else { 3 },
        )?;
        let curve = match kind {
            "LINE" => {
                let vector = named_parameters(records, reference(&params[2])?, "VECTOR", 3)?;
                positive(&vector[2])?;
                Curve::Line {
                    origin: point_from_vector(
                        scene.vector(reference(&params[1])?, "CARTESIAN_POINT")?,
                    ),
                    direction: scene.direction(&vector[1])?.ok_or("LINE lacks direction")?,
                    forward,
                }
            }
            "CIRCLE" => Curve::Circle {
                frame: scene.frame(reference(&params[1])?, 1.0)?,
                radius: positive(&params[2])?,
                forward,
            },
            "ELLIPSE" => {
                let major = positive(&params[2])?;
                let minor = positive(&params[3])?;
                if minor > major {
                    return Err("native ellipse semiaxes are out of order".into());
                }
                Curve::Ellipse {
                    frame: scene.frame(reference(&params[1])?, 1.0)?,
                    major,
                    minor,
                    forward,
                }
            }
            _ => return Err(format!("unsupported native edge {kind}")),
        };
        Ok(Self {
            vertices,
            points,
            curve,
            carrier,
            finite_ellipse: None,
        })
    }

    pub(super) fn interval(
        &self,
        frame: Matrix4,
        radius: f64,
        forward: bool,
        epsilon: f64,
    ) -> Result<(f64, f64), String> {
        let chart = Surface {
            frame,
            kind: SurfaceKind::Plane,
        };
        let local = self.points.map(|p| chart.local(p));
        for p in local {
            if p.z.hypot(p.x.hypot(p.y) - radius) > epsilon {
                return Err("source boundary vertex is off its circle".into());
            }
        }
        let [start, end] = local.map(|p| p.y.atan2(p.x));
        self.sweep(start, end, forward)
    }

    fn sweep(&self, start: f64, end: f64, forward: bool) -> Result<(f64, f64), String> {
        let sense = if forward { 1.0 } else { -1.0 };
        if self.vertices[0] == self.vertices[1] {
            return Ok((start, sense * TAU));
        }
        let sweep = ((end - start) * sense).rem_euclid(TAU);
        if self.points[0] == self.points[1] || sweep <= 0.0 || sweep >= TAU {
            return Err("ambiguous authored conic interval".into());
        }
        Ok((start, sense * sweep))
    }

    pub(super) fn ellipse(&self, epsilon: f64) -> Result<(Matrix4, f64, f64, f64), String> {
        let Curve::Ellipse {
            frame,
            major,
            minor,
            forward,
        } = self.curve
        else {
            return Err("expected an elliptical trim".into());
        };
        let chart = frame * Matrix4::from_nonuniform_scale(major, minor, 1.0);
        if let Some(trim) = &self.finite_ellipse {
            let (start, sweep) = trim.interval;
            let attachment = self
                .points
                .into_iter()
                .zip([start, start + sweep])
                .map(|(point, angle)| (point - conic_point(chart, angle)).magnitude())
                .fold(0.0, f64::max);
            let error = attachment + trim.residual;
            if self.vertices[0] == self.vertices[1] || !error.is_finite() || error > epsilon {
                return Err(
                    "native rational ellipse does not match its oriented finite endpoints".into(),
                );
            }
            return Ok((chart, major, start, sweep));
        }
        let local = Surface {
            frame,
            kind: SurfaceKind::Plane,
        };
        let [start, end] = self.points.map(|p| {
            let q = local.local(p);
            (q.y / minor).atan2(q.x / major)
        });
        for (point, angle) in self.points.into_iter().zip([start, end]) {
            let residual = (point - conic_point(chart, angle)).magnitude();
            if !residual.is_finite() || residual > epsilon {
                return Err("source boundary vertex is off its ellipse".into());
            }
        }
        let (_, sweep) = self.sweep(start, end, forward)?;
        Ok((chart, major, start, sweep))
    }

    pub(super) fn cylinder_sweep(
        &self,
        surface: Surface,
        radius: f64,
        epsilon: f64,
    ) -> Result<f64, String> {
        let Curve::Circle {
            frame,
            radius: edge_radius,
            forward,
        } = self.curve
        else {
            return Err("native cylinder lacks its circular trim".into());
        };
        let axis = surface.frame.z.truncate();
        let center = surface.local(point_from_vector(frame.w.truncate()));
        let radial_error = center.x.hypot(center.y)
            + (radius - edge_radius).abs()
            + radius * frame.z.truncate().cross(axis).magnitude();
        if radial_error > epsilon {
            return Err("source circular boundary is off its CYLINDRICAL_SURFACE".into());
        }
        let (_, sweep) = self.interval(frame, edge_radius, forward, epsilon)?;
        Ok(sweep * frame.z.truncate().dot(axis).signum())
    }

    pub(super) fn sample(&self, deflection: f64, epsilon: f64) -> Result<Vec<Point3>, String> {
        if let Some(carrier) = &self.carrier {
            carrier.validate(self, epsilon)?;
        }
        match self.curve {
            Curve::Line {
                origin,
                direction,
                forward,
            } => {
                let [a, b] = self.points.map(|p| p - origin);
                let chord = b - a;
                if a.cross(direction).magnitude() > epsilon
                    || b.cross(direction).magnitude() > epsilon
                    || (if forward { 1.0 } else { -1.0 }) * chord.dot(direction) <= 0.0
                {
                    return Err("source boundary is off its oriented LINE".into());
                }
                Ok(self.points.to_vec())
            }
            Curve::Circle {
                frame,
                radius,
                forward,
            } => {
                let (start, sweep) = self.interval(frame, radius, forward, epsilon)?;
                self.sample_conic(
                    frame * Matrix4::from_scale(radius),
                    radius,
                    start,
                    sweep,
                    deflection,
                )
            }
            Curve::Ellipse { .. } => {
                let (chart, major, start, sweep) = self.ellipse(epsilon)?;
                // The ellipse is a linear image of the unit circle with operator
                // norm equal to its major semiaxis, including on a finite arc.
                self.sample_conic(chart, major, start, sweep, deflection)
            }
        }
    }

    fn sample_conic(
        &self,
        chart: Matrix4,
        major: f64,
        start: f64,
        sweep: f64,
        deflection: f64,
    ) -> Result<Vec<Point3>, String> {
        let mut count = (sweep.abs() / circle_step(major, deflection)).ceil() as usize;
        let mut points = Vec::new();
        while (1..=4096).contains(&count) {
            points.clear();
            points.push(self.points[0]);
            points.extend(
                (1..count).map(|i| conic_point(chart, start + sweep * i as f64 / count as f64)),
            );
            points.push(self.points[1]);
            if !matches!(self.curve, Curve::Ellipse { .. }) {
                return Ok(points);
            }
            // Stretching a circle preserves the sagitta bound above, but can
            // amplify the angle between chords near the tip of an ellipse.
            let within_turn = |a: Point3, b: Point3, c: Point3| {
                let u = (b - a).normalize();
                let v = (c - b).normalize();
                u.dot(v) >= ANGULAR_TOLERANCE.cos()
            };
            let joins_fit = points.windows(3).all(|p| within_turn(p[0], p[1], p[2]));
            let seam_fits = self.vertices[0] != self.vertices[1]
                || within_turn(points[count - 1], points[0], points[1]);
            let spans_fit = points.windows(2).enumerate().all(|(i, p)| {
                let middle = conic_point(chart, start + sweep * (i as f64 + 0.5) / count as f64);
                within_turn(p[0], middle, p[1])
            });
            if joins_fit && seam_fits && spans_fit {
                return Ok(points);
            }
            if count == 4096 {
                break;
            }
            count = (2 * count).min(4096);
        }
        Err("native conic subdivision limit".into())
    }

    pub(super) fn include_bounds(
        &self,
        world: Matrix4,
        epsilon: f64,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<(), String> {
        for point in self.points {
            let point = world.transform_point(point);
            check_coordinate(point)?;
            super::super::bounds::push(bounds, point)?;
        }
        let (chart, start, sweep) = match self.curve {
            Curve::Line { .. } => return Ok(()),
            Curve::Circle {
                frame,
                radius,
                forward,
            } => {
                let (start, sweep) = self.interval(frame, radius, forward, epsilon)?;
                (world * frame * Matrix4::from_scale(radius), start, sweep)
            }
            Curve::Ellipse { .. } => {
                let (chart, _, start, sweep) = self.ellipse(epsilon)?;
                (world * chart, start, sweep)
            }
        };
        let (lo, hi) = (start.min(start + sweep), start.max(start + sweep));
        let reduction_margin = self.finite_ellipse.as_ref().map(|trim| {
            Vector3::new(
                Vector3::new(world.x.x, world.y.x, world.z.x).magnitude(),
                Vector3::new(world.x.y, world.y.y, world.z.y).magnitude(),
                Vector3::new(world.x.z, world.y.z, world.z.z).magnitude(),
            ) * trim.residual
        });
        let mut include = |angle| -> Result<(), String> {
            let point = conic_point(chart, angle);
            check_coordinate(point)?;
            if let Some(margin) = reduction_margin {
                // Enclose the original rational span as well as its reduction.
                super::super::bounds::push(bounds, point - margin)?;
                super::super::bounds::push(bounds, point + margin)
            } else {
                super::super::bounds::push(bounds, point)
            }
        };
        include(lo)?;
        include(hi)?;
        for axis in 0..3 {
            for angle in super::super::bounds::ellipse_extrema(chart.x[axis], chart.y[axis], lo, hi)
            {
                include(angle)?;
            }
        }
        Ok(())
    }
}

fn point_from_vector(v: Vector3<f64>) -> Point3 {
    Point3::new(v.x, v.y, v.z)
}

pub(super) fn positive(parameter: &Parameter) -> Result<f64, String> {
    let value = scene::real(parameter)?;
    if value <= 0.0 {
        return Err("expected positive analytic geometry dimension".into());
    }
    Ok(value)
}

fn conic_point(frame: Matrix4, angle: f64) -> Point3 {
    // Keep the endpoint and extremum evaluations identical.
    frame.transform_point(Point3::new(angle.cos(), angle.sin(), 0.0))
}

pub(super) fn circle_step(radius: f64, deflection: f64) -> f64 {
    // Stable inversion of the circular chord sagitta.
    (4.0 * (deflection / (2.0 * radius)).min(1.0).sqrt().asin()).min(ANGULAR_TOLERANCE)
}
