use super::*;

#[test]
fn membership_transition_trims_only_explicitly_removed_linked_closures() {
    let (mut before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    for (source_id, state) in [
        ("matrix/source-matrix/r0c0/diode", TransitionState::Local),
        ("matrix/source-matrix/r1c1", TransitionState::Local),
        ("matrix/source-matrix/r0c3", TransitionState::Local),
        ("matrix/source-matrix/r0c2", TransitionState::Omitted),
    ] {
        let mirror_id = source_id.replace("source-matrix", "mirror-matrix");
        for association in &mut context.associations {
            if (association.source_id == source_id && association.mirror_id == mirror_id)
                || (association.parent_source_id.as_deref() == Some(source_id)
                    && association.parent_mirror_id.as_deref() == Some(mirror_id.as_str()))
            {
                if association.explicit_state && association.source_id != source_id {
                    continue;
                }
                association.state = state;
                association.explicit_state = association.source_id == source_id;
                context.owners.insert(
                    association.mirror_id.clone(),
                    match state {
                        TransitionState::Local => {
                            FinalOwner::Board(context.mirror_board_id.clone())
                        }
                        TransitionState::Omitted => FinalOwner::Absent,
                        TransitionState::Linked => {
                            FinalOwner::Board(context.mirror_board_id.clone())
                        }
                    },
                );
            }
        }
        context.pair_right.exceptions.push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id,
            state: match state {
                TransitionState::Local => MirroredExceptionState::Local,
                TransitionState::Omitted => MirroredExceptionState::Omitted,
                TransitionState::Linked => unreachable!(),
            },
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    }
    before.first_halves_mut().clone_from(&context.pair_right);
    let mut sentinel_matrix = before.matrices.by_id("mirror-matrix").clone();
    for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)] {
        let cell = sentinel_matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        let assembly = cell.assemblies.by_id_mut("diode");
        assembly.offset = Vec2 { x, y };
        assembly.rotation = Some(rotation);
        assembly.side = Some(Side::Front);
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &sentinel_matrix)
        .expect("Local child sentinels remain a valid matrix");
    let omitted_mirror_ids = [
        "matrix/mirror-matrix/r0c2",
        "matrix/mirror-matrix/r0c2/diode",
    ];
    before
        .parts
        .retain(|part| !omitted_mirror_ids.contains(&part.id.as_str()));
    before
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|part_id| !omitted_mirror_ids.contains(&part_id.as_str()));
    for omitted_mirror_id in omitted_mirror_ids {
        assert!(before.parts.iter().all(|part| part.id != omitted_mirror_id));
        assert!(before.boards.iter().all(|board| {
            !board
                .part_ids
                .iter()
                .any(|part_id| part_id == omitted_mirror_id)
        }));
    }
    assert_eq!(
        context
            .pair_right
            .exceptions
            .iter()
            .filter(|exception| {
                context.pair_right.overlay_source(exception) == Some("matrix/source-matrix/r0c2")
                    && exception.mirror_id == "matrix/mirror-matrix/r0c2"
                    && exception.state == MirroredExceptionState::Omitted
            })
            .count(),
        1,
        "the owner record alone expresses the inherited Omitted closure"
    );
    assert!(context.pair_right.exceptions.iter().all(|exception| {
        context.pair_right.overlay_source(exception) != Some("matrix/source-matrix/r0c2/diode")
    }));
    assert!(
        context.pair_right.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && record.source_id == "matrix/source-matrix/r0c2/diode"
                && record.mirror_id == "matrix/mirror-matrix/r0c2/diode"
                && record.companion_mode == Some(MirroredCompanionMode::LiteralMirror)
        }),
        "the inherited child keeps its correspondence and per-item projection settings"
    );
    before.nets.push(Net {
        id: "retained-local-net".into(),
        name: "RETAINED LOCAL NET".into(),
        pins: vec![Pin {
            part_id: "matrix/mirror-matrix/r0c3/diode".into(),
            pad_id: "pad-two".into(),
        }],
    });
    before
        .boards
        .by_id_mut("mirror-board")
        .net_ids
        .push("retained-local-net".into());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the exact parent Omitted association owns its absent child closure");
    let source_before = before.matrices.by_id("source-matrix");
    let mirror_before = before.matrices.by_id("mirror-matrix");
    let mut post_source = before.clone();
    let mut trimmed = source_before.clone();
    trimmed.columns = 2;
    trimmed.cells.retain(|cell| cell.column < 2);
    boardstudio_core_kernel::matrix::set_matrix(&mut post_source, &trimmed)
        .expect("the source dimension trim is materialized");

    let mut source_identity_map = BTreeMap::new();
    for id in &source_before.part_ids {
        let (_, column, _) =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                id,
                "source-matrix",
            )
            .unwrap();
        source_identity_map.insert(id.clone(), (column < 2).then(|| id.clone()));
    }
    let mirror_identity_map: BTreeMap<_, _> = mirror_before
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "mirror-matrix",
                )
                .unwrap();
            let locally_retained = id.starts_with("matrix/mirror-matrix/r0c3");
            let omitted = context.associations.iter().any(|association| {
                association.mirror_id == *id && association.state == TransitionState::Omitted
            });
            (
                id.clone(),
                ((column < 2 || locally_retained) && !omitted).then(|| id.clone()),
            )
        })
        .collect();
    let mut explicit_removal_closure = BTreeSet::new();
    for (source_id, target_id) in &source_identity_map {
        if target_id.is_none() {
            explicit_removal_closure.insert(source_id.clone());
            explicit_removal_closure
                .insert(source_id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
        }
    }
    let mut final_owners = context.owners.clone();
    for part in &before.parts {
        let locally_retained = context.associations.iter().any(|association| {
            association.state == TransitionState::Local
                && association.mirror_id == part.id
                && mirror_identity_map
                    .get(&part.id)
                    .is_some_and(Option::is_some)
        });
        let is_removed_matrix_part = (source_identity_map.get(&part.id) == Some(&None)
            || mirror_identity_map.get(&part.id) == Some(&None))
            && !locally_retained;
        let owner = if is_removed_matrix_part {
            FinalOwner::Absent
        } else {
            let board = post_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id));
            board
                .map(|board| FinalOwner::Board(board.id.clone()))
                .unwrap_or(FinalOwner::Absent)
        };
        final_owners.insert(part.id.clone(), owner);
    }

    let output = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    })
    .expect("the explicit trim removes the corresponding target closures");
    let mirror = output.draft.matrices.by_id("mirror-matrix");
    assert_eq!((mirror.rows, mirror.columns), (2, 2));
    assert_eq!(mirror.part_ids.len(), 8);
    assert_eq!(output.context_delta.retired.len(), 8);
    assert_eq!(output.context.associations.len(), 9);
    for removed_source_id in [
        "matrix/source-matrix/r0c2",
        "matrix/source-matrix/r0c2/diode",
        "matrix/source-matrix/r0c3",
        "matrix/source-matrix/r0c3/diode",
    ] {
        assert!(
            output
                .context
                .pair_right
                .exceptions
                .iter()
                .all(|exception| {
                    exception.mirror_id
                        != removed_source_id.replace("source-matrix", "mirror-matrix")
                }),
            "source removal cleans only the exception for removed item {removed_source_id}"
        );
        assert!(
            output
                .context
                .pair_right
                .correspondences
                .iter()
                .all(|record| { record.source_id != removed_source_id })
        );
        assert!(
            output
                .context
                .associations
                .iter()
                .all(|association| { association.source_id != removed_source_id })
        );
    }
    for id in [
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
    ] {
        assert!(
            output.draft.parts.iter().any(|part| part.id == id),
            "Local geometry {id} is retained"
        );
        assert!(
            output
                .draft
                .boards
                .by_id("mirror-board")
                .part_ids
                .contains(&id.to_owned())
        );
        assert!(
            !mirror.part_ids.contains(&id.to_owned()),
            "out-of-bounds Local geometry is detached from matrix membership"
        );
        assert_eq!(
            output.final_owners.get(id),
            Some(&FinalOwner::Board("mirror-board".into()))
        );
    }
    assert_eq!(
        output.draft.nets.by_id("retained-local-net").pins,
        vec![Pin {
            part_id: "matrix/mirror-matrix/r0c3/diode".into(),
            pad_id: "pad-two".into(),
        }]
    );
    for row in 0..2 {
        for column in 0..2 {
            let id = format!("matrix/mirror-matrix/r{row}c{column}");
            let before_part = before.parts.by_id(&id);
            let after_part = output.draft.parts.by_id(&id);
            assert_eq!(after_part.properties, before_part.properties);
            assert_eq!(
                after_part.generator_parameters,
                before_part.generator_parameters
            );
        }
        for column in 2..4 {
            if row == 0 && column == 3 {
                continue;
            }
            for id in [
                format!("matrix/mirror-matrix/r{row}c{column}"),
                format!("matrix/mirror-matrix/r{row}c{column}/diode"),
            ] {
                assert!(!output.draft.parts.iter().any(|part| part.id == id));
                assert_eq!(output.final_owners.get(&id), Some(&FinalOwner::Absent));
            }
        }
    }
    for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)] {
        let cell = mirror
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        let assembly = cell.assemblies.by_id("diode");
        assert_eq!(assembly.offset, Vec2 { x, y });
        assert_eq!(assembly.rotation, Some(rotation));
        assert_eq!(assembly.side, Some(Side::Front));
        let id = format!("matrix/mirror-matrix/r{row}c{column}/diode");
        let before_part = before.parts.by_id(&id);
        let after_part = output.draft.parts.by_id(&id);
        assert_eq!(
            after_part, before_part,
            "Local child payload {id} is preserved"
        );
    }
    let round_tripped: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
    assert_eq!(round_tripped, output.draft);
    assert_eq!(round_tripped.first_halves(), &output.context.pair_right);
    let repeated_source_map: BTreeMap<_, _> = round_tripped
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated_mirror_map: BTreeMap<_, _> = round_tripped
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &round_tripped,
        post_source: &round_tripped,
        context: &output.context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &repeated_source_map,
        mirror_identity_map: &repeated_mirror_map,
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("source retirement converges after a document round trip");
    assert_eq!(repeated.draft, round_tripped);
    assert_eq!(repeated.context, output.context);
    assert!(repeated.changed_document_ids.is_empty());
}

