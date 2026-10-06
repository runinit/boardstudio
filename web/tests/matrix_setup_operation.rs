#[path = "../src/matrix_setup_operation.rs"]
mod matrix_setup_operation;

use boardstudio_application::{
    Completion, Durability, Effect, Event, ExecutorEpoch, OperationId, RequestId, SaveResult,
    SelectionMode, Session, TerminalOutcome,
};
use boardstudio_core::{CoreEngine, model::*};
use matrix_setup_operation::{
    MatrixSetupPreset, MatrixSetupRequest, next_matrix_id, prepare_matrix,
};
use std::collections::BTreeMap;

fn definition(id: &str, source: &str, kind: PartKind) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: BTreeMap::new(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: Some(PartGenerator {
            source: source.into(),
            version: "fixture".into(),
            parameters: BTreeMap::new(),
        }),
    }
}

fn catalogue() -> Vec<PartDefinition> {
    vec![
        definition(
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/diode_tht_sod123",
            "ceoloide/diode_tht_sod123",
            PartKind::Passive,
        ),
        definition(
            "ergogen:ceoloide/led_sk6812mini-e",
            "ceoloide/led_sk6812mini-e",
            PartKind::Passive,
        ),
    ]
}

fn session_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("matrix-session", "Matrix session");
    document.boards.push(Board {
        id: "board-main".into(),
        name: "Main".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn session_core_effect(effects: &[Effect]) -> (RequestId, ExecutorEpoch, CoreRequest) {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => Some((*request_id, *executor_epoch, (**request).clone())),
            _ => None,
        })
        .expect("one Core operation")
}

fn complete_session_core(
    session: &mut Session,
    engine: &mut CoreEngine,
    effects: Vec<Effect>,
) -> Vec<Effect> {
    let (request_id, executor_epoch, request) = session_core_effect(&effects);
    session.complete(Completion::Core {
        request_id,
        executor_epoch,
        reply: Box::new(engine.handle(request)),
    })
}

fn persist_session_committed(session: &mut Session, effects: Vec<Effect>) -> Vec<Effect> {
    let (save_attempt_id, document) = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Persist {
                save_attempt_id,
                document,
                ..
            } => Some((*save_attempt_id, (**document).clone())),
            _ => None,
        })
        .expect("the accepted edit schedules persistence");
    assert_eq!(
        session.read_model().durability,
        Durability::Saving {
            revision: document.revision
        }
    );
    session.complete(Completion::Persist {
        save_attempt_id,
        result: SaveResult::Committed,
    })
}

fn open_session_saved(
    session: &mut Session,
    engine: &mut CoreEngine,
    document: ProjectDoc,
    id: u64,
) {
    let effects = session.submit(Event::Open {
        operation_id: OperationId(id),
        document,
    });
    let effects = complete_session_core(session, engine, effects);
    persist_session_committed(session, effects);
    let model = session.read_model();
    assert_eq!(model.lifecycle, boardstudio_application::Lifecycle::Ready);
    assert!(model.accepted.is_some());
    assert!(matches!(model.durability, Durability::Saved { .. }));
}

fn session_history_action(session: &mut Session, engine: &mut CoreEngine, event: Event) {
    let effects = session.submit(event);
    let effects = complete_session_core(session, engine, effects);
    persist_session_committed(session, effects);
}

#[test]
fn matrix_setup_builds_the_reference_mx_rgb_cell_recipe_and_owned_definition_snapshots() {
    let prepared = prepare_matrix(
        "matrix-created".into(),
        "board-main".into(),
        MatrixSetupRequest {
            rows: 2,
            columns: 1,
            preset: MatrixSetupPreset::MxRgb,
        },
        false,
        &catalogue(),
    )
    .expect("valid matrix setup");

    assert_eq!(prepared.matrix.id, "matrix-created");
    assert_eq!(prepared.matrix.board_id.as_deref(), Some("board-main"));
    assert_eq!((prepared.matrix.rows, prepared.matrix.columns), (2, 1));
    assert_eq!(prepared.matrix.pitch.x, 19.05);
    assert_eq!(prepared.matrix.pitch.y, 19.05);
    assert_eq!(prepared.matrix.origin.x, 0.0);
    assert_eq!(prepared.matrix.origin.y, 0.0);
    assert_eq!(
        prepared.matrix.edge_gap.as_ref().map(|gap| (gap.x, gap.y)),
        Some((1.0, 1.0))
    );
    assert_eq!(prepared.matrix.cells.len(), 2);
    assert_eq!(
        prepared.matrix.cells[0].variant.as_deref(),
        Some("preset/mx-rgb/south")
    );
    assert_eq!(
        prepared.matrix.cells[0].definition_id,
        Some(prepared.matrix.definition_id.clone())
    );
    assert_eq!(prepared.matrix.cells[0].assemblies_local, Some(true));
    assert_eq!(
        prepared.matrix.cells[0]
            .assemblies
            .iter()
            .map(|member| member.id.as_str())
            .collect::<Vec<_>>(),
        ["diode", "led"]
    );
    assert_eq!(prepared.matrix.cells[0].assemblies[0].offset.x, 7.4);
    assert_eq!(prepared.matrix.cells[0].assemblies[0].offset.y, -1.5);
    assert_eq!(prepared.matrix.cells[0].assemblies[0].rotation, Some(90.0));
    assert_eq!(prepared.matrix.cells[1].row, 1);
    assert_eq!(prepared.matrix.cells[1].column, 0);
    assert_eq!(prepared.definitions.len(), 3);
    assert_eq!(prepared.definitions[0].id, prepared.matrix.definition_id);
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["hotswap"],
        false
    );
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["solder"],
        true
    );
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["include_keycap"],
        true
    );
}

