//! Opt-in preview tolerance: sub-floor discrepancies pass only when the floor is set.
use monstertruck_io::step::load::{PreviewTolerance, Table, step_geometry::Curve3D, step_p21::tables::IntoOwned};

const TETRA: &str = include_str!("fixtures/tetra-mm.step");

fn tetra_with_plane_offset(declared_mm: f64, offset_mm: f64) -> Table {
    let text = TETRA
        .replace(
            "#103=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102)));",
            "#103=(GEOMETRIC_REPRESENTATION_CONTEXT(3) REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#102))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#300)));",
        )
        .replace(
            "#57=AXIS2_PLACEMENT_3D('',#1,#55,#56);",
            &format!("#310=CARTESIAN_POINT('',(0.,0.,{offset_mm}));#57=AXIS2_PLACEMENT_3D('',#310,#55,#56);"),
        )
        .replace(
            "ENDSEC;END-ISO-10303-21;",
            &format!("#300=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({declared_mm}),#102,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;"),
        );
    Table::from_step(&text).unwrap()
}

fn import(table: &Table) -> Result<monstertruck_io::step::load::convert::SourceBoundStepSolid, String> {
    table
        .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&101], 104)
        .map_err(|e| e.to_string())
}

#[test]
fn attachment_gap_above_declared_precision_needs_the_preview_floor() {
    let mut table = tetra_with_plane_offset(0.001, 0.00102);
    let error = import(&table).err().expect("strict default refuses");
    assert!(error.contains("source bound") || error.contains("connectivity"), "{error}");
    table.set_preview_floor_mm(0.01);
    let solid = import(&table).expect("preview floor accepts a sub-floor gap");
    let attachments = solid.shells()[0].connectivity_attachments();
    assert!(!attachments.is_empty());
    for attachment in attachments {
        let accuracy = attachment.accuracy();
        assert_eq!(accuracy.declared_bound(), 0.001);
        assert_eq!(accuracy.bound(), 0.01);
    }
}

#[test]
fn attachment_gap_clearly_above_the_floor_still_refuses() {
    let mut table = tetra_with_plane_offset(0.001, 0.05);
    assert!(import(&table).is_err());
    table.set_preview_floor_mm(0.01);
    assert!(import(&table).is_err());
}

#[test]
fn larger_declared_precision_is_not_lowered_by_the_floor() {
    let mut table = tetra_with_plane_offset(0.02, 0.015);
    table.set_preview_floor_mm(0.01);
    let solid = import(&table).expect("declared precision wins when larger");
    for attachment in solid.shells()[0].connectivity_attachments() {
        assert_eq!(attachment.accuracy().bound(), attachment.accuracy().declared_bound());
    }
}

#[test]
fn unretained_uv_line_direction_needs_the_preview_floor() {
    let text = include_str!("fixtures/source-pcurve-cube.step");
    let authored = "#67 = CARTESIAN_POINT('',(0.,0.));";
    assert_eq!(text.matches(authored).count(), 1);
    // At 1e20 the unit direction is lost to working-precision rounding.
    let text = text.replace(authored, "#67 = CARTESIAN_POINT('',(1.E20,1.E20));");
    let table = Table::from_step(&text).unwrap();
    let source = table.surface_curve[&59].clone().into_owned(&table).unwrap();
    let error = Curve3D::try_from(&source).err().expect("strict default refuses");
    assert!(error.to_string().contains("not retained"), "{error}");
    assert!(Curve3D::try_from_surface_curve(&source, true).is_ok());
    // Finite and nonzero checks stay in force.
    let zero = text.replace("#69 = DIRECTION('',(0.,-1.));", "#69 = DIRECTION('',(0.,0.));");
    if let Ok(table) = Table::from_step(&zero)
        && let Ok(holder) = table.surface_curve[&59].clone().into_owned(&table)
    {
        assert!(Curve3D::try_from_surface_curve(&holder, true).is_err());
    }
}

