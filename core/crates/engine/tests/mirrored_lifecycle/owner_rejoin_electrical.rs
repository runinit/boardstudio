use super::*;
use crate::independent_exceptions::independent_lifecycle_fixture;
use crate::matrix_and_circuit_lifecycle::{add_lifecycle_association, circuit_lifecycle_fixture};

#[test]
fn matrix_owner_rejoin_projects_members_added_while_local() {
    check_growth(Growth::SourceOnly);
}

#[test]
fn cutover_rejoin_adopts_owned_overlap() {
    check_growth(Growth::OwnedOverlap);
}

#[test]
fn cutover_rejoin_refuses_unrelated_collision() {
    check_growth(Growth::UnrelatedCollision);
}

enum Growth {
    SourceOnly,
    OwnedOverlap,
    UnrelatedCollision,
}

fn check_growth(growth: Growth) {
    let (before, mut context) = independent_lifecycle_fixture();
    for a in context.associations.iter_mut().filter(|a| {
        a.kind == MirroredItemKind::Part
            && a.source_id.starts_with("matrix/")
            && a.parent_source_id.is_none()
    }) {
        a.parent_source_id = Some("source-matrix".into());
        a.parent_mirror_id = Some("mirror-matrix".into());
    }
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Local,
    )
    .unwrap();
    let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &pending,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let target = local.draft.matrices.by_id("mirror-matrix").clone();
    let mut post = local.draft.clone();
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
    boardstudio_core_kernel::matrix::set_matrix(&mut post, &source).unwrap();
    post.parts.by_id_mut("matrix/source-matrix/r0c4").properties =
        Some(BTreeMap::from([("new-source".into(), json!(true))]));
    let mut target = target;
    if matches!(growth, Growth::OwnedOverlap) {
        target.columns = 6;
        for column in [4, 5] {
            for row in 0..2 {
                let mut cell = target
                    .cells
                    .iter()
                    .find(|c| c.row == row && c.column == 0)
                    .unwrap()
                    .clone();
                cell.column = column;
                target.cells.push(cell);
            }
        }
        boardstudio_core_kernel::matrix::set_matrix(&mut post, &target).unwrap();
        target = post.matrices.by_id(&target.id).clone();
    }
    if matches!(growth, Growth::UnrelatedCollision) {
        let mut foreign = post.parts.by_id("matrix/mirror-matrix/r0c0").clone();
        foreign.id = "matrix/mirror-matrix/r0c4".into();
        foreign.reference = "FOREIGN".into();
        post.boards
            .by_id_mut("mirror-board")
            .part_ids
            .push(foreign.id.clone());
        post.parts.push(foreign);
    }
    let expanded = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &post,
        &local.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        expanded
            .draft
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix"),
        Some(&target)
    );
    let rejoin = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &expanded.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Linked,
    )
    .unwrap();
    let output = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &expanded.draft,
        &expanded.draft,
        &rejoin,
        crate::support::test_dispatch(),
    );
    if matches!(growth, Growth::UnrelatedCollision) {
        assert!(output.is_err(), "an unrelated Part cannot be adopted");
        return;
    }
    let output = output.expect(
        "an explicit owner Rejoin projects the current source, including new owned members",
    );
    assert!(
        !output
            .draft
            .parts
            .iter()
            .any(|p| p.id.starts_with("matrix/mirror-matrix/r0c5")
                || p.id.starts_with("matrix/mirror-matrix/r1c5"))
    );
    for row in 0..2 {
        for suffix in ["", "/diode"] {
            let source_id = format!("matrix/source-matrix/r{row}c4{suffix}");
            let mirror_id = format!("matrix/mirror-matrix/r{row}c4{suffix}");
            assert_eq!(
                output
                    .context
                    .pair_right
                    .correspondences
                    .iter()
                    .filter(|r| r.source_id == source_id && r.mirror_id == mirror_id)
                    .count(),
                1
            );
            assert_eq!(
                output.draft.parts.by_id(&mirror_id).properties,
                output.draft.parts.by_id(&source_id).properties
            );
        }
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&output.draft).unwrap();
    let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &output.draft,
        &output.draft,
        &output.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeat.draft, output.draft);
    assert_eq!(repeat.context, output.context);
    assert!(repeat.changed_document_ids.is_empty());
}

