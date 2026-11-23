use boardstudio_core_engine::CoreEngine;
use boardstudio_core_engine::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Position, ProjectDoc, Vec2,
};
use serde_json::json;

fn document() -> ProjectDoc {
    let mut value = serde_json::to_value(ProjectDoc::empty("mirror", "Mirror")).unwrap();
    value["boards"] = json!([
        {"id":"source","name":"Source","partIds":["switch"],"outlineIds":[],"netIds":[],"thickness":1.6},
        {"id":"destination","name":"Destination","partIds":[],"outlineIds":[],"netIds":[],"thickness":1.6}
    ]);
    value["definitions"] = json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]);
    value["parts"] = json!([
        {"id":"switch","definitionId":"switch","reference":"SW1","pose":{"at":{"x":2.0,"y":3.0},"rotation":15.0},"side":"front"}
    ]);
    value["mirroredPairs"] = json!([{
        "id":"pair",
        "source":{"kind":"secondBoard","id":"source"},
        "destination":{"kind":"secondBoard","id":"destination"},
        "mapping":{"at":{"x":10.0,"y":0.0},"rotation":0.0},
        "overrides":[]
    }]);
    serde_json::from_value(value).unwrap()
}

fn accepted(reply: CoreReply) -> (ProjectDoc, Vec2) {
    let CoreReply::Scene {
        document, scene, ..
    } = reply
    else {
        panic!("Expected accepted scene, got {reply:?}");
    };
    let target = scene
        .transforms
        .iter()
        .find(|part| part.id == "4/pair/part/switch")
        .unwrap();
    assert_eq!(document.parts.len(), 1);
    (*document, target.pose.at)
}

fn move_part(engine: &mut CoreEngine, revision: u64, part: &str, x: f64) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "move".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "move".into(),
            phase: EditPhase::Commit,
            target_ids: vec![part.into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: part.into(),
                    at: Vec2 { x, y: 3.0 },
                }],
            },
        },
    })
}

#[test]
fn linked_edit_refusal_and_seeded_undo_redo_keep_canonical_geometry_exact() {
    for seed in 0..32 {
        exercise_history(seed);
    }
}

fn exercise_history(seed: u32) {
    let mut engine = CoreEngine::new();
    let (mut canonical, initial) = accepted(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document(),
    }));
    assert_eq!(initial, Vec2 { x: 8.0, y: 3.0 });
    let refusal = move_part(&mut engine, canonical.revision, "4/pair/part/switch", 3.0);
    assert!(matches!(refusal, CoreReply::Error { message, .. } if message.contains("is linked")));
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));

    let mut seed = 0x84ba_318a_u32 ^ seed;
    let mut previous = initial;
    for _ in 0..40 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = f64::from(seed) / 65536.0;
        let before = canonical.clone();
        let (edited, geometry) = accepted(move_part(&mut engine, canonical.revision, "switch", x));
        assert_eq!(
            geometry,
            Vec2 {
                x: 10.0 - x,
                y: 3.0
            }
        );

        let (mut undone, geometry) =
            accepted(engine.handle(CoreRequest::Undo { id: "undo".into() }));
        assert_eq!(geometry, previous);
        undone.revision = before.revision;
        assert_eq!(undone, before);

        let (redone, geometry) = accepted(engine.handle(CoreRequest::Redo { id: "redo".into() }));
        let mut expected = edited;
        expected.revision = redone.revision;
        assert_eq!(redone, expected);
        previous = geometry;
        canonical = redone;
    }

    let saved = serde_json::to_vec(&canonical).unwrap();
    let (reopened, geometry) = accepted(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: serde_json::from_slice(&saved).unwrap(),
    }));
    assert_eq!(serde_json::to_vec(&reopened).unwrap(), saved);
    assert_eq!(geometry, previous);
}

