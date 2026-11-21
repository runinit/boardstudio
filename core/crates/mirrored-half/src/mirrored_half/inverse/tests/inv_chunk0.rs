use super::*;

#[test]
fn set_constraint_compiles_mixed_linked_and_local_overlay_endpoints() {
    let document = rotated_fixture();
    let canonical = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "constraint/mixed-local".into(),
                source_part_id: "right/U1".into(),
                target_part_id: "right/HEADER".into(),
                offset: Vec2 { x: 5.0, y: -2.0 },
                rotation: 15.0,
            },
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &canonical.prepared_constraints[0]
    else {
        panic!("the mixed SetConstraint must prepare one relation");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/mixed-local".into(),
            source: crate::constraints::ConstraintEndpoint::LinkedMirror {
                pair_id: "pair".into(),
                source_part_id: "left/U1".into(),
                mirror_part_id: "right/U1".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: 5.0, y: -2.0 },
                rotation: 15.0,
            },
        }
    );
}

#[test]
fn set_constraint_refuses_an_omitted_legacy_endpoint() {
    let document = rotated_fixture();
    let error = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: Constraint::Offset {
                id: "constraint/omitted".into(),
                source_part_id: "left/U1".into(),
                target_part_id: "right/GONE".into(),
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 3.0,
            },
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("omitted"), "{error}");
    assert!(error.contains("right/GONE"), "{error}");
}

#[test]
fn moving_a_linked_target_retunes_the_offset_in_its_authored_target_view() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/linked-target".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/U1".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "left/U1".into(),
                at: Vec2 { x: -20.0, y: 4.0 },
            }],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the linked target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        source,
        target,
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
    } = record
    else {
        panic!("the target retune must retain the Offset relation");
    };
    assert_eq!(id, "constraint/linked-target");
    assert_eq!(
        source,
        &crate::constraints::ConstraintEndpoint::Part {
            part_id: "right/HEADER".into(),
        }
    );
    assert_eq!(
        target,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "left/U1".into(),
            mirror_part_id: "right/U1".into(),
        }
    );
    // The rotated pair maps the old and requested authoritative poses into the authored target
    // view; their literal visible delta is (-4, -4), added to the stored (5, -2).
    assert!((offset.x - 1.0).abs() < 1e-12, "{offset:?}");
    assert!((offset.y + 6.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_a_part_offset_target_uses_the_linked_source_view_and_keeps_position_heading() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::PartOffset {
        id: "constraint/part-view-target".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/HEADER".into(),
                at: Vec2 { x: 228.0, y: 32.0 },
            }],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the PartOffset target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::PartOffset { offset, rotation },
        ..
    } = record
    else {
        panic!("the PartOffset relation must stay in its authored axes");
    };
    // Source view heading is 270 degrees for the 90-degree pair map, so (4, 2) turns back to
    // (-2, 4) in the relation's source axes.
    assert!((offset.x - 3.0).abs() < 1e-12, "{offset:?}");
    assert!((offset.y - 2.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_a_companion_with_a_constrained_key_keeps_both_prepared_intents() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.extend([
        Constraint::Offset {
            id: "constraint/key-owner".into(),
            source_part_id: "left/U1".into(),
            target_part_id: "matrix/right-keys/r0c0".into(),
            offset: Vec2 { x: 2.0, y: 1.0 },
            rotation: -0.0,
        },
        Constraint::Offset {
            id: "constraint/constrained-owner-child".into(),
            source_part_id: "left/U1".into(),
            target_part_id: "matrix/right-keys/r0c0/diode".into(),
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    ]);
    let key = crate::model::Position {
        id: "matrix/right-keys/r0c0".into(),
        at: Vec2 { x: 190.0, y: -5.0 },
    };
    let companion = crate::model::Position {
        id: "matrix/right-keys/r0c0/diode".into(),
        at: Vec2 { x: 198.0, y: -7.0 },
    };
    for positions in [vec![key.clone(), companion.clone()], vec![companion, key]] {
        let prepared = canonicalize(&document, &EditOperation::MoveParts { positions }).unwrap();
        let mut upsert_ids = Vec::new();
        let mut deferred = Vec::new();
        for edit in &prepared.prepared_constraints {
            match edit {
                super::constraints::PreparedConstraintEdit::Upsert(
                    crate::constraints::FramedConstraint::Framed { id, .. },
                ) => upsert_ids.push(id.as_str()),
                super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent) => {
                    deferred.push(intent)
                }
                other => panic!("unexpected prepared edit: {other:?}"),
            }
        }
        assert_eq!(upsert_ids, ["constraint/key-owner"]);
        assert_eq!(deferred.len(), 1);
        assert_eq!(
            deferred[0].target_owner.as_deref(),
            Some("matrix/left-keys/r0c0")
        );
        assert_eq!(deferred[0].requested.id, "matrix/left-keys/r0c0/diode");
    }
}

#[test]
fn deliberately_unchanged_and_delegated_requests_run_as_written() {
    for request in [
        json!({"kind": "set-key-binding", "boardId": "right", "keyId": "right/U1", "binding": "A"}),
        json!({"kind": "apply-mechanical-edit", "edit": {"target": {"kind": "instance", "instanceId": "left"}, "change": "enable"}}),
    ] {
        expect_operation(&fixture(), request.clone(), request);
    }
}

#[test]
fn set_layout_on_native_mirror_maps_linked_members_and_preserves_hidden_and_local_members() {
    let document = native_layout_fixture();
    let request = EditOperation::SetLayout {
        layout: document
            .layouts
            .iter()
            .find(|layout| layout.id == "right-layout")
            .unwrap()
            .clone(),
    };
    let result = canonicalize(&document, &request).expect("native mirror layout translates");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.board_id, "left");
    assert_eq!(layout.matrix_ids, ["left-keys"]);
    assert_eq!(layout.part_ids, ["left/U1", "left/HEADER", "left/GONE"]);
}

