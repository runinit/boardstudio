
#[test]
fn linked_outline_and_fixed_and_linked_versions_project_literal_anchored_payload() {
    let (mut document, projection) = outline_fixture();
    document.assemblies.push(
        serde_json::from_value(serde_json::json!({
            "id": "shared-assembly",
            "name": "Shared assembly",
            "members": []
        }))
        .unwrap(),
    );
    let assemblies_before = document.assemblies.clone();
    let pair_before = pair_state_bytes(&document);
    let changed = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid outline projection");

    let target = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .expect("mapped outline remains under its stable identity");
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        operation,
        ..
    } = target
    else {
        panic!("the target feature keeps its source variant")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-matrix/r0c1"));
    assert_eq!(
        points,
        &vec![point(-5.0, 6.0), point(-3.0, 4.0), point(-1.0, 2.0)]
    );
    assert_eq!(*operation, Operation::Subtract);

    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .expect("target board outline");
    assert_eq!(
        target_outline.active_version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert_eq!(target_outline.generated_last_valid, None);
    assert_eq!(
        target_outline.retired_version_ids,
        ["mirror-board/retired-version"]
    );
    assert_eq!(target_outline.versions.len(), 3);
    let linked = target_outline
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    let fixed = target_outline
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/fixed")
        .unwrap();
    assert_eq!(linked.ownership, OutlineOwnership::Linked);
    assert_eq!(fixed.ownership, OutlineOwnership::Fixed);
    assert_eq!(linked.source.revision, 17);
    assert_eq!(
        linked.source.version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert_eq!(linked.geometry.expected_regions, 3);
    assert_eq!(linked.geometry.features[0].id(), "mirror-board/feature");
    assert_eq!(
        linked.geometry.bridges[0].id,
        "mirror-board/source-board/bridge"
    );
    assert_eq!(linked.geometry.bridges[0].matrix_ids, ["mirror-matrix"]);
    assert_eq!(
        linked.geometry.bridges[0].points,
        vec![point(92.0, -37.0), point(90.0, -39.0)]
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].id,
        "mirror-board/source-board/snapshot-gap"
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].points,
        vec![
            control(88.0, -41.0, None),
            control(-13.0, 14.0, Some("matrix/mirror-matrix/r0c1")),
        ]
    );
    assert_eq!(
        linked.geometry.settings.repair.as_ref().unwrap().keep_gaps[0].id,
        "mirror-board/source-board/setting-gap"
    );
    assert!(changed.iter().any(|id| id == "mirror-board/feature"));
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        document.assemblies, assemblies_before,
        "shared definitions are reused"
    );

    let first_output = document.clone();
    let repeat = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("updated repeat");
    assert!(repeat.is_empty());
    assert_eq!(document, first_output);
}

#[test]
fn missing_target_outline_version_state_refuses_after_feature_projection() {
    let (mut document, projection) = outline_fixture();
    // The fixture's mirror feature differs from its source, so the projector writes
    // it to the draft before discovering that the target version state is absent.
    document
        .board_outlines
        .retain(|outline| outline.board_id != "mirror-board");
    let before = document.clone();
    let before_context = projection_state(&projection);

    let error = reconcile_candidate(&mut document, "pair-source-mirror", &projection).unwrap_err();

    assert_eq!(error, "The selected target board outline is unavailable");
    assert_eq!(document, before);
    assert_eq!(projection_state(&projection), before_context);
}

