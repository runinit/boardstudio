use super::*;

fn mechanical_fixture(fit_ids: &[&str]) -> ProjectDoc {
    let mut document = fixture();
    document.boards.push(Board {
        id: "main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut configuration: MechanicalConfiguration = serde_json::from_value(serde_json::json!({
        "boardId":"main","method":"printed","mount":"rigid",
        "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
        "bottomFoamThickness":0.4,"batteryHeight":4.0,"bottomThickness":3.0,
        "plateToPcb":3.5,"wallThickness":2.0,"clearance":0.3,
        "openings":[],"mounts":[],"closureMounts":[],"profiles":[]
    }))
    .unwrap();
    configuration.critical_fits = Some(
        fit_ids
            .iter()
            .map(|id| MechanicalCriticalFit {
                id: (*id).into(),
                part_id: "case-closure/part".into(),
                label: "Critical dimension".into(),
                from: Vec2 { x: 0.0, y: 0.0 },
                to: Vec2 { x: 10.0, y: 0.0 },
                tolerance: "±0.2 mm".into(),
            })
            .collect(),
    );
    document.mechanical = Some(configuration);
    document
}

fn mechanical_template() -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number": "", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

fn mechanical_command(transaction: &str, change: MechanicalChange) -> EditCommand {
    EditCommand {
        base_revision: 0,
        transaction_id: transaction.into(),
        phase: EditPhase::Commit,
        target_ids: vec!["main".into()],
        operation: EditOperation::ApplyMechanicalEdit {
            edit: Box::new(MechanicalEdit {
                target: boardstudio_core::mechanical::MechanicalTarget::Board {
                    board_id: "main".into(),
                },
                change,
                mounting_hole: Some(Box::new(mechanical_template())),
            }),
        },
    }
}

fn add_fit_command(transaction: &str) -> EditCommand {
    mechanical_command(
        transaction,
        MechanicalChange::AddCriticalFit {
            part_id: "case-closure/part".into(),
        },
    )
}

fn remove_fit_command(transaction: &str, fit_id: &str) -> EditCommand {
    mechanical_command(
        transaction,
        MechanicalChange::RemoveCriticalFit {
            fit_id: fit_id.into(),
        },
    )
}

/// A setter authored against `fit_id`. It retires when that exact identity is gone
/// instead of redirecting its value at whatever later addition reused the place.
fn stale_fit_label_resolver(fit_id: &str, label: &str) -> EditResolver {
    let fit_id = fit_id.to_owned();
    let label = label.to_owned();
    EditResolver::new("critical-fit-label", move |accepted: &AcceptedSnapshot| {
        let edit = MechanicalEdit {
            target: boardstudio_core::mechanical::MechanicalTarget::Board {
                board_id: "main".into(),
            },
            change: MechanicalChange::SetCriticalFitText {
                fit_id: fit_id.clone(),
                field: MechanicalCriticalFitTextField::Label,
                value: label.clone(),
            },
            mounting_hole: Some(Box::new(mechanical_template())),
        };
        // The production controller runs Core's read-only evaluator first: an
        // Unchanged or Retire answer settles locally; a removed target the evaluator
        // cannot classify reaches Core as a rejected edit.
        match boardstudio_core::mechanical::evaluate_edit(
            &accepted.document,
            &accepted.scene.board_contours,
            &edit,
        ) {
            Ok(boardstudio_core::mechanical::MechanicalEditDisposition::Unchanged) => {
                Resolution::Unchanged
            }
            Ok(boardstudio_core::mechanical::MechanicalEditDisposition::Retire) => {
                Resolution::Retire("The mechanical target is no longer available.".into())
            }
            _ => Resolution::submit(
                vec!["main".into()],
                EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(edit),
                },
            ),
        }
    })
}

fn fit_labels(document: &ProjectDoc) -> Vec<(String, String)> {
    document
        .mechanical
        .as_ref()
        .and_then(|configuration| configuration.critical_fits.as_ref())
        .into_iter()
        .flatten()
        .map(|fit| (fit.id.clone(), fit.label.clone()))
        .collect()
}

