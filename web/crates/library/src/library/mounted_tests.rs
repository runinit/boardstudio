use super::*;
use boardstudio_application::{Completion, Effect, OperationId, SaveResult, Session};
use boardstudio_core::CoreEngine;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::{all_by_role_name, by_role_name};
use boardstudio_web_ui_shared::test_wait::{
    WAIT_BOUND_MS, wait_for_step, wait_until, wait_until_step,
};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{
    Event as DomEvent, HtmlInputElement, KeyboardEvent as DomKeyboardEvent, KeyboardEventInit,
};

wasm_bindgen_test_configure!(run_in_browser);

struct Seed {
    state: Rc<RefCell<Option<Signal<u64>>>>,
    show_library: Rc<RefCell<Option<Signal<bool>>>>,
}

#[derive(Clone, Copy)]
struct LibraryTestMode(bool);

fn host() -> Element {
    let project_menu = try_use_context::<LibraryTestMode>().is_none_or(|mode| mode.0);
    let seed = use_context::<Rc<Seed>>();
    let version = use_signal(|| 0_u64);
    *seed.state.borrow_mut() = Some(version);
    let show_library = use_signal(|| true);
    *seed.show_library.borrow_mut() = Some(show_library);
    let notify_version = version;
    let runtime = use_context::<Rc<Runtime>>();
    runtime.subscribe(Rc::new(move || {
        let mut version = notify_version;
        version.set(version() + 1);
    }));
    let project_created = use_signal(|| None::<SetupGuideRequest>);
    let pending_new = use_signal(|| None::<PendingNewKeyboard>);
    let new_error = use_signal(String::new);
    use_context_provider(|| version);
    use_context_provider(|| project_created);
    use_context_provider(|| pending_new);
    use_context_provider(|| new_error);
    if show_library() {
        rsx! {
            details { "data-project-menu": "", open: true,
                summary { role: "button", "Project" }
                Library { project_menu }
            }
        }
    } else {
        rsx! { div { "Library unmounted" } }
    }
}

fn project_tools_host() -> Element {
    use boardstudio_web_ui_shared::ui::command::use_command_registry;
    let registry = use_command_registry();
    use_project_tools();
    let request = use_context::<ProjectToolRequest>();
    let seed = use_context::<Rc<Seed>>();
    let version = use_signal(|| 0u64);
    *seed.state.borrow_mut() = Some(version);
    let mut mounted = use_signal(|| false);
    let browser = ProjectBrowserState(use_signal(|| false));
    let created = use_signal(|| None::<SetupGuideRequest>);
    let pending = use_signal(|| None::<PendingNewKeyboard>);
    let error = use_signal(String::new);
    use_context_provider(|| version);
    use_context_provider(|| browser);
    use_context_provider(|| created);
    use_context_provider(|| pending);
    use_context_provider(|| error);
    let noop = use_callback(|()| {});
    let count = registry.commands().len();
    let captured = use_hook(|| {
        Rc::new(RefCell::new(
            None::<boardstudio_web_ui_shared::ui::command::Command>,
        ))
    });
    rsx! {
        ProjectHeaderMenu { project_name: "Test project", on_undo: noop, on_redo: noop, on_export: noop, on_settings: noop }
        for label in ["Rename project", "Open demo"] {
            {
                let capture = captured.clone();
                rsx! { button { onclick: move |_| *capture.borrow_mut() = registry.commands().into_iter().find(|c| c.label == label), "Capture {label}" } }
            }
        }
        button { onclick: move |_| captured.borrow().as_ref().unwrap().on_select.call(()), "Run captured tool" }
        button { onclick: move |_| mounted.set(!mounted()), "Toggle Library owner" }
        for label in ["Rename project", "Open demo"] {
            button { onclick: move |_| registry.commands().into_iter().find(|c| c.label == label).unwrap().on_select.call(()), "Palette {label}" }
        }
        output { role: "status", aria_label: "Registered tools", "{count}" }
        output { role: "status", aria_label: "Header version", "{version}" }
        output { role: "status", aria_label: "Tool pending", "{request.0().is_some()}" }
        if mounted() { Library { project_menu: false } }
    }
}

