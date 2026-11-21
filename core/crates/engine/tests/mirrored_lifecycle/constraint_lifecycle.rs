use super::*;

pub(crate) fn omitted_halves() -> Value {
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

pub(crate) fn board(id: &str, parts: &[&str]) -> Board {
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

pub(crate) fn part(id: &str) -> Part {
    serde_json::from_value(json!({
        "id": id,
        "definitionId": "test-footprint",
        "reference": "U1",
        "pose": { "at": { "x": 12.0, "y": 3.0 }, "rotation": 0.0 },
        "side": "front"
    }))
    .unwrap()
}

pub(crate) fn valid_omitted_document() -> ProjectDoc {
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
        instances: vec![
            base_fixtures::fixture_instance("source-instance", "source-board"),
            base_fixtures::fixture_instance("mirror-instance", "mirror-board"),
        ],
        ..HardwareConfiguration::default()
    });
    for instance in &mut document.hardware.as_mut().unwrap().instances {
        instance.construction_ownership = MechanicalConstructionOwnership::Pair;
    }
    document
}

#[test]
fn valid_durable_omission_accepts_an_absent_mirror_part() {
    let document = valid_omitted_document();
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(&document).is_ok(),
        "a source-bound Omitted record remains valid after its target geometry is removed"
    );
}

#[test]
fn omitted_absence_and_source_identity_survive_document_round_trip() {
    let document = valid_omitted_document();
    let encoded = serde_json::to_vec(&document).unwrap();
    let decoded: ProjectDoc = serde_json::from_slice(&encoded).unwrap();

    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&decoded).is_ok());
    assert!(decoded.parts.iter().all(|part| part.id != "mirror-part"));
    assert!(
        decoded
            .boards
            .iter()
            .all(|board| !board.part_ids.iter().any(|id| id == "mirror-part"))
    );
    let halves = decoded.first_halves();
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

