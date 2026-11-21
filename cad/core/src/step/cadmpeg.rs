//! Joins decoded millimetre geometry to the authored shell order and scene.
mod analytic;
mod styles;
mod topology;
use super::*;
use cadmpeg_codec_step::StepCodec;
use cadmpeg_ir::{Codec, DecodeOptions, ids::FaceId};
use cadmpeg_tessellate::{Options, Tessellator};
use std::io::Cursor;

pub(super) fn build(
    bytes: Vec<u8>,
    exchange: Exchange,
    items: HashMap<u64, Vec<u64>>,
    factors: HashMap<u64, f64>,
    precision: Vec<StepPrecision>,
    options: StepImportOptions,
    deflection: f64,
) -> Result<ImportedStep, String> {
    let records = source_records(&exchange);
    assembly::validate_pds_definition_targets(&records)?;
    let ids = assembly::solids(&records);
    let solid_set: HashSet<_> = ids.iter().copied().collect();
    units::solid_representation_units(&ids, &items, &factors)?;
    let scene = scene::Scene::new(&exchange, &items, &factors, &ids);
    scene.validate()?;
    let (instances, curves): (Vec<_>, Vec<_>) = assembly::occurrences(
        &records,
        &scene,
        &items,
        &factors,
        &source_occurrence_ids(&exchange),
    )?
    .into_iter()
    .partition(|(id, _, _)| solid_set.contains(id));
    if instances.is_empty()
        || ids
            .iter()
            .any(|id| !instances.iter().any(|(solid, _, _)| solid == id))
    {
        return Err("empty or unreferenced exact geometry".into());
    }
    let wireframe = wireframe::Wireframe::build(&exchange, &curves, &factors, &precision)?;
    // Validate the authored edge uses before cadmpeg can reorder or omit them.
    let mut closed = true;
    let mut shells = HashMap::new();
    let mut retained: HashSet<_> = ids.iter().copied().collect();
    for &solid in &ids {
        let mut uses = topology::ShellUses::default();
        let definition = parameters(named_record(&records, solid, "MANIFOLD_SOLID_BREP")?, 2)?;
        let shell = reference(&definition[1])?;
        let parts = parameters(named_record(&records, shell, "CLOSED_SHELL")?, 2)?;
        let Parameter::List(members) = &parts[1] else {
            return Err("malformed shell membership".into());
        };
        let mut faces = Vec::new();
        for member in members {
            let id = reference(member)?;
            let args = parameters(named_record(&records, id, "ADVANCED_FACE")?, 4)?;
            uses.face(&records, id, &args[1])?;
            faces.push((id, reference(&args[2])?));
        }
        if faces.is_empty() {
            return Err(format!("empty shell #{shell}"));
        }
        closed &= uses.finish(&records, &mut retained)?;
        shells.insert(solid, (shell, faces));
    }
    let decoded = StepCodec::default()
        .decode(&mut Cursor::new(&bytes), &DecodeOptions::default())
        .map_err(|e| format!("cadmpeg decode: {e}"))?;
    let ir = decoded.ir();
    let faces: HashMap<_, _> = ir.model.faces.iter().map(|f| (f.id.as_str(), f)).collect();
    let fills = styles::suppressed_fills(&records);
    let mut source_faces = Vec::new();
    let mut face_ids = Vec::<FaceId>::new();
    let mut by_solid = HashMap::new();
    for &solid in &ids {
        let start = source_faces.len();
        let body_id = format!("step:data:body#{solid}");
        let body = ir
            .model
            .bodies
            .iter()
            .find(|b| b.id.as_str() == body_id)
            .ok_or_else(|| format!("cadmpeg omitted body #{solid}"))?;
        let (shell, faces_of_shell) = &shells[&solid];
        let shell = *shell;
        for &(id, support_id) in faces_of_shell {
            let support = records.get(&support_id).ok_or("missing STEP support")?;
            // The codec scopes shared topology by the shell and, for shared shells,
            // by the root solid. Use only an exact owner-specific identity.
            let key = format!("step:data:face#{id}");
            let face = [
                format!("{key}-root-{solid}-shell-{shell}"),
                format!("{key}-shell-{shell}"),
                key,
            ]
            .iter()
            .find_map(|k| faces.get(k.as_str()))
            .ok_or_else(|| format!("cadmpeg omitted solid #{solid} shell #{shell} face #{id}"))?;
            face_ids.push(face.id.clone());
            source_faces.push(StepSourceFace {
                solid_id: solid,
                shell_index: 0,
                face_id: id,
                support_id,
                color: face
                    .color
                    .map(|c| [c.r(), c.g(), c.b(), c.a()])
                    .or_else(|| fills.get(&id).copied())
                    .or_else(|| body.color.map(|c| [c.r(), c.g(), c.b(), c.a()]))
                    .or_else(|| fills.get(&solid).copied()),
                support_kind: support
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join("+"),
            });
            retained.insert(id);
        }
        by_solid.insert(solid, start..source_faces.len());
    }
    if face_ids.iter().collect::<HashSet<_>>().len() != faces.len() {
        return Err("source and cadmpeg face catalogues disagree".into());
    }
    retained.extend(instances.iter().map(|(_, _, rep)| *rep));
    for curve in wireframe.presentation().occurrences() {
        retained.extend([
            curve.trimmed_curve_id,
            curve.curve_set_id,
            curve.representation_id,
        ]);
    }
    scene.require_complete_presentations(&retained)?;
    let geometry = analytic::Geometry::new(ir);
    let mut mesh = MeshBuffers::default();
    let mut positions = Vec::new();
    let mut triangle_faces = Vec::new();
    let mut face_occurrences = Vec::new();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let include = |bounds: &mut [[f64; 3]; 2], point: [f64; 3]| {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(point[axis]);
            bounds[1][axis] = bounds[1][axis].max(point[axis]);
        }
    };
    let mut bounds_from_source_geometry = true;
    let mut relaxed = false;
    let mut fallback_faces = 0;
    let mut discarded = 0;
    let mut degraded = 0;
    let mut sampled = 0;
    let mut max_residual: Option<f64> = None;
    for (occurrence, &(solid, matrix, representation_id)) in instances.iter().enumerate() {
        units::validate_placement_matrix(matrix)?;
        let scale = [matrix.x, matrix.y, matrix.z]
            .map(|v| v.truncate().magnitude())
            .into_iter()
            .fold(0., f64::max);
        let owner = precision
            .iter()
            .find(|p| p.representation_id == representation_id)
            .ok_or("missing precision context")?;
        // A generic context owns no uncertainty, so it may not borrow any
        // allowance beyond numerical noise unless the caller selects an explicit budget.
        let allowance = if owner.uncertainty_mm().is_some() {
            repair_allowance(
                options.preview_repair_budget_mm,
                owner.uncertainty_mm(),
                matrix,
            )
        } else {
            options.preview_repair_budget_mm.unwrap_or(1e-8)
        };
        let settings = Options {
            closure_tolerance: owner.uncertainty_mm(),
            chord: deflection / scale,
            angle: 0.5,
            ..Options::default()
        };
        let mut tess = Tessellator::new(ir, settings);
        let mut checked_edges = HashSet::new();
        let mut triangles = Vec::new();
        for source_face in by_solid[&solid].clone() {
            let id = &face_ids[source_face];
            let surface = tess
                .face(id)
                .map_err(|e| format!("solid #{solid} {id}: {e}"))?;
            if surface.triangles.is_empty() {
                return Err(format!("empty mesh for {id}"));
            }
            for trace in &surface.coedges {
                if let Some((edge, gap)) =
                    geometry.bspline_endpoint_gap(&trace.coedge, &mut checked_edges)
                {
                    let gap_mm = gap * scale;
                    if gap_mm
                        .partial_cmp(&allowance)
                        .is_none_or(std::cmp::Ordering::is_gt)
                    {
                        return Err(format!(
                            "{edge} B-spline endpoint is {gap_mm:.6} mm from its authored vertex, beyond the {allowance} mm allowance"
                        ));
                    }
                    if let Some(declared) = owner.uncertainty_mm() {
                        relaxed |= gap * scale > declared * scale;
                    }
                }
            }
            let residual = surface.deviation * scale;
            // On analytic supports the inverse is closed-form, so a sampled
            // incidence failure witnesses an off-support source point. A spline
            // projection can instead stop at a stationary parameter.
            if matches!(
                source_faces[source_face].support_kind.as_str(),
                "PLANE" | "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE"
            ) && residual > allowance
            {
                return Err(format!(
                    "{id} support incidence exceeds {allowance} mm: sampled support residual {residual:.6e} mm"
                ));
            }
            if !residual.is_finite() {
                return Err(format!("{id} has nonfinite support residual"));
            }
            let linear_plane = geometry.lines_on_plane(id, &surface);
            if !surface.coedges.is_empty() {
                if let Some(declared) = owner.uncertainty_mm() {
                    relaxed |= residual > declared * scale;
                }
                if !linear_plane {
                    sampled += surface.coedges.len();
                    max_residual = Some(max_residual.unwrap_or(0.0).max(residual));
                }
            }
            let exact = (residual <= 1e-8 || (linear_plane && residual <= allowance))
                .then(|| geometry.face_bounds(id, &surface, matrix))
                .flatten();
            if let Some(mut face_bounds) = exact {
                if linear_plane && owner.uncertainty_mm().is_some() {
                    // A fractional placement can round down even when the
                    // extremal face itself has no measurable plane residual.
                    let [low, high] = &mut face_bounds;
                    for (axis, (low, high)) in low.iter_mut().zip(high).enumerate() {
                        if matrix.w[axis].fract() != 0.0 {
                            *low = low.next_down();
                            *high = high.next_up();
                        }
                    }
                }
                include(&mut bounds, face_bounds[0]);
                include(&mut bounds, face_bounds[1]);
            } else {
                bounds_from_source_geometry = false;
            }
            let tag = face_occurrences.len();
            face_occurrences.push(StepFaceOccurrence {
                source_face,
                occurrence,
                representation_id,
                source_uncertainty_mm: owner.uncertainty_mm().map(|v| v * scale),
                repair_displacement_mm: 0.0,
            });
            let start = triangles.len();
            let mut face_degraded = false;
            for corners in surface.triangles {
                let mut triangle = corners.map(|i| {
                    let p = surface.positions[i];
                    matrix.transform_point(Point3::new(p.x, p.y, p.z))
                });
                if matrix.determinant() < 0.0 {
                    triangle.swap(1, 2);
                }
                if let Err(error) = crate::mesh::push_triangle(&mut mesh, triangle) {
                    if !fallback_triangle_has_representable_area(triangle) {
                        discarded += 1;
                        closed = false;
                        continue;
                    }
                    // A positive-area triangle finer than the f32 render grid keeps
                    // its exact f64 analysis coverage. Its render image may collapse,
                    // but only within the preview deflection of the analysis geometry.
                    if render_displacement(triangle) > deflection {
                        return Err(format!("{id}: {error}"));
                    }
                    push_render_degraded_triangle(&mut mesh, triangle)?;
                    face_degraded = true;
                    degraded += 1;
                }
                triangles.push(triangle);
                triangle_faces.push(tag);
                for p in triangle {
                    let xyz = [p.x, p.y, p.z];
                    positions.push(xyz);
                }
            }
            if triangles.len() == start {
                return Err(format!("{id} has no positive-area triangle"));
            }
            fallback_faces += usize::from(face_degraded);
        }
        let origin = triangles[0][0];
        let volume6: f64 = triangles
            .iter()
            .map(|[a, b, c]| (*a - origin).dot((*b - origin).cross(*c - origin)))
            .sum();
        closed &= volume6.is_finite() && volume6 > 0.0 && closed_oriented_mesh(&triangles);
    }
    if !bounds_from_source_geometry {
        // A mixed asset reports preview bounds. Include the displayed points
        // of every face, even when some faces also have analytic bounds.
        for &point in &positions {
            include(&mut bounds, point);
        }
    }
    wireframe.include_bounds(&mut bounds);
    let mut asset = ImportedStep {
        original_bytes: bytes,
        analysis_options: options,
        _original_source: exchange,
        solid_count: ids.len(),
        occurrences: instances,
        bounds,
        mesh,
        preview_positions_f64: positions,
        source_faces,
        face_occurrences,
        triangle_faces,
        precision,
        wireframe,
        accuracy: StepAccuracy {
            preview_deflection_mm: deflection,
            preview_angular_rad: 0.5,
            analysis_error_bound_mm: None,
            bounds_from_source_geometry,
            mesh_closed_oriented: closed,
            preview_floor_mm: options.preview_repair_budget_mm.unwrap_or(PREVIEW_FLOOR_MM),
            preview_declared_multiplier: options
                .preview_repair_budget_mm
                .map_or(PREVIEW_DECLARED_MULTIPLIER, |_| 0.0),
            selected_repair_budget_mm: options.preview_repair_budget_mm,
            max_repair_displacement_mm: None,
            relaxed_beyond_declared_precision: relaxed,
            sampled_support_checks: sampled,
            max_sampled_support_residual_mm: max_residual,
            fallback_meshed_faces: fallback_faces,
            render_degraded_fallback_triangles: degraded,
            discarded_display_triangles: discarded,
        },
    };
    cuboid::certify_cuboid(&mut asset);
    asset.validate_invariants().map_err(|e| e.join("; "))?;
    Ok(asset)
}

/// The largest distance an f32 render projection moves one of the vertices.
fn render_displacement(points: [Point3; 3]) -> f64 {
    points
        .iter()
        .map(|p| {
            let q = Point3::new(
                f64::from(p.x as f32),
                f64::from(p.y as f32),
                f64::from(p.z as f32),
            );
            (q - p).magnitude()
        })
        .fold(
            0.0,
            |a: f64, b| if b.is_nan() { f64::INFINITY } else { a.max(b) },
        )
}
