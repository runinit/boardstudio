use super::*;

#[test]
fn a_new_framed_part_endpoint_naming_a_linked_mirror_normalizes_to_its_binding() {
    let document = rotated_fixture();
    let constraint = Constraint::Framed {
        id: "constraint/framed-command".into(),
        source: crate::model::ConstraintEndpoint::Part {
            part_id: "right/U1".into(),
        },
        target: crate::model::ConstraintEndpoint::Part {
            part_id: "left/U2".into(),
        },
        relation: crate::model::ConstraintRelation::Offset {
            offset: Vec2 { x: 3.0, y: -8.0 },
            rotation: 17.0,
        },
    };

    let canonical = canonicalize(&document, &EditOperation::SetConstraint { constraint })
        .expect("a new Part endpoint naming a linked mirror is normalized");

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &canonical.prepared_constraints[0]
    else {
        panic!("the Framed command compiles into one prepared record");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/framed-command".into(),
            source: crate::constraints::ConstraintEndpoint::LinkedMirror {
                pair_id: "pair".into(),
                source_part_id: "left/U1".into(),
                mirror_part_id: "right/U1".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "left/U2".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: 3.0, y: -8.0 },
                rotation: 17.0,
            },
        }
    );
}

#[test]
fn local_constraint_edit_stays_local_and_remove_keeps_the_exact_global_id() {
    let document = rotated_fixture();
    let constraint = Constraint::Offset {
        id: "constraint/local-3".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: -4.0, y: 9.0 },
        rotation: -0.0,
    };
    let prepared = canonicalize(
        &document,
        &EditOperation::SetConstraint {
            constraint: constraint.clone(),
        },
    )
    .unwrap();
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the local SetConstraint must prepare its record");
    };
    assert_eq!(
        record,
        &crate::constraints::FramedConstraint::Framed {
            id: "constraint/local-3".into(),
            source: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            target: crate::constraints::ConstraintEndpoint::Part {
                part_id: "right/HEADER".into(),
            },
            relation: crate::constraints::ConstraintRelation::Offset {
                offset: Vec2 { x: -4.0, y: 9.0 },
                rotation: -0.0,
            },
        }
    );

    let removed = canonicalize(
        &document,
        &EditOperation::RemoveConstraint {
            id: "constraint/authored-on-mirror".into(),
        },
    )
    .unwrap();
    assert_eq!(
        removed.operation,
        EditOperation::RemoveConstraint {
            id: "constraint/authored-on-mirror".into(),
        }
    );
    assert_eq!(
        removed.prepared_constraints,
        [super::constraints::PreparedConstraintEdit::Remove {
            id: "constraint/authored-on-mirror".into(),
        }]
    );
}

#[test]
fn convert_offset_prepares_the_same_part_axes_values_as_the_legacy_operation() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Offset {
        id: "constraint/convert-axes".into(),
        source_part_id: "right/U1".into(),
        target_part_id: "right/HEADER".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: 15.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::ConvertOffsetConstraint {
            constraint_id: "constraint/convert-axes".into(),
            to: crate::model::OffsetAxes::Part,
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("ConvertOffsetConstraint must prepare its converted relation");
    };
    let crate::constraints::FramedConstraint::Framed {
        id,
        source,
        target,
        relation: crate::constraints::ConstraintRelation::PartOffset { offset, rotation },
    } = record
    else {
        panic!("the converted relation must remain a part-offset");
    };
    assert_eq!(id, "constraint/convert-axes");
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
        &crate::constraints::ConstraintEndpoint::Part {
            part_id: "right/HEADER".into(),
        }
    );
    assert_eq!(offset.x, -5.0);
    assert!((offset.y - 2.0).abs() < 1e-12, "{offset:?}");
    assert_eq!(rotation.to_bits(), 15.0_f64.to_bits());
}

