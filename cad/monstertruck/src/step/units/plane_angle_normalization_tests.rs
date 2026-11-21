use super::*;

const DEGREE_RAD: f64 = 0.0174532925;

fn angle_exchange(angle: &str, assignment: &str, extra: &str) -> String {
    super::test_fixture::exchange(&format!(
        "#1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE({DEGREE_RAD}),#1);\
             #3=(CONVERSION_BASED_UNIT('DEGREE',#2) NAMED_UNIT(#4) PLANE_ANGLE_UNIT());\
             #4=DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\
             #5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(({assignment})));\
             #6=CONICAL_SURFACE('',#9,1.,{angle});\
             #7=SHAPE_REPRESENTATION('cone',(#6),#5);\
             #9=AXIS2_PLACEMENT_3D('',#10,#11,#12);\
             #10=CARTESIAN_POINT('',(0.,0.,0.));\
             #11=DIRECTION('',(0.,0.,1.));\
             #12=DIRECTION('',(1.,0.,0.));\
             {extra}"
    ))
}

fn normalized_angle(step: &str) -> f64 {
    let exchange = normalize_conical_angles(step).unwrap();
    let EntityInstance::Simple { record, .. } = exchange.data[0]
        .entities
        .iter()
        .find(|entity| match entity {
            EntityInstance::Simple { record, .. } => record.name == "CONICAL_SURFACE",
            EntityInstance::Complex { subsuper, .. } => {
                subsuper.0.iter().any(|r| r.name == "CONICAL_SURFACE")
            }
        })
        .unwrap()
    else {
        panic!("expected simple conical surface record")
    };
    let Parameter::List(params) = &record.parameter else {
        panic!("expected conical parameters")
    };
    match params[3] {
        Parameter::Real(value) => value,
        _ => panic!("expected real angle"),
    }
}

#[test]
fn degrees_and_radians_are_normalized_by_declared_context_for_small_and_large_values() {
    for degrees in [2.0_f64, 45.0] {
        let degrees_text = angle_exchange(&degrees.to_string(), "#3", "");
        let radians = degrees * DEGREE_RAD;
        let radians_text = angle_exchange(&radians.to_string(), "#1", "");
        let actual = normalized_angle(&degrees_text);
        let expected = normalized_angle(&radians_text);
        assert!(
            (actual - expected).abs() < 1e-14,
            "{degrees} degrees -> {actual}, expected {expected}"
        );
        assert!((actual - radians).abs() < 1e-14);
    }
}

#[test]
fn absent_angle_unit_retains_radian_interpretation() {
    let step = angle_exchange("2.0", "", "").replace(
        "#5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(()));",
        "#5=REPRESENTATION_CONTEXT('3D','MODEL');",
    );
    assert_eq!(normalized_angle(&step), 2.0);
}

#[test]
fn keeps_schema_permitted_signed_angles_and_zero_without_guessing_a_range() {
    for angle in [0.0_f64, -0.25, 90.0, 270.0] {
        let step = angle_exchange(&angle.to_string(), "#3", "");
        let normalized = normalized_angle(&step);
        assert!((normalized - angle * DEGREE_RAD).abs() < 1e-14);
    }
}

#[test]
fn rejects_non_finite_normalized_cone_angle() {
    let step = angle_exchange("1.E999", "#3", "");
    assert!(
        normalize_conical_angles(&step)
            .unwrap_err()
            .contains("invalid normalized semi-angle")
    );
}

#[test]
fn conflicting_or_malformed_plane_angle_units_fail_closed() {
    let conflict = angle_exchange("45.", "#1,#3", "");
    assert!(
        normalize_conical_angles(&conflict)
            .unwrap_err()
            .contains("assigns 2 plane-angle units")
    );

    let malformed = angle_exchange(
        "45.",
        "#20",
        "#20=(CONVERSION_BASED_UNIT('BAD',#21) NAMED_UNIT(#4) PLANE_ANGLE_UNIT());#21=PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(1.),#404);",
    );
    assert!(
        normalize_conical_angles(&malformed)
            .unwrap_err()
            .contains("missing plane-angle unit #404")
    );
}

#[test]
fn shared_cone_with_conflicting_representation_angle_contexts_is_rejected() {
    let step = angle_exchange(
        "45.",
        "#3",
        "#13=REPRESENTATION_CONTEXT('3D','RADIAN');#14=SHAPE_REPRESENTATION('same',(#6),#13);",
    );
    assert!(
        normalize_conical_angles(&step)
            .unwrap_err()
            .contains("shared by representation contexts with conflicting plane-angle units")
    );
}

#[test]
fn mapped_representation_keeps_its_own_angle_context() {
    let step = angle_exchange(
        "2.0",
        "#3",
        "#13=REPRESENTATION_CONTEXT('3D','MODEL');\
             #14=SHAPE_REPRESENTATION('mapped cone',(#6),#13);\
             #15=REPRESENTATION_MAP(#16,#14);\
             #16=AXIS2_PLACEMENT_3D('',#10,#11,#12);\
             #17=MAPPED_ITEM('mapped',#15,#18);\
             #18=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#11,#12,$,1.);\
             #19=SHAPE_REPRESENTATION('outer',(#17),#5);",
    )
    .replace(
        "#7=SHAPE_REPRESENTATION('cone',(#6),#5);",
        "#7=SHAPE_REPRESENTATION('cone',(),#5);",
    );
    // The mapped geometry's context has no assigned angle unit, so it
    // retains radians even though the outer mapped-item context declares
    // degrees. Traversal through #15 -> #14 must stop at #14.
    assert_eq!(normalized_angle(&step), 2.0);
}

#[test]
fn complex_conical_surface_encoding_is_rejected_explicitly() {
    let step = angle_exchange("2.", "#3", "")
        .replace(
            "#6=CONICAL_SURFACE('',#9,1.,2.);",
            "#6=(CONICAL_SURFACE(2.) ELEMENTARY_SURFACE(#9) SURFACE() GEOMETRIC_REPRESENTATION_ITEM() REPRESENTATION_ITEM(''));",
        );
    assert!(
        normalize_conical_angles(&step)
            .unwrap_err()
            .contains("complex CONICAL_SURFACE #6 encoding is unsupported")
    );
}

#[test]
fn ignores_unowned_orphan_conical_surface() {
    let step = angle_exchange("45.", "#3", "#13=CONICAL_SURFACE('',#9,1.,45.);");
    let exchange = normalize_conical_angles(&step).unwrap();
    let orphan = exchange.data[0]
        .entities
        .iter()
        .find(|entity| matches!(entity, EntityInstance::Simple { id: 13, .. }))
        .unwrap();
    let EntityInstance::Simple { record, .. } = orphan else {
        unreachable!()
    };
    let Parameter::List(params) = &record.parameter else {
        unreachable!()
    };
    assert!(matches!(params[3], Parameter::Real(45.0)));
}
