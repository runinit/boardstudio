// E5: two independent strategies; only the isolated prepared bottom family.
fn planned_cuts(ir: &PreparedCase, profile_holes: bool) -> Result<Vec<Solid>, String> {
    if ir.body.kind != CaseKind::Plate || ir.body.gasket.is_some() {
        return Err("Unsupported body in Boolean planning experiment".into());
    }
    let z = ir.body.z.unwrap_or(0.);
    let height = ir.body.thickness;
    let mut output = Vec::new();
    for region in &ir.regions {
        if !region.cavities.is_empty() || !region.gaskets.is_empty()
            || region.mounts.iter().any(|mount| mount.kind != MountKind::Hole)
        {
            return Err("Unsupported region in Boolean planning experiment".into());
        }
        let mut edges = polygon_edges(&region.outer, z)?;
        for hole in &region.holes { edges.extend(polygon_edges(hole,z)?); }
        let mut cutters = Vec::new();
        for mount in &region.mounts {
            if profile_holes {
                edges.push(Edge::circle(mount.hole_diameter/2.,DVec3::Z).map_err(cadrum_error)?
                    .translate(DVec3::new(mount.at.x,mount.at.y,z)));
            } else {
                cutters.push(make_cylinder(&mount.at,z,mount.hole_diameter,height)?);
            }
        }
        let mut solids = vec![Solid::extrude(&edges,DVec3::Z*height).map_err(cadrum_error)?];
        if profile_holes {
            apply_openings(&ir.body,region,&mut solids)?;
        } else {
            let applicable: Vec<_> = ir.body.openings.as_deref().unwrap_or_default().iter()
                .filter(|opening| opening_intersects_region(&ir.body,region,opening)).collect();
            for (index,opening) in applicable.iter().enumerate() {
                if let Some(profiles) = opening_remainder(opening,&applicable[..index]) {
                    for points in profiles { cutters.push(make_prism(&points,opening.z,opening.height)?); }
                } else { cutters.push(make_prism(&opening.points,opening.z,opening.height)?); }
            }
            subtract_many(&mut solids,&cutters)?;
        }
        output.extend(solids);
    }
    Ok(output)
}
