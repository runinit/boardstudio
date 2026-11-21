
#[test]
fn independent_part_anchor_and_protected_gap_use_full_heading_coordinates() {
    let (mut document, projection) = outline_fixture();
    let mut source_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c1")
        .unwrap()
        .clone();
    source_part.id = "source-board/independent-part".into();
    source_part.reference = "SRC-INDEPENDENT".into();
    let mut mirror_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    mirror_part.id = "mirror-board/independent-part".into();
    mirror_part.reference = "MIR-INDEPENDENT".into();
    document.parts.extend([source_part, mirror_part]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("source-board/independent-part".into());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("mirror-board/independent-part".into());
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: "source-board/independent-part".into(),
        mirror_id: "mirror-board/independent-part".into(),
    });
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        source_id: "source-board/independent-anchor".into(),
        mirror_id: "mirror-board/independent-anchor".into(),
    });
    let source_outline = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source_outline.versions[0]
        .geometry
        .protected_gaps
        .push(ProtectedOutlineGap {
            id: "source-board/independent-gap".into(),
            points: vec![control(7.0, -11.0, Some("source-board/independent-part"))],
        });
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("source-board/independent-part".into()),
        id: "source-board/independent-anchor".into(),
        points: vec![point(2.0, 4.0), point(6.0, 8.0), point(10.0, 12.0)],
        operation: Operation::Subtract,
    });
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("mirror-board/independent-part".into()),
        id: "mirror-board/independent-anchor".into(),
        points: vec![point(0.0, 0.0); 3],
        operation: Operation::Add,
    });
    for (board_id, feature_id) in [
        ("source-board", "source-board/independent-anchor"),
        ("mirror-board", "mirror-board/independent-anchor"),
    ] {
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .outline_ids
            .push(feature_id.into());
    }

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid independent-part outline and protected-gap anchors");
    let independent_anchor = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/independent-anchor")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = independent_anchor
    else {
        panic!("the mapped independent anchor remains a polygon")
    };
    assert_eq!(
        anchor_part_id.as_deref(),
        Some("mirror-board/independent-part")
    );
    assert_eq!(
        points,
        &vec![point(10.0, -12.0), point(6.0, -8.0), point(2.0, -4.0)]
    );
    let linked = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    assert_eq!(
        linked.geometry.protected_gaps[1].points,
        [control(7.0, 11.0, Some("mirror-board/independent-part"))]
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].points[1],
        control(-13.0, 14.0, Some("matrix/mirror-matrix/r0c1")),
        "the key anchor keeps its separate -x,y reflection rule"
    );
}

#[test]
fn changed_source_outline_bridge_keeps_assembly_references_consistent() {
    let (mut document, projection) = outline_assembly_fixture();
    crate::mechanical::halves::validate_document(&document)
        .expect("the matrix assembly and outline references begin valid");
    let assemblies_before = document.assemblies.clone();
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("initial source assembly reference projects");
    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].part_ids,
        ["matrix/mirror-matrix/r0c1/diode"]
    );
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].matrix_ids,
        ["mirror-matrix"]
    );

    let source_outline = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source_outline.versions[0].geometry.bridges[0].part_ids =
        vec!["matrix/source-matrix/r0c1/fallback".into()];
    source_outline.versions[0].geometry.bridges[0].width = 2.25;
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("changed source assembly reference projects");

    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].part_ids,
        ["matrix/mirror-matrix/r0c1/fallback"]
    );
    assert_eq!(target_outline.versions[0].geometry.bridges[0].width, 2.25);
    assert_eq!(
        document.assemblies, assemblies_before,
        "the referenced document-scoped assembly definition remains shared"
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    crate::mechanical::halves::validate_document(&document)
        .expect("mapped bridge references remain valid after the source edit");

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("changed assembly reference repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn removing_source_version_prunes_only_its_mapped_target_and_retains_other_choices() {
    let (mut document, projection) = outline_fixture();
    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/version/fixed".into(),
            "mirror-board/version/fixed".into(),
        ),
        document.clone(),
    );
    let private_context_before = projection.outline_removal_baselines.borrow().clone();
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source
        .versions
        .retain(|version| version.id != "source-board/version/fixed");
    source.active_version_id = Some("source-board/version/linked".into());

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source version removal");
    let target = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target.active_version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert!(
        target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/linked")
    );
    assert!(
        !target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/fixed")
    );
    assert!(
        target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/local-version")
    );
    assert_eq!(
        target.retired_version_ids,
        ["mirror-board/retired-version", "mirror-board/version/fixed"]
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated version-removal repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.outline_removal_baselines.borrow(),
        private_context_before
    );
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn outline_removal_refuses_a_foreign_unpaired_target_atomically() {
    let (mut document, projection) = outline_fixture();
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .push(version(
            "mirror-board/foreign-version",
            OutlineOwnership::Fixed,
            OutlineSnapshot {
                features: vec![],
                settings: Default::default(),
                expected_regions: 123,
                bridges: vec![],
                protected_gaps: vec![],
            },
        ));
    let baseline = document.clone();
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/version/fixed".into(),
            "mirror-board/foreign-version".into(),
        ),
        baseline,
    );
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap()
        .versions
        .retain(|version| version.id != "source-board/version/fixed");
    let post_source = document.clone();
    let post_source_context = projection_state(&projection);
    let error = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect_err("a valid but unrelated target version is not owned by this removal");
    assert!(
        error.contains("lacks one exact baseline correspondence"),
        "{error}"
    );
    assert_eq!(
        document, post_source,
        "refusal is atomic across the whole document"
    );
    assert_eq!(projection_state(&projection), post_source_context);
}

