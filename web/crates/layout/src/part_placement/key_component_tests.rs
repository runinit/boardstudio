//! Native tests for applying a component to a selected matrix key and for the
//! captured placement owner.

use super::test_support::*;
use super::*;
use boardstudio_application::{Scope, SessionEpoch};
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase, MatrixCell, PartKind, Vec2};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};

#[test]
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

#[test]
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
            source_workspace: Workspace::Parts,
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