#[test]
fn child_exceptions_can_be_selected_while_the_matrix_owner_is_unlinked() {
    for (owner_state, child_state) in [
        (TransitionState::Local, TransitionState::Local),
        (TransitionState::Local, TransitionState::Omitted),
        (TransitionState::Omitted, TransitionState::Omitted),
    ] {
        let (before, mut context) = independent_lifecycle_fixture();
        for a in context.associations.iter_mut().filter(|a| {
            a.kind == MirroredItemKind::Part
                && a.source_id.starts_with("matrix/")
                && a.parent_source_id.is_none()
        }) {
            a.parent_source_id = Some("source-matrix".into());
            a.parent_mirror_id = Some("mirror-matrix".into());
        }
        let source_id = "matrix/source-matrix/r0c0";
        let mirror_id = "matrix/mirror-matrix/r0c0";
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            owner_state,
        )
        .unwrap();
        let owner = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &before,
            &before,
            &pending,
            crate::support::test_dispatch(),
        )
        .unwrap();
        let visible = owner
            .draft
            .parts
            .iter()
            .find(|p| p.id == mirror_id)
            .cloned();
        let selected = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &owner.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            child_state,
        )
        .expect(
            "a child selection authenticates its own durable row, separately from inherited state",
        );
        let child = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &owner.draft,
            &owner.draft,
            &selected,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(child.context.pair_right.exceptions.len(), 2);
        assert_eq!(
            child
                .draft
                .parts
                .iter()
                .find(|p| p.id == mirror_id)
                .cloned(),
            if child_state == TransitionState::Local {
                visible.clone()
            } else {
                None
            }
        );
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &child.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &child.draft,
            &child.draft,
            &pending,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(rejoined.context.pair_right.exceptions.len(), 1);
        assert_eq!(
            rejoined.context.pair_right.exceptions[0].mirror_id,
            mirror_id
        );
        assert_eq!(
            rejoined
                .draft
                .parts
                .iter()
                .find(|p| p.id == mirror_id)
                .cloned(),
            if child_state == TransitionState::Local {
                visible
            } else {
                None
            }
        );
        let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &rejoined.draft,
            &rejoined.draft,
            &rejoined.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(repeat.draft, rejoined.draft);
        assert_eq!(repeat.context, rejoined.context);
    }
}

#[test]
fn child_rejoin_clears_only_its_exception_and_inherits_the_current_owner() {
    for owner_state in [TransitionState::Local, TransitionState::Omitted] {
        let (before, mut context) = independent_lifecycle_fixture();
        for a in context.associations.iter_mut().filter(|a| {
            a.kind == MirroredItemKind::Part
                && a.source_id.starts_with("matrix/")
                && a.parent_source_id.is_none()
        }) {
            a.parent_source_id = Some("source-matrix".into());
            a.parent_mirror_id = Some("mirror-matrix".into());
        }
        let source_id = "matrix/source-matrix/r0c0";
        let mirror_id = "matrix/mirror-matrix/r0c0";
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Local,
        )
        .unwrap();
        let child = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &before,
            &before,
            &pending,
            crate::support::test_dispatch(),
        )
        .unwrap();
        let visible = child.draft.parts.by_id(mirror_id).clone();
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &child.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            owner_state,
        )
        .unwrap();
        let owner = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &child.draft,
            &child.draft,
            &pending,
            crate::support::test_dispatch(),
        )
        .unwrap();
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &owner.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Linked,
        )
        .unwrap();
        let inherited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &owner.draft,
            &owner.draft,
            &pending,
            crate::support::test_dispatch(),
        )
        .expect("clearing a child exception resumes inheritance even when the owner is unlinked");
        assert_eq!(inherited.context.pair_right.exceptions.len(), 1);
        assert_eq!(
            inherited.context.pair_right.exceptions[0].mirror_id,
            "mirror-matrix"
        );
        let association = inherited
            .context
            .associations
            .iter()
            .find(|a| a.source_id == source_id)
            .unwrap();
        assert_eq!(association.state, owner_state);
        assert!(!association.explicit_state);
        assert_eq!(
            inherited.draft.parts.iter().find(|p| p.id == mirror_id),
            if owner_state == TransitionState::Local {
                Some(&visible)
            } else {
                None
            }
        );
        let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &inherited.draft,
            &inherited.draft,
            &inherited.context,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert_eq!(repeat.draft, inherited.draft);
        assert_eq!(repeat.context, inherited.context);
        let mut post = inherited.draft.clone();
        post.parts.by_id_mut(source_id).properties =
            Some(BTreeMap::from([("current".into(), json!(true))]));
        let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
            &inherited.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
            &inherited.draft,
            &post,
            &pending,
            crate::support::test_dispatch(),
        )
        .unwrap();
        assert!(rejoined.context.pair_right.exceptions.is_empty());
        assert_eq!(
            rejoined.draft.parts.by_id(mirror_id).properties,
            post.parts.by_id(source_id).properties
        );
    }
}

