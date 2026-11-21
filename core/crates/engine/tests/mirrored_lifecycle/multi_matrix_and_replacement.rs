use super::*;
use crate::constraint_lifecycle::{part, valid_omitted_document};
use crate::independent_exceptions::independent_lifecycle_fixture;
use crate::matrix_and_circuit_lifecycle::circuit_lifecycle_fixture;
use crate::owner_rejoin_electrical::{electrical_copper_fixture, omit_screen};

#[test]
fn saved_copper_that_collides_with_live_copper_refuses_the_rejoin_atomically() {
    let (before, context) = electrical_copper_fixture();
    let omitted = omit_screen(&before, &context);
    let mut occupied = omitted.draft.clone();
    occupied
        .boards
        .iter_mut()
        .find(|b| b.id == "mirror-board")
        .unwrap()
        .traces
        .push(CopperTrace {
            id: "t-sda".into(),
            start: Vec2 { x: 0.0, y: 0.0 },
            end: Vec2 { x: 1.0, y: 1.0 },
            width: 0.25,
            layer: Side::Front,
            net_id: None,
        });
    let frozen = (occupied.clone(), omitted.context.clone());
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &occupied,
            &occupied,
            &pending,
            crate::support::test_dispatch()
        )
        .is_err()
    );
    assert_eq!(frozen, (occupied, omitted.context));
}

pub(crate) fn add_second_matrix_pair(
    document: &mut ProjectDoc,
    context: &mut MembershipContext,
) -> Result<(), String> {
    let mut source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .ok_or("fixture source Matrix missing")?
        .clone();
    source.id = "source-matrix-2".into();
    source.name = Some("second source Matrix".into());
    source.part_ids.clear();
    boardstudio_core_kernel::matrix::set_matrix(document, &source)?;

    let mut mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .ok_or("fixture mirror Matrix missing")?
        .clone();
    mirror.id = "mirror-matrix-2".into();
    mirror.name = Some("second mirror Matrix".into());
    mirror.part_ids.clear();
    let correspondence = MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
        companion_mode: None,
        companion_tweak: None,
    };
    mirror = boardstudio_core_mirrored_half::mirrored_half::reconcile::project_matrix(
        &source,
        &mirror,
        &correspondence,
        context.mapping,
    )?;
    boardstudio_core_kernel::matrix::set_matrix(document, &mirror)?;

    context
        .pair_right
        .correspondences
        .push(correspondence.clone());
    context.associations.push(TransitionAssociation {
        kind: MirroredItemKind::Matrix,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
        parent_source_id: None,
        parent_mirror_id: None,
        state: TransitionState::Linked,
        explicit_state: false,
        raw_mode_override: None,
        raw_tweak: None,
        effective_mode: context.pair_right.companion_mode,
        effective_tweak: None,
    });
    let original_children: Vec<_> = context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| {
            record.kind == MirroredItemKind::Part
                && record.source_id.starts_with("matrix/source-matrix/")
        })
        .cloned()
        .collect();
    for original in original_children {
        let source_id =
            original
                .source_id
                .replacen("matrix/source-matrix/", "matrix/source-matrix-2/", 1);
        let mirror_id =
            original
                .mirror_id
                .replacen("matrix/mirror-matrix/", "matrix/mirror-matrix-2/", 1);
        let record = MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id: source_id.clone(),
            mirror_id: mirror_id.clone(),
            companion_mode: original.companion_mode,
            companion_tweak: original.companion_tweak,
        };
        context.pair_right.correspondences.push(record);
        let parent_source_id = source_id
            .contains("/diode")
            .then(|| source_id.trim_end_matches("/diode").to_owned());
        let parent_mirror_id = mirror_id
            .contains("/diode")
            .then(|| mirror_id.trim_end_matches("/diode").to_owned());
        context.associations.push(TransitionAssociation {
            kind: MirroredItemKind::Part,
            source_id,
            mirror_id,
            parent_source_id,
            parent_mirror_id,
            state: TransitionState::Linked,
            explicit_state: false,
            raw_mode_override: None,
            raw_tweak: None,
            effective_mode: context.pair_right.companion_mode,
            effective_tweak: None,
        });
    }
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    boardstudio_core_mechanical::mechanical::halves::validate_document(document)?;
    Ok(())
}

