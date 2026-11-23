use super::apply::{affects_outline, apply};
use super::preview::{preview_electrical_loss_findings, preview_electrical_notice_id};
use super::*;

#[test]
fn assembly_save_and_placement_each_make_one_undo_entry() {
    use serde_json::{from_value, json};

    let mut start = ProjectDoc::empty("assembly-project", "Assemblies");
    start.boards.push(
        from_value(json!({
            "id": "pcb", "name": "PCB", "outlineIds": ["edge"], "partIds": [], "netIds": [],
            "thickness": 1.6
        }))
        .unwrap(),
    );
    start.outline.push(
        from_value(json!({
            "kind": "part-envelope", "id": "edge", "partIds": [], "margin": 2, "operation": "add"
        }))
        .unwrap(),
    );
    let mut core = CoreEngine::new();
    let initial = core.open(start).unwrap().document;
    let mut asset: Asset = from_value(json!({
        "id": "model", "name": "Model", "mediaType": "model/step", "sha256": "model-digest"
    }))
    .unwrap();
    let assembly: AssemblyDefinition = from_value(json!({
        "id": "recipe", "name": "Recipe", "members": [{
            "id": "member", "definitionId": "template", "side": "front", "models": [],
            "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}
        }]
    }))
    .unwrap();
    let mut definition: PartDefinition = from_value(json!({
        "id": "template", "name": "Switch", "kind": "switch", "pads": [],
        "courtyard": [{"x": -2, "y": -2}, {"x": 2, "y": -2}, {"x": 2, "y": 2}, {"x": -2, "y": 2}]
    }))
    .unwrap();
    let save = |assembly, definitions, assets| EditOperation::UpsertAssembly {
        assembly: Box::new(assembly),
        definitions,
        assets,
    };
    let commit = |core: &mut CoreEngine, base_revision, operation| {
        core.commit_edit(EditCommand {
            base_revision,
            transaction_id: "assembly".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        })
    };
    let saved = commit(
        &mut core,
        0,
        save(
            assembly.clone(),
            vec![definition.clone()],
            vec![asset.clone()],
        ),
    )
    .unwrap();
    assert_eq!(saved.scene.changed_ids, ["recipe"]);
    assert_eq!(saved.document.assemblies, [assembly.clone()]);

    definition.id = "placed/definition/member".into();
    let part: Part = from_value(json!({
        "id": "placed/member", "definitionId": definition.id, "reference": "Recipe member",
        "pose": {"at": {"x": 2, "y": 3}, "rotation": 0}, "side": "front"
    }))
    .unwrap();
    let place = |parts, assets| EditOperation::PlaceAssembly {
        board_id: "pcb".into(),
        placement_id: "placed".into(),
        definitions: vec![definition.clone()],
        parts,
        assets,
    };
    let placed = commit(&mut core, 1, place(vec![part.clone()], vec![asset.clone()])).unwrap();
    assert_eq!(placed.scene.changed_ids, ["pcb", "placed/member"]);
    assert_eq!(placed.document.boards[0].part_ids, ["placed/member"]);
    assert!(
        matches!(&placed.document.outline[0], OutlineFeature::PartEnvelope { part_ids, .. } if part_ids == &["placed/member"])
    );
    assert_eq!(
        placed.document.parts[0].properties.as_ref().unwrap()["assemblyId"],
        "placed"
    );
    assert_eq!(placed.document.assets, [asset.clone()]);

    asset.sha256.push('x');
    assert!(
        commit(&mut core, 2, save(assembly, vec![], vec![asset]))
            .unwrap_err()
            .contains("conflicts")
    );
    assert_eq!(core.snapshot().unwrap().document, placed.document);
    let without_revision = |mut document: ProjectDoc| {
        document.revision = 0;
        document
    };
    assert_eq!(
        without_revision(core.undo().unwrap().document),
        without_revision(saved.document)
    );
    assert_eq!(without_revision(core.undo().unwrap().document), initial);
    core.redo().unwrap();
    assert_eq!(
        without_revision(core.redo().unwrap().document),
        without_revision(placed.document)
    );
}

