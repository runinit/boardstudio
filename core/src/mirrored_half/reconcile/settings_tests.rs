//! Keycap settings and plain nets follow their source after creation (ticket 147).
//!
//! The pair is built by the production creation path, an edit is applied to one half as 111 will
//! apply it (`canonicalize` the request, execute the source-equivalent operation), and the mirror
//! is reconciled from its source.

use super::{MatrixLink, PartProjection, Projection, reconcile};
use crate::mirrored_half::inverse;
use crate::mirrored_half::matrix_addition::tests::{create_from, halves, matrix, source_half};
use crate::model::{
    EditOperation, KeycapBoardChange, KeycapKeyChange, KeycapMatrixChange, KeycapProfile,
    MirroredException, MirroredExceptionState, MirroredItemKind, Net, Pin, ProjectDoc,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const DIODE: &str = "generator:ceoloide/diode_tht_sod123";

fn pin(part: &str, pad: &str) -> Pin {
    Pin {
        part_id: part.into(),
        pad_id: pad.into(),
    }
}

fn net(id: &str, name: &str, pins: Vec<Pin>) -> Net {
    Net {
        id: id.into(),
        name: name.into(),
        pins,
    }
}

/// A left half with a matrix, a controller, an independent `left/J1`, three nets and keycap
/// settings, mirrored by the creation path.
fn paired() -> ProjectDoc {
    let mut document = source_half();
    document.parts.push(
        serde_json::from_value(
            json!({"id": "left/J1", "definitionId": DIODE, "reference": "J1",
            "pose": {"at": {"x": -24, "y": 30}, "rotation": 0}, "side": "front"}),
        )
        .unwrap(),
    );
    let board = document.boards.iter_mut().find(|b| b.id == "left").unwrap();
    board.part_ids.push("left/J1".into());
    board.net_ids = vec!["left-col0".into(), "left-aux".into(), "left-solo".into()];
    document.nets = vec![
        net(
            "left-col0",
            "LEFT_COL0",
            vec![pin("matrix/left-keys/r0c0", "pad-1"), pin("left/U1", "P1")],
        ),
        net(
            "left-aux",
            "LEFT_AUX",
            vec![pin("left/J1", "pad-1"), pin("left/U1", "P2")],
        ),
        net("left-solo", "LEFT_SOLO", vec![pin("left/J1", "pad-2")]),
    ];
    let mut keycaps = crate::model::KeycapConfiguration::default();
    keycaps.boards.insert("left".into(), Default::default());
    keycaps.keys.insert(
        "matrix/left-keys/r0c0".into(),
        crate::model::KeycapKeySettings {
            legend: Some("Q".into()),
            ..Default::default()
        },
    );
    keycaps.keys.insert(
        "left/J1".into(),
        crate::model::KeycapKeySettings {
            legend: Some("J".into()),
            ..Default::default()
        },
    );
    document.keycaps = Some(keycaps);
    let document = create_from(
        document,
        json!({"at": {"x": 200, "y": 0}, "rotation": 0}),
        "literal-mirror",
        None,
    );
    crate::mechanical::halves::validate_document(&document).expect("the fixture is valid");
    document
}

fn reconciled(document: &ProjectDoc) -> ProjectDoc {
    let mut document = document.clone();
    let pair = document.hardware.as_ref().unwrap().pairs[0].clone();
    let source = matrix(&document, "left-keys").clone();
    let mirror = matrix(&document, "right-keys").clone();
    let keys: BTreeSet<&String> = source
        .part_ids
        .iter()
        .filter(|id| !id.ends_with("/diode"))
        .collect();
    let parts = halves(&document)
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Part && keys.contains(&item.source_id))
        .map(|item| PartProjection {
            source_id: item.source_id.clone(),
            mirror_id: item.mirror_id.clone(),
        })
        .collect();
    let mut final_owners = BTreeMap::new();
    for id in &source.part_ids {
        final_owners.insert(id.clone(), "left".to_owned());
    }
    for id in &mirror.part_ids {
        final_owners.insert(id.clone(), "right".to_owned());
    }
    let projection = Projection {
        matrix: Some(MatrixLink {
            source_matrix_id: "left-keys".into(),
            mirror_matrix_id: "right-keys".into(),
        }),
        preserve_matrix_geometry: false,
        source_board_id: "left".into(),
        mirror_board_id: "right".into(),
        mapping: pair.mapping,
        parts,
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: Default::default(),
        applied_hosted_operations: Default::default(),
        applied_hosted_circuit_additions: Default::default(),
        hosted_removal_baselines: Default::default(),
        hosted_omission_baselines: Default::default(),
        outline_removal_baselines: Default::default(),
        locally_protected_mirror_parts: BTreeSet::new(),
        final_owners,
    };
    reconcile(&mut document, &pair.id, &projection)
        .unwrap_or_else(|error| panic!("the mirror reconciles: {error}"));
    document
}

