use super::*;
use crate::constraint_lifecycle::{board, part, valid_omitted_document};
use crate::independent_exceptions::{independent_lifecycle_fixture, local_transition_fixture};
use crate::multi_matrix_and_replacement::prepare_exception_intent;

#[test]
fn local_matrix_without_part_links_rejects_stale_lifecycle_board_ids() {
    let (mut doc, original) = local_transition_fixture().unwrap();
    let pair = &mut doc.hardware.as_mut().unwrap().pairs[0];
    let halves = pair.halves.as_mut().unwrap();
    halves
        .correspondences
        .retain(|row| row.kind == MirroredItemKind::Matrix);
    halves.exceptions = vec![MirroredException {
        kind: MirroredItemKind::Matrix,
        mirror_id: "mirror-matrix".into(),
        state: MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    }];
    let context = lifecycle_context(&doc, &original.pair_id, original.matrix_pair).unwrap();
    let prepare = |context: &MembershipContext| {
        prepare_lifecycle_transition(&doc, &doc, context, crate::support::test_dispatch())
    };
    prepare(&context).expect("canonical matrix-only context is valid");
    for source in [true, false] {
        let mut stale = context.clone();
        if source {
            stale.source_board_id = "stale-source".into();
        } else {
            stale.mirror_board_id = "stale-mirror".into();
        }
        assert_eq!(
            prepare(&stale).err().as_deref(),
            Some("The reconciliation input does not match its canonical pair")
        );
    }
}

#[test]
fn matrix_owner_lifecycle_commits_one_exception_and_preserves_explicit_child() {
    let (before, mut context) = independent_lifecycle_fixture();
    for association in context.associations.iter_mut().filter(|a| {
        a.kind == MirroredItemKind::Part
            && a.source_id.starts_with("matrix/")
            && a.parent_source_id.is_none()
    }) {
        association.parent_source_id = Some("source-matrix".into());
        association.parent_mirror_id = Some("mirror-matrix".into());
    }
    let child_source = "matrix/source-matrix/r0c0";
    let child_mirror = "matrix/mirror-matrix/r0c0";
    let child_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        child_source,
        child_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let child = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &child_context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let explicit_part = child
        .draft
        .parts
        .iter()
        .find(|p| p.id == child_mirror)
        .unwrap()
        .clone();
    let owner_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &child.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Local,
    )
    .unwrap();
    let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &child.draft,
        &child.draft,
        &owner_context,
        crate::support::test_dispatch(),
    )
    .expect("a Matrix owner can become Local through the complete transaction");
    let local_matrix = local
        .draft
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .clone();
    let local_parts: Vec<_> = local
        .draft
        .parts
        .iter()
        .filter(|p| local_matrix.part_ids.contains(&p.id))
        .cloned()
        .collect();
    let mut post = local.draft.clone();
    let mut source = post
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .clone();
    source.origin.x += 12.0;
    boardstudio_core_kernel::matrix::set_matrix(&mut post, &source).unwrap();
    let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &post,
        &local.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        edited
            .draft
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix"),
        Some(&local_matrix)
    );
    assert_eq!(
        edited
            .draft
            .parts
            .iter()
            .filter(|p| local_matrix.part_ids.contains(&p.id))
            .cloned()
            .collect::<Vec<_>>(),
        local_parts
    );
    let omitted_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &edited.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
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
    assert!(
        omitted
            .draft
            .matrices
            .iter()
            .all(|m| m.id != "mirror-matrix")
    );
    assert_eq!(
        omitted.draft.parts.iter().find(|p| p.id == child_mirror),
        Some(&explicit_part)
    );
    assert_eq!(omitted.context.pair_right.exceptions.len(), 2);
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted.draft).unwrap()).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&decoded).unwrap();
    let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &decoded,
        &decoded,
        &omitted.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeat.draft, decoded);
    assert_eq!(repeat.context, omitted.context);
    let rejoin_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &repeat.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &repeat.draft,
        &repeat.draft,
        &rejoin_context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        rejoined.draft.parts.iter().find(|p| p.id == child_mirror),
        Some(&explicit_part)
    );
    assert_eq!(rejoined.context.pair_right.exceptions.len(), 1);
    assert_eq!(
        rejoined.context.pair_right.exceptions[0].mirror_id,
        child_mirror
    );
    assert!(rejoined.draft.parts.iter().any(|p| p.id == "mirror-screen"));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();
    let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &rejoined.draft,
        &rejoined.draft,
        &rejoined.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeat.draft, rejoined.draft);
    assert_eq!(repeat.context, rejoined.context);
    assert!(repeat.changed_document_ids.is_empty());
}