#[test]
fn moving_source_and_target_in_one_batch_retunes_by_target_delta_not_final_fit() {
    let mut document = rotated_fixture();
    document.constraints.push(Constraint::Offset {
        id: "constraint/batch".into(),
        source_part_id: "right/HEADER".into(),
        target_part_id: "right/FREE".into(),
        offset: Vec2 { x: 5.0, y: -2.0 },
        rotation: -0.0,
    });
    let prepared = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![
                crate::model::Position {
                    id: "right/FREE".into(),
                    at: Vec2 { x: 302.0, y: 4.0 },
                },
                crate::model::Position {
                    id: "right/HEADER".into(),
                    at: Vec2 { x: 225.0, y: 31.0 },
                },
            ],
        },
    )
    .unwrap();

    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &prepared.prepared_constraints[0]
    else {
        panic!("the batch target move must prepare one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the batch must retain Offset");
    };
    // Existing MoveParts semantics add only target's requested delta (2, 4). A fit against the
    // moved source would replace this value with the target's final source-relative position.
    assert_eq!(*offset, Vec2 { x: 7.0, y: 2.0 });
    assert_eq!(rotation.to_bits(), (-0.0_f64).to_bits());

    let reversed = canonicalize(
        &document,
        &EditOperation::MoveParts {
            positions: vec![
                crate::model::Position {
                    id: "right/HEADER".into(),
                    at: Vec2 { x: 225.0, y: 31.0 },
                },
                crate::model::Position {
                    id: "right/FREE".into(),
                    at: Vec2 { x: 302.0, y: 4.0 },
                },
            ],
        },
    )
    .unwrap();
    let super::constraints::PreparedConstraintEdit::Upsert(record) =
        &reversed.prepared_constraints[0]
    else {
        panic!("the reordered batch must retain one retuned record");
    };
    let crate::constraints::FramedConstraint::Framed {
        relation: crate::constraints::ConstraintRelation::Offset { offset, rotation },
        ..
    } = record
    else {
        panic!("the reordered batch must retain Offset");
    };
    assert_eq!(*offset, Vec2 { x: 7.0, y: 2.0 });
    assert_eq!(rotation.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn moving_a_linked_key_and_companion_in_either_order_keeps_the_owner_intent() {
    let mut document = rotated_fixture();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = Some("left".into());
    document.constraints.push(Constraint::Offset {
        id: "constraint/batched-companion".into(),
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
            panic!("the linked key and companion moves must remain in the canonical batch");
        };
        assert_eq!(positions.len(), 2);
        let source_key = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0")
            .unwrap();
        let source_companion = positions
            .iter()
            .find(|position| position.id == "matrix/left-keys/r0c0/diode")
            .unwrap();
        // The independently worked inverse of the 90-degree map is
        // (-30 - y, 100 - x): key (-25, -90), companion (-23, -98).
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
            (source_companion.at.x + 23.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );
        assert!(
            (source_companion.at.y + 98.0).abs() < 1e-12,
            "{:?}",
            source_companion.at
        );

        let [super::constraints::PreparedConstraintEdit::DeferredTargetRetune(intent)] =
            prepared.prepared_constraints.as_slice()
        else {
            panic!("the companion stays deferred until its key owner is resolved");
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
fn pair_creation_mechanical_lifecycle_half_creation_and_replace_are_passed_through() {
    let document = fixture();
    let operations = [
        (
            op(json!({"kind":"create-mirrored-pair",
                "left":{"id":"left-layout","name":"Left","boardId":"left",
                    "matrixIds": ["left-matrix"],"partIds":[]},
                "right":{"id":"right-layout","name":"Right","boardId":"left",
                    "matrixIds": ["right-matrix"],"partIds":[],
                    "mirrorLink":{"sourceId":"left-layout","axisX":100.0}},
                "matrix":{"id":"left-matrix","rows":0,"columns":0,
                    "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
                    "definitionId":"switch","partIds":[],"boardId":"left"}})),
            Coverage::Delegated(
                "CreateMirroredPair creates both linked layouts in one Core operation",
            ),
        ),
        (
            op(
                json!({"kind":"apply-mechanical-edit","edit":{"target":{"kind":"instance",
                "instanceId":"left"},"change":"enable"}}),
            ),
            Coverage::Delegated(
                "the typed mechanical lifecycle already edits both halves of a pair; inverting it again would apply it twice",
            ),
        ),
        (
            op(json!({"kind":"create-mirrored-half","pairId":"new-pair",
                "sourceInstanceId":"left",
                "mode":{"kind":"new-board","boardId":"right-new","instanceId":"right-instance"},
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "idRule":"board-prefix","companionMode":"literal-mirror"})),
            Coverage::Delegated(
                "CreateMirroredHalf derives both halves through the Core mirror lifecycle",
            ),
        ),
        (
            EditOperation::ReplaceDocument {
                document: Box::new(ProjectDoc::empty("replacement", "Replacement")),
            },
            Coverage::Delegated(
                "ticket 116 owns source-authoritative Open and ReplaceDocument repair",
            ),
        ),
    ];
    for (operation, expected_coverage) in operations {
        let result = canonicalize(&document, &operation).unwrap();
        assert_eq!(result.coverage, expected_coverage);
        assert_eq!(result.operation, operation);
    }
}

#[test]
fn set_layout_native_mirror_reorders_projected_local_and_linked_members() {
    let document = native_layout_fixture();
    let mut requested = document.layouts[1].clone();
    requested.part_ids = vec![
        "right/HEADER".into(),
        "right/U1".into(),
        "right/FREE".into(),
    ];
    let result = canonicalize(&document, &EditOperation::SetLayout { layout: requested })
        .expect("a projected-member reorder can be expressed by source order");
    let EditOperation::SetLayout { layout } = result.operation else {
        panic!("SetLayout remains a SetLayout")
    };
    assert_eq!(layout.id, "left-layout");
    assert_eq!(layout.part_ids, ["left/HEADER", "left/U1", "left/GONE"]);
}

#[test]
fn native_mirror_target_owned_name_and_membership_edits_pass_through_deliberately() {
    let document = native_layout_fixture();
    let cases = [
        (document.clone(), {
            let mut layout = document.layouts[1].clone();
            layout.name = "Mirror-specific name".into();
            layout
        }),
        (document.clone(), {
            let mut layout = document.layouts[1].clone();
            layout.part_ids.retain(|id| id != "right/FREE");
            layout
        }),
        (
            {
                let mut document = document.clone();
                document.layouts[1].part_ids.retain(|id| id != "right/FREE");
                document
            },
            {
                let mut layout = document.layouts[1].clone();
                layout.part_ids.retain(|id| id != "right/FREE");
                layout.part_ids.push("right/FREE".into());
                layout
            },
        ),
    ];
    for (document, layout) in cases {
        let request = EditOperation::SetLayout { layout };
        let result = canonicalize(&document, &request).expect("target-owned edit passes through");
        assert_eq!(result.operation, request);
        assert!(matches!(
            result.coverage,
            Coverage::DeliberatelyUnchanged(_)
        ));
    }
}

#[test]
fn set_outline_on_a_linked_mirror_maps_feature_geometry_and_identity_back_to_source() {
    let mut document = fixture();
    document.boards[0].outline_ids.push("left/cutout".into());
    document.boards[1].outline_ids.push("right/cutout".into());
    document.outline.extend([
        serde_json::from_value(
            json!({"kind": "polygon", "id": "left/cutout", "anchorPartId": null,
            "points": [{"x": 190, "y": 4}, {"x": 195, "y": 4}, {"x": 195, "y": 8}],
            "operation": "subtract"}),
        )
        .unwrap(),
        serde_json::from_value(
            json!({"kind": "polygon", "id": "right/cutout", "anchorPartId": null,
            "points": [{"x": 5, "y": 8}, {"x": 5, "y": 4}, {"x": 10, "y": 4}],
            "operation": "subtract"}),
        )
        .unwrap(),
    ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(serde_json::from_value(corr("outline", "left/cutout", "right/cutout")).unwrap());
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"set-feature": {"feature": {"kind": "polygon", "id": "right/cutout",
            "anchorPartId": null, "points": [{"x": 3, "y": 9}, {"x": 3, "y": 6}, {"x": 7, "y": 6}],
            "operation": "subtract"}}},
        "seed": 5, "reviewed": null}});
    let translated =
        canonical(&document, &request).expect("a linked mirror outline edit translates");
    assert_eq!(translated.coverage, Coverage::Translated);
    assert_close(
        &serde_json::to_value(translated.operation).unwrap(),
        &json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "board", "boardId": "left"},
            "change": {"set-feature": {"feature": {"kind": "polygon", "id": "left/cutout",
                "points": [{"x": 193, "y": 6}, {"x": 197, "y": 6}, {"x": 197, "y": 9}],
                "operation": "subtract"}}},
            "seed": 5}}),
        "operation",
    );
}