/// Applies a source-side request as written.
fn applied(document: &ProjectDoc, request: &EditOperation) -> ProjectDoc {
    let mut next = document.clone();
    crate::apply(&mut next, &[], request).expect("the request applies");
    next
}

/// Applies a request the way 111 will: the canonical, source-equivalent operation executes.
fn applied_canonical(document: &ProjectDoc, request: &EditOperation) -> ProjectDoc {
    let canonical = inverse::canonicalize(document, request).unwrap_or_else(|r| panic!("{r}"));
    applied(document, &canonical.operation)
}

fn key<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a crate::model::KeycapKeySettings> {
    document.keycaps.as_ref().and_then(|k| k.keys.get(id))
}

fn legend(document: &ProjectDoc, id: &str) -> Option<String> {
    key(document, id).and_then(|settings| settings.legend.clone())
}

fn net_of<'a>(document: &'a ProjectDoc, id: &str) -> Option<&'a Net> {
    document.nets.iter().find(|net| net.id == id)
}

fn set_legend(id: &str, value: &str) -> EditOperation {
    EditOperation::SetKeycapKey {
        key_id: id.into(),
        change: KeycapKeyChange::Legend {
            value: Some(value.into()),
        },
    }
}

fn exception(document: &mut ProjectDoc, mirror_id: &str, state: MirroredExceptionState) {
    exception_of(document, MirroredItemKind::Part, mirror_id, state);
}

fn exception_of(
    document: &mut ProjectDoc,
    kind: MirroredItemKind,
    mirror_id: &str,
    state: MirroredExceptionState,
) {
    let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
    pair.halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind,
            mirror_id: mirror_id.into(),
            state,
            removed_copper: None,
        });
}

/// What Omit leaves: the mirror item gone from parts, board and nets, plus the overlay.
fn omit_j1(document: &mut ProjectDoc) {
    document.parts.retain(|part| part.id != "right/J1");
    for board in &mut document.boards {
        board.part_ids.retain(|id| id != "right/J1");
    }
    for net in &mut document.nets {
        net.pins.retain(|pin| pin.part_id != "right/J1");
    }
    exception(document, "right/J1", MirroredExceptionState::Omitted);
}

#[test]
fn the_fixture_reconciles_to_itself() {
    let document = paired();
    assert_eq!(reconciled(&document), document);
}

#[test]
fn a_source_keycap_board_setting_reaches_the_mirror_board() {
    let document = applied(
        &paired(),
        &EditOperation::SetKeycapBoard {
            board_id: "left".into(),
            change: KeycapBoardChange::Color {
                value: "#112233".into(),
            },
        },
    );
    let after = reconciled(&document);
    let boards = &after.keycaps.as_ref().unwrap().boards;
    assert_eq!(boards["right"].color, "#112233");
    assert_eq!(boards["right"], boards["left"]);
    assert_eq!(
        after.parts, document.parts,
        "unrelated geometry is untouched"
    );
    assert_eq!(after.keymap, document.keymap, "bindings are never mirrored");
}

#[test]
fn a_source_key_setting_reaches_the_mirror_key_and_legends_are_copied() {
    let document = applied(&paired(), &set_legend("matrix/left-keys/r0c0", "Z"));
    let after = reconciled(&document);
    assert_eq!(
        legend(&after, "matrix/right-keys/r0c0").as_deref(),
        Some("Z")
    );
    assert_eq!(
        key(&after, "matrix/right-keys/r0c0"),
        key(&after, "matrix/left-keys/r0c0")
    );
    assert_eq!(
        legend(&after, "right/J1").as_deref(),
        Some("J"),
        "an unrelated mirror key keeps its value"
    );
}