pub(crate) fn add_lifecycle_association(
    document: &mut ProjectDoc,
    context: &mut MembershipContext,
    kind: MirroredItemKind,
    source: &str,
    mirror: &str,
    parent: Option<(&str, &str)>,
) {
    context
        .pair_right
        .correspondences
        .push(MirroredCorrespondence {
            kind,
            source_id: source.into(),
            mirror_id: mirror.into(),
            companion_mode: None,
            companion_tweak: None,
        });
    context.associations.push(TransitionAssociation {
        kind,
        source_id: source.into(),
        mirror_id: mirror.into(),
        parent_source_id: parent.map(|p| p.0.into()),
        parent_mirror_id: parent.map(|p| p.1.into()),
        state: TransitionState::Linked,
        explicit_state: false,
        raw_mode_override: None,
        raw_tweak: None,
        effective_mode: context.pair_right.companion_mode,
        effective_tweak: None,
    });
    if kind == MirroredItemKind::Part {
        context.owners.insert(
            source.into(),
            FinalOwner::Board(context.source_board_id.clone()),
        );
        context.owners.insert(
            mirror.into(),
            FinalOwner::Board(context.mirror_board_id.clone()),
        );
    }
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
}

pub(crate) fn circuit_lifecycle_fixture() -> (ProjectDoc, MembershipContext) {
    let (mut document, mut context) = independent_lifecycle_fixture();
    document.definitions.push(serde_json::from_value(json!({"id":"circuit-part","name":"Circuit part","kind":"custom","courtyard":[],
        "pads":[{"id":"pad-1","number":"1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"}]})).unwrap());
    for part in document
        .parts
        .iter_mut()
        .filter(|part| part.id.ends_with("-screen"))
    {
        part.definition_id = "circuit-part".into();
    }
    for member in ["U1", "U2"] {
        for (half, board_id) in [("source", "source-board"), ("mirror", "mirror-board")] {
            let id = format!("embedded/{half}-circuit/{member}");
            let mut part = document.parts[0].clone();
            part.id = id.clone();
            part.reference = format!("{half}-{member}");
            part.definition_id = "circuit-part".into();
            part.properties = Some(BTreeMap::from([("revision".into(), json!(1))]));
            document.parts.push(part);
            document
                .boards
                .iter_mut()
                .find(|b| b.id == board_id)
                .unwrap()
                .part_ids
                .push(id);
        }
    }
    for half in ["source", "mirror"] {
        let private = format!("embedded/{half}-circuit/N1");
        let joined = format!("{half}-host-net");
        document.nets.extend([
            Net {
                id: private.clone(),
                name: "PRIVATE".into(),
                pins: vec![Pin {
                    part_id: format!("embedded/{half}-circuit/U1"),
                    pad_id: "pad-1".into(),
                }],
            },
            Net {
                id: joined.clone(),
                name: format!("{half} HOST"),
                pins: vec![
                    Pin {
                        part_id: format!("embedded/{half}-circuit/U1"),
                        pad_id: "pad-1".into(),
                    },
                    Pin {
                        part_id: format!("embedded/{half}-circuit/U2"),
                        pad_id: "pad-1".into(),
                    },
                    Pin {
                        part_id: format!("{half}-screen"),
                        pad_id: "pad-1".into(),
                    },
                ],
            },
        ]);
        let board_id = format!("{half}-board");
        document
            .boards
            .iter_mut()
            .find(|b| b.id == board_id)
            .unwrap()
            .net_ids
            .extend([private.clone(), joined.clone()]);
        document.embedded_circuits.push(EmbeddedCircuit {
            id: format!("{half}-circuit"),
            definition_id: "circuit-module".into(),
            host_board_id: board_id,
            part_ids: ["U1", "U2"]
                .map(|member| format!("embedded/{half}-circuit/{member}"))
                .to_vec(),
            net_ids: vec![private, joined.clone()],
            ports: BTreeMap::from([("signal".into(), joined)]),
        });
    }
    document.module_definitions.push(serde_json::from_value(json!({
        "id":"circuit-module","name":"Circuit module","family":"fixture","variant":"fixture",
        "source":{"repository":"https://example.invalid/fixture","revision":"fixture","path":"fixture.kicad_pcb","license":"MIT"},
        "board":{"contours":[]},"mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},"constituents":[]
    })).unwrap());
    for half in ["source", "mirror"] {
        document.modules.push(serde_json::from_value(json!({
            "id":format!("{half}-module"),"definitionId":"circuit-module","hostBoardId":format!("{half}-board"),
            "hostFace":"front","facingFace":"back","at":{"x":4,"y":3},"rotation":12,"gap":0.4,"attachment":"board",
            "connection":{"hostConnectorPartId":format!("{half}-screen"),"modulePortId":"host","busId":"","cableType":"fixture","assignments":{},"railVoltages":{}}
        })).unwrap());
    }
    document.layouts.push(Layout {
        id: "independent-layout".into(),
        name: "Independent".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec!["mirror-screen".into()],
        mirror_link: None,
    });
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        None,
    );
    for member in ["U1", "U2"] {
        add_lifecycle_association(
            &mut document,
            &mut context,
            MirroredItemKind::Part,
            &format!("embedded/source-circuit/{member}"),
            &format!("embedded/mirror-circuit/{member}"),
            Some(("source-circuit", "mirror-circuit")),
        );
    }
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::Net,
        "embedded/source-circuit/N1",
        "embedded/mirror-circuit/N1",
        Some(("source-circuit", "mirror-circuit")),
    );
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::Net,
        "source-host-net",
        "mirror-host-net",
        None,
    );
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::Module,
        "source-module",
        "mirror-module",
        None,
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    (document, context)
}

