use super::super::{
    BaselineKind, FinalOwner, MembershipContext, MembershipTransitionInput, PlannedAddition,
    TransitionAssociation, TransitionState, prepare_membership_transition,
};
use super::{
    ConstraintLifecycleEffect, prepare_constraint_lifecycle_delta, retire_replacement_records,
};
use crate::constraints::{ConstraintEndpoint, ConstraintRelation, FramedConstraint};
use crate::mirrored_half::reconcile::tests::fixture_document;
use crate::model::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn lifecycle_constraint(
    id: &str,
    source: ConstraintEndpoint,
    target: ConstraintEndpoint,
) -> FramedConstraint {
    FramedConstraint::Framed {
        id: id.into(),
        source,
        target,
        relation: ConstraintRelation::Offset {
            offset: Vec2 { x: 3.5, y: -2.25 },
            rotation: 17.0,
        },
    }
}

fn literal_endpoint(part_id: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::Part {
        part_id: part_id.into(),
    }
}

fn linked_endpoint(
    pair_id: &str,
    source_part_id: &str,
    mirror_part_id: &str,
) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: pair_id.into(),
        source_part_id: source_part_id.into(),
        mirror_part_id: mirror_part_id.into(),
    }
}

fn lifecycle_record<'a>(records: &'a [FramedConstraint], id: &str) -> &'a FramedConstraint {
    records
        .iter()
        .find(
            |record| matches!(record, FramedConstraint::Framed { id: actual, .. } if actual == id),
        )
        .unwrap_or_else(|| panic!("missing lifecycle constraint {id}"))
}

#[test]
fn make_local_rebinds_each_matching_visible_endpoint_without_changing_the_relation() {
    let linked = linked_endpoint("pair", "source-part", "mirror-part");
    let records = vec![
        lifecycle_constraint("outgoing", linked.clone(), literal_endpoint("other-target")),
        lifecycle_constraint("incoming", literal_endpoint("other-source"), linked),
        lifecycle_constraint(
            "source-owned",
            literal_endpoint("other-source"),
            literal_endpoint("source-part"),
        ),
    ];
    let effect = ConstraintLifecycleEffect::MakeLocal {
        pair_id: "pair".into(),
        source_part_id: "source-part".into(),
        mirror_part_id: "mirror-part".into(),
    };

    let delta = prepare_constraint_lifecycle_delta(&records, &[effect]).unwrap();

    assert!(delta.removals.is_empty());
    assert_eq!(delta.upserts.len(), 2);
    let FramedConstraint::Framed {
        id,
        source,
        target,
        relation,
    } = lifecycle_record(&delta.upserts, "outgoing");
    assert_eq!(id, "outgoing");
    assert_eq!(source, &literal_endpoint("mirror-part"));
    assert_eq!(target, &literal_endpoint("other-target"));
    assert_eq!(
        relation,
        &ConstraintRelation::Offset {
            offset: Vec2 { x: 3.5, y: -2.25 },
            rotation: 17.0,
        }
    );
    let FramedConstraint::Framed { source, target, .. } =
        lifecycle_record(&delta.upserts, "incoming");
    assert_eq!(source, &literal_endpoint("other-source"));
    assert_eq!(target, &literal_endpoint("mirror-part"));
    assert!(!delta.removals.iter().any(|id| id == "source-owned"));
    assert!(!delta.upserts.iter().any(|record| {
        matches!(record, FramedConstraint::Framed { id, .. } if id == "source-owned")
    }));
}

#[test]
fn omit_removes_only_constraints_naming_a_removed_visible_endpoint() {
    let records = vec![
        lifecycle_constraint(
            "source-owned",
            literal_endpoint("source-part"),
            literal_endpoint("other-part"),
        ),
        lifecycle_constraint(
            "linked-visible",
            literal_endpoint("source-part"),
            linked_endpoint("pair", "source-part", "mirror-part"),
        ),
        lifecycle_constraint(
            "local-visible",
            literal_endpoint("other-part"),
            literal_endpoint("local-mirror-part"),
        ),
        lifecycle_constraint(
            "other-pair",
            literal_endpoint("other-part"),
            linked_endpoint("other-pair", "other-source", "other-mirror"),
        ),
    ];
    let effect = ConstraintLifecycleEffect::RemoveVisibleParts {
        part_ids: BTreeSet::from(["mirror-part".into(), "local-mirror-part".into()]),
    };

    let delta = prepare_constraint_lifecycle_delta(&records, &[effect]).unwrap();

    assert_eq!(delta.removals, ["linked-visible", "local-visible"]);
    assert!(delta.upserts.is_empty());
    assert!(records.iter().any(|record| {
        matches!(record, FramedConstraint::Framed { id, .. } if id == "source-owned")
    }));
    assert!(records.iter().any(|record| {
        matches!(record, FramedConstraint::Framed { id, .. } if id == "other-pair")
    }));
}

#[test]
fn rejoin_retires_incoming_local_constraints_and_rebinds_outgoing_sources() {
    let records = vec![
        lifecycle_constraint(
            "incoming",
            literal_endpoint("other-source"),
            literal_endpoint("mirror-part"),
        ),
        lifecycle_constraint(
            "outgoing",
            literal_endpoint("mirror-part"),
            literal_endpoint("other-target"),
        ),
        lifecycle_constraint(
            "unrelated",
            literal_endpoint("unrelated-source"),
            literal_endpoint("unrelated-target"),
        ),
    ];
    let effect = ConstraintLifecycleEffect::Rejoin {
        pair_id: "pair".into(),
        source_part_id: "source-part".into(),
        mirror_part_id: "mirror-part".into(),
    };

    let delta = prepare_constraint_lifecycle_delta(&records, &[effect]).unwrap();

    assert_eq!(delta.removals, ["incoming"]);
    assert_eq!(delta.upserts.len(), 1);
    let FramedConstraint::Framed {
        id,
        source,
        target,
        relation,
    } = lifecycle_record(&delta.upserts, "outgoing");
    assert_eq!(id, "outgoing");
    assert_eq!(
        source,
        &linked_endpoint("pair", "source-part", "mirror-part")
    );
    assert_eq!(target, &literal_endpoint("other-target"));
    assert_eq!(
        relation,
        &ConstraintRelation::Offset {
            offset: Vec2 { x: 3.5, y: -2.25 },
            rotation: 17.0,
        }
    );
    assert!(
        delta.upserts.iter().all(
            |record| !matches!(record, FramedConstraint::Framed { id, .. } if id == "unrelated")
        )
    );
}

#[test]
fn rejoin_does_not_synthesize_constraints_retired_by_omit() {
    let effect = ConstraintLifecycleEffect::Rejoin {
        pair_id: "pair".into(),
        source_part_id: "source-part".into(),
        mirror_part_id: "mirror-part".into(),
    };

    assert_eq!(
        prepare_constraint_lifecycle_delta(&[], &[effect]).unwrap(),
        super::ConstraintLifecycleDelta::default(),
        "Rejoin cannot synthesize a constraint that Omit already retired"
    );
}

#[test]
fn unlink_flattens_only_selected_pair_endpoints_without_cloning_records() {
    let records = vec![
        lifecycle_constraint(
            "selected-pair",
            linked_endpoint("pair", "source-one", "mirror-one"),
            linked_endpoint("pair", "source-two", "mirror-two"),
        ),
        lifecycle_constraint(
            "other-pair",
            linked_endpoint("other-pair", "other-source", "other-mirror"),
            literal_endpoint("fixed-target"),
        ),
    ];
    let effect = ConstraintLifecycleEffect::Unlink {
        pair_id: "pair".into(),
    };

    let delta = prepare_constraint_lifecycle_delta(&records, &[effect]).unwrap();

    assert!(delta.removals.is_empty());
    assert_eq!(delta.upserts.len(), 1);
    let FramedConstraint::Framed {
        id,
        source,
        target,
        relation,
    } = lifecycle_record(&delta.upserts, "selected-pair");
    assert_eq!(id, "selected-pair");
    assert_eq!(source, &literal_endpoint("mirror-one"));
    assert_eq!(target, &literal_endpoint("mirror-two"));
    assert_eq!(
        relation,
        &ConstraintRelation::Offset {
            offset: Vec2 { x: 3.5, y: -2.25 },
            rotation: 17.0,
        }
    );
}

fn omitted_halves() -> Value {
    json!({
        "sourceBoardId": "source-board",
        "mirrorBoardId": "mirror-board",
        "correspondences": [{
            "kind": "part",
            "sourceId": "source-part",
            "mirrorId": "mirror-part"
        }],
        "exceptions": [{
            "kind": "part",
            "mirrorId": "mirror-part",
            "state": "omitted"
        }]
    })
}

#[test]
fn omitted_exception_round_trip_keeps_identity_in_its_correspondence() {
    let halves: MirroredHalves = serde_json::from_value(omitted_halves()).unwrap();
    let encoded = serde_json::to_value(halves).unwrap();

    assert_eq!(
        encoded["correspondences"][0]["sourceId"], "source-part",
        "the correspondence keeps the source identity while the mirror geometry is absent"
    );
    assert!(encoded["exceptions"][0].get("sourceId").is_none());
    assert_eq!(encoded["exceptions"][0]["state"], "omitted");
}

fn board(id: &str, parts: &[&str]) -> Board {
    serde_json::from_value(json!({
        "id": id,
        "name": id,
        "outlineIds": [],
        "partIds": parts,
        "netIds": [],
        "thickness": 1.6
    }))
    .unwrap()
}

fn part(id: &str) -> Part {
    serde_json::from_value(json!({
        "id": id,
        "definitionId": "test-footprint",
        "reference": "U1",
        "pose": { "at": { "x": 12.0, "y": 3.0 }, "rotation": 0.0 },
        "side": "front"
    }))
    .unwrap()
}

