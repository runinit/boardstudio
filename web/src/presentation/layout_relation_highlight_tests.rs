//! Relations references highlight their parts on the Layout canvas. The highlight is
//! transient view state: it changes neither the selection nor the document.
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
}

impl Probe {
    fn select(&self, context: objects::TreeContext) {
        let adapter = self.adapter.borrow().clone().expect("mounted adapter");
        selection::submit_context(
            &self.runtime,
            &adapter,
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

    fn element(&self, selector: &str) -> web_sys::Element {
        query(self.root_id, selector).unwrap_or_else(|| panic!("no element for {selector}"))
    }

    /// Open the matrix Relations tab for the current selection.
    async fn open_relations(&self) {
        click(
            self.root_id,
            ".m1-layout-component-tabs button:nth-child(2)",
        );
        wait_until(
            "the Relations tab lists the Inherited by references",
            || {
                query(
                    self.root_id,
                    "li[data-relation-label='Inherited by'] button",
                )
                .is_some()
            },
        )
        .await;
    }

    /// The first reference button under the "Inherited by" structure rows.
    fn inherited_by_reference(&self) -> web_sys::HtmlElement {
        self.element("li[data-relation-label='Inherited by'] button")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
    }

    /// Land an edit, then re-render the host the way the page does after an accepted revision.
    async fn land(&self, command: EditCommand) {
        support::submit_fixed_command(&self.runtime, command);
        support::run_pending(&self.runtime).await;
        let mut version = self.version.borrow().expect("mounted version signal");
        version += 1;
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
    let mut document = serde_json::to_value(ProjectDoc::empty(
        "relation-highlight",
        "Relation highlight",
    ))
    .unwrap();
    document["definitions"] = json!([
        definition("switch", "switch"),
        definition("diode", "passive")
    ]);
    document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
        "partIds": [], "netIds": [], "thickness": 1.6 }]);
    serde_json::from_value(document).unwrap()
}

/// A ROWS x COLUMNS matrix. `extra` replaces top-level matrix fields, such as `cells`.
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

/// The key cells of `column`, each removed by a landed edit.
fn removed_column(column: u32) -> serde_json::Value {
    let cells: Vec<_> = (0..ROWS)
        .map(|row| json!({ "row": row, "column": column, "enabled": false, "deleted": true }))
        .collect();
    json!({ "cells": cells })
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

/// The `data-column` of every highlighted canvas key.
fn highlighted_columns(root_id: &str) -> Vec<String> {
    let list = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all(&format!("#{root_id} .m1-matrix-key.is-relation-highlight"))
        .unwrap();
    (0..list.length())
        .map(|index| {
            list.item(index)
                .unwrap()
                .dyn_into::<web_sys::Element>()
                .unwrap()
                .get_attribute("data-column")
                .unwrap_or_default()
        })
        .collect()
}

fn click(root_id: &str, selector: &str) {
    query(root_id, selector)
        .unwrap_or_else(|| panic!("no element for {selector}"))
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

fn dispatch(element: &web_sys::Element, kind: &str) {
    let event = web_sys::PointerEvent::new(kind).unwrap();
    element.dispatch_event(event.as_ref()).unwrap();
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
    mount_with(root_id, matrix_json(json!({}))).await
}

async fn mount_with(root_id: &'static str, matrix: serde_json::Value) -> Probe {
    let runtime = support::new_runtime();
    support::install(&runtime, Session::new(), CoreEngine::new());
    support::install_memory_persistence(&runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: base(),
    });
    support::run_pending(&runtime).await;
    let revision = runtime.model().accepted.unwrap().document.revision;
    support::submit_fixed_command(&runtime, set_matrix_command(revision, matrix));
    support::run_pending(&runtime).await;
    let probe = Probe {
        runtime: runtime.clone(),
        root_id,
        adapter: Rc::default(),
        version: Rc::default(),
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
    wait_until("the Layout canvas draws the matrix keys", || {
        count(root_id, ".m1-matrix-key") > 0
    })
    .await;
    probe
}

/// Column 3 is selected and its Relations tab is open; "Inherited by Column 4" is first.
async fn mount_column_relations(root_id: &'static str) -> Probe {
    let probe = mount(root_id).await;
    probe.select(objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column: 2,
    });
    wait_until("the Relations tab button renders", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(2)").is_some()
    })
    .await;
    probe.open_relations().await;
    assert_eq!(
        probe.inherited_by_reference().text_content().unwrap(),
        "Column 4"
    );
    probe
}

