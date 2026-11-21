use super::*;

use super::test_fixture::exchange;

#[test]
fn recognizes_shape_representations_but_not_relationship_records() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('plain',(),#2);\
             #4=ADVANCED_BREP_SHAPE_REPRESENTATION('brep',(),#2);\
             #5=GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION('wire',(),#2);\
             #6=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#7,#8);\
             #7=REPRESENTATION_RELATIONSHIP('placement','',#3,#4);\
             #8=REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#7,#9);\
             #9=ITEM_DEFINED_TRANSFORMATION('','',#10,#11);\
             #10=AXIS2_PLACEMENT_3D('',#12,#13,#14);\
             #11=AXIS2_PLACEMENT_3D('',#12,#13,#14);\
             #12=CARTESIAN_POINT('',(0.,0.,0.));\
             #13=DIRECTION('',(0.,0.,1.));\
             #14=DIRECTION('',(1.,0.,0.));\
             ",
    );
    assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
    assert!(is_shape_representation_kind("SHAPE_REPRESENTATION"));
    assert!(is_shape_representation_kind(
        "ADVANCED_BREP_SHAPE_REPRESENTATION"
    ));
    assert!(is_shape_representation_kind(
        "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION"
    ));
    assert!(!is_shape_representation_kind(
        "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
    ));
}

#[test]
fn resolves_only_context_linked_typed_units() {
    let step = exchange(
        "/* #99 = (LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.KILO.,.METRE.)); */\
             #1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('unused',#404));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('decoy GLOBAL_UNIT_ASSIGNED_CONTEXT SI_UNIT',(),#3);\
             #5=PRODUCT('GLOBAL_UNIT_ASSIGNED_CONTEXT((#2))', 'quoted #99 = SI_UNIT', #4, #4);\
             ",
    );
    assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
}

#[test]
fn resolves_conversion_based_chains_and_si_prefixes() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#1);\
             #3=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('INCH',#2));\
             #4=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(12.),#3);\
             #5=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('FOOT',#4));\
             #6=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#5)));\
             #7=ADVANCED_BREP_SHAPE_REPRESENTATION('shape',(),#6);\
             ",
    );
    let factor = resolve_mm_per_unit(&step).unwrap();
    assert!((factor - 304.8).abs() < 1e-10, "factor={factor}");
}

#[test]
fn accepts_equivalent_si_and_conversion_context_factors() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#2);\
             #4=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('MILLIMETRE',#3));\
             #5=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #6=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#4)));\
             #7=SHAPE_REPRESENTATION('si',(),#5);\
             #8=SHAPE_REPRESENTATION('conversion',(),#6);\
             ",
    );
    assert!((resolve_mm_per_unit(&step).unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn validates_length_angle_and_solid_angle_assigned_units() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #3=(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT());\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3)));\
             #5=SHAPE_REPRESENTATION('shape',(),#4);\
             ",
    );
    assert_eq!(resolve_mm_per_unit(&step).unwrap(), 1.0);
}

#[test]
fn defaults_absent_length_assignment_to_mm_but_validates_context() {
    let missing = exchange(
        "#1=REPRESENTATION_CONTEXT('3D','MODEL');\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
    );
    assert_eq!(resolve_mm_per_unit(&missing).unwrap(), 1.0);

    let non_length_units = exchange(
        "#1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT());\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
    );
    assert_eq!(resolve_mm_per_unit(&non_length_units).unwrap(), 1.0);

    let malformed_empty = exchange(
        "#1=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT(()));\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&malformed_empty)
            .unwrap_err()
            .contains("empty GLOBAL_UNIT_ASSIGNED_CONTEXT")
    );

    let dangling_context = exchange(
        "#1=SHAPE_REPRESENTATION('shape',(),#404);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&dangling_context)
            .unwrap_err()
            .contains("references missing context entity #404")
    );

    let wrong_context_type = exchange(
        "#1=PRODUCT('not a representation context','','',());\
             #2=SHAPE_REPRESENTATION('shape',(),#1);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&wrong_context_type)
            .unwrap_err()
            .contains("0 REPRESENTATION_CONTEXT records")
    );
}

