use super::*;

#[test]
fn membership_transition_expands_with_planned_pair_identities_and_references() {
    // Consolidated onto the shared expansion fixture: the planned source column,
    // identity maps, additions, and owners are sourced from the operation once
    // instead of re-typed here. All expectations below derive from that source.
    let (
        before,
        post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        additions,
        final_owners,
    ) = membership_expansion_fixture();
    let source_matrix = post_source.matrices.by_id("source-matrix");
    let before_associations = context.associations.len();

    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("a complete planned expansion is projected atomically");
    // Operation-sequence invariants, all derived from the source operation:
    // the mirror frame follows the source frame, every slot is an exact
    // row/column identity, and the association delta equals the addition count.
    let mirror = output.draft.matrices.by_id("mirror-matrix");
    assert_eq!(
        (mirror.rows, mirror.columns),
        (source_matrix.rows, source_matrix.columns),
        "mirror frame follows the source operation"
    );
    assert_exact_matrix_identities(&output.draft, "mirror-matrix");
    assert_exact_matrix_identities(&output.draft, "source-matrix");
    for addition in &additions {
        assert_eq!(
            output.draft.parts.by_id(&addition.mirror_id).reference,
            addition.mirror_reference
        );
    }
    assert_eq!(output.context_delta.added.len(), additions.len());
    assert_eq!(
        output.context.associations.len(),
        before_associations + additions.len()
    );
    assert_eq!(output.context.pair_right, context.pair_right);
}

#[test]
fn membership_transition_mutated_row_slot_breaks_exact_identity() {
    // Deliberate input mutation: retargeting one surviving source row proves the
    // transition checks exact row identities instead of accepting any shape.
    let (
        before,
        post_source,
        context,
        mut source_identity_map,
        mirror_identity_map,
        additions,
        final_owners,
    ) = membership_expansion_fixture();
    let mutated = source_identity_map
        .iter()
        .find(|(old, _)| *old == "matrix/source-matrix/r0c0")
        .map(|(_, new)| new.clone())
        .unwrap();
    source_identity_map.insert(
        "matrix/source-matrix/r0c0".into(),
        mutated.map(|_| "matrix/source-matrix/r1c0".into()),
    );
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect_err("a single mistyped row identity is refused");
    assert_membership_inputs_unchanged(&before, &snapshot, &context, &context_snapshot);
    assert!(
        error.contains("not injective") || error.contains("does not cover"),
        "row mutation is refused as an identity mismatch: {error}"
    );
}