#[test]
fn membership_transition_compacts_ids_and_carries_logical_payload_and_settings() {
    let (mut before, _, context, identity_map, final_owners, explicit_closure) =
        membership_compaction_fixture().expect("the materialized compaction fixture is valid");
    add_membership_cleanup_sentinels(&mut before);
    let mut post_source = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post_source,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the source operation remaps populated dependency sentinels");
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("populated before evidence remains canonical");
    let mirror_identity_map: BTreeMap<_, _> = identity_map
        .iter()
        .map(|(old, new)| {
            let mirror_old = old.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
            let mirror_new = new
                .as_ref()
                .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
            (mirror_old, mirror_new)
        })
        .collect();
    let additions = [];
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_closure,
    })
    .expect("the explicit axis-removal operation compacts the mirror");

    let removed = "matrix/mirror-matrix/r0c1/diode";
    let shifted = "matrix/mirror-matrix/r0c2/diode";
    assert_eq!(
        output.draft.keymap.as_ref().unwrap().layers[0]
            .bindings
            .get(removed),
        Some(&KeyBinding::KeyPress {
            keycode: "B".into()
        }),
        "the shifted identity carries its payload across the removed key"
    );
    assert_eq!(
        output.draft.keymap.as_ref().unwrap().layers[0]
            .sensors
            .get(removed)
            .unwrap()
            .clockwise,
        KeyBinding::KeyPress {
            keycode: "E".into()
        }
    );
    assert!(
        !output.draft.keymap.as_ref().unwrap().layers[0]
            .sensors
            .contains_key(shifted)
    );
    assert!(
        !output
            .draft
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .contains_key(shifted)
    );
    assert_eq!(
        output
            .draft
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
        output.draft.hardware.as_ref().unwrap().instances[1].controller_part_id,
        None
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .locks
            .get(&format!("direct/{removed}"))
            .map(String::as_str),
        None
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .assignments
            .get(&format!("link/{removed}"))
            .map(String::as_str),
        Some("assignment sentinel")
    );
    assert!(
        !output.draft.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .contains_key(shifted)
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .get(removed)
            .map(String::as_str),
        Some("binding sentinel")
    );
    assert_eq!(output.draft.embedded_circuits[0].part_ids, vec![removed]);
    assert!(
        output
            .draft
            .modules
            .by_id("closure-module")
            .connection
            .is_none()
    );
    assert_eq!(
        output
            .draft
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
    } = output
        .draft
        .outline
        .iter()
        .find(|feature| feature.id() == "closure-envelope")
        .unwrap()
    else {
        panic!("the closure envelope remains a part envelope");
    };
    let current_members: BTreeSet<_> = output
        .draft
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
    let outline = output
        .draft
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

    let mirror = output.draft.matrices.by_id("mirror-matrix");
    assert_eq!((mirror.rows, mirror.columns), (2, 3));
    assert_eq!(mirror.part_ids.len(), 12);
    let d_diode = output.draft.parts.by_id("matrix/mirror-matrix/r0c2/diode");
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
    let moved_diode = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
        .unwrap();
    assert_eq!(moved_diode.mirror_id, "matrix/mirror-matrix/r0c2/diode");
    assert_eq!(
        moved_diode.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirrorWithTweak)
    );
    assert_eq!(
        moved_diode.raw_tweak,
        Some(MirroredCompanionTweak {
            x: 1.0,
            y: -2.0,
            rotation: 15.0
        })
    );
    let preserved_a = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    assert_eq!(preserved_a.raw_mode_override, None);
    assert_eq!(
        preserved_a.effective_mode,
        MirroredCompanionMode::LiteralMirror
    );
    let preserved_c = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
        .unwrap();
    assert_eq!(
        preserved_c.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirror)
    );
    assert_eq!(preserved_c.raw_tweak, None);
    let retired_b = output
        .context_delta
        .retired
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
        .unwrap();
    assert_eq!(
        retired_b.raw_mode_override,
        Some(MirroredCompanionMode::DuplicatePosition)
    );
    assert!(
        output
            .changed_document_ids
            .contains(&"mirror-board".to_owned())
    );
    let expected_changed_ids: BTreeSet<_> = std::iter::once("mirror-board".to_owned())
        .chain(std::iter::once("mirror-matrix".to_owned()))
        .chain([
            "closure-circuit".to_owned(),
            "closure-envelope".to_owned(),
            "closure-module".to_owned(),
            "retained-second-key-net".to_owned(),
            "retained-second-key-offset".to_owned(),
            "retained-keys-envelope".to_owned(),
            "shifted-module".to_owned(),
        ])
        .chain((0..2).flat_map(|row| {
            (1..4).flat_map(move |column| {
                let key = format!("matrix/mirror-matrix/r{row}c{column}");
                [key.clone(), format!("{key}/diode")]
            })
        }))
        .collect();
    assert_eq!(
        output
            .changed_document_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_changed_ids
    );
    assert_eq!(output.context_delta.retired.len(), 4);
    assert_eq!(output.context_delta.relocated.len(), 8);
    assert_eq!(output.context.associations.len(), 13);
    assert_eq!(output.context_delta.owners, output.final_owners);
    assert_eq!(output.draft.definitions, post_source.definitions);
    assert_eq!(
        output
            .draft
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe"),
        post_source
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe")
    );
    let retained_net = output.draft.nets.by_id("retained-second-key-net");
    assert_eq!(retained_net.name, "RETAINED SECOND KEY");
    assert_eq!(
        retained_net.pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into(),
        }]
    );
    assert!(!output.draft.constraints.iter().any(|constraint| {
        matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset")
    }));
    for id in [
        "matrix/source-matrix/r0c3",
        "matrix/source-matrix/r0c3/diode",
        "matrix/source-matrix/r1c3",
        "matrix/source-matrix/r1c3/diode",
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
        "matrix/mirror-matrix/r1c3",
        "matrix/mirror-matrix/r1c3/diode",
    ] {
        assert_eq!(
            output.final_owners.get(id),
            Some(&FinalOwner::Absent),
            "{id}"
        );
    }

    let remapped = output.context.clone();
    let next_map: BTreeMap<_, _> = output
        .draft
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let next_mirror_map: BTreeMap<_, _> = output
        .draft
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &output.draft,
        post_source: &output.draft,
        context: &remapped,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &next_map,
        mirror_identity_map: &next_mirror_map,
        planned_additions: &additions,
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the chained private state is valid for an identity repeat");
    assert_eq!(repeated.draft, output.draft);
    assert_eq!(repeated.context, output.context);
    assert!(repeated.changed_document_ids.is_empty());
}

