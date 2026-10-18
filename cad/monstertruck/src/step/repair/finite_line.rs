//! Exact, bidirectional bounds for the converter's finite native LINE operation.
//! This does not authenticate a face trim or any subsequent meshing operation.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

type Point = [Q; 3];

pub(super) struct Source {
    points: [Point; 2],
    origin: Point,
    direction: Point,
    same_sense: bool,
}

pub(super) struct Movement {
    pub occurrence: usize,
    pub shell: usize,
    pub edge: u64,
    pub bound_mm: f64,
    pub numeric_bound_mm: f64,
    pub projected_attachment: bool,
    squared_mm: Q,
}

impl Movement {
    pub fn within(&self, allowance: f64) -> bool {
        Q::from_float(allowance)
            .filter(|value| value.is_positive())
            .is_some_and(|value| self.squared_mm <= &value * &value)
    }
}

fn decimal(literal: &str) -> Option<Q> {
    if literal.len() > 80 {
        return None;
    }
    let (mantissa, exponent) = literal.split_once(['e', 'E']).unwrap_or((literal, "0"));
    let exponent: i32 = exponent.parse().ok()?;
    if !(-64..=64).contains(&exponent) {
        return None;
    }
    let fraction = mantissa.split_once('.').map_or(0, |(_, tail)| tail.len());
    let digits = mantissa.replace('.', "");
    let power = exponent - i32::try_from(fraction).ok()?;
    let value = if power >= 0 {
        format!("{}{}", digits, "0".repeat(power as usize))
    } else {
        format!("{digits}/1{}", "0".repeat((-power) as usize))
    };
    value.parse().ok()
}

fn numeric_values(mut entity: &str) -> Option<Vec<Q>> {
    let mut values = Vec::new();
    values.try_reserve_exact(3).ok()?;
    while !entity.is_empty() {
        if let Ok((rest, _)) = parser::combinator::separator(entity) {
            entity = rest;
        } else if entity.starts_with('\'') {
            entity = parser::token::string(entity).ok()?.0;
        } else if entity.starts_with('#') {
            entity = parser::token::entity_instance_name(entity).ok()?.0;
        } else if entity.as_bytes()[0].is_ascii_uppercase() {
            entity = parser::token::standard_keyword(entity).ok()?.0;
        } else if matches!(entity.as_bytes()[0], b'+' | b'-' | b'.' | b'0'..=b'9') {
            if values.len() == 3 {
                return None;
            }
            let rest = parser::token::real(entity)
                .map(|(rest, _)| rest)
                .or_else(|_| parser::token::integer(entity).map(|(rest, _)| rest))
                .ok()?;
            // The outer parser has already sealed this entity span. Failure is
            // local to its source ID; other complete authority survives.
            values.push(decimal(&entity[..entity.len() - rest.len()])?);
            entity = rest;
        } else if matches!(entity.as_bytes()[0], b'(' | b')' | b',' | b'=' | b';') {
            entity = &entity[1..];
        } else {
            return None;
        }
    }
    Some(values)
}

// Part21's own parsers delimit instances/tokens; strings/comments/references never
// become coordinate authority. Keep the original numeric spelling, not AST f64.
fn numbers(bytes: &[u8], needed: &HashSet<u64>) -> Option<HashMap<u64, Vec<Q>>> {
    let text = units::decode_step_bytes(bytes);
    let mut input = text.as_str();
    let mut result = HashMap::new();
    let mut seen = HashSet::new();
    while !input.is_empty() {
        if let Ok((rest, _)) = parser::combinator::separator(input) {
            input = rest;
        } else if input.starts_with('\'') {
            input = parser::token::string(input).ok()?.0;
        } else if input.starts_with('#') {
            let (rest, instance) = parser::exchange::entity_instance(input).ok()?;
            let (id, record) = match instance {
                EntityInstance::Simple { id, record } => (id, Some(record)),
                EntityInstance::Complex { id, .. } => (id, None),
            };
            let wanted = needed.contains(&id);
            let unique = !wanted || seen.insert(id);
            if wanted && !unique {
                result.remove(&id);
            }
            if wanted
                && unique
                && let Some(record) = record
                && matches!(
                    record.name.as_str(),
                    "CARTESIAN_POINT" | "DIRECTION" | "VECTOR"
                )
                && let Some(values) = numeric_values(&input[..input.len() - rest.len()])
            {
                result.insert(id, values);
            }
            input = rest;
        } else {
            input = &input[input.chars().next()?.len_utf8()..];
        }
    }
    Some(result)
}

