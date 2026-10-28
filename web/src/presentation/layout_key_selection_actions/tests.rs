use super::*;
use crate::presentation::*;
use crate::presentation::{
    SelectionAdapter, WorkspaceState,
    objects::{ScopedTreeContext, TreeContext},
};
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::*;
use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_step};
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
    rsx! {
        SelectedKeysActions {}
        output { class: "selected-keys-test-render", "{version()}" }
    }
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
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
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

fn mirrored_companion_fixture() -> ProjectDoc {
    let (mut document, _) = fixture();
    document.definitions[0].kind = PartKind::Switch;
    document.matrices[0].columns = 2;
    let mut second_cell = document.matrices[0].cells[0].clone();
    second_cell.column = 1;
    document.matrices[0].cells.push(second_cell);
    let mut second_key = document.parts[0].clone();
    second_key.id = "matrix/matrix/r0c1".into();
    second_key.reference = "K2".into();
    second_key.pose.at.x = 19.0;
    let mut second_led = document.parts[1].clone();
    second_led.id = "matrix/matrix/r0c1/led".into();
    second_led.pose.at.x = 19.0;
    document.matrices[0]
        .part_ids
        .extend([second_key.id.clone(), second_led.id.clone()]);
    document
        .parts
        .extend([second_key.clone(), second_led.clone()]);
    document.boards.push(Board {
        id: "mirror-board".into(),
        name: "Mirror board".into(),
        outline_ids: vec![],
        part_ids: vec![
            "matrix/other/r0c0".into(),
            "matrix/other/r0c0/led".into(),
            "matrix/other/r0c1".into(),
            "matrix/other/r0c1/led".into(),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let source_matrix = document.matrices[0].clone();
    let mut mirror_matrix = source_matrix.clone();
    mirror_matrix.id = "other".into();
    mirror_matrix.board_id = Some("mirror-board".into());
    mirror_matrix.origin.x = 50.0;
    mirror_matrix.part_ids = vec![
        "matrix/other/r0c0".into(),
        "matrix/other/r0c0/led".into(),
        "matrix/other/r0c1".into(),
        "matrix/other/r0c1/led".into(),
    ];
    document.matrices.push(mirror_matrix);
    let mut mirror_key = document.parts[0].clone();
    mirror_key.id = "matrix/other/r0c0".into();
    mirror_key.pose.at.x = 50.0;
    document.parts.push(mirror_key.clone());
    let mut mirror_led = document.parts[1].clone();
    mirror_led.id = "matrix/other/r0c0/led".into();
    mirror_led.pose.at.x = 50.0;
    let mut mirror_second_key = second_key;
    mirror_second_key.id = "matrix/other/r0c1".into();
    mirror_second_key.pose.at.x = 69.0;
    let mut mirror_second_led = second_led;
    mirror_second_led.id = "matrix/other/r0c1/led".into();
    mirror_second_led.pose.at.x = 69.0;
    document
        .parts
        .extend([mirror_led, mirror_second_key, mirror_second_led]);
    document.boards[0]
        .part_ids
        .push("matrix/matrix/r0c0/led".into());
    document.boards[0]
        .part_ids
        .extend(["matrix/matrix/r0c1".into(), "matrix/matrix/r0c1/led".into()]);
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology":"split", "transport":"wireless",
            "instances":[
                {"id":"source-instance","name":"Source","boardId":"board","half":"left","role":"primary","flipped":false,"controllerPartId":null,"mechanical":null},
                {"id":"mirror-instance","name":"Mirror","boardId":"mirror-board","half":"right","role":"secondary","flipped":true,"controllerPartId":null,"mechanical":null}
            ],
            "pairs":[{
                "id":"pair/A", "members":["source-instance","mirror-instance"],
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "construction":{"method":"printed","mount":"tray","integratedPlateFrame":false,"plateThickness":1.5,"plateFoamThickness":0.0,"bottomFoamThickness":0.0,"bottomThickness":1.5,"plateToPcb":5.0,"wallThickness":2.0,"clearance":0.2},
                "constructionSource":"source-instance",
                "halves":{"sourceBoardId":"board","mirrorBoardId":"mirror-board","correspondences":[
                    {"kind":"matrix","sourceId":"matrix","mirrorId":"other"},
                    {"kind":"part","sourceId":"matrix/matrix/r0c0","mirrorId":"matrix/other/r0c0"},
                    {"kind":"part","sourceId":"matrix/matrix/r0c0/led","mirrorId":"matrix/other/r0c0/led"},
                    {"kind":"part","sourceId":"matrix/matrix/r0c1","mirrorId":"matrix/other/r0c1"},
                    {"kind":"part","sourceId":"matrix/matrix/r0c1/led","mirrorId":"matrix/other/r0c1/led"}
                ]}
            }]
        }))
        .expect("synthetic mirrored switch-companion pair parses"),
    );
    document
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
    wait_until("the selected key actions mount", || {
        root.query_selector("button[aria-label='Disable selected keys']")
            .is_ok_and(|button| button.is_some())
    })
    .await;
    (probe, root)
}