#[test]
fn independent_transition_rejects_wrong_board_final_owner() {
    let (before, post_source, context, source_map, mut final_owners, closure) =
        membership_compaction_fixture().unwrap();
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let id = "matrix/mirror-matrix/r1c1/diode";
    final_owners.insert(id.into(), FinalOwner::Board("source-board".into()));
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_err(),
        "a contradictory final owner for the second-row companion must be rejected"
    );
}

#[test]
fn independent_transition_rejects_inconsistent_raw_effective_settings() {
    let (before, post_source, mut context, source_map, final_owners, closure) =
        membership_compaction_fixture().unwrap();
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let child = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == "matrix/source-matrix/r0c3/diode")
        .unwrap();
    assert_eq!(
        child.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirrorWithTweak)
    );
    assert_eq!(
        child.raw_tweak,
        Some(MirroredCompanionTweak {
            x: 1.0,
            y: -2.0,
            rotation: 15.0
        })
    );
    child.effective_mode = MirroredCompanionMode::DuplicatePosition;
    child.effective_tweak = None;
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_err(),
        "effective settings must agree with the preserved raw override and tweak"
    );
}

#[test]
fn membership_transition_rejects_missing_second_retained_child_owner_atomically() {
    let (before, post_source, context, source_map, mut final_owners, closure) =
        membership_compaction_fixture().expect("the materialized compaction fixture is valid");
    let missing = "matrix/mirror-matrix/r1c1/diode";
    assert!(final_owners.remove(missing).is_some());
    let before_snapshot = before.clone();
    let post_source_snapshot = post_source.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect(),
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(result.is_err());
    assert_eq!(before, before_snapshot);
    assert_eq!(post_source, post_source_snapshot);
    assert_eq!(context, context_snapshot);
}

