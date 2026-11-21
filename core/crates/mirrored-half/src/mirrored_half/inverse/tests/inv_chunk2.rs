use super::*;

#[test]
fn set_constraint_prepares_one_global_record_with_two_live_mirror_views() {
    let document = rotated_fixture();
    let constraint = Constraint::Offset {
        id: "constraint/durable-7".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "matrix/right-keys/r0c0".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    };
    let canonical = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: constraint.clone(),
        },
    )
    .unwrap();

    assert_eq!(
        canonical.operation,
        EditOperation::SetConstraint { constraint }
    );
    assert_eq!(canonical.prepared_constraints.len(), 1);
    let super::constraints::PreparedConstraintEdit::Upsert(
        crate::constraints::FramedConstraint::Framed {
            id,
            source,
            target,
            relation,
        },
    ) = &canonical.prepared_constraints[0]
    else {
        panic!("SetConstraint did not prepare one framed record");
    };
    assert_eq!(id, "constraint/durable-7");
    assert_eq!(
        source,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "left/U1".into(),
            mirror_part_id: "right/U1".into(),
        }
    );
    assert_eq!(
        target,
        &crate::constraints::ConstraintEndpoint::LinkedMirror {
            pair_id: "pair".into(),
            source_part_id: "matrix/left-keys/r0c0".into(),
            mirror_part_id: "matrix/right-keys/r0c0".into(),
        }
    );
    assert_eq!(
        relation,
        &crate::constraints::ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        }
    );
}

#[test]
fn convert_offset_keeps_legacy_missing_target_precedence_for_mirrors() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Mirror {
        id: "constraint/missing-target".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "does-not-exist".into(),
        axis: crate::model::MirrorAxis::Vertical,
        coordinate: 4.0,
    });
    let error = canonicalize(
        &document,
        &EditOperation::ConvertOffsetConstraint {
            constraint_id: "constraint/missing-target".into(),
            to: crate::model::OffsetAxes::Part,
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("target is missing"), "{error}");
}

#[test]
fn moving_a_visible_mirror_constraint_target_uses_its_literal_inverse_position() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/raw-mirror-target".into(),
        source_part_id: "left/HEADER".into(),
        target_part_id: "right/U1".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/U1".into(),
                at: Vec2 { x: 104.0, y: -1.0 },
            }],
        },
    )
    .unwrap();

    let EditOperation::MoveParts { positions } = &prepared.operation else {
        panic!("a visible mirror target move must stay a MoveParts operation");
    };
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].id, "left/U1");
    assert!(
        (positions[0].at.x + 29.0).abs() < 1e-12,
        "{:?}",
        positions[0]
    );
    assert!(
        (positions[0].at.y + 4.0).abs() < 1e-12,
        "{:?}",
        positions[0]
    );
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("moving the visible mirror target must retune the same global record");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the authored Offset relation must remain an Offset");
    };
    assert_eq!(id, "constraint/raw-mirror-target");
    // The pair map sends the original source (-24, 0) to (100, -6); the visible
    // request (104, -1) contributes (4, 5) to the legacy stored (5, -2).
    assert_eq!(*offset, Vec2 { x: 9.0, y: 3.0 });
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_a_legacy_mirror_constraint_target_is_refused() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Mirror {
        id: "constraint/mirror-target".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/FREE".into(),
        axis: crate::model::MirrorAxis::Vertical,
        coordinate: 200.0,
    });
    let error = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "right/FREE".into(),
                at: Vec2 { x: 302.0, y: 4.0 },
            }],
        },
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("mirror"), "{error}");
    assert!(error.contains("drag"), "{error}");
}

#[test]
fn moving_a_companion_target_defers_until_its_key_owner_pose_is_resolved() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/companion-target".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "matrix/right-keys/r0c0/diode".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![crate::model::Position {
                id: "matrix/right-keys/r0c0/diode".into(),
                at: Vec2 { x: 187.0, y: -4.0 },
            }],
        },
    )
    .unwrap();

    let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
        prepared.prepared_constraints.as_slice()
    else {
        panic!("a companion retune must wait for its live key owner pose");
    };
    let crate::constraints::FramedConstraint::Framed { id, .. } = &intent.record;
    assert_eq!(id, "constraint/companion-target");
    assert_eq!(
        intent.target_owner.as_deref(),
        Some("matrix/left-keys/r0c0")
    );
    assert_eq!(intent.source_owner, None);
    assert_eq!(intent.requested.id, "matrix/left-keys/r0c0/diode");
}

