use super::*;

#[test]
fn repair_fresh_refuses_invalid_planned_additions_table() {
    // Table-driven: each row corrupts the planned additions differently;
    // every corruption must be refused before staging.
    for (name, refusal) in [
        (
            "unbound planned source reference",
            "planned source reference must bind the actual post-source Part",
        ),
        (
            "missing planned child parent",
            "planned child parent closure must exist and match before staging",
        ),
        (
            "duplicate planned mirror references",
            "distinct planned additions cannot publish duplicate references",
        ),
    ] {
        let (
            before,
            post_source,
            context,
            source_identity_map,
            mirror_identity_map,
            mut additions,
            final_owners,
        ) = membership_expansion_fixture();
        match name {
            "unbound planned source reference" => {
                additions[0].source_reference = "SUPPLIED_BUT_NOT_ACTUAL_SOURCE_REFERENCE".into();
            }
            "missing planned child parent" => {
                additions[1].parent_source_id = Some("matrix/source-matrix/r0c99".into());
                additions[1].parent_mirror_id = Some("matrix/mirror-matrix/r0c99".into());
            }
            "duplicate planned mirror references" => {
                additions[2].mirror_reference = additions[0].mirror_reference.clone();
            }
            _ => unreachable!("unknown planned-addition case {name}"),
        }
        let result = prepare_membership_transition(MembershipTransitionInput {
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
        });
        assert!(result.is_err(), "{name}: {refusal}");
    }
}