#[test]
fn native_layout_refuses_unrepresentable_name_local_membership_and_legacy_overlap() {
    let document = native_layout_fixture();
    let mut renamed = document.layouts[1].clone();
    renamed.name = "Target-only rename".into();
    let rename_request = EditOperation::SetLayout { layout: renamed };
    let renamed_result = canonicalize(&document, &rename_request).unwrap();
    assert_eq!(renamed_result.operation, rename_request);
    assert!(matches!(
        renamed_result.coverage,
        Coverage::DeliberatelyUnchanged(_)
    ));

    let mut reassigned = document.layouts[1].clone();
    reassigned.matrix_ids = vec!["left-keys".into()];
    assert!(matches!(
        canonicalize(&document, &EditOperation::SetLayout { layout: reassigned }),
        Err(Refusal::Request(message)) if message.contains("ownership")
    ));

    let mut linked = document.layouts[1].clone();
    linked.mirror_link = Some(
        serde_json::from_value(json!({
            "sourceId":"left-layout","axisX":100.0
        }))
        .unwrap(),
    );
    assert!(matches!(
        canonicalize(&document, &EditOperation::SetLayout { layout: linked }),
        Err(Refusal::Request(message)) if message.contains("legacy mirror link")
    ));

    let mut source_with_new_legacy_link = document.layouts[0].clone();
    source_with_new_legacy_link.mirror_link =
        Some(serde_json::from_value(json!({"sourceId":"right-layout","axisX":100.0})).unwrap());
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: source_with_new_legacy_link
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));

    let mut new_overlap = document.clone();
    add_empty_local_matrix(&mut new_overlap, "new-legacy-peer-matrix");
    new_overlap.layouts.push(
        serde_json::from_value(json!({"id":"new-legacy-peer","name":"New legacy peer",
            "boardId":"right","matrixIds": ["new-legacy-peer-matrix"],"partIds":[]}))
        .unwrap(),
    );
    let mut new_legacy_link = new_overlap.layouts[2].clone();
    new_legacy_link.mirror_link =
        Some(serde_json::from_value(json!({"sourceId":"right-layout","axisX":100.0})).unwrap());
    assert!(matches!(
        canonicalize(
            &new_overlap,
            &EditOperation::SetLayout {
                layout: new_legacy_link
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));

    let mut changed_local_members = document.layouts[1].clone();
    changed_local_members
        .part_ids
        .retain(|id| id != "right/FREE");
    let local_request = EditOperation::SetLayout {
        layout: changed_local_members,
    };
    let local_result = canonicalize(&document, &local_request).unwrap();
    assert_eq!(local_result.operation, local_request);
    assert!(matches!(
        local_result.coverage,
        Coverage::DeliberatelyUnchanged(_)
    ));

    let mut removed_source_local = document.layouts[1].clone();
    removed_source_local
        .part_ids
        .retain(|id| id != "right/HEADER");
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: removed_source_local
            }
        ),
        Err(Refusal::Request(message)) if message.contains("cannot be removed")
    ));

    let mut mixed_target_change = document.layouts[1].clone();
    mixed_target_change.name = "Mixed target-owned name".into();
    mixed_target_change.part_ids = vec![
        "right/HEADER".into(),
        "right/U1".into(),
        "right/FREE".into(),
    ];
    assert!(matches!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: mixed_target_change
            }
        ),
        Err(Refusal::Request(message)) if message.contains("source-projected membership changes")
    ));

    let mut overlap = document.clone();
    add_empty_local_matrix(&mut overlap, "right-local-matrix");
    overlap.layouts.push(
        serde_json::from_value(json!({"id":"legacy-peer","name":"Legacy peer",
            "boardId":"right","matrixIds": ["right-local-matrix"],"partIds":[],
            "mirrorLink":{"sourceId":"right-layout","axisX":100.0}}))
        .unwrap(),
    );
    for id in ["right-layout", "left-layout"] {
        let operation = EditOperation::SetLayout {
            layout: overlap
                .layouts
                .iter()
                .find(|layout| layout.id == id)
                .unwrap()
                .clone(),
        };
        let result = canonicalize(&overlap, &operation);
        assert!(
            matches!(
                &result,
                Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
                    && message.contains("legacy layout mirror link")
            ),
            "{operation:?}: {result:?}"
        );
    }

    let mut repair = overlap
        .layouts
        .iter()
        .find(|layout| layout.id == "legacy-peer")
        .unwrap()
        .clone();
    repair.mirror_link = None;
    let repair_request = EditOperation::SetLayout { layout: repair };
    assert_eq!(
        canonicalize(&overlap, &repair_request).unwrap().operation,
        repair_request
    );

    let mut direct_overlap = document.clone();
    add_empty_local_matrix(&mut direct_overlap, "native-legacy-source-matrix");
    direct_overlap.layouts.push(
        serde_json::from_value(json!({"id":"native-legacy-source",
            "name":"Native legacy source","boardId":"right",
            "matrixIds": ["native-legacy-source-matrix"],"partIds":[]}))
        .unwrap(),
    );
    let native_mirror = direct_overlap
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "right-layout")
        .unwrap();
    native_mirror.mirror_link = Some(
        serde_json::from_value(json!({"sourceId":"native-legacy-source","axisX":100.0})).unwrap(),
    );
    let mut direct_repair = native_mirror.clone();
    direct_repair.mirror_link = None;
    let mut changed_while_unlinking = direct_repair.clone();
    changed_while_unlinking.name = "Changed during unlink".into();
    assert!(matches!(
        canonicalize(
            &direct_overlap,
            &EditOperation::SetLayout {
                layout: changed_while_unlinking
            }
        ),
        Err(Refusal::Request(message)) if message.contains("native mirrored-pair")
    ));
    let direct_repair_request = EditOperation::SetLayout {
        layout: direct_repair,
    };
    assert_eq!(
        canonicalize(&direct_overlap, &direct_repair_request)
            .unwrap()
            .operation,
        direct_repair_request
    );
}