#[test]
fn independent_part_exception_intents_keep_exact_matrix_owners_and_children() {
    for matrix_count in [1, 2] {
        let (mut before, mut fixture_context) = independent_lifecycle_fixture();
        if matrix_count == 2 {
            add_second_matrix_pair(&mut before, &mut fixture_context).unwrap();
        }
        let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
        let matrices_before = before.matrices.clone();
        let children_before: Vec<_> = before
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/"))
            .cloned()
            .collect();

        let local = prepare_exception_intent(
            &before,
            &before,
            &pair_id,
            MirroredItemKind::Part,
            "source-screen",
            "mirror-screen",
            TransitionState::Local,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"))
        .pop()
        .unwrap();
        assert_eq!(local.draft.matrices, matrices_before);
        for child in &children_before {
            assert_eq!(
                local.draft.parts.iter().find(|part| part.id == child.id),
                Some(child)
            );
        }
        assert!(local.context.pair_right.exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part && exception.mirror_id == "mirror-screen"
        }));

        let mut edited_source = local.draft.clone();
        let source = edited_source
            .parts
            .iter_mut()
            .find(|part| part.id == "source-encoder")
            .unwrap();
        source.pose.at.x += 23.0;
        source.properties = Some(BTreeMap::from([("revision".into(), json!(2))]));
        let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &local.draft,
            &edited_source,
            &local.context,
            crate::support::test_dispatch(),
        )
        .expect("unrelated linked Part follows its current source");
        let source = edited_source
            .parts
            .iter()
            .find(|part| part.id == "source-encoder")
            .unwrap();
        let mirror = edited
            .draft
            .parts
            .iter()
            .find(|part| part.id == "mirror-encoder")
            .unwrap();
        assert_eq!(
            mirror,
            &boardstudio_core_mirrored_half::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                fixture_context.mapping,
                boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
                    &edited_source.definitions,
                    &source.definition_id,
                ),
            )
        );
        assert_eq!(edited.draft.matrices, matrices_before);

        let omitted = prepare_exception_intent(
            &edited.draft,
            &edited.draft,
            &pair_id,
            MirroredItemKind::Part,
            "source-screen",
            "mirror-screen",
            TransitionState::Omitted,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"))
        .pop()
        .unwrap();
        assert!(
            omitted
                .draft
                .parts
                .iter()
                .all(|part| part.id != "mirror-screen")
        );
        assert_eq!(omitted.draft.matrices, matrices_before);
        let rejoined = prepare_exception_intent(
            &omitted.draft,
            &omitted.draft,
            &pair_id,
            MirroredItemKind::Part,
            "source-screen",
            "mirror-screen",
            TransitionState::Linked,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"))
        .pop()
        .unwrap();
        let source = rejoined
            .draft
            .parts
            .iter()
            .find(|part| part.id == "source-screen")
            .unwrap();
        let mirror = rejoined
            .draft
            .parts
            .iter()
            .find(|part| part.id == "mirror-screen")
            .unwrap();
        assert_eq!(
            mirror,
            &boardstudio_core_mirrored_half::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                fixture_context.mapping,
                boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
                    &rejoined.draft.definitions,
                    &source.definition_id,
                ),
            )
        );
        assert_eq!(rejoined.draft.matrices, matrices_before);
        assert!(rejoined.context.pair_right.exceptions.is_empty());
    }
}

