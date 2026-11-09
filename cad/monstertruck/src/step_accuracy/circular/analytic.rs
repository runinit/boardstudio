//! Direct analytic cylinders with authenticated3D trims or selected/alternate charts.
//! shortcut: only canonical Z cylinders with a proved seam or two periodic loops.
use super::*;
mod periodic;

fn frame(doc: &Document, id: u64) -> Result<Point> {
    let place = doc.record(id, "AXIS2_PLACEMENT_3D")?;
    let axis = if place[2] == Parameter::NotProvided {
        [0.0, 0.0, 1.0]
    } else {
        unit(vector(&doc.record(reference(&place[2])?, "DIRECTION")?[1])?)?
    };
    let x = if place[3] == Parameter::NotProvided {
        [1.0, 0.0, 0.0]
    } else {
        unit(vector(&doc.record(reference(&place[3])?, "DIRECTION")?[1])?)?
    };
    if axis != [0.0, 0.0, 1.0] || x != [1.0, 0.0, 0.0] {
        return Err(Error::Unsupported);
    }
    doc.point(reference(&place[1])?)
}

fn xy(doc: &Document, id: u64, name: &str) -> Result<[f64; 2]> {
    let values = list(&doc.record(id, name)?[1])?;
    if values.len() != 2 {
        return Err(Error::Geometry);
    }
    Ok([number(&values[0])?, number(&values[1])?])
}

fn uv_curve(doc: &Document, id: u64, used: &mut BTreeSet<u64>) -> Result<(u64, u64)> {
    used.insert(id);
    let pcurve = doc.record(id, "PCURVE")?;
    let definition = doc.record(reference(&pcurve[2])?, "DEFINITIONAL_REPRESENTATION")?;
    let curves = refs(&definition[1])?;
    if curves.len() != 1 {
        return Err(Error::Unsupported);
    }
    let context = reference(&definition[2])?;
    doc.record(context, "PARAMETRIC_REPRESENTATION_CONTEXT")?;
    if number(&doc.record(context, "GEOMETRIC_REPRESENTATION_CONTEXT")?[0])? != 2.0 {
        return Err(Error::Geometry);
    }
    used.insert(curves[0]);
    Ok((reference(&pcurve[1])?, curves[0]))
}

fn uv_line(doc: &Document, id: u64) -> Result<([f64; 2], [f64; 2])> {
    let line = doc.record(id, "LINE")?;
    let v = doc.record(reference(&line[2])?, "VECTOR")?;
    if number(&v[2])? <= 0.0 {
        return Err(Error::Geometry);
    }
    Ok((
        xy(doc, reference(&line[1])?, "CARTESIAN_POINT")?,
        xy(doc, reference(&v[1])?, "DIRECTION")?,
    ))
}

fn master(curve: &[Parameter]) -> Result<Vec<u64>> {
    let associations = refs(&curve[2])?;
    if associations.len() != 2 {
        return Err(Error::Unsupported);
    }
    match curve.get(3) {
        Some(Parameter::Enumeration(value))
            if ["CURVE_3D", "PCURVE_S1", "PCURVE_S2"].contains(&value.as_str()) =>
        {
            Ok(associations)
        }
        _ => Err(Error::Unsupported),
    }
}

