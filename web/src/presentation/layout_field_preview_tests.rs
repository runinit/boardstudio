//! Ticket 16: hovering or focusing an Inspector number field previews, in
//! amber, the keys a change to that field would move, and the canvas hint
//! reads "Moves N keys (n per half)". The preview clears on leave, blur or
//! selection change and never writes the document.
use super::*;
use crate::presentation::test_mount_helpers::{
    self, EditorProbe, MountedRoot, matrix_part_definition, mount_editor,
};
use boardstudio_core::model::ProjectDoc;
use boardstudio_web_ui_shared::test_wait::wait_until;
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const ROWS: u32 = 2;
const COLUMNS: u32 = 4;

/// A mirrored pair opened directly, following the ticket-14 split-document
/// pattern: the pair link (not an edit) is what the preview reads for the
/// mirrored half. The right origin is the reflection of the left (-20)
/// about the pair axis (10).
fn mirrored_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("field-preview", "Field preview");
    let courtyard = [[-7.0, -7.0], [7.0, 7.0], [7.0, -7.0], [-7.0, 7.0]];
    document.definitions.push(
        serde_json::from_value(matrix_part_definition("switch", "switch", courtyard)).unwrap(),
    );
    // Materialized key parts with canonical member ids, following the
    // ticket-14 split-document pattern: scenes resolve members against
    // these, so the preview names both halves.
    let mut board_parts = Vec::new();
    let mut matrices = Vec::new();
    for (id, origin_x, prefix) in [("left-matrix", -20.0, "L")] {
        let mut part_ids = Vec::new();
        let mut cells = Vec::new();
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let part_id = format!("matrix/{id}/r{row}c{column}");
                part_ids.push(part_id.clone());
                board_parts.push(part_id.clone());
                document.parts.push(
                    serde_json::from_value(json!({
                        "id": part_id, "definitionId": "switch",
                        "reference": format!("{prefix}{row}{column}"),
                        "pose": {"at": {"x": origin_x + column as f64 * 19.0,
                                                "y": row as f64 * 19.0},
                                         "rotation": 0},
                        "side": "front"
                    }))
                    .unwrap(),
                );
                cells.push(json!({"row": row, "column": column, "enabled": true}));
            }
        }
        matrices.push(json!({
            "id": id, "rows": ROWS, "columns": COLUMNS,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": origin_x, "y": 0.0 },
            "definitionId": "switch", "boardId": "board", "partIds": part_ids,
            "cells": cells
        }));
    }
    for matrix in matrices {
        document
            .matrices
            .push(serde_json::from_value(matrix).unwrap());
    }
    document.boards.push(
        serde_json::from_value(json!({
            "id": "board", "name": "Board", "outlineIds": [],
            "partIds": board_parts, "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    // Layout records go through JSON so additive optional link fields
    // (half-frame contract work in flight) cannot break this fixture.
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "source-layout", "name": "Left half", "boardId": "board",
            "matrixIds": ["left-matrix"], "partIds": []
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id": "paired-layout", "name": "Right half", "boardId": "board",
            "matrixIds": [], "partIds": []
        }))
        .unwrap(),
    );
    document.mirrored_pairs = serde_json::from_value(json!([{
        "id": "pair",
        "source": {"kind": "sameBoardLayout", "id": "source-layout"},
        "destination": {"kind": "sameBoardLayout", "id": "paired-layout"},
        "mapping": {"at": {"x": 20.0, "y": 0.0}, "rotation": 0.0},
        "overrides": []
    }]))
    .unwrap();
    document
}

fn query(root_id: &str, selector: &str) -> Option<web_sys::Element> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} {selector}"))
        .unwrap()
}

fn count(root_id: &str, selector: &str) -> u32 {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all(&format!("#{root_id} {selector}"))
        .unwrap()
        .length()
}

fn dispatch(element: &web_sys::Element, kind: &str) {
    let event = web_sys::PointerEvent::new(kind).unwrap();
    element
        .closest("[data-transform-number]")
        .unwrap()
        .unwrap()
        .dispatch_event(event.as_ref())
        .unwrap();
}

/// Open the mirrored pair through the Session and Core and mount the real
/// Layout editor, without waiting: the tests below wait for their own state.
/// Column 2 of the left half is selected: Spread carries columns 2-4 on both
/// halves, 12 keys, "Moves 12 keys (6 per half)".
async fn mount_bare(root_id: &'static str) -> (EditorProbe, MountedRoot) {
    let runtime = test_mount_helpers::open_document(mirrored_document()).await;
    mount_editor(root_id, runtime)
}

async fn mount(root_id: &'static str) -> (EditorProbe, MountedRoot) {
    let (probe, root) = mount_bare(root_id).await;
    wait_until("the Layout canvas draws the mirrored keys", || {
        count(root_id, "[data-matrix-id][data-row][data-column]") == ROWS * COLUMNS * 2
    })
    .await;
    probe.select(objects::TreeContext::Column {
        matrix_id: "left-matrix".into(),
        column: 1,
    });
    wait_until("the Inspector shows the column Spread field", || {
        query(root_id, "input[aria-label='Spread']").is_some()
    })
    .await;
    (probe, root)
}

