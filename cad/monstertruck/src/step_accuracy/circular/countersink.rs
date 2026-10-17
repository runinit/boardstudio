//! Bounded source proof for a rectangular plate with one contained blind conical seat.
use super::*;

fn strict_line(doc: &Document, id: u64, ends: [Point; 2]) -> Result<()> {
    let line = doc.record(id, "LINE")?;
    let origin = doc.point(reference(&line[1])?)?;
    let vector_record = doc.record(reference(&line[2])?, "VECTOR")?;
    if number(&vector_record[2])? <= 0.0 {
        return Err(Error::Geometry);
    }
    let direction = vector(&doc.record(reference(&vector_record[1])?, "DIRECTION")?[1])?;
    let direction = direction
        .map(exact)
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
    let length2: Q = direction.iter().map(|v| v * v).sum();
    if length2 == integer(0) {
        return Err(Error::Geometry);
    }
    for point in ends {
        let delta = (0..3)
            .map(|i| Ok(exact(point[i])? - exact(origin[i])?))
            .collect::<Result<Vec<_>>>()?;
        let distance2: Q = (0..3)
            .map(|i| {
                let a = (i + 1) % 3;
                let b = (i + 2) % 3;
                let cross = &delta[a] * &direction[b] - &delta[b] * &direction[a];
                &cross * &cross
            })
            .sum();
        if distance2 > exact(JOIN)? * exact(JOIN)? * &length2 {
            return Err(Error::Geometry);
        }
    }
    Ok(())
}

