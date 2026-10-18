use boardstudio_cad::{StepImportOptions, import_step, import_step_with_options};

const BOX: &[u8] = include_bytes!("fixtures/zero-repair-cuboid-mm.step");
const OFFSET_BOX: &[u8] = include_bytes!("fixtures/zero-repair-cuboid-offset-mm.step");

#[test]
fn original_integer_cuboids_have_complete_zero_repair_without_analysis_certification() {
    for (bytes, bounds) in [
        (BOX, [[0., 0., 0.], [2., 4., 8.]]),
        (OFFSET_BOX, [[-3., 2., -4.], [5., 8., 2.]]),
    ] {
        let model = import_step(bytes.to_vec()).expect("literal native LINE/plane cuboid imports");
        assert_eq!(model.bounds(), bounds);
        assert_eq!(model.original_bytes(), bytes);
        assert_eq!(model.source_faces().len(), 6);
        assert_eq!(model.face_occurrences().len(), 6);
        assert_eq!(model.accuracy().max_repair_displacement_mm, Some(0.0));
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }
}

fn replace_once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1);
    text.replace(old, new)
}

fn remains_unknown_or_refuses(text: String) {
    let bytes = text.into_bytes();
    if let Ok(model) = import_step(bytes.clone()) {
        assert_eq!(model.original_bytes(), bytes);
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }
}

#[test]
fn inconsistent_original_line_and_plane_supports_cannot_claim_zero() {
    let text = std::str::from_utf8(BOX).unwrap();
    // The importer may emit the vertex chord, but that does not repair an
    // unambiguously authored finite original LINE interval.
    remains_unknown_or_refuses(replace_once(
        text,
        "#32=LINE('',#1,#31);",
        "#900=CARTESIAN_POINT('',(0.,1.,0.));#32=LINE('',#900,#31);",
    ));
    remains_unknown_or_refuses(replace_once(
        text,
        "#108=AXIS2_PLACEMENT_3D('',#1,#106,#107);",
        "#900=CARTESIAN_POINT('',(0.,0.,1.));#108=AXIS2_PLACEMENT_3D('',#900,#106,#107);",
    ));
}

#[test]
fn altered_trim_reference_or_orientation_cannot_claim_zero() {
    let text = std::str::from_utf8(BOX).unwrap();
    for (old, new) in [
        (
            "#33=EDGE_CURVE('',#11,#12,#32, .T.);",
            "#33=EDGE_CURVE('',#11,#13,#32, .T.);",
        ),
        (
            "#100=ORIENTED_EDGE('',*,*,#45,.F.);",
            "#100=ORIENTED_EDGE('',*,*,#33,.F.);",
        ),
        (
            "#100=ORIENTED_EDGE('',*,*,#45,.F.);",
            "#100=ORIENTED_EDGE('',*,*,#45,.T.);",
        ),
        (
            "#33=EDGE_CURVE('',#11,#12,#32, .T.);",
            "#33=EDGE_CURVE('',#11,#12,#32, .F.);",
        ),
    ] {
        remains_unknown_or_refuses(replace_once(text, old, new));
    }
}

#[test]
fn rounded_numeric_authority_and_absent_or_non_mm_units_remain_unknown() {
    let text = std::str::from_utf8(BOX).unwrap();
    // This decimal rounds to the same parsed binary64 value as the original 2.
    // The original literal authority must still refuse it.
    let rounded = replace_once(
        text,
        "#2=CARTESIAN_POINT('',(2.,0.,0.));",
        "#2=CARTESIAN_POINT('',(2.0000000000000001,0.,0.));",
    );
    let model = import_step(rounded.clone().into_bytes()).unwrap();
    assert_eq!(model.bounds(), [[0., 0., 0.], [2., 4., 8.]]);
    assert_eq!(model.original_bytes(), rounded.as_bytes());
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    let absent = text
        .replace(
            "#302=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));",
            "",
        )
        .replace("GLOBAL_UNIT_ASSIGNED_CONTEXT((#302))", "");
    remains_unknown_or_refuses(absent);
    remains_unknown_or_refuses(text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)"));
}