#[test]
fn a_cross_matrix_anchor_uses_its_own_matrix_frame_rule() {
    let (mut document, projection) = outline_fixture();
    let source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    let mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    let source_thumb_id = "source-thumb";
    let mirror_thumb_id = "mirror-thumb";
    let mut source_thumb = source_matrix.clone();
    source_thumb.id = source_thumb_id.into();
    source_thumb.name = Some("Source thumb".into());
    source_thumb.origin.x += 40.0;
    source_thumb.part_ids = source_matrix
        .part_ids
        .iter()
        .map(|id| id.replacen("matrix/source-matrix/", "matrix/source-thumb/", 1))
        .collect();
    let mut mirror_thumb = mirror_matrix.clone();
    mirror_thumb.id = mirror_thumb_id.into();
    mirror_thumb.name = Some("Mirror thumb".into());
    mirror_thumb.origin = crate::mechanical::pair::forward(projection.mapping, source_thumb.origin);
    mirror_thumb.part_ids = mirror_matrix
        .part_ids
        .iter()
        .map(|id| id.replacen("matrix/mirror-matrix/", "matrix/mirror-thumb/", 1))
        .collect();

    for (old_id, new_id) in source_matrix.part_ids.iter().zip(&source_thumb.part_ids) {
        let mut part = document
            .parts
            .iter()
            .find(|part| part.id == *old_id)
            .unwrap()
            .clone();
        part.id.clone_from(new_id);
        part.reference = format!("THUMB-{}", part.reference);
        document.parts.push(part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(new_id.clone());
    }
    for (old_id, new_id) in mirror_matrix.part_ids.iter().zip(&mirror_thumb.part_ids) {
        let mut part = document
            .parts
            .iter()
            .find(|part| part.id == *old_id)
            .unwrap()
            .clone();
        part.id.clone_from(new_id);
        part.reference = format!("THUMB-{}", part.reference);
        document.parts.push(part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(new_id.clone());
    }
    document.matrices.extend([source_thumb, mirror_thumb]);
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: source_thumb_id.into(),
        mirror_id: mirror_thumb_id.into(),
    });
    for (source_id, mirror_id) in source_matrix.part_ids.iter().zip(&mirror_matrix.part_ids) {
        halves.correspondences.push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_id.replacen("source-matrix", "source-thumb", 1),
            mirror_id: mirror_id.replacen("mirror-matrix", "mirror-thumb", 1),
        });
    }

    let source_feature = document
        .outline
        .iter_mut()
        .find(|feature| feature.id() == "source-board/feature")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = source_feature
    else {
        panic!("the source feature remains a polygon")
    };
    *anchor_part_id = Some("matrix/source-thumb/r0c0".into());
    *points = vec![point(2.0, 3.0), point(4.0, 5.0), point(6.0, 7.0)];
    let source_linked_version = document
        .board_outlines
        .iter_mut()
        .find(|state| state.board_id == "source-board")
        .unwrap()
        .versions
        .iter_mut()
        .find(|version| version.id == "source-board/version/linked")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = &mut source_linked_version.geometry.features[0]
    else {
        panic!("the source version feature remains a polygon")
    };
    *anchor_part_id = Some("matrix/source-thumb/r0c0".into());
    *points = vec![point(2.0, 3.0), point(4.0, 5.0), point(6.0, 7.0)];

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("the active base matrix and linked thumb matrix both remain valid");
    let mapped = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = mapped
    else {
        panic!("the projected feature remains a polygon")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-thumb/r0c0"));
    assert_eq!(
        points,
        &vec![point(-6.0, 7.0), point(-4.0, 5.0), point(-2.0, 3.0)]
    );
    let mirror_linked_version = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = &mirror_linked_version.geometry.features[0]
    else {
        panic!("the mirrored version feature remains a polygon")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-thumb/r0c0"));
    assert_eq!(
        points,
        &vec![point(-6.0, 7.0), point(-4.0, 5.0), point(-2.0, 3.0)]
    );
}

#[test]
fn outline_projection_never_copies_generated_case_closure_members() {
    let (mut document, projection) = outline_fixture();
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    let envelope = OutlineFeature::PartEnvelope {
        connections: vec![OutlineConnection {
            id: "closure-connection".into(),
            width: 1.25,
            points: vec![control(0.0, 0.0, None), control(2.0, 0.0, None)],
        }],
        settings: Default::default(),
        id: "source-board/envelope".into(),
        part_ids: vec![
            "matrix/source-matrix/r0c1".into(),
            "case-closure/generated-mount".into(),
        ],
        margin: 1.75,
        operation: Operation::Add,
    };
    source.versions[0].geometry.features.push(envelope);
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "source-board/envelope".into(),
        part_ids: vec![
            "matrix/source-matrix/r0c1".into(),
            "case-closure/generated-mount".into(),
        ],
        margin: 1.75,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .push("source-board/envelope".into());
    let mirror_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    mirror_board
        .outline_ids
        .push("mirror-board/envelope".into());
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "mirror-board/envelope".into(),
        part_ids: vec!["matrix/mirror-matrix/r0c1".into()],
        margin: 0.0,
        operation: Operation::Add,
    });
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        source_id: "source-board/envelope".into(),
        mirror_id: "mirror-board/envelope".into(),
    });

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source outline");
    let mirrored = &document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions[0]
        .geometry
        .features;
    let envelope = mirrored
        .iter()
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, part_ids, .. } if id == "mirror-board/envelope" => {
                Some(part_ids)
            }
            _ => None,
        })
        .expect("mapped source envelope");
    assert_eq!(envelope, &["matrix/mirror-matrix/r0c1"]);
    assert!(!envelope.iter().any(|id| id.starts_with("case-closure/")));
}