pub(crate) fn remove_matrix_correspondence(document: &mut ProjectDoc) {
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves
        .correspondences
        .retain(|record| record.kind != MirroredItemKind::Matrix);
}

#[test]
fn matrixless_exception_refuses_stale_ambiguous_and_late_invalid_inputs_atomically() {
    let (mut before, _) = independent_lifecycle_fixture();
    let pair_id = before.hardware.as_ref().unwrap().pairs[0].id.clone();
    remove_matrix_correspondence(&mut before);
    let baseline = before.clone();

    let stale = prepare_exception_intent(
        &before,
        &before,
        &pair_id,
        MirroredItemKind::Part,
        "stale-source",
        "stale-mirror",
        TransitionState::Local,
    )
    .unwrap_err();
    assert!(stale.contains("exact private association"), "{stale}");
    assert_eq!(before, baseline);

    let matrix_owned = prepare_exception_intent(
        &before,
        &before,
        &pair_id,
        MirroredItemKind::Part,
        "matrix/source-matrix/r0c0",
        "matrix/mirror-matrix/r0c0",
        TransitionState::Local,
    )
    .unwrap_err();
    assert!(matrix_owned.contains("matrix-owned Part"), "{matrix_owned}");
    assert_eq!(before, baseline);

    let mut ambiguous = before.clone();
    let duplicate = ambiguous.hardware.as_ref().unwrap().pairs[0].clone();
    ambiguous.hardware.as_mut().unwrap().pairs.push(duplicate);
    let ambiguous_snapshot = ambiguous.clone();
    let error = prepare_exception_intent(
        &ambiguous,
        &ambiguous,
        &pair_id,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap_err();
    assert!(error.contains("ambiguous"), "{error}");
    assert_eq!(ambiguous, ambiguous_snapshot);

    let (mut late_baseline, _) = circuit_lifecycle_fixture();
    remove_matrix_correspondence(&mut late_baseline);
    let mut late_invalid = late_baseline.clone();
    late_invalid
        .modules
        .iter_mut()
        .find(|module| module.id == "source-module")
        .unwrap()
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "missing-source-connector".into();
    let post_snapshot = late_invalid.clone();
    let error = prepare_exception_intent(
        &late_baseline,
        &late_invalid,
        &pair_id,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(before, baseline);
    assert_eq!(late_invalid, post_snapshot);
}

#[test]
fn circuit_owner_lifecycle_preserves_explicit_child_connector_layout_and_joined_net() {
    use boardstudio_core_mirrored_half::mirrored_half::reconcile::{
        MembershipTransitionOutput,
        lifecycle::{prepare_lifecycle_transition, transition_exception_context},
    };

    let dispatch = crate::support::test_dispatch();
    let set_owner = |context: &MembershipContext, state| {
        transition_exception_context(
            context,
            MirroredItemKind::EmbeddedCircuit,
            "source-circuit",
            "mirror-circuit",
            state,
        )
        .unwrap()
    };

    let (before, context) = circuit_lifecycle_fixture();
    let child_context = transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "embedded/source-circuit/U2",
        "embedded/mirror-circuit/U2",
        TransitionState::Local,
    )
    .unwrap();
    let child = prepare_lifecycle_transition(&before, &before, &child_context, dispatch).unwrap();
    let retained = child
        .draft
        .parts
        .iter()
        .find(|p| p.id == "embedded/mirror-circuit/U2")
        .unwrap()
        .clone();
    let module = child
        .draft
        .modules
        .iter()
        .find(|m| m.id == "mirror-module")
        .unwrap()
        .clone();
    let layout = child.draft.layouts[0].clone();
    let inherited = child
        .draft
        .parts
        .iter()
        .find(|p| p.id == "embedded/mirror-circuit/U1")
        .unwrap()
        .clone();
    let pending = set_owner(&child.context, TransitionState::Local);
    let local =
        prepare_lifecycle_transition(&child.draft, &child.draft, &pending, dispatch).unwrap();
    let mut post = local.draft.clone();
    let source = post
        .parts
        .iter_mut()
        .find(|p| p.id == "embedded/source-circuit/U1")
        .unwrap();
    source.pose.at.x += 19.0;
    source.properties = Some(BTreeMap::from([("revision".into(), json!(3))]));
    let edited =
        prepare_lifecycle_transition(&local.draft, &post, &local.context, dispatch).unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == inherited.id),
        Some(&inherited)
    );
    let pending = set_owner(&edited.context, TransitionState::Omitted);
    let omitted = prepare_lifecycle_transition(&edited.draft, &edited.draft, &pending, dispatch)
        .expect(
            "a circuit Omit removes its inherited closure through the existing domain deletion",
        );
    assert!(
        omitted
            .draft
            .embedded_circuits
            .iter()
            .all(|c| c.id != "mirror-circuit")
    );
    assert!(omitted.draft.parts.iter().all(|p| p.id != inherited.id));
    assert_eq!(
        omitted.draft.parts.iter().find(|p| p.id == retained.id),
        Some(&retained)
    );
    assert_eq!(
        omitted.draft.modules.iter().find(|m| m.id == module.id),
        Some(&module)
    );
    assert_eq!(omitted.draft.layouts[0], layout);
    assert!(omitted.draft.parts.iter().any(|p| p.id == "mirror-screen"));
    assert!(
        omitted
            .draft
            .nets
            .iter()
            .all(|n| n.id != "embedded/mirror-circuit/N1")
    );
    let joined = omitted
        .draft
        .nets
        .iter()
        .find(|n| n.id == "mirror-host-net")
        .unwrap();
    assert_eq!(
        joined
            .pins
            .iter()
            .map(|p| p.part_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["embedded/mirror-circuit/U2", "mirror-screen"])
    );
    assert_eq!(omitted.context.pair_right.exceptions.len(), 2);
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted.draft).unwrap()).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&decoded).unwrap();
    let repeat =
        prepare_lifecycle_transition(&decoded, &decoded, &omitted.context, dispatch).unwrap();
    assert_eq!(repeat.draft, decoded);
    assert_eq!(repeat.context, omitted.context);
    let pending = set_owner(&repeat.context, TransitionState::Linked);
    let rejoined =
        prepare_lifecycle_transition(&repeat.draft, &repeat.draft, &pending, dispatch).unwrap();
    assert_eq!(
        rejoined.draft.parts.iter().find(|p| p.id == retained.id),
        Some(&retained)
    );
    assert_eq!(
        rejoined
            .draft
            .parts
            .iter()
            .find(|p| p.id == inherited.id)
            .unwrap()
            .properties,
        rejoined
            .draft
            .parts
            .iter()
            .find(|p| p.id == "embedded/source-circuit/U1")
            .unwrap()
            .properties
    );
    assert_eq!(rejoined.context.pair_right.exceptions.len(), 1);
    assert!(
        rejoined
            .draft
            .embedded_circuits
            .iter()
            .any(|c| c.id == "mirror-circuit")
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();
    let repeat = prepare_lifecycle_transition(
        &rejoined.draft,
        &rejoined.draft,
        &rejoined.context,
        dispatch,
    )
    .unwrap();
    assert_eq!(repeat.draft, rejoined.draft);
    assert_eq!(repeat.context, rejoined.context);
    assert!(repeat.changed_document_ids.is_empty());

    // The same circuit lifecycle holds with no Matrix correspondences (ticket 150).
    let (mut matrixless, _) = circuit_lifecycle_fixture();
    remove_matrix_correspondence(&mut matrixless);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&matrixless).unwrap();
    let pair_id = matrixless.hardware.as_ref().unwrap().pairs[0].id.clone();
    let child_id = "embedded/mirror-circuit/U2";
    let child_geometry = matrixless
        .parts
        .iter()
        .find(|p| p.id == child_id)
        .unwrap()
        .clone();
    let mut rejoined: Option<MembershipTransitionOutput> = None;
    for (kind, source, mirror, state) in [
        (
            MirroredItemKind::Part,
            "embedded/source-circuit/U2",
            child_id,
            TransitionState::Local,
        ),
        (
            MirroredItemKind::EmbeddedCircuit,
            "source-circuit",
            "mirror-circuit",
            TransitionState::Local,
        ),
        (
            MirroredItemKind::EmbeddedCircuit,
            "source-circuit",
            "mirror-circuit",
            TransitionState::Linked,
        ),
    ] {
        let before = rejoined
            .as_ref()
            .map_or(&matrixless, |output| &output.draft);
        let step = prepare_exception_intent(before, before, &pair_id, kind, source, mirror, state)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(
            step.draft.parts.iter().find(|p| p.id == child_id),
            Some(&child_geometry),
            "matrixless {mirror} {state:?} keeps the explicit child's Local geometry"
        );
        rejoined = Some(step);
    }
    let rejoined = rejoined.unwrap();
    assert_eq!(rejoined.context.pair_right.exceptions.len(), 1);
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();
}

