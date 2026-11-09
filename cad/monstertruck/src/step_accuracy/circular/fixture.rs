//! Compare emitted faces with independently authored fixture trims. No general validity claim.
use super::*;

const MATCH: f64 = 8.0 * JOIN;

fn close(a: Point, b: Point) -> bool {
    a.into_iter().zip(b).all(|(a, b)| (a - b).abs() <= MATCH)
}

fn surface(doc: &Document, face: &Face, expected: &FixtureSurface) -> Result<()> {
    match *expected {
        FixtureSurface::Plane { origin, outward } => {
            let (actual_origin, actual_normal) = doc.plane(face.surface)?;
            let normal = unit(actual_normal)?.map(|x| x * if face.sense { 1. } else { -1. });
            if !close(normal, outward) || dot(sub(actual_origin, origin), outward).abs() > MATCH {
                return Err(Error::Geometry);
            }
        }
        FixtureSurface::Cylinder {
            center,
            radius,
            outward,
            tolerance,
        } => {
            let cylinder = doc.record(face.surface, "CYLINDRICAL_SURFACE")?;
            let place = doc.record(reference(&cylinder[1])?, "AXIS2_PLACEMENT_3D")?;
            let axis = unit(vector(&doc.record(reference(&place[2])?, "DIRECTION")?[1])?)?;
            let origin = doc.point(reference(&place[1])?)?;
            if axis != [0., 0., 1.]
                || face.sense != outward
                || (number(&cylinder[2])? - radius).abs() > tolerance
                || (origin[0] - center[0]).abs() > tolerance
                || (origin[1] - center[1]).abs() > tolerance
            {
                return Err(Error::Geometry);
            }
        }
    }
    Ok(())
}

fn edge(doc: &Document, id: u64, forward: bool, expected: &FixtureEdge) -> Result<()> {
    let data = doc.record(id, "EDGE_CURVE")?;
    if !boolean(&data[4])? {
        return Err(Error::Unsupported);
    }
    let ends = [
        doc.vertex(reference(&data[1])?)?,
        doc.vertex(reference(&data[2])?)?,
    ];
    let directed = if forward { ends } else { [ends[1], ends[0]] };
    let (from, to) = match *expected {
        FixtureEdge::Line { from, to } | FixtureEdge::Arc { from, to, .. } => (from, to),
    };
    if !close(directed[0], from) || !close(directed[1], to) {
        return Err(Error::Geometry);
    }
    let curve = doc.record(reference(&data[3])?, "SURFACE_CURVE")?;
    if curve[3] != Parameter::Enumeration("CURVE_3D".into()) {
        return Err(Error::Unsupported);
    }
    let curve = reference(&curve[1])?;
    match *expected {
        FixtureEdge::Line { .. } => {
            let line = doc.record(curve, "LINE")?;
            let origin = doc.point(reference(&line[1])?)?;
            let v = doc.record(reference(&line[2])?, "VECTOR")?;
            let direction = unit(vector(&doc.record(reference(&v[1])?, "DIRECTION")?[1])?)?;
            if number(&v[2])? <= 0. || dot(direction, unit(sub(ends[1], ends[0]))?) <= 0. {
                return Err(Error::Geometry);
            }
            check_supporting_line(origin, direction, ends[0], ends[1])?;
        }
        FixtureEdge::Arc {
            from,
            to: _,
            center,
            radius,
            turn,
        } => {
            let mut arc = Arc::from_curve(doc, curve)?;
            if !close(arc.points[0], ends[0])
                || !close(*arc.points.last().ok_or(Error::Geometry)?, ends[1])
                || arc.points.iter().any(|p| (p[2] - from[2]).abs() > JOIN)
            {
                return Err(Error::Geometry);
            }
            // Only remove a few ulps in a nominally horizontal reference trim. This matcher
            // compares coordinates; it does not turn that adjustment into an accuracy bound.
            for point in &mut arc.points {
                point[2] = from[2];
            }
            let actual = signed_turn(
                &arc,
                CircularExtrusion {
                    center,
                    radius,
                    z: [from[2]; 2],
                },
                from[2],
            )? * if forward { 1. } else { -1. };
            if (actual - turn).abs() > MATCH {
                return Err(Error::Geometry);
            }
        }
    }
    Ok(())
}