#[test]
fn outline_projection_retains_independent_target_envelope_members_stably() {
    let (mut document, projection) = outline_fixture();
    let make_part = |id: &str, reference: &str| {
        let mut part = document.parts[0].clone();
        part.id = id.into();
        part.reference = reference.into();
        part
    };
    let target_only = make_part("mirror-board/independent", "MI1");
    let local_source = make_part("source-board/local", "LS1");
    let local_target = make_part("mirror-board/local", "LT1");
    let hosted = make_part("mirror-board/hosted", "HOST1");
    let foreign = make_part("source-board/foreign", "FOREIGN1");
    let duplicate = make_part("mirror-board/duplicate", "DUP1");
    document.parts.extend([
        target_only.clone(),
        local_source.clone(),
        local_target.clone(),
        hosted.clone(),
        foreign.clone(),
        duplicate.clone(),
    ]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .extend([
            target_only.id.clone(),
            local_target.id.clone(),
            hosted.id.clone(),
            duplicate.id.clone(),
        ]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .extend([local_source.id.clone(), foreign.id.clone()]);
    document
        .embedded_circuits
        .push(crate::modules::EmbeddedCircuit {
            id: "mirror-board/embedded-circuit".into(),
            definition_id: "embedded-fixture".into(),
            host_board_id: "mirror-board".into(),
            part_ids: vec![hosted.id.clone()],
            net_ids: vec![],
            ports: Default::default(),
        });

    let source_envelope = OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "source-board/envelope".into(),
        part_ids: vec!["matrix/source-matrix/r0c1".into()],
        margin: 1.75,
        operation: Operation::Add,
    };
    document.outline.push(source_envelope.clone());
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "mirror-board/envelope".into(),
        part_ids: vec![
            "matrix/mirror-matrix/r0c1",
            target_only.id.as_str(),
            target_only.id.as_str(),
            local_target.id.as_str(),
            hosted.id.as_str(),
            foreign.id.as_str(),
            duplicate.id.as_str(),
            "stale/missing",
            "case-closure/generated",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        margin: 1.75,
        operation: Operation::Add,
    });
    for (board_id, outline_id) in [
        ("source-board", "source-board/envelope"),
        ("mirror-board", "mirror-board/envelope"),
    ] {
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .outline_ids
            .push(outline_id.into());
    }
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: local_source.id,
            mirror_id: local_target.id.clone(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Outline,
            source_id: "source-board/envelope".into(),
            mirror_id: "mirror-board/envelope".into(),
        },
    ]);
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: local_target.id.clone(),
        state: crate::model::MirroredExceptionState::Local,
        removed_copper: None,
    });

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("the independent target Part is a valid target envelope member");
    let target_envelope = || {
        document
            .outline
            .iter()
            .find_map(|feature| match feature {
                OutlineFeature::PartEnvelope { id, part_ids, .. }
                    if id == "mirror-board/envelope" =>
                {
                    Some(part_ids.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(
        target_envelope(),
        vec![
            "matrix/mirror-matrix/r0c1",
            target_only.id.as_str(),
            local_target.id.as_str(),
            duplicate.id.as_str(),
        ],
        "source members project first, then unique independent, Local, and duplicate-listed members; stale, foreign, generated, and embedded members are omitted"
    );
    let first = document.clone();
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("repeat projection retains target membership")
            .is_empty()
    );
    assert_eq!(document, first, "reconciliation is stable");
}

#[test]
fn outline_exception_is_targeted_and_unrelated_local_version_survives_projection() {
    let (mut document, projection) = outline_fixture();
    let before_local = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/local-version")
        .unwrap()
        .clone();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::OutlineVersion,
        removed_copper: None,
        mirror_id: "mirror-board/local-version".into(),
        state: crate::model::MirroredExceptionState::Local,
    });
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        removed_copper: None,
        mirror_id: "mirror-board/omitted-outline".into(),
        state: crate::model::MirroredExceptionState::Omitted,
    });

    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/omitted-outline".into());
    let before_fixed_local = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/local-version")
        .unwrap()
        .clone();
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid outline exceptions");
    let target = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target
            .versions
            .iter()
            .find(|version| version.id == "mirror-board/local-version")
            .unwrap(),
        &before_fixed_local,
        "Local keeps the complete authored target payload"
    );
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/omitted-outline"),
        "Omitted stays absent"
    );
    assert!(
        document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .contains(&"mirror-board/omitted-outline".to_owned()),
        "the exception identity remains registered for durable pair validity"
    );
    assert_eq!(
        target
            .versions
            .iter()
            .find(|version| version.id == "mirror-board/local-version")
            .unwrap(),
        &before_local,
        "unrelated target-owned choices remain unchanged"
    );
    let linked_feature = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .expect("other linked outline still projects");
    let OutlineFeature::Polygon { points, .. } = linked_feature else {
        panic!("linked target feature keeps its source variant")
    };
    assert_eq!(
        points,
        &vec![point(-5.0, 6.0), point(-3.0, 4.0), point(-1.0, 2.0)]
    );
    let first_output = document.clone();
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated exception repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