#[test]
fn removing_source_outline_retires_only_its_exact_target_and_updates_versions() {
    let (mut document, projection) = outline_fixture();
    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/feature".into(),
            "mirror-board/feature".into(),
        ),
        document.clone(),
    );
    document
        .outline
        .retain(|feature| feature.id() != "source-board/feature");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .retain(|id| id != "source-board/feature");
    for outline in document
        .board_outlines
        .iter_mut()
        .filter(|outline| outline.board_id == "source-board")
    {
        for version in &mut outline.versions {
            version
                .geometry
                .features
                .retain(|feature| feature.id() != "source-board/feature");
        }
    }

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source outline retirement");
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    );
    assert!(
        !document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .contains(&"mirror-board/feature".to_owned())
    );
    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert!(target_outline.versions.iter().all(|version| {
        !version
            .geometry
            .features
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    }));
    assert_eq!(
        target_outline.retired_version_ids,
        ["mirror-board/retired-version"]
    );
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated outline-retirement repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

#[test]
fn source_outline_version_without_correspondence_is_refused_atomically() {
    let (mut document, projection) = outline_fixture();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::OutlineVersion
            && association.source_id == "source-board/version/fixed")
    });
    let before = document.clone();
    let error = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect_err("source version work without a selected target must not silently no-op");
    assert!(error.contains("Source outline version \"source-board/version/fixed\" has no selected mirror correspondence"), "{error}");
    assert_eq!(
        document, before,
        "the refusal leaves the complete document untouched"
    );
}

#[test]
fn outline_only_removal_cleans_the_exact_target_feature_and_board_reference() {
    let (mut document, projection) = outline_fixture();
    document.board_outlines.clear();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves
        .correspondences
        .retain(|association| association.kind != MirroredItemKind::OutlineVersion);
    document.outline.push(OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: "mirror-board/unrelated-outline".into(),
        center: point(3.0, 4.0),
        size: point(7.0, 9.0),
        radius: 0.0,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/unrelated-outline".into());
    crate::mechanical::halves::validate_document(&document)
        .expect("the pre-source outline-only baseline is valid");

    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/feature".into(),
            "mirror-board/feature".into(),
        ),
        document.clone(),
    );
    document
        .outline
        .retain(|feature| feature.id() != "source-board/feature");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .retain(|id| id != "source-board/feature");

    let changed = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid authenticated outline-only removal");
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    );
    let mirror_board = document
        .boards
        .iter()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    assert!(
        !mirror_board
            .outline_ids
            .contains(&"mirror-board/feature".to_owned())
    );
    assert!(
        mirror_board
            .outline_ids
            .contains(&"mirror-board/unrelated-outline".to_owned())
    );
    assert!(
        document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/unrelated-outline")
    );
    assert!(changed.contains(&"mirror-board/feature".to_owned()));
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("outline-only removal repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn clearing_source_active_version_deselects_only_the_linked_target_version() {
    let (mut document, projection) = outline_fixture();
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("initial linked selection projects");
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source.active_version_id = None;
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("clearing the source selection projects generated selection");
    let mirror = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(mirror.active_version_id, None);
    assert!(
        mirror
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/fixed")
    );
    assert!(
        mirror
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/local-version")
    );
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("generated selection repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn clearing_source_active_version_preserves_a_local_target_selection() {
    let (mut document, projection) = outline_fixture();
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
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap()
        .active_version_id = None;
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .active_version_id = Some("mirror-board/local-version".into());
    crate::mechanical::halves::validate_document(&document)
        .expect("the independent local target selection is valid");
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("source generated selection does not overwrite a local target choice");
    assert_eq!(
        document
            .board_outlines
            .iter()
            .find(|outline| outline.board_id == "mirror-board")
            .unwrap()
            .active_version_id
            .as_deref(),
        Some("mirror-board/local-version")
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("local selection repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