#[test]
fn repair_fresh_refuses_invalid_before_pad_identity() {
    let (mut before, _, context, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
    let net = before.nets.by_id_mut("retained-second-key-net");
    assert_eq!(net.pins[0].pad_id, "pad-one");
    net.pins[0].pad_id = "1".into();
    let part = before
        .parts
        .iter()
        .find(|p| p.id == net.pins[0].part_id)
        .unwrap();
    let definition = before.definitions.by_id(&part.definition_id);
    assert!(definition.pads.iter().any(|p| p.number == "1"));
    assert!(!definition.pads.iter().any(|p| p.id == "1"));
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let source_map = before
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = before
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &context.owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "a touched historical net referencing Pad.number instead of actual Pad.id must refuse before staging"
    );
}

#[test]
fn repair_fresh_refuses_chained_identity_ambiguous_actual_board_owner() {
    let (before, post, context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let output = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .expect("establish an actually returned private draft and full context first");
    let mut invalid = output.draft.clone();
    let second_child = "matrix/mirror-matrix/r1c1/diode";
    invalid
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(second_child.into());
    assert_eq!(
        invalid
            .boards
            .iter()
            .filter(|b| b.part_ids.iter().any(|id| id == second_child))
            .count(),
        2
    );
    let source_map = invalid
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = invalid
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let snapshot = invalid.clone();
    let context_snapshot = output.context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &invalid,
        post_source: &invalid,
        context: &output.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(invalid, snapshot);
    assert_eq!(output.context, context_snapshot);
    assert!(
        result.is_err(),
        "a chained no-op must validate the second child has exactly one actual board owner"
    );
}

#[test]
fn repair_fresh_refuses_wrong_historical_owner_context() {
    let (before, post, mut context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let second_child = "matrix/mirror-matrix/r1c1/diode";
    assert_eq!(
        context.owners[second_child],
        FinalOwner::Board("mirror-board".into())
    );
    context.owners.insert(
        second_child.into(),
        FinalOwner::Board("source-board".into()),
    );
    assert!(
        before
            .boards
            .by_id("mirror-board")
            .part_ids
            .iter()
            .any(|id| id == second_child)
    );
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let before_snapshot = before.clone();
    let post_snapshot = post.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    });
    assert_eq!(before, before_snapshot);
    assert_eq!(post, post_snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "the valid-before owner context must match the historical board registration, even when final owners are correct"
    );
}

