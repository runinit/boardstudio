//! Source-authored analytic BReps. Every occurrence must be supported before
//! the importer can publish native preview data for the asset.
mod assembly;
mod carrier;
mod cone;
mod cuboid;
mod cylinder_band;
mod facet;
mod geometry;
mod oblique;
mod placement;
mod planar_repair;
mod rational_trim;
mod sphere;
mod sphere_bounds;
mod trim;

use super::*;
use cgmath::Vector3;
use geometry::{Edge, Surface, SurfaceKind};

const ANGULAR_TOLERANCE: f64 = 0.5;

struct Face {
    source: usize,
    surface: Surface,
    same_sense: bool,
    loops: Vec<Boundary>,
}

struct Boundary {
    explicit_outer: bool,
    edges: Vec<(u64, bool)>,
}

struct Solid {
    edges: HashMap<u64, Edge>,
    faces: Vec<Face>,
}

pub(super) struct Asset {
    solid_count: usize,
    occurrences: Vec<(u64, Matrix4, u64)>,
    bounds: [[f64; 3]; 2],
    mesh: MeshBuffers,
    positions: Vec<[f64; 3]>,
    source_faces: Vec<StepSourceFace>,
    face_occurrences: Vec<StepFaceOccurrence>,
    triangle_faces: Vec<usize>,
    deflection_mm: f64,
    relaxed_beyond_declared_precision: bool,
}

impl Asset {
    pub(super) fn finish(
        self,
        bytes: Vec<u8>,
        exchange: Exchange,
        precision: Vec<StepPrecision>,
        options: StepImportOptions,
    ) -> Result<ImportedStep, String> {
        let mut imported = ImportedStep {
            original_bytes: bytes,
            analysis_options: options,
            _original_source: exchange,
            _repair_ledger: repair::Ledger::default(),
            #[cfg(test)]
            solids: HashMap::new(),
            solid_count: self.solid_count,
            occurrences: self.occurrences,
            bounds: self.bounds,
            mesh: self.mesh,
            preview_positions_f64: self.positions,
            source_faces: self.source_faces,
            face_occurrences: self.face_occurrences,
            triangle_faces: self.triangle_faces,
            precision,
            accuracy: StepAccuracy {
                preview_deflection_mm: self.deflection_mm,
                preview_angular_rad: ANGULAR_TOLERANCE,
                analysis_error_bound_mm: None,
                bounds_from_source_geometry: true,
                mesh_closed_oriented: true,
                preview_floor_mm: options.preview_repair_budget_mm.unwrap_or(PREVIEW_FLOOR_MM),
                preview_declared_multiplier: options
                    .preview_repair_budget_mm
                    .map_or(PREVIEW_DECLARED_MULTIPLIER, |_| 0.0),
                selected_repair_budget_mm: options.preview_repair_budget_mm,
                // Finite boundary movement does not qualify the whole delivered asset.
                max_repair_displacement_mm: None,
                relaxed_beyond_declared_precision: self.relaxed_beyond_declared_precision,
                sampled_support_checks: 0,
                max_sampled_support_residual_mm: None,
                fallback_meshed_faces: 0,
                render_degraded_fallback_triangles: 0,
            },
            wireframe: wireframe::Wireframe::default(),
        };
        certify_cuboid(&mut imported);
        imported
            .validate_invariants()
            .map_err(|errors| format!("import invariants failed: {}", errors.join("; ")))?;
        Ok(imported)
    }
}

/// The one exact-zero repair producer, for whichever route delivered the asset.
/// It re-derives an integer box from the original decimal spellings and then
/// requires the delivered mesh to tile exactly that box; the route's own repair
/// accounting is never consulted.
pub(super) fn certify_cuboid(model: &mut ImportedStep) {
    if cuboid::certify(model) {
        model.accuracy.max_repair_displacement_mm = Some(0.0);
    }
}