#[test]
fn rejects_conflicting_and_mixed_context_units() {
    let conflicting = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&conflicting)
            .unwrap_err()
            .contains("assigns 2 length units")
    );

    let mixed = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#2)));\
             #5=SHAPE_REPRESENTATION('shape-mm',(),#3);\
             #6=SHAPE_REPRESENTATION('shape-m',(),#4);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&mixed)
            .unwrap_err()
            .contains("mixed shape representation")
    );
}

#[test]
fn rejects_tiny_but_materially_different_context_factors() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.ATTO.,.METRE.));\
             #2=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.FEMTO.,.METRE.));\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#2)));\
             #5=SHAPE_REPRESENTATION('atto',(),#3);\
             #6=SHAPE_REPRESENTATION('femto',(),#4);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&step)
            .unwrap_err()
            .contains("mixed shape representation")
    );
}

fn conversion_chain(conversion_units: u64) -> String {
    let mut data = String::from("#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.));\n");
    let mut base = 1_u64;
    for index in 0..conversion_units {
        let measure = 1000 + index;
        let unit = 2 + index;
        data.push_str(&format!(
            "#{measure}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#{base});\n\
                 #{unit}=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('CHAIN',#{measure}));\n"
        ));
        base = unit;
    }
    data.push_str(&format!(
        "#2000=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#{base})));\n\
             #2001=SHAPE_REPRESENTATION('depth',(),#2000);\n"
    ));
    exchange(&data)
}

#[test]
fn rejects_overdeep_conversion_chains() {
    assert!(
        resolve_mm_per_unit(&conversion_chain(70))
            .unwrap_err()
            .contains("exceeds maximum depth")
    );
}

#[test]
fn conversion_chain_depth_boundary_is_explicit() {
    assert!(resolve_mm_per_unit(&conversion_chain(63)).is_ok());
    assert!(
        resolve_mm_per_unit(&conversion_chain(64))
            .unwrap_err()
            .contains("exceeds maximum depth 64")
    );
}

#[test]
fn rejects_cyclic_conversion_chains() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('LOOP',#2));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('cycle',(),#3);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&step)
            .unwrap_err()
            .contains("cyclic length unit conversion")
    );
}

#[test]
fn rejects_unit_with_conflicting_si_and_conversion_records() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) CONVERSION_BASED_UNIT('conflict',#2));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&step)
            .unwrap_err()
            .contains("has 2 SI_UNIT, CONVERSION_BASED_UNIT, or DERIVED_UNIT records")
    );
}

#[test]
fn rejects_dangling_and_non_unit_assignment_references() {
    let dangling = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#404)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&dangling)
            .unwrap_err()
            .contains("references missing entity #404")
    );

    let wrong_type = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=PRODUCT('not a unit','','',());\
             #3=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2)));\
             #4=SHAPE_REPRESENTATION('shape',(),#3);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&wrong_type)
            .unwrap_err()
            .contains("not exactly one supported named")
    );
}

#[test]
fn rejects_assignment_on_non_representation_context_entity() {
    let step = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&step)
            .unwrap_err()
            .contains("has 0 REPRESENTATION_CONTEXT records")
    );
}

#[test]
fn rejects_nonlength_conversion_base() {
    let nonlength_base = exchange(
        "#1=(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.));\
             #2=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\
             #3=(LENGTH_UNIT() NAMED_UNIT(*) CONVERSION_BASED_UNIT('BAD_LENGTH',#2));\
             #4=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#3)));\
             #5=SHAPE_REPRESENTATION('shape',(),#4);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&nonlength_base)
            .unwrap_err()
            .contains("conversion base entity #1 is not a LENGTH_UNIT")
    );
}

#[test]
fn rejects_duplicate_si_definition_records() {
    let duplicate_si = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) SI_UNIT(.CENTI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&duplicate_si)
            .unwrap_err()
            .contains("has 2 SI_UNIT, CONVERSION_BASED_UNIT, or DERIVED_UNIT records")
    );
}

#[test]
fn rejects_duplicate_context_assignment_records() {
    let duplicate_context_assignment = exchange(
        "#1=(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.));\
             #2=(REPRESENTATION_CONTEXT('3D','MODEL') GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)) GLOBAL_UNIT_ASSIGNED_CONTEXT((#1)));\
             #3=SHAPE_REPRESENTATION('shape',(),#2);\
             ",
    );
    assert!(
        resolve_mm_per_unit(&duplicate_context_assignment)
            .unwrap_err()
            .contains("has 2 GLOBAL_UNIT_ASSIGNED_CONTEXT records")
    );
}
