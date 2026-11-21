use super::*;

#[test]
fn fixed_membership_assembly_rejections_are_atomic() {
    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
            panic!("expected atomic rejection containing {reason:?}");
        };
        assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    for (part_id, authored) in [
        ("matrix/mirror-matrix/r0c1", true),
        ("matrix/source-matrix/r0c1", true),
        ("matrix/mirror-matrix/r0c1", false),
        ("matrix/source-matrix/r0c1", false),
    ] {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        if authored {
            document
                .parts
                .by_id_mut(part_id)
                .properties
                .as_mut()
                .unwrap()
                .insert("layoutOverride".into(), serde_json::json!(1));
        } else {
            document.constraints.push(Constraint::Offset {
                id: "linked-key-constraint".into(),
                source_part_id: "assembly-probe".into(),
                target_part_id: part_id.into(),
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
            });
        }
        assert_rejected(
            document,
            &projection,
            "Linked matrix keys cannot have authored or constrained placement",
        );
    }

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "unavailable");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .first_halves_mut()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "companion association is missing");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document
        .first_halves_mut()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    association.source_id = "matrix/mirror-matrix/r0c0/diode".into();
    association.mirror_id = "matrix/source-matrix/r0c0/diode".into();
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document
        .first_halves_mut()
        .correspondences
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap()
        .clone();
    document
        .first_halves_mut()
        .correspondences
        .push(association);
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document
        .first_halves_mut()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    association.mirror_id = "matrix/mirror-matrix/r0c1/diode".into();
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    projection
        .final_owners
        .remove("matrix/mirror-matrix/r1c1/diode");
    assert_rejected(document, &projection, "final owner input is incomplete");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != "matrix/mirror-matrix/r1c1/diode");
    assert_rejected(document, &projection, "no board owner");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push("matrix/mirror-matrix/r1c1/diode".into());
    assert_rejected(document, &projection, "ambiguous board ownership");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != "matrix/mirror-matrix/r0c1/diode");
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push("matrix/mirror-matrix/r0c1/diode".into());
    assert_rejected(
        document,
        &projection,
        "companion has an incomplete or stale owner input",
    );

    let (mut document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let matrix = document.matrices.by_id_mut("source-matrix");
    matrix
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap()
        .assemblies
        .clear();
    matrix
        .part_ids
        .retain(|id| id != "matrix/source-matrix/r0c0/diode");
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .retain(|id| id != "matrix/source-matrix/r0c0/diode");
    document
        .first_halves_mut()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
    projection
        .final_owners
        .remove("matrix/source-matrix/r0c0/diode");
    assert_rejected(document, &projection, "membership is not canonical");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document.matrices.by_id_mut("source-matrix").cells[0].assemblies[0]
        .offset
        .x = f64::NAN;
    assert_rejected(document, &projection, "Matrix assembly is invalid");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let association = document
        .first_halves_mut()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
        .unwrap();
    association.companion_tweak.as_mut().unwrap().rotation = f64::NAN;
    assert_rejected(document, &projection, "finite");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let mut source = document.matrices.by_id("source-matrix").clone();
    source
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (1, 0))
        .unwrap()
        .assemblies[0]
        .offset
        .x = f64::MAX * 0.9;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source).unwrap();
    let association = document
        .first_halves_mut()
        .correspondences
        .iter_mut()
        .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
        .unwrap();
    association.companion_tweak.as_mut().unwrap().x = -f64::MAX * 0.9;
    assert_rejected(document, &projection, "non-finite values");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    document.constraints.push(Constraint::Offset {
        id: "linked-companion-constraint".into(),
        source_part_id: "assembly-probe".into(),
        target_part_id: "matrix/mirror-matrix/r0c1/diode".into(),
        offset: Vec2 { x: 1.0, y: 2.0 },
        rotation: 0.0,
    });
    assert_rejected(document, &projection, "authored or constrained placement");

    let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
    let companion = document.parts.by_id_mut("matrix/mirror-matrix/r0c1/diode");
    companion
        .properties
        .get_or_insert_with(BTreeMap::new)
        .insert("layoutOverride".into(), serde_json::json!(1));
    assert_rejected(document, &projection, "authored or constrained placement");
}