#[test]
fn independent_connector_exceptions_preserve_linked_modules_and_clean_only_their_reference() {
    let (before, context) = circuit_lifecycle_fixture();
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
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
        &pending,
        crate::support::test_dispatch(),
    )
    .expect("a Local connector remains a valid reference for a linked module");
    let connector = local.draft.parts.by_id("mirror-screen").clone();
    let module = local.draft.modules.by_id("mirror-module").clone();
    let mut post = local.draft.clone();
    post.parts.by_id_mut("source-screen").pose.at.x += 30.0;
    post.modules.by_id_mut("source-module").gap += 1.0;
    let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &post,
        &local.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == "mirror-screen"),
        Some(&connector)
    );
    let edited_module = edited.draft.modules.by_id("mirror-module");
    assert_eq!(edited_module.gap, module.gap + 1.0);
    assert_eq!(
        edited_module
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        "mirror-screen"
    );
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
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
        &pending,
        crate::support::test_dispatch(),
    )
    .expect("Omit clears the selected connector reference without removing its module");
    let mut expected = edited_module.clone();
    expected.connection = None;
    assert_eq!(
        omitted
            .draft
            .modules
            .iter()
            .find(|m| m.id == "mirror-module"),
        Some(&expected)
    );
    let repeat = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &omitted.draft,
        &omitted.draft,
        &omitted.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeat.draft, omitted.draft);
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &omitted.draft,
        &omitted.draft,
        &pending,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(
        rejoined
            .draft
            .modules
            .iter()
            .find(|m| m.id == "mirror-module"),
        Some(edited_module)
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();
}

pub(crate) fn electrical_definition(id: &str, pads: &[(&str, f64, f64)]) -> PartDefinition {
    serde_json::from_value(json!({
        "id": id, "name": id, "kind": "custom", "courtyard": [],
        "pads": pads.iter().map(|(pad, x, y)| json!({
            "id": pad, "number": pad, "at": {"x": x, "y": y},
            "size": {"x": 1, "y": 1}, "shape": "circle"
        })).collect::<Vec<_>>()
    }))
    .unwrap()
}

pub(crate) fn pad_world(document: &ProjectDoc, part_id: &str, pad_id: &str) -> Vec2 {
    let part = document.parts.by_id(part_id);
    let definition = document.definitions.by_id(&part.definition_id);
    let pad = definition.pads.iter().find(|p| p.id == pad_id).unwrap();
    // Ticket 12: pad world positions evaluate half-frame parts at board.
    let pose = boardstudio_core_kernel::matrix::layout::evaluated_part_pose(document, part);
    boardstudio_core_kernel::outline_controls::world(pad.at, &pose, part.side.clone())
}