#[test]
fn local_outline_versions_stay_local_while_omitted_versions_refuse_edits() {
    let mut document = fixture();
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend([
            serde_json::from_value(corr("outline-version", "left-local", "right-local")).unwrap(),
            serde_json::from_value(corr("outline-version", "left-omitted", "right-omitted"))
                .unwrap(),
        ]);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .extend([
            serde_json::from_value(
                json!({"kind": "outline-version", "mirrorId": "right-local", "state": "local"}),
            )
            .unwrap(),
            serde_json::from_value(
                json!({"kind": "outline-version", "mirrorId": "right-omitted", "state": "omitted"}),
            )
            .unwrap(),
        ]);
    let local = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-local"},
        "change": "activate", "seed": 2
    }});
    expect_operation(&document, local.clone(), local);
    let omitted = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "version", "boardId": "right", "versionId": "right-omitted"},
        "change": "activate", "seed": 2
    }});
    assert!(refusal(&document, omitted).contains("omitted"));
}

#[test]
fn a_forward_projected_reviewed_contour_returns_exactly_to_its_source_shape() {
    let document = rotated_fixture();
    let source = vec![
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
    let mirror: Vec<_> = source
        .iter()
        .rev()
        .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
        .collect();
    let request = json!({"kind": "apply-outline-edit", "edit": {
        "target": {"kind": "board", "boardId": "right"},
        "change": {"edit-generated-perimeter": {"contour": 0, "points": mirror}},
        "seed": 9,
        "reviewed": [{"points": mirror, "hole": false}]
    }});
    let translated = canonical(&document, &request).expect("the reviewed contour translates");
    let value = serde_json::to_value(translated.operation).unwrap();
    assert_close(
        &value["edit"]["reviewed"][0]["points"],
        &serde_json::to_value(source).unwrap(),
        "reviewed round trip",
    );
}

#[test]
fn apply_outline_edit_translates_fixed_and_linked_version_reviews() {
    let mut document = rotated_fixture();
    let mut versions = Vec::new();
    let mut source_versions = Vec::new();
    let mut mirror_versions = Vec::new();
    for (ownership, source_points) in [
        (
            "fixed",
            [
                Vec2 { x: 1.25, y: -3.5 },
                Vec2 { x: 7.75, y: 8.5 },
                Vec2 { x: -9.5, y: 2.25 },
            ],
        ),
        (
            "linked",
            [
                Vec2 { x: 21.5, y: 3.25 },
                Vec2 { x: 29.75, y: 9.0 },
                Vec2 { x: 16.25, y: 12.5 },
            ],
        ),
    ] {
        let mirror_points: Vec<_> = source_points
            .iter()
            .rev()
            .map(|point| crate::mechanical::pair::forward(ROTATED, *point))
            .collect();
        let source_id = format!("left-version-{ownership}");
        let mirror_id = format!("right-version-{ownership}");
        let source_feature_id = format!("left/{ownership}-outline");
        let mirror_feature_id = format!("right/{ownership}-outline");
        document.boards[0]
            .outline_ids
            .push(source_feature_id.clone());
        document.boards[1]
            .outline_ids
            .push(mirror_feature_id.clone());
        let source_feature = json!({"kind": "polygon", "id": source_feature_id,
            "points": source_points, "operation": "add"});
        let mirror_feature = json!({"kind": "polygon", "id": mirror_feature_id,
            "points": mirror_points, "operation": "add"});
        document.outline.extend(
            [source_feature.clone(), mirror_feature.clone()]
                .map(|feature| serde_json::from_value(feature).unwrap()),
        );
        versions.push((source_id.clone(), mirror_id.clone()));
        source_versions.push(json!({"id": source_id, "name": ownership,
            "ownership": ownership, "source": {"revision": 0},
            "geometry": {"features": [source_feature], "settings": {}, "expectedRegions": 1}}));
        mirror_versions.push(json!({"id": mirror_id, "name": ownership,
            "ownership": ownership, "source": {"revision": 0},
            "geometry": {"features": [mirror_feature], "settings": {}, "expectedRegions": 1}}));
    }
    document.board_outlines = vec![
        serde_json::from_value(
            json!({"boardId": "left", "activeVersionId": "left-version-fixed",
            "versions": source_versions}),
        )
        .unwrap(),
        serde_json::from_value(
            json!({"boardId": "right", "activeVersionId": "right-version-fixed",
            "versions": mirror_versions}),
        )
        .unwrap(),
    ];
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .extend(versions.iter().map(|(source, mirror)| {
            serde_json::from_value(corr("outline-version", source, mirror)).unwrap()
        }));
    let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);

    for (ownership, (source_version, mirror_version)) in
        ["fixed", "linked"].into_iter().zip(versions)
    {
        let source_geometry = &document
            .board_outlines
            .iter()
            .find(|state| state.board_id == "left")
            .unwrap()
            .versions
            .iter()
            .find(|version| version.id == source_version)
            .unwrap()
            .geometry;
        let mirror_geometry = &document
            .board_outlines
            .iter()
            .find(|state| state.board_id == "right")
            .unwrap()
            .versions
            .iter()
            .find(|version| version.id == mirror_version)
            .unwrap()
            .geometry;
        let source_review =
            crate::geometry::feature_source_contours(&document, &cache, &source_geometry.features);
        let mirror_review =
            crate::geometry::feature_source_contours(&document, &cache, &mirror_geometry.features);
        let request = json!({"kind": "apply-outline-edit", "edit": {
            "target": {"kind": "version", "boardId": "right", "versionId": mirror_version},
            "change": {"set-margin": {"margin": 3.0}}, "seed": 5,
            "reviewed": mirror_review
        }});
        let translated = canonical(&document, &request)
            .unwrap_or_else(|error| panic!("the {ownership} version edit translates: {error}"));
        let value = serde_json::to_value(translated.operation).unwrap();
        assert_eq!(value["edit"]["target"]["boardId"], "left");
        assert_eq!(value["edit"]["target"]["versionId"], source_version);
        assert_close(
            &value["edit"]["reviewed"],
            &serde_json::to_value(&source_review).unwrap(),
            &format!("{ownership} version review"),
        );

        let mut stale = request;
        stale["edit"]["reviewed"][0]["points"][0]["x"] = json!(54321.0);
        let translated = canonical(&document, &stale).unwrap();
        let value = serde_json::to_value(translated.operation).unwrap();
        assert_ne!(
            value["edit"]["reviewed"],
            serde_json::to_value(&source_review).unwrap()
        );
    }
}

