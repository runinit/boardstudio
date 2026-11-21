//! Exact, bidirectional bounds for the converter's finite native LINE operation.
//! This does not authenticate a face trim or any subsequent meshing operation.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, ToPrimitive, Zero};

type Point = [Q; 3];

pub(in crate::step) struct Source {
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
    let (negative, mantissa) = match mantissa.as_bytes().first() {
        Some(b'-') => (true, &mantissa[1..]),
        Some(b'+') => (false, &mantissa[1..]),
        _ => (false, mantissa),
    };
    let mut digits = Q::zero().to_integer();
    let mut count = 0;
    let mut dots = 0;
    for byte in mantissa.bytes() {
        if byte == b'.' {
            dots += 1;
            if dots > 1 {
                return None;
            }
        } else if byte.is_ascii_digit() {
            digits *= 10;
            digits += byte - b'0';
            count += 1;
        } else {
            return None;
        }
    }
    if count == 0 {
        return None;
    }
    let power = exponent - i32::try_from(fraction).ok()?;
    let digits = if negative { -digits } else { digits };
    Some(Q::from_integer(digits) * Q::from_integer(10.into()).pow(power))
}

struct Scalars {
    values: [Q; 3],
    len: usize,
}
impl Scalars {
    fn as_slice(&self) -> &[Q] {
        &self.values[..self.len]
    }
    fn point(&self) -> Option<Point> {
        (self.len == 3).then(|| self.values.clone())
    }
}

fn numeric_values(mut entity: &[u8]) -> Option<Scalars> {
    let mut values = Scalars {
        values: std::array::from_fn(|_| Q::zero()),
        len: 0,
    };
    while !entity.is_empty() {
        entity = spelling::separators(entity)?;
        if entity.is_empty() {
            break;
        }
        if entity.starts_with(b"'") {
            entity = spelling::string(entity)?;
        } else if entity.starts_with(b"#") {
            entity = spelling::reference(entity)?.0;
        } else if matches!(entity[0], b'+' | b'-' | b'0'..=b'9') {
            let slot = values.values.get_mut(values.len)?;
            let (rest, literal) = spelling::number(entity)?;
            *slot = decimal(literal)?;
            values.len += 1;
            entity = rest;
        } else if matches!(entity[0], b'(' | b')' | b',') {
            entity = &entity[1..];
        } else {
            return None;
        }
    }
    Some(values)
}

// Only source IDs requested by the parsed Table can supply numeric authority.
// Strings, comments and complex records never supply coordinates.
fn numbers(bytes: &[u8], needed: &HashSet<u64>) -> Option<HashMap<u64, Scalars>> {
    let mut result = HashMap::new();
    let mut seen = HashSet::new();
    for instance in spelling::instances(bytes) {
        let instance = instance?;
        if !needed.contains(&instance.id) {
            continue;
        }
        if !seen.insert(instance.id) {
            result.remove(&instance.id);
        } else if matches!(instance.kind, b"CARTESIAN_POINT" | b"DIRECTION" | b"VECTOR")
            && let Some(values) = numeric_values(instance.parameters)
        {
            result.insert(instance.id, values);
        }
    }
    Some(result)
}

pub(in crate::step) struct NativeSources {
    pub lines: HashMap<u64, Source>,
    pub planes: HashMap<u64, Plane>,
}

pub(in crate::step) struct Plane {
    origin: Point,
    normal: Point,
}

// The native reader supplies only IDs used by planned trim repairs. Scan DATA
// once for the entire asset, including every plane incident to those endpoints.
pub(in crate::step) fn capture_native(
    bytes: &[u8],
    lines: &HashMap<u64, ([u64; 4], bool)>,
    planes: &HashMap<u64, [u64; 2]>,
) -> Option<NativeSources> {
    #[cfg(test)]
    NATIVE_SCANS.with(|count| count.set(count.get() + 1));
    let needed = lines
        .values()
        .flat_map(|(ids, _)| *ids)
        .chain(planes.values().flatten().copied())
        .collect();
    let values = numbers(bytes, &needed)?;
    let point = |id| values.get(&id)?.point();
    let lines = lines
        .iter()
        .map(|(&edge, &([origin, direction, start, end], same_sense))| {
            Some((
                edge,
                Source {
                    origin: point(origin)?,
                    direction: point(direction)?,
                    points: [point(start)?, point(end)?],
                    same_sense,
                },
            ))
        })
        .collect::<Option<_>>()?;
    let planes = planes
        .iter()
        .map(|(&id, &[origin, normal])| {
            Some((
                id,
                Plane {
                    origin: point(origin)?,
                    normal: point(normal)?,
                },
            ))
        })
        .collect::<Option<_>>()?;
    Some(NativeSources { lines, planes })
}

