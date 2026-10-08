use super::*;
use crate::presentation::*;
use crate::presentation::{
    SelectionAdapter, WorkspaceState,
    objects::{ScopedTreeContext, TreeContext},
};
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    version: Rc<RefCell<Option<Signal<u64>>>>,
    adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
}
#[component]
fn host() -> Element {
    let probe = use_context::<Probe>();
    use_context_provider(|| probe.runtime.clone());
    let version = use_signal(|| 0u64);
    if probe.version.borrow().is_none() {
        *probe.version.borrow_mut() = Some(version);
    }
    use_context_provider(|| version);
    let workspace = use_signal(|| "Layout");
    if probe.workspace.borrow().is_none() {
        *probe.workspace.borrow_mut() = Some(workspace);
    }
    use_context_provider(|| WorkspaceState(workspace));
    let selected_context = use_signal(|| {
        Some(ScopedTreeContext {
            scope: probe.runtime.scope().unwrap(),
            context: TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
            },
        })
    });
    let anchor_scope = use_signal(|| probe.runtime.scope());
    let generation = use_signal(|| 1u64);
    let adapter = SelectionAdapter::new(selected_context, anchor_scope, generation);
    if probe.adapter.borrow().is_none() {
        *probe.adapter.borrow_mut() = Some(adapter.clone());
    }
    use_context_provider(|| adapter);
    rsx! { SelectedKeysActions {} }
}
fn definition(id: &str, name: &str) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: name.into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    }
}

/// A one-key matrix document. The test opens it through the real Session and Core, which
/// generate the scene the Inspector reads.
fn fixture() -> (ProjectDoc, TreeContext) {
    let mut document = ProjectDoc::empty("matrix-transform-mounted", "Matrix transform fixture");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/matrix/r0c0".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions = vec![
        definition("key-definition", "Key"),
        definition("assembly-current/definition/led", "Current LED snapshot"),
        definition("assembly-other/definition/led", "Unrelated LED snapshot"),
        definition("replacement-definition", "Replacement LED"),
    ];
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: "matrix/matrix/r0c0".into(),
        definition_id: "key-definition".into(),
        reference: "K1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.matrices.push(Matrix {
        id: "matrix".into(),
        name: Some("Keys".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "key-definition".into(),
        part_ids: vec!["matrix/matrix/r0c0".into()],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![MatrixCell {
            deleted: false,
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("key-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "led".into(),
                definition_id: "assembly-current/definition/led".into(),
                offset: Vec2::default(),
                rotation: None,
                side: None,
            }],
            assemblies_local: None,
        }],
    });
    let mut led = document.parts[0].clone();
    led.id = "matrix/matrix/r0c0/led".into();
    led.definition_id = "assembly-current/definition/led".into();
    document.boards[0].part_ids.push(led.id.clone());
    document.matrices[0].part_ids.push(led.id.clone());
    document.parts.push(led);
    (
        document,
        TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
        },
    )
}

async fn mount() -> (Probe, web_sys::Element) {
    let (mut doc, _) = fixture();
    let mut other = doc.matrices[0].clone();
    other.id = "other".into();
    other.part_ids = vec!["matrix/other/r0c0".into()];
    other.origin.x = 60.0;
    let mut other_part = doc.parts[0].clone();
    other_part.id = "matrix/other/r0c0".into();
    other_part.pose.at.x = 60.0;
    doc.parts.push(other_part);
    doc.boards[0].part_ids.push("matrix/other/r0c0".into());
    doc.matrices.push(other);
    let runtime = support::new_runtime();
    support::open_document(&runtime, doc).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["matrix/matrix/r0c0".into(), "matrix/other/r0c0".into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().selected_part_ids.len(),
        2,
        "fixture selects two real primary keys"
    );
    let probe = Probe {
        runtime,
        version: Rc::default(),
        adapter: Rc::default(),
        workspace: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(40).await;
    (probe, root)
}

#[wasm_bindgen_test]
async fn cross_matrix_selection_has_atomic_enable_disable_and_delete() {
    let (probe, root) = mount().await;
    let disable = root
        .query_selector("button[aria-label='Disable selected keys']")
        .unwrap()
        .expect("selection across matrices exposes Disable selected keys");
    disable.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    support::run_pending(&probe.runtime).await;
    probe.version.borrow().unwrap().set(1);
    gloo_timers::future::TimeoutFuture::new(40).await;
    let model = probe.runtime.model();
    assert!(
        model
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|m| !m.cells[0].enabled)
    );
    let enable = root
        .query_selector("button[aria-label='Enable selected keys']")
        .unwrap()
        .unwrap();
    enable.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    support::run_pending(&probe.runtime).await;
    probe.version.borrow().unwrap().set(2);
    gloo_timers::future::TimeoutFuture::new(40).await;
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|m| m.cells[0].enabled)
    );
    let delete = root
        .query_selector("button[aria-label='Delete selected keys']")
        .unwrap()
        .unwrap();
    delete.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    support::run_pending(&probe.runtime).await;
    probe.version.borrow().unwrap().set(3);
    gloo_timers::future::TimeoutFuture::new(40).await;
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|m| m.cells[0].deleted)
    );
    probe.runtime.submit(Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    support::run_pending(&probe.runtime).await;
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|m| !m.cells[0].deleted && m.cells[0].enabled),
        "one undo restores both matrices"
    );
    root.remove();
}

