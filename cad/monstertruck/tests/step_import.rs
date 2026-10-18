use boardstudio_cad::{
    StepImportOptions, import_step, import_step_with_options, mesh_imported_step,
};

const TETRA: &[u8] = include_bytes!("fixtures/tetra-mm.step");

fn assert_bounds(actual: [[f64; 3]; 2], expected: [[f64; 3]; 2]) {
    for (actual, expected) in actual
        .into_iter()
        .flatten()
        .zip(expected.into_iter().flatten())
    {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }
}

#[test]
fn independent_tetrahedron_retains_exact_geometry_original_bytes_and_mm_mesh() {
    let bytes = TETRA.to_vec();
    let imported = import_step(bytes.clone()).expect("complete tetrahedron imports");
    drop(bytes);
    assert_eq!(imported.original_bytes(), TETRA);
    assert_eq!(imported.solid_count(), 1);
    assert_eq!(imported.occurrence_count(), 1);
    assert_bounds(imported.bounds(), [[0., 0., 0.], [2., 3., 4.]]);
    let mesh = mesh_imported_step(&imported).unwrap();
    assert!(!mesh.positions.is_empty());
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert_eq!(mesh.positions.len() % 9, 0);
    assert!(
        mesh.positions
            .iter()
            .chain(&mesh.normals)
            .all(|v| v.is_finite())
    );
}

#[test]
fn absent_units_mean_mm_and_metre_coordinates_convert_once() {
    let text = String::from_utf8(TETRA.to_vec()).unwrap();
    let absent = text
        .replace(
            "#102=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));",
            "",
        )
        .replace("GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))", "");
    assert_bounds(
        import_step(absent.into_bytes()).unwrap().bounds(),
        [[0., 0., 0.], [2., 3., 4.]],
    );
    let metre = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    assert_bounds(
        import_step(metre.into_bytes()).unwrap().bounds(),
        [[0., 0., 0.], [2000., 3000., 4000.]],
    );
}

#[test]
fn invalid_size_and_malformed_sources_are_errors() {
    for bytes in [Vec::new(), vec![b' '; 32 * 1024 * 1024 + 1]] {
        assert_eq!(
            import_step(bytes).err().unwrap(),
            "STEP import failed: invalid file size"
        );
    }
    assert!(import_step(b"not STEP".to_vec()).is_err());
}

#[test]
fn dropped_face_or_unreferenced_solid_cannot_succeed_with_partial_mesh() {
    let text = String::from_utf8(TETRA.to_vec()).unwrap();
    let missing_face = text.replace(
        "#100=CLOSED_SHELL('',(#59,#69,#79,#89));",
        "#100=CLOSED_SHELL('',(#69,#79,#89));",
    );
    let error = import_step(missing_face.into_bytes()).err().unwrap();
    assert!(
        error.contains("incomplete or nonmanifold source shell edge uses"),
        "wrong refusal: {error}"
    );
    for text in [
        text.replace(
            "#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
            "#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(),#103);",
        ),
        text.replace(
            "#89=ADVANCED_FACE('',(#84),#88,.T.);",
            "#89=ADVANCED_FACE('',(#84),#999,.T.);",
        ),
        text.replace(
            "ENDSEC;END-ISO-10303-21;",
            "#999=MANIFOLD_SOLID_BREP('',#100);ENDSEC;END-ISO-10303-21;",
        ),
    ] {
        assert!(import_step(text.into_bytes()).is_err());
    }
}

const ASSEMBLY: &[u8] = include_bytes!("fixtures/two-occurrences-mm.step");

#[test]
fn shared_exact_solid_keeps_both_rotated_and_translated_occurrences() {
    let imported = import_step(ASSEMBLY.to_vec()).unwrap();
    assert_eq!(imported.solid_count(), 1);
    assert_eq!(imported.occurrence_count(), 2);
    // First: (x,y,z) -> (10-y,20+x,30+z); second: (x-5,y+7,z+11).
    assert_bounds(imported.bounds(), [[-5., 7., 11.], [10., 22., 34.]]);
}

#[test]
fn incomplete_occurrences_and_invalid_placements_refuse() {
    let text = String::from_utf8(ASSEMBLY.to_vec()).unwrap();
    for text in [
        text.replace("#251=PRODUCT_DEFINITION_SHAPE('','',#250);", ""),
        text.replace(
            "#252=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#253,#251);",
            "",
        ),
        text.replace(
            "#250=NEXT_ASSEMBLY_USAGE_OCCURRENCE('first','rotated tetra','',#230,#240,$);",
            "#250=NEXT_ASSEMBLY_USAGE_OCCURRENCE('first','rotated tetra','',#230,#999,$);",
        ),
        text.replace(
            "#205=DIRECTION('',(0.,1.,0.));",
            "#205=DIRECTION('',(0.,0.,0.));",
        ),
    ] {
        assert!(import_step(text.into_bytes()).is_err());
    }
}

#[test]
fn latin1_metadata_keeps_original_bytes_alive() {
    let mut bytes = TETRA.to_vec();
    let at = bytes
        .windows(b"independent".len())
        .position(|s| s == b"independent")
        .unwrap();
    bytes[at] = 0xe9;
    let imported = import_step(bytes.clone()).unwrap();
    assert_eq!(imported.original_bytes(), bytes);
    assert_bounds(imported.bounds(), [[0., 0., 0.], [2., 3., 4.]]);
}

#[test]
fn authenticated_inch_and_mm_closed_surfaces_keep_the_same_geometry() {
    for bytes in [
        include_bytes!("fixtures/source-closed-cylinder-mm.step").as_slice(),
        include_bytes!("fixtures/source-closed-cylinder-inch.step").as_slice(),
    ] {
        let imported = import_step(bytes.to_vec()).unwrap();
        assert_bounds(imported.bounds(), [[-1., -1., 0.], [1., 1., 1.]]);
        assert_eq!(imported.solid_count(), 1);
        assert_eq!(imported.original_bytes(), bytes);
    }
}

#[test]
fn mixed_child_metre_parent_mm_scales_geometry_without_rescaling_parent_translation() {
    let text = String::from_utf8(ASSEMBLY.to_vec()).unwrap();
    let text = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)")
        .replace("#220=SHAPE_REPRESENTATION('',(#200,#201,#202),#103);", "#301=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#303=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#301)));#220=SHAPE_REPRESENTATION('',(#200,#201,#202),#303);");
    let imported = import_step(text.into_bytes()).unwrap();
    assert_eq!(imported.occurrence_count(), 2);
    assert_bounds(
        imported.bounds(),
        [[-2990., 7., 11.], [1995., 3007., 4030.]],
    );
}