#[wasm_bindgen_test]
async fn palette_project_tools_consume_once_and_reject_rename_after_project_switch() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    let runtime = support::new_runtime();
    let (session, core) = accepted(ProjectDoc::empty("palette-tools-a", "Project A"));
    support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(project_tools_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(seed.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    let status = |name| by_role_name(&root, "status", name).map(|e| e.text_content().unwrap());
    let click = |name| by_role_name(&root, "button", name).unwrap().click();
    wait_until("header tools registered", || {
        status("Registered tools").is_some_and(|s| s == "7")
    })
    .await;
    click("Capture Rename project");
    let mut version = seed.state.borrow().unwrap();
    version += 1;
    wait_until("same project refreshed", || {
        status("Header version").as_deref() == Some("1")
    })
    .await;
    click("Run captured tool");
    wait_until("rename queued before Library mount", || {
        status("Tool pending").as_deref() == Some("true")
    })
    .await;
    let (session, core) = accepted(ProjectDoc::empty("palette-tools-b", "Project B"));
    support::install(&runtime, session, core);
    let mut version = seed.state.borrow().unwrap();
    version += 1;
    click("Toggle Library owner");
    wait_until("old project request consumed", || {
        status("Tool pending").as_deref() == Some("false")
            && by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "textbox", "Project name").is_none(),
        "stale rename must not select the new project"
    );
    click("Run captured tool");
    assert_eq!(
        status("Tool pending").as_deref(),
        Some("false"),
        "old project callback must remain stale"
    );
    click("Palette Rename project");
    wait_until("current project rename focused", || {
        by_role_name(&root, "textbox", "Project name")
            .is_some_and(|e| document.active_element().as_ref() == Some(e.as_ref()))
    })
    .await;
    assert_eq!(field().value(), "Project B");
    assert_eq!(status("Tool pending").as_deref(), Some("false"));
    click("Toggle Library owner");
    wait_until("Library unmounted", || {
        by_role_name(&root, "textbox", "Project name").is_none()
    })
    .await;
    click("Toggle Library owner");
    wait_until("Library remounted", || {
        by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "textbox", "Project name").is_none(),
        "consumed rename must not replay"
    );
    click("Capture Open demo");
    version += 1;
    wait_until("same project refreshed before demo", || {
        status("Header version").as_deref() == Some("3")
    })
    .await;
    click("Run captured tool");
    wait_until("demo tool focused", || {
        document
            .active_element()
            .is_some_and(|e| e.text_content().as_deref() == Some("Open demo"))
    })
    .await;
    let demo = document.active_element().unwrap();
    assert_eq!(demo.get_attribute("aria-expanded").as_deref(), Some("true"));
    assert_eq!(status("Tool pending").as_deref(), Some("false"));
    click("Toggle Library owner");
    wait_until("demo owner unmounted", || !root.contains(Some(&demo))).await;
    click("Toggle Library owner");
    wait_until("demo owner remounted", || {
        by_role_name(&root, "button", "Setup guide").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "button", "Open demo")
            .unwrap()
            .get_attribute("aria-expanded")
            .as_deref()
            == Some("false"),
        "consumed demo must not replay"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "palette-tools-b"
    );
    root.remove();
}

fn saved_card_count(root: &web_sys::Element) -> u32 {
    root.query_selector_all("[data-project-id] [aria-label^='Open '], [data-project-id] [aria-label^='Preview '], [data-project-id] [aria-label^='Recover from ']")
        .unwrap()
        .length()
}

/// The saved-keyboard listing's alert, when the listing failed.
fn list_alert(root: &web_sys::Element) -> Option<web_sys::Element> {
    root.query_selector("[role='alert']").unwrap()
}

/// The current dialog for deleting a keyboard is open (its DOM `open` attribute).
fn delete_dialog_open(root: &web_sys::Element) -> bool {
    root.query_selector("[role=alertdialog]")
        .unwrap()
        .is_some_and(|dialog| {
            dialog
                .closest("[data-slot=alert-dialog][data-state=open]")
                .unwrap()
                .is_some()
        })
}

/// The title of the only saved keyboard card, when the filtered listing shows one.
fn saved_card_title(root: &web_sys::Element) -> Option<String> {
    root.query_selector("[data-project-id] [aria-label^='Open '], [data-project-id] [aria-label^='Preview '], [data-project-id] [aria-label^='Recover from ']")
        .unwrap()
        .and_then(|action| action.first_element_child())
        .and_then(|preview| preview.next_element_sibling())
        .and_then(|title| title.text_content())
}

/// Drains held work, then takes one event-loop turn so a render that work scheduled can
/// run. This is not a wait for any condition. A negative assertion after it is only
/// drain-protected: no render signal proves the menu re-rendered (recorded in the Outcome).
async fn drain_and_yield(runtime: &Rc<Runtime>) {
    boardstudio_web_test_runtime::project_name_test_support::run_pending(runtime).await;
    gloo_timers::future::TimeoutFuture::new(0).await;
}

async fn wait_for_saved_cards(root: &web_sys::Element, expected: u32) {
    wait_until(&format!("{expected} saved keyboard cards load"), || {
        saved_card_count(root) == expected
    })
    .await;
}

async fn wait_for_project_cards(root: &web_sys::Element, expected: u32) {
    wait_until(&format!("{expected} saved project cards load"), || {
        saved_card_count(root) == expected
    })
    .await;
}

async fn wait_for_list_error(root: &web_sys::Element) -> web_sys::Element {
    wait_until("the saved-keyboard list error renders", || {
        list_alert(root).is_some()
    })
    .await;
    list_alert(root).expect("the list alert rendered")
}

#[wasm_bindgen_test]
async fn unreadable_record_keeps_healthy_card_and_requires_confirmation_to_delete() {
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let healthy = ProjectDoc::empty("partial-list-healthy", "Healthy saved keyboard");
    let mut future = ProjectDoc::empty("partial-list-future", "Future saved keyboard");
    future.format_version = 999;
    runtime
        .store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    runtime
        .store
        .save_document(&future, &Default::default())
        .await
        .unwrap();
    let (session, core) = accepted(healthy.clone());
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "partial-list-mounted-regression");
    wait_until(
        "healthy card and raw recovery actions render together",
        || {
            root.text_content()
                .unwrap_or_default()
                .contains("Healthy saved keyboard")
                && by_role_name(&root, "button", "Download raw saved data (JSON only)").is_some()
                && by_role_name(&root, "button", "Delete unreadable keyboard…").is_some()
        },
    )
    .await;
    for name in [
        "Download raw saved data (JSON only)",
        "Delete unreadable keyboard…",
    ] {
        let action = by_role_name(&root, "button", name).expect("recovery action is named");
        assert!(
            !action
                .get_attribute("class")
                .unwrap_or_default()
                .split_whitespace()
                .any(|class| class.starts_with("m1-")),
            "recovery actions do not depend on removed legacy class names"
        );
    }
    assert!(root.text_content().unwrap().contains("partial-list-future"));
    click(&root, "Delete unreadable keyboard…");
    wait_until("unreadable deletion exposes an Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    assert!(
        dialog.text_content().unwrap().contains(&future.id),
        "delete confirmation must name its target"
    );
    click(&dialog, "Cancel");
    wait_until("Cancel keeps the unreadable record", || {
        !delete_dialog_open(&root)
    })
    .await;
    drain_and_yield(&runtime).await;
    assert!(
        runtime
            .store
            .raw_project(&wasm_bindgen::JsValue::from_str(&future.id))
            .await
            .unwrap()
            .is_some()
    );
    click(&root, "Delete unreadable keyboard…");
    wait_until("unreadable deletion confirmation reopens", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    click(&dialog, "Delete unreadable keyboard");
    wait_until_step("confirmed unreadable record is deleted", async || {
        runtime
            .store
            .raw_project(&wasm_bindgen::JsValue::from_str(&future.id))
            .await
            .unwrap()
            .is_none()
    })
    .await;
    assert_eq!(
        runtime
            .store
            .load_document(healthy.id.clone())
            .await
            .unwrap()
            .unwrap(),
        healthy
    );
    clean_saved_projects(&runtime, &[&healthy.id]).await;
    remove_test_root("partial-list-mounted-regression");
}

/// Quota-recovery RED: an unreadable record with no comparable token must
/// stay deletable behind the explicit "permanently delete what we cannot
/// read" confirm, and the delete must use the token-independent by-key
/// path instead of the guarded delete that refuses tokenless records.
#[wasm_bindgen_test]
async fn tokenless_unreadable_record_stays_deletable_by_explicit_confirm() {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let healthy = ProjectDoc::empty("tokenless-healthy", "Healthy board");
    runtime
        .store
        .save_document(&healthy, &Default::default())
        .await
        .unwrap();
    let raw = js_sys::Object::new();
    js_sys::Reflect::set(
        &raw,
        &"id".into(),
        &wasm_bindgen::JsValue::from_str("tokenless-unreadable"),
    )
    .unwrap();
    let huge = js_sys::BigInt::new(&wasm_bindgen::JsValue::from_str(
        "1267650600228229401496703205376",
    ))
    .unwrap();
    js_sys::Reflect::set(&raw, &"marker".into(), &huge).unwrap();
    runtime
        .store
        .put_raw_record_for_test(raw.into())
        .await
        .unwrap();
    let (session, core) = accepted(healthy.clone());
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "tokenless-unreadable-mounted-regression");
    wait_until(
        "tokenless unreadable record renders with a delete action",
        || by_role_name(&root, "button", "Delete unreadable keyboard…").is_some(),
    )
    .await;
    let button = by_role_name(&root, "button", "Delete unreadable keyboard…").unwrap();
    assert!(
        button.get_attribute("disabled").is_none(),
        "records we cannot read stay deletable without a token"
    );
    click(&root, "Delete unreadable keyboard…");
    wait_until("tokenless deletion exposes an Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete unreadable keyboard?").unwrap();
    let message = dialog.text_content().unwrap();
    assert!(
        message
            .to_lowercase()
            .contains("permanently delete what we cannot read")
    );
    assert!(
        message.contains("tokenless-unreadable"),
        "delete confirmation must name its target"
    );
    click(&dialog, "Delete unreadable keyboard");
    wait_until_step("tokenless unreadable record is deleted", async || {
        runtime
            .store
            .list_projects()
            .await
            .unwrap()
            .unreadable
            .is_empty()
    })
    .await;
    assert_eq!(
        runtime
            .store
            .load_document(healthy.id.clone())
            .await
            .unwrap()
            .unwrap(),
        healthy
    );
    clean_saved_projects(&runtime, &[&healthy.id]).await;
    remove_test_root("tokenless-unreadable-mounted-regression");
}

