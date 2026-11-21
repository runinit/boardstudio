use super::*;
use boardstudio_application::AcceptedSnapshot;
use boardstudio_core::model::{
    Board, Finding, MechanicalAssembly, Scope as FindingScope, Severity,
};
use boardstudio_web_test_runtime::in_process_support;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::{
    test_mount::MountedFixture, test_role::by_role_name, test_wait::wait_until,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

async fn accepted_fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
    let runtime = support::new_runtime();
    let mut document = in_process_support::board_document();
    document.mechanical = Some(
        boardstudio_web_host::case_settings::initial_settings(&document, "main-board")
            .expect("test board has initial mechanical settings"),
    );
    support::open_document(&runtime, document).await;
    let accepted = runtime
        .model()
        .accepted
        .expect("the Case workspace project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted Case workspace project has a scope");
    let mechanical = serde_json::from_value::<MechanicalAssembly>(serde_json::json!({
        "suggestedMounts": [],
        "nominalPlateContours": [],
        "revision": accepted.document.revision,
        "plateContours": [],
        "case": {
            "revision": accepted.document.revision,
            "bodies": [{
                "revision": accepted.document.revision,
                "body": {
                    "id": "plate",
                    "name": "Plate body",
                    "boardId": scope.board_id,
                    "kind": "plate",
                    "thickness": 1.5,
                    "clearance": 0.2
                },
                "contours": []
            }]
        },
        "stack": [{ "id": "plate", "z": 0.0, "thickness": 1.5 }],
        "diagnostics": [{
            "id": "stale-case-finding",
            "severity": "error",
            "scope": "case",
            "message": "Stale diagnostic from previous geometry",
            "targetIds": ["plate"]
        }]
    }))
    .expect("minimal completed mechanical assembly fixture");
    runtime.set_cad_scene_test(Some(Rc::new(CadScene {
        scope: scope.clone(),
        token: accepted.token,
        snapshot: accepted.clone(),
        result: boardstudio_web_host::cad_jobs::CadResult {
            revision: accepted.document.revision,
            ..Default::default()
        },
        prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
            revision: accepted.document.revision,
            bodies: Vec::new(),
        },
        // Keep this cached scene stale after the accepted-token transition so the
        // display projection can retain it while the strict-current projection drops it.
        physical_fingerprint: Some([0xA5; 32]),
        mechanical: Some(mechanical),
        exact: true,
        contours: Vec::new(),
    })));
    (runtime, accepted, scope)
}

fn scene_with_current_finding(
    base: &Rc<CadScene>,
    accepted: AcceptedSnapshot,
    id: &str,
    message: &str,
) -> Rc<CadScene> {
    let mut scene = CadScene {
        scope: base.scope.clone(),
        token: base.token,
        snapshot: base.snapshot.clone(),
        result: base.result.clone(),
        prepared: base.prepared.clone(),
        physical_fingerprint: base.physical_fingerprint,
        mechanical: base.mechanical.clone(),
        exact: base.exact,
        contours: base.contours.clone(),
    };
    scene.token = accepted.token;
    scene.snapshot = accepted.clone();
    scene.result.revision = accepted.document.revision;
    scene.prepared.revision = accepted.document.revision;
    let assembly = scene
        .mechanical
        .as_mut()
        .expect("test scene has a mechanical assembly");
    assembly.diagnostics.clear();
    assembly.diagnostics.push(Finding {
        id: id.into(),
        severity: Severity::Error,
        scope: FindingScope::Case,
        message: message.into(),
        target_ids: vec!["plate".into()],
    });
    Rc::new(scene)
}

