use super::*;

mod cache;
mod planar_mesh;
pub use cache::{export_cached_assembly, preview_body};

#[wasm_bindgen]
pub fn build_case(input: JsValue) -> Result<JsValue, JsValue> {
    let ir: PreparedCase = deserialize(input)?;
    let result = build_case_data(ir).map_err(js_error)?;
    case_result_to_js(result)
}

#[wasm_bindgen]
pub fn build_assembly(input: JsValue) -> Result<JsValue, JsValue> {
    let ir: PreparedAssembly = deserialize(input)?;
    let result = build_assembly_data(ir).map_err(js_error)?;
    case_result_to_js(result)
}

fn build_case_data(ir: PreparedCase) -> Result<CaseResultData, String> {
    if ir.regions.is_empty() {
        return Err("Case requires at least one prepared region".into());
    }
    let revision = ir.revision;
    let solids = build_body(&ir)?;
    export_case(solids, revision, None)
}

fn build_assembly_data(ir: PreparedAssembly) -> Result<CaseResultData, String> {
    if ir.bodies.is_empty() {
        return Err("Case assembly requires at least one body".into());
    }

    let revision = ir.revision;
    let mut solids = Vec::new();
    let mut meshes = Vec::with_capacity(ir.bodies.len());
    for body in &ir.bodies {
        if body.revision != revision {
            return Err("Case assembly contains a stale body revision".into());
        }
        if body.regions.is_empty() {
            return Err("Case requires at least one prepared region".into());
        }
        let body_solids = build_body(body)?;
        meshes.push(BodyMeshData {
            id: body.body.id.clone(),
            name: body.body.name.clone(),
            mesh: mesh_data(&body_solids)?,
        });
        solids.extend(body_solids);
    }

    export_case(solids, revision, Some(meshes))
}

fn validate_feature_regions(ir: &PreparedCase) -> Result<(), String> {
    for feature in &ir.body.features {
        if let CaseFeature::SupportPrism {
            id,
            points,
            z,
            height,
        } = feature
        {
            if !z.is_finite() || !height.is_finite() || *height <= 0. {
                return Err(format!("Support '{id}' has invalid elevation or height"));
            }
            let footprint = make_prism(points, 0., 1.)?;
            let mut owners = 0;
            for region in &ir.regions {
                let exterior = make_prism(&region.outer, 0., 1.)?;
                let outside = (Boolean::from(&footprint.clone()) - &exterior)
                    .build_vec()
                    .map_err(cadrum_error)?;
                if outside.iter().map(Solid::volume).sum::<f64>() < 1e-8 {
                    owners += 1;
                }
            }
            if owners != 1 {
                return Err(format!(
                    "Support '{id}' must remain inside exactly one case exterior"
                ));
            }
        }
    }
    Ok(())
}

fn build_body(ir: &PreparedCase) -> Result<Vec<Solid>, String> {
    validate_feature_regions(ir)?;
    let mut solids = Vec::new();
    for region in &ir.regions {
        solids.extend(build_region(&ir.body, region)?);
    }
    if solids.is_empty() {
        return Err("OpenCascade returned an empty case solid".into());
    }
    Ok(solids)
}

#[derive(Clone, Copy, Default)]
struct BuildMode {
    cavities: bool,
    mount_holes: bool,
    gasket_profile: bool,
    boss_unions: bool,
}

fn build_region(body: &CaseBody, region: &PreparedRegion) -> Result<Vec<Solid>, String> {
    let mode = BuildMode {
        cavities: false,
        mount_holes: true,
        gasket_profile: true,
        boss_unions: true,
    };
    let mut solids =
        cache::upstream_region(body, region, || build_region_upstream(body, region, mode))?;
    apply_features(body, region, &mut solids)?;
    apply_openings(body, region, &mut solids)?;
    if !body.features.is_empty() && solids.len() != 1 {
        return Err("Case features leave disconnected material in a region".into());
    }
    Ok(solids)
}