#[test]
fn fixed_membership_view_rejects_legacy_exception_aliases_and_foreign_owners() {
    fn legacy_child_fixture() -> (ProjectDoc, Projection, String) {
        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let source_id = "matrix/source-matrix/r0c1/diode";
        let mirror_id = "matrix/mirror-matrix/r0c1/diode".to_owned();
        let halves = document.first_halves_mut();
        halves.correspondences.retain(|association| {
            !(association.kind == MirroredItemKind::Part
                && association.source_id == source_id
                && association.mirror_id == mirror_id)
        });
        halves.exceptions.push(MirroredException {
            companion_mode: Some(MirroredCompanionMode::LiteralMirror),
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.clone(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
        (document, projection, mirror_id)
    }

    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
            panic!("expected atomic rejection containing {reason:?}");
        };
        assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    let (mut document, projection, _) = legacy_child_fixture();
    document.first_halves_mut().exceptions[0].mirror_id = "matrix/mirror-matrix/r0c2/diode".into();
    assert_rejected(document, &projection, "companion association is missing");

    let (mut document, projection, _) = legacy_child_fixture();
    let halves = document.first_halves_mut();
    halves.exceptions[0].kind = MirroredItemKind::Net;
    assert_rejected(
        document,
        &projection,
        "companion exception is aliased or inconsistent",
    );

    let (mut document, projection, _) = legacy_child_fixture();
    let source_id = "matrix/source-matrix/r0c1/diode";
    let mirror_id = "matrix/mirror-matrix/r0c1/diode";
    let halves = document.first_halves_mut();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: mirror_id.into(),
        mirror_id: source_id.into(),
    });
    assert_rejected(
        document,
        &projection,
        "companion association is aliased or inconsistent",
    );

    let (mut document, projection, _mirror_id) = legacy_child_fixture();
    let exception = document.first_halves_mut().exceptions[0].clone();
    document.first_halves_mut().exceptions.push(exception);
    assert_rejected(
        document,
        &projection,
        "companion exception is aliased or inconsistent",
    );
    let _ = mirror_id;

    let (mut document, projection, mirror_id) = legacy_child_fixture();
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != &mirror_id);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(mirror_id);
    assert_rejected(document, &projection, "incomplete or stale owner input");
}

#[test]
fn fixed_membership_rejections_are_atomic_and_reach_the_intended_guard() {
    fn assert_rejected(mut document: ProjectDoc, projection: &Projection, expected_reason: &str) {
        let before = serde_json::to_value(&document).unwrap();
        let error = reconcile(&mut document, "pair-source-mirror", projection).unwrap_err();
        assert!(
            error.contains(expected_reason),
            "expected {expected_reason:?}, got {error:?}"
        );
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
    }

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "mirror-matrix".into(),
            mirror_id: "source-matrix".into(),
        });
    assert_rejected(document, &projection, "matrix correspondence is ambiguous");

    let (document, mut projection) = fixed_geometry_fixture().unwrap();
    projection.parts.push(PartProjection {
        source_id: "matrix/source-matrix/r0c1".into(),
        mirror_id: "matrix/mirror-matrix/r0c1".into(),
    });
    assert_rejected(
        document,
        &projection,
        "linked matrix associations contain aliases",
    );

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .parts
        .retain(|part| part.id != "matrix/source-matrix/r0c0");
    assert_rejected(document, &projection, "unavailable");

    let (document, mut projection) = fixed_geometry_fixture().unwrap();
    projection.final_owners.remove("matrix/mirror-matrix/r1c1");
    assert_rejected(document, &projection, "final owner input is incomplete");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let part_id = "matrix/source-matrix/r0c1";
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .retain(|id| id != part_id);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push(part_id.into());
    assert_rejected(document, &projection, "owner input");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document.matrices.by_id_mut("source-matrix").cells[1].enabled = false;
    assert_rejected(document, &projection, "fixed active membership");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .matrices
        .by_id_mut("source-matrix")
        .part_ids
        .push("matrix/source-matrix/r0c0".into());
    assert_rejected(document, &projection, "membership is not canonical");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document.matrices.by_id_mut("source-matrix").column_layout[1].splay = f64::NAN;
    assert_rejected(document, &projection, "finite");

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document.matrices.by_id_mut("source-matrix").column_layout = vec![
        boardstudio_model::model::ColumnLayout {
            splay: f64::MAX,
            ..Default::default()
        },
        boardstudio_model::model::ColumnLayout {
            splay: f64::MAX,
            ..Default::default()
        },
        Default::default(),
    ];
    assert_rejected(
        document,
        &projection,
        "layout values and evaluated results must be finite",
    );

    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    document
        .first_halves_mut()
        .correspondences
        .retain(|association| association.source_id != "matrix/source-matrix/r1c1");
    assert_rejected(
        document,
        &projection,
        "canonical matrix part association is missing",
    );
}

