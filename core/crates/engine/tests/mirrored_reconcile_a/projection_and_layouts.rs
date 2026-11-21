use super::*;

#[test]
fn reconcile_projects_associated_independent_parts_and_preserves_fixed_layouts() {
    let (mut document, mut projection) = fixed_geometry_fixture().unwrap();
    let mut source = document.parts.by_id("matrix/source-matrix/r0c1").clone();
    source.id = "independent-source".into();
    source.reference = "J1".into();
    source.pose = Pose2 {
        at: Vec2 { x: 10.0, y: 15.0 },
        rotation: 25.0,
    };
    source.keycap = Some(Vec2 { x: 22.0, y: 18.0 });
    source.outline = Some(PartOutline {
        excluded: true,
        margin: Some(3.5),
        allow_body_overhang: true,
    });
    source.side = Side::Back;
    source.locked = Some(false);
    source.properties = Some(BTreeMap::from([
        (
            "custom".into(),
            serde_json::json!({"board":"source","revision":7}),
        ),
        ("userFlag".into(), serde_json::json!(true)),
    ]));
    source.generator_parameters = Some(BTreeMap::from([
        ("variant".into(), serde_json::json!("non-default")),
        ("width".into(), serde_json::json!(2.25)),
    ]));
    let mut target = source.clone();
    target.id = "independent-mirror".into();
    target.definition_id = "mirror-switch".into();
    target.reference = "J9".into();
    target.pose = Pose2 {
        at: Vec2::default(),
        rotation: 0.0,
    };
    target.keycap = None;
    target.outline = None;
    target.side = Side::Front;
    target.locked = Some(true);
    target.properties = Some(BTreeMap::from([(
        "local".into(),
        serde_json::json!("target"),
    )]));
    target.generator_parameters = Some(BTreeMap::from([("local".into(), serde_json::json!(9))]));
    let mut target_definition = document.definitions[0].clone();
    target_definition.id = "mirror-switch".into();
    target_definition.name = "Mirror switch".into();
    document.definitions.push(target_definition);
    document.parts.extend([source.clone(), target]);
    let mut unrelated_part = document.parts.by_id("geometry-probe").clone();
    unrelated_part.id = "unrelated-part".into();
    unrelated_part.reference = "LOCAL1".into();
    document.parts.push(unrelated_part);
    let mut added_source = source.clone();
    added_source.id = "independent-source-added".into();
    added_source.reference = "J2".into();
    added_source.pose.at = Vec2 { x: -8.0, y: 6.0 };
    document.parts.push(added_source.clone());
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(source.id.clone());
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(added_source.id.clone());
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push("independent-mirror".into());
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push("unrelated-part".into());

    let halves = document.first_halves_mut();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: "independent-source".into(),
            mirror_id: "independent-mirror".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-layout".into(),
            mirror_id: "mirror-layout".into(),
        },
    ]);
    let source_layout = Layout {
        id: "source-layout".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec!["independent-source".into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout".into(),
        name: "My target layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec!["geometry-probe".into(), "independent-mirror".into()],
        mirror_link: None,
    };
    projection.independent_part_transitions = vec![IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "mirror-switch".into(),
        mirror_reference: "J99".into(),
    }];
    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixture_matrix("unrelated-matrix", "mirror-board"),
    )
    .unwrap();
    let unrelated_layout = Layout {
        id: "unrelated-layout".into(),
        name: "Unrelated local layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["unrelated-matrix".into()],
        part_ids: vec!["unrelated-part".into()],
        mirror_link: None,
    };
    document.layouts = vec![
        source_layout.clone(),
        mirror_layout.clone(),
        unrelated_layout.clone(),
    ];

    assert!(
        document
            .parts
            .iter()
            .all(|part| part.id != "independent-mirror-added")
    );
    assert!(document.parts.iter().all(|part| part.reference != "J99"));

    crate::support::apply_for_test(
        &mut document,
        &[],
        &boardstudio_model::model::EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![
                    "independent-source-added".into(),
                    "independent-source".into(),
                ],
                ..source_layout
            },
        },
    )
    .expect("the ordinary source SetLayout operation records its membership change");

    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    let source_before = document.parts.by_id("independent-source").clone();
    let unrelated_part_before = document.parts.by_id("unrelated-part").clone();
    let unrelated_owner_ids_before: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
        .map(|board| board.id.clone())
        .collect();
    let original = document.clone();

    let valid_addition_evidence = projection.independent_part_transitions.clone();
    let mut invalid_addition_outcomes = Vec::new();

    let mut unrelated_projection = projection.clone();
    unrelated_projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "unrelated-part".into(),
        mirror_definition_id: unrelated_part_before.definition_id.clone(),
        mirror_reference: unrelated_part_before.reference.clone(),
    };
    let mut unrelated_target = original.clone();
    let unrelated_target_before = unrelated_target.clone();
    let unrelated_result = reconcile(
        &mut unrelated_target,
        "pair-source-mirror",
        &unrelated_projection,
    );
    invalid_addition_outcomes.push((
        "first add cannot adopt an unrelated mirror-owned target",
        unrelated_result.is_err(),
        unrelated_target == unrelated_target_before,
    ));

    let mut two_sources = original.clone();
    let mut second_source = added_source.clone();
    second_source.id = "independent-source-outside-layout".into();
    second_source.reference = "Q77".into();
    second_source.pose = Pose2 {
        at: Vec2 { x: 31.0, y: -17.0 },
        rotation: 71.0,
    };
    second_source.properties = Some(BTreeMap::from([(
        "sibling".into(),
        serde_json::json!("distinct payload"),
    )]));
    assert_ne!(second_source.pose, added_source.pose);
    assert_ne!(second_source.properties, added_source.properties);
    two_sources.parts.push(second_source.clone());
    two_sources
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(second_source.id.clone());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&two_sources)
        .expect("the sibling source outside the paired Layout is valid");
    assert!(
        two_sources.layouts[0]
            .part_ids
            .iter()
            .all(|id| id != &second_source.id)
    );
    assert!(
        two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q91"
        })
    );
    assert!(
        two_sources.parts.iter().all(|part| {
            part.id == "independent-source-outside-layout" || part.reference != "Q92"
        })
    );
    assert!(two_sources.parts.iter().all(|part| {
        !matches!(
            part.id.as_str(),
            "planned-independent-target"
                | "planned-independent-target-a"
                | "planned-independent-target-b"
        )
    }));

    let mut aliased_id_projection = projection.clone();
    aliased_id_projection.independent_part_transitions = vec![
        IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "planned-independent-target".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q91".into(),
        },
        IndependentPartTransition::Add {
            source_id: second_source.id.clone(),
            mirror_id: "planned-independent-target".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q91".into(),
        },
    ];
    let mut aliased_target_id = two_sources.clone();
    let aliased_target_id_before = aliased_target_id.clone();
    let aliased_id_result = reconcile(
        &mut aliased_target_id,
        "pair-source-mirror",
        &aliased_id_projection,
    );
    invalid_addition_outcomes.push((
        "sibling additions cannot select the same fresh target ID and reference",
        aliased_id_result.is_err(),
        aliased_target_id == aliased_target_id_before,
    ));

    let mut aliased_reference_projection = projection.clone();
    aliased_reference_projection.independent_part_transitions = vec![
        IndependentPartTransition::Add {
            source_id: "independent-source-added".into(),
            mirror_id: "planned-independent-target-a".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q92".into(),
        },
        IndependentPartTransition::Add {
            source_id: second_source.id,
            mirror_id: "planned-independent-target-b".into(),
            mirror_definition_id: "mirror-switch".into(),
            mirror_reference: "Q92".into(),
        },
    ];
    let mut aliased_target_reference = two_sources.clone();
    let aliased_target_reference_before = aliased_target_reference.clone();
    let aliased_reference_result = reconcile(
        &mut aliased_target_reference,
        "pair-source-mirror",
        &aliased_reference_projection,
    );
    invalid_addition_outcomes.push((
        "sibling additions cannot select distinct targets with one fresh reference",
        aliased_reference_result.is_err(),
        aliased_target_reference == aliased_target_reference_before,
    ));
    assert!(
        invalid_addition_outcomes
            .iter()
            .all(|(_, refused, unchanged)| *refused && *unchanged),
        "invalid independent Add evidence must refuse atomically: {invalid_addition_outcomes:?}"
    );

    projection.independent_part_transitions = valid_addition_evidence.clone();
    projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "missing-mirror-definition".into(),
        mirror_reference: "J99".into(),
    };
    let mut unavailable_definition = original.clone();
    let refused = reconcile(
        &mut unavailable_definition,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refused.contains("target definition"));
    assert_eq!(unavailable_definition, original);

    projection.independent_part_transitions[0] = IndependentPartTransition::Add {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
        mirror_definition_id: "mirror-switch".into(),
        mirror_reference: "LOCAL1".into(),
    };
    let mut conflicting_reference = original.clone();
    let refused = reconcile(
        &mut conflicting_reference,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refused.contains("reference") || refused.contains("already in use"));
    assert_eq!(conflicting_reference, original);

    projection.independent_part_transitions = valid_addition_evidence.clone();
    let mut stale_source_owner = original.clone();
    stale_source_owner
        .boards
        .by_id_mut("source-board")
        .part_ids
        .retain(|id| id != "independent-source-added");
    stale_source_owner
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push("independent-source-added".into());
    let stale_source_owner_before = stale_source_owner.clone();
    let refused =
        reconcile(&mut stale_source_owner, "pair-source-mirror", &projection).unwrap_err();
    assert!(refused.contains("stale source ownership"));
    assert_eq!(stale_source_owner, stale_source_owner_before);

    let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert_eq!(
        changed,
        vec![
            "independent-mirror",
            "independent-mirror-added",
            "matrix/mirror-matrix/r0c1",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c2",
            "mirror-board",
            "mirror-layout",
            "mirror-matrix",
        ]
    );
    let mirrored = document.parts.by_id("independent-mirror");
    assert_eq!(mirrored.id, "independent-mirror");
    assert_eq!(mirrored.definition_id, "mirror-switch");
    assert_eq!(mirrored.reference, "J9");
    assert_eq!(mirrored.keycap, source.keycap);
    assert_eq!(mirrored.outline, source.outline);
    assert_eq!(mirrored.side, source.side);
    assert_eq!(mirrored.locked, source.locked);
    assert_eq!(mirrored.properties, source.properties);
    assert_eq!(mirrored.generator_parameters, source.generator_parameters);
    assert_eq!(
        mirrored.pose,
        Pose2 {
            at: Vec2 { x: 85.0, y: -40.0 },
            rotation: 245.0,
        }
    );
    assert!(changed.contains(&"independent-mirror".to_owned()));
    assert!(changed.contains(&"independent-mirror-added".to_owned()));
    let added_mirror = document
        .parts
        .iter()
        .find(|part| part.id == "independent-mirror-added")
        .expect("the explicit independent addition creates its selected target ID");
    assert_eq!(added_mirror.definition_id, "mirror-switch");
    assert_eq!(added_mirror.reference, "J99");
    assert_eq!(added_mirror.keycap, added_source.keycap);
    assert_eq!(added_mirror.outline, added_source.outline);
    assert_eq!(added_mirror.side, added_source.side);
    assert_eq!(added_mirror.locked, added_source.locked);
    assert_eq!(added_mirror.properties, added_source.properties);
    assert_eq!(
        added_mirror.generator_parameters,
        added_source.generator_parameters
    );
    assert_eq!(
        added_mirror.pose,
        Pose2 {
            at: Vec2 { x: 94.0, y: -22.0 },
            rotation: 245.0,
        }
    );
    assert!(
        document
            .boards
            .iter()
            .any(|board| board.id == "mirror-board"
                && board.part_ids.contains(&"independent-mirror-added".into()))
    );
    assert_eq!(document.parts.by_id("independent-source"), &source_before);
    assert_eq!(document.layouts[1].id, mirror_layout.id);
    assert_eq!(document.layouts[1].name, mirror_layout.name);
    assert_eq!(document.layouts[1].board_id, mirror_layout.board_id);
    assert_eq!(document.layouts[1].matrix_ids, mirror_layout.matrix_ids);
    assert_eq!(document.layouts[1].mirror_link, mirror_layout.mirror_link);
    assert_eq!(
        document.layouts[1].part_ids,
        vec![
            "independent-mirror-added",
            "independent-mirror",
            "geometry-probe"
        ]
    );
    assert!(changed.contains(&"mirror-layout".to_owned()));
    assert_eq!(document.layouts[2], unrelated_layout);
    assert_eq!(
        document.parts.by_id("unrelated-part"),
        &unrelated_part_before
    );
    let unrelated_owner_ids_after: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&"unrelated-part".to_owned()))
        .map(|board| board.id.clone())
        .collect();
    assert_eq!(unrelated_owner_ids_after, unrelated_owner_ids_before);

    let mut repeated_from_same_input = original.clone();
    reconcile(
        &mut repeated_from_same_input,
        "pair-source-mirror",
        &projection,
    )
    .unwrap();
    assert_eq!(repeated_from_same_input, document);
    let first_output = document.clone();
    let applied_additions_before_repeat = projection.applied_independent_additions.borrow().clone();
    let second_changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
    assert!(second_changed.is_empty());
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.applied_independent_additions.borrow(),
        applied_additions_before_repeat,
        "updated private Add provenance converges with the repeated output"
    );

    let mut removal_source = document.clone();
    removal_source.nets.push(boardstudio_model::model::Net {
        id: "independent-removal-net".into(),
        name: "Independent removal control".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "independent-mirror-added".into(),
                pad_id: "pad-two".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    removal_source
        .boards
        .by_id_mut("mirror-board")
        .net_ids
        .push("independent-removal-net".into());
    removal_source
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "independent-removal-offset".into(),
            source_part_id: "independent-mirror-added".into(),
            target_part_id: "geometry-probe".into(),
            offset: Vec2 { x: 4.0, y: -2.0 },
            rotation: 5.0,
        });
    let mut expected_retained_keys = removal_source
        .outline
        .iter()
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, part_ids, .. } if id == "retained-keys-envelope" => {
                Some(part_ids.clone())
            }
            _ => None,
        })
        .expect("the baseline carries a registered retained-key envelope");
    expected_retained_keys.push("independent-mirror-added".into());
    if let Some(OutlineFeature::PartEnvelope { part_ids, .. }) =
        removal_source.outline.iter_mut().find(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, .. }
            if id == "retained-keys-envelope")
        })
    {
        part_ids.push("independent-mirror-added".into());
    }
    crate::support::apply_for_test(
        &mut removal_source,
        &[],
        &boardstudio_model::model::EditOperation::RemoveParts {
            ids: vec!["independent-source-added".into()],
        },
    )
    .unwrap();
    assert!(
        removal_source
            .parts
            .iter()
            .all(|part| part.id != "independent-source-added")
    );
    assert!(removal_source.outline.iter().any(|feature| {
        matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
            if id == "retained-keys-envelope"
                && part_ids.contains(&"independent-mirror-added".into()))
    }));
    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
    }];
    let persisted_pair_before = removal_source.first_halves().correspondences.clone();
    let mut refused_removal = removal_source.clone();
    refused_removal.layouts[1]
        .part_ids
        .push("independent-mirror".into());
    let refusal_before = refused_removal.clone();
    let refusal = reconcile(&mut refused_removal, "pair-source-mirror", &projection).unwrap_err();
    assert!(refusal.contains("paired layout contains duplicate part membership"));
    assert_eq!(refused_removal, refusal_before);

    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror".into(),
    }];
    let mut conflicting_removal_identity = removal_source.clone();
    let conflicting_removal_before = conflicting_removal_identity.clone();
    let refusal = reconcile(
        &mut conflicting_removal_identity,
        "pair-source-mirror",
        &projection,
    )
    .unwrap_err();
    assert!(refusal.contains("conflicting pair provenance"));
    assert_eq!(conflicting_removal_identity, conflicting_removal_before);
    projection.independent_part_transitions = vec![IndependentPartTransition::Remove {
        source_id: "independent-source-added".into(),
        mirror_id: "independent-mirror-added".into(),
    }];

    let removed = reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
    assert!(
        removal_source
            .parts
            .iter()
            .all(|part| part.id != "independent-mirror-added")
    );
    assert!(
        removal_source
            .boards
            .iter()
            .all(|board| !board.part_ids.contains(&"independent-mirror-added".into()))
    );
    assert!(
        removal_source.layouts[1]
            .part_ids
            .iter()
            .all(|id| id != "independent-mirror-added")
    );
    assert_eq!(
        removal_source.nets.by_id("independent-removal-net").pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into(),
        }]
    );
    assert!(
        removal_source
            .constraints
            .iter()
            .all(|constraint| constraint.id() != "independent-removal-offset")
    );
    expected_retained_keys.retain(|id| id != "independent-mirror-added");
    assert!(removal_source.outline.iter().any(|feature| {
        matches!(feature, OutlineFeature::PartEnvelope { id, part_ids, .. }
            if id == "retained-keys-envelope" && part_ids == &expected_retained_keys)
    }));
    assert_eq!(
        removed,
        vec![
            "independent-mirror-added",
            "independent-removal-net",
            "independent-removal-offset",
            "mirror-board",
            "mirror-layout",
            "retained-keys-envelope",
        ]
    );
    assert_eq!(
        removal_source.first_halves().correspondences,
        persisted_pair_before
    );
    let first_removal_output = removal_source.clone();
    let repeated_removal =
        reconcile(&mut removal_source, "pair-source-mirror", &projection).unwrap();
    assert!(repeated_removal.is_empty());
    assert_eq!(removal_source, first_removal_output);
}