pub(crate) fn nonempty_unlink_document() -> ProjectDoc {
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

pub(crate) fn expected_closure_ids(board_id: &str) -> BTreeSet<String> {
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

pub(crate) fn unlink_diff_ids(before: &ProjectDoc, after: &ProjectDoc) -> BTreeSet<String> {
    fn rows(doc: &ProjectDoc) -> std::collections::BTreeMap<(String, String), serde_json::Value> {
        let mut result = std::collections::BTreeMap::new();
        let doc = serde_json::to_value(doc).unwrap();
        for field in [
            "parts",
            "definitions",
            "boards",
            "matrices",
            "nets",
            "constraints",
            "outline",
            "layouts",
            "modules",
            "embeddedCircuits",
        ] {
            for row in doc[field].as_array().into_iter().flatten() {
                let id = row["id"].as_str().unwrap();
                result.insert((field.into(), id.into()), row.clone());
            }
        }
        result
    }
    let before_rows = rows(before);
    let after_rows = rows(after);
    let mut ids: Vec<_> = before_rows
        .keys()
        .chain(after_rows.keys())
        .filter(|key| before_rows.get(*key) != after_rows.get(*key))
        .map(|(_, id)| id.clone())
        .collect();

    let mut expected = before.clone();
    let hardware = expected.hardware.as_mut().unwrap();
    let updated = after.hardware.as_ref().unwrap();
    hardware.pairs.clone_from(&updated.pairs);
    hardware.instances.clone_from(&updated.instances);
    // Identified rows may change (for example, newly materialized closure mounts).
    // The returned IDs below account for those changes and for the changed instances.
    expected.parts.clone_from(&after.parts);
    expected.definitions.clone_from(&after.definitions);
    expected.boards.clone_from(&after.boards);
    expected.matrices.clone_from(&after.matrices);
    expected.nets.clone_from(&after.nets);
    expected.constraints.clone_from(&after.constraints);
    expected.outline.clone_from(&after.outline);
    for (before_empty, after_empty) in [
        (before.layouts.is_empty(), after.layouts.is_empty()),
        (before.modules.is_empty(), after.modules.is_empty()),
        (
            before.embedded_circuits.is_empty(),
            after.embedded_circuits.is_empty(),
        ),
    ] {
        assert_eq!(before_empty, after_empty);
    }
    expected.layouts.clone_from(&after.layouts);
    expected.modules.clone_from(&after.modules);
    expected
        .embedded_circuits
        .clone_from(&after.embedded_circuits);
    assert!(expected == *after, "Unlink changes only identified rows");

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

pub(crate) fn reuse_board_unlink_document() -> ProjectDoc {
    let mut document = nonempty_unlink_document();
    document.boards.pop();
    let hardware = document.hardware.as_mut().unwrap();
    hardware.instances[1].board_id = "source-board".into();
    let pair = &mut hardware.pairs[0];
    pair.mapping = boardstudio_model::model::Pose2 {
        at: boardstudio_model::model::Vec2::default(),
        rotation: 0.0,
    };
    let halves = pair.halves.as_mut().unwrap();
    halves.mirror_board_id = "source-board".into();
    document
}

#[test]
fn unlink_uses_the_exact_pair_and_materializes_nonempty_owned_closures() {
    let mut document = nonempty_unlink_document();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    let before = document.clone();
    let before_effective: Vec<_> = ["source-instance", "mirror-instance"]
        .into_iter()
        .map(|instance_id| {
            boardstudio_core_mechanical::mechanical::inputs::effective_inputs(
                &before,
                &[],
                &boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
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
    let changed =
        boardstudio_core_mechanical::mechanical::halves::unlink_pair(&mut document, "unlink-pair")
            .unwrap();

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
            boardstudio_model::model::MechanicalConstructionOwnership::Local
        );
        let current = instance.mechanical.as_ref().unwrap();
        let mounts = current.closure_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 4);
        let actual: BTreeSet<_> = mounts.iter().map(|mount| mount.id.as_str()).collect();
        assert_eq!(actual.len(), 4);
        assert_eq!(current, &before_effective[index]);
        let after_effective = boardstudio_core_mechanical::mechanical::inputs::effective_inputs(
            &document,
            &[],
            &boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
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
        let owner = document.boards.by_id(board_id);
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
    boardstudio_core_mechanical::mechanical::halves::unlink_pair(&mut expected, "unlink-pair")
        .unwrap();

    let (prepared, changed) = crate::support::prepare_transaction(&accepted, &[], &operation)
        .expect("prepared Unlink derives the normalized context internally");

    assert_eq!(prepared, expected);
    assert!(prepared.hardware.as_ref().unwrap().pairs.is_empty());
    assert_eq!(
        changed.into_iter().collect::<BTreeSet<_>>(),
        unlink_diff_ids(&accepted, &prepared)
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&prepared).unwrap();
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), accepted_bytes);
}

#[test]
fn prepared_unlink_publication_undo_redo_and_reopen_preserve_materialized_closures() {
    let mut engine = boardstudio_core_engine::CoreEngine::new();
    assert!(matches!(
        engine.handle(boardstudio_model::model::CoreRequest::Open {
            id: "open-unlink".into(),
            document: nonempty_unlink_document(),
        }),
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    let accepted = engine.document_for_test().clone();
    let operation: EditOperation = serde_json::from_value(json!({
        "kind": "apply-mechanical-edit",
        "edit": {
            "target": {"kind": "instance", "instanceId": "source-instance"},
            "change": {"unlink-mirror-pair": {"instance_id": "source-instance"}}
        }
    }))
    .expect("the public UnlinkMirrorPair wire shape parses");
    let (prepared, changed) = crate::support::prepare_transaction(&accepted, &[], &operation)
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

    let (cache, contours, findings) =
        boardstudio_core_kernel::geometry::outlines(&prepared, None, &changed);
    let publication = engine.publish_prepared_for_test(
        "unlink-prepared".into(),
        "unlink-prepared".into(),
        prepared,
        changed,
        cache,
        contours,
        findings,
    );
    assert!(matches!(
        publication,
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    assert_eq!(engine.undo_for_test().len(), 1);
    assert!(engine.redo_for_test().is_empty());
    assert!(
        engine
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .pairs
            .is_empty()
    );
    let committed_closures: BTreeSet<_> = engine
        .document_for_test()
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    assert_eq!(committed_closures, expected_closures);

    assert!(matches!(
        engine.handle(boardstudio_model::model::CoreRequest::Undo {
            id: "undo-unlink".into(),
        }),
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    assert_eq!(engine.undo_for_test().len(), 0);
    assert_eq!(engine.redo_for_test().len(), 1);
    assert_eq!(
        engine
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .pairs
            .len(),
        1
    );
    assert_eq!(
        engine.document_for_test().hardware.as_ref().unwrap().pairs[0].id,
        "unlink-pair"
    );
    assert_eq!(
        engine
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .instances[0]
            .construction_ownership,
        MechanicalConstructionOwnership::Pair
    );

    assert!(matches!(
        engine.handle(boardstudio_model::model::CoreRequest::Redo {
            id: "redo-unlink".into(),
        }),
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    assert_eq!(engine.undo_for_test().len(), 1);
    assert!(engine.redo_for_test().is_empty());
    assert!(
        engine
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .pairs
            .is_empty()
    );
    let redone_closures: BTreeSet<_> = engine
        .document_for_test()
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    let redone_configurations: BTreeMap<_, _> = engine
        .document_for_test()
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
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .all(|instance| instance.construction_ownership
                == MechanicalConstructionOwnership::Local)
    );

    let mut reopened = boardstudio_core_engine::CoreEngine::new();
    assert!(matches!(
        reopened.handle(boardstudio_model::model::CoreRequest::Open {
            id: "reopen-unlinked".into(),
            document: engine.document_for_test().clone(),
        }),
        boardstudio_model::model::CoreReply::Scene { .. }
    ));
    let reopened_closures: BTreeSet<_> = reopened
        .document_for_test()
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .map(|part| part.id.clone())
        .collect();
    let reopened_configurations: BTreeMap<_, _> = reopened
        .document_for_test()
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .map(|instance| (instance.id.clone(), instance.mechanical.clone().unwrap()))
        .collect();
    assert_eq!(reopened_closures, expected_closures);
    assert_eq!(reopened_configurations, expected_configurations);
    assert_eq!(
        reopened
            .document_for_test()
            .hardware
            .as_ref()
            .unwrap()
            .pairs
            .len(),
        0
    );
}

#[test]
fn unlink_reuse_board_keeps_one_nonempty_closure_set_and_flipped_geometry() {
    let mut document = reuse_board_unlink_document();
    let before = document.clone();
    let flipped_before = document.hardware.as_ref().unwrap().instances[1].flipped;
    let before_effective: Vec<_> = ["source-instance", "mirror-instance"]
        .into_iter()
        .map(|instance_id| {
            boardstudio_core_mechanical::mechanical::inputs::effective_inputs(
                &before,
                &[],
                &boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
                    instance_id: instance_id.into(),
                },
            )
            .unwrap()
            .configuration
        })
        .collect();
    let changed =
        boardstudio_core_mechanical::mechanical::halves::unlink_pair(&mut document, "unlink-pair")
            .unwrap();

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
        let after_effective = boardstudio_core_mechanical::mechanical::inputs::effective_inputs(
            &document,
            &[],
            &boardstudio_core_mechanical::mechanical::MechanicalTarget::Instance {
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

    let error =
        boardstudio_core_mechanical::mechanical::halves::unlink_pair(&mut document, "another-pair")
            .unwrap_err();

    assert!(error.contains("another-pair"), "{error}");
    assert_eq!(document, before);
}

#[test]
fn unlink_refuses_a_construction_source_on_the_wrong_board_atomically() {
    let mut document = nonempty_unlink_document();
    document.hardware.as_mut().unwrap().pairs[0].construction_source =
        Some("mirror-instance".into());
    let before = document.clone();

    let error =
        boardstudio_core_mechanical::mechanical::halves::unlink_pair(&mut document, "unlink-pair")
            .unwrap_err();

    assert!(
        error.contains("sourceBoardId") || error.contains("source board"),
        "{error}"
    );
    assert_eq!(document, before);
}