/// An open shell whose single face is a cylinder bounded by one edge. The edge's
/// selected master is a unit-speed 2D LINE along the periodic `u` axis, so the
/// master reports no period of its own and is not a regular native circle.
fn cylinder_line_pcurve_shell(start: &str, end: &str) -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
#1=CARTESIAN_POINT('',({start}));#2=VERTEX_POINT('',#1);\
#3=CARTESIAN_POINT('',(0.,0.,0.));#4=AXIS2_PLACEMENT_3D('',#3,$,$);\
#5=CIRCLE('',#4,1.);#6=CYLINDRICAL_SURFACE('',#4,1.);\
#7=SURFACE_CURVE('',#5,(#31),.PCURVE_S1.);\
#8=EDGE_CURVE('',#2,#32,#7,.T.);\
#9=ORIENTED_EDGE('',*,*,#8,.T.);#10=EDGE_LOOP('',(#9));\
#11=FACE_OUTER_BOUND('',#10,.T.);#12=ADVANCED_FACE('',(#11),#6,.T.);\
#13=OPEN_SHELL('',(#12));#14=SHELL_BASED_SURFACE_MODEL('',(#13));\
#15=SHAPE_REPRESENTATION('',(#14),#16);\
#16=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#17))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#18))REPRESENTATION_CONTEXT('','3D'));\
#17=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\
#18=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#17,'connectivity',$);\
#26=CARTESIAN_POINT('',(0.,0.));#27=DIRECTION('',(1.,0.));#28=VECTOR('',#27,1.);\
#29=LINE('',#26,#28);#30=DEFINITIONAL_REPRESENTATION('',(#29),#33);\
#33=(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('','2D'));\
#31=PCURVE('',#6,#30);\
#34=CARTESIAN_POINT('',({end}));#32=VERTEX_POINT('',#34);\
ENDSEC;END-ISO-10303-21;"
    )
}

fn checked_shell(text: &str, floor_mm: Option<f64>) -> Result<(), String> {
    let mut table = Table::from_step(text).unwrap();
    table.entity_report.require_empty().unwrap();
    if let Some(floor) = floor_mm {
        table.set_preview_floor_mm(floor);
    }
    table
        .to_source_bound_shell_checked(&table.shell[&13])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn closed_edge_on_a_non_circle_master_needs_the_preview_floor() {
    let text = cylinder_line_pcurve_shell("1.,0.,0.", "1.,0.,0.");
    let error = checked_shell(&text, None).err().expect("strict default refuses");
    assert!(error.contains("selected-master") || error.contains("lost"), "{error}");
    checked_shell(&text, Some(0.01)).expect("preview imports the closed edge over one master period");
}

#[test]
fn wrapped_open_edge_without_a_period_needs_the_preview_floor() {
    // 270 degrees to 180 degrees counter-clockwise: the located start parameter
    // follows the end parameter until one surface period is added.
    let text = cylinder_line_pcurve_shell("0.,-1.,0.", "-1.,0.,0.");
    checked_shell(&text, Some(0.01)).expect("preview imports the wrapped edge");
}

#[test]
fn preview_closed_edge_endpoint_beyond_the_allowance_still_refuses() {
    // The vertex lies 0.05 mm off the unit cylinder, farther than the 0.01 mm floor.
    let text = cylinder_line_pcurve_shell("1.05,0.,0.", "1.05,0.,0.");
    assert!(checked_shell(&text, None).is_err());
    let error = checked_shell(&text, Some(0.01)).err().expect("off-curve vertex refuses");
    assert!(error.contains("selected-master") || error.contains("lost"), "{error}");
}

/// Shared tail: units, a 1 um declared connectivity bound, and the 2D representation context.
const SHELL_TAIL: &str = "\
#13=OPEN_SHELL('',(#12));#14=SHELL_BASED_SURFACE_MODEL('',(#13));\
#15=SHAPE_REPRESENTATION('',(#14),#16);\
#16=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#17))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#18))REPRESENTATION_CONTEXT('','3D'));\
#17=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\
#18=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#17,'connectivity',$);\
#33=(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('','2D'));\
ENDSEC;END-ISO-10303-21;";

/// One planar face bounded by one CIRCLE edge from 270 to 90 degrees counter-clockwise, so
/// the interval crosses the seam of the closed rational quadratic circle PCURVE (domain
/// 0..3, no period reported, two domain ends coincident).
fn seam_crossing_nurbs_circle_shell() -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
#1=CARTESIAN_POINT('',(0.,-1.,0.));#2=VERTEX_POINT('',#1);\
#3=CARTESIAN_POINT('',(0.,0.,0.));#4=AXIS2_PLACEMENT_3D('',#3,$,$);\
#5=CIRCLE('',#4,1.);#6=PLANE('',#4);\
#7=SURFACE_CURVE('',#5,(#31),.PCURVE_S1.);\
#8=EDGE_CURVE('',#2,#32,#7,.T.);\
#9=ORIENTED_EDGE('',*,*,#8,.T.);#10=EDGE_LOOP('',(#9));\
#11=FACE_OUTER_BOUND('',#10,.T.);#12=ADVANCED_FACE('',(#11),#6,.T.);\
#34=CARTESIAN_POINT('',(0.,1.,0.));#32=VERTEX_POINT('',#34);\
#40=CARTESIAN_POINT('',(1.,0.));#41=CARTESIAN_POINT('',(1.,1.7320508075688772));\
#42=CARTESIAN_POINT('',(-0.5,0.8660254037844386));#43=CARTESIAN_POINT('',(-2.,0.));\
#44=CARTESIAN_POINT('',(-0.5,-0.8660254037844386));#45=CARTESIAN_POINT('',(1.,-1.7320508075688772));\
#46=CARTESIAN_POINT('',(1.,0.));\
#29=(BOUNDED_CURVE()B_SPLINE_CURVE(2,(#40,#41,#42,#43,#44,#45,#46),.UNSPECIFIED.,.T.,.F.)\
B_SPLINE_CURVE_WITH_KNOTS((3,2,2,3),(0.,1.,2.,3.),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()\
RATIONAL_B_SPLINE_CURVE((1.,0.5,1.,0.5,1.,0.5,1.))REPRESENTATION_ITEM(''));\
#30=DEFINITIONAL_REPRESENTATION('',(#29),#33);\
#31=PCURVE('',#6,#30);{SHELL_TAIL}"
    )
}