fn spread_field(root_id: &str) -> web_sys::HtmlElement {
    query(root_id, "input[aria-label='Spread']")
        .expect("column Spread field")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
}

/// The `(row, column)` of every amber field-preview canvas key.
fn previewed_keys(root_id: &str) -> Vec<(String, String, String)> {
    let list = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all(&format!(
            "#{root_id} [data-matrix-id][data-field-preview='true']"
        ))
        .unwrap();
    (0..list.length())
        .map(|index| {
            let key = list
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::Element>()
                .unwrap();
            (
                key.get_attribute("data-matrix-id").unwrap_or_default(),
                key.get_attribute("data-row").unwrap_or_default(),
                key.get_attribute("data-column").unwrap_or_default(),
            )
        })
        .collect()
}

fn hint_text(root_id: &str) -> Option<String> {
    query(root_id, "[data-field-preview-hint]")
        .map(|element| element.text_content().unwrap_or_default())
}

#[wasm_bindgen_test]
async fn hovering_a_column_field_previews_its_keys_on_both_halves() {
    let root_id = "field-preview-hover";
    let (probe, _root) = mount(root_id).await;
    let before = probe.revision();
    let field = spread_field(root_id);
    dispatch(&field, "pointerenter");
    wait_until(
        "hovering Spread previews columns 2-4 on both halves",
        || previewed_keys(root_id).len() == (COLUMNS - 1) as usize * ROWS as usize * 2,
    )
    .await;
    let previewed = previewed_keys(root_id);
    assert_eq!(
        previewed.len(),
        12,
        "Spread carries 6 own + 6 mirrored keys"
    );
    assert!(
        previewed
            .iter()
            .all(|(_, _, column)| ["1", "2", "3"].contains(&column.as_str())),
        "only columns 2-4 move: {previewed:?}"
    );
    assert!(
        previewed
            .iter()
            .filter(|(matrix, _, _)| matrix == "left-matrix")
            .count()
            == 6
            && previewed
                .iter()
                .filter(|(matrix, _, _)| matrix == "mirror_70616972_left-matrix")
                .count()
                == 6,
        "both halves move: {previewed:?}"
    );
    assert!(
        previewed.iter().all(
            |(matrix, _, _)| matrix == "left-matrix" || matrix == "mirror_70616972_left-matrix"
        ),
        "no other matrix moves: {previewed:?}"
    );
    assert_eq!(
        count(
            root_id,
            "[data-matrix-id][data-field-preview='true'][data-relation-highlight='true']"
        ),
        0,
        "the amber preview never uses the violet reference treatment"
    );
    assert_eq!(
        count(root_id, "[data-matrix-id][data-relation-highlight='true']"),
        0,
        "no transient relation highlight remains"
    );
    assert_eq!(
        count(root_id, "[data-matrix-id][data-anchor-target='true']"),
        2,
        "only the persistent anchor target stays violet"
    );
    assert_eq!(
        count(
            root_id,
            "[data-matrix-id='left-matrix'][data-anchor-target='true'][data-column='0']"
        ),
        2,
        "the previous column is the anchor target"
    );
    assert_eq!(
        hint_text(root_id).as_deref(),
        Some("Moves 12 keys (6 per half)"),
        "the canvas hint names the move"
    );
    assert_eq!(probe.revision(), before, "previewing writes no revision");
    dispatch(&field, "pointerleave");
    wait_until("leaving the field clears its preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
    assert_eq!(hint_text(root_id), None);
    assert_eq!(probe.revision(), before, "clearing writes no revision");
}

#[wasm_bindgen_test]
async fn keyboard_focus_previews_and_blur_clears() {
    let root_id = "field-preview-focus";
    let (_probe, _root) = mount(root_id).await;
    let field = spread_field(root_id);
    field.focus().unwrap();
    wait_until("keyboard focus previews the same keys as hover", || {
        previewed_keys(root_id).len() == 12
    })
    .await;
    assert_eq!(
        hint_text(root_id).as_deref(),
        Some("Moves 12 keys (6 per half)"),
        "focus names the move like hover does"
    );
    field.blur().unwrap();
    wait_until("blurring the field clears its preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
}

#[wasm_bindgen_test]
async fn changing_the_selection_clears_the_field_preview() {
    let root_id = "field-preview-selection";
    let (probe, _root) = mount(root_id).await;
    dispatch(&spread_field(root_id), "pointerenter");
    wait_until("hovering the field previews every carried key", || {
        previewed_keys(root_id).len() == 12
    })
    .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: "left-matrix".into(),
        row: 0,
        column: 0,
    });
    wait_until("a selection change clears the field preview", || {
        previewed_keys(root_id).is_empty() && hint_text(root_id).is_none()
    })
    .await;
    assert!(previewed_keys(root_id).is_empty());
}