#[test]
fn moving_a_duplicate_position_companion_in_either_order_uses_its_requested_key_owner() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .companion_mode = crate::model::MirroredCompanionMode::DuplicatePosition;
    document.constraints.push(Constraint::Offset {
        id: "constraint/duplicate-position-child".into(),
        source_part_id: "left/U1".into(),
        target_part_id: "matrix/right-keys/r0c0/diode".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let key = crate::model::Position {
        id: "matrix/right-keys/r0c0".into(),
        at: Vec2 { x: 190.0, y: -5.0 },
    };
    let companion = crate::model::Position {
        id: "matrix/right-keys/r0c0/diode".into(),
        at: Vec2 { x: 198.0, y: -7.0 },
    };
    let key_then_companion = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![key.clone(), companion.clone()],
        },
    )
    .unwrap();
    let companion_then_key = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![companion, key],
        },
    )
    .unwrap();

    for prepared in [&key_then_companion, &companion_then_key] {
        let EditOperation::MoveParts { positions } = &prepared.operation else {
            panic!("the duplicate-position move must retain its ordered batch");
        };
        let source_key = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0")
            .unwrap();
        let source_companion = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0/diode")
            .unwrap();
        // DuplicatePosition carries the mirror-local companion delta (8, -2) onto
        // the source key at (-25, -90), giving source companion (-17, -92).
        assert!(
            (source_key.at.x + 25.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_key.at.y + 90.0).abs() < 1e-12,
            "{:?}",
            source_key.at
        );
        assert!(
            (source_companion.at.x + 17.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );
        assert!(
            (source_companion.at.y + 92.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );

        let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
            prepared.prepared_constraints.as_slice()
        else {
            panic!("the duplicate-position child must wait for its requested key owner");
        };
        assert_eq!(
            intent.target_owner.as_deref(),
            Some("matrix/left-keys/r0c0")
        );
        assert_eq!(intent.requested.id, "matrix/left-keys/r0c0/diode");
        assert_eq!(intent.requested.at, source_companion.at);
    }
}

#[test]
fn independent_case_bodies_and_adr_excluded_bindings_keep_targeted_semantics() {
    let document = fixture();
    let case = op(json!({"kind":"set-case","body":{"id":"right/shell",
        "name":"Right shell","boardId":"right","kind":"tray","thickness":2.0,
        "clearance":0.2}}));
    let case_result = canonicalize(&document, &case).unwrap();
    assert_eq!(
        case_result.coverage,
        Coverage::DeliberatelyUnchanged("case bodies are independently authored per board")
    );
    assert_eq!(case_result.operation, case);

    for binding in [
        op(json!({"kind":"set-key-binding","boardId":"right","keyId":"right/U1","binding":"A"})),
        op(
            json!({"kind":"edit-keymap","change":{"kind":"add-layer","id":"right-layer","name":"Right"}}),
        ),
    ] {
        let result = canonicalize(&document, &binding).unwrap();
        assert_eq!(
            result.coverage,
            Coverage::DeliberatelyUnchanged(
                "bindings are never reflected; the edit keeps its targeted-half semantics (ADR-0010)"
            )
        );
        assert_eq!(result.operation, binding);
    }
}

#[test]
fn set_layout_native_mirror_removes_linked_members_from_source_membership() {
    let document = native_layout_fixture();
    let mut requested = document.layouts[1].clone();
    requested.part_ids = vec!["right/HEADER".into(), "right/FREE".into()];
    let result = canonicalize(&document, &EditOperation::SetLayout { layout: requested })
        .expect("removing a linked member maps to source membership");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.part_ids, ["left/HEADER", "left/GONE"]);
}

