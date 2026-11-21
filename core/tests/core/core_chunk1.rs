use super::*;

#[test]
fn open_preserves_generator_and_copper() {
    let mut doc = matrix_doc();
    doc.definitions[0].pads.push(Pad {
        id: "p1".into(),
        number: "1".into(),
        at: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Circle,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    });
    doc.definitions[0].pads[0].plated = Some(false);
    doc.definitions[0].pads[0].side = Some(Side::Back);
    doc.definitions[0].pads[0].rotation = Some(90.0);
    doc.definitions[0].generator = Some(PartGenerator {
        source: "builtin:mx-switch".into(),
        version: "1".into(),
        parameters: Default::default(),
    });
    doc.boards[0].traces.push(CopperTrace {
        id: "trace".into(),
        start: Vec2 { x: 0.0, y: 0.0 },
        end: Vec2 { x: 1.0, y: 0.0 },
        width: 0.25,
        layer: Side::Front,
        net_id: None,
    });
    doc.boards[0].vias.push(CopperVia {
        id: "via".into(),
        at: Vec2 { x: 1.0, y: 0.0 },
        size: 0.8,
        drill: 0.4,
        net_id: None,
    });
    let mut engine = CoreEngine::new();
    let json = serde_json::to_string(&CoreRequest::Open {
        id: "open".into(),
        document: doc,
    })
    .unwrap();
    let (_, reopened) = scene(serde_json::from_str(&engine.request(&json)).unwrap());
    assert_eq!(
        reopened.definitions[0].generator.as_ref().unwrap().source,
        "builtin:mx-switch"
    );
    assert_eq!(reopened.boards[0].traces[0].id, "trace");
    assert_eq!(reopened.boards[0].vias[0].id, "via");
    assert_eq!(reopened.definitions[0].pads[0].plated, Some(false));
    assert_eq!(reopened.definitions[0].pads[0].rotation, Some(90.0));
}

#[test]
fn added_part_joins_board_and_envelope_then_removes_cleanly() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -1.0, y: -1.0 },
            Vec2 { x: 1.0, y: -1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: -1.0, y: 1.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "env".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 1.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["env".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let part = Part {
        anchor: None,
        id: "k1".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    };
    let (added, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::AddPart {
            part,
            board_id: None,
            definition: None,
            layout_id: None,
        },
    )));
    assert!(added.readiness.outline);
    assert_eq!(doc.boards[0].part_ids, vec!["k1"]);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &doc.outline[0] else {
        panic!("expected envelope")
    };
    assert_eq!(part_ids, &vec!["k1"]);
    let (_, removed) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["k1".into()],
        },
    )));
    assert!(removed.boards[0].part_ids.is_empty());
    let OutlineFeature::PartEnvelope { part_ids, .. } = &removed.outline[0] else {
        panic!("expected envelope")
    };
    assert!(part_ids.is_empty());
}

#[test]
fn invalid_definition_geometry_blocks_own_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    for (id, x) in [("left", -20.0), ("right", 20.0)] {
        doc.outline
            .push(rect(id, x, 0.0, 10.0, 10.0, Operation::Add));
        doc.boards.push(Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![id.into()],
            part_ids: vec![format!("{id}-part")],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(Part {
            anchor: None,
            id: format!("{id}-part"),
            definition_id: format!("{id}-def"),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
        doc.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            mechanical_profile: None,
            id: format!("{id}-def"),
            name: id.into(),
            kind: PartKind::Custom,
            courtyard: vec![Vec2 { x: 0.0, y: 0.0 }],
            pads: vec![Pad {
                id: "p1".into(),
                number: "1".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: PadShape::Roundrect,
                drill: Some(0.5),
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            }],
            models: None,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
        });
    }
    let bad = &mut doc.definitions[1];
    bad.courtyard[0].x = f64::NAN;
    bad.pads[0].size.x = -1.0;
    bad.pads[0].drill = Some(-0.5);
    bad.pads.push(Pad {
        id: "p1".into(),
        number: "1".into(),
        at: Vec2 {
            x: f64::INFINITY,
            y: 0.0,
        },
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Oval,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    });
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert!(scene.board_readiness[0].pcb);
    assert!(!scene.board_readiness[1].pcb);
    for suffix in [
        "courtyard",
        "pad:p1:size",
        "pad:p1:drill",
        "pad:p1:position",
        "pad:p1:duplicate-id",
    ] {
        assert!(
            scene.findings.iter().any(|finding| finding.id
                == format!("definition:right-def:{suffix}")
                && finding.target_ids.contains(&"right-part".to_string())),
            "missing {suffix}"
        );
    }
}