fn trim_loop(doc: &Document, actual: &[(u64, bool)], expected: &[FixtureEdge]) -> Result<()> {
    if actual.len() < expected.len() || actual.is_empty() {
        return Err(Error::Topology);
    }
    let vertices = actual
        .iter()
        .map(|&(id, forward)| {
            let data = doc.record(id, "EDGE_CURVE")?;
            let ends = [reference(&data[1])?, reference(&data[2])?];
            Ok(if forward { ends } else { [ends[1], ends[0]] })
        })
        .collect::<Result<Vec<_>>>()?;
    if vertices
        .iter()
        .zip(vertices.iter().cycle().skip(1))
        .any(|(a, b)| a[1] != b[0])
    {
        return Err(Error::Topology);
    }
    let points = vertices
        .iter()
        .map(|v| Ok([doc.vertex(v[0])?, doc.vertex(v[1])?]))
        .collect::<Result<Vec<_>>>()?;
    // Cyclic starts and collinear line subdivisions change representation only. Every actual
    // use must still traverse the same reference segment monotonically, with no gap or extra trim.
    let matches = |offset: usize| -> Result<()> {
        let mut used = 0;
        for expected in expected {
            if let FixtureEdge::Line { from, to } = *expected {
                let d = sub(to, from);
                let length2 = dot(d, d);
                if length2 == 0. {
                    return Err(Error::Geometry);
                }
                let mut previous = from;
                loop {
                    if used == actual.len() {
                        return Err(Error::Topology);
                    }
                    let index = (offset + used) % actual.len();
                    let [a, b] = points[index];
                    let t = dot(sub(b, from), d) / length2;
                    let projected = std::array::from_fn(|i| from[i] + t * d[i]);
                    if !close(a, previous)
                        || !close(b, projected)
                        || dot(sub(b, a), d) <= 0.
                        || t > 1. + MATCH / length2.sqrt()
                    {
                        return Err(Error::Geometry);
                    }
                    let (id, forward) = actual[index];
                    edge(doc, id, forward, &FixtureEdge::Line { from: a, to: b })?;
                    used += 1;
                    if close(b, to) {
                        break;
                    }
                    previous = b;
                }
            } else {
                if used == actual.len() {
                    return Err(Error::Topology);
                }
                let (id, forward) = actual[(offset + used) % actual.len()];
                edge(doc, id, forward, expected)?;
                used += 1;
            }
        }
        if used == actual.len() {
            Ok(())
        } else {
            Err(Error::Topology)
        }
    };
    if (0..actual.len()).any(|offset| matches(offset).is_ok()) {
        Ok(())
    } else {
        Err(Error::Geometry)
    }
}

fn face_matches(doc: &Document, actual: &Face, expected: &FixtureFace) -> Result<()> {
    surface(doc, actual, &expected.surface)?;
    if actual.loops.len() != expected.loops.len() {
        return Err(Error::Topology);
    }
    let mut remaining: Vec<_> = actual.loops.iter().collect();
    for loop_ in &expected.loops {
        let at = remaining
            .iter()
            .position(|actual| trim_loop(doc, actual, loop_).is_ok())
            .ok_or(Error::Geometry)?;
        remaining.remove(at);
    }
    Ok(())
}

pub(super) fn compare(text: &str, expected: &[(&str, Vec<FixtureFace>)]) -> Result<()> {
    let doc = parse(text)?;
    let mut products = direct_products(&doc)?;
    if products.len() != expected.len() {
        return Err(Error::Topology);
    }
    doc.check_complex_scope()?;
    let solids = doc.ids("MANIFOLD_SOLID_BREP");
    if solids.len() != expected.len() {
        return Err(Error::Topology);
    }
    if doc.solid_scales(&solids)?.values().any(|&s| s != 1.) {
        return Err(Error::Units);
    }
    let mut seen = BTreeSet::new();
    for (product, expected) in expected {
        let (_, owned) = products.remove(*product).ok_or(Error::Topology)?;
        if owned.len() != 1 {
            return Err(Error::Topology);
        }
        let id = owned[0];
        let shell = reference(&doc.record(id, "MANIFOLD_SOLID_BREP")?[1])?;
        let ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
        if ids.len() != expected.len() || ids.iter().any(|id| !seen.insert(*id)) {
            return Err(Error::Topology);
        }
        let faces = ids
            .iter()
            .map(|&id| face_with_bounds(&doc, id, usize::MAX))
            .collect::<Result<Vec<_>>>()?;
        let mut uses: BTreeMap<u64, Vec<(u64, bool)>> = BTreeMap::new();
        for face in &faces {
            for &(id, forward) in face.loops.iter().flatten() {
                uses.entry(id).or_default().push((face.surface, forward));
            }
        }
        for (id, uses) in uses {
            if uses.len() != 2 || uses[0].1 == uses[1].1 {
                return Err(Error::Topology);
            }
            let data = doc.record(id, "EDGE_CURVE")?;
            let curve = doc.record(reference(&data[3])?, "SURFACE_CURVE")?;
            let associated = refs(&curve[2])?;
            if associated.len() != 2
                || associated.into_iter().collect::<BTreeSet<_>>()
                    != uses.iter().map(|u| u.0).collect()
            {
                return Err(Error::Topology);
            }
        }
        let mut remaining: Vec<_> = faces.iter().collect();
        for expected in expected {
            let at = remaining
                .iter()
                .position(|face| face_matches(&doc, face, expected).is_ok())
                .ok_or(Error::Geometry)?;
            remaining.remove(at);
        }
    }
    if seen != doc.ids("ADVANCED_FACE").into_iter().collect() {
        return Err(Error::Topology);
    }
    Ok(())
}