#[test]
fn repair_fresh_final_diff_reports_real_module_and_circuit_ids() {
    let (mut before, _, _, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
    add_membership_cleanup_sentinels(&mut before);
    let mut after = before.clone();
    after
        .modules
        .by_id_mut("shifted-module")
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "matrix/mirror-matrix/r0c1/diode".into();
    after
        .embedded_circuits
        .by_id_mut("closure-circuit")
        .part_ids = vec!["matrix/mirror-matrix/r0c1/diode".into()];
    let (ids, paths) = changed_document_fields(&before, &after).unwrap();
    assert!(
        ids.contains(&"shifted-module".into()) && ids.contains(&"closure-circuit".into()),
        "changed dependency objects with actual IDs need real IDs, while idless maps use paths: IDs={ids:?}, paths={paths:?}"
    );
}

#[test]
fn repair_fresh_final_addition_copies_nondefault_source_parameters() {
    let (
        before,
        mut post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        additions,
        final_owners,
    ) = membership_expansion_fixture();

    let source_id = additions[0].source_id.clone();
    let mirror_id = additions[0].mirror_id.clone();
    let source = post_source.parts.by_id_mut(&source_id);
    source.properties = Some(BTreeMap::from([(
        "new-source-label".into(),
        serde_json::json!("ADDED SOURCE LABEL"),
    )]));
    source.generator_parameters = Some(BTreeMap::from([(
        "new-source-parameter".into(),
        serde_json::json!(17),
    )]));
    source.outline = Some(boardstudio_model::model::PartOutline {
        excluded: true,
        margin: Some(2.75),
        allow_body_overhang: true,
    });
    let source_snapshot = source.clone();
    assert!(!source_snapshot.properties.as_ref().unwrap().is_empty());
    assert!(
        !source_snapshot
            .generator_parameters
            .as_ref()
            .unwrap()
            .is_empty()
    );
    let output = prepare_membership_transition(MembershipTransitionInput {
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
    .unwrap();
    let mirror = output.draft.parts.by_id(&mirror_id);
    assert_eq!(
        (
            &mirror.properties,
            &mirror.generator_parameters,
            &mirror.outline,
        ),
        (
            &source_snapshot.properties,
            &source_snapshot.generator_parameters,
            &source_snapshot.outline,
        ),
        "new linked materialized Part must copy nondefault labels, parameters, and PartOutline"
    );
}

#[test]
fn consolidated_fresh_refusals_preserve_history_and_exact_identity() {
    type Mutation = fn(&mut ProjectDoc, &mut MembershipContext);
    let cases: [(&str, Mutation); 5] = [
        ("must reference exactly one actual Part", |before, _| {
            let Constraint::Offset { target_part_id, .. } = before
                .constraints
                .iter_mut()
                .find(|c| c.id() == "retained-second-key-offset")
                .unwrap()
            else {
                unreachable!()
            };
            *target_part_id = "missing-historical-endpoint".into();
            assert!(
                !before
                    .parts
                    .iter()
                    .any(|p| p.id == "missing-historical-endpoint")
            );
            assert!(
                boardstudio_core_mechanical::mechanical::halves::validate_document(before).is_ok()
            );
        }),
        ("parent identity", |_, context| {
            let child = context
                .associations
                .iter_mut()
                .find(|a| a.source_id == "matrix/source-matrix/r1c3/diode")
                .unwrap();
            child.parent_source_id = None;
            child.parent_mirror_id = None;
        }),
        ("r99c99", |before, context| {
            let mut phantom = context
                .associations
                .iter()
                .find(|a| a.source_id == "matrix/source-matrix/r0c0")
                .unwrap()
                .clone();
            phantom.source_id = "matrix/source-matrix/r99c99".into();
            phantom.mirror_id = "matrix/mirror-matrix/r99c99".into();
            assert!(
                !before
                    .parts
                    .iter()
                    .any(|p| p.id == phantom.source_id || p.id == phantom.mirror_id)
            );
            context.associations.push(phantom);
        }),
        ("exactly one", |before, _| {
            let id = "matrix/mirror-matrix/r1c1/diode";
            before
                .boards
                .by_id_mut("mirror-board")
                .part_ids
                .push(id.into());
        }),
        ("non-finite geometry", |before, _| {
            before
                .parts
                .by_id_mut("matrix/mirror-matrix/r1c1/diode")
                .pose
                .at
                .x = f64::INFINITY;
        }),
    ];
    for (expected, mutate) in cases {
        run_fresh_identity_refusal(mutate, expected);
    }
}

/// The finite check is one `Constraint::is_finite`, so every kind a transition can meet
/// must be refused by it with the message that names the constraint.
#[test]
fn consolidated_fresh_names_a_nonfinite_constraint_of_every_kind() {
    for kind in ["offset", "part-offset", "mirror"] {
        for nonfinite in [f64::INFINITY, f64::NAN] {
            let (mut before, _, context, source_map, owners, _) =
                consolidated_fresh_identity_fixture().unwrap();
            let existing = before
                .constraints
                .iter_mut()
                .find(|c| c.id() == "retained-second-key-offset")
                .unwrap();
            let (id, source_part_id, target_part_id) = (
                existing.id().to_owned(),
                existing.source().to_owned(),
                existing.target().to_owned(),
            );
            *existing = match kind {
                "offset" => Constraint::Offset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 {
                        x: nonfinite,
                        y: 1.0,
                    },
                    rotation: 0.0,
                },
                "part-offset" => Constraint::PartOffset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 { x: 1.0, y: 1.0 },
                    rotation: nonfinite,
                },
                _ => Constraint::Mirror {
                    id,
                    source_part_id,
                    target_part_id,
                    axis: MirrorAxis::Vertical,
                    coordinate: nonfinite,
                },
            };
            let mirror_map: BTreeMap<_, _> = before
                .matrices
                .by_id("mirror-matrix")
                .part_ids
                .iter()
                .map(|id| (id.clone(), Some(id.clone())))
                .collect();
            let message = prepare_membership_transition(MembershipTransitionInput {
                dispatch: crate::support::test_dispatch(),
                valid_before: &before,
                post_source: &before,
                context: &context,
                baseline_kind: BaselineKind::MaterializedCurrentSchema,
                source_identity_map: &source_map,
                mirror_identity_map: &mirror_map,
                planned_additions: &[],
                final_owners: &owners,
                explicit_removal_closure: &BTreeSet::new(),
            })
            .map(|_| ())
            .unwrap_err();
            assert!(
                message.contains("retained-second-key-offset")
                    && message.contains("non-finite geometry"),
                "{kind} {nonfinite}: {message}"
            );
        }
    }
}

#[test]
fn consolidated_fresh_refuses_local_mirror_deletion_on_identity() {
    let (before, _, context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();

    let mut mirror_map: BTreeMap<_, _> = before
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    mirror_map.insert("matrix/mirror-matrix/r1c1/diode".into(), None);

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
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "identity success cannot silently ignore a Local child's None mirror mapping"
    );
}

#[test]
fn consolidated_fresh_refuses_materialized_omitted_child_on_chained_identity() {
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
    .expect("valid preceding compaction and private context control");
    let before = output.draft;
    let mut context = output.context;
    let owners = output.final_owners;
    let source_map = before
        .matrices
        .by_id("source-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let mut mirror_map: BTreeMap<_, _> = before
        .matrices
        .by_id("mirror-matrix")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let child = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == "matrix/source-matrix/r1c2/diode")
        .unwrap();
    child.state = TransitionState::Omitted;
    child.explicit_state = true;
    let id = child.mirror_id.clone();
    assert!(before.parts.iter().any(|p| p.id == id));
    assert_eq!(
        owners.get(&id),
        Some(&FinalOwner::Board("mirror-board".into()))
    );
    mirror_map.insert(id.clone(), None);
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "chained Omitted state must bind absent geometry/owner before no-op success"
    );
}

