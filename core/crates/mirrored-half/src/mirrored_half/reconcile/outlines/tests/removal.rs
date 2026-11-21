
#[test]
fn independent_part_anchor_and_protected_gap_use_full_heading_coordinates() {
    let mut document = outline_fixture();
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

    reopen(&mut document)
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
    let mut document = outline_assembly_fixture();
    crate::mechanical::halves::validate_document(&document)
        .expect("the matrix assembly and outline references begin valid");
    let assemblies_before = document.assemblies.clone();
    let pair_before = pair_state_bytes(&document);

    reopen(&mut document)
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
    reopen(&mut document)
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
    assert!(
        reopen(&mut document)
            .expect("changed assembly reference repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}




#[test]
fn source_outline_version_without_correspondence_is_refused_atomically() {
    let mut document = outline_fixture();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::OutlineVersion
            && association.source_id == "source-board/version/fixed")
    });
    let before = document.clone();
    let error = reopen(&mut document)
        .expect_err("source version work without a selected target must not silently no-op");
    assert!(error.contains("Source outline version \"source-board/version/fixed\" has no selected mirror correspondence"), "{error}");
    assert_eq!(
        document, before,
        "the refusal leaves the complete document untouched"
    );
}


#[test]
fn clearing_source_active_version_deselects_only_the_linked_target_version() {
    let mut document = outline_fixture();
    reopen(&mut document)
        .expect("initial linked selection projects");
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source.active_version_id = None;
    let pair_before = pair_state_bytes(&document);

    reopen(&mut document)
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
    assert!(
        reopen(&mut document)
            .expect("generated selection repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

#[test]
fn clearing_source_active_version_preserves_a_local_target_selection() {
    let mut document = outline_fixture();
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

    reopen(&mut document)
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
    assert!(
        reopen(&mut document)
            .expect("local selection repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

