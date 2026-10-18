use monstertruck_io::step::load::{
    Table,
    convert::StepRepairSource,
    step_geometry::re_exports::{BoundedCurve, Invertible},
};

const SOURCE: &[u8] = include_bytes!("partial_closed_spline/solid32377-source.step");

#[test]
fn repair_movement_planar_seam_retains_actual_extension_trim_and_output() {
    let text = include_str!("../../../../../monstertruck/tests/fixtures/repair-planar-seam.step");
    let mut table = Table::from_step(text).unwrap();
    table.set_preview_floor_mm(0.02);
    let original = StepRepairSource::from_step_bytes(text.as_bytes()).unwrap();
    let owner = table
        .to_source_bound_solid_with_repair_checked(14, Some(15), &original)
        .unwrap();
    let legacy = table
        .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&14], 15)
        .unwrap();
    assert_eq!(owner.source().shells()[0].shell(), legacy.shells()[0].shell());
    let operations = owner.operations(0).filter(|op| op.source_ids().2 == Some(8)).collect::<Vec<_>>();
    let extension = operations.iter().find(|op| op.stage() == "same-master-planar-pcurve-seam-extension-v1")
        .expect("the successful real seam extension needs its same-call finite pair");
    let extension = extension.curve_change().expect("actual selected carrier capture");
    assert_eq!(extension.domains(), ((0., 3.), (0., 6.)));
    let trim = operations.iter().find(|op| op.stage() == "same-master-planar-pcurve-trim-v1")
        .unwrap().curve_change().unwrap();
    assert_eq!(extension.curves().1, trim.curves().0);
    assert_eq!(trim.domains().0, trim.domains().1);
    let (u,v) = trim.domains().0;
    assert!(u > 2. && u < 3. && v > 3. && v < 4., "{u}..{v}");
    assert_eq!(trim.curves().1, &owner.source().shells()[0].shell().edges[0].curve);
    assert!(owner.complete_bound(0).is_err(), "selection/master remain unsupported");
}

#[test]
fn repair_movement_projected_line_captures_attachment_not_native_support() {
    use monstertruck_io::step::load::step_geometry::Curve3D;
    let original =
        include_str!("../../../../../monstertruck/tests/fixtures/repair-projected-line.step");
    for reversed in [false, true] {
        let mut text = original.to_owned();
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
        let mut table = Table::from_step_bytes(text.as_bytes()).unwrap();
        table.set_preview_floor_mm(0.02);
        let authority = StepRepairSource::from_step_bytes(text.as_bytes()).unwrap();
        let owner = table
            .to_source_bound_solid_with_repair_checked(101, Some(104), &authority)
            .unwrap();
        let legacy = table
            .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&101], 104)
            .unwrap();
        assert_eq!(
            owner.source().shells()[0].shell(),
            legacy.shells()[0].shell()
        );
        let operations = owner
            .operations(0)
            .filter(|op| op.source_ids().2 == Some(23))
            .collect::<Vec<_>>();
        let stage = operations
            .iter()
            .find(|op| op.stage() == "native-line-attachment-projection-v1")
            .unwrap();
        let view = stage.curve_change().unwrap();
        assert_eq!(view.domains(), ((0., 1.), (0., 1.)));
        let (Curve3D::Line(before), Curve3D::Line(after)) = view.curves() else {
            panic!("whole finite affine carriers");
        };
        assert_eq!(
            [before.0.y, before.1.y],
            [0.; 2],
            "attachment chord BEFORE, not native LINE support"
        );
        assert_eq!([after.0.y, after.1.y], [0.015625; 2]);
        let changes = operations
            .iter()
            .filter_map(|op| op.curve_change())
            .collect::<Vec<_>>();
        assert_eq!(changes.len(), if reversed { 2 } else { 1 });
        for pair in changes.windows(2) {
            assert_eq!(pair[0].curves().1, pair[1].curves().0);
            assert_eq!(pair[0].domains().1, pair[1].domains().0);
        }
        let shell = &owner.source().shells()[0];
        let attachment = shell
            .connectivity_attachments()
            .iter()
            .find(|a| a.edge_id() == 23)
            .unwrap();
        assert_eq!(
            changes.last().unwrap().curves().1,
            &shell.shell().edges[attachment.edge_index()].curve
        );
        if reversed {
            assert_eq!(changes.last().unwrap().domains().1, (1., 0.));
        }
        assert!(
            operations
                .iter()
                .any(|op| op.stage() == "native-finite-interval-v1"
                    && op.bound().is_err()
                    && op.curve_change().is_none())
        );
        assert!(owner.complete_bound(0).is_err());
    }
}

