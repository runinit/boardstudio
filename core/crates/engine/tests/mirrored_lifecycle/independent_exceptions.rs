use super::*;
use crate::constraint_lifecycle::exception_context;
use crate::matrix_and_circuit_lifecycle::{
    add_lifecycle_association, remove_matrix_correspondence,
};
use crate::multi_matrix_and_replacement::prepare_exception_intent;

#[test]
fn independent_part_exception_is_scoped_to_its_owned_companion() {
    let mut context = exception_context();
    for (id, owner) in [
        ("source-part", "source-board"),
        ("source-part/diode", "source-board"),
        ("mirror-part", "mirror-board"),
        ("mirror-part/diode", "mirror-board"),
    ] {
        context.owners.insert(
            id.into(),
            boardstudio_core_mirrored_half::mirrored_half::reconcile::FinalOwner::Board(
                owner.into(),
            ),
        );
    }

    let localized = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        boardstudio_model::model::MirroredItemKind::Part,
        "source-part",
        "mirror-part",
        boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState::Local,
    )
    .expect("the selected independent Part and its owned companion can become Local");

    let selected = localized
        .associations
        .iter()
        .find(|association| association.source_id == "source-part")
        .unwrap();
    let companion = localized
        .associations
        .iter()
        .find(|association| association.source_id == "source-part/diode")
        .unwrap();
    let sibling_exception = localized
        .associations
        .iter()
        .find(|association| association.source_id == "source-layout")
        .unwrap();
    assert_eq!(
        selected.state,
        boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState::Local
    );
    assert!(selected.explicit_state);
    assert_eq!(
        companion.state,
        boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState::Local
    );
    assert!(!companion.explicit_state);
    assert_eq!(
        companion.effective_mode,
        boardstudio_model::model::MirroredCompanionMode::DuplicatePosition
    );
    assert_eq!(companion.effective_tweak, None);
    assert_eq!(
        sibling_exception.state,
        boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState::Omitted
    );
    assert!(sibling_exception.explicit_state);
    assert_eq!(
        localized.owners["mirror-part"],
        boardstudio_core_mirrored_half::mirrored_half::reconcile::FinalOwner::Board(
            "mirror-board".into()
        )
    );
    assert_eq!(
        localized.owners["mirror-part/diode"],
        boardstudio_core_mirrored_half::mirrored_half::reconcile::FinalOwner::Board(
            "mirror-board".into()
        )
    );
    assert_eq!(localized.pair_right, context.pair_right);
    assert_eq!(
        localized.pending_pair_transitions.get("source-part"),
        Some(&boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState::Linked)
    );
}

