//! Vertical-wall prisms: two horizontal caps joined by planar and Z-cylindrical walls.
//! shortcut: one constant cross-section per solid; stepped layers, cones and tilted walls are refused.
use super::*;

const WALL: f64 = 8.0 * JOIN;
const AXIS: f64 = 1.0e-12;
/// Chord subdivision for winding probes; the matching sagitta margin is `SAGITTA * radius`.
const CHORD: f64 = std::f64::consts::PI / 64.0;
const SAGITTA: f64 = 3.0e-4;

#[derive(Clone, Copy)]
enum Kind {
    Cap { z: f64, outward: f64 },
    Wall { origin: [f64; 2], outward: [f64; 2] },
    Cylinder { center: [f64; 2], radius: f64 },
}

enum Shape {
    Line,
    Arc(Arc),
}

struct EdgeData {
    vertices: [u64; 2],
    ends: [Point; 2],
    shape: Shape,
}

fn xy(point: Point) -> [f64; 2] {
    [point[0], point[1]]
}

fn near2(a: [f64; 2], b: [f64; 2]) -> bool {
    a.into_iter().zip(b).all(|(a, b)| (a - b).abs() <= WALL)
}

fn edge(doc: &Document, id: u64) -> Result<EdgeData> {
    let record = doc.record(id, "EDGE_CURVE")?;
    let vertices = [reference(&record[1])?, reference(&record[2])?];
    if !boolean(&record[4])? {
        return Err(Error::Unsupported);
    }
    let ends = [doc.vertex(vertices[0])?, doc.vertex(vertices[1])?];
    let surface_curve = doc.record(reference(&record[3])?, "SURFACE_CURVE")?;
    let curve = reference(&surface_curve[1])?;
    let shape = if let Ok(line) = doc.record(curve, "LINE") {
        let origin = doc.point(reference(&line[1])?)?;
        let magnitude = doc.record(reference(&line[2])?, "VECTOR")?;
        let direction = unit(vector(
            &doc.record(reference(&magnitude[1])?, "DIRECTION")?[1],
        )?)?;
        let length = number(&magnitude[2])?;
        let end = std::array::from_fn(|i| origin[i] + direction[i] * length);
        if length <= 0.0 || !near_point(origin, ends[0], WALL) || !near_point(end, ends[1], WALL) {
            return Err(Error::Geometry);
        }
        Shape::Line
    } else {
        let arc = Arc::from_curve(doc, curve)?;
        let last = arc.points.len() - 1;
        if !near_point(arc.points[0], ends[0], WALL) || !near_point(arc.points[last], ends[1], WALL)
        {
            return Err(Error::Geometry);
        }
        Shape::Arc(arc)
    };
    Ok(EdgeData {
        vertices,
        ends,
        shape,
    })
}

fn near_point(a: Point, b: Point, band: f64) -> bool {
    a.into_iter().zip(b).all(|(a, b)| (a - b).abs() <= band)
}

fn classify(doc: &Document, face: &Face) -> Result<Kind> {
    if let Ok(cylinder) = doc.record(face.surface, "CYLINDRICAL_SURFACE") {
        let place = doc.record(reference(&cylinder[1])?, "AXIS2_PLACEMENT_3D")?;
        let axis = match place.get(2) {
            Some(Parameter::NotProvided) => [0.0, 0.0, 1.0],
            Some(axis) => unit(vector(&doc.record(reference(axis)?, "DIRECTION")?[1])?)?,
            None => return Err(Error::Geometry),
        };
        let radius = number(&cylinder[2])?;
        if axis != [0.0, 0.0, 1.0] || radius <= 0.0 {
            return Err(Error::Unsupported);
        }
        let origin = doc.point(reference(&place[1])?)?;
        return Ok(Kind::Cylinder {
            center: xy(origin),
            radius,
        });
    }
    let (origin, normal) = doc.plane(face.surface)?;
    let normal = unit(normal)?;
    let flip = if face.sense { 1.0 } else { -1.0 };
    if normal[0].abs() <= AXIS && normal[1].abs() <= AXIS {
        Ok(Kind::Cap {
            z: origin[2],
            outward: normal[2].signum() * flip,
        })
    } else if normal[2].abs() <= AXIS {
        Ok(Kind::Wall {
            origin: xy(origin),
            outward: [normal[0] * flip, normal[1] * flip],
        })
    } else {
        Err(Error::Unsupported)
    }
}

