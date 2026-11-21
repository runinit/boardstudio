use super::*;
use crate::independent_exceptions::independent_lifecycle_fixture;

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
pub(crate) fn electrical_copper_fixture() -> ProjectDoc {
    let mut document = independent_lifecycle_fixture();
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
    add_link(
        &mut document,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
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
            document.hardware.as_ref().unwrap().pairs[0].mapping,
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
        add_link(
            &mut document,
            MirroredItemKind::Net,
            &format!("source-{name}"),
            &format!("mirror-{name}"),
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
    add_link(
        &mut document,
        MirroredItemKind::ElectricalBoard,
        "source-board",
        "mirror-board",
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
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    document
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
    let before = electrical_copper_fixture();
    let localized = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-mcu",
        HalfExceptionState::Local,
    )
    .unwrap();
    assert!(
        localized
            .first_halves()
            .exceptions
            .iter()
            .any(|association| {
                association.kind == MirroredItemKind::Part
                    && association.mirror_id == "mirror-mcu"
                    && association.state == MirroredExceptionState::Local
            })
    );

    let omitted = exception(
        &localized,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Omitted,
    )
    .unwrap();
    let retained_net = omitted
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
    let mirror_board = omitted.boards.by_id("mirror-board");
    assert!(mirror_board.net_ids.contains(&"mirror-sda".into()));
    boardstudio_core_mechanical::mechanical::halves::validate_document(&omitted).unwrap();

    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted).unwrap()).unwrap();
    let repeated = support::prepare_open(&decoded, &[]).unwrap().0;
    assert_eq!(repeated, decoded);
    let mut reopened = boardstudio_core_engine::CoreEngine::new();
    assert!(matches!(
        reopened.handle(boardstudio_model::model::CoreRequest::Open {
            id: "reopen-local-controller-net".into(),
            document: repeated,
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
fn omit_frees_the_items_pins_and_locks_every_mirrored_item_to_the_source() {
    let before = electrical_copper_fixture();
    let omitted = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Omitted,
    )
    .unwrap();
    let mirror = mirror_electrical(&omitted);
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
        source_electrical(&omitted),
        source_electrical(&before),
        "the source half never changes"
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&omitted).unwrap();

    let rejoined = exception(
        &omitted,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Linked,
    )
    .unwrap();
    let mirror = mirror_electrical(&rejoined);
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
    assert_eq!(source_electrical(&rejoined), source_electrical(&before));
}

#[test]
fn omit_removes_the_footprint_private_nets_and_only_the_copper_it_owned() {
    let before = electrical_copper_fixture();
    let omitted = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Omitted,
    )
    .unwrap();
    let draft = &omitted;
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
        .first_halves()
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
    let repeat = support::prepare_open(&decoded, &[]).unwrap().0;
    assert_eq!(repeat, decoded);
}

#[test]
fn rejoin_restores_saved_copper_only_when_the_pads_are_where_they_were() {
    let before = electrical_copper_fixture();
    let omitted = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Omitted,
    )
    .unwrap();
    let rejoined = exception(
        &omitted,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Linked,
    )
    .unwrap();
    assert!(rejoined.first_halves().exceptions.is_empty());
    let board = |d: &ProjectDoc| d.boards.by_id("mirror-board").clone();
    let (original, restored) = (board(&before), board(&rejoined));
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
        let restored = rejoined.nets.by_id(net);
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
    boardstudio_core_mechanical::mechanical::halves::validate_document(&rejoined).unwrap();

    // The source screen moved while the mirror was omitted: its pads no longer match.
    let mut post = omitted.clone();
    post.parts.by_id_mut("source-screen").pose.at.x += 4.0;
    let moved = exception(
        &post,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Linked,
    )
    .unwrap();
    let board = moved.boards.by_id("mirror-board");
    assert_eq!(
        board
            .traces
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec!["t-gnd-other"]
    );
    assert!(board.vias.is_empty());
    assert!(
        moved.first_halves().exceptions.is_empty(),
        "the saved copper is dropped with the overlay"
    );
    assert!(
        moved.nets.iter().any(|n| n.id == "mirror-sda"),
        "the net itself is restored"
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&moved).unwrap();
}

#[test]
fn a_local_controller_keeps_its_mapped_locks_and_assignments() {
    let before = electrical_copper_fixture();
    let local = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-mcu",
        HalfExceptionState::Local,
    )
    .unwrap();
    let config = mirror_electrical(&local);
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
    let mut post = local.clone();
    post.hardware
        .as_mut()
        .unwrap()
        .boards
        .iter_mut()
        .find(|b| b.board_id == "source-board")
        .unwrap()
        .assignments
        .insert("direct/source-encoder".into(), "GP5".into());
    let edited = support::prepare_open(&post, &[]).unwrap().0;
    let config = mirror_electrical(&edited);
    assert_eq!(config.controller_part_id.as_deref(), Some("mirror-mcu"));
    assert_eq!(
        config
            .assignments
            .get("direct/mirror-encoder")
            .map(String::as_str),
        Some("GP5")
    );
    assert_eq!(
        edited.parts.iter().find(|p| p.id == "mirror-mcu"),
        local.parts.iter().find(|p| p.id == "mirror-mcu"),
        "the Local controller itself is untouched"
    );
}