pub(crate) fn mirror_electrical(document: &ProjectDoc) -> &ElectricalBoardConfiguration {
    document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|b| b.board_id == "mirror-board")
        .unwrap()
}

pub(crate) fn source_electrical(document: &ProjectDoc) -> &ElectricalBoardConfiguration {
    document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|b| b.board_id == "source-board")
        .unwrap()
}

/// A screen on SDA/SCL (GP2/GP3), an encoder on GP4, an MCU, two private nets, a shared ground,
/// and routed mirror copper. The mirror parts sit exactly where the source mapping puts them, so a
/// Rejoin of an unmoved source lands every pad where its saved copper expects.
pub(crate) fn electrical_copper_fixture() -> (ProjectDoc, MembershipContext) {
    let (mut document, mut context) = independent_lifecycle_fixture();
    document.definitions.push(electrical_definition(
        "screen-def",
        &[("sda", -2.0, 0.0), ("scl", 2.0, 0.0), ("gnd", 0.0, 3.0)],
    ));
    document.definitions.push(electrical_definition(
        "encoder-def",
        &[("a", -1.0, 0.0), ("gnd", 1.0, 0.0)],
    ));
    document.definitions.push(electrical_definition(
        "mcu-def",
        &[("gp2", 0.0, 0.0), ("gp3", 3.0, 0.0), ("gnd", 6.0, 0.0)],
    ));
    for (name, definition) in [("screen", "screen-def"), ("encoder", "encoder-def")] {
        for half in ["source", "mirror"] {
            document
                .parts
                .iter_mut()
                .find(|p| p.id == format!("{half}-{name}"))
                .unwrap()
                .definition_id = definition.into();
        }
    }
    for (half, board_id, x) in [
        ("source", "source-board", 30.0),
        ("mirror", "mirror-board", 70.0),
    ] {
        let mut part = document.parts[0].clone();
        part.id = format!("{half}-mcu");
        part.reference = format!("{half}_mcu");
        part.definition_id = "mcu-def".into();
        part.pose.at = Vec2 { x, y: 10.0 };
        part.properties = Some(BTreeMap::from([("revision".into(), json!(1))]));
        document.parts.push(part);
        document
            .boards
            .by_id_mut(board_id)
            .part_ids
            .push(format!("{half}-mcu"));
    }
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        None,
    );
    // The mirror parts follow the pair map, so their pads are where a Rejoin will put them.
    for name in ["screen", "encoder", "mcu"] {
        let source = document
            .parts
            .iter()
            .find(|p| p.id == format!("source-{name}"))
            .unwrap()
            .clone();
        let target = document
            .parts
            .iter_mut()
            .find(|p| p.id == format!("mirror-{name}"))
            .unwrap();
        *target = boardstudio_core_mirrored_half::mirrored_half::derive::part(
            &source,
            &target.id,
            &target.definition_id,
            &target.reference,
            context.mapping,
            boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
                &document.definitions,
                &source.definition_id,
            ),
        );
    }
    let nets: [(&str, &[(&str, &str)]); 3] = [
        ("sda", &[("mcu", "gp2"), ("screen", "sda")]),
        ("scl", &[("mcu", "gp3"), ("screen", "scl")]),
        (
            "gnd",
            &[("screen", "gnd"), ("encoder", "gnd"), ("mcu", "gnd")],
        ),
    ];
    for (name, pins) in nets {
        for (half, board_id) in [("source", "source-board"), ("mirror", "mirror-board")] {
            let id = format!("{half}-{name}");
            document.nets.push(Net {
                id: id.clone(),
                name: name.to_uppercase(),
                pins: pins
                    .iter()
                    .map(|(part, pad)| Pin {
                        part_id: format!("{half}-{part}"),
                        pad_id: (*pad).into(),
                    })
                    .collect(),
            });
            document.boards.by_id_mut(board_id).net_ids.push(id);
        }
        add_lifecycle_association(
            &mut document,
            &mut context,
            MirroredItemKind::Net,
            &format!("source-{name}"),
            &format!("mirror-{name}"),
            None,
        );
    }
    let assignments = |half: &str| {
        BTreeMap::from([
            (format!("i2c/{half}-screen/SDA"), "GP2".to_owned()),
            (format!("i2c/{half}-screen/SCL"), "GP3".to_owned()),
            (format!("direct/{half}-encoder"), "GP4".to_owned()),
        ])
    };
    for (half, board_id) in [("source", "source-board"), ("mirror", "mirror-board")] {
        document
            .hardware
            .as_mut()
            .unwrap()
            .boards
            .push(ElectricalBoardConfiguration {
                board_id: board_id.into(),
                controller_part_id: Some(format!("{half}-mcu")),
                assignments: assignments(half),
                ..ElectricalBoardConfiguration::default()
            });
    }
    add_lifecycle_association(
        &mut document,
        &mut context,
        MirroredItemKind::ElectricalBoard,
        "source-board",
        "mirror-board",
        None,
    );
    let trace = |id: &str, net: Option<&str>, from: Vec2, to: Vec2| CopperTrace {
        id: id.into(),
        start: from,
        end: to,
        width: 0.25,
        layer: Side::Front,
        net_id: net.map(str::to_owned),
    };
    let at = |part: &str, pad: &str| pad_world(&document, part, pad);
    let copper = (
        vec![
            trace(
                "t-sda",
                Some("mirror-sda"),
                at("mirror-mcu", "gp2"),
                at("mirror-screen", "sda"),
            ),
            trace(
                "t-gnd-screen",
                Some("mirror-gnd"),
                at("mirror-screen", "gnd"),
                at("mirror-encoder", "gnd"),
            ),
            trace(
                "t-gnd-other",
                Some("mirror-gnd"),
                at("mirror-encoder", "gnd"),
                at("mirror-mcu", "gnd"),
            ),
        ],
        vec![
            CopperVia {
                id: "v-sda".into(),
                at: Vec2 { x: 1.0, y: 1.0 },
                size: 0.8,
                drill: 0.4,
                net_id: Some("mirror-sda".into()),
            },
            CopperVia {
                id: "v-gnd".into(),
                at: at("mirror-screen", "gnd"),
                size: 0.8,
                drill: 0.4,
                net_id: Some("mirror-gnd".into()),
            },
        ],
    );
    let board = document.boards.by_id_mut("mirror-board");
    (board.traces, board.vias) = copper;
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    (document, context)
}