pub(super) fn capture(table: &Table, bytes: &[u8]) -> HashMap<u64, Source> {
    let mut needed = HashSet::new();
    for edge in table.edge_curve.values() {
        let Some(line) = entity_id(&edge.edge_geometry).and_then(|id| table.line.get(&id)) else {
            continue;
        };
        if let Some(id) = entity_id(&line.pnt) {
            needed.insert(id);
        }
        if let Some(id) = entity_id(&line.dir) {
            needed.insert(id);
            if let Some(vector) = table.vector.get(&id)
                && let Some(id) = entity_id(&vector.orientation)
            {
                needed.insert(id);
            }
        }
        for vertex in [&edge.edge_start, &edge.edge_end] {
            if let Some(vertex) = entity_id(vertex).and_then(|id| table.vertex_point.get(&id))
                && let Some(id) = entity_id(&vertex.vertex_geometry)
            {
                needed.insert(id);
            }
        }
    }
    if needed.is_empty() {
        return HashMap::new();
    }
    let Some(numbers) = numbers(bytes, &needed) else {
        return HashMap::new();
    };
    let point = |id| -> Option<Point> { numbers.get(&id)?.clone().try_into().ok() };
    let vertex = |holder: &PlaceHolder<VertexPointHolder>| -> Option<Point> {
        point(entity_id(
            &table.vertex_point.get(&entity_id(holder)?)?.vertex_geometry,
        )?)
    };
    table
        .edge_curve
        .iter()
        .filter_map(|(&id, edge)| {
            let source = table.line.get(&entity_id(&edge.edge_geometry)?)?;
            let vector_id = entity_id(&source.dir)?;
            let vector = table.vector.get(&vector_id)?;
            let [magnitude] = numbers.get(&vector_id)?.as_slice() else {
                return None;
            };
            let direction = point(entity_id(&vector.orientation)?)?.map(|value| value * magnitude);
            (!dot(&direction, &direction).is_zero()).then_some((
                id,
                Source {
                    points: [vertex(&edge.edge_start)?, vertex(&edge.edge_end)?],
                    origin: point(entity_id(&source.pnt)?)?,
                    direction,
                    same_sense: edge.same_sense,
                },
            ))
        })
        .collect()
}

fn dot(a: &Point, b: &Point) -> Q {
    (0..3).map(|i| &a[i] * &b[i]).sum()
}
fn sub(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|i| &a[i] - &b[i])
}
fn rational_point(p: Point3) -> Option<Point> {
    Some([
        Q::from_float(p.x)?,
        Q::from_float(p.y)?,
        Q::from_float(p.z)?,
    ])
}

fn world_square(
    before: &Point,
    after: &Point,
    factor: f64,
    placement: Matrix4,
) -> Result<Q, &'static str> {
    if !(0..4).all(|col| (0..4).all(|row| placement[col][row].is_finite()))
        || placement.x.w != 0.0
        || placement.y.w != 0.0
        || placement.z.w != 0.0
        || placement.w.w != 1.0
    {
        return Err("nonfinite or non-affine finite LINE occurrence");
    }
    let factor = Q::from_float(factor)
        .filter(|f| f.is_positive())
        .ok_or("invalid finite LINE unit transform")?;
    let delta = sub(after, before);
    let mut world = std::array::from_fn(|_| Q::zero());
    for row in 0..3 {
        for col in 0..3 {
            let coefficient = Q::from_float(placement[col][row])
                .ok_or("invalid finite LINE occurrence transform")?;
            world[row] += coefficient * &factor * &delta[col];
        }
    }
    Ok(dot(&world, &world))
}

impl Source {
    fn projected_controls(&self) -> [Point; 2] {
        let project = |point: &Point| {
            let parameter = dot(&sub(point, &self.origin), &self.direction)
                / dot(&self.direction, &self.direction);
            std::array::from_fn(|i| &self.origin[i] + &self.direction[i] * &parameter)
        };
        self.points.each_ref().map(project)
    }