fn host() -> Element {
    let mut render_generation = use_signal(|| 0u64);
    let _ = render_generation();
    use_context_provider(|| render_generation);
    super::super::use_empty_test_instance_selection();
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<InstanceSelection>();
    let model = runtime.model();
    let scope = runtime.scope();
    let state = use_case_workspace_state();
    let case_selection = state.selection();
    let initial_scope = scope.clone();
    let selection_for_hook = case_selection;
    use_hook({
        move || {
            if let Some(scope) = initial_scope {
                selection_for_hook.select_layer(scope, "plate".into());
            }
        }
    });
    let selected_context = use_signal(|| None::<ScopedTreeContext>);
    let expanded = state.tree_expanded();
    let display_scene = scope
        .as_ref()
        .and_then(|scope| workspace_display_scene(runtime.cad_scene(), scope));
    let selected_layer_id = scope
        .as_ref()
        .map_or_else(String::new, |scope| case_selection.layer_id(scope));
    let tree = if model.accepted.is_some() {
        objects(ObjectsInput {
            model: &model,
            scope: scope.clone(),
            instance_scope_pending: false,
            scene: display_scene,
            selected_context,
            selected_body_id: None,
            selected_layer_id,
            case_selection,
            expanded,
            on_action: EventHandler::new(|_| {}),
            on_select: EventHandler::new(|_| {}),
            on_display: EventHandler::new(|_| {}),
        })
    } else {
        rsx! {}
    };
    let workspace = use_signal(|| "Case");
    let generation = use_signal(|| 1u64);
    let mut shown_finding = use_signal(String::new);
    let mechanical = super::super::mechanical_settings_mount::use_mechanical_settings_mount(
        runtime,
        generation,
        workspace,
        instance_selection,
        case_selection,
        EventHandler::new(
            move |request: super::super::mechanical_settings_mount::MechanicalFindingNavigation| {
                shown_finding.set(request.finding_id)
            },
        ),
    );
    let inspector = mechanical.props.map(super::super::MechanicalSettings);
    rsx! {
        button {
            id: "case11-accepted-transition",
            onclick: move |_| render_generation += 1,
            "Advance accepted owner"
        }
        p { id: "case11-shown-finding", "{shown_finding}" }
        div { id: "case11-workspace-evidence", {tree} {inspector} }
    }
}

fn navigation_host() -> Element {
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    super::super::use_empty_test_instance_selection();
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<InstanceSelection>();
    let state = use_case_workspace_state();
    let mut workspace = use_signal(|| "Layout");
    let generation = use_signal(|| 1u64);
    let mechanical = super::super::mechanical_settings_mount::use_mechanical_settings_mount(
        runtime.clone(),
        generation,
        workspace,
        instance_selection,
        state.selection(),
        EventHandler::new(|_| {}),
    );
    let physical_setup = super::super::pcb_physical_setup::use_controller(
        runtime.clone(),
        version,
        generation,
        Rc::new(|| false),
        instance_selection,
        Rc::new(|_, _| true),
        Rc::new(|_, _| Box::pin(async { panic!("navigation must not prepare physical edits") })),
    );
    let content = if workspace() == "Case" {
        inspector(InspectorInput {
            physical_setup,
            mechanical_settings: mechanical,
            instance_scope_pending: false,
            scope: runtime.scope(),
            scene: None,
            case_selection: state.selection(),
            selected_layer_id: String::new(),
            selected_context: None,
            selected_part_summary: None,
            on_show_configured_board: EventHandler::new(|_| {}),
            on_display: EventHandler::new(|_| {}),
        })
    } else {
        rsx! { p { "Layout workspace" } }
    };
    // A page-owned hook after dispatch catches leaked child hooks on either transition.
    let marker = use_signal(|| "page owner retained");
    rsx! {
        button { id: "case-navigation-enter", onclick: move |_| workspace.set("Case"), "Case" }
        button { id: "case-navigation-leave", onclick: move |_| workspace.set("Layout"), "Layout" }
        p { id: "case-navigation-owner", "{marker}" }
        div { id: "case-navigation-content", {content} }
    }
}

#[wasm_bindgen_test]
async fn case_inspector_navigation_keeps_hooks_in_the_mounted_child() {
    let (runtime, _, _) = accepted_fixture().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-navigation-root");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(navigation_host);
    dom.provide_root_context(runtime);
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    for _ in 0..2 {
        root.query_selector("#case-navigation-enter")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(
            root.query_selector("#case-navigation-content [aria-label='Case Inspector']")
                .unwrap()
                .is_some()
        );
        assert!(
            root.query_selector(
                "#case-navigation-content [aria-label='Mechanical stack settings']"
            )
            .unwrap()
            .is_some(),
            "entering Case mounts its real mechanical Inspector"
        );
        root.query_selector("#case-navigation-leave")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(
            root.query_selector("#case-navigation-content")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Layout workspace"),
            "leaving Case restores the previous workspace"
        );
        assert_eq!(
            root.query_selector("#case-navigation-owner")
                .unwrap()
                .unwrap()
                .text_content()
                .unwrap(),
            "page owner retained"
        );
    }
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(10).await;
}