pub(crate) fn omit_screen(
    before: &ProjectDoc,
    context: &MembershipContext,
) -> boardstudio_core_mirrored_half::mirrored_half::reconcile::MembershipTransitionOutput {
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    )
    .unwrap();
    boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        before,
        before,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect("Omit commits")
}

pub(crate) fn rejoin_screen(
    draft: &ProjectDoc,
    post: &ProjectDoc,
    context: &MembershipContext,
) -> boardstudio_core_mirrored_half::mirrored_half::reconcile::MembershipTransitionOutput {
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        draft,
        post,
        &pending,
        crate::support::test_dispatch(),
    )
    .expect("Rejoin commits")
}

pub(crate) fn pin_values(config: &ElectricalBoardConfiguration) -> BTreeSet<String> {
    config
        .assignments
        .values()
        .chain(config.locks.values())
        .cloned()
        .collect()
}

#[test]
fn local_controller_own_pin_keeps_a_private_net_through_omit_and_reopen() {
    let (before, context) = electrical_copper_fixture();
    let local_controller = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let localized = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &local_controller,
        crate::support::test_dispatch(),
    )
    .expect("the controller can become Local");
    assert!(localized.context.associations.iter().any(|association| {
        association.kind == MirroredItemKind::Part
            && association.mirror_id == "mirror-mcu"
            && association.state == TransitionState::Local
    }));

    let omitted = omit_screen(&localized.draft, &localized.context);
    let retained_net = omitted
        .draft
        .nets
        .iter()
        .find(|net| net.id == "mirror-sda")
        .expect("the Local controller's own pin keeps its net");
    assert_eq!(
        retained_net.pins,
        [Pin {
            part_id: "mirror-mcu".into(),
            pad_id: "gp2".into(),
        }]
    );
    let mirror_board = omitted.draft.boards.by_id("mirror-board");
    assert!(mirror_board.net_ids.contains(&"mirror-sda".into()));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&omitted.draft).unwrap();

    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted.draft).unwrap()).unwrap();
    let repeated = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &decoded,
        &decoded,
        &omitted.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    assert_eq!(repeated.draft, decoded);
    let mut reopened = boardstudio_core_engine::CoreEngine::new();
    assert!(matches!(
        reopened.handle(boardstudio_model::model::CoreRequest::Open {
            id: "reopen-local-controller-net".into(),
            document: repeated.draft,
        }),
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    assert!(
        reopened
            .document_for_test()
            .nets
            .iter()
            .any(|net| net.id == "mirror-sda")
    );
    assert!(
        reopened
            .document_for_test()
            .boards
            .by_id("mirror-board")
            .net_ids
            .contains(&"mirror-sda".into())
    );
}

#[test]
fn malformed_local_controller_ownership_refuses_omit_atomically() {
    let (before, context) = electrical_copper_fixture();
    let local_controller = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let localized = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &local_controller,
        crate::support::test_dispatch(),
    )
    .expect("the controller can become Local");
    let mut malformed = localized.context.clone();
    let controller = malformed
        .associations
        .iter()
        .find(|association| {
            association.kind == MirroredItemKind::Part && association.mirror_id == "mirror-mcu"
        })
        .unwrap()
        .clone();
    malformed.associations.push(controller);
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &malformed,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    );
    let error = match pending {
        Ok(pending) => {
            let frozen = (localized.draft.clone(), malformed.clone(), pending.clone());
            let result = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
                &localized.draft,
                &localized.draft,
                &pending,
                crate::support::test_dispatch(),
            );
            let Err(error) = result else {
                panic!("duplicate controller ownership must refuse Omit");
            };
            assert_eq!(frozen, (localized.draft, malformed, pending));
            error
        }
        Err(error) => error,
    };
    assert!(!error.is_empty());
}