#[test]
fn curved_source_honors_independent_deflection_and_angular_limits() {
    let source =
        std::str::from_utf8(include_bytes!("fixtures/source-closed-cylinder-mm.step")).unwrap();
    // The small radius makes angular refinement necessary; the large radius
    // makes spatial refinement necessary. Both are exact source circles.
    for radius in [0.1, 10.] {
        let unit = format!(
            "#81=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#82=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE({radius}),#81);#80=(CONVERSION_BASED_UNIT('scaled millimetre',#82)LENGTH_UNIT()NAMED_UNIT(*));"
        );
        let source = source.replace(
            "#80=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));",
            &unit,
        );
        let model = import_step(source.into_bytes()).unwrap();
        let mut side_boundary_chords = 0;
        for triangle in model.mesh().positions.as_chunks::<9>().0 {
            let points = triangle.as_chunks::<3>().0;
            if points.iter().all(|p| (p[2] - points[0][2]).abs() < 1e-8) {
                continue;
            }
            for i in 0..3 {
                let (a, b) = (points[i], points[(i + 1) % 3]);
                if (a[2] - b[2]).abs() > 1e-8 {
                    continue;
                }
                let angle = (f64::from(a[0]) * f64::from(b[1]) - f64::from(a[1]) * f64::from(b[0]))
                    .atan2(f64::from(a[0]) * f64::from(b[0]) + f64::from(a[1]) * f64::from(b[1]))
                    .abs();
                assert!(
                    angle <= 0.5 + 1e-6,
                    "topology-edge chord angle {angle} exceeds 0.5 radians at radius {radius}"
                );
                let chord_midpoint_radius = ((f64::from(a[0]) + f64::from(b[0])).powi(2)
                    + (f64::from(a[1]) + f64::from(b[1])).powi(2))
                .sqrt()
                    / 2.;
                assert!(
                    (radius - chord_midpoint_radius).abs() <= 0.1 + 1e-6,
                    "exact circle midpoint deflection exceeds 0.1mm at radius {radius}"
                );
                side_boundary_chords += 1;
            }
        }
        assert!(side_boundary_chords > 0);
    }
}

#[test]
fn curved_geometry_bounds_are_independent_of_the_product_mesh_grid() {
    let imported =
        import_step(include_bytes!("fixtures/rotated-cylinder-10mm.step").to_vec()).unwrap();
    // Radius 10 cylinder with its parameter frame rotated pi/32 radians around Z:
    // rotation preserves the exact circle, including extrema between mesh samples.
    for (actual, expected) in imported
        .bounds()
        .into_iter()
        .flatten()
        .zip([-10., -10., 0., 10., 10., 10.])
    {
        assert!(
            (actual - expected).abs() <= 0.03,
            "geometry bound {actual} != independently known {expected} (0.03mm tolerance)"
        );
    }
}

#[test]
fn oriented_volume_classification_is_stable_at_a_large_finite_placement() {
    let source = std::str::from_utf8(ASSEMBLY).unwrap().replace(
        "#206=CARTESIAN_POINT('',(10.,20.,30.));",
        "#206=CARTESIAN_POINT('',(1000000000.,1000000000.,1000000000.));",
    );
    // Exact volume remains 4 under translation; the later f32 mesh projection
    // cannot represent this small body at 1e9 mm and must refuse specifically.
    let error = import_step(source.into_bytes())
        .err()
        .expect("unrepresentable mesh must refuse");
    assert!(
        error.contains("f32 render precision"),
        "wrong refusal: {error}"
    );
}

#[test]
fn moderate_fractional_translation_preserves_positive_volume_and_a_representable_mesh() {
    let source = std::str::from_utf8(ASSEMBLY).unwrap().replace(
        "#206=CARTESIAN_POINT('',(10.,20.,30.));",
        "#206=CARTESIAN_POINT('',(1000000.1,1000000.2,1000000.3));",
    );
    // The same volume 4 tetrahedron remains representable in f32 here. The
    // unconditioned world-origin triple product evaluates to a negative volume.
    let model = import_step(source.into_bytes()).unwrap();
    assert!(model.accuracy().mesh_closed_oriented);
    assert_eq!(model.occurrence_count(), 2);
    assert_bounds(
        model.bounds(),
        [[-5., 7., 11.], [1000000.1, 1000002.2, 1000004.3]],
    );
    assert!(
        model
            .mesh()
            .positions
            .as_chunks::<9>()
            .0
            .iter()
            .all(|tri| tri[0..3] != tri[3..6] && tri[0..3] != tri[6..9] && tri[3..6] != tri[6..9])
    );
}

#[test]
fn trimmed_cylinder_sector_excludes_the_full_circle_extrema() {
    let model =
        import_step(include_bytes!("fixtures/trimmed-cylinder-sector.step").to_vec()).unwrap();
    assert_bounds(
        model.bounds(),
        [[0., 0., 0.], [75f64.sqrt(), 75f64.sqrt(), 2.]],
    );
}

