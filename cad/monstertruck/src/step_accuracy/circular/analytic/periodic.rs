//! Full periodic Z cylinder with two rational circular boundary loops, no PCUV masters.
use super::*;

pub(super) fn certify(doc: &Document, shape: CircularExtrusion) -> Result<CircularCertificate> {
    for record in doc.0.values().flatten() {
        if !supported_record(record)
            && !matches!(
                record.name.as_str(),
                "CYLINDRICAL_SURFACE"
                    | "BOUNDED_CURVE"
                    | "B_SPLINE_CURVE"
                    | "B_SPLINE_CURVE_WITH_KNOTS"
                    | "CURVE"
                    | "RATIONAL_B_SPLINE_CURVE"
                    | "GEOMETRIC_REPRESENTATION_ITEM"
                    | "REPRESENTATION_ITEM"
            )
        {
            return Err(Error::Unsupported);
        }
    }
    if !doc.ids("PCURVE").is_empty()
        || !doc.ids("SEAM_CURVE").is_empty()
        || !doc.ids("LINE").is_empty()
        || doc.ids("CLOSED_SHELL").len() != 1
        || doc.ids("PLANE").len() != 2
        || doc.ids("CYLINDRICAL_SURFACE").len() != 1
    {
        return Err(Error::Unsupported);
    }
    let solid = owning_solid(doc)?;
    let cylinder = doc.ids("CYLINDRICAL_SURFACE")[0];
    let surface = doc.record(cylinder, "CYLINDRICAL_SURFACE")?;
    let origin = frame(doc, reference(&surface[1])?)?;
    // With3D masters and full periodic trims, moving the origin along this axis
    // only reparameterizes the same cylinder. Cap/trim heights are proved below.
    if (number(&surface[2])? - shape.radius).abs() > JOIN
        || (0..2).any(|axis| (origin[axis] - shape.center[axis]).abs() > JOIN)
    {
        return Err(Error::Geometry);
    }
    let shell = reference(&doc.record(solid, "MANIFOLD_SOLID_BREP")?[1])?;
    let ids = refs(&doc.record(shell, "CLOSED_SHELL")?[1])?;
    if ids.len() != 3
        || ids.iter().copied().collect::<BTreeSet<_>>()
            != doc.ids("ADVANCED_FACE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let faces = ids
        .into_iter()
        .map(|id| face(doc, id))
        .collect::<Result<Vec<_>>>()?;
    let mut edges = BTreeMap::new();
    let mut vertices = BTreeSet::new();
    let mut carriers = BTreeSet::new();
    let mut curves = BTreeSet::new();
    let mut turns = BTreeMap::new();
    let mut uses: BTreeMap<u64, Vec<(u64, bool)>> = BTreeMap::new();
    for face in &faces {
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
            let curve_id = reference(&record[3])?;
            curves.insert(curve_id);
            let curve = doc.record(curve_id, "SURFACE_CURVE")?;
            if curve[3] != Parameter::Enumeration("CURVE_3D".into()) {
                return Err(Error::Unsupported);
            }
            let associated: BTreeSet<_> = refs(&curve[2])?.into_iter().collect();
            if associated.len() != 2 {
                return Err(Error::Topology);
            }
            let carrier = reference(&curve[1])?;
            carriers.insert(carrier);
            let arc = Arc::from_curve(doc, carrier)?;
            // Both vertices, including the second endpoint of either cap, must
            // attach to the actual authored3D carrier under the unchanged JOIN.
            if !near(doc.vertex(ends[0])?, arc.points[0])
                || !near(
                    doc.vertex(ends[1])?,
                    *arc.points.last().ok_or(Error::Geometry)?,
                )
            {
                return Err(Error::Geometry);
            }
            let level = shape
                .z
                .iter()
                .position(|z| *z == arc.points[0][2])
                .ok_or(Error::Geometry)?;
            turns.insert(id, (level, signed_turn(&arc, shape, shape.z[level])?));
            edges.insert(
                id,
                Edge {
                    vertices: ends,
                    associated,
                },
            );
        }
        for boundary in &face.loops {
            if boundary.len() < 2 {
                return Err(Error::Unsupported);
            }
            let mut starts = BTreeSet::new();
            for ((a, fa), (b, fb)) in boundary
                .iter()
                .zip(boundary.iter().cycle().skip(1))
                .take(boundary.len())
            {
                if !starts.insert(edges[a].vertices[usize::from(!*fa)])
                    || edges[a].vertices[usize::from(*fa)] != edges[b].vertices[usize::from(!*fb)]
                {
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
        || carriers != doc.ids("B_SPLINE_CURVE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let mut caps = [None, None];
    let mut cap_error = [0.0, 0.0];
    let mut side_seen = false;
    let mut planes = BTreeSet::new();
    for face in &faces {
        let side = face.surface == cylinder;
        if side {
            if side_seen || !face.sense || face.loops.len() != 2 {
                return Err(Error::Geometry);
            }
            side_seen = true;
        } else if face.loops.len() != 1 || !planes.insert(face.surface) {
            return Err(Error::Topology);
        }
        let mut levels = BTreeSet::new();
        for boundary in &face.loops {
            let level = turns[&boundary[0].0].0;
            if !levels.insert(level) || boundary.iter().any(|(id, _)| turns[id].0 != level) {
                return Err(Error::Topology);
            }
            let positive = if side { level == 0 } else { level == 1 };
            // Whole-span arcs must progress in one direction; net turn alone
            // permits canceling retraces through geometrically aliased vertices.
            if boundary.iter().any(|(id, forward)| {
                (turns[id].1 * if *forward { 1.0 } else { -1.0 } > 0.0) != positive
            }) {
                return Err(Error::Topology);
            }
            let turn: f64 = boundary
                .iter()
                .map(|(id, forward)| turns[id].1 * if *forward { 1.0 } else { -1.0 })
                .sum();
            let target = if positive {
                std::f64::consts::TAU
            } else {
                -std::f64::consts::TAU
            };
            if (turn - target).abs() > 1.0e-8 {
                return Err(Error::Geometry);
            }
            if !side {
                let (origin, normal) = doc.plane(face.surface)?;
                let (cap, band) = cap_plane_band(origin, normal, shape)?;
                if cap != level
                    || ((normal[2] > 0.0) == face.sense) != (level == 1)
                    || caps[level].replace(face.surface).is_some()
                {
                    return Err(Error::Geometry);
                }
                cap_error[level] = band;
            }
        }
    }
    if !side_seen
        || caps.iter().any(Option::is_none)
        || planes != doc.ids("PLANE").into_iter().collect()
    {
        return Err(Error::Topology);
    }
    let height = shape.z[1] - shape.z[0];
    let radius = shape.radius;
    let radial_error =
        radial_squared_band(radius) / radius + (4.0 * edges.len() as f64 + 4.0) * JOIN;
    let axial_error: f64 = cap_error.iter().sum::<f64>() + 4.0 * JOIN;
    let volume = std::f64::consts::PI * radius * radius * height;
    let error =
        std::f64::consts::PI * (2.0 * radius * radial_error + radial_error * radial_error) * height
            + 1.0e-8 * (radius * radius + radial_squared_band(radius)) * height
            + std::f64::consts::PI * (radius + radial_error).powi(2) * axial_error
            + 64.0 * f64::EPSILON * volume
            + 1.0e-10;
    if !volume.is_finite() || volume <= 0.0 || !error.is_finite() {
        return Err(Error::Geometry);
    }
    Ok(CircularCertificate {
        bounds: [
            [
                shape.center[0] - radius,
                shape.center[1] - radius,
                shape.z[0],
            ],
            [
                shape.center[0] + radius,
                shape.center[1] + radius,
                shape.z[1],
            ],
        ],
        bound_error: radial_error + axial_error,
        volume: [volume - error, volume + error],
        topology: [vertices.len(), edges.len(), faces.len()],
        shape,
        inner_radius: None,
    })
}