fn button(root: &web_sys::Element, label: &str) -> web_sys::HtmlElement {
    root.query_selector(&format!("button[aria-label='{label}']"))
        .unwrap()
        .expect(label)
        .dyn_into()
        .unwrap()
}

async fn refresh(probe: &Probe, root: &web_sys::Element) {
    let mut version = probe.version.borrow().unwrap();
    let expected = *version.peek() + 1;
    version += 1;
    wait_until("the selected-keys host renders its latest version", || {
        root.query_selector(".selected-keys-test-render")
            .unwrap()
            .and_then(|label| label.text_content())
            .and_then(|text| text.parse::<u64>().ok())
            .is_some_and(|rendered| rendered >= expected)
    })
    .await;
}

#[wasm_bindgen_test]
async fn rapid_cross_matrix_disable_then_enable_keeps_fields_available() {
    let (probe, root) = mount().await;
    let (entered, release) = support::gate_next_core_reply(&probe.runtime);
    button(&root, "Disable selected keys").click();
    support::drive_pending(&probe.runtime);
    entered.await.unwrap();
    refresh(&probe, &root).await;
    let enable = button(&root, "Enable selected keys");
    assert!(
        !enable.has_attribute("disabled"),
        "enable stays editable while disable is pending"
    );
    enable.click();
    release.send(()).unwrap();
    support::run_pending(&probe.runtime).await;
    refresh(&probe, &root).await;
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
    refresh(&probe, &root).await;
    release.send(()).unwrap();
    support::run_pending(&probe.runtime).await;
    refresh(&probe, &root).await;
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
    refresh(&probe, &root).await;
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
    rsx! {
        Editor {}
        output { class: "editor-selected-context-test-render", "{selected:?}" }
        output { class: "selected-keys-test-render", "{version()}" }
    }
}