#[test]
fn complete_vendor_sphere_can_display_without_exact_interior_bounds() {
    let model = import_step(include_bytes!("fixtures/unsupported-non-ruled-sphere.step").to_vec())
        .expect("complete vendor geometry can preview without an exact tight BRep bound");
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert!(!model.mesh().positions.is_empty());
    assert!(!model.accuracy().bounds_from_source_geometry);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn non_ruled_source_patch_requires_closed_outer_and_source_witness_gaps() {
    let model =
        import_step(include_bytes!("fixtures/tetra-with-degree2-source-face.step").to_vec())
            .unwrap();
    assert_bounds(model.bounds(), [[0., 0., 0.], [2., 3., 4.]]);
}

#[test]
fn source_surface_curves_keep_native_3d_bounds_and_associated_pcurves() {
    let bytes = include_bytes!("fixtures/source-pcurve-cube.step");
    let model = import_step(bytes.to_vec()).unwrap();
    assert_bounds(model.bounds(), [[0., 0., 0.], [10., 10., 10.]]);
    assert_eq!(model.original_bytes(), bytes);
}

#[test]
fn mapped_representation_retains_both_source_occurrences_and_placement() {
    let model = import_step(include_bytes!("fixtures/mapped-occurrence.step").to_vec()).unwrap();
    assert_eq!(model.occurrence_count(), 2);
    assert_bounds(model.bounds(), [[0., 0., 0.], [102., 3., 4.]]);
    assert_eq!(model.mesh().positions.len(), 8 * 9);
}

#[test]
fn mapped_origin_rotation_and_uniform_scale_compose_once() {
    let source=std::str::from_utf8(include_bytes!("fixtures/mapped-occurrence.step")).unwrap()
        .replace("#104=SHAPE_REPRESENTATION('',(#101,#205),#103);","#104=SHAPE_REPRESENTATION('',(#205),#103);")
        .replace("#200=AXIS2_PLACEMENT_3D('',#1,$,$);","#206=CARTESIAN_POINT('',(1.,2.,3.));#207=DIRECTION('',(0.,1.,0.));#200=AXIS2_PLACEMENT_3D('',#206,$,$);")
        .replace("#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#201,1.,$);","#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',#207,$,#201,2.,$);");
    let model = import_step(source.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 1);
    assert_bounds(model.bounds(), [[98., -2., -6.], [104., 2., 2.]]);
}

#[test]
fn nested_mapping_retains_every_occurrence_and_composes_translation() {
    let source=std::str::from_utf8(include_bytes!("fixtures/mapped-occurrence.step")).unwrap()
        .replace("#105=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);","#105=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101,#210),#103);#210=MAPPED_ITEM('',#211,#203);#211=REPRESENTATION_MAP(#200,#212);#212=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);");
    let model = import_step(source.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 3);
    assert_bounds(model.bounds(), [[0., 0., 0.], [202., 3., 4.]]);
}

#[test]
fn mapped_cycles_dangling_references_and_unknown_items_refuse() {
    let source = std::str::from_utf8(include_bytes!("fixtures/mapped-occurrence.step")).unwrap();
    for source in [
        source.replace(
            "REPRESENTATION_MAP(#200,#105)",
            "REPRESENTATION_MAP(#200,#104)",
        ),
        source.replace("MAPPED_ITEM('',#204,#203)", "MAPPED_ITEM('',#204,#999)"),
        source.replace("MAPPED_ITEM('',#204,#203)", "MAPPED_ITEM(1,#204,#203)"),
        source.replace("(#101,#205)", "(#101,#201)"),
        source.replace("(#101,#205)", "(#101,#205,#205)"),
        source.replace("#201,1.,$)", "#201,0.,$)"),
    ] {
        assert!(import_step(source.into_bytes()).is_err());
    }
}

#[test]
fn mapped_mirroring_preserves_a_positive_solid_and_all_occurrences() {
    let source=std::str::from_utf8(include_bytes!("fixtures/mapped-occurrence.step")).unwrap()
        .replace("#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#201,1.,$);","#207=DIRECTION('',(0.,-1.,0.));#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#207,#201,1.,$);");
    let model = import_step(source.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 2);
    assert_bounds(model.bounds(), [[0., -3., 0.], [102., 3., 4.]]);
}

#[test]
fn assembly_occurrence_walk_includes_nested_mapped_representation_items() {
    let source=std::str::from_utf8(ASSEMBLY).unwrap()
        .replace("#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);","#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101,#315),#103);#300=AXIS2_PLACEMENT_3D('',#1,$,$);#310=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);#311=CARTESIAN_POINT('',(100.,0.,0.));#313=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#311,1.,$);#314=REPRESENTATION_MAP(#300,#310);#315=MAPPED_ITEM('',#314,#313);");
    let model = import_step(source.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 4);
    assert_bounds(model.bounds(), [[-5., 7., 11.], [97., 122., 34.]]);
}

#[test]
fn a_source_boundary_off_its_plane_cannot_be_accepted_as_the_same_tetrahedron() {
    let source = std::str::from_utf8(TETRA).unwrap();
    for z in [1., 0.001] {
        let source = source.replace(
            "#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);",
            &format!(
                "#300=CARTESIAN_POINT('',(0.,0.,{z}));#57=AXIS2_PLACEMENT_3D('',#300,#55,#56);"
            ),
        );
        let error = import_step(source.into_bytes())
            .err()
            .expect("off-support source boundary must refuse");
        assert!(
            error.contains("source boundary") || error.contains("support incidence"),
            "wrong refusal: {error}"
        );
    }
}

#[test]
fn mapped_uniform_scale_keeps_the_absolute_world_mesh_deflection() {
    let source=std::str::from_utf8(include_bytes!("fixtures/source-closed-cylinder-mm.step")).unwrap()
        .replace("ENDSEC;END-ISO-10303-21;","#300=AXIS2_PLACEMENT_3D('',#40,$,$);#303=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#40,20.,$);#304=REPRESENTATION_MAP(#300,#84);#305=MAPPED_ITEM('',#304,#303);#306=SHAPE_REPRESENTATION('',(#305),#83);ENDSEC;END-ISO-10303-21;");
    let model = import_step(source.into_bytes()).unwrap();
    assert_bounds(model.bounds(), [[-20., -20., 0.], [20., 20., 20.]]);
    let mut checked = 0;
    for triangle in model.mesh().positions.as_chunks::<9>().0 {
        let points = triangle.as_chunks::<3>().0;
        if points.iter().all(|p| (p[2] - points[0][2]).abs() < 1e-6) {
            continue;
        }
        for i in 0..3 {
            let (a, b) = (points[i], points[(i + 1) % 3]);
            if (a[2] - b[2]).abs() > 1e-6 {
                continue;
            }
            let midpoint_radius = ((f64::from(a[0]) + f64::from(b[0])).powi(2)
                + (f64::from(a[1]) + f64::from(b[1])).powi(2))
            .sqrt()
                / 2.;
            assert!(
                (20. - midpoint_radius).abs() <= 0.1 + 1e-5,
                "scaled source chord deflection exceeds 0.1mm"
            );
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn represented_mapping_outside_the_product_graph_cannot_disappear() {
    let source=std::str::from_utf8(ASSEMBLY).unwrap()
        .replace("ENDSEC;END-ISO-10303-21;","#300=AXIS2_PLACEMENT_3D('',#1,$,$);#310=SHAPE_REPRESENTATION('',(#315),#103);#311=CARTESIAN_POINT('',(100.,0.,0.));#313=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#311,1.,$);#314=REPRESENTATION_MAP(#300,#104);#315=MAPPED_ITEM('',#314,#313);ENDSEC;END-ISO-10303-21;");
    let error = import_step(source.into_bytes())
        .err()
        .expect("untraversed represented mapping must refuse");
    assert!(
        error.contains("unreferenced mapped occurrence"),
        "wrong refusal: {error}"
    );
}

#[test]
fn native_cylinder_source_support_is_checked_over_the_whole_boundary() {
    let text = include_str!("fixtures/source-closed-cylinder-mm.step");
    let line = text.lines().find(|line| line.starts_with("#30=")).unwrap();
    let native = text
        .replace(line, "#30=CYLINDRICAL_SURFACE('',#44,1);")
        .replace(
            "#63=ADVANCED_FACE('',(#62),#30,.F.);",
            "#63=ADVANCED_FACE('',(#62),#30,.T.);",
        );
    assert_bounds(
        import_step(native.as_bytes().to_vec())
            .expect("valid native cylinder")
            .bounds(),
        [[-1., -1., 0.], [1., 1., 1.]],
    );
    let invalid = native.replace(
        "#30=CYLINDRICAL_SURFACE('',#44,1);",
        "#30=CYLINDRICAL_SURFACE('',#44,1.001);",
    );
    let error = import_step(invalid.into_bytes())
        .err()
        .expect("off-support cylinder boundary must refuse");
    assert!(
        error.contains("support incidence"),
        "wrong refusal: {error}"
    );
}

#[test]
fn native_cone_support_keeps_valid_trim_and_rejects_a_radius_mismatch() {
    let text = include_str!("fixtures/source-closed-cylinder-mm.step");
    let line = text.lines().find(|line| line.starts_with("#30=")).unwrap();
    let cone = text
        .replace(line, "#30=CONICAL_SURFACE('',#44,1,0.7853981633974483);")
        .replace(
            "#63=ADVANCED_FACE('',(#62),#30,.F.);",
            "#63=ADVANCED_FACE('',(#62),#30,.T.);",
        )
        .replace(
            "#19=CARTESIAN_POINT('',(1,0,1));",
            "#19=CARTESIAN_POINT('',(2,0,1));",
        )
        .replace("#47=CIRCLE('',#45,1);", "#47=CIRCLE('',#45,2);")
        .replace(
            "#50=VECTOR('',#42,1);",
            "#74=DIRECTION('',(1,0,1));#50=VECTOR('',#74,1.4142135623730951);",
        );
    assert_bounds(
        import_step(cone.as_bytes().to_vec())
            .expect("valid native cone")
            .bounds(),
        [[-2., -2., 0.], [2., 2., 1.]],
    );
    let mismatch = cone.replace("CONICAL_SURFACE('',#44,1,", "CONICAL_SURFACE('',#44,1.001,");
    let error = import_step(mismatch.into_bytes())
        .err()
        .expect("off-support cone boundary must refuse");
    assert!(
        error.contains("support incidence"),
        "wrong refusal: {error}"
    );
}

#[test]
fn source_face_catalogue_is_original_shared_and_triangle_complete() {
    let model = import_step(include_bytes!("fixtures/mapped-occurrence.step").to_vec()).unwrap();
    assert_eq!(model.source_faces().len(), 4);
    assert_eq!(model.face_occurrences().len(), 8);
    assert_eq!(
        model.triangle_faces().len(),
        model.mesh().positions.len() / 9
    );
    assert_eq!(
        model.preview_positions_f64().len(),
        model.mesh().positions.len() / 3
    );
    for (index, tag) in model.triangle_faces().iter().enumerate() {
        let occurrence = &model.face_occurrences()[*tag];
        let source = &model.source_faces()[occurrence.source_face];
        assert_eq!(source.solid_id, 101);
        assert!([59, 69, 79, 89].contains(&source.face_id));
        assert!([58, 68, 78, 88].contains(&source.support_id));
        assert_eq!(source.support_kind, "PLANE");
        assert!(occurrence.occurrence < 2);
        assert_eq!(occurrence.source_uncertainty_mm, None);
        assert_eq!(occurrence.repair_displacement_mm, 0.);
        let p = model.preview_positions_f64()[index * 3];
        assert!(if occurrence.occurrence == 0 {
            p[0] <= 2.
        } else {
            p[0] >= 100.
        });
    }
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    assert_eq!(model.accuracy().preview_deflection_mm, 0.1);
}

fn tetra_with_precision(uncertainty: &str) -> String {
    std::str::from_utf8(TETRA).unwrap().replace("#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));", "#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));" )
        .replace("ENDSEC;END-ISO-10303-21;", &format!("{uncertainty}ENDSEC;END-ISO-10303-21;"))
}

#[test]
fn declared_precision_is_typed_context_owned_and_unit_converted() {
    let text = tetra_with_precision(
        "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.00001),#301,'distance_accuracy_value','test');#301=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));",
    );
    let model = import_step(text.into_bytes()).unwrap();
    let precision = model
        .precision()
        .iter()
        .find(|p| p.representation_id == 104)
        .unwrap();
    assert_eq!(precision.context_id, 103);
    assert_eq!(precision.uncertainty_mm(), Some(0.01));
    assert_eq!(precision.declarations[0].entity_id, 300);
    assert_eq!(precision.declarations[0].length_unit_id, 301);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm == Some(0.01))
    );
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    let unassigned=tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value','test');").replace("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300))","");
    let model = import_step(unassigned.into_bytes()).unwrap();
    assert!(
        model
            .precision()
            .iter()
            .all(|p| p.uncertainty_mm().is_none())
    );
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn malformed_or_misdirected_precision_cannot_authorize_repair() {
    for declaration in [
        "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(-0.01),#102,'distance_accuracy_value','test');",
        "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#999,'distance_accuracy_value','test');",
        "#300=UNCERTAINTY_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.01),#102,'distance_accuracy_value','test');",
    ] {
        assert!(import_step(tetra_with_precision(declaration).into_bytes()).is_err());
    }
}

#[test]
fn precision_with_optional_description_is_still_authenticated() {
    let model=import_step(tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value',$);").into_bytes()).unwrap();
    assert_eq!(model.precision()[0].uncertainty_mm(), Some(0.01));
}

#[test]
fn mapped_precision_is_scaled_in_world_mm_without_borrowing_another_context() {
    let text=include_str!("fixtures/mapped-occurrence.step")
        .replace("#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));","#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));")
        .replace("#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#201,1.,$);","#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#201,2.,$);")
        .replace("ENDSEC;END-ISO-10303-21;","#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
    let model = import_step(text.into_bytes()).unwrap();
    for face in model.face_occurrences() {
        assert_eq!(
            face.source_uncertainty_mm,
            Some(if face.occurrence == 0 { 0.001 } else { 0.002 })
        );
    }
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn multiple_or_unrelated_uncertainties_do_not_inflate_a_repair_allowance() {
    let text=tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);#301=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.5),#102,'other',$);")
        .replace("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300))","GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300,#301))");
    let model = import_step(text.into_bytes()).unwrap();
    assert_eq!(model.precision()[0].declarations.len(), 2);
    assert_eq!(model.precision()[0].uncertainty_mm(), None);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm.is_none())
    );
    let text=tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#102,'distance_accuracy_value',$);")
        .replace("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300))","GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300,#300))");
    assert!(import_step(text.into_bytes()).is_err());
}

#[test]
fn complex_uncertainty_keeps_its_measure_unit_and_optional_description() {
    let text = tetra_with_precision(
        "#300=(MEASURE_WITH_UNIT(LENGTH_MEASURE(0.002),#102) UNCERTAINTY_MEASURE_WITH_UNIT('distance_accuracy_value',$));",
    );
    let model = import_step(text.into_bytes()).unwrap();
    assert_eq!(model.precision()[0].uncertainty_mm(), Some(0.002));
}

#[test]
fn generic_context_cannot_borrow_an_untraversed_representation_precision() {
    let text = std::str::from_utf8(TETRA).unwrap().replace(
        "ENDSEC;END-ISO-10303-21;",
        "#400=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('other','') GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));#401=UNCERTAINTY_ASSIGNED_REPRESENTATION('',(#101),#400,(#403));#403=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;",
    );
    let model = import_step(text.clone().into_bytes())
        .expect("exact generic-context geometry needs no connectivity authority");
    assert_eq!(model.triangle_faces().len(), 4);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|face| face.source_uncertainty_mm.is_none())
    );
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    let outside = text.replace("#22=LINE('',#1,#21);", "#410=CARTESIAN_POINT('',(1.999,0.,0.));#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#410),.UNSPECIFIED.,.F.,.F.,(2,2),(0.,1.),.UNSPECIFIED.);");
    let error = import_step(outside.into_bytes()).err().expect(
        "generic context must not borrow the unrelated0.01 allowance to repair a source endpoint",
    );
    assert!(error.contains("B-spline"), "{error}");
}

#[test]
fn owned_plane_support_allowance_keeps_original_source_and_unqualified_preview() {
    let text = tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value',$);")
        .replace("#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);", "#310=CARTESIAN_POINT('',(0.,0.,0.001));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);");
    let model = import_step(text.clone().into_bytes())
        .expect("complete preview within authenticated source allowance");
    assert_eq!(model.original_bytes(), text.as_bytes());
    assert_eq!(model.source_faces().len(), 4);
    assert_eq!(model.face_occurrences().len(), 4);
    assert_eq!(model.triangle_faces().len(), 4);
    assert!(!model.accuracy().bounds_from_source_geometry);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm == Some(0.01))
    );
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    let unknown = text.replace("GEOMETRIC_REPRESENTATION_CONTEXT(3) ", "");
    let error = import_step(unknown.into_bytes())
        .err()
        .expect("untyped precision cannot authorize support discrepancy");
    assert!(error.contains("support incidence"), "{error}");
}