#[test]
fn case_readiness_uses_its_board() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "case-board".into(),
        name: "Case Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.boards.push(Board {
        id: "empty".into(),
        name: "Empty".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "case".into(),
        name: "Case".into(),
        board_id: "case-board".into(),
        kind: CaseKind::Plate,
        thickness: 3.0,
        clearance: 0.5,
        material_id: None,
        z: None,
        wall_height: None,
        wall_thickness: None,
        mounts: None,
        pcb_source: None,
        gasket: None,
    });
    let (valid_scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    assert!(!valid_scene.readiness.pcb);
    assert!(valid_scene.board_readiness[0].pcb);
    assert!(!valid_scene.board_readiness[1].pcb);
    assert!(valid_scene.board_readiness[0].case_ready);
    assert!(!valid_scene.board_readiness[1].case_ready);
    assert!(valid_scene.readiness.case_ready);
    assert!(boardstudio_core::authored_case_geometry_ready(
        &doc,
        &valid_scene,
        "case-board"
    ));
    let mut pcb_unready_scene = valid_scene.clone();
    pcb_unready_scene.board_readiness[0].pcb = false;
    pcb_unready_scene.board_readiness[0].case_ready = false;
    assert!(boardstudio_core::authored_case_geometry_ready(
        &doc,
        &pcb_unready_scene,
        "case-board"
    ));
    let mut stale_scene = valid_scene.clone();
    stale_scene.revision += 1;
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &doc,
        &stale_scene,
        "case-board"
    ));
    let mut outline_unready_scene = valid_scene.clone();
    outline_unready_scene.board_readiness[0].outline = false;
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &doc,
        &outline_unready_scene,
        "case-board"
    ));
    let mut invalid = doc;
    invalid.case_bodies[0].thickness = -1.0;
    let (invalid_scene, invalid) = scene(engine.handle(CoreRequest::Open {
        id: "invalid".into(),
        document: invalid,
    }));
    assert!(!invalid_scene.board_readiness[0].case_ready);
    assert!(invalid_scene.board_readiness[0].pcb);
    assert!(!boardstudio_core::authored_case_geometry_ready(
        &invalid,
        &invalid_scene,
        "case-board"
    ));
}

#[test]
fn case_mounts_walls_and_gasket_are_validated() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.outline
        .push(rect("edge", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "tray".into(),
        name: "Tray".into(),
        board_id: "board".into(),
        kind: CaseKind::Tray,
        thickness: 3.0,
        clearance: 0.5,
        material_id: None,
        z: Some(0.0),
        wall_height: None,
        wall_thickness: None,
        mounts: Some(vec![Mount {
            id: "mount".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            kind: MountKind::Boss,
            hole_diameter: 3.0,
            boss_diameter: Some(2.0),
            height: None,
        }]),
        pcb_source: None,
        gasket: Some(Gasket {
            inset: 1.0,
            width: 2.0,
            depth: 3.0,
        }),
    });
    let (bad, _) = scene(engine.handle(CoreRequest::Open {
        id: "bad".into(),
        document: doc.clone(),
    }));
    assert!(bad.readiness.pcb);
    assert!(!bad.readiness.case_ready);
    for id in [
        "case:tray:walls",
        "case:tray:mount:mount:boss",
        "case:tray:gasket",
    ] {
        assert!(bad.findings.iter().any(|finding| finding.id == id));
    }
    doc.case_bodies[0].wall_height = Some(8.0);
    doc.case_bodies[0].wall_thickness = Some(2.0);
    doc.case_bodies[0].mounts.as_mut().unwrap()[0].boss_diameter = Some(5.0);
    doc.case_bodies[0].mounts.as_mut().unwrap()[0].height = Some(4.0);
    doc.case_bodies[0].gasket.as_mut().unwrap().depth = 1.0;
    let (good, document) = scene(engine.handle(CoreRequest::Open {
        id: "good".into(),
        document: doc,
    }));
    assert!(good.readiness.case_ready);
    let wire = serde_json::to_value(&document).unwrap();
    assert_eq!(wire["caseBodies"][0]["wallHeight"], 8.0);
    assert_eq!(wire["caseBodies"][0]["mounts"][0]["bossDiameter"], 5.0);
}