#[wasm_bindgen_test]
async fn damaged_saved_preview_keeps_open_action_and_healthy_card_available() {
    let mut damaged = ProjectDoc::empty("damaged-preview", "Damaged preview keyboard");
    damaged.definitions = vec![
        serde_json::from_value(serde_json::json!({
            "id": "switch",
            "name": "Switch",
            "kind": "switch",
            "courtyard": [],
            "pads": []
        }))
        .unwrap(),
    ];
    damaged.parts.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch-1",
            "definitionId": "switch",
            "reference": "SW1",
            "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
            "side": "front",
            "keycap": { "x": 0.0, "y": 18.0 }
        }))
        .unwrap(),
    );
    let healthy = ProjectDoc::empty("healthy-preview", "Healthy keyboard");
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        results.clear();
        results.push_back(Ok(vec![damaged, healthy]));
    });

    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let (root, _mounted) = mount_menu(runtime, "damaged-preview-fallback-mounted-regression");
    wait_for_project_cards(&root, 2).await;

    let damaged_card = by_role_name(&root, "button", "Open Damaged preview keyboard")
        .expect("damaged preview remains an accessible Open action");
    assert!(damaged_card.get_attribute("disabled").is_none());
    assert!(
        damaged_card
            .text_content()
            .unwrap()
            .contains("Preview unavailable")
    );
    assert!(
        damaged_card
            .text_content()
            .unwrap()
            .contains("Open to check this keyboard")
    );
    assert!(
        by_role_name(&root, "button", "Open Healthy keyboard").is_some(),
        "one damaged preview does not hide a healthy saved card"
    );

    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    remove_test_root("damaged-preview-fallback-mounted-regression");
}

#[wasm_bindgen_test]
async fn saved_project_list_loading_error_and_retry_recover_in_mounted_library() {
    let (session, core) = accepted(ProjectDoc::empty("list-retry-current", "Current keyboard"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        results.clear();
        results.extend([
            Err(boardstudio_web_host::host::PersistError(
                "fixture list failure".into(),
            )),
            Ok(vec![ProjectDoc::empty(
                "list-retry-recovered",
                "Recovered keyboard",
            )]),
        ]);
    });

    let (root, _mounted) = mount_menu(runtime, "saved-project-list-retry-mounted-regression");
    // The test-only hold on this listing keeps the Loading status on screen for 120 ms.
    wait_until("the initial loading status renders", || {
        root.query_selector("[role='status']")
            .unwrap()
            .is_some_and(|status| {
                status
                    .text_content()
                    .unwrap()
                    .contains("Loading saved keyboards")
            })
    })
    .await;
    assert!(
        root.query_selector("[role='status']")
            .unwrap()
            .is_some_and(|status| status
                .text_content()
                .unwrap()
                .contains("Loading saved keyboards")),
        "initial list request renders its distinct loading status"
    );
    let alert = wait_for_list_error(&root).await;
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("could not be loaded")
    );
    assert!(
        alert.text_content().unwrap().contains("Try again"),
        "failed listing exposes retry"
    );
    assert_eq!(
        saved_card_count(&root),
        1,
        "the current project remains available while listing fails"
    );

    click(&root, "Try again");
    wait_for_project_cards(&root, 2).await;
    assert!(root.query_selector("[role='alert']").unwrap().is_none());
    assert!(root.text_content().unwrap().contains("Recovered keyboard"));
    assert!(
        root.query_selector("[data-project-id] [aria-current='true']")
            .unwrap()
            .is_some(),
        "retry keeps the current-project marker"
    );
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    remove_test_root("saved-project-list-retry-mounted-regression");
}

async fn clean_saved_projects(runtime: &Runtime, ids: &[&str]) {
    for id in ids {
        let _ = runtime.store.delete_project((*id).into()).await;
    }
}

async fn wait_outcome(
    runtime: &Rc<Runtime>,
    slot: &crate::operation_outcomes::OutcomeSlot,
) -> boardstudio_application::TerminalOutcome {
    let found = RefCell::new(None);
    let waited = wait_for_step("the operation settles", WAIT_BOUND_MS, async || {
        boardstudio_web_test_runtime::project_name_test_support::run_pending(runtime).await;
        let outcome = slot.borrow().clone();
        *found.borrow_mut() = outcome.clone();
        outcome.is_some()
    })
    .await;
    match waited {
        Ok(()) => found
            .into_inner()
            .expect("the operation recorded its outcome"),
        Err(message) => panic!(
            "{message}; outcome={:?}, accepted_revision={:?}, status={}",
            *slot.borrow(),
            runtime
                .model()
                .accepted
                .as_ref()
                .map(|snapshot| snapshot.document.revision),
            runtime.status()
        ),
    }
}

fn submit(
    runtime: &Rc<Runtime>,
    event: impl FnOnce(OperationId) -> Event,
) -> crate::operation_outcomes::OutcomeSlot {
    let operation = runtime.operation();
    let slot = boardstudio_web_test_runtime::project_name_test_support::observe(runtime, operation);
    runtime.submit(event(operation));
    slot
}

fn advance(session: &mut Session, core: &mut CoreEngine, initial: Vec<Effect>) {
    let mut pending = initial;
    while let Some(effect) = pending.pop() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => {
                let reply = core.handle(*request);
                pending.extend(session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(reply),
                }));
            }
            Effect::Persist {
                save_attempt_id, ..
            } => {
                pending.extend(session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                }));
            }
            _ => {}
        }
    }
}

fn accepted(document: ProjectDoc) -> (Session, CoreEngine) {
    let mut session = Session::new(boardstudio_core::CoreEngine::validate_open);
    let mut core = CoreEngine::new();
    let effects = session.submit(boardstudio_application::Event::Open {
        operation_id: OperationId(40),
        document,
    });
    advance(&mut session, &mut core, effects);
    assert!(session.read_model().accepted.is_some());
    (session, core)
}

