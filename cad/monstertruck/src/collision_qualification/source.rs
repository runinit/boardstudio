use super::exact::*;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, BTreeSet};
use step_p21::ast::{EntityInstance, Name, Parameter as P, Record};

struct Document {
    records: BTreeMap<u64, Vec<Record>>,
    numbers: BTreeMap<u64, Vec<String>>,
}
pub(super) struct Face {
    pub solid: u64,
    pub shell: usize,
    pub id: u64,
    pub support: u64,
    pub loops: Vec<Vec<Point>>,
}
pub(super) struct Source {
    pub faces: Vec<Face>,
    pub solids: usize,
    pub precision: Q,
    pub placement: [Q; 16],
    pub placement_error_per_radius: Q,
    pub child_origin: Point,
}

fn planar_frame(frame: [Point; 3]) -> Check<(Point, Point, Point, Q)> {
    let [origin, axis, reference] = frame;
    if axis != [Q::zero(), Q::zero(), Q::one()] || !reference[2].is_zero() {
        return Err("unsupported source axis");
    }
    let length = sqrt_bounds(&norm2(&reference))?;
    if length[0] <= 0.0 {
        return Err("source frame normalization");
    }
    let hi = Q::from_float(length[1]).unwrap();
    let lo = Q::from_float(length[0]).unwrap();
    let x = mul(&reference, &(Q::one() / &hi));
    let y = [-x[1].clone(), x[0].clone(), Q::zero()];
    Ok((origin, x, y, (hi.clone() - lo) / hi))
}
fn placement(child: [Point; 3], parent: [Point; 3]) -> Check<([Q; 16], Q, Point)> {
    let (oc, xc, yc, ec) = planar_frame(child)?;
    let (op, xp, yp, ep) = planar_frame(parent)?;
    let mut m = std::array::from_fn(|_| Q::zero());
    m[15] = Q::one();
    m[10] = Q::one();
    for i in 0..2 {
        for j in 0..2 {
            m[j * 4 + i] = &xp[i] * &xc[j] + &yp[i] * &yc[j];
        }
    }
    for i in 0..3 {
        m[12 + i] = &op[i] - (0..3).map(|j| &m[j * 4 + i] * &oc[j]).sum::<Q>();
    }
    Ok((m, ec + ep, oc))
}

fn list(p: &P) -> Check<&[P]> {
    if let P::List(x) = p {
        Ok(x)
    } else {
        Err("source list")
    }
}
fn reference(p: &P) -> Check<u64> {
    if let P::Ref(Name::Entity(x)) = p {
        Ok(*x)
    } else {
        Err("source reference")
    }
}
fn refs(p: &P) -> Check<Vec<u64>> {
    list(p)?.iter().map(reference).collect()
}
fn flag(p: &P) -> Check<bool> {
    match p {
        P::Enumeration(x) if x == "T" => Ok(true),
        P::Enumeration(x) if x == "F" => Ok(false),
        _ => Err("source flag"),
    }
}