#[test]
fn every_outline_change_arm_is_classified_and_keeps_the_targeted_scalar_or_lifecycle_value() {
    let changes = [
        json!({"set-margin": {"margin": 3.0}}),
        json!({"set-corners": {"corners": "fillet"}}),
        json!({"set-size": {"size": 4.0}}),
        json!({"set-bridge-width": {"bridge_width": 5.0}}),
        json!({"set-repair-enabled": {"enabled": false}}),
        json!({"set-maximum-gap-span": {"value": 6.0}}),
        json!({"set-minimum-connection-width": {"value": 1.5}}),
        json!({"set-edge-clearance": {"value": 0.25}}),
        json!({"set-protected-gap": {"gap_id": "right/gap", "points": []}}),
        json!({"remove-protected-gap": {"gap_id": "right/gap"}}),
        json!({"add-feature": {"feature": {"kind": "polygon", "id": "right/new", "points": [], "operation": "add"}}}),
        json!({"set-feature": {"feature": {"kind": "polygon", "id": "right/old", "points": [], "operation": "add"}}}),
        json!({"remove-feature": {"feature_id": "right/old"}}),
        json!({"add-connection": {"connection": {"id": "right/connection", "width": 2.0, "points": []}}}),
        json!({"remove-connection": {"connection_id": "right/connection"}}),
        json!({"create-generated": {"margin": 2.0, "settings": {}, "part_ids": ["right/U1"]}}),
        json!({"create-linked-refinement": {"version_id": "right-version", "name": "Linked"}}),
        json!({"copy": {"version_id": "right-version", "name": "Fixed"}}),
        json!({"freeze": {"name": "Frozen"}}),
        json!({"edit-generated-perimeter": {"contour": 2, "points": []}}),
        json!({"edit-feature-perimeter": {"feature_id": "right/old", "points": []}}),
        json!("activate"),
        json!({"rename": {"name": "Renamed"}}),
        json!("delete"),
    ];
    assert_eq!(changes.len(), 24);
    for change in changes {
        let is_linked_refinement = change.get("create-linked-refinement").is_some();
        let request = json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "right"}, "change": change,
            "seed": 77, "reviewed": null
        }});
        let result = canonical(&fixture(), &request).expect("every outline change is classified");
        assert_eq!(result.coverage, Coverage::Translated);
        let result = serde_json::to_value(result.operation).unwrap();
        assert_eq!(result["edit"]["target"]["boardId"], "left");
        assert_eq!(
            result["edit"]["seed"], 77,
            "captured allocation seed is preserved"
        );
        if is_linked_refinement {
            assert_eq!(
                result["edit"]["change"]["create-linked-refinement"]["version_id"],
                "left-version"
            );
        }
    }
}