#[test]
fn independent_circuit_exception_intents_work_beside_multiple_matrix_owners() {
    let (mut before, mut fixture_context) = circuit_lifecycle_fixture();
    add_second_matrix_pair(&mut before, &mut fixture_context).unwrap();
    let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
    let matrices_before = before.matrices.clone();
    let explicit_children: Vec<_> = before
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/"))
        .cloned()
        .collect();

    let local = prepare_exception_intent(
        &before,
        &before,
        &pair_id,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Local,
    )
    .unwrap_or_else(|error| panic!("circuit Local: {error}"))
    .pop()
    .unwrap();
    let omitted = prepare_exception_intent(
        &local.draft,
        &local.draft,
        &pair_id,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Omitted,
    )
    .unwrap_or_else(|error| panic!("circuit Omit: {error}"))
    .pop()
    .unwrap();
    assert!(
        omitted
            .draft
            .embedded_circuits
            .iter()
            .all(|circuit| circuit.id != "mirror-circuit")
    );
    let rejoined = prepare_exception_intent(
        &omitted.draft,
        &omitted.draft,
        &pair_id,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Linked,
    )
    .unwrap_or_else(|error| panic!("circuit Rejoin: {error}"))
    .pop()
    .unwrap();
    assert!(
        rejoined
            .draft
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit")
    );
    assert_eq!(rejoined.draft.matrices, matrices_before);
    for child in &explicit_children {
        assert_eq!(
            rejoined.draft.parts.iter().find(|part| part.id == child.id),
            Some(child)
        );
    }
    assert!(
        rejoined
            .context
            .pair_right
            .exceptions
            .iter()
            .all(|exception| {
                exception.kind != MirroredItemKind::EmbeddedCircuit
                    || exception.mirror_id != "mirror-circuit"
            })
    );
}

#[test]
fn independent_none_context_validates_all_matrix_owners_without_claiming_one() {
    let (mut before, mut context) = independent_lifecycle_fixture();
    add_second_matrix_pair(&mut before, &mut context).unwrap();
    let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
    let matrices_before = before.matrices.clone();
    let independent =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &before, &pair_id, None,
        )
        .unwrap();
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &independent,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();

    let output = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect("independent selection validates each Matrix child against its own owner");
    assert_eq!(output.draft.matrices, matrices_before);
    assert!(
        output
            .context
            .pair_right
            .exceptions
            .iter()
            .any(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == "mirror-screen"
            })
    );
}