#[test]
fn saved_copper_that_collides_with_live_copper_refuses_the_rejoin_atomically() {
    let before = electrical_copper_fixture();
    let mut omitted = exception(
        &before,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Omitted,
    )
    .unwrap();
    omitted.boards.by_id_mut("mirror-board").traces.push(
        before
            .boards
            .by_id("mirror-board")
            .traces
            .iter()
            .find(|t| t.id == "t-sda")
            .unwrap()
            .clone(),
    );
    let frozen = omitted.clone();
    let error = exception(
        &omitted,
        MirroredItemKind::Part,
        "mirror-screen",
        HalfExceptionState::Linked,
    )
    .unwrap_err();
    assert!(error.contains("t-sda"), "{error}");
    assert_eq!(omitted, frozen);
}

const MATRIX: &str = "mirror-matrix";
const CHILD: &str = "matrix/mirror-matrix/r0c0";
const SOURCE_CHILD: &str = "matrix/source-matrix/r0c0";

#[test]
fn child_exceptions_can_be_selected_while_the_matrix_owner_is_unlinked() {
    use HalfExceptionState::{Linked, Local, Omitted};
    use MirroredItemKind::{Matrix, Part};

    for (owner_state, child_state) in [(Local, Local), (Local, Omitted), (Omitted, Omitted)] {
        let start = independent_lifecycle_fixture();
        let mut owner = exception(&start, Matrix, MATRIX, owner_state).unwrap();
        if child_state == Local {
            owner.parts.by_id_mut(CHILD).properties =
                Some(BTreeMap::from([("mirror-authored".into(), json!(true))]));
        }
        let payload = owner
            .parts
            .iter()
            .find(|p| p.id == CHILD && child_state == Local);
        let child = exception(&owner, Part, CHILD, child_state)
            .unwrap_or_else(|e| panic!("{owner_state:?} owner, {child_state:?} child: {e}"));
        let overlays = &child.first_halves().exceptions;
        assert_eq!(overlays.len(), 2);
        assert!(overlays.contains(&owner.first_halves().exceptions[0]));
        let explicit = overlays
            .iter()
            .find(|e| e.kind == Part && e.mirror_id == CHILD)
            .unwrap();
        assert_eq!(
            explicit.state,
            if child_state == Local {
                MirroredExceptionState::Local
            } else {
                MirroredExceptionState::Omitted
            }
        );
        assert_eq!(child.parts.iter().find(|p| p.id == CHILD), payload);
        let diode = "matrix/mirror-matrix/r0c0/diode";
        assert_eq!(
            child.parts.iter().any(|p| p.id == diode),
            child_state == Local
        );

        let rejoined = exception(&child, Matrix, MATRIX, Linked).unwrap();
        assert_eq!(
            rejoined.first_halves().exceptions,
            std::slice::from_ref(explicit)
        );
        assert!(rejoined.matrices.iter().any(|m| m.id == MATRIX));
        assert_eq!(rejoined.parts.iter().find(|p| p.id == CHILD), payload);
        assert_eq!(
            rejoined.parts.iter().any(|p| p.id == diode),
            child_state == Local
        );
        assert_eq!(
            exception(&rejoined, Matrix, MATRIX, Linked).unwrap(),
            rejoined
        );

        // Rejoining only the child while its matrix owner remains unlinked clears
        // the child exception and inherits the current Local or Omitted owner.
        let child_rejoined = exception(&child, Part, CHILD, Linked).unwrap_or_else(|e| {
            panic!("child rejoin under {owner_state:?} owner ({child_state:?} child): {e}")
        });
        assert_eq!(
            child_rejoined.first_halves().exceptions,
            std::slice::from_ref(&owner.first_halves().exceptions[0])
        );
        assert_eq!(
            child_rejoined.matrices.iter().any(|m| m.id == MATRIX),
            owner_state == Local
        );
        assert_eq!(
            child_rejoined.parts.iter().find(|p| p.id == CHILD),
            (owner_state == Local).then(|| owner.parts.by_id(CHILD))
        );
        assert_eq!(
            child_rejoined.parts.iter().find(|p| p.id == diode),
            (owner_state == Local).then(|| owner.parts.by_id(diode))
        );
        assert_eq!(
            exception(&child_rejoined, Part, CHILD, Linked).unwrap(),
            child_rejoined
        );
    }
}