#[test]
fn endpoint_support_attachments_do_not_authorize_a_bowed_curve_interior() {
    let text = tetra_with_precision("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value',$);")
        .replace("#22=LINE('',#1,#21);", "#320=CARTESIAN_POINT('',(1.,0.,0.1));#22=B_SPLINE_CURVE_WITH_KNOTS('',2,(#1,#320,#2),.UNSPECIFIED.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.);");
    let error = import_step(text.into_bytes())
        .err()
        .expect("whole selected curve exceeds its owned support allowance");
    assert!(error.contains("support incidence"), "{error}");
}

#[test]
fn owned_support_allowance_uses_actual_unit_and_occurrence_scale() {
    let declared = "#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#102,'distance_accuracy_value',$);";
    let source = tetra_with_precision(declared).replace(
        "#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);",
        "#310=CARTESIAN_POINT('',(0.,0.,0.009));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);",
    );
    let mapped = source.replace(
        "#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
        "#104=SHAPE_REPRESENTATION('',(#101,#405),#103);#400=AXIS2_PLACEMENT_3D('',#1,$,$);#401=CARTESIAN_POINT('',(100.,0.,0.));#402=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#401,20.,$);#403=REPRESENTATION_MAP(#400,#404);#404=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);#405=MAPPED_ITEM('',#403,#402);",
    );
    let model = import_step(mapped.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 2);
    assert_eq!(model.triangle_faces().len(), 8);
    for face in model.face_occurrences() {
        assert_eq!(
            face.source_uncertainty_mm,
            Some(if face.occurrence == 0 { 0.01 } else { 0.2 })
        );
    }
    let metres = source
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)")
        .replace("LENGTH_MEASURE(0.01)", "LENGTH_MEASURE(0.00001)")
        .replace("(0.,0.,0.009)", "(0.,0.,0.000001)");
    let model = import_step(metres.into_bytes()).unwrap();
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm == Some(0.01))
    );
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn source_face_boundary_must_retain_its_authored_oriented_closed_order() {
    for (old, new) in [
        (
            "#53=EDGE_LOOP('',(#50,#51,#52));",
            "#53=EDGE_LOOP('',(#52,#51,#50));",
        ),
        (
            "#51=ORIENTED_EDGE('',*,*,#35,.F.);",
            "#51=ORIENTED_EDGE('',*,*,#35,.T.);",
        ),
    ] {
        let source = std::str::from_utf8(TETRA).unwrap().replace(old, new);
        let error = import_step(source.into_bytes()).err().expect(
            "an unordered or open authored boundary cannot become an ordinary tetra preview",
        );
        assert!(
            error.contains("source face #59") && error.contains("boundary"),
            "{error}"
        );
    }
}