#[test]
fn durable_local_exception_is_atomic_and_repeats_through_parent_kernel() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let mirror_id = projection
        .locally_protected_mirror_parts
        .iter()
        .next()
        .unwrap()
        .clone();
    let selected = PartProjection {
        source_id: mirror_id.replace("mirror-matrix", "source-matrix"),
        mirror_id,
    };
    let other_retained = projection
        .locally_protected_mirror_parts
        .iter()
        .find(|id| **id != selected.mirror_id)
        .unwrap()
        .clone();
    let target = document.parts.by_id_mut(&selected.mirror_id);
    target.reference = "LOCAL-OWNER".into();
    target.pose = Pose2 {
        at: Vec2 { x: -23.5, y: 41.25 },
        rotation: 117.0,
    };
    target.properties.get_or_insert_with(BTreeMap::new).insert(
        "localOwnerPayload".into(),
        serde_json::json!({"revision": 9}),
    );
    let expected_local = target.clone();

    let halves = document.first_halves_mut();
    for mirror_id in [&selected.mirror_id, &other_retained] {
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.clone(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
    }

    // This is a positive parent-kernel control: the selected Local owner is already present
    // in the validated retained-member partition. The child transition regressions separately
    // prove construction of that partition from the durable source/mirror exception.
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the durable Local exception and its matrix item are valid");
    let serialized = serde_json::to_vec(&document).unwrap();
    document = serde_json::from_slice(&serialized).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the Local association and visible geometry survive document round-trip");
    let pair_before = pair_state_bytes(&document);
    let independent_context_before = projection.applied_independent_additions.borrow().clone();
    let hosted_context_before = projection.applied_hosted_operations.borrow().clone();
    let circuit_context_before = projection.applied_hosted_circuit_additions.borrow().clone();
    let hosted_removal_before = projection.hosted_removal_baselines.borrow().clone();
    let hosted_omission_before = projection.hosted_omission_baselines.borrow().clone();
    let outline_removal_before = projection.outline_removal_baselines.borrow().clone();

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the exact durable Local owner state reconciles successfully");
    let first_output = document.clone();
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == selected.mirror_id),
        Some(&expected_local),
        "Local owns the complete target payload while the linked matrix path runs"
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the same complete owner state is repeatable");
    assert!(
        repeated.is_empty(),
        "repeat reconciliation reports no changed IDs"
    );
    assert_eq!(document, first_output);
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_independent_additions.borrow(),
        independent_context_before
    );
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        hosted_context_before
    );
    assert_eq!(
        *projection.applied_hosted_circuit_additions.borrow(),
        circuit_context_before
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        hosted_removal_before
    );
    assert_eq!(
        *projection.hosted_omission_baselines.borrow(),
        hosted_omission_before
    );
    assert_eq!(
        *projection.outline_removal_baselines.borrow(),
        outline_removal_before
    );

    let source_matrix_before = document.matrices.by_id("source-matrix").clone();
    let linked_before: BTreeMap<_, _> = document
        .parts
        .iter()
        .filter(|part| {
            part.id.starts_with("matrix/mirror-matrix/")
                && part.id != selected.mirror_id
                && part.id != other_retained
        })
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let local_matrix_cell = selected
        .source_id
        .strip_prefix("matrix/source-matrix/")
        .unwrap();
    let (local_row, local_column) = local_matrix_cell.split_once('c').unwrap();
    let local_row = local_row.strip_prefix('r').unwrap().parse::<u32>().unwrap();
    let local_column = local_column.parse::<u32>().unwrap();
    let local_slot_before = source_matrix_before
        .cells
        .iter()
        .find(|cell| cell.row == local_row && cell.column == local_column)
        .unwrap();
    assert!(local_slot_before.enabled);

    let mut edited_source_matrix = source_matrix_before;
    edited_source_matrix.origin.x += 7.0;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &edited_source_matrix)
        .expect("an ordinary Core matrix edit materializes before reconciliation");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the Local exception remains valid during a linked source matrix edit");
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == selected.mirror_id),
        Some(&expected_local),
        "the Local target keeps its visible geometry after the ordinary source edit"
    );
    assert!(
        linked_before.iter().any(|(id, pose)| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .is_some_and(|part| part.pose != *pose)
        }),
        "unselected linked matrix members still follow the ordinary source edit"
    );

    let mut invalid_projection = projection.clone();
    invalid_projection.final_owners.remove(&selected.mirror_id);
    let mut invalid_document = first_output.clone();
    let invalid_document_before = invalid_document.clone();
    let invalid_context_before = invalid_projection
        .applied_hosted_operations
        .borrow()
        .clone();
    assert!(
        reconcile(
            &mut invalid_document,
            "pair-source-mirror",
            &invalid_projection
        )
        .is_err(),
        "an incomplete owner partition is refused"
    );
    assert_eq!(invalid_document, invalid_document_before);
    assert_eq!(
        *invalid_projection.applied_hosted_operations.borrow(),
        invalid_context_before
    );
}