#[test]
fn rotated_outline_controls_anchors_reviews_and_identity_are_inverse_mapped_together() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("outline-version", "left-version", "right-version"))
                .unwrap(),
            serde_json::from_value(corr("outline", "left/feature", "right/feature")).unwrap(),
        ]);
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-version"},
        "change": {"set-feature": {"feature": {"kind": "part-envelope", "id": "right/feature",
            "partIds": ["right/U1", "matrix/right-keys/r0c0", "right/FREE"], "margin": 2.0, "operation": "add",
            "connections": [{"id": "right/connection", "width": 2.0, "points": [
                {"at": {"x": 80, "y": -25}, "partId": null},
                {"at": {"x": 4, "y": 5}, "partId": "right/U1"}]}]}}},
        "seed": 4, "reviewed": [{"points": [{"x": 80, "y": -25}, {"x": 70, "y": -20}, {"x": 65, "y": -18}], "hole": false}]
    }});
    let translated = canonical(&document, &request).expect("rotated reviewed outline translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(value["edit"]["target"]["boardId"], "left");
    assert_eq!(value["edit"]["target"]["versionId"], "left-version");
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["id"],
        "left/feature"
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["partIds"],
        json!(["left/U1", "matrix/left-keys/r0c0", "right/FREE"])
    );
    assert_close(
        &value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][0]["at"],
        &json!({"x": -5.0, "y": 20.0}),
        "world control",
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][1]["at"],
        json!({"x": 4.0, "y": -5.0})
    );
    assert_eq!(
        value["edit"]["change"]["set-feature"]["feature"]["connections"][0]["points"][1]["partId"],
        "left/U1"
    );
    assert_close(
        &value["edit"]["reviewed"][0]["points"][0],
        &json!({"x": -12.0, "y": 35.0}),
        "review first",
    );
    assert_close(
        &value["edit"]["reviewed"][0]["points"][2],
        &json!({"x": -5.0, "y": 20.0}),
        "review last",
    );
}