#[cfg(test)]
fn build_region_mode(
    body: &CaseBody,
    region: &PreparedRegion,
    mode: BuildMode,
) -> Result<Vec<Solid>, String> {
    let mut solids = build_region_upstream(body, region, mode)?;
    apply_features(body, region, &mut solids)?;
    apply_openings(body, region, &mut solids)?;
    if !body.features.is_empty() && solids.len() != 1 {
        return Err("Case features leave disconnected material in a region".into());
    }
    Ok(solids)
}

fn build_region_upstream(
    body: &CaseBody,
    region: &PreparedRegion,
    mode: BuildMode,
) -> Result<Vec<Solid>, String> {
    let base_z = body.z.unwrap_or(0.0);
    let wall_height = if body.kind == CaseKind::Plate {
        0.0
    } else {
        body.wall_height.unwrap_or(0.0)
    };
    let total_height = body.thickness + wall_height;
    if !base_z.is_finite() || !total_height.is_finite() || total_height <= 0.0 {
        return Err("Case body has invalid height or elevation".into());
    }

    let mut edges = polygon_edges(&region.outer, base_z)?;
    for hole in &region.holes {
        edges.extend(polygon_edges(hole, base_z)?);
    }
    // Cadrum extrudes the first loop with all following loops as holes in one operation.
    let mut solids = {
        let _stage = Stage::new("baseExtrusion");
        metrics::count("extrusions", 1);
        vec![Solid::extrude(&edges, DVec3::Z * total_height).map_err(cadrum_error)?]
    };

    if body.kind != CaseKind::Plate {
        let cavity_z = if body.kind == CaseKind::Tray {
            base_z + body.thickness
        } else {
            base_z
        };
        if mode.cavities && region.cavities.len() > 1 {
            let _stage = Stage::new("cavityCuts");
            let cutters = region
                .cavities
                .iter()
                .map(|cavity| make_prism(cavity, cavity_z, wall_height))
                .collect::<Result<Vec<_>, _>>()?;
            subtract_many(&mut solids, &cutters)?;
        } else {
            for cavity in &region.cavities {
                let _stage = Stage::new("cavityCuts");
                subtract(&mut solids, make_prism(cavity, cavity_z, wall_height)?)?;
            }
        }

        for gasket in &region.gaskets {
            let _stage = Stage::new("gasketCuts");
            let depth = body
                .gasket
                .as_ref()
                .map(|gasket| gasket.depth)
                .ok_or_else(|| "Prepared gasket has no body gasket dimensions".to_string())?;
            let groove_z = if body.kind == CaseKind::Tray {
                base_z + total_height - depth
            } else {
                base_z
            };
            if mode.gasket_profile && can_extrude_gasket(gasket) {
                if !depth.is_finite() || depth <= 0. {
                    return Err("Case gasket has invalid depth".into());
                }
                let mut edges = polygon_edges(&gasket.outer, groove_z)?;
                for hole in &gasket.holes {
                    edges.extend(polygon_edges(hole, groove_z)?);
                }
                metrics::count("extrusions", 1);
                let cutter = Solid::extrude(&edges, DVec3::Z * depth).map_err(cadrum_error)?;
                subtract(&mut solids, cutter)?;
            } else {
                let mut groove = vec![make_prism(&gasket.outer, groove_z, depth)?];
                for hole in &gasket.holes {
                    subtract(&mut groove, make_prism(hole, groove_z, depth)?)?;
                }
                for cutter in groove {
                    subtract(&mut solids, cutter)?;
                }
            }
        }
    }

    if !region.mounts.is_empty()
        && (mode.mount_holes || mode.boss_unions)
        && can_defer_mount_holes(&region.mounts)
    {
        let mut bosses = Vec::new();
        for mount in &region.mounts {
            if mount.kind != MountKind::Boss {
                continue;
            }
            let _stage = Stage::new("bossUnions");
            let height = mount
                .height
                .ok_or_else(|| "Boss mount has no height".to_string())?;
            let diameter = mount
                .boss_diameter
                .ok_or_else(|| "Boss mount has no diameter".to_string())?;
            let z = if body.kind == CaseKind::Lid {
                base_z + wall_height - height
            } else {
                base_z + body.thickness
            };
            let boss = make_cylinder(&mount.at, z, diameter, height)?;
            if mode.boss_unions {
                bosses.push(boss);
            } else {
                fuse(&mut solids, boss)?;
            }
        }
        if !bosses.is_empty() {
            let _stage = Stage::new("bossUnions");
            let expression = solids
                .iter()
                .chain(bosses.iter())
                .map(Boolean::from)
                .reduce(|a, b| a + b)
                .ok_or_else(|| "OpenCascade could not join an empty case solid".to_string())?;
            metrics::count("booleanEvaluations", 1);
            solids = expression.build_vec().map_err(cadrum_error)?;
            if solids.is_empty() {
                return Err("OpenCascade returned an empty fused case solid".into());
            }
        }
        let _stage = Stage::new("mountHoleCuts");
        let cutters = region
            .mounts
            .iter()
            .map(|mount| make_cylinder(&mount.at, base_z, mount.hole_diameter, total_height))
            .collect::<Result<Vec<_>, _>>()?;
        if mode.mount_holes {
            subtract_many(&mut solids, &cutters)?;
        } else {
            for cutter in cutters {
                subtract(&mut solids, cutter)?;
            }
        }
    } else {
        for mount in &region.mounts {
            if mount.kind == MountKind::Boss {
                let _stage = Stage::new("bossUnions");
                let boss_height = mount
                    .height
                    .ok_or_else(|| "Boss mount has no height".to_string())?;
                let boss_diameter = mount
                    .boss_diameter
                    .ok_or_else(|| "Boss mount has no diameter".to_string())?;
                let boss_z = if body.kind == CaseKind::Lid {
                    base_z + wall_height - boss_height
                } else {
                    base_z + body.thickness
                };
                fuse(
                    &mut solids,
                    make_cylinder(&mount.at, boss_z, boss_diameter, boss_height)?,
                )?;
            }
            let _stage = Stage::new("mountHoleCuts");
            subtract(
                &mut solids,
                make_cylinder(&mount.at, base_z, mount.hole_diameter, total_height)?,
            )?;
        }
    }

    Ok(solids)
}