fn valid_omitted_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("omitted-doc", "Omitted lifecycle fixture");
    document.boards = vec![
        board("source-board", &["source-part"]),
        board("mirror-board", &[]),
    ];
    document.parts = vec![part("source-part")];

    let pair: MechanicalMirrorPair = serde_json::from_value(json!({
        "id": "pair",
        "members": ["source-instance", "mirror-instance"],
        "mapping": { "at": { "x": 100.0, "y": 0.0 }, "rotation": 0.0 },
        "constructionSource": "source-instance",
        "construction": {
            "method": "printed",
            "mount": "tray",
            "integratedPlateFrame": false,
            "plateThickness": 1.3,
            "plateFoamThickness": 2.0,
            "bottomFoamThickness": 2.0,
            "bottomThickness": 3.0,
            "plateToPcb": 2.2,
            "wallThickness": 2.0,
            "clearance": 0.3
        },
        "halves": {
            "sourceBoardId": "source-board",
            "mirrorBoardId": "mirror-board",
            "correspondences": [{
                "kind": "part",
                "sourceId": "source-part",
                "mirrorId": "mirror-part"
            }],
            "exceptions": [{
                "kind": "part",
                "mirrorId": "mirror-part",
                "state": "omitted"
            }]
        }
    }))
    .unwrap();
    document.hardware = Some(HardwareConfiguration {
        pairs: vec![pair],
        ..HardwareConfiguration::default()
    });
    document
}

#[test]
fn valid_durable_omission_accepts_an_absent_mirror_part() {
    let document = valid_omitted_document();
    assert!(
        crate::mechanical::halves::validate_document(&document).is_ok(),
        "a source-bound Omitted record remains valid after its target geometry is removed"
    );
}

#[test]
fn omitted_absence_and_source_identity_survive_document_round_trip() {
    let document = valid_omitted_document();
    let encoded = serde_json::to_vec(&document).unwrap();
    let decoded: ProjectDoc = serde_json::from_slice(&encoded).unwrap();

    assert!(crate::mechanical::halves::validate_document(&decoded).is_ok());
    assert!(decoded.parts.iter().all(|part| part.id != "mirror-part"));
    assert!(
        decoded
            .boards
            .iter()
            .all(|board| !board.part_ids.iter().any(|id| id == "mirror-part"))
    );
    let halves = decoded.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    let exception = &halves.exceptions[0];
    assert_eq!(halves.overlay_source(exception), Some("source-part"));
    assert_eq!(exception.mirror_id, "mirror-part");
}

#[test]
fn legacy_exception_without_source_identity_remains_decodable() {
    let legacy = json!({
        "sourceBoardId": "source-board",
        "mirrorBoardId": "mirror-board",
        "exceptions": [{
            "kind": "part",
            "mirrorId": "mirror-part",
            "state": "local"
        }]
    });
    let decoded: MirroredHalves = serde_json::from_value(legacy).unwrap();
    let encoded = serde_json::to_value(decoded).unwrap();

    assert_eq!(encoded["exceptions"][0]["mirrorId"], "mirror-part");
    assert!(encoded["exceptions"][0].get("sourceId").is_none());
}

fn nonempty_unlink_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("unlink-doc", "Synthetic Unlink fixture");
    document.boards = vec![board("source-board", &[]), board("mirror-board", &[])];
    document.hardware = Some(
        serde_json::from_value(json!({
            "topology": "split",
            "transport": "wired",
            "instances": [
                {"id":"source-instance","name":"Source","boardId":"source-board",
                 "half":"left","role":"central","flipped":false,
                 "constructionOwnership":"pair","mechanical":{
                    "boardId":"source-board","method":"printed","mount":"rigid",
                    "integratedPlateFrame":false,"bottomStyle":"shell","middleFrame":false,
                    "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
                    "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
                    "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
                    "openingAllowance":0.25,"nextOpeningIdentity":1,
                    "mounts":[],"closureMounts":[],"profiles":[]}},
                {"id":"mirror-instance","name":"Mirror","boardId":"mirror-board",
                 "half":"right","role":"peripheral","flipped":true,
                 "constructionOwnership":"pair","mechanical":{
                    "boardId":"mirror-board","method":"printed","mount":"rigid",
                    "integratedPlateFrame":false,"bottomStyle":"shell","middleFrame":false,
                    "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
                    "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
                    "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
                    "openingAllowance":0.25,"nextOpeningIdentity":1,
                    "mounts":[],"closureMounts":[],"profiles":[]}}
            ],
            "boards": [],
            "pairs": [{
                "id":"unlink-pair",
                "members":["source-instance","mirror-instance"],
                "mapping":{"at":{"x":100.0,"y":0.0},"rotation":0.0},
                "construction":{
                    "method":"printed","mount":"rigid","integratedPlateFrame":false,
                    "bottomStyle":"shell","middleFrame":false,"plateThickness":1.5,
                    "plateFoamThickness":0.5,"bottomFoamThickness":0.4,
                    "bottomThickness":3.0,"plateToPcb":3.5,"wallThickness":2.0,
                    "clearance":0.3
                },
                "constructionSource":"source-instance",
                "halves":{"sourceBoardId":"source-board","mirrorBoardId":"mirror-board",
                          "correspondences":[],"exceptions":[]},
                "features":[
                    {"id":"closure-1","kind":"closure-mount","author":"source-instance",
                     "value":{"kind":"mount","mount":{"id":"closure-1","at":{"x":-24.0,"y":-16.0},"kind":"hole","holeDiameter":2.2}},
                     "members":[{"instanceId":"source-instance","featureId":"closure-1","state":"linked"},{"instanceId":"mirror-instance","featureId":"closure-1","state":"linked"}]},
                    {"id":"closure-2","kind":"closure-mount","author":"source-instance",
                     "value":{"kind":"mount","mount":{"id":"closure-2","at":{"x":-8.0,"y":-16.0},"kind":"hole","holeDiameter":2.2}},
                     "members":[{"instanceId":"source-instance","featureId":"closure-2","state":"linked"},{"instanceId":"mirror-instance","featureId":"closure-2","state":"linked"}]},
                    {"id":"closure-3","kind":"closure-mount","author":"source-instance",
                     "value":{"kind":"mount","mount":{"id":"closure-3","at":{"x":8.0,"y":-16.0},"kind":"hole","holeDiameter":2.2}},
                     "members":[{"instanceId":"source-instance","featureId":"closure-3","state":"linked"},{"instanceId":"mirror-instance","featureId":"closure-3","state":"linked"}]},
                    {"id":"closure-4","kind":"closure-mount","author":"source-instance",
                     "value":{"kind":"mount","mount":{"id":"closure-4","at":{"x":24.0,"y":-16.0},"kind":"hole","holeDiameter":2.2}},
                     "members":[{"instanceId":"source-instance","featureId":"closure-4","state":"linked"},{"instanceId":"mirror-instance","featureId":"closure-4","state":"linked"}]}
                ]
            }]
        }))
        .unwrap(),
    );
    document
}

fn expected_closure_ids(board_id: &str) -> BTreeSet<String> {
    let points = if board_id == "mirror-board" {
        [(-124, -16), (-108, -16), (-92, -16), (-76, -16)]
    } else {
        [(-24, -16), (-8, -16), (8, -16), (24, -16)]
    };
    points
        .into_iter()
        .map(|(x, y)| format!("case-closure/{board_id}/{x}.00000/{y}.00000"))
        .collect()
}

fn unlink_diff_ids(before: &ProjectDoc, after: &ProjectDoc) -> BTreeSet<String> {
    let (mut ids, fields) = super::super::changed_document_fields(before, after).unwrap();
    assert!(
        fields.iter().all(|path| {
            path == "$.hardware.pairs" || path.starts_with("$.hardware.instances[")
        }),
        "Unlink changes only identified hardware pairs/instances: {fields:?}"
    );

    let before_hardware = before.hardware.as_ref().unwrap();
    let after_hardware = after.hardware.as_ref().unwrap();
    let before_instances: std::collections::BTreeMap<_, _> = before_hardware
        .instances
        .iter()
        .map(|instance| (instance.id.as_str(), instance))
        .collect();
    let after_instances: std::collections::BTreeMap<_, _> = after_hardware
        .instances
        .iter()
        .map(|instance| (instance.id.as_str(), instance))
        .collect();
    for id in before_instances.keys().chain(after_instances.keys()) {
        if before_instances.get(id) != after_instances.get(id) {
            ids.push((*id).to_owned());
        }
    }

    let before_pairs: std::collections::BTreeMap<_, _> = before_hardware
        .pairs
        .iter()
        .map(|pair| (pair.id.as_str(), pair))
        .collect();
    let after_pairs: std::collections::BTreeMap<_, _> = after_hardware
        .pairs
        .iter()
        .map(|pair| (pair.id.as_str(), pair))
        .collect();
    for id in before_pairs.keys().chain(after_pairs.keys()) {
        if before_pairs.get(id) != after_pairs.get(id) {
            ids.push((*id).to_owned());
        }
    }

    ids.into_iter().collect()
}

fn reuse_board_unlink_document() -> ProjectDoc {
    let mut document = nonempty_unlink_document();
    document.boards.pop();
    let hardware = document.hardware.as_mut().unwrap();
    hardware.instances[1].board_id = "source-board".into();
    let pair = &mut hardware.pairs[0];
    pair.mapping = crate::model::Pose2 {
        at: crate::model::Vec2::default(),
        rotation: 0.0,
    };
    let halves = pair.halves.as_mut().unwrap();
    halves.mirror_board_id = "source-board".into();
    document
}