#[test]
fn selected_pcurves_and_mixed_assets_remain_unknown() {
    let selected = include_bytes!("fixtures/source-pcurve-cube.step");
    let model = import_step(selected.to_vec()).unwrap();
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    let text = std::str::from_utf8(BOX).unwrap();
    // A second occurrence of the same source cannot borrow the first occurrence's proof.
    let second = std::str::from_utf8(OFFSET_BOX).unwrap();
    let (_, second) = second.split_once("ENDSEC;DATA;").unwrap();
    let second = second.strip_suffix("ENDSEC;END-ISO-10303-21;\n").unwrap();
    let mixed = replace_once(
        text,
        "ENDSEC;END-ISO-10303-21;",
        &format!("{second}ENDSEC;END-ISO-10303-21;"),
    );
    let model = import_step(mixed.into_bytes()).unwrap();
    assert_eq!(model.occurrence_count(), 2);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    let unsupported = second.replace(
        "#1002=CARTESIAN_POINT('',(5.,2.,-4.));",
        "#1002=CARTESIAN_POINT('',(5.0000000000000001,2.,-4.));",
    );
    remains_unknown_or_refuses(replace_once(
        text,
        "ENDSEC;END-ISO-10303-21;",
        &format!("{unsupported}ENDSEC;END-ISO-10303-21;"),
    ));
}

#[test]
fn explicit_precision_budget_and_refinement_do_not_change_zero_or_analysis_distinctions() {
    let text = std::str::from_utf8(BOX).unwrap();
    let declared = replace_once(text,
        "#303=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302)));",
        "#303=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#900)));")
        .replace("ENDSEC;END-ISO-10303-21;", "#900=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#302,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;");
    let model = import_step_with_options(
        declared.clone().into_bytes(),
        StepImportOptions {
            preview_repair_budget_mm: Some(0.03),
            target_deflection_mm: Some(0.01),
        },
    )
    .unwrap();
    assert_eq!(model.original_bytes(), declared.as_bytes());
    assert_eq!(model.precision()[0].uncertainty_mm(), Some(0.001));
    assert_eq!(model.accuracy().selected_repair_budget_mm, Some(0.03));
    assert_eq!(model.accuracy().preview_deflection_mm, 0.01);
    assert_eq!(model.accuracy().max_repair_displacement_mm, Some(0.0));
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn source_integer_authority_respects_part21_comments_and_quoted_labels() {
    let text = std::str::from_utf8(BOX).unwrap();
    let text = replace_once(
        text,
        "#1=CARTESIAN_POINT('',(0.,0.,0.));",
        "/* #999=CARTESIAN_POINT('',(0.0001,0.,0.)); */ #1=CARTESIAN_POINT('quoted '' #999=VECTOR(0.123) ',(0.,/* exact */0.,0.));",
    );
    let model = import_step(text.clone().into_bytes()).unwrap();
    assert_eq!(model.original_bytes(), text.as_bytes());
    assert_eq!(model.accuracy().max_repair_displacement_mm, Some(0.0));
}

#[test]
fn non_surface_presentation_cannot_borrow_the_solid_proof() {
    let text = std::str::from_utf8(BOX).unwrap();
    let text = replace_once(text,"#304=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#301),#303);", "#304=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#301,#902),#303);")
        .replace("ENDSEC;END-ISO-10303-21;", "#901=TRIMMED_CURVE('extra-line',#32,(PARAMETER_VALUE(0.)),(PARAMETER_VALUE(1.)),.T.,.PARAMETER.);#902=GEOMETRIC_CURVE_SET('',(#901));ENDSEC;END-ISO-10303-21;");
    let model = import_step(text.into_bytes()).unwrap();
    assert_eq!(model.source_faces().len(), 6);
    assert!(model.line_presentation().occurrences().len() > 0);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
}

#[test]
fn real_preview_fallback_import_stays_unknown() {
    let model = import_step(include_bytes!("fixtures/circular-segment-sliver-prism.step").to_vec())
        .unwrap();
    assert!(model.accuracy().fallback_meshed_faces > 0);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}