#[test]
fn seam_crossing_edge_on_a_closed_spline_master_needs_the_preview_floor() {
    let text = seam_crossing_nurbs_circle_shell();
    let error = checked_shell(&text, None).err().expect("strict default refuses");
    assert!(error.contains("selected-master") || error.contains("lost"), "{error}");
    checked_shell(&text, Some(0.01)).expect("preview trims a contiguous copy across the seam");
}

#[test]
fn seam_crossing_edge_endpoint_beyond_the_allowance_still_refuses() {
    // The end vertex is 0.05 mm off the unit circle, farther than the 0.01 mm floor.
    let text = seam_crossing_nurbs_circle_shell().replace("(0.,1.,0.)", "(0.,1.05,0.)");
    let error = checked_shell(&text, Some(0.01)).err().expect("off-curve vertex refuses");
    assert!(error.contains("selected-master") || error.contains("lost"), "{error}");
}

/// A cylinder face bounded by one edge whose 3D CIRCLE lies at `z = 0.5` while the selected
/// PCURVE (a unit-speed LINE along `u`) runs at `z = 0`. The vertices sit at `leader_z`.
fn cylinder_with_offset_master(leader_z: f64) -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
#1=CARTESIAN_POINT('',(0.,-1.,{leader_z}));#2=VERTEX_POINT('',#1);\
#3=CARTESIAN_POINT('',(0.,0.,0.));#4=AXIS2_PLACEMENT_3D('',#3,$,$);\
#20=CARTESIAN_POINT('',(0.,0.,0.5));#21=AXIS2_PLACEMENT_3D('',#20,$,$);\
#5=CIRCLE('',#21,1.);#6=CYLINDRICAL_SURFACE('',#4,1.);\
#7=SURFACE_CURVE('',#5,(#31),.PCURVE_S1.);\
#8=EDGE_CURVE('',#2,#32,#7,.T.);\
#9=ORIENTED_EDGE('',*,*,#8,.T.);#10=EDGE_LOOP('',(#9));\
#11=FACE_OUTER_BOUND('',#10,.T.);#12=ADVANCED_FACE('',(#11),#6,.T.);\
#26=CARTESIAN_POINT('',(0.,0.));#27=DIRECTION('',(1.,0.));#28=VECTOR('',#27,1.);\
#29=LINE('',#26,#28);#30=DEFINITIONAL_REPRESENTATION('',(#29),#33);\
#31=PCURVE('',#6,#30);\
#34=CARTESIAN_POINT('',(-1.,0.,{leader_z}));#32=VERTEX_POINT('',#34);{SHELL_TAIL}"
    )
}

#[test]
fn offset_selected_master_falls_back_to_the_3d_curve_in_preview() {
    let text = cylinder_with_offset_master(0.5);
    assert!(checked_shell(&text, None).is_err(), "strict default refuses");
    let mut table = Table::from_step(&text).unwrap();
    table.set_preview_floor_mm(0.01);
    let (_, report) = table
        .to_compressed_shell_in_representation_reported(&table.shell[&13], 15)
        .expect("preview converts");
    assert!(report.is_lossless(), "{report}");
    assert_eq!(report.alternate_master_edges().count, 1);
    assert_eq!(report.alternate_master_edges().first_id, Some(8));
}