#[test]
fn source_local_and_legacy_only_layout_requests_keep_their_authored_targets() {
    let document = native_layout_fixture();
    let mut source = document
        .layouts
        .iter()
        .find(|layout| layout.id == "left-layout")
        .unwrap()
        .clone();
    source.name = "Source renamed".into();
    let source_request = EditOperation::SetLayout { layout: source };
    assert_eq!(
        canonicalize(&document, &source_request).unwrap().operation,
        source_request
    );

    let mut local_document = document.clone();
    local_document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(json!({"kind":"layout",
            "mirrorId":"right-layout","state":"local"}))
            .unwrap(),
        );
    let mut local = local_document.layouts[1].clone();
    local.name = "Local target name".into();
    let local_request = EditOperation::SetLayout { layout: local };
    assert_eq!(
        canonicalize(&local_document, &local_request)
            .unwrap()
            .operation,
        local_request
    );

    let mut legacy_document = fixture();
    add_empty_local_matrix(&mut legacy_document, "legacy-source-matrix");
    add_empty_local_matrix(&mut legacy_document, "legacy-target-matrix");
    legacy_document.layouts = vec![
        serde_json::from_value(json!({"id":"legacy-source","name":"Legacy source",
            "boardId":"right","matrixIds": ["legacy-source-matrix"],"partIds":[]}))
        .unwrap(),
        serde_json::from_value(json!({"id":"legacy-mirror","name":"Legacy mirror",
            "boardId":"right","matrixIds": ["legacy-target-matrix"],"partIds":[],
            "mirrorLink":{"sourceId":"legacy-source","axisX":100.0}}))
        .unwrap(),
    ];
    let mut legacy_target = legacy_document.layouts[1].clone();
    legacy_target.name = "Legacy target renamed".into();
    let legacy_request = EditOperation::SetLayout {
        layout: legacy_target,
    };
    assert_eq!(
        canonicalize(&legacy_document, &legacy_request)
            .unwrap()
            .operation,
        legacy_request
    );
}

#[test]
fn omitted_native_layout_is_refused_instead_of_recreated() {
    let mut document = native_layout_fixture();
    let omitted_request = EditOperation::SetLayout {
        layout: document.layouts[1].clone(),
    };
    let mut source_request = document.layouts[0].clone();
    source_request.name = "Source edit while mirror omitted".into();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(json!({"kind":"layout",
            "mirrorId":"right-layout","state":"omitted"}))
            .unwrap(),
        );
    document
        .layouts
        .retain(|layout| layout.id != "right-layout");
    assert_eq!(
        canonicalize(
            &document,
            &EditOperation::SetLayout {
                layout: source_request.clone()
            }
        )
        .unwrap()
        .operation,
        EditOperation::SetLayout {
            layout: source_request
        }
    );
    assert!(matches!(
        canonicalize(&document, &omitted_request),
        Err(Refusal::Request(message)) if message.contains("omitted")
    ));
}

#[test]
fn an_anchor_in_another_matrix_uses_its_own_matrix_frame_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-other/r0c0";
    let mirror_id = "matrix/right-other/r0c0";
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    source_matrix.id = "left-other".into();
    source_matrix.part_ids = vec![source_id.into()];
    let mut mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .clone();
    mirror_matrix.id = "right-other".into();
    mirror_matrix.part_ids = vec![mirror_id.into()];
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
    document.matrices.extend([source_matrix, mirror_matrix]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("matrix", "left-other", "right-other")).unwrap(),
            serde_json::from_value(corr(
                "part",
                "matrix/left-other/r0c0",
                "matrix/right-other/r0c0",
            ))
            .unwrap(),
        ]);
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/new", "anchorPartId": "matrix/right-other/r0c0",
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 13
    }});

    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["id"],
        "left/new"
    );
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["anchorPartId"],
        "matrix/left-other/r0c0"
    );
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": -12.0, "y": 14.0}, {"x": -8.0, "y": 10.0}, {"x": -4.0, "y": 6.0}])
    );
}

#[test]
fn legacy_matrix_membership_without_board_id_uses_matrix_frame_rule() {
    let mut document = fixture();
    let source_id = "matrix/left-legacy/r0c0";
    let mirror_id = "matrix/right-legacy/r0c0";
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-keys")
        .unwrap()
        .clone();
    source_matrix.id = "left-legacy".into();
    source_matrix.board_id = None;
    source_matrix.part_ids = vec![source_id.into()];
    let mut mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "right-keys")
        .unwrap()
        .clone();
    mirror_matrix.id = "right-legacy".into();
    mirror_matrix.board_id = None;
    mirror_matrix.part_ids = vec![mirror_id.into()];
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
    document.matrices.extend([source_matrix, mirror_matrix]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("matrix", "left-legacy", "right-legacy")).unwrap(),
            serde_json::from_value(corr("part", source_id, mirror_id)).unwrap(),
        ]);

    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"add-feature": {"feature": {"kind": "polygon",
            "id": "right/legacy-anchor", "anchorPartId": mirror_id,
            "points": [{"x": 4, "y": 6}, {"x": 8, "y": 10}, {"x": 12, "y": 14}],
            "operation": "add"}}},
        "seed": 18
    }});
    let translated = canonical(&document, &request).expect("the linked anchor translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_eq!(
        value["edit"]["change"]["add-feature"]["feature"]["points"],
        json!([{"x": -12.0, "y": 14.0}, {"x": -8.0, "y": 10.0}, {"x": -4.0, "y": 6.0}])
    );
}