// The AST owns structure. This independent lexical pass retains every original decimal.
fn number_lexemes(text: &str) -> Check<BTreeMap<u64, Vec<String>>> {
    let b = text.as_bytes();
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\'' {
            i += 1;
            while i < b.len() {
                if b[i] == b'\'' {
                    i += 1;
                    if b.get(i) != Some(&b'\'') {
                        break;
                    }
                }
                i += 1;
            }
            continue;
        }
        if b.get(i..i + 2) == Some(b"/*") {
            i += 2;
            while b.get(i..i + 2) != Some(b"*/") {
                i += 1;
                if i >= b.len() {
                    return Err("source comment");
                }
            }
            i += 2;
            continue;
        }
        if b[i] != b'#' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let id: u64 = text[start..i].parse().map_err(|_| "source id")?;
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if b.get(i) != Some(&b'=') {
            return Err("source entity assignment");
        }
        i += 1;
        let mut values = Vec::new();
        while i < b.len() && b[i] != b';' {
            if b[i] == b'\'' {
                i += 1;
                while i < b.len() {
                    if b[i] == b'\'' {
                        i += 1;
                        if b.get(i) != Some(&b'\'') {
                            break;
                        }
                    }
                    i += 1;
                }
                continue;
            }
            if b.get(i..i + 2) == Some(b"/*") {
                i += 2;
                while b.get(i..i + 2) != Some(b"*/") {
                    i += 1;
                    if i >= b.len() {
                        return Err("source comment");
                    }
                }
                i += 2;
                continue;
            }
            if b[i] == b'#' {
                i += 1;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                continue;
            }
            if b[i].is_ascii_alphabetic() || b[i] == b'_' {
                while b
                    .get(i)
                    .is_some_and(|x| x.is_ascii_alphanumeric() || *x == b'_')
                {
                    i += 1;
                }
                continue;
            }
            if b[i].is_ascii_digit()
                || b[i] == b'+'
                || b[i] == b'-'
                || (b[i] == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
            {
                let start = i;
                i += 1;
                while b
                    .get(i)
                    .is_some_and(|x| x.is_ascii_digit() || b".eE+-".contains(x))
                {
                    i += 1;
                }
                values.push(text[start..i].to_owned());
                continue;
            }
            i += 1;
        }
        if out.insert(id, values).is_some() {
            return Err("duplicate source id");
        }
        if out.len() > 10_000 {
            return Err("source entity cap");
        }
        i += 1;
    }
    Ok(out)
}
impl Document {
    fn new(bytes: &[u8]) -> Check<Self> {
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("source byte cap");
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "source encoding")?;
        let exchange = step_p21::parser::parse(text).map_err(|_| "source syntax")?;
        if exchange.data.len() != 1 {
            return Err("source data sections");
        }
        let mut records = BTreeMap::new();
        for entity in exchange.data.into_iter().flat_map(|x| x.entities) {
            let (id, items) = match entity {
                EntityInstance::Simple { id, record } => (id, vec![record]),
                EntityInstance::Complex { id, subsuper } => (id, subsuper.0),
            };
            if records.insert(id, items).is_some() {
                return Err("duplicate source id");
            }
        }
        let numbers = number_lexemes(text)?;
        if records.keys().ne(numbers.keys()) {
            return Err("source lexical census");
        }
        Ok(Self { records, numbers })
    }
    fn ids(&self, name: &str) -> Vec<u64> {
        self.records
            .iter()
            .filter(|(_, r)| r.iter().any(|x| x.name == name))
            .map(|(&i, _)| i)
            .collect()
    }
    fn record(&self, id: u64, name: &str, arity: usize) -> Check<&[P]> {
        let record = self
            .records
            .get(&id)
            .ok_or("missing source record")?
            .iter()
            .find(|x| x.name == name)
            .ok_or("unsupported source class")?;
        let args = list(&record.parameter)?;
        if args.len() != arity {
            return Err("source arity");
        }
        Ok(args)
    }
    fn vector(&self, id: u64, kind: &str) -> Check<Point> {
        let r = self.record(id, kind, 2)?;
        if list(&r[1])?.len() != 3 {
            return Err("source coordinate dimension");
        }
        let n = self.numbers.get(&id).ok_or("source lexemes")?;
        if n.len() != 3 {
            return Err("source numeric lexemes");
        }
        Ok([decimal(&n[0])?, decimal(&n[1])?, decimal(&n[2])?])
    }
    fn vertex(&self, id: u64) -> Check<Point> {
        self.vector(
            reference(&self.record(id, "VERTEX_POINT", 2)?[1])?,
            "CARTESIAN_POINT",
        )
    }
    fn frame(&self, id: u64) -> Check<[Point; 3]> {
        let a = self.record(id, "AXIS2_PLACEMENT_3D", 4)?;
        Ok([
            self.vector(reference(&a[1])?, "CARTESIAN_POINT")?,
            self.vector(reference(&a[2])?, "DIRECTION")?,
            self.vector(reference(&a[3])?, "DIRECTION")?,
        ])
    }
    fn loop_points(&self, bound: u64) -> Check<Vec<Point>> {
        let args = self
            .record(bound, "FACE_OUTER_BOUND", 3)
            .or_else(|_| self.record(bound, "FACE_BOUND", 3))?;
        let loop_id = reference(&args[1])?;
        let mut points = Vec::new();
        let mut first = None;
        let mut prior = None;
        for oe in refs(&self.record(loop_id, "EDGE_LOOP", 2)?[1])? {
            let oriented = self.record(oe, "ORIENTED_EDGE", 5)?;
            let edge = self.record(reference(&oriented[3])?, "EDGE_CURVE", 5)?;
            let start = reference(&edge[1])?;
            let end = reference(&edge[2])?;
            let line = self.record(reference(&edge[3])?, "LINE", 3)?;
            let origin = self.vector(reference(&line[1])?, "CARTESIAN_POINT")?;
            let vector_id = reference(&line[2])?;
            let vector = self.record(vector_id, "VECTOR", 3)?;
            if self.numbers[&vector_id].len() != 1
                || decimal(&self.numbers[&vector_id][0])? <= Q::zero()
            {
                return Err("source line magnitude");
            }
            let direction = self.vector(reference(&vector[1])?, "DIRECTION")?;
            if norm2(&direction).is_zero() {
                return Err("source zero line");
            }
            for p in [self.vertex(start)?, self.vertex(end)?] {
                if !norm2(&cross(&sub(&p, &origin), &direction)).is_zero() {
                    return Err("inconsistent source line");
                }
            }
            let (a, b) = if flag(&oriented[4])? {
                (start, end)
            } else {
                (end, start)
            };
            if prior.is_some_and(|p| p != a) {
                return Err("source loop discontinuity");
            }
            first.get_or_insert(a);
            prior = Some(b);
            points.push(self.vertex(a)?);
        }
        if points.len() < 3 || first != prior {
            return Err("source open loop");
        }
        if !flag(&args[2])? {
            points.reverse();
        }
        Ok(points)
    }
}