#[test]
fn membership_transition_chains_two_by_three_expansion_and_two_by_two_trim() {
    let assert_pose = |document: &ProjectDoc, id: &str, expected: Pose2| {
        let actual = document.parts.by_id(id).pose;
        assert!(
            (actual.at.x - expected.at.x).abs() < 1e-9,
            "{id}: {actual:?}"
        );
        assert!(
            (actual.at.y - expected.at.y).abs() < 1e-9,
            "{id}: {actual:?}"
        );
        assert!(
            (actual.rotation - expected.rotation).abs() < 1e-9,
            "{id}: {actual:?}"
        );
    };
    let (mut before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut before,
        "source-matrix",
        3,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the source fixture narrows to three columns");
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut before,
        "mirror-matrix",
        3,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the mirror fixture narrows to three columns");
    let mut initial_source = before.matrices.by_id("source-matrix").clone();
    initial_source.origin = Vec2 { x: 3.0, y: -2.0 };
    initial_source.edge_gap = Some(Vec2 { x: 4.25, y: 6.5 });
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &initial_source)
        .expect("the initial source frame materializes at the approved 2x3 origin");
    let matrix_association = context
        .pair_right
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected_initial = project_matrix(
        before.matrices.by_id("source-matrix"),
        before.matrices.by_id("mirror-matrix"),
        matrix_association,
        context.mapping,
    )
    .expect("the initial mirror frame is derived from the source frame");
    let mut initial_mirror = projected_initial;
    assert_eq!(
        initial_mirror.edge_gap,
        Some(Vec2 { x: 4.25, y: 6.5 }),
        "matrix projection preserves a nondefault edge gap"
    );
    for (row, column, x, y) in [(0, 1, -93.0, 62.0), (1, 1, -113.0, 42.0)] {
        let cell = initial_mirror
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        cell.offset = Some(Vec2 { x, y });
        cell.rotation = Some(-90.0);
        let diode = cell.assemblies.by_id_mut("diode");
        diode.offset = Vec2 { x: 2.0, y: 3.0 };
        diode.rotation = Some(30.0);
        diode.side = Some(Side::Back);
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &initial_mirror)
        .expect("both Local keys and diode children are materialized independently");
    let mirror_members_now = before.matrices.by_id("mirror-matrix").part_ids.clone();
    before.nets.push(boardstudio_model::model::Net {
        id: "net-clean".into(),
        name: "PROBE OMITTED LINKED".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r0c2/diode".into(),
                pad_id: "pad-one".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    for (id, part_id, pad_id) in [
        ("net-local-r0", "matrix/mirror-matrix/r0c1/diode", "pad-one"),
        ("net-local-r1", "matrix/mirror-matrix/r1c1/diode", "pad-two"),
    ] {
        before.nets.push(boardstudio_model::model::Net {
            id: id.into(),
            name: format!("LOCAL DEPENDENCY {id}"),
            pins: vec![boardstudio_model::model::Pin {
                part_id: part_id.into(),
                pad_id: pad_id.into(),
            }],
        });
    }
    before.boards.by_id_mut("mirror-board").net_ids.extend([
        "net-clean".into(),
        "net-local-r0".into(),
        "net-local-r1".into(),
    ]);
    before
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "constraint-omit".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c2/diode".into(),
            offset: Vec2 { x: 95.0, y: -57.0 },
            rotation: 60.0,
        });
    for (id, target_part_id, offset) in [
        (
            "constraint-local-r0",
            "matrix/mirror-matrix/r0c1/diode",
            Vec2 { x: 38.0, y: 51.0 },
        ),
        (
            "constraint-local-r1",
            "matrix/mirror-matrix/r1c1/diode",
            Vec2 { x: 38.0, y: 71.0 },
        ),
    ] {
        before
            .constraints
            .push(boardstudio_model::model::Constraint::Offset {
                id: id.into(),
                source_part_id: "geometry-probe".into(),
                target_part_id: target_part_id.into(),
                offset,
                rotation: 30.0,
            });
    }
    let mut outline_members: Vec<_> = mirror_members_now.into_iter().collect();
    outline_members.push("geometry-probe".into());
    before
        .outline
        .push(boardstudio_model::model::OutlineFeature::PartEnvelope {
            id: "outline-mirror".into(),
            connections: vec![
                OutlineConnection {
                    id: "local-r0-connection".into(),
                    width: 1.25,
                    points: vec![OutlineControlPoint {
                        at: Vec2 { x: 2.0, y: 3.0 },
                        part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                    }],
                },
                OutlineConnection {
                    id: "local-r1-connection".into(),
                    width: 1.5,
                    points: vec![OutlineControlPoint {
                        at: Vec2 { x: 2.0, y: 3.0 },
                        part_id: Some("matrix/mirror-matrix/r1c1/diode".into()),
                    }],
                },
            ],
            settings: boardstudio_model::model::OutlineSettings {
                corners: boardstudio_model::model::CornerStyle::Fillet,
                size: 3.5,
                bridge_width: 7.25,
                repair: Some(boardstudio_model::model::OutlineRepairSettings {
                    enabled: true,
                    maximum_gap_span: 12.0,
                    minimum_connection_width: 1.25,
                    edge_clearance: 0.75,
                    keep_gaps: vec![boardstudio_model::model::ProtectedOutlineGap {
                        id: "local-r0-keep-gap".into(),
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 1.5, y: 2.5 },
                            part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                        }],
                    }],
                }),
            },
            part_ids: outline_members,
            margin: 1.75,
            operation: boardstudio_model::model::Operation::Add,
        });
    before
        .boards
        .by_id_mut("mirror-board")
        .outline_ids
        .push("outline-mirror".into());
    before.outline.retain(|feature| {
        !matches!(feature, boardstudio_model::model::OutlineFeature::PartEnvelope { id, .. } if id == "retained-keys-envelope")
    });
    before
        .boards
        .by_id_mut("mirror-board")
        .outline_ids
        .retain(|id| id != "retained-keys-envelope");
    if let Some(boardstudio_model::model::Constraint::Offset { offset, .. }) = before.constraints.iter_mut()
        .find(|constraint| matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset"))
    {
        *offset = Vec2 { x: 36.0, y: 68.0 };
    }
    let source_members = before.matrices.by_id("source-matrix").part_ids.clone();
    let mirror_members = before.matrices.by_id("mirror-matrix").part_ids.clone();
    let halves = before.first_halves_mut();
    halves.correspondences.retain(|association| {
        association.kind == MirroredItemKind::Matrix
            || source_members.contains(&association.source_id)
    });
    context.associations.retain(|association| {
        association.kind == MirroredItemKind::Matrix
            || source_members.contains(&association.source_id)
                && mirror_members.contains(&association.mirror_id)
    });
    context.pair_right = halves.clone();
    context
        .owners
        .retain(|id, _| before.parts.iter().any(|part| part.id == *id));
    for (source_id, state) in [
        ("matrix/source-matrix/r0c1", TransitionState::Local),
        ("matrix/source-matrix/r0c1/diode", TransitionState::Local),
        ("matrix/source-matrix/r1c1", TransitionState::Local),
        ("matrix/source-matrix/r1c1/diode", TransitionState::Local),
        ("matrix/source-matrix/r0c2", TransitionState::Omitted),
        ("matrix/source-matrix/r0c2/diode", TransitionState::Omitted),
    ] {
        let association = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == source_id)
            .expect("the three-column context contains each state");
        association.state = state;
        association.explicit_state = !source_id.ends_with("/diode");
    }
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());

    let local_ids = BTreeSet::from([
        "matrix/mirror-matrix/r0c1/diode".to_owned(),
        "matrix/mirror-matrix/r1c1/diode".to_owned(),
    ]);
    let mut local_keymap = KeymapConfiguration::default();
    local_keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c1/diode".into(),
        KeyBinding::KeyPress {
            keycode: "LOCAL-R0".into(),
        },
    );
    local_keymap.layers[0].sensors.insert(
        "matrix/mirror-matrix/r1c1/diode".into(),
        EncoderBinding {
            clockwise: KeyBinding::KeyPress {
                keycode: "LOCAL-R1-CW".into(),
            },
            counterclockwise: KeyBinding::KeyPress {
                keycode: "LOCAL-R1-CCW".into(),
            },
        },
    );
    before.keymap = Some(local_keymap);
    let mut local_keycaps = KeycapConfiguration::default();
    local_keycaps.keys.insert(
        "matrix/mirror-matrix/r0c1/diode".into(),
        KeycapKeySettings {
            legend: Some("LOCAL R0".into()),
            ..KeycapKeySettings::default()
        },
    );
    local_keycaps.keys.insert(
        "matrix/mirror-matrix/r1c1/diode".into(),
        KeycapKeySettings {
            legend: Some("LOCAL R1".into()),
            ..KeycapKeySettings::default()
        },
    );
    before.keycaps = Some(local_keycaps);
    let local_part_payloads: BTreeMap<_, _> = before
        .parts
        .iter()
        .filter(|part| local_ids.contains(&part.id))
        .map(|part| (part.id.clone(), part.clone()))
        .collect();
    let local_net_pins: BTreeMap<_, _> = before
        .nets
        .iter()
        .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
        .map(|net| (net.id.clone(), net.pins.clone()))
        .collect();
    let local_constraints: Vec<_> = before
        .constraints
        .iter()
        .filter(|constraint| {
            local_ids.contains(constraint.source()) || local_ids.contains(constraint.target())
        })
        .cloned()
        .collect();
    let (local_connections, local_outline_settings) = match before
        .outline
        .iter()
        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
        .unwrap()
    {
        OutlineFeature::PartEnvelope { connections, settings, .. } => {
            (connections.clone(), settings.clone())
        }
        _ => unreachable!(),
    };
    let local_bindings: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
        .bindings
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let local_sensors: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
        .sensors
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let local_keycap_settings: BTreeMap<_, _> = before
        .keycaps
        .as_ref()
        .unwrap()
        .keys
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let assert_local_retention = |document: &ProjectDoc, phase: &str| {
        for (id, expected) in &local_part_payloads {
            let actual: Vec<_> = document
                .parts
                .iter()
                .filter(|part| part.id == *id)
                .collect();
            assert_eq!(
                actual,
                vec![expected],
                "{phase}: full Local Part payload {id}"
            );
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.part_ids.contains(id))
                    .count(),
                1,
                "{phase}: Local board registration {id}"
            );
            assert_eq!(
                document
                    .matrices
                    .by_id("mirror-matrix")
                    .part_ids
                    .iter()
                    .filter(|member| *member == id)
                    .count(),
                1,
                "{phase}: Local matrix registration {id}"
            );
        }
        let actual_net_pins: BTreeMap<_, _> = document
            .nets
            .iter()
            .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
            .map(|net| (net.id.clone(), net.pins.clone()))
            .collect();
        assert_eq!(
            actual_net_pins, local_net_pins,
            "{phase}: Local dependency nets"
        );
        let actual_constraints: Vec<_> = document
            .constraints
            .iter()
            .filter(|constraint| {
                local_ids.contains(constraint.source()) || local_ids.contains(constraint.target())
            })
            .cloned()
            .collect();
        assert_eq!(
            actual_constraints, local_constraints,
            "{phase}: Local constraints"
        );
        let (actual_connections, actual_settings) = match document
            .outline
            .iter()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
            .unwrap()
        {
            OutlineFeature::PartEnvelope { connections, settings, .. } => (connections, settings),
            _ => unreachable!(),
        };
        assert_eq!(
            actual_connections, &local_connections,
            "{phase}: Local outline controls"
        );
        assert_eq!(
            actual_settings, &local_outline_settings,
            "{phase}: nondefault outline settings and Local keep-gap reference"
        );
        let actual_bindings: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
            .bindings
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_bindings, local_bindings,
            "{phase}: Local key bindings"
        );
        let actual_sensors: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
            .sensors
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_sensors, local_sensors,
            "{phase}: Local sensor bindings"
        );
        let actual_keycaps: BTreeMap<_, _> = document
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_keycaps, local_keycap_settings,
            "{phase}: Local keycap settings"
        );
    };
    assert_local_retention(&before, "before");

    let source_before = before.matrices.by_id("source-matrix");
    let mirror_before = before.matrices.by_id("mirror-matrix");
    let mut expanded_source = source_before.clone();
    expanded_source.columns = 4;
    expanded_source.origin = Vec2 { x: 5.0, y: 1.0 };
    for row in 0..2 {
        expanded_source.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 3,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 3.0 },
                rotation: Some(30.0),
                side: Some(Side::Back),
            }],
            assemblies_local: None,
        });
    }
    let mut expanded_source_document = before.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut expanded_source_document, &expanded_source)
        .expect("the post-source expansion is materialized");
    let source_identity_map: BTreeMap<_, _> = source_before
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = mirror_before
        .part_ids
        .iter()
        .map(|id| {
            let omitted =
                id == "matrix/mirror-matrix/r0c2" || id == "matrix/mirror-matrix/r0c2/diode";
            (id.clone(), (!omitted).then(|| id.clone()))
        })
        .collect();
    let mut additions = Vec::new();
    for row in 0..2 {
        let source_key = format!("matrix/source-matrix/r{row}c3");
        let mirror_key = format!("matrix/mirror-matrix/r{row}c3");
        for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
            (
                source_key.clone(),
                mirror_key.clone(),
                None,
                None,
                format!("CHAIN_M_R{row}C3"),
            ),
            (
                format!("{source_key}/diode"),
                format!("{mirror_key}/diode"),
                Some(source_key.clone()),
                Some(mirror_key.clone()),
                format!("CHAIN_M_D_R{row}C3"),
            ),
        ] {
            let source_part = expanded_source_document.parts.by_id(&source_id);
            additions.push(PlannedAddition {
                source_id,
                mirror_id,
                source_reference: source_part.reference.clone(),
                mirror_reference,
                parent_source_id,
                parent_mirror_id,
            });
        }
    }
    let omitted_key_id = "matrix/mirror-matrix/r0c2";
    let omitted_mirror_id = "matrix/mirror-matrix/r0c2/diode";
    let mut expanded_owners: BTreeMap<_, _> = expanded_source_document
        .parts
        .iter()
        .map(|part| {
            let owner = expanded_source_document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            (part.id.clone(), FinalOwner::Board(owner.id.clone()))
        })
        .collect();
    for addition in &additions {
        expanded_owners.insert(
            addition.mirror_id.clone(),
            FinalOwner::Board("mirror-board".into()),
        );
    }
    expanded_owners.insert(omitted_key_id.into(), FinalOwner::Absent);
    expanded_owners.insert(omitted_mirror_id.into(), FinalOwner::Absent);
    let expanded = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &expanded_source_document,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &expanded_owners,
        explicit_removal_closure: &BTreeSet::from([omitted_key_id.into(), omitted_mirror_id.into()]),
    })
    .expect("the approved expansion removes the initial Omitted companion and adds four planned identities");
    assert_local_retention(&expanded.draft, "expanded");
    let cleaned_net = expanded.draft.nets.by_id("net-clean");
    assert_eq!(
        cleaned_net.pins,
        vec![
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into()
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                pad_id: "pad-two".into()
            },
        ]
    );
    assert!(expanded.context.associations.iter().any(|association| {
        association.state == TransitionState::Omitted && association.mirror_id == omitted_mirror_id
    }));
    assert!(!expanded.draft.constraints.iter().any(|constraint| {
        matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "constraint-omit")
    }));
    let repeat_source_map: BTreeMap<_, _> = expanded
        .draft
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeat_mirror_map: BTreeMap<_, _> = expanded
        .draft
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &expanded.draft,
        post_source: &expanded.draft,
        context: &expanded.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &repeat_source_map,
        mirror_identity_map: &repeat_mirror_map,
        planned_additions: &[],
        final_owners: &expanded.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the exact private expansion state repeats without changes");
    assert_local_retention(&repeated.draft, "repeated expansion");
    assert_eq!(repeated.draft, expanded.draft);
    assert_eq!(repeated.context, expanded.context);
    assert!(repeated.changed_document_ids.is_empty());

    let mut shrink_before = expanded.draft.clone();
    shrink_before
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "constraint-linked".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r1c2/diode".into(),
            offset: Vec2 { x: 72.0, y: -59.0 },
            rotation: 60.0,
        });
    let probe = shrink_before.parts.by_id("geometry-probe");
    let linked_endpoint = shrink_before.parts.by_id("matrix/mirror-matrix/r1c2/diode");
    assert_eq!(
        probe.pose,
        Pose2 {
            at: Vec2 { x: 4.0, y: 2.0 },
            rotation: 0.0
        }
    );
    assert_pose(
        &shrink_before,
        "matrix/mirror-matrix/r1c2/diode",
        Pose2 {
            at: Vec2 { x: 76.0, y: -57.0 },
            rotation: 60.0,
        },
    );
    assert!((linked_endpoint.pose.at.x - probe.pose.at.x - 72.0).abs() < 1e-9);
    assert!((linked_endpoint.pose.at.y - probe.pose.at.y + 59.0).abs() < 1e-9);

    let mut trimmed_source = shrink_before.clone();
    let source_matrix = shrink_before.matrices.by_id("source-matrix");
    let mut trimmed = source_matrix.clone();
    trimmed.columns = 2;
    trimmed.cells.retain(|cell| cell.column < 2);
    boardstudio_core_kernel::matrix::set_matrix(&mut trimmed_source, &trimmed)
        .expect("the post-source trim is materialized");
    let source_identity_map: BTreeMap<_, _> = source_matrix
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "source-matrix",
                )
                .unwrap();
            (id.clone(), (column < 2).then(|| id.clone()))
        })
        .collect();
    let mirror_matrix = shrink_before.matrices.by_id("mirror-matrix");
    let mirror_identity_map: BTreeMap<_, _> = mirror_matrix
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "mirror-matrix",
                )
                .unwrap();
            (id.clone(), (column < 2).then(|| id.clone()))
        })
        .collect();
    let mut removal_closure = BTreeSet::new();
    for (id, mapped) in &source_identity_map {
        if mapped.is_none() {
            removal_closure.insert(id.clone());
            removal_closure.insert(id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
        }
    }
    removal_closure.insert(omitted_mirror_id.into());
    let mut final_owners = BTreeMap::new();
    for part in &shrink_before.parts {
        let removed = source_identity_map.get(&part.id) == Some(&None)
            || mirror_identity_map.get(&part.id) == Some(&None)
            || part.id == omitted_mirror_id;
        let owner = if removed {
            FinalOwner::Absent
        } else {
            let board = trimmed_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id));
            board
                .map(|board| FinalOwner::Board(board.id.clone()))
                .unwrap_or(FinalOwner::Absent)
        };
        final_owners.insert(part.id.clone(), owner);
    }
    for id in expanded.context.owners.keys() {
        final_owners.entry(id.clone()).or_insert(FinalOwner::Absent);
    }
    let trimmed = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &shrink_before,
        post_source: &trimmed_source,
        context: &expanded.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &removal_closure,
    })
    .expect("the private expansion draft and full context chain into the two-column trim");
    assert_local_retention(&trimmed.draft, "trimmed");
    let final_mirror = trimmed.draft.matrices.by_id("mirror-matrix");
    assert_eq!((final_mirror.rows, final_mirror.columns), (2, 2));
    assert_eq!(final_mirror.part_ids.len(), 8);
    assert_eq!(
        trimmed
            .draft
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/source-matrix/"))
            .count(),
        8
    );
    assert_eq!(
        trimmed
            .draft
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/mirror-matrix/"))
            .count(),
        8
    );
    assert_eq!(trimmed.context_delta.retired.len(), 8);
    let expected_shrink_ids: BTreeSet<_> = [
        "constraint-linked",
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
        "matrix/mirror-matrix/r1c2",
        "matrix/mirror-matrix/r1c2/diode",
        "matrix/mirror-matrix/r1c3",
        "matrix/mirror-matrix/r1c3/diode",
        "mirror-board",
        "mirror-matrix",
        "net-clean",
        "outline-mirror",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(
        trimmed
            .changed_document_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_shrink_ids
    );
    assert_eq!(trimmed.changed_document_ids.len(), 11);
    assert_eq!(
        trimmed
            .final_owners
            .keys()
            .filter(|id| id.starts_with("matrix/source-matrix/")
                || id.starts_with("matrix/mirror-matrix/"))
            .count(),
        32
    );
    assert_eq!(trimmed.context.associations.len(), 9);
    assert_eq!(
        trimmed
            .context
            .associations
            .iter()
            .filter(|association| association.state == TransitionState::Linked)
            .count(),
        5
    );
    assert_eq!(
        trimmed
            .context
            .associations
            .iter()
            .filter(|association| association.state == TransitionState::Local)
            .count(),
        4
    );
    assert_eq!(
        trimmed.draft.nets.by_id("net-clean").pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into()
        },]
    );
    for id in [
        "matrix/mirror-matrix/r0c1/diode",
        "matrix/mirror-matrix/r1c1/diode",
    ] {
        assert!(trimmed.draft.parts.iter().any(|part| part.id == id));
        assert!(trimmed.context.associations.iter().any(|association| {
            association.mirror_id == id && association.state == TransitionState::Local
        }));
    }
    for id in [
        "matrix/source-matrix/r0c1/diode",
        "matrix/source-matrix/r1c1/diode",
    ] {
        let association = trimmed
            .context
            .associations
            .iter()
            .find(|association| association.source_id == id)
            .unwrap();
        assert_eq!(association.state, TransitionState::Local);
        assert!(!association.explicit_state, "{id} remains inherited");
    }
    let retired_omitted_child = trimmed
        .context_delta
        .retired
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
        .unwrap();
    assert_eq!(retired_omitted_child.state, TransitionState::Omitted);
    assert!(!retired_omitted_child.explicit_state);
    assert_eq!(trimmed.context.pair_right, context.pair_right);
    let final_source = trimmed.draft.matrices.by_id("source-matrix");
    let repeat_source_map: BTreeMap<_, _> = final_source
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeat_mirror_map: BTreeMap<_, _> = final_mirror
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &trimmed.draft,
        post_source: &trimmed.draft,
        context: &trimmed.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &repeat_source_map,
        mirror_identity_map: &repeat_mirror_map,
        planned_additions: &[],
        final_owners: &trimmed.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the exact post-shrink draft and context repeat without changes");
    assert_local_retention(&repeated.draft, "repeated trim");
    assert_eq!(repeated.draft, trimmed.draft);
    assert_eq!(repeated.context, trimmed.context);
    assert!(repeated.changed_document_ids.is_empty());
}