fn mount_menu(runtime: Rc<Runtime>, root_id: &str) -> (web_sys::Element, MountedFixture) {
    mount_library(runtime, root_id, true)
}

fn mount_library(
    runtime: Rc<Runtime>,
    root_id: &str,
    project_menu: bool,
) -> (web_sys::Element, MountedFixture) {
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id(root_id);
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(LibraryTestMode(project_menu));
    dom.provide_root_context(runtime.clone());
    let mounted = MountedFixture::new(root.clone(), dom);
    (root, mounted)
}

#[wasm_bindgen_test]
async fn import_waits_for_storage_initialization() {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::set_storage_initializing(
        &runtime, true,
    );
    let (root, _mounted) = mount_library(runtime.clone(), "import-readiness", false);
    wait_until("import button renders", || {
        by_role_name(&root, "button", "Import project").is_some()
    })
    .await;
    let input = by_role_name(&root, "button", "Import project").unwrap();
    assert!(
        input.has_attribute("disabled"),
        "import must be disabled while storage initializes"
    );
    boardstudio_web_test_runtime::project_name_test_support::set_storage_initializing(
        &runtime, false,
    );
    wait_until(
        "import becomes enabled after storage initialization",
        || !input.has_attribute("disabled"),
    )
    .await;
    boardstudio_web_test_runtime::project_name_test_support::fail_storage_initialization(
        &runtime,
        "injected failure",
    );
    runtime.report("another status");
    wait_until(
        "storage failure remains visible and import is disabled",
        || {
            input.has_attribute("disabled")
                && root
                    .text_content()
                    .is_some_and(|text| text.contains("injected failure"))
        },
    )
    .await;
    root.remove();
    let (menu, _menu_mounted) = mount_menu(runtime, "import-readiness-menu");
    wait_until("menu import renders disabled after storage failure", || {
        by_role_name(&menu, "button", "Open project…")
            .is_some_and(|button| button.has_attribute("disabled"))
            && menu
                .text_content()
                .is_some_and(|text| text.contains("injected failure"))
    })
    .await;
    menu.remove();
}

#[wasm_bindgen_test]
async fn browser_selection_previews_without_opening_and_focuses_the_handoff() {
    let (session, core) = accepted(ProjectDoc::empty(
        "browser-selection-current",
        "Current keyboard",
    ));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    PROJECT_LIST_TEST_RESULTS.with(|results| {
        results.borrow_mut().clear();
        results.borrow_mut().push_back(Ok(vec![ProjectDoc::empty(
            "browser-selection-other",
            "Other keyboard",
        )]));
    });
    let (root, _mounted) = mount_library(runtime.clone(), "browser-selection-test-root", false);
    wait_for_saved_cards(&root, 2).await;
    click(&root, "Preview Other keyboard");
    wait_until("selection reveals and focuses Open keyboard", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element()
            .is_some_and(|element| element.id() == "m1-browser-open-keyboard")
    })
    .await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.id,
        "browser-selection-current"
    );
    assert!(
        root.query_selector("[aria-label='Keyboard details and starting guidance']")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap()
            .contains("Other keyboard")
    );
    click(&root, "Preview Current keyboard");
    wait_until("current keyboard preview is selected", || {
        root.query_selector("[aria-label='Keyboard details and starting guidance'] > h2")
            .unwrap()
            .and_then(|heading| heading.text_content())
            .as_deref()
            == Some("Current keyboard")
    })
    .await;
    click(&root, "Rename project");
    type_value(&field(), "Renamed current");
    let rename_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    assert_eq!(
        wait_outcome(&runtime, &rename_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("selected preview follows the accepted rename", || {
        root.query_selector("[aria-label='Keyboard details and starting guidance'] > h2")
            .unwrap()
            .and_then(|heading| heading.text_content())
            .as_deref()
            == Some("Renamed current")
    })
    .await;
    PROJECT_LIST_TEST_RESULTS.with(|results| results.borrow_mut().clear());
    root.remove();
}

fn click(root: &web_sys::Element, name: &str) {
    by_role_name(root, "button", name)
        .unwrap_or_else(|| panic!("missing button {name}"))
        .click();
}

fn remove_test_root(root_id: &str) {
    if let Some(root) = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(root_id)
        && let Some(parent) = root.parent_node()
    {
        let _ = parent.remove_child(&root);
    }
}

fn project_name_action() -> ProjectNameCommitAction {
    PROJECT_NAME_ACTION_PROBE.with(|probe| {
        probe
            .borrow()
            .clone()
            .expect("the mounted Library publishes the action used by its name field")
    })
}

#[wasm_bindgen_test]
async fn project_menu_exposes_the_whole_project_copy_action() {
    remove_test_root("project-copy-mounted-regression");
    let (session, core) = accepted(ProjectDoc::empty(
        "project-copy-ui",
        "Project copy keyboard",
    ));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime, "project-copy-mounted-regression");
    wait_until("the Project menu renders its copy action", || {
        by_role_name(&root, "button", "Save project copy…").is_some()
    })
    .await;

    let copy = by_role_name(&root, "button", "Save project copy…")
        .expect("the Project menu exposes Save project copy");
    let trigger = by_role_name(&root, "button", "Project").expect("project menu trigger");
    assert_eq!(copy.text_content().unwrap().trim(), "Save project copy…");
    assert!(
        !copy
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-"))
    );

    copy.dyn_into::<web_sys::HtmlElement>().unwrap().click();
    // The menu starts open, so "closed" is false before the click and proves its effect.
    wait_until("starting a project copy closes the menu", || {
        root.query_selector("[aria-label='Project menu']")
            .unwrap()
            .and_then(|menu| menu.closest("details").unwrap())
            .is_some_and(|menu| !menu.has_attribute("open"))
    })
    .await;
    let menu = root
        .query_selector("[aria-label='Project menu']")
        .unwrap()
        .unwrap()
        .closest("details")
        .unwrap()
        .unwrap();
    assert!(
        !menu.has_attribute("open"),
        "starting a project copy closes the menu"
    );
    assert_eq!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .active_element(),
        Some(trigger.into()),
        "closing the project menu returns focus to its trigger"
    );
    remove_test_root("project-copy-mounted-regression");
}

