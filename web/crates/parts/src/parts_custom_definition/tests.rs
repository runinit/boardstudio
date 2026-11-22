#[cfg(not(target_arch = "wasm32"))]
use super::edit::{next_pad_number, parse_number};
use super::*;
use boardstudio_application::{SessionEpoch, SnapshotToken};
use boardstudio_model::model::{ProjectDoc, SceneDelta};
use std::sync::Arc;

fn definition(pads: serde_json::Value) -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":"custom","name":"Custom","kind":"custom","courtyard":[],"pads":pads
    }))
    .expect("definition")
}
fn snapshot(definition: PartDefinition) -> AcceptedSnapshot {
    let mut document = ProjectDoc::empty("project", "Fixture");
    document.definitions.push(definition);
    snapshot_doc(document)
}
fn snapshot_doc(document: ProjectDoc) -> AcceptedSnapshot {
    let scene: SceneDelta = serde_json::from_value(serde_json::json!({
        "revision":0,"transactionId":"fixture","changedIds":[],"transforms":[],
        "matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],
        "readiness":{"layout":true,"outline":true,"pcb":true,"case":false}
    }))
    .expect("scene");
    AcceptedSnapshot {
        session_epoch: SessionEpoch(1),
        token: SnapshotToken(7),
        document: Arc::new(document),
        scene: Arc::new(scene),
    }
}
fn scope() -> Scope {
    Scope {
        session_epoch: SessionEpoch(1),
        document_id: "project".into(),
        board_id: "board".into(),
        instance_id: None,
    }
}
fn pad(id: &str, number: &str) -> serde_json::Value {
    serde_json::json!({ "id":id,"number":number,"at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle" })
}

#[test]
fn accepted_pad_rename_preserves_the_row_through_scalar_refresh() {
    let mut pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
    let mut rows = PadRowKeys::default();
    let before = rows.for_pads("owner", &pads);
    pads[0].id = "renamed".into();
    pads[0].at.y = 4.0;
    assert_eq!(rows.for_pads("owner", &pads), before);
}

#[test]
fn pad_list_edits_preserve_survivors_and_retire_deleted_rows() {
    let mut pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
    let mut rows = PadRowKeys::default();
    let before = rows.for_pads("owner", &pads);
    let removed = pads.remove(0);
    assert_eq!(rows.for_pads("owner", &pads), vec![before[1]]);
    pads.push(removed);
    let restored = rows.for_pads("owner", &pads);
    assert_eq!(restored[0], before[1]);
    assert_ne!(restored[1], before[0]);
    pads.swap(0, 1);
    assert_eq!(
        rows.for_pads("owner", &pads),
        vec![restored[1], restored[0]]
    );
}

#[test]
fn ambiguous_replacements_and_owner_switches_retire_pad_rows() {
    let pads = definition(serde_json::json!([pad("a", "1"), pad("b", "2")])).pads;
    let replacement = definition(serde_json::json!([pad("c", "1"), pad("d", "2")])).pads;
    let mut rows = PadRowKeys::default();
    let before = rows.for_pads("owner", &pads);
    let replaced = rows.for_pads("owner", &replacement);
    assert!(replaced.iter().all(|key| !before.contains(key)));
    let next_owner = rows.for_pads("other-owner", &replacement);
    assert!(next_owner.iter().all(|key| !replaced.contains(key)));
}