#[cfg(test)]
mod preview_electrical_loss_tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.parts.push(Part {
            anchor: None,
            keycap: None,
            outline: None,
            id: "part:one".into(),
            definition_id: "definition".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.nets.push(Net {
            id: "net:one".into(),
            name: "CTRL".into(),
            pins: vec![
                Pin {
                    part_id: "part:one".into(),
                    pad_id: "pad:1".into(),
                },
                Pin {
                    part_id: "part:one".into(),
                    pad_id: "pad-2".into(),
                },
            ],
        });
        document.boards.push(Board {
            id: "board:one".into(),
            name: "Right".into(),
            outline_ids: vec![],
            part_ids: vec!["part:one".into()],
            net_ids: vec!["net:one".into()],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    #[test]
    fn whole_net_removal_reports_each_distinct_pin_net_and_registration() {
        let accepted = document();
        let mut staged = accepted.clone();
        staged.nets.clear();
        staged.boards[0].net_ids.clear();

        let findings = preview_electrical_loss_findings(&accepted, &staged);

        assert_eq!(findings.len(), 4);
        let messages: BTreeSet<_> = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect();
        assert!(
            messages
                .contains("Preview removes pad pad:1 on U1 (part:one) from net CTRL (net:one).")
        );
        assert!(
            messages
                .contains("Preview removes pad pad-2 on U1 (part:one) from net CTRL (net:one).")
        );
        assert!(messages.contains("Preview removes net CTRL (net:one)."));
        assert!(
            messages.contains("Preview removes net CTRL (net:one) from board Right (board:one).")
        );
        assert!(findings.iter().all(|finding| {
            finding.id.starts_with("preview:electrical:")
                && finding.severity == Severity::Info
                && finding.scope == Scope::Pcb
        }));
        assert_eq!(
            findings,
            preview_electrical_loss_findings(&accepted, &staged)
        );
    }

    #[test]
    fn retained_net_and_board_registration_report_only_the_removed_pin() {
        let accepted = document();
        let mut staged = accepted.clone();
        staged.nets[0].pins.retain(|pin| pin.pad_id != "pad:1");

        let findings = preview_electrical_loss_findings(&accepted, &staged);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "Preview removes pad pad:1 on U1 (part:one) from net CTRL (net:one)."
        );
        assert!(preview_electrical_loss_findings(&accepted, &accepted).is_empty());
    }

    #[test]
    fn repeated_identities_are_deduplicated_and_notice_ids_are_collision_safe() {
        let mut accepted = document();
        let repeated_pin = accepted.nets[0].pins[0].clone();
        accepted.nets[0].pins.push(repeated_pin);
        accepted.boards[0].net_ids.push("net:one".into());
        let mut staged = accepted.clone();
        staged.nets.clear();
        staged.boards[0].net_ids.clear();

        let findings = preview_electrical_loss_findings(&accepted, &staged);
        let ids: BTreeSet<_> = findings.iter().map(|finding| finding.id.as_str()).collect();
        assert_eq!(findings.len(), 4);
        assert_eq!(ids.len(), findings.len());
        assert_ne!(
            preview_electrical_notice_id("pin", &["a:b", "c"]),
            preview_electrical_notice_id("pin", &["a", "b:c"])
        );
        assert_ne!(
            preview_electrical_notice_id("pin", &["é", "pad"]),
            preview_electrical_notice_id("pin", &["e", "\u{301}pad"])
        );
    }
}

#[cfg(test)]
mod matrix_protocol_tests {
    use super::*;
    use serde_json::json;

    fn draft(id: &str, rows: u64) -> serde_json::Value {
        json!({ "id": id, "rows": rows, "columns": 2, "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 0.0, "y": 0.0}, "definitionId": "missing", "partIds": [] })
    }

    #[test]
    fn draft_request_is_stateless_and_invalid_batches_fail_atomically() {
        let mut engine = CoreEngine::default();
        let request = json!({ "kind": "project-matrices", "id": "p", "baseRevision": 0, "matrices": [draft("ok", 1)] });
        let reply: CoreReply = serde_json::from_str(&engine.request(&request.to_string())).unwrap();
        let CoreReply::MatrixProjections {
            revision,
            matrix_scenes,
            ..
        } = reply
        else {
            panic!("expected projections")
        };
        assert_eq!(revision, 0);
        assert_eq!(matrix_scenes.len(), 1);
        let invalid = json!({ "kind": "project-matrices", "id": "p2", "baseRevision": 0, "matrices": [draft("ok", 1), draft("bad", 0)] });
        let reply: CoreReply = serde_json::from_str(&engine.request(&invalid.to_string())).unwrap();
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(engine.document.revision, 0);
        assert!(engine.undo.is_empty());
        assert!(engine.redo.is_empty());
    }
}

#[cfg(test)]
mod set_wiring_mode_tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.boards.push(Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    fn set_mode(mode: electrical::ElectricalMode) -> EditOperation {
        EditOperation::SetWiringMode {
            board_id: "left".into(),
            mode,
        }
    }

    #[test]
    fn setting_direct_creates_board_configuration_and_reports_only_the_board() {
        let mut document = document();

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Direct),
        )
        .unwrap();

        assert_eq!(changed, ["left"]);
        let configuration = document.hardware.unwrap().boards.pop().unwrap();
        assert_eq!(
            configuration,
            ElectricalBoardConfiguration {
                board_id: "left".into(),
                mode: electrical::ElectricalMode::Direct,
                ..Default::default()
            }
        );
    }

    #[test]
    fn setting_the_current_mode_is_a_no_op() {
        let mut document = document();
        let operation = set_mode(electrical::ElectricalMode::Direct);
        apply(&mut document, &[], &operation).unwrap();
        let before = document.clone();

        let changed = apply(&mut document, &[], &operation).unwrap();

        assert!(changed.is_empty());
        assert_eq!(document, before);
    }

    #[test]
    fn changing_mode_preserves_the_board_configuration() {
        let mut document = document();
        let mut configuration = ElectricalBoardConfiguration {
            board_id: "left".into(),
            controller_part_id: Some("controller".into()),
            mode: electrical::ElectricalMode::Matrix,
            ..Default::default()
        };
        configuration.locks.insert("row".into(), "P1".into());
        document
            .hardware
            .get_or_insert_with(Default::default)
            .boards
            .push(configuration.clone());

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Direct),
        )
        .unwrap();

        assert_eq!(changed, ["left"]);
        configuration.mode = electrical::ElectricalMode::Direct;
        assert_eq!(document.hardware.unwrap().boards, [configuration]);
    }

    #[test]
    fn setting_the_default_mode_is_a_no_op() {
        let mut document = document();

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Matrix),
        )
        .unwrap();

        assert!(changed.is_empty());
        assert!(document.hardware.is_none());
    }

    #[test]
    fn setting_mode_for_an_unknown_board_is_an_edit_error() {
        let mut document = document();
        let operation = EditOperation::SetWiringMode {
            board_id: "missing".into(),
            mode: electrical::ElectricalMode::Direct,
        };

        assert_eq!(
            apply(&mut document, &[], &operation),
            Err("Unknown board missing".into())
        );
    }

    #[test]
    fn setting_wiring_mode_does_not_recompute_outlines() {
        assert!(!affects_outline(&set_mode(
            electrical::ElectricalMode::Direct
        )));
    }
}

