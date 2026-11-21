use super::*;

// These controls inspect the vendor conversion ledger directly. The production
// analytic importer has no vendor operations to census.
fn legacy_model(bytes: Vec<u8>) -> Result<ImportedStep, String> {
    let exchange = parser::parse(&units::decode_step_bytes(&bytes)).map_err(|e| e.to_string())?;
    let precision = units::precision::resolve(&exchange)?;
    let lengths = units::length_factors(&exchange)?;
    let items = units::representation_items(&exchange)?;
    drop(exchange);
    import_legacy(
        bytes,
        StepImportOptions::default(),
        TOLERANCE_MM,
        precision,
        lengths,
        items,
    )
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_line_authority_borrows_holder_graph_and_preserves_refusal() {
    let fixture = include_str!("../../../tests/fixtures/repair-finite-domain.step");
    let source = fixture.replace(
        "#23=EDGE_CURVE('',",
        &format!("#23=EDGE_CURVE('{}',", "x".repeat(131_072)),
    );
    let table = Table::from_step_bytes(source.as_bytes()).unwrap();
    let (original, largest) = crate::allocation_tests::largest_request(|| {
        OriginalGeometry::capture(&table, source.as_bytes())
    });
    assert_eq!(original.lines.len(), 6);
    assert!(original.lines[&23].input.is_some());
    assert_eq!(
        original.finite_lines[&23].projected_exceeds(1.0, Matrix4::identity(), 0.02),
        Ok(true)
    );
    assert!(largest < 16_384, "LINE authority allocated {largest} bytes");
}

#[test]
fn normalized_legacy_preview_keeps_default_explicit_and_scaled_settings() {
    let cylinder = include_str!("../../../tests/fixtures/source-closed-cylinder-mm.step");
    let surface = cylinder
        .lines()
        .find(|line| line.starts_with("#30="))
        .unwrap();
    let radians = cylinder
        .replace(surface, "#30=CONICAL_SURFACE('',#44,1,0.7853981633974483);")
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
    let degrees = radians
            .replace("0.7853981633974483", "45.")
            .replace("#83=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#80)));",
                "#90=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));\
                 #91=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#90);\
                 #92=(CONVERSION_BASED_UNIT('DEGREE',#91)NAMED_UNIT(#93)PLANE_ANGLE_UNIT());\
                 #93=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\
                 #94=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.000001),#80,'distance_accuracy_value','');\
                 #83=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#94))REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#80,#92)));");
    let original = StepRepairInput::parse(degrees.as_bytes().to_vec()).unwrap();
    let normalized = units::normalize_conical_exchange(original.exchange()).unwrap();
    assert_ne!(normalized.as_ref(), original.exchange());
    let mapped = |text: &str| {
        text.replace(
            "#84=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#73),#83);",
            "#84=SHAPE_REPRESENTATION('',(#73,#185),#83);\
             #181=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#73),#83);\
             #182=REPRESENTATION_MAP(#44,#181);\
             #184=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#40,2.,$);\
             #185=MAPPED_ITEM('',#182,#184);",
        )
    };
    for (text, reference) in [
        (&degrees, radians.clone()),
        (&mapped(&degrees), mapped(&radians)),
    ] {
        let reference = import_step(reference.into_bytes()).unwrap();
        for budget in [None, Some(0.02)] {
            let model = import_step_with_options(
                text.as_bytes().to_vec(),
                StepImportOptions {
                    preview_repair_budget_mm: budget,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(model.original_bytes(), text.as_bytes());
            assert_eq!(model.bounds(), reference.bounds());
            assert_eq!(
                model.face_occurrences().len(),
                reference.face_occurrences().len()
            );
            assert_eq!(model.accuracy().max_repair_displacement_mm, None);
            assert_eq!(
                model._repair_ledger.2, 0,
                "normalized preview cannot lend sealed original pairs"
            );
        }
    }
    let mut table = Table::from_data_section(&normalized.data[0]);
    table.set_preview_floor_mm(0.02);
    let original_geometry = OriginalGeometry::capture(&table, degrees.as_bytes());
    let frame = ProjectionFrame {
        occurrence: 0,
        factor: 1.0,
        placement: Matrix4::from_scale(2.0),
        allowance: 0.02,
    };
    // This is the retained legacy scaled-settings branch. No original Table
    // is constructed or used to lend captures to the normalized conversion.
    let mut scaled = table.clone();
    scaled.set_preview_tolerance(PreviewTolerance {
        floor_mm: 0.01,
        declared_multiplier: 0.0,
    });
    let converted = original_geometry
        .convert_normalized_with_refusal(&scaled, (73, Some(84)), &[frame])
        .unwrap();
    assert!(
        converted
            .complete_bound(0)
            .unwrap_err()
            .contains("normalized")
    );
    assert_eq!(converted.operations(0).count(), 0);
    assert_eq!(converted.source().shells()[0].shell().faces.len(), 3);
    let over_budget = degrees.replace(
        "#51=LINE('',#10,#50);",
        "#190=CARTESIAN_POINT('',(1,0.020000000000000001,0));#51=LINE('',#190,#50);",
    );
    let error = import_step(over_budget.into_bytes())
        .err()
        .expect("normalization cannot erase the raw LINE refusal");
    assert!(error.contains("finite LINE repair edge #54"), "{error}");
}

#[test]
#[ignore = "requires the existing authoritative34 manifest and original corpus bytes"]
fn approved_corpus_operation_census() {
    use monstertruck_io::step::load::step_geometry::{ElementarySurface, Surface};
    let manifest = std::env::var("BOARDSTUDIO_CAD_CORPUS_MANIFEST").unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    let rows = manifest["authoritative_imports"].as_array().unwrap();
    assert_eq!(rows.len(), 34);
    for row in rows {
        let path = row["source_path"].as_str().unwrap();
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap();
        assert_eq!(bytes.len() as u64, row["source_bytes"].as_u64().unwrap());
        let model = legacy_model(bytes.clone()).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(model.original_bytes(), bytes);
        let expected = &row["expected"]["bounds_mm"];
        let bounds = model.bounds();
        if let Some(tolerance) = expected["tolerance_mm"]
            .as_f64()
            .or_else(|| expected["tolerance"].as_f64())
        {
            for (side, name) in ["min", "max"].iter().enumerate() {
                for axis in 0..3 {
                    assert!(
                        (bounds[side][axis] - expected[name][axis].as_f64().unwrap()).abs()
                            <= tolerance,
                        "{path}: unchanged independent bounds oracle {name}[{axis}]"
                    );
                }
            }
        } else {
            assert!(
                expected.is_null(),
                "retain explicit missing independent oracle"
            );
        }
        let raw = Table::from_data_section(&model._original_source.data[0]);
        let original = OriginalGeometry::capture(&raw, &bytes);
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        let mut note = |name| {
            *counts.entry(name).or_default() += 1;
        };
        for (solid, _, representation) in &model.occurrences {
            for source in model.solids[solid].source[representation].shells() {
                let ids = original
                    .bind_edges(source)
                    .expect("original edge identities remain complete");
                for (edge, id) in source.shell().edges.iter().zip(ids) {
                    let input = raw.edge_curve[&id].clone().into_owned(&raw).unwrap();
                    match &input.edge_geometry {
                        CurveAny::Line(_) => {
                            let p = Point3::from(&input.edge_start.vertex_geometry);
                            let q = Point3::from(&input.edge_end.vertex_geometry);
                            match &edge.curve {
                                Curve3D::Line(line) if [line.0, line.1] == [p, q] => {
                                    note("native_LINE_vertex_chord")
                                }
                                Curve3D::Line(_) => note("native_LINE_projected_interval"),
                                _ => note("native_LINE_changed_family"),
                            }
                        }
                        CurveAny::BoundedCurve(_) => {
                            match Curve3D::try_from(&input.edge_geometry) {
                                Ok(curve) if curve == edge.curve => {
                                    note("bounded_full_carrier_and_domain_identical")
                                }
                                Ok(_) => note("bounded_trim_or_relabel_requires_witness"),
                                Err(_) => note("bounded_normalization_requires_raw_basis_witness"),
                            }
                        }
                        CurveAny::Conic(_) => note("native_conic_finite_interval"),
                        CurveAny::Pcurve(_) => note("native_PCurve_lifted_interval"),
                        CurveAny::SurfaceCurve(_) => {
                            if let Curve3D::SurfaceCurve(actual) = &edge.curve {
                                match Curve3D::try_from(&input.edge_geometry) {
                                    Ok(Curve3D::SurfaceCurve(raw))
                                        if raw.master_representation()
                                            == actual.master_representation() =>
                                    {
                                        note("surface_curve_selected_master_retained")
                                    }
                                    Ok(Curve3D::SurfaceCurve(_)) => {
                                        note("surface_curve_alternate_master_SELECTED")
                                    }
                                    _ => note(
                                        "surface_curve_original_master_construction_UNSUPPORTED",
                                    ),
                                }
                            } else {
                                note("surface_curve_changed_family");
                            }
                        }
                    }
                }
                for face in &source.shell().faces {
                    note(match &face.surface {
                        Surface::ElementarySurface(ElementarySurface::Plane(_)) => "plane_support",
                        Surface::ElementarySurface(ElementarySurface::CylindricalSurface(_)) => {
                            "cylinder_support"
                        }
                        Surface::ElementarySurface(ElementarySurface::ConicalSurface(_)) => {
                            "cone_support"
                        }
                        Surface::ElementarySurface(ElementarySurface::Sphere(_)) => {
                            "sphere_support"
                        }
                        Surface::ElementarySurface(ElementarySurface::ToroidalSurface(_)) => {
                            "torus_support"
                        }
                        Surface::BsplineSurface(_) => "polynomial_spline_support",
                        Surface::NurbsSurface(_) => "rational_spline_support",
                        Surface::SweepSurface(_) => "sweep_support",
                    });
                }
            }
        }
        let mut outcomes = std::collections::BTreeMap::<String, [usize; 2]>::new();
        for operation in &model._repair_ledger.0 {
            note(operation.stage);
            outcomes.entry(operation.stage.to_string()).or_default()
                [usize::from(operation.bound_mm.is_err())] += 1;
        }
        for _ in 0..model._repair_ledger.2 {
            note("captured_actual_finite_curve_pair");
        }
        let movements: Vec<_> = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .collect();
        let mut unsupported = std::collections::BTreeMap::<&str, usize>::new();
        for reason in model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().err())
        {
            *unsupported.entry(reason).or_default() += 1;
        }
        let covered = model
            .triangle_faces()
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        assert_eq!(covered.len(), model.face_occurrences().len());
        println!(
            "REPAIR_CENSUS {}",
            serde_json::json!({"path":path,"routes":counts,"stageOutcomesOkErr":outcomes,
                "sourceFaces":model.source_faces().len(),"faceOccurrences":model.face_occurrences().len(),
                "lineOccurrences":model.line_presentation().occurrences().len(),
                "retainedOriginalFiniteLineAuthorities":original.finite_lines.len(),
                "projectedAttachmentOperations":movements.iter().filter(|op|op.projected_attachment).count(),
                "finiteLineUnsupportedReasons":unsupported,
                "finiteLineOperations":movements.len(),"nonzeroFiniteLineOperations":movements.iter().filter(|op| op.bound_mm>0.0).count(),
                "unknownFiniteLineOperations":model._repair_ledger.1.iter().filter(|op|op.is_err()).count(),
                "fallbackFaces":model.accuracy().fallback_meshed_faces,"bounds":model.bounds()})
        );
    }
}

fn shifted_line_tetra() -> String {
    include_str!("../../../tests/fixtures/repair-finite-domain.step")
        .replace("0.03125", "0.001953125")
}

// Reverse the carrier edge and both uses together to preserve the oriented boundary.
fn with_edge_sense(text: &str, carrier: u64, reversed: bool) -> String {
    if !reversed {
        return text.to_owned();
    }
    let forward = format!("#23=EDGE_CURVE('',#11,#12,#{carrier},.T.);");
    assert_eq!(text.matches(&forward).count(), 1);
    text.replace(
        &forward,
        &format!("#23=EDGE_CURVE('',#12,#11,#{carrier},.F.);"),
    )
    .replace(
        "#52=ORIENTED_EDGE('',*,*,#23,.F.);",
        "#52=ORIENTED_EDGE('',*,*,#23,.T.);",
    )
    .replace(
        "#60=ORIENTED_EDGE('',*,*,#23,.T.);",
        "#60=ORIENTED_EDGE('',*,*,#23,.F.);",
    )
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn projected_native_line_has_actual_nonzero_attachment_movement() {
    let text = projected_line_tetra("0.015625");
    let model = import_step(text.as_bytes().to_vec()).unwrap();
    let source = &model.solids[&101].source[&104];
    let shell = &source.shells()[0];
    let attachment = shell
        .connectivity_attachments()
        .iter()
        .find(|a| a.edge_id() == 23)
        .unwrap();
    let Curve3D::Line(line) = &shell.shell().edges[attachment.edge_index()].curve else {
        panic!("actual native LINE route");
    };
    assert_eq!([line.0.y, line.1.y], [0.015625; 2]);
    let movement = model
        ._repair_ledger
        .1
        .iter()
        .filter_map(|op| op.as_ref().ok())
        .find(|op| op.edge == 23)
        .expect("actual same-call finite projected attachment must be accounted");
    assert_eq!(movement.bound_mm, 0.015625);
    assert_eq!(model.original_bytes(), text.as_bytes());
    assert_eq!(model.face_occurrences().len(), 4);
    assert_eq!(model._repair_ledger.complete_bound_mm(), None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn raw_spline_normalization_is_a_certified_identity_stage_and_keeps_asset_unknown() {
    let single = include_str!("../../../tests/fixtures/repair-raw-spline.step");
    let spline = "#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#2),.POLYLINE_FORM.,.F.,.F.,(2,2),(3.,3.00000008),.UNSPECIFIED.);";
    assert_eq!(single.matches(spline).count(), 1);
    // A real multispan carrier: the interior label must follow the same recipe.
    let multispan = single.replace(
            spline,
            "#320=CARTESIAN_POINT('',(1.,0.,0.));#22=B_SPLINE_CURVE_WITH_KNOTS('',1,(#1,#320,#2),.POLYLINE_FORM.,.F.,.F.,(2,1,2),(3.,3.00000004,3.00000008),.UNSPECIFIED.);",
        );
    for (text, reversed) in [single, multispan.as_str()]
        .into_iter()
        .flat_map(|text| [(text, false), (text, true)])
    {
        let text = with_edge_sense(text, 22, reversed);
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        assert_eq!(model.original_bytes(), text.as_bytes());
        let stages = model
            ._repair_ledger
            .0
            .iter()
            .filter(|op| op.stage == "raw-spline-normalization-v1")
            .collect::<Vec<_>>();
        let [stage] = stages.as_slice() else {
            panic!("exactly one raw normalization stage, reversed {reversed}");
        };
        assert_eq!(stage.source_edge, Some(23));
        assert!(!stage.diagnostic_only);
        assert_eq!(stage.bound_mm, Ok(0.0), "ideal positive affine knot map");
        assert!(
            model
                ._repair_ledger
                .0
                .iter()
                .any(|op| op.stage == "raw-spline-normalization-numeric-v1"
                    && op.diagnostic_only
                    && op.bound_mm.is_err()),
            "knot rounding stays a separate unproved numeric term"
        );
        assert_eq!(model._repair_ledger.complete_bound_mm(), None);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn full_interval_spline_selection_is_the_authored_domain_with_exact_attachment_residual() {
    let text = include_str!("../../../tests/fixtures/repair-full-interval-spline.step");
    for reversed in [false, true] {
        let text = with_edge_sense(text, 22, reversed);
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        let ops = &model._repair_ledger.0;
        let attachment: Vec<_> = ops
            .iter()
            .filter(|op| op.stage == "native-full-interval-attachment-v1")
            .collect();
        let [attachment] = attachment.as_slice() else {
            panic!("one attachment residual stage, reversed {reversed}");
        };
        assert_eq!(attachment.source_edge, Some(23));
        assert!(!attachment.diagnostic_only);
        let measured = attachment.bound_mm.unwrap();
        assert!(
            (0.001953125..=0.001953125 * (1. + 1e-14)).contains(&measured),
            "outward-rounded exact curve-end to vertex distance {measured}, reversed {reversed}"
        );
        for stage in ["native-finite-interval-v1", "selected-master-v1"] {
            assert!(
                !ops.iter()
                    .any(|op| op.stage == stage && op.source_edge == Some(23)),
                "{stage} placeholder for the resolved edge"
            );
            assert!(
                ops.iter()
                    .any(|op| op.stage == stage && op.source_edge.is_some()),
                "{stage} placeholders remain for unresolved routes"
            );
        }
        assert_eq!(model._repair_ledger.complete_bound_mm(), None);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn planar_uv_line_master_selection_is_unique_with_exact_attachment_residual() {
    let text = include_str!("../../../tests/fixtures/repair-uvline-master.step");
    for reversed in [false, true] {
        let text = with_edge_sense(text, 317, reversed);
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        let ops = &model._repair_ledger.0;
        let stages = |name: &str| {
            ops.iter()
                .filter(|op| op.stage == name && op.source_edge == Some(23))
                .collect::<Vec<_>>()
        };
        let trims = stages("same-master-planar-uvline-trim-v1");
        let [trim] = trims.as_slice() else {
            panic!("one UV line trim stage, reversed {reversed}");
        };
        assert_eq!(trim.bound_mm, Ok(0.0), "restriction of the same master");
        assert_eq!(
            stages("same-master-planar-uvline-sense-inversion-v1").len(),
            usize::from(reversed)
        );
        let attachments = stages("planar-uvline-attachment-v1");
        let [attachment] = attachments.as_slice() else {
            panic!("one attachment residual, reversed {reversed}");
        };
        let measured = attachment.bound_mm.unwrap();
        assert!(
            (0.001953125..=0.001953125 * (1. + 1e-14)).contains(&measured),
            "exact lifted end to vertex distance {measured}, reversed {reversed}"
        );
        for stage in ["native-finite-interval-v1", "selected-master-v1"] {
            assert!(stages(stage).is_empty(), "{stage} resolved for edge 23");
        }
        assert!(
            ops.iter()
                .any(|op| op.stage == "planar-uvline-numeric-conversion-v1"
                    && op.source_edge == Some(23)
                    && op.diagnostic_only
                    && op.bound_mm.is_err()),
            "binary64 UV line retention stays a separate unbounded numeric term"
        );
        assert_eq!(model._repair_ledger.complete_bound_mm(), None);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn circle_edges_keep_their_source_support_with_an_enclosed_end_residual() {
    let text = include_str!("../../../tests/fixtures/source-closed-cylinder-mm.step");
    let model = import_step(text.as_bytes().to_vec()).unwrap();
    let ops = &model._repair_ledger.0;
    let mut residuals = Vec::new();
    for edge in [52, 53] {
        let attachment: Vec<_> = ops
            .iter()
            .filter(|op| op.stage == "native-conic-attachment-v1" && op.source_edge == Some(edge))
            .collect();
        assert!(
            !attachment.is_empty(),
            "circle edge #{edge} attachment stage"
        );
        for op in attachment {
            assert!(!op.diagnostic_only);
            residuals.push(op.bound_mm.unwrap_or_else(|e| panic!("edge #{edge}: {e}")));
        }
        for stage in ["native-finite-interval-v1", "selected-master-v1"] {
            assert!(
                !ops.iter()
                    .any(|op| op.stage == stage && op.source_edge == Some(edge)),
                "{stage} placeholder for resolved circle #{edge}"
            );
        }
    }
    // The stored full turn ends at 2*pi rounded to binary64: sin(v) = -2.449e-16.
    assert!(residuals.iter().all(|r| *r < 1e-12), "{residuals:?}");
    assert!(
        residuals.iter().any(|r| (2.4e-16..1e-15).contains(r)),
        "enclosure sees the rounded full-turn parameter: {residuals:?}"
    );
    assert!(
        ops.iter()
            .any(|op| op.stage == "native-finite-interval-v1" && op.source_edge == Some(54)),
        "the seam LINE keeps its unresolved placeholder"
    );
    assert_eq!(model._repair_ledger.complete_bound_mm(), None);
}

fn projected_line_tetra(offset: &str) -> String {
    let source = include_str!("../../../tests/fixtures/repair-projected-line.step");
    assert_eq!(source.matches("0.015625").count(), 1);
    source.replace("0.015625", offset)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn projected_native_line_exact_default_boundary_refuses_over_budget() {
    let text = projected_line_tetra("0.020000000000000001");
    let result = import_step(text.into_bytes());
    assert!(
        result.is_err(),
        "actual projected attachment above exact default .02 must refuse"
    );
    let error = result.err().unwrap();
    assert!(error.contains("finite LINE repair"), "{error}");
}

#[test]
fn projected_native_line_attachment_does_not_borrow_native_selection_sense() {
    let source = projected_line_tetra("0.015625");
    let source = source.replace(
        "#23=EDGE_CURVE('',#11,#12,#22,.T.);",
        "#23=EDGE_CURVE('',#11,#12,#22,.F.);",
    );
    let model = import_step(source.as_bytes().to_vec()).unwrap();
    let movement = model
        ._repair_ledger
        .1
        .iter()
        .filter_map(|op| op.as_ref().ok())
        .find(|op| op.edge == 23)
        .expect("native interval sense is not attachment-carrier authority");
    assert_eq!(movement.bound_mm, 0.015625);
    assert!(movement.projected_attachment);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    let error = import_step(
        source
            .replace("0.015625", "0.020000000000000001")
            .into_bytes(),
    )
    .err()
    .unwrap();
    assert!(error.contains("finite LINE repair edge #23"), "{error}");
}

#[test]
fn projected_native_line_budget_numeric_decomposition_and_sense_are_separate() {
    for reversed in [false, true] {
        let mut text = projected_line_tetra("0.020000000000000001");
        if reversed {
            for (a, b) in [
                (
                    "#23=EDGE_CURVE('',#11,#12,#22,.T.);",
                    "#23=EDGE_CURVE('',#12,#11,#22,.F.);",
                ),
                (
                    "#52=ORIENTED_EDGE('',*,*,#23,.F.);",
                    "#52=ORIENTED_EDGE('',*,*,#23,.T.);",
                ),
                (
                    "#60=ORIENTED_EDGE('',*,*,#23,.T.);",
                    "#60=ORIENTED_EDGE('',*,*,#23,.F.);",
                ),
            ] {
                assert_eq!(text.matches(a).count(), 1);
                text = text.replace(a, b);
            }
        }
        let model = import_step_with_options(
            text.as_bytes().to_vec(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.02_f64.next_up()),
                ..Default::default()
            },
        )
        .unwrap();
        let movements = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .filter(|op| op.edge == 23)
            .collect::<Vec<_>>();
        assert_eq!(
            movements.len(),
            1,
            "one actual attachment correction, not both views"
        );
        let movement = movements[0];
        assert!(movement.projected_attachment);
        assert_eq!(movement.bound_mm, 0.02_f64.next_up());
        assert!(movement.numeric_bound_mm > 0. && movement.numeric_bound_mm < 1e-16);
        assert!(!movement.within(0.02));
        assert!(movement.within(0.02_f64.next_up()));
        assert_eq!(model.original_bytes(), text.as_bytes());
    }
    let text = projected_line_tetra("0.03125");
    let model = import_step_with_options(
        text.into_bytes(),
        StepImportOptions {
            preview_repair_budget_mm: Some(0.03125),
            ..Default::default()
        },
    )
    .unwrap();
    let movement = model
        ._repair_ledger
        .1
        .iter()
        .filter_map(|op| op.as_ref().ok())
        .find(|op| op.edge == 23)
        .unwrap();
    assert_eq!(
        (movement.bound_mm, movement.numeric_bound_mm),
        (0.03125, 0.)
    );
    assert!(movement.within(0.03125));
    assert!(movement.within(0.03125_f64.next_up()));
    assert!(!movement.within(0.03125_f64.next_down()));
}

#[test]
fn projected_native_line_caps_and_unrelated_authority_cannot_erase_refusal() {
    let unused = "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);";
    for padded in [false, true] {
        for before in [false, true] {
            let mutate = |source: String| {
                let mut text = if before {
                    source.replace("DATA;", &format!("DATA;{unused}"))
                } else {
                    source.replace(
                        "ENDSEC;END-ISO-10303-21;",
                        &format!("{unused}ENDSEC;END-ISO-10303-21;"),
                    )
                };
                if padded {
                    text.push_str(&" ".repeat(300_000));
                }
                text
            };
            let text = mutate(projected_line_tetra("0.020000000000000001"));
            let error = import_step(text.into_bytes())
                .err()
                .expect("known exact over-budget projection refuses even if trace capped");
            assert!(error.contains("finite LINE repair edge #23"), "{error}");
            let text = mutate(projected_line_tetra("0.015625"));
            let model = import_step(text.as_bytes().to_vec()).unwrap();
            assert_eq!(model.original_bytes(), text.as_bytes());
            assert_eq!(model.face_occurrences().len(), 4);
            assert_eq!(model.accuracy().max_repair_displacement_mm, None);
            assert!(
                model
                    ._repair_ledger
                    .1
                    .iter()
                    .filter_map(|op| op.as_ref().ok())
                    .any(|op| op.edge == 23),
                "padding/order/unknown scalar must preserve the sealed actual capture"
            );
        }
    }
}

#[test]
fn projected_native_line_occurrences_apply_metre_units_and_rotation_once() {
    let assembly = include_str!("../../../tests/fixtures/two-occurrences-mm.step");
    let tail = &assembly
        [assembly.find("#200=").unwrap()..assembly.find("ENDSEC;END-ISO-10303-21;").unwrap()];
    for metre in [false, true] {
        let text = projected_line_tetra(if metre { "0.000015625" } else { "0.015625" });
        let mut text = text.replace(
            "ENDSEC;END-ISO-10303-21;",
            &format!("{tail}ENDSEC;END-ISO-10303-21;"),
        );
        if metre {
            text = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        }
        let model = import_step(text.as_bytes().to_vec()).unwrap();
        let movements = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .filter(|op| op.edge == 23)
            .collect::<Vec<_>>();
        assert_eq!(movements.len(), 2);
        assert!(
            movements.iter().all(|op| op.projected_attachment
                && op.bound_mm == 0.015625
                && op.within(0.015625))
        );
        assert_ne!(movements[0].occurrence, movements[1].occurrence);
        for occurrence in 0..2 {
            let operations = model
                ._repair_ledger
                .0
                .iter()
                .filter(|op| op.occurrence == occurrence)
                .collect::<Vec<_>>();
            let stage = |name| operations.iter().position(|op| op.stage == name).unwrap();
            for name in ["source-binding-v1", "original conversion route coverage"] {
                assert_eq!(
                    operations.iter().filter(|op| op.stage == name).count(),
                    1,
                    "{name}: same-call conversion recorded once per occurrence"
                );
            }
            assert_eq!(
                operations
                    .iter()
                    .filter(|op| op.stage == "native-line-attachment-projection-v1"
                        && op.source_edge == Some(23))
                    .count(),
                1,
                "the changed shared edge is recorded once per occurrence"
            );
            assert!(stage("source-binding-v1") < stage("native-line-attachment-projection-v1"));
            assert!(
                stage("native-line-attachment-projection-v1")
                    < stage("original conversion route coverage")
            );
            assert!(
                stage("original conversion route coverage")
                    < stage("complete meshing route coverage"),
                "conversion stays before meshing on each occurrence path"
            );
        }
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.face_occurrences().len(), 8);
        let equality = import_step_with_options(
            text.as_bytes().to_vec(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.015625),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(equality.face_occurrences().len(), 8);
        let error = import_step_with_options(
            text.into_bytes(),
            StepImportOptions {
                preview_repair_budget_mm: Some(0.015625_f64.next_down()),
                ..Default::default()
            },
        )
        .err()
        .unwrap();
        assert!(
            error.contains("finite LINE repair")
                || error.contains("EDGE_CURVE attachment exceeds its source connectivity bound"),
            "{error}"
        );
    }
}

#[test]
fn projected_native_line_refusal_does_not_borrow_mismatched_input_or_attachments() {
    let original = projected_line_tetra("0.020000000000000001");
    let original_table = Table::from_step_bytes(original.as_bytes()).unwrap();
    let geometry = OriginalGeometry::capture(&original_table, original.as_bytes());
    let authority = StepRepairSource::from_step_bytes(original.as_bytes()).unwrap();
    let changed = projected_line_tetra("0.015625");
    let mut table = Table::from_step_bytes(changed.as_bytes()).unwrap();
    table.set_preview_floor_mm(0.02);
    let frames = [ProjectionFrame {
        occurrence: 0,
        factor: 1.,
        placement: Matrix4::identity(),
        allowance: 0.02,
    }];
    let result = geometry
        .convert_with_projected_refusal(&table, &authority, (101, Some(104)), &frames)
        .unwrap();
    assert!(result.complete_bound(0).unwrap_err().contains("mismatch"));
    assert!(result.operations(0).all(|op| op.curve_change().is_none()));
    let shell = &result.source().shells()[0];
    let input = geometry.lines[&23].input.as_ref().unwrap();
    let attachment = shell
        .connectivity_attachments()
        .iter()
        .find(|a| a.edge_id() == 23)
        .unwrap();
    assert!(input.projected_branch(shell, attachment.edge_index(), 23));
    assert!(!input.projected_branch(shell, attachment.edge_index(), 27));
    let other = shell
        .connectivity_attachments()
        .iter()
        .find(|a| a.edge_id() == 27)
        .unwrap();
    assert!(!input.projected_branch(shell, other.edge_index(), 23));
    let ids_changed = original.replace(
        "#22=LINE('',#310,#21);",
        "#311=CARTESIAN_POINT('',(0.,0.020000000000000001,0.));#22=LINE('',#311,#21);",
    );
    let mut table = Table::from_step_bytes(ids_changed.as_bytes()).unwrap();
    table.set_preview_floor_mm(0.02);
    let result = geometry
        .convert_with_projected_refusal(&table, &authority, (101, Some(104)), &frames)
        .unwrap();
    assert!(
        result.complete_bound(0).is_err(),
        "different support identity cannot borrow original authority"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn finite_line_repair_exceeding_preview_budget_refuses_without_attachment_metadata() {
    let text = include_str!("../../../tests/fixtures/repair-finite-domain.step");
    let result = import_step(text.as_bytes().to_vec());
    assert!(
        result.is_err(),
        "a complete finite LINE moved 0.03125 mm even though emitted endpoint attachments are exact"
    );
    assert!(result.err().unwrap().contains("finite LINE repair"));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unrelated_unsupported_numeric_entity_cannot_erase_an_over_budget_operation() {
    let original = include_str!("../../../tests/fixtures/repair-finite-domain.step");
    let source = original.replace("ENDSEC;END-ISO-10303-21;",
            "#4000=DIRECTION('',(1.E-100,1.,0.));#4001=VECTOR('',#4000,1.);#4002=LINE('',#1,#4001);#4003=EDGE_CURVE('',#11,#12,#4002,.T.);ENDSEC;END-ISO-10303-21;");
    assert_ne!(source, original);
    let error = import_step(source.as_bytes().to_vec()).err().unwrap();
    assert!(error.contains("finite LINE repair edge #23"), "{error}");
    let model = import_step_with_options(
        source.as_bytes().to_vec(),
        StepImportOptions {
            preview_repair_budget_mm: Some(0.03125),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(model.original_bytes(), source.as_bytes());
    assert_eq!(model.source_faces().len(), 4);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn whole_finite_line_movement_honors_exact_budget_boundary_and_retains_unknown_asset() {
    let text = include_str!("../../../tests/fixtures/repair-finite-domain.step");
    let budget = 0.03125_f64;
    for allowance in [budget, budget.next_up()] {
        let model = import_step_with_options(
            text.as_bytes().to_vec(),
            StepImportOptions {
                preview_repair_budget_mm: Some(allowance),
                ..Default::default()
            },
        )
        .unwrap();
        let operation = model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .find(|op| op.edge == 23)
            .unwrap();
        assert_eq!(operation.bound_mm, budget);
        assert_eq!(operation.occurrence, 0);
        assert_eq!(model.original_bytes(), text.as_bytes());
        assert_eq!(model.source_faces().len(), 4);
        assert_eq!(model.face_occurrences().len(), 4);
        assert_eq!(
            model
                .triangle_faces()
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            4
        );
        assert_eq!(model.accuracy().selected_repair_budget_mm, Some(allowance));
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }
    let error = import_step_with_options(
        text.as_bytes().to_vec(),
        StepImportOptions {
            preview_repair_budget_mm: Some(budget.next_down()),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("finite LINE repair"), "{error}");
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn finite_line_default_floor_and_metre_occurrence_budget_are_world_mm() {
    let text = shifted_line_tetra().replace("0.001953125", "0.015625");
    let model = import_step(text.into_bytes()).unwrap();
    assert_eq!(
        model
            ._repair_ledger
            .1
            .iter()
            .filter_map(|op| op.as_ref().ok())
            .find(|op| op.edge == 23)
            .unwrap()
            .bound_mm,
        0.015625
    );
    let text = include_str!("../../../tests/fixtures/two-occurrences-mm.step")
        .replace(
            "#22=LINE('',#1,#21);",
            "#310=CARTESIAN_POINT('',(0.,0.000015625,0.));#22=LINE('',#310,#21);",
        )
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let model = import_step(text.clone().into_bytes()).unwrap();
    let bounds: Vec<_> = model
        ._repair_ledger
        .1
        .iter()
        .filter_map(|op| op.as_ref().ok())
        .filter(|op| op.edge == 23)
        .map(|op| (op.occurrence, op.bound_mm))
        .collect();
    assert_eq!(bounds, [(0, 0.015625), (1, 0.015625)]);
    assert_eq!(model.original_bytes(), text.as_bytes());
    assert_eq!(model.face_occurrences().len(), 8);
    let error = import_step_with_options(
        text.into_bytes(),
        StepImportOptions {
            preview_repair_budget_mm: Some(0.015625_f64.next_down()),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.contains("finite LINE repair"), "{error}");
}

#[test]
fn ledger_sums_successive_stages_and_maxes_disjoint_regions_and_occurrences() {
    let op = |occurrence, source_edge, value| Operation {
        occurrence,
        shell: Some(0),
        source_edge,
        stage: "finite-domain test stage",
        bound_mm: Ok(value),
        diagnostic_only: false,
    };
    let mut ledger = Ledger(
        vec![
            op(0, Some(1), 0.125),
            op(0, Some(1), 0.125),
            op(0, Some(2), 0.125),
            op(1, Some(1), 0.125),
        ],
        Vec::new(),
        0,
    );
    assert_eq!(ledger.complete_bound_mm(), Some(0.25_f64.next_up()));
    assert!(
        ledger.enforce_finite_lines(0, 0.25).is_ok(),
        "exact accumulated budget equality passes"
    );
    assert!(
        ledger
            .enforce_finite_lines(0, 0.25_f64.next_down())
            .is_err()
    );
    let mut separate = Ledger::default();
    let mut first = op(0, Some(1), 0.015625);
    first.shell = Some(0);
    let mut second = op(0, Some(1), 0.015625);
    second.shell = Some(1);
    separate.0.extend([first, second]);
    assert_eq!(separate.complete_bound_mm(), Some(0.015625));
    assert!(
        separate.enforce_finite_lines(0, 0.02).is_ok(),
        "disjoint shell occurrences are maxed"
    );
    ledger.unknown(1, "omitted-stage control", "whole domain was not captured");
    assert_eq!(ledger.complete_bound_mm(), None);
    assert_eq!(Ledger::default().complete_bound_mm(), None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn native_line_support_translation_is_measured_on_the_real_import_route() {
    let text = shifted_line_tetra();
    let model = import_step(text.clone().into_bytes()).unwrap();
    let operation = model
        ._repair_ledger
        .0
        .iter()
        .find(|op| op.source_edge == Some(23))
        .unwrap();
    let bound = operation
        .bound_mm
        .expect("the whole native LINE support translation is bounded");
    assert!((0.001953125..0.001953126).contains(&bound), "{bound}");
    assert_eq!(model.original_bytes(), text.as_bytes());
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn support_measurement_uses_world_units_and_keeps_occurrences_separate() {
    let assembly = include_str!("../../../tests/fixtures/two-occurrences-mm.step");
    let text = assembly
        .replace(
            "#22=LINE('',#1,#21);",
            "#310=CARTESIAN_POINT('',(0.,0.001953125,0.));#22=LINE('',#310,#21);",
        )
        .replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    assert!(import_step(text.clone().into_bytes()).is_err());
    let model = import_step_with_options(
        text.into_bytes(),
        StepImportOptions {
            preview_repair_budget_mm: Some(2.0),
            ..Default::default()
        },
    )
    .unwrap();
    let operations: Vec<_> = model
        ._repair_ledger
        .0
        .iter()
        .filter(|op| {
            op.source_edge == Some(23)
                && op.stage == "axis-aligned native LINE support substitution"
        })
        .collect();
    assert_eq!(
        operations.len(),
        2,
        "one correction per occurrence, not per face use"
    );
    assert_ne!(operations[0].occurrence, operations[1].occurrence);
    for op in operations {
        let bound = op.bound_mm.unwrap();
        assert!((1.953125..1.953126).contains(&bound), "{op:?}");
    }
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tilted_native_support_and_selected_pcurves_cannot_borrow_the_line_proof() {
    let tilted = shifted_line_tetra().replace(
        "#20=DIRECTION('',(2.,0.,0.));",
        "#20=DIRECTION('',(2.,0.000001,0.));",
    );
    let model = import_step(tilted.into_bytes()).unwrap();
    let operation = model
        ._repair_ledger
        .0
        .iter()
        .find(|op| op.source_edge == Some(23))
        .unwrap();
    assert_eq!(
        operation.bound_mm,
        Err("native LINE support is not axis-aligned")
    );
    let model =
        import_step(include_bytes!("../../../tests/fixtures/source-pcurve-cube.step").to_vec())
            .unwrap();
    assert!(
        model
            ._repair_ledger
            .0
            .iter()
            .filter(|op| op.stage == "axis-aligned native LINE support substitution")
            .all(|op| op.bound_mm.is_err())
    );
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn unchanged_parsed_line_support_does_not_certify_the_asset_or_source_trim() {
    let bytes = include_bytes!("../../../tests/fixtures/tetra-mm.step");
    let native = import_step(bytes.to_vec()).unwrap();
    assert_eq!(native.accuracy().max_repair_displacement_mm, None);
    assert_eq!(native.accuracy().analysis_error_bound_mm, None);
    let model = legacy_model(bytes.to_vec()).unwrap();
    let operation = model
        ._repair_ledger
        .0
        .iter()
        .find(|op| op.source_edge == Some(23))
        .unwrap();
    assert_eq!(
        operation.bound_mm,
        Ok(0.0),
        "only this support is unchanged"
    );
    assert_eq!(model.accuracy().sampled_support_checks, 0);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert!(
        model
            ._repair_ledger
            .0
            .iter()
            .any(|op| op.stage == "curve trims and surface conversion" && op.bound_mm.is_err())
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn original_topology_identity_mismatch_refuses_a_support_proof() {
    let text = shifted_line_tetra();
    let model = import_step(text.clone().into_bytes()).unwrap();
    let altered = text.replace(
        "#50=ORIENTED_EDGE('',*,*,#27,.T.);",
        "#50=ORIENTED_EDGE('',*,*,#23,.T.);",
    );
    let exchange = parser::parse(&altered).unwrap();
    let raw = OriginalGeometry::capture(
        &Table::from_data_section(&exchange.data[0]),
        altered.as_bytes(),
    );
    assert!(
        raw.bind_edges(&model.solids[&101].source[&104].shells()[0])
            .is_none()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn world_translation_bound_covers_reflection_scale_and_underflow() {
    let placement = Matrix4::from_translation(monstertruck_modeling::Vector3::new(1e10, 0., 0.))
        * Matrix4::from_scale(-3.0);
    let bound = placed_translation_bound([0., 0.125, 0.], 2.0, placement).unwrap();
    assert!((0.75..0.750001).contains(&bound));
    assert!(
        placed_translation_bound([f64::from_bits(1), 0., 0.], 0.5, Matrix4::identity()).unwrap()
            > 0.0
    );
    assert!(placed_translation_bound([f64::MAX, 0., 0.], 2.0, Matrix4::identity()).is_err());
}