#[test]
fn repair_fresh_omission_cleans_idless_dependencies() {
    let (mut before, _, mut context, source_map, mut owners, mut closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let parent = "matrix/source-matrix/r0c0";
    let target_parent = "matrix/mirror-matrix/r0c0";
    let target_child = "matrix/mirror-matrix/r0c0/diode";
    for association in context
        .associations
        .iter_mut()
        .filter(|a| a.source_id == parent || a.parent_source_id.as_deref() == Some(parent))
    {
        association.state = TransitionState::Omitted;
        association.explicit_state = association.parent_source_id.is_none();
    }
    let mut keymap = KeymapConfiguration::default();
    keymap.layers[0].bindings.insert(
        target_child.into(),
        KeyBinding::KeyPress {
            keycode: "OMITTED_SENTINEL".into(),
        },
    );
    before.keymap = Some(keymap);
    let mut post = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .unwrap();
    let mut mirror_map: BTreeMap<_, _> = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    for id in [target_parent, target_child] {
        mirror_map.insert(id.into(), None);
        owners.insert(id.into(), FinalOwner::Absent);
        closure.insert(id.into());
    }
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .expect("a complete explicit Omitted closure under the accepted source operation is supported");
    assert!(!output.draft.parts.iter().any(|p| p.id == target_child));
    assert_eq!(output.final_owners[target_child], FinalOwner::Absent);
    assert!(
        !output.draft.keymap.as_ref().unwrap().layers[0]
            .bindings
            .contains_key(target_child),
        "omitted geometry removal must clean populated keymap dependencies too"
    );
}

#[test]
fn repair_fresh_accepts_shifted_local_parent_with_explicit_omitted_child() {
    let (before, post, mut context, source_map, mut owners, mut closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let parent = "matrix/source-matrix/r1c2";
    let target_child = "matrix/mirror-matrix/r1c2/diode";
    let parent_record = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == parent)
        .unwrap();
    parent_record.state = TransitionState::Local;
    parent_record.explicit_state = true;
    let child = context
        .associations
        .iter_mut()
        .find(|a| a.parent_source_id.as_deref() == Some(parent))
        .unwrap();
    child.state = TransitionState::Omitted;
    child.explicit_state = true;
    assert_eq!(
        child.parent_mirror_id.as_deref(),
        Some("matrix/mirror-matrix/r1c2")
    );
    let mut mirror_map: BTreeMap<_, _> = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    mirror_map.insert(target_child.into(), None);
    owners.insert("matrix/mirror-matrix/r1c1/diode".into(), FinalOwner::Absent);
    closure.insert(target_child.into());
    let result = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_ok(),
        "a valid explicit Omitted child override must survive its shifted Local parent compaction policy: {:?}",
        result.err()
    );
}

