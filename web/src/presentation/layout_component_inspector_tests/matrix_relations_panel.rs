use super::super::layout_workspace;
use super::super::*;
use super::{
    by_role_name, click_component_inspector, component_inspector_control,
    settle_component_inspector, wait_until,
};
use boardstudio_application::{Event, Session};
use boardstudio_core::CoreEngine;
use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditPhase, ProjectDoc, Vec2};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_test::wasm_bindgen_test;

const MATRIX: &str = "main";

#[derive(Clone)]
struct RelationsProbe {
    runtime: Rc<crate::runtime::Runtime>,
    root_id: &'static str,
    render_generation: Rc<RefCell<Option<Signal<u64>>>>,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    selected: Rc<RefCell<Option<Signal<Option<objects::ScopedTreeContext>>>>>,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
}

impl RelationsProbe {
    fn scope(&self) -> Scope {
        self.runtime.scope().expect("opened project scope")
    }

    fn select(&self, context: objects::TreeContext) {
        let adapter = self.adapter.borrow().clone().expect("mounted adapter");
        selection::submit_context(
            &self.runtime,
            &adapter,
            selection::ContextRequest {
                scope: self.scope(),
                context,
                mode: boardstudio_application::SelectionMode::Replace,
            },
        );
    }

    fn selected_context(&self) -> Option<objects::TreeContext> {
        let signal = (*self.selected.borrow())?;
        signal
            .peek()
            .as_ref()
            .map(|selected| selected.context.clone())
    }

    fn bump(&self) {
        let mut version = self.version.borrow().expect("mounted version signal");
        version += 1;
    }

    async fn settle(&self) {
        wait_until("the mounted relations Inspector host renders", || {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!("#{} [data-test-render-generation]", self.root_id))
                .unwrap()
                .is_some()
        })
        .await;
        let generation =
            { (*self.render_generation.borrow()).expect("mounted render-generation signal") };
        settle_component_inspector(self.root_id, generation).await;
    }

    /// Drive every pending effect, re-render the host and let effects settle.
    async fn land(&self) {
        boardstudio_web_test_runtime::project_name_test_support::run_pending(&self.runtime).await;
        self.settle().await;
        boardstudio_web_test_runtime::project_name_test_support::run_pending(&self.runtime).await;
        self.bump();
        self.settle().await;
    }

    fn text(&self) -> String {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(self.root_id)
            .unwrap()
            .text_content()
            .unwrap_or_default()
    }

    fn click(&self, role: &str, name: &str) {
        click_component_inspector(self.root_id, role, name);
    }

    fn reference(&self, part_id: &str) -> web_sys::HtmlElement {
        let name = self
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .find(|part| part.id == part_id)
            .expect("referenced part")
            .reference
            .clone();
        component_inspector_control(self.root_id, "button", &name)
    }

    fn exists(&self, selector: &str) -> bool {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} {selector}", self.root_id))
            .unwrap()
            .is_some()
    }

    async fn open_relations_tab(&self) {
        self.click("tab", "Relations");
        self.settle().await;
    }

    fn matrix(&self) -> boardstudio_core::model::Matrix {
        self.runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == MATRIX)
            .cloned()
            .expect("the matrix exists")
    }
}