#[test]
fn a_matrix_shaped_anchor_without_matrix_membership_uses_full_heading_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-loose/r0c0";
    let mirror_id = "matrix/right-loose/r0c0";
    for (old_id, new_id, board_id) in [
        ("matrix/left-keys/r0c0", source_id, "left"),
        ("matrix/right-keys/r0c0", mirror_id, "right"),
    ] {
        let mut new_part = document
            .parts
            .iter()
            .find(|part| part.id == old_id)
            .unwrap()
            .clone();
        new_part.id = new_id.into();
        document.parts.push(new_part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(new_id.into());
    }
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("part", source_id, mirror_id)).unwrap());

    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/new", "anchorPartId": mirror_id,
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 17
    }});
    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": 12.0, "y": -14.0}, {"x": 8.0, "y": -10.0}, {"x": 4.0, "y": -6.0}])
    );
}

#[test]
fn an_unpaired_document_and_source_side_requests_are_returned_unchanged() {
    let request = json!({"kind": "move-parts", "positions": [
        {"id": "matrix/left-keys/r0c0", "at": {"x": 3, "y": 4}},
        {"id": "left/U1", "at": {"x": -20, "y": 1}},
        {"id": "right/FREE", "at": {"x": 310, "y": 5}}]});
    // The unpaired mirror-board part and source parts execute as written.
    expect_operation(&fixture(), request.clone(), request.clone());
    let mut unpaired = fixture();
    unpaired.hardware.as_mut().unwrap().pairs[0].halves = None;
    let moved = json!({"kind": "move-parts", "positions": [
        {"id": "right/U1", "at": {"x": 1, "y": 2}}]});
    expect_operation(&unpaired, moved.clone(), moved);
    let result = canonical(&fixture(), &request).unwrap();
    assert!(result.additions.is_empty());
}

#[test]
fn a_mixed_batch_translates_only_linked_mirror_targets_and_keeps_order() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": 1, "y": 1}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}},
            {"id": "right/HEADER", "at": {"x": 220, "y": 31}},
            {"id": "right/FREE", "at": {"x": 301, "y": 2}},
            {"id": "matrix/right-keys/r0c1", "at": {"x": 175, "y": 6}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": 1, "y": 1}},
            {"id": "left/U1", "at": {"x": -30, "y": 7}},
            {"id": "right/HEADER", "at": {"x": 220, "y": 31}},
            {"id": "right/FREE", "at": {"x": 301, "y": 2}},
            {"id": "matrix/left-keys/r0c1", "at": {"x": 25, "y": 6}}]}),
    );
}