#[test]
fn repair_movement_pairs_chain_to_actual_selected_partial_carriers() {
    let table = Table::from_step_bytes(SOURCE).unwrap();
    let authority = StepRepairSource::from_step_bytes(SOURCE).unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(32377, Some(17), &authority)
        .unwrap();
    let existing = table
        .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&32377], 17)
        .unwrap();
    assert_eq!(traced.source().shells().len(), existing.shells().len());
    for (actual, legacy) in traced.source().shells().iter().zip(existing.shells()) {
        assert_eq!(actual.shell(), legacy.shell(), "same actual legacy output");
        assert_eq!(actual.source_face_ids(), legacy.source_face_ids());
    }
    assert!(
        traced.complete_bound(0).is_err(),
        "captured selection is not original selection proof"
    );
    for (id, interval) in [(32105, (0.0, 0.5)), (32205, (0.5, 1.0))] {
        let operations: Vec<_> = traced
            .operations(0)
            .filter(|op| op.source_ids().2 == Some(id))
            .collect();
        let changes: Vec<_> = operations
            .iter()
            .filter_map(|op| op.curve_change())
            .collect();
        assert!(
            !changes.is_empty(),
            "source edge {id} needs actual mutation capture"
        );
        for pair in changes.windows(2) {
            assert_eq!(pair[0].curves().1, pair[1].curves().0);
            assert_eq!(pair[0].domains().1, pair[1].domains().0);
        }
        assert_eq!(
            changes[0].domains().0,
            interval,
            "capture selected finite domain, not full support"
        );
        let shell = &traced.source().shells()[0];
        let attachment = shell
            .connectivity_attachments()
            .iter()
            .find(|a| a.edge_id() == id)
            .unwrap();
        assert_eq!(
            changes.last().unwrap().curves().1,
            &shell.shell().edges[attachment.edge_index()].curve
        );
        assert!(
            operations
                .iter()
                .any(|op| op.curve_change().is_none() && op.bound().is_err()),
            "earlier source selection/master coverage must not be manufactured"
        );
    }
}

#[test]
fn repair_movement_orientation_has_its_own_immediate_pair() {
    // Swapping vertices and source sense preserves physical output orientation
    // only after the converter's actual inversion; keep that stage explicit.
    let text = std::str::from_utf8(SOURCE).unwrap();
    let source = text.replace(
        "#32105=EDGE_CURVE('',#32006,#31998,#32104,.T.);",
        "#32105=EDGE_CURVE('',#31998,#32006,#32104,.F.);",
    );
    assert_ne!(source, text);
    let table = Table::from_step_bytes(source.as_bytes()).unwrap();
    let original = StepRepairSource::from_step_bytes(source.as_bytes()).unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(32377, Some(17), &original)
        .unwrap();
    let ops: Vec<_> = traced
        .operations(0)
        .filter(|op| op.source_ids().2 == Some(32105))
        .collect();
    let inversion = ops
        .iter()
        .find(|op| op.stage() == "native-sense-inversion-v1")
        .unwrap();
    let change = inversion.curve_change().unwrap();
    assert_eq!(change.curves().0.inverse(), *change.curves().1);
    let (a, b) = change.curves().1.range_tuple();
    assert_eq!(change.domains().1, (b, a));
    let changes: Vec<_> = ops.iter().filter_map(|op| op.curve_change()).collect();
    for pair in changes.windows(2) {
        assert_eq!(pair[0].curves().1, pair[1].curves().0);
    }
}