pub(super) fn build(
    bytes: &[u8],
    exchange: &Exchange,
    items: &HashMap<u64, Vec<u64>>,
    units: &HashMap<u64, f64>,
    precision: &[StepPrecision],
    options: StepImportOptions,
    deflection_mm: f64,
) -> Result<Asset, String> {
    let mut schemas = exchange.header.iter().filter(|r| r.name == "FILE_SCHEMA");
    let schema = schemas.next().and_then(|record| {
        if let Parameter::List(params) = &record.parameter
            && let [Parameter::List(names)] = params.as_slice()
            && let [Parameter::String(schema)] = names.as_slice()
        {
            return Some(schema.as_str());
        }
        None
    });
    let config_control = schema == Some("CONFIG_CONTROL_DESIGN");
    let automotive = schema.and_then(|schema| schema.split_ascii_whitespace().next())
        == Some("AUTOMOTIVE_DESIGN");
    if !(automotive || config_control)
        || schemas.next().is_some()
        || !exchange.reference.is_empty()
        || !exchange.anchor.is_empty()
    {
        return Err("outside native analytic exchange class".into());
    }
    let records = source_records(exchange);
    let mut ids = Vec::new();
    let mut has_cone = false;
    for (&id, parts) in &records {
        for part in parts {
            match part.name.as_str() {
                "CONICAL_SURFACE" => has_cone = true,
                "MANIFOLD_SOLID_BREP" if parts.len() == 1 => ids.push(id),
                "MANIFOLD_SOLID_BREP"
                | "BREP_WITH_VOIDS"
                | "FACETED_BREP"
                | "OPEN_SHELL"
                | "ORIENTED_CLOSED_SHELL"
                | "SHELL_BASED_SURFACE_MODEL"
                | "GEOMETRIC_CURVE_SET" => {
                    return Err(format!("native preview does not handle {}", part.name));
                }
                _ => {}
            }
        }
    }
    ids.sort_unstable();
    let factors = units::solid_representation_units(&ids, items, units)?;
    let scene = scene::Scene::new(exchange, items, units, &ids);
    scene.validate()?;
    let traced = assembly::occurrences(&records, &scene, config_control)?;
    let occurrences: Vec<_> = traced.iter().map(|(instance, _)| *instance).collect();
    if occurrences.is_empty()
        || occurrences
            .iter()
            .any(|(id, _, _)| !factors.contains_key(id))
        || ids
            .iter()
            .any(|id| !occurrences.iter().any(|(placed, _, _)| placed == id))
    {
        return Err("empty or unreferenced native solid".into());
    }
    let owners: HashMap<_, _> = precision.iter().map(|p| (p.representation_id, p)).collect();
    let cone_angles = if has_cone {
        units::conical_angle_factors(exchange)?
    } else {
        HashMap::new()
    };
    let mut retained = HashSet::new();
    let mut source_faces = Vec::new();
    let mut solids = HashMap::new();
    for id in ids.iter().copied() {
        let solid = Solid::read(id, &records, &scene, &cone_angles, &mut source_faces)?;
        retained.extend(solid.edges.keys().copied());
        solids.insert(id, solid);
        retained.insert(id);
    }
    retained.extend(source_faces.iter().map(|f| f.face_id));
    retained.extend(occurrences.iter().map(|(_, _, owner)| *owner));
    scene.require_complete_presentations(&retained)?;
    let mut asset = Asset {
        solid_count: solids.len(),
        occurrences,
        bounds: [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]],
        mesh: MeshBuffers::default(),
        positions: Vec::new(),
        source_faces,
        face_occurrences: Vec::new(),
        triangle_faces: Vec::new(),
        deflection_mm,
        relaxed_beyond_declared_precision: false,
    };
    let mut geometry = Vec::with_capacity(asset.occurrences.len());
    let mut definition_epsilon = std::collections::BTreeMap::<u64, f64>::new();
    for &(id, placement, representation) in &asset.occurrences {
        units::validate_placement_matrix(placement)?;
        let world = placement * Matrix4::from_scale(factors[&id]);
        let scale = placement
            .x
            .truncate()
            .magnitude()
            .max(placement.y.truncate().magnitude())
            .max(placement.z.truncate().magnitude());
        let source_to_world = scale * factors[&id];
        if !source_to_world.is_finite() || source_to_world <= 0.0 {
            return Err("invalid native source-to-world scale".into());
        }
        let owner = owners
            .get(&representation)
            .ok_or("missing native precision context")?;
        if owner.geometric_dimension.is_some_and(|d| d != 3) {
            return Err("native solid representation is not three dimensional".into());
        }
        let uncertainty = owner.uncertainty_mm().map(|value| value * scale);
        if uncertainty.is_some_and(|value| !value.is_finite()) {
            return Err("nonfinite placed source precision".into());
        }
        // Attachment validation stays limited to floating-point noise. Planar
        // trim repair separately checks the actual movement against the allowance.
        let allowance = repair_allowance(
            options.preview_repair_budget_mm,
            owner.uncertainty_mm(),
            placement,
        );
        let world_epsilon = 1e-8_f64
            .min(allowance)
            .min(uncertainty.unwrap_or(f64::INFINITY))
            .min(deflection_mm * 1e-4);
        let epsilon = world_epsilon / source_to_world;
        geometry.push((
            source_to_world,
            planar_repair::OccurrenceGeometry {
                world,
                epsilon,
                world_epsilon,
                allowance_mm: allowance,
                uncertainty_mm: uncertainty,
            },
        ));
        // Every occurrence uses the same definition-level repaired vertices.
        definition_epsilon
            .entry(id)
            .and_modify(|previous| *previous = previous.min(epsilon))
            .or_insert(epsilon);
    }
    let repairs = planar_repair::Repairs::prepare(
        &solids,
        &definition_epsilon,
        &asset.source_faces,
        &records,
        bytes,
        &traced,
    )?;
    if repairs.is_some() && records.len() != exchange.data[0].entities.len() {
        return Err("native shared trim repair requires unique DATA entity IDs".into());
    }
    for (index, ((id, placement, representation), (source_to_world, frame))) in
        asset.occurrences.iter().copied().zip(geometry).enumerate()
    {
        let planar_repair::OccurrenceGeometry {
            world,
            epsilon,
            uncertainty_mm: uncertainty,
            ..
        } = frame;
        let solid = &solids[&id];
        let repaired = if let Some(repairs) = &repairs {
            repairs.apply(id, solid, index, &frame, &mut asset.bounds)?
        } else {
            None
        };
        let edges = repaired
            .as_ref()
            .map_or(&solid.edges, |repair| &repair.edges);
        let deflection = deflection_mm / source_to_world;
        // Both faces sharing a spherical boundary must use the same samples.
        // Reserve slack on the edges for the interior spherical facets.
        let edge_deflection = if solid
            .faces
            .iter()
            .any(|face| matches!(face.surface.kind, SurfaceKind::Sphere(_)))
        {
            deflection * 0.5
        } else {
            deflection
        };
        let mut samples = HashMap::with_capacity(edges.len());
        for (&edge_id, edge) in edges {
            samples.insert(
                edge_id,
                edge.sample(edge_deflection, epsilon)
                    .map_err(|error| format!("native edge #{edge_id}: {error}"))?,
            );
            // Along each admitted cylindrical or conical generator a world
            // coordinate is affine in height. Its extrema lie on the trim.
            edge.include_bounds(world, epsilon, &mut asset.bounds)?;
        }
        let mut triangles = Vec::new();
        for face in &solid.faces {
            let face_mesh = face
                .mesh(
                    edges,
                    &samples,
                    epsilon,
                    deflection,
                    world,
                    &mut asset.bounds,
                )
                .map_err(|error| {
                    format!(
                        "native face #{}: {error}",
                        asset.source_faces[face.source].face_id
                    )
                })?;
            if face_mesh.is_empty() {
                return Err("native face produced no triangles".into());
            }
            let tag = asset.face_occurrences.len();
            let repair_displacement_mm = repaired.as_ref().map_or(0.0, |repair| {
                face.loops
                    .iter()
                    .flat_map(|ring| &ring.edges)
                    .filter_map(|(id, _)| repair.movement_mm.get(id).copied())
                    .fold(0.0, f64::max)
            });
            asset.relaxed_beyond_declared_precision |=
                uncertainty.is_some_and(|declared| repair_displacement_mm > declared);
            asset.face_occurrences.push(StepFaceOccurrence {
                source_face: face.source,
                occurrence: index,
                representation_id: representation,
                source_uncertainty_mm: uncertainty,
                repair_displacement_mm,
            });
            asset
                .triangle_faces
                .extend(std::iter::repeat_n(tag, face_mesh.len()));
            let mirrored = placement.determinant() < 0.0;
            triangles.extend(face_mesh.into_iter().map(|mut triangle| {
                if mirrored {
                    triangle.swap(1, 2);
                }
                triangle.map(|point| world.transform_point(point))
            }));
        }
        let origin = triangles
            .first()
            .ok_or("native solid produced no triangles")?[0];
        let mut volume = 0.0;
        for &[a, b, c] in &triangles {
            for p in [a, b, c] {
                check_coordinate(p)?;
                asset.positions.push(p.into());
            }
            volume += (a - origin).dot((b - origin).cross(c - origin)) / 6.0;
            crate::mesh::push_triangle(&mut asset.mesh, [a, b, c])?;
        }
        if !volume.is_finite() || volume <= 0.0 || !closed_oriented_mesh(&triangles) {
            return Err("native preview is not a positively oriented closed mesh".into());
        }
    }
    Ok(asset)
}