#[component]
fn relations_host() -> Element {
    let probe = use_context::<RelationsProbe>();
    let render_generation = use_signal(|| 0u64);
    let _ = render_generation();
    *probe.render_generation.borrow_mut() = Some(render_generation);
    let version = use_signal(|| 0u64);
    *probe.version.borrow_mut() = Some(version);
    let _ = version();
    let workspace = use_signal(|| "Layout");
    let scope_generation = use_signal(|| 1u64);
    let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    *probe.selected.borrow_mut() = Some(selected_context);
    let anchor_scope = use_signal(|| None::<Scope>);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    *probe.adapter.borrow_mut() = Some(adapter.clone());
    // The key actions mounted by the Inspector read the Editor's shared contexts.
    use_context_provider(|| probe.runtime.clone());
    use_context_provider(|| adapter.clone());
    use_context_provider(|| version);
    use_context_provider(|| WorkspaceState(workspace));
    let relation_highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| RelationHighlight(relation_highlight));
    let state = layout_workspace::use_layout_workspace_state(selected_context, workspace);
    let runtime = probe.runtime.clone();
    let lifetime = use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
    let model = runtime.model();
    let selected = selected_context();
    let owner_key = layout_component_inspector_owner_key(&model, workspace(), selected.as_ref());
    let generation = lifetime.update(owner_key);
    let component_inspector = layout_component_inspector_projection(
        &model,
        selected.as_ref(),
        generation,
        scope_generation(),
    );
    let matrix_inspector = objects::use_matrix_inspector(
        runtime.clone(),
        version,
        selected_context,
        workspace,
        adapter.generation,
    );
    let board_inspector = board_inspector::use_board_inspector(
        runtime.clone(),
        selected_context,
        workspace,
        adapter.generation,
    );
    let key_size = objects::use_key_size(
        runtime.clone(),
        version,
        selected_context,
        workspace,
        adapter.generation,
    );
    let transform = objects::use_workspace_matrix_transform(
        runtime.clone(),
        version,
        selected_context,
        workspace,
        adapter.generation,
        state.splay_affect(),
        "Layout",
    );
    let (x, y, margin, error) = state.component_inspector_drafts();
    let (layout_pending, constraint_pending, remove_pending, mirrored_half_pending) =
        state.component_inspector_pending_controls();
    let (companion_pending, companion_error, companion_edits) =
        state.companion_placement_controls();
    let (x_failure, y_failure, margin_failure) = state.component_inspector_field_failures();
    let select_runtime = runtime.clone();
    let select_adapter = adapter.clone();
    rsx! {
        {layout_workspace::inspector(layout_workspace::InspectorInput {
            context_title: None,
            context_detail: None,
            show_position_inspector: false,
            component_inspector,
            component_inspector_pending_edits: state.component_inspector_edits(),
            component_inspector_x: x,
            component_inspector_y: y,
            component_inspector_margin: margin,
            component_inspector_error: error,
            component_inspector_x_failure: x_failure,
            component_inspector_y_failure: y_failure,
            component_inspector_margin_failure: margin_failure,
            component_inspector_layout_pending: layout_pending,
            component_inspector_constraint_pending: constraint_pending,
            component_inspector_remove_pending: remove_pending,
            component_inspector_mirrored_half_pending: mirrored_half_pending,
            companion_placement: None,
            companion_placement_pending: companion_pending,
            companion_placement_error: companion_error,
            companion_placement_edits: companion_edits,
            on_companion_placement_action: EventHandler::new(|_| {}),
            on_component_inspector_action: EventHandler::new(|_| {}),
            matrix_inspector,
            key_size,
            matrix_transform_inspector: transform,
            on_pick_splay_origin: EventHandler::new(|()| {}),
            splay_origin_pick_pending: false,
            on_cancel_splay_origin_pick: EventHandler::new(|()| {}),
            on_pick_anchor: EventHandler::new(|()| {}),
            anchor_pick_pending: false,
            on_cancel_anchor_pick: EventHandler::new(|()| {}),
            inspector_tab: state.inspector_tab(),
            matrix_relations: matrix_context_relations(&model, selected.as_ref()),
            relation_target: {
                let runtime = runtime.clone();
                Rc::new(move |request| relation_target_request(&runtime, request))
            },
            relation_scope: selected.as_ref().map(|selected| selected.scope.clone()),
            on_select_context: EventHandler::new(move |request: objects::TreeSelectRequest| {
                selection::submit_context(&select_runtime, &select_adapter, request.into());
            }),
            on_navigate_anchor_part: EventHandler::new(|_| {}),
            outline_inspector: None,
            findings_return_available: false,
            on_findings_return: EventHandler::new(|()| {}),
            board_inspector_mount: board_inspector,
            findings_page: None,
        })}
        output { "data-test-render-generation": "", "{render_generation()}" }
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
        serde_json::to_value(ProjectDoc::empty("relations-doc", "Relations")).unwrap();
    document["definitions"] = json!([
        definition("switch", "switch"),
        definition("diode", "passive")
    ]);
    document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
            "partIds": [], "netIds": [], "thickness": 1.6 }]);
    serde_json::from_value(document).unwrap()
}