#[test]
fn embedded_circuit_owner_transitions_keep_explicit_child_and_unrelated_module_linked() {
    use boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionState;

    let mut context = exception_context();
    let mut circuit = context.associations[0].clone();
    circuit.kind = boardstudio_model::model::MirroredItemKind::EmbeddedCircuit;
    circuit.source_id = "source-circuit".into();
    circuit.mirror_id = "mirror-circuit".into();
    circuit.parent_source_id = None;
    circuit.parent_mirror_id = None;

    let mut member = context.associations[1].clone();
    member.kind = boardstudio_model::model::MirroredItemKind::Part;
    member.source_id = "embedded/source-circuit/U1".into();
    member.mirror_id = "embedded/mirror-circuit/U1".into();
    member.parent_source_id = Some("source-circuit".into());
    member.parent_mirror_id = Some("mirror-circuit".into());

    let mut net = context.associations[2].clone();
    net.kind = boardstudio_model::model::MirroredItemKind::Net;
    net.source_id = "embedded/source-circuit/N$1".into();
    net.mirror_id = "embedded/mirror-circuit/N$1".into();
    net.parent_source_id = Some("source-circuit".into());
    net.parent_mirror_id = Some("mirror-circuit".into());

    let mut explicit_net = context.associations[3].clone();
    explicit_net.kind = boardstudio_model::model::MirroredItemKind::Net;
    explicit_net.source_id = "embedded/source-circuit/N$LOCAL".into();
    explicit_net.mirror_id = "embedded/mirror-circuit/N$LOCAL".into();
    explicit_net.parent_source_id = Some("source-circuit".into());
    explicit_net.parent_mirror_id = Some("mirror-circuit".into());
    explicit_net.state = TransitionState::Omitted;
    explicit_net.explicit_state = true;

    let mut unrelated_module = context.associations[0].clone();
    unrelated_module.kind = boardstudio_model::model::MirroredItemKind::Module;
    unrelated_module.source_id = "source-module".into();
    unrelated_module.mirror_id = "mirror-module".into();
    unrelated_module.parent_source_id = None;
    unrelated_module.parent_mirror_id = None;

    context.associations = vec![circuit, member, net, explicit_net, unrelated_module];
    context.pair_right = serde_json::from_value(serde_json::json!({
        "sourceBoardId":"source-board", "mirrorBoardId":"mirror-board",
        "correspondences":[
            {"kind":"embedded-circuit", "sourceId":"source-circuit", "mirrorId":"mirror-circuit"},
            {"kind":"part", "sourceId":"embedded/source-circuit/U1", "mirrorId":"embedded/mirror-circuit/U1"},
            {"kind":"net", "sourceId":"embedded/source-circuit/N$1", "mirrorId":"embedded/mirror-circuit/N$1"},
            {"kind":"module", "sourceId":"source-module", "mirrorId":"mirror-module"},
            {"kind":"net", "sourceId":"embedded/source-circuit/N$LOCAL", "mirrorId":"embedded/mirror-circuit/N$LOCAL"}
        ],
        "exceptions":[
            {"kind":"net", "mirrorId":"embedded/mirror-circuit/N$LOCAL", "state":"omitted"}
        ]
    }))
    .unwrap();

    let localized = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        boardstudio_model::model::MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Local,
    )
    .expect("the exact circuit owner and inherited children can become Local");
    assert_eq!(localized.associations[1].state, TransitionState::Local);
    assert_eq!(localized.associations[2].state, TransitionState::Local);
    assert_eq!(localized.associations[3].state, TransitionState::Omitted);
    assert!(localized.associations[3].explicit_state);
    assert_eq!(localized.associations[4], context.associations[4]);

    let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &localized,
        boardstudio_model::model::MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Linked,
    )
    .expect("the circuit can rejoin without changing an explicit child exception");
    assert_eq!(rejoined.associations[0].state, TransitionState::Linked);
    assert_eq!(rejoined.associations[1].state, TransitionState::Linked);
    assert_eq!(rejoined.associations[2].state, TransitionState::Linked);
    assert_eq!(rejoined.associations[3].state, TransitionState::Omitted);
    assert!(rejoined.associations[3].explicit_state);
    assert_eq!(rejoined.associations[4], context.associations[4]);
    assert_eq!(rejoined.pair_right, context.pair_right);
    assert!(rejoined.pending_pair_baseline.is_none());
}