#[test]
fn current_projected_review_keeps_exact_source_coordinates_while_stale_review_stays_stale() {
    let mut document = rotated_fixture();
    document.boards[0].outline_ids.push("left/boundary".into());
    document.boards[1].outline_ids.push("right/boundary".into());
    let source_points = [
        Vec2 {
            x: 1.23456789,
            y: -3.45678901,
        },
        Vec2 {
            x: 7.65432109,
            y: 8.76543210,
        },
        Vec2 {
            x: -9.87654321,
            y: 2.34567891,
        },
    ];
    let mirror_points: Vec<_> = source_points
        .iter()
        .rev()
        .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
        .collect();
    document.outline.extend([
        serde_json::from_value(json!({"kind": "polygon", "id": "left/boundary",
            "points": source_points, "operation": "add"}))
        .unwrap(),
        serde_json::from_value(json!({"kind": "polygon", "id": "right/boundary",
            "points": mirror_points, "operation": "add"}))
        .unwrap(),
    ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("outline", "left/boundary", "right/boundary")).unwrap());
    let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);
    let source_review =
        crate::geometry::generated_source(&document, &cache, &document.boards[0]).source;
    let mirror_review =
        crate::geometry::generated_source(&document, &cache, &document.boards[1]).source;
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"}, "change": {"add-feature": {"feature": {
            "kind": "polygon", "id": "right/added", "points": [
                {"x": 8, "y": 9}, {"x": 10, "y": 12}, {"x": 14, "y": 9}], "operation": "add"}}},
        "seed": 11, "reviewed": mirror_review
    }});
    let translated = canonical(&document, &request).unwrap();
    let EditOperation::ApplyOutlineEdit { edit } = translated.operation else {
        panic!("the operation remains a typed outline edit")
    };
    assert_eq!(
        edit.reviewed.as_ref().unwrap(),
        &source_review,
        "inverse mapping preserves the exact current source review"
    );
    assert_eq!(
        crate::outline_edit::evaluate(&document, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Changed
    );

    let mut relevant_change = document.clone();
    let Some(crate::model::OutlineFeature::Polygon { points, .. }) = relevant_change
        .outline
        .iter_mut()
        .find(|feature| feature.id() == "left/boundary")
    else {
        panic!("the reviewed source feature remains present")
    };
    points[0].x += 1.0;
    assert_eq!(
        crate::outline_edit::evaluate(&relevant_change, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Retire,
        "a changed reviewed source retires the edit"
    );

    let mut unrelated_change = document.clone();
    unrelated_change.boards[1].name.push_str(" (renamed)");
    assert_eq!(
        crate::outline_edit::evaluate(&unrelated_change, &edit).unwrap(),
        crate::outline_edit::OutlineEditDisposition::Changed,
        "an unrelated board rename preserves the review"
    );

    let mut stale = request;
    stale["edit"]["reviewed"][0]["points"][0]["x"] = json!(12345.0);
    let translated = canonical(&document, &stale).unwrap();
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_ne!(
        value["edit"]["reviewed"],
        serde_json::to_value(source_review).unwrap()
    );
}

#[test]
fn an_omitted_mirror_target_is_refused_not_recreated() {
    let message = refusal(
        &fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/GONE", "at": {"x": 1, "y": 1}}]}),
    );
    assert!(
        message.contains("right/GONE") && message.contains("omitted"),
        "{message}"
    );
}

#[test]
fn a_mirror_side_addition_maps_a_linked_layout_to_the_source_layout() {
    let request = json!({"kind": "add-part", "boardId": "right", "layoutId": "right-layout", "part": {
        "id": "right/LED", "definitionId": "u", "reference": "right-D9",
        "pose": {"at": {"x": 230, "y": 12}, "rotation": 180}, "side": "front"}});
    let result = canonical(&native_layout_fixture(), &request).unwrap();
    let EditOperation::AddPart {
        part,
        board_id,
        layout_id,
        ..
    } = result.operation
    else {
        panic!("the source-side addition remains AddPart")
    };
    assert_eq!(board_id.as_deref(), Some("left"));
    assert_eq!(part.id, "left/LED");
    assert_eq!(layout_id.as_deref(), Some("left-layout"));
    assert_eq!(result.additions[0].mirror_layout_id, None);
}

#[test]
fn a_mirror_side_addition_whose_source_identity_is_taken_allocates_the_next_free_one() {
    let request = json!({"kind": "add-part", "boardId": "right", "part": {
        "id": "right/U1-new", "definitionId": "u", "reference": "RU9",
        "pose": {"at": {"x": 210, "y": 1}, "rotation": 0}, "side": "front"}});
    let mut taken = fixture();
    taken
        .parts
        .push(serde_json::from_value(part("left/U1-new", "u", "X1", 0.0, 0.0, 0.0)).unwrap());
    let result = canonical(&taken, &request).unwrap();
    let EditOperation::AddPart { part, board_id, .. } = result.operation else {
        panic!("an addition is expected");
    };
    assert_eq!(board_id.as_deref(), Some("left"));
    assert_eq!(part.id, "left/U1-new-2");
    assert_eq!(result.additions[0].source_id, "left/U1-new-2");
    assert_eq!(result.additions[0].mirror_id, "right/U1-new");
}

#[test]
fn a_literal_mirror_companion_moves_its_source_to_the_mirrored_point() {
    expect_operation(
        &fixture(),
        move_diode((190.0, -5.0)),
        moved_source_diode((10.0, -5.0)),
    );
}

#[test]
fn a_tweaked_companion_removes_the_tweak_turned_by_the_mirror_key_heading() {
    let mut document = fixture();
    set_companion(
        &mut document,
        "literal-mirror-with-tweak",
        json!({"x": 1.0, "y": 2.0, "rotation": 0.0}),
    );
    // Mirror key heading 0: the tweak (1, 2) is removed as it stands.
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((11.0, -7.0)),
    );
    // Mirror key heading 90: the tweak is (-2, 1) in the world.
    pose_of(&mut document, "matrix/right-keys/r0c0").rotation = 90.0;
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((8.0, -6.0)),
    );
}

