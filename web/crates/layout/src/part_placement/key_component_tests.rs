//! Browser tests for applying a component to a selected matrix key and for the
//! captured placement owner (the production `selected_key_component_operation` is
//! browser-gated through its matrix-input helper).
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use super::test_support::*;
use super::*;
use boardstudio_application::{Scope, SessionEpoch};
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase, MatrixCell, PartKind, Vec2};
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};

#[wasm_bindgen_test::wasm_bindgen_test]
fn selected_key_application_uses_set_matrix_and_preserves_other_cell_fields() {
    let mut accepted = fixture();
    accepted.matrices.push(boardstudio_model::model::Matrix {
        id: "matrix-main".into(),
        name: None,
        rows: 2,
        columns: 3,
        pitch: Vec2 { x: 19.05, y: 19.05 },
        origin: Vec2::default(),
        definition_id: "switch:base".into(),
        part_ids: vec![],
        board_id: Some("board-main".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 1,
            column: 2,
            enabled: false,
            definition_id: None,
            variant: Some("existing-variant".into()),
            offset: Some(Vec2 { x: 0.5, y: -0.5 }),
            rotation: Some(12.0),
            assemblies: vec![],
            assemblies_local: Some(true),
        }],
    });
    let scope = Scope {
        session_epoch: SessionEpoch(7),
        document_id: "project".into(),
        board_id: "board-main".into(),
        instance_id: None,
    };
    let selection = ScopedTreeContext {
        scope: scope.clone(),
        context: TreeContext::Key {
            matrix_id: "matrix-main".into(),
            row: 1,
            column: 2,
        },
    };
    let original = accepted.clone();
    let operation = selected_key_component_operation(
        &accepted,
        &scope,
        &selection,
        &passive_definition("imported:reset-switch"),
    )
    .unwrap();
    let EditOperation::SetMatrix {
        matrix,
        definitions,
    } = operation
    else {
        panic!("applying to a selected key uses SetMatrix");
    };
    assert_eq!(definitions.unwrap()[0].id, "imported:reset-switch");
    assert_eq!(matrix.cells.len(), 1);
    let cell = &matrix.cells[0];
    assert!(cell.enabled);
    assert_eq!(cell.variant.as_deref(), Some("existing-variant"));
    assert_eq!(cell.offset, Some(Vec2 { x: 0.5, y: -0.5 }));
    assert_eq!(cell.rotation, Some(12.0));
    assert_eq!(cell.assemblies_local, Some(true));
    assert_eq!(cell.assemblies[0].id, "library-imported-reset-switch-1");
    assert_eq!(accepted, original);

    let operation = selected_key_component_operation(
        &accepted,
        &scope,
        &selection,
        &switch_definition("switch:mx"),
    )
    .unwrap();
    let EditOperation::SetMatrix {
        matrix,
        definitions,
    } = operation
    else {
        panic!("switch applies through SetMatrix");
    };
    assert_eq!(matrix.cells[0].definition_id.as_deref(), Some("switch:mx"));
    assert!(matrix.cells[0].assemblies.is_empty());
    assert_eq!(definitions.unwrap()[0].id, "switch:mx");
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn linked_target_key_replacement_stays_local_through_core_sync_and_history() {
    let mut document = fixture();
    document.definitions.push(switch_definition("switch:base"));
    let mut source = matrix_fixture();
    source.id = "matrix-source".into();
    source.cells.push(MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 0,
        enabled: true,
        definition_id: Some("switch:base".into()),
        variant: None,
        offset: None,
        rotation: None,
        assemblies: Vec::new(),
        assemblies_local: None,
    });
    let mut target = matrix_fixture();
    target.id = "matrix-target".into();
    document.matrices = vec![source, target];
    document.layouts = vec![
        boardstudio_model::model::Layout {
            id: "layout-source".into(),
            name: "Left".into(),
            board_id: "board-main".into(),
            matrix_ids: vec!["matrix-source".into()],
            part_ids: Vec::new(),
            mirror_link: None,
        },
        boardstudio_model::model::Layout {
            id: "layout-target".into(),
            name: "Right".into(),
            board_id: "board-main".into(),
            matrix_ids: vec!["matrix-target".into()],
            part_ids: Vec::new(),
            mirror_link: Some(boardstudio_model::model::LayoutMirrorLink {
                correspondences: vec![boardstudio_model::model::MirroredCorrespondence {
                    companion_mode: None,
                    companion_tweak: None,
                    kind: boardstudio_model::model::MirroredItemKind::Matrix,
                    source_id: "matrix-source".into(),
                    mirror_id: "matrix-target".into(),
                }],
                exceptions: vec![],
                source_id: "layout-source".into(),
                axis_x: 0.0,
                split_gap: None,
                spacing: None,
            }),
        },
    ];
    let mut engine = boardstudio_core::CoreEngine::new();
    let opened = engine.handle(boardstudio_model::model::CoreRequest::Open {
        id: "open-linked".into(),
        document,
    });
    let boardstudio_model::model::CoreReply::Scene {
        document: opened_document,
        ..
    } = opened
    else {
        panic!("the linked source fixture opens in Core: {opened:?}");
    };
    let accepted = *opened_document;
    let target_before = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix-target")
        .unwrap();
    assert_eq!(
        target_before.cells[0].definition_id.as_deref(),
        Some("switch:base")
    );
    assert_eq!(target_before.cells[0].assemblies_local, None);

    let scope = Scope {
        session_epoch: SessionEpoch(7),
        document_id: accepted.id.clone(),
        board_id: "board-main".into(),
        instance_id: None,
    };
    let selection = ScopedTreeContext {
        scope,
        context: TreeContext::Key {
            matrix_id: "matrix-target".into(),
            row: 0,
            column: 0,
        },
    };
    let EditOperation::SetMatrix {
        matrix,
        definitions,
    } = selected_key_component_operation(
        &accepted,
        &selection.scope,
        &selection,
        &switch_definition("switch:replacement"),
    )
    .unwrap()
    else {
        panic!("a switch replacement uses SetMatrix");
    };
    assert_eq!(
        matrix.cells[0].definition_id.as_deref(),
        Some("switch:replacement")
    );
    let replacement = engine.handle(boardstudio_model::model::CoreRequest::Edit {
        id: "replace-linked-key".into(),
        command: EditCommand {
            base_revision: accepted.revision,
            transaction_id: "replace-linked-key".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["matrix-target".into(), "switch:replacement".into()],
            operation: EditOperation::SetMatrix {
                matrix,
                definitions,
            },
        },
    });
    let boardstudio_model::model::CoreReply::Scene {
        document: replaced_document,
        ..
    } = replacement
    else {
        panic!("Core accepts and synchronizes the linked-key replacement");
    };
    let replaced = *replaced_document;
    let target_after = replaced
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix-target")
        .unwrap();
    assert_eq!(
        target_after.cells[0].definition_id.as_deref(),
        Some("switch:replacement")
    );
    assert_eq!(target_after.cells[0].assemblies_local, Some(true));

    let undone = engine.handle(boardstudio_model::model::CoreRequest::Undo {
        id: "undo-linked-key".into(),
    });
    let boardstudio_model::model::CoreReply::Scene {
        document: undone_document,
        ..
    } = undone
    else {
        panic!("the linked-key replacement is undoable");
    };
    let undone_target = undone_document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix-target")
        .unwrap();
    assert_eq!(
        undone_target.cells[0].definition_id.as_deref(),
        Some("switch:base")
    );
    assert_eq!(undone_target.cells[0].assemblies_local, None);

    let redone = engine.handle(boardstudio_model::model::CoreRequest::Redo {
        id: "redo-linked-key".into(),
    });
    let boardstudio_model::model::CoreReply::Scene {
        document: redone_document,
        ..
    } = redone
    else {
        panic!("the linked-key replacement is redoable");
    };
    let redone_target = redone_document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "matrix-target")
        .unwrap();
    assert_eq!(
        redone_target.cells[0].definition_id.as_deref(),
        Some("switch:replacement")
    );
    assert_eq!(redone_target.cells[0].assemblies_local, Some(true));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn captured_owner_rejects_changed_snapshot_scope_generation_or_guide_stage() {
    let initial = accepted(fixture(), 11);
    let scope = Scope {
        session_epoch: SessionEpoch(7),
        document_id: "project".into(),
        board_id: "board-main".into(),
        instance_id: None,
    };
    let owner = PlacementOwner::capture(
        &initial,
        PlacementCapture {
            scope: scope.clone(),
            generation: 4,
            part_id: "part-controller".into(),
            definition_id: "catalog:controller".into(),
            kind: PartKind::Controller,
            workflow: PlacementWorkflow::WiringController,
            intent: PlacementIntent::Existing,
            source_workspace: "Parts",
            layout_id: None,
            at: Vec2 { x: 3.0, y: 9.0 },
        },
    )
    .unwrap();

    assert!(owner.is_current(&initial, Some(&scope), 4, true, Some("project"), true));
    assert!(!owner.is_current(
        &accepted(fixture(), 12),
        Some(&scope),
        4,
        true,
        Some("project"),
        true
    ));
    assert!(!owner.is_current(&initial, Some(&scope), 5, true, Some("project"), true));
    assert!(!owner.is_current(&initial, Some(&scope), 4, false, Some("project"), true));
    assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("other"), true));
    assert!(!owner.is_current(&initial, Some(&scope), 4, true, Some("project"), false));
}