async fn wait_for_current_resolution_inspector_text(root: &web_sys::Element) -> String {
    let current_resolution_finding =
        "Case body 'mechanical-frame-outline' requires at least one outer contour";
    let mut last_text = String::new();
    for _ in 0..50 {
        if let Some(inspector) = root
            .query_selector("[aria-label='Mechanical stack settings']")
            .unwrap()
        {
            last_text = inspector.text_content().unwrap_or_default();
            if last_text.contains(current_resolution_finding) {
                return last_text;
            }
        }
        gloo_timers::future::TimeoutFuture::new(5).await;
    }
    panic!("current mechanical resolution did not reach the mounted Inspector: {last_text:?}");
}

#[wasm_bindgen_test]
// The Objects tree retains and labels the stale same-scope scene. The Inspector
// independently prefers an exact current mechanical resolution when it settles.
async fn production_workspace_keeps_same_scope_layer_context_then_retires_it_on_owner_change() {
    let (runtime, original, scope) = accepted_fixture().await;
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("case11-workspace-evidence-root");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;

    let mut edited = original.document.as_ref().clone();
    edited.name.push_str(" updated");
    let updated = support::replace_document(
        &runtime,
        "case-workspace-accepted-edit",
        &scope.board_id,
        edited,
    )
    .await;
    root.query_selector("#case11-accepted-transition")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    // The opened project already resolved its settings once, so the Inspector text is
    // present before the accepted-owner render lands; let that render finish first.
    settle().await;

    let text = wait_for_current_resolution_inspector_text(&root).await;
    let root_text = root.text_content().unwrap_or_default();
    let plate = root
        .query_selector("[id='m1-object-tree-case-generated:plate']")
        .unwrap()
        .unwrap_or_else(|| {
            panic!("same-scope layer missing from production Objects tree: {root_text}")
        });
    assert!(
        plate
            .text_content()
            .unwrap_or_default()
            .contains("Previous geometry"),
        "the same-scope plate row is labelled previous; plate={:?}; root={root_text:?}",
        plate.text_content()
    );
    let row = plate.closest("[role='treeitem']").unwrap().unwrap();
    assert_eq!(row.get_attribute("aria-selected").as_deref(), Some("true"));
    assert!(
        !text.contains("previous generated geometry"),
        "the current resolved Inspector must not mark its layer previous; inspector={text:?}; root={root_text:?}"
    );
    assert!(text.contains("Resolved thickness 1.50 mm."));
    assert!(text.contains("Plate thickness"));
    assert!(!text.contains("Stale diagnostic from previous geometry"));
    assert_eq!(
        root.query_selector("#case11-shown-finding")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("")
    );

    let (current_assembly, _) = runtime
        .resolve_mechanical_settings(
            updated.clone(),
            scope.clone(),
            updated.document.as_ref().clone(),
        )
        .await
        .expect("the accepted fixture has a current mechanical resolution");
    let current_finding = current_assembly
        .diagnostics
        .iter()
        .find(|finding| {
            finding.message.contains(
                "Case body 'mechanical-frame-outline' requires at least one outer contour",
            )
        })
        .expect("the current resolution reports the fixture's missing outline");
    let stale_scene = runtime
        .cad_scene()
        .expect("same-scope completed geometry remains available for display");
    runtime.set_cad_scene_test(Some(scene_with_current_finding(
        &stale_scene,
        updated,
        "current-case-finding",
        "Current diagnostic for accepted geometry",
    )));
    root.query_selector("#case11-accepted-transition")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle().await;
    let inspector = root
        .query_selector("[aria-label='Mechanical stack settings']")
        .unwrap()
        .unwrap();
    let text = inspector.text_content().unwrap_or_default();
    assert!(text.contains(&current_finding.message));
    assert!(!text.contains("Current diagnostic for accepted geometry"));
    assert!(!text.contains("Stale diagnostic from previous geometry"));
    by_role_name(
        &inspector,
        "button",
        &format!("Show finding {}", current_finding.id),
    )
    .expect("current finding has a Show action")
    .click();
    settle().await;
    assert_eq!(
        root.query_selector("#case11-shown-finding")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some(current_finding.id.as_str())
    );

    // The accepted owner moves to another board: a real edit adds it and its mechanical
    // settings, then the Session navigates there. Navigating retires the cached geometry, so
    // the old board's scene is put back to show the display projection still ignores it.
    let mut with_other_board = runtime
        .model()
        .accepted
        .expect("the Case workspace project stays accepted")
        .document
        .as_ref()
        .clone();
    with_other_board.boards.push(Board {
        id: "other-board".into(),
        name: "Other board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    with_other_board.mechanical = Some(
        boardstudio_web_host::case_settings::initial_settings(&with_other_board, "other-board")
            .expect("replacement board has initial mechanical settings"),
    );
    support::replace_document(
        &runtime,
        "case-workspace-other-board",
        &scope.board_id,
        with_other_board,
    )
    .await;
    let previous_board_scene = runtime.cad_scene();
    support::navigate(&runtime, "other-board").await;
    assert_eq!(
        runtime.scope().map(|scope| scope.board_id),
        Some("other-board".to_owned())
    );
    runtime.set_cad_scene_test(previous_board_scene);
    root.query_selector("#case11-accepted-transition")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    settle().await;
    assert!(
        root.query_selector("[id='m1-object-tree-case-generated:plate']")
            .unwrap()
            .is_none()
    );
    let inspector = root
        .query_selector("[aria-label='Mechanical stack settings']")
        .unwrap()
        .unwrap();
    let text = inspector.text_content().unwrap_or_default();
    assert!(!text.contains("previous generated geometry"));
    assert!(!text.contains("Previous resolved stack"));
}

fn key(element: &web_sys::HtmlElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    element
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

fn tree_host() -> Element {
    super::super::use_empty_test_instance_selection();
    let runtime = use_context::<Rc<Runtime>>();
    let model = runtime.model();
    let scope = runtime.scope();
    let state = use_case_workspace_state();
    let case_selection = state.selection();
    let selected_context = use_signal(|| None::<ScopedTreeContext>);
    let expanded = state.tree_expanded();
    let mut action_calls = use_signal(String::new);
    let mut display_calls = use_signal(String::new);
    let tree = objects(ObjectsInput {
        model: &model,
        scope: scope.clone(),
        instance_scope_pending: false,
        scene: None,
        selected_context,
        selected_body_id: None,
        selected_layer_id: String::new(),
        case_selection,
        expanded,
        on_action: Callback::new(move |action: TreeAction| {
            if matches!(action, TreeAction::SelectAssembly { .. }) {
                action_calls.set("select-assembly".into());
            }
        }),
        on_select: EventHandler::new(|_| {}),
        on_display: Callback::new(move |request: DisplayRequest| {
            if matches!(request.action, DisplayAction::ToggleVisibility) {
                display_calls.set("toggle-visibility".into());
            }
        }),
    });
    rsx! {
        {tree}
        p { id: "tree-action-calls", "{action_calls}" }
        p { id: "tree-display-calls", "{display_calls}" }
    }
}

#[wasm_bindgen_test]
async fn case_tree_uses_copied_controls_for_rows_and_visibility() {
    let (runtime, _, _) = accepted_fixture().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(tree_host);
    dom.provide_root_context(runtime);
    let _mounted = MountedFixture::new(root.clone(), dom);
    let label = "Main board case assembly";
    wait_until("the case tree mounts", || {
        by_role_name(&root, "button", &format!("Collapse {label}")).is_some()
    })
    .await;
    let disclosure =
        by_role_name(&root, "button", &format!("Collapse {label}")).expect("tree disclosure");
    assert_eq!(
        disclosure.get_attribute("data-slot").as_deref(),
        Some("button")
    );
    disclosure.click();
    wait_until("the tree collapses", || {
        by_role_name(&root, "button", &format!("Expand {label}")).is_some()
    })
    .await;
    by_role_name(&root, "button", &format!("Expand {label}"))
        .unwrap()
        .click();
    wait_until("the tree reopens", || {
        by_role_name(&root, "button", &format!("Collapse {label}")).is_some()
    })
    .await;
    let visibility =
        by_role_name(&root, "button", &format!("Hide {label}")).expect("visibility toggle");
    assert_eq!(
        visibility.get_attribute("data-slot").as_deref(),
        Some("tooltip-trigger")
    );
    visibility.click();
    let calls = || {
        root.query_selector("#tree-display-calls")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
    };
    wait_until("visibility toggles through its owner", || {
        calls() == "toggle-visibility"
    })
    .await;
    let select = by_role_name(&root, "button", &format!("\u{25b0}{label}")).expect("row select");
    assert_eq!(select.get_attribute("data-slot").as_deref(), Some("button"));
    select.click();
    wait_until("assembly selects through its owner", || {
        root.query_selector("#tree-action-calls")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
            == "select-assembly"
    })
    .await;
}

async fn split_hardware_fixture() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    let mut document =
        serde_json::to_value(ProjectDoc::empty("case-physical-controls", "Physical")).unwrap();
    document["boards"] = (0..13)
        .map(|index| {
            serde_json::json!({
                "id": format!("board-{index}"),
                "name": format!("Board {index}"),
                "outlineIds": [],
                "partIds": [],
                "netIds": [],
                "thickness": 1.6,
            })
        })
        .collect();
    document["hardware"] = serde_json::json!({"topology": "split", "transport": "wireless", "boards": [], "instances": [
        {"id": "left", "name": "Left", "boardId": "board-0", "half": "left", "role": "central", "flipped": false, "controllerPartId": null, "mechanical": null, "constructionLinked": false},
        {"id": "right", "name": "Right", "boardId": "board-0", "half": "right", "role": "peripheral", "flipped": true, "controllerPartId": null, "mechanical": null, "constructionLinked": false},
    ]});
    support::open_document(&runtime, serde_json::from_value(document).unwrap()).await;
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: "board-0".into(),
        instance_id: Some("left".into()),
    });
    support::run_pending(&runtime).await;
    runtime
}