#[test]
fn exact_second_matrix_child_transitions_preserve_first_matrix_local_child() {
    let (mut before, mut context) = independent_lifecycle_fixture();
    add_second_matrix_pair(&mut before, &mut context).unwrap();
    let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
    let first_source = "matrix/source-matrix/r0c1";
    let first_mirror = "matrix/mirror-matrix/r0c1";
    let first_context =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &before,
            &pair_id,
            Some(
                boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                    source_id: "source-matrix".into(),
                    mirror_id: "mirror-matrix".into(),
                },
            ),
        )
        .unwrap();
    let first_pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &first_context,
        MirroredItemKind::Part,
        first_source,
        first_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let first_local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &first_pending,
        crate::support::test_dispatch(),
    )
    .expect("first Matrix child can be independently made Local");
    let retained_first_child = first_local
        .draft
        .parts
        .iter()
        .find(|part| part.id == first_mirror)
        .unwrap()
        .clone();
    let retained_first_matrix = first_local
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    let second_source = "matrix/source-matrix-2/r0c0";
    let second_mirror = "matrix/mirror-matrix-2/r0c0";
    let second_context =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &first_local.draft,
            &pair_id,
            Some(
                boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                    source_id: "source-matrix-2".into(),
                    mirror_id: "mirror-matrix-2".into(),
                },
            ),
        )
        .unwrap();
    let local_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &second_context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &first_local.draft,
        &first_local.draft,
        &local_context,
        crate::support::test_dispatch(),
    )
    .expect("the selected second-Matrix child uses its exact owner");
    assert_eq!(
        local
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix"),
        Some(&retained_first_matrix)
    );
    assert_eq!(
        local
            .draft
            .parts
            .iter()
            .find(|part| part.id == first_mirror),
        Some(&retained_first_child)
    );

    let omit_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &local.context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &local.draft,
        &omit_context,
        crate::support::test_dispatch(),
    )
    .expect("the exact second-Matrix child can be omitted");
    assert!(
        omitted
            .draft
            .parts
            .iter()
            .all(|part| part.id != second_mirror)
    );
    assert_eq!(
        omitted
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix"),
        Some(&retained_first_matrix)
    );
    assert_eq!(
        omitted
            .draft
            .parts
            .iter()
            .find(|part| part.id == first_mirror),
        Some(&retained_first_child)
    );

    let rejoin_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &omitted.draft,
        &omitted.draft,
        &rejoin_context,
        crate::support::test_dispatch(),
    )
    .expect("the exact second-Matrix child rejoins independently");
    assert_eq!(
        rejoined
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix"),
        Some(&retained_first_matrix)
    );
    assert_eq!(
        rejoined
            .draft
            .parts
            .iter()
            .find(|part| part.id == first_mirror),
        Some(&retained_first_child)
    );
    assert!(
        rejoined
            .draft
            .parts
            .iter()
            .any(|part| part.id == second_mirror)
    );
    assert!(
        rejoined
            .context
            .pair_right
            .exceptions
            .iter()
            .any(|exception| {
                exception.kind == MirroredItemKind::Part && exception.mirror_id == first_mirror
            })
    );
    assert!(
        rejoined
            .context
            .pair_right
            .exceptions
            .iter()
            .all(|exception| {
                exception.kind != MirroredItemKind::Part || exception.mirror_id != second_mirror
            })
    );
}

#[test]
fn matrix_pair_resolution_refuses_duplicate_exact_durable_owner_atomically() {
    let (mut document, _) = independent_lifecycle_fixture();
    let pair_id = document.hardware.as_ref().unwrap().pairs[0].id.clone();
    let halves = document
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .iter_mut()
        .find(|pair| pair.id == pair_id)
        .unwrap()
        .halves
        .as_mut()
        .unwrap();
    let exact_owner = halves
        .correspondences
        .iter()
        .find(|record| {
            record.kind == MirroredItemKind::Matrix
                && record.source_id == "source-matrix"
                && record.mirror_id == "mirror-matrix"
        })
        .unwrap()
        .clone();
    halves.correspondences.push(exact_owner);
    let frozen = document.clone();
    let halves = document
        .hardware
        .as_ref()
        .unwrap()
        .pairs
        .iter()
        .find(|pair| pair.id == pair_id)
        .unwrap()
        .halves
        .as_ref()
        .unwrap();

    let error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::matrix_pair_for_association(
        &document,
        halves,
        MirroredItemKind::Part,
        "matrix/source-matrix/r0c0/diode",
        "matrix/mirror-matrix/r0c0/diode",
    )
    .expect_err("duplicate exact durable Matrix owners are ambiguous");

    assert_eq!(
        error,
        "A matrix-owned Part has an ambiguous durable Matrix owner"
    );
    assert_eq!(document, frozen);
}

#[test]
fn lifecycle_context_refuses_missing_companion_membership_without_pending_selection() {
    let (document, _) = independent_lifecycle_fixture();
    let pair_id = document.hardware.as_ref().unwrap().pairs[0].id.clone();
    let context =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &document, &pair_id, None,
        )
        .unwrap();
    assert!(context.pending_pair_transitions.is_empty());
    assert!(context.matrix_pair.is_none());

    let mut wrong_membership = document.clone();
    wrong_membership
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .retain(|id| id != "matrix/mirror-matrix/r0c0/diode");
    let frozen_document = wrong_membership.clone();
    let frozen_context = context.clone();

    let error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::validate_lifecycle_context(&wrong_membership, &context)
        .expect_err("a companion must remain in its actual mirror Matrix membership");

    assert_eq!(
        error,
        "Lifecycle ownership does not match actual target membership"
    );
    assert_eq!(wrong_membership, frozen_document);
    assert_eq!(context, frozen_context);
}