#[test]
fn omitting_a_key_under_a_local_matrix_round_trips_through_undo_and_redo() {
    use HalfExceptionState::{Local, Omitted};
    use MirroredItemKind::{Matrix, Part};

    let start = independent_lifecycle_fixture();
    let local_owner = exception(&start, Matrix, MATRIX, Local).unwrap();
    let pair_id = local_owner.hardware.as_ref().unwrap().pairs[0].id.clone();
    let mut engine = boardstudio_core_engine::CoreEngine::new();
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "open-local-matrix".into(),
            document: local_owner,
        }),
        CoreReply::Scene { .. }
    ));

    let before_omit = engine.document_for_test().clone();
    let diode = "matrix/mirror-matrix/r0c0/diode";
    assert!(before_omit.parts.iter().any(|p| p.id == CHILD));
    assert!(before_omit.parts.iter().any(|p| p.id == diode));

    let operation = EditOperation::SetHalfException {
        pair_id,
        kind: Part,
        mirror_id: CHILD.into(),
        state: Omitted,
    };
    let (expected_omitted, _) =
        support::prepare_transaction(&before_omit, &[], &operation).unwrap();

    let commit = CoreRequest::Edit {
        id: "omit-key".into(),
        command: EditCommand {
            base_revision: before_omit.revision,
            transaction_id: "omit-key".into(),
            phase: EditPhase::Commit,
            target_ids: vec![CHILD.into()],
            operation,
        },
    };
    assert!(matches!(engine.handle(commit), CoreReply::Scene { .. }));
    let committed = engine.document_for_test().clone();
    assert_eq!(committed.parts, expected_omitted.parts);
    assert_eq!(committed.first_halves(), expected_omitted.first_halves());
    assert!(!committed.parts.iter().any(|p| p.id == CHILD));
    assert!(!committed.parts.iter().any(|p| p.id == diode));
    assert_eq!(committed.first_halves().exceptions.len(), 2);

    assert!(matches!(
        engine.handle(CoreRequest::Undo {
            id: "undo-omit-key".into(),
        }),
        CoreReply::Scene { .. }
    ));
    let undone = engine.document_for_test().clone();
    assert_eq!(undone.parts, before_omit.parts);
    assert_eq!(undone.first_halves(), before_omit.first_halves());
    assert!(undone.parts.iter().any(|p| p.id == CHILD));
    assert!(undone.parts.iter().any(|p| p.id == diode));
    assert_eq!(undone.first_halves().exceptions.len(), 1);

    assert!(matches!(
        engine.handle(CoreRequest::Redo {
            id: "redo-omit-key".into(),
        }),
        CoreReply::Scene { .. }
    ));
    let redone = engine.document_for_test().clone();
    assert_eq!(redone.parts, committed.parts);
    assert_eq!(redone.first_halves(), committed.first_halves());
    assert!(!redone.parts.iter().any(|p| p.id == CHILD));
    assert!(!redone.parts.iter().any(|p| p.id == diode));
    assert_eq!(redone.first_halves().exceptions.len(), 2);

    // Omitting and rejoining only the companion diode under a Local matrix preserves the key
    // and restores the diode matching its pre-omit definition and pose.
    let omit_diode = EditOperation::SetHalfException {
        pair_id: undone.hardware.as_ref().unwrap().pairs[0].id.clone(),
        kind: Part,
        mirror_id: diode.into(),
        state: Omitted,
    };
    let (diode_omitted, _) = support::prepare_transaction(&before_omit, &[], &omit_diode).unwrap();
    assert!(diode_omitted.parts.iter().any(|p| p.id == CHILD));
    assert!(!diode_omitted.parts.iter().any(|p| p.id == diode));

    let rejoin_diode = EditOperation::SetHalfException {
        pair_id: undone.hardware.as_ref().unwrap().pairs[0].id.clone(),
        kind: Part,
        mirror_id: diode.into(),
        state: HalfExceptionState::Linked,
    };
    let (diode_rejoined, _) =
        support::prepare_transaction(&diode_omitted, &[], &rejoin_diode).unwrap();
    assert_eq!(
        diode_rejoined.first_halves().exceptions,
        before_omit.first_halves().exceptions
    );
    assert_eq!(
        diode_rejoined.parts.by_id(diode),
        before_omit.parts.by_id(diode)
    );
}