#[test]
fn moving_a_linked_independent_part_moves_its_source_through_the_inverse_pair_map() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
        json!({"kind": "move-parts", "positions": [{"id": "left/U1", "at": {"x": -30, "y": 7}}]}),
    );
    // Rotated map: mirror (90, -50) is the image of source (20, 10).
    expect_operation(
        &rotated_fixture(),
        json!({"kind": "move-parts", "positions": [{"id": "right/U1", "at": {"x": 90, "y": -50}}]}),
        json!({"kind": "move-parts", "positions": [{"id": "left/U1", "at": {"x": 20, "y": 10}}]}),
    );
}

#[test]
fn moving_a_linked_key_switch_moves_the_source_key() {
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/right-keys/r0c1", "at": {"x": 175, "y": 6}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "matrix/left-keys/r0c1", "at": {"x": 25, "y": 6}}]}),
    );
}

#[test]
fn naming_both_halves_of_one_link_is_one_move_when_they_agree_and_a_refusal_when_they_do_not() {
    // Source (-30, 7) is the mirror's (230, 7): one canonical value, so one entry.
    expect_operation(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -30, "y": 7}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -30, "y": 7}}]}),
    );
    let message = refusal(
        &fixture(),
        json!({"kind": "move-parts", "positions": [
            {"id": "left/U1", "at": {"x": -20, "y": 0}},
            {"id": "right/U1", "at": {"x": 230, "y": 7}}]}),
    );
    assert!(
        message.contains("left/U1") && message.contains("right/U1"),
        "{message}"
    );
}