#[test]
fn a_duplicate_position_companion_keeps_the_mirror_offset_in_the_source_key_frame() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    // Offset from the mirror key (200, 0) is (-10, -5); the source key (0, 0) heads 0.
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((-10.0, -5.0)),
    );
    // A source key heading 90 turns that offset to (5, -10).
    pose_of(&mut document, "matrix/left-keys/r0c0").rotation = 90.0;
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((5.0, -10.0)),
    );
}

#[test]
fn a_mirror_matrix_payload_becomes_the_source_matrix_through_the_inverse_pair_map() {
    let matrix = translated_matrix(
        &fixture(),
        mirror_payload(json!({
        "origin": {"x": 190, "y": 5}, "rotation": 10,
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "offset": {"x": 1, "y": 2}, "rotation": 5,
             "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -3}, "rotation": -80}]},
            {"row": 0, "column": 1, "enabled": false}
        ]})),
    );
    // Source identity, name, board and members are the source's own; geometry is inverted.
    assert_eq!(matrix["id"], "left-keys");
    assert_eq!(matrix["name"], "Left keys");
    assert_eq!(matrix["boardId"], "left");
    assert_eq!(
        matrix["partIds"],
        json!([
            "matrix/left-keys/r0c0",
            "matrix/left-keys/r0c0/diode",
            "matrix/left-keys/r0c1"
        ])
    );
    assert_close(
        &matrix,
        &json!({
        "id": "left-keys", "name": "Left keys", "rows": 1, "columns": 2,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 10, "y": 5}, "definitionId": "sw",
        "boardId": "left", "rotation": -10,
        "partIds": [
            "matrix/left-keys/r0c0", "matrix/left-keys/r0c0/diode", "matrix/left-keys/r0c1"],
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "offset": {"x": 1, "y": 2}, "rotation": 5,
             "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -3}, "rotation": 80}]},
            {"row": 0, "column": 1, "enabled": false}
        ]}),
        "matrix",
    );
}