#[test]
fn version_and_linked_target_errors_preserve_the_accepted_snapshot() {
    use boardstudio_core_mirrored_half::{MirrorEditRefused, admit_edit};

    let mut engine = CoreEngine::new();
    let (accepted_doc, accepted_pose) = accepted(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document(),
    }));
    for version in [0, 7, 9, u32::MAX] {
        let mut rejected = accepted_doc.clone();
        rejected.format_version = version;
        assert!(matches!(
            engine.handle(CoreRequest::Open { id: "bad-version".into(), document: rejected }),
            CoreReply::Error { message, .. } if message == "Unsupported formatVersion: expected 8"
        ));
    }
    let target_id = "4/pair/part/switch".to_owned();
    assert_eq!(
        admit_edit(
            &accepted_doc,
            &EditOperation::RemoveParts {
                ids: vec![target_id.clone()]
            }
        ),
        Err(MirrorEditRefused::LinkedTarget {
            target_id,
            source_id: "switch".into(),
            pair_id: "pair".into(),
        })
    );
    assert_eq!(
        accepted(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        })),
        (accepted_doc, accepted_pose)
    );
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Error { .. }
    ));
}

#[test]
fn projection_reflects_matrix_geometry_outline_and_keycap_settings() {
    use boardstudio_core_mirrored_half::project_document;

    let mut value = serde_json::to_value(document()).unwrap();
    value["matrices"] = json!([{
        "id":"keys","rows":2,"columns":3,"pitch":{"x":19,"y":20},
        "origin":{"x":3,"y":4},"rotation":20,"definitionId":"switch",
        "partIds":[],"boardId":"source",
        "cells":[{"row":1,"column":2,"enabled":false}]
    }]);
    value["boards"][0]["outlineIds"] = json!(["perimeter"]);
    value["outline"] = json!([{"kind":"polygon","id":"perimeter","operation":"add",
        "points":[{"x":0,"y":0},{"x":3,"y":0},{"x":0,"y":4}]}]);
    value["keycaps"] = json!({"keys":{"switch":{"legend":"A"}}});
    let canonical: ProjectDoc = serde_json::from_value(value).unwrap();
    let projection = project_document(&canonical);
    assert_eq!(projection, project_document(&canonical));
    let geometry = projection.geometry();
    let reflected = &geometry.matrices[1];
    assert_eq!((reflected.rows, reflected.columns), (2, 3));
    assert_eq!(reflected.origin, Vec2 { x: 7.0, y: 4.0 });
    assert_eq!(reflected.rotation, Some(-20.0));
    assert_eq!(reflected.cells, canonical.matrices[0].cells);
    let outline = &geometry.outline[1];
    assert_eq!(geometry.boards[1].outline_ids, [outline.id()]);
    assert!(
        matches!(outline, boardstudio_core_engine::model::OutlineFeature::Polygon {points, ..}
        if points == &[Vec2 {x:10.0,y:4.0}, Vec2 {x:7.0,y:0.0}, Vec2{x:10.0,y:0.0}])
    );
    assert_eq!(
        geometry.keycaps.as_ref().unwrap().keys["4/pair/part/switch"]
            .legend
            .as_deref(),
        Some("A")
    );
}

#[test]
fn routed_copper_and_constraints_block_omission_and_pose_overrides_are_sparse() {
    use boardstudio_core_engine::model::{HalfExceptionState, Pose2};
    use boardstudio_core_mirrored_half::{project_document, set_companion_pose, set_exception};

    let mut value = serde_json::to_value(document()).unwrap();
    let target = "4/pair/part/switch";
    value["nets"] =
        json!([{"id":"route","name":"Route","pins":[{"partId":"switch","padId":"pad"}]}]);
    value["boards"][1]["traces"] = json!([{"id":"trace-on-target","layer":"front","netId":"4/pair/net/route","width":0.2,
        "start":{"x":0,"y":0},"end":{"x":1,"y":1}}]);
    let mut doc: ProjectDoc = serde_json::from_value(value).unwrap();
    let original = doc.clone();
    let error = set_exception(&mut doc, "pair", target, HalfExceptionState::Omitted).unwrap_err();
    assert!(error.contains("trace-on-target"));
    assert_eq!(doc, original);

    doc.boards[1].traces.clear();
    doc.constraints = serde_json::from_value(json!([{
        "kind":"offset","id":"held-target","sourcePartId":"switch","targetPartId":target,"offset":{"x":0,"y":0},"rotation":0
    }]))
    .unwrap();
    let error = set_exception(&mut doc, "pair", target, HalfExceptionState::Omitted).unwrap_err();
    assert!(error.contains("held-target"));
    doc.constraints.clear();

    let pose = Pose2 {
        at: Vec2 { x: 7.0, y: 8.0 },
        rotation: 22.0,
    };
    set_companion_pose(&mut doc, "pair", target, pose).unwrap();
    assert_eq!(project_document(&doc).geometry().parts[1].pose, pose);
    assert_eq!(doc.parts, original.parts);
    set_exception(&mut doc, "pair", target, HalfExceptionState::Linked).unwrap();
    assert_eq!(
        project_document(&doc).geometry().parts[1].pose.at,
        Vec2 { x: 8.0, y: 3.0 }
    );
}