#[test]
fn a_source_matrix_keycap_choice_reaches_the_mirror_matrix() {
    let document = applied(
        &paired(),
        &EditOperation::SetMatrixKeycaps {
            matrix_id: "left-keys".into(),
            change: KeycapMatrixChange::Profile {
                value: Some(KeycapProfile::Choc),
            },
        },
    );
    let after = reconciled(&document);
    let matrices = &after.keycaps.as_ref().unwrap().matrices;
    assert_eq!(matrices["right-keys"].profile, Some(KeycapProfile::Choc));
}

#[test]
fn a_mirror_key_setting_the_source_does_not_have_is_removed() {
    let mut document = paired();
    document.keycaps.as_mut().unwrap().keys.insert(
        "matrix/right-keys/r0c1".into(),
        crate::model::KeycapKeySettings {
            legend: Some("stray".into()),
            ..Default::default()
        },
    );
    let after = reconciled(&document);
    assert!(key(&after, "matrix/right-keys/r0c1").is_none());
}

#[test]
fn a_mirror_side_keycap_edit_reaches_the_source_then_the_mirror_exactly_once() {
    let document = paired();
    let request = set_legend("matrix/right-keys/r0c0", "M");
    let source_edit = applied_canonical(&document, &request);
    assert_eq!(
        legend(&source_edit, "matrix/left-keys/r0c0").as_deref(),
        Some("M")
    );
    let once = reconciled(&source_edit);
    assert_eq!(
        legend(&once, "matrix/right-keys/r0c0").as_deref(),
        Some("M")
    );
    assert_eq!(
        once.keycaps.as_ref().unwrap().keys.len(),
        source_edit.keycaps.as_ref().unwrap().keys.len(),
        "no second entry appears"
    );
    assert_eq!(reconciled(&once), once, "a second pass changes nothing");
}

#[test]
fn a_mirror_side_keycap_board_edit_reaches_the_source_then_the_mirror_exactly_once() {
    let document = paired();
    let request = EditOperation::SetKeycapBoard {
        board_id: "right".into(),
        change: KeycapBoardChange::Clearance { value: 1.5 },
    };
    let source_edit = applied_canonical(&document, &request);
    assert_eq!(
        source_edit.keycaps.as_ref().unwrap().boards["left"].clearance,
        1.5
    );
    let once = reconciled(&source_edit);
    assert_eq!(
        once.keycaps.as_ref().unwrap().boards["right"].clearance,
        1.5
    );
    assert_eq!(reconciled(&once), once);
}

#[test]
fn a_local_key_keeps_its_own_mirror_value_across_a_source_edit() {
    let mut document = paired();
    exception(&mut document, "right/J1", MirroredExceptionState::Local);
    document
        .keycaps
        .as_mut()
        .unwrap()
        .keys
        .get_mut("right/J1")
        .unwrap()
        .legend = Some("OWN".into());
    let document = applied(&document, &set_legend("left/J1", "J2"));
    let after = reconciled(&document);
    assert_eq!(legend(&after, "right/J1").as_deref(), Some("OWN"));
    assert_eq!(legend(&after, "left/J1").as_deref(), Some("J2"));
}

#[test]
fn an_omitted_part_has_no_mirror_keycap_setting_and_is_not_resurrected() {
    let mut document = paired();
    omit_j1(&mut document);
    // The omission left a stale entry behind, and the source edits the item meanwhile.
    let document = applied(&document, &set_legend("left/J1", "J2"));
    let after = reconciled(&document);
    assert!(key(&after, "right/J1").is_none());
    assert!(after.parts.iter().all(|part| part.id != "right/J1"));
}

#[test]
fn a_net_that_exists_only_because_of_an_omitted_part_is_removed_not_left_empty() {
    let mut document = paired();
    omit_j1(&mut document);
    // `right-solo` is left empty by the omission: every non-controller source pin is omitted.
    assert!(net_of(&document, "right-solo").unwrap().pins.is_empty());
    let after = reconciled(&document);
    assert!(net_of(&after, "right-solo").is_none());
    let right = after.boards.iter().find(|b| b.id == "right").unwrap();
    assert!(!right.net_ids.iter().any(|id| id == "right-solo"));
    // A pin on the controller does not keep a net: `right-aux` is private to the omitted part too.
    assert!(net_of(&after, "right-aux").is_none());
    // A net with a pin on a mirrored part is untouched.
    assert_eq!(
        net_of(&after, "right-col0"),
        net_of(&document, "right-col0")
    );
    assert!(
        crate::mechanical::halves::validate_document(&after).is_ok(),
        "the absence is the private-net rule"
    );
}