pub(crate) fn named_companion_fixture(component: &str) -> (ProjectDoc, MembershipContext) {
    let (mut document, mut context) = local_transition_fixture().unwrap();
    let rename = |id: &str| {
        id.strip_suffix("/diode")
            .map_or_else(|| id.to_owned(), |parent| format!("{parent}/{component}"))
    };
    let definition = document
        .definitions
        .iter_mut()
        .find(|d| d.id == "diode")
        .unwrap();
    definition.id = component.into();
    definition.name = component.into();
    definition.kind = PartKind::Custom;
    for matrix in &mut document.matrices {
        for cell in &mut matrix.cells {
            for assembly in &mut cell.assemblies {
                assembly.id = component.into();
                assembly.definition_id = component.into();
            }
        }
        matrix.part_ids = matrix.part_ids.iter().map(|id| rename(id)).collect();
    }
    for part in &mut document.parts {
        part.id = rename(&part.id);
        if part.definition_id == "diode" {
            part.definition_id = component.into();
        }
    }
    for board in &mut document.boards {
        board.part_ids = board.part_ids.iter().map(|id| rename(id)).collect();
    }
    for record in &mut context.pair_right.correspondences {
        record.source_id = rename(&record.source_id);
        record.mirror_id = rename(&record.mirror_id);
    }
    for association in &mut context.associations {
        association.source_id = rename(&association.source_id);
        association.mirror_id = rename(&association.mirror_id);
    }
    context.owners = context
        .owners
        .into_iter()
        .map(|(id, owner)| (rename(&id), owner))
        .collect();
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    (document, context)
}