#[test]
fn unlink_uses_the_exact_pair_and_materializes_nonempty_owned_closures() {
    let mut document = nonempty_unlink_document();
    crate::mechanical::halves::validate_document(&document).unwrap();
    let before = document.clone();
    let before_effective: Vec<_> = ["source-instance", "mirror-instance"]
        .into_iter()
        .map(|instance_id| {
            crate::mechanical::inputs::effective_inputs(
                &before,
                &[],
                &crate::mechanical::MechanicalTarget::Instance {
                    instance_id: instance_id.into(),
                },
            )
            .unwrap()
            .configuration
        })
        .collect();
    let before_labels: Vec<_> = before
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .map(|instance| {
            (
                instance.name.clone(),
                instance.half.clone(),
                instance.role.clone(),
                instance.flipped,
            )
        })
        .collect();
    let changed = crate::mechanical::halves::unlink_pair(&mut document, "unlink-pair").unwrap();

    assert!(document.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(changed.contains(&"unlink-pair".to_owned()));
    for (index, instance) in document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .enumerate()
    {
        assert_eq!(
            instance.construction_ownership,
            crate::model::MechanicalConstructionOwnership::Local
        );
        let current = instance.mechanical.as_ref().unwrap();
        let mounts = current.closure_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 4);
        let actual: BTreeSet<_> = mounts.iter().map(|mount| mount.id.as_str()).collect();
        assert_eq!(actual.len(), 4);
        assert_eq!(current, &before_effective[index]);
        let after_effective = crate::mechanical::inputs::effective_inputs(
            &document,
            &[],
            &crate::mechanical::MechanicalTarget::Instance {
                instance_id: instance.id.clone(),
            },
        )
        .unwrap()
        .configuration;
        assert_eq!(after_effective, before_effective[index]);
        assert_eq!(
            (
                instance.name.clone(),
                instance.half.clone(),
                instance.role.clone(),
                instance.flipped
            ),
            before_labels[index]
        );
    }
    for board_id in ["source-board", "mirror-board"] {
        let holes: Vec<_> = document
            .parts
            .iter()
            .filter(|part| part.id.starts_with(&format!("case-closure/{board_id}/")))
            .collect();
        assert_eq!(
            holes.len(),
            4,
            "expected four generated closure holes on {board_id}"
        );
        let hole_ids: BTreeSet<_> = holes.iter().map(|part| part.id.as_str()).collect();
        let owner = document
            .boards
            .iter()
            .find(|board| board.id == board_id)
            .unwrap();
        let registered: BTreeSet<_> = owner.part_ids.iter().map(String::as_str).collect();
        assert_eq!(
            registered, hole_ids,
            "the board records exactly its four generated hole identities"
        );
        assert_eq!(
            owner
                .part_ids
                .iter()
                .filter(|id| hole_ids.contains(id.as_str()))
                .count(),
            hole_ids.len(),
            "hole membership is unique"
        );
        assert_eq!(
            hole_ids
                .iter()
                .map(|id| (*id).to_owned())
                .collect::<BTreeSet<_>>(),
            expected_closure_ids(board_id),
            "the four independently specified closure positions produce exact part identities"
        );
        let definitions: BTreeSet<_> = document
            .definitions
            .iter()
            .filter(|definition| {
                definition
                    .id
                    .starts_with(&format!("assembly-closure/definition/{board_id}/"))
            })
            .map(|definition| definition.id.clone())
            .collect();
        assert_eq!(
            definitions,
            expected_closure_ids(board_id)
                .into_iter()
                .map(|part_id| {
                    format!(
                        "assembly-closure/definition/{}",
                        part_id.strip_prefix("case-closure/").unwrap()
                    )
                })
                .collect()
        );
    }
    let unrelated_parts: Vec<_> = document
        .parts
        .iter()
        .filter(|part| !part.id.starts_with("case-closure/"))
        .collect();
    assert_eq!(unrelated_parts, before.parts.iter().collect::<Vec<_>>());
    assert_eq!(document.matrices, before.matrices);
    assert_eq!(document.modules, before.modules);
    assert_eq!(document.embedded_circuits, before.embedded_circuits);
    assert_eq!(document.nets, before.nets);
    assert_eq!(before.boards.len(), document.boards.len());
    assert_eq!(
        before
            .parts
            .iter()
            .map(|part| &part.id)
            .collect::<BTreeSet<_>>(),
        document
            .parts
            .iter()
            .filter(|part| !part.id.starts_with("case-closure/"))
            .map(|part| &part.id)
            .collect()
    );
    assert_eq!(
        changed.into_iter().collect::<BTreeSet<_>>(),
        unlink_diff_ids(&before, &document)
    );
}

#[test]
fn prepared_unlink_uses_the_normalized_mechanical_route_on_a_private_draft() {
    let accepted = nonempty_unlink_document();
    let accepted_bytes = serde_json::to_vec(&accepted).unwrap();
    let operation: EditOperation = serde_json::from_value(json!({
        "kind": "apply-mechanical-edit",
        "edit": {
            "target": {"kind": "instance", "instanceId": "source-instance"},
            "change": {"unlink-mirror-pair": {"instance_id": "source-instance"}}
        }
    }))
    .expect("the public UnlinkMirrorPair wire shape parses");
    let mut expected = accepted.clone();
    crate::mechanical::halves::unlink_pair(&mut expected, "unlink-pair").unwrap();

    let (prepared, changed) = crate::mirrored_half::prepare_transaction(&accepted, &[], &operation)
        .expect("prepared Unlink derives the normalized context internally");

    assert_eq!(prepared, expected);
    assert!(prepared.hardware.as_ref().unwrap().pairs.is_empty());
    assert_eq!(
        changed.into_iter().collect::<BTreeSet<_>>(),
        unlink_diff_ids(&accepted, &prepared)
    );
    crate::mechanical::halves::validate_document(&prepared).unwrap();
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn prepared_unlink_publication_undo_redo_and_reopen_preserve_materialized_closures() {
    let mut engine = crate::CoreEngine::default();
    assert!(matches!(
        engine.handle(crate::model::CoreRequest::Open {
            id: "open-unlink".into(),
            document: nonempty_unlink_document(),
        }),
        crate::model::CoreReply::Scene { .. }
    ));
    let accepted = engine.document.clone();
    let operation: EditOperation = serde_json::from_value(json!({
        "kind": "apply-mechanical-edit",
        "edit": {
            "target": {"kind": "instance", "instanceId": "source-instance"},
            "change": {"unlink-mirror-pair": {"instance_id": "source-instance"}}
        }
    }))
    .expect("the public UnlinkMirrorPair wire shape parses");
    let (prepared, changed) = crate::mirrored_half::prepare_transaction(&accepted, &[], &operation)
        .expect("the nonempty pair closure prepares for Unlink");
    let expected_closures: BTreeSet<_> = prepared
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    let expected_configurations: BTreeMap<_, _> = prepared
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .map(|instance| (instance.id.clone(), instance.mechanical.clone().unwrap()))
        .collect();
    assert!(!expected_closures.is_empty());
    assert!(prepared.hardware.as_ref().unwrap().pairs.is_empty());

    let (cache, contours, findings) = crate::geometry::outlines(&prepared, None, &changed);
    let publication = engine.publish_prepared_edit(
        "unlink-prepared".into(),
        crate::PreparedEdit {
            transaction: "unlink-prepared".into(),
            document: prepared,
            changed,
            cache,
            contours,
            findings,
        },
    );
    assert!(matches!(publication, crate::model::CoreReply::Scene { .. }));
    assert_eq!(engine.undo.len(), 1);
    assert!(engine.redo.is_empty());
    assert!(engine.document.hardware.as_ref().unwrap().pairs.is_empty());
    let committed_closures: BTreeSet<_> = engine
        .document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    assert_eq!(committed_closures, expected_closures);

    assert!(matches!(
        engine.handle(crate::model::CoreRequest::Undo {
            id: "undo-unlink".into(),
        }),
        crate::model::CoreReply::Scene { .. }
    ));
    assert_eq!(engine.undo.len(), 0);
    assert_eq!(engine.redo.len(), 1);
    assert_eq!(engine.document.hardware.as_ref().unwrap().pairs.len(), 1);
    assert_eq!(
        engine.document.hardware.as_ref().unwrap().pairs[0].id,
        "unlink-pair"
    );
    assert_eq!(
        engine.document.hardware.as_ref().unwrap().instances[0].construction_ownership,
        MechanicalConstructionOwnership::Pair
    );

    assert!(matches!(
        engine.handle(crate::model::CoreRequest::Redo {
            id: "redo-unlink".into(),
        }),
        crate::model::CoreReply::Scene { .. }
    ));
    assert_eq!(engine.undo.len(), 1);
    assert!(engine.redo.is_empty());
    assert!(engine.document.hardware.as_ref().unwrap().pairs.is_empty());
    let redone_closures: BTreeSet<_> = engine
        .document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    let redone_configurations: BTreeMap<_, _> = engine
        .document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .map(|instance| (instance.id.clone(), instance.mechanical.clone().unwrap()))
        .collect();
    assert_eq!(redone_closures, expected_closures);
    assert_eq!(redone_configurations, expected_configurations);
    assert!(
        engine
            .document
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .all(|instance| instance.construction_ownership
                == MechanicalConstructionOwnership::Local)
    );

    let mut reopened = crate::CoreEngine::default();
    assert!(matches!(
        reopened.handle(crate::model::CoreRequest::Open {
            id: "reopen-unlinked".into(),
            document: engine.document.clone(),
        }),
        crate::model::CoreReply::Scene { .. }
    ));
    let reopened_closures: BTreeSet<_> = reopened
        .document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    let reopened_configurations: BTreeMap<_, _> = reopened
        .document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .map(|instance| (instance.id.clone(), instance.mechanical.clone().unwrap()))
        .collect();
    assert_eq!(reopened_closures, expected_closures);
    assert_eq!(reopened_configurations, expected_configurations);
    assert_eq!(reopened.document.hardware.as_ref().unwrap().pairs.len(), 0);
}

#[test]
fn unlink_reuse_board_keeps_one_nonempty_closure_set_and_flipped_geometry() {
    let mut document = reuse_board_unlink_document();
    let before = document.clone();
    let flipped_before = document.hardware.as_ref().unwrap().instances[1].flipped;
    let before_effective: Vec<_> = ["source-instance", "mirror-instance"]
        .into_iter()
        .map(|instance_id| {
            crate::mechanical::inputs::effective_inputs(
                &before,
                &[],
                &crate::mechanical::MechanicalTarget::Instance {
                    instance_id: instance_id.into(),
                },
            )
            .unwrap()
            .configuration
        })
        .collect();
    let changed = crate::mechanical::halves::unlink_pair(&mut document, "unlink-pair").unwrap();

    let holes: Vec<_> = document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/source-board/"))
        .collect();
    assert_eq!(holes.len(), 4);
    let ids: BTreeSet<_> = holes.iter().map(|part| part.id.as_str()).collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(
        ids,
        expected_closure_ids("source-board")
            .iter()
            .map(String::as_str)
            .collect()
    );
    assert_eq!(document.boards.len(), 1);
    assert!(document.hardware.as_ref().unwrap().pairs.is_empty());
    assert_eq!(
        document.hardware.as_ref().unwrap().instances[1].flipped,
        flipped_before
    );
    let board = &document.boards[0];
    assert_eq!(board.part_ids.len(), 4);
    assert!(ids.is_subset(&board.part_ids.iter().map(String::as_str).collect()));
    for (index, instance) in document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .enumerate()
    {
        assert_eq!(
            instance.mechanical.as_ref().unwrap(),
            &before_effective[index]
        );
        let after_effective = crate::mechanical::inputs::effective_inputs(
            &document,
            &[],
            &crate::mechanical::MechanicalTarget::Instance {
                instance_id: instance.id.clone(),
            },
        )
        .unwrap()
        .configuration;
        assert_eq!(after_effective, before_effective[index]);
    }
    assert!(before.boards[0].part_ids.is_empty());
    assert_eq!(
        changed.into_iter().collect::<BTreeSet<_>>(),
        unlink_diff_ids(&before, &document)
    );
}

#[test]
fn unlink_refuses_an_unrequested_pair_without_mutating_the_document() {
    let mut document = nonempty_unlink_document();
    let before = document.clone();

    let error = crate::mechanical::halves::unlink_pair(&mut document, "another-pair").unwrap_err();

    assert!(error.contains("another-pair"), "{error}");
    assert_eq!(document, before);
}

#[test]
fn unlink_refuses_a_construction_source_on_the_wrong_board_atomically() {
    let mut document = nonempty_unlink_document();
    document.hardware.as_mut().unwrap().pairs[0].construction_source =
        Some("mirror-instance".into());
    let before = document.clone();

    let error = crate::mechanical::halves::unlink_pair(&mut document, "unlink-pair").unwrap_err();

    assert!(
        error.contains("sourceBoardId") || error.contains("source board"),
        "{error}"
    );
    assert_eq!(document, before);
}

fn exception_context() -> super::super::MembershipContext {
    use super::super::{
        FinalOwner, MatrixPairContext, MembershipContext, TransitionAssociation, TransitionState,
    };
    use crate::model::{MirroredCompanionMode, Pose2, Vec2};
    use std::collections::BTreeMap;

    let pair_right: MirroredHalves = serde_json::from_value(json!({
        "sourceBoardId":"source-board","mirrorBoardId":"mirror-board",
        "correspondences":[
            {"kind":"matrix","sourceId":"source-matrix","mirrorId":"mirror-matrix"},
            {"kind":"part","sourceId":"source-part","mirrorId":"mirror-part"},
            {"kind":"part","sourceId":"source-part/diode","mirrorId":"mirror-part/diode"},
            {"kind":"layout","sourceId":"source-layout","mirrorId":"mirror-layout"}
        ],
        "exceptions":[
            {"kind":"layout","mirrorId":"mirror-layout","state":"omitted"}
        ]
    }))
    .unwrap();
    MembershipContext {
        pair_id: "pair".into(),
        source_board_id: "source-board".into(),
        mirror_board_id: "mirror-board".into(),
        matrix_pair: Some(MatrixPairContext {
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }),
        mapping: Pose2 {
            at: Vec2 { x: 100.0, y: -30.0 },
            rotation: 90.0,
        },
        pair_right,
        pending_rejoins: BTreeMap::new(),
        pending_pair_transitions: BTreeMap::new(),
        pending_pair_baseline: None,
        associations: vec![
            TransitionAssociation {
                kind: crate::model::MirroredItemKind::Matrix,
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
                parent_source_id: None,
                parent_mirror_id: None,
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: None,
                raw_tweak: None,
                effective_mode: MirroredCompanionMode::LiteralMirror,
                effective_tweak: None,
            },
            TransitionAssociation {
                kind: crate::model::MirroredItemKind::Part,
                source_id: "source-part".into(),
                mirror_id: "mirror-part".into(),
                parent_source_id: Some("source-matrix".into()),
                parent_mirror_id: Some("mirror-matrix".into()),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                raw_tweak: Some(crate::model::MirroredCompanionTweak {
                    x: 0.25,
                    y: -0.5,
                    rotation: 15.0,
                }),
                effective_mode: MirroredCompanionMode::LiteralMirrorWithTweak,
                effective_tweak: Some(crate::model::MirroredCompanionTweak {
                    x: 0.25,
                    y: -0.5,
                    rotation: 15.0,
                }),
            },
            TransitionAssociation {
                kind: crate::model::MirroredItemKind::Part,
                source_id: "source-part/diode".into(),
                mirror_id: "mirror-part/diode".into(),
                parent_source_id: Some("source-part".into()),
                parent_mirror_id: Some("mirror-part".into()),
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: Some(MirroredCompanionMode::DuplicatePosition),
                raw_tweak: None,
                effective_mode: MirroredCompanionMode::DuplicatePosition,
                effective_tweak: None,
            },
            TransitionAssociation {
                kind: crate::model::MirroredItemKind::Layout,
                source_id: "source-layout".into(),
                mirror_id: "mirror-layout".into(),
                parent_source_id: Some("source-matrix".into()),
                parent_mirror_id: Some("mirror-matrix".into()),
                state: TransitionState::Omitted,
                explicit_state: true,
                raw_mode_override: None,
                raw_tweak: None,
                effective_mode: MirroredCompanionMode::LiteralMirror,
                effective_tweak: None,
            },
        ],
        owners: BTreeMap::<String, FinalOwner>::new(),
    }
}

fn root_exception_context(state: super::super::TransitionState) -> super::super::MembershipContext {
    let mut context = exception_context();
    let root = &mut context.associations[0];
    root.state = state;
    root.explicit_state = true;
    for inherited in &mut context.associations[1..3] {
        inherited.state = state;
        inherited.explicit_state = false;
    }
    context
        .pair_right
        .exceptions
        .push(crate::model::MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: crate::model::MirroredItemKind::Matrix,
            mirror_id: "mirror-matrix".into(),
            state: match state {
                super::super::TransitionState::Linked => {
                    unreachable!("linked roots have no exception")
                }
                super::super::TransitionState::Local => crate::model::MirroredExceptionState::Local,
                super::super::TransitionState::Omitted => {
                    crate::model::MirroredExceptionState::Omitted
                }
            },
            removed_copper: None,
        });
    context
}

