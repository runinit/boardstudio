use monstertruck_io::step::load::step_p21::{
    ast::{EntityInstance, Record},
    parser,
};
use monstertruck_io::step::load::{
    convert::*,
    report::{LossCategory, LossReason},
    *,
};
use monstertruck_meshing::prelude::*;
use monstertruck_modeling::{Matrix4, Point3, Rad, SquareMatrix, Transform, Vector3};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

#[path = "../unit_normalization.rs"]
mod unit_normalization;

type Shell = StepCompressedShell;

// Preserve source certificates before any unit/placement mutation.
enum ImportedSolid {
    Normalized(Vec<Shell>),
    SourceBound {
        solid: SourceBoundStepSolid,
        mm_per_unit: f64,
        options: monstertruck_meshing::tessellation::TessellationOptions,
    },
}
impl ImportedSolid {
    fn shells(&self) -> Vec<&Shell> {
        match self {
            Self::Normalized(shells) => shells.iter().collect(),
            Self::SourceBound { solid, .. } => solid.shells().iter().map(|s| s.shell()).collect(),
        }
    }
    fn mesh(
        &self,
        index: usize,
    ) -> Result<monstertruck_qualification::step_meshing::StepMesh, String> {
        match self {
            Self::Normalized(shells) => Ok(shells[index].robust_triangulation(0.0005)),
            Self::SourceBound {
                solid,
                mm_per_unit,
                options,
            } => monstertruck_qualification::step_meshing::triangulate_source_bound_shell(
                &solid.shells()[index],
                *mm_per_unit,
                *options,
            )
            .map(|result| result.mesh),
        }
    }
}