#[test]
fn duplicate_pad_ids_never_share_or_transfer_a_row_lifetime() {
    let pads = definition(serde_json::json!([pad("a", "1"), pad("a", "2")])).pads;
    let mut rows = PadRowKeys::default();
    let before = rows.for_pads("owner", &pads);
    assert_ne!(before[0], before[1]);
    let refreshed = rows.for_pads("owner", &pads);
    assert!(refreshed.iter().all(|key| !before.contains(key)));
}
#[test]
fn field_admission_ignores_departed_owners_but_not_newer_revisions() {
    let current = snapshot(definition(serde_json::json!([pad("a", "1")])));
    let owner_scope = scope();
    let capture = DefinitionPanelCapture::new(
        &current,
        Some(owner_scope.clone()),
        &current.document.definitions[0],
    );
    let selected = Some((Some(owner_scope.clone()), "custom".into()));
    assert!(capture.owner_matches(&current, Some(owner_scope.clone()), selected.clone()));
    assert!(
        !capture.owner_matches(
            &current,
            Some(owner_scope.clone()),
            Some((Some(owner_scope.clone()), "other".into()))
        ),
        "a stale selection cannot commit field drafts"
    );
    assert!(
        !capture.owner_matches(&current, None, selected.clone()),
        "a lost scope cannot commit field drafts"
    );

    // A newer accepted revision keeps the owner live: field edits queue freely and
    // resolve against the document accepted when they run (ADR-0005).
    let mut newer_document = current.document.as_ref().clone();
    newer_document.revision += 1;
    let newer = AcceptedSnapshot {
        token: SnapshotToken(current.token.0 + 1),
        document: Arc::new(newer_document),
        ..current.clone()
    };
    assert!(capture.owner_matches(&newer, Some(owner_scope), selected));
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use boardstudio_application::{Event, TerminalOutcome};
    use boardstudio_web_runtime::{operation_outcomes::OutcomeSlot, runtime::Runtime};
    use std::rc::Rc;

    /// Drive the native test Runtime (real Session and Core, in-memory saves) and report
    /// each field edit's settlement.
    struct Driver {
        runtime: Rc<Runtime>,
        next_seed: u64,
    }

    impl Driver {
        fn open(document: ProjectDoc) -> Self {
            let runtime = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
            runtime.submit(Event::Open {
                operation_id: runtime.operation(),
                document,
            });
            assert!(runtime.model().accepted.is_some());
            Self {
                runtime,
                next_seed: 2,
            }
        }

        /// Submit one field edit; the pad id seed is explicit so AddPad identities are
        /// deterministic in assertions. The slot stays empty while a gate holds the edit.
        fn submit_at(&self, seed: u64, edit: DefinitionEdit) -> OutcomeSlot {
            let operation = self.runtime.operation();
            let slot = self.runtime.observe_operation(operation);
            self.runtime.submit(Event::ResolveEdit {
                operation_id: operation,
                label: "parts-definition-field".into(),
                resolver: definition_field_resolver("custom".into(), edit, seed),
            });
            slot
        }

        fn resolve_at(&mut self, seed: u64, edit: DefinitionEdit) -> Vec<TerminalOutcome> {
            self.submit_at(seed, edit)
                .borrow()
                .iter()
                .cloned()
                .collect()
        }

        fn resolve(&mut self, edit: DefinitionEdit) -> Vec<TerminalOutcome> {
            let seed = self.next_seed;
            self.next_seed += 1;
            self.resolve_at(seed, edit)
        }

        /// One landed field edit: settles Completed and moves the revision by one.
        fn lands_one_edit(&mut self, edit: DefinitionEdit) {
            let before = self.accepted().revision;
            assert_eq!(
                self.resolve(edit),
                vec![TerminalOutcome::Completed],
                "the field edit lands"
            );
            assert_eq!(self.accepted().revision, before + 1);
        }

        fn accepted(&self) -> ProjectDoc {
            self.runtime
                .model()
                .accepted
                .expect("accepted document")
                .document
                .as_ref()
                .clone()
        }

        fn custom(&self) -> PartDefinition {
            self.accepted()
                .definitions
                .iter()
                .find(|definition| definition.id == "custom")
                .unwrap()
                .clone()
        }
    }

    #[test]
    fn every_supported_field_resolves_one_individual_accepted_edit() {
        let mut initial = definition(serde_json::json!([pad("a", "1")]));
        initial.pads[0].at.x = 1.0;
        initial.courtyard = vec![
            Vec2 { x: -5.0, y: -3.0 },
            Vec2 { x: 5.0, y: -3.0 },
            Vec2 { x: 5.0, y: 3.0 },
            Vec2 { x: -5.0, y: 3.0 },
        ];
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(initial);
        let mut driver = Driver::open(document);

        driver.lands_one_edit(DefinitionEdit::Kind(PartKind::Connector));
        assert!(matches!(driver.custom().kind, PartKind::Connector));

        driver.lands_one_edit(DefinitionEdit::CourtyardWidth("14".into()));
        assert_eq!(
            courtyard_bounds(&driver.custom().courtyard),
            (14.0, 6.0, Vec2 { x: 0.0, y: 0.0 })
        );

        driver.lands_one_edit(DefinitionEdit::CourtyardHeight("8".into()));
        assert_eq!(courtyard_bounds(&driver.custom().courtyard).1, 8.0);

        driver.lands_one_edit(DefinitionEdit::PadNumber {
            pad_id: "a".into(),
            value: "9".into(),
        });
        assert_eq!(driver.custom().pads[0].number, "9");

        driver.lands_one_edit(DefinitionEdit::PadCoordinate {
            pad_id: "a".into(),
            axis: Axis::Y,
            value: "2".into(),
        });
        assert_eq!(driver.custom().pads[0].at.y, 2.0);

        driver.lands_one_edit(DefinitionEdit::PadSize {
            pad_id: "a".into(),
            axis: Axis::X,
            value: "3".into(),
        });
        driver.lands_one_edit(DefinitionEdit::PadSize {
            pad_id: "a".into(),
            axis: Axis::Y,
            value: "4".into(),
        });
        assert_eq!(driver.custom().pads[0].size, Vec2 { x: 3.0, y: 4.0 });

        driver.lands_one_edit(DefinitionEdit::PadDrill {
            pad_id: "a".into(),
            value: "0.5".into(),
        });
        assert_eq!(driver.custom().pads[0].drill, Some(0.5));

        driver.lands_one_edit(DefinitionEdit::PadDrill {
            pad_id: "a".into(),
            value: String::new(),
        });
        assert_eq!(driver.custom().pads[0].drill, None);

        driver.lands_one_edit(DefinitionEdit::PadShape {
            pad_id: "a".into(),
            shape: PadShape::Oval,
        });
        assert!(matches!(driver.custom().pads[0].shape, PadShape::Oval));
    }

    #[test]
    fn an_unchanged_field_edit_lands_without_moving_the_revision() {
        let mut initial = definition(serde_json::json!([pad("a", "1")]));
        initial.pads[0].at.x = 1.0;
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(initial);
        let mut driver = Driver::open(document);
        let before = driver.accepted();

        assert_eq!(
            driver.resolve(DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "1".into(),
            }),
            vec![TerminalOutcome::Completed]
        );
        assert_eq!(
            driver.resolve(DefinitionEdit::Kind(PartKind::Custom)),
            vec![TerminalOutcome::Completed]
        );
        let after = driver.accepted();
        assert_eq!(after.revision, before.revision);
    }

    #[test]
    fn add_pad_keeps_free_default_and_repairs_collision_as_one_edit() {
        // Expected-red reference oracle: pads.len()+1 is 3 for [1, 3], so literal
        // reference code creates a duplicate. The reviewed correction advances to 4.
        let pads: Vec<Pad> = vec![
            serde_json::from_value(pad("a", "1")).unwrap(),
            serde_json::from_value(pad("b", "3")).unwrap(),
        ];
        let old_reference_candidate = pads.len() + 1;
        assert_eq!(old_reference_candidate.to_string(), "3");
        assert!(
            pads.iter()
                .any(|pad| pad.number == old_reference_candidate.to_string())
        );
        assert_eq!(next_pad_number(&pads), "4");
        let ordinary = vec![
            serde_json::from_value(pad("a", "1")).unwrap(),
            serde_json::from_value(pad("b", "2")).unwrap(),
        ];
        assert_eq!(next_pad_number(&ordinary), "3");

        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition(serde_json::json!([
            pad("a", "1"),
            pad("b", "3")
        ])));
        let mut driver = Driver::open(document);
        assert_eq!(
            driver.resolve_at(9, DefinitionEdit::AddPad),
            vec![TerminalOutcome::Completed]
        );
        let pads = &driver.custom().pads;
        assert_eq!(pads.len(), 3);
        assert_eq!(pads[2].number, "4");
        assert_eq!(pads[2].id, "pad-9");
    }

    #[test]
    fn numeric_fields_keep_reference_blank_and_validation_semantics() {
        assert_eq!(parse_number("", false, "bad").unwrap(), 0.0);
        assert!(parse_number("", true, "bad").is_err());
        let mut document = ProjectDoc::empty("project", "Fixture");
        document
            .definitions
            .push(definition(serde_json::json!([pad("a", "1")])));
        let mut driver = Driver::open(document);
        assert_eq!(
            driver.resolve(DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "".into(),
            }),
            vec![TerminalOutcome::Completed]
        );
        assert_eq!(driver.custom().pads[0].at.x, 0.0);
    }

    #[test]
    fn field_edits_retire_vanished_or_ineligible_targets_with_reasons() {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition(serde_json::json!([
            pad("a", "1"),
            pad("b", "2")
        ])));
        let mut driver = Driver::open(document);
        assert_eq!(
            driver.resolve(DefinitionEdit::PadCoordinate {
                pad_id: "missing".into(),
                axis: Axis::X,
                value: "1".into(),
            }),
            vec![TerminalOutcome::Rejected(PAD_GONE.into())]
        );
        assert_eq!(
            driver.resolve(DefinitionEdit::PadId {
                pad_id: "a".into(),
                value: "b".into(),
            }),
            vec![TerminalOutcome::Rejected(PAD_IDS_UNIQUE.into())]
        );
        assert_eq!(
            driver.resolve(DefinitionEdit::PadNumber {
                pad_id: "a".into(),
                value: "2".into(),
            }),
            vec![TerminalOutcome::Rejected(PAD_NUMBERS_UNIQUE.into())]
        );

        let mut generator_document = ProjectDoc::empty("project", "Fixture");
        let mut generated = definition(serde_json::json!([pad("a", "1")]));
        generated.generator = Some(boardstudio_model::model::PartGenerator {
            source: "generator/source".into(),
            version: "1".into(),
            parameters: Default::default(),
        });
        generator_document.definitions.push(generated);
        let mut driver = Driver::open(generator_document);
        assert_eq!(
            driver.resolve(DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "1".into(),
            }),
            vec![TerminalOutcome::Rejected(GENERATOR_LOCKED.into())]
        );

        let mut imported_document = ProjectDoc::empty("project", "Fixture");
        let mut imported = definition(serde_json::json!([pad("a", "1")]));
        imported.kicad_source = Some(boardstudio_model::model::KicadSource {
            format_version: 1,
            source: "part.kicad_mod".into(),
        });
        imported_document.definitions.push(imported);
        let mut driver = Driver::open(imported_document);
        assert_eq!(
            driver.resolve(DefinitionEdit::PadCoordinate {
                pad_id: "a".into(),
                axis: Axis::X,
                value: "1".into(),
            }),
            vec![TerminalOutcome::Rejected(IMPORTED_PADS.into())]
        );

        let mut missing_document = ProjectDoc::empty("project", "Fixture");
        let mut other = definition(serde_json::json!([]));
        other.id = "other".into();
        missing_document.definitions.push(other);
        let mut driver = Driver::open(missing_document);
        assert_eq!(
            driver.resolve(DefinitionEdit::Kind(PartKind::Connector)),
            vec![TerminalOutcome::Rejected(DEFINITION_GONE.into())]
        );
    }

    #[test]
    fn pad_rename_and_removal_keep_instance_pin_relationships_scoped() {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.definitions.push(definition(serde_json::json!([
            pad("old", "1"),
            pad("keep", "2")
        ])));
        document.parts = serde_json::from_value(serde_json::json!([
            {"id":"instance-a","definitionId":"custom","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"instance-b","definitionId":"custom","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"other","definitionId":"other-definition","reference":"R1","pose":{"at":{"x":2.0,"y":0.0},"rotation":0.0},"side":"front"}
        ])).unwrap();
        document.nets = serde_json::from_value(serde_json::json!([
            {"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"},{"partId":"other","padId":"old"}]},
            {"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}
        ])).unwrap();
        let mut driver = Driver::open(document);

        driver.lands_one_edit(DefinitionEdit::PadId {
            pad_id: "old".into(),
            value: "renamed".into(),
        });
        let renamed = driver.accepted();
        assert_eq!(renamed.definitions[0].pads[0].id, "renamed");
        assert_eq!(renamed.nets[0].pins[0].pad_id, "renamed");
        assert_eq!(renamed.nets[0].pins[1].pad_id, "renamed");
        assert_eq!(renamed.nets[0].pins[2].pad_id, "old");
        assert_eq!(renamed.nets[1].pins[0].pad_id, "keep");

        driver.lands_one_edit(DefinitionEdit::RemovePad {
            pad_id: "renamed".into(),
        });
        let removed = driver.accepted();
        assert_eq!(
            removed.definitions[0]
                .pads
                .iter()
                .map(|pad| pad.id.as_str())
                .collect::<Vec<_>>(),
            vec!["keep"]
        );
        assert!(
            removed.nets[0]
                .pins
                .iter()
                .all(|pin| pin.pad_id != "renamed" || pin.part_id == "other")
        );
        assert_eq!(removed.nets[1].pins[0].pad_id, "keep");
        assert_eq!(removed.parts.len(), 3);
    }
}