#[test]
fn omit_frees_the_items_pins_and_locks_every_mirrored_item_to_the_source() {
    let (before, context) = electrical_copper_fixture();
    let omitted = omit_screen(&before, &context);
    let mirror = mirror_electrical(&omitted.draft);
    assert!(
        mirror
            .assignments
            .keys()
            .chain(mirror.locks.keys())
            .all(|key| !key.contains("mirror-screen")),
        "the omitted screen's functions are gone: {mirror:?}"
    );
    let held = pin_values(mirror);
    assert!(
        !held.contains("GP2") && !held.contains("GP3"),
        "its pins are free: {held:?}"
    );
    assert_eq!(
        mirror
            .assignments
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP4")
    );
    assert_eq!(
        mirror
            .locks
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP4"),
        "an item that is still mirrored stays locked to the source's pin"
    );
    assert_eq!(mirror.controller_part_id.as_deref(), Some("mirror-mcu"));
    assert_eq!(
        source_electrical(&omitted.draft),
        source_electrical(&before),
        "the source half never changes"
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&omitted.draft).unwrap();

    let rejoined = rejoin_screen(&omitted.draft, &omitted.draft, &omitted.context);
    let mirror = mirror_electrical(&rejoined.draft);
    assert_eq!(
        mirror
            .assignments
            .get("i2c/mirror-screen/SDA")
            .map(String::as_str),
        Some("GP2")
    );
    assert_eq!(
        mirror
            .assignments
            .get("i2c/mirror-screen/SCL")
            .map(String::as_str),
        Some("GP3")
    );
    assert_eq!(
        mirror
            .locks
            .get("i2c/mirror-screen/SDA")
            .map(String::as_str),
        Some("GP2")
    );
    assert_eq!(
        source_electrical(&rejoined.draft),
        source_electrical(&before)
    );
}