#[test]
fn matrix_setup_maps_every_reference_preset_without_losing_construction_flags() {
    let cases = [
        (
            MatrixSetupPreset::MxSolder,
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            false,
            false,
            "preset/mx-solder/reversible/south",
        ),
        (
            MatrixSetupPreset::MxHotswap,
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            true,
            false,
            "preset/mx-hotswap/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocSolder,
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            false,
            false,
            "preset/choc-solder/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocHotswap,
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            true,
            false,
            "preset/choc-hotswap/reversible/south",
        ),
        (
            MatrixSetupPreset::MxRgb,
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            false,
            true,
            "preset/mx-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocRgb,
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            false,
            true,
            "preset/choc-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::MxHotswapRgb,
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            true,
            true,
            "preset/mx-hotswap-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocHotswapRgb,
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            true,
            true,
            "preset/choc-hotswap-rgb/reversible/south",
        ),
    ];
    for (preset, definition_id, generator_source, hotswap, led, expected_variant) in cases {
        let prepared = prepare_matrix(
            format!("matrix-{}", preset.as_str()),
            "board-main".into(),
            MatrixSetupRequest {
                rows: 1,
                columns: 1,
                preset,
            },
            true,
            &catalogue(),
        )
        .expect("preset is supported by the fixture catalogue");
        assert!(prepared.matrix.definition_id.starts_with(&format!(
            "assembly-preset-{}-south-reversible-matrix-{}-0/definition/switch",
            preset.as_str(),
            preset.as_str()
        )));
        assert_eq!(
            prepared.definitions[0].generator.as_ref().unwrap().source,
            generator_source
        );
        assert!(prepared.definitions[0].id.ends_with("/definition/switch"));
        assert_ne!(prepared.definitions[0].id, definition_id);
        assert_eq!(prepared.matrix.definition_id, prepared.definitions[0].id);
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["hotswap"],
            hotswap
        );
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["solder"],
            !hotswap
        );
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["reversible"],
            true
        );
        assert_eq!(
            prepared.matrix.cells[0].assemblies.len(),
            1 + usize::from(led)
        );
        assert_eq!(
            prepared.matrix.cells[0].variant.as_deref(),
            Some(expected_variant)
        );
    }
}

#[test]
fn matrix_setup_rejects_invalid_size_or_missing_real_catalogue_entries() {
    let catalogue = catalogue();
    for (rows, columns) in [(0, 1), (1, 0), (65, 64), (u32::MAX, 2)] {
        assert!(
            prepare_matrix(
                "matrix-invalid".into(),
                "board-main".into(),
                MatrixSetupRequest {
                    rows,
                    columns,
                    preset: MatrixSetupPreset::MxSolder
                },
                false,
                &catalogue,
            )
            .is_err(),
            "invalid size {rows}×{columns} must not create a matrix"
        );
    }
    assert!(
        prepare_matrix(
            "matrix-missing".into(),
            "board-main".into(),
            MatrixSetupRequest {
                rows: 1,
                columns: 1,
                preset: MatrixSetupPreset::MxSolder
            },
            false,
            &catalogue[1..2],
        )
        .is_err(),
        "a missing diode must be reported rather than fabricated"
    );
}