fn point_in_polygon(point: &Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
        if cross.abs() < 1e-9
            && point.x >= a.x.min(b.x)
            && point.x <= a.x.max(b.x)
            && point.y >= a.y.min(b.y)
            && point.y <= a.y.max(b.y)
        {
            return true;
        }
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    inside
}

fn apply_features(
    body: &CaseBody,
    region: &PreparedRegion,
    solids: &mut Vec<Solid>,
) -> Result<(), String> {
    // Additions join the cavity shell before any seats or access cuts remove material.
    for feature in &body.features {
        if let CaseFeature::SupportPrism {
            id,
            points,
            z,
            height,
        } = feature
        {
            if points
                .first()
                .is_some_and(|point| point_in_polygon(point, &region.outer))
            {
                fuse(solids, make_prism(points, *z, *height)?)?;
                if solids.len() != 1 {
                    return Err(format!("Support '{id}' must join its case region"));
                }
            }
        }
    }
    for feature in &body.features {
        match feature {
            CaseFeature::RoundSeat {
                id,
                at,
                z,
                height,
                diameter,
            } => {
                subtract(
                    solids,
                    make_cylinder(at, *z, *diameter, *height)
                        .map_err(|error| format!("Seat '{id}': {error}"))?,
                )?;
            }
            CaseFeature::ConicalSeat {
                id,
                at,
                z,
                height,
                diameter,
                end_diameter,
            } => {
                if ![at.x, at.y, *z, *height, *diameter, *end_diameter]
                    .iter()
                    .all(|v| v.is_finite())
                    || *height <= 0.
                    || *end_diameter <= 0.
                    || *diameter <= *end_diameter
                {
                    return Err(format!("Conical seat '{id}' has invalid dimensions"));
                }
                let tool = Solid::cone(*diameter / 2., *end_diameter / 2., DVec3::Z * *height)
                    .translate(DVec3::new(at.x, at.y, *z));
                subtract(solids, tool)?;
            }
            CaseFeature::SupportPrism { .. } => {}
        }
    }
    Ok(())
}