async fn apply_and_undo_editor_companion_mode(
    root: &web_sys::Element,
    runtime: &Rc<Runtime>,
    selection: &str,
    source_ids: &[String],
    undo: bool,
) {
    use boardstudio_core::model::MirroredCompanionMode;

    let details = root
        .query_selector("details.m1-layout-companion-placement")
        .unwrap()
        .expect("Editor renders companion placement for this selection");
    details.set_attribute("open", "").unwrap();
    let mode = root
        .query_selector("details.m1-layout-companion-placement select")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap();
    mode.set_value("duplicate-position");
    let change = web_sys::EventInit::new();
    change.set_bubbles(true);
    mode.dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
        .unwrap();
    wait_until("Editor accepts the selected companion mode", || {
        root.query_selector("details.m1-layout-companion-placement button")
            .unwrap()
            .is_some_and(|button| !button.has_attribute("disabled"))
    })
    .await;
    let apply = root
        .query_selector("details.m1-layout-companion-placement button")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let accepted = runtime.model().accepted.unwrap();
    let before = accepted.document.revision;
    let original_modes = accepted.document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .iter()
        .filter(|link| link.source_id.ends_with("/led"))
        .map(|link| (link.source_id.clone(), link.companion_mode))
        .collect::<Vec<_>>();
    apply.click();
    wait_until_step(
        &format!("Core applies the Editor-owned {selection} companion selection"),
        async || {
            support::run_pending(runtime).await;
            let accepted = runtime.model().accepted.unwrap();
            let links = &accepted.document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences;
            let landed = accepted.document.revision > before;
            let error = root
                .query_selector("details.m1-layout-companion-placement [role=alert]")
                .unwrap()
                .and_then(|alert| alert.text_content());
            let unchanged = links
                .iter()
                .filter(|link| link.source_id.ends_with("/led"))
                .map(|link| {
                    let expected = if source_ids.contains(&link.source_id) {
                        Some(MirroredCompanionMode::DuplicatePosition)
                    } else {
                        original_modes
                            .iter()
                            .find(|(id, _)| id == &link.source_id)
                            .and_then(|(_, mode)| *mode)
                    };
                    (link.source_id.clone(), link.companion_mode, expected)
                })
                .all(|(_, actual, expected)| actual == expected);
            landed && unchanged || error.is_some()
        },
    )
    .await;
    assert!(
        runtime.model().accepted.unwrap().document.revision > before,
        "companion placement refused: {:?}",
        root.query_selector("details.m1-layout-companion-placement [role=alert]")
            .unwrap()
            .and_then(|alert| alert.text_content())
    );
    if !undo {
        return;
    }
    let accepted_revision = runtime.model().accepted.unwrap().document.revision;
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step(
        "Undo restores the Editor-owned companion selection",
        async || {
            support::run_pending(runtime).await;
            let accepted = runtime.model().accepted.unwrap();
            accepted.document.revision > accepted_revision
                && accepted.document.hardware.as_ref().unwrap().pairs[0]
                    .halves
                    .as_ref()
                    .unwrap()
                    .correspondences
                    .iter()
                    .filter(|link| link.source_id.ends_with("/led"))
                    .all(|link| {
                        original_modes
                            .iter()
                            .find(|(id, _)| id == &link.source_id)
                            .is_some_and(|(_, mode)| link.companion_mode == *mode)
                    })
        },
    )
    .await;
}