#[test]
fn child_rejoin_clears_only_its_exception_and_inherits_the_current_owner() {
    use HalfExceptionState::{Linked, Local, Omitted};
    use MirroredItemKind::{Matrix, Part};

    for owner_state in [Local, Omitted] {
        let start = independent_lifecycle_fixture();
        let mut child = exception(&start, Part, CHILD, Local).unwrap();
        child.parts.by_id_mut(CHILD).properties =
            Some(BTreeMap::from([("mirror-authored".into(), json!(true))]));
        let payload = child.parts.by_id(CHILD);
        let owner = exception(&child, Matrix, MATRIX, owner_state).unwrap();
        let overlays = &owner.first_halves().exceptions;
        assert_eq!(overlays.len(), 2);
        assert!(overlays.contains(&child.first_halves().exceptions[0]));
        let explicit = overlays
            .iter()
            .find(|e| e.kind == Matrix && e.mirror_id == MATRIX)
            .unwrap();
        let mut inherited = exception(&owner, Part, CHILD, Linked).unwrap();
        assert_eq!(
            inherited.first_halves().exceptions,
            std::slice::from_ref(explicit)
        );
        assert_eq!(
            inherited.matrices.iter().any(|m| m.id == MATRIX),
            owner_state == Local
        );
        assert_eq!(
            inherited.parts.iter().find(|p| p.id == CHILD),
            (owner_state == Local).then_some(payload)
        );
        assert_eq!(
            exception(&inherited, Part, CHILD, Linked).unwrap(),
            inherited
        );

        inherited.parts.by_id_mut(SOURCE_CHILD).properties =
            Some(BTreeMap::from([("source-edited".into(), json!(true))]));
        let rejoined = exception(&inherited, Matrix, MATRIX, Linked).unwrap();
        assert!(rejoined.first_halves().exceptions.is_empty());
        assert_eq!(
            rejoined.parts.by_id(CHILD).properties,
            inherited.parts.by_id(SOURCE_CHILD).properties
        );
    }
}