#[test]
fn a_source_side_addition_plans_its_mirror_without_changing_the_authored_request() {
    let source = json!({"kind": "add-part", "boardId": "left", "part": {
        "id": "left/LED", "definitionId": "u", "reference": "D9",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 30}, "side": "front"}});
    let result = canonical(&fixture(), &source).unwrap();
    let operation = serde_json::to_value(result.operation).unwrap();
    assert_close(&operation, &source, "operation");
    assert_eq!(result.additions.len(), 1);
    assert_eq!(result.additions[0].source_id, "left/LED");
    assert_eq!(result.additions[0].mirror_id, "right/LED");
}

#[test]
fn removing_linked_mirror_parts_removes_their_sources_once() {
    expect_operation(
        &fixture(),
        json!({"kind": "remove-parts", "ids": [
            "right/U1", "right/FREE", "left/U1", "right/HEADER",
            "matrix/right-keys/r0c0/diode"]}),
        json!({"kind": "remove-parts", "ids": [
            "left/U1", "right/FREE", "right/HEADER", "matrix/left-keys/r0c0/diode"]}),
    );
}

#[test]
fn a_stabiliser_keeps_the_literal_position_in_every_mode() {
    let mut document = fixture();
    set_companion(&mut document, "duplicate-position", json!(null));
    document
        .parts
        .iter_mut()
        .find(|part| part.id == DIODE)
        .unwrap()
        .definition_id = "kicad:marbastlib/STAB_MX_2u".into();
    expect_operation(
        &document,
        move_diode((190.0, -5.0)),
        moved_source_diode((10.0, -5.0)),
    );
}

