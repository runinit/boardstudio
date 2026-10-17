use monstertruck_io::step::load::{Table, step_geometry::Curve3D, step_p21::tables::IntoOwned};

const MALFORMED_CLOSED_MASTER: &str = r#"
ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;
#1=CARTESIAN_POINT('',(1.,0.,0.));#2=VERTEX_POINT('',#1);
#3=CARTESIAN_POINT('',(0.,0.,0.));#4=AXIS2_PLACEMENT_3D('',#3,$,$);
#5=CIRCLE('',#4,1.);#6=PLANE('',#4);
#7=SURFACE_CURVE('',#5,(#6),.PCURVE_S1.);
#8=EDGE_CURVE('',#2,#2,#7,.T.);
#9=ORIENTED_EDGE('',*,*,#8,.T.);#10=EDGE_LOOP('',(#9));
#11=FACE_OUTER_BOUND('',#10,.T.);#12=ADVANCED_FACE('',(#11),#6,.T.);
#13=OPEN_SHELL('',(#12));#14=SHELL_BASED_SURFACE_MODEL('',(#13));
#15=SHAPE_REPRESENTATION('',(#14),#16);
#16=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#17))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#18))REPRESENTATION_CONTEXT('','3D'));
#17=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));
#18=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.01),#17,'connectivity',$);
ENDSEC;END-ISO-10303-21;
"#;

#[test]
fn a_closed_edge_cannot_bypass_the_selected_master_slot_guard() {
    let table = Table::from_step(MALFORMED_CLOSED_MASTER.trim()).unwrap();
    table.entity_report.require_empty().unwrap();
    let raw = table.surface_curve[&7].clone().into_owned(&table).unwrap();
    assert!(
        Curve3D::try_from(&raw).is_err(),
        "raw SurfaceCurve slot guard is active"
    );
    let edge = table.edge_curve[&8].clone().into_owned(&table).unwrap();
    let raw_edge_refused = edge.parse_curve3d().is_err();
    let source = table.to_source_bound_shell_checked(&table.shell[&13]);
    eprintln!(
        "closed malformed selected master: raw_edge_refused={raw_edge_refused}, checked_source_success={}",
        source.is_ok()
    );
    assert!(
        raw_edge_refused,
        "same-vertex EDGE_CURVE must also reject a non-PCurve selected slot"
    );
    assert!(
        source.is_err(),
        "checked source authority cannot certify a malformed selected master"
    );
}