#[test]
fn mirror_only_part_is_target_owned_across_preview_history_and_open() {
    let (mut engine, mut accepted) = public_native_pair();
    accepted.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "mirror-only-envelope".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    let target_board_id = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    accepted
        .boards
        .iter_mut()
        .find(|board| board.id == target_board_id)
        .unwrap()
        .outline_ids
        .push("mirror-only-envelope".into());
    engine.handle(CoreRequest::Open {
        id: "reopen-fixture".into(),
        document: accepted.clone(),
    });
    let (_, accepted) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-fixture".into(),
    }));
    let halves = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .clone();
    let source_parts: Vec<_> = accepted
        .parts
        .iter()
        .filter(|part| {
            accepted.boards.iter().any(|board| {
                board.id == halves.source_board_id && board.part_ids.contains(&part.id)
            })
        })
        .cloned()
        .collect();
    let pair_before = serde_json::to_value(&accepted.hardware.as_ref().unwrap().pairs[0]).unwrap();
    let mut part = accepted.parts[0].clone();
    part.id = "mirror/only".into();
    part.reference = "MO1".into();
    part.pose.at = Vec2 { x: 230.0, y: 18.0 };
    let operation = EditOperation::AddMirrorOnlyPart {
        part: part.clone(),
        board_id: halves.mirror_board_id.clone(),
        definition: None,
        layout_id: None,
    };

    assert!(matches!(
        engine.handle(edit(
            accepted.revision,
            EditPhase::Preview,
            operation.clone()
        )),
        CoreReply::Preview { .. }
    ));
    let (_, after_preview) = scene(engine.handle(CoreRequest::Snapshot {
        id: "after-preview".into(),
    }));
    assert_eq!(
        after_preview.parts, accepted.parts,
        "Preview does not publish the target Part"
    );

    let (_, added) = scene(engine.handle(edit(accepted.revision, EditPhase::Commit, operation)));
    assert_eq!(
        added
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    assert_eq!(
        added
            .boards
            .iter()
            .find(|board| board.id == halves.mirror_board_id)
            .unwrap()
            .part_ids
            .iter()
            .filter(|id| **id == part.id)
            .count(),
        1
    );
    let resulting_source_parts: Vec<_> = added
        .parts
        .iter()
        .filter(|candidate| {
            added.boards.iter().any(|board| {
                board.id == halves.source_board_id && board.part_ids.contains(&candidate.id)
            })
        })
        .cloned()
        .collect();
    assert_eq!(
        resulting_source_parts, source_parts,
        "mirror-only addition leaves source parts untouched"
    );
    let pair_after = serde_json::to_value(&added.hardware.as_ref().unwrap().pairs[0]).unwrap();
    assert_eq!(
        pair_after, pair_before,
        "mirror-only addition creates no pair provenance"
    );
    let OutlineFeature::PartEnvelope { part_ids, .. } = added
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-only-envelope")
        .unwrap()
    else {
        panic!("expected target envelope")
    };
    assert_eq!(part_ids.iter().filter(|id| **id == part.id).count(), 1);
    let OutlineFeature::PartEnvelope { part_ids, .. } = added
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror/synthetic-envelope")
        .unwrap()
    else {
        panic!("expected native linked target envelope")
    };
    assert_eq!(
        part_ids.iter().filter(|id| **id == part.id).count(),
        1,
        "the target-owned Part is retained in the linked target envelope"
    );

    let (_, undone) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-mirror-only".into(),
    }));
    assert!(!undone.parts.iter().any(|candidate| candidate.id == part.id));
    let (_, redone) = scene(engine.handle(CoreRequest::Redo {
        id: "redo-mirror-only".into(),
    }));
    assert_eq!(
        redone
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    let serialized = serde_json::to_value(&redone).unwrap();
    let (_, reopened) = scene(engine.handle(CoreRequest::Open {
        id: "reopen-mirror-only".into(),
        document: serde_json::from_value(serialized).unwrap(),
    }));
    assert_eq!(
        reopened
            .parts
            .iter()
            .filter(|candidate| candidate.id == part.id)
            .count(),
        1
    );
    let OutlineFeature::PartEnvelope { part_ids, .. } = reopened
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror/synthetic-envelope")
        .unwrap()
    else {
        panic!("expected native linked target envelope after Open")
    };
    assert_eq!(
        part_ids.iter().filter(|id| **id == part.id).count(),
        1,
        "Open retains the independent target envelope membership"
    );
}

