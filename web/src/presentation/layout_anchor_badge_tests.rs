//! Ticket 17: the Layout canvas draws an anchor badge for the selection: the
//! authored icon at what the selection is placed from, a violet dashed target
//! on that reference, and a dashed tie to the selection. The badge clears when
//! the selection clears.
use super::*;
use crate::presentation::test_mount_helpers::{
    self, EditorProbe, MountedRoot, matrix_part_definition, mount_editor,
};
use boardstudio_core::model::{EditCommand, EditPhase, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_wait::wait_until;
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

const MATRIX: &str = "main";
const ROWS: u32 = 2;
const COLUMNS: u32 = 5;

/// Ticket 02 renamed the canvas label class: the badge text now reads
/// from the "Placed from" readout, stripping its prefix.
fn badge_label(root_id: &str) -> String {
    element(root_id, "[data-canvas-anchor-readout]")
        .text_content()
        .unwrap_or_default()
        .strip_prefix("Placed from ")
        .unwrap_or_default()
        .to_owned()
}

fn set_matrix_command(base_revision: u64, matrix: serde_json::Value) -> EditCommand {
    EditCommand {
        base_revision,
        transaction_id: "set-matrix".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation: serde_json::from_value(json!({ "kind": "set-matrix", "matrix": matrix }))
            .expect("the operation parses"),
    }
}

fn replace_command(base_revision: u64, document: ProjectDoc) -> EditCommand {
    EditCommand {
        base_revision,
        transaction_id: "replace".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation: serde_json::from_value(json!({
            "kind": "replace-document",
            "document": serde_json::to_value(document).unwrap(),
        }))
        .expect("the operation parses"),
    }
}

fn anchor_command(base_revision: u64, id: &str, anchor: serde_json::Value) -> EditCommand {
    EditCommand {
        base_revision,
        transaction_id: "anchor".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation: serde_json::from_value(json!({
            "kind": "add-anchored-matrix", "id": id, "anchor": anchor,
        }))
        .expect("the operation parses"),
    }
}

fn base() -> ProjectDoc {
    let mut document =
        serde_json::to_value(ProjectDoc::empty("anchor-badge", "Anchor badge")).unwrap();
    let courtyard = [[-7.0, -7.0], [7.0, -7.0], [7.0, 7.0], [-7.0, 7.0]];
    document["definitions"] = json!([
        matrix_part_definition("switch", "switch", courtyard),
        matrix_part_definition("diode", "passive", courtyard)
    ]);
    document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
        "partIds": [], "netIds": [], "thickness": 1.6 }]);
    serde_json::from_value(document).unwrap()
}

/// A ROWS x COLUMNS matrix. `extra` replaces top-level matrix fields.
fn matrix_json(extra: serde_json::Value) -> serde_json::Value {
    let mut matrix = json!({
        "id": MATRIX, "rows": ROWS, "columns": COLUMNS,
        "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
        "definitionId": "switch", "boardId": "board", "partIds": []
    });
    for (key, value) in extra.as_object().into_iter().flatten() {
        matrix[key] = value.clone();
    }
    matrix
}

fn dom() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn element(root_id: &str, selector: &str) -> web_sys::Element {
    query(root_id, selector).unwrap_or_else(|| panic!("no element for {selector}"))
}

fn query(root_id: &str, selector: &str) -> Option<web_sys::Element> {
    dom()
        .query_selector(&format!("#{root_id} {selector}"))
        .unwrap()
}

fn query_all(root_id: &str, selector: &str) -> Vec<web_sys::Element> {
    let list = dom()
        .query_selector_all(&format!("#{root_id} {selector}"))
        .unwrap();
    (0..list.length())
        .map(|index| {
            list.item(index)
                .unwrap()
                .dyn_into::<web_sys::Element>()
                .unwrap()
        })
        .collect()
}

fn count(root_id: &str, selector: &str) -> u32 {
    query_all(root_id, selector).len() as u32
}

/// The `data-column` of every badge-target canvas key.
fn highlighted_columns(root_id: &str) -> Vec<String> {
    query_all(root_id, "[data-matrix-id][data-anchor-target='true']")
        .iter()
        .map(|key| key.get_attribute("data-column").unwrap_or_default())
        .collect()
}