#[test]
fn source_layout_removal_prunes_linked_projection_and_keeps_local_and_unpaired_members() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_linked = "layout-linked-source";
    let mirror_linked = "layout-linked-mirror";
    let source_local = "layout-local-source";
    let mirror_local = "layout-local-mirror";
    let mut linked_source_part = document.parts.by_id("matrix/source-matrix/r0c1").clone();
    linked_source_part.id = source_linked.into();
    linked_source_part.reference = "LINKED-SOURCE".into();
    let mut linked_mirror_part = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    linked_mirror_part.id = mirror_linked.into();
    linked_mirror_part.reference = "LINKED-MIRROR".into();
    let mut local_source_part = linked_source_part.clone();
    local_source_part.id = source_local.into();
    local_source_part.reference = "LOCAL-SOURCE".into();
    let mut local_mirror_part = local_source_part.clone();
    local_mirror_part.id = mirror_local.into();
    local_mirror_part.reference = "LOCAL-MIRROR".into();
    document.parts.extend([
        linked_source_part,
        linked_mirror_part,
        local_source_part,
        local_mirror_part,
    ]);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .extend([source_linked.into(), source_local.into()]);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .extend([mirror_linked.into(), mirror_local.into()]);

    let halves = document.first_halves_mut();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_linked.into(),
            mirror_id: mirror_linked.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_local.into(),
            mirror_id: mirror_local.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-layout-removal".into(),
            mirror_id: "mirror-layout-removal".into(),
        },
    ]);
    halves.exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_local, "state": "local"
        }))
        .unwrap(),
    );

    let source_layout = Layout {
        id: "source-layout-removal".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_linked.into(), source_local.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-removal".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_linked.into(),
            mirror_local.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout];
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();

    let request = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![mirror_local.into(), "geometry-probe".into()],
            ..document.layouts[1].clone()
        },
    };
    let canonical = crate::support::canonicalize(&document, &request)
        .expect("the mirror request maps into source membership");
    let expected_source = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![source_local.into()],
            ..source_layout
        },
    };
    assert_eq!(canonical.operation, expected_source);
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the translated source layout removes its linked member");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the removal and retained target members reconcile");

    assert_eq!(
        document.layouts[1].part_ids,
        vec![mirror_local.to_owned(), "geometry-probe".to_owned()]
    );
}