#[test]
fn repair_fresh_refuses_native_and_legacy_matrix_overlap() {
    let (mut before, _, mut context, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
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
    for part in &before.parts {
        let board = before
            .boards
            .iter()
            .find(|b| b.part_ids.contains(&part.id))
            .unwrap();
        context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
    }
    let source_map = before
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = before
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &context.owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert!(
        result.is_err(),
        "a matrix with both valid native and same-board legacy links must refuse this transition"
    );
}

#[test]
fn cutover_replay_compensation_survives_shifted_local_key() {
    let (before, post, mut context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let source = "matrix/source-matrix/r1c2";
    for a in context
        .associations
        .iter_mut()
        .filter(|a| a.source_id == source || a.parent_source_id.as_deref() == Some(source))
    {
        a.state = TransitionState::Local;
        a.explicit_state = a.source_id == source;
    }
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .unwrap();
    let mut regenerated = output.draft.clone();
    let matrix = output.draft.matrices.by_id("mirror-matrix");
    boardstudio_core_kernel::matrix::set_matrix(&mut regenerated, matrix).unwrap();
    for suffix in ["", "/diode"] {
        let old = format!("matrix/mirror-matrix/r1c2{suffix}");
        let new = format!("matrix/mirror-matrix/r1c1{suffix}");
        let expected = before.parts.by_id(&old).pose;
        for doc in [&output.draft, &regenerated] {
            assert_eq!(doc.parts.by_id(&new).pose, expected, "{new}");
        }
    }
}