// The exact positive-weight two-row tensor is linear in height. Bound its radial
// residual and angular derivative over BOTH parameters, not only its cap rows.
fn cone(doc: &Document, id: u64, shape: Countersink) -> Result<f64> {
    let data = doc.record(id, "B_SPLINE_SURFACE")?;
    if number(&data[0])? != 1.0 || number(&data[1])? != 2.0 {
        return Err(Error::Unsupported);
    }
    let rows = list(&data[2])?
        .iter()
        .map(|p| {
            refs(p)?
                .into_iter()
                .map(|id| doc.point(id))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let weights = list(&doc.record(id, "RATIONAL_B_SPLINE_SURFACE")?[0])?
        .iter()
        .map(|p| list(p)?.iter().map(number).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    let knot = doc.record(id, "B_SPLINE_SURFACE_WITH_KNOTS")?;
    let numbers = |p: &Parameter| list(p)?.iter().map(number).collect::<Result<Vec<_>>>();
    if rows.len() != 2
        || rows.iter().any(|row| row.len() != 9)
        || weights.len() != 2
        || weights.iter().any(|row| row.len() != 9)
        || weights[0] != weights[1]
        || weights[0].iter().any(|w| *w <= 0.0)
        || numbers(&knot[0])? != [2., 2.]
        || numbers(&knot[1])? != [3., 2., 2., 2., 3.]
        || numbers(&knot[2])? != [0., 1.]
        || numbers(&knot[3])? != [0., 0.25, 0.5, 0.75, 1.]
    {
        return Err(Error::Unsupported);
    }
    let directions = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    for (row, points) in rows.iter().enumerate() {
        for (point, direction) in points.iter().zip(directions) {
            if point[2] != shape.z[row]
                || (0..2).any(|axis| {
                    (point[axis] - (shape.center[axis] + shape.radii[row] * direction[axis])).abs()
                        > JOIN
                })
            {
                return Err(Error::Geometry);
            }
        }
    }
    let minimum = weights[0].iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_radius = shape.radii[0].max(shape.radii[1]);
    let band = radial_squared_band(maximum_radius);
    for quarter in 0..4 {
        let offset = quarter * 2;
        let homogeneous = |row: usize, axis: usize| -> Result<Polynomial> {
            let p = (0..3)
                .map(|i| {
                    let w = exact(weights[row][offset + i])?;
                    Ok(if axis == 2 {
                        w
                    } else {
                        (exact(rows[row][offset + i][axis])? - exact(shape.center[axis])?) * w
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(vec![
                p[0].clone(),
                integer(2) * (&p[1] - &p[0]),
                &p[0] - integer(2) * &p[1] + &p[2],
            ])
        };
        let w = homogeneous(0, 2)?;
        let xy = (0..2)
            .map(|axis| {
                let a = homogeneous(0, axis)?;
                let b = homogeneous(1, axis)?;
                Ok([a.clone(), add(&b, &scale(&a, &integer(-1)))])
            })
            .collect::<Result<Vec<_>>>()?;
        let radius = [
            exact(shape.radii[0])?,
            exact(shape.radii[1])? - exact(shape.radii[0])?,
        ];
        let mut residual = vec![vec![integer(0)]; 3];
        let mut torque = vec![vec![integer(0)]; 3];
        for u in 0..2 {
            for v in 0..2 {
                residual[u + v] = add(
                    &residual[u + v],
                    &add(
                        &add(&mul(&xy[0][u], &xy[0][v]), &mul(&xy[1][u], &xy[1][v])),
                        &scale(&mul(&w, &w), &(-&radius[u] * &radius[v])),
                    ),
                );
                torque[u + v] = add(
                    &torque[u + v],
                    &add(
                        &mul(&xy[0][u], &derivative(&xy[1][v])),
                        &scale(&mul(&xy[1][u], &derivative(&xy[0][v])), &integer(-1)),
                    ),
                );
            }
        }
        let error: Q = residual.iter().flatten().map(Signed::abs).sum();
        if error > exact(band)? * exact(minimum)? * exact(minimum)? {
            return Err(Error::Geometry);
        }
        for j in 0..=2 {
            let mut polynomial = vec![integer(0)];
            for (k, coefficient) in torque.iter().enumerate().take(j + 1) {
                polynomial = add(
                    &polynomial,
                    &scale(
                        coefficient,
                        &(integer(choose(j, k)) / integer(choose(2, k))),
                    ),
                );
            }
            if !positive_bernstein(&polynomial) {
                return Err(Error::Geometry);
            }
        }
        // The quadrant template and positive weights keep each complete patch in
        // its open diagonal half-plane. There is one ordered turn, without folds.
        let diagonal = [
            directions[offset][0] + directions[offset + 2][0],
            directions[offset][1] + directions[offset + 2][1],
        ];
        for row in 0..2 {
            let projection = add(
                &scale(&homogeneous(row, 0)?, &exact(diagonal[0])?),
                &scale(&homogeneous(row, 1)?, &exact(diagonal[1])?),
            );
            if !positive_bernstein(&projection) {
                return Err(Error::Geometry);
            }
        }
    }
    Ok(band / shape.radii[0].min(shape.radii[1]))
}

fn rectangle(
    doc: &Document,
    boundary: &[(u64, bool)],
    edges: &BTreeMap<u64, Edge>,
    shape: Countersink,
    axis: usize,
    normal: Point,
) -> Result<()> {
    let a = (axis + 1) % 3;
    let b = (axis + 2) % 3;
    let center: Point = std::array::from_fn(|i| (shape.bounds[0][i] + shape.bounds[1][i]) / 2.0);
    let mut area = integer(0);
    for &(id, forward) in boundary {
        let edge = &edges[&id];
        let start = doc.vertex(edge.vertices[usize::from(!forward)])?;
        let end = doc.vertex(edge.vertices[usize::from(forward)])?;
        if (0..3).any(|i| {
            start[i] < shape.bounds[0][i]
                || start[i] > shape.bounds[1][i]
                || end[i] < shape.bounds[0][i]
                || end[i] > shape.bounds[1][i]
        }) || !((start[a] == end[a]
            && [shape.bounds[0][a], shape.bounds[1][a]].contains(&start[a]))
            ^ (start[b] == end[b] && [shape.bounds[0][b], shape.bounds[1][b]].contains(&start[b])))
            || dot(cross(sub(start, center), sub(end, center)), normal) <= 0.0
        {
            return Err(Error::Geometry);
        }
        area += exact(start[a])? * exact(end[b])? - exact(start[b])? * exact(end[a])?;
    }
    let expected = integer(2)
        * (exact(shape.bounds[1][a])? - exact(shape.bounds[0][a])?)
        * (exact(shape.bounds[1][b])? - exact(shape.bounds[0][b])?);
    if area.abs() != expected {
        return Err(Error::Geometry);
    }
    Ok(())
}

pub(super) fn certify(text: &str, shape: Countersink) -> Result<CountersinkCertificate> {
    if !shape
        .bounds
        .into_iter()
        .flatten()
        .chain(shape.center)
        .chain(shape.z)
        .chain(shape.radii)
        .all(f64::is_finite)
        || (0..3).any(|i| shape.bounds[1][i] <= shape.bounds[0][i])
        || shape.radii.iter().any(|r| *r <= 1e-6)
        || shape.z[0] <= shape.bounds[0][2]
        || shape.z[1] != shape.bounds[1][2]
        || shape.z[1] - shape.z[0] <= 4.0 * JOIN
    {
        return Err(Error::Geometry);
    }
    let max_radius = shape.radii[0].max(shape.radii[1]);
    if (0..2).any(|i| {
        shape.center[i] - max_radius <= shape.bounds[0][i] + 4.0 * JOIN
            || shape.center[i] + max_radius >= shape.bounds[1][i] - 4.0 * JOIN
    }) {
        return Err(Error::Geometry);
    }
    let doc = parse(text)?;
    doc.check_complex_scope()?;
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
    if !doc.ids("PCURVE").is_empty() || !doc.ids("SEAM_CURVE").is_empty() {
        return Err(Error::Unsupported);
    }
    let solids = doc.ids("MANIFOLD_SOLID_BREP");
    if solids.len() != 1 || doc.ids("CLOSED_SHELL").len() != 1 {
        return Err(Error::Unsupported);
    }
    if doc.solid_scales(&solids)?[&solids[0]] != 1.0 {
        return Err(Error::Units);
    }
    let shell = reference(&doc.record(solids[0], "MANIFOLD_SOLID_BREP")?[1])?;
    let ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
    if ids.len() != 8
        || ids.iter().copied().collect::<BTreeSet<_>>()
            != doc.ids("ADVANCED_FACE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let faces = ids
        .into_iter()
        .map(|id| face(&doc, id))
        .collect::<Result<Vec<_>>>()?;
    let mut edges = BTreeMap::new();
    let mut vertices = BTreeSet::new();
    let mut carriers = BTreeSet::new();
    let mut curves = BTreeSet::new();
    let mut uses: BTreeMap<u64, Vec<(u64, bool)>> = BTreeMap::new();
    let mut turns = BTreeMap::new();
    for face in &faces {
        for &(id, forward) in &face.edges {
            uses.entry(id).or_default().push((face.surface, forward));
            if edges.contains_key(&id) {
                continue;
            }
            let edge = doc.record(id, "EDGE_CURVE")?;
            if !boolean(&edge[4])? {
                return Err(Error::Unsupported);
            }
            let ends = [reference(&edge[1])?, reference(&edge[2])?];
            vertices.extend(ends);
            let curve_id = reference(&edge[3])?;
            curves.insert(curve_id);
            let curve = doc.record(curve_id, "SURFACE_CURVE")?;
            if curve[3] != Parameter::Enumeration("CURVE_3D".into()) {
                return Err(Error::Unsupported);
            }
            let associated: BTreeSet<_> = refs(&curve[2])?.into_iter().collect();
            if associated.len() != 2 {
                return Err(Error::Unsupported);
            }
            let carrier = reference(&curve[1])?;
            carriers.insert(carrier);
            let points = [doc.vertex(ends[0])?, doc.vertex(ends[1])?];
            if doc.record(carrier, "B_SPLINE_CURVE").is_ok() {
                let arc = Arc::from_curve(&doc, carrier)?;
                if !near(points[0], arc.points[0])
                    || !near(points[1], *arc.points.last().ok_or(Error::Geometry)?)
                {
                    return Err(Error::Geometry);
                }
                let level = shape
                    .z
                    .iter()
                    .position(|z| *z == arc.points[0][2])
                    .ok_or(Error::Geometry)?;
                let turn = signed_turn(
                    &arc,
                    CircularExtrusion {
                        center: shape.center,
                        radius: shape.radii[level],
                        z: shape.z,
                    },
                    shape.z[level],
                )?;
                turns.insert(id, (level, turn));
            } else {
                strict_line(&doc, carrier, points)?;
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
            for ((a, fa), (b, fb)) in boundary
                .iter()
                .zip(boundary.iter().cycle().skip(1))
                .take(boundary.len())
            {
                if edges[a].vertices[usize::from(*fa)] != edges[b].vertices[usize::from(!*fb)] {
                    return Err(Error::Topology);
                }
            }
        }
    }
    if uses.iter().any(|(id, u)| {
        u.len() != 2
            || u[0].1 == u[1].1
            || u.iter().map(|v| v.0).collect::<BTreeSet<_>>() != edges[id].associated
    }) || edges.keys().copied().collect::<BTreeSet<_>>()
        != doc.ids("EDGE_CURVE").into_iter().collect()
        || vertices != doc.ids("VERTEX_POINT").into_iter().collect()
        || curves != doc.ids("SURFACE_CURVE").into_iter().collect()
        || carriers
            != doc
                .ids("LINE")
                .into_iter()
                .chain(doc.ids("B_SPLINE_CURVE"))
                .collect()
    {
        return Err(Error::Topology);
    }
    let mut planes = BTreeSet::new();
    let mut tensors = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut cone_error = 0.0;
    for face in &faces {
        let mut circular = Vec::new();
        let mut rectangular = Vec::new();
        for boundary in &face.loops {
            if boundary.iter().all(|(id, _)| turns.contains_key(id)) {
                let level = turns[&boundary[0].0].0;
                if boundary.iter().any(|(id, _)| turns[id].0 != level) {
                    return Err(Error::Topology);
                }
                let turn: f64 = boundary
                    .iter()
                    .map(|(id, f)| turns[id].1 * if *f { 1.0 } else { -1.0 })
                    .sum();
                if (turn.abs() - std::f64::consts::TAU).abs() > 1e-8 {
                    return Err(Error::Geometry);
                }
                circular.push((level, turn));
            } else if boundary.iter().all(|(id, _)| !turns.contains_key(id)) {
                rectangular.push(boundary);
            } else {
                return Err(Error::Unsupported);
            }
        }
        if doc.record(face.surface, "PLANE").is_ok() {
            if !planes.insert(face.surface) {
                return Err(Error::Topology);
            }
            let (origin, natural) = doc.plane(face.surface)?;
            let axis = (0..3).find(|i| natural[*i] != 0.0).ok_or(Error::Geometry)?;
            if (0..3).any(|i| i != axis && natural[i] != 0.0) {
                return Err(Error::Unsupported);
            }
            let normal = natural.map(|v| v * if face.sense { 1.0 } else { -1.0 });
            let role = if axis == 2 && origin[2] == shape.z[0] {
                6
            } else if origin[axis] == shape.bounds[0][axis] && normal[axis] < 0.0 {
                axis * 2
            } else if origin[axis] == shape.bounds[1][axis] && normal[axis] > 0.0 {
                axis * 2 + 1
            } else {
                return Err(Error::Geometry);
            };
            if !roles.insert(role) {
                return Err(Error::Topology);
            }
            for &(id, _) in &face.edges {
                for vertex in edges[&id].vertices {
                    if doc.vertex(vertex)?[axis] != origin[axis] {
                        return Err(Error::Geometry);
                    }
                }
            }
            match role {
                6 if rectangular.is_empty()
                    && circular.len() == 1
                    && circular[0].0 == 0
                    && circular[0].1 > 0.0
                    && normal[2] > 0.0 => {}
                5 if rectangular.len() == 1
                    && circular.len() == 1
                    && circular[0].0 == 1
                    && circular[0].1 < 0.0 =>
                {
                    rectangle(&doc, rectangular[0], &edges, shape, axis, normal)?
                }
                0..=4 if rectangular.len() == 1 && circular.is_empty() => {
                    rectangle(&doc, rectangular[0], &edges, shape, axis, normal)?
                }
                _ => return Err(Error::Geometry),
            }
        } else {
            if !tensors.insert(face.surface)
                || !face.sense
                || !rectangular.is_empty()
                || circular.len() != 2
                || !circular.iter().any(|(z, t)| *z == 0 && *t < 0.0)
                || !circular.iter().any(|(z, t)| *z == 1 && *t > 0.0)
            {
                return Err(Error::Geometry);
            }
            cone_error = cone(&doc, face.surface, shape)?;
        }
    }
    if roles.len() != 7
        || tensors.len() != 1
        || planes != doc.ids("PLANE").into_iter().collect()
        || tensors != doc.ids("B_SPLINE_SURFACE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let bound_error = cone_error + edges.len() as f64 * 4.0 * JOIN;
    let height = shape.z[1] - shape.z[0];
    let removed = std::f64::consts::PI
        * height
        * (shape.radii[0] * shape.radii[0]
            + shape.radii[0] * shape.radii[1]
            + shape.radii[1] * shape.radii[1])
        / 3.0;
    let volume = (0..3)
        .map(|i| shape.bounds[1][i] - shape.bounds[0][i])
        .product::<f64>()
        - removed;
    let error = std::f64::consts::PI
        * (2.0 * max_radius * bound_error + bound_error * bound_error)
        * height
        + 4.0e-8 * max_radius * max_radius * height
        + 64.0 * f64::EPSILON * volume;
    if !volume.is_finite() || volume <= 0.0 || !error.is_finite() {
        return Err(Error::Geometry);
    }
    Ok(CountersinkCertificate {
        bounds: shape.bounds,
        bound_error,
        volume: [volume - error, volume + error],
        topology: [vertices.len(), edges.len(), faces.len()],
        shape,
    })
}