#[wasm_bindgen_test]
async fn hovering_an_inherited_column_highlights_only_its_keys() {
    let root_id = "relation-highlight-hover";
    let probe = mount_column_relations(root_id).await;
    let before = probe.revision();
    let reference = probe.inherited_by_reference();
    dispatch(&reference, "pointerenter");
    wait_until("hovering Column 4 highlights its keys", || {
        highlighted_columns(root_id) == vec!["3".to_owned(); ROWS as usize]
    })
    .await;
    assert_eq!(
        highlighted_columns(root_id),
        vec!["3".to_owned(); ROWS as usize],
        "hovering Column 4 marks exactly Column 4's keys (data-column 3)"
    );
    assert_eq!(
        count(root_id, ".m1-matrix-key.is-selected"),
        ROWS,
        "the selected Column 3 keys keep the selection treatment"
    );
    dispatch(&reference, "pointerleave");
    wait_until("leaving the reference clears its highlight", || {
        count(root_id, ".m1-matrix-key.is-relation-highlight") == 0
    })
    .await;
    assert_eq!(count(root_id, ".m1-matrix-key.is-relation-highlight"), 0);
    assert_eq!(
        probe.revision(),
        before,
        "highlighting never writes a document revision, so no Undo entry"
    );
}

#[wasm_bindgen_test]
async fn keyboard_focus_highlights_and_blur_clears() {
    let root_id = "relation-highlight-focus";
    let probe = mount_column_relations(root_id).await;
    let reference = probe.inherited_by_reference();
    reference.focus().unwrap();
    wait_until("keyboard focus highlights Column 4's keys", || {
        highlighted_columns(root_id) == vec!["3".to_owned(); ROWS as usize]
    })
    .await;
    assert_eq!(
        highlighted_columns(root_id),
        vec!["3".to_owned(); ROWS as usize],
        "keyboard focus gives the same highlight as hover"
    );
    reference.blur().unwrap();
    wait_until("blurring the reference clears its highlight", || {
        count(root_id, ".m1-matrix-key.is-relation-highlight") == 0
    })
    .await;
    assert_eq!(count(root_id, ".m1-matrix-key.is-relation-highlight"), 0);
}

#[wasm_bindgen_test]
async fn changing_the_selection_clears_the_highlight() {
    let root_id = "relation-highlight-selection";
    let probe = mount_column_relations(root_id).await;
    dispatch(&probe.inherited_by_reference(), "pointerenter");
    wait_until("hovering the reference highlights every key", || {
        count(root_id, ".m1-matrix-key.is-relation-highlight") == ROWS
    })
    .await;
    assert_eq!(count(root_id, ".m1-matrix-key.is-relation-highlight"), ROWS);
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 0,
        column: 0,
    });
    wait_until("a selection change clears the Relations highlight", || {
        count(root_id, ".m1-matrix-key.is-relation-highlight") == 0
    })
    .await;
    assert_eq!(
        count(root_id, ".m1-matrix-key.is-relation-highlight"),
        0,
        "a selection change clears the Relations highlight"
    );
}

#[wasm_bindgen_test]
async fn a_key_removed_while_highlighted_leaves_nothing_highlighted() {
    let root_id = "relation-highlight-removed";
    let probe = mount_column_relations(root_id).await;
    dispatch(&probe.inherited_by_reference(), "pointerenter");
    wait_until("hovering the reference highlights Column 4", || {
        count(root_id, ".m1-matrix-key.is-relation-highlight") == ROWS
    })
    .await;
    let keys_before = count(root_id, ".m1-matrix-key");
    let revision = probe.revision();
    probe
        .land(set_matrix_command(revision, matrix_json(removed_column(3))))
        .await;
    wait_until(
        "the removed keys leave the canvas and their highlight",
        || {
            count(root_id, ".m1-matrix-key") == keys_before - ROWS
                && count(root_id, ".m1-matrix-key.is-relation-highlight") == 0
        },
    )
    .await;
    assert!(
        probe.revision() > revision,
        "the removal of Column 4's keys landed"
    );
    assert_eq!(
        count(root_id, ".m1-matrix-key"),
        keys_before - ROWS,
        "the removed keys leave the canvas"
    );
    assert_eq!(
        count(root_id, ".m1-matrix-key.is-relation-highlight"),
        0,
        "the removed keys are not highlighted"
    );
    assert_eq!(count(root_id, "[role='alert']"), 0, "no error is raised");
}

/// Row 0 shifts (10, 0) then bends 90; row 1 shifts (10, 0) into the turned
/// heading. The mounted key explanation must show the transported (10, 10)
/// matrix-axes total from real Core facts, and activating its Row shift entry
/// must open the row's editor without writing the document or history.
#[wasm_bindgen_test]
async fn bent_key_explains_its_cumulative_shift_and_opens_its_row() {
    let root_id = "relation-bent-key";
    let probe = mount_with(
        root_id,
        matrix_json(json!({
            "columns": 1,
            "rowLayout": [{ "shift": { "x": 10.0, "y": 0.0 }, "bend": 90.0 },
                          { "shift": { "x": 10.0, "y": 0.0 } }],
        })),
    )
    .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 1,
        column: 0,
    });
    wait_until("the Relations tab button renders", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(2)").is_some()
    })
    .await;
    click(root_id, ".m1-layout-component-tabs button:nth-child(2)");
    wait_until(
        "the key chain shows its Core-backed cumulative shift",
        || {
            query(root_id, "li[data-relation-label='Cumulative shift']").is_some_and(|item| {
                item.text_content()
                    .unwrap_or_default()
                    .contains("10, 10 mm in matrix axes")
            })
        },
    )
    .await;
    assert_eq!(
        probe
            .element("#m1-relations-structure-heading")
            .text_content()
            .unwrap(),
        "How it is placed"
    );
    let before = probe.revision();
    click(root_id, "li[data-relation-label='Row shift'] button");
    wait_until("activating Row shift opens the Properties tab", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(1)")
            .is_some_and(|tab| tab.get_attribute("aria-selected").unwrap_or_default() == "true")
    })
    .await;
    assert_eq!(
        probe.revision(),
        before,
        "parameter navigation writes nothing, so no Undo entry"
    );
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