#[test]
fn mirror_only_part_refuses_reusing_an_existing_part_identity_atomically() {
    let (mut engine, document) = public_native_pair();
    let mirror_board_id = document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    let reused = document.parts[0].clone();
    let reply = engine.handle(edit(
        document.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: reused,
            board_id: mirror_board_id,
            definition: None,
            layout_id: None,
        },
    ));
    assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
    let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot-after-identity-reuse".into(),
    }));
    assert_eq!(after_refusal, document);
}

#[test]
fn mirror_only_part_refuses_ids_owned_by_other_document_entities_atomically() {
    let (mut engine, accepted) = public_native_pair();
    let mirror_board_id = accepted.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .mirror_board_id
        .clone();
    let reused_ids = [
        ("board", mirror_board_id.clone()),
        ("matrix", accepted.matrices[0].id.clone()),
        ("layout", accepted.layouts[0].id.clone()),
        ("outline", accepted.outline[0].id().to_owned()),
        ("net", accepted.nets[0].id.clone()),
    ];
    let mut prior = accepted.parts[0].clone();
    prior.id = "mirror/prior-redo".into();
    prior.reference = "PRIOR".into();
    scene(engine.handle(edit(
        accepted.revision,
        EditPhase::Commit,
        EditOperation::AddMirrorOnlyPart {
            part: prior,
            board_id: mirror_board_id.clone(),
            definition: None,
            layout_id: None,
        },
    )));
    let (_, before_refusal) = scene(engine.handle(CoreRequest::Snapshot {
        id: "before-identity-refusals".into(),
    }));

    for (kind, reused_id) in reused_ids {
        assert!(
            accepted.parts.iter().all(|part| part.id != reused_id),
            "the {kind} fixture identity must not already belong to a Part"
        );
        let mut part = accepted.parts[0].clone();
        part.id = reused_id;
        part.reference = format!("COLL-{kind}");
        let reply = engine.handle(edit(
            before_refusal.revision,
            EditPhase::Commit,
            EditOperation::AddMirrorOnlyPart {
                part,
                board_id: mirror_board_id.clone(),
                definition: None,
                layout_id: None,
            },
        ));
        assert!(
            matches!(reply, CoreReply::Error { .. }),
            "the {kind} identity was reused as a Part: {reply:?}"
        );
        let (_, after_refusal) = scene(engine.handle(CoreRequest::Snapshot {
            id: format!("after-{kind}-identity-refusal"),
        }));
        assert_eq!(
            after_refusal, before_refusal,
            "refusal preserves the full document and revision for {kind} identity reuse"
        );
    }

    let (_, undone) = scene(engine.handle(CoreRequest::Undo {
        id: "undo-after-identity-refusals".into(),
    }));
    let mut expected = accepted.clone();
    expected.revision = undone.revision;
    assert_eq!(
        undone, expected,
        "every refusal preserves the seeded commit's Undo entry"
    );
}