#[wasm_bindgen_test]
async fn project_delete_alert_cancel_keeps_the_named_project() {
    use boardstudio_web_ui_shared::test_role::by_role_name;
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let document = ProjectDoc::empty("alert-safe-project", "Safe project");
    let (session, core) = accepted(document.clone());
    runtime
        .store
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_library(runtime.clone(), "alert-safe-root", false);
    wait_for_saved_cards(&root, 1).await;
    let opener = by_role_name(&root, "button", "Delete Safe project").unwrap();
    opener.focus().unwrap();
    opener.click();
    wait_until("the named Alert Dialog opens", || {
        by_role_name(&root, "alertdialog", "Delete “Safe project”?").is_some()
    })
    .await;
    let dialog = by_role_name(&root, "alertdialog", "Delete “Safe project”?").unwrap();
    assert_eq!(dialog.get_attribute("aria-modal").as_deref(), Some("true"));
    let confirm = by_role_name(&dialog, "button", "Delete keyboard").unwrap();
    assert!(
        !confirm
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-"))
    );
    let cancel = by_role_name(&dialog, "button", "Cancel").unwrap();
    cancel.click();
    wait_until("Cancel closes the named Alert Dialog", || {
        by_role_name(&root, "alertdialog", "Delete “Safe project”?").is_none()
    })
    .await;
    assert!(
        runtime
            .store
            .load_document(document.id.clone())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(runtime.model().accepted.unwrap().document.id, document.id);
    runtime.store.delete_project(document.id).await.unwrap();
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn current_project_delete_cancel_and_confirm_use_the_observed_replacement_path() {
    remove_test_root("delete-current-mounted-regression");
    remove_test_root("delete-current-failure-regression");
    let document = ProjectDoc::empty("delete-current-ui", "Current board");
    let (session, core) = accepted(document.clone());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    clean_saved_projects(
        &runtime,
        &[
            "delete-current-ui",
            "delete-failure-current",
            "delete-failure-alpha",
        ],
    )
    .await;
    runtime
        .store
        .save_document(&document, &Default::default())
        .await
        .unwrap();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "delete-current-mounted-regression");
    wait_for_saved_cards(&root, 1).await;

    let trigger = by_role_name(&root, "button", "Delete Current board").unwrap();
    trigger.focus().unwrap();
    trigger.click();
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    let dialog = by_role_name(&root, "alertdialog", "Delete “Current board”?").unwrap();
    assert_eq!(dialog.get_attribute("aria-modal").as_deref(), Some("true"));
    click(&dialog, "Cancel");
    wait_until("Cancel closes the delete dialog", || {
        !delete_dialog_open(&root)
    })
    .await;
    // Cancel submits nothing: drain held work, then the store must still hold the record.
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    assert!(
        runtime
            .store
            .load_document("delete-current-ui".into())
            .await
            .unwrap()
            .is_some()
    );
    // Trusted focus trapping and restoration run in the packaged workflow, which serves the primitive asset.
    trigger.click();
    wait_until("the delete dialog reopens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");
    let replacement = RefCell::new(None::<String>);
    wait_until_step(
        "the durable fallback is active and deletion closes its dialog",
        async || {
            let accepted = runtime.model().accepted;
            if let Some(accepted) = accepted
                && accepted.document.id != "delete-current-ui"
                && runtime.store.active_project_id("").unwrap() == accepted.document.id
                && !delete_dialog_open(&root)
                && runtime
                    .store
                    .load_document("delete-current-ui".into())
                    .await
                    .unwrap()
                    .is_none()
            {
                *replacement.borrow_mut() = Some(accepted.document.id.clone());
                return true;
            }
            false
        },
    )
    .await;
    let replacement_id = replacement
        .into_inner()
        .expect("the durable fallback must precede deletion");
    assert_eq!(runtime.store.active_project_id("").unwrap(), replacement_id);
    assert!(!delete_dialog_open(&root));
    clean_saved_projects(&runtime, &[&replacement_id]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn current_project_delete_sorts_remaining_records_by_stored_name() {
    let current = ProjectDoc::empty("delete-sort-current", "Current keyboard");
    let alpha = ProjectDoc::empty("delete-sort-alpha", "Alpha keyboard");
    let unnamed = ProjectDoc::empty("delete-sort-unnamed", "");
    let (session, core) = accepted(current.clone());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    clean_saved_projects(
        &runtime,
        &[
            "delete-sort-current",
            "delete-sort-alpha",
            "delete-sort-unnamed",
        ],
    )
    .await;
    for document in [&current, &alpha, &unnamed] {
        runtime
            .store
            .save_document(document, &Default::default())
            .await
            .unwrap();
    }
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let (root, _mounted) = mount_menu(runtime.clone(), "delete-current-sort-regression");
    wait_for_saved_cards(&root, 3).await;
    click(&root, "Delete Current keyboard");
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");

    wait_until_step(
        "the raw blank name sorts before the displayed Untitled fallback",
        async || {
            runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.document.id == "delete-sort-unnamed")
                && runtime
                    .store
                    .load_document("delete-sort-current".into())
                    .await
                    .unwrap()
                    .is_none()
        },
    )
    .await;
    assert!(
        runtime
            .store
            .load_document("delete-sort-alpha".into())
            .await
            .unwrap()
            .is_some()
    );
    clean_saved_projects(&runtime, &["delete-sort-alpha", "delete-sort-unnamed"]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn failed_current_project_replacement_keeps_the_record_and_dialog_available() {
    remove_test_root("delete-current-failure-regression");
    let current = ProjectDoc::empty("delete-failure-current", "Current board");
    let candidate = ProjectDoc::empty("delete-failure-alpha", "Alpha board");
    let (session, core) = accepted(current.clone());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    clean_saved_projects(
        &runtime,
        &["delete-failure-current", "delete-failure-alpha"],
    )
    .await;
    for document in [&current, &candidate] {
        runtime
            .store
            .save_document(document, &Default::default())
            .await
            .unwrap();
    }
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
        &runtime,
        "replacement storage failed",
    );
    let (root, _mounted) = mount_menu(runtime.clone(), "delete-current-failure-regression");
    wait_for_saved_cards(&root, 2).await;
    click(&root, "Delete Current board");
    wait_until("the delete dialog opens", || delete_dialog_open(&root)).await;
    click(&root, "Delete keyboard");

    wait_until(
        "a failed replacement stays actionable in the dialog",
        || {
            root.query_selector("[role=alertdialog] [role=alert]")
                .unwrap()
                .is_some()
        },
    )
    .await;
    assert!(delete_dialog_open(&root));
    assert_eq!(
        runtime.model().accepted.unwrap().document.id,
        "delete-failure-current"
    );
    assert_eq!(
        runtime.model().lifecycle,
        boardstudio_application::Lifecycle::Ready
    );
    assert!(
        runtime
            .store
            .load_document("delete-failure-current".into())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        runtime
            .store
            .load_document("delete-failure-alpha".into())
            .await
            .unwrap()
            .is_some()
    );
    click(&root, "Delete keyboard");
    wait_until_step(
        "the still-open dialog can retry and finish deletion",
        async || {
            runtime
                .model()
                .accepted
                .as_ref()
                .is_some_and(|accepted| accepted.document.id == "delete-failure-alpha")
                && runtime
                    .store
                    .load_document("delete-failure-current".into())
                    .await
                    .unwrap()
                    .is_none()
        },
    )
    .await;
    clean_saved_projects(&runtime, &["delete-failure-alpha"]).await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn noncurrent_delete_preserves_accepted_document_and_undo_history() {
    remove_test_root("delete-noncurrent-mounted-regression");
    let current = ProjectDoc::empty("delete-noncurrent-active", "Current board");
    let saved = ProjectDoc::empty("delete-noncurrent-target", "Other board");
    let (session, core) = accepted(current.clone());
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    clean_saved_projects(
        &runtime,
        &["delete-noncurrent-active", "delete-noncurrent-target"],
    )
    .await;
    for document in [&current, &saved] {
        runtime
            .store
            .save_document(document, &Default::default())
            .await
            .unwrap();
    }
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);

    let initial = runtime.model().accepted.unwrap();
    let mut renamed = (*initial.document).clone();
    renamed.name = "Current board edited".into();
    let edit = submit(&runtime, |operation_id| {
        boardstudio_web_test_runtime::project_name_test_support::fixed_command_event(
            operation_id,
            EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "delete-noncurrent-history-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec![renamed.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(renamed),
                },
            },
        )
    });
    assert_eq!(
        wait_outcome(&runtime, &edit).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let before_delete = runtime.model().accepted.unwrap();
    runtime
        .delete_saved_project("delete-noncurrent-target".into())
        .await
        .unwrap();
    let after_delete = runtime.model().accepted.unwrap();
    assert_eq!(after_delete.document, before_delete.document);
    assert_eq!(after_delete.session_epoch, before_delete.session_epoch);
    assert_eq!(after_delete.token, before_delete.token);
    assert!(
        runtime
            .store
            .load_document("delete-noncurrent-target".into())
            .await
            .unwrap()
            .is_none()
    );

    let undo = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &undo).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Current board"
    );
    clean_saved_projects(&runtime, &["delete-noncurrent-active"]).await;
}