#[test]
fn owner_rejoin_clears_only_its_exception_and_keeps_explicit_child_state_and_modes() {
    let context = root_exception_context(super::super::TransitionState::Omitted);
    let rejoined = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        super::super::TransitionState::Linked,
    )
    .expect("the exact owner can rejoin");

    assert_eq!(
        rejoined.associations[0].state,
        super::super::TransitionState::Linked
    );
    assert!(!rejoined.associations[0].explicit_state);
    assert_eq!(
        rejoined.associations[1].state,
        super::super::TransitionState::Linked
    );
    assert!(!rejoined.associations[1].explicit_state);
    assert_eq!(
        rejoined.associations[2].effective_mode,
        crate::model::MirroredCompanionMode::DuplicatePosition
    );
    assert_eq!(
        rejoined.associations[1].effective_tweak.as_ref().unwrap().x,
        0.25
    );
    assert_eq!(
        rejoined.associations[3].state,
        super::super::TransitionState::Omitted
    );
    assert!(rejoined.associations[3].explicit_state);
    assert_eq!(rejoined.pair_right, context.pair_right);
    assert_eq!(
        rejoined.pending_pair_baseline.as_ref(),
        Some(&context.pair_right),
        "the helper retains the exact ordered durable baseline until the kernel succeeds"
    );
    assert_eq!(
        rejoined.pending_pair_transitions.get("source-matrix"),
        Some(&super::super::TransitionState::Omitted)
    );
}

#[test]
fn owner_rejoin_from_local_restores_inherited_children_only() {
    let context = root_exception_context(super::super::TransitionState::Local);
    let rejoined = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        super::super::TransitionState::Linked,
    )
    .expect("the exact Local owner can rejoin");

    assert_eq!(
        rejoined.associations[0].state,
        super::super::TransitionState::Linked
    );
    assert!(!rejoined.associations[0].explicit_state);
    assert_eq!(
        rejoined.associations[1].state,
        super::super::TransitionState::Linked
    );
    assert!(!rejoined.associations[1].explicit_state);
    assert_eq!(
        rejoined.associations[2].state,
        super::super::TransitionState::Linked
    );
    assert!(!rejoined.associations[2].explicit_state);
    assert_eq!(
        rejoined.associations[3].state,
        super::super::TransitionState::Omitted
    );
    assert!(rejoined.associations[3].explicit_state);
    assert_eq!(rejoined.pair_right, context.pair_right);
    assert!(rejoined.pending_pair_baseline.is_some());
}

#[test]
fn owner_localization_protects_only_inherited_children_and_preserves_child_exceptions() {
    let context = exception_context();
    let localized = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        super::super::TransitionState::Local,
    )
    .expect("the exact linked owner can become local");

    assert_eq!(
        localized.associations[0].state,
        super::super::TransitionState::Local
    );
    assert!(localized.associations[0].explicit_state);
    assert_eq!(
        localized.associations[1].state,
        super::super::TransitionState::Local
    );
    assert!(!localized.associations[1].explicit_state);
    assert_eq!(
        localized.associations[2].state,
        super::super::TransitionState::Local
    );
    assert!(!localized.associations[2].explicit_state);
    assert_eq!(
        localized.associations[3].state,
        super::super::TransitionState::Omitted
    );
    assert!(localized.associations[3].explicit_state);
    assert_eq!(localized.pair_right, context.pair_right);
    assert_eq!(
        localized.pending_pair_transitions.get("source-matrix"),
        Some(&super::super::TransitionState::Linked)
    );
}

#[test]
fn owner_omission_removes_only_inherited_children_and_keeps_explicit_child_state() {
    let context = exception_context();
    let omitted = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        super::super::TransitionState::Omitted,
    )
    .expect("the exact linked owner can be omitted");

    assert_eq!(
        omitted.associations[0].state,
        super::super::TransitionState::Omitted
    );
    assert!(omitted.associations[0].explicit_state);
    assert_eq!(
        omitted.associations[1].state,
        super::super::TransitionState::Omitted
    );
    assert!(!omitted.associations[1].explicit_state);
    assert_eq!(
        omitted.associations[2].state,
        super::super::TransitionState::Omitted
    );
    assert!(!omitted.associations[2].explicit_state);
    assert_eq!(
        omitted.associations[1].effective_tweak.as_ref().unwrap().x,
        0.25
    );
    assert_eq!(
        omitted.associations[2].effective_mode,
        crate::model::MirroredCompanionMode::DuplicatePosition
    );
    assert_eq!(
        omitted.associations[3].state,
        super::super::TransitionState::Omitted
    );
    assert!(omitted.associations[3].explicit_state);
    assert_eq!(omitted.pair_right, context.pair_right);
    assert_eq!(
        omitted.pending_pair_transitions.get("source-matrix"),
        Some(&super::super::TransitionState::Linked)
    );
}

