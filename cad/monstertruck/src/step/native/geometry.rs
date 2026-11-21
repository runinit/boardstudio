//! Analytic edge intervals and surface frames in authored length units.
use super::*;
use std::f64::consts::TAU;

#[derive(Clone, Copy)]
pub(super) struct Surface {
    pub(super) frame: Matrix4,
    pub(super) radius: Option<f64>,
}

impl Surface {
    pub(super) fn read(record: &Record, scene: &scene::Scene<'_>) -> Result<Self, String> {
        let params = match record.name.as_str() {
            "PLANE" => parameters(record, 2)?,
            "CYLINDRICAL_SURFACE" => parameters(record, 3)?,
            _ => return Err(format!("unsupported native surface {}", record.name)),
        };
        if !matches!(params[0], Parameter::String(_)) {
            return Err("invalid surface label".into());
        }
        Ok(Self {
            frame: scene.frame(reference(&params[1])?, 1.0)?,
            radius: params.get(2).map(positive).transpose()?,
        })
    }

    pub(super) fn local(self, point: Point3) -> Vector3<f64> {
        let delta = point - point_from_vector(self.frame.w.truncate());
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
}

impl Edge {
    pub(super) fn read(
        id: u64,
        records: &SourceRecords<'_>,
        scene: &scene::Scene<'_>,
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
        let curve_id = reference(&edge[3])?;
        let kind = simple_record(records, curve_id)?.name.as_str();
        let params = named_parameters(records, curve_id, kind, 3)?;
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
            _ => return Err(format!("unsupported native edge {kind}")),
        };
        Ok(Self {
            vertices,
            points,
            curve,
        })
    }

    fn interval(
        &self,
        frame: Matrix4,
        radius: f64,
        forward: bool,
        epsilon: f64,
    ) -> Result<(f64, f64), String> {
        let chart = Surface {
            frame,
            radius: Some(radius),
        };
        let local = self.points.map(|p| chart.local(p));
        for p in local {
            if p.z.hypot(p.x.hypot(p.y) - radius) > epsilon {
                return Err("source boundary vertex is off its circle".into());
            }
        }
        let [start, end] = local.map(|p| p.y.atan2(p.x));
        let sense = if forward { 1.0 } else { -1.0 };
        if self.vertices[0] == self.vertices[1] {
            return Ok((start, sense * TAU));
        }
        let sweep = ((end - start) * sense).rem_euclid(TAU);
        if self.points[0] == self.points[1] || sweep <= 0.0 || sweep >= TAU {
            return Err("ambiguous authored circle interval".into());
        }
        Ok((start, sense * sweep))
    }

    pub(super) fn sample(&self, deflection: f64, epsilon: f64) -> Result<Vec<Point3>, String> {
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
                // Stable inversion of the circular chord sagitta.
                let limit = (4.0 * (deflection / (2.0 * radius)).min(1.0).sqrt().asin())
                    .min(ANGULAR_TOLERANCE);
                let count = (sweep.abs() / limit).ceil() as usize;
                if !(1..=4096).contains(&count) {
                    return Err("native circle subdivision limit".into());
                }
                let mut points = Vec::with_capacity(count + 1);
                points.push(self.points[0]);
                let chart = frame * Matrix4::from_scale(radius);
                points.extend(
                    (1..count)
                        .map(|i| circle_point(chart, start + sweep * i as f64 / count as f64)),
                );
                points.push(self.points[1]);
                Ok(points)
            }
        }
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
        if let Curve::Circle {
            frame,
            radius,
            forward,
        } = self.curve
        {
            let (start, sweep) = self.interval(frame, radius, forward, epsilon)?;
            let chart = world * frame * Matrix4::from_scale(radius);
            let (lo, hi) = (start.min(start + sweep), start.max(start + sweep));
            let mut include = |angle| -> Result<(), String> {
                let point = circle_point(chart, angle);
                check_coordinate(point)?;
                super::super::bounds::push(bounds, point)
            };
            include(lo)?;
            include(hi)?;
            for axis in 0..3 {
                for angle in
                    super::super::bounds::ellipse_extrema(chart.x[axis], chart.y[axis], lo, hi)
                {
                    include(angle)?;
                }
            }
        }
        Ok(())
    }
}

fn point_from_vector(v: Vector3<f64>) -> Point3 {
    Point3::new(v.x, v.y, v.z)
}

fn positive(parameter: &Parameter) -> Result<f64, String> {
    let value = scene::real(parameter)?;
    if value <= 0.0 {
        return Err("expected positive analytic geometry dimension".into());
    }
    Ok(value)
}

fn circle_point(frame: Matrix4, angle: f64) -> Point3 {
    // Keep the endpoint and extremum evaluations identical.
    frame.transform_point(Point3::new(angle.cos(), angle.sin(), 0.0))
}