#[test]
fn omit_removes_the_footprint_private_nets_and_only_the_copper_it_owned() {
    let (before, context) = electrical_copper_fixture();
    let omitted = omit_screen(&before, &context);
    let draft = &omitted.draft;
    assert!(draft.parts.iter().all(|p| p.id != "mirror-screen"));
    for net in ["mirror-sda", "mirror-scl"] {
        assert!(
            draft.nets.iter().all(|n| n.id != net),
            "{net} is private to the screen"
        );
        assert!(
            draft
                .boards
                .iter()
                .all(|b| !b.net_ids.contains(&net.to_owned()))
        );
    }
    let ground = draft
        .nets
        .iter()
        .find(|n| n.id == "mirror-gnd")
        .expect("a shared net stays");
    assert!(
        ground.pins.iter().all(|pin| pin.part_id != "mirror-screen"),
        "it loses only the omitted pin"
    );
    assert_eq!(ground.pins.len(), 2);
    let board = draft.boards.by_id("mirror-board");
    assert_eq!(
        board
            .traces
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec!["t-gnd-other"],
        "the private-net trace and the trace ending on the omitted pad are gone, the unrelated one stays"
    );
    assert!(board.vias.is_empty());
    let overlay = omitted
        .context
        .pair_right
        .exceptions
        .iter()
        .find(|e| e.mirror_id == "mirror-screen")
        .unwrap();
    let saved = overlay
        .removed_copper
        .as_ref()
        .expect("the copper is kept with its overlay");
    let ids: BTreeSet<_> = saved.traces.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, BTreeSet::from(["t-sda", "t-gnd-screen"]));
    let via_ids: BTreeSet<_> = saved.vias.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(via_ids, BTreeSet::from(["v-sda", "v-gnd"]));
    assert_eq!(saved.pads.len(), 3);
    // The source half keeps every net, pin and trace it had.
    let source_board = |d: &ProjectDoc| d.boards.by_id("source-board").clone();
    assert_eq!(source_board(draft), source_board(&before));
    for net in ["source-sda", "source-scl", "source-gnd"] {
        assert_eq!(
            draft.nets.iter().find(|n| n.id == net),
            before.nets.iter().find(|n| n.id == net)
        );
    }
    let decoded: ProjectDoc = serde_json::from_slice(&serde_json::to_vec(draft).unwrap()).unwrap();
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
}