fn main() {
    self_check();
    let mut paths = Vec::<PathBuf>::new();
    let mut source_certified = false;
    let mut angular = None;
    for arg in std::env::args_os().skip(1) {
        if arg == "--source-certified-meshing" {
            source_certified = true;
        } else if let Some(value) = arg
            .to_str()
            .and_then(|s| s.strip_prefix("--edge-angular-tolerance="))
        {
            let angle = value
                .parse::<f64>()
                .ok()
                .filter(|a| a.is_finite() && *a > 0.0)
                .unwrap_or_else(|| {
                    eprintln!("edge angular tolerance must be finite positive radians");
                    std::process::exit(2)
                });
            angular = Some(angle);
        } else {
            paths.push(arg.into());
        }
    }
    if angular.is_some() && !source_certified {
        eprintln!("--edge-angular-tolerance requires --source-certified-meshing");
        std::process::exit(2);
    }
    let options =
        source_certified.then_some(monstertruck_meshing::tessellation::TessellationOptions {
            tolerance: 0.0005,
            edge_angular_tolerance: angular,
            ..Default::default()
        });
    let paths = if paths.is_empty() {
        [
            "ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Cap_Green_T430.step",
            "ergogen/library/vendor/thqwgd001/3d_models/THQWGD001-rotation.stp",
            "ergogen/library/vendor/kicad/3d_models/PinSocket_1x05_P2.54mm_Vertical.step",
            "ergogen/library/vendor/kiswitch/3d_models/SW_Hotswap_Kailh_MX.stp",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect()
    } else {
        paths
    };
    let mut failed = false;
    for path in paths {
        match inspect(&path, options) {
            Ok(result) => {
                println!("{}", result);
                failed |= result.contains("gate=ERROR");
            }
            Err(error) => {
                println!("file={} gate=ERROR error={}", path.display(), error);
                failed = true;
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
}

fn self_check() {
    let transform = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0))
        * Matrix4::from_angle_z(Rad(std::f64::consts::FRAC_PI_2));
    let p = transform.transform_point(Point3::new(1.0, 0.0, 0.0));
    assert!((p.x - 5.0).abs() < 1e-10 && (p.y - 1.0).abs() < 1e-10);
    let scaled_translation = unit_normalization::placement_translation_to_mm(
        Matrix4::from_translation(Vector3::new(1.0, 2.0, 3.0))
            * Matrix4::from_angle_z(Rad(std::f64::consts::FRAC_PI_2)),
        1000.0,
    );
    let p = scaled_translation.transform_point(Point3::new(1000.0, 0.0, 0.0));
    assert!((p.x - 1000.0).abs() < 1e-10);
    assert!((p.y - 3000.0).abs() < 1e-10);
    assert!((p.z - 3000.0).abs() < 1e-10);
    assert!(unit_normalization::validate_placement_matrix(Matrix4::identity()).is_ok());
    assert!(unit_normalization::normalized_placement_mm(&NodeMatrix::Identity, 1.0, 1.0).is_ok());
    assert!(unit_normalization::normalized_placement_mm(&NodeMatrix::Identity, 25.4, 1.0).is_err());
    assert!(unit_normalization::validate_placement_matrix(Matrix4::from_scale(0.0)).is_err());
    let generic_rotation = Matrix4::from_angle_x(Rad(0.37))
        * Matrix4::from_angle_y(Rad(-0.81))
        * Matrix4::from_angle_z(Rad(1.21));
    assert!((generic_rotation.determinant() - 1.0).abs() < 1e-12);
    assert!(unit_normalization::validate_placement_matrix(generic_rotation).is_ok());
    let typed_uniform = "ISO-10303-21;HEADER;FILE_DESCRIPTION(('gate'),'2;1');FILE_NAME('gate','','',(),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#2=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));#3=SHAPE_REPRESENTATION('shape',(),#2);ENDSEC;END-ISO-10303-21;";
    assert_eq!(
        unit_normalization::resolve_mm_per_unit(typed_uniform).unwrap(),
        1.0
    );

    let [a, b, c, d] = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
    ];
    let tetra = [[a, c, b], [a, b, d], [a, d, c], [b, c, d]];
    assert_eq!(mesh_edge_issues(&tetra, 1.0), MeshEdgeIssues::default());
    let tetra_inches =
        tetra.map(|face| face.map(|p| Point3::new(p.x / 25.4, p.y / 25.4, p.z / 25.4)));
    assert_eq!(
        mesh_edge_issues(&tetra_inches, 25.4),
        mesh_edge_issues(&tetra, 1.0)
    );
    let four_uses: Vec<_> = tetra.into_iter().chain(tetra).collect();
    assert_eq!(mesh_edge_issues(&four_uses, 1.0).invalid_edges, 6);
    let collapsed = mesh_edge_issues(&[[a, a, b]], 1.0);
    assert_eq!(collapsed.degenerate_edges, 1);
}

fn inspect(
    path: &PathBuf,
    source_options: Option<monstertruck_meshing::tessellation::TessellationOptions>,
) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let text = unit_normalization::decode_step_bytes(&bytes);
    let source_occurrence_ids = source_occurrence_ids(&text)?;
    let mm_per_unit_by_representation =
        unit_normalization::resolve_mm_per_unit_by_representation(&text)?;
    let shape_items = unit_normalization::shape_representation_items(&text)?;
    let angle_normalized_exchange = unit_normalization::normalize_conical_angles(&text)?;
    let table = Table::from_data_section(&angle_normalized_exchange.data[0]);
    validate_pds_definition_targets(&table)?;
    let entity_loss = table
        .entity_report
        .require_empty()
        .err()
        .map(|e| e.to_string());
    let dummy_types = dummy_entity_counts(&table);
    let mut loss = Vec::new();
    if let Some(error) = entity_loss.as_ref() {
        loss.push(format!("entity:{error}"));
    }

    let mut shells: HashMap<u64, ImportedSolid> = HashMap::new();
    let mut brep_error = None;
    let mut ids: Vec<_> = table.manifold_solid_brep.keys().copied().collect();
    ids.sort_unstable();
    let solid_units = unit_normalization::solid_representation_units(
        &ids,
        &shape_items,
        &mm_per_unit_by_representation,
    )?;
    for id in &ids {
        let holder = &table.manifold_solid_brep[id];
        if let Some(options) = source_options {
            let mm_per_unit = solid_units
                .get(id)
                .copied()
                .ok_or_else(|| format!("solid #{id} has no resolved representation unit"))?;
            match table.to_source_bound_solid_checked(holder) {
                Ok(solid) => {
                    shells.insert(
                        *id,
                        ImportedSolid::SourceBound {
                            solid,
                            mm_per_unit,
                            options,
                        },
                    );
                }
                Err(error) => brep_error = Some(format!("solid#{id}:{error}")),
            }
            continue;
        }
        match table.to_compressed_solid_reported(holder) {
            Ok((solid, reports)) => {
                for report in reports {
                    if let Err(error) = report.require_lossless() {
                        loss.push(format!("solid#{id}:{error}"));
                    }
                }
                let mut boundaries = solid.boundaries;
                let mm_per_unit = solid_units
                    .get(id)
                    .copied()
                    .ok_or_else(|| format!("solid #{id} has no resolved representation unit"))?;
                for shell in &mut boundaries {
                    unit_normalization::normalize_shell_to_mm(shell, mm_per_unit);
                }
                shells.insert(*id, ImportedSolid::Normalized(boundaries));
            }
            Err(error) => brep_error = Some(format!("solid#{id}:{error}")),
        }
    }

    // step_assy is mandatory: malformed graphs and matrices never become identity placements.
    let assembly = table.step_assy().map_err(|e| format!("assembly: {e}"))?;
    let standalone = assembly.is_empty()
        && ids.len() == 1
        && table.product_definition_shape.is_empty()
        && table.next_assembly_usage_occurrence.is_empty();
    if assembly.is_empty() && !standalone {
        return Err("assembly graph absent for a non-standalone STEP model".into());
    }
    let assembly_items: Vec<Vec<u64>> = assembly
        .node_indices()
        .map(|index| assembly.node(index).shape().clone())
        .collect();
    let assembly_edges: usize = assembly
        .node_indices()
        .map(|index| assembly.node(index).edges().count())
        .sum();
    let typed_occurrences = table.next_assembly_usage_occurrence.len();
    let typed_occurrence_ids: HashSet<_> = table
        .next_assembly_usage_occurrence
        .keys()
        .copied()
        .collect();
    let occurrence_pds_ids = occurrence_product_definition_shape_ids(&table);
    let occurrence_cdsr_links = occurrence_context_dependent_shape_links(&table);
    validate_occurrence_identity(
        &source_occurrence_ids,
        &typed_occurrence_ids,
        &occurrence_pds_ids,
        &occurrence_cdsr_links,
    )?;
    let declared_assembly_occurrences = source_occurrence_ids.len();
    let mut traversed_assembly_edges = HashSet::new();
    let mut instances = Vec::<(u64, Matrix4)>::new();
    let mut relationship_count = 0usize;
    let mut related_item_losses = 0usize;
    let mut ignored_axis_placement_items = 0usize;
    let mut unclassified_related_items = 0usize;
    let mut unresolved_relationships = 0usize;
    let mut ignored_wireframe_relationships = 0usize;
    let mut relationship_loss_details = Vec::new();
    if standalone {
        if shells.contains_key(&ids[0]) {
            instances.push((ids[0], Matrix4::identity()));
        }
    } else {
        for top in assembly.top_nodes() {
            for path in assembly.paths_iter(top.index()) {
                let nodes = path.nodes();
                if nodes.len() != path.edges().len() + 1 {
                    return Err("assembly path node/edge sequence is inconsistent".into());
                }
                let mut transform = Matrix4::identity();
                for (edge_index, edge) in path.edges().iter().enumerate() {
                    traversed_assembly_edges.insert(std::ptr::from_ref(edge.entity()) as usize);
                    let parent = nodes[edge_index].entity();
                    let child = nodes[edge_index + 1].entity();
                    let parent_representation =
                        table.shape_representation_of_node(parent).ok_or_else(|| {
                            "assembly parent node lacks a shape representation".to_owned()
                        })?;
                    let child_representation =
                        table.shape_representation_of_node(child).ok_or_else(|| {
                            "assembly child node lacks a shape representation".to_owned()
                        })?;
                    let parent_factor = mm_per_unit_by_representation
                        .get(&parent_representation)
                        .copied()
                        .ok_or_else(|| format!("parent representation #{parent_representation} lacks a resolved unit"))?;
                    let child_factor = mm_per_unit_by_representation
                        .get(&child_representation)
                        .copied()
                        .ok_or_else(|| {
                            format!(
                                "child representation #{child_representation} lacks a resolved unit"
                            )
                        })?;
                    let next = unit_normalization::normalized_placement_mm(
                        edge.matrix(),
                        child_factor,
                        parent_factor,
                    )?;
                    transform = transform * next;
                }
                for id in path.terminal_node().shape() {
                    if shells.contains_key(id) {
                        instances.push((*id, transform));
                    }
                }
                let node = path.terminal_node();
                if let Some(representation) = table.shape_representation_of_node(node.entity()) {
                    let (related, report) = table.solids_via_shape_relationship(representation);
                    relationship_count += report.count(LossCategory::Representation).listed;
                    related_item_losses += report.lost(LossCategory::Solid);
                    relationship_loss_details.extend(report.losses().map(
                        |(category, reason, tally)| {
                            if reason == LossReason::RelatedRepresentationUnresolved {
                                if tally.count == 1
                                    && tally
                                        .first_id
                                        .is_some_and(|id| is_wireframe_representation(&text, id))
                                {
                                    ignored_wireframe_relationships += tally.count;
                                } else {
                                    unresolved_relationships += tally.count;
                                }
                            } else if reason == LossReason::RepresentationItemNotASolid {
                                if tally.count == 1
                                    && tally
                                        .first_detail
                                        .as_deref()
                                        .is_some_and(|detail| detail.contains("AXIS2_PLACEMENT_3D"))
                                {
                                    ignored_axis_placement_items += tally.count;
                                } else {
                                    unclassified_related_items += tally.count;
                                }
                            }
                            format!(
                                "{category:?}/{reason:?}:{}@{:?}={:?}",
                                tally.count, tally.first_id, tally.first_detail
                            )
                        },
                    ));
                    for id in related {
                        if shells.contains_key(&id) {
                            instances.push((id, transform));
                        }
                    }
                }
            }
        }
    }
    validate_assembly_completeness(
        declared_assembly_occurrences,
        assembly_edges,
        traversed_assembly_edges.len(),
    )?;
    let missing: Vec<_> = ids
        .iter()
        .filter(|id| {
            shells.contains_key(id) && !instances.iter().any(|(placed_id, _)| placed_id == *id)
        })
        .collect();
    let unreferenced = !missing.is_empty();
    let no_instances = instances.is_empty();
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut signed_volume = 0.0;
    let mut shell_count = 0;
    let mut unmeshed_faces = 0usize;
    let mut empty_triangle_faces = 0usize;
    let mut empty_triangle_face_ids = Vec::new();
    let mut invalid_mesh_edges = 0usize;
    let mut degenerate_mesh_edges = 0usize;
    let mut mesh_triangles = 0usize;
    let mut pcurve_edge_uses = 0usize;
    let mut pcurve_direct_matches = 0usize;
    for (id, matrix) in &instances {
        for (shell_index, shell) in shells[id].shells().into_iter().enumerate() {
            shell_count += 1;
            let (direct, uses) = unit_normalization::pcurve_edge_use_counts(shell);
            pcurve_direct_matches += direct;
            pcurve_edge_uses += uses;
            let mesh = shells[id].mesh(shell_index)?;
            let mut triangles = Vec::new();
            for (face_index, face) in mesh.faces.iter().enumerate() {
                match append_face_triangles(
                    face.surface.as_ref(),
                    face.orientation,
                    matrix,
                    &mut triangles,
                ) {
                    None => unmeshed_faces += 1,
                    Some(0) => {
                        empty_triangle_faces += 1;
                        empty_triangle_face_ids.push(format!("{id}:{shell_index}:{face_index}"));
                    }
                    Some(_) => {}
                }
            }
            signed_volume += triangles
                .iter()
                .map(|[a, b, c]| {
                    (a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x)
                        + a.z * (b.x * c.y - b.y * c.x))
                        / 6.0
                })
                .sum::<f64>();
            mesh_triangles += triangles.len();
            let edge_issues = mesh_edge_issues(&triangles, 1.0);
            invalid_mesh_edges += edge_issues.invalid_edges;
            degenerate_mesh_edges += edge_issues.degenerate_edges;
            for tri in &triangles {
                for p in tri {
                    for (i, v) in [p.x, p.y, p.z].into_iter().enumerate() {
                        min[i] = min[i].min(v);
                        max[i] = max[i].max(v);
                    }
                }
            }
        }
    }
    let gate_error = entity_loss.is_some()
        || !loss.is_empty()
        || brep_error.is_some()
        || unreferenced
        || no_instances
        || unmeshed_faces > 0
        || empty_triangle_faces > 0
        || unresolved_relationships > 0
        || unclassified_related_items > 0
        || invalid_mesh_edges > 0
        || degenerate_mesh_edges > 0
        || signed_volume <= 0.0;
    let bounds_delta = if path.ends_with("THQWGD001-rotation.stp") {
        Some(max_abs_bounds_delta(
            min,
            max,
            [-9.216318757, -8.812124802, -3.5],
            [9.166318857, 9.183731752, 20.372216785],
        ))
    } else {
        None
    };
    let gate_error = gate_error || bounds_delta.is_some_and(|delta| delta > 0.03);
    Ok(format!(
        "file={} bytes={} sha256={} ms={} solids={} dummy_types={:?} assembly_nodes={} assembly_edges={} assembly_items={:?} source_occurrences={} typed_occurrences={} pds_unlinked_nauo={} pds_duplicate_link_nauo={} cdsr_unlinked_occurrence_pds={} cdsr_duplicate_occurrence_pds={} relationships={} related_item_losses={} ignored_axis_placement_items={} unclassified_related_items={} unresolved_relationships={} ignored_wireframe_relationships={} relationship_loss_details={:?} instances={} shells={} units_mm_by_representation={:?} pcurve_direct_matches={}/{} volume_mm3={} bounds_mm={:?}..{:?} bounds_oracle_max_delta_mm={:?} mesh_triangles={} invalid_mesh_edges={} degenerate_mesh_edges={} unmeshed_faces={} empty_triangle_faces={} empty_triangle_face_ids={:?} entity_loss={} conversion_loss={} brep_error={} placement_error={} unreferenced={} no_instances={} gate={}",
        path.display(),
        bytes.len(),
        sha256(&bytes),
        started.elapsed().as_millis(),
        ids.len(),
        dummy_types,
        assembly_items.len(),
        assembly_edges,
        assembly_items,
        declared_assembly_occurrences,
        typed_occurrences,
        source_occurrence_ids
            .iter()
            .filter(|id| occurrence_pds_ids.get(id).is_none_or(Vec::is_empty))
            .count(),
        source_occurrence_ids
            .iter()
            .filter(|id| occurrence_pds_ids.get(id).is_some_and(|ids| ids.len() > 1))
            .count(),
        occurrence_pds_ids
            .values()
            .filter(|ids| ids.len() == 1
                && occurrence_cdsr_links.get(&ids[0]).copied().unwrap_or(0) == 0)
            .count(),
        occurrence_pds_ids
            .values()
            .filter(|ids| ids.len() == 1
                && occurrence_cdsr_links
                    .get(&ids[0])
                    .is_some_and(|count| *count > 1))
            .count(),
        relationship_count,
        related_item_losses,
        ignored_axis_placement_items,
        unclassified_related_items,
        unresolved_relationships,
        ignored_wireframe_relationships,
        relationship_loss_details,
        instances.len(),
        shell_count,
        BTreeMap::from_iter(
            mm_per_unit_by_representation
                .iter()
                .map(|(id, factor)| (*id, *factor)),
        ),
        pcurve_direct_matches,
        pcurve_edge_uses,
        signed_volume,
        min,
        max,
        bounds_delta,
        mesh_triangles,
        invalid_mesh_edges,
        degenerate_mesh_edges,
        unmeshed_faces,
        empty_triangle_faces,
        empty_triangle_face_ids,
        entity_loss.unwrap_or_else(|| "none".into()),
        if loss.is_empty() {
            "none".into()
        } else {
            loss.join("|").replace(' ', "_")
        },
        brep_error.unwrap_or_else(|| "none".into()),
        "none",
        unreferenced,
        no_instances,
        if gate_error { "ERROR" } else { "PASS" }
    ))
}