#[test]
fn matrix_owned_exception_context_rejects_wrong_owner_and_membership_atomically() {
    let (mut before, mut context) = independent_lifecycle_fixture();
    add_second_matrix_pair(&mut before, &mut context).unwrap();
    let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
    let first_owner =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &before,
            &pair_id,
            Some(
                boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                    source_id: "source-matrix".into(),
                    mirror_id: "mirror-matrix".into(),
                },
            ),
        )
        .unwrap();
    let second_child = "matrix/source-matrix-2/r0c0";
    let second_mirror = "matrix/mirror-matrix-2/r0c0";
    let frozen = before.clone();
    let mut wrong_owner = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &first_owner,
        MirroredItemKind::Part,
        second_child,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    wrong_owner.matrix_pair = Some(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        },
    );
    let wrong_owner_error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &wrong_owner,
        crate::support::test_dispatch(),
    )
    .expect_err("a selected child cannot borrow another Matrix's owner");
    assert!(
        wrong_owner_error.contains("ownership"),
        "{wrong_owner_error}"
    );
    assert_eq!(before, frozen);

    let mut wrong_membership = before.clone();
    wrong_membership
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == "mirror-matrix-2")
        .unwrap()
        .part_ids
        .retain(|id| id != second_mirror);
    let exact_second =
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::lifecycle_context(
            &before,
            &pair_id,
            Some(
                boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                    source_id: "source-matrix-2".into(),
                    mirror_id: "mirror-matrix-2".into(),
                },
            ),
        )
        .unwrap();
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &exact_second,
        MirroredItemKind::Part,
        second_child,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let changed_after_selection = prepare_lifecycle_transition(
        &before,
        &wrong_membership,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect_err("a valid selection cannot authorize a stale post-operation membership");
    assert!(
        changed_after_selection.contains("membership"),
        "{changed_after_selection}"
    );
    let membership_error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &wrong_membership,
        &wrong_membership,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect_err("actual Matrix target membership remains mandatory");
    assert!(
        membership_error.contains("membership"),
        "{membership_error}"
    );
    assert_eq!(before, frozen);
}

