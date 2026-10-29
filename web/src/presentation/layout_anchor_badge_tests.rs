//! Ticket 17: the Layout canvas draws an anchor badge for the selection: the
//! authored icon at what the selection is placed from, a violet dashed target
//! on that reference, and a dashed tie to the selection. The badge clears when
//! the selection clears.
use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_application::{Event, Session};
use boardstudio_core::CoreEngine;
use boardstudio_core::model::{EditCommand, EditPhase, ProjectDoc};
use boardstudio_web_ui_shared::test_wait::wait_until;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

const MATRIX: &str = "main";
const ROWS: u32 = 2;
const COLUMNS: u32 = 5;

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    root_id: &'static str,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    hidden: Rc<RefCell<Option<Signal<BTreeSet<String>>>>>,
}

impl Probe {
    fn select(&self, context: objects::TreeContext) {
        selection::submit_context(
            &self.runtime,
            &self.adapter.borrow().clone().expect("mounted adapter"),
            selection::ContextRequest {
                scope: self.runtime.scope().expect("opened project scope"),
                context,
                mode: boardstudio_application::SelectionMode::Replace,
            },
        );
    }

    fn revision(&self) -> u64 {
        self.runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .revision
    }

    fn badge_label(&self) -> String {
        element(self.root_id, ".m1-anchor-badge-label")
            .text_content()
            .unwrap_or_default()
    }
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

fn definition(id: &str, kind: &str) -> serde_json::Value {
    json!({
        "id": id, "name": id, "kind": kind,
        "courtyard": [
            { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
            { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
        ],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
    })
}

fn base() -> ProjectDoc {
    let mut document =
        serde_json::to_value(ProjectDoc::empty("anchor-badge", "Anchor badge")).unwrap();
    document["definitions"] = json!([
        definition("switch", "switch"),
        definition("diode", "passive")
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
    query_all(root_id, ".m1-matrix-key.is-anchor-target")
        .iter()
        .map(|key| key.get_attribute("data-column").unwrap_or_default())
        .collect()
}

/// The `(row, column)` of every badge-target canvas key.
fn highlighted_keys(root_id: &str) -> Vec<(String, String)> {
    query_all(root_id, ".m1-matrix-key.is-anchor-target")
        .iter()
        .map(|key| {
            (
                key.get_attribute("data-row").unwrap_or_default(),
                key.get_attribute("data-column").unwrap_or_default(),
            )
        })
        .collect()
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let workspace = use_signal(|| "Layout");
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    let version = use_signal(|| 0u64);
    *probe.version.borrow_mut() = Some(version);
    use_context_provider(|| version);
    let created = use_signal(|| None::<SetupGuideRequest>);
    use_context_provider(|| created);
    let objects_open = use_signal(|| true);
    let inspector_open = use_signal(|| true);
    use_context_provider(|| CompactPanelState {
        objects_open,
        inspector_open,
    });
    let warning = use_signal(|| false);
    use_context_provider(|| PreferenceStorageWarning(warning));
    let theme = use_signal(|| "light");
    use_context_provider(|| ThemeState(theme));
    let resolved = use_memo(|| "light");
    use_context_provider(|| ResolvedTheme(resolved));
    let selected = use_signal(|| None);
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    let adapter = SelectionAdapter::new(selected, anchor, generation);
    *probe.adapter.borrow_mut() = Some(adapter.clone());
    use_context_provider(|| adapter);
    let hidden = use_signal(BTreeSet::new);
    let modules_hidden = use_signal(BTreeSet::new);
    let footprints = use_signal(|| false);
    *probe.hidden.borrow_mut() = Some(hidden);
    use_context_provider(|| LayerVisibility {
        hidden,
        modules_hidden,
        footprints,
    });
    boardstudio_web_ui_model::state::use_test_case_generation_state();
    rsx! { Editor {} }
}

/// Create the matrix through the Session and Core, then mount the real Layout editor.
async fn mount(root_id: &'static str) -> Probe {
    let runtime = support::new_runtime();
    support::install(&runtime, Session::new(), CoreEngine::new());
    support::install_memory_persistence(&runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: base(),
    });
    support::run_pending(&runtime).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(
        &runtime,
        set_matrix_command(revision, matrix_json(json!({}))),
    );
    support::run_pending(&runtime).await;
    let probe = launch_host(root_id, runtime);
    wait_until("the Layout canvas draws the matrix keys", || {
        count(root_id, ".m1-matrix-key") > 0
    })
    .await;
    probe
}

fn launch_host(root_id: &'static str, runtime: Rc<crate::runtime::Runtime>) -> Probe {
    let probe = Probe {
        runtime: runtime.clone(),
        root_id,
        adapter: Rc::default(),
        version: Rc::default(),
        hidden: Rc::default(),
    };
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    root.set_id(root_id);
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    probe
}

/// Mount main, join it to a layout, and anchor a 1x1 thumb to its key (0, 0).
async fn mount_anchored_thumb(root_id: &'static str) -> Probe {
    let probe = mount(root_id).await;
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
    probe
}

/// Ticket 17: selecting a column draws the badge for the previous column it
/// carries, and the previous column keeps the violet dashed target.
#[wasm_bindgen_test]
async fn column_selection_badges_the_previous_column_it_carries() {
    let root_id = "anchor-badge-column";
    let probe = mount(root_id).await;
    let before = probe.revision();
    probe.select(objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column: 2,
    });
    wait_until("the anchor badge renders for the column", || {
        query(root_id, ".m1-anchor-badge").is_some()
    })
    .await;
    assert_eq!(
        probe.badge_label(),
        "Column 2 · spread, stagger, splay carry",
        "the badge names what the column is placed from"
    );
    wait_until("the previous column keeps the violet target", || {
        highlighted_columns(root_id) == vec!["1".to_owned(); ROWS as usize]
    })
    .await;
    assert!(
        query(root_id, ".m1-anchor-tie").is_some(),
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
    let probe = mount_anchored_thumb(root_id).await;
    probe.select(objects::TreeContext::Matrix {
        matrix_id: "thumb".into(),
    });
    wait_until("the anchor badge renders for the matrix", || {
        query(root_id, ".m1-anchor-badge").is_some()
    })
    .await;
    assert_eq!(
        probe.badge_label(),
        "main R1C1 · follows X, Y, rotation",
        "the badge names the anchor key"
    );
    wait_until("the anchor key keeps the violet target", || {
        highlighted_keys(root_id) == vec![("0".to_owned(), "0".to_owned())]
    })
    .await;
    assert!(
        query(root_id, ".m1-anchor-tie").is_some(),
        "a dashed tie runs to the selection"
    );
}

/// Ticket 17: a carried key badges the key below it, and that key keeps the
/// violet dashed target.
#[wasm_bindgen_test]
async fn key_selection_badges_the_key_below_it() {
    let root_id = "anchor-badge-key";
    let probe = mount(root_id).await;
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 1,
        column: 2,
    });
    wait_until("the anchor badge renders for the key", || {
        query(root_id, ".m1-anchor-badge").is_some()
    })
    .await;
    assert_eq!(
        probe.badge_label(),
        "Key R1C3 · carried from below",
        "the badge names the key below"
    );
    wait_until("the key below keeps the violet target", || {
        highlighted_keys(root_id) == vec![("0".to_owned(), "2".to_owned())]
    })
    .await;
    assert!(
        query(root_id, ".m1-anchor-tie").is_some(),
        "a dashed tie runs to the selection"
    );
}

/// A split pair through edits: main on the left, paired on the right, joined
/// by a legacy (board-coordinate) mirror link. Focus starts on the Left half,
/// so the right half dims.
async fn mount_split(root_id: &'static str) -> Probe {
    let runtime = support::new_runtime();
    support::install(&runtime, Session::new(), CoreEngine::new());
    support::install_memory_persistence(&runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: base(),
    });
    support::run_pending(&runtime).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(
        &runtime,
        set_matrix_command(
            revision,
            matrix_json(json!({"rows": 2, "columns": 2, "origin": {"x": -40.0, "y": 0.0}})),
        ),
    );
    support::run_pending(&runtime).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(
        &runtime,
        set_matrix_command(
            revision,
            matrix_json(
                json!({"id": "paired", "rows": 2, "columns": 2, "origin": {"x": 40.0, "y": 0.0}}),
            ),
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
            "matrixIds": ["paired"], "partIds": [],
            "mirrorLink": { "sourceId": "left", "axisX": 0.0,
                "correspondences": [{ "kind": "matrix", "sourceId": "main", "mirrorId": "paired" }],
                "exceptions": [] } }),
        )
        .unwrap(),
    );
    support::submit_fixed_command(&runtime, replace_command(document.revision, document));
    support::run_pending(&runtime).await;
    let probe = launch_host(root_id, runtime);
    wait_until("the Layout canvas draws both halves", || {
        count(root_id, ".m1-matrix-key") == 8
    })
    .await;
    probe
}