#[test]
fn rejoin_restores_saved_copper_only_when_the_pads_are_where_they_were() {
    let (before, context) = electrical_copper_fixture();
    let omitted = omit_screen(&before, &context);
    let rejoined = rejoin_screen(&omitted.draft, &omitted.draft, &omitted.context);
    assert!(rejoined.notices.is_empty(), "{:?}", rejoined.notices);
    assert!(rejoined.context.pair_right.exceptions.is_empty());
    let board = |d: &ProjectDoc| d.boards.by_id("mirror-board").clone();
    let (original, restored) = (board(&before), board(&rejoined.draft));
    let by_id = |mut traces: Vec<CopperTrace>| {
        traces.sort_by(|a, b| a.id.cmp(&b.id));
        traces
    };
    assert_eq!(
        by_id(restored.traces),
        by_id(original.traces),
        "the exact traces come back"
    );
    let mut vias = restored.vias.clone();
    vias.sort_by(|a, b| a.id.cmp(&b.id));
    let mut expected = original.vias.clone();
    expected.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(vias, expected);
    for net in ["mirror-sda", "mirror-scl"] {
        let restored = rejoined.draft.nets.by_id(net);
        let original = before.nets.by_id(net);
        let pins = |n: &Net| {
            n.pins
                .iter()
                .cloned()
                .map(|p| (p.part_id, p.pad_id))
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(pins(restored), pins(original));
        assert_eq!(restored.name, original.name);
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined.draft).unwrap();

    // The source screen moved while the mirror was omitted: its pads no longer match.
    let mut post = omitted.draft.clone();
    post.parts.by_id_mut("source-screen").pose.at.x += 4.0;
    let moved = rejoin_screen(&omitted.draft, &post, &omitted.context);
    let board = moved.draft.boards.by_id("mirror-board");
    assert_eq!(
        board
            .traces
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec!["t-gnd-other"]
    );
    assert!(board.vias.is_empty());
    assert_eq!(moved.notices.len(), 1, "{:?}", moved.notices);
    assert!(moved.notices[0].contains("mirror-screen") && moved.notices[0].contains("re-route"));
    assert!(
        moved.context.pair_right.exceptions.is_empty(),
        "the saved copper is dropped with the overlay"
    );
    assert!(
        moved.draft.nets.iter().any(|n| n.id == "mirror-sda"),
        "the net itself is restored"
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&moved.draft).unwrap();
}

#[test]
fn rejoin_reclaims_its_pins_and_releases_a_mirror_only_assignment_that_took_one() {
    let (before, context) = electrical_copper_fixture();
    let omitted = omit_screen(&before, &context);
    let mut replaced = omitted.draft.clone();
    {
        let config = replaced
            .hardware
            .as_mut()
            .unwrap()
            .boards
            .iter_mut()
            .find(|b| b.board_id == "mirror-board")
            .unwrap();
        // A mirror-only replacement took the freed GP2 and, separately, an unused GP7.
        for map in [&mut config.assignments, &mut config.locks] {
            map.insert("replacement/mirror-oled/SDA".into(), "GP2".into());
            map.insert("replacement/mirror-oled/RST".into(), "GP7".into());
        }
    }
    let stable = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &replaced,
        &replaced,
        &omitted.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let held = mirror_electrical(&stable.draft);
    assert_eq!(
        held.assignments
            .get("replacement/mirror-oled/SDA")
            .map(String::as_str),
        Some("GP2"),
        "a freed pin may be used by an entry the mirror authored"
    );
    let rejoined = rejoin_screen(&stable.draft, &stable.draft, &stable.context);
    let config = mirror_electrical(&rejoined.draft);
    assert_eq!(
        config
            .assignments
            .get("i2c/mirror-screen/SDA")
            .map(String::as_str),
        Some("GP2")
    );
    for map in [&config.assignments, &config.locks] {
        assert!(
            !map.contains_key("replacement/mirror-oled/SDA"),
            "the conflicting entry is released"
        );
        assert_eq!(
            map.get("replacement/mirror-oled/RST").map(String::as_str),
            Some("GP7"),
            "an unrelated mirror-only entry is kept"
        );
    }
}

#[test]
fn a_local_controller_keeps_its_mapped_locks_and_assignments() {
    let (before, context) = electrical_copper_fixture();
    let pending = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let local = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &before,
        &before,
        &pending,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let config = mirror_electrical(&local.draft);
    assert_eq!(config.controller_part_id.as_deref(), Some("mirror-mcu"));
    assert_eq!(
        config
            .assignments
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP4")
    );
    assert_eq!(
        config
            .locks
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP4")
    );
    // A later source edit still reaches the mirror's assignments while the controller stays Local.
    let mut post = local.draft.clone();
    post.hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|b| b.board_id == "source-board")
        .unwrap()
        .assignments
        .insert("direct/source-encoder".into(), "GP5".into());
    let edited = boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::prepare_lifecycle_transition(
        &local.draft,
        &post,
        &local.context,
        crate::support::test_dispatch(),
    )
    .unwrap();
    let config = mirror_electrical(&edited.draft);
    assert_eq!(config.controller_part_id.as_deref(), Some("mirror-mcu"));
    assert_eq!(
        config
            .assignments
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP5")
    );
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == "mirror-mcu"),
        local.draft.parts.iter().find(|p| p.id == "mirror-mcu"),
        "the Local controller itself is untouched"
    );
}