impl Segment {
    fn start(&self) -> [f64; 2] {
        match *self {
            Segment::Line { from, .. } | Segment::Arc { from, .. } => from,
        }
    }

    fn end(&self) -> [f64; 2] {
        match *self {
            Segment::Line { to, .. } | Segment::Arc { to, .. } => to,
        }
    }

    fn reversed(&self) -> Segment {
        match *self {
            Segment::Line { from, to } => Segment::Line { from: to, to: from },
            Segment::Arc {
                center,
                radius,
                from,
                to,
                turn,
            } => Segment::Arc {
                center,
                radius,
                from: to,
                to: from,
                turn: -turn,
            },
        }
    }

    fn length(&self) -> f64 {
        match *self {
            Segment::Line { from, to } => (to[0] - from[0]).hypot(to[1] - from[1]),
            Segment::Arc { radius, turn, .. } => radius * turn.abs(),
        }
    }

    /// Twice the Green's-theorem area contribution `∫ x dy − y dx`.
    fn twice_area(&self) -> f64 {
        match *self {
            Segment::Line { from, to } => from[0] * to[1] - to[0] * from[1],
            Segment::Arc {
                center,
                radius,
                from,
                to,
                turn,
            } => {
                radius * radius * turn + center[0] * (to[1] - from[1])
                    - center[1] * (to[0] - from[0])
            }
        }
    }

    /// Points where the extreme x/y of this segment can occur, besides its end points.
    fn extremes(&self) -> Vec<[f64; 2]> {
        let Segment::Arc {
            center,
            radius,
            from,
            turn,
            ..
        } = *self
        else {
            return vec![];
        };
        let start = (from[1] - center[1]).atan2(from[0] - center[0]);
        (0..4)
            .filter_map(|quarter| {
                let angle = quarter as f64 * std::f64::consts::FRAC_PI_2;
                let sweep = if turn > 0.0 {
                    angle - start
                } else {
                    start - angle
                }
                .rem_euclid(std::f64::consts::TAU);
                (sweep <= turn.abs()).then(|| {
                    [
                        center[0] + radius * angle.cos(),
                        center[1] + radius * angle.sin(),
                    ]
                })
            })
            .collect()
    }

    fn distance(&self, p: [f64; 2]) -> f64 {
        match *self {
            Segment::Line { from, to } => {
                let d = [to[0] - from[0], to[1] - from[1]];
                let t = (((p[0] - from[0]) * d[0] + (p[1] - from[1]) * d[1])
                    / (d[0] * d[0] + d[1] * d[1]))
                    .clamp(0.0, 1.0);
                (p[0] - from[0] - t * d[0]).hypot(p[1] - from[1] - t * d[1])
            }
            Segment::Arc {
                center,
                radius,
                from,
                to,
                turn,
            } => {
                let start = (from[1] - center[1]).atan2(from[0] - center[0]);
                let angle = (p[1] - center[1]).atan2(p[0] - center[0]);
                let sweep = if turn > 0.0 {
                    angle - start
                } else {
                    start - angle
                }
                .rem_euclid(std::f64::consts::TAU);
                if sweep <= turn.abs() {
                    ((p[0] - center[0]).hypot(p[1] - center[1]) - radius).abs()
                } else {
                    (p[0] - from[0])
                        .hypot(p[1] - from[1])
                        .min((p[0] - to[0]).hypot(p[1] - to[1]))
                }
            }
        }
    }

    fn margin(&self) -> f64 {
        match *self {
            Segment::Line { .. } => WALL,
            Segment::Arc { radius, .. } => WALL + SAGITTA * radius,
        }
    }

    /// Signed angle subtended at `p`; arcs use chords short enough that `margin` bounds the error.
    fn subtended(&self, p: [f64; 2]) -> f64 {
        let angle = |a: [f64; 2], b: [f64; 2]| {
            let (a, b) = ([a[0] - p[0], a[1] - p[1]], [b[0] - p[0], b[1] - p[1]]);
            (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1])
        };
        match *self {
            Segment::Line { from, to } => angle(from, to),
            Segment::Arc {
                center,
                radius,
                from,
                turn,
                ..
            } => {
                let start = (from[1] - center[1]).atan2(from[0] - center[0]);
                let pieces = (turn.abs() / CHORD).ceil().max(1.0) as usize;
                let at = |k: usize| {
                    let a = start + turn * k as f64 / pieces as f64;
                    [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
                };
                (0..pieces).map(|k| angle(at(k), at(k + 1))).sum()
            }
        }
    }
}