#[cfg(test)]
mod mechanical_edit_engine_tests {
    use super::*;
    use crate::mechanical::change::MechanicalDimension;
    use crate::mechanical::{MechanicalChange, MechanicalEdit, MechanicalTarget};

    fn board() -> Board {
        Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(serde_json::json!({
            "boardId": board_id, "method": "printed", "mount": "rigid",
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4, "batteryHeight": 0.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "mounts": [], "closureMounts": [], "profiles": []
        }))
        .unwrap()
    }

    fn template() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    fn edit_op(board_id: &str, change: MechanicalChange, mounting_hole: bool) -> EditOperation {
        EditOperation::ApplyMechanicalEdit {
            edit: Box::new(MechanicalEdit {
                target: MechanicalTarget::Board {
                    board_id: board_id.into(),
                },
                change,
                mounting_hole: mounting_hole.then(|| Box::new(template())),
            }),
        }
    }

    fn command(base: u64, operation: EditOperation) -> CoreRequest {
        CoreRequest::Edit {
            id: "mechanical".into(),
            command: EditCommand {
                base_revision: base,
                transaction_id: "mechanical".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        }
    }

    fn open(engine: &mut CoreEngine, document: ProjectDoc) -> SceneDelta {
        match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document,
        }) {
            CoreReply::Scene { scene, .. } => scene,
            reply => panic!("open failed: {reply:?}"),
        }
    }

    fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
        match reply {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            reply => panic!("expected scene: {reply:?}"),
        }
    }

    fn generated_parts(document: &ProjectDoc) -> Vec<&str> {
        document
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .filter(|id| id.starts_with("case-closure/"))
            .collect()
    }

    fn generated_definitions(document: &ProjectDoc) -> Vec<&str> {
        document
            .definitions
            .iter()
            .map(|definition| definition.id.as_str())
            .filter(|id| id.starts_with("assembly-closure/definition/"))
            .collect()
    }

    #[test]
    fn explicit_board_clear_preserves_other_boards_configuration() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let mut other = board();
        other.id = "other".into();
        document.boards.push(other);
        document.mechanical = Some(configuration("board"));
        let before = document.clone();

        // Ticket 37: raw SetMechanical is deleted; Disable is the live typed clear.
        apply(
            &mut document,
            &[],
            &edit_op("other", MechanicalChange::Disable, false),
        )
        .expect("clearing another board is an unchanged explicit target");
        assert_eq!(
            document, before,
            "the existing slot belongs to a different board"
        );

        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Disable, false),
        )
        .expect("clearing the slot's own board removes its configuration");
        assert!(document.mechanical.is_none());
    }

    #[test]
    fn targeted_instance_set_mechanical_requires_preparation_before_activation() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let before = document.clone();

        // Ticket 37: raw SetMechanical is deleted; an instance-targeted typed
        // edit through raw apply must not activate instance configuration.
        let refusal = apply(
            &mut document,
            &[],
            &EditOperation::ApplyMechanicalEdit {
                edit: Box::new(MechanicalEdit {
                    target: MechanicalTarget::Instance {
                        instance_id: "physical-half".into(),
                    },
                    change: MechanicalChange::Disable,
                    mounting_hole: None,
                }),
            },
        )
        .expect_err("a missing instance stays refused through raw apply");

        assert!(refusal.contains("Physical instances are unavailable"));
        assert_eq!(document, before);
    }

    #[test]
    fn explicit_board_set_mechanical_keeps_board_slot_semantics() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let mut other = board();
        other.id = "other".into();
        document.boards.push(other);
        // Ticket 37: raw SetMechanical is deleted; Enable/Disable are the live
        // typed board-slot writes.
        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Enable, false),
        )
        .expect("an explicit Board target uses the existing board slot");
        assert_eq!(
            document
                .mechanical
                .as_ref()
                .map(|item| item.board_id.as_str()),
            Some("board")
        );

        let before = document.clone();
        apply(
            &mut document,
            &[],
            &edit_op("other", MechanicalChange::Disable, false),
        )
        .expect("disabling another board leaves this slot alone");
        assert_eq!(
            document, before,
            "a target mismatch leaves the document alone"
        );

        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Disable, false),
        )
        .expect("disabling the slot's own board clears its configuration");
        assert!(document.mechanical.is_none());
    }

    /// Case 3: a no-op mechanical edit adds no history step and no revision bump.
    #[test]
    fn a_no_op_mechanical_edit_keeps_revision_and_history() {
        let mut engine = CoreEngine::new();
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        document.mechanical = Some(configuration("board"));
        open(&mut engine, document);

        let revision = engine.document.revision;
        let undo = engine.undo.len();
        let redo = engine.redo.len();
        let reply = engine.handle(command(
            revision,
            edit_op(
                "board",
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 2.0,
                },
                false,
            ),
        ));
        let CoreReply::Scene {
            scene, document, ..
        } = reply
        else {
            panic!("expected scene")
        };
        assert!(scene.changed_ids.is_empty());
        assert_eq!(document.revision, revision, "no revision bump");
        assert_eq!(engine.document.revision, revision);
        assert_eq!(engine.undo.len(), undo, "no undo entry");
        assert_eq!(engine.redo.len(), redo, "redo is not cleared");
    }

    /// Case 2: generated clearance parts, definitions and memberships appear in
    /// the same reply and disappear together on one Undo.
    #[test]
    fn closure_projection_lands_atomically_and_undoes_in_one_step() {
        let mut engine = CoreEngine::new();
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        let mut board = board();
        board.outline_ids = vec!["outline".into()];
        document.boards.push(board);
        // Absent closure mounts are initialized by the edit.
        document.mechanical = Some(configuration("board"));
        document.mechanical.as_mut().unwrap().closure_mounts = None;
        open(&mut engine, document);

        let revision = engine.document.revision;
        let (reply_scene, projected) = scene(engine.handle(command(
            revision,
            edit_op("board", MechanicalChange::InitializeClosures, true),
        )));

        let parts = generated_parts(&projected);
        let definitions = generated_definitions(&projected);
        assert!(!parts.is_empty(), "generated clearance parts appear");
        assert!(
            !definitions.is_empty(),
            "generated clearance definitions appear"
        );
        assert!(
            projected.boards[0]
                .part_ids
                .iter()
                .any(|id| id.starts_with("case-closure/")),
            "the board membership includes the generated parts"
        );
        for id in parts.iter().chain(definitions.iter()) {
            assert!(
                reply_scene.changed_ids.contains(&(*id).to_string()),
                "changed ids name generated {id}"
            );
        }
        assert!(reply_scene.changed_ids.contains(&"board".to_string()));

        let undo_reply = engine.handle(CoreRequest::Undo { id: "undo".into() });
        let (_, undone) = scene(undo_reply);
        assert!(
            generated_parts(&undone).is_empty(),
            "one Undo removes every generated part"
        );
        assert!(
            generated_definitions(&undone).is_empty(),
            "one Undo removes every generated definition"
        );
        assert!(
            undone.boards[0]
                .part_ids
                .iter()
                .all(|id| !id.starts_with("case-closure/")),
            "one Undo removes the generated memberships"
        );
        assert_eq!(
            undone.mechanical.as_ref().unwrap().closure_mounts,
            None,
            "one Undo restores the absent closure mounts"
        );
    }

    /// Case 6: a pure settings edit does not rebuild board outlines; the
    /// outline-version recovery snapshot is the observable.
    #[test]
    fn pure_settings_edits_preserve_the_outline_recovery_snapshot() {
        let mut source: ProjectDoc = serde_json::from_str(include_str!(
            "../../../tests/fixtures/synthetic-outline.json"
        ))
        .unwrap();
        source.format_version = crate::document_format::CURRENT_VERSION;
        // Use only two authored square courtyards with a 4 mm gap. This creates
        // an independent automatic bridge pocket for the recovery contract.
        source
            .parts
            .iter_mut()
            .find(|part| part.id == "main/SW2")
            .expect("the second synthetic switch exists")
            .pose
            .at = Vec2 { x: 24.0, y: 0.0 };
        let protected_parts = vec!["main/SW1".to_string(), "main/SW2".to_string()];
        let OutlineFeature::PartEnvelope {
            part_ids,
            margin,
            settings,
            ..
        } = &mut source.outline[0]
        else {
            unreachable!("the synthetic outline is a part envelope")
        };
        *part_ids = protected_parts.clone();
        *margin = 1.0;
        settings.bridge_width = 10.0;
        source.boards[0].part_ids = protected_parts;
        // Protect a generated gap so opening the project writes a recovery snapshot.
        let mut probe = CoreEngine::new();
        let probe_scene = open(&mut probe, source.clone());
        let gap = probe_scene.board_outline_scenes[0]
            .gaps
            .first()
            .cloned()
            .expect("the repaired outline exposes a gap");
        if let OutlineFeature::PartEnvelope { settings, .. } = &mut source.outline[0] {
            settings.repair = Some(OutlineRepairSettings {
                keep_gaps: vec![ProtectedOutlineGap {
                    id: gap.id,
                    points: gap.points,
                }],
                ..Default::default()
            });
        }
        source.mechanical = Some(configuration("main"));

        let mut engine = CoreEngine::new();
        open(&mut engine, source);
        assert!(
            engine
                .document
                .board_outlines
                .iter()
                .any(|state| state.generated_last_valid.is_some()),
            "the protected outline has a recovery snapshot to observe"
        );
        let revision = engine.document.revision;
        let before = engine.document.board_outlines.clone();
        let (_, document) = scene(engine.handle(command(
            revision,
            edit_op(
                "main",
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 2.5,
                },
                false,
            ),
        )));
        assert_eq!(document.revision, revision + 1, "the settings edit commits");
        assert_eq!(
            document.board_outlines, before,
            "a pure settings edit leaves the outline recovery snapshot untouched"
        );
    }
}