fn apply_openings(
    body: &CaseBody,
    region: &PreparedRegion,
    solids: &mut Vec<Solid>,
) -> Result<(), String> {
    if let Some(openings) = &body.openings {
        let _stage = Stage::new("openingCuts");
        let cutters = {
            let _stage = Stage::new("openingPrisms");
            let applicable: Vec<_> = openings
                .iter()
                .filter(|opening| opening_intersects_region(body, region, opening))
                .collect();
            let mut cutters = Vec::new();
            for (index, opening) in applicable.iter().enumerate() {
                if let Some(profiles) = opening_remainder(opening, &applicable[..index]) {
                    metrics::count("reducedOpeningProfiles", 1);
                    for points in profiles {
                        cutters.push(make_prism(&points, opening.z, opening.height)?);
                    }
                } else {
                    cutters.push(make_prism(&opening.points, opening.z, opening.height)?);
                }
            }
            cutters
        };
        let _stage = Stage::new("openingBoolean");
        subtract_many(solids, &cutters)?;
    }

    Ok(())
}

// A deeper rectangular pocket already removes the center of an overlapping
// orthogonal pocket. Partition only the remaining material into exact rectangles
// so the Boolean kernel need not intersect those redundant cutter faces. Inputs
// outside this proven, bounded case keep the original kernel path.
fn opening_remainder(opening: &CaseOpening, previous: &[&CaseOpening]) -> Option<Vec<Vec<Vec2>>> {
    if !opening.z.is_finite()
        || !opening.height.is_finite()
        || opening.height <= 0.
        || !simple_orthogonal_polygon(&opening.points)
    {
        return None;
    }
    for covered in previous {
        if covered.points.len() != 4
            || !simple_orthogonal_polygon(&covered.points)
            || !covered.z.is_finite()
            || !covered.height.is_finite()
            || covered.height <= 0.
            || covered.z > opening.z
            || covered.z + covered.height < opening.z + opening.height
        {
            continue;
        }
        let mut rect_x: Vec<_> = covered.points.iter().map(|p| p.x).collect();
        let mut rect_y: Vec<_> = covered.points.iter().map(|p| p.y).collect();
        rect_x.sort_by(f64::total_cmp);
        rect_x.dedup();
        rect_y.sort_by(f64::total_cmp);
        rect_y.dedup();
        if rect_x.len() != 2 || rect_y.len() != 2 {
            continue;
        }
        let mut xs: Vec<_> = opening
            .points
            .iter()
            .map(|p| p.x)
            .chain(rect_x.iter().copied())
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        let mut rectangles: Vec<[f64; 4]> = Vec::new();
        let mut contains_rectangle = true;
        for slab in xs.windows(2) {
            let middle = slab[0] + (slab[1] - slab[0]) / 2.;
            if middle <= slab[0] || middle >= slab[1] {
                return None;
            }
            let mut ys: Vec<_> = opening
                .points
                .iter()
                .zip(opening.points.iter().cycle().skip(1))
                .take(opening.points.len())
                .filter(|(a, b)| a.y == b.y && middle > a.x.min(b.x) && middle < a.x.max(b.x))
                .map(|(a, _)| a.y)
                .collect();
            ys.sort_by(f64::total_cmp);
            if ys.len() % 2 != 0 {
                return None;
            }
            let within = middle > rect_x[0] && middle < rect_x[1];
            if within
                && !ys
                    .chunks_exact(2)
                    .any(|pair| pair[0] <= rect_y[0] && pair[1] >= rect_y[1])
            {
                contains_rectangle = false;
                break;
            }
            for pair in ys.chunks_exact(2) {
                let intervals = if within {
                    [
                        (pair[0], pair[1].min(rect_y[0])),
                        (pair[0].max(rect_y[1]), pair[1]),
                    ]
                } else {
                    [(pair[0], pair[1]), (0., 0.)]
                };
                for (low, high) in intervals {
                    if high <= low {
                        continue;
                    }
                    if let Some(rectangle) = rectangles
                        .iter_mut()
                        .find(|r| r[2] == slab[0] && r[1] == low && r[3] == high)
                    {
                        rectangle[2] = slab[1];
                    } else {
                        rectangles.push([slab[0], low, slab[1], high]);
                    }
                }
            }
            if rectangles.len() > 64 {
                return None;
            }
        }
        if contains_rectangle {
            return Some(
                rectangles
                    .into_iter()
                    .map(|[x0, y0, x1, y1]| {
                        vec![
                            Vec2 { x: x0, y: y0 },
                            Vec2 { x: x1, y: y0 },
                            Vec2 { x: x1, y: y1 },
                            Vec2 { x: x0, y: y1 },
                        ]
                    })
                    .collect(),
            );
        }
    }
    None
}