pub(crate) fn local_transition_fixture() -> Result<(ProjectDoc, MembershipContext), String> {
    let mut document = fixture_document()?;
    let mut diode = document.definitions[0].clone();
    diode.id = "diode".into();
    diode.name = "Diode".into();
    document.definitions.push(diode);

    let matrix_cells = || {
        (0..2)
            .flat_map(|row| {
                (0..4).map(move |column| MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
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
                })
            })
            .collect::<Vec<_>>()
    };
    let mut source_matrix = document.matrices.by_id("source-matrix").clone();
    source_matrix.rows = 2;
    source_matrix.columns = 4;
    source_matrix.pitch = Vec2 { x: 10.0, y: 20.0 };
    source_matrix.cells = matrix_cells();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)?;

    let mut mirror_matrix = document.matrices.by_id("mirror-matrix").clone();
    mirror_matrix.rows = 2;
    mirror_matrix.columns = 4;
    mirror_matrix.pitch = source_matrix.pitch;
    mirror_matrix.cells = matrix_cells();
    let matrix_correspondence = document
        .first_halves()
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix)
        .unwrap()
        .clone();
    mirror_matrix = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        &source_matrix,
        &mirror_matrix,
        &matrix_correspondence,
        document.hardware.as_ref().unwrap().pairs[0].mapping,
    )?;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &mirror_matrix)?;

    let halves = document.first_halves_mut();
    halves.correspondences.clear();
    halves.exceptions.clear();
    halves.correspondences.push(MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
        companion_mode: None,
        companion_tweak: None,
    });
    for row in 0..2 {
        for column in 0..4 {
            let source = format!("matrix/source-matrix/r{row}c{column}");
            let mirror = format!("matrix/mirror-matrix/r{row}c{column}");
            halves.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Part,
                source_id: source.clone(),
                mirror_id: mirror.clone(),
                companion_mode: None,
                companion_tweak: None,
            });
            halves.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Part,
                source_id: format!("{source}/diode"),
                mirror_id: format!("{mirror}/diode"),
                companion_mode: None,
                companion_tweak: None,
            });
        }
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)?;

    let pair = document.hardware.as_ref().unwrap().pairs[0].clone();
    let halves = pair.halves.as_ref().unwrap();
    let associations = halves
        .correspondences
        .iter()
        .map(|record| {
            let child = record.source_id.ends_with("/diode");
            TransitionAssociation {
                kind: record.kind,
                source_id: record.source_id.clone(),
                mirror_id: record.mirror_id.clone(),
                parent_source_id: child
                    .then(|| record.source_id.trim_end_matches("/diode").to_owned()),
                parent_mirror_id: child
                    .then(|| record.mirror_id.trim_end_matches("/diode").to_owned()),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: record.companion_mode,
                raw_tweak: record.companion_tweak,
                effective_mode: record.companion_mode.unwrap_or(halves.companion_mode),
                effective_tweak: record.companion_tweak,
            }
        })
        .collect();
    let owners = document
        .parts
        .iter()
        .map(|part| {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            (part.id.clone(), FinalOwner::Board(owner.id.clone()))
        })
        .collect();
    Ok((
        document,
        MembershipContext {
            pair_id: pair.id.clone(),
            source_board_id: halves.source_board_id.clone(),
            mirror_board_id: halves.mirror_board_id.clone(),
            matrix_pair: Some(
                boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                    source_id: "source-matrix".into(),
                    mirror_id: "mirror-matrix".into(),
                },
            ),
            mapping: pair.mapping,
            pair_right: halves.clone(),
            associations,
            owners,
            pending_rejoins: BTreeMap::new(),
            pending_pair_transitions: BTreeMap::new(),
            pending_pair_baseline: None,
        },
    ))
}

