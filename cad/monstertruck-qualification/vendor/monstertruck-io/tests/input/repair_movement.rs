use monstertruck_io::step::load::{
    Table,
    convert::StepRepairSource,
    step_geometry::re_exports::{BoundedCurve, Invertible},
};

const SOURCE: &[u8] = include_bytes!("partial_closed_spline/solid32377-source.step");

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