#[test]
fn generated_matrix_identity_skips_existing_matrix_definition_and_member_ids() {
    let preset = MatrixSetupPreset::MxSolder;
    let occupied_definition = definition(
        "assembly-preset-mx-solder-south-matrix-1-0/definition/switch",
        "ceoloide/switch_mx",
        PartKind::Switch,
    );
    assert_eq!(
        next_matrix_id(&[], &[occupied_definition], &[], preset.as_str(), false),
        "matrix-2"
    );

    let occupied_part = Part {
        keycap: None,
        outline: None,
        id: "matrix/matrix-1/r0c0".into(),
        definition_id: "existing-definition".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    assert_eq!(
        next_matrix_id(&[], &[], &[occupied_part], preset.as_str(), false),
        "matrix-2"
    );
}

#[test]
fn matrix_setup_uses_saved_session_history_selection_and_reopen() {
    let mut session = Session::new();
    let mut engine = CoreEngine::new();
    open_session_saved(&mut session, &mut engine, session_document(), 1);

    let prepared = prepare_matrix(
        "matrix-new".into(),
        "board-main".into(),
        MatrixSetupRequest {
            rows: 2,
            columns: 3,
            preset: MatrixSetupPreset::MxSolder,
        },
        false,
        &catalogue(),
    )
    .expect("the selected MX solder recipe exists");
    let matrix_id = prepared.matrix.id.clone();
    let edit_effects = session.submit(Event::Edit {
        operation_id: OperationId(2),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "matrix-setup-1".into(),
            phase: EditPhase::Commit,
            target_ids: vec![matrix_id.clone()],
            operation: EditOperation::SetMatrix {
                matrix: prepared.matrix,
                definitions: Some(prepared.definitions),
            },
        },
    });
    let save_effects = complete_session_core(&mut session, &mut engine, edit_effects);
    let completion_effects = persist_session_committed(&mut session, save_effects);
    assert!(completion_effects.iter().any(|effect| matches!(
        effect,
        Effect::Settled {
            operation_id: OperationId(2),
            outcome: TerminalOutcome::Completed,

        ..}
    )));

    let accepted = session
        .read_model()
        .accepted
        .as_ref()
        .expect("saved matrix");
    let matrix = accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .expect("Core accepted the requested matrix");
    let matrix_part_ids = matrix.part_ids.clone();
    assert_eq!((matrix.rows, matrix.columns), (2, 3));
    assert_eq!(matrix.board_id.as_deref(), Some("board-main"));
    assert_eq!(
        matrix_part_ids.len(),
        12,
        "six switches plus six diodes are live"
    );
    assert!(matrix_part_ids.iter().all(|id| {
        accepted.document.parts.iter().any(|part| part.id == *id)
            && accepted
                .document
                .boards
                .iter()
                .find(|board| board.id == "board-main")
                .is_some_and(|board| board.part_ids.contains(id))
    }));

    let selection = session.submit(Event::SelectParts {
        operation_id: OperationId(3),
        part_ids: matrix_part_ids.clone(),
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    assert!(
        selection.iter().any(|effect| matches!(
            effect,
            Effect::Settled {
                operation_id: OperationId(3),
                outcome: TerminalOutcome::Completed,

            ..}
        )),
        "selection settles synchronously in Session"
    );
    assert_eq!(session.read_model().selected_part_ids, matrix_part_ids);

    session_history_action(
        &mut session,
        &mut engine,
        Event::Undo {
            operation_id: OperationId(4),
        },
    );
    let undone = session.read_model().accepted.as_ref().expect("saved undo");
    assert!(
        !undone
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert!(
        undone
            .document
            .parts
            .iter()
            .all(|part| !part.id.starts_with("matrix/matrix-new/"))
    );
    assert!(matches!(
        session.read_model().durability,
        Durability::Saved { revision: 2 }
    ));

    session_history_action(
        &mut session,
        &mut engine,
        Event::Redo {
            operation_id: OperationId(5),
        },
    );
    let redone = session.read_model().accepted.as_ref().expect("saved redo");
    assert!(
        redone
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert_eq!(redone.document.revision, 3);

    let durable_document = (*redone.document).clone();
    let mut reopened_session = Session::new();
    let mut reopened_engine = CoreEngine::new();
    open_session_saved(
        &mut reopened_session,
        &mut reopened_engine,
        durable_document,
        6,
    );
    let reopened = reopened_session
        .read_model()
        .accepted
        .as_ref()
        .expect("reopened saved document");
    assert!(
        reopened
            .document
            .matrices
            .iter()
            .any(|matrix| matrix.id == matrix_id)
    );
    assert_eq!(reopened.document.revision, 3);
}