#[test]
fn a_part_added_on_the_mirror_board_becomes_a_source_addition_with_its_mirror_planned() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front",
        "properties": {"note": "kept"}}});
    let result = canonical(&fixture(), &request).unwrap();
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &json!({"kind": "add-part", "boardId": "left", "part": {
            "id": "left/LED", "definitionId": "u", "reference": "left-D9",
            "pose": {"at": {"x": -30, "y": 12}, "rotation": 0}, "side": "front",
            "properties": {"note": "kept"}}}),
        "operation",
    );
    assert_eq!(
        result.additions,
        vec![PartAddition {
            source_id: "left/LED".into(),
            mirror_id: "right/LED".into(),
            mirror_definition_id: "u".into(),
            mirror_reference: "right-D9".into(),
            mirror_layout_id: None,
        }]
    );
}

#[test]
fn a_controller_added_on_the_mirror_lands_on_the_source_without_the_half_turn() {
    // A controller definition in the document switches the AddPart inverse to the reflected
    // heading. The axis map sends (x, y) to (200 - x, y): the authored mirror pose (230, 12)
    // at heading 30 lands on the source (-30, 12) at heading -30 = 330, not 150.
    let mut document = fixture();
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "mcu", "name": "MCU", "kind": "controller",
            "courtyard": [{"x":-5,"y":-4},{"x":5,"y":-4},{"x":5,"y":4},{"x":-5,"y":4}],
            "pads": []
        }))
        .unwrap(),
    );
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/MCU", "definitionId": "mcu", "reference": "right-MCU",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 30}, "side": "front"}});
    let result = canonical(&document, &request).unwrap();
    let EditOperation::AddPart { part, .. } = result.operation else {
        panic!("an addition is expected");
    };
    assert_close(
        &serde_json::to_value(part.pose).unwrap(),
        &json!({"at": {"x": -30, "y": 12}, "rotation": 330}),
        "pose",
    );
    assert_eq!(result.additions[0].source_id, "left/MCU");
    assert_eq!(result.additions[0].mirror_id, "right/MCU");
    // The controller rule is its own inverse: the source addition reflects back to exactly
    // what was authored on the mirror, (200 - -30, 12) at (0 - 330) mod 360 = 30.
    assert_eq!(200.0 - part.pose.at.x, 230.0);
    assert_eq!(part.pose.at.y, 12.0);
    assert_eq!((0.0 - part.pose.rotation).rem_euclid(360.0), 30.0);
}

#[test]
fn a_rotated_pair_map_places_and_turns_an_added_part_back_on_the_source() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 90, "y": -50}, "rotation": 30}, "side": "front"}});
    let result = canonical(&rotated_fixture(), &request).unwrap();
    let EditOperation::AddPart { part, .. } = result.operation else {
        panic!("an addition is expected");
    };
    // Mirror (90, -50) is the image of source (20, 10); the heading is 90 - 30 + 180.
    assert_close(
        &serde_json::to_value(part.pose).unwrap(),
        &json!({"at": {"x": 20, "y": 10}, "rotation": 240}),
        "pose",
    );
}

#[test]
fn a_mirror_side_addition_keeps_an_unpaired_target_layout_membership() {
    let mut document = native_layout_fixture();
    let mut local_layout = document.layouts[1].clone();
    local_layout.id = "right-local-layout".into();
    local_layout.name = "Target only".into();
    local_layout.part_ids.clear();
    document.layouts.push(local_layout);
    let request = json!({"kind": "add-part", "boardId": "right", "layoutId": "right-local-layout", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front"}});
    let result = canonical(&document, &request).unwrap();
    let EditOperation::AddPart { layout_id, .. } = result.operation else {
        panic!("the source-side addition remains AddPart")
    };
    assert!(layout_id.is_none());
    assert_eq!(
        result.additions[0].mirror_layout_id.as_deref(),
        Some("right-local-layout")
    );
}

#[test]
fn removing_an_omitted_mirror_part_is_refused() {
    let message = refusal(
        &fixture(),
        json!({"kind": "remove-parts", "ids": ["right/GONE"]}),
    );
    assert!(
        message.contains("right/GONE") && message.contains("omitted"),
        "{message}"
    );
}

#[test]
fn a_duplicate_position_companion_follows_its_key_when_both_move_in_one_request() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    // The mirror key goes to (210, 0): its source (-10, 0); the offset is (-20, -5).
    expect_operation(
        &document,
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/right-keys/r0c0", "at": {"x": 210, "y": 0}},
            {"id": DIODE, "at": {"x": 190, "y": -5}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c0", "at": {"x": -10, "y": 0}},
            {"id": "matrix/left-keys/r0c0/diode", "at": {"x": -30, "y": -5}}]}),
    );
}