impl Solid {
    fn read(
        id: u64,
        records: &SourceRecords<'_>,
        scene: &scene::Scene<'_>,
        cone_angles: &HashMap<u64, f64>,
        catalogue: &mut Vec<StepSourceFace>,
    ) -> Result<Self, String> {
        let solid = named_parameters(records, id, "MANIFOLD_SOLID_BREP", 2)?;
        let shell = named_parameters(records, reference(&solid[1])?, "CLOSED_SHELL", 2)?;
        let mut faces = Vec::new();
        let mut edges = HashMap::new();
        let mut uses = HashMap::<u64, Vec<(u64, bool)>>::new();
        let mut spherical_supports = HashSet::new();
        for face_id in references(&shell[1])? {
            let face = named_parameters(records, face_id, "ADVANCED_FACE", 4)?;
            let support_id = reference(&face[2])?;
            let support = simple_record(records, support_id)?;
            let surface = Surface::read(support, scene, cone_angles.get(&support_id).copied())?;
            if matches!(surface.kind, SurfaceKind::Sphere(_)) {
                spherical_supports.insert(support_id);
            }
            let same_sense = flag(&face[3])?;
            let mut loops = Vec::new();
            for bound_id in references(&face[1])? {
                let bound = simple_record(records, bound_id)?;
                let explicit_outer = match bound.name.as_str() {
                    "FACE_OUTER_BOUND" => true,
                    "FACE_BOUND" => false,
                    _ => return Err("unsupported native face bound".into()),
                };
                let bound = parameters(bound, 3)?;
                let ring = named_parameters(records, reference(&bound[1])?, "EDGE_LOOP", 2)?;
                let mut boundary = Vec::new();
                for oriented_id in references(&ring[1])? {
                    let item = named_parameters(records, oriented_id, "ORIENTED_EDGE", 5)?;
                    if !matches!(
                        (&item[1], &item[2]),
                        (Parameter::Omitted, Parameter::Omitted)
                    ) {
                        return Err("explicit ORIENTED_EDGE vertex override".into());
                    }
                    let edge_id = reference(&item[3])?;
                    if let std::collections::hash_map::Entry::Vacant(entry) = edges.entry(edge_id) {
                        entry.insert(
                            Edge::read(edge_id, records, scene, cone_angles)
                                .map_err(|error| format!("native edge #{edge_id}: {error}"))?,
                        );
                    }
                    boundary.push((edge_id, flag(&item[4])?));
                }
                if !flag(&bound[2])? {
                    boundary.reverse();
                    for (_, forward) in &mut boundary {
                        *forward = !*forward;
                    }
                }
                let endpoints = |(id, forward): (u64, bool)| {
                    let [a, b] = edges[&id].vertices;
                    if forward { [a, b] } else { [b, a] }
                };
                for (i, &(id, forward)) in boundary.iter().enumerate() {
                    if endpoints((id, forward))[1]
                        != endpoints(boundary[(i + 1) % boundary.len()])[0]
                    {
                        return Err(format!(
                            "source face #{face_id} has a disconnected boundary"
                        ));
                    }
                    uses.entry(id).or_default().push((support_id, forward));
                }
                loops.push(Boundary {
                    explicit_outer,
                    edges: boundary,
                });
            }
            faces.push(Face {
                source: catalogue.len(),
                surface,
                same_sense,
                loops,
            });
            catalogue.push(StepSourceFace {
                solid_id: id,
                shell_index: 0,
                face_id,
                support_id,
                support_kind: support.name.clone(),
            });
        }
        for (id, uses) in uses {
            let [(a, forward), (b, reverse)] = uses[..] else {
                return Err("incomplete or nonmanifold native shell edge uses".into());
            };
            if forward == reverse {
                return Err("incomplete or nonmanifold native shell edge uses".into());
            }
            // The spherical slice covers fillets that meet cylindrical strips.
            for (support, adjacent) in [(a, b), (b, a)] {
                if spherical_supports.contains(&support)
                    && named_record(records, adjacent, "CYLINDRICAL_SURFACE").is_err()
                {
                    return Err(format!(
                        "native edge #{id}: spherical fillet requires a cylindrical neighbor"
                    ));
                }
            }
            // Both adjacent oblique strips must certify the reduced trim
            // against their supports over the full finite interval.
            if edges[&id].finite_ellipse.is_some()
                && [a, b]
                    .into_iter()
                    .any(|support| named_record(records, support, "CYLINDRICAL_SURFACE").is_err())
            {
                return Err(format!(
                    "native edge #{id}: rational elliptic trim requires two cylindrical face uses"
                ));
            }
            if let Some(carrier) = &edges[&id].carrier
                && carrier.supports != [a, b]
                && carrier.supports != [b, a]
            {
                return Err(format!(
                    "native edge #{id}: pcurve supports differ from face uses"
                ));
            }
        }
        Ok(Self { edges, faces })
    }
}