#[test]
fn a_rotated_pair_map_turns_the_matrix_origin_and_heading_back_once() {
    let matrix = translated_matrix(
        &rotated_fixture(),
        mirror_payload(json!({"origin": {"x": 90, "y": -50}})),
    );
    // Mirror origin (90, -50) is the image of source (20, 10); no stored heading becomes the
    // map's 90 degrees on the source, and the mirror flag clears.
    assert_close(&matrix["origin"], &json!({"x": 20, "y": 10}), "origin");
    assert_close(&matrix["rotation"], &json!(90), "rotation");
    assert!(
        matrix.get("mirror").is_none(),
        "the source frame is not flipped"
    );
}

#[test]
fn a_stabiliser_recovers_its_source_turn_from_its_keys_world_heading() {
    let matrix = companion_matrix(&fixture(), "kicad:marbastlib/STAB_MX_2u", -60.0);
    // Cell 0: stored Key rotation copies unchanged; the source heading is +20, so -60 - 2(20) = -100.
    assert_close(
        &matrix["cells"][0]["assemblies"][0],
        &json!({"id": "diode", "definitionId": "kicad:marbastlib/STAB_MX_2u",
                "offset": {"x": 8, "y": -3}, "rotation": -100}),
        "cell 0",
    );
    // Cell 1: key heading is the column splay 15; the turn is 10 + 0 - 30 = -20.
    assert_close(
        &matrix["cells"][1]["assemblies"][0],
        &json!({"id": "diode", "definitionId": "kicad:marbastlib/STAB_MX_2u",
                "offset": {"x": 8, "y": -3}, "rotation": -20}),
        "cell 1",
    );
}