#[test]
fn repair_movement_source_mismatch_cannot_lend_a_control_net() {
    let source = std::str::from_utf8(SOURCE).unwrap();
    let table = Table::from_step_bytes(SOURCE).unwrap();
    // A harmless source-label change suffices: source authority must match the
    // actual supplied table, not a geometrically similar alternate conversion.
    let alternate = source.replacen("EDGE_CURVE('',", "EDGE_CURVE('alternate',", 1);
    assert_ne!(alternate, source);
    let original = StepRepairSource::from_step_bytes(alternate.as_bytes()).unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(32377, Some(17), &original)
        .unwrap();
    assert!(traced.complete_bound(0).unwrap_err().contains("mismatch"));
    assert_eq!(
        traced
            .operations(0)
            .filter(|op| op.curve_change().is_some())
            .count(),
        0
    );
    assert_eq!(traced.source().shells()[0].source_face_ids().len(), 12);
}

#[test]
fn repair_movement_missing_finite_curve_authority_is_not_zero() {
    let bytes =
        include_bytes!("../../../../../monstertruck/tests/fixtures/repair-finite-domain.step");
    let table = Table::from_step_bytes(bytes).unwrap();
    let original = StepRepairSource::from_step_bytes(bytes).unwrap();
    let id = *table.manifold_solid_brep.keys().next().unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(id, None, &original)
        .unwrap();
    assert_eq!(traced.source().shells()[0].source_face_ids().len(), 4);
    assert_eq!(
        traced
            .operations(0)
            .filter(|op| op.curve_change().is_some())
            .count(),
        0
    );
    assert!(traced.complete_bound(0).is_err());
}

#[test]
fn repair_movement_interior_source_change_is_not_endpoint_authority() {
    use monstertruck_io::step::load::{step_geometry::Curve3D, step_p21::tables::IntoOwned};
    let text = std::str::from_utf8(SOURCE).unwrap();
    let changed = text.replace(
        "#32008=CARTESIAN_POINT('',(7.666318757,-4.850271014,-1.58504582));",
        "#32008=CARTESIAN_POINT('',(7.666318757,-4.850271015,-1.58504582));",
    );
    assert_ne!(changed, text);
    let table = Table::from_step_bytes(changed.as_bytes()).unwrap();
    let original_table = Table::from_step_bytes(SOURCE).unwrap();
    let carrier = |table: &Table| {
        let raw = table.edge_curve[&32105].clone().into_owned(table).unwrap();
        Curve3D::try_from(&raw.edge_geometry).unwrap()
    };
    let (before, after) = (carrier(&original_table), carrier(&table));
    assert_eq!(before.front(), after.front());
    assert_eq!(before.back(), after.back());
    assert_ne!(
        before, after,
        "immutable interior controls differ despite equal endpoints"
    );
    let authority = StepRepairSource::from_step_bytes(SOURCE).unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(32377, Some(17), &authority)
        .unwrap();
    assert!(traced.complete_bound(0).unwrap_err().contains("mismatch"));
    assert!(traced.operations(0).all(|op| op.curve_change().is_none()));
}

#[test]
fn repair_movement_report_resource_cap_preserves_the_admitted_output() {
    let mut bytes = SOURCE.to_vec();
    bytes.extend(std::iter::repeat_n(b' ', 300_000));
    let original = bytes.clone();
    let table = Table::from_step_bytes(&bytes).unwrap();
    let authority = StepRepairSource::from_step_bytes(&bytes).unwrap();
    let traced = table
        .to_source_bound_solid_with_repair_checked(32377, Some(17), &authority)
        .unwrap();
    assert!(
        traced
            .complete_bound(0)
            .unwrap_err()
            .contains("resource cap")
    );
    assert_eq!(traced.source().shells()[0].source_face_ids().len(), 12);
    assert!(traced.operations(0).all(|op| op.curve_change().is_none()));
    assert_eq!(bytes, original);
}

#[test]
fn repair_movement_empty_finite_face_coverage_is_incomplete() {
    let bytes = b"ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;#1=CLOSED_SHELL('',());#2=MANIFOLD_SOLID_BREP('',#1);ENDSEC;END-ISO-10303-21;";
    let table = Table::from_step_bytes(bytes).unwrap();
    let authority = StepRepairSource::from_step_bytes(bytes).unwrap();
    let report = table
        .to_source_bound_solid_with_repair_checked(2, None, &authority)
        .unwrap();
    assert!(
        report.complete_bound(0).is_err(),
        "empty finite face coverage is never a zero movement certificate"
    );
}