#[test]
fn explicit_companion_lifecycle_keeps_mode_tweak_and_omission_through_owner_rejoin() {
    for component in ["diode", "led"] {
        let (mut before, mut context) = named_companion_fixture(component);
        let source_name = format!("matrix/source-matrix/r0c1/{component}");
        let mirror_name = format!("matrix/mirror-matrix/r0c1/{component}");
        let source_id = source_name.as_str();
        let mirror_id = mirror_name.as_str();
        let tweak = MirroredCompanionTweak {
            x: 0.4,
            y: -0.7,
            rotation: 17.0,
        };
        let association = context
            .associations
            .iter_mut()
            .find(|a| a.source_id == source_id)
            .unwrap();
        association.raw_mode_override = Some(MirroredCompanionMode::LiteralMirrorWithTweak);
        association.raw_tweak = Some(tweak);
        association.effective_mode = MirroredCompanionMode::LiteralMirrorWithTweak;
        association.effective_tweak = Some(tweak);
        let record = context
            .pair_right
            .correspondences
            .iter_mut()
            .find(|r| r.source_id == source_id)
            .unwrap();
        record.companion_mode = association.raw_mode_override;
        record.companion_tweak = association.raw_tweak;
        before.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
        let local_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Local,
        )
        .unwrap();
        let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &before,
            &before,
            &local_context,
            crate::support::test_dispatch(),
        )
        .expect("a companion is selectively Local through the complete transaction");
        let retained = local
            .draft
            .parts
            .iter()
            .find(|p| p.id == mirror_id)
            .unwrap()
            .clone();
        let mut post = local.draft.clone();
        let mut source = post
            .matrices
            .iter()
            .find(|m| m.id == "source-matrix")
            .unwrap()
            .clone();
        source
            .cells
            .iter_mut()
            .find(|c| c.row == 0 && c.column == 1)
            .unwrap()
            .assemblies[0]
            .offset
            .x += 5.0;
        boardstudio_core_kernel::matrix::set_matrix(&mut post, &source).unwrap();
        let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &local.draft,
            &post,
            &local.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(
            edited.draft.parts.iter().find(|p| p.id == mirror_id),
            Some(&retained)
        );
        let omitted_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &edited.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
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
        assert!(omitted.draft.parts.iter().all(|p| p.id != mirror_id));
        let pair = &omitted.context.pair_right;
        let exception = pair
            .exceptions
            .iter()
            .find(|e| pair.overlay_source(e) == Some(source_id))
            .unwrap();
        assert_eq!(
            exception.companion_mode, None,
            "the overlay is only the state"
        );
        let correspondence = pair.overlaid_correspondence(exception).unwrap();
        assert_eq!(
            correspondence.companion_mode,
            Some(MirroredCompanionMode::LiteralMirrorWithTweak)
        );
        assert_eq!(correspondence.companion_tweak, Some(tweak));
        let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &omitted.draft,
            &omitted.draft,
            &omitted.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(repeat.draft, omitted.draft);
        let parent_context = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &repeat.context,
            MirroredItemKind::Part,
            "matrix/source-matrix/r0c1",
            "matrix/mirror-matrix/r0c1",
            TransitionState::Local,
        )
        .unwrap();
        let parent_local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &repeat.draft,
            &repeat.draft,
            &parent_context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        let rejoin_parent = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &parent_local.context,
            MirroredItemKind::Part,
            "matrix/source-matrix/r0c1",
            "matrix/mirror-matrix/r0c1",
            TransitionState::Linked,
        )
        .unwrap();
        let parent_linked = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &parent_local.draft,
            &parent_local.draft,
            &rejoin_parent,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert!(parent_linked.draft.parts.iter().all(|p| p.id != mirror_id));
        assert_eq!(parent_linked.context.pair_right.exceptions.len(), 1);
        let rejoin = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &parent_linked.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &parent_linked.draft,
            &parent_linked.draft,
            &rejoin,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert!(rejoined.draft.parts.iter().any(|p| p.id == mirror_id));
        let record = rejoined
            .context
            .pair_right
            .correspondences
            .iter()
            .find(|r| r.source_id == source_id)
            .unwrap();
        assert_eq!(
            record.companion_mode,
            Some(MirroredCompanionMode::LiteralMirrorWithTweak)
        );
        assert_eq!(record.companion_tweak, Some(tweak));
        let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &rejoined.draft,
            &rejoined.draft,
            &rejoined.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(repeat.draft, rejoined.draft);
        assert_eq!(repeat.context, rejoined.context);
        assert!(repeat.changed_document_ids.is_empty());
    }
}

