//! A source certificate for the circular rational profiles used by the case builder.
//! shortcut: only certified circular extrusions and blind seats, extend for another authored family.
use super::*;
mod analytic;
mod countersink;
mod fixture;

pub(super) fn compare_fixture(text: &str, expected: &[(&str, Vec<FixtureFace>)]) -> Result<()> {
    fixture::compare(text, expected)
}

pub(super) fn certify_countersink(
    text: &str,
    shape: Countersink,
) -> Result<CountersinkCertificate> {
    countersink::certify(text, shape)
}
use num_rational::BigRational as Q;
use num_traits::{Signed, ToPrimitive};

type Polynomial = Vec<Q>;
const JOIN: f64 = 1.0e-10;

fn radial_squared_band(radius: f64) -> f64 {
    1.0e-10 * (radius * radius).max(1.0)
}

#[derive(Clone)]
struct Arc {
    points: Vec<Point>,
    weights: Vec<f64>,
    knots: Vec<f64>,
}

fn exact(value: f64) -> Result<Q> {
    Q::from_float(value).ok_or(Error::Geometry)
}
fn integer(value: i64) -> Q {
    Q::from_integer(value.into())
}
fn add(a: &[Q], b: &[Q]) -> Polynomial {
    let mut result = vec![integer(0); a.len().max(b.len())];
    for (i, value) in a.iter().enumerate() {
        result[i] += value;
    }
    for (i, value) in b.iter().enumerate() {
        result[i] += value;
    }
    result
}
fn scale(a: &[Q], factor: &Q) -> Polynomial {
    a.iter().map(|x| x * factor).collect()
}
fn mul(a: &[Q], b: &[Q]) -> Polynomial {
    let mut result = vec![integer(0); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            result[i + j] += x * y;
        }
    }
    result
}
fn derivative(a: &[Q]) -> Polynomial {
    a.iter()
        .enumerate()
        .skip(1)
        .map(|(i, x)| x * integer(i as i64))
        .collect()
}
fn choose(n: usize, k: usize) -> i64 {
    (0..k).fold(1, |value, i| value * (n - i) as i64 / (i + 1) as i64)
}
fn positive_bernstein(a: &[Q]) -> bool {
    let degree = a.len() - 1;
    (0..=degree).all(|j| {
        let coefficient: Q = (0..=j)
            .map(|k| &a[k] * integer(choose(j, k)) / integer(choose(degree, k)))
            .sum();
        coefficient > integer(0)
    })
}

fn knots(multiplicities: &Parameter, values: &Parameter, count: usize) -> Result<Vec<f64>> {
    let multiplicities = list(multiplicities)?;
    let values = list(values)?;
    if multiplicities.len() != values.len() || values.len() < 2 || count > 64 {
        return Err(Error::Unsupported);
    }
    let mut result = Vec::new();
    if values.windows(2).any(|pair| {
        number(&pair[0])
            .ok()
            .zip(number(&pair[1]).ok())
            .is_none_or(|(a, b)| a >= b)
    }) {
        return Err(Error::Geometry);
    }
    for (index, (multiplicity, value)) in multiplicities.iter().zip(values).enumerate() {
        let Parameter::Integer(multiplicity) = multiplicity else {
            return Err(Error::Geometry);
        };
        if !(1..=3).contains(multiplicity)
            || (index > 0 && index + 1 < values.len() && *multiplicity > 2)
        {
            return Err(Error::Geometry);
        }
        result.extend(std::iter::repeat_n(number(value)?, *multiplicity as usize));
    }
    if result.len() != count + 3
        || result.windows(2).any(|pair| pair[0] > pair[1])
        || result[0] >= result[result.len() - 1]
        || result[..3].iter().any(|&x| x != result[0])
        || result[result.len() - 3..]
            .iter()
            .any(|&x| x != result[result.len() - 1])
    {
        return Err(Error::Geometry);
    }
    Ok(result)
}