#[test]
fn finite_source_lines_share_the_asset_without_becoming_triangle_faces() {
    let bytes = include_bytes!("fixtures/tetra-and-finite-wires.step").to_vec();
    let model = import_step(bytes.clone())
        .expect("complete finite source lines accompany the solid carrier");
    assert_eq!(model.original_bytes(), bytes);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert_eq!(model.source_faces().len(), 4);
    assert_eq!(model.triangle_faces().len(), 4);
    assert_eq!(model.mesh().positions.len(), 36);
    assert_eq!(model.mesh().normals.len(), 36);
    assert_bounds(model.bounds(), [[0., 0., 0.], [6., 10., 4.]]);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    let error = mesh_imported_step(&model)
        .expect_err("triangle-only delivery cannot present the whole source asset");
    assert!(error.contains("line-capable presentation"), "{error}");
}

#[test]
fn source_curve_sets_never_drop_an_unsupported_or_invalid_member() {
    let source = include_str!("fixtures/tetra-and-finite-wires.step");
    for (old, new) in [
        ("(#414,#419)", "(#414,#999)"),
        ("(#414,#419)", "(#414,#415)"),
        ("(#414,#419)", "(#414,#414)"),
        (
            "#418=LINE('',#415,#417);",
            "#418=CARTESIAN_POINT('',(0.,0.,0.));",
        ),
        ("#417=VECTOR('',#416,0.75);", "#417=VECTOR('',#416,0.);"),
        (
            "#419=TRIMMED_CURVE('finite-three-quarter-vector',#418,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(1.)),.T.,.PARAMETER.);",
            "#419=LINE('',#415,#417);",
        ),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        let error = import_step(source.replace(old, new).into_bytes())
            .err()
            .expect("every authored curve-set member must be retained or refused");
        assert!(
            error.contains("source curve") || error.contains("missing entity"),
            "{error}"
        );
    }
}