fn simple_orthogonal_polygon(points: &[Vec2]) -> bool {
    if points.len() < 4
        || points.len() > 64
        || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return false;
    }
    let edges: Vec<_> = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .collect();
    for (i, &(a, b)) in edges.iter().enumerate() {
        if (a.x == b.x) == (a.y == b.y) {
            return false;
        }
        let c = edges[(i + 1) % edges.len()].1;
        if (b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y) < 0. {
            return false;
        }
        for (j, &(c, d)) in edges.iter().enumerate().skip(i + 1) {
            if j == i + 1 || (i == 0 && j == edges.len() - 1) {
                continue;
            }
            if a.x.min(b.x).max(c.x.min(d.x)) <= a.x.max(b.x).min(c.x.max(d.x))
                && a.y.min(b.y).max(c.y.min(d.y)) <= a.y.max(b.y).min(c.y.max(d.y))
            {
                return false;
            }
        }
    }
    true
}

// Use a single profile only for a proven nested convex pair. Unusual or
// invalid prepared contours retain the sequential subtraction semantics.
fn can_extrude_gasket(gasket: &PreparedGasket) -> bool {
    fn cross(a: &Vec2, b: &Vec2, p: &Vec2) -> f64 {
        (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
    }
    fn orientation(points: &[Vec2]) -> Option<f64> {
        if points.len() < 3 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return None;
        }
        let mut sign = 0.;
        for (a, b) in points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
        {
            if (a.x - b.x).hypot(a.y - b.y) <= 1e-7 {
                return None;
            }
            for p in points {
                let value = cross(a, b, p);
                if value.abs() <= 1e-7 {
                    continue;
                }
                if sign == 0. {
                    sign = value.signum();
                }
                if value * sign < 0. {
                    return None;
                }
            }
        }
        (sign != 0.).then_some(sign)
    }
    if gasket.holes.len() != 1 || orientation(&gasket.holes[0]).is_none() {
        return false;
    }
    let Some(sign) = orientation(&gasket.outer) else {
        return false;
    };
    gasket
        .outer
        .iter()
        .zip(gasket.outer.iter().cycle().skip(1))
        .take(gasket.outer.len())
        .all(|(a, b)| gasket.holes[0].iter().all(|p| cross(a, b, p) * sign > 1e-7))
}

// Moving a cut past a later boss is safe only when it cannot cut that boss.
// Require a surviving annulus for every boss, so intermediate full-removal
// errors are preserved. Mixed hole/boss sequences use the original ordering.
fn can_defer_mount_holes(mounts: &[Mount]) -> bool {
    if mounts.iter().all(|mount| mount.kind == MountKind::Hole) {
        return true;
    }
    mounts.iter().all(|mount| {
        mount.kind == MountKind::Boss
            && mount.hole_diameter.is_finite()
            && mount.hole_diameter > 0.
            && mount
                .boss_diameter
                .is_some_and(|diameter| diameter.is_finite() && diameter > mount.hole_diameter)
    }) && mounts.iter().enumerate().all(|(index, hole)| {
        mounts.iter().enumerate().all(|(other, boss)| {
            index == other
                || (hole.at.x - boss.at.x).hypot(hole.at.y - boss.at.y)
                    > (hole.hole_diameter + boss.boss_diameter.unwrap_or(0.)) / 2. + 1e-7
        })
    })
}