#[test]
fn durable_omitted_independent_part_stays_absent_through_parent_kernel() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let mirror_id = "matrix/mirror-matrix/r0c1";
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != mirror_id);

    document.parts.retain(|part| part.id != mirror_id);
    let halves = document.first_halves_mut();
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: mirror_id.into(),
        state: MirroredExceptionState::Omitted,
        removed_copper: None,
    });
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the source-bound Omitted target may be physically absent");
    let serialized = serde_json::to_vec(&document).unwrap();
    document = serde_json::from_slice(&serialized).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the source-bound Omitted absence survives document round-trip");
    let pair_before = pair_state_bytes(&document);
    let linked_before: BTreeMap<_, _> = document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/mirror-matrix/") && part.id != mirror_id)
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut source_matrix = document.matrices.by_id("source-matrix").clone();
    source_matrix.origin.x += 5.0;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)
        .expect("an ordinary Core matrix edit changes the omitted item's source");

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("a repeated reconciliation leaves the selected Omitted item absent");
    assert!(document.parts.iter().all(|part| part.id != mirror_id));
    assert!(
        document
            .boards
            .iter()
            .all(|board| !board.part_ids.iter().any(|id| id == mirror_id))
    );
    assert!(
        linked_before.iter().any(|(id, pose)| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .is_some_and(|part| part.pose != *pose)
        }),
        "linked matrix members continue to follow the ordinary source edit"
    );
    let first_output = document.clone();
    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mut invalid_projection = projection.clone();
    invalid_projection
        .final_owners
        .insert(mirror_id.into(), "source-board".into());
    let mut invalid_document = document.clone();
    assert!(
        reconcile(
            &mut invalid_document,
            "pair-source-mirror",
            &invalid_projection
        )
        .is_err(),
        "Omitted absence does not waive the exact final board owner"
    );
    assert_eq!(invalid_document, document);
}