#[test]
fn no_representation_within_the_allowance_still_refuses() {
    // Vertices 0.05 mm from the 3D circle (z = 0.5) and 0.55 mm from the PCURVE line (z = 0).
    let text = cylinder_with_offset_master(0.55);
    let error = checked_shell(&text, Some(0.01)).err().expect("no representation fits");
    assert!(error.contains("selected-master") || error.contains("lost"), "{error}");
}

/// A bilinear B-spline patch over `u, v` in `0..1` (the plane `z = 0`) bounded by one LINE
/// edge whose start vertex has `u = start_u`: outside the patch domain, on its plane extension.
fn patch_with_vertex_off_domain(start_u: f64, start_z: f64) -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
#1=CARTESIAN_POINT('',({start_u},0.5,{start_z}));#2=VERTEX_POINT('',#1);\
#34=CARTESIAN_POINT('',(0.5,0.5,0.));#32=VERTEX_POINT('',#34);\
#20=CARTESIAN_POINT('',(0.,0.,0.));#21=CARTESIAN_POINT('',(0.,1.,0.));\
#22=CARTESIAN_POINT('',(1.,0.,0.));#23=CARTESIAN_POINT('',(1.,1.,0.));\
#6=B_SPLINE_SURFACE_WITH_KNOTS('',1,1,((#20,#21),(#22,#23)),.UNSPECIFIED.,.F.,.F.,.F.,(2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.);\
#35=DIRECTION('',(1.,0.,0.));#36=VECTOR('',#35,1.);\
#5=LINE('',#1,#36);\
#8=EDGE_CURVE('',#2,#32,#5,.T.);\
#9=ORIENTED_EDGE('',*,*,#8,.T.);#10=EDGE_LOOP('',(#9));\
#11=FACE_OUTER_BOUND('',#10,.T.);#12=ADVANCED_FACE('',(#11),#6,.T.);{SHELL_TAIL}"
    )
}

#[test]
fn vertex_slightly_beyond_a_patch_domain_needs_the_preview_floor() {
    // 5 percent of the u span outside the patch, exactly on its plane.
    let text = patch_with_vertex_off_domain(-0.05, 0.0);
    let error = checked_shell(&text, None).err().expect("strict default refuses");
    assert!(error.contains("attachment") || error.contains("lost"), "{error}");
    checked_shell(&text, Some(0.01)).expect("preview witnesses the vertex on the natural extension");
}

#[test]
fn vertex_far_beyond_a_patch_domain_or_off_its_surface_still_refuses() {
    // 50 percent of the span outside the patch: the extension is not a small overshoot.
    let far = patch_with_vertex_off_domain(-0.5, 0.0);
    assert!(checked_shell(&far, Some(0.01)).is_err());
    // A small overshoot, but 0.05 mm off the surface: beyond the allowance.
    let off = patch_with_vertex_off_domain(-0.05, 0.05);
    assert!(checked_shell(&off, Some(0.01)).is_err());
}

fn policy(floor_mm: f64, declared_multiplier: f64) -> PreviewTolerance {
    PreviewTolerance { floor_mm, declared_multiplier }
}

const PREVIEW_POLICY: PreviewTolerance = PreviewTolerance { floor_mm: 0.02, declared_multiplier: 2.0 };

#[test]
fn preview_allowance_is_the_larger_of_the_floor_and_the_scaled_declaration() {
    // Declared 0.015: the scaled declaration (0.03) beats the floor (0.02).
    let mut table = tetra_with_plane_offset(0.015, 0.025);
    table.set_preview_floor_mm(0.01);
    assert!(import(&table).is_err(), "a unit multiplier leaves 0.025 above the 0.015 declaration");
    table.set_preview_tolerance(PREVIEW_POLICY);
    let solid = import(&table).expect("0.025 is within 2 x 0.015");
    for attachment in solid.shells()[0].connectivity_attachments() {
        let accuracy = attachment.accuracy();
        assert_eq!(accuracy.declared_bound(), 0.015);
        assert!((accuracy.bound() - 0.03).abs() < 1e-12, "{}", accuracy.bound());
    }
    // Declared 0.001: the floor (0.02) wins.
    let mut table = tetra_with_plane_offset(0.001, 0.019);
    table.set_preview_tolerance(PREVIEW_POLICY);
    let solid = import(&table).expect("0.019 is within the 0.02 floor");
    for attachment in solid.shells()[0].connectivity_attachments() {
        assert_eq!(attachment.accuracy().bound(), 0.02);
    }
    let mut table = tetra_with_plane_offset(0.001, 0.025);
    table.set_preview_tolerance(PREVIEW_POLICY);
    assert!(import(&table).is_err(), "0.025 is above both 0.02 and 2 x 0.001");
}