#[test]
fn finite_line_occurrences_keep_the_native_trim_frame_through_units_and_mapping() {
    let source = include_str!("fixtures/tetra-and-finite-wires.step");
    let mapped = source.replace(
        "#431=SHAPE_REPRESENTATION('fixture',(#400),#103);",
        "#431=SHAPE_REPRESENTATION('fixture',(#400,#453),#103);#450=CARTESIAN_POINT('',(-100.,-100.,0.));#454=DIRECTION('',(-1.,0.,0.));#455=DIRECTION('',(0.,-1.,0.));#456=DIRECTION('',(0.,0.,1.));#451=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',#454,#455,#450,2.,#456);#452=REPRESENTATION_MAP(#400,#430);#453=MAPPED_ITEM('',#452,#451);",
    );
    let model = import_step(mapped.into_bytes()).unwrap();
    assert_bounds(model.bounds(), [[-112., -120., 0.], [6., 10., 4.]]);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert_eq!(model.triangle_faces().len(), 4);
    let lines = model.line_presentation();
    let segments: Vec<_> = lines.segments_mm().collect();
    assert_eq!(segments.len(), 4);
    assert_eq!(lines.segment_occurrences(), &[0, 1, 2, 3]);
    let mut identities = Vec::new();
    for occurrence in lines.occurrences() {
        let translation = occurrence.placement_f64[12];
        let expected = match (occurrence.trimmed_curve_id, translation) {
            (414, 0.) => [[5.5, 0., 0.], [6., 0., 0.]],
            (419, 0.) => [[0., 10., 0.], [0., 9.25, 0.]],
            (414, -100.) => [[-111., -100., 0.], [-112., -100., 0.]],
            (419, -100.) => [[-100., -120., 0.], [-100., -118.5, 0.]],
            other => panic!("unexpected mapped curve {other:?}"),
        };
        assert_eq!(segments[occurrence.line_occurrence_index], expected);
        if translation == -100. {
            assert_eq!(
                occurrence.placement_f64,
                [
                    -2., 0., 0., 0., 0., -2., 0., 0., 0., 0., 2., 0., -100., -100., 0., 1.
                ]
            );
        }
        identities.push((occurrence.trimmed_curve_id, translation as i32));
    }
    identities.sort_unstable();
    assert_eq!(identities, [(414, -100), (414, 0), (419, -100), (419, 0)]);
    let metres = source.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let model = import_step(metres.into_bytes()).unwrap();
    assert_bounds(model.bounds(), [[0., 0., 0.], [6000., 10000., 4000.]]);
    let ratios = source.replace(
        "#411=DIRECTION('',(1.,0.,0.));",
        "#411=DIRECTION('',(4.,0.,0.));",
    );
    assert_bounds(
        import_step(ratios.into_bytes()).unwrap().bounds(),
        [[0., 0., 0.], [6., 10., 4.]],
    );
}

#[test]
fn finite_source_lines_apply_units_before_render_coordinate_limits() {
    let mut source = include_str!("fixtures/tetra-and-finite-wires.step")
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT(.FEMTO.,.METRE.)");
    for (old, new) in [
        (
            "#2=CARTESIAN_POINT('',(2.,0.,0.));",
            "#2=CARTESIAN_POINT('',(2000000000000.,0.,0.));",
        ),
        (
            "#3=CARTESIAN_POINT('',(0.,3.,0.));",
            "#3=CARTESIAN_POINT('',(0.,3000000000000.,0.));",
        ),
        (
            "#4=CARTESIAN_POINT('',(0.,0.,4.));",
            "#4=CARTESIAN_POINT('',(0.,0.,4000000000000.));",
        ),
        (
            "#410=CARTESIAN_POINT('',(5.,0.,0.));",
            "#410=CARTESIAN_POINT('',(5000000000000.,0.,0.));",
        ),
        (
            "#415=CARTESIAN_POINT('',(0.,10.,0.));",
            "#415=CARTESIAN_POINT('',(0.,10000000000000.,0.));",
        ),
        (
            "#412=VECTOR('',#411,0.25);",
            "#412=VECTOR('',#411,250000000000.);",
        ),
        (
            "#417=VECTOR('',#416,0.75);",
            "#417=VECTOR('',#416,750000000000.);",
        ),
    ] {
        assert_eq!(source.matches(old).count(), 1);
        source = source.replace(old, new);
    }
    for (id, direction) in [(21, 20), (25, 24), (29, 28), (33, 32), (37, 36), (41, 40)] {
        let old = format!("#{id}=VECTOR('',#{direction},1.);");
        assert_eq!(source.matches(&old).count(), 1);
        source = source.replace(
            &old,
            &format!("#{id}=VECTOR('',#{direction},1000000000000.);"),
        );
    }
    let model = import_step(source.into_bytes())
        .expect("render limits apply to normalized world mm, not source units");
    assert_bounds(model.bounds(), [[0., 0., 0.], [6., 10., 4.]]);
}