#[test]
fn mirrored_pair_deletion_keeps_surviving_half_and_cleans_membership() {
    // Ticket 21 supersedes the old linked survival: a plain linked
    // counterpart follows the source deletion, while a Local counterpart
    // survives complete and its layout turns independent. Name kept for the
    // native baseline; the linked follow-deletion itself is proven in
    // `matrix_anchors::linked_source_removal_deletes_linked_partner_and_keeps_unrelated_pair`.
    let (mut engine, doc) = linked_pair();
    let removed = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveParts {
            ids: vec!["matrix/left-matrix/r0c0".into()],
        },
    )))
    .1;
    assert_eq!(removed.parts.len(), 10);
    assert_reflected_keys(&removed);
    // The surviving half goes Local before the source is deleted.
    let mut right_layout = removed.layouts[1].clone();
    right_layout
        .mirror_link
        .as_mut()
        .expect("mirror link")
        .exceptions
        .push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            mirror_id: "right-matrix".into(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
    let removed = scene(engine.handle(edit(
        removed.revision,
        EditPhase::Commit,
        EditOperation::SetLayout {
            layout: right_layout,
        },
    )))
    .1;
    let surviving = removed.matrices[1].clone();
    let removed = scene(engine.handle(edit(
        removed.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrix {
            id: "left-matrix".into(),
        },
    )))
    .1;
    assert_eq!(removed.matrices, vec![surviving]);
    assert_eq!(removed.layouts.len(), 1);
    assert!(removed.layouts[0].mirror_link.is_none());
    assert_eq!(removed.parts.len(), 5);
}

#[test]
fn mirrored_pair_invalid_links_and_failed_previews_do_not_change_document() {
    let (mut engine, doc) = linked_pair();
    let mut invalid = doc.matrices[0].clone();
    invalid.mirror = Some(Mirror::Y);
    assert!(matches!(
        engine.handle(edit(
            doc.revision,
            EditPhase::Preview,
            EditOperation::SetMatrix {
                matrix: invalid,
                definitions: None
            }
        )),
        CoreReply::Error { .. }
    ));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
    let mut cycle = doc.layouts[0].clone();
    cycle.mirror_link = Some(LayoutMirrorLink {
        correspondences: vec![],
        exceptions: vec![],
        source_id: "right-layout".into(),
        axis_x: 10.0,
        split_gap: None,
        spacing: None,
    });
    assert!(matches!(
        engine.handle(edit(
            doc.revision,
            EditPhase::Commit,
            EditOperation::SetLayout { layout: cycle }
        )),
        CoreReply::Error { .. }
    ));
    assert_eq!(
        scene(engine.handle(CoreRequest::Snapshot {
            id: "snapshot".into()
        }))
        .1,
        doc
    );
}

#[test]
fn matrix_duplicate_allocates_new_member_ids() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, original) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    let mut duplicate = original.matrices[0].clone();
    duplicate.id = "copy".into();
    duplicate.origin.x = 30.0;
    let (_, doc) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: duplicate,
            definitions: None,
        },
    )));
    assert_eq!(doc.matrices[0].part_ids, vec!["matrix/main/r0c0"]);
    assert_eq!(doc.matrices[1].part_ids, vec!["matrix/copy/r0c0"]);
    assert_ne!(doc.parts[0].reference, doc.parts[1].reference);
}