#[test]
fn a_mirror_net_returns_and_registers_on_the_mirror_board_when_it_is_not_private() {
    let mut document = paired();
    document.nets.retain(|net| net.id != "right-solo");
    for board in &mut document.boards {
        board.net_ids.retain(|id| id != "right-solo");
    }
    let after = reconciled(&document);
    let solo = net_of(&after, "right-solo").expect("the net is projected again");
    assert_eq!(solo.name, "RIGHT_SOLO");
    assert_eq!(solo.pins, vec![pin("right/J1", "pad-2")]);
    let right = after.boards.iter().find(|b| b.id == "right").unwrap();
    assert!(right.net_ids.iter().any(|id| id == "right-solo"));
    let left = after.boards.iter().find(|b| b.id == "left").unwrap();
    assert!(!left.net_ids.iter().any(|id| id == "right-solo"));
}

#[test]
fn reconciling_twice_is_the_same_as_once_for_every_edit() {
    // Ticket 37: SetNet is deleted; the legend edit still drives a mirror
    // projection so double-reconcile idempotence stays guarded.
    let edits = [set_legend("matrix/left-keys/r0c0", "Z")];
    let mut document = paired();
    for edit in &edits {
        document = applied(&document, edit);
    }
    let once = reconciled(&document);
    assert_eq!(reconciled(&once), once);
    assert_ne!(once, document);
}

#[test]
fn an_omitted_net_is_not_projected_again() {
    let mut document = paired();
    exception_of(
        &mut document,
        MirroredItemKind::Net,
        "right-col0",
        MirroredExceptionState::Omitted,
    );
    document.nets.retain(|net| net.id != "right-col0");
    for board in &mut document.boards {
        board.net_ids.retain(|id| id != "right-col0");
    }
    let after = reconciled(&document);
    assert!(net_of(&after, "right-col0").is_none());
}

#[test]
fn a_pin_the_mirror_authored_on_its_own_part_keeps_an_otherwise_private_net() {
    let mut document = paired();
    omit_j1(&mut document);
    document.parts.push(
        serde_json::from_value(
            json!({"id": "right/X1", "definitionId": DIODE, "reference": "X1",
            "pose": {"at": {"x": 150, "y": 30}, "rotation": 0}, "side": "front"}),
        )
        .unwrap(),
    );
    let board = document
        .boards
        .iter_mut()
        .find(|b| b.id == "right")
        .unwrap();
    board.part_ids.push("right/X1".into());
    let net = document
        .nets
        .iter_mut()
        .find(|n| n.id == "right-solo")
        .unwrap();
    net.pins.push(pin("right/X1", "pad-1"));
    let after = reconciled(&document);
    assert_eq!(
        net_of(&after, "right-solo").unwrap().pins,
        vec![pin("right/X1", "pad-1")],
        "a net the mirror still uses is not removed by the rule"
    );
}

/// The projection alone, on a document edited by hand: states `reconcile` itself would refuse.
fn projected(document: &ProjectDoc) -> ProjectDoc {
    let mut staged = document.clone();
    super::settings::project(document, &mut staged, halves(document)).unwrap();
    staged
}

#[test]
fn an_omitted_overlay_removes_the_entry_even_while_the_part_is_still_present() {
    let mut document = paired();
    exception(&mut document, "right/J1", MirroredExceptionState::Omitted);
    let after = projected(&document);
    assert!(key(&after, "right/J1").is_none());
    assert!(key(&after, "matrix/right-keys/r0c0").is_some());
}

#[test]
fn a_linked_part_that_is_absent_from_the_mirror_has_no_entry() {
    // The child of an Omitted matrix or circuit stores no overlay of its own.
    let mut document = paired();
    document.parts.retain(|part| part.id != "right/J1");
    let after = projected(&document);
    assert!(key(&after, "right/J1").is_none());
}

#[test]
fn a_circuit_net_is_left_to_the_circuit_projection() {
    let mut document = paired();
    document
        .nets
        .iter_mut()
        .find(|net| net.id == "left-col0")
        .unwrap()
        .pins
        .push(pin("left/U1", "P9"));
    document.embedded_circuits.push(
        serde_json::from_value(
            json!({"id": "c", "definitionId": "d", "hostBoardId": "left",
            "partIds": [], "netIds": ["left-col0"], "ports": {}}),
        )
        .unwrap(),
    );
    let after = projected(&document);
    assert_eq!(
        net_of(&after, "right-col0"),
        net_of(&document, "right-col0")
    );
}