#[test]
fn owner_transition_refuses_ambiguous_identity_without_changing_full_context() {
    let mut context = exception_context();
    context.associations.push(context.associations[0].clone());
    let before = context.clone();

    let result = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        super::super::TransitionState::Local,
    );

    assert!(
        result.is_err(),
        "duplicate source/mirror identity must be refused"
    );
    assert_eq!(
        context, before,
        "the complete private context remains unchanged"
    );
}

#[test]
fn owner_transition_refuses_missing_or_mismatched_association_atomically() {
    for (source_id, mirror_id) in [
        ("missing-source", "mirror-matrix"),
        ("source-matrix", "foreign-mirror"),
    ] {
        let context = exception_context();
        let before = context.clone();
        let result = super::transition_exception_context(
            &context,
            crate::model::MirroredItemKind::Matrix,
            source_id,
            mirror_id,
            super::super::TransitionState::Local,
        );

        assert!(
            result.is_err(),
            "unbound identity {source_id:?} -> {mirror_id:?} must be refused"
        );
        assert_eq!(
            context, before,
            "failed transitions leave the full context unchanged"
        );
    }
}

#[test]
fn independent_part_exception_is_scoped_to_its_owned_companion() {
    let mut context = exception_context();
    for (id, owner) in [
        ("source-part", "source-board"),
        ("source-part/diode", "source-board"),
        ("mirror-part", "mirror-board"),
        ("mirror-part/diode", "mirror-board"),
    ] {
        context
            .owners
            .insert(id.into(), super::super::FinalOwner::Board(owner.into()));
    }

    let localized = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::Part,
        "source-part",
        "mirror-part",
        super::super::TransitionState::Local,
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
    assert_eq!(selected.state, super::super::TransitionState::Local);
    assert!(selected.explicit_state);
    assert_eq!(companion.state, super::super::TransitionState::Local);
    assert!(!companion.explicit_state);
    assert_eq!(
        companion.effective_mode,
        crate::model::MirroredCompanionMode::DuplicatePosition
    );
    assert_eq!(companion.effective_tweak, None);
    assert_eq!(
        sibling_exception.state,
        super::super::TransitionState::Omitted
    );
    assert!(sibling_exception.explicit_state);
    assert_eq!(
        localized.owners["mirror-part"],
        super::super::FinalOwner::Board("mirror-board".into())
    );
    assert_eq!(
        localized.owners["mirror-part/diode"],
        super::super::FinalOwner::Board("mirror-board".into())
    );
    assert_eq!(localized.pair_right, context.pair_right);
    assert_eq!(
        localized.pending_pair_transitions.get("source-part"),
        Some(&super::super::TransitionState::Linked)
    );
}