#[test]
fn changed_source_matrix_projects_linked_neighbor_after_entry_snapshot() {
    let mut document = fixture_document().unwrap();
    let mut moved_source = document.matrices.by_id("source-matrix").clone();
    moved_source.origin = Vec2 { x: 27.0, y: -14.0 };
    moved_source.rotation = Some(43.0);
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &moved_source).unwrap();

    let definition = document.definitions.by_id_mut("switch");
    definition.pads = vec![
        boardstudio_model::model::Pad {
            id: "pad-one".into(),
            number: "1".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
        boardstudio_model::model::Pad {
            id: "pad-two".into(),
            number: "2".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
    ];

    let probe_id = "projection-probe";
    let mut probe = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    probe.id = probe_id.into();
    probe.reference = "PROBE".into();
    probe.pose.at = Vec2 { x: 4.0, y: 2.0 };
    probe.pose.rotation = 0.0;
    document.parts.push(probe);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push(probe_id.into());

    let retained_net_id = "retained-two-pin-net";
    document.nets.push(boardstudio_model::model::Net {
        id: retained_net_id.into(),
        name: "RETAINED".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r0c0".into(),
                pad_id: "pad-one".into(),
            },
            boardstudio_model::model::Pin {
                part_id: probe_id.into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    document
        .boards
        .by_id_mut("mirror-board")
        .net_ids
        .push(retained_net_id.into());

    let retained_offset = boardstudio_model::model::Constraint::Offset {
        id: "retained-stationary-offset".into(),
        source_part_id: probe_id.into(),
        target_part_id: "matrix/mirror-matrix/r0c0".into(),
        offset: Vec2 { x: -4.0, y: -2.0 },
        rotation: 0.0,
    };
    document.constraints.push(retained_offset.clone());

    let retained_envelope_id = "retained-global-envelope";
    document
        .outline
        .push(boardstudio_model::model::OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: Default::default(),
            id: retained_envelope_id.into(),
            part_ids: vec![
                "matrix/mirror-matrix/r0c0".into(),
                "matrix/mirror-matrix/r0c1".into(),
                probe_id.into(),
            ],
            margin: 2.0,
            operation: boardstudio_model::model::Operation::Add,
        });
    document
        .boards
        .by_id_mut("mirror-board")
        .outline_ids
        .push(retained_envelope_id.into());

    let assert_retained_net_is_valid = |document: &ProjectDoc| {
        let net = document.nets.by_id(retained_net_id);
        assert_eq!(
            net.pins,
            vec![
                boardstudio_model::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c0".into(),
                    pad_id: "pad-one".into(),
                },
                boardstudio_model::model::Pin {
                    part_id: probe_id.into(),
                    pad_id: "pad-two".into(),
                },
            ]
        );
        let owner = document
            .boards
            .iter()
            .find(|board| board.net_ids.contains(&net.id))
            .unwrap();
        assert_eq!(owner.id, "mirror-board");
        assert_eq!(
            document
                .boards
                .iter()
                .filter(|board| board.net_ids.contains(&net.id))
                .count(),
            1
        );
        for pin in &net.pins {
            assert!(owner.part_ids.contains(&pin.part_id));
            let part = document.parts.by_id(&pin.part_id);
            let definition = document.definitions.by_id(&part.definition_id);
            assert!(
                definition
                    .pads
                    .iter()
                    .any(|pad| pad.id == pin.pad_id && !pad.id.is_empty())
            );
        }

        let retained_mirror = document.parts.by_id("matrix/mirror-matrix/r0c0");
        assert_eq!(
            retained_mirror.pose,
            boardstudio_model::model::Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            }
        );
        let probe = document.parts.by_id(probe_id);
        assert_eq!(
            probe.pose,
            boardstudio_model::model::Pose2 {
                at: Vec2 { x: 4.0, y: 2.0 },
                rotation: 0.0,
            }
        );

        let envelope = document
            .outline
            .iter()
            .find(|feature| {
                matches!(feature, boardstudio_model::model::OutlineFeature::PartEnvelope { id, .. } if id == retained_envelope_id)
            })
            .unwrap();
        let boardstudio_model::model::OutlineFeature::PartEnvelope { part_ids, .. } = envelope
        else {
            unreachable!("the retained feature was found as a part envelope");
        };
        assert_eq!(
            part_ids,
            &vec![
                "matrix/mirror-matrix/r0c0".to_owned(),
                "matrix/mirror-matrix/r0c1".to_owned(),
                probe_id.to_owned(),
            ]
        );
        assert_eq!(
            document
                .boards
                .iter()
                .filter(|board| board.outline_ids.contains(&retained_envelope_id.to_owned()))
                .map(|board| board.id.as_str())
                .collect::<Vec<_>>(),
            vec!["mirror-board"]
        );
        assert!(
            document
                .boards
                .by_id("source-board")
                .net_ids
                .iter()
                .all(|net_id| net_id != retained_net_id)
        );
    };
    assert_retained_net_is_valid(&document);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();

    // The entry snapshot is deliberately taken after the source's ordinary edit/materialization.
    let entry = document.clone();
    let entry_source_matrix = entry.matrices.by_id("source-matrix").clone();
    let entry_mirror_matrix = entry.matrices.by_id("mirror-matrix").clone();
    let matrix_correspondence = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: entry_source_matrix.id.clone(),
        mirror_id: entry_mirror_matrix.id.clone(),
    };
    let expected_mirror_matrix = project_matrix(
        &entry_source_matrix,
        &entry_mirror_matrix,
        &matrix_correspondence,
        fixture_mapping(),
    )
    .unwrap();
    let mut expected_document = entry.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut expected_document, &expected_mirror_matrix)
        .unwrap();

    let protected_local_id = "matrix/mirror-matrix/r0c0";
    let linked_source_id = "matrix/source-matrix/r0c1";
    let linked_mirror_id = "matrix/mirror-matrix/r0c1";
    let entry_probe = entry.parts.by_id(probe_id).clone();
    let entry_probe_owner = entry
        .boards
        .iter()
        .find(|board| board.part_ids.contains(&probe_id.to_owned()))
        .unwrap()
        .id
        .clone();
    let entry_local = entry.parts.by_id(protected_local_id);

    let mut final_owners = BTreeMap::new();
    for id in [
        "matrix/source-matrix/r0c0",
        linked_source_id,
        protected_local_id,
        linked_mirror_id,
    ] {
        let owner = entry
            .boards
            .iter()
            .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
            .map(|board| board.id.clone())
            .unwrap();
        final_owners.insert(id.to_owned(), owner);
    }
    let projection = Projection {
        matrix: Some(MatrixLink {
            source_matrix_id: entry_source_matrix.id.clone(),
            mirror_matrix_id: entry_mirror_matrix.id.clone(),
        }),
        preserve_matrix_geometry: false,
        source_board_id: "source-board".into(),
        mirror_board_id: "mirror-board".into(),
        mapping: fixture_mapping(),
        parts: vec![PartProjection {
            source_id: linked_source_id.into(),
            mirror_id: linked_mirror_id.into(),
        }],
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: BTreeSet::from([protected_local_id.into()]),
        final_owners: final_owners.clone(),
    };

    let entry_source_matrix_json = serde_json::to_value(&entry_source_matrix).unwrap();
    let entry_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
        .into_iter()
        .map(|id| serde_json::to_value(entry.parts.by_id(id)).unwrap())
        .collect();
    let entry_mirror_ids = entry_mirror_matrix.part_ids.clone();
    let entry_local_json = serde_json::to_value(entry_local).unwrap();
    let protected_references = serde_json::json!({
        "boards": &entry.boards,
        "nets": &entry.nets,
        "constraints": &entry.constraints,
        "boardOutlines": &entry.board_outlines,
        "definitions": &entry.definitions,
        "outline": &entry.outline,
    });
    let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

    assert_retained_net_is_valid(&document);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    assert_eq!(
        document.constraints.iter().find(|constraint| {
            matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-stationary-offset")
        }),
        Some(&retained_offset)
    );
    let actual_probe = document.parts.by_id(probe_id);
    assert_eq!(actual_probe, &entry_probe);
    let actual_probe_owner = document
        .boards
        .iter()
        .find(|board| board.part_ids.contains(&probe_id.to_owned()))
        .unwrap()
        .id
        .clone();
    assert_eq!(actual_probe_owner, entry_probe_owner);
    assert!(
        document
            .boards
            .by_id(&entry_probe_owner)
            .part_ids
            .contains(&probe_id.to_owned())
    );

    let actual_neighbor = document.parts.by_id(linked_mirror_id);
    assert!(
        (actual_neighbor.pose.at.x - 45.48221151584141).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected x position"
    );
    assert!(
        (actual_neighbor.pose.at.y - -46.956048106863605).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected y position"
    );
    assert!(
        (actual_neighbor.pose.rotation - -12.0).abs() < 1e-9,
        "the linked neighbor must receive the independently calculated reflected heading"
    );

    assert_eq!(
        serde_json::to_value(document.matrices.by_id("source-matrix")).unwrap(),
        entry_source_matrix_json
    );
    let actual_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
        .into_iter()
        .map(|id| serde_json::to_value(document.parts.by_id(id)).unwrap())
        .collect();
    assert_eq!(actual_source_parts_json, entry_source_parts_json);
    let actual_mirror_matrix = document.matrices.by_id("mirror-matrix");
    assert_eq!(actual_mirror_matrix.id, entry_mirror_matrix.id);
    assert_eq!(actual_mirror_matrix.part_ids, entry_mirror_ids);
    let mut rematerialized = entry.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut rematerialized, actual_mirror_matrix).unwrap();
    let rematerialized_matrix = rematerialized.matrices.by_id("mirror-matrix");
    assert_eq!(
        serde_json::to_value(actual_mirror_matrix).unwrap(),
        serde_json::to_value(rematerialized_matrix).unwrap(),
        "the final active matrix must already be a complete normalized materializer input"
    );
    let actual_retained = document.parts.by_id(protected_local_id);
    let rematerialized_retained = rematerialized.parts.by_id(protected_local_id);
    for pose in [&actual_retained.pose, &rematerialized_retained.pose] {
        assert!((pose.at.x - 0.0).abs() < 1e-9);
        assert!((pose.at.y - 0.0).abs() < 1e-9);
        assert!((pose.rotation - 0.0).abs() < 1e-9);
    }
    let mut actual_non_pose = serde_json::to_value(actual_retained).unwrap();
    let mut rematerialized_non_pose = serde_json::to_value(rematerialized_retained).unwrap();
    actual_non_pose.as_object_mut().unwrap().remove("pose");
    rematerialized_non_pose
        .as_object_mut()
        .unwrap()
        .remove("pose");
    assert_eq!(
        actual_non_pose, rematerialized_non_pose,
        "the retained member's identity and non-pose data must agree with rematerialization"
    );
    assert_eq!(
        serde_json::to_value(document.parts.by_id(protected_local_id)).unwrap(),
        entry_local_json
    );
    assert_eq!(
        serde_json::json!({
            "boards": &document.boards,
            "nets": &document.nets,
            "constraints": &document.constraints,
            "boardOutlines": &document.board_outlines,
            "definitions": &document.definitions,
            "outline": &document.outline,
        }),
        protected_references
    );

    let actual_final_owners: BTreeMap<_, _> = final_owners
        .keys()
        .map(|id| {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .map(|board| board.id.clone())
                .unwrap();
            (id.clone(), owner)
        })
        .collect();
    assert_eq!(actual_final_owners, final_owners);

    let mut expected_changed_ids = BTreeSet::new();
    if serde_json::to_value(&expected_mirror_matrix).unwrap()
        != serde_json::to_value(&entry_mirror_matrix).unwrap()
    {
        expected_changed_ids.insert(entry_mirror_matrix.id.clone());
    }
    let before = entry.parts.by_id(linked_mirror_id);
    let after = expected_document.parts.by_id(linked_mirror_id);
    if serde_json::to_value(before).unwrap() != serde_json::to_value(after).unwrap() {
        expected_changed_ids.insert(linked_mirror_id.into());
    }
    assert_eq!(
        changed_ids,
        expected_changed_ids.into_iter().collect::<Vec<_>>()
    );
    let before_second_reconcile = document.clone();
    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(document, before_second_reconcile);

    let before_rejected = serde_json::to_value(&document).unwrap();
    let unsupported = Projection {
        matrix: projection.matrix.clone(),
        preserve_matrix_geometry: projection.preserve_matrix_geometry,
        source_board_id: projection.source_board_id.clone(),
        mirror_board_id: projection.mirror_board_id.clone(),
        mapping: projection.mapping,
        parts: vec![],
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
        final_owners: projection.final_owners.clone(),
    };
    assert!(reconcile(&mut document, "pair-source-mirror", &unsupported).is_err());
    assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);

    let mismatched_pair = Projection {
        matrix: projection.matrix.clone(),
        preserve_matrix_geometry: projection.preserve_matrix_geometry,
        mapping: Pose2 {
            at: projection.mapping.at,
            rotation: projection.mapping.rotation + 1.0,
        },
        source_board_id: projection.source_board_id.clone(),
        mirror_board_id: projection.mirror_board_id.clone(),
        parts: projection
            .parts
            .iter()
            .map(|part| PartProjection {
                source_id: part.source_id.clone(),
                mirror_id: part.mirror_id.clone(),
            })
            .collect(),
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
        final_owners: projection.final_owners.clone(),
    };
    assert!(reconcile(&mut document, "pair-source-mirror", &mismatched_pair).is_err());
    assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);
}