#[test]
fn queued_stale_fit_setter_retires_after_remove_add_first_gap_reuse() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: mechanical_fixture(&["case-critical-fit-1"]),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(
        OperationId(2),
        remove_fit_command("remove-fit", "case-critical-fit-1"),
    ));
    assert!(has_core_request(&effects), "the removal starts immediately");
    let queued = session.submit(resolved_edit(OperationId(3), add_fit_command("add-fit")));
    assert!(
        !has_core_request(&queued),
        "the addition is queued behind it"
    );
    let queued = session.submit(Event::ResolveEdit {
        operation_id: OperationId(4),
        label: "stale-critical-fit-label".into(),
        resolver: stale_fit_label_resolver("case-critical-fit-1", "stale value"),
    });
    assert!(
        !has_core_request(&queued),
        "the stale setter is queued behind the replacement sequence"
    );

    let (_, add_effects) = settle_core_and_save(&mut session, &mut engine, effects);
    assert!(
        has_core_request(&add_effects),
        "the queued addition runs after the removal"
    );
    let (_, stale_effects) = settle_core_and_save(&mut session, &mut engine, add_effects);
    let (request_id, epoch, request) = core_effect(&stale_effects);
    let reply = engine.handle(request);
    assert!(
        matches!(reply, CoreReply::Error { .. }),
        "the stale setter must be rejected: {reply:?}"
    );
    let resolved = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&resolved, OperationId(4));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none());

    let accepted = session.read_model().accepted.clone().unwrap();
    let labels = fit_labels(&accepted.document);
    assert_eq!(
        labels,
        [("case-critical-fit-2".into(), "Critical dimension".into())]
    );
}

#[test]
fn queued_stale_fit_setter_retires_after_undo_then_add() {
    let mut session = Session::new(CoreEngine::validate_open);
    let mut engine = CoreEngine::new();
    let effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: mechanical_fixture(&["case-critical-fit-1"]),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(OperationId(2), add_fit_command("first-add")));
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);
    let accepted = session.read_model().accepted.clone().unwrap();
    assert!(
        fit_labels(&accepted.document)
            .iter()
            .any(|(id, _)| id == "case-critical-fit-2")
    );

    let effects = session.submit(Event::Undo {
        operation_id: OperationId(3),
    });
    let (_, _) = settle_core_and_save(&mut session, &mut engine, effects);

    let effects = session.submit(resolved_edit(OperationId(4), add_fit_command("second-add")));
    assert!(has_core_request(&effects));
    let queued = session.submit(Event::ResolveEdit {
        operation_id: OperationId(5),
        label: "stale-critical-fit-label".into(),
        resolver: stale_fit_label_resolver("case-critical-fit-2", "stale value"),
    });
    assert!(!has_core_request(&queued));

    let (_, stale_effects) = settle_core_and_save(&mut session, &mut engine, effects);
    let (request_id, epoch, request) = core_effect(&stale_effects);
    let reply = engine.handle(request);
    assert!(
        matches!(reply, CoreReply::Error { .. }),
        "the stale setter must be rejected: {reply:?}"
    );
    let resolved = session.complete(Completion::Core {
        request_id,
        executor_epoch: epoch,
        reply: Box::new(reply),
    });
    let (outcome, landing) = settled_landing(&resolved, OperationId(5));
    assert!(matches!(outcome, TerminalOutcome::Rejected(_)));
    assert!(landing.is_none());

    let accepted = session.read_model().accepted.clone().unwrap();
    assert_eq!(
        fit_labels(&accepted.document),
        [
            ("case-critical-fit-1".into(), "Critical dimension".into()),
            ("case-critical-fit-3".into(), "Critical dimension".into()),
        ],
        "undo/add must mint a fresh identity and the stale setter must not redirect"
    );
}