#[test]
fn embedded_circuit_owner_transitions_keep_explicit_child_and_unrelated_module_linked() {
    use super::super::TransitionState;

    let mut context = exception_context();
    let mut circuit = context.associations[0].clone();
    circuit.kind = crate::model::MirroredItemKind::EmbeddedCircuit;
    circuit.source_id = "source-circuit".into();
    circuit.mirror_id = "mirror-circuit".into();
    circuit.parent_source_id = None;
    circuit.parent_mirror_id = None;

    let mut member = context.associations[1].clone();
    member.kind = crate::model::MirroredItemKind::Part;
    member.source_id = "embedded/source-circuit/U1".into();
    member.mirror_id = "embedded/mirror-circuit/U1".into();
    member.parent_source_id = Some("source-circuit".into());
    member.parent_mirror_id = Some("mirror-circuit".into());

    let mut net = context.associations[2].clone();
    net.kind = crate::model::MirroredItemKind::Net;
    net.source_id = "embedded/source-circuit/N$1".into();
    net.mirror_id = "embedded/mirror-circuit/N$1".into();
    net.parent_source_id = Some("source-circuit".into());
    net.parent_mirror_id = Some("mirror-circuit".into());

    let mut explicit_net = context.associations[3].clone();
    explicit_net.kind = crate::model::MirroredItemKind::Net;
    explicit_net.source_id = "embedded/source-circuit/N$LOCAL".into();
    explicit_net.mirror_id = "embedded/mirror-circuit/N$LOCAL".into();
    explicit_net.parent_source_id = Some("source-circuit".into());
    explicit_net.parent_mirror_id = Some("mirror-circuit".into());
    explicit_net.state = TransitionState::Omitted;
    explicit_net.explicit_state = true;

    let mut unrelated_module = context.associations[0].clone();
    unrelated_module.kind = crate::model::MirroredItemKind::Module;
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

    let localized = super::transition_exception_context(
        &context,
        crate::model::MirroredItemKind::EmbeddedCircuit,
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

    let rejoined = super::transition_exception_context(
        &localized,
        crate::model::MirroredItemKind::EmbeddedCircuit,
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

fn local_transition_fixture() -> Result<(ProjectDoc, MembershipContext), String> {
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
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    source_matrix.rows = 2;
    source_matrix.columns = 4;
    source_matrix.pitch = Vec2 { x: 10.0, y: 20.0 };
    source_matrix.cells = matrix_cells();
    crate::matrix::set_matrix(&mut document, &source_matrix)?;

    let mut mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    mirror_matrix.rows = 2;
    mirror_matrix.columns = 4;
    mirror_matrix.pitch = source_matrix.pitch;
    mirror_matrix.cells = matrix_cells();
    let matrix_correspondence = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix)
        .unwrap()
        .clone();
    mirror_matrix = super::super::project_matrix(
        &source_matrix,
        &mirror_matrix,
        &matrix_correspondence,
        document.hardware.as_ref().unwrap().pairs[0].mapping,
    )?;
    crate::matrix::set_matrix(&mut document, &mirror_matrix)?;

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    crate::mechanical::halves::validate_document(&document)?;

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
            matrix_pair: Some(super::super::MatrixPairContext {
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            }),
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
    let local_context = super::transition_exception_context(
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
    let persisted_halves = before.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
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
    let local_key = post_source
        .parts
        .iter_mut()
        .find(|part| part.id == mirror_id)
        .unwrap();
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
    let mut expanded = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
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
    crate::matrix::set_matrix(&mut post_source, &expanded)
        .expect("the planned source additions materialize");

    let output = commit_matrix_step(&before, &post_source, &context)
        .expect("a complete planned expansion is projected atomically");
    assert_eq!(
        output
            .draft
            .parts
            .iter()
            .find(|part| part.id == mirror_id)
            .unwrap(),
        &local_key_snapshot,
        "an ordinary source expansion preserves the explicitly Local target payload"
    );
    let mirror = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
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
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap(),
        &output.context.pair_right
    );

    let rejoined_context = super::transition_exception_context(
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
    let current_source = rejoin_before
        .parts
        .iter()
        .find(|part| part.id == source_id)
        .unwrap();
    let expected_pose = Pose2 {
        at: crate::mechanical::pair::forward(rejoined_context.mapping, current_source.pose.at),
        rotation: crate::mechanical::pair::forward_angle(
            rejoined_context.mapping,
            current_source.pose.rotation,
        ),
    };
    let rejoined_part = rejoined
        .draft
        .parts
        .iter()
        .find(|part| part.id == mirror_id)
        .unwrap();
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

fn commit_matrix_step(
    before: &ProjectDoc,
    post_source: &ProjectDoc,
    context: &MembershipContext,
) -> Result<super::super::MembershipTransitionOutput, String> {
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
        let source_part = post_source.parts.iter().find(|p| p.id == *id).unwrap();
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

fn assert_committed_repeat(output: &super::super::MembershipTransitionOutput) {
    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
    assert_eq!(decoded, output.draft);
    crate::mechanical::halves::validate_document(&decoded).unwrap();
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
    let omitted_context = super::transition_exception_context(
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
    let mut source = post
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .clone();
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
    crate::matrix::set_matrix(&mut post, &source).unwrap();
    post.parts
        .iter_mut()
        .find(|p| p.id == source_id)
        .unwrap()
        .properties = Some(BTreeMap::from([(
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
    let rejoin_context = super::transition_exception_context(
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
    let source = expanded
        .draft
        .parts
        .iter()
        .find(|p| p.id == source_id)
        .unwrap();
    let target = rejoined
        .draft
        .parts
        .iter()
        .find(|p| p.id == mirror_id)
        .unwrap();
    let expected_at = crate::mechanical::pair::forward(context.mapping, source.pose.at);
    assert!((target.pose.at.x - expected_at.x).abs() < 1e-10);
    assert!((target.pose.at.y - expected_at.y).abs() < 1e-10);
    assert!(
        (target.pose.rotation
            - crate::mechanical::pair::forward_angle(context.mapping, source.pose.rotation))
        .abs()
            < 1e-10
    );
    assert_eq!(target.properties, source.properties);
    assert_eq!(target.generator_parameters, source.generator_parameters);
    assert_eq!(target.side, source.side);
    assert!(rejoined.draft.parts.iter().any(|p| p.id == child_id));
    assert!(rejoined.context.pair_right.exceptions.is_empty());
}

fn independent_lifecycle_fixture() -> (ProjectDoc, MembershipContext) {
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
                .iter_mut()
                .find(|b| b.id == board_id)
                .unwrap()
                .part_ids
                .push(id.clone());
            context
                .owners
                .insert(id, FinalOwner::Board(board_id.into()));
        }
        let association = super::super::TransitionAssociation {
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
    crate::mechanical::halves::validate_document(&document).unwrap();
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
    let local_context = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    let local = super::prepare_lifecycle_transition(&before, &before, &local_context)
        .expect("an independent screen can become Local without leaving the pair");
    let local_part = local
        .draft
        .parts
        .iter()
        .find(|p| p.id == "mirror-screen")
        .unwrap()
        .clone();
    let mut post = local.draft.clone();
    for source_id in ["source-screen", "source-encoder"] {
        let source = post.parts.iter_mut().find(|p| p.id == source_id).unwrap();
        source.pose.at.x += 17.0;
        source.properties = Some(BTreeMap::from([("revision".into(), json!(2))]));
    }
    let edited = super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == "mirror-screen"),
        Some(&local_part)
    );
    assert_eq!(
        edited
            .draft
            .parts
            .iter()
            .find(|p| p.id == "mirror-encoder")
            .unwrap()
            .properties,
        post.parts
            .iter()
            .find(|p| p.id == "source-encoder")
            .unwrap()
            .properties
    );
    let omitted_context = super::transition_exception_context(
        &edited.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted =
        super::prepare_lifecycle_transition(&edited.draft, &edited.draft, &omitted_context)
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
    crate::mechanical::halves::validate_document(&decoded).unwrap();
    let repeated =
        super::prepare_lifecycle_transition(&decoded, &decoded, &omitted.context).unwrap();
    assert_eq!(repeated.draft, decoded);
    assert_eq!(repeated.context, omitted.context);
    assert!(repeated.changed_document_ids.is_empty());
    let rejoin_context = super::transition_exception_context(
        &repeated.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined =
        super::prepare_lifecycle_transition(&repeated.draft, &repeated.draft, &rejoin_context)
            .unwrap();
    let source = rejoined
        .draft
        .parts
        .iter()
        .find(|p| p.id == "source-screen")
        .unwrap();
    let target = rejoined
        .draft
        .parts
        .iter()
        .find(|p| p.id == "mirror-screen")
        .unwrap();
    assert_eq!(target.properties, source.properties);
    let expected = crate::mirrored_half::derive::part(
        source,
        &target.id,
        &target.definition_id,
        &target.reference,
        context.mapping,
        crate::mirrored_half::derive::is_controller(
            &rejoined.draft.definitions,
            &source.definition_id,
        ),
    );
    assert_eq!(target, &expected);
    assert!(rejoined.context.pair_right.exceptions.is_empty());
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();

    // The same matrixless part lifecycle is reachable straight through the private intent route.
    let (mut matrixless, _) = independent_lifecycle_fixture();
    remove_matrix_correspondence(&mut matrixless);
    crate::mechanical::halves::validate_document(&matrixless).unwrap();
    let pair_id = matrixless.hardware.as_ref().unwrap().pairs[0].id.clone();
    let mut prepared = super::prepare_exception_intent(
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
        let prepared = super::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            "source-screen",
            "mirror-screen",
            state,
        )
        .unwrap();
        let committed = super::prepare_lifecycle_transition(&before, &before, &prepared).unwrap();
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
        crate::matrix::deletion::remove_parts_by_id(
            &mut post,
            &BTreeSet::from(["source-screen".into()]),
        );
        let removed =
            super::prepare_lifecycle_transition(&committed.draft, &post, &committed.context)
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
        crate::mechanical::halves::validate_document(&removed.draft).unwrap();
        let repeat =
            super::prepare_lifecycle_transition(&removed.draft, &removed.draft, &removed.context)
                .unwrap();
        assert_eq!(repeat.draft, removed.draft);
        assert_eq!(repeat.context, removed.context);
        assert!(repeat.changed_document_ids.is_empty());
    }
}

#[test]
fn lifecycle_refuses_forged_independent_ownership_without_mutating_inputs() {
    let (before, context) = independent_lifecycle_fixture();
    let mut pending = super::transition_exception_context(
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
    let error = super::prepare_lifecycle_transition(&before, &before, &pending)
        .expect_err("an independent encoder is not owned by a screen");
    assert!(error.contains("ownership"), "{error}");
    assert_eq!((before, pending), frozen);
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
    let child_context = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        child_source,
        child_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let child = super::prepare_lifecycle_transition(&before, &before, &child_context).unwrap();
    let explicit_part = child
        .draft
        .parts
        .iter()
        .find(|p| p.id == child_mirror)
        .unwrap()
        .clone();
    let owner_context = super::transition_exception_context(
        &child.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Local,
    )
    .unwrap();
    let local = super::prepare_lifecycle_transition(&child.draft, &child.draft, &owner_context)
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
    crate::matrix::set_matrix(&mut post, &source).unwrap();
    let edited = super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
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
    let omitted_context = super::transition_exception_context(
        &edited.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted =
        super::prepare_lifecycle_transition(&edited.draft, &edited.draft, &omitted_context)
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
    crate::mechanical::halves::validate_document(&decoded).unwrap();
    let repeat = super::prepare_lifecycle_transition(&decoded, &decoded, &omitted.context).unwrap();
    assert_eq!(repeat.draft, decoded);
    assert_eq!(repeat.context, omitted.context);
    let rejoin_context = super::transition_exception_context(
        &repeat.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined =
        super::prepare_lifecycle_transition(&repeat.draft, &repeat.draft, &rejoin_context).unwrap();
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
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();
    let repeat =
        super::prepare_lifecycle_transition(&rejoined.draft, &rejoined.draft, &rejoined.context)
            .unwrap();
    assert_eq!(repeat.draft, rejoined.draft);
    assert_eq!(repeat.context, rejoined.context);
    assert!(repeat.changed_document_ids.is_empty());
}

fn add_lifecycle_association(
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

fn circuit_lifecycle_fixture() -> (ProjectDoc, MembershipContext) {
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
    crate::mechanical::halves::validate_document(&document).unwrap();
    (document, context)
}

fn remove_matrix_correspondence(document: &mut ProjectDoc) {
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

    let stale = super::prepare_exception_intent(
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

    let matrix_owned = super::prepare_exception_intent(
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
    let error = super::prepare_exception_intent(
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
    let error = super::prepare_exception_intent(
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
    let (before, context) = circuit_lifecycle_fixture();
    let child_context = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "embedded/source-circuit/U2",
        "embedded/mirror-circuit/U2",
        TransitionState::Local,
    )
    .unwrap();
    let child = super::prepare_lifecycle_transition(&before, &before, &child_context).unwrap();
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
    let owner_context = super::transition_exception_context(
        &child.context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Local,
    )
    .unwrap();
    let local =
        super::prepare_lifecycle_transition(&child.draft, &child.draft, &owner_context).unwrap();
    let mut post = local.draft.clone();
    let source = post
        .parts
        .iter_mut()
        .find(|p| p.id == "embedded/source-circuit/U1")
        .unwrap();
    source.pose.at.x += 19.0;
    source.properties = Some(BTreeMap::from([("revision".into(), json!(3))]));
    let edited = super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == inherited.id),
        Some(&inherited)
    );
    let omitted_context = super::transition_exception_context(
        &edited.context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted =
        super::prepare_lifecycle_transition(&edited.draft, &edited.draft, &omitted_context).expect(
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
    crate::mechanical::halves::validate_document(&decoded).unwrap();
    let repeat = super::prepare_lifecycle_transition(&decoded, &decoded, &omitted.context).unwrap();
    assert_eq!(repeat.draft, decoded);
    assert_eq!(repeat.context, omitted.context);
    let rejoin_context = super::transition_exception_context(
        &repeat.context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined =
        super::prepare_lifecycle_transition(&repeat.draft, &repeat.draft, &rejoin_context).unwrap();
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
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();
    let repeat =
        super::prepare_lifecycle_transition(&rejoined.draft, &rejoined.draft, &rejoined.context)
            .unwrap();
    assert_eq!(repeat.draft, rejoined.draft);
    assert_eq!(repeat.context, rejoined.context);
    assert!(repeat.changed_document_ids.is_empty());

    // The same circuit lifecycle holds with no Matrix correspondences (ticket 150).
    let (mut matrixless, _) = circuit_lifecycle_fixture();
    remove_matrix_correspondence(&mut matrixless);
    crate::mechanical::halves::validate_document(&matrixless).unwrap();
    let pair_id = matrixless.hardware.as_ref().unwrap().pairs[0].id.clone();
    let child = super::prepare_exception_intent(
        &matrixless,
        &matrixless,
        &pair_id,
        MirroredItemKind::Part,
        "embedded/source-circuit/U2",
        "embedded/mirror-circuit/U2",
        TransitionState::Local,
    )
    .unwrap()
    .pop()
    .unwrap();
    let child_geometry = child
        .draft
        .parts
        .iter()
        .find(|p| p.id == "embedded/mirror-circuit/U2")
        .unwrap()
        .clone();
    let owner = super::prepare_exception_intent(
        &child.draft,
        &child.draft,
        &pair_id,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Local,
    )
    .unwrap()
    .pop()
    .unwrap();
    let rejoined = super::prepare_exception_intent(
        &owner.draft,
        &owner.draft,
        &pair_id,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Linked,
    )
    .unwrap()
    .pop()
    .unwrap();
    assert_eq!(
        rejoined
            .draft
            .parts
            .iter()
            .find(|p| p.id == "embedded/mirror-circuit/U2"),
        Some(&child_geometry),
        "matrixless Rejoin keeps the explicit child's Local geometry"
    );
    assert_eq!(rejoined.context.pair_right.exceptions.len(), 1);
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();
}

fn named_companion_fixture(component: &str) -> (ProjectDoc, MembershipContext) {
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
        let local_context = super::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Local,
        )
        .unwrap();
        let local = super::prepare_lifecycle_transition(&before, &before, &local_context)
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
        crate::matrix::set_matrix(&mut post, &source).unwrap();
        let edited =
            super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
        assert_eq!(
            edited.draft.parts.iter().find(|p| p.id == mirror_id),
            Some(&retained)
        );
        let omitted_context = super::transition_exception_context(
            &edited.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Omitted,
        )
        .unwrap();
        let omitted =
            super::prepare_lifecycle_transition(&edited.draft, &edited.draft, &omitted_context)
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
        let repeat =
            super::prepare_lifecycle_transition(&omitted.draft, &omitted.draft, &omitted.context)
                .unwrap();
        assert_eq!(repeat.draft, omitted.draft);
        let parent_context = super::transition_exception_context(
            &repeat.context,
            MirroredItemKind::Part,
            "matrix/source-matrix/r0c1",
            "matrix/mirror-matrix/r0c1",
            TransitionState::Local,
        )
        .unwrap();
        let parent_local =
            super::prepare_lifecycle_transition(&repeat.draft, &repeat.draft, &parent_context)
                .unwrap();
        let rejoin_parent = super::transition_exception_context(
            &parent_local.context,
            MirroredItemKind::Part,
            "matrix/source-matrix/r0c1",
            "matrix/mirror-matrix/r0c1",
            TransitionState::Linked,
        )
        .unwrap();
        let parent_linked = super::prepare_lifecycle_transition(
            &parent_local.draft,
            &parent_local.draft,
            &rejoin_parent,
        )
        .unwrap();
        assert!(parent_linked.draft.parts.iter().all(|p| p.id != mirror_id));
        assert_eq!(parent_linked.context.pair_right.exceptions.len(), 1);
        let rejoin = super::transition_exception_context(
            &parent_linked.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined = super::prepare_lifecycle_transition(
            &parent_linked.draft,
            &parent_linked.draft,
            &rejoin,
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
        let repeat = super::prepare_lifecycle_transition(
            &rejoined.draft,
            &rejoined.draft,
            &rejoined.context,
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
    let pending = super::transition_exception_context(
        &context,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
        TransitionState::Omitted,
    )
    .unwrap();
    let frozen = (before.clone(), pending.clone());
    let error = super::prepare_lifecycle_transition(&before, &before, &pending)
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
        crate::mechanical::halves::validate_document(&document).is_err(),
        "an Omitted target cannot alias live geometry on another board"
    );
}

#[test]
fn pending_baseline_and_selection_tampering_refuse_atomically() {
    let (before, context) = independent_lifecycle_fixture();
    let pending = super::transition_exception_context(
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
            super::prepare_lifecycle_transition(&before, &before, &tampered).is_err(),
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
    let output = super::prepare_lifecycle_transition(&document, &document, &context).unwrap();
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
        super::transition_exception_context(
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
                super::transition_exception_context(&context, kind, source_id, mirror_id, state)
                    .unwrap();
            let committed =
                super::prepare_lifecycle_transition(&before, &before, &pending).unwrap();
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
                crate::apply(
                    &mut post,
                    &[],
                    &EditOperation::RemoveMatrix {
                        id: source_id.into(),
                    },
                )
                .unwrap();
            } else {
                crate::modules::remove_circuit(&mut post, source_id).unwrap();
            }
            let removed =
                super::prepare_lifecycle_transition(&committed.draft, &post, &committed.context)
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
            crate::mechanical::halves::validate_document(&removed.draft).unwrap();
            let repeated = super::prepare_lifecycle_transition(
                &removed.draft,
                &removed.draft,
                &removed.context,
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
            let edited = super::prepare_lifecycle_transition(
                &repeated.draft,
                &edited_source,
                &repeated.context,
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

#[test]
fn matrix_owner_rejoin_projects_members_added_while_local() {
    let (before, mut context) = independent_lifecycle_fixture();
    for a in context.associations.iter_mut().filter(|a| {
        a.kind == MirroredItemKind::Part
            && a.source_id.starts_with("matrix/")
            && a.parent_source_id.is_none()
    }) {
        a.parent_source_id = Some("source-matrix".into());
        a.parent_mirror_id = Some("mirror-matrix".into());
    }
    let pending = super::transition_exception_context(
        &context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Local,
    )
    .unwrap();
    let local = super::prepare_lifecycle_transition(&before, &before, &pending).unwrap();
    let target = local
        .draft
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .clone();
    let mut post = local.draft.clone();
    let mut source = post
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .clone();
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
    crate::matrix::set_matrix(&mut post, &source).unwrap();
    post.parts
        .iter_mut()
        .find(|p| p.id == "matrix/source-matrix/r0c4")
        .unwrap()
        .properties = Some(BTreeMap::from([("new-source".into(), json!(true))]));
    let expanded =
        super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
    assert_eq!(
        expanded
            .draft
            .matrices
            .iter()
            .find(|m| m.id == "mirror-matrix"),
        Some(&target)
    );
    let rejoin = super::transition_exception_context(
        &expanded.context,
        MirroredItemKind::Matrix,
        "source-matrix",
        "mirror-matrix",
        TransitionState::Linked,
    )
    .unwrap();
    let output = super::prepare_lifecycle_transition(&expanded.draft, &expanded.draft, &rejoin)
        .expect(
            "an explicit owner Rejoin projects the current source, including new owned members",
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
                output
                    .draft
                    .parts
                    .iter()
                    .find(|p| p.id == mirror_id)
                    .unwrap()
                    .properties,
                output
                    .draft
                    .parts
                    .iter()
                    .find(|p| p.id == source_id)
                    .unwrap()
                    .properties
            );
        }
    }
    crate::mechanical::halves::validate_document(&output.draft).unwrap();
    let repeat =
        super::prepare_lifecycle_transition(&output.draft, &output.draft, &output.context).unwrap();
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
        let pending = super::transition_exception_context(
            &context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            owner_state,
        )
        .unwrap();
        let owner = super::prepare_lifecycle_transition(&before, &before, &pending).unwrap();
        let visible = owner
            .draft
            .parts
            .iter()
            .find(|p| p.id == mirror_id)
            .cloned();
        let selected = super::transition_exception_context(
            &owner.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            child_state,
        )
        .expect(
            "a child selection authenticates its own durable row, separately from inherited state",
        );
        let child =
            super::prepare_lifecycle_transition(&owner.draft, &owner.draft, &selected).unwrap();
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
        let pending = super::transition_exception_context(
            &child.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined =
            super::prepare_lifecycle_transition(&child.draft, &child.draft, &pending).unwrap();
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
        let repeat = super::prepare_lifecycle_transition(
            &rejoined.draft,
            &rejoined.draft,
            &rejoined.context,
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
        let pending = super::transition_exception_context(
            &context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Local,
        )
        .unwrap();
        let child = super::prepare_lifecycle_transition(&before, &before, &pending).unwrap();
        let visible = child
            .draft
            .parts
            .iter()
            .find(|p| p.id == mirror_id)
            .unwrap()
            .clone();
        let pending = super::transition_exception_context(
            &child.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            owner_state,
        )
        .unwrap();
        let owner =
            super::prepare_lifecycle_transition(&child.draft, &child.draft, &pending).unwrap();
        let pending = super::transition_exception_context(
            &owner.context,
            MirroredItemKind::Part,
            source_id,
            mirror_id,
            TransitionState::Linked,
        )
        .unwrap();
        let inherited = super::prepare_lifecycle_transition(&owner.draft, &owner.draft, &pending)
            .expect(
                "clearing a child exception resumes inheritance even when the owner is unlinked",
            );
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
        let repeat = super::prepare_lifecycle_transition(
            &inherited.draft,
            &inherited.draft,
            &inherited.context,
        )
        .unwrap();
        assert_eq!(repeat.draft, inherited.draft);
        assert_eq!(repeat.context, inherited.context);
        let mut post = inherited.draft.clone();
        post.parts
            .iter_mut()
            .find(|p| p.id == source_id)
            .unwrap()
            .properties = Some(BTreeMap::from([("current".into(), json!(true))]));
        let pending = super::transition_exception_context(
            &inherited.context,
            MirroredItemKind::Matrix,
            "source-matrix",
            "mirror-matrix",
            TransitionState::Linked,
        )
        .unwrap();
        let rejoined =
            super::prepare_lifecycle_transition(&inherited.draft, &post, &pending).unwrap();
        assert!(rejoined.context.pair_right.exceptions.is_empty());
        assert_eq!(
            rejoined
                .draft
                .parts
                .iter()
                .find(|p| p.id == mirror_id)
                .unwrap()
                .properties,
            post.parts
                .iter()
                .find(|p| p.id == source_id)
                .unwrap()
                .properties
        );
    }
}

#[test]
fn independent_connector_exceptions_preserve_linked_modules_and_clean_only_their_reference() {
    let (before, context) = circuit_lifecycle_fixture();
    let pending = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();
    let local = super::prepare_lifecycle_transition(&before, &before, &pending)
        .expect("a Local connector remains a valid reference for a linked module");
    let connector = local
        .draft
        .parts
        .iter()
        .find(|p| p.id == "mirror-screen")
        .unwrap()
        .clone();
    let module = local
        .draft
        .modules
        .iter()
        .find(|m| m.id == "mirror-module")
        .unwrap()
        .clone();
    let mut post = local.draft.clone();
    post.parts
        .iter_mut()
        .find(|p| p.id == "source-screen")
        .unwrap()
        .pose
        .at
        .x += 30.0;
    post.modules
        .iter_mut()
        .find(|m| m.id == "source-module")
        .unwrap()
        .gap += 1.0;
    let edited = super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
    assert_eq!(
        edited.draft.parts.iter().find(|p| p.id == "mirror-screen"),
        Some(&connector)
    );
    let edited_module = edited
        .draft
        .modules
        .iter()
        .find(|m| m.id == "mirror-module")
        .unwrap();
    assert_eq!(edited_module.gap, module.gap + 1.0);
    assert_eq!(
        edited_module
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        "mirror-screen"
    );
    let pending = super::transition_exception_context(
        &edited.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted = super::prepare_lifecycle_transition(&edited.draft, &edited.draft, &pending)
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
    let repeat =
        super::prepare_lifecycle_transition(&omitted.draft, &omitted.draft, &omitted.context)
            .unwrap();
    assert_eq!(repeat.draft, omitted.draft);
    let pending = super::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined =
        super::prepare_lifecycle_transition(&omitted.draft, &omitted.draft, &pending).unwrap();
    assert_eq!(
        rejoined
            .draft
            .modules
            .iter()
            .find(|m| m.id == "mirror-module"),
        Some(edited_module)
    );
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();
}

fn electrical_definition(id: &str, pads: &[(&str, f64, f64)]) -> PartDefinition {
    serde_json::from_value(json!({
        "id": id, "name": id, "kind": "custom", "courtyard": [],
        "pads": pads.iter().map(|(pad, x, y)| json!({
            "id": pad, "number": pad, "at": {"x": x, "y": y},
            "size": {"x": 1, "y": 1}, "shape": "circle"
        })).collect::<Vec<_>>()
    }))
    .unwrap()
}

fn pad_world(document: &ProjectDoc, part_id: &str, pad_id: &str) -> Vec2 {
    let part = document.parts.iter().find(|p| p.id == part_id).unwrap();
    let definition = document
        .definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .unwrap();
    let pad = definition.pads.iter().find(|p| p.id == pad_id).unwrap();
    crate::outline_controls::world(pad.at, part)
}

fn mirror_electrical(document: &ProjectDoc) -> &ElectricalBoardConfiguration {
    document
        .hardware
        .as_ref()
        .unwrap()
        .boards
        .iter()
        .find(|b| b.board_id == "mirror-board")
        .unwrap()
}

fn source_electrical(document: &ProjectDoc) -> &ElectricalBoardConfiguration {
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
fn electrical_copper_fixture() -> (ProjectDoc, MembershipContext) {
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
            .iter_mut()
            .find(|b| b.id == board_id)
            .unwrap()
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
        *target = crate::mirrored_half::derive::part(
            &source,
            &target.id,
            &target.definition_id,
            &target.reference,
            context.mapping,
            crate::mirrored_half::derive::is_controller(
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
            document
                .boards
                .iter_mut()
                .find(|b| b.id == board_id)
                .unwrap()
                .net_ids
                .push(id);
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
    let board = document
        .boards
        .iter_mut()
        .find(|b| b.id == "mirror-board")
        .unwrap();
    (board.traces, board.vias) = copper;
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(context.pair_right.clone());
    crate::mechanical::halves::validate_document(&document).unwrap();
    (document, context)
}

fn omit_screen(
    before: &ProjectDoc,
    context: &MembershipContext,
) -> super::super::MembershipTransitionOutput {
    let pending = super::transition_exception_context(
        context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    )
    .unwrap();
    super::prepare_lifecycle_transition(before, before, &pending).expect("Omit commits")
}

fn rejoin_screen(
    draft: &ProjectDoc,
    post: &ProjectDoc,
    context: &MembershipContext,
) -> super::super::MembershipTransitionOutput {
    let pending = super::transition_exception_context(
        context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    super::prepare_lifecycle_transition(draft, post, &pending).expect("Rejoin commits")
}

fn pin_values(config: &ElectricalBoardConfiguration) -> BTreeSet<String> {
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
    let local_controller = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let localized = super::prepare_lifecycle_transition(&before, &before, &local_controller)
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
    let mirror_board = omitted
        .draft
        .boards
        .iter()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    assert!(mirror_board.net_ids.contains(&"mirror-sda".into()));
    crate::mechanical::halves::validate_document(&omitted.draft).unwrap();

    let decoded: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&omitted.draft).unwrap()).unwrap();
    let repeated =
        super::prepare_lifecycle_transition(&decoded, &decoded, &omitted.context).unwrap();
    assert_eq!(repeated.draft, decoded);
    let mut reopened = crate::CoreEngine::default();
    assert!(matches!(
        reopened.handle(crate::model::CoreRequest::Open {
            id: "reopen-local-controller-net".into(),
            document: repeated.draft,
        }),
        crate::model::CoreReply::Scene { .. }
    ));
    assert!(
        reopened
            .document
            .nets
            .iter()
            .any(|net| net.id == "mirror-sda")
    );
    assert!(
        reopened
            .document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .contains(&"mirror-sda".into())
    );
}

#[test]
fn malformed_local_controller_ownership_refuses_omit_atomically() {
    let (before, context) = electrical_copper_fixture();
    let local_controller = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let localized = super::prepare_lifecycle_transition(&before, &before, &local_controller)
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
    let pending = super::transition_exception_context(
        &malformed,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Omitted,
    );
    let error = match pending {
        Ok(pending) => {
            let frozen = (localized.draft.clone(), malformed.clone(), pending.clone());
            let result =
                super::prepare_lifecycle_transition(&localized.draft, &localized.draft, &pending);
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
    crate::mechanical::halves::validate_document(&omitted.draft).unwrap();

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
    let board = draft
        .boards
        .iter()
        .find(|b| b.id == "mirror-board")
        .unwrap();
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
    let source_board = |d: &ProjectDoc| {
        d.boards
            .iter()
            .find(|b| b.id == "source-board")
            .unwrap()
            .clone()
    };
    assert_eq!(source_board(draft), source_board(&before));
    for net in ["source-sda", "source-scl", "source-gnd"] {
        assert_eq!(
            draft.nets.iter().find(|n| n.id == net),
            before.nets.iter().find(|n| n.id == net)
        );
    }
    let decoded: ProjectDoc = serde_json::from_slice(&serde_json::to_vec(draft).unwrap()).unwrap();
    crate::mechanical::halves::validate_document(&decoded).unwrap();
    let repeat = super::prepare_lifecycle_transition(&decoded, &decoded, &omitted.context).unwrap();
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
    let board = |d: &ProjectDoc| {
        d.boards
            .iter()
            .find(|b| b.id == "mirror-board")
            .unwrap()
            .clone()
    };
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
        let restored = rejoined.draft.nets.iter().find(|n| n.id == net).unwrap();
        let original = before.nets.iter().find(|n| n.id == net).unwrap();
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
    crate::mechanical::halves::validate_document(&rejoined.draft).unwrap();

    // The source screen moved while the mirror was omitted: its pads no longer match.
    let mut post = omitted.draft.clone();
    post.parts
        .iter_mut()
        .find(|p| p.id == "source-screen")
        .unwrap()
        .pose
        .at
        .x += 4.0;
    let moved = rejoin_screen(&omitted.draft, &post, &omitted.context);
    let board = moved
        .draft
        .boards
        .iter()
        .find(|b| b.id == "mirror-board")
        .unwrap();
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
    crate::mechanical::halves::validate_document(&moved.draft).unwrap();
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
    let stable =
        super::prepare_lifecycle_transition(&replaced, &replaced, &omitted.context).unwrap();
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
    let pending = super::transition_exception_context(
        &context,
        MirroredItemKind::Part,
        "source-mcu",
        "mirror-mcu",
        TransitionState::Local,
    )
    .unwrap();
    let local = super::prepare_lifecycle_transition(&before, &before, &pending).unwrap();
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
    let edited = super::prepare_lifecycle_transition(&local.draft, &post, &local.context).unwrap();
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
    let pending = super::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Linked,
    )
    .unwrap();
    assert!(super::prepare_lifecycle_transition(&occupied, &occupied, &pending).is_err());
    assert_eq!(frozen, (occupied, omitted.context));
}

fn add_second_matrix_pair(
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
    crate::matrix::set_matrix(document, &source)?;

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
    mirror = super::super::project_matrix(&source, &mirror, &correspondence, context.mapping)?;
    crate::matrix::set_matrix(document, &mirror)?;

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
    crate::mechanical::halves::validate_document(document)?;
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

        let local = super::prepare_exception_intent(
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
        let edited =
            super::prepare_lifecycle_transition(&local.draft, &edited_source, &local.context)
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
            &crate::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                fixture_context.mapping,
                crate::mirrored_half::derive::is_controller(
                    &edited_source.definitions,
                    &source.definition_id,
                ),
            )
        );
        assert_eq!(edited.draft.matrices, matrices_before);

        let omitted = super::prepare_exception_intent(
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
        let rejoined = super::prepare_exception_intent(
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
            &crate::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                fixture_context.mapping,
                crate::mirrored_half::derive::is_controller(
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

    let local = super::prepare_exception_intent(
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
    let omitted = super::prepare_exception_intent(
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
    let rejoined = super::prepare_exception_intent(
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
    let independent = super::lifecycle_context(&before, &pair_id, None).unwrap();
    let pending = super::transition_exception_context(
        &independent,
        MirroredItemKind::Part,
        "source-screen",
        "mirror-screen",
        TransitionState::Local,
    )
    .unwrap();

    let output = super::prepare_lifecycle_transition(&before, &before, &pending)
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
    let first_context = super::lifecycle_context(
        &before,
        &pair_id,
        Some(super::super::MatrixPairContext {
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }),
    )
    .unwrap();
    let first_pending = super::transition_exception_context(
        &first_context,
        MirroredItemKind::Part,
        first_source,
        first_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let first_local = super::prepare_lifecycle_transition(&before, &before, &first_pending)
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
    let second_context = super::lifecycle_context(
        &first_local.draft,
        &pair_id,
        Some(super::super::MatrixPairContext {
            source_id: "source-matrix-2".into(),
            mirror_id: "mirror-matrix-2".into(),
        }),
    )
    .unwrap();
    let local_context = super::transition_exception_context(
        &second_context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let local =
        super::prepare_lifecycle_transition(&first_local.draft, &first_local.draft, &local_context)
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

    let omit_context = super::transition_exception_context(
        &local.context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Omitted,
    )
    .unwrap();
    let omitted = super::prepare_lifecycle_transition(&local.draft, &local.draft, &omit_context)
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

    let rejoin_context = super::transition_exception_context(
        &omitted.context,
        MirroredItemKind::Part,
        second_source,
        second_mirror,
        TransitionState::Linked,
    )
    .unwrap();
    let rejoined =
        super::prepare_lifecycle_transition(&omitted.draft, &omitted.draft, &rejoin_context)
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

    let error = super::matrix_pair_for_association(
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
    let context = super::lifecycle_context(&document, &pair_id, None).unwrap();
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

    let error = super::validate_lifecycle_context(&wrong_membership, &context)
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
    let first_owner = super::lifecycle_context(
        &before,
        &pair_id,
        Some(super::super::MatrixPairContext {
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        }),
    )
    .unwrap();
    let second_child = "matrix/source-matrix-2/r0c0";
    let second_mirror = "matrix/mirror-matrix-2/r0c0";
    let frozen = before.clone();
    let mut wrong_owner = super::transition_exception_context(
        &first_owner,
        MirroredItemKind::Part,
        second_child,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    wrong_owner.matrix_pair = Some(super::super::MatrixPairContext {
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
    });
    let wrong_owner_error = super::prepare_lifecycle_transition(&before, &before, &wrong_owner)
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
    let exact_second = super::lifecycle_context(
        &before,
        &pair_id,
        Some(super::super::MatrixPairContext {
            source_id: "source-matrix-2".into(),
            mirror_id: "mirror-matrix-2".into(),
        }),
    )
    .unwrap();
    let pending = super::transition_exception_context(
        &exact_second,
        MirroredItemKind::Part,
        second_child,
        second_mirror,
        TransitionState::Local,
    )
    .unwrap();
    let membership_error =
        super::prepare_lifecycle_transition(&wrong_membership, &wrong_membership, &pending)
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
fn replacement_retirement_fixture() -> ProjectDoc {
    use crate::model::MirroredItemKind as Kind;

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

fn remove_replacement_retirement_sources(mut replacement: ProjectDoc) -> ProjectDoc {
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
    crate::mechanical::halves::validate_document(&accepted).unwrap();
    let mut replacement = remove_replacement_retirement_sources(accepted.clone());

    retire_replacement_records(&accepted, &mut replacement).unwrap();

    let halves = replacement.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
    assert!(crate::mechanical::halves::validate_document(&replacement).is_ok());
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
    assert!(crate::mechanical::halves::validate_document(&accepted).is_ok());
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
    use crate::model::MirroredItemKind;

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
    assert!(crate::mechanical::halves::validate_document(&accepted).is_ok());
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