fn owning_solid(doc: &Document) -> Result<u64> {
    let solids = doc.ids("MANIFOLD_SOLID_BREP");
    if solids.len() != 1 || doc.solid_scales(&solids)?[&solids[0]] != 1.0 {
        return Err(Error::Unsupported);
    }
    // The actual owning context must declare radians; unrelated global angle units do not count.
    let definitions = doc.ids("SHAPE_DEFINITION_REPRESENTATION");
    if definitions.len() != 1 {
        return Err(Error::Unsupported);
    }
    let definition = doc.record(definitions[0], "SHAPE_DEFINITION_REPRESENTATION")?;
    let representation_id = reference(&definition[1])?;
    let representation = doc
        .record(representation_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")
        .or_else(|_| doc.record(representation_id, "SHAPE_REPRESENTATION"))?;
    let units = doc.record(
        reference(&representation[2])?,
        "GLOBAL_UNIT_ASSIGNED_CONTEXT",
    )?;
    let angles: Vec<_> = refs(&units[0])?
        .into_iter()
        .filter(|&id| doc.record(id, "PLANE_ANGLE_UNIT").is_ok())
        .collect();
    if angles.len() != 1
        || doc.record(angles[0], "SI_UNIT")?
            != [
                Parameter::NotProvided,
                Parameter::Enumeration("RADIAN".into()),
            ]
    {
        return Err(Error::Units);
    }
    Ok(solids[0])
}

pub(super) fn certify(doc: &Document, shape: CircularExtrusion) -> Result<CircularCertificate> {
    // source-circle-006.step pins the rational two-loop periodic boundary route.
    if !doc.ids("B_SPLINE_CURVE").is_empty() {
        return periodic::certify(doc, shape);
    }
    for record in doc.0.values().flatten() {
        if !supported_record(record)
            && !matches!(
                record.name.as_str(),
                "CIRCLE" | "CYLINDRICAL_SURFACE" | "AXIS2_PLACEMENT_2D"
            )
        {
            return Err(Error::Unsupported);
        }
    }
    let solid = owning_solid(doc)?;
    let surfaces = doc.ids("CYLINDRICAL_SURFACE");
    if surfaces.len() != 1 || doc.ids("PLANE").len() != 2 {
        return Err(Error::Unsupported);
    }
    let cylinder = surfaces[0];
    let surface = doc.record(cylinder, "CYLINDRICAL_SURFACE")?;
    let origin = frame(doc, reference(&surface[1])?)?;
    if (number(&surface[2])? - shape.radius).abs() > JOIN
        || !near(origin, [shape.center[0], shape.center[1], shape.z[0]])
    {
        return Err(Error::Geometry);
    }
    let shell = reference(&doc.record(solid, "MANIFOLD_SOLID_BREP")?[1])?;
    let face_ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
    if face_ids.len() != 3
        || face_ids.iter().copied().collect::<BTreeSet<_>>()
            != doc.ids("ADVANCED_FACE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let faces = face_ids
        .into_iter()
        .map(|id| face(doc, id))
        .collect::<Result<Vec<_>>>()?;
    if faces.iter().any(|face| face.loops.len() != 1) {
        return Err(Error::Unsupported);
    }
    let side = faces
        .iter()
        .find(|face| face.surface == cylinder)
        .ok_or(Error::Topology)?;
    if !side.sense || side.edges.len() != 4 {
        return Err(Error::Geometry);
    }
    let mut caps = [None, None];
    for face in faces.iter().filter(|face| face.surface != cylinder) {
        let plane = doc.record(face.surface, "PLANE")?;
        let p = frame(doc, reference(&plane[1])?)?;
        let cap = shape
            .z
            .iter()
            .position(|&z| (p[2] - z).abs() <= JOIN)
            .ok_or(Error::Geometry)?;
        if face.sense != (cap == 1)
            || face.edges.len() != 1
            || face.edges[0].1 != (cap == 1)
            || caps[cap].is_some()
        {
            return Err(Error::Geometry);
        }
        caps[cap] = Some((face.surface, face.edges[0].0));
    }
    let [Some(bottom), Some(top)] = caps else {
        return Err(Error::Topology);
    };
    if bottom.0 == top.0 || bottom.1 == top.1 {
        return Err(Error::Topology);
    }
    let edge_ids = doc.ids("EDGE_CURVE");
    if edge_ids.len() != 3 {
        return Err(Error::Topology);
    }
    let seam = *edge_ids
        .iter()
        .find(|&&id| id != bottom.1 && id != top.1)
        .ok_or(Error::Topology)?;
    let expected_loop = [
        (bottom.1, true),
        (seam, true),
        (top.1, false),
        (seam, false),
    ];
    if !(0..4).any(|shift| {
        side.edges
            .iter()
            .copied()
            .eq(expected_loop.iter().cycle().skip(shift).take(4).copied())
    }) {
        return Err(Error::Topology);
    }
    let mut used = BTreeSet::new();
    let mut vertices = BTreeSet::new();
    let mut circle_vertices = [0, 0];
    for (cap, (plane_id, edge_id)) in [bottom, top].into_iter().enumerate() {
        let edge = doc.record(edge_id, "EDGE_CURVE")?;
        let start = reference(&edge[1])?;
        if start != reference(&edge[2])? || !boolean(&edge[4])? {
            return Err(Error::Unsupported);
        }
        circle_vertices[cap] = start;
        vertices.insert(start);
        let point = doc.vertex(start)?;
        if (point[2] - shape.z[cap]).abs() > JOIN
            || ((point[0] - shape.center[0]).hypot(point[1] - shape.center[1]) - shape.radius).abs()
                > JOIN
        {
            return Err(Error::Geometry);
        }
        let curve = doc.record(reference(&edge[3])?, "SURFACE_CURVE")?;
        let circle_id = reference(&curve[1])?;
        used.insert(circle_id);
        let circle = doc.record(circle_id, "CIRCLE")?;
        if (number(&circle[2])? - shape.radius).abs() > JOIN
            || !near(
                frame(doc, reference(&circle[1])?)?,
                [shape.center[0], shape.center[1], shape.z[cap]],
            )
        {
            return Err(Error::Geometry);
        }
        let mut backing = BTreeSet::new();
        for chart in master(curve)? {
            let (surface, uv) = uv_curve(doc, chart, &mut used)?;
            backing.insert(surface);
            if surface == cylinder {
                let (p, d) = uv_line(doc, uv)?;
                if d[0] <= 0.0 || d[1] != 0.0 || (p[1] + origin[2] - shape.z[cap]).abs() > JOIN {
                    return Err(Error::Geometry);
                }
            } else if surface == plane_id {
                let circle = doc.record(uv, "CIRCLE")?;
                let place = doc.record(reference(&circle[1])?, "AXIS2_PLACEMENT_2D")?;
                let p = xy(doc, reference(&place[1])?, "CARTESIAN_POINT")?;
                let plane = doc.record(plane_id, "PLANE")?;
                let base = frame(doc, reference(&plane[1])?)?;
                if place[2] != Parameter::NotProvided {
                    let d = xy(doc, reference(&place[2])?, "DIRECTION")?;
                    if d[0].hypot(d[1]) == 0.0 {
                        return Err(Error::Geometry);
                    }
                }
                if (number(&circle[2])? - shape.radius).abs() > JOIN
                    || (p[0] + base[0] - shape.center[0]).abs() > JOIN
                    || (p[1] + base[1] - shape.center[1]).abs() > JOIN
                {
                    return Err(Error::Geometry);
                }
            } else {
                return Err(Error::Geometry);
            }
        }
        if backing != [cylinder, plane_id].into_iter().collect() {
            return Err(Error::Geometry);
        }
    }
    let edge = doc.record(seam, "EDGE_CURVE")?;
    if [reference(&edge[1])?, reference(&edge[2])?] != circle_vertices || !boolean(&edge[4])? {
        return Err(Error::Topology);
    }
    let start = doc.vertex(circle_vertices[0])?;
    let end = doc.vertex(circle_vertices[1])?;
    let curve = doc.record(reference(&edge[3])?, "SEAM_CURVE")?;
    let line_id = reference(&curve[1])?;
    used.insert(line_id);
    let line = doc.record(line_id, "LINE")?;
    let v = doc.record(reference(&line[2])?, "VECTOR")?;
    let direction = vector(&doc.record(reference(&v[1])?, "DIRECTION")?[1])?;
    if number(&v[2])? <= 0.0 || direction[0] != 0.0 || direction[1] != 0.0 || direction[2] <= 0.0 {
        return Err(Error::Geometry);
    }
    let line_origin = doc.point(reference(&line[1])?)?;
    for point in [start, end] {
        if !near([line_origin[0], line_origin[1], point[2]], point) {
            return Err(Error::Geometry);
        }
    }
    check_supporting_line(line_origin, direction, start, end)?;
    let mut seam_angles = Vec::new();
    for chart in master(curve)? {
        let (surface, uv) = uv_curve(doc, chart, &mut used)?;
        if surface != cylinder {
            return Err(Error::Geometry);
        }
        let (p, d) = uv_line(doc, uv)?;
        if d[0] != 0.0 || d[1] <= 0.0 {
            return Err(Error::Geometry);
        }
        for point in [start, end] {
            if !near(
                [
                    origin[0] + shape.radius * p[0].cos(),
                    origin[1] + shape.radius * p[0].sin(),
                    point[2],
                ],
                point,
            ) {
                return Err(Error::Geometry);
            }
        }
        seam_angles.push(p[0]);
    }
    if ((seam_angles[1] - seam_angles[0]).abs() - std::f64::consts::TAU).abs() > JOIN {
        return Err(Error::Geometry);
    }
    if vertices != doc.ids("VERTEX_POINT").into_iter().collect()
        || used
            != doc
                .ids("LINE")
                .into_iter()
                .chain(doc.ids("CIRCLE"))
                .chain(doc.ids("PCURVE"))
                .collect()
    {
        return Err(Error::Topology);
    }
    let height = shape.z[1] - shape.z[0];
    let volume = std::f64::consts::PI * shape.radius * shape.radius * height;
    let error = 16.0 * std::f64::consts::PI * shape.radius * height.max(shape.radius) * JOIN
        + 64.0 * f64::EPSILON * volume;
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
        bound_error: 8.0 * JOIN,
        volume: [volume - error, volume + error],
        topology: [2, 3, 3],
        shape,
        inner_radius: None,
    })
}