#[test]
fn public_source_line_view_keeps_native_identity_units_and_world_endpoints() {
    let source = include_str!("fixtures/tetra-and-finite-wires.step");
    for (text, unit, first, second) in [
        (
            source.to_owned(),
            1.,
            [[5.5, 0., 0.], [6., 0., 0.]],
            [[0., 10., 0.], [0., 9.25, 0.]],
        ),
        (
            source.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)"),
            1000.,
            [[5500., 0., 0.], [6000., 0., 0.]],
            [[0., 10000., 0.], [0., 9250., 0.]],
        ),
    ] {
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        let lines = model.line_presentation();
        let segments: Vec<_> = lines.segments_mm().collect();
        let occurrences: Vec<_> = lines.occurrences().collect();
        assert_eq!(segments.len(), 2);
        assert_eq!(occurrences.len(), 2);
        assert_eq!(lines.render_positions().len(), 4);
        assert_eq!(lines.segment_occurrences(), &[0, 1]);
        for (index, occurrence) in occurrences.iter().enumerate() {
            assert_eq!(occurrence.line_occurrence_index, index);
            assert_eq!(occurrence.curve_set_id, 420);
            assert_eq!(occurrence.representation_id, 430);
            assert_eq!(occurrence.source_unit_to_mm, unit);
            assert_eq!(occurrence.declared_uncertainty_mm, None);
            assert_eq!(
                occurrence.placement_f64,
                [
                    1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.
                ]
            );
            match occurrence.trimmed_curve_id {
                414 => {
                    assert_eq!(occurrence.source_line_id, 413);
                    assert_eq!(segments[index], first);
                }
                419 => {
                    assert_eq!(occurrence.source_line_id, 418);
                    assert_eq!(segments[index], second);
                }
                other => panic!("unexpected source curve {other}"),
            }
            assert_eq!(
                &lines.render_positions()[index * 2..index * 2 + 2],
                &segments[index].map(|point| point.map(|coordinate| coordinate as f32))
            );
        }
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.triangle_faces().len(), 4);
        assert!(mesh_imported_step(&model).is_err());
    }

    let surface_only = import_step(TETRA.to_vec()).unwrap();
    let lines = surface_only.line_presentation();
    assert_eq!(lines.segments_mm().len(), 0);
    assert_eq!(lines.occurrences().len(), 0);
    assert!(lines.render_positions().is_empty());
    assert!(lines.segment_occurrences().is_empty());
    assert!(mesh_imported_step(&surface_only).is_ok());
}

#[test]
fn selected_pcurve_cube_units_keep_full_original_parameter_extent() {
    let original = include_str!("fixtures/source-pcurve-cube.step");
    for (factor, unit) in [
        (
            1.,
            "#346 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
        ),
        (
            25.4,
            "#346=(CONVERSION_BASED_UNIT('inch',#362)LENGTH_UNIT()NAMED_UNIT(*));#362=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#363);#363=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));",
        ),
        (
            1000.,
            "#346 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.) );",
        ),
    ] {
        let source = original.replace(
            "#346 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
            unit,
        );
        let model = import_step(source.clone().into_bytes())
            .expect("selected PCurve active extent survives unit normalization");
        assert_bounds(model.bounds(), [[0., 0., 0.], [10. * factor; 3]]);
        assert_eq!(model.original_bytes(), source.as_bytes());
        assert_eq!(model.source_faces().len(), 6);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
        assert!(model.accuracy().bounds_from_source_geometry);
    }
}

fn tetra_with_plane_gap(declared_mm: &str, gap_mm: &str) -> String {
    tetra_with_precision(&format!("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({declared_mm}),#102,'distance_accuracy_value',$);"))
        .replace("#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);", &format!("#310=CARTESIAN_POINT('',(0.,0.,{gap_mm}));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);"))
}

#[test]
fn attachment_gap_above_declared_precision_is_accepted_under_the_preview_floor_and_recorded() {
    let model = import_step(tetra_with_plane_gap("0.001", "0.00102").into_bytes())
        .expect("a sub-floor gap is preview geometry, not a refusal");
    assert_eq!(model.face_occurrences().len(), 4);
    assert_eq!(model.accuracy().preview_floor_mm, 0.02);
    assert!(model.accuracy().relaxed_beyond_declared_precision);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm == Some(0.001))
    );
}

#[test]
fn attachment_gap_clearly_above_the_preview_floor_still_refuses() {
    let error = import_step(tetra_with_plane_gap("0.001", "0.05").into_bytes())
        .err()
        .expect("a gap well above the floor is not preview-acceptable");
    assert!(!error.is_empty());
}

#[test]
fn gap_just_above_the_declared_precision_imports_under_twice_the_declaration() {
    let model = import_step(tetra_with_plane_gap("0.01", "0.0127").into_bytes())
        .expect("0.0127 mm is within max(2 x 0.01, 0.02) mm");
    assert_eq!(model.face_occurrences().len(), 4);
    assert_eq!(model.accuracy().preview_declared_multiplier, 2.0);
    assert!(model.accuracy().relaxed_beyond_declared_precision);
}

#[test]
fn gap_above_twice_the_declared_precision_and_the_floor_still_refuses() {
    let error = import_step(tetra_with_plane_gap("0.01", "0.025").into_bytes())
        .err()
        .expect("0.025 mm exceeds max(2 x 0.01, 0.02) mm");
    assert!(!error.is_empty());
}

#[test]
fn gap_between_the_old_floor_and_the_new_allowance_imports_under_a_tiny_declaration() {
    import_step(tetra_with_plane_gap("0.001", "0.015").into_bytes())
        .expect("0.015 mm is within the 0.02 mm floor");
}

#[test]
fn declared_precision_above_the_floor_is_not_marked_relaxed() {
    let model = import_step(tetra_with_plane_gap("0.02", "0.").into_bytes()).unwrap();
    assert_eq!(model.accuracy().preview_floor_mm, 0.02);
    assert!(!model.accuracy().relaxed_beyond_declared_precision);
}

/// Native cylinder whose seam edge is a SURFACE_CURVE mastered by a PCurve on the
/// cylinder. Exact certification does not support that master; the sampled preview
/// fallback must decide instead.
fn cylinder_with_pcurve_seam(radius: &str) -> String {
    let text = include_str!("fixtures/source-closed-cylinder-mm.step");
    let line = text.lines().find(|line| line.starts_with("#30=")).unwrap();
    text.replace(line, &format!("#30=CYLINDRICAL_SURFACE('',#44,{radius});"))
        .replace(
            "#63=ADVANCED_FACE('',(#62),#30,.F.);",
            "#63=ADVANCED_FACE('',(#62),#30,.T.);",
        )
        .replace(
            "#54=EDGE_CURVE('',#48,#49,#51,.T.);",
            "#54=EDGE_CURVE('',#48,#49,#90,.T.);",
        )
        .replace(
            "ENDSEC;END-ISO-10303-21;",
            "#90=SURFACE_CURVE('',#51,(#91),.PCURVE_S1.);#91=PCURVE('',#30,#92);\
             #92=DEFINITIONAL_REPRESENTATION('',(#93),#96);#93=LINE('',#94,#95);\
             #94=CARTESIAN_POINT('',(0.,0.));#95=VECTOR('',#97,1.);#97=DIRECTION('',(0.,1.));\
             #96=(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('2D SPACE',''));\
             ENDSEC;END-ISO-10303-21;",
        )
}