#[test]
fn matrix_shrink_prunes_net_pins_and_stale_members() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 2),
            definitions: None,
        },
    )));
    doc.nets.push(Net {
        id: "row".into(),
        name: "ROW".into(),
        pins: vec![Pin {
            part_id: "matrix/main/r0c1".into(),
            pad_id: "p1".into(),
        }],
    });
    doc.constraints.push(Constraint::Offset {
        id: "linked".into(),
        source_part_id: "matrix/main/r0c0".into(),
        target_part_id: "matrix/main/r0c1".into(),
        offset: Vec2 { x: 19.0, y: 0.0 },
        rotation: 0.0,
    });
    let (_, doc) = scene(engine.handle(CoreRequest::Open {
        id: "with-net".into(),
        document: doc,
    }));
    assert_eq!(doc.nets[0].pins.len(), 1);
    let (_, shrunk) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(1, 1),
            definitions: None,
        },
    )));
    assert!(shrunk.nets[0].pins.is_empty());
    assert!(shrunk.constraints.is_empty());
    assert_eq!(shrunk.matrices[0].part_ids, vec!["matrix/main/r0c0"]);
    assert!(
        shrunk
            .parts
            .iter()
            .all(|part| part.id != "matrix/main/r0c1")
    );
}

#[test]
fn matrix_accepts_stable_ergogen_catalogue_ids() {
    let definition: PartDefinition = serde_json::from_value(serde_json::json!({
        "id": "generator:ceoloide/switch_mx",
        "name": "MX switch",
        "kind": "switch",
        "courtyard": [{"x": -5.0, "y": -5.0}, {"x": 5.0, "y": -5.0}, {"x": 5.0, "y": 5.0}, {"x": -5.0, "y": 5.0}],
        "pads": [{"id": "pad-0", "number": "1", "at": {"x": -2.0, "y": 0.0}, "size": {"x": 1.0, "y": 1.0}, "shape": "rect"}, {"id": "pad-1", "number": "2", "at": {"x": 2.0, "y": 0.0}, "size": {"x": 1.0, "y": 1.0}, "shape": "rect"}],
        "matrixTerminals": {"row": "from", "column": "to"},
        "generator": {"source": "ceoloide/switch_mx", "version": "bundled-1", "parameters": {}}
    })).unwrap();
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut incoming = matrix(1, 1);
    incoming.definition_id = definition.id.clone();
    let reply = engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: incoming,
            definitions: Some(vec![definition]),
        },
    ));
    let (_, generated) = scene(reply);
    assert_eq!(
        generated.matrices[0].definition_id,
        "generator:ceoloide/switch_mx"
    );
}

#[test]
fn matrix_custom_splay_origin_survives_export_and_resize() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let mut value = matrix(2, 3);
    value.pitch = Vec2 { x: 20.0, y: 20.0 };
    {
        let values = vec![0.0, 90.0];
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.splay = 0.0;
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    }
    {
        let values = (vec![None, Some(Vec2 { x: -20.0, y: -20.0 })])
            .into_iter()
            .map(|value| value.unwrap_or_default())
            .collect::<Vec<_>>();
        value.column_layout.resize_with(
            values.len().max(value.column_layout.len()),
            Default::default,
        );
        for record in &mut value.column_layout {
            record.splay_origin = Vec2::default();
        }
        for (record, value) in value.column_layout.iter_mut().zip(values) {
            record.splay_origin = value;
        }
    }
    let (_, doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
    let key = doc
        .parts
        .iter()
        .find(|p| p.id == "matrix/main/r0c1")
        .unwrap();
    assert!((key.pose.at.x + 20.0).abs() < 1e-9);
    assert!(key.pose.at.y.abs() < 1e-9);
    assert_eq!(key.pose.rotation, 90.0);
    let reopened: ProjectDoc = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        reopened.matrices[0].column_layout[1].splay_origin,
        Vec2 { x: -20.0, y: -20.0 }
    );
    let mut value = reopened.matrices[0].clone();
    value.columns = 1;
    let (_, shrunk) = scene(engine.handle(edit(
        1,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: value,
            definitions: None,
        },
    )));
    assert_eq!(
        shrunk.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>(),
        vec![Vec2::default()]
    );
    let (_, restored) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(
        restored.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>(),
        reopened.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.splay_origin)
            .collect::<Vec<_>>()
    );
}