#[test]
fn mirror_layout_reorder_survives_source_apply_and_reconciliation() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_header = "layout-header-source";
    let source_u1 = "layout-u1-source";
    let mirror_header = "layout-header-mirror";
    let mirror_u1 = "layout-u1-mirror";
    let mut source_header_part = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source_header_part.id = source_header.into();
    source_header_part.reference = "LAYOUT-HEADER-SOURCE".into();
    let mut source_u1_part = document.parts.by_id("matrix/source-matrix/r0c1").clone();
    source_u1_part.id = source_u1.into();
    source_u1_part.reference = "LAYOUT-U1-SOURCE".into();
    let mut mirror_header_part = document.parts.by_id("matrix/mirror-matrix/r0c0").clone();
    mirror_header_part.id = mirror_header.into();
    mirror_header_part.reference = "LAYOUT-HEADER-MIRROR".into();
    let mut mirror_u1_part = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    mirror_u1_part.id = mirror_u1.into();
    mirror_u1_part.reference = "LAYOUT-U1-MIRROR".into();
    document.parts.extend([
        source_header_part,
        source_u1_part,
        mirror_header_part,
        mirror_u1_part,
    ]);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .extend([source_header.into(), source_u1.into()]);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .extend([mirror_header.into(), mirror_u1.into()]);
    document.first_halves_mut().correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_header.into(),
            mirror_id: mirror_header.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_u1.into(),
            mirror_id: mirror_u1.into(),
        },
    ]);
    document.first_halves_mut().exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_u1, "state": "local"
        }))
        .unwrap(),
    );
    let source_layout = Layout {
        id: "source-layout-reorder".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_header.into(), source_u1.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-reorder".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_header.into(),
            mirror_u1.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout.clone()];
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: source_layout.id.clone(),
            mirror_id: mirror_layout.id.clone(),
        });

    let requested = Layout {
        part_ids: vec![
            mirror_u1.into(),
            mirror_header.into(),
            "geometry-probe".into(),
        ],
        ..mirror_layout
    };
    let canonical = crate::support::canonicalize(
        &document,
        &EditOperation::SetLayout {
            layout: requested.clone(),
        },
    )
    .expect("the reordered mirror request maps to source member order");
    assert_eq!(
        canonical.operation,
        EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![source_u1.into(), source_header.into()],
                ..source_layout
            },
        }
    );
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the canonical source layout applies");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the reordered source layout reconciles");

    assert_eq!(document.layouts[1].part_ids, requested.part_ids);
}
