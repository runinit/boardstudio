//! Native unit tests for the pure placement planning functions in `super::plan`.
use super::plan::*;
use super::test_support::*;
use boardstudio_model::model::{EditOperation, OutlineFeature, PartKind, Vec2};
use boardstudio_web_ui_model::state::LayoutSnapSettings;

fn test_layout(id: &str, matrix_id: &str) -> boardstudio_model::model::Layout {
    boardstudio_model::model::Layout {
        id: id.into(),
        name: id.into(),
        board_id: "board-main".into(),
        matrix_ids: vec![matrix_id.into()],
        part_ids: vec![],
        mirror_link: None,
    }
}

fn snap_options(
    fraction: f64,
    geometry: bool,
    free: bool,
    pitch: Option<Vec2>,
) -> PlacementSnapOptions {
    PlacementSnapOptions {
        snap_fraction: fraction,
        geometry_snap: geometry,
        gap: None,
        free,
        pitch,
    }
}

#[test]
fn standalone_placement_accepts_a_non_controller_and_updates_memberships_atomically() {
    let mut accepted = fixture();
    accepted.layouts = vec![
        test_layout("layout-main", "matrix-main"),
        test_layout("layout-other", "matrix-other"),
    ];
    let original = accepted.clone();
    let definition = passive_definition("imported:reset-switch");
    let placed = part(&definition.id, "part-reset-switch", "J1");

    let proposed = replacement(
        placement_operation(
            &accepted,
            "board-main",
            &definition,
            &placed,
            Some("layout-main"),
        )
        .expect("the ordinary component action accepts non-controller definitions"),
    );

    assert_eq!(proposed.definitions, vec![definition]);
    assert_eq!(proposed.parts, vec![placed]);
    assert_eq!(proposed.boards[0].part_ids, vec!["part-reset-switch"]);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &proposed.outline[0] else {
        panic!("fixture outline is an envelope");
    };
    assert_eq!(part_ids, &["part-reset-switch"]);
    assert_eq!(proposed.layouts[0].part_ids, vec!["part-reset-switch"]);
    assert!(proposed.layouts[1].part_ids.is_empty());
    assert_eq!(
        accepted, original,
        "proposal construction does not mutate accepted state"
    );
}

#[test]
fn catalogue_placement_emits_typed_linked_and_mirror_only_operations_only_for_pair_intents() {
    let mut accepted = mirrored_fixture();
    accepted.layouts = vec![
        test_layout("layout-main", "matrix-main"),
        // The mirror layout belongs to the mirror board, not the main board.
        boardstudio_model::model::Layout {
            board_id: "board-mirror".into(),
            ..test_layout("layout-mirror", "matrix-mirror")
        },
    ];
    let definition = passive_definition("catalog:linked-part");
    let placed = part(&definition.id, "part-new", "J1");

    for (board_id, layout_id) in [
        ("board-main", "layout-main"),
        ("board-mirror", "layout-mirror"),
    ] {
        let operation = placement_operation_for_intent(
            &accepted,
            board_id,
            &definition,
            &placed,
            Some(layout_id),
            PlacementIntent::LinkedCatalogueDefault,
        )
        .unwrap();
        assert_eq!(
            operation,
            EditOperation::AddPart {
                part: placed.clone(),
                board_id: Some(board_id.into()),
                definition: Some(definition.clone()),
                layout_id: Some(layout_id.into()),
            }
        );
    }

    assert_eq!(
        placement_operation_for_intent(
            &accepted,
            "board-mirror",
            &definition,
            &placed,
            Some("layout-mirror"),
            PlacementIntent::MirrorOnly,
        )
        .unwrap(),
        EditOperation::AddMirrorOnlyPart {
            part: placed.clone(),
            board_id: "board-mirror".into(),
            definition: Some(definition.clone()),
            layout_id: Some("layout-mirror".into()),
        }
    );

    assert!(matches!(
        placement_operation_for_intent(
            &fixture(),
            "board-main",
            &definition,
            &placed,
            None,
            PlacementIntent::LinkedCatalogueDefault,
        ),
        Ok(EditOperation::ReplaceDocument { .. })
    ));
    assert!(
        placement_operation_for_intent(
            &accepted,
            "board-main",
            &definition,
            &placed,
            None,
            PlacementIntent::MirrorOnly,
        )
        .is_err()
    );
    let mut ambiguous = accepted.clone();
    let duplicate_pair = ambiguous.hardware.as_ref().unwrap().pairs[0].clone();
    ambiguous
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .push(duplicate_pair);
    assert!(
        placement_operation_for_intent(
            &ambiguous,
            "board-mirror",
            &definition,
            &placed,
            None,
            PlacementIntent::LinkedCatalogueDefault,
        )
        .is_err()
    );
    assert!(matches!(
        placement_operation_for_intent(
            &accepted,
            "board-main",
            &definition,
            &placed,
            None,
            PlacementIntent::Existing,
        ),
        Ok(EditOperation::ReplaceDocument { .. })
    ));
}