#[test]
fn axis_map_projects_world_geometry_without_changing_established_identities() {
    let (mut document, mut projection) = outline_fixture();
    let mapping = crate::model::Pose2 {
        at: point(100.0, -30.0),
        rotation: 0.0,
    };
    document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
    projection.mapping = mapping;
    document.outline.push(OutlineFeature::Rect {
        rotation: Some(25.0),
        anchor_part_id: None,
        id: "source-board/axis-rect".into(),
        center: point(3.0, 4.0),
        size: point(8.0, 5.0),
        radius: 0.75,
        operation: Operation::Subtract,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .push("source-board/axis-rect".into());
    document.outline.push(OutlineFeature::Rect {
        rotation: Some(0.0),
        anchor_part_id: None,
        id: "mirror-board/axis-rect".into(),
        center: point(-9.0, -8.0),
        size: point(1.0, 1.0),
        radius: 0.0,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/axis-rect".into());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Outline,
            source_id: "source-board/axis-rect".into(),
            mirror_id: "mirror-board/axis-rect".into(),
        });

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid axis-mapped outline");
    let OutlineFeature::Rect {
        rotation,
        center,
        size,
        radius,
        operation,
        ..
    } = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/axis-rect")
        .unwrap()
    else {
        panic!("the projected world rectangle keeps its variant")
    };
    assert_eq!(*rotation, Some(-25.0));
    assert_eq!(*center, point(97.0, -26.0));
    assert_eq!(*size, point(8.0, 5.0));
    assert_eq!(*radius, 0.75);
    assert_eq!(*operation, Operation::Subtract);
}

