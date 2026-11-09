use super::*;
use monstertruck_io::step::load::Table;

const MM: &str = include_str!("../../tests/fixtures/source-closed-cylinder-mm.step");

#[test]
fn checked_cylinder_metadata_is_bound_to_its_source() {
    let table = Table::from_step(MM).unwrap();
    let solid = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    let source = &solid.shells()[0];
    assert_eq!(source.source_face_ids(), &[63, 67, 71]);
    assert_eq!(source.certified_closed_axes()[0], Some([false, true]));
    let result = triangulate_source_bound_shell(
        source,
        1.0,
        TessellationOptions {
            tolerance: 0.05,
            edge_angular_tolerance: Some(0.5),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        result.mesh.faces[0]
            .surface
            .as_ref()
            .is_some_and(|m| !m.faces().is_empty()),
        "checked source-certified closed face must retain fill"
    );
    let mesh = result.mesh.faces[0].surface.as_ref().unwrap();
    let area: f64 = mesh
        .faces()
        .triangle_iter()
        .map(|t| {
            let p = t.map(|v| mesh.uv_coords()[v.uv.unwrap()]);
            ((p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x)).abs()
                / 2.0
        })
        .sum();
    assert!(
        (area - 1.0).abs() < 1e-6,
        "full source cylinder trim area expected, got {area}"
    );
    assert_eq!(result.certified_faces, vec![63]);
}

#[test]
fn checked_source_closed_face_is_meshed_without_caller_metadata() {
    let text = include_str!("../../tests/fixtures/c2-source-closed-face.step");
    let table = Table::from_step(text).unwrap();
    let source = table
        .to_source_bound_shell_checked(&table.shell[&50000])
        .unwrap();
    assert_eq!(source.source_face_ids(), &[12722]);
    assert_eq!(source.certified_closed_axes()[0], Some([false, true]));
    let result = triangulate_source_bound_shell(
        &source,
        1.0,
        TessellationOptions {
            tolerance: 0.1,
            edge_angular_tolerance: Some(0.5),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        result.mesh.faces[0]
            .surface
            .as_ref()
            .is_some_and(|m| !m.faces().is_empty()),
        "checked source-certified closed face must retain fill"
    );
    assert_eq!(result.certified_faces, vec![12722]);
}

use super::unit_normalization as units;

fn options() -> TessellationOptions {
    TessellationOptions {
        tolerance: 0.05,
        edge_angular_tolerance: Some(0.5),
        ..Default::default()
    }
}

#[test]
fn incomplete_conversion_and_missing_source_identity_are_refused() {
    use monstertruck_io::step::load::{FaceAnyHolder, step_p21::tables::PlaceHolder};
    let damaged = MM.replace(
        "CLOSED_SHELL('',(#63,#67,#71))",
        "CLOSED_SHELL('',(#9999,#63,#67,#71))",
    );
    let table = Table::from_step(&damaged).unwrap();
    let (partial, reports) = table
        .to_compressed_solid_reported(&table.manifold_solid_brep[&73])
        .unwrap();
    assert_eq!(partial.boundaries[0].faces.len(), 3);
    assert!(!reports[0].is_lossless());
    assert!(
        table
            .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
            .is_err()
    );

    let mut table = Table::from_step(MM).unwrap();
    table.shell.get_mut(&72).unwrap().cfs_faces[0] =
        PlaceHolder::Owned(FaceAnyHolder::FaceSurface(table.face_surface[&63].clone()));
    assert!(
        table
            .to_compressed_solid_reported(&table.manifold_solid_brep[&73])
            .unwrap()
            .1[0]
            .is_lossless()
    );
    assert!(
        table
            .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
            .unwrap_err()
            .to_string()
            .contains("no entity identity")
    );
}

#[test]
fn closure_flags_do_not_certify_changed_source_rows() {
    let changed = MM.replace(
        "#18=CARTESIAN_POINT('',(1,0,0));",
        "#18=CARTESIAN_POINT('',(1.01,0,0));",
    );
    assert_ne!(MM, changed);
    let table = Table::from_step(&changed).unwrap();
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    assert_eq!(
        source.shells()[0].certified_closed_axes()[0],
        Some([false, false])
    );
    let result = triangulate_source_bound_shell(&source.shells()[0], 1.0, options()).unwrap();
    assert!(result.certified_faces.is_empty());
}

#[test]
fn oriented_source_faces_and_shells_keep_canonical_identity() {
    use monstertruck_io::step::load::{
        OrientedFaceHolder, OrientedShellHolder,
        step_p21::{ast::Name, tables::PlaceHolder},
    };
    let mut table = Table::from_step(MM).unwrap();
    table.oriented_face.insert(
        90,
        OrientedFaceHolder {
            label: String::new(),
            face_element: PlaceHolder::Ref(Name::Entity(63)),
            orientation: false,
        },
    );
    table.shell.get_mut(&72).unwrap().cfs_faces[0] = PlaceHolder::Ref(Name::Entity(90));
    table.oriented_shell.insert(
        91,
        OrientedShellHolder {
            label: String::new(),
            shell_element: PlaceHolder::Ref(Name::Entity(72)),
            orientation: false,
        },
    );
    table.manifold_solid_brep.get_mut(&73).unwrap().outer = PlaceHolder::Ref(Name::Entity(91));
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    assert_eq!(source.shells()[0].source_face_ids(), &[63, 67, 71]);
    assert_eq!(
        source.shells()[0].certified_closed_axes()[0],
        Some([false, true])
    );
    assert!(!source.shells()[0].shell().faces[0].orientation);
}

#[test]
fn source_certificates_are_snapshots_not_rebound_to_a_mutated_table() {
    let mut table = Table::from_step(MM).unwrap();
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    let before = source.shells()[0].shell().faces[0].surface.clone();
    table.cartesian_point.get_mut(&18).unwrap().coordinates[0] += 0.01;
    let changed = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    assert_eq!(
        changed.shells()[0].certified_closed_axes()[0],
        Some([false, false])
    );
    assert_eq!(source.shells()[0].shell().faces[0].surface, before);
    assert_eq!(
        source.shells()[0].certified_closed_axes()[0],
        Some([false, true])
    );
    assert_eq!(
        triangulate_source_bound_shell(&source.shells()[0], 1.0, options())
            .unwrap()
            .certified_faces,
        vec![63]
    );
}

fn mesh_from_text(text: &str) -> SourceCertifiedStepMesh {
    let factors = units::resolve_mm_per_unit_by_representation(text).unwrap();
    let items = units::shape_representation_items(text).unwrap();
    let factor = units::solid_representation_units(&[73], &items, &factors).unwrap()[&73];
    let exchange = units::normalize_conical_angles(text).unwrap();
    let table = Table::from_data_section(&exchange.data[0]);
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    triangulate_source_bound_shell(&source.shells()[0], factor, options()).unwrap()
}

#[test]
fn inch_and_mm_source_geometry_have_equal_physical_sampling_and_all_coordinates() {
    let mm = mesh_from_text(MM);
    let inch = mesh_from_text(include_str!(
        "../../tests/fixtures/source-closed-cylinder-inch.step"
    ));
    assert_eq!(mm.certified_faces, inch.certified_faces);
    assert_eq!(mm.mesh.vertices.len(), inch.mesh.vertices.len());
    let near = |a: Point3, b: Point3| assert!((a - b).magnitude() < 1e-11, "{a:?} != {b:?}");
    for (a, b) in mm.mesh.vertices.iter().zip(&inch.mesh.vertices) {
        near(*a, *b);
    }
    for (a, b) in mm.mesh.edges.iter().zip(&inch.mesh.edges) {
        assert_eq!(
            a.curve.len(),
            b.curve.len(),
            "physical tolerance must select equal shared samples"
        );
        for (a, b) in a.curve.iter().zip(b.curve.iter()) {
            near(*a, *b);
        }
    }
    for (a, b) in mm.mesh.faces.iter().zip(&inch.mesh.faces) {
        let a = a.surface.as_ref().unwrap();
        let b = b.surface.as_ref().unwrap();
        assert_eq!(a.positions().len(), b.positions().len());
        for (a, b) in a.positions().iter().zip(b.positions()) {
            near(*a, *b);
        }
    }
}

#[test]
fn mixed_context_geometry_is_scaled_before_exactly_one_occurrence_placement() {
    use monstertruck_io::step::load::{convert::NodeMatrix, step_p21::tables::IntoOwned};
    use monstertruck_modeling::Transform;
    let inch = include_str!("../../tests/fixtures/source-closed-cylinder-inch.step");
    let extra = "#90=CARTESIAN_POINT('',(1.,0.,0.));#91=AXIS2_PLACEMENT_3D('',#90,#42,#43);#92=CARTESIAN_POINT('',(100.,0.,0.));#93=AXIS2_PLACEMENT_3D('',#92,#42,#43);#94=ITEM_DEFINED_TRANSFORMATION($,$,#91,#93);#96=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#81)));#97=SHAPE_REPRESENTATION('parent',(),#96);";
    let text = inch.replace(
        "ENDSEC;END-ISO-10303-21;",
        &format!("{extra}ENDSEC;END-ISO-10303-21;"),
    );
    let factors = units::resolve_mm_per_unit_by_representation(&text).unwrap();
    assert_eq!(factors[&84], 25.4);
    assert_eq!(factors[&97], 1.0);
    let local = mesh_from_text(&text);
    let table = Table::from_step(&text).unwrap();
    let transform = table.item_defined_transformation[&94]
        .clone()
        .into_owned(&table)
        .unwrap();
    let matrix =
        units::normalized_placement_mm(&NodeMatrix::Transform(Box::new(transform)), 25.4, 1.0)
            .unwrap();
    let point = local.mesh.vertices[0];
    let placed = matrix.transform_point(point);
    assert!((placed.x - (point.x + 74.6)).abs() < 1e-11);
    assert!((placed.y - point.y).abs() < 1e-11 && (placed.z - point.z).abs() < 1e-11);
    assert!(units::normalized_placement_mm(&NodeMatrix::Identity, 25.4, 1.0).is_err());
}

#[test]
fn pcurve_angle_uv_stays_radians_and_planar_uv_normalizes_with_xyz() {
    let mm = include_str!("../../vendor/resources/step/occt-cylinder.step");
    let inch = mm.replace("#114 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );", "#114=(CONVERSION_BASED_UNIT('INCH',#9001)LENGTH_UNIT()NAMED_UNIT(*));#9000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#9001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#9000);");
    assert_ne!(mm, inch);
    let mut meshes = Vec::new();
    for (text, expected) in [(mm, 1.0), (inch.as_str(), 25.4)] {
        let factor = units::resolve_mm_per_unit(text).unwrap();
        assert_eq!(factor, expected);
        let exchange = units::normalize_conical_angles(text).unwrap();
        let table = Table::from_data_section(&exchange.data[0]);
        let source = table
            .to_source_bound_solid_checked(&table.manifold_solid_brep[&15])
            .unwrap();
        assert!(!table.pcurve.is_empty());
        assert!(source.shells()[0].shell().edges.iter().any(|e| matches!(
            e.curve,
            monstertruck_io::step::load::step_geometry::Curve3D::SurfaceCurve(_)
        )));
        let mut opts = options();
        opts.tolerance *= factor;
        meshes.push(triangulate_source_bound_shell(&source.shells()[0], factor, opts).unwrap());
    }
    let a = &meshes[0].mesh;
    let b = &meshes[1].mesh;
    let near = |a: Point3, b: Point3| assert!((a * 25.4 - b).magnitude() < 1e-10);
    for (a, b) in a.vertices.iter().zip(&b.vertices) {
        near(*a, *b);
    }
    for (a, b) in a.edges.iter().zip(&b.edges) {
        assert_eq!(a.curve.len(), b.curve.len());
        for (a, b) in a.curve.iter().zip(b.curve.iter()) {
            near(*a, *b);
        }
    }
    for (index, (a, b)) in a.faces.iter().zip(&b.faces).enumerate() {
        let a = a.surface.as_ref().unwrap();
        let b = b.surface.as_ref().unwrap();
        assert_eq!(a.positions().len(), b.positions().len());
        for (a, b) in a.positions().iter().zip(b.positions()) {
            near(*a, *b);
        }
        assert_eq!(a.uv_coords().len(), b.uv_coords().len());
        for (a, b) in a.uv_coords().iter().zip(b.uv_coords()) {
            let expected = if index == 0 { *a } else { *a * 25.4 };
            assert!(
                (expected - *b).magnitude() < 1e-10,
                "cylinder angles stay radians; planar UV lengths normalize once"
            );
        }
    }
}

#[test]
fn invalid_units_and_tolerances_refuse_without_meshing() {
    let table = Table::from_step(MM).unwrap();
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    for factor in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(triangulate_source_bound_shell(&source.shells()[0], factor, options()).is_err());
    }
    let mut opts = options();
    opts.edge_angular_tolerance = Some(f64::NAN);
    assert!(triangulate_source_bound_shell(&source.shells()[0], 1.0, opts).is_err());
}

#[test]
fn normalized_certificate_refuses_a_different_face_scale_or_surface() {
    use monstertruck_modeling::{Matrix4, Transformed};
    let table = Table::from_step(MM).unwrap();
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    let shell = &source.shells()[0];
    let mut scaled = shell.shell().faces[0].surface.clone();
    scaled.transform_by(Matrix4::from_scale(25.4));
    assert_eq!(
        shell.certified_closed_axes_after_uniform_scale(0, &scaled, 25.4),
        Some([false, true])
    );
    assert_eq!(
        shell.certified_closed_axes_after_uniform_scale(0, &scaled, 1.0),
        None
    );
    assert_eq!(
        shell.certified_closed_axes_after_uniform_scale(1, &scaled, 25.4),
        None
    );
    assert_eq!(
        shell.certified_closed_axes_after_uniform_scale(usize::MAX, &scaled, 25.4),
        None
    );
    let mut wrong = scaled.clone();
    wrong.transform_by(Matrix4::from_scale(1.01));
    assert_eq!(
        shell.certified_closed_axes_after_uniform_scale(0, &wrong, 25.4),
        None
    );
}

#[test]
fn certified_choc_seam_uses_the_assigned_finite_boundary_representative() {
    let text = include_str!("../../tests/fixtures/choc-source-certified-seam.step");
    let table = Table::from_step(text).unwrap();
    let source = table
        .to_source_bound_shell_checked(&table.shell[&50000])
        .unwrap();
    assert_eq!(source.source_face_ids(), &[27343]);
    assert_eq!(source.certified_closed_axes(), &[Some([true, false])]);
    let result = triangulate_source_bound_shell(
        &source,
        1.0,
        TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.certified_faces, vec![27343]);
    assert!(
        result.mesh.faces[0]
            .surface
            .as_ref()
            .is_some_and(|m| !m.faces().is_empty()),
        "source-certified seam must preserve the face when inverse search canonicalizes the equivalent endpoint"
    );
    let mesh = result.mesh.faces[0].surface.as_ref().unwrap();
    // This source face is trimmed by the four sides of its finite chart rectangle.
    let expected_area = 6.28318530718 * 6.284428582167;
    let area: f64 = mesh
        .faces()
        .triangle_iter()
        .map(|triangle| {
            let p = triangle.map(|v| mesh.uv_coords()[v.uv.unwrap()]);
            ((p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x)).abs()
                / 2.0
        })
        .sum();
    assert!(
        (area - expected_area).abs() < 1e-6,
        "finite source trim area: {area} vs {expected_area}"
    );
    let seam = source.shell().faces[0].boundaries[0]
        .iter()
        .find(|edge| {
            source.shell().faces[0].boundaries[0]
                .iter()
                .filter(|other| other.index == edge.index)
                .count()
                == 2
        })
        .unwrap()
        .index;
    let used_positions = mesh
        .faces()
        .triangle_iter()
        .flat_map(|triangle| triangle.map(|vertex| vertex.pos))
        .collect::<std::collections::BTreeSet<_>>();
    let samples = &result.mesh.edges[seam].curve;
    for point in samples.iter().skip(1).take(samples.len() - 2) {
        assert!(
            used_positions
                .iter()
                .filter(|index| mesh.positions()[**index] == *point)
                .count()
                >= 2,
            "both pinned seam uses retain each original shared XYZ sample in actual triangles"
        );
    }
    use monstertruck_modeling::ParametricSurface;
    let surface = &source.shell().faces[0].surface;
    use monstertruck_modeling::ParametricSurface3D;
    assert!(
        !surface.is_affine_plane(),
        "a curved source must retain its actual divisions"
    );
    // Independent interior parameter fractions of the source's full rectangular trim.
    let witnesses = [0.123, 0.287, 0.431, 0.619, 0.813]
        .into_iter()
        .flat_map(|u| {
            [0.13, 0.37, 0.61, 0.87]
                .map(|v| surface.evaluate(u * 6.28318530718, v * 6.284428582167))
        })
        .collect::<Vec<_>>();
    assert!(
        mesh.neighborhood_include(&witnesses, 0.0005),
        "a full UV trim must retain the actual curved source interior, not only its seam boundary fan"
    );
    let zero_edges = mesh
        .faces()
        .triangle_iter()
        .flat_map(|t| {
            let p = t.map(|v| mesh.positions()[v.pos]);
            [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])]
        })
        .filter(|(a, b)| a == b)
        .count();
    assert_eq!(
        zero_edges, 0,
        "the represented seam must be separated by curved interior samples, not discarded triangles"
    );
}