/// Ticket 17: the badge draws on the selected half, including a dimmed one.
/// A key selected on the unfocused half badges its anchor with the violet
/// target, which wins over dimming like the selection does.
#[wasm_bindgen_test]
async fn badge_draws_on_the_dimmed_half() {
    let root_id = "anchor-badge-dimmed";
    let probe = mount_split(root_id).await;
    wait_until("the right half dims", || {
        query(root_id, ".m1-split-focus-label").is_some_and(|label| {
            label
                .text_content()
                .unwrap_or_default()
                .contains("Right half")
        })
    })
    .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: "paired".into(),
        row: 1,
        column: 1,
    });
    wait_until("the anchor badge renders on the dimmed half", || {
        query(root_id, ".m1-anchor-badge").is_some()
    })
    .await;
    assert_eq!(
        probe.badge_label(),
        "Key R1C2 · carried from below",
        "the badge names the anchor on the dimmed half"
    );
    let target = element(
        root_id,
        ".m1-matrix-key[data-matrix-id='paired'][data-row='0'][data-column='1']",
    );
    let class = target.get_attribute("class").unwrap_or_default();
    assert!(
        class.contains("is-anchor-target"),
        "the badge target keeps the violet dashed outline, not the dim: {class}"
    );
    assert!(
        !class.contains("is-dimmed"),
        "the badge target does not fade with the dimmed half: {class}"
    );
}

/// Ticket 17: the badge clears when the selection clears.
#[wasm_bindgen_test]
async fn clearing_the_selection_clears_the_badge() {
    let root_id = "anchor-badge-clear";
    let probe = mount(root_id).await;
    probe.select(objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column: 2,
    });
    wait_until("the anchor badge renders for the column", || {
        query(root_id, ".m1-anchor-badge").is_some()
    })
    .await;
    probe.select(objects::TreeContext::Board {
        board_id: "board".into(),
    });
    wait_until("clearing the selection clears the badge", || {
        query(root_id, ".m1-anchor-badge").is_none()
    })
    .await;
    assert_eq!(
        count(root_id, ".m1-matrix-key.is-anchor-target"),
        0,
        "the badge target clears with the badge"
    );
}