    // Refusal ONLY: even one authenticated attachment point over budget disproves
    // compliance. This result never supplies a complete movement certificate.
    pub(super) fn projected_exceeds(
        &self,
        factor: f64,
        placement: Matrix4,
        allowance: f64,
    ) -> Result<bool, &'static str> {
        let budget = Q::from_float(allowance)
            .filter(|q| q.is_positive())
            .ok_or("invalid projected LINE allowance")?;
        for (before, after) in self.points.iter().zip(self.projected_controls()) {
            if world_square(before, &after, factor, placement)? > &budget * &budget {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn movement(
        &self,
        line: &monstertruck_modeling::Line<Point3>,
        parsed_vertices: [Point3; 2],
        factor: f64,
        placement: Matrix4,
        identity: (usize, usize, u64),
    ) -> Result<Movement, &'static str> {
        if [line.0, line.1] != parsed_vertices {
            return Err("projected finite LINE route requires its conversion-time domain witness");
        }
        self.measure(line, factor, placement, identity, false)
    }

    pub(super) fn projected_movement(
        &self,
        line: &monstertruck_modeling::Line<Point3>,
        parsed_vertices: [Point3; 2],
        change: monstertruck_io::step::load::convert::StepRepairCurveChange<'_>,
        factor: f64,
        placement: Matrix4,
        identity: (usize, usize, u64),
    ) -> Result<Movement, &'static str> {
        let (Curve3D::Line(before), Curve3D::Line(after)) = change.curves() else {
            return Err("projected attachment carriers are not finite affine LINEs");
        };
        let expected_before = if self.same_sense {
            parsed_vertices
        } else {
            [parsed_vertices[1], parsed_vertices[0]]
        };
        let expected_after = if self.same_sense {
            [line.0, line.1]
        } else {
            [line.1, line.0]
        };
        if change.domains() != ((0., 1.), (0., 1.))
            || [before.0, before.1] != expected_before
            || [after.0, after.1] != expected_after
        {
            return Err("projected attachment source/domain/output correspondence mismatch");
        }
        self.measure(line, factor, placement, identity, true)
    }

    fn measure(
        &self,
        line: &monstertruck_modeling::Line<Point3>,
        factor: f64,
        placement: Matrix4,
        identity: (usize, usize, u64),
        projected_attachment: bool,
    ) -> Result<Movement, &'static str> {
        if line.0 == line.1 {
            return Err("repaired finite LINE interval is degenerate");
        }
        let projected = self.projected_controls();
        if !projected_attachment {
            // This is native source-interval authority only. Attachment
            // projection remains well-defined when that selection is unproved.
            let advance = dot(&sub(&projected[1], &projected[0]), &self.direction);
            if advance.is_zero() || advance.is_positive() != self.same_sense {
                return Err("original finite LINE interval is degenerate or contradicts its sense");
            }
        }
        let (before, after) = if projected_attachment {
            (&self.points, &projected)
        } else {
            (&projected, &self.points)
        };
        // Complete affine [0,1] correspondence in both directions. The original
        // chord branch and attachment-projection branch are opposite operations.
        let squared = before
            .iter()
            .zip(after)
            .map(|(a, b)| world_square(a, b, factor, placement))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap();
        // Ideal target -> ACTUAL emitted binary64 controls is numerical error,
        // separately retained; never charged as geometric repair.
        let numeric_squared = after
            .iter()
            .zip([line.0, line.1])
            .map(|(a, b)| {
                world_square(
                    a,
                    &rational_point(b).ok_or("nonfinite repaired LINE")?,
                    factor,
                    placement,
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap();
        let (occurrence, shell, edge) = identity;
        Ok(Movement {
            occurrence,
            shell,
            edge,
            bound_mm: sqrt_upper(&squared, "finite LINE movement range")?,
            numeric_bound_mm: sqrt_upper(&numeric_squared, "finite LINE movement range")?,
            squared_mm: squared,
            projected_attachment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_or_ambiguous_scalar_entities_do_not_erase_other_authority() {
        let clear = "#1=CARTESIAN_POINT('',(0.,1.,2.));";
        for other in [
            "#2=DIRECTION('',(1.E-100,1.,0.));",
            "#2=DIRECTION('',(0.,1.,0.,4.));",
            "#2=DIRECTION('',(.F.,1.,0.));",
            "#2=DIRECTION('',(0.,1.,0.));#2=VECTOR('',#1,1.);",
        ] {
            let source = format!("{clear}{other}");
            let authority = numbers(source.as_bytes(), &HashSet::from([1, 2])).unwrap();
            assert_eq!(
                authority[&1],
                vec![
                    Q::zero(),
                    Q::from_integer(1.into()),
                    Q::from_integer(2.into())
                ]
            );
            assert!(!authority.contains_key(&2));
        }
    }
}