#[test]
fn certified_choc_representatives_do_not_bypass_changed_or_nonclosed_sources() {
    let text = include_str!("../../tests/fixtures/choc-source-certified-seam.step");
    let mut changed = Table::from_step(text).unwrap();
    changed.cartesian_point.get_mut(&27638).unwrap().coordinates[0] += 0.01;
    let nonclosed = text.replace(
        ")),.UNSPECIFIED.,.T.,.F.,.F.) B_SPLINE_SURFACE_WITH_KNOTS",
        ")),.UNSPECIFIED.,.F.,.F.,.F.) B_SPLINE_SURFACE_WITH_KNOTS",
    );
    assert_ne!(text, nonclosed);
    for table in [changed, Table::from_step(&nonclosed).unwrap()] {
        let source = table
            .to_source_bound_shell_checked(&table.shell[&50000])
            .unwrap();
        assert_eq!(source.certified_closed_axes(), &[Some([false, false])]);
        let result = triangulate_source_bound_shell(
            &source,
            1.0,
            TessellationOptions {
                tolerance: 0.0005,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            result.certified_faces.is_empty(),
            "no source certificate means no endpoint completion"
        );
    }
}

#[test]
fn certified_choc_conflicting_adjacent_seam_pins_still_refuse() {
    let text = include_str!("../../tests/fixtures/choc-source-certified-seam.step");
    let conflicting = text.replace(
        "EDGE_LOOP('',(#27346,#27391,#27401,#27402))",
        "EDGE_LOOP('',(#27346,#27401))",
    );
    assert_ne!(text, conflicting);
    let table = Table::from_step(&conflicting).unwrap();
    let source = table
        .to_source_bound_shell_checked(&table.shell[&50000])
        .unwrap();
    assert_eq!(source.certified_closed_axes(), &[Some([true, false])]);
    let result = triangulate_source_bound_shell(
        &source,
        1.0,
        TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.certified_faces, vec![27343]);
    assert!(
        result.mesh.faces[0].surface.is_none(),
        "opposite chart pins at one join cannot be reconciled by endpoint completion"
    );
}

#[test]
fn certified_endpoint_completion_refuses_a_source_seam_away_from_chart_boundaries() {
    // Move only the topological seam by .01 radians; keep the certified net unchanged.
    // The shift lies between adjacent circle seeds, so ring checks cannot mask a bad lift.
    let moved = MM
        .replace("#43=DIRECTION('',(1.,0.,0.));", "#43=DIRECTION('',(0.9999500004166653,0.009999833334166664,0.));")
        .replace("#48=VERTEX_POINT('',#10);", "#48=VERTEX_POINT('',#900);")
        .replace("#49=VERTEX_POINT('',#19);", "#49=VERTEX_POINT('',#901);")
        .replace("#51=LINE('',#10,#50);", "#51=LINE('',#900,#50);")
        .replace("ENDSEC;END-ISO-10303-21;", "#900=CARTESIAN_POINT('',(0.9999500004166653,0.009999833334166664,0.));#901=CARTESIAN_POINT('',(0.9999500004166653,0.009999833334166664,1.));ENDSEC;END-ISO-10303-21;");
    let table = Table::from_step(&moved).unwrap();
    let source = table
        .to_source_bound_solid_checked(&table.manifold_solid_brep[&73])
        .unwrap();
    assert_eq!(
        source.shells()[0].certified_closed_axes()[0],
        Some([false, true])
    );
    let result = triangulate_source_bound_shell(
        &source.shells()[0],
        1.0,
        TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.certified_faces, vec![63]);
    assert!(
        result.mesh.faces[0].surface.is_none(),
        "an interior seam cannot be moved onto finite chart endpoints with a different XYZ"
    );
}

#[test]
fn explicit_plane_keeps_boundary_only_tessellation_at_tight_tolerance() {
    use monstertruck_modeling::{Point3, Shell, Vertex, Wire, builder};
    let vertices = Vertex::from_points(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    ]);
    let wire: Wire = (0..4)
        .map(|i| builder::line(&vertices[i], &vertices[(i + 1) % 4]))
        .collect();
    let face = builder::try_attach_plane(vec![wire]).unwrap();
    for reversed in [false, true] {
        let shell: Shell = vec![if reversed {
            face.inverse()
        } else {
            face.clone()
        }]
        .into();
        let meshed = shell.triangulation(0.0005);
        let mesh = meshed[0].surface().unwrap();
        assert_eq!(
            mesh.faces().triangle_iter().count(),
            2,
            "an explicit affine plane needs only its boundary, irrespective of tight spatial tolerance"
        );
        let area: f64 = mesh
            .faces()
            .triangle_iter()
            .map(|t| {
                let p = t.map(|v| mesh.positions()[v.pos]);
                (p[1] - p[0]).cross(p[2] - p[0]).z / 2.0
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-12);
        assert!(mesh.positions().iter().all(|p| p.z == 0.0));
    }
}

#[test]
fn affine_plane_capability_preserves_wrappers_and_refuses_unproved_carriers() {
    use monstertruck_modeling::{
        BsplineSurface, Invertible, KnotVector, Matrix4, ParametricSurface3D, Plane, Point3,
        Processor, Surface, Transformed, Vector3,
    };
    let plane = Plane::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    assert!(plane.is_affine_plane());
    assert!(<&Plane as ParametricSurface3D>::is_affine_plane(&&plane));
    assert!(<Box<Plane> as ParametricSurface3D>::is_affine_plane(
        &Box::new(plane)
    ));
    assert!(Surface::from(plane).is_affine_plane());
    assert!(
        monstertruck_io::step::load::step_geometry::Surface::from(
            monstertruck_io::step::load::step_geometry::ElementarySurface::Plane(plane)
        )
        .is_affine_plane()
    );
    let mut processed: Processor<Plane, Matrix4> = Processor::new(plane);
    processed.transform_by(Matrix4::from_translation(Vector3::new(7.0, -3.0, 2.0)));
    processed.transform_by(Matrix4::from_nonuniform_scale(-2.0, 3.0, 4.0));
    assert!(processed.is_affine_plane());
    processed.invert();
    assert!(processed.is_affine_plane());
    let mut projective = Matrix4::from_scale(1.0);
    projective.x.w = 1e-15;
    let mut projected: Processor<Plane, Matrix4> = Processor::new(plane);
    projected.transform_by(projective);
    assert!(!projected.is_affine_plane());
    for matrix in [
        Matrix4::from_scale(0.0),
        Matrix4::from_scale(f64::INFINITY),
        Matrix4::from_scale(f64::NAN),
        Matrix4::from_scale(1e200),
    ] {
        let mut unsupported: Processor<Plane, Matrix4> = Processor::new(plane);
        unsupported.transform_by(matrix);
        assert!(!unsupported.is_affine_plane());
    }
    let large_plane = Plane::new(
        Point3::new(1e100, 0.0, 0.0),
        Point3::new(2e100, 0.0, 0.0),
        Point3::new(1e100, 1.0, 0.0),
    );
    assert!(large_plane.is_affine_plane());
    let overflowed = Processor::with_transform(
        large_plane,
        Matrix4::from_nonuniform_scale(1e250, 1e-250, 1.0),
    );
    assert!(
        !overflowed.is_affine_plane(),
        "finite affine transform coefficients must not transfer an overflowing plane certificate"
    );
    for invalid in [
        Plane::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ),
        Plane::new(
            Point3::new(f64::NAN, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ),
    ] {
        assert!(!invalid.is_affine_plane());
    }
    // Even an actually planar generic net is unproved by the optional capability.
    let net = BsplineSurface::new(
        (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
        vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)],
        ],
    );
    assert!(!net.is_affine_plane());
    assert!(!Surface::from(net).is_affine_plane());
}