#[test]
fn deleting_matrix_row_preserves_surviving_part_and_binding_identity() {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let (_, mut doc) = scene(engine.handle(edit(
        0,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: matrix(3, 1),
            definitions: None,
        },
    )));
    let survivor = doc
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/main/r2c0")
        .unwrap();
    survivor.keycap = Some(Vec2 { x: 1.0, y: 1.5 });
    let reference = survivor.reference.clone();
    doc.keymap = Some(KeymapConfiguration::default());
    doc.keymap.as_mut().unwrap().layers[0].bindings.insert(
        "matrix/main/r2c0".into(),
        KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    );
    engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: doc,
    });
    let operation = serde_json::from_value(
        serde_json::json!({"kind":"remove-matrix-row","matrixId":"main","row":1}),
    )
    .expect("Core must remove a row rather than only shrink the last row");
    let (_, doc) = scene(engine.handle(edit(1, EditPhase::Commit, operation)));
    assert_eq!(doc.matrices[0].rows, 2);
    let survivor = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r1c0")
        .unwrap();
    assert_eq!(survivor.reference, reference);
    assert_eq!(survivor.keycap, Some(Vec2 { x: 1.0, y: 1.5 }));
    assert_eq!(
        doc.keymap.as_ref().unwrap().layers[0].bindings["matrix/main/r1c0"],
        KeyBinding::KeyPress {
            keycode: "A".into()
        }
    );
    assert!(
        !doc.keymap.as_ref().unwrap().layers[0]
            .bindings
            .contains_key("matrix/main/r2c0")
    );
}

#[test]
fn unrelated_matrix_edit_never_resurrects_a_tombstoned_member_correspondence() {
    let (mut engine, doc) = linked_pair();
    let (_, deleted) = scene(engine.handle(edit(
        doc.revision,
        EditPhase::Commit,
        EditOperation::RemoveMatrixCells {
            matrix_id: "left-matrix".into(),
            cells: vec![(0, 1)],
        },
    )));
    let retired = |link: &LayoutMirrorLink| {
        link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        })
    };
    let link = deleted.layouts[1].mirror_link.as_ref().unwrap();
    assert!(!retired(link), "delete must retire the pair: {link:?}");
    let mut left = deleted.matrices[0].clone();
    left.edge_gap = Some(Vec2 { x: 6.0, y: 6.0 });
    let (_, edited) = scene(engine.handle(edit(
        deleted.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: left,
            definitions: None,
        },
    )));
    let link = edited.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !retired(link),
        "an unrelated edit must not resurrect the tombstoned pair: {link:?}"
    );
}

#[test]
fn unrelated_matrix_edit_keeps_a_local_exception_unassociated() {
    let (mut engine, mut doc) = linked_pair();
    let link = doc.layouts[1].mirror_link.as_mut().unwrap();
    link.correspondences.retain(|record| {
        !(record.kind == MirroredItemKind::Part && record.mirror_id == "matrix/right-matrix/r0c1")
    });
    link.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: "matrix/right-matrix/r0c1".into(),
        state: MirroredExceptionState::Local,
        removed_copper: None,
    });
    let (_, opened) = scene(engine.handle(CoreRequest::Open {
        id: "open-unassociated".into(),
        document: doc,
    }));
    let mut left = opened.matrices[0].clone();
    left.edge_gap = Some(Vec2 { x: 6.0, y: 6.0 });
    let (_, edited) = scene(engine.handle(edit(
        opened.revision,
        EditPhase::Commit,
        EditOperation::SetMatrix {
            matrix: left,
            definitions: None,
        },
    )));
    let link = edited.layouts[1].mirror_link.as_ref().unwrap();
    assert!(
        !link.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && (record.source_id.ends_with("left-matrix/r0c1")
                    || record.mirror_id.ends_with("right-matrix/r0c1"))
        }),
        "pairing must not infer a source for a local exception: {link:?}"
    );
}