/// The `(row, column)` of every badge-target canvas key.
fn highlighted_keys(root_id: &str) -> Vec<(String, String)> {
    query_all(root_id, "[data-matrix-id][data-anchor-target='true']")
        .iter()
        .map(|key| {
            (
                key.get_attribute("data-row").unwrap_or_default(),
                key.get_attribute("data-column").unwrap_or_default(),
            )
        })
        .collect()
}

/// Create the matrix through the Session and Core, then mount the real Layout editor.
async fn mount(root_id: &'static str) -> (EditorProbe, MountedRoot) {
    let runtime = test_mount_helpers::open_document(base()).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(
        &runtime,
        set_matrix_command(revision, matrix_json(json!({}))),
    );
    support::run_pending(&runtime).await;
    let (probe, root) = mount_editor(root_id, runtime);
    wait_until("the Layout canvas draws the matrix keys", || {
        count(root_id, "[data-matrix-id][data-row][data-column]") > 0
    })
    .await;
    (probe, root)
}

/// Mount main, join it to a layout, and anchor a 1x1 thumb to its key (0, 0).
async fn mount_anchored_thumb(root_id: &'static str) -> (EditorProbe, MountedRoot) {
    let (probe, root) = mount(root_id).await;
    let accepted = probe.runtime.model().accepted.expect("accepted document");
    let mut document = (*accepted.document).clone();
    document.layouts.push(
        serde_json::from_value(
            json!({ "id": "layout", "name": "Layout", "boardId": "board",
            "matrixIds": ["main"], "partIds": [] }),
        )
        .unwrap(),
    );
    support::submit_fixed_command(&probe.runtime, replace_command(document.revision, document));
    support::run_pending(&probe.runtime).await;
    let mut version = probe.version.borrow().expect("mounted version signal");
    version += 1;
    support::submit_fixed_command(
        &probe.runtime,
        anchor_command(
            probe.revision(),
            "thumb",
            json!({ "matrixId": "main", "row": 0, "column": 0,
                    "follow": ["x", "y", "rotation"] }),
        ),
    );
    support::run_pending(&probe.runtime).await;
    let mut version = probe.version.borrow().expect("mounted version signal");
    version += 1;
    (probe, root)
}

/// Ticket 17: selecting a column draws the badge for the previous column it
/// carries, and the previous column keeps the violet dashed target.
#[wasm_bindgen_test]
async fn column_selection_badges_the_previous_column_it_carries() {
    let root_id = "anchor-badge-column";
    let (probe, _root) = mount(root_id).await;
    let before = probe.revision();
    probe.select(objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column: 2,
    });
    wait_until("the anchor badge renders for the column", || {
        query(root_id, "[data-anchor-chrome='badge']").is_some()
    })
    .await;
    assert_eq!(
        badge_label(root_id),
        "Column 2 · spread, stagger, splay carry",
        "the badge names what the column is placed from"
    );
    wait_until("the previous column keeps the violet target", || {
        highlighted_columns(root_id) == vec!["1".to_owned(); ROWS as usize]
    })
    .await;
    assert!(
        query(root_id, "[data-anchor-chrome='tie']").is_some(),
        "a dashed tie runs to the selection"
    );
    assert_eq!(
        probe.revision(),
        before,
        "drawing the badge never writes a document revision"
    );
}

/// Ticket 17: an anchored matrix badges its anchor key ("follows X, Y,
/// rotation"), and the key keeps the violet dashed target.
#[wasm_bindgen_test]
async fn anchored_matrix_badges_its_anchor_key() {
    let root_id = "anchor-badge-matrix";
    let (probe, _root) = mount_anchored_thumb(root_id).await;
    probe.select(objects::TreeContext::Matrix {
        matrix_id: "thumb".into(),
    });
    wait_until("the anchor badge renders for the matrix", || {
        query(root_id, "[data-anchor-chrome='badge']").is_some()
    })
    .await;
    assert_eq!(
        badge_label(root_id),
        "main R1C1 · follows X, Y, rotation",
        "the badge names the anchor key"
    );
    wait_until("the anchor key keeps the violet target", || {
        highlighted_keys(root_id) == vec![("0".to_owned(), "0".to_owned())]
    })
    .await;
    assert!(
        query(root_id, "[data-anchor-chrome='tie']").is_some(),
        "a dashed tie runs to the selection"
    );
}