type QuantizedPoint = (i64, i64, i64);
type EdgeKey = (QuantizedPoint, QuantizedPoint);

fn append_face_triangles(
    surface: Option<&PolygonMesh>,
    orientation: bool,
    matrix: &Matrix4,
    triangles: &mut Vec<[Point3; 3]>,
) -> Option<usize> {
    let mesh = surface?;
    let initial_count = triangles.len();
    let positions = mesh.positions();
    for polygon in mesh.face_iter() {
        for k in 1..polygon.len().saturating_sub(1) {
            let mut tri =
                [polygon[0].pos, polygon[k].pos, polygon[k + 1].pos].map(|i| positions[i]);
            if !orientation {
                tri.swap(1, 2);
            }
            triangles.push(tri.map(|p| matrix.transform_point(p)));
        }
    }
    Some(triangles.len() - initial_count)
}

#[cfg(test)]
mod empty_face_tests {
    use super::*;

    #[test]
    fn none_surface_remains_unmeshed() {
        let mut triangles = Vec::new();
        assert_eq!(
            append_face_triangles(None, true, &Matrix4::identity(), &mut triangles),
            None
        );
        assert!(triangles.is_empty());
    }

    #[test]
    fn present_empty_polygon_mesh_is_detected_from_actual_output() {
        let mesh = PolygonMesh::default();
        let mut triangles = Vec::new();
        assert_eq!(
            append_face_triangles(Some(&mesh), true, &Matrix4::identity(), &mut triangles),
            Some(0)
        );
        assert!(triangles.is_empty());
    }