#[test]
fn a_source_and_a_local_mirror_matrix_execute_as_written() {
    let source = set_matrix(json!({
        "id": "left-keys", "name": "Left keys", "rows": 1, "columns": 1,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 3, "y": 3}, "definitionId": "sw",
        "boardId": "left", "partIds": []}));
    expect_operation(&fixture(), source.clone(), source);
    let local = set_matrix(mirror_payload(json!({"name": "Local rename"})));
    expect_operation(&with_matrix_state(fixture(), "local"), local.clone(), local);
}

#[test]
fn a_planned_matrix_identity_skips_ids_already_in_use() {
    let mut document = fixture();
    document.matrices.push(
        serde_json::from_value(json!({"id": "left-thumbs", "rows": 1, "columns": 1,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 90}, "definitionId": "sw",
            "boardId": "left", "partIds": []}))
        .unwrap(),
    );
    let result = canonical(
        &document,
        &new_matrix("right-thumbs", "right", (190.0, 40.0)),
    )
    .unwrap();
    assert_eq!(addition(&result).source_id, "left-thumbs-2");
    assert_eq!(addition(&result).mirror_id, "right-thumbs");
    let value = serde_json::to_value(&result.operation).unwrap();
    assert_eq!(value["matrix"]["id"], "left-thumbs-2");
}

#[test]
fn an_existing_unpaired_matrix_and_a_matrix_on_an_unpaired_board_are_not_additions() {
    let mut document = fixture();
    document.matrices.push(
        serde_json::from_value(json!({"id": "right-extra", "rows": 1, "columns": 1,
            "pitch": {"x": 19, "y": 19}, "origin": {"x": 300, "y": 0}, "definitionId": "sw",
            "boardId": "right", "partIds": []}))
        .unwrap(),
    );
    let existing = set_matrix(json!({"id": "right-extra", "rows": 1, "columns": 1,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 310, "y": 0}, "definitionId": "sw",
        "boardId": "right", "partIds": []}));
    let result = canonical(&document, &existing).unwrap();
    assert!(result.matrix_additions.is_empty());
    expect_operation(&document, existing.clone(), existing);
    let elsewhere = new_matrix("other-thumbs", "other-board", (0.0, 0.0));
    let result = canonical(&fixture(), &elsewhere).unwrap();
    assert!(result.matrix_additions.is_empty());
    expect_operation(&fixture(), elsewhere.clone(), elsewhere);
}

#[test]
fn matrix_structure_requests_on_the_mirror_name_the_source_matrix() {
    for (request, expected) in [
        (
            json!({"kind": "remove-matrix", "id": "right-keys"}),
            json!({"kind": "remove-matrix", "id": "left-keys"}),
        ),
        (
            json!({"kind": "remove-matrix-cells", "matrixId": "right-keys", "cells": [[0, 1]]}),
            json!({"kind": "remove-matrix-cells", "matrixId": "left-keys", "cells": [[0, 1]]}),
        ),
        (
            json!({"kind": "remove-matrix-row", "matrixId": "right-keys", "row": 0}),
            json!({"kind": "remove-matrix-row", "matrixId": "left-keys", "row": 0}),
        ),
        (
            json!({"kind": "remove-matrix-column", "matrixId": "right-keys", "column": 1}),
            json!({"kind": "remove-matrix-column", "matrixId": "left-keys", "column": 1}),
        ),
    ] {
        expect_operation(&fixture(), request, expected);
    }
}

#[test]
fn an_omitted_matrix_is_refused() {
    let message = refusal(
        &with_matrix_state(fixture(), "omitted"),
        json!({"kind": "remove-matrix", "id": "right-keys"}),
    );
    assert!(
        message.contains("right-keys") && message.contains("omitted"),
        "{message}"
    );
}