fn matrix_json(rows: u32, columns: u32, extra: serde_json::Value) -> serde_json::Value {
    let mut matrix = json!({
        "id": MATRIX, "rows": rows, "columns": columns,
        "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
        "definitionId": "switch", "boardId": "board", "partIds": []
    });
    for (key, value) in extra.as_object().into_iter().flatten() {
        matrix[key] = value.clone();
    }
    matrix
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

/// Open `document`, then create the matrix through the Session and Core.
async fn mount(
    root_id: &'static str,
    document: ProjectDoc,
    matrix: Option<serde_json::Value>,
) -> (RelationsProbe, web_sys::Element) {
    let support = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(
        &support,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        CoreEngine::new(),
    );
    boardstudio_web_test_runtime::project_name_test_support::install_memory_persistence(&support);
    support.submit(Event::Open {
        operation_id: support.operation(),
        document,
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&support).await;
    if let Some(matrix) = matrix {
        let revision = support.model().accepted.unwrap().document.revision;
        boardstudio_web_test_runtime::project_name_test_support::submit_fixed_command(
            &support,
            set_matrix_command(revision, matrix),
        );
        boardstudio_web_test_runtime::project_name_test_support::run_pending(&support).await;
    }
    let probe = RelationsProbe {
        runtime: support,
        root_id,
        render_generation: Rc::default(),
        version: Rc::default(),
        selected: Rc::default(),
        adapter: Rc::default(),
    };
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    root.set_id(root_id);
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(relations_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    probe.settle().await;
    (probe, root)
}

fn matrix_ctx() -> objects::TreeContext {
    objects::TreeContext::Matrix {
        matrix_id: MATRIX.into(),
    }
}
fn column_ctx(column: u32) -> objects::TreeContext {
    objects::TreeContext::Column {
        matrix_id: MATRIX.into(),
        column,
    }
}
fn key_ctx(row: u32, column: u32) -> objects::TreeContext {
    objects::TreeContext::Key {
        matrix_id: MATRIX.into(),
        row,
        column,
    }
}

#[wasm_bindgen_test]
async fn a_column_offset_departure_resets_with_undo_and_converts_to_a_stagger() {
    let (probe, root) = mount(
            "relations-departure-test",
            base(),
            Some(matrix_json(
                2,
                3,
                json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 2.0}}]
}),
            )),
        )
        .await;
    probe.select(column_ctx(1));
    probe.land().await;
    probe.open_relations_tab().await;
    assert!(probe.exists("[data-relation-departure='Column offset']"));

    probe.click("button", "Reset offsets: Column offset");
    probe.land().await;
    assert_eq!(
        probe.matrix().column_layout[1].offset,
        Vec2 { x: 0.0, y: 0.0 },
        "Reset lands through SetMatrixPlacement"
    );
    assert!(
        !probe.exists("[data-relation-departure='Column offset']"),
        "the departure disappears: {}",
        probe.text()
    );

    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    probe.land().await;
    assert_eq!(
        probe.matrix().column_layout[1].offset,
        Vec2 { x: 1.5, y: 2.0 }
    );
    assert!(
        probe.exists("[data-relation-departure='Column offset']"),
        "Undo restores the departure: {}",
        probe.text()
    );

    probe.click("button", "Convert to stagger: Column offset");
    probe.land().await;
    let matrix = probe.matrix();
    assert_eq!(matrix.column_layout[1].stagger, 2.0);
    assert_eq!(matrix.column_layout[1].offset, Vec2 { x: 1.5, y: 0.0 });
    assert!(
        probe.text().contains("2 mm from Column 1, cumulative 2 mm"),
        "Structure shows the new stagger: {}",
        probe.text()
    );
    root.remove();
}

/// A splayed column keeps its Column offset departure, but Convert is
/// disabled with the Core-owned reason wired through `aria-describedby`.
/// Rendering the refusal writes nothing.
#[wasm_bindgen_test]
async fn a_blocked_convert_in_relations_names_the_core_reason() {
    let (probe, root) = mount(
            "relations-blocked-convert-test",
            base(),
            Some(matrix_json(
                2,
                3,
                json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 2.0}, "splay": 10.0}]
}),
            )),
        )
        .await;
    let before = probe
        .runtime
        .model()
        .accepted
        .expect("accepted document")
        .document
        .revision;
    probe.select(column_ctx(1));
    probe.land().await;
    probe.open_relations_tab().await;
    assert!(
        probe.exists("[data-relation-departure='Column offset']"),
        "the departure is listed: {}",
        probe.text()
    );
    let button = by_role_name(&root, "button", "Convert to stagger: Column offset")
        .expect("Convert is mounted");
    assert!(
        button.has_attribute("disabled"),
        "splay blocks Convert in Relations too"
    );
    let reason_id = button
        .get_attribute("aria-describedby")
        .expect("described reason");
    let reason = root
        .query_selector(&format!("#{reason_id}"))
        .unwrap()
        .expect("the Core reason is shown");
    assert_eq!(
        reason.text_content().as_deref(),
        Some("Converting requires zero Splay, Column rotation and carried Bend")
    );
    assert_eq!(
        button.get_attribute("aria-describedby").as_deref(),
        reason.get_attribute("id").as_deref(),
        "the refusal is programmatically named"
    );
    assert!(
        button
            .get_attribute("aria-describedby")
            .as_deref()
            .is_some_and(|id| !id.is_empty()),
        "the description reference is not empty"
    );
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .revision,
        before,
        "rendering a refusal writes nothing, so no Undo entry"
    );
    root.remove();
}