pub(super) fn read(bytes: &[u8]) -> Check<Source> {
    let d = Document::new(bytes)?;
    for name in [
        "BREP_WITH_VOIDS",
        "BREP_WITH_VOIDS_AND_SHELL_BASED_SURFACE_MODEL",
        "MAPPED_ITEM",
        "REPRESENTATION_MAP",
        "SHELL_BASED_SURFACE_MODEL",
        "GEOMETRIC_CURVE_SET",
        "FACE_SURFACE",
        "OPEN_SHELL",
    ] {
        if !d.ids(name).is_empty() {
            return Err("unsupported source class");
        }
    }
    if d.records
        .values()
        .flatten()
        .any(|r| r.name.starts_with("CARTESIAN_TRANSFORMATION_OPERATOR"))
    {
        return Err("unsupported source placement");
    }
    let instances = d.ids("NEXT_ASSEMBLY_USAGE_OCCURRENCE");
    let transforms = d.ids("ITEM_DEFINED_TRANSFORMATION");
    let links = d.ids("CONTEXT_DEPENDENT_SHAPE_REPRESENTATION");
    if instances.len() > 1 || instances.len() != transforms.len() || links.len() != instances.len()
    {
        return Err("unsupported source occurrence graph");
    }
    let mut source_placement =
        std::array::from_fn(|i| if i % 5 == 0 { Q::one() } else { Q::zero() });
    let mut source_placement_error = Q::zero();
    let mut child_origin = std::array::from_fn(|_| Q::zero());
    if let Some(&id) = transforms.first() {
        let a = d.record(id, "ITEM_DEFINED_TRANSFORMATION", 4)?;
        let link = d.record(links[0], "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION", 2)?;
        let relation = reference(&link[0])?;
        if reference(
            &d.record(
                relation,
                "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                1,
            )?[0],
        )? != id
        {
            return Err("source transform ownership");
        }
        let relation = d.record(relation, "REPRESENTATION_RELATIONSHIP", 4)?;
        let child = reference(&relation[2])?;
        let parent = reference(&relation[3])?;
        let child_items = refs(&d.record(child, "SHAPE_REPRESENTATION", 3)?[1])?;
        let parent_items = refs(&d.record(parent, "SHAPE_REPRESENTATION", 3)?[1])?;
        if !child_items.contains(&reference(&a[2])?) || !parent_items.contains(&reference(&a[3])?) {
            return Err("source frame representation order");
        }
        let pds = d.record(reference(&link[1])?, "PRODUCT_DEFINITION_SHAPE", 3)?;
        if reference(&pds[2])? != instances[0] {
            return Err("source instance ownership");
        }
        let instance = d.record(instances[0], "NEXT_ASSEMBLY_USAGE_OCCURRENCE", 6)?;
        for (rep, pd) in [
            (parent, reference(&instance[3])?),
            (child, reference(&instance[4])?),
        ] {
            let matches = d
                .ids("SHAPE_DEFINITION_REPRESENTATION")
                .into_iter()
                .filter(|&i| {
                    d.record(i, "SHAPE_DEFINITION_REPRESENTATION", 2)
                        .and_then(|r| {
                            Ok(reference(&r[1])? == rep
                                && reference(
                                    &d.record(reference(&r[0])?, "PRODUCT_DEFINITION_SHAPE", 3)?[2],
                                )? == pd)
                        })
                        .unwrap_or(false)
                })
                .count();
            if matches != 1 {
                return Err("source representation product ownership");
            }
        }
        for brep in d.ids("ADVANCED_BREP_SHAPE_REPRESENTATION") {
            let count = d
                .ids("SHAPE_REPRESENTATION_RELATIONSHIP")
                .into_iter()
                .filter(|&i| {
                    d.record(i, "SHAPE_REPRESENTATION_RELATIONSHIP", 4)
                        .and_then(|r| Ok(reference(&r[2])? == child && reference(&r[3])? == brep))
                        .unwrap_or(false)
                })
                .count();
            if count != 1 {
                return Err("source definition representation ownership");
            }
        }
        (source_placement, source_placement_error, child_origin) =
            placement(d.frame(reference(&a[2])?)?, d.frame(reference(&a[3])?)?)?;
    }
    for rep in d
        .ids("SHAPE_REPRESENTATION")
        .into_iter()
        .map(|i| (i, "SHAPE_REPRESENTATION"))
        .chain(
            d.ids("ADVANCED_BREP_SHAPE_REPRESENTATION")
                .into_iter()
                .map(|i| (i, "ADVANCED_BREP_SHAPE_REPRESENTATION")),
        )
    {
        let context = reference(&d.record(rep.0, rep.1, 3)?[2])?;
        let units = d.record(context, "GLOBAL_UNIT_ASSIGNED_CONTEXT", 1)?;
        let lengths = refs(&units[0])?
            .into_iter()
            .filter(|&u| d.record(u, "LENGTH_UNIT", 0).is_ok())
            .collect::<Vec<_>>();
        if lengths.len() != 1 {
            return Err("source length unit assignment");
        }
        let r = d.record(lengths[0], "SI_UNIT", 2)?;
        if !matches!((&r[0],&r[1]),(P::Enumeration(a),P::Enumeration(b)) if a=="MILLI" && b=="METRE")
        {
            return Err("unsupported source units");
        }
    }
    let solid_ids = d.ids("MANIFOLD_SOLID_BREP");
    let mut faces = Vec::new();
    let mut shell_ids = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for &solid in &solid_ids {
        let shell = reference(&d.record(solid, "MANIFOLD_SOLID_BREP", 2)?[1])?;
        if !shell_ids.insert(shell) {
            return Err("shared source shell unsupported");
        }
        let mut shell_edges = BTreeMap::<u64, (usize, i32)>::new();
        for id in refs(&d.record(shell, "CLOSED_SHELL", 2)?[1])? {
            if !seen.insert(id) {
                return Err("duplicate source face use");
            }
            let a = d.record(id, "ADVANCED_FACE", 4)?;
            let support = reference(&a[2])?;
            let plane = d.record(support, "PLANE", 2)?;
            let frame = d.frame(reference(&plane[1])?)?;
            if norm2(&frame[1]).is_zero() || norm2(&cross(&frame[1], &frame[2])).is_zero() {
                return Err("source plane axes");
            }
            let bounds = refs(&a[1])?;
            for &bound in &bounds {
                let r = d
                    .record(bound, "FACE_OUTER_BOUND", 3)
                    .or_else(|_| d.record(bound, "FACE_BOUND", 3))?;
                for oe in refs(&d.record(reference(&r[1])?, "EDGE_LOOP", 2)?[1])? {
                    let e = d.record(oe, "ORIENTED_EDGE", 5)?;
                    let use_ = shell_edges.entry(reference(&e[3])?).or_default();
                    use_.0 += 1;
                    use_.1 += if flag(&e[4])? == flag(&r[2])? { 1 } else { -1 };
                }
            }
            let mut loops = Vec::new();
            let outer = bounds
                .iter()
                .filter(|&&i| d.record(i, "FACE_OUTER_BOUND", 3).is_ok())
                .copied()
                .collect::<Vec<_>>();
            if outer.len() != 1 {
                return Err("source outer loop");
            }
            for bound in
                std::iter::once(outer[0]).chain(bounds.into_iter().filter(|i| *i != outer[0]))
            {
                let points = d.loop_points(bound)?;
                if points
                    .iter()
                    .any(|p| !dot(&sub(p, &frame[0]), &frame[1]).is_zero())
                {
                    return Err("inconsistent source plane");
                }
                loops.push(points);
            }
            faces.push(Face {
                solid,
                shell: 0,
                id,
                support,
                loops,
            });
        }
        if shell_edges
            .values()
            .any(|&(count, direction)| count != 2 || direction != 0)
        {
            return Err("source shell edge census");
        }
    }
    if faces.is_empty()
        || seen != d.ids("ADVANCED_FACE").into_iter().collect()
        || shell_ids != d.ids("CLOSED_SHELL").into_iter().collect()
    {
        return Err("incomplete source census");
    }
    let mut precision = Q::zero();
    let mut represented = BTreeSet::new();
    for id in d.ids("ADVANCED_BREP_SHAPE_REPRESENTATION") {
        let a = d.record(id, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
        for item in refs(&a[1])? {
            if !solid_ids.contains(&item) || !represented.insert(item) {
                return Err("source representation census");
            }
        }
        let context = reference(&a[2])?;
        let uncertainty = d.record(context, "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT", 1)?;
        let uncertainty = refs(&uncertainty[0])?;
        if uncertainty.is_empty() {
            return Err("missing source precision");
        }
        for u in uncertainty {
            let r = d.record(u, "UNCERTAINTY_MEASURE_WITH_UNIT", 4)?;
            let unit = reference(&r[1])?;
            let unit = d.record(unit, "SI_UNIT", 2)?;
            if !matches!((&unit[0],&unit[1]),(P::Enumeration(a),P::Enumeration(b)) if a=="MILLI" && b=="METRE")
            {
                return Err("source precision units");
            }
            let numbers = &d.numbers[&u];
            if numbers.len() != 1 {
                return Err("source precision lexemes");
            }
            let value = decimal(&numbers[0])?;
            if value <= Q::zero() {
                return Err("source precision");
            }
            precision = precision.max(value);
        }
    }
    if represented != solid_ids.iter().copied().collect() {
        return Err("source representation census");
    }
    Ok(Source {
        faces,
        solids: solid_ids.len(),
        precision,
        placement: source_placement,
        placement_error_per_radius: source_placement_error,
        child_origin,
    })
}

fn area(r: &[Point], axis: usize) -> Q {
    r.iter()
        .zip(r.iter().cycle().skip(1))
        .take(r.len())
        .map(|(a, b)| cross(a, b)[axis].clone())
        .sum()
}
fn proper_cross(a: &Point, b: &Point, c: &Point, d: &Point, axis: usize) -> bool {
    orient(a, b, c, axis) * orient(a, b, d, axis) < Q::zero()
        && orient(c, d, a, axis) * orient(c, d, b, axis) < Q::zero()
}
fn strictly_inside(p: &Point, t: &Triangle, axis: usize) -> bool {
    let signs = edges(t)
        .map(|(a, b)| orient(a, b, p, axis))
        .collect::<Vec<_>>();
    signs.iter().all(Q::is_positive) || signs.iter().all(Q::is_negative)
}
fn overlap(a: &Triangle, b: &Triangle, axis: usize) -> bool {
    edges(a).any(|(p, q)| edges(b).any(|(r, s)| proper_cross(p, q, r, s, axis)))
        || a.iter().any(|p| strictly_inside(p, b, axis))
        || b.iter().any(|p| strictly_inside(p, a, axis))
        || strictly_inside(
            &mul(&add(&add(&a[0], &a[1]), &a[2]), &Q::new(1.into(), 3.into())),
            b,
            axis,
        )
}

// Snapped triangles have disjoint interiors, opposite interior edge uses, the
// complete oriented source boundary and equal area. Their union is the trim.
// Paired vertices then give both surface directions by barycentric interpolation.
pub(super) fn face_bound(loops: &[Vec<Point>], actual: &[Triangle]) -> Check<Q> {
    let ring = &loops[0];
    let n = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| cross(a, b))
        .fold(std::array::from_fn(|_| Q::zero()), |a, b| add(&a, &b));
    let axis = (0..3).max_by_key(|&i| n[i].abs()).unwrap();
    if n[axis].is_zero() {
        return Err("source zero area");
    }
    let mut loops = loops.to_vec();
    for (index, r) in loops.iter_mut().enumerate() {
        if area(r, axis).is_negative() != (index > 0) {
            r.reverse();
        }
        for i in 0..r.len() {
            for j in i + 1..r.len() {
                if (i + 1) % r.len() != j
                    && (j + 1) % r.len() != i
                    && proper_cross(
                        &r[i],
                        &r[(i + 1) % r.len()],
                        &r[j],
                        &r[(j + 1) % r.len()],
                        axis,
                    )
                {
                    return Err("source self crossing");
                }
            }
        }
    }
    let mut edges_in = BTreeMap::<(Point, Point), (usize, i32)>::new();
    for t in actual {
        for (a, b) in edges(t) {
            let (key, direction) = if a < b {
                ((a.clone(), b.clone()), 1)
            } else {
                ((b.clone(), a.clone()), -1)
            };
            let e = edges_in.entry(key).or_default();
            e.0 += 1;
            e.1 += direction;
        }
    }
    let boundary_vertices = edges_in
        .iter()
        .filter(|(_, v)| v.0 == 1)
        .flat_map(|((a, b), _)| [a.clone(), b.clone()])
        .collect::<BTreeSet<_>>();
    let source_edges = loops
        .iter()
        .flat_map(|r| r.iter().zip(r.iter().cycle().skip(1)).take(r.len()))
        .collect::<Vec<_>>();
    let mut maximum = Q::zero();
    let mut snap = BTreeMap::new();
    for p in actual.iter().flatten() {
        if snap.contains_key(p) {
            continue;
        }
        let mut projected = p.clone();
        projected[axis] = &ring[0][axis]
            - (0..3)
                .filter(|&i| i != axis)
                .map(|i| &n[i] * (&p[i] - &ring[0][i]))
                .sum::<Q>()
                / &n[axis];
        if boundary_vertices.contains(p) {
            // A rounded corner can project into either adjacent edge. Authenticate the
            // unique nearby source corner first, then prove the complete boundary below.
            let corners = loops
                .iter()
                .flatten()
                .filter(|q| norm2(&sub(p, q)) <= decimal("1e-16").unwrap())
                .collect::<BTreeSet<_>>();
            projected = match corners.len() {
                0 => source_edges
                    .iter()
                    .map(|(a, b)| closest_segment(&projected, a, b))
                    .min_by_key(|q| norm2(&sub(p, q)))
                    .ok_or("source boundary")?,
                1 => (*corners.first().unwrap()).clone(),
                _ => return Err("ambiguous source corner"),
            };
        }
        maximum = maximum.max(norm2(&sub(p, &projected)));
        snap.insert(p.clone(), projected);
    }
    let mut triangles = actual
        .iter()
        .map(|t| t.each_ref().map(|p| snap[p].clone()))
        .collect::<Vec<_>>();
    let sign = triangles
        .first()
        .map(|t| orient(&t[0], &t[1], &t[2], axis).is_positive())
        .ok_or("missing face triangles")?;
    for t in &mut triangles {
        if !sign {
            t.swap(1, 2);
        }
        if orient(&t[0], &t[1], &t[2], axis) <= Q::zero() {
            return Err("snapped triangle orientation");
        }
    }
    for i in 0..triangles.len() {
        for j in i + 1..triangles.len() {
            if overlap(&triangles[i], &triangles[j], axis) {
                return Err("overlapping face triangles");
            }
        }
    }
    let mut emitted = BTreeMap::<(Point, Point), (usize, i32)>::new();
    for t in &triangles {
        for (a, b) in edges(t) {
            let (key, dir) = if a < b {
                ((a.clone(), b.clone()), 1)
            } else {
                ((b.clone(), a.clone()), -1)
            };
            let e = emitted.entry(key).or_default();
            e.0 += 1;
            e.1 += dir;
        }
    }
    let mut coverage = vec![Vec::<(Q, Q)>::new(); source_edges.len()];
    for ((a, b), (count, direction)) in emitted {
        if count == 2 && direction == 0 {
            continue;
        }
        if count != 1 {
            return Err("face edge incidence");
        }
        let (a, b) = if direction > 0 { (a, b) } else { (b, a) };
        let mut matched = false;
        for (i, (p, q)) in source_edges.iter().enumerate() {
            if closest_segment(&a, p, q) != a || closest_segment(&b, p, q) != b {
                continue;
            }
            let d = sub(q, p);
            let n = norm2(&d);
            if n.is_zero() {
                return Err("zero source edge");
            }
            let u = dot(&sub(&a, p), &d) / &n;
            let v = dot(&sub(&b, p), &d) / n;
            if v <= u {
                return Err("source boundary order");
            }
            coverage[i].push((u, v));
            matched = true;
            break;
        }
        if !matched {
            return Err("missing source boundary correspondence");
        }
    }
    for chain in &mut coverage {
        chain.sort();
        let mut end = Q::zero();
        for (a, b) in chain {
            if *a != end {
                return Err("source boundary gap or overlap");
            }
            end = b.clone();
        }
        if end != Q::one() {
            return Err("incomplete source boundary");
        }
    }
    let expected: Q = loops.iter().map(|r| area(r, axis)).sum();
    let covered: Q = triangles
        .iter()
        .map(|t| orient(&t[0], &t[1], &t[2], axis))
        .sum();
    if expected <= Q::zero() || expected != covered {
        return Err("source face area mismatch");
    }
    Ok(maximum)
}