#[cfg(test)]
thread_local! {
    pub(in crate::step) static NATIVE_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
    let point = |id| -> Option<Point> { numbers.get(&id)?.point() };
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

pub(in crate::step) struct ExactPlacement {
    axes: [Point; 3],
    origin: Point,
    cofactors: [Point; 3],
}

impl ExactPlacement {
    pub(in crate::step) fn new(factor: f64, placement: Matrix4) -> Option<Self> {
        if [placement.x.w, placement.y.w, placement.z.w, placement.w.w] != [0., 0., 0., 1.] {
            return None;
        }
        let factor = Q::from_float(factor).filter(|f| f.is_positive())?;
        let point = |v: cgmath::Vector4<f64>| rational_point(Point3::new(v.x, v.y, v.z));
        let [Some(x), Some(y), Some(z)] = [placement.x, placement.y, placement.z].map(point) else {
            return None;
        };
        let axes = [x, y, z].map(|v| v.map(|q| q * &factor));
        let cross = |a: &Point, b: &Point| {
            std::array::from_fn(|i| {
                let j = (i + 1) % 3;
                let k = (i + 2) % 3;
                &a[j] * &b[k] - &a[k] * &b[j]
            })
        };
        let [x, y, z] = &axes;
        let cofactors = [cross(y, z), cross(z, x), cross(x, y)];
        if dot(x, &cofactors[0]).is_zero() {
            return None;
        }
        Some(Self {
            axes,
            origin: point(placement.w)?,
            cofactors,
        })
    }

    fn point(&self, p: &Point) -> Point {
        std::array::from_fn(|row| {
            (0..3).fold(self.origin[row].clone(), |sum, col| {
                sum + &self.axes[col][row] * &p[col]
            })
        })
    }
}

impl Plane {
    pub(in crate::step) fn contains(
        &self,
        local: Point3,
        emitted: Point3,
        placement: &ExactPlacement,
        source_epsilon: f64,
        world_epsilon: f64,
    ) -> Option<bool> {
        let contains = |point: Point, origin, normal, epsilon| {
            let epsilon = Q::from_float(epsilon)?;
            let squared_normal = dot(normal, normal);
            let residual = dot(&sub(&point, origin), normal);
            Some(
                squared_normal.is_positive()
                    && &residual * &residual <= &epsilon * &epsilon * squared_normal,
            )
        };
        // Cofactors transform normal covectors without rounding an inverse.
        let normal = std::array::from_fn(|row| {
            (0..3)
                .map(|col| &placement.cofactors[col][row] * &self.normal[col])
                .sum()
        });
        Some(
            contains(
                rational_point(local)?,
                &self.origin,
                &self.normal,
                source_epsilon,
            )? && contains(
                rational_point(emitted)?,
                &placement.point(&self.origin),
                &normal,
                world_epsilon,
            )?,
        )
    }
}

fn include_exact(bounds: &mut [[f64; 3]; 2], point: &Point) -> Option<()> {
    let mut lower = [0.; 3];
    let mut upper = [0.; 3];
    for (i, exact) in point.iter().enumerate() {
        let rounded = exact.to_f64()?;
        let comparison = Q::from_float(rounded)?.cmp(exact);
        lower[i] = if comparison.is_gt() {
            rounded.next_down()
        } else {
            rounded
        };
        upper[i] = if comparison.is_lt() {
            rounded.next_up()
        } else {
            rounded
        };
    }
    super::bounds::push(bounds, Point3::from(lower)).ok()?;
    super::bounds::push(bounds, Point3::from(upper)).ok()
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
    /// Bound the original endpoint chord, its projected LINE interval, and the
    /// repaired chord against one another. Affine correspondence places each
    /// maximum at an endpoint, in either direction after instance placement.
    pub(in crate::step) fn shared_repair_bound(
        &self,
        repaired: [Point3; 2],
        world: Matrix4,
        placement: &ExactPlacement,
        allowance: f64,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<f64, &'static str> {
        if dot(&self.direction, &self.direction).is_zero() {
            return Err("zero authored LINE direction");
        }
        let projected = self.projected_controls();
        let candidate = repaired.map(rational_point);
        let [Some(a), Some(b)] = candidate else {
            return Err("nonfinite repaired LINE");
        };
        for (start, end) in [(&projected[0], &projected[1]), (&a, &b)] {
            let advance = dot(&sub(end, start), &self.direction);
            if advance.is_zero() || advance.is_positive() != self.same_sense {
                return Err("finite LINE interval is degenerate or contradicts its sense");
            }
        }
        let mut squared = Q::zero();
        for ((vertex, line), candidate) in self.points.iter().zip(&projected).zip(repaired) {
            let vertex = placement.point(vertex);
            let line = placement.point(line);
            // Match the mesh emission's exact operation order, including the
            // rounding of the addition of the occurrence translation.
            let actual = rational_point(world.transform_point(candidate))
                .ok_or("nonfinite emitted world LINE")?;
            for (before, after) in [(&vertex, &line), (&vertex, &actual), (&line, &actual)] {
                let delta = sub(before, after);
                squared = squared.max(dot(&delta, &delta));
            }
            for point in [&vertex, &line, &actual] {
                include_exact(bounds, point).ok_or("nonfinite shared LINE bounds")?;
            }
        }
        let budget = Q::from_float(allowance)
            .filter(|q| q.is_positive())
            .ok_or("invalid shared LINE repair allowance")?;
        if squared > &budget * &budget {
            return Err("finite LINE displacement exceeds preview repair budget");
        }
        sqrt_upper(&squared, "shared LINE movement range")
    }

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

    /// Numeric authority comes only from the DATA section of a whole exchange.
    fn exchange(header: &str, data: &str) -> String {
        format!("ISO-10303-21;HEADER;{header}ENDSEC;DATA;{data}ENDSEC;END-ISO-10303-21;")
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn numeric_authority_does_not_copy_ignored_labels_or_source_text() {
        let label = "x".repeat(131_072);
        let source = exchange(
            "",
            &format!(
                "#1=CARTESIAN_POINT('{label}',(0.,1.,2.));\n\
                 /* {label} #1=CARTESIAN_POINT('',(9.,9.,9.)); */\n\
                 #99=IGNORED('{label}'';#1=CARTESIAN_POINT('''',(9.,9.,9.));');\n\
                 #2=DIRECTION('',(0.,1.,0.));\n\
                 #2=(DIRECTION('',(0.,1.,0.))VECTOR('',#1,1.));"
            ),
        );
        let needed = HashSet::from([1, 2]);
        let (authority, largest) =
            crate::allocation_tests::largest_request(|| numbers(source.as_bytes(), &needed));
        let authority = authority.unwrap();
        assert_eq!(
            authority[&1].as_slice(),
            &[
                Q::zero(),
                Q::from_integer(1.into()),
                Q::from_integer(2.into())
            ]
        );
        assert!(!authority.contains_key(&2));
        assert!(largest < 4096, "numeric spelling allocated {largest} bytes");
    }

    #[test]
    fn numeric_spelling_keeps_exact_decimals_and_opaque_metadata() {
        for (literal, rational) in [
            ("+12.50E-1", "5/4"),
            ("-0.025", "-1/40"),
            ("1.E+2", "100"),
            ("0.E-64", "0"),
        ] {
            assert_eq!(decimal(literal), Some(rational.parse::<Q>().unwrap()));
        }
        for literal in ["1.E65", "1.E-65", "1..0", ".", "+", "1 0"] {
            assert!(decimal(literal).is_none(), "{literal}");
        }
        assert!(decimal(&"1".repeat(81)).is_none());

        let data: &[u8] =
            b"#001 /* label */ = CARTESIAN_POINT('a'';#1=\xff',(-0.025,+12.50E-1,1.E+2));\n\
            #2=(DIRECTION('',(0.,1.,0.))VECTOR('',#1,1.));\n\
            #3=VECTOR('',#1,2.);\n\
            #3=IGNORED('duplicate');";
        let exchange = |data: &[u8]| {
            [
                &b"ISO-10303-21;HEADER;FILE_DESCRIPTION(('ignore #1=VECTOR('',#2,4.);'),'2;1');\
                   /* #1= */ENDSEC;DATA;"[..],
                data,
                b"ENDSEC;END-ISO-10303-21;",
            ]
            .concat()
        };
        let bytes = exchange(data);
        let needed = HashSet::from([1, 2, 3]);
        let authority = numbers(&bytes, &needed).unwrap();
        assert_eq!(authority.len(), 1);
        assert_eq!(
            authority[&1].as_slice(),
            &[
                "-1/40".parse().unwrap(),
                "5/4".parse().unwrap(),
                "100".parse().unwrap()
            ]
        );
        let unterminated = exchange(&[data, b"/* unterminated"].concat());
        assert!(numbers(&unterminated, &needed).is_none());
    }

    #[test]
    fn unsupported_or_ambiguous_scalar_entities_do_not_erase_other_authority() {
        let clear = "#1=CARTESIAN_POINT('',(0.,1.,2.));";
        for other in [
            "#2=DIRECTION('',(1.E-100,1.,0.));",
            "#2=DIRECTION('',(0.,1.,0.,4.));",
            "#2=DIRECTION('',(.F.,1.,0.));",
            "#2=DIRECTION('',(0.,1.,0.));#2=VECTOR('',#1,1.);",
        ] {
            let source = exchange("", &format!("{clear}{other}"));
            let authority = numbers(source.as_bytes(), &HashSet::from([1, 2])).unwrap();
            assert_eq!(
                authority[&1].as_slice(),
                &[
                    Q::zero(),
                    Q::from_integer(1.into()),
                    Q::from_integer(2.into())
                ]
            );
            assert!(!authority.contains_key(&2));
        }
    }
}