/// The `tie` offset constraint wins over its companion's attachment to its
/// key: Core derives the Constrained mode, and Relations names that
/// precedence in text.
#[wasm_bindgen_test]
async fn a_won_constraint_names_its_priority_over_the_key_attachment() {
    let (document, _) = constrained_companion_document();
    let (probe, root) = mount("relations-precedence-test", document, None).await;
    probe.select(matrix_ctx());
    probe.land().await;
    probe.open_relations_tab().await;
    let row = root
        .query_selector("[data-relation-constraint='tie']")
        .unwrap()
        .expect("the constraint is listed");
    assert!(
        row.text_content()
            .unwrap_or_default()
            .contains("Takes priority over the attachment to its key."),
        "Core-owned precedence is explained: {}",
        probe.text()
    );
    root.remove();
}

/// Two keys' companions, one tied to the first key by an explicit constraint. Core
/// generates the part ids, so a scratch engine does the work and the document that
/// results opens in the Session.
fn constrained_companion_document() -> (ProjectDoc, String) {
    let mut engine = CoreEngine::new();
    let opened = match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }) {
        CoreReply::Scene { scene, .. } => scene,
        other => panic!("{other:?}"),
    };
    let assemblies: Vec<serde_json::Value> = ["a", "b"]
        .iter()
        .enumerate()
        .map(|(index, id)| {
            json!({ "id": id, "definitionId": "diode",
                        "offset": { "x": 0.0, "y": 8.0 + index as f64 * 3.0 } })
        })
        .collect();
    let matrix = matrix_json(
        1,
        2,
        json!({ "cells": [{ "row": 0, "column": 0, "enabled": true, "assemblies": assemblies }] }),
    );
    let (scene, mut document) = match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: set_matrix_command(opened.revision, matrix),
    }) {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("{other:?}"),
    };
    let companion = scene.matrix_scenes[0].companions[1].part_id.clone();
    document
        .constraints
        .push(boardstudio_core::model::Constraint::Offset {
            id: "tie".into(),
            source_part_id: "matrix/main/r0c0".into(),
            target_part_id: companion.clone(),
            offset: Vec2 { x: 0.0, y: 12.0 },
            rotation: 0.0,
        });
    (document, companion)
}

#[wasm_bindgen_test]
async fn a_constraint_reference_selects_its_object_and_escape_returns() {
    let (document, companion) = constrained_companion_document();
    let (probe, root) = mount("relations-constraint-test", document, None).await;
    probe.select(matrix_ctx());
    probe.land().await;
    probe.open_relations_tab().await;
    assert!(
        probe.exists("[data-relation-constraint='tie']"),
        "the constraint is listed: {}",
        probe.text()
    );

    probe.reference("matrix/main/r0c0").click();
    probe.land().await;
    assert_eq!(probe.selected_context(), Some(key_ctx(0, 0)));

    probe.select(matrix_ctx());
    probe.land().await;
    probe.open_relations_tab().await;
    probe.reference(&companion).click();
    probe.land().await;
    assert!(
        matches!(
            probe.selected_context(),
            Some(objects::TreeContext::Component { part_id: Some(ref id), .. }) if *id == companion
        ),
        "the companion is selected: {:?}",
        probe.selected_context()
    );

    let options = web_sys::KeyboardEventInit::new();
    options.set_key("Escape");
    options.set_bubbles(true);
    let event =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options).unwrap();
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .dispatch_event(&event)
        .unwrap();
    probe.land().await;
    assert!(
        matches!(
            probe.selected_context(),
            Some(objects::TreeContext::Board { .. })
        ),
        "Escape returns: {:?} text={}",
        probe.selected_context(),
        probe.text()
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_reference_to_a_removed_key_disappears_and_a_queued_click_does_nothing() {
    let (document, _) = constrained_companion_document();
    let (probe, root) = mount("relations-removed-test", document, None).await;
    probe.select(matrix_ctx());
    probe.land().await;
    probe.open_relations_tab().await;
    let source = "[data-relation-constraint='tie'] [data-relation-role='source']";
    assert!(probe.exists(source), "{}", probe.text());

    let queued_source = probe.reference("matrix/main/r0c0");
    // Remove the referenced key, letting the edit land without re-rendering the panel:
    // the button is still in the DOM, as when a click was queued before the removal.
    let accepted = probe.runtime.model().accepted.unwrap();
    boardstudio_web_test_runtime::project_name_test_support::submit_fixed_command(
        &probe.runtime,
        EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "remove-key".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: boardstudio_core::model::EditOperation::RemoveMatrixCells {
                matrix_id: MATRIX.into(),
                cells: vec![(0, 0)],
            },
        },
    );
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
    let landed = probe.runtime.model().accepted.unwrap();
    assert!(
        landed.document.revision > accepted.document.revision,
        "the removal landed"
    );
    let before = probe.selected_context();
    queued_source.click();
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
    assert_eq!(
        probe.selected_context(),
        before,
        "the stale click did nothing"
    );

    probe.bump();
    probe.settle().await;
    assert!(
        !probe.exists(source),
        "the removed key's row is gone: {}",
        probe.text()
    );
    root.remove();
}