/// Ticket 17: a carried key badges the key below it, and that key keeps the
/// violet dashed target.
#[wasm_bindgen_test]
async fn key_selection_badges_the_key_below_it() {
    let root_id = "anchor-badge-key";
    let (probe, _root) = mount(root_id).await;
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 1,
        column: 2,
    });
    wait_until("the anchor badge renders for the key", || {
        query(root_id, "[data-anchor-chrome='badge']").is_some()
    })
    .await;
    assert_eq!(
        badge_label(root_id),
        "Key R1C3 · carried from below",
        "the badge names the key below"
    );
    wait_until("the key below keeps the violet target", || {
        highlighted_keys(root_id) == vec![("0".to_owned(), "2".to_owned())]
    })
    .await;
    assert!(
        query(root_id, "[data-anchor-chrome='tie']").is_some(),
        "a dashed tie runs to the selection"
    );
}

/// A split pair through edits: main on the left, paired on the right, joined
/// by a legacy (board-coordinate) mirror link. Focus starts on the Left half,
/// so the right half dims.
async fn mount_split(root_id: &'static str) -> (EditorProbe, MountedRoot) {
    let runtime = test_mount_helpers::open_document(base()).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(
        &runtime,
        set_matrix_command(
            revision,
            matrix_json(json!({"rows": 2, "columns": 2, "origin": {"x": -40.0, "y": 0.0}})),
        ),
    );
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.expect("accepted document");
    let mut document = (*accepted.document).clone();
    document.layouts.push(
        serde_json::from_value(
            json!({ "id": "left", "name": "Left half", "boardId": "board",
            "matrixIds": ["main"], "partIds": [] }),
        )
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(
            json!({ "id": "right", "name": "Right half", "boardId": "board",
            "matrixIds": [], "partIds": [] }),
        )
        .unwrap(),
    );
    document.mirrored_pairs = serde_json::from_value(json!([{
        "id": "pair",
        "source": {"kind": "sameBoardLayout", "id": "left"},
        "destination": {"kind": "sameBoardLayout", "id": "right"},
        "mapping": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0},
        "overrides": []
    }]))
    .unwrap();
    support::submit_fixed_command(&runtime, replace_command(document.revision, document));
    support::run_pending(&runtime).await;
    let (probe, root) = mount_editor(root_id, runtime);
    wait_until("the Layout canvas draws both halves", || {
        count(root_id, "[data-matrix-id][data-row][data-column]") == 8
    })
    .await;
    (probe, root)
}

/// Ticket 17: the badge draws on the selected half, including a dimmed one.
/// A key selected on the unfocused half badges its anchor with the violet
/// target, which wins over dimming like the selection does.
#[wasm_bindgen_test]
async fn badge_draws_on_the_dimmed_half() {
    let root_id = "anchor-badge-dimmed";
    let (probe, _root) = mount_split(root_id).await;
    wait_until("the right half dims", || {
        query(root_id, "[data-focus-label='split']").is_some_and(|label| {
            label
                .text_content()
                .unwrap_or_default()
                .contains("Right half")
        })
    })
    .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: "mirror_70616972_main".into(),
        row: 1,
        column: 1,
    });
    wait_until("the anchor badge renders on the dimmed half", || {
        query(root_id, "[data-anchor-chrome='badge']").is_some()
    })
    .await;
    assert_eq!(
        badge_label(root_id),
        "Key R1C2 · carried from below",
        "the badge names the anchor on the dimmed half"
    );
    let target = element(
        root_id,
        "[data-matrix-id='mirror_70616972_main'][data-row='0'][data-column='1']",
    );
    assert!(
        target.get_attribute("data-anchor-target").as_deref() == Some("true"),
        "the badge target keeps the violet dashed outline, not the dim"
    );
    assert!(
        target.get_attribute("data-dimmed").as_deref() == Some("false"),
        "the badge target does not fade with the dimmed half"
    );
}

/// Ticket 17: the badge clears when the selection clears.
#[wasm_bindgen_test]
async fn clearing_the_selection_clears_the_badge() {
    let root_id = "anchor-badge-clear";
    let (probe, _root) = mount(root_id).await;
    probe.select(objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column: 2,
    });
    wait_until("the anchor badge renders for the column", || {
        query(root_id, "[data-anchor-chrome='badge']").is_some()
    })
    .await;
    probe.select(objects::TreeContext::Board {
        board_id: "board".into(),
    });
    wait_until("clearing the selection clears the badge", || {
        query(root_id, "[data-anchor-chrome='badge']").is_none()
    })
    .await;
    assert_eq!(
        count(root_id, "[data-matrix-id][data-anchor-target='true']"),
        0,
        "the badge target clears with the badge"
    );
}