/// A Follows reference navigates to its anchor key without writing: the key's
/// chain then names the matrix following it.
#[wasm_bindgen_test]
async fn anchored_follows_navigates_to_its_key_without_writing() {
    let root_id = "relation-anchored-follows";
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
    probe
        .land(replace_command(document.revision, document))
        .await;
    probe
        .land(anchor_command(
            probe.revision(),
            "thumb",
            json!({ "matrixId": "main", "row": 0, "column": 0,
                    "follow": ["x", "y", "rotation"] }),
        ))
        .await;
    // The anchor key's own chain names its follower.
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 0,
        column: 0,
    });
    wait_until("the Relations tab button renders", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(2)").is_some()
    })
    .await;
    click(root_id, ".m1-layout-component-tabs button:nth-child(2)");
    wait_until("the anchor key names the matrix following it", || {
        query(root_id, "li[data-relation-label='Followed by']").is_some_and(|item| {
            item.text_content()
                .unwrap_or_default()
                .contains("Matrix thumb")
        })
    })
    .await;
    // Selecting another object returns to Properties, so wait for the reset
    // to settle before re-opening Relations.
    probe.select(objects::TreeContext::Matrix {
        matrix_id: "thumb".into(),
    });
    wait_until("the tab resets to Properties for the new selection", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(1)")
            .is_some_and(|tab| tab.get_attribute("aria-selected").unwrap_or_default() == "true")
    })
    .await;
    click(root_id, ".m1-layout-component-tabs button:nth-child(2)");
    wait_until("the anchored matrix names its anchor key", || {
        query(root_id, "li[data-relation-label='Follows']").is_some_and(|item| {
            item.text_content()
                .unwrap_or_default()
                .contains("row 1, column 1 of matrix main")
        })
    })
    .await;
    let before = probe.revision();
    click(root_id, "li[data-relation-label='Follows'] button");
    // The reference navigates by changing the selection; the key view itself
    // is proven above, so poll the selection rather than racing its render.
    let adapter = probe.adapter.clone();
    let anchor_key = objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 0,
        column: 0,
    };
    wait_until("Follows selects the anchor key", || {
        adapter
            .borrow()
            .clone()
            .expect("mounted adapter")
            .selected_context
            .read()
            .clone()
            .is_some_and(|scoped| scoped.context == anchor_key)
    })
    .await;
    assert_eq!(
        probe.revision(),
        before,
        "reference navigation writes nothing, so no Undo entry"
    );
}

/// Selecting parts in two matrices must not merge their chains: the Relations
/// panel keeps the single key context and explains the spanning selection.
#[wasm_bindgen_test]
async fn selecting_parts_in_two_matrices_explains_the_single_context_chain() {
    let root_id = "relation-cross-matrix";
    let probe = mount(root_id).await;
    let revision = probe.revision();
    probe
        .land(set_matrix_command(
            revision,
            matrix_json(json!({"id": "second", "rows": 1, "columns": 1})),
        ))
        .await;
    probe.select(objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row: 0,
        column: 0,
    });
    probe.runtime.submit(Event::SelectParts {
        operation_id: probe.runtime.operation(),
        part_ids: vec!["matrix/main/r0c0".into(), "matrix/second/r0c0".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    support::run_pending(&probe.runtime).await;
    wait_until("the Relations tab button renders", || {
        query(root_id, ".m1-layout-component-tabs button:nth-child(2)").is_some()
    })
    .await;
    click(root_id, ".m1-layout-component-tabs button:nth-child(2)");
    wait_until("the spanning selection is explained, not merged", || {
        query(root_id, ".m1-relations-cross-matrix").is_some_and(|note| {
            note.text_content()
                .unwrap_or_default()
                .contains("Parts in 2 matrices are selected")
        })
    })
    .await;
    let note = probe
        .element(".m1-relations-cross-matrix")
        .text_content()
        .unwrap();
    assert!(
        note.contains("select one matrix context for detail"),
        "{note}"
    );
    assert_eq!(
        query(root_id, "li[data-relation-label='Pitch cell']").is_some(),
        true,
        "the single key chain still renders"
    );
}