    #[test]
    fn present_triangle_mesh_emits_one_triangle() {
        let positions = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut faces = Faces::default();
        faces.push([0usize, 1, 2]);
        let mesh = PolygonMesh::new(
            StandardAttributes {
                positions,
                ..Default::default()
            },
            faces,
        );
        let mut triangles = Vec::new();
        assert_eq!(
            append_face_triangles(Some(&mesh), true, &Matrix4::identity(), &mut triangles),
            Some(1)
        );
        assert_eq!(triangles.len(), 1);
    }
}

fn validate_assembly_completeness(
    declared_occurrences: usize,
    decoded_edges: usize,
    traversed_edges: usize,
) -> Result<(), String> {
    if declared_occurrences != decoded_edges {
        return Err(format!(
            "incomplete assembly import: {declared_occurrences} NEXT_ASSEMBLY_USAGE_OCCURRENCE records but {decoded_edges} decoded graph edges"
        ));
    }
    if decoded_edges != traversed_edges {
        return Err(format!(
            "incomplete assembly import: {decoded_edges} decoded graph edges but only {traversed_edges} occur in assembly paths"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod assembly_completeness_tests {
    use super::validate_assembly_completeness;

    #[test]
    fn accepts_complete_and_standalone_graphs() {
        assert!(validate_assembly_completeness(5, 5, 5).is_ok());
        assert!(validate_assembly_completeness(0, 0, 0).is_ok());
    }

    #[test]
    fn rejects_dropped_or_invented_occurrences() {
        assert!(
            validate_assembly_completeness(5, 4, 4)
                .unwrap_err()
                .contains("5 NEXT_ASSEMBLY_USAGE_OCCURRENCE records but 4 decoded graph edges")
        );
        assert!(
            validate_assembly_completeness(5, 5, 4)
                .unwrap_err()
                .contains("5 decoded graph edges but only 4 occur in assembly paths")
        );
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MeshEdgeIssues {
    invalid_edges: usize,
    degenerate_edges: usize,
}

#[derive(Default)]
struct EdgeUses {
    count: usize,
    direction: i64,
}

fn mesh_edge_issues(triangles: &[[Point3; 3]], mm_per_unit: f64) -> MeshEdgeIssues {
    let point = |p: &Point3| {
        (
            (p.x * mm_per_unit * 1e7).round() as i64,
            (p.y * mm_per_unit * 1e7).round() as i64,
            (p.z * mm_per_unit * 1e7).round() as i64,
        )
    };
    let mut counts: HashMap<EdgeKey, EdgeUses> = HashMap::new();
    let mut degenerate_edges = 0;
    for tri in triangles {
        for i in 0..3 {
            let (a, b) = (point(&tri[i]), point(&tri[(i + 1) % 3]));
            if a == b {
                degenerate_edges += 1;
                continue;
            }
            let (key, direction) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            let uses = counts.entry(key).or_default();
            uses.count += 1;
            uses.direction += direction;
        }
    }
    MeshEdgeIssues {
        invalid_edges: counts
            .values()
            .filter(|uses| uses.count != 2 || uses.direction != 0)
            .count(),
        degenerate_edges,
    }
}

fn max_abs_bounds_delta(
    min: [f64; 3],
    max: [f64; 3],
    expected_min: [f64; 3],
    expected_max: [f64; 3],
) -> f64 {
    (0..3)
        .map(|i| {
            (min[i] - expected_min[i])
                .abs()
                .max((max[i] - expected_max[i]).abs())
        })
        .fold(0.0, f64::max)
}

fn dummy_entity_counts(table: &Table) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for dummy in table.dummy.values() {
        let name = dummy.record.split('"').nth(1).unwrap_or(&dummy.record);
        let entity = name
            .trim_start_matches('(')
            .split(['(', ')', ','])
            .next()
            .unwrap_or(name)
            .to_owned();
        *counts.entry(entity).or_default() += 1;
    }
    counts
}

fn is_wireframe_representation(text: &str, id: u64) -> bool {
    let marker = format!("#{id}=");
    let Some(start) = text.find(&marker).map(|i| i + marker.len()) else {
        return false;
    };
    text[start..]
        .trim_start()
        .trim_start_matches('(')
        .split(['(', ')', ','])
        .next()
        == Some("GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION")
}

fn sha256(bytes: &[u8]) -> String {
    use std::io::Write;
    let mut child = std::process::Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("sha256sum required for evidence");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned()
}

fn validate_pds_definition_targets(table: &Table) -> Result<(), String> {
    for (&pds_id, pds) in &table.product_definition_shape {
        if let monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
            monstertruck_io::step::load::step_p21::ast::Name::Entity(definition_id),
        ) = &pds.definition
        {
            let target_exists = table.product_definition.contains_key(definition_id)
                || table
                    .next_assembly_usage_occurrence
                    .contains_key(definition_id)
                || table.product_definition_shape.contains_key(definition_id);
            if !target_exists {
                return Err(format!(
                    "incomplete assembly import: PRODUCT_DEFINITION_SHAPE #{pds_id} references missing definition entity #{definition_id}"
                ));
            }
        }
    }
    Ok(())
}

fn source_occurrence_ids(text: &str) -> Result<HashSet<u64>, String> {
    let exchange = parser::parse(text).map_err(|error| format!("STEP AST: {error}"))?;
    Ok(exchange
        .data
        .iter()
        .flat_map(|section| &section.entities)
        .filter_map(|entity| match entity {
            EntityInstance::Simple { id, record } => {
                (record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE").then_some(*id)
            }
            EntityInstance::Complex { id, subsuper } => subsuper
                .0
                .iter()
                .any(|record: &Record| record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
                .then_some(*id),
        })
        .collect())
}

fn occurrence_product_definition_shape_ids(table: &Table) -> HashMap<u64, Vec<u64>> {
    let mut links = HashMap::<u64, Vec<u64>>::new();
    for (&pds_id, pds) in &table.product_definition_shape {
        match &pds.definition {
            monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
                monstertruck_io::step::load::step_p21::ast::Name::Entity(id),
            ) if table.next_assembly_usage_occurrence.contains_key(id) => {
                links.entry(*id).or_default().push(pds_id);
            }
            _ => {}
        }
    }
    links
}

fn occurrence_context_dependent_shape_links(table: &Table) -> HashMap<u64, usize> {
    let mut links = HashMap::new();
    for cdsr in table.context_dependent_shape_representation.values() {
        if let monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
            monstertruck_io::step::load::step_p21::ast::Name::Entity(pds_id),
        ) = &cdsr.represented_product_relation
        {
            *links.entry(*pds_id).or_default() += 1;
        }
    }
    links
}

fn validate_occurrence_identity(
    source_ids: &HashSet<u64>,
    typed_ids: &HashSet<u64>,
    pds_ids_by_nauo: &HashMap<u64, Vec<u64>>,
    cdsr_links_by_pds: &HashMap<u64, usize>,
) -> Result<(), String> {
    if source_ids != typed_ids {
        let source_missing: Vec<_> = source_ids.difference(typed_ids).copied().collect();
        let table_extra: Vec<_> = typed_ids.difference(source_ids).copied().collect();
        return Err(format!(
            "incomplete assembly import: source/typed NAUO entity IDs differ (missing typed={source_missing:?}, extra typed={table_extra:?})"
        ));
    }
    for id in source_ids {
        let pds_ids = pds_ids_by_nauo
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if pds_ids.len() != 1 {
            return Err(format!(
                "incomplete assembly import: NEXT_ASSEMBLY_USAGE_OCCURRENCE #{id} has {} PRODUCT_DEFINITION_SHAPE definition links; exactly one is supported",
                pds_ids.len()
            ));
        }
        let cdsr_links = cdsr_links_by_pds.get(&pds_ids[0]).copied().unwrap_or(0);
        if cdsr_links != 1 {
            return Err(format!(
                "incomplete assembly import: occurrence PRODUCT_DEFINITION_SHAPE #{} has {cdsr_links} CONTEXT_DEPENDENT_SHAPE_REPRESENTATION links; exactly one is supported",
                pds_ids[0]
            ));
        }
    }
    if pds_ids_by_nauo.keys().any(|id| !source_ids.contains(id)) {
        return Err(
            "incomplete assembly import: typed PDS links reference a non-source NAUO".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod source_occurrence_tests {
    use super::source_occurrence_ids;
    use std::collections::HashSet;

    #[test]
    fn parsed_occurrence_count_ignores_quoted_decoys() {
        let step = "ISO-10303-21;HEADER;FILE_DESCRIPTION(('NEXT_ASSEMBLY_USAGE_OCCURRENCE'),'2;1');FILE_NAME('gate','','',(),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;#1=NEXT_ASSEMBLY_USAGE_OCCURRENCE('NEXT_ASSEMBLY_USAGE_OCCURRENCE','x','',#2,#3,$);ENDSEC;END-ISO-10303-21;";
        assert_eq!(source_occurrence_ids(step).unwrap(), HashSet::from([1]));
    }
}

#[cfg(test)]
mod occurrence_identity_tests {
    use super::validate_occurrence_identity;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn requires_exact_source_typed_ids_and_unique_pds_cdsr_links() {
        let source = HashSet::from([1, 2, 3]);
        let typed = HashSet::from([1, 2, 3]);
        let pds_ids = HashMap::from([(1, vec![11]), (2, vec![12]), (3, vec![13])]);
        let cdsr_links = HashMap::from([(11, 1), (12, 1), (13, 1)]);
        assert!(validate_occurrence_identity(&source, &typed, &pds_ids, &cdsr_links).is_ok());

        let replaced = HashSet::from([1, 2, 4]);
        assert!(
            validate_occurrence_identity(&source, &replaced, &pds_ids, &cdsr_links)
                .unwrap_err()
                .contains("source/typed NAUO entity IDs differ")
        );

        // Balanced dropped/duplicated PDS links leave the same number of graph edges.
        let one_source = HashSet::from([1]);
        let one_pds_link_map = HashMap::from([(1, Vec::new())]);
        assert!(
            validate_occurrence_identity(&one_source, &one_source, &one_pds_link_map, &cdsr_links)
                .unwrap_err()
                .contains("#1 has 0 PRODUCT_DEFINITION_SHAPE definition links")
        );
        let duplicate_pds = HashMap::from([(2, vec![12, 22])]);
        assert!(
            validate_occurrence_identity(
                &HashSet::from([2]),
                &HashSet::from([2]),
                &duplicate_pds,
                &cdsr_links
            )
            .unwrap_err()
            .contains("#2 has 2 PRODUCT_DEFINITION_SHAPE definition links")
        );

        let duplicate_cdsr = HashMap::from([(11, 2), (12, 1), (13, 1)]);
        assert!(
            validate_occurrence_identity(&source, &typed, &pds_ids, &duplicate_cdsr)
                .unwrap_err()
                .contains("occurrence PRODUCT_DEFINITION_SHAPE #11 has 2 CONTEXT_DEPENDENT_SHAPE_REPRESENTATION links")
        );
    }
}