#[test]
fn a_literal_mirror_companion_flips_offset_and_turn() {
    let matrix = companion_matrix(&fixture(), "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -3}, "rotation": 60}]),
        "assemblies",
    );
}

#[test]
fn a_tweaked_companion_subtracts_its_tweak_before_flipping() {
    let mut document = fixture();
    set_companion(
        &mut document,
        "literal-mirror-with-tweak",
        json!({"x": 1.0, "y": 2.0, "rotation": 10.0}),
    );
    // Payload offset (-8, -3) and turn -60: source x = -(-8 - 1), y = -3 - 2, turn = -(-60 - 10).
    let matrix = companion_matrix(&document, "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": 9, "y": -5}, "rotation": 70}]),
        "assemblies",
    );
}

#[test]
fn a_mirror_only_matrix_value_or_unrepresentable_cell_is_refused_not_dropped() {
    for (edits, needle) in [
        (json!({"name": "Renamed right"}), "name"),
        (json!({"definitionId": "other"}), "definition"),
        (json!({"boardId": "left"}), "board"),
        (json!({"mirror": "y"}), "horizontal"),
        (
            json!({"cells": [{"row": 0, "column": 0, "enabled": true, "assembliesLocal": true,
                "assemblies": []}]}),
            "local",
        ),
    ] {
        let message = refusal(&fixture(), set_matrix(mirror_payload(edits.clone())));
        assert!(message.contains(needle), "{edits}: {message}");
    }
}

#[test]
fn matrix_splay_origin_is_a_world_point_and_angles_are_matrix_frame_values() {
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 1,
            "change": {"kind": "origin", "world": {"x": 90, "y": -50}}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 1,
            "change": {"kind": "origin", "world": {"x": 20, "y": 10}}}),
    );
    expect_operation(
        &fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 1,
            "change": {"kind": "origin", "world": null}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 1,
            "change": {"kind": "origin", "world": null}}),
    );
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-splay", "matrixId": "right-keys", "column": 0,
            "change": {"kind": "angle", "angle": 7.5, "affect": "following"}}),
        json!({"kind": "set-matrix-splay", "matrixId": "left-keys", "column": 0,
            "change": {"kind": "angle", "angle": 7.5, "affect": "following"}}),
    );
}

#[test]
fn matrix_placement_values_are_matrix_frame_and_only_the_id_changes() {
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "set-matrix-placement", "matrixId": "right-keys",
            "change": {"kind": "column-offset", "column": 1, "value": {"x": 2.5, "y": -1}}}),
        json!({"kind": "set-matrix-placement", "matrixId": "left-keys",
            "change": {"kind": "column-offset", "column": 1, "value": {"x": 2.5, "y": -1}}}),
    );
}

#[test]
fn a_matrix_linked_natively_and_by_a_legacy_layout_is_refused_from_either_half() {
    let document = with_legacy_link(fixture());
    for matrix in ["left-keys", "right-keys"] {
        let message = refusal(
            &document,
            json!({"kind": "remove-matrix-row", "matrixId": matrix, "row": 0}),
        );
        assert!(
            message.contains("legacy") && message.contains(matrix),
            "{message}"
        );
    }
}