#[test]
fn parent_kernel_commits_local_source_edit_and_rejoin_through_real_lifecycle_context() {
    let (linked_before, linked_context) =
        local_transition_fixture().expect("the linked matrix fixture is valid");
    let source_id = "matrix/source-matrix/r1c1";
    let mirror_id = "matrix/mirror-matrix/r1c1";
    let local_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &linked_context,
        MirroredItemKind::Part,
        source_id,
        mirror_id,
        TransitionState::Local,
    )
    .expect("the linked key and inherited child become one exact Local owner");
    let local_output = commit_matrix_step(&linked_before, &linked_before, &local_context)
        .expect("the real kernel transaction publishes the Local exception");
    let before = local_output.draft;
    let context = local_output.context;
    let persisted_halves = before.first_halves();
    assert_eq!(
        persisted_halves
            .correspondences
            .iter()
            .filter(|record| {
                record.kind == MirroredItemKind::Part
                    && record.source_id == source_id
                    && record.mirror_id == mirror_id
            })
            .count(),
        1,
        "the correspondence is kept in every state"
    );
    assert_eq!(
        persisted_halves
            .exceptions
            .iter()
            .filter(|exception| {
                exception.kind == MirroredItemKind::Part
                    && persisted_halves.overlay_source(exception) == Some(source_id)
                    && exception.mirror_id == mirror_id
                    && exception.state == MirroredExceptionState::Local
            })
            .count(),
        1,
        "the lifecycle transaction publishes this exact Local overlay on its correspondence"
    );
    assert_eq!(persisted_halves, &context.pair_right);

    let local_pose = Pose2 {
        at: Vec2 { x: -23.0, y: 41.0 },
        rotation: 29.0,
    };
    let mut post_source = before.clone();
    let local_key = post_source.parts.by_id_mut(mirror_id);
    local_key.pose = local_pose;
    local_key
        .properties
        .get_or_insert_with(BTreeMap::new)
        .insert("layoutOverride".into(), serde_json::json!(1));
    let local_key_snapshot = local_key.clone();
    assert_eq!(
        local_key_snapshot.properties.as_ref().unwrap()["layoutOverride"],
        serde_json::json!(1)
    );
    let mut expanded = before.matrices.by_id("source-matrix").clone();
    expanded.columns = 5;
    for row in 0..2 {
        expanded.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 4,
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
    expanded
        .cells
        .iter_mut()
        .find(|cell| cell.row == 1 && cell.column == 1)
        .unwrap()
        .offset = Some(Vec2 { x: 13.0, y: -6.0 });
    boardstudio_core_kernel::matrix::set_matrix(&mut post_source, &expanded)
        .expect("the planned source additions materialize");

    let output = commit_matrix_step(&before, &post_source, &context)
        .expect("a complete planned expansion is projected atomically");
    assert_eq!(
        output.draft.parts.by_id(mirror_id),
        &local_key_snapshot,
        "an ordinary source expansion preserves the explicitly Local target payload"
    );
    let mirror = output.draft.matrices.by_id("mirror-matrix");
    assert_eq!((mirror.rows, mirror.columns), (2, 5));
    assert_eq!(mirror.part_ids.len(), 20);
    assert_eq!(output.context_delta.added.len(), 4);
    assert_eq!(output.context.associations.len(), 21);
    for added in &output.context_delta.added {
        assert!(
            output
                .context
                .pair_right
                .correspondences
                .iter()
                .any(|record| {
                    record.kind == added.kind
                        && record.source_id == added.source_id
                        && record.mirror_id == added.mirror_id
                        && record.companion_mode == added.raw_mode_override
                        && record.companion_tweak == added.raw_tweak
                })
        );
    }
    assert_eq!(output.draft.first_halves(), &output.context.pair_right);

    let rejoined_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &output.context,
        MirroredItemKind::Part,
        source_id,
        mirror_id,
        TransitionState::Linked,
    )
    .expect("the exact Local exception can be Rejoined");
    let rejoin_before = output.draft.clone();
    let rejoined = commit_matrix_step(&rejoin_before, &rejoin_before, &rejoined_context)
        .expect("Rejoin projects the current source into the former Local target");
    let current_source = rejoin_before.parts.by_id(source_id);
    let expected_pose = Pose2 {
        at: boardstudio_core_mechanical::mechanical::pair::forward(
            rejoined_context.mapping,
            current_source.pose.at,
        ),
        rotation: boardstudio_core_mechanical::mechanical::pair::forward_angle(
            rejoined_context.mapping,
            current_source.pose.rotation,
        ),
    };
    let rejoined_part = rejoined.draft.parts.by_id(mirror_id);
    assert!(
        (rejoined_part.pose.at.x - expected_pose.at.x).abs() < 1e-9
            && (rejoined_part.pose.at.y - expected_pose.at.y).abs() < 1e-9
            && (rejoined_part.pose.rotation - expected_pose.rotation).abs() < 1e-9,
        "Rejoin derives the matrix key pose from the current source: actual={:?}, expected={expected_pose:?}",
        rejoined_part.pose
    );
    assert_eq!(rejoined_part.reference, local_key_snapshot.reference);
    assert_eq!(rejoined_part.properties, current_source.properties);
    assert!(
        rejoined_part
            .properties
            .as_ref()
            .is_none_or(|properties| !properties.contains_key("layoutOverride"))
    );
    let repeated = commit_matrix_step(&rejoined.draft, &rejoined.draft, &rejoined.context)
        .expect("the full Rejoined document and context converge on repeat");
    assert_eq!(repeated.draft, rejoined.draft);
    assert_eq!(repeated.context, rejoined.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());
}