#[test]
fn projection_reflects_hosted_modules_and_physical_mechanical_features() {
    use boardstudio_core_mirrored_half::project_document;

    let mut value = serde_json::to_value(document()).unwrap();
    value["modules"] = json!([{
        "id":"daughter","definitionId":"daughter","hostBoardId":"source",
        "hostFace":"front","facingFace":"back","at":{"x":3,"y":4},"rotation":20,
        "gap":1,"attachment":"board"
    }]);
    value["hardware"] = json!({"instances":[
        {"id":"left","name":"Left","boardId":"source","half":"left","role":"controller",
         "flipped":false,"mechanical":{
            "boardId":"source","method":"printed","mount":"rigid","plateThickness":1.5,
            "plateFoamThickness":0.5,"pcbThickness":1.6,"bottomFoamThickness":0.4,
            "batteryHeight":4,"bottomThickness":3,"plateToPcb":3.5,"wallThickness":2,
            "clearance":0.3,"profiles":[],
            "mounts":[{"id":"mount","at":{"x":3,"y":4},"kind":"hole","holeDiameter":2.2}]
         }},
        {"id":"right","name":"Right","boardId":"destination","half":"right","role":"peripheral","flipped":false}
    ]});
    let doc: ProjectDoc = serde_json::from_value(value).unwrap();
    let projected = project_document(&doc);
    let geometry = projected.geometry();
    let module = &geometry.modules[1];
    assert_eq!(module.host_board_id, "destination");
    assert_eq!(module.at, Vec2 { x: 7.0, y: 4.0 });
    assert_eq!(module.rotation, -20.0);
    let mechanical = geometry.hardware.as_ref().unwrap().instances[1]
        .mechanical
        .as_ref()
        .unwrap();
    assert_eq!(mechanical.board_id, "destination");
    assert_eq!(mechanical.mounts[0].at, Vec2 { x: 7.0, y: 4.0 });
    assert_eq!(projected, project_document(&doc));
}

#[test]
fn local_part_rotation_source_deletion_and_removal_round_trip() {
    use boardstudio_core_engine::model::{HalfExceptionState, MirroredItemKind};

    let mut engine = CoreEngine::new();
    let target = "4/pair/part/switch";
    let mut reply = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document(),
    });
    for operation in [
        EditOperation::SetHalfException {
            pair_id: "pair".into(),
            kind: MirroredItemKind::Part,
            mirror_id: target.into(),
            state: HalfExceptionState::Local,
        },
        EditOperation::SetPartRotation {
            part_id: target.into(),
            rotation: 37.0,
        },
        EditOperation::RemoveParts {
            ids: vec!["switch".into()],
        },
    ] {
        let CoreReply::Scene { document, .. } = reply else {
            panic!("{reply:?}")
        };
        reply = engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command: EditCommand {
                base_revision: document.revision,
                transaction_id: "edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        });
    }
    let CoreReply::Scene {
        document, scene, ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    assert!(document.parts.is_empty());
    assert_eq!(scene.transforms.len(), 1);
    assert_eq!(scene.transforms[0].id, target);
    assert_eq!(scene.transforms[0].pose.rotation, 37.0);
    let result = engine.handle(CoreRequest::Edit {
        id: "remove-local".into(),
        command: EditCommand {
            base_revision: document.revision,
            transaction_id: "remove-local".into(),
            phase: EditPhase::Commit,
            target_ids: vec![target.into()],
            operation: EditOperation::RemoveParts {
                ids: vec![target.into()],
            },
        },
    });
    assert!(matches!(result, CoreReply::Scene { scene, .. } if scene.transforms.is_empty()));
}

#[test]
fn derived_board_geometry_edits_refuse_without_publishing() {
    let mut engine = CoreEngine::new();
    let opened = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document(),
    });
    let (original, _) = accepted(opened);
    let candidate = json!({"kind":"edit","id":"outline","command":{
        "baseRevision":original.revision,"transactionId":"outline","phase":"commit","targetIds":[],
        "operation":{"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"destination"},"change":{"set-margin":{"margin":4}},
            "seed":1
        }}
    }});
    let refusal: serde_json::Value =
        serde_json::from_str(&engine.request(&candidate.to_string())).unwrap();
    assert_eq!(refusal["kind"], "error");
    assert!(refusal["message"].as_str().unwrap().contains("edit source"));
    let (snapshot, _) = accepted(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(snapshot, original);
}