#[test]
fn an_overlay_with_saved_copper_round_trips_and_a_legacy_exception_is_not_linked() {
    let value = json!({
        "sourceBoardId": "source-board", "mirrorBoardId": "mirror-board",
        "correspondences": [{ "kind": "part", "sourceId": "source-part", "mirrorId": "mirror-part",
            "companionMode": "duplicate-position" }],
        "exceptions": [
            { "kind": "part", "mirrorId": "mirror-part", "state": "omitted", "removedCopper": {
                "traces": [{ "id": "t", "start": {"x": 0.0, "y": 0.0}, "end": {"x": 1.0, "y": 0.0},
                    "width": 0.2, "layer": "front", "netId": "n" }],
                "vias": [{ "id": "v", "at": {"x": 2.0, "y": 0.0}, "size": 0.8, "drill": 0.4 }],
                "pads": [{ "partId": "mirror-part", "padId": "1", "at": {"x": 3.0, "y": 4.0} }] } },
            { "kind": "part", "mirrorId": "legacy-part", "state": "local" }
        ]
    });
    let halves: MirroredHalves = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&halves).unwrap(),
        value,
        "the shape is stable"
    );
    let omitted = &halves.exceptions[0];
    assert_eq!(halves.overlay_source(omitted), Some("source-part"));
    assert_eq!(
        omitted.removed_copper.as_ref().unwrap().pads[0].at,
        Vec2 { x: 3.0, y: 4.0 }
    );
    let legacy = &halves.exceptions[1];
    assert_eq!(
        halves.overlay_source(legacy),
        None,
        "a legacy record infers no source"
    );
    assert!(legacy.removed_copper.is_none());
    assert_eq!(
        halves.correspondences.len(),
        1,
        "and no correspondence is invented for it"
    );
}
pub(crate) fn replacement_retirement_fixture() -> ProjectDoc {
    use boardstudio_model::model::MirroredItemKind as Kind;

    let mut document = valid_omitted_document();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    let mut rows = vec![(Kind::Part, "source-part", "mirror-part")];
    for (kind, source_id, mirror_id) in [
        (Kind::Matrix, "source-matrix", "mirror-matrix"),
        (Kind::Layout, "source-layout", "mirror-layout"),
        (Kind::Net, "source-net", "mirror-net"),
        (Kind::EmbeddedCircuit, "source-circuit", "mirror-circuit"),
    ] {
        rows.push((kind, source_id, mirror_id));
    }
    halves.correspondences = rows
        .iter()
        .map(|(kind, source_id, mirror_id)| MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: *kind,
            source_id: (*source_id).into(),
            mirror_id: (*mirror_id).into(),
        })
        .collect();
    halves.exceptions = rows
        .iter()
        .map(|(kind, _, mirror_id)| MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: *kind,
            mirror_id: (*mirror_id).into(),
            state: MirroredExceptionState::Omitted,
            removed_copper: None,
        })
        .collect();
    document.matrices.push(
        serde_json::from_value(json!({
            "id":"source-matrix","name":"Source","rows":1,"columns":1,
            "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
            "definitionId":"test-footprint","partIds":[],"boardId":"source-board"
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id":"source-layout","name":"Source","boardId":"source-board",
            "matrixIds": ["source-matrix"],"partIds":[]
        }))
        .unwrap(),
    );
    document.nets.push(Net {
        id: "source-net".into(),
        name: "Source net".into(),
        pins: Vec::new(),
    });
    document.boards[0].net_ids.push("source-net".into());
    document.embedded_circuits.push(
        serde_json::from_value(json!({
            "id":"source-circuit","definitionId":"test-circuit",
            "hostBoardId":"source-board","partIds":[],"netIds":[],"ports":{}
        }))
        .unwrap(),
    );
    document
}

pub(crate) fn remove_replacement_retirement_sources(mut replacement: ProjectDoc) -> ProjectDoc {
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    replacement
        .matrices
        .retain(|matrix| matrix.id != "source-matrix");
    replacement
        .layouts
        .retain(|layout| layout.id != "source-layout");
    replacement.nets.retain(|net| net.id != "source-net");
    replacement.boards[0]
        .net_ids
        .retain(|net_id| net_id != "source-net");
    replacement
        .embedded_circuits
        .retain(|circuit| circuit.id != "source-circuit");
    replacement
}

#[test]
fn replacement_batch_retires_typed_part_matrix_layout_net_and_circuit_rows() {
    let accepted = replacement_retirement_fixture();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).unwrap();
    let mut replacement = remove_replacement_retirement_sources(accepted.clone());

    retire_replacement_records(&accepted, &mut replacement).unwrap();

    let halves = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(&replacement).is_ok()
    );
}

#[test]
fn replacement_batch_keeps_local_target_geometry_byte_for_byte() {
    let mut accepted = valid_omitted_document();
    let mirror = part("mirror-part");
    accepted.parts.push(mirror.clone());
    accepted.boards[1].part_ids.push("mirror-part".into());
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions[0].state = MirroredExceptionState::Local;
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).is_ok());
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");

    retire_replacement_records(&accepted, &mut replacement).unwrap();

    assert_eq!(
        replacement
            .parts
            .iter()
            .find(|part| part.id == "mirror-part"),
        Some(&mirror)
    );
    assert!(
        replacement.boards[1]
            .part_ids
            .iter()
            .any(|id| id == "mirror-part")
    );
    let halves = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
}