pub(crate) fn commit_matrix_step(
    before: &ProjectDoc,
    post_source: &ProjectDoc,
    context: &MembershipContext,
) -> Result<
    boardstudio_core_mirrored_half::mirrored_half::reconcile::MembershipTransitionOutput,
    String,
> {
    let source = before
        .matrices
        .iter()
        .find(|m| m.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        .unwrap();
    let mirror = before
        .matrices
        .iter()
        .find(|m| m.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        .unwrap();
    let post_members: BTreeSet<_> = post_source
        .matrices
        .iter()
        .filter(|m| m.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        .flat_map(|m| m.part_ids.iter().cloned())
        .collect();
    let source_map: BTreeMap<_, _> = source
        .part_ids
        .iter()
        .map(|id| (id.clone(), post_members.contains(id).then(|| id.clone())))
        .collect();
    let mirror_map: BTreeMap<_, _> = mirror
        .part_ids
        .iter()
        .map(|id| {
            let association = context
                .associations
                .iter()
                .find(|a| a.kind == MirroredItemKind::Part && a.mirror_id == *id)
                .unwrap();
            let keep = association.state != TransitionState::Omitted
                && (post_members.contains(&association.source_id)
                    || association.state == TransitionState::Local);
            (id.clone(), keep.then(|| id.clone()))
        })
        .collect();
    let mut owners = context.owners.clone();
    for part in &post_source.parts {
        let owner = post_source
            .boards
            .iter()
            .find(|b| b.part_ids.contains(&part.id))
            .unwrap();
        owners.insert(part.id.clone(), FinalOwner::Board(owner.id.clone()));
    }
    for association in context
        .associations
        .iter()
        .filter(|a| a.kind == MirroredItemKind::Part)
    {
        owners.insert(
            association.mirror_id.clone(),
            if association.state == TransitionState::Omitted {
                FinalOwner::Absent
            } else {
                FinalOwner::Board(context.mirror_board_id.clone())
            },
        );
    }
    let mut additions = Vec::new();
    for id in post_members
        .iter()
        .filter(|id| !source.part_ids.contains(id))
    {
        let target_id = id.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
        let source_part = post_source.parts.by_id(id);
        let child = id.ends_with("/diode");
        additions.push(PlannedAddition {
            source_id: id.clone(),
            mirror_id: target_id.clone(),
            source_reference: source_part.reference.clone(),
            mirror_reference: format!("ADDED_{}", id.replace('/', "_")),
            parent_source_id: child.then(|| id.trim_end_matches("/diode").to_owned()),
            parent_mirror_id: child.then(|| target_id.trim_end_matches("/diode").to_owned()),
        });
        owners.insert(
            target_id,
            FinalOwner::Board(context.mirror_board_id.clone()),
        );
    }
    let closure: BTreeSet<_> = source_map
        .iter()
        .chain(&mirror_map)
        .filter(|(_, mapped)| mapped.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    for (id, mapped) in &source_map {
        if mapped.is_none() {
            owners.insert(id.clone(), FinalOwner::Absent);
        }
    }
    prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: before,
        post_source,
        context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &additions,
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
}

pub(crate) fn assert_committed_repeat(
    output: &boardstudio_core_mirrored_half::mirrored_half::reconcile::MembershipTransitionOutput,
) {
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
    assert_eq!(decoded, output.draft);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&decoded).unwrap();
    assert_eq!(
        decoded.hardware.as_ref().unwrap().pairs[0].halves.as_ref(),
        Some(&output.context.pair_right)
    );
    let repeated = commit_matrix_step(&decoded, &decoded, &output.context)
        .expect("the committed transaction repeats");
    assert_eq!(repeated.draft, decoded);
    assert_eq!(repeated.context, output.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());
}

#[test]
fn matrix_key_omit_source_expansion_and_rejoin_commit_complete_documents() {
    let (before, context) = local_transition_fixture().unwrap();
    let source_id = "matrix/source-matrix/r1c1";
    let mirror_id = "matrix/mirror-matrix/r1c1";
    let child_id = format!("{mirror_id}/diode");
    let omitted_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        source_id,
        mirror_id,
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted = commit_matrix_step(&before, &before, &omitted_context)
        .expect("Omit commits the selected owner and inherited companion absence");
    assert_committed_repeat(&omitted);
    for id in [mirror_id, child_id.as_str()] {
        assert!(omitted.draft.parts.iter().all(|p| p.id != id));
        assert!(
            omitted
                .draft
                .boards
                .iter()
                .all(|b| b.part_ids.iter().all(|p| p != id))
        );
    }
    assert_eq!(omitted.context.pair_right.exceptions.len(), 1);
    assert_eq!(
        omitted
            .context
            .pair_right
            .overlay_source(&omitted.context.pair_right.exceptions[0]),
        Some(source_id)
    );
    let mut post = omitted.draft.clone();
    let mut source = post.matrices.by_id("source-matrix").clone();
    source.columns = 5;
    for row in 0..2 {
        let mut cell = source
            .cells
            .iter()
            .find(|c| c.row == row && c.column == 0)
            .unwrap()
            .clone();
        cell.column = 4;
        source.cells.push(cell);
    }
    source
        .cells
        .iter_mut()
        .find(|c| c.row == 1 && c.column == 1)
        .unwrap()
        .offset = Some(Vec2 { x: 13.0, y: -6.0 });
    boardstudio_core_kernel::matrix::set_matrix(&mut post, &source).unwrap();
    post.parts.by_id_mut(source_id).properties = Some(BTreeMap::from([(
        "current-source".into(),
        json!({"revision": 2}),
    )]));
    let expanded = commit_matrix_step(&omitted.draft, &post, &omitted.context)
        .expect("ordinary source expansion keeps the omitted closure absent");
    assert_committed_repeat(&expanded);
    assert!(
        expanded
            .draft
            .parts
            .iter()
            .all(|p| p.id != mirror_id && p.id != child_id)
    );
    let rejoin_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &expanded.context,
        MirroredItemKind::Part,
        source_id,
        mirror_id,
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined = commit_matrix_step(&expanded.draft, &expanded.draft, &rejoin_context)
        .expect("Rejoin restores the current source and inherited companion");
    assert_committed_repeat(&rejoined);
    let source = expanded.draft.parts.by_id(source_id);
    let target = rejoined.draft.parts.by_id(mirror_id);
    let expected_at =
        boardstudio_core_mechanical::mechanical::pair::forward(context.mapping, source.pose.at);
    assert!((target.pose.at.x - expected_at.x).abs() < 1e-10);
    assert!((target.pose.at.y - expected_at.y).abs() < 1e-10);
    assert!(
        (target.pose.rotation
            - boardstudio_core_mechanical::mechanical::pair::forward_angle(
                context.mapping,
                source.pose.rotation
            ))
        .abs()
            < 1e-10
    );
    assert_eq!(target.properties, source.properties);
    assert_eq!(target.generator_parameters, source.generator_parameters);
    assert_eq!(target.side, source.side);
    assert!(rejoined.draft.parts.iter().any(|p| p.id == child_id));
    assert!(rejoined.context.pair_right.exceptions.is_empty());
}

pub(crate) fn independent_lifecycle_fixture() -> (ProjectDoc, MembershipContext) {
    let (mut document, mut context) = local_transition_fixture().unwrap();
    for name in ["screen", "encoder"] {
        for (half, board_id, x) in [
            ("source", "source-board", 8.0),
            ("mirror", "mirror-board", 90.0),
        ] {
            let id = format!("{half}-{name}");
            let mut part = document.parts[0].clone();
            part.id = id.clone();
            part.reference = format!("{half}_{name}");
            part.pose.at = Vec2 { x, y: 42.0 };
            part.properties = Some(BTreeMap::from([("revision".into(), json!(1))]));
            document.parts.push(part);
            document
                .boards
                .by_id_mut(board_id)
                .part_ids
                .push(id.clone());
            context
                .owners
                .insert(id, FinalOwner::Board(board_id.into()));
        }
        let association =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::TransitionAssociation {
                kind: MirroredItemKind::Part,
                source_id: format!("source-{name}"),
                mirror_id: format!("mirror-{name}"),
                parent_source_id: None,
                parent_mirror_id: None,
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: None,
                raw_tweak: None,
                effective_mode: context.pair_right.companion_mode,
                effective_tweak: None,
            };
        context
            .pair_right
            .correspondences
            .push(MirroredCorrespondence {
                kind: association.kind,
                source_id: association.source_id.clone(),
                mirror_id: association.mirror_id.clone(),
                companion_mode: None,
                companion_tweak: None,
            });
        context.associations.push(association);
    }
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    (document, context)
}

#[test]
fn independent_screen_local_omit_and_rejoin_commit_current_source() {
    let (mut before, mut context) = independent_lifecycle_fixture();
    for half in ["source", "mirror"] {
        before.layouts.push(Layout {
            id: format!("{half}-layout"),
            name: "Controls".into(),
            board_id: format!("{half}-board"),
            matrix_ids: vec![format!("{half}-matrix")],
            part_ids: vec![format!("{half}-screen"), format!("{half}-encoder")],
            mirror_link: None,
        });
    }
    add_lifecycle_association(
        &mut before,
        &mut context,
        MirroredItemKind::Layout,
        "source-layout",
        "mirror-layout",
        None,
    );
    let local_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &local_context,
        crate::support::test_dispatch(),
    )
    .expect("an independent screen can become Local without leaving the pair");
    let local_part = local.draft.parts.by_id("mirror-screen").clone();
    let mut post = local.draft.clone();
    for source_id in ["source-screen", "source-encoder"] {
        let source = post.parts.by_id_mut(source_id);
        source.pose.at.x += 17.0;
        source.properties = Some(BTreeMap::from([("revision".into(), json!(2))]));
    }
    let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &post,
        &local.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == "mirror-screen"),
        Some(&local_part)
    );
    assert_eq!(
        edited.draft.parts.by_id("mirror-encoder").properties,
        post.parts.by_id("source-encoder").properties
    );
    let omitted_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &edited.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &edited.draft,
        &edited.draft,
        &omitted_context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert!(omitted.draft.parts.iter().all(|p| p.id != "mirror-screen"));
    assert!(
        omitted
            .draft
            .boards
            .iter()
            .all(|b| !b.part_ids.contains(&"mirror-screen".into()))
    );
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted.draft).unwrap()).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&decoded).unwrap();
    let repeated = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &decoded,
        &decoded,
        &omitted.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeated.draft, decoded);
    assert_eq!(repeated.context, omitted.context);
    assert!(repeated.changed_document_ids.is_empty());
    let rejoin_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &repeated.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &repeated.draft,
        &repeated.draft,
        &rejoin_context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let source = rejoined.draft.parts.by_id("source-screen");
    let target = rejoined.draft.parts.by_id("mirror-screen");
    assert_eq!(target.properties, source.properties);
    let expected = boardstudio_core_mirrored_half::mirrored_half::derive::part(
        source,
        &target.id,
        &target.definition_id,
        &target.reference,
        context.mapping,
        boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
            &rejoined.draft.definitions,
            &source.definition_id,
        ),
    );
    assert_eq!(target, &expected);
    assert!(rejoined.context.pair_right.exceptions.is_empty());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();

    // The same matrixless part lifecycle is reachable straight through the private intent route.
    let (mut matrixless, _) = independent_lifecycle_fixture();
    remove_matrix_correspondence(&mut matrixless);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&matrixless).unwrap();
    let pair_id = matrixless.hardware.as_ref().unwrap().pairs[0].id.clone();
    let mut prepared = prepare_exception_intent(
        &matrixless,
        &matrixless,
        &pair_id,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    assert_eq!(prepared.len(), 1, "the Local intent must prepare one draft");
    let prepared = prepared.pop().unwrap();
    assert_eq!(
        prepared
            .draft
            .parts
            .iter()
            .find(|p| p.id == "mirror-screen"),
        matrixless.parts.iter().find(|p| p.id == "mirror-screen"),
        "matrixless Local keeps the selected geometry byte-for-byte"
    );
}