/// One planar face bounded by a cubic B-spline edge (a single span of 6e-4 in
/// parameter, like THQWGD001 edge #11776) and a LINE back. The B-spline's start
/// vertex sits `gap_mm` off the curve, inside the plane, at x = 0.5.
fn spline_edge_shell(declared_mm: f64, gap_mm: f64) -> String {
    format!(
        "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
#1=CARTESIAN_POINT('',(0.5,{gap_mm},0.));#2=VERTEX_POINT('',#1);\
#3=CARTESIAN_POINT('',(10.,0.,0.));#4=VERTEX_POINT('',#3);\
#5=CARTESIAN_POINT('',(0.,0.,0.));#6=AXIS2_PLACEMENT_3D('',#5,$,$);\
#7=PLANE('',#6);\
#10=CARTESIAN_POINT('',(0.,0.,0.));#11=CARTESIAN_POINT('',(3.,0.,0.));\
#12=CARTESIAN_POINT('',(7.,0.,0.));#13=CARTESIAN_POINT('',(10.,0.,0.));\
#14=B_SPLINE_CURVE_WITH_KNOTS('',3,(#10,#11,#12,#13),.UNSPECIFIED.,.F.,.F.,(4,4),(0.0000075,0.000635),.UNSPECIFIED.);\
#15=EDGE_CURVE('',#2,#4,#14,.T.);\
#20=DIRECTION('',(-1.,0.,0.));#21=VECTOR('',#20,1.);\
#22=LINE('',#3,#21);#23=EDGE_CURVE('',#4,#2,#22,.T.);\
#30=ORIENTED_EDGE('',*,*,#15,.T.);#31=ORIENTED_EDGE('',*,*,#23,.T.);\
#32=EDGE_LOOP('',(#30,#31));#33=FACE_OUTER_BOUND('',#32,.T.);\
#34=ADVANCED_FACE('',(#33),#7,.T.);\
#35=OPEN_SHELL('',(#34));#36=SHELL_BASED_SURFACE_MODEL('',(#35));\
#37=SHAPE_REPRESENTATION('',(#36),#38);\
#38=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#39))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#40))REPRESENTATION_CONTEXT('','3D'));\
#39=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\
#40=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({declared_mm}),#39,'connectivity',$);\
ENDSEC;END-ISO-10303-21;"
    )
}

fn checked_spline_shell(
    declared_mm: f64,
    gap_mm: f64,
    tolerance: Option<PreviewTolerance>,
) -> Result<(), String> {
    let mut table = Table::from_step(&spline_edge_shell(declared_mm, gap_mm)).unwrap();
    table.entity_report.require_empty().unwrap();
    if let Some(tolerance) = tolerance {
        table.set_preview_tolerance(tolerance);
    }
    table
        .to_source_bound_shell_checked(&table.shell[&35])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn bspline_edge_vertex_off_its_curve_needs_the_preview_allowance() {
    // THQWGD001 declares 1e-5 mm against vertices 0.0101-0.0148 mm off the curve.
    for gap in [0.0101, 0.0148] {
        let error = checked_spline_shell(1e-5, gap, None).err().expect("strict default refuses");
        assert!(!error.is_empty());
        // The previous 0.01 mm floor still refused these.
        assert!(checked_spline_shell(1e-5, gap, Some(policy(0.01, 1.0))).is_err(), "{gap}");
        checked_spline_shell(1e-5, gap, Some(PREVIEW_POLICY)).unwrap_or_else(|e| panic!("{gap}: {e}"));
    }
    // A declaration of 0.01 scales to the 0.02 floor, so an endpoint gap of 0.0127 imports.
    checked_spline_shell(0.01, 0.0127, Some(PREVIEW_POLICY)).expect("0.0127 is within 0.02");
}

#[test]
fn bspline_edge_vertex_beyond_the_preview_allowance_still_refuses() {
    for gap in [0.025, 0.5] {
        assert!(checked_spline_shell(0.01, gap, Some(PREVIEW_POLICY)).is_err(), "{gap}");
        assert!(checked_spline_shell(1e-5, gap, Some(PREVIEW_POLICY)).is_err(), "{gap}");
    }
}