fn opening_intersects_region(
    body: &CaseBody,
    region: &PreparedRegion,
    opening: &CaseOpening,
) -> bool {
    if opening.points.len() < 3
        || !opening.z.is_finite()
        || !opening.height.is_finite()
        || opening.height <= 0.
        || opening
            .points
            .iter()
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return true;
    }
    let base = body.z.unwrap_or(0.);
    let height = body.thickness
        + if body.kind == CaseKind::Plate {
            0.
        } else {
            body.wall_height.unwrap_or(0.)
        };
    if opening.z >= base + height || opening.z + opening.height <= base {
        return false;
    }
    let bounds = |points: &[Vec2]| {
        points.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
        )
    };
    let a = bounds(&region.outer);
    let b = bounds(&opening.points);
    a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
}

fn polygon_edges(points: &[Vec2], z: f64) -> Result<Vec<Edge>, String> {
    if points.len() < 3
        || !z.is_finite()
        || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err("Case feature has an invalid polygon".into());
    }
    Edge::polygon(
        &points
            .iter()
            .map(|p| DVec3::new(p.x, p.y, z))
            .collect::<Vec<_>>(),
    )
    .map_err(cadrum_error)
}

fn subtract_many(solids: &mut Vec<Solid>, cutters: &[Solid]) -> Result<(), String> {
    if cutters.is_empty() {
        return Ok(());
    }
    let mut result = Vec::new();
    for solid in solids.iter() {
        let expression = cutters
            .iter()
            .fold(Boolean::from(solid), |expression, cutter| {
                expression - cutter
            });
        metrics::count("booleanEvaluations", 1);
        result.extend(expression.build_vec().map_err(cadrum_error)?);
    }
    if result.is_empty() {
        return Err("OpenCascade subtraction removed the whole case solid".into());
    }
    *solids = result;
    Ok(())
}

fn make_prism(points: &[Vec2], z: f64, height: f64) -> Result<Solid, String> {
    if points.len() < 3 || !z.is_finite() || !height.is_finite() || height <= 0.0 {
        return Err("Case feature has an invalid polygon or extrusion height".into());
    }
    let vertices: Vec<DVec3> = points
        .iter()
        .map(|point| DVec3::new(point.x, point.y, z))
        .collect();
    if vertices.iter().any(|point| !point.is_finite()) {
        return Err("Case feature contains a non-finite coordinate".into());
    }
    let edges = Edge::polygon(&vertices).map_err(cadrum_error)?;
    metrics::count("extrusions", 1);
    Solid::extrude(&edges, DVec3::Z * height).map_err(cadrum_error)
}

fn make_cylinder(at: &Vec2, z: f64, diameter: f64, height: f64) -> Result<Solid, String> {
    if !at.x.is_finite()
        || !at.y.is_finite()
        || !z.is_finite()
        || !diameter.is_finite()
        || diameter <= 0.0
        || !height.is_finite()
        || height <= 0.0
    {
        return Err("Case mount has invalid cylinder dimensions".into());
    }
    metrics::count("cylinders", 1);
    Ok(Solid::cylinder(diameter / 2.0, DVec3::Z * height).translate(DVec3::new(at.x, at.y, z)))
}

fn subtract(solids: &mut Vec<Solid>, tool: Solid) -> Result<(), String> {
    let mut result = Vec::new();
    for solid in solids.drain(..) {
        metrics::count("booleanEvaluations", 1);
        result.extend(
            (Boolean::from(&solid) - &tool)
                .build_vec()
                .map_err(cadrum_error)?,
        );
    }
    if result.is_empty() {
        return Err("OpenCascade subtraction removed the whole case solid".into());
    }
    *solids = result;
    Ok(())
}

fn fuse(solids: &mut Vec<Solid>, tool: Solid) -> Result<(), String> {
    let expression = solids
        .iter()
        .map(Boolean::from)
        .reduce(|left, right| left + right)
        .ok_or_else(|| "OpenCascade could not join an empty case solid".to_string())?;
    metrics::count("booleanEvaluations", 1);
    *solids = (expression + Boolean::from(&tool))
        .build_vec()
        .map_err(cadrum_error)?;
    if solids.is_empty() {
        return Err("OpenCascade returned an empty fused case solid".into());
    }
    Ok(())
}

#[cfg(test)]
mod equivalence;

#[cfg(test)]
mod gasket_features;