#[test]
fn mixed_lifecycle_history_and_two_board_unlink_preserve_geometry() {
    use boardstudio_core_engine::model::{
        CompanionPlacementTarget, HalfExceptionState, MirroredCompanionMode,
        MirroredCompanionTweak, MirroredItemKind,
    };

    let target = "4/pair/part/switch";
    let exception = |state| EditOperation::SetHalfException {
        pair_id: "pair".into(),
        kind: MirroredItemKind::Part,
        mirror_id: target.into(),
        state,
    };
    for seed in 0..8_u32 {
        let mut engine = CoreEngine::new();
        let mut current = accepted(engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: document(),
        }));
        let mut random = seed + 1;
        let mut local = false;
        for _ in 0..40 {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = f64::from(random >> 8) / 1024.0;
            let operation = match (random >> 28) % 3 {
                0 => EditOperation::MoveParts {
                    positions: vec![Position {
                        id: "switch".into(),
                        at: Vec2 { x, y: 3.0 },
                    }],
                },
                1 => {
                    local = !local;
                    exception(if local {
                        HalfExceptionState::Local
                    } else {
                        HalfExceptionState::Linked
                    })
                }
                _ if local => {
                    local = false;
                    exception(HalfExceptionState::Linked)
                }
                _ => EditOperation::SetCompanionPlacement {
                    pair_id: "pair".into(),
                    targets: vec![CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id: target.into(),
                    }],
                    mode: MirroredCompanionMode::LiteralMirrorWithTweak,
                    tweak: Some(MirroredCompanionTweak {
                        x,
                        y: 3.0,
                        rotation: 20.0,
                    }),
                },
            };
            let next = accepted(engine.handle(CoreRequest::Edit {
                id: "edit".into(),
                command: EditCommand {
                    base_revision: current.0.revision,
                    transaction_id: "edit".into(),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation,
                },
            }));
            let mut undo = accepted(engine.handle(CoreRequest::Undo { id: "undo".into() }));
            undo.0.revision = current.0.revision;
            assert_eq!(undo, current);
            let redo = accepted(engine.handle(CoreRequest::Redo { id: "redo".into() }));
            let mut expected = next;
            expected.0.revision = redo.0.revision;
            assert_eq!(redo, expected);
            current = redo;
        }
        let unlinked = engine.handle(CoreRequest::Edit {
            id: "unlink".into(),
            command: EditCommand {
                base_revision: current.0.revision,
                transaction_id: "unlink".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::UnlinkHalves {
                    pair_id: "pair".into(),
                },
            },
        });
        let CoreReply::Scene {
            document, scene, ..
        } = unlinked
        else {
            panic!("{unlinked:?}")
        };
        assert!(document.mirrored_pairs.is_empty());
        assert_eq!(document.parts.len(), 2);
        assert_eq!(
            scene
                .transforms
                .iter()
                .find(|part| part.id == target)
                .unwrap()
                .pose
                .at,
            current.1
        );
    }
}