#[test]
fn replacement_batch_retires_exact_record_with_unrelated_halves_metadata_change() {
    let accepted = valid_omitted_document();
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    let halves = replacement.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.companion_mode = MirroredCompanionMode::DuplicatePosition;

    retire_replacement_records(&accepted, &mut replacement).unwrap();

    let halves = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
    assert_eq!(
        halves.companion_mode,
        MirroredCompanionMode::DuplicatePosition
    );
}

#[test]
fn replacement_batch_refuses_incomplete_matrix_owned_part_closure_atomically() {
    let mut accepted = valid_omitted_document();
    accepted.matrices.push(
        serde_json::from_value(json!({
            "id":"owner-matrix","rows":1,"columns":1,
            "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
            "definitionId":"test-footprint","partIds":["source-part"],"boardId":"source-board"
        }))
        .unwrap(),
    );
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    let halves_before = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();

    let refusal = retire_replacement_records(&accepted, &mut replacement).unwrap_err();

    assert!(
        refusal.contains("incomplete ownership closure"),
        "{refusal}"
    );
    assert_eq!(
        replacement.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap(),
        &halves_before,
        "a refused batch does not partially retire any metadata"
    );
}

#[test]
fn replacement_batch_leaves_unowned_kind_to_existing_document_validation() {
    use boardstudio_model::model::MirroredItemKind;

    let mut accepted = valid_omitted_document();
    let halves = accepted.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences[0].kind = MirroredItemKind::Outline;
    halves.correspondences[0].source_id = "source-outline".into();
    halves.correspondences[0].mirror_id = "mirror-outline".into();
    halves.exceptions[0].kind = MirroredItemKind::Outline;
    halves.exceptions[0].mirror_id = "mirror-outline".into();
    accepted.boards[0].outline_ids.push("source-outline".into());
    accepted.outline.push(
        serde_json::from_value(json!({
            "kind":"rect","id":"source-outline","center":{"x":0.0,"y":0.0},
            "size":{"x":10.0,"y":10.0},"radius":0.0,"operation":"add"
        }))
        .unwrap(),
    );
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).is_ok());
    let mut replacement = accepted.clone();
    replacement
        .outline
        .retain(|outline| outline.id() != "source-outline");
    replacement.boards[0]
        .outline_ids
        .retain(|outline_id| outline_id != "source-outline");
    let halves_before = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();

    let refusal = retire_replacement_records(&accepted, &mut replacement).unwrap_err();

    assert!(
        refusal.contains("names outline"),
        "existing halves validation owns unsupported-kind refusal: {refusal}"
    );
    assert_eq!(
        replacement.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap(),
        &halves_before
    );
}

pub(crate) fn prepare_exception_intent(
    before: &boardstudio_model::model::ProjectDoc,
    post_source: &boardstudio_model::model::ProjectDoc,
    pair_id: &str,
    kind: boardstudio_model::model::MirroredItemKind,
    source_id: &str,
    mirror_id: &str,
    state: TransitionState,
) -> Result<
    Vec<boardstudio_core_mirrored_half::mirrored_half::reconcile::MembershipTransitionOutput>,
    String,
> {
    let hardware = before
        .hardware
        .as_ref()
        .ok_or("Lifecycle hardware is unavailable")?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or("The exact lifecycle pair is unavailable")?;
    if pairs.next().is_some() {
        return Err("The exact lifecycle pair is ambiguous".into());
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or("The exact lifecycle pair has no halves record")?;
    let matrix_pair = matrix_pair_for_association(before, halves, kind, source_id, mirror_id)?;
    let context = lifecycle_context(before, pair_id, matrix_pair)?;
    let pending = transition_exception_context(&context, kind, source_id, mirror_id, state)?;
    prepare_lifecycle_transition(
        before,
        post_source,
        &pending,
        crate::support::test_dispatch(),
    )
    .map(|output| vec![output])
}