fn button(root: &web_sys::Element, label: &str) -> web_sys::HtmlElement {
    root.query_selector(&format!("button[aria-label='{label}']"))
        .unwrap()
        .expect(label)
        .dyn_into()
        .unwrap()
}

async fn refresh(probe: &Probe) {
    let mut version = probe.version.borrow().unwrap();
    version += 1;
    gloo_timers::future::TimeoutFuture::new(40).await;
}

#[wasm_bindgen_test]
async fn rapid_cross_matrix_disable_then_enable_keeps_fields_available() {
    let (probe, root) = mount().await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    button(&root, "Disable selected keys").click();
    support::drive_pending(&probe.runtime);
    entered.await.unwrap();
    refresh(&probe).await;
    let enable = button(&root, "Enable selected keys");
    assert!(
        !enable.has_attribute("disabled"),
        "enable stays editable while disable is pending"
    );
    enable.click();
    release.send(()).unwrap();
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|matrix| matrix.cells[0].enabled)
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn queued_cross_matrix_enable_composes_with_rename_and_outlives_selection() {
    let (probe, root) = mount().await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    let operation_id = probe.runtime.operation();
    probe.runtime.submit(Event::ResolveEdit {
        operation_id,
        label: "rename-before-disable".into(),
        resolver: EditResolver::new("rename-before-disable", |accepted| {
            let mut document = accepted.document.as_ref().clone();
            document
                .parts
                .iter_mut()
                .find(|part| part.id == "matrix/matrix/r0c0")
                .unwrap()
                .reference = "Renamed key".into();
            Resolution::submit(
                vec!["matrix/matrix/r0c0".into()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        }),
    });
    support::drive_pending(&probe.runtime);
    entered.await.unwrap();
    button(&root, "Disable selected keys").click();
    let mut selected_context = probe.adapter.borrow().as_ref().unwrap().selected_context;
    selected_context.set(None);
    refresh(&probe).await;
    release.send(()).unwrap();
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|matrix| !matrix.cells[0].enabled),
        "committed valid targets still change after selection departure and label edits"
    );
    assert!(
        root.query_selector("[role='alert']").unwrap().is_none(),
        "owner departure retires observation silently"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mixed_primary_and_companion_selection_never_filters_into_bulk_keys() {
    let (probe, root) = mount().await;
    let model = probe.runtime.model();
    let companion = model
        .accepted
        .as_ref()
        .unwrap()
        .document
        .parts
        .iter()
        .find(|part| part.definition_id == "assembly-current/definition/led")
        .unwrap()
        .id
        .clone();
    probe.runtime.submit(Event::SelectParts {
        operation_id: probe.runtime.operation(),
        part_ids: vec![
            "matrix/matrix/r0c0".into(),
            "matrix/other/r0c0".into(),
            companion,
        ],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    assert!(
        root.query_selector("button[aria-label='Delete selected keys']")
            .unwrap()
            .is_none()
    );
    root.remove();
}

fn editor_host() -> Element {
    let probe = use_context::<Probe>();
    use_context_provider(|| probe.runtime.clone());
    let workspace = use_signal(|| "Layout");
    if probe.workspace.borrow().is_none() {
        *probe.workspace.borrow_mut() = Some(workspace);
    }
    use_context_provider(|| WorkspaceState(workspace));
    use_context_provider(|| ExportReturnWorkspace(workspace));
    let version = use_signal(|| 0u64);
    if probe.version.borrow().is_none() {
        *probe.version.borrow_mut() = Some(version);
    }
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
    let selected = use_signal(|| {
        Some(ScopedTreeContext {
            scope: probe.runtime.scope().unwrap(),
            context: TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 0,
                column: 0,
            },
        })
    });
    let anchor = use_signal(|| None);
    let generation = use_signal(|| 0u64);
    let adapter = SelectionAdapter::new(selected, anchor, generation);
    if probe.adapter.borrow().is_none() {
        *probe.adapter.borrow_mut() = Some(adapter.clone());
    }
    use_context_provider(|| adapter);
    use_hook({
        let runtime = probe.runtime.clone();
        let weak = Rc::downgrade(&runtime);
        move || {
            runtime.subscribe(Rc::new(move || {
                if let Some(runtime) = weak.upgrade() {
                    let model = runtime.model();
                    if selected.peek().as_ref().is_some_and(|context| {
                        !crate::presentation::selection::context_is_current(
                            &model,
                            &context.scope,
                            &context.context,
                        )
                    }) {
                        let mut selected = selected;
                        selected.set(None);
                    }
                    let mut version = version;
                    version += 1;
                }
            }))
        }
    });
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

#[wasm_bindgen_test]
async fn editor_cross_matrix_disable_reenable_survives_context_reconciliation() {
    let (mut doc, _) = fixture();
    let mut other = doc.matrices[0].clone();
    other.id = "other".into();
    other.part_ids = vec!["matrix/other/r0c0".into()];
    other.origin.x = 60.0;
    let mut other_part = doc.parts[0].clone();
    other_part.id = "matrix/other/r0c0".into();
    other_part.pose.at.x = 60.0;
    doc.parts.push(other_part);
    doc.boards[0].part_ids.push("matrix/other/r0c0".into());
    doc.matrices.push(other);
    let runtime = support::new_runtime();
    support::open_document(&runtime, doc).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["matrix/matrix/r0c0".into(), "matrix/other/r0c0".into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    let probe = Probe {
        runtime: runtime.clone(),
        version: Rc::default(),
        adapter: Rc::default(),
        workspace: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(editor_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    gloo_timers::future::TimeoutFuture::new(80).await;
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let scope = runtime.scope().unwrap();
    for (matrix_id, mode) in [
        ("matrix", SelectionMode::Replace),
        ("other", SelectionMode::Add),
    ] {
        crate::presentation::selection::submit_context(
            &runtime,
            &adapter,
            crate::presentation::selection::ContextRequest {
                scope: scope.clone(),
                context: TreeContext::Key {
                    matrix_id: matrix_id.into(),
                    row: 0,
                    column: 0,
                },
                mode,
            },
        );
    }
    support::run_pending(&runtime).await;
    refresh(&probe).await;
    assert_eq!(
        runtime.model().selected_part_ids.len(),
        2,
        "the real additive route selects keys in both matrices"
    );
    button(&root, "Disable selected keys").click();
    support::run_pending(&runtime).await;
    refresh(&probe).await;
    assert!(
        runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|matrix| !matrix.cells[0].enabled)
    );
    button(&root, "Enable selected keys").click();
    support::run_pending(&runtime).await;
    refresh(&probe).await;
    assert!(
        runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .matrices
            .iter()
            .all(|matrix| matrix.cells[0].enabled)
    );
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn disabled_cross_matrix_targets_end_at_explicit_same_context_reselection() {
    let (probe, root) = mount().await;
    button(&root, "Disable selected keys").click();
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    assert!(
        root.query_selector("button[aria-label='Enable selected keys']")
            .unwrap()
            .is_some()
    );
    let adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let context = adapter.selected_context.peek().as_ref().unwrap().clone();
    crate::presentation::selection::submit_context(
        &probe.runtime,
        &adapter,
        crate::presentation::selection::ContextRequest {
            scope: context.scope,
            context: context.context,
            mode: SelectionMode::Replace,
        },
    );
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    assert!(
        root.query_selector("button[aria-label='Enable selected keys']")
            .unwrap()
            .is_none(),
        "an explicit same-context ghost selection cannot revive the previous cross-matrix group"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn disabled_cross_matrix_targets_end_at_workspace_departure() {
    let (probe, root) = mount().await;
    button(&root, "Disable selected keys").click();
    support::run_pending(&probe.runtime).await;
    refresh(&probe).await;
    let mut workspace = probe.workspace.borrow().unwrap();
    workspace.set("Parts");
    refresh(&probe).await;
    workspace.set("Layout");
    refresh(&probe).await;
    assert!(
        root.query_selector("button[aria-label='Enable selected keys']")
            .unwrap()
            .is_none(),
        "returning to Layout cannot revive the previous disabled selection"
    );
    root.remove();
}