#[test]
fn boundary_with_unsupported_exact_certification_is_accepted_by_sampling_and_recorded() {
    let model = import_step(cylinder_with_pcurve_seam("1.").into_bytes())
        .expect("a valid boundary on its cylinder is preview geometry");
    let accuracy = model.accuracy();
    assert!(accuracy.sampled_support_checks >= 1);
    let residual = accuracy.max_sampled_support_residual_mm.unwrap();
    assert!(residual < 1e-9, "residual {residual}");
    assert_eq!(model.source_faces().len(), 3);
}

#[test]
fn boundary_off_its_cylinder_beyond_the_allowance_refuses_with_the_measured_residual() {
    let text = include_str!("fixtures/source-closed-cylinder-mm.step");
    let line = text.lines().find(|line| line.starts_with("#30=")).unwrap();
    let off = text
        .replace(line, "#30=CYLINDRICAL_SURFACE('',#44,1.001);")
        .replace(
            "#63=ADVANCED_FACE('',(#62),#30,.F.);",
            "#63=ADVANCED_FACE('',(#62),#30,.T.);",
        );
    let error = import_step(off.into_bytes())
        .err()
        .expect("sampling must not accept a boundary off its support");
    assert!(
        error.contains("support incidence"),
        "wrong refusal: {error}"
    );
    assert!(error.contains("sampled support residual 0.001"), "{error}");
}

#[test]
fn exactly_certified_geometry_records_no_sampled_checks() {
    let model = import_step(TETRA.to_vec()).unwrap();
    assert_eq!(model.accuracy().sampled_support_checks, 0);
    assert_eq!(model.accuracy().max_sampled_support_residual_mm, None);
    assert_eq!(
        model.accuracy().analysis_error_bound_mm,
        None,
        "exact support incidence does not certify mesh coverage or query numerics"
    );
}

#[test]
fn explicit_preview_repair_budget_preserves_source_and_keeps_movement_unknown() {
    let source = tetra_with_plane_gap("0.001", "0.025");
    assert!(
        import_step(source.clone().into_bytes()).is_err(),
        "0.025 mm exceeds the default max(2 x 0.001, 0.02) mm allowance"
    );
    assert!(
        import_step_with_options(
            source.clone().into_bytes(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.024),
                target_deflection_mm: None,
            },
        )
        .is_err(),
        "over-budget repair must refuse"
    );
    let model = import_step_with_options(
        source.clone().into_bytes(),
        StepImportOptions {
            preview_repair_budget_mm: Some(0.03),
            target_deflection_mm: None,
        },
    )
    .expect("0.025 mm gap imports under an explicit 0.03 mm preview repair budget");
    assert_eq!(model.original_bytes(), source.as_bytes());
    assert_eq!(model.precision()[0].uncertainty_mm(), Some(0.001));
    assert_eq!(model.accuracy().selected_repair_budget_mm, Some(0.03));
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert!(model.accuracy().relaxed_beyond_declared_precision);
    assert_eq!(
        model.accuracy().analysis_error_bound_mm,
        None,
        "an explicit preview repair allowance is never an analysis error certificate"
    );
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|f| f.source_uncertainty_mm == Some(0.001))
    );
}

#[test]
fn on_demand_deflection_refinement_does_not_certify_a_nominal_tolerance() {
    let cylinder = include_bytes!("fixtures/freecad-cylinder.step").to_vec();
    let default_import = import_step(cylinder.clone()).expect("default cylinder import");
    assert_eq!(default_import.accuracy().analysis_error_bound_mm, None);

    let refined_import = import_step_with_options(
        cylinder,
        StepImportOptions {
            preview_repair_budget_mm: None,
            target_deflection_mm: Some(0.004),
        },
    )
    .expect("refined cylinder import");
    assert_eq!(refined_import.accuracy().analysis_error_bound_mm, None);
    assert_eq!(refined_import.accuracy().preview_deflection_mm, 0.004);
    assert!(
        refined_import.mesh().positions.len() > default_import.mesh().positions.len(),
        "tighter deflection refines curved cylinder mesh"
    );
}

#[test]
fn import_options_keep_default_behavior_and_reject_invalid_requests() {
    let ordinary = import_step(TETRA.to_vec()).unwrap();
    let explicit_default =
        import_step_with_options(TETRA.to_vec(), StepImportOptions::default()).unwrap();
    assert_eq!(ordinary.accuracy(), explicit_default.accuracy());
    assert_eq!(ordinary.mesh(), explicit_default.mesh());
    assert_eq!(
        ordinary.face_occurrences(),
        explicit_default.face_occurrences()
    );
    for invalid in [0.0, -0.01, f64::NAN, f64::INFINITY] {
        for options in [
            StepImportOptions {
                preview_repair_budget_mm: Some(invalid),
                target_deflection_mm: None,
            },
            StepImportOptions {
                preview_repair_budget_mm: None,
                target_deflection_mm: Some(invalid),
            },
        ] {
            assert!(import_step_with_options(TETRA.to_vec(), options).is_err());
        }
    }
    let coarser = import_step_with_options(
        TETRA.to_vec(),
        StepImportOptions {
            target_deflection_mm: Some(1.0),
            ..StepImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(coarser.accuracy().preview_deflection_mm, 0.1);
}

/// Faces the ordinary mesher cannot chart (pole and seam bands, sub-tolerance slivers) are
/// meshed from their shared boundary samples instead of failing the import.
#[test]
#[ignore = "loads vendor corpus files by repo path"]
fn corpus_faces_without_a_uv_chart_are_meshed_from_shared_samples() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (path, minimum) in [
        (
            "ergogen/library/vendor/vik/3d_models/Ariamelon-Kiwano/3DP/Trackball Mount Low Profile.step",
            3,
        ),
        (
            "ergogen/library/vendor/vik/3d_models/george-norton-peacock/case/peacock-low-case-v5.step",
            15,
        ),
    ] {
        let bytes = std::fs::read(root.join(path)).expect("corpus file present");
        let imported = import_step(bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(
            imported.accuracy().fallback_meshed_faces >= minimum,
            "{path}"
        );
    }
}