#[test]
fn lifecycle_refuses_target_membership_that_disagrees_with_actual_circuit_ownership() {
    let (mut before, context) = circuit_lifecycle_fixture();
    before
        .embedded_circuits
        .iter_mut()
        .find(|c| c.id == "mirror-circuit")
        .unwrap()
        .part_ids
        .retain(|id| id != "embedded/mirror-circuit/U1");
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Omitted,
    )
    .unwrap();
    let frozen = (before.clone(), pending.clone());
    let error = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect_err("target namespace alone is not actual circuit ownership");
    assert!(error.contains("ownership"), "{error}");
    assert_eq!((before, pending), frozen);
}

#[test]
fn omitted_target_on_a_different_board_is_not_absent() {
    let mut document = valid_omitted_document();
    document.parts.push(part("mirror-part"));
    document
        .boards
        .push(board("unrelated-board", &["mirror-part"]));
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(&document).is_err(),
        "an Omitted target cannot alias live geometry on another board"
    );
}

#[test]
fn pending_baseline_and_selection_tampering_refuse_atomically() {
    let (before, context) = independent_lifecycle_fixture();
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    for case in 0..5 {
        let mut tampered = pending.clone();
        match case {
            0 => tampered
                .pending_pair_baseline
                .as_mut()
                .unwrap()
                .correspondences
                .reverse(),
            1 => {
                tampered.pending_pair_baseline = None;
            }
            2 => {
                tampered
                    .pending_pair_transitions
                    .insert("unrelated-source".into(), TransitionState::Linked);
            }
            3 => {
                tampered
                    .pending_rejoins
                    .insert("source-screen".into(), TransitionState::Omitted);
            }
            _ => {
                tampered
                    .associations
                    .iter_mut()
                    .find(|a| a.source_id == "source-screen")
                    .unwrap()
                    .mirror_id = "mirror-encoder".into();
            }
        }
        let frozen = (before.clone(), tampered.clone());
        assert!(
            boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &before,
                &before,
                &tampered,
                crate::support::test_dispatch()
            )
            .is_err(),
            "case {case}"
        );
        assert_eq!((before.clone(), tampered), frozen, "case {case}");
    }
}