#[wasm_bindgen_test]
async fn editor_owned_selection_kinds_apply_only_eligible_companions_and_undo() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, mirrored_companion_fixture()).await;
    assert!(
        runtime.model().accepted.is_some(),
        "mirrored companion fixture opens: {:?}",
        runtime.model().last_error
    );
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
    wait_until("the real Editor mounts", || {
        probe.adapter.borrow().is_some()
    })
    .await;
    let mut editor_adapter = probe.adapter.borrow().as_ref().unwrap().clone();
    let model = runtime.model();
    let active_board = model.active_board_id.clone();
    let matrix_id = model
        .accepted
        .as_ref()
        .unwrap()
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.board_id.as_deref() == Some(active_board.as_str()))
        .unwrap()
        .id
        .clone();
    let key0 = format!("matrix/{matrix_id}/r0c0");
    let key1 = format!("matrix/{matrix_id}/r0c1");
    let led0 = format!("{key0}/led");
    let led1 = format!("{key1}/led");
    let scope = runtime.scope().unwrap();
    let selection = ScopedTreeContext {
        scope: scope.clone(),
        context: TreeContext::Key {
            matrix_id: matrix_id.clone(),
            row: 0,
            column: 0,
        },
    };
    assert!(selection::context_is_current(
        &runtime.model(),
        &selection.scope,
        &selection.context
    ));
    editor_adapter.selected_context.set(Some(selection.clone()));
    wait_until("the real Editor retains the selected context", || {
        editor_adapter.selected_context.read().as_ref() == Some(&selection)
    })
    .await;
    let expected_context = format!("{:?}", selection);
    wait_until("the Editor renders the selected context", || {
        root.query_selector(".editor-selected-context-test-render")
            .unwrap()
            .is_some_and(|node| {
                node.text_content()
                    .is_some_and(|text| text.contains(&expected_context))
            })
    })
    .await;
    wait_until("the real Editor renders companion placement", || {
        root.query_selector("details.m1-layout-companion-placement")
            .unwrap()
            .is_some()
    })
    .await;
    let projection = companion_placement_projection(&runtime.model(), Some(&selection), 0, 0)
        .expect("Editor selection projects eligible companions");
    assert!(!projection.targets.is_empty());
    apply_and_undo_editor_companion_mode(
        &root,
        &runtime,
        "single key",
        &["matrix/matrix/r0c0/led".into()],
        false,
    )
    .await;

    let selected_parts = vec![led0, led1];
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_parts.clone(),
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    wait_until("the Editor retains both selected companions", || {
        runtime.model().selected_part_ids == selected_parts
    })
    .await;
    let multi_context = ScopedTreeContext {
        scope,
        context: objects::context_for_part(&runtime.model(), &selected_parts[0])
            .expect("selected companion has an Editor component context"),
    };
    editor_adapter
        .selected_context
        .set(Some(multi_context.clone()));
    wait_until("the full Editor shows mixed companion modes", || {
        root.query_selector("details.m1-layout-companion-placement summary small")
            .unwrap()
            .is_some_and(|label| label.text_content().as_deref() == Some("Mixed"))
    })
    .await;
    let multi_projection =
        companion_placement_projection(&runtime.model(), Some(&multi_context), 0, 0)
            .expect("multi-selection projects linked companions");
    assert_eq!(multi_projection.targets.len(), 2);
    apply_and_undo_editor_companion_mode(
        &root,
        &runtime,
        "multiple selected parts",
        &[
            "matrix/matrix/r0c0/led".into(),
            "matrix/matrix/r0c1/led".into(),
        ],
        true,
    )
    .await;
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step("the original companion mode is restored", async || {
        support::run_pending(&runtime).await;
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .hardware
            .as_ref()
            .unwrap()
            .pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .correspondences
            .iter()
            .filter(|link| link.source_id.ends_with("/led"))
            .all(|link| link.companion_mode.is_none())
    })
    .await;
    root.remove();
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
    wait_until("the mounted Editor selection adapter is available", || {
        probe.adapter.borrow().is_some()
    })
    .await;
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
    refresh(&probe, &root).await;
    assert_eq!(
        runtime.model().selected_part_ids.len(),
        2,
        "the real additive route selects keys in both matrices"
    );
    button(&root, "Disable selected keys").click();
    support::run_pending(&runtime).await;
    refresh(&probe, &root).await;
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
    refresh(&probe, &root).await;
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
    refresh(&probe, &root).await;
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
    refresh(&probe, &root).await;
    assert!(
        root.query_selector("button[aria-label='Enable selected keys']")
            .unwrap()
            .is_none(),
        "an explicit same-context ghost selection cannot revive the previous cross-matrix group"
    );
    root.remove();

    let (probe, root) = mount().await;
    button(&root, "Disable selected keys").click();
    support::run_pending(&probe.runtime).await;
    refresh(&probe, &root).await;
    let mut workspace = probe.workspace.borrow().unwrap();
    workspace.set("Parts");
    refresh(&probe, &root).await;
    workspace.set("Layout");
    refresh(&probe, &root).await;
    assert!(
        root.query_selector("button[aria-label='Enable selected keys']")
            .unwrap()
            .is_none(),
        "returning to Layout cannot revive the previous disabled selection"
    );
    root.remove();
}