fn named_parameters<'a>(
    records: &SourceRecords<'a>,
    id: u64,
    kind: &str,
    count: usize,
) -> Result<&'a [Parameter], String> {
    let params = parameters(named_record(records, id, kind)?, count)?;
    if !matches!(params.first(), Some(Parameter::String(_))) {
        return Err(format!("invalid {kind} name for #{id}"));
    }
    Ok(params)
}

fn references(list: &Parameter) -> Result<Vec<u64>, String> {
    let Parameter::List(list) = list else {
        return Err("missing native reference aggregate".into());
    };
    let ids = list.iter().map(reference).collect::<Result<Vec<_>, _>>()?;
    if ids.is_empty() || ids.iter().copied().collect::<HashSet<_>>().len() != ids.len() {
        return Err("empty or repeated native topology references".into());
    }
    Ok(ids)
}

fn flag(param: &Parameter) -> Result<bool, String> {
    match param {
        Parameter::Enumeration(value) if value == "T" => Ok(true),
        Parameter::Enumeration(value) if value == "F" => Ok(false),
        _ => Err("invalid native orientation flag".into()),
    }
}

fn check_coordinate(p: Point3) -> Result<(), String> {
    if [p.x, p.y, p.z]
        .iter()
        .any(|v| !(*v as f32).is_finite() || (v * 1e7).abs() >= i64::MAX as f64)
    {
        return Err("nonfinite or unsupported mesh coordinate range".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