#[test]
fn legacy_v5_local_parts_are_preserved_without_inventing_source_associations() {
    let (mut document, mut context) = independent_lifecycle_fixture();
    document.format_version = 5;
    let mut legacy = document
        .parts
        .iter()
        .find(|p| p.id == "mirror-screen")
        .unwrap()
        .clone();
    legacy.id = "legacy-local-part".into();
    legacy.reference = "LEGACY1".into();
    document.parts.push(legacy.clone());
    document
        .boards
        .iter_mut()
        .find(|b| b.id == "mirror-board")
        .unwrap()
        .part_ids
        .push(legacy.id.clone());
    context
        .owners
        .insert(legacy.id.clone(), FinalOwner::Board("mirror-board".into()));
    context.pair_right.exceptions.push(MirroredException {
        kind: MirroredItemKind::Part,
        mirror_id: legacy.id.clone(),
        state: MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    });
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    assert_eq!(document.format_version, 5);
    let output = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &document,
        &document,
        &context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
    assert_eq!(decoded.format_version, 5);
    assert_eq!(
        decoded.parts.iter().find(|p| p.id == legacy.id),
        Some(&legacy)
    );
    assert!(
        output
            .context
            .pair_right
            .exceptions
            .iter()
            .any(|e| e.mirror_id == legacy.id
                && output.context.pair_right.overlay_source(e).is_none())
    );
    let frozen = output.context.clone();
    assert!(
        boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &output.context,
            MirroredItemKind::Part,
            "source-screen",
            &legacy.id,
            TransitionState::Linked
        )
        .is_err()
    );
    assert_eq!(output.context, frozen);
}

