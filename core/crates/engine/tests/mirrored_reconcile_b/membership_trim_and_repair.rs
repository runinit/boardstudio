use super::*;

#[test]
fn prepared_compaction_relocates_outline_controls_and_member_payload() {
    let mut before = compaction_document().unwrap();
    add_membership_cleanup_sentinels(&mut before);
    let keycaps = before.keycaps.as_mut().unwrap();
    for id in ["r0c1/diode", "r0c2/diode"] {
        let setting = keycaps.keys[&format!("matrix/mirror-matrix/{id}")].clone();
        keycaps
            .keys
            .insert(format!("matrix/source-matrix/{id}"), setting);
    }
    let (after, changed) = crate::support::prepare_transaction(
        &before,
        &[],
        &EditOperation::RemoveMatrixColumn {
            matrix_id: "source-matrix".into(),
            column: 1,
        },
    )
    .unwrap();
    let removed = "matrix/mirror-matrix/r0c1/diode";
    let shifted = "matrix/mirror-matrix/r0c2/diode";
    assert_eq!(
        after.keymap.as_ref().unwrap().layers[0]
            .bindings
            .get(removed),
        Some(&KeyBinding::KeyPress {
            keycode: "B".into()
        }),
        "the shifted identity carries its payload across the removed key"
    );
    assert_eq!(
        after.keymap.as_ref().unwrap().layers[0]
            .sensors
            .get(removed)
            .unwrap()
            .clockwise,
        KeyBinding::KeyPress {
            keycode: "E".into()
        }
    );
    assert!(
        !after.keymap.as_ref().unwrap().layers[0]
            .sensors
            .contains_key(shifted)
    );
    assert!(!after.keycaps.as_ref().unwrap().keys.contains_key(shifted));
    assert_eq!(
        after
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .get(removed)
            .unwrap()
            .legend
            .as_deref(),
        Some("shifted sentinel")
    );
    assert_eq!(
        after.hardware.as_ref().unwrap().instances[1].controller_part_id,
        None
    );
    assert_eq!(
        after.hardware.as_ref().unwrap().boards[0]
            .locks
            .get(&format!("direct/{removed}"))
            .map(String::as_str),
        None
    );
    assert_eq!(
        after.hardware.as_ref().unwrap().boards[0]
            .assignments
            .get(&format!("link/{removed}"))
            .map(String::as_str),
        Some("assignment sentinel")
    );
    assert!(
        !after.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .contains_key(shifted)
    );
    assert_eq!(
        after.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .get(removed)
            .map(String::as_str),
        Some("binding sentinel")
    );
    assert_eq!(after.embedded_circuits[0].part_ids, vec![removed]);
    assert!(after.modules.by_id("closure-module").connection.is_none());
    assert_eq!(
        after
            .modules
            .by_id("shifted-module")
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        removed
    );
    let OutlineFeature::PartEnvelope {
        part_ids,
        connections,
        settings,
        ..
    } = after
        .outline
        .iter()
        .find(|feature| feature.id() == "closure-envelope")
        .unwrap()
    else {
        panic!("the closure envelope remains a part envelope");
    };
    let current_members: BTreeSet<_> = after
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .cloned()
        .collect();
    assert_eq!(
        part_ids.iter().cloned().collect::<BTreeSet<_>>(),
        current_members
    );
    assert_eq!(connections[0].points[0].part_id, None);
    assert_eq!(connections[0].points[1].part_id.as_deref(), Some(removed));
    assert_eq!(
        settings.repair.as_ref().unwrap().keep_gaps[0].points[0].part_id,
        None
    );
    let outline = after
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    let version = outline
        .versions
        .iter()
        .find(|version| version.id == "closure-version")
        .unwrap();
    assert!(
        matches!(&version.geometry.features[0], OutlineFeature::PartEnvelope { part_ids, connections, .. }
        if part_ids == &vec![removed.to_owned()]
            && connections[0].points[0].part_id.is_none()
            && connections[0].points[1].part_id.as_deref() == Some(removed))
    );
    assert_eq!(version.geometry.bridges[0].part_ids, vec![removed]);
    assert_eq!(
        version.geometry.protected_gaps[0].points[0]
            .part_id
            .as_deref(),
        Some(removed)
    );
    assert_eq!(
        outline.generated_last_valid.as_ref().unwrap().bridges[0].part_ids,
        vec![removed]
    );

    let mirror = after.matrices.by_id("mirror-matrix");
    assert_eq!((mirror.rows, mirror.columns), (2, 3));
    assert_eq!(mirror.part_ids.len(), 12);
    let d_diode = after.parts.by_id("matrix/mirror-matrix/r0c2/diode");
    assert_eq!(
        d_diode.pose,
        Pose2 {
            at: Vec2 { x: 98.0, y: -56.0 },
            rotation: 75.0
        }
    );
    assert_eq!(
        d_diode.properties.as_ref().unwrap()["logical-payload"],
        serde_json::json!("matrix/mirror-matrix/r0c3/diode")
    );
    for id in [
        "shifted-module",
        "closure-circuit",
        "closure-envelope",
        "source-matrix",
        "mirror-matrix",
    ] {
        assert!(changed.iter().any(|item| item == id), "{id}: {changed:?}");
    }
    let halves = after.first_halves();
    for row in 0..2 {
        for column in 0..3 {
            let old_column = if column >= 1 { column + 1 } else { column };
            for suffix in ["", "/diode"] {
                let moved = |c| format!("matrix/source-matrix/r{row}c{c}{suffix}");
                let row_for = |h: &MirroredHalves, c| {
                    h.correspondences
                        .iter()
                        .find(|r| r.source_id == moved(c))
                        .unwrap()
                        .clone()
                };
                let mut expected = row_for(before.first_halves(), old_column);
                expected.source_id = moved(column);
                expected.mirror_id = moved(column).replace("source-matrix", "mirror-matrix");
                assert_eq!(row_for(halves, column), expected);
            }
        }
    }
}

#[test]
fn repair_fresh_refuses_native_and_legacy_matrix_overlap() {
    let mut before = compaction_document().unwrap();
    let mut third = before.matrices.by_id("source-matrix").clone();
    third.id = "legacy-third-matrix".into();
    third.part_ids.clear();
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &third).unwrap();
    before.layouts = vec![
        Layout {
            id: "legacy-source-layout".into(),
            name: "Legacy source".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["source-matrix".into()],
            part_ids: vec![],
            mirror_link: None,
        },
        Layout {
            id: "legacy-third-layout".into(),
            name: "Legacy third".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["legacy-third-matrix".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                correspondences: vec![MirroredCorrespondence {
                    kind: MirroredItemKind::Matrix,
                    source_id: "source-matrix".into(),
                    mirror_id: "legacy-third-matrix".into(),
                    companion_mode: None,
                    companion_tweak: None,
                }],
                exceptions: vec![],
                source_id: "legacy-source-layout".into(),
                axis_x: 10.0,
                split_gap: None,
                spacing: None,
            }),
        },
    ];
    before
        .constraints
        .retain(|constraint| constraint.id() != "retained-second-key-offset");
    boardstudio_core_kernel::matrix::layout::validate(&before)
        .expect("the legacy pair alone is valid on one board");
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the native record alone is valid");
    let result = crate::support::prepare_transaction(
        &before,
        &[],
        &EditOperation::RemoveMatrixColumn {
            matrix_id: "source-matrix".into(),
            column: 1,
        },
    );
    assert!(result.is_err(), "overlapping legacy and native membership");
}