fn physical_host() -> Element {
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    super::super::use_empty_test_instance_selection();
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<InstanceSelection>();
    let state = use_case_workspace_state();
    let generation = use_signal(|| 1u64);
    let physical_setup = super::super::pcb_physical_setup::use_controller(
        runtime.clone(),
        version,
        generation,
        Rc::new(|| false),
        instance_selection,
        Rc::new(|_, _| true),
        Rc::new(|_, _| Box::pin(async { panic!("physical controls test must not prepare edits") })),
    );
    rsx! {
        {inspector(InspectorInput {
            physical_setup,
            mechanical_settings: MechanicalSettingsMount {
                props: None,
                generation_ready: false,
            },
            instance_scope_pending: false,
            scope: runtime.scope(),
            scene: None,
            case_selection: state.selection(),
            selected_layer_id: String::new(),
            selected_context: None,
            selected_part_summary: None,
            on_show_configured_board: EventHandler::new(|_| {}),
            on_display: EventHandler::new(|_| {}),
        })}
    }
}

#[wasm_bindgen_test]
async fn physical_assembly_uses_select_combobox_checkbox_and_disclosure() {
    let runtime = split_hardware_fixture().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(physical_host);
    dom.provide_root_context(runtime);
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("assembly setup mounts", || {
        root.text_content()
            .unwrap_or_default()
            .contains("Assembly setup")
    })
    .await;
    let setup = by_role_name(&root, "button", "Assembly setup").expect("assembly setup trigger");
    assert_eq!(
        setup.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    setup.click();
    wait_until("assembly setup opens", || {
        setup.get_attribute("aria-expanded").as_deref() == Some("true")
    })
    .await;
    let half = by_role_name(&root, "button", "Half connection").expect("half connection");
    assert_eq!(
        half.get_attribute("data-slot").as_deref(),
        Some("select-trigger")
    );
    half.click();
    wait_until("half connection options open", || {
        by_role_name(&root, "listbox", "Half connection").is_some()
    })
    .await;
    assert!(
        by_role_name(
            &root,
            "option",
            "Wireless \u{b7} local battery on each half"
        )
        .is_some()
    );
    key(
        &by_role_name(&root, "listbox", "Half connection").unwrap(),
        "Escape",
    );
    wait_until("half connection options close", || {
        by_role_name(&root, "listbox", "Half connection").is_none()
    })
    .await;
    let design = by_role_name(&root, "combobox", "PCB design").expect("thirteen-board picker");
    assert_eq!(design.tag_name(), "INPUT");
    design.focus().unwrap();
    key(&design, "ArrowDown");
    wait_until("the long board list opens", || {
        by_role_name(&root, "option", "Board 12").is_some()
    })
    .await;
    assert!(by_role_name(&root, "checkbox", "Turn PCB over for this half").is_some());
}