#[test]
fn independent_source_deletion_retires_only_exact_records_and_keeps_local_geometry() {
    for state in [
        TransitionState::Linked,
        TransitionState::Local,
        TransitionState::Omitted,
    ] {
        let (before, context) = independent_lifecycle_fixture();
        let prepared = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            "source-screen",
            "mirror-screen",
            state,
        )
        .unwrap();
        let committed = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &before,
            &before,
            &prepared,
            crate::support::test_dispatch(),
        )
        .unwrap();
        let local_part = committed
            .draft
            .parts
            .iter()
            .find(|p| p.id == "mirror-screen")
            .cloned();
        let unrelated: Vec<_> = committed
            .context
            .pair_right
            .correspondences
            .iter()
            .filter(|r| r.source_id != "source-screen")
            .cloned()
            .collect();
        let mut post = committed.draft.clone();
        boardstudio_core_kernel::matrix::deletion::remove_parts_by_id(
            &mut post,
            &BTreeSet::from(["source-screen".into()]),
        );
        let removed = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &committed.draft,
            &post,
            &committed.context,
            crate::support::test_dispatch(),
        )
        .expect("source deletion retires the exact association in every exception state");
        assert_eq!(removed.context.pair_right.correspondences, unrelated);
        assert!(removed.context.pair_right.exceptions.is_empty());
        assert!(
            removed
                .context
                .associations
                .iter()
                .all(|a| a.source_id != "source-screen")
        );
        assert_eq!(
            removed
                .draft
                .parts
                .iter()
                .find(|p| p.id == "mirror-screen")
                .cloned(),
            if state == TransitionState::Local {
                local_part
            } else {
                None
            }
        );
        boardstudio_core_mechanical::mechanical::halves::validate_document(&removed.draft).unwrap();
        let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &removed.draft,
            &removed.draft,
            &removed.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(repeat.draft, removed.draft);
        assert_eq!(repeat.context, removed.context);
        assert!(repeat.changed_document_ids.is_empty());
    }
}

#[test]
fn lifecycle_refuses_forged_independent_ownership_without_mutating_inputs() {
    let (before, context) = independent_lifecycle_fixture();
    let mut pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    let encoder = pending
        .associations
        .iter_mut()
        .find(|a| a.source_id == "source-encoder")
        .unwrap();
    encoder.parent_source_id = Some("source-screen".into());
    encoder.parent_mirror_id = Some("mirror-screen".into());
    encoder.state = TransitionState::Local;
    let frozen = (before.clone(), pending.clone());
    let error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect_err("an independent encoder is not owned by a screen");
    assert!(error.contains("ownership"), "{error}");
    assert_eq!((before, pending), frozen);
}