#[test]
fn owner_source_deletion_preserves_local_objects_and_retires_only_removed_sources() {
    for kind in [MirroredItemKind::Matrix, MirroredItemKind::EmbeddedCircuit] {
        for state in [
            TransitionState::Linked,
            TransitionState::Local,
            TransitionState::Omitted,
        ] {
            let (before, mut context) = circuit_lifecycle_fixture();
            for association in context.associations.iter_mut().filter(|a| {
                a.kind == MirroredItemKind::Part
                    && a.source_id.starts_with("matrix/")
                    && a.parent_source_id.is_none()
            }) {
                association.parent_source_id = Some("source-matrix".into());
                association.parent_mirror_id = Some("mirror-matrix".into());
            }
            let (source_id, mirror_id) = if kind == MirroredItemKind::Matrix {
                ("source-matrix", "mirror-matrix")
            } else {
                ("source-circuit", "mirror-circuit")
            };
            let pending =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(&context, kind, source_id, mirror_id, state)
                    .unwrap();
            let committed = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &before,
                &before,
                &pending,
                crate::support::test_dispatch(),
            )
            .unwrap();
            let retained_parts: Vec<_> = if state == TransitionState::Local {
                committed
                    .context
                    .associations
                    .iter()
                    .filter(|a| {
                        a.kind == MirroredItemKind::Part && a.state == TransitionState::Local
                    })
                    .filter_map(|a| {
                        committed
                            .draft
                            .parts
                            .iter()
                            .find(|p| p.id == a.mirror_id)
                            .cloned()
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let untouched: Vec<_> = committed
                .context
                .pair_right
                .correspondences
                .iter()
                .filter(|r| {
                    if kind == MirroredItemKind::Matrix {
                        !r.source_id.starts_with("matrix/source-matrix/")
                            && r.source_id != source_id
                    } else {
                        !r.source_id.starts_with("embedded/source-circuit/")
                            && r.source_id != source_id
                    }
                })
                .cloned()
                .collect();
            let mut post = committed.draft.clone();
            if kind == MirroredItemKind::Matrix {
                crate::support::apply_for_test(
                    &mut post,
                    &[],
                    &EditOperation::RemoveMatrix {
                        id: source_id.into(),
                    },
                )
                .unwrap();
            } else {
                boardstudio_core_mechanical::modules::remove_circuit(&mut post, source_id).unwrap();
            }
            let removed = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &committed.draft,
                &post,
                &committed.context,
                crate::support::test_dispatch(),
            )
            .unwrap_or_else(|error| panic!("{kind:?} {state:?}: {error}"));
            assert_eq!(
                removed.context.pair_right.correspondences, untouched,
                "{kind:?} {state:?}"
            );
            assert!(removed.context.pair_right.exceptions.is_empty());
            for retained in &retained_parts {
                assert_eq!(
                    removed.draft.parts.iter().find(|p| p.id == retained.id),
                    Some(retained)
                );
            }
            assert_eq!(
                removed.draft.matrices.iter().any(|m| m.id == mirror_id)
                    || removed
                        .draft
                        .embedded_circuits
                        .iter()
                        .any(|c| c.id == mirror_id),
                state == TransitionState::Local
            );
            assert!(removed.draft.parts.iter().any(|p| p.id == "mirror-screen"));
            assert!(
                removed
                    .draft
                    .modules
                    .iter()
                    .any(|m| m.id == "mirror-module")
            );
            assert!(removed.draft.nets.iter().any(|n| n.id == "mirror-host-net"));
            boardstudio_core_mechanical::mechanical::halves::validate_document(&removed.draft)
                .unwrap();
            let repeated = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &removed.draft,
                &removed.draft,
                &removed.context,
                crate::support::test_dispatch(),
            )
            .unwrap();
            assert_eq!(repeated.draft, removed.draft);
            assert_eq!(repeated.context, removed.context);
            assert!(repeated.changed_document_ids.is_empty());
            let mut edited_source = repeated.draft.clone();
            edited_source
                .parts
                .iter_mut()
                .find(|p| p.id == "source-encoder")
                .unwrap()
                .properties = Some(BTreeMap::from([("revision".into(), json!(92))]));
            let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &repeated.draft,
                &edited_source,
                &repeated.context,
                crate::support::test_dispatch(),
            )
            .unwrap();
            assert_eq!(
                edited
                    .draft
                    .parts
                    .iter()
                    .find(|p| p.id == "mirror-encoder")
                    .unwrap()
                    .properties,
                edited_source
                    .parts
                    .iter()
                    .find(|p| p.id == "source-encoder")
                    .unwrap()
                    .properties
            );
        }
    }
}