impl Arc {
    fn from_curve(doc: &Document, id: u64) -> Result<Self> {
        let curve = doc.record(id, "B_SPLINE_CURVE")?;
        if number(&curve[0])? != 2.0 {
            return Err(Error::Unsupported);
        }
        let points = refs(&curve[1])?
            .into_iter()
            .map(|id| doc.point(id))
            .collect::<Result<Vec<_>>>()?;
        let weights = list(&doc.record(id, "RATIONAL_B_SPLINE_CURVE")?[0])?
            .iter()
            .map(number)
            .collect::<Result<Vec<_>>>()?;
        let knot_data = doc.record(id, "B_SPLINE_CURVE_WITH_KNOTS")?;
        let knots = knots(&knot_data[0], &knot_data[1], points.len())?;
        if weights.len() != points.len() || weights.iter().any(|&w| w <= 0.0) {
            return Err(Error::Geometry);
        }
        Ok(Self {
            points,
            weights,
            knots,
        })
    }

    fn turn(&self, shape: CircularExtrusion, z: f64) -> Result<f64> {
        if self.points.iter().any(|p| p[2] != z) {
            return Err(Error::Geometry);
        }
        let control = self
            .points
            .iter()
            .zip(&self.weights)
            .map(|(p, w)| {
                let weight = exact(*w)?;
                Ok([
                    (exact(p[0])? - exact(shape.center[0])?) * &weight,
                    (exact(p[1])? - exact(shape.center[1])?) * &weight,
                    weight,
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        let all_knots = self
            .knots
            .iter()
            .copied()
            .map(exact)
            .collect::<Result<Vec<_>>>()?;
        let minimum = self.weights.iter().copied().fold(f64::INFINITY, f64::min);
        let radial_error = exact(radial_squared_band(shape.radius))?;
        let r2 = exact(shape.radius)? * exact(shape.radius)?;
        let mut turn = 0.0;
        for span in 2..control.len() {
            let left = &all_knots[span];
            let right = &all_knots[span + 1];
            if left == right {
                continue;
            }
            let parameter = vec![left.clone(), right - left];
            let mut basis: Vec<Polynomial> = (0..all_knots.len() - 1)
                .map(|i| vec![integer(i64::from(i == span))])
                .collect();
            for rank in 1..=2 {
                let mut next = Vec::new();
                for j in 0..all_knots.len() - rank - 1 {
                    let first_denominator = &all_knots[j + rank] - &all_knots[j];
                    let second_denominator = &all_knots[j + rank + 1] - &all_knots[j + 1];
                    let first = if first_denominator == integer(0) {
                        vec![integer(0)]
                    } else {
                        scale(
                            &mul(&basis[j], &add(&parameter, &[-&all_knots[j]])),
                            &(integer(1) / first_denominator),
                        )
                    };
                    let second = if second_denominator == integer(0) {
                        vec![integer(0)]
                    } else {
                        scale(
                            &mul(
                                &basis[j + 1],
                                &add(
                                    &[all_knots[j + rank + 1].clone()],
                                    &scale(&parameter, &integer(-1)),
                                ),
                            ),
                            &(integer(1) / second_denominator),
                        )
                    };
                    next.push(add(&first, &second));
                }
                basis = next;
            }
            let mut sum = vec![integer(0)];
            for polynomial in &basis {
                sum = add(&sum, polynomial);
            }
            if add(&sum, &[integer(-1)]).iter().any(|x| x != &integer(0)) {
                return Err(Error::Geometry);
            }
            let homogeneous: [Polynomial; 3] = std::array::from_fn(|axis| {
                let mut result = vec![integer(0)];
                for (basis, point) in basis.iter().zip(&control) {
                    result = add(&result, &scale(basis, &point[axis]));
                }
                result
            });
            let [x, y, w] = &homogeneous;
            let residual = add(&add(&mul(x, x), &mul(y, y)), &scale(&mul(w, w), &(-&r2)));
            let bound: Q = residual
                .iter()
                .map(|x| if x < &integer(0) { -x } else { x.clone() })
                .sum();
            if bound > &radial_error * exact(minimum)? * exact(minimum)? {
                return Err(Error::Geometry);
            }
            // Exact half-span restriction also authenticates the quarter-circle tensor rows.
            for second in [false, true] {
                let restrict = |p: &[Q]| -> Polynomial {
                    (0..p.len())
                        .map(|j| {
                            if second {
                                (j..p.len())
                                    .map(|k| &p[k] * integer(choose(k, j)) / integer(1 << k))
                                    .sum()
                            } else {
                                &p[j] / integer(1 << j)
                            }
                        })
                        .collect()
                };
                let (x, y) = (restrict(x), restrict(y));
                let torque = add(
                    &mul(&x, &derivative(&y)),
                    &scale(&mul(&y, &derivative(&x)), &integer(-1)),
                );
                if !positive_bernstein(&torque) {
                    return Err(Error::Geometry);
                }
                // A common open half-plane rules out hidden turns between the exact endpoints.
                let projection = add(&scale(&x, &x[0]), &scale(&y, &y[0]));
                if !positive_bernstein(&projection) {
                    return Err(Error::Geometry);
                }
                let end_x: Q = x.iter().sum();
                let end_y: Q = y.iter().sum();
                let cross = &x[0] * &end_y - &y[0] * &end_x;
                let dot = &x[0] * &end_x + &y[0] * &end_y;
                if cross <= integer(0) || dot <= integer(0) {
                    return Err(Error::Geometry);
                }
                let ratio = (cross / dot).to_f64().ok_or(Error::Geometry)?;
                if !ratio.is_finite() {
                    return Err(Error::Geometry);
                }
                let angle = ratio.atan();
                if !angle.is_finite() || angle <= 0.0 || angle >= std::f64::consts::PI {
                    return Err(Error::Geometry);
                }
                turn += angle;
            }
        }
        Ok(turn)
    }
}

fn signed_turn(arc: &Arc, shape: CircularExtrusion, z: f64) -> Result<f64> {
    if let Ok(turn) = arc.turn(shape, z) {
        return Ok(turn);
    }
    let reversed = Arc {
        points: arc.points.iter().rev().copied().collect(),
        weights: arc.weights.iter().rev().copied().collect(),
        knots: arc.knots.iter().rev().map(|k| -k).collect(),
    };
    reversed.turn(shape, z).map(|turn| -turn)
}

struct Edge {
    vertices: [u64; 2],
    associated: BTreeSet<u64>,
}
struct Face {
    surface: u64,
    sense: bool,
    edges: Vec<(u64, bool)>,
    loops: Vec<Vec<(u64, bool)>>,
}

fn face(doc: &Document, id: u64) -> Result<Face> {
    face_with_bounds(doc, id, 2)
}

fn face_with_bounds(doc: &Document, id: u64, limit: usize) -> Result<Face> {
    let record = doc.record(id, "ADVANCED_FACE")?;
    let bounds = refs(&record[1])?;
    if bounds.is_empty() || bounds.len() > limit {
        return Err(Error::Unsupported);
    }
    let loops = bounds
        .into_iter()
        .map(|id| {
            let bound = doc
                .record(id, "FACE_BOUND")
                .or_else(|_| doc.record(id, "FACE_OUTER_BOUND"))?;
            let mut edges = refs(&doc.record(reference(&bound[1])?, "EDGE_LOOP")?[1])?
                .into_iter()
                .map(|id| {
                    let oriented = doc.record(id, "ORIENTED_EDGE")?;
                    Ok((reference(&oriented[3])?, boolean(&oriented[4])?))
                })
                .collect::<Result<Vec<_>>>()?;
            if !boolean(&bound[2])? {
                edges.reverse();
                for (_, forward) in &mut edges {
                    *forward = !*forward;
                }
            }
            if edges.is_empty() {
                return Err(Error::Topology);
            }
            Ok(edges)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Face {
        surface: reference(&record[2])?,
        sense: boolean(&record[3])?,
        edges: loops.iter().flatten().copied().collect(),
        loops,
    })
}

fn near(a: Point, b: Point) -> bool {
    a.into_iter().zip(b).all(|(a, b)| (a - b).abs() <= JOIN)
}

fn cap_plane_band(origin: Point, normal: Point, shape: CircularExtrusion) -> Result<(usize, f64)> {
    let [nx, ny, nz] = [exact(normal[0])?, exact(normal[1])?, exact(normal[2])?];
    if nz == integer(0) {
        return Err(Error::Geometry);
    }
    let center_height = exact(origin[2])?
        - (&nx * (exact(shape.center[0])? - exact(origin[0])?)
            + &ny * (exact(shape.center[1])? - exact(origin[1])?))
            / &nz;
    let radius = exact(shape.radius)?;
    let radial_band = exact(radial_squared_band(shape.radius))?;
    let outer_extent = &radius + radial_band / &radius;
    let variation = outer_extent * (nx.abs() + ny.abs()) / nz.abs();
    for cap in 0..2 {
        let band = (&center_height - exact(shape.z[cap])?).abs() + &variation;
        if band <= exact(JOIN)? {
            return Ok((cap, band.to_f64().ok_or(Error::Geometry)?.next_up()));
        }
    }
    Err(Error::Geometry)
}

fn side(
    doc: &Document,
    surface: u64,
    edges: &[(u64, bool)],
    arcs: &BTreeMap<u64, Arc>,
    shape: CircularExtrusion,
    inner: bool,
) -> Result<()> {
    let record = doc.record(surface, "B_SPLINE_SURFACE")?;
    if number(&record[0])? != 2.0 || number(&record[1])? != 1.0 || edges.len() != 4 {
        return Err(Error::Unsupported);
    }
    let points = list(&record[2])?
        .iter()
        .map(|row| {
            refs(row)?
                .into_iter()
                .map(|id| doc.point(id))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let weights = list(&doc.record(surface, "RATIONAL_B_SPLINE_SURFACE")?[0])?
        .iter()
        .map(|row| list(row)?.iter().map(number).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    if points.len() != weights.len()
        || points.iter().any(|row| row.len() != 2)
        || weights
            .iter()
            .any(|row| row.len() != 2 || row[0] != row[1] || row[0] <= 0.0)
        || points.iter().any(|row| {
            row[0][..2] != row[1][..2] || row[0][2] != shape.z[0] || row[1][2] != shape.z[1]
        })
    {
        return Err(Error::Geometry);
    }
    let knot_data = doc.record(surface, "B_SPLINE_SURFACE_WITH_KNOTS")?;
    let u_knots = knots(&knot_data[0], &knot_data[2], points.len())?;
    let v_mult = list(&knot_data[1])?
        .iter()
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    let v_knots = list(&knot_data[3])?
        .iter()
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    if v_mult != [2.0, 2.0] || v_knots.len() != 2 || v_knots[0] >= v_knots[1] {
        return Err(Error::Geometry);
    }
    for (column, z) in shape.z.into_iter().enumerate() {
        let matched: Vec<_> = edges
            .iter()
            .filter_map(|(id, forward)| arcs.get(id).map(|arc| (arc, forward)))
            .filter(|(arc, _)| arc.points[0][2] == z)
            .collect();
        if matched.len() != 1 {
            return Err(Error::Topology);
        }
        let (arc, forward) = matched[0];
        if *forward != ((column == 0) != inner)
            || arc.knots != u_knots
            || arc.points != points.iter().map(|row| row[column]).collect::<Vec<_>>()
            || arc.weights != weights.iter().map(|row| row[column]).collect::<Vec<_>>()
        {
            return Err(Error::Geometry);
        }
    }
    Ok(())
}

pub(super) fn certify(text: &str, shape: CircularExtrusion) -> Result<CircularCertificate> {
    certify_rings(text, shape, None)
}

fn certify_rings(
    text: &str,
    shape: CircularExtrusion,
    inner_radius: Option<f64>,
) -> Result<CircularCertificate> {
    if !shape
        .center
        .into_iter()
        .chain([shape.radius])
        .chain(shape.z)
        .all(f64::is_finite)
        || shape.radius <= 1.0e-6
        || shape.z[1] - shape.z[0] <= 4.0 * JOIN
        || inner_radius.is_some_and(|inner| !inner.is_finite() || inner <= 1.0e-6)
    {
        return Err(Error::Geometry);
    }
    if let Some(inner) = inner_radius {
        let outer = exact(shape.radius)?;
        let inner_exact = exact(inner)?;
        let outer_minimum = &outer - exact(radial_squared_band(shape.radius))? / &outer;
        let inner_maximum = &inner_exact + exact(radial_squared_band(inner))? / &inner_exact;
        if inner_maximum + integer(4) * exact(JOIN)? >= outer_minimum {
            return Err(Error::Geometry);
        }
    }
    let doc = parse(text)?;
    doc.check_complex_scope()?;
    if !doc.ids("CYLINDRICAL_SURFACE").is_empty() {
        if inner_radius.is_some() {
            return Err(Error::Unsupported);
        }
        return analytic::certify(&doc, shape);
    }
    let solids = doc.ids("MANIFOLD_SOLID_BREP");
    if solids.len() != 1 {
        return Err(Error::Unsupported);
    }
    let scales = doc.solid_scales(&solids)?;
    if scales[&solids[0]] != 1.0 {
        return Err(Error::Unsupported);
    }
    for record in doc.0.values().flatten() {
        if !supported_record(record)
            && !matches!(
                record.name.as_str(),
                "BOUNDED_CURVE"
                    | "B_SPLINE_CURVE"
                    | "B_SPLINE_CURVE_WITH_KNOTS"
                    | "CURVE"
                    | "RATIONAL_B_SPLINE_CURVE"
                    | "GEOMETRIC_REPRESENTATION_ITEM"
                    | "REPRESENTATION_ITEM"
                    | "BOUNDED_SURFACE"
                    | "B_SPLINE_SURFACE"
                    | "B_SPLINE_SURFACE_WITH_KNOTS"
                    | "RATIONAL_B_SPLINE_SURFACE"
                    | "SURFACE"
            )
        {
            return Err(Error::Unsupported);
        }
    }
    let shell = reference(&doc.record(solids[0], "MANIFOLD_SOLID_BREP")?[1])?;
    let face_ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
    if face_ids.iter().copied().collect::<BTreeSet<_>>()
        != doc.ids("ADVANCED_FACE").into_iter().collect()
        || face_ids.len() < 4
        || face_ids.len() > 66
    {
        return Err(Error::Topology);
    }
    let faces = face_ids
        .into_iter()
        .map(|id| face(&doc, id))
        .collect::<Result<Vec<_>>>()?;
    let mut uses: BTreeMap<u64, Vec<(u64, bool)>> = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut vertices = BTreeSet::new();
    let mut arcs = BTreeMap::new();
    let mut arc_inner = BTreeMap::new();
    let mut turns = BTreeMap::new();
    let mut used_carriers = BTreeSet::new();
    for face in &faces {
        if face.edges.is_empty() {
            return Err(Error::Topology);
        }
        for &(id, forward) in &face.edges {
            uses.entry(id).or_default().push((face.surface, forward));
            if edges.contains_key(&id) {
                continue;
            }
            let record = doc.record(id, "EDGE_CURVE")?;
            if !boolean(&record[4])? {
                return Err(Error::Unsupported);
            }
            let ends = [reference(&record[1])?, reference(&record[2])?];
            vertices.extend(ends);
            let curve = doc.record(reference(&record[3])?, "SURFACE_CURVE")?;
            if curve.get(3) != Some(&Parameter::Enumeration("CURVE_3D".into())) {
                return Err(Error::Unsupported);
            }
            let associated: BTreeSet<_> = refs(&curve[2])?.into_iter().collect();
            if associated.len() != 2 {
                return Err(Error::Unsupported);
            }
            let carrier = reference(&curve[1])?;
            used_carriers.insert(carrier);
            let (start, end) = (doc.vertex(ends[0])?, doc.vertex(ends[1])?);
            if doc.record(carrier, "B_SPLINE_CURVE").is_ok() {
                let arc = Arc::from_curve(&doc, carrier)?;
                if !near(start, arc.points[0])
                    || !near(end, *arc.points.last().ok_or(Error::Geometry)?)
                {
                    return Err(Error::Geometry);
                }
                let z = arc.points[0][2];
                if !shape.z.contains(&z) {
                    return Err(Error::Geometry);
                }
                let endpoint_radius =
                    (arc.points[0][0] - shape.center[0]).hypot(arc.points[0][1] - shape.center[1]);
                let inner = inner_radius.is_some_and(|inner| {
                    (endpoint_radius - inner).abs() < (endpoint_radius - shape.radius).abs()
                });
                let radius = if inner {
                    inner_radius.unwrap()
                } else {
                    shape.radius
                };
                turns.insert(id, arc.turn(CircularExtrusion { radius, ..shape }, z)?);
                arc_inner.insert(id, inner);
                arcs.insert(id, arc);
            } else {
                let line = doc.record(carrier, "LINE")?;
                let line_vector = doc.record(reference(&line[2])?, "VECTOR")?;
                let direction = vector(&doc.record(reference(&line_vector[1])?, "DIRECTION")?[1])?;
                if number(&line_vector[2])? <= 0.0 {
                    return Err(Error::Geometry);
                }
                let line_origin = doc.point(reference(&line[1])?)?;
                if direction[0] != 0.0
                    || direction[1] != 0.0
                    || direction[2] <= 0.0
                    || !near([line_origin[0], line_origin[1], start[2]], start)
                {
                    return Err(Error::Geometry);
                }
                check_supporting_line(line_origin, direction, start, end)?;
                if start[..2] != end[..2] || start[2] != shape.z[0] || end[2] != shape.z[1] {
                    return Err(Error::Geometry);
                }
            }
            edges.insert(
                id,
                Edge {
                    vertices: ends,
                    associated,
                },
            );
        }
        for boundary in &face.loops {
            for ((a, forward_a), (b, forward_b)) in boundary
                .iter()
                .zip(boundary.iter().cycle().skip(1))
                .take(boundary.len())
            {
                if edges[a].vertices[usize::from(*forward_a)]
                    != edges[b].vertices[usize::from(!*forward_b)]
                {
                    return Err(Error::Topology);
                }
            }
        }
    }
    if uses.iter().any(|(id, used)| {
        used.len() != 2
            || used[0].1 == used[1].1
            || used
                .iter()
                .map(|(surface, _)| *surface)
                .collect::<BTreeSet<_>>()
                != edges[id].associated
    }) || edges.keys().copied().collect::<BTreeSet<_>>()
        != doc.ids("EDGE_CURVE").into_iter().collect()
        || vertices != doc.ids("VERTEX_POINT").into_iter().collect()
        || used_carriers
            != doc
                .ids("B_SPLINE_CURVE")
                .into_iter()
                .chain(doc.ids("LINE"))
                .collect()
    {
        return Err(Error::Topology);
    }
    let rings = 1 + usize::from(inner_radius.is_some());
    let mut cap_turns = [[None, None], [None, None]];
    let mut cap_offsets = [0.0, 0.0];
    let mut surfaces = BTreeSet::new();
    let mut side_count = [0, 0];
    for face in &faces {
        if !surfaces.insert(face.surface) {
            return Err(Error::Topology);
        }
        if doc.record(face.surface, "PLANE").is_ok() {
            let (origin, normal) = doc.plane(face.surface)?;
            let place = doc.record(
                reference(&doc.record(face.surface, "PLANE")?[1])?,
                "AXIS2_PLACEMENT_3D",
            )?;
            if place[3] != Parameter::NotProvided {
                let ref_axis = vector(&doc.record(reference(&place[3])?, "DIRECTION")?[1])?;
                unit(cross(normal, ref_axis))?;
            }
            let (cap, axial_band) = cap_plane_band(origin, normal, shape)?;
            if ((normal[2] > 0.0) == face.sense) != (cap == 1) || face.loops.len() != rings {
                return Err(Error::Geometry);
            }
            cap_offsets[cap] = axial_band;
            for boundary in &face.loops {
                let inner = *arc_inner.get(&boundary[0].0).ok_or(Error::Geometry)?;
                let ring = usize::from(inner);
                if cap_turns[cap][ring].is_some()
                    || boundary.iter().any(|(id, forward)| {
                        !arcs.contains_key(id)
                            || arc_inner[id] != inner
                            || *forward != ((cap == 1) != inner)
                            || arcs[id].points[0][2] != shape.z[cap]
                    })
                {
                    return Err(Error::Geometry);
                }
                cap_turns[cap][ring] = Some(boundary.iter().map(|(id, _)| turns[id]).sum::<f64>());
            }
        } else {
            let labels: Vec<_> = face
                .edges
                .iter()
                .filter_map(|(id, _)| arc_inner.get(id))
                .copied()
                .collect();
            if face.loops.len() != 1
                || labels.len() != 2
                || labels[0] != labels[1]
                || face.sense == labels[0]
            {
                return Err(Error::Geometry);
            }
            side(&doc, face.surface, &face.edges, &arcs, shape, labels[0])?;
            side_count[usize::from(labels[0])] += 1;
        }
    }
    let total_side_count: usize = side_count.iter().sum();
    if surfaces
        != doc
            .ids("PLANE")
            .into_iter()
            .chain(doc.ids("B_SPLINE_SURFACE"))
            .collect()
        || vertices.len() != total_side_count * 2
        || edges.len() != total_side_count * 3
        || faces.len() != total_side_count + 2
        || side_count[..rings].iter().any(|&count| count < 2)
        || cap_turns.iter().any(|caps| {
            caps[..rings].iter().any(|turn| {
                turn.is_none_or(|turn| {
                    !turn.is_finite() || (turn - std::f64::consts::TAU).abs() > 1.0e-8
                })
            })
        })
    {
        return Err(Error::Topology);
    }
    let radii = [shape.radius, inner_radius.unwrap_or(0.0)];
    let height = shape.z[1] - shape.z[0];
    let volume = std::f64::consts::PI * (radii[0] * radii[0] - radii[1] * radii[1]) * height;
    let mut error = 64.0 * f64::EPSILON * volume + 1.0e-10;
    let axial_error: f64 = cap_offsets.iter().sum();
    let mut coordinate_error: f64 = 0.0;
    for ring in 0..rings {
        let radius = radii[ring];
        let band = radial_squared_band(radius);
        error += (std::f64::consts::PI * band
            + 0.5e-8 * (radius * radius + band)
            + 8.0 * side_count[ring] as f64 * radius * JOIN)
            * height;
        error += std::f64::consts::PI * (radius * radius + band) * axial_error;
        coordinate_error = coordinate_error.max(band / radius);
    }
    if !volume.is_finite() || volume <= 0.0 || !error.is_finite() {
        return Err(Error::Geometry);
    }
    Ok(CircularCertificate {
        bounds: [
            [
                shape.center[0] - shape.radius,
                shape.center[1] - shape.radius,
                shape.z[0],
            ],
            [
                shape.center[0] + shape.radius,
                shape.center[1] + shape.radius,
                shape.z[1],
            ],
        ],
        bound_error: coordinate_error + total_side_count as f64 * JOIN * 4.0 + 1.0e-10,
        volume: [volume - error, volume + error],
        topology: [vertices.len(), edges.len(), faces.len()],
        shape,
        inner_radius,
    })
}

pub(super) fn certify_annulus(
    text: &str,
    shape: CircularExtrusion,
    inner_radius: f64,
) -> Result<CircularCertificate> {
    certify_rings(text, shape, Some(inner_radius))
}