fn field() -> HtmlInputElement {
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .document_element()
        .unwrap();
    all_by_role_name(&root, "textbox", "Project name")
        .pop()
        .expect("mounted name field")
        .dyn_into()
        .unwrap()
}

fn name_field_rendered() -> bool {
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .document_element()
        .unwrap();
    !all_by_role_name(&root, "textbox", "Project name").is_empty()
}

fn search_field() -> HtmlInputElement {
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .document_element()
        .unwrap();
    by_role_name(&root, "searchbox", "Search saved keyboards")
        .expect("mounted search")
        .dyn_into()
        .unwrap()
}

fn type_value(input: &HtmlInputElement, value: &str) {
    let _ = input.focus();
    input.set_value(value);
    let event = DomEvent::new("input").unwrap();
    event.init_event_with_bubbles_and_cancelable("input", true, true);
    input.dispatch_event(&event).unwrap();
}

fn press(input: &HtmlInputElement, key: &str) {
    let init = KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = DomKeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    input.dispatch_event(&event).unwrap();
}

#[wasm_bindgen_test]
async fn project_menu_name_draft_survives_unrelated_accepted_revision_and_commits_latest_document()
{
    let (session, core) = accepted(ProjectDoc::empty("menu-name", "Sofle v2"));
    let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let mut search_one = ProjectDoc::empty("menu-search-alpha", "Alpha Board");
    search_one.revision = 1;
    let mut search_two = ProjectDoc::empty("menu-search-ergonomic", "Ergonomic 75%");
    search_two.revision = 1;
    let mut search_three = ProjectDoc::empty("menu-search-untitled", "  ");
    search_three.revision = 1;
    runtime
        .store
        .save_document(&search_one, &Default::default())
        .await
        .unwrap();
    runtime
        .store
        .save_document(&search_two, &Default::default())
        .await
        .unwrap();
    runtime
        .store
        .save_document(&search_three, &Default::default())
        .await
        .unwrap();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-mounted-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed.clone());
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_for_saved_cards(&root, 4).await;

    let project_menu = root
        .query_selector("[aria-label='Project menu']")
        .unwrap()
        .expect("menu instance uses the dropdown composition");
    let heading = project_menu.first_element_child().expect("menu heading");
    let actions = heading.next_element_sibling().expect("project actions");
    let current = actions.next_element_sibling().expect("current project");
    let content = current.next_element_sibling().expect("saved keyboards");
    let footer = content.next_element_sibling().expect("menu footer");
    assert!(
        footer.next_element_sibling().is_none(),
        "unexpected direct menu child"
    );
    assert!(heading.text_content().unwrap().contains("Keyboards"));
    assert_eq!(
        current.get_attribute("aria-label").as_deref(),
        Some("Current project")
    );
    assert!(content.text_content().unwrap().contains("Saved keyboards"));
    let new_project = by_role_name(&actions, "button", "New project").expect("new project action");
    assert!(
        !new_project
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class.starts_with("m1-")),
        "the project action does not depend on a dead legacy class"
    );
    assert!(by_role_name(&actions, "button", "Open project…").is_some());
    assert!(by_role_name(&footer, "button", "Setup guide").is_some());

    let search = search_field();
    assert_eq!(
        search.get_attribute("placeholder").as_deref(),
        Some("Search your keyboards")
    );
    let search_wrapper = search
        .closest("[data-slot=input-group]")
        .unwrap()
        .unwrap()
        .parent_element()
        .unwrap();
    search_wrapper.remove_attribute("class").unwrap();
    new_project.focus().unwrap();
    focus_library_search();
    wait_until(
        "search receives focus without a legacy wrapper class",
        || search.matches(":focus").unwrap(),
    )
    .await;
    type_value(&search, "  sOfLe  ");
    wait_until("the trimmed search leaves the current keyboard", || {
        saved_card_count(&root) == 1 && saved_card_title(&root).as_deref() == Some("Sofle v2")
    })
    .await;
    assert_eq!(
        saved_card_count(&root),
        1,
        "trimmed case-insensitive search includes the current keyboard"
    );
    assert_eq!(saved_card_title(&root).as_deref(), Some("Sofle v2"));
    type_value(&search, "alpha");
    wait_until("the alpha search shows Alpha Board", || {
        saved_card_count(&root) == 1 && saved_card_title(&root).as_deref() == Some("Alpha Board")
    })
    .await;
    assert_eq!(saved_card_count(&root), 1);
    assert_eq!(saved_card_title(&root).as_deref(), Some("Alpha Board"));
    type_value(&search, "untitled");
    wait_until("the untitled search shows the Untitled fallback", || {
        saved_card_count(&root) == 1
            && saved_card_title(&root).as_deref() == Some("Untitled keyboard")
    })
    .await;
    assert_eq!(saved_card_count(&root), 1);
    assert_eq!(
        saved_card_title(&root).as_deref(),
        Some("Untitled keyboard")
    );
    type_value(&search, "missing");
    wait_until("the missing search reports no matches", || {
        root.text_content()
            .unwrap()
            .contains("No keyboards match your search.")
    })
    .await;
    assert!(
        root.text_content()
            .unwrap()
            .contains("No keyboards match your search.")
    );
    let clear_search = by_role_name(&root, "button", "Clear search").unwrap();
    clear_search.click();
    wait_until("clearing the search restores all four saved cards", || {
        search.value().is_empty() && saved_card_count(&root) == 4
    })
    .await;
    assert_eq!(search.value(), "");
    assert_eq!(saved_card_count(&root), 4);

    assert_eq!(field().value(), "Sofle v2");
    let label = field().closest("[data-slot=label]").unwrap().unwrap();
    assert_eq!(label.text_content().unwrap().trim(), "Current project");
    type_value(&field(), "  My custom keyboard  ");

    let mut unrelated = snapshot.document.as_ref().clone();
    unrelated
        .parameters
        .insert("independent".into(), serde_json::json!(42));
    let unrelated_slot = submit(&runtime, |operation_id| {
        boardstudio_web_test_runtime::project_name_test_support::fixed_command_event(
            operation_id,
            EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "project-name-unrelated-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["independent".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated),
                },
            },
        )
    });
    assert_eq!(
        wait_outcome(&runtime, &unrelated_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    drain_and_yield(&runtime).await;

    assert_eq!(
        field().value(),
        "  My custom keyboard  ",
        "an unrelated accepted revision must not discard the menu's active name draft"
    );
    let latest = runtime.model().accepted.unwrap();
    let rename_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    assert_eq!(
        wait_outcome(&runtime, &rename_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the committed name renders in the field", || {
        field().value() == "My custom keyboard"
    })
    .await;
    let committed = runtime.model().accepted.unwrap();
    assert_eq!(committed.document.revision, latest.document.revision + 1);
    assert_eq!(committed.document.name, "My custom keyboard");
    assert_eq!(
        committed.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "the latest accepted unrelated field must survive the rename"
    );
    assert_eq!(field().value(), "My custom keyboard");
    let stored = runtime
        .store
        .load_document("menu-name".into())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.name, "My custom keyboard",
        "the rename must reach durable browser storage"
    );
    assert_eq!(
        stored.parameters.get("independent"),
        Some(&serde_json::json!(42))
    );

    let undo_slot = submit(&runtime, |operation_id| Event::Undo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &undo_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let undone = runtime.model().accepted.unwrap();
    assert_eq!(undone.document.name, "Sofle v2");
    assert_eq!(
        undone.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "Undoing the rename must leave the earlier unrelated edit accepted"
    );
    wait_until("the field follows Undo to the earlier name", || {
        field().value() == "Sofle v2"
    })
    .await;

    let redo_slot = submit(&runtime, |operation_id| Event::Redo { operation_id });
    assert_eq!(
        wait_outcome(&runtime, &redo_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let redone = runtime.model().accepted.unwrap();
    assert_eq!(redone.document.name, "My custom keyboard");
    wait_until("the field follows Redo to the renamed keyboard", || {
        field().value() == "My custom keyboard"
    })
    .await;

    type_value(&field(), "Keyboard entered");
    let enter_slot =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    press(&field(), "Enter");
    assert_eq!(
        wait_outcome(&runtime, &enter_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    let entered = runtime.model().accepted.unwrap();
    assert_eq!(entered.document.name, "Keyboard entered");

    type_value(&field(), "Canceled draft");
    press(&field(), "Escape");
    wait_until("Escape restores the accepted name", || {
        field().value() == "Keyboard entered"
    })
    .await;
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "   ");
    let _ = field().blur();
    wait_until("a blank draft restores the accepted name", || {
        field().value() == "Keyboard entered"
    })
    .await;
    assert_eq!(field().value(), "Keyboard entered");
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "Keyboard entered");
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        entered.document.revision
    );

    type_value(&field(), "Draft before same-project reopen");
    let reopen_slot = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: ProjectDoc::empty("menu-name", "Sofle v2"),
    });
    assert_eq!(
        wait_outcome(&runtime, &reopen_slot).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the same-project reopen restores the accepted name", || {
        field().value() == "Sofle v2"
    })
    .await;
    type_value(&field(), "Stale same-name epoch draft");
    let same_name_reopen = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: ProjectDoc::empty("menu-name", "Sofle v2"),
    });
    assert_eq!(
        wait_outcome(&runtime, &same_name_reopen).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the same-name reopen replaces the stale draft", || {
        field().value() == "Sofle v2"
    })
    .await;
    let _ = field().blur();
    drain_and_yield(&runtime).await;
    assert_eq!(runtime.model().accepted.unwrap().document.revision, 0);

    let reopen_saved = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document: stored.clone(),
    });
    assert_eq!(
        wait_outcome(&runtime, &reopen_saved).await,
        boardstudio_application::TerminalOutcome::Completed
    );
    wait_until("the reopened field shows the saved name", || {
        field().value() == stored.name
    })
    .await;
    let reopened = runtime.model().accepted.unwrap();
    assert_eq!(reopened.document.name, stored.name);
    assert_eq!(
        reopened.document.parameters.get("independent"),
        Some(&serde_json::json!(42)),
        "reopening the saved renamed document restores its unrelated accepted data"
    );
    assert_eq!(field().value(), stored.name);
    let _ = runtime.store.delete_project("menu-name".into()).await;
    let _ = runtime
        .store
        .delete_project("menu-search-alpha".into())
        .await;
    let _ = runtime
        .store
        .delete_project("menu-search-ergonomic".into())
        .await;
    let _ = runtime
        .store
        .delete_project("menu-search-untitled".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn retained_project_name_action_is_rejected_after_library_unmount() {
    let (session, core) = accepted(ProjectDoc::empty("menu-name-unmount", "Original"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-unmount-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed.clone());
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the mounted Library publishes its name action", || {
        PROJECT_NAME_ACTION_PROBE.with(|probe| probe.borrow().is_some())
            && seed.show_library.borrow().is_some()
    })
    .await;

    let retained_action = project_name_action();
    assert!(retained_action.mounted.get());
    let mut show_library = seed
        .show_library
        .borrow()
        .expect("host exposes its mounted state");
    show_library.set(false);
    wait_until(
        "Library cleanup retires the exact guard captured by its name action",
        || !retained_action.mounted.get(),
    )
    .await;

    let next_operation =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    retained_action.commit("Stale retained rename");
    drain_and_yield(&runtime).await;
    assert!(
        next_operation.borrow().is_none(),
        "the retained name action must not submit after unmount"
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.name, "Original");
    assert_eq!(accepted.document.revision, 0);
    let _ = runtime
        .store
        .delete_project("menu-name-unmount".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}

#[wasm_bindgen_test]
async fn pending_project_rename_preserves_typing_and_escape_does_not_commit() {
    let (session, core) = accepted(ProjectDoc::empty("menu-name-typing", "Original"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-typing-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the Library mounts the name field", name_field_rendered).await;

    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Committed");
    let _ = field().blur();
    wait_until_step("the rename reaches the gated Core reply", async || {
        support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    type_value(&field(), "Canceled");
    press(&field(), "Escape");
    let _ = field().blur();
    release.send(()).unwrap();
    wait_until_step("the gated rename lands as Committed", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Committed"
    })
    .await;
    // Escape and blur submit nothing after the landing: drain held work, then recheck.
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Committed",
        "Escape and blur must not queue a rename of the old accepted name"
    );
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Next committed");
    let _ = field().blur();
    wait_until_step("the next rename reaches the gated Core reply", async || {
        support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    type_value(&field(), "Still typing");
    release.send(()).unwrap();
    wait_until_step(
        "the older landing is accepted as Next committed",
        async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.name == "Next committed"
        },
    )
    .await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Next committed"
    );
    // The DOM is checked after the landing's Core state; no render signal proves the
    // menu re-rendered, so this is drain-protected only (recorded in the Outcome).
    drain_and_yield(&runtime).await;
    assert_eq!(
        field().value(),
        "Still typing",
        "older landing preserves the newer uncommitted draft"
    );
    let _ = field().blur();
    wait_until_step("the blur of Still typing lands", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Still typing"
    })
    .await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Still typing"
    );
    runtime
        .store
        .delete_project("menu-name-typing".into())
        .await
        .unwrap();
    root.remove();
}

async fn mounted_name_field(
    project_id: &str,
) -> (
    Rc<crate::runtime::Runtime>,
    web_sys::Element,
    MountedFixture,
) {
    let (session, core) = accepted(ProjectDoc::empty(project_id, "Original"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the Library mounts the name field", name_field_rendered).await;
    (runtime, root, mounted)
}

fn name_alert(root: &web_sys::Element) -> Option<String> {
    root.query_selector("[role=alert]")
        .unwrap()
        .and_then(|alert| alert.text_content())
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_project_name_restores_the_accepted_name_with_an_inline_failure() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-unchanged-failure").await;
    support::fail_next_core_reply(&runtime, "name executor failed");
    type_value(&field(), "Rejected");
    let _ = field().blur();
    wait_until_step(
        "the unchanged draft restores with its inline failure",
        async || {
            support::run_pending(&runtime).await;
            field().value() == "Original" && name_alert(&root).is_some()
        },
    )
    .await;
    assert_eq!(runtime.model().accepted.unwrap().document.name, "Original");
    assert_eq!(field().value(), "Original", "the unchanged draft restores");
    assert!(name_alert(&root).is_some_and(|message| message.contains("name executor failed")));
    runtime
        .store
        .delete_project("menu-name-unchanged-failure".into())
        .await
        .unwrap();
    root.remove();
}

#[wasm_bindgen_test]
async fn a_newer_project_name_draft_survives_an_older_failure_that_reports_inline() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-newer-failure").await;
    support::fail_next_core_reply(&runtime, "name executor failed");
    type_value(&field(), "Rejected");
    let _ = field().blur();
    // The older rename has not settled when the user types the next draft.
    type_value(&field(), "Still typing");
    // The inline failure renders with the draft, so its presence is the render sentinel.
    wait_until_step(
        "the older failure reports beside the newer draft",
        async || {
            support::run_pending(&runtime).await;
            name_alert(&root).is_some()
        },
    )
    .await;
    assert_eq!(field().value(), "Still typing", "a newer draft survives");
    assert!(
        name_alert(&root).is_some_and(|message| message.contains("name executor failed")),
        "the older failure reports inline beside the newer text"
    );
    runtime
        .store
        .delete_project("menu-name-newer-failure".into())
        .await
        .unwrap();
    root.remove();
}

#[wasm_bindgen_test]
async fn two_project_name_commits_keep_the_latest_observation_and_both_undo_steps() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let (runtime, root, _mounted) = mounted_name_field("menu-name-latest-observation").await;
    let (mut first_entered, first_release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "First name");
    let _ = field().blur();
    wait_until_step(
        "the first rename reaches its gated Core reply",
        async || {
            support::drive_pending(&runtime);
            matches!(first_entered.try_recv(), Ok(Some(())))
        },
    )
    .await;

    let (mut second_entered, second_release) = support::gate_next_core_reply(&runtime);
    type_value(&field(), "Latest name");
    let _ = field().blur();
    first_release.send(()).unwrap();
    wait_until_step(
        "the latest rename reaches its gated Core reply",
        async || {
            support::drive_pending(&runtime);
            matches!(second_entered.try_recv(), Ok(Some(())))
        },
    )
    .await;
    wait_until("the first landing is accepted as First name", || {
        runtime.model().accepted.unwrap().document.name == "First name"
    })
    .await;
    gloo_timers::future::TimeoutFuture::new(0).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "First name"
    );
    assert_eq!(
        field().value(),
        "Latest name",
        "the first landing cannot restore over the latest committed name"
    );
    assert!(name_alert(&root).is_none());

    second_release.send(()).unwrap();
    wait_until_step("the latest rename lands as Latest name", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.name == "Latest name"
    })
    .await;
    drain_and_yield(&runtime).await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Latest name"
    );
    assert_eq!(field().value(), "Latest name");
    assert!(name_alert(&root).is_none());
    for expected in ["First name", "Original"] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        wait_until_step("Undo lands and the field follows it", async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.name == expected
                && field().value() == expected
        })
        .await;
        assert_eq!(runtime.model().accepted.unwrap().document.name, expected);
        assert_eq!(field().value(), expected, "the settled field follows Undo");
    }
    runtime
        .store
        .delete_project("menu-name-latest-observation".into())
        .await
        .unwrap();
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn delayed_rename_persist_after_owner_replacement_cannot_update_new_project() {
    let (session, core) = accepted(ProjectDoc::empty("menu-name-delayed", "Original"));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let seed = Rc::new(Seed {
        state: Rc::new(RefCell::new(None)),
        show_library: Rc::new(RefCell::new(None)),
    });
    let root = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();
    root.set_id("project-name-delayed-regression");
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(seed);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the Library mounts the name field", name_field_rendered).await;

    let (mut entered, release) =
        boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(&runtime);
    type_value(&field(), "Delayed old rename");
    let slot = boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let _ = field().blur();
    wait_until_step("the actual Persist effect reaches the gate", async || {
        boardstudio_web_test_runtime::project_name_test_support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    let (replacement, _replacement_core) =
        accepted(ProjectDoc::empty("menu-name-replacement", "Replacement"));
    boardstudio_web_test_runtime::project_name_test_support::replace_session(&runtime, replacement);
    wait_until("the replacement session renders its name", || {
        field().value() == "Replacement"
    })
    .await;
    release.send(()).expect("release the captured old save");
    wait_until_step(
        "the released old save finishes before fixture cleanup",
        async || {
            runtime
                .store
                .load_document("menu-name-delayed".into())
                .await
                .unwrap()
                .is_some_and(|document| document.name == "Delayed old rename")
        },
    )
    .await;
    drain_and_yield(&runtime).await;
    assert!(
        slot.borrow().is_none(),
        "a completion owned by the prior session epoch is ignored"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Replacement"
    );
    assert!(!runtime.status().contains("Saved locally"));
    let _ = runtime
        .store
        .delete_project("menu-name-delayed".into())
        .await;
    let _ = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .remove_child(&root);
}