#[test]
fn a_duplicate_position_companion_keeps_the_mirror_values_as_the_source_values() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    let matrix = companion_matrix(&document, "d", -60.0);
    assert_close(
        &matrix["cells"][0]["assemblies"],
        &json!([{"id": "diode", "definitionId": "d", "offset": {"x": -8, "y": -3}, "rotation": -60}]),
        "assemblies",
    );
}

#[test]
fn a_matrix_added_on_the_mirror_board_becomes_a_source_matrix_with_a_planned_pair() {
    let result = canonical(
        &fixture(),
        &new_matrix("right-thumbs", "right", (190.0, 40.0)),
    )
    .unwrap();
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_close(
        &value["matrix"],
        &json!({
            "id": "left-thumbs", "name": "Thumbs", "rows": 1, "columns": 2,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 10, "y": 40}, "definitionId": "sw",
            "boardId": "left", "partIds": [],
            "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": [
                {"id": "diode", "definitionId": "d", "offset": {"x": 8, "y": -2},
                 "rotation": 90}]}]}),
        "matrix",
    );
    let planned = addition(&result);
    assert_eq!(planned.source_id, "left-thumbs");
    assert_eq!(planned.mirror_id, "right-thumbs");
    assert_eq!(planned.source_board_id, "left");
    assert_eq!(planned.mirror_board_id, "right");
    // The cell with no stored entry is an enabled default cell, so it is a member too.
    let pair = |source: &str, mirror: &str| (source.to_owned(), mirror.to_owned());
    assert_eq!(
        planned.members,
        vec![
            pair("matrix/left-thumbs/r0c0", "matrix/right-thumbs/r0c0"),
            pair(
                "matrix/left-thumbs/r0c0/diode",
                "matrix/right-thumbs/r0c0/diode"
            ),
            pair("matrix/left-thumbs/r0c1", "matrix/right-thumbs/r0c1"),
        ]
    );
}

#[test]
fn a_matrix_added_on_the_source_board_runs_as_written_and_plans_its_mirror() {
    let request = new_matrix("left-thumbs", "left", (10.0, 40.0));
    let result = canonical(&fixture(), &request).unwrap();
    assert_close(
        &serde_json::to_value(&result.operation).unwrap(),
        &request,
        "operation",
    );
    let planned = addition(&result);
    assert_eq!(planned.source_id, "left-thumbs");
    assert_eq!(planned.mirror_id, "right-thumbs");
    assert_eq!(planned.members.len(), 3);
    assert_eq!(planned.members[1].1, "matrix/right-thumbs/r0c0/diode");
}

#[test]
fn matrix_definitions_travel_with_the_translated_request_unchanged() {
    let definition =
        json!({"id": "extra", "name": "Extra", "kind": "passive", "courtyard": [], "pads": []});
    let result = canonical(
        &fixture(),
        &json!({"kind": "set-matrix", "matrix": mirror_payload(json!({})),
                "definitions": [definition]}),
    )
    .unwrap();
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["definitions"], json!([definition]));
}

#[test]
fn selected_cell_requests_move_to_the_source_key_and_union_both_halves() {
    for kind in ["set-matrix-cells-enabled", "remove-selected-matrix-cells"] {
        let build = |cells: Value| {
            let mut request = json!({"kind": kind, "cells": cells});
            if kind == "set-matrix-cells-enabled" {
                request["enabled"] = json!(false);
            }
            request
        };
        // The mirror's list is kept as written when nothing else names the link.
        expect_operation(
            &fixture(),
            build(json!({"right-keys": [[0, 1], [0, 0]], "elsewhere": [[2, 2]]})),
            build(json!({"left-keys": [[0, 1], [0, 0]], "elsewhere": [[2, 2]]})),
        );
        // Both halves of one link are one set of coordinates, ordered and without repeats.
        expect_operation(
            &fixture(),
            build(json!({"right-keys": [[0, 1]], "left-keys": [[0, 1], [0, 0]]})),
            build(json!({"left-keys": [[0, 0], [0, 1]]})),
        );
    }
}

#[test]
fn source_and_local_matrix_requests_execute_as_written() {
    let request = json!({"kind": "remove-matrix-row", "matrixId": "left-keys", "row": 0});
    expect_operation(&fixture(), request.clone(), request);
    let local = json!({"kind": "remove-matrix", "id": "right-keys"});
    expect_operation(&with_matrix_state(fixture(), "local"), local.clone(), local);
}