#[test]
fn conflicting_definitions_and_missing_layout_targets_reject_without_mutating_source() {
    let mut accepted = fixture();
    let original = accepted.clone();
    let definition = controller_definition("catalog:controller");
    let placed = part(&definition.id, "part-controller", "U1");
    accepted.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "catalog:controller",
            "name": "Conflicting controller",
            "kind": "controller",
            "courtyard": [],
            "pads": []
        }))
        .unwrap(),
    );
    let conflict = placement_operation(&accepted, "board-main", &definition, &placed, None);
    assert!(conflict.is_err());
    assert_eq!(accepted.revision, original.revision);
    assert!(accepted.parts.is_empty());

    let missing_layout = placement_operation(
        &original,
        "board-main",
        &definition,
        &placed,
        Some("missing-layout"),
    );
    assert!(missing_layout.is_err());
    assert!(original.parts.is_empty());
}

#[test]
fn component_references_use_react_kind_prefixes_and_lowest_free_number() {
    let mut document = fixture();
    document.parts = ["S1", "S3", "U1", "E2", "J1"]
        .iter()
        .enumerate()
        .map(|(index, reference)| part("existing", &format!("part-{index}"), reference))
        .collect();
    for (kind, expected) in [
        (PartKind::Switch, "S2"),
        (PartKind::Controller, "U2"),
        (PartKind::Encoder, "E1"),
        (PartKind::Passive, "J2"),
    ] {
        assert_eq!(next_component_reference(&document, &kind), expected);
    }
}

#[test]
fn cursor_placement_uses_project_grid_and_alt_free_position_policy() {
    let pending = controller_part(
        controller_definition("catalog:controller"),
        "part-controller".into(),
        "U1".into(),
        Vec2::default(),
    )
    .unwrap();
    let point = Vec2 { x: 4.6, y: -4.6 };
    // (snap fraction, expected grid point): quarter pitch, absolute 0.5 mm, disabled.
    for (fraction, expected) in [
        (
            0.25,
            Vec2 {
                x: 4.7625,
                y: -4.7625,
            },
        ),
        (-0.5, Vec2 { x: 4.5, y: -4.5 }),
        (0.0, point),
    ] {
        assert_eq!(grid_snap_point(point, fraction, None), expected);
        assert_eq!(
            snap_placement_at(
                &fixture(),
                "board-main",
                &pending,
                point,
                snap_options(fraction, false, false, None)
            ),
            expected
        );
    }
    // Alt-free placement bypasses even geometry snap.
    assert_eq!(
        snap_placement_at(
            &fixture(),
            "board-main",
            &pending,
            point,
            snap_options(0.25, true, true, None)
        ),
        point
    );
}

#[test]
fn initial_placement_uses_world_view_center_after_pan() {
    assert_eq!(
        canvas_world_center(-40.0, 60.0, -20.0, 80.0, Vec2 { x: 13.0, y: -7.0 }),
        Vec2 { x: 23.0, y: 23.0 }
    );
}

#[test]
fn placement_commits_only_on_primary_pointer_release() {
    for (button, expected) in [(0, true), (1, false), (2, false)] {
        assert_eq!(pointer_release_commits(button), expected);
    }
}

#[test]
fn placement_geometry_snap_uses_pending_definition_and_current_gap_preference() {
    let mut document = fixture();
    let obstacle_definition = controller_definition("existing:controller");
    document.definitions.push(obstacle_definition.clone());
    let mut obstacle = part(&obstacle_definition.id, "existing-controller", "U2");
    obstacle.pose.at = Vec2 { x: 11.0, y: 0.0 };
    document.parts.push(obstacle);
    document.boards[0]
        .part_ids
        .push("existing-controller".into());

    let definition = controller_definition("catalog:controller");
    let pending = controller_part(
        definition.clone(),
        "part-controller".into(),
        "U1".into(),
        Vec2::default(),
    )
    .unwrap();
    let snap_document = document_with_pending_definition(&document, &definition);
    let at = snap_placement_at(
        &snap_document,
        "board-main",
        &pending,
        Vec2 { x: 10.2, y: 0.0 },
        snap_options(0.25, true, false, None),
    );
    assert_eq!(at.x, 11.0);

    let mut settings = LayoutSnapSettings::default();
    assert_eq!(placement_gap(&settings), Some(1.0));
    settings.gap_override = "0.7".into();
    assert_eq!(placement_gap(&settings), Some(0.7));
    settings.gap_snap = false;
    assert_eq!(placement_gap(&settings), None);
}

#[test]
fn component_placement_snap_uses_the_selected_matrix_pitch() {
    // Ticket 22: the selected matrix uses a 20 mm pitch, so a quarter snap
    // steps 5 mm (ticket 08 decision 1: pitch x snap fraction, 19.05 mm only
    // when none is selected).
    let mut document = fixture();
    document.matrices.push(
        serde_json::from_value(serde_json::json!({
            "id": "matrix",
            "rows": 1,
            "columns": 2,
            "pitch": {"x": 20.0, "y": 20.0},
            "origin": {"x": 0.0, "y": 0.0},
            "definitionId": "catalog:controller",
            "partIds": [],
            "boardId": "board-main",
            "cells": []
        }))
        .unwrap(),
    );
    let definition = controller_definition("catalog:controller");
    let pending = controller_part(
        definition,
        "part-controller".into(),
        "U1".into(),
        Vec2::default(),
    )
    .expect("controller placement");
    assert_eq!(
        snap_placement_at(
            &document,
            "board-main",
            &pending,
            Vec2 { x: 4.6, y: -4.6 },
            snap_options(0.25, false, false, Some(Vec2 { x: 20.0, y: 20.0 })),
        ),
        Vec2 { x: 5.0, y: -5.0 }
    );
}