/// Winding number of `p` about a closed loop, or `None` when `p` is within the proved margin.
fn winding(loop_: &[Segment], p: [f64; 2]) -> Option<i32> {
    if loop_.iter().any(|s| s.distance(p) <= s.margin()) {
        return None;
    }
    let turns = loop_.iter().map(|s| s.subtended(p)).sum::<f64>() / std::f64::consts::TAU;
    ((turns - turns.round()).abs() < 0.05).then_some(turns.round() as i32)
}

impl PrismCertificate {
    pub fn contains(&self, point: [f64; 3]) -> Option<bool> {
        if !point.into_iter().all(f64::is_finite) {
            return None;
        }
        let e = self.bound_error;
        if self.z.iter().any(|z| (point[2] - z).abs() <= e) {
            return None;
        }
        if point[2] < self.z[0] || point[2] > self.z[1] {
            return Some(false);
        }
        let mut total = 0;
        for loop_ in &self.loops {
            total += winding(loop_, [point[0], point[1]])?;
        }
        match total {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
}

/// Edge ids with traversal direction of one face loop, as segments in that direction.
fn loop_segments(
    loop_: &[(u64, bool)],
    edges: &BTreeMap<u64, EdgeData>,
    circles: &BTreeMap<u64, ([f64; 2], f64, f64)>,
    z: f64,
) -> Result<Vec<Segment>> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut previous_end: Option<u64> = None;
    let mut first_start = 0;
    for (index, &(id, forward)) in loop_.iter().enumerate() {
        let data = &edges[&id];
        if data.ends.iter().any(|p| (p[2] - z).abs() > WALL) {
            return Err(Error::Geometry);
        }
        let curve = match data.shape {
            Shape::Line => Segment::Line {
                from: xy(data.ends[0]),
                to: xy(data.ends[1]),
            },
            Shape::Arc(_) => {
                let &(center, radius, turn) = circles.get(&id).ok_or(Error::Topology)?;
                Segment::Arc {
                    center,
                    radius,
                    from: xy(data.ends[0]),
                    to: xy(data.ends[1]),
                    turn,
                }
            }
        };
        let (segment, start, end) = if forward {
            (curve, data.vertices[0], data.vertices[1])
        } else {
            (curve.reversed(), data.vertices[1], data.vertices[0])
        };
        if index == 0 {
            first_start = start;
        } else if previous_end != Some(start) {
            return Err(Error::Topology);
        }
        previous_end = Some(end);
        segments.push(segment);
    }
    if previous_end != Some(first_start) {
        return Err(Error::Topology);
    }
    Ok(segments)
}

struct Cap {
    outward: f64,
    loops: Vec<Vec<Segment>>,
    area: f64,
}

fn cap_loops(
    face: &Face,
    kind: Kind,
    edges: &BTreeMap<u64, EdgeData>,
    circles: &BTreeMap<u64, ([f64; 2], f64, f64)>,
) -> Result<Cap> {
    let Kind::Cap { z, outward } = kind else {
        return Err(Error::Topology);
    };
    let loops = face
        .loops
        .iter()
        .map(|loop_| loop_segments(loop_, edges, circles, z))
        .collect::<Result<Vec<_>>>()?;
    // Counter-clockwise about the outward normal bounds material; clockwise loops are holes.
    let signed: Vec<f64> = loops
        .iter()
        .map(|loop_| outward * loop_.iter().map(Segment::twice_area).sum::<f64>() / 2.0)
        .collect();
    let outer: Vec<_> = (0..signed.len()).filter(|&i| signed[i] > 0.0).collect();
    if outer.len() != 1 || signed.iter().any(|&a| a == 0.0 || !a.is_finite()) {
        return Err(Error::Geometry);
    }
    Ok(Cap {
        outward,
        area: signed.iter().sum(),
        loops,
    })
}

fn solid(doc: &Document, id: u64, seen: &mut BTreeSet<u64>) -> Result<PrismCertificate> {
    let shell = reference(&doc.record(id, "MANIFOLD_SOLID_BREP")?[1])?;
    let face_ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
    if face_ids.len() < 3 || face_ids.iter().any(|id| !seen.insert(*id)) {
        return Err(Error::Topology);
    }
    let faces = face_ids
        .iter()
        .map(|&id| face_with_bounds(doc, id, usize::MAX))
        .collect::<Result<Vec<_>>>()?;
    let kinds = faces
        .iter()
        .map(|face| classify(doc, face))
        .collect::<Result<Vec<_>>>()?;

    // Every edge borders exactly two faces, traversed in opposite directions.
    let mut uses: BTreeMap<u64, Vec<(usize, bool)>> = BTreeMap::new();
    for (index, face) in faces.iter().enumerate() {
        for &(edge, forward) in face.loops.iter().flatten() {
            uses.entry(edge).or_default().push((index, forward));
        }
    }
    if uses
        .values()
        .any(|users| users.len() != 2 || users[0].1 == users[1].1)
    {
        return Err(Error::Topology);
    }
    let edges = uses
        .keys()
        .map(|&id| Ok((id, edge(doc, id)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;

    let caps: Vec<usize> = (0..faces.len())
        .filter(|&i| matches!(kinds[i], Kind::Cap { .. }))
        .collect();
    if caps.len() != 2 {
        return Err(Error::Unsupported);
    }
    let (Kind::Cap { z: za, outward: oa }, Kind::Cap { z: zb, outward: ob }) =
        (kinds[caps[0]], kinds[caps[1]])
    else {
        unreachable!()
    };
    let (bottom, top) = match (oa, ob) {
        (o, p) if o < 0.0 && p > 0.0 && za < zb => (caps[0], caps[1]),
        (o, p) if o > 0.0 && p < 0.0 && zb < za => (caps[1], caps[0]),
        _ => return Err(Error::Geometry),
    };
    let z = [
        match kinds[bottom] {
            Kind::Cap { z, .. } => z,
            _ => unreachable!(),
        },
        match kinds[top] {
            Kind::Cap { z, .. } => z,
            _ => unreachable!(),
        },
    ];
    let height = z[1] - z[0];
    if !height.is_finite() || height <= 4.0 * JOIN {
        return Err(Error::Geometry);
    }

    // Arcs take their circle from the cylindrical wall they bound; the rational control net must
    // lie on that circle exactly, so no arc is ever measured as chords.
    let mut circles: BTreeMap<u64, ([f64; 2], f64, f64)> = BTreeMap::new();
    for (index, face) in faces.iter().enumerate() {
        let Kind::Cylinder { center, radius } = kinds[index] else {
            continue;
        };
        for &(id, _) in face.loops.iter().flatten() {
            match &edges[&id].shape {
                Shape::Line => {
                    let [a, b] = edges[&id].ends;
                    if !near2(xy(a), xy(b))
                        || (a[2] - b[2]).abs() <= WALL
                        || ((a[0] - center[0]).hypot(a[1] - center[1]) - radius).abs() > WALL
                    {
                        return Err(Error::Geometry);
                    }
                }
                Shape::Arc(arc) => {
                    // Cap heights carry rotation noise of a few ulps; the net must still sit on one
                    // level within the join band before it is snapped for the exact circle proof.
                    let level = z
                        .into_iter()
                        .find(|level| arc.points.iter().all(|p| (p[2] - level).abs() <= WALL))
                        .ok_or(Error::Geometry)?;
                    let flat = Arc {
                        points: arc.points.iter().map(|p| [p[0], p[1], level]).collect(),
                        weights: arc.weights.clone(),
                        knots: arc.knots.clone(),
                    };
                    let turn = signed_turn(&flat, CircularExtrusion { center, radius, z }, level)?;
                    if turn == 0.0 || turn.abs() > std::f64::consts::TAU + 1.0e-9 {
                        return Err(Error::Geometry);
                    }
                    if circles.insert(id, (center, radius, turn)).is_some() {
                        return Err(Error::Topology);
                    }
                }
            }
        }
    }
    for (index, face) in faces.iter().enumerate() {
        if let Kind::Wall { origin, outward } = kinds[index] {
            let norm = outward[0].hypot(outward[1]);
            for &(id, _) in face.loops.iter().flatten() {
                let data = &edges[&id];
                if !matches!(data.shape, Shape::Line)
                    || data.ends.iter().any(|p| {
                        ((p[0] - origin[0]) * outward[0] + (p[1] - origin[1]) * outward[1]).abs()
                            > WALL * norm
                    })
                {
                    return Err(Error::Geometry);
                }
            }
        }
    }

    let top_cap = cap_loops(&faces[top], kinds[top], &edges, &circles)?;
    let bottom_cap = cap_loops(&faces[bottom], kinds[bottom], &edges, &circles)?;
    let perimeter: f64 = top_cap
        .loops
        .iter()
        .flatten()
        .map(Segment::length)
        .sum::<f64>()
        + bottom_cap
            .loops
            .iter()
            .flatten()
            .map(Segment::length)
            .sum::<f64>();
    let area_error = perimeter * WALL + 64.0 * f64::EPSILON * top_cap.area.abs();
    if (top_cap.area - bottom_cap.area).abs() > area_error
        || top_cap.loops.len() != bottom_cap.loops.len()
    {
        return Err(Error::Geometry);
    }

    // Walls must be the exact vertical sweep of the cap trims, on the material side.
    for (cap, index) in [(&top_cap, top), (&bottom_cap, bottom)] {
        for (loop_ids, loop_) in faces[index].loops.iter().zip(&cap.loops) {
            for (&(id, _), segment) in loop_ids.iter().zip(loop_) {
                let other = uses[&id]
                    .iter()
                    .map(|&(face, _)| face)
                    .find(|&face| face != index)
                    .ok_or(Error::Topology)?;
                match (kinds[other], segment) {
                    (Kind::Wall { outward, .. }, Segment::Line { from, to }) => {
                        let d = [to[0] - from[0], to[1] - from[1]];
                        let length = d[0].hypot(d[1]);
                        let norm = outward[0].hypot(outward[1]);
                        // Material lies left of a cap traversal, so the wall's outward normal is right.
                        let right = (outward[0] * d[1] - outward[1] * d[0]) * cap.outward;
                        if right < 0.999 * length * norm {
                            return Err(Error::Geometry);
                        }
                    }
                    (Kind::Cylinder { .. }, Segment::Arc { turn, .. }) => {
                        let convex = *turn * cap.outward > 0.0;
                        if convex != faces[other].sense {
                            return Err(Error::Geometry);
                        }
                    }
                    _ => return Err(Error::Topology),
                }
            }
        }
    }
    for (index, face) in faces.iter().enumerate() {
        if matches!(kinds[index], Kind::Cap { .. }) {
            continue;
        }
        let mut levels: [Vec<Segment>; 2] = [vec![], vec![]];
        for &(id, _) in face.loops.iter().flatten() {
            let data = &edges[&id];
            let level = if (data.ends[0][2] - z[0]).abs() <= WALL
                && (data.ends[1][2] - z[0]).abs() <= WALL
            {
                0
            } else if (data.ends[0][2] - z[1]).abs() <= WALL
                && (data.ends[1][2] - z[1]).abs() <= WALL
            {
                1
            } else if matches!(data.shape, Shape::Line) && near2(xy(data.ends[0]), xy(data.ends[1]))
            {
                continue;
            } else {
                return Err(Error::Geometry);
            };
            levels[level].push(curve_segment(id, &edges, &circles, z[level])?);
        }
        let matched = levels[0].len() == levels[1].len()
            && levels[1]
                .iter()
                .all(|top| levels[0].iter().any(|bottom| same_sweep(bottom, top)));
        if !matched || levels[0].is_empty() {
            return Err(Error::Geometry);
        }
    }

    let holes = top_cap.loops.len() - 1;
    let vertices: BTreeSet<u64> = edges.values().flat_map(|e| e.vertices).collect();
    let extra_loops: usize = faces.iter().map(|f| f.loops.len() - 1).sum();
    let euler = vertices.len() as isize - edges.len() as isize + faces.len() as isize
        - extra_loops as isize;
    if euler != 2 - 2 * holes as isize {
        return Err(Error::Topology);
    }

    let mut bounds = [[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]];
    for segment in top_cap.loops.iter().flatten() {
        for p in [segment.start(), segment.end()]
            .into_iter()
            .chain(segment.extremes())
        {
            for axis in 0..2 {
                bounds[0][axis] = bounds[0][axis].min(p[axis]);
                bounds[1][axis] = bounds[1][axis].max(p[axis]);
            }
        }
    }
    let volume_error = area_error * height + 64.0 * f64::EPSILON * top_cap.area * height;
    let volume = top_cap.area * height;
    Ok(PrismCertificate {
        bounds: [
            [bounds[0][0], bounds[0][1], z[0]],
            [bounds[1][0], bounds[1][1], z[1]],
        ],
        bound_error: 8.0 * JOIN,
        area: [top_cap.area - area_error, top_cap.area + area_error],
        volume: [volume - volume_error, volume + volume_error],
        topology: [vertices.len(), edges.len(), faces.len()],
        holes,
        cylinders: kinds
            .iter()
            .filter_map(|kind| match kind {
                Kind::Cylinder { center, radius } => Some((*center, *radius)),
                _ => None,
            })
            .collect(),
        planar_walls: kinds
            .iter()
            .filter(|kind| matches!(kind, Kind::Wall { .. }))
            .count(),
        z,
        loops: top_cap.loops,
    })
}

/// One edge as a segment in its own curve sense, for comparing the two ends of a wall.
fn curve_segment(
    id: u64,
    edges: &BTreeMap<u64, EdgeData>,
    circles: &BTreeMap<u64, ([f64; 2], f64, f64)>,
    z: f64,
) -> Result<Segment> {
    let data = &edges[&id];
    if data.ends.iter().any(|p| (p[2] - z).abs() > WALL) {
        return Err(Error::Geometry);
    }
    Ok(match data.shape {
        Shape::Line => Segment::Line {
            from: xy(data.ends[0]),
            to: xy(data.ends[1]),
        },
        Shape::Arc(_) => {
            let &(center, radius, turn) = circles.get(&id).ok_or(Error::Topology)?;
            Segment::Arc {
                center,
                radius,
                from: xy(data.ends[0]),
                to: xy(data.ends[1]),
                turn,
            }
        }
    })
}

fn same_sweep(a: &Segment, b: &Segment) -> bool {
    same_oriented(a, b) || same_oriented(a, &b.reversed())
}

fn same_oriented(a: &Segment, b: &Segment) -> bool {
    match (*a, *b) {
        (Segment::Line { from: a0, to: a1 }, Segment::Line { from: b0, to: b1 }) => {
            near2(a0, b0) && near2(a1, b1)
        }
        (
            Segment::Arc {
                center: ac,
                radius: ar,
                from: a0,
                to: a1,
                turn: at,
            },
            Segment::Arc {
                center: bc,
                radius: br,
                from: b0,
                to: b1,
                turn: bt,
            },
        ) => {
            near2(ac, bc)
                && (ar - br).abs() <= WALL
                && near2(a0, b0)
                && near2(a1, b1)
                && (at - bt).abs() <= WALL
        }
        _ => false,
    }
}

pub(super) fn certify(text: &str) -> Result<Vec<PrismCertificate>> {
    let doc = parse(text)?;
    doc.check_complex_scope()?;
    for record in doc.0.values().flatten() {
        if !supported_record(record)
            && !matches!(
                record.name.as_str(),
                "CYLINDRICAL_SURFACE"
                    | "BOUNDED_CURVE"
                    | "B_SPLINE_CURVE"
                    | "B_SPLINE_CURVE_WITH_KNOTS"
                    | "CURVE"
                    | "GEOMETRIC_REPRESENTATION_ITEM"
                    | "RATIONAL_B_SPLINE_CURVE"
                    | "REPRESENTATION_ITEM"
            )
        {
            return Err(Error::Unsupported);
        }
    }
    let solids = doc.ids("MANIFOLD_SOLID_BREP");
    if solids.is_empty() {
        return Err(Error::NoSolids);
    }
    if doc
        .solid_scales(&solids)?
        .values()
        .any(|&scale| scale != 1.0)
    {
        return Err(Error::Units);
    }
    let mut seen = BTreeSet::new();
    let certificates = solids
        .into_iter()
        .map(|id| solid(&doc, id, &mut seen))
        .collect::<Result<Vec<_>>>()?;
    if seen != doc.ids("ADVANCED_FACE").into_iter().collect() {
        return Err(Error::Topology);
    }
    Ok(certificates)
}
